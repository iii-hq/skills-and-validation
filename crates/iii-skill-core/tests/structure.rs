mod common;

use common::RenderedTemplate;
use std::path::Path;
use tempfile::TempDir;

/// Build a minimal worker dir with a rendered README/skill, no leaves.
/// Tests then mutate one specific surface to trigger one specific check.
fn write_minimal_worker(dir: &Path, name: &str) {
    std::fs::write(
        dir.join("iii.worker.yaml"),
        format!(
            "iii: v1\nname: {name}\nlanguage: rust\ndeploy: binary\nmanifest: Cargo.toml\nbin: {name}\ndescription: A small fixture worker for tests.\ntags: \"test, fixture\"\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("config.yaml"), "# Fixture config.\nkey: value\n").unwrap();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(
        dir.join("docs").join("intro.md"),
        "A small fixture worker for tests.\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs").join("quickstart.md"),
        "```rust\nfn main() {}\n```\n",
    )
    .unwrap();

    let outputs = iii_skill_core::render::render_worker(dir).unwrap();
    std::fs::write(dir.join("README.md"), &outputs.readme).unwrap();
    std::fs::write(dir.join("skill.md"), &outputs.skill).unwrap();
}

#[test]
fn example_worker_passes_structure_check() {
    let rendered = RenderedTemplate::lock();
    let violations = iii_skill_core::structure::check(rendered.worker())
        .expect("structure check should not error");
    assert!(
        violations.is_empty(),
        "expected zero violations against example-worker, got: {violations:?}"
    );
}

#[test]
fn flags_install_mismatch() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");
    let readme = std::fs::read_to_string(tmp.path().join("README.md")).unwrap();
    let bad = readme.replace("iii worker add fixture", "iii worker add wrong-name");
    std::fs::write(tmp.path().join("README.md"), bad).unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.message.to_lowercase().contains("install")),
        "expected an install-mismatch violation, got: {violations:?}"
    );
}

#[test]
fn flags_missing_install_section() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");
    let readme = "<!-- generated -->\n\n# fixture\n\nIntro paragraph.\n\n## Quickstart\n\n```rust\nfn main() {}\n```\n\n## Configuration\n\n```yaml\nkey: value\n```\n";
    std::fs::write(tmp.path().join("README.md"), readme).unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations.iter().any(|v| v.message.contains("## Install")),
        "expected a missing-section violation, got: {violations:?}"
    );
}

#[test]
fn flags_unbalanced_llm_only() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");
    let mut readme = std::fs::read_to_string(tmp.path().join("README.md")).unwrap();
    readme.push_str("\n<!-- llm-only:start -->\norphan body without close\n");
    std::fs::write(tmp.path().join("README.md"), readme).unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.message.to_lowercase().contains("llm-only")),
        "expected an unbalanced-llm-only violation, got: {violations:?}"
    );
}

#[test]
fn flags_cargo_build_block() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");
    let mut readme = std::fs::read_to_string(tmp.path().join("README.md")).unwrap();
    readme.push_str("\n```bash\ncargo build --release\n```\n");
    std::fs::write(tmp.path().join("README.md"), readme).unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.message.to_lowercase().contains("cargo build")
                || v.message.to_lowercase().contains("source")),
        "expected a source-build violation, got: {violations:?}"
    );
}

#[test]
fn flags_manifest_jq_verification_step() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");
    let mut readme = std::fs::read_to_string(tmp.path().join("README.md")).unwrap();
    readme.push_str("\n```bash\nfixture --manifest | jq\n```\n");
    std::fs::write(tmp.path().join("README.md"), readme).unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations.iter().any(|v| v.message.contains("--manifest")),
        "expected --manifest | jq violation, got: {violations:?}"
    );
}

#[test]
fn flags_bin_help_verification_step() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");
    let mut readme = std::fs::read_to_string(tmp.path().join("README.md")).unwrap();
    readme.push_str("\n```bash\nfixture --help\n```\n");
    std::fs::write(tmp.path().join("README.md"), readme).unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations.iter().any(|v| v.message.contains("--help")),
        "expected --help violation, got: {violations:?}"
    );
}

#[test]
fn flags_bin_manifest_without_jq() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");
    let mut readme = std::fs::read_to_string(tmp.path().join("README.md")).unwrap();
    readme.push_str("\n```bash\nfixture --manifest\n```\n");
    std::fs::write(tmp.path().join("README.md"), readme).unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations.iter().any(|v| v.message.contains("--manifest")),
        "expected --manifest violation, got: {violations:?}"
    );
}

#[test]
fn does_not_flag_help_in_unrelated_context() {
    // The check is bin-name-scoped: `iii --help` in prose or in a different
    // command should not trigger a violation against the fixture worker.
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");
    let mut readme = std::fs::read_to_string(tmp.path().join("README.md")).unwrap();
    readme.push_str("\nRun `iii --help` to see CLI options.\n");
    std::fs::write(tmp.path().join("README.md"), readme).unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        !violations.iter().any(|v| v.message.contains("--help")),
        "should not flag `iii --help` (different bin name), got: {violations:?}"
    );
}

#[test]
fn flags_leaf_with_no_h1() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");

    // A source leaf with no top-level H1. The structure layer reads source
    // leaves directly (docs/leaves/*.md), since they're inlined into the
    // artifacts rather than rendered to a skills/ dir.
    let leaves_src = tmp.path().join("docs").join("leaves");
    std::fs::create_dir_all(&leaves_src).unwrap();
    std::fs::write(
        leaves_src.join("verb.md"),
        "## When to use\n\n- A bullet, but no top-level H1.\n",
    )
    .unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.file == "docs/leaves/verb.md" && v.message.to_lowercase().contains("h1")),
        "expected a missing-H1 violation on docs/leaves/verb.md, got: {violations:?}"
    );
}

#[test]
fn flags_leaf_whose_only_h1_lives_inside_llm_only_block() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");

    // The H1 lives only inside an llm-only block. check_leaf_h1 strips both
    // visibility-block types before scanning, so this counts as no H1 — the
    // README inlining would drop the title entirely, leaving the HOWTO
    // untitled. The structure check must flag it.
    let leaves_src = tmp.path().join("docs").join("leaves");
    std::fs::create_dir_all(&leaves_src).unwrap();
    std::fs::write(
        leaves_src.join("verb.md"),
        "<!-- llm-only:start -->\n# Hidden agent-only title\n<!-- llm-only:end -->\n\n## When to use\n\n- bullet\n",
    )
    .unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.file == "docs/leaves/verb.md" && v.message.to_lowercase().contains("h1")),
        "expected a missing-H1 violation (H1 only inside an llm-only block), got: {violations:?}"
    );
}

#[test]
fn flags_missing_frontmatter_in_skill() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");
    // Strip the frontmatter from the rendered skill.md.
    let skill = std::fs::read_to_string(tmp.path().join("skill.md")).unwrap();
    let body = skill.splitn(2, "\n---\n").nth(1).unwrap_or(&skill);
    std::fs::write(tmp.path().join("skill.md"), format!("# fixture\n\n{body}")).unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.file == "skill.md" && v.message.to_lowercase().contains("frontmatter")),
        "expected a missing-frontmatter violation on skill.md, got: {violations:?}"
    );
}
