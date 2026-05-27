use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

#[test]
fn read_manifest_parses_example_worker() {
    let example = repo_root().join("templates/example-worker");
    let manifest = iii_skill_core::introspect::read_manifest(&example)
        .expect("read_manifest should succeed for example-worker");

    assert_eq!(manifest.name, "textstats");
    assert!(
        manifest.description.starts_with("Text analysis on the iii bus"),
        "unexpected description: {}",
        manifest.description
    );
    assert!(
        !manifest.tags.trim().is_empty(),
        "example-worker should carry tags"
    );
}

#[test]
fn read_manifest_errors_on_missing_description_or_tags() {
    let tmp = tempfile::tempdir().unwrap();
    // name + tags present, description missing.
    std::fs::write(
        tmp.path().join("iii.worker.yaml"),
        "iii: v1\nname: foo\ntags: \"a, b\"\n",
    )
    .unwrap();
    assert!(
        iii_skill_core::introspect::read_manifest(tmp.path()).is_err(),
        "expected an error when description is missing"
    );

    // name + description present, tags missing.
    std::fs::write(
        tmp.path().join("iii.worker.yaml"),
        "iii: v1\nname: foo\ndescription: A worker.\n",
    )
    .unwrap();
    assert!(
        iii_skill_core::introspect::read_manifest(tmp.path()).is_err(),
        "expected an error when tags is missing"
    );

    // tags present but empty.
    std::fs::write(
        tmp.path().join("iii.worker.yaml"),
        "iii: v1\nname: foo\ndescription: A worker.\ntags: \"  \"\n",
    )
    .unwrap();
    assert!(
        iii_skill_core::introspect::read_manifest(tmp.path()).is_err(),
        "expected an error when tags is empty"
    );
}

#[test]
fn read_manifest_errors_when_file_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let result = iii_skill_core::introspect::read_manifest(tmp.path());
    assert!(
        result.is_err(),
        "expected an error when iii.worker.yaml is absent"
    );
}

#[test]
fn read_manifest_errors_on_invalid_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("iii.worker.yaml"), "not: [valid: yaml").unwrap();
    let result = iii_skill_core::introspect::read_manifest(tmp.path());
    assert!(result.is_err(), "expected a parse error on malformed YAML");
}
