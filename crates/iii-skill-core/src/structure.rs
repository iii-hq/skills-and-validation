use anyhow::Context;
use std::path::Path;

/// Severity of a single violation. `Error` fails the run; `Warning` is
/// surfaced through the same channels but does not affect exit code.
/// Defaults to `Error` so existing call sites that don't set the field
/// preserve current behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Severity {
    #[default]
    Error,
    Warning,
}

impl Severity {
    /// Human-readable lowercase label used in CLI output and matched by
    /// the annotation/summary scripts.
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub file: String,
    pub line: Option<usize>,
    pub message: String,
    pub severity: Severity,
}

impl Violation {
    /// Construct an error-severity violation. Most call sites should
    /// use this constructor; reserve direct struct-literal construction
    /// for code that needs to vary severity programmatically.
    pub fn error(file: impl Into<String>, line: Option<usize>, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line,
            message: message.into(),
            severity: Severity::Error,
        }
    }

    /// Construct a warning-severity violation. Warnings surface in the
    /// same output channels as errors but do not fail the run.
    pub fn warning(
        file: impl Into<String>,
        line: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            line,
            message: message.into(),
            severity: Severity::Warning,
        }
    }
}

/// Run Layer 1 deterministic structure checks against a worker dir.
///
/// Reads `<dir>/iii.worker.yaml`, `<dir>/README.md`, `<dir>/skill.md`, and the
/// source leaves under `<dir>/docs/leaves/*.md` (now inlined into the two
/// artifacts rather than rendered to a `skills/` dir). Returns one Violation
/// per finding; empty Vec means the artifacts are clean at this layer.
pub fn check(dir: &Path) -> anyhow::Result<Vec<Violation>> {
    let manifest = crate::introspect::read_manifest(dir)?;
    let name = &manifest.name;

    let readme_path = dir.join("README.md");
    let skill_path = dir.join("skill.md");

    let readme = std::fs::read_to_string(&readme_path)
        .with_context(|| format!("reading {}", readme_path.display()))?;
    let skill = std::fs::read_to_string(&skill_path)
        .with_context(|| format!("reading {}", skill_path.display()))?;
    let leaves = list_source_leaves(&dir.join("docs").join("leaves"))?;

    let mut violations = Vec::new();
    violations.extend(check_required_sections(&readme));
    violations.extend(check_install_line(&readme, name));
    violations.extend(check_forbidden_install_patterns(&readme, name));

    // Rendered artifacts: frontmatter present + visibility markers balanced.
    // (The renderer consumes markers, so a net imbalance here signals a
    // hand-edit of the generated file.)
    for (label, content) in [("README.md", readme.as_str()), ("skill.md", skill.as_str())] {
        violations.extend(check_frontmatter(label, content));
        violations.extend(check_llm_only_balance(label, content));
        violations.extend(check_human_only_balance(label, content));
    }

    // Source leaves: each must carry a top-level H1 (it becomes the `### Title`
    // when inlined under `## Additional HOWTOs`) and have balanced visibility
    // markers (an unclosed block would silently truncate the inlined output).
    for (leaf, body) in &leaves {
        let label = format!("docs/leaves/{leaf}.md");
        violations.extend(check_llm_only_balance(&label, body));
        violations.extend(check_human_only_balance(&label, body));
        violations.extend(check_leaf_h1(leaf, body));
    }

    Ok(violations)
}

/// `README.md` and `skill.md` must open with a YAML frontmatter block
/// carrying `name`, `description`, and `tags`. The renderer always emits it;
/// this catches a hand-edit that drops or mangles it.
fn check_frontmatter(label: &str, content: &str) -> Vec<Violation> {
    if !content.starts_with("---\n") {
        return vec![Violation::error(
            label,
            Some(1),
            "missing leading YAML frontmatter (`--- name/description/tags ---`)",
        )];
    }
    let after = &content[4..];
    let block = match after.find("\n---") {
        Some(end) => &after[..end],
        None => {
            return vec![Violation::error(
                label,
                Some(1),
                "frontmatter block is not closed with `---`",
            )]
        }
    };
    let mut violations = Vec::new();
    for key in ["name", "description", "tags"] {
        let present = block
            .lines()
            .any(|l| l.trim_start().starts_with(&format!("{key}:")));
        if !present {
            violations.push(Violation::error(
                label,
                None,
                format!("frontmatter missing `{key}:` (sourced from iii.worker.yaml)"),
            ));
        }
    }
    violations
}

