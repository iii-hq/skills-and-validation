//! Drift check for docs mode: per-doc, compare the on-disk
//! `<source>.skill.md` against a freshly-rendered output.
//!
//! Returns one human-readable message per drifted or orphaned skill file
//! (mirrors `render::check_rendered` for worker mode). An empty Vec means
//! every in-scope doc has an up-to-date sibling skill artifact.

use crate::config::DocsConfig;
use crate::docs::enumerate::{enumerate, DiscoveredDoc};
use crate::docs::render::render_doc;
use std::collections::HashSet;
use std::path::Path;

pub fn check_rendered(root: &Path, config: &DocsConfig) -> anyhow::Result<Vec<String>> {
    let docs = enumerate(root, config)?;

    let mut drift: Vec<String> = Vec::new();
    let mut expected_skills: HashSet<std::path::PathBuf> = HashSet::new();
    for doc in &docs {
        let skill_path = doc.skill_path();
        expected_skills.insert(skill_path.clone());

        let rendered = match render_doc(&doc.abs) {
            Ok(r) => r,
            Err(e) => {
                // Surface render failures the same way drift is surfaced —
                // one line per source — so the verify pipeline picks them
                // up alongside skill drift instead of bailing.
                drift.push(format!("{}: render failed: {e}", doc.rel));
                continue;
            }
        };

        let on_disk = std::fs::read_to_string(&skill_path).unwrap_or_default();
        if on_disk != rendered.body {
            drift.push(format!(
                "{}.skill.md is out of date — re-run `iii-skill-render {}`",
                doc.rel, doc.rel
            ));
        }
    }

    // Orphan detection: any *.skill.md whose source is gone (or no longer
    // in scope per the config) shouldn't be on disk.
    drift.extend(orphan_skills(root, &docs, &expected_skills));

    Ok(drift)
}

fn orphan_skills(
    root: &Path,
    in_scope: &[DiscoveredDoc],
    expected: &HashSet<std::path::PathBuf>,
) -> Vec<String> {
    // Use the in-scope set's roots so we don't traverse the whole tree
    // again when the consumer's docs are deeply nested. We just need every
    // sibling .skill.md within the relevant subtrees.
    let mut messages: Vec<String> = Vec::new();
    let mut seen_dirs: HashSet<std::path::PathBuf> = HashSet::new();
    for doc in in_scope {
        if let Some(parent) = doc.abs.parent() {
            if !seen_dirs.insert(parent.to_path_buf()) {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.file_name().and_then(|s| s.to_str()).map_or(false, |n| n.ends_with(".skill.md")) {
                        continue;
                    }
                    if !expected.contains(&path) {
                        let rel = path
                            .strip_prefix(root)
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                            .unwrap_or_else(|_| path.to_string_lossy().to_string());
                        messages.push(format!("{rel} is orphaned — its source is missing or out of scope; delete the file"));
                    }
                }
            }
        }
    }
    messages.sort();
    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::render::render_doc;
    use tempfile::TempDir;

    fn fm(ty: &str) -> String {
        format!("---\ntitle: t\ndescription: d\ntype: {ty}\n---\n\n# t\n\nbody\n")
    }

    fn cfg(include: &[&str]) -> DocsConfig {
        DocsConfig {
            include: include.iter().map(|s| s.to_string()).collect(),
            exclude: vec![],
        }
    }

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn empty_when_skill_matches_source() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "guides/foo.mdx", &fm("how-to"));
        // Render once to produce the sibling.
        let rendered = render_doc(&tmp.path().join("guides/foo.mdx")).unwrap();
        std::fs::write(tmp.path().join("guides/foo.mdx.skill.md"), &rendered.body).unwrap();

        let drift = check_rendered(tmp.path(), &cfg(&["**/*.mdx"])).unwrap();
        assert!(drift.is_empty(), "no drift expected, got: {drift:?}");
    }

    #[test]
    fn flags_missing_skill_file() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "guides/foo.mdx", &fm("how-to"));
        // Don't render — skill file missing.
        let drift = check_rendered(tmp.path(), &cfg(&["**/*.mdx"])).unwrap();
        assert_eq!(drift.len(), 1);
        assert!(drift[0].contains("guides/foo.mdx.skill.md is out of date"));
    }

    #[test]
    fn flags_stale_skill_file() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "guides/foo.mdx", &fm("how-to"));
        std::fs::write(tmp.path().join("guides/foo.mdx.skill.md"), "stale\n").unwrap();
        let drift = check_rendered(tmp.path(), &cfg(&["**/*.mdx"])).unwrap();
        assert_eq!(drift.len(), 1);
        assert!(drift[0].contains("foo.mdx.skill.md is out of date"));
    }

    #[test]
    fn flags_orphan_skill_file() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "guides/foo.mdx", &fm("how-to"));
        let rendered = render_doc(&tmp.path().join("guides/foo.mdx")).unwrap();
        std::fs::write(tmp.path().join("guides/foo.mdx.skill.md"), &rendered.body).unwrap();
        // Orphan: skill file with no source.
        std::fs::write(tmp.path().join("guides/dead.mdx.skill.md"), "anything\n").unwrap();

        let drift = check_rendered(tmp.path(), &cfg(&["**/*.mdx"])).unwrap();
        assert_eq!(drift.len(), 1);
        assert!(drift[0].contains("guides/dead.mdx.skill.md is orphaned"));
    }
}
