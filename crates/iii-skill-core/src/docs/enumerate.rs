//! Walk a docs root and decide which files belong to the skill set.
//!
//! Decision pipeline per file:
//!   1. Path matches at least one entry in `docs.include`.
//!   2. Path matches NO entry in `docs.exclude`.
//!   3. Read the first chunk of the file: a top-of-file
//!      `<!-- skill:include-doc -->` keeps a glob-rejected file IN; a
//!      `<!-- skill:exclude-doc -->` drops a glob-accepted file OUT.
//!
//! The `<source>.skill.md` artifact lives sibling to the source, so this
//! module also filters out any `.skill.md` files encountered during the
//! walk — they're outputs, not sources.

use crate::config::DocsConfig;
use crate::docs::markers::{scan_doc_scope, DocScope};
use anyhow::Context;
use glob::Pattern;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// One in-scope source doc, with its path resolved relative to the docs
/// root (for portable display in violation messages) and absolute (for
/// reading the file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDoc {
    /// Absolute path to the source file (`.md` or `.mdx`).
    pub abs: PathBuf,
    /// Path relative to the docs root, with forward slashes — the form
    /// users see in violation reports and PR comments.
    pub rel: String,
}

impl DiscoveredDoc {
    /// Path of the rendered skill artifact on disk.
    pub fn skill_path(&self) -> PathBuf {
        let mut s = self.abs.as_os_str().to_owned();
        s.push(".skill.md");
        PathBuf::from(s)
    }
}

/// Walk `root` and return the docs in scope per the include/exclude globs
/// + per-doc opt-in/out comments. The returned list is sorted by `rel`
/// for stable output across runs.
pub fn enumerate(root: &Path, config: &DocsConfig) -> anyhow::Result<Vec<DiscoveredDoc>> {
    let includes = compile_patterns(&config.include).context("compiling docs.include")?;
    let excludes = compile_patterns(&config.exclude).context("compiling docs.exclude")?;

    let mut found: Vec<DiscoveredDoc> = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.context("walking docs root")?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "md" && ext != "mdx" {
            continue;
        }
        // Skip rendered skill artifacts: `<source>.skill.md`.
        if path.file_name().and_then(|s| s.to_str()).map_or(false, |n| n.ends_with(".skill.md")) {
            continue;
        }

        let rel = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().to_string());

        let glob_match = matches_any(&includes, &rel);
        let excluded = matches_any(&excludes, &rel);

        // Per-doc opt-in/out — read just enough of the file to find any
        // top-of-file scope marker. Scanning the whole body would be
        // wasteful here; markers conventionally sit in the first ~20 lines.
        let scope = peek_doc_scope(path).unwrap_or(None);

        let in_scope = match scope {
            Some(DocScope::Include) => true,
            Some(DocScope::Exclude) => false,
            None => glob_match && !excluded,
        };
        if !in_scope {
            continue;
        }

        found.push(DiscoveredDoc {
            abs: path.to_path_buf(),
            rel,
        });
    }
    found.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(found)
}

fn compile_patterns(patterns: &[String]) -> anyhow::Result<Vec<Pattern>> {
    patterns
        .iter()
        .map(|p| Pattern::new(p).map_err(|e| anyhow::anyhow!("invalid glob `{p}`: {e}")))
        .collect()
}

fn matches_any(patterns: &[Pattern], rel: &str) -> bool {
    patterns.iter().any(|p| p.matches(rel))
}

fn peek_doc_scope(path: &Path) -> std::io::Result<Option<DocScope>> {
    // Reading the whole file is fine — even Mintlify docs are typically
    // a few KB. If this becomes a hot path we can switch to a bounded read.
    let body = std::fs::read_to_string(path)?;
    Ok(scan_doc_scope(&body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    fn cfg(include: &[&str], exclude: &[&str]) -> DocsConfig {
        DocsConfig {
            include: include.iter().map(|s| s.to_string()).collect(),
            exclude: exclude.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn fm() -> &'static str {
        "---\ntitle: t\ndescription: d\ntype: how-to\n---\n\nbody\n"
    }

    #[test]
    fn includes_matching_md_and_mdx() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "guides/foo.mdx", fm());
        write(tmp.path(), "guides/bar.md", fm());
        write(tmp.path(), "skipped.txt", fm());

        let cfg = cfg(&["**/*.mdx", "**/*.md"], &[]);
        let docs = enumerate(tmp.path(), &cfg).unwrap();
        let rels: Vec<_> = docs.iter().map(|d| d.rel.as_str()).collect();
        assert_eq!(rels, vec!["guides/bar.md", "guides/foo.mdx"]);
    }

    #[test]
    fn exclude_drops_matched_files() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "guides/foo.mdx", fm());
        write(tmp.path(), "guides/CHANGELOG.md", fm());

        let cfg = cfg(&["**/*.md", "**/*.mdx"], &["**/CHANGELOG.md"]);
        let docs = enumerate(tmp.path(), &cfg).unwrap();
        let rels: Vec<_> = docs.iter().map(|d| d.rel.as_str()).collect();
        assert_eq!(rels, vec!["guides/foo.mdx"]);
    }

    #[test]
    fn doc_marker_overrides_glob_exclude() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "guides/CHANGELOG.md",
            &format!("{}\n\n<!-- skill:include-doc -->\n", fm()),
        );

        let cfg = cfg(&["**/*.md"], &["**/CHANGELOG.md"]);
        let docs = enumerate(tmp.path(), &cfg).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].rel, "guides/CHANGELOG.md");
    }

    #[test]
    fn doc_marker_overrides_glob_include() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "guides/draft.mdx",
            &format!("{}\n\n<!-- skill:exclude-doc -->\n", fm()),
        );

        let cfg = cfg(&["**/*.mdx"], &[]);
        let docs = enumerate(tmp.path(), &cfg).unwrap();
        assert!(docs.is_empty(), "exclude-doc marker should drop the file");
    }

    #[test]
    fn skill_md_outputs_are_ignored() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "foo.mdx", fm());
        write(tmp.path(), "foo.mdx.skill.md", "rendered output\n");

        let cfg = cfg(&["**/*.mdx", "**/*.md"], &[]);
        let docs = enumerate(tmp.path(), &cfg).unwrap();
        let rels: Vec<_> = docs.iter().map(|d| d.rel.as_str()).collect();
        assert_eq!(rels, vec!["foo.mdx"]);
    }

    #[test]
    fn skill_path_appends_suffix() {
        let d = DiscoveredDoc {
            abs: PathBuf::from("/x/foo.mdx"),
            rel: "foo.mdx".to_string(),
        };
        assert_eq!(d.skill_path(), PathBuf::from("/x/foo.mdx.skill.md"));
    }
}
