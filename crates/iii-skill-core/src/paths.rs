//! Repo-root discovery and repo-root-relative path display.
//!
//! Rendered artifacts embed their source path in the
//! `<!-- … DO NOT EDIT … Edit <path>. -->` header, and violation messages
//! cite source paths too. Those paths must be stable across machines and
//! working directories: an absolute path (`/Users/me/repo/docs/x.md`)
//! leaks the author's home dir into the committed `.skill.md` and makes
//! the same source re-render to different bytes in CI, which the drift
//! check then reports as spurious drift. Normalizing every embedded path
//! to "relative to the repository root, forward slashes" makes it
//! identical no matter how the renderer was invoked (relative target,
//! absolute target, or from a different CWD).

use std::path::{Path, PathBuf};

/// Resolve `path` to an absolute, symlink-free location. Prefers
/// [`std::fs::canonicalize`] (the file usually exists by the time we
/// display it); for not-yet-existing outputs it falls back to joining the
/// current dir, then to the path as given.
pub fn absolutize(path: &Path) -> PathBuf {
    if let Ok(c) = std::fs::canonicalize(path) {
        return c;
    }
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(path),
        Err(_) => path.to_path_buf(),
    }
}

/// Walk up from `start` to the repository root — the nearest ancestor
/// holding a `.git` entry. Falls back to the nearest ancestor with a
/// `.skill-check.yaml` (the project root this tool keys off) when the tree
/// isn't a git checkout, and to `None` when neither marker is found.
/// `start` is canonicalized first so relative inputs and symlinks resolve
/// to a single stable location.
pub fn repo_root(start: &Path) -> Option<PathBuf> {
    let canon = std::fs::canonicalize(start).ok()?;
    let mut cur: &Path = if canon.is_file() {
        canon.parent()?
    } else {
        &canon
    };
    let mut fallback: Option<PathBuf> = None;
    loop {
        if cur.join(".git").exists() {
            return Some(cur.to_path_buf());
        }
        if fallback.is_none() && cur.join(".skill-check.yaml").is_file() {
            fallback = Some(cur.to_path_buf());
        }
        cur = match cur.parent() {
            Some(p) => p,
            None => return fallback,
        };
    }
}

/// Render `path` for embedding in a header or violation message,
/// normalized to forward-slash, repo-root-relative form. Falls back to the
/// path as given (lossy, slashes normalized) when the repo root can't be
/// found or the path doesn't live under it (e.g. synthetic test paths).
pub fn display_relative(path: &Path) -> String {
    if let (Some(root), Ok(canon)) = (repo_root(path), std::fs::canonicalize(path)) {
        if let Ok(rel) = canon.strip_prefix(&root) {
            return rel.to_string_lossy().replace('\\', "/");
        }
    }
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn repo_root_finds_git_dir() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("content/skills")).unwrap();
        let doc = root.join("content/skills/foo.md");
        fs::write(&doc, "x").unwrap();

        let found = repo_root(&doc).unwrap();
        assert_eq!(found, fs::canonicalize(root).unwrap());
    }

    #[test]
    fn display_relative_is_repo_root_relative() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("content/skills")).unwrap();
        let doc = root.join("content/skills/foo.md");
        fs::write(&doc, "x").unwrap();

        // Absolute input.
        assert_eq!(display_relative(&doc), "content/skills/foo.md");
    }

    #[test]
    fn display_relative_falls_back_when_no_repo() {
        // Synthetic path that doesn't exist and has no repo root: returned
        // as-given so tests and odd layouts still get a readable string.
        let p = Path::new("/nonexistent/x/foo.mdx");
        assert_eq!(display_relative(p), "/nonexistent/x/foo.mdx");
    }

    #[test]
    fn repo_root_falls_back_to_skill_check_yaml() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        // No .git anywhere, but a .skill-check.yaml marks the project root.
        fs::write(root.join(".skill-check.yaml"), "version: 2\n").unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        let doc = root.join("docs/foo.md");
        fs::write(&doc, "x").unwrap();

        let found = repo_root(&doc).unwrap();
        assert_eq!(found, fs::canonicalize(root).unwrap());
    }
}
