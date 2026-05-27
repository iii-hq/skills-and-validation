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
        manifest
            .description()
            .is_some_and(|d| d.starts_with("Text analysis on the iii bus")),
        "unexpected description: {:?}",
        manifest.description()
    );
    assert!(
        manifest.tags().is_some(),
        "example-worker should carry tags"
    );
}

#[test]
fn read_manifest_succeeds_without_description_or_tags() {
    let tmp = tempfile::tempdir().unwrap();
    // Only name present — description and tags are optional.
    std::fs::write(
        tmp.path().join("iii.worker.yaml"),
        "iii: v1\nname: foo\n",
    )
    .unwrap();
    let m = iii_skill_core::introspect::read_manifest(tmp.path())
        .expect("read_manifest should succeed with only name");
    assert_eq!(m.name, "foo");
    assert!(m.description().is_none(), "absent description should be None");
    assert!(m.tags().is_none(), "absent tags should be None");

    // Blank values are treated as absent.
    std::fs::write(
        tmp.path().join("iii.worker.yaml"),
        "iii: v1\nname: foo\ndescription: \"  \"\ntags: \"\"\n",
    )
    .unwrap();
    let m = iii_skill_core::introspect::read_manifest(tmp.path()).unwrap();
    assert!(m.description().is_none(), "blank description should be None");
    assert!(m.tags().is_none(), "blank tags should be None");
}

#[test]
fn read_manifest_errors_on_missing_name() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("iii.worker.yaml"),
        "iii: v1\nname: \"\"\ndescription: A worker.\n",
    )
    .unwrap();
    assert!(
        iii_skill_core::introspect::read_manifest(tmp.path()).is_err(),
        "expected an error when name is empty"
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
