use std::path::Path;
use tempfile::TempDir;

fn write_minimal_worker(dir: &Path) {
    std::fs::write(
        dir.join("iii.worker.yaml"),
        "iii: v1\nname: t\nlanguage: rust\ndeploy: binary\nmanifest: Cargo.toml\nbin: t\ndescription: A small fixture worker for tests.\n",
    )
    .unwrap();
    std::fs::write(dir.join("config.yaml"), "# Fixture.\nkey: value\n").unwrap();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs").join("intro.md"), "An intro.\n").unwrap();
    std::fs::write(
        dir.join("docs").join("quickstart.md"),
        "```rust\nfn main() {}\n```\n",
    )
    .unwrap();
}

fn render_to_disk(dir: &Path) {
    let out = iii_skill_core::render::render_worker(dir).unwrap();
    std::fs::write(dir.join("README.md"), &out.readme).unwrap();
    std::fs::write(dir.join("skill.md"), &out.skill).unwrap();
    std::fs::create_dir_all(dir.join("skills")).unwrap();
    for (leaf, body) in &out.leaves {
        std::fs::write(dir.join("skills").join(format!("{leaf}.md")), body).unwrap();
    }
}

#[test]
fn check_rendered_returns_empty_when_in_sync() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path());
    render_to_disk(tmp.path());
    let drift = iii_skill_core::render::check_rendered(tmp.path()).unwrap();
    assert!(drift.is_empty(), "expected no drift, got: {drift:?}");
}

#[test]
fn check_rendered_flags_readme_out_of_date() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path());
    render_to_disk(tmp.path());
    std::fs::write(tmp.path().join("README.md"), "# tampered\n").unwrap();
    let drift = iii_skill_core::render::check_rendered(tmp.path()).unwrap();
    assert!(
        drift.iter().any(|d| d.contains("README.md is out of date")),
        "expected README drift, got: {drift:?}"
    );
}

#[test]
fn check_rendered_flags_orphan_leaf() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path());
    render_to_disk(tmp.path());
    // Create a skills/<name>.md without a corresponding docs/leaves/<name>.md.
    std::fs::write(tmp.path().join("skills").join("orphan.md"), "stale\n").unwrap();

    let drift = iii_skill_core::render::check_rendered(tmp.path()).unwrap();
    assert!(
        drift
            .iter()
            .any(|d| d.contains("orphan.md") && d.contains("orphaned")),
        "expected orphan flag for skills/orphan.md, got: {drift:?}"
    );
}

#[test]
fn list_rendered_leaves_returns_md_stems_sorted_ignoring_other_files() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join("skills")).unwrap();
    std::fs::write(tmp.path().join("skills").join("foo.md"), "").unwrap();
    std::fs::write(tmp.path().join("skills").join("bar.md"), "").unwrap();
    std::fs::write(tmp.path().join("skills").join("baz.txt"), "").unwrap();

    let names = iii_skill_core::render::list_rendered_leaves(tmp.path());
    assert_eq!(names, vec!["bar".to_string(), "foo".to_string()]);
}

#[test]
fn list_rendered_leaves_returns_empty_when_skills_dir_absent() {
    let tmp = TempDir::new().unwrap();
    let names = iii_skill_core::render::list_rendered_leaves(tmp.path());
    assert!(names.is_empty(), "expected empty, got: {names:?}");
}
