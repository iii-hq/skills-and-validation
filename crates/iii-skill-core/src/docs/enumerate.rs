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

        let rel = relative_path(root, path);
        if !decide(&rel, path, &includes, &excludes) {
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

/// Decide whether a single source path should be in the skill set,
/// applying the same rules as [`enumerate`]. Use this from per-file
/// invocation paths (binary's `iii-skill-render <doc>` and
/// `iii-skill-check verify <doc>`) so a permissive action-level glob
/// can't cause the binary to attempt validation on out-of-scope files.
///
/// `path` should exist; the function reads it briefly to honour
/// `<!-- skill:include-doc -->` / `<!-- skill:exclude-doc -->` overrides.
pub fn is_in_scope(path: &Path, root: &Path, config: &DocsConfig) -> anyhow::Result<bool> {
    if !path.is_file() {
        return Ok(false);
    }
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if ext != "md" && ext != "mdx" {
        return Ok(false);
    }
    if path.file_name().and_then(|s| s.to_str()).map_or(false, |n| n.ends_with(".skill.md")) {
        return Ok(false);
    }
    let includes = compile_patterns(&config.include).context("compiling docs.include")?;
    let excludes = compile_patterns(&config.exclude).context("compiling docs.exclude")?;
    let rel = relative_path(root, path);
    Ok(decide(&rel, path, &includes, &excludes))
}

fn decide(rel: &str, abs: &Path, includes: &[Pattern], excludes: &[Pattern]) -> bool {
    // Path-based excludes are always hard — they describe trees the
    // controlling config considers off-limits (vendored fixtures,
    // unrelated subprojects, etc.). Even a per-file
    // `<!-- skill:include-doc -->` doesn't pull a path-excluded file back
    // in; the marker is a "include even if the include list missed me"
    // override, not an exclude bypass.
    if matches_any(excludes, rel) {
        return false;
    }
    let scope = peek_doc_scope(abs).unwrap_or(None);
    match scope {
        Some(DocScope::Include) => true,
        Some(DocScope::Exclude) => false,
        None => matches_any(includes, rel),
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
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
    fn path_exclude_beats_include_doc_marker() {
        // Path-based excludes are a hard signal — even
        // `<!-- skill:include-doc -->` doesn't pull a path-excluded file
        // back in. The marker only overrides the include list.
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vendored/CHANGELOG.md",
            &format!("{}\n\n<!-- skill:include-doc -->\n", fm()),
        );
        let cfg = cfg(&["**/*.md"], &["vendored/**"]);
        let docs = enumerate(tmp.path(), &cfg).unwrap();
        assert!(docs.is_empty(), "path exclude should win, got: {docs:?}");
    }

    #[test]
    fn include_doc_marker_overrides_glob_miss() {
        // The include-doc marker still pulls in files the include list
        // missed, as long as no path exclude blocks them.
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "guides/orphan.notes",
            &format!("{}\n\n<!-- skill:include-doc -->\n", fm()),
        );
        // Even with no include match (filename has wrong extension), the
        // marker does NOT pull this in — extension filter still applies.
        let cfg = cfg(&["**/*.md"], &[]);
        let docs = enumerate(tmp.path(), &cfg).unwrap();
        assert!(docs.is_empty());

        // But the .md case works.
        write(
            tmp.path(),
            "guides/orphan.md",
            &format!("{}\n\n<!-- skill:include-doc -->\n", fm()),
        );
        let docs = enumerate(tmp.path(), &cfg).unwrap();
        let rels: Vec<_> = docs.iter().map(|d| d.rel.as_str()).collect();
        assert_eq!(rels, vec!["guides/orphan.md"]);
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

    #[test]
    fn is_in_scope_filters_per_file() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "guides/foo.mdx", fm());
        write(tmp.path(), "queues/README.md", "rendered worker artifact, no frontmatter\n");

        let cfg = cfg(&["guides/**/*.mdx"], &[]);
        // In-scope: matches include glob.
        assert!(is_in_scope(&tmp.path().join("guides/foo.mdx"), tmp.path(), &cfg).unwrap());
        // Out of scope: outside the include glob — even though it's a .md
        // file under the root, it's not what the consumer asked us to check.
        assert!(!is_in_scope(&tmp.path().join("queues/README.md"), tmp.path(), &cfg).unwrap());
    }

    #[test]
    fn is_in_scope_path_exclude_beats_marker() {
        // Mirrors path_exclude_beats_include_doc_marker but via the per-
        // file is_in_scope helper.
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vendored/CHANGELOG.md",
            &format!("{}\n<!-- skill:include-doc -->\n", fm()),
        );
        let cfg = cfg(&["**/*.md"], &["vendored/**"]);
        assert!(!is_in_scope(&tmp.path().join("vendored/CHANGELOG.md"), tmp.path(), &cfg).unwrap());
    }

    #[test]
    fn is_in_scope_skips_skill_artifacts() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "foo.mdx.skill.md", "rendered\n");
        let cfg = cfg(&["**/*.md"], &[]);
        assert!(!is_in_scope(&tmp.path().join("foo.mdx.skill.md"), tmp.path(), &cfg).unwrap());
    }
}
