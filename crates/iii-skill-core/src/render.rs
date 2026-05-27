use crate::human_only::{strip_human_only, unwrap_human_only};
use crate::llm_only::{strip_llm_only, unwrap_llm_only};
use anyhow::Context;
use std::path::{Path, PathBuf};

/// Output of rendering one worker: a single human-facing `README.md` and a
/// single agent-facing `skill.md`. Per-function leaves are inlined into both
/// under `## Additional HOWTOs`; there is no longer a `skills/` directory.
pub struct RenderOutput {
    pub readme: String,
    pub skill: String,
}

/// Which audience an artifact targets. Decides the visibility-block passes
/// applied to each partial: the README strips `llm-only` and reveals
/// `human-only`; `skill.md` does the inverse.
#[derive(Clone, Copy)]
enum Audience {
    Readme,
    Skill,
}

/// Render a worker dir into its two artifact strings (no IO writes).
///
/// Reads:
///   - `<dir>/iii.worker.yaml` for `name`, `description`, `tags` (frontmatter)
///   - `<dir>/config.yaml` (inlined verbatim into ## Configuration)
///   - `<dir>/docs/intro.md` (used in README and skill.md)
///   - `<dir>/docs/quickstart.md`
///   - `<dir>/docs/companions.md` (optional; appended inside ## Install in README only)
///   - `<dir>/docs/migration.md` (optional)
///   - `<dir>/docs/leaves/*.md` (inlined under ## Additional HOWTOs in both artifacts)
///
/// Does NOT parse `<dir>/src/` — function ids, signatures, descriptions, and
/// custom trigger payloads are auto-generated elsewhere.
pub fn render_worker(dir: &Path) -> anyhow::Result<RenderOutput> {
    let manifest = crate::introspect::read_manifest(dir)?;
    let name = &manifest.name;

    let intro = read_partial(&dir.join("docs").join("intro.md"))?;
    let quickstart = read_partial(&dir.join("docs").join("quickstart.md"))?;
    let config = read_partial(&dir.join("config.yaml"))?;
    let companions = read_optional_partial(&dir.join("docs").join("companions.md"))?;
    let migration = read_optional_partial(&dir.join("docs").join("migration.md"))?;

    // Leaf source bodies, alphabetical by leaf name. Inlined into both
    // artifacts; the per-audience visibility passes happen at render time.
    let mut leaves: Vec<(String, String)> = Vec::new();
    for (leaf_name, leaf_path) in list_leaves(dir)? {
        leaves.push((leaf_name, read_partial(&leaf_path)?));
    }

    let frontmatter = frontmatter_block(&manifest.name, manifest.description(), manifest.tags());

    let readme = render_readme(
        &frontmatter,
        name,
        &intro,
        &quickstart,
        &config,
        companions.as_deref(),
        migration.as_deref(),
        &leaves,
    );
    let skill = render_skill(&frontmatter, name, &intro, companions.as_deref(), &leaves);

    Ok(RenderOutput { readme, skill })
}

/// Build the leading YAML frontmatter block. Identical in README.md and
/// skill.md (audience-neutral: no llm-only / human-only content). `name` is
/// always present; `description` and `tags` are omitted when absent (the
/// structure layer warns about missing metadata). Serialized via serde_yaml
/// so values are quoted/escaped correctly.
fn frontmatter_block(name: &str, description: Option<&str>, tags: Option<&str>) -> String {
    #[derive(serde::Serialize)]
    struct Frontmatter<'a> {
        name: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tags: Option<&'a str>,
    }
    let yaml = serde_yaml::to_string(&Frontmatter {
        name,
        description,
        tags,
    })
    .expect("frontmatter struct always serializes");
    format!("---\n{yaml}---")
}