fn check_required_sections(readme: &str) -> Vec<Violation> {
    let required = ["## Install", "## Quickstart", "## Configuration"];
    let mut violations = Vec::new();
    let mut last_pos: Option<usize> = None;
    for header in &required {
        match readme.find(header) {
            Some(pos) => {
                if let Some(prev) = last_pos {
                    if pos < prev {
                        violations.push(Violation::error(
                            "README.md",
                            None,
                            format!("section {header} appears out of expected order"),
                        ));
                    }
                }
                last_pos = Some(pos);
            }
            None => {
                violations.push(Violation::error(
                    "README.md",
                    None,
                    format!("missing required section {header}"),
                ));
            }
        }
    }
    violations
}

fn check_install_line(readme: &str, name: &str) -> Vec<Violation> {
    let expected = format!("iii worker add {name}");
    if readme.contains(&expected) {
        return Vec::new();
    }
    vec![Violation::error(
        "README.md",
        None,
        format!("install command should be `{expected}` matching iii.worker.yaml.name"),
    )]
}

fn check_forbidden_install_patterns(readme: &str, name: &str) -> Vec<Violation> {
    // Order matters: more specific patterns first so a single line gets the
    // most-informative violation when multiple patterns would match.
    let blocked: Vec<String> = vec![
        "cargo build".to_string(),
        "cargo install".to_string(),
        "--manifest | jq".to_string(),
        format!("{name} --help"),
        format!("{name} --manifest"),
    ];
    let mut violations = Vec::new();
    for (i, line) in readme.lines().enumerate() {
        let lower = line.to_lowercase();
        if let Some(needle) = blocked.iter().find(|n| lower.contains(&n.to_lowercase())) {
            violations.push(Violation::error(
                "README.md",
                Some(i + 1),
                format!(
                    "forbidden pattern `{needle}` in published README (binary verification or source-build steps belong in contributor docs, not the published README)"
                ),
            ));
        }
    }
    violations
}

fn check_llm_only_balance(label: &str, content: &str) -> Vec<Violation> {
    // Line-exact match so an inline backtick example inside prose doesn't
    // count as a real marker. Both HTML and MDX comment forms are
    // recognised by the predicates.
    let starts = content
        .lines()
        .filter(|l| crate::llm_only::is_llm_only_start(l))
        .count();
    let ends = content
        .lines()
        .filter(|l| crate::llm_only::is_llm_only_end(l))
        .count();
    if starts == ends {
        return Vec::new();
    }
    vec![Violation::error(
        label,
        None,
        format!("unbalanced llm-only blocks: {starts} start markers, {ends} end markers"),
    )]
}

fn check_human_only_balance(label: &str, content: &str) -> Vec<Violation> {
    let starts = content
        .lines()
        .filter(|l| crate::human_only::is_human_only_start(l))
        .count();
    let ends = content
        .lines()
        .filter(|l| crate::human_only::is_human_only_end(l))
        .count();
    if starts == ends {
        return Vec::new();
    }
    vec![Violation::error(
        label,
        None,
        format!("unbalanced human-only blocks: {starts} start markers, {ends} end markers"),
    )]
}

/// Each source leaf (`docs/leaves/<leaf>.md`) must carry a top-level H1: it
/// becomes the `### Title` when the leaf is inlined under `## Additional
/// HOWTOs`. The body is stripped of both visibility-block types first, so an
/// H1 that lives only inside an `llm-only` / `human-only` block doesn't count
/// (it wouldn't survive to the inlined output for at least one audience).
/// Worker mode only; docs mode has its own H1 expectations elsewhere.
fn check_leaf_h1(leaf: &str, body: &str) -> Vec<Violation> {
    let visible = crate::human_only::strip_human_only(&crate::llm_only::strip_llm_only(body));
    for line in visible.lines() {
        if line.trim_start().starts_with("# ") {
            return Vec::new();
        }
    }
    vec![Violation::error(
        format!("docs/leaves/{leaf}.md"),
        None,
        "missing top-level H1 (it becomes the `### Title` when inlined under `## Additional HOWTOs`; add a topical phrase, e.g. `# Sizing text before provider calls`)",
    )]
}

fn list_source_leaves(leaves_dir: &Path) -> anyhow::Result<Vec<(String, String)>> {
    let mut leaves: Vec<(String, String)> = Vec::new();
    if !leaves_dir.exists() {
        return Ok(leaves);
    }
    for entry in std::fs::read_dir(leaves_dir)
        .with_context(|| format!("reading {}", leaves_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("invalid leaf filename: {}", path.display()))?
            .to_string();
        let body = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        leaves.push((name, body));
    }
    leaves.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(leaves)
}
