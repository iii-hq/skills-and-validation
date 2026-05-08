use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

#[test]
fn find_content_root_locates_repo_content_dir() {
    // Test binaries live under target/debug/deps/. The walk-up should find
    // <repo>/content/ since it has both project-rules/ and .vale.ini.
    let found = iii_skill_core::bundle::find_content_root()
        .expect("expected to find a content/ dir");
    let expected = repo_root().join("content");
    let canon_found = found.canonicalize().unwrap();
    let canon_expected = expected.canonicalize().unwrap();
    assert_eq!(
        canon_found, canon_expected,
        "expected to find {canon_expected:?}, got {canon_found:?}"
    );
}
