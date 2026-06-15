//! CLI-level coverage for path handling: the binary must resolve a
//! relative target from any working directory (walk up to the controlling
//! `.skill-check.yaml`) and write a repo-root-relative header into the
//! rendered artifact — never an absolute, machine-specific path.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Build a minimal docs-mode repo in a temp dir:
///   <root>/.git/                      (marks the repo root)
///   <root>/.skill-check.yaml          (docs mode, include content/**)
///   <root>/content/skills/foo.md      (one in-scope source)
fn scaffold(root: &Path) {
    fs::create_dir(root.join(".git")).unwrap();
    fs::write(
        root.join(".skill-check.yaml"),
        "version: 2\nmode: docs\ndocs:\n  include:\n    - \"content/**/*.md\"\n\
         ai_check:\n  provider: anthropic\n  model: claude-sonnet-4-6\n  \
         api_key_env_var: ANTHROPIC_API_KEY\n  max_tokens: 6000\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("content/skills")).unwrap();
    fs::write(
        root.join("content/skills/foo.md"),
        "---\ntitle: Foo\ndescription: d\ntype: how-to\n---\n\n# Foo\n\nBody.\n",
    )
    .unwrap();
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_iii-skill-render")
}

#[test]
fn renders_relative_target_from_a_subdirectory() {
    // The walk-up used to fail for relative targets: `Path::parent()`
    // bottoms out at "" before reaching the repo root, so the
    // `.skill-check.yaml` above the CWD was never found.
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    scaffold(root);

    // Run from inside content/skills with a bare relative filename.
    let out = Command::new(bin())
        .current_dir(root.join("content/skills"))
        .arg("foo.md")
        .arg("--allow-old-version")
        .env("SKV_NO_UPDATE_CHECK", "1")
        .output()
        .expect("binary runs");

    assert!(
        out.status.success(),
        "expected success, stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("content/skills/foo.md"),
        "expected repo-root-relative path in output, got:\n{stdout}"
    );
}

#[test]
fn header_is_repo_root_relative_for_absolute_target() {
    // An absolute target must still yield a repo-root-relative header in
    // the written artifact — no absolute path leaks into the committed
    // `.skill.md`.
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    scaffold(root);

    let abs = root.join("content/skills/foo.md");
    let out = Command::new(bin())
        .arg(&abs)
        .arg("--write")
        .arg("--allow-old-version")
        .env("SKV_NO_UPDATE_CHECK", "1")
        .output()
        .expect("binary runs");
    assert!(
        out.status.success(),
        "expected success, stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let rendered = fs::read_to_string(root.join("content/skills/foo.md.skill.md")).unwrap();
    assert!(
        rendered.contains("Edit content/skills/foo.md."),
        "header should be repo-root-relative, got first line:\n{}",
        rendered.lines().next().unwrap_or("")
    );
    assert!(
        !rendered.contains(root.to_string_lossy().as_ref()),
        "header must not leak the absolute repo path:\n{}",
        rendered.lines().next().unwrap_or("")
    );
}
