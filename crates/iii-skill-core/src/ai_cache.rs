//! Persistent skip-cache for the AI verification layer.
//!
//! The AI layer is the expensive layer. Pushing a new commit to a PR
//! re-validates every artifact in scope, not just the changed files (the
//! shell loop in `verify-*.sh` walks the configured glob each run). For a
//! repo with N workers, that's N Anthropic calls on every push — even
//! when only one worker actually changed.
//!
//! [`PassCache`] is the persistent skip layer: hash the inputs that
//! determine the verdict, store hashes of PASSes in a file, and on the
//! next run skip the API call when the hash is already present. The hash
//! includes the artifact text, the project rules, the system prompt, the
//! model name, and the doc type — so any rule edit, prompt change, or
//! model bump busts the cache automatically.
//!
//! Only PASSes are recorded. FAILs always re-run on the next invocation
//! so a flaky model response doesn't get pinned.
//!
//! File format: one 64-char hex SHA-256 per line. Lines starting with `#`
//! are treated as comments. Append-only; duplicate lines are harmless and
//! deduped at load time.
//!
//! Wired into CI via the `SKV_AI_CACHE` env var and `actions/cache` in
//! `action.yml`. Unset = no caching (current default for direct CLI use).

use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Version tag baked into the hash so the format can be evolved later.
/// Bumping this string invalidates every existing cache entry — do it
/// whenever the cache contract changes (new hash inputs, normalization
/// changes, etc).
const CACHE_KEY_VERSION: &str = "v1";

/// Compute the cache key for one AI check. All inputs that could change
/// the verdict must contribute — anything missing here would let a stale
/// PASS survive a real regression.
pub fn cache_key(
    artifact_text: &str,
    rules: &str,
    system_prompt: &str,
    model: &str,
    doc_type: Option<crate::docs::frontmatter::DocType>,
) -> String {
    let mut hasher = Sha256::new();
    // NUL separators between fields prevent boundary-ambiguity collisions
    // (e.g. `"ab" + "cd"` vs `"abc" + "d"`).
    hasher.update(CACHE_KEY_VERSION.as_bytes());
    hasher.update([0u8]);
    hasher.update(artifact_text.as_bytes());
    hasher.update([0u8]);
    hasher.update(rules.as_bytes());
    hasher.update([0u8]);
    hasher.update(system_prompt.as_bytes());
    hasher.update([0u8]);
    hasher.update(model.as_bytes());
    hasher.update([0u8]);
    let doc_type_tag = match doc_type {
        Some(t) => t.to_string(),
        None => "none".to_string(),
    };
    hasher.update(doc_type_tag.as_bytes());
    to_hex(&hasher.finalize())
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize]);
        s.push(HEX[(b & 0x0f) as usize]);
    }
    s
}

const HEX: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

/// On-disk PASS-skip cache. Load once at the start of a verify run, query
/// per artifact, and record on each PASS.
pub struct PassCache {
    path: PathBuf,
    seen: HashSet<String>,
}

impl PassCache {
    /// Open or initialize a cache file at `path`. A missing file is fine —
    /// the cache starts empty and the file is created on the first
    /// [`record`](Self::record) call. Returns an error only on I/O failures
    /// other than not-found.
    pub fn load(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let mut seen = HashSet::new();
        match OpenOptions::new().read(true).open(&path) {
            Ok(f) => {
                for line in BufReader::new(f).lines() {
                    let line = line?;
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    seen.insert(trimmed.to_string());
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        Ok(Self { path, seen })
    }

    /// True if the key is in the cache (PASS recorded on a previous run).
    pub fn contains(&self, key: &str) -> bool {
        self.seen.contains(key)
    }

    /// Record a PASS. Inserts into the in-memory set and appends one line
    /// to the cache file. The file is created with a header comment on
    /// first write so a human inspecting it sees what it is.
    pub fn record(&mut self, key: &str) -> io::Result<()> {
        if !self.seen.insert(key.to_string()) {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let is_new = !self.path.exists();
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        if is_new {
            writeln!(
                f,
                "# iii-skill-check AI pass cache — one SHA-256 per line; safe to delete"
            )?;
        }
        writeln!(f, "{key}")?;
        Ok(())
    }

    /// Path the cache is reading from / writing to. Exposed for diagnostic
    /// logging — not normally needed by callers.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
