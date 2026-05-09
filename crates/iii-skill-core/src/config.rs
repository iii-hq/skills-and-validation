use anyhow::Context;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Schema version of the `.skill-check.yaml` itself.
    ///
    /// - `1`: implicit `mode: worker`. The original schema; consumers using v1
    ///   continue to validate worker dirs.
    /// - `2`: introduces the `mode` field (`worker` or `docs`) and the
    ///   `docs:` block. Worker behaviour is preserved when `mode: worker`.
    pub version: u32,
    /// Validation mode. Optional on v1 (defaults to `worker`); on v2 this is
    /// the field that picks the validation surface.
    #[serde(default)]
    pub mode: Option<Mode>,
    pub rules: Option<PathSource>,
    pub styles: Option<PathSource>,
    pub ai_check: AiCheck,
    /// Docs-mode glob lists. Only consulted when `mode == Docs`.
    pub docs: Option<DocsConfig>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Worker,
    Docs,
}

impl Config {
    /// Resolved validation mode. v1 with no `mode:` set falls back to worker
    /// mode; v2 requires `mode:` to be explicit (callers can choose to
    /// enforce that — `load` returns whatever the file declared).
    pub fn resolved_mode(&self) -> Mode {
        self.mode.unwrap_or(Mode::Worker)
    }
}

#[derive(Debug, Deserialize)]
pub struct PathSource {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct AiCheck {
    pub provider: String,
    pub model: String,
    pub api_key_env_var: String,
    pub max_tokens: u32,
}

#[derive(Debug, Deserialize, Default)]
pub struct DocsConfig {
    /// Glob patterns relative to the docs root. Files matching any include
    /// pattern are candidates for the skill set; the per-doc opt-in/out
    /// HTML comment can still override.
    #[serde(default)]
    pub include: Vec<String>,
    /// Glob patterns evaluated after `include` to drop matches.
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Load `.skill-check.yaml` from a path.
pub fn load(path: &Path) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let config: Config = serde_yaml::from_str(&content)
        .with_context(|| format!("parsing {}", path.display()))?;

    // v2 requires explicit mode. v1 implicitly = worker.
    if config.version >= 2 && config.mode.is_none() {
        anyhow::bail!(
            "{} is schema v{} but is missing required `mode:` (worker | docs)",
            path.display(),
            config.version
        );
    }
    if config.resolved_mode() == Mode::Docs && config.docs.is_none() {
        anyhow::bail!(
            "{} declares `mode: docs` but has no `docs:` block (need at least `docs.include`)",
            path.display()
        );
    }
    Ok(config)
}