fn render_readme(
    frontmatter: &str,
    name: &str,
    intro: &str,
    quickstart: &str,
    config: &str,
    companions: Option<&str>,
    migration: Option<&str>,
    leaves: &[(String, String)],
) -> String {
    // Two-pass strip for each markdown partial: first drop every
    // `llm-only` block + inline (markers AND body — the README is the
    // human-facing artifact, so LLM-only prose must not leak through as
    // visible markdown). Then `unwrap_human_only` removes human-only
    // markers and expands human-only inlines so the README reader sees
    // their payload. config.yaml is YAML inside a code fence; skip it.
    let intro = unwrap_human_only(&strip_llm_only(intro));
    let quickstart = unwrap_human_only(&strip_llm_only(quickstart));
    let companions = companions.map(|c| unwrap_human_only(&strip_llm_only(c)));
    let migration = migration.map(|m| unwrap_human_only(&strip_llm_only(m)));

    let mut install_section = format!(
        "## Install\n\n```bash\niii worker add {name}\n```\n\n`iii worker add` fetches the binary, writes a config block into the engine's `config.yaml`, and the engine starts the worker on the next `iii worker start`."
    );
    if let Some(c) = &companions {
        install_section.push_str("\n\n");
        install_section.push_str(c);
    }

    let mut sections: Vec<String> = vec![
        frontmatter.to_string(),
        "<!-- generated by iii-skill-render. DO NOT EDIT (changes here are overwritten on the next render). Edit docs/intro.md, docs/quickstart.md, docs/companions.md, docs/migration.md, docs/leaves/*.md, iii.worker.yaml, or config.yaml. -->".to_string(),
        format!("# {name}"),
        intro,
        install_section,
        format!("## Quickstart\n\n{quickstart}"),
        format!("## Configuration\n\n```yaml\n{config}\n```"),
    ];
    if let Some(m) = migration {
        sections.push(format!("## Migration notes\n\n{m}"));
    }
    if let Some(s) = render_additional_howtos(leaves, Audience::Readme) {
        sections.push(s);
    }
    sections.join("\n\n") + "\n"
}

fn render_skill(
    frontmatter: &str,
    name: &str,
    intro: &str,
    companions: Option<&str>,
    leaves: &[(String, String)],
) -> String {
    let intro_unwrapped = unwrap_llm_only(&strip_human_only(intro));
    let mut sections = vec![
        frontmatter.to_string(),
        "<!-- generated by iii-skill-render. DO NOT EDIT (changes here are overwritten on the next render). Edit docs/intro.md, docs/companions.md, or docs/leaves/*.md. -->".to_string(),
        format!("# {name}"),
        intro_unwrapped,
    ];
    if let Some(c) = companions {
        sections.push(unwrap_llm_only(&strip_human_only(c)));
    }
    if let Some(s) = render_additional_howtos(leaves, Audience::Skill) {
        sections.push(s);
    }
    sections.join("\n\n") + "\n"
}

/// Inline every leaf body under a single `## Additional HOWTOs` H2. Each leaf
/// is run through the audience-appropriate visibility passes, then its ATX
/// headings are demoted by two levels so the leaf's own `# Title` nests as an
/// `### Title` beneath the section. Returns `None` when there are no leaves.
fn render_additional_howtos(leaves: &[(String, String)], audience: Audience) -> Option<String> {
    if leaves.is_empty() {
        return None;
    }
    let mut out = String::from("## Additional HOWTOs");
    for (_leaf, body) in leaves {
        let visible = match audience {
            // README is human-facing: drop llm-only, reveal human-only.
            Audience::Readme => unwrap_human_only(&strip_llm_only(body)),
            // skill.md is agent-facing: reveal llm-only, drop human-only.
            Audience::Skill => unwrap_llm_only(&strip_human_only(body)),
        };
        let demoted = demote_headings(visible.trim(), 2);
        out.push_str("\n\n");
        out.push_str(&demoted);
    }
    Some(out)
}

/// Demote every ATX heading in `body` by `by` levels (capped at H6). Lines
/// inside fenced code blocks are left untouched so a `# comment` in an
/// example is not mistaken for a heading. Headings must start at column 0.
fn demote_headings(body: &str, by: usize) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut in_fence = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            out.push(line.to_string());
            continue;
        }
        if !in_fence {
            let hashes = line.chars().take_while(|&c| c == '#').count();
            if (1..=6).contains(&hashes) {
                let rest = &line[hashes..];
                if rest.is_empty() || rest.starts_with(' ') {
                    let level = (hashes + by).min(6);
                    out.push(format!("{}{}", "#".repeat(level), rest));
                    continue;
                }
            }
        }
        out.push(line.to_string());
    }
    out.join("\n")
}

