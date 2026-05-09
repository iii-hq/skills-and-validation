use std::path::PathBuf;

/// Locate the bundled `content/` dir whose layout matches the release bundle:
/// both `project-rules/` and `.vale.ini` must be present.
///
/// Resolution order:
///   1. Walk up from the running binary's directory. In a release tarball the
///      binary lives at `<prefix>/bin/iii-skill-*` and content at
///      `<prefix>/content/`, so the first walk-up step finds it. In a `cargo`
///      workspace the test/dev binary is under `target/...`; the loop walks up
///      until it reaches the workspace root.
///   2. Fall back to `$SKV_DIR/current/content/` (default
///      `$HOME/.local/share/skill-check/current/content/`) — the standard
///      install layout — so `iii-skill-check` works when invoked from a binary
///      that doesn't ship next to its content (e.g. `cargo install` builds, or
///      a symlink resolved by `current_exe()` on platforms that don't follow
///      symlinks).
pub fn find_content_root() -> Option<PathBuf> {
    if let Some(exe) = std::env::current_exe().ok() {
        if let Some(parent) = exe.parent() {
            let mut cur = parent.to_path_buf();
            loop {
                let candidate = cur.join("content");
                if is_bundle_root(&candidate) {
                    return Some(candidate);
                }
                match cur.parent() {
                    Some(p) => cur = p.to_path_buf(),
                    None => break,
                }
            }
        }
    }

    let skv_dir = std::env::var_os("SKV_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/skill-check")))?;
    let candidate = skv_dir.join("current").join("content");
    if is_bundle_root(&candidate) {
        return Some(candidate);
    }
    None
}

fn is_bundle_root(p: &std::path::Path) -> bool {
    p.join("project-rules").is_dir() && p.join(".vale.ini").is_file()
}
