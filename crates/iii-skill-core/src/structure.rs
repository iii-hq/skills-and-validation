use anyhow::Context;
use std::collections::HashSet;
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
/// Reads `<dir>/iii.worker.yaml`, `<dir>/README.md`, `<dir>/skill.md`, and any
/// `<dir>/skills/*.md` files. Returns one Violation per finding; empty Vec means
/// the artifacts are clean at this layer.
pub fn check(dir: &Path) -> anyhow::Result<Vec<Violation>> {
    let manifest = crate::introspect::read_manifest(dir)?;
    let name = &manifest.name;

    let readme_path = dir.join("README.md");
    let skill_path = dir.join("skill.md");
    let skills_dir = dir.join("skills");

    let readme = std::fs::read_to_string(&readme_path)
        .with_context(|| format!("reading {}", readme_path.display()))?;
    let skill = std::fs::read_to_string(&skill_path)
        .with_context(|| format!("reading {}", skill_path.display()))?;
    let leaves = list_existing_leaves(&skills_dir)?;

    let mut violations = Vec::new();
    violations.extend(check_required_sections(&readme));
    violations.extend(check_install_line(&readme, name));
    violations.extend(check_forbidden_install_patterns(&readme, name));

    let known_leaves: HashSet<String> = leaves.iter().map(|(n, _)| n.clone()).collect();

    let mut artifacts: Vec<(String, &str)> =
        vec![("README.md".to_string(), readme.as_str()), ("skill.md".to_string(), skill.as_str())];
    for (leaf, body) in &leaves {
        artifacts.push((format!("skills/{leaf}.md"), body.as_str()));
    }

    for (label, content) in &artifacts {
        violations.extend(check_llm_only_balance(label, content));
        violations.extend(check_human_only_balance(label, content));
        violations.extend(check_iii_links(label, content, name, &known_leaves));
    }

    Ok(violations)
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

fn check_iii_links(
    label: &str,
    content: &str,
    worker_name: &str,
    known_leaves: &HashSet<String>,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let prefix = format!("iii://{worker_name}/");
    for (i, line) in content.lines().enumerate() {
        let mut start = 0;
        while let Some(rel) = line[start..].find(&prefix) {
            let abs = start + rel;
            let after = &line[abs + prefix.len()..];
            let leaf: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '/')
                .collect();
            if !leaf.is_empty() && !known_leaves.contains(&leaf) {
                violations.push(Violation::error(
                    label,
                    Some(i + 1),
                    format!(
                        "iii://{worker_name}/{leaf} points to a leaf that is not registered (no skills/{leaf}.md)"
                    ),
                ));
            }
            start = abs + prefix.len();
        }
    }
    violations
}

fn list_existing_leaves(skills_dir: &Path) -> anyhow::Result<Vec<(String, String)>> {
    let mut leaves: Vec<(String, String)> = Vec::new();
    if !skills_dir.exists() {
        return Ok(leaves);
    }
    for entry in std::fs::read_dir(skills_dir)
        .with_context(|| format!("reading {}", skills_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("invalid skill filename: {}", path.display()))?
            .to_string();
        let body = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        leaves.push((name, body));
    }
    leaves.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(leaves)
}