fn read_partial(path: &Path) -> anyhow::Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    Ok(content.trim_end_matches('\n').to_string())
}

fn read_optional_partial(path: &Path) -> anyhow::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Some(content.trim_end_matches('\n').to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(anyhow::Error::new(e).context(format!("reading {}", path.display()))),
    }
}

/// Re-render `dir` and report drift between source and on-disk artifacts.
/// Returns one message per out-of-date file; an empty Vec means the worker is
/// in sync. A leftover `skills/` directory (from the pre-inlining layout) is
/// reported as stale so `--write` removes it.
pub fn check_rendered(dir: &Path) -> anyhow::Result<Vec<String>> {
    let outputs = render_worker(dir)?;
    let mut drift: Vec<String> = Vec::new();

    let readme_disk = std::fs::read_to_string(dir.join("README.md")).unwrap_or_default();
    if readme_disk != outputs.readme {
        drift.push("README.md is out of date".into());
    }
    let skill_disk = std::fs::read_to_string(dir.join("skill.md")).unwrap_or_default();
    if skill_disk != outputs.skill {
        drift.push("skill.md is out of date".into());
    }
    if dir.join("skills").is_dir() {
        drift.push(
            "skills/ is stale (leaves are now inlined into skill.md; re-render to remove it)"
                .into(),
        );
    }
    drift.sort();
    Ok(drift)
}

fn list_leaves(dir: &Path) -> anyhow::Result<Vec<(String, PathBuf)>> {
    let leaves_dir = dir.join("docs").join("leaves");
    let mut leaves: Vec<(String, PathBuf)> = Vec::new();
    if !leaves_dir.exists() {
        return Ok(leaves);
    }
    for entry in std::fs::read_dir(&leaves_dir)
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
        leaves.push((name, path));
    }
    leaves.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(leaves)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwrap_handles_multiple_blocks() {
        let input = "<!-- llm-only:start -->\nA\n<!-- llm-only:end -->\nmid\n<!-- llm-only:start -->\nB\n<!-- llm-only:end -->";
        let expected = "A\nmid\nB";
        assert_eq!(unwrap_llm_only(input), expected);
    }

    #[test]
    fn demote_shifts_atx_headings_by_two() {
        let input = "# Title\n\n## When to use\n\n- bullet\n\n## Notes\n";
        let expected = "### Title\n\n#### When to use\n\n- bullet\n\n#### Notes";
        assert_eq!(demote_headings(input, 2), expected);
    }

    #[test]
    fn demote_caps_at_h6() {
        assert_eq!(demote_headings("##### Deep", 2), "###### Deep");
        assert_eq!(demote_headings("###### Deepest", 2), "###### Deepest");
    }

    #[test]
    fn demote_leaves_code_fence_comments_alone() {
        let input = "## Heading\n\n```bash\n# not a heading\niii worker add foo\n```\n";
        let expected = "#### Heading\n\n```bash\n# not a heading\niii worker add foo\n```";
        assert_eq!(demote_headings(input, 2), expected);
    }

    #[test]
    fn frontmatter_block_carries_all_three_fields() {
        let fm = frontmatter_block("textstats", Some("Text analysis worker."), Some("text, nlp"));
        assert!(fm.starts_with("---\n"));
        assert!(fm.ends_with("---"));
        assert!(fm.contains("name: textstats"));
        assert!(fm.contains("description: Text analysis worker."));
        assert!(fm.contains("tags: text, nlp"));
    }

    #[test]
    fn frontmatter_block_omits_absent_fields() {
        let fm = frontmatter_block("textstats", None, None);
        assert!(fm.contains("name: textstats"));
        assert!(!fm.contains("description:"), "absent description should be omitted: {fm}");
        assert!(!fm.contains("tags:"), "absent tags should be omitted: {fm}");

        let only_tags = frontmatter_block("textstats", None, Some("text"));
        assert!(!only_tags.contains("description:"));
        assert!(only_tags.contains("tags: text"));
    }
}
