use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

fn broken_dir() -> PathBuf {
    repo_root().join("fixtures/broken-worker")
}

fn vale_config() -> PathBuf {
    repo_root().join("content/.vale.ini")
}

/// The broken fixture's rendered files must stay in sync with their sources.
/// Without this, a stale render could mask the layer violations the other
/// tests rely on.
#[test]
fn render_broken_matches_golden() {
    let dir = broken_dir();
    let outputs = iii_skill_core::render::render_worker(&dir)
        .expect("render_worker should succeed against the broken-worker fixture");

    let expected_readme = std::fs::read_to_string(dir.join("README.md")).unwrap();
    similar_asserts::assert_eq!(expected_readme, outputs.readme);

    let expected_skill = std::fs::read_to_string(dir.join("skill.md")).unwrap();
    similar_asserts::assert_eq!(expected_skill, outputs.skill);
}

/// The broken fixture deliberately seeds three forbidden patterns
/// (cargo build, <bin> --help, <bin> --manifest | jq) in quickstart and a
/// leaf with no top-level H1. Structure layer should catch all of them.
#[test]
fn structure_broken_fails_with_multiple_violations() {
    let v = iii_skill_core::structure::check(&broken_dir())
        .expect("structure check should not error");

    let messages: Vec<String> = v.iter().map(|x| x.message.to_lowercase()).collect();
    assert!(
        messages.iter().any(|m| m.contains("cargo build")),
        "expected `cargo build` flag, got: {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("--help")),
        "expected `--help` flag, got: {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("--manifest")),
        "expected `--manifest | jq` flag, got: {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("h1")),
        "expected a missing-H1 flag for the leaf, got: {messages:?}"
    );
    assert!(
        v.len() >= 4,
        "expected at least 4 structure violations, got {}: {v:?}",
        v.len()
    );
}

/// The broken fixture's intro and leaf use forbidden marketing phrasing
/// ("blazing fast", "revolutionary"). Vale should flag them in the rendered
/// README + skill.md (the leaf content is inlined into both now).
#[test]
fn vale_broken_fails_with_multiple_violations() {
    let dir = broken_dir();
    let readme = dir.join("README.md");
    let skill = dir.join("skill.md");
    let artifacts: Vec<&Path> = vec![&readme, &skill];

    let v = iii_skill_core::vale::run(&artifacts, &vale_config())
        .expect("vale should run against the broken fixture");

    let lowered: Vec<String> = v.iter().map(|x| x.message.to_lowercase()).collect();
    assert!(
        lowered.iter().any(|m| m.contains("blazing fast")),
        "expected 'blazing fast' flag, got: {lowered:?}"
    );
    assert!(
        lowered.iter().any(|m| m.contains("revolutionary")),
        "expected 'revolutionary' flag, got: {lowered:?}"
    );
    assert!(
        v.len() >= 3,
        "expected at least 3 vale violations across the broken artifacts, got {}: {v:?}",
        v.len()
    );
}
