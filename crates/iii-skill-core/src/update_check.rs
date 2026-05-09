use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const REPO: &str = "iii-hq/skills-and-validation";
const CACHE_TTL_SECS: u64 = 24 * 60 * 60;

/// Result of an update check.
#[derive(Debug, Clone)]
pub enum UpdateStatus {
    /// The running binary is at the latest released version (or newer — a dev build).
    UpToDate { current: String, latest: String },
    /// A newer version is available.
    OutOfDate {
        current: String,
        latest: String,
        install_cmd: String,
    },
    /// The check was skipped (env var, dev build, network failure, parse failure).
    Skipped { reason: String },
}

/// Version baked into the binary at build time.
///
/// The release pipeline sets `RELEASE_VERSION` before `cargo build`; that
/// value is captured here via `option_env!`. Local dev builds without the
/// env var fall back to the workspace's `CARGO_PKG_VERSION`, which is
/// treated as a "dev" build and skips the update check.
pub fn installed_version() -> &'static str {
    option_env!("RELEASE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn is_release_build() -> bool {
    option_env!("RELEASE_VERSION").is_some()
}

/// Run the update check and return the resulting status.
///
/// Honors `SKV_NO_UPDATE_CHECK=1` (any non-empty value disables the check).
/// Caches the latest version in `~/.cache/skill-check/update-check.json` for
/// `CACHE_TTL_SECS`. A network failure on a stale cache falls through to
/// `Skipped` rather than failing the caller.
pub fn check() -> UpdateStatus {
    if std::env::var_os("SKV_NO_UPDATE_CHECK").is_some_and(|v| !v.is_empty()) {
        return UpdateStatus::Skipped {
            reason: "SKV_NO_UPDATE_CHECK is set".into(),
        };
    }
    if !is_release_build() {
        return UpdateStatus::Skipped {
            reason: "running a dev build (RELEASE_VERSION unset at compile time)".into(),
        };
    }

    let current = installed_version().to_string();
    let latest = match cached_or_fetched_latest() {
        Ok(v) => v,
        Err(reason) => return UpdateStatus::Skipped { reason },
    };

    if is_newer(&latest, &current) {
        UpdateStatus::OutOfDate {
            current,
            latest: latest.clone(),
            install_cmd: install_command(&latest),
        }
    } else {
        UpdateStatus::UpToDate { current, latest }
    }
}

fn install_command(_latest: &str) -> String {
    format!(
        "curl -fsSL https://raw.githubusercontent.com/{REPO}/latest/scripts/install.sh | bash"
    )
}

fn cached_or_fetched_latest() -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    if let Some(cached) = read_cache() {
        if now.saturating_sub(cached.checked_at) < CACHE_TTL_SECS {
            return Ok(cached.latest);
        }
    }

    let latest = fetch_latest_from_api()?;
    let _ = write_cache(&CacheEntry {
        checked_at: now,
        latest: latest.clone(),
    });
    Ok(latest)
}

fn fetch_latest_from_api() -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = reqwest::blocking::Client::new()
        .get(&url)
        .header("user-agent", concat!("skill-check/", env!("CARGO_PKG_VERSION")))
        .send()
        .map_err(|e| format!("API request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("API returned {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().map_err(|e| format!("parsing API JSON: {e}"))?;
    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "API response missing tag_name".to_string())?;
    Ok(tag.trim_start_matches('v').to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    checked_at: u64,
    latest: String,
}

fn cache_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("skill-check").join("update-check.json"))
}

fn read_cache() -> Option<CacheEntry> {
    let path = cache_path()?;
    let body = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&body).ok()
}

fn write_cache(entry: &CacheEntry) -> std::io::Result<()> {
    let Some(path) = cache_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string(entry).expect("CacheEntry is always serializable");
    std::fs::write(path, body)
}

/// True when `latest` parses as a strictly newer X.Y.Z than `current`. Any
/// non-X.Y.Z input (dev builds with a `-dev` suffix, etc.) is treated as
/// not-newer so we don't nag on local builds.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_xyz_versions() {
        assert_eq!(parse_version("0.1.5"), Some((0, 1, 5)));
        assert_eq!(parse_version("1.20.300"), Some((1, 20, 300)));
        assert_eq!(parse_version("0.1"), None);
        assert_eq!(parse_version("0.1.5-rc.1"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn is_newer_compares_strictly() {
        assert!(is_newer("0.1.5", "0.1.4"));
        assert!(is_newer("0.2.0", "0.1.99"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.4", "0.1.4"));
        assert!(!is_newer("0.1.3", "0.1.4"));
        assert!(!is_newer("0.1.0-rc.1", "0.1.0")); // unparseable -> not newer
        assert!(!is_newer("not-a-version", "0.1.0"));
    }

    #[test]
    fn install_command_uses_latest_floating_tag() {
        let cmd = install_command("0.1.5");
        assert!(cmd.contains("/latest/"));
        assert!(cmd.contains("install.sh"));
    }

    #[test]
    fn cache_roundtrip() {
        // Use a temp HOME so we don't clobber the user's real cache dir.
        let tmp = tempfile::tempdir().unwrap();
        let prev_xdg = std::env::var_os("XDG_CACHE_HOME");
        std::env::set_var("XDG_CACHE_HOME", tmp.path());

        let entry = CacheEntry {
            checked_at: 1_700_000_000,
            latest: "0.1.5".into(),
        };
        write_cache(&entry).unwrap();
        let read = read_cache().expect("cache read");
        assert_eq!(read.checked_at, 1_700_000_000);
        assert_eq!(read.latest, "0.1.5");

        match prev_xdg {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
    }
}
