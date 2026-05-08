use std::path::PathBuf;

/// Walk up from the running binary's directory looking for a sibling
/// `content/` dir whose layout matches the release bundle: both
/// `project-rules/` and `.vale.ini` must be present.
///
/// In a release tarball the binary lives at `<prefix>/bin/iii-skill-*` and
/// content at `<prefix>/content/`, so the first walk-up step finds it. In a
/// `cargo` workspace the test/dev binary is under `target/...`; the loop
/// walks up until it reaches the workspace root.
pub fn find_content_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut cur = exe.parent()?.to_path_buf();
    loop {
        let candidate = cur.join("content");
        if candidate.join("project-rules").is_dir() && candidate.join(".vale.ini").is_file() {
            return Some(candidate);
        }
        match cur.parent() {
            Some(p) => cur = p.to_path_buf(),
            None => return None,
        }
    }
}
