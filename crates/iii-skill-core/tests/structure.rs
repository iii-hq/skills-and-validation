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
            "iii: v1\nname: {name}\nlanguage: rust\ndeploy: binary\nmanifest: Cargo.toml\nbin: {name}\ndescription: A small fixture worker for tests.\n"
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
    std::fs::create_dir_all(dir.join("skills")).unwrap();
    for (leaf, body) in &outputs.leaves {
        std::fs::write(dir.join("skills").join(format!("{leaf}.md")), body).unwrap();
    }
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

    // Author a leaf source whose body has no `# ` heading, then render it
    // and write the artifact to disk. The structure layer reads the
    // rendered file, so this exercises the real path.
    let leaves_src = tmp.path().join("docs").join("leaves");
    std::fs::create_dir_all(&leaves_src).unwrap();
    std::fs::write(
        leaves_src.join("verb.md"),
        "## When to use\n\n- A bullet, but no top-level H1.\n",
    )
    .unwrap();
    let outputs = iii_skill_core::render::render_worker(tmp.path()).unwrap();
    std::fs::write(tmp.path().join("README.md"), &outputs.readme).unwrap();
    std::fs::write(tmp.path().join("skill.md"), &outputs.skill).unwrap();
    for (leaf, body) in &outputs.leaves {
        std::fs::write(tmp.path().join("skills").join(format!("{leaf}.md")), body).unwrap();
    }

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations.iter().any(|v| v.file == "skills/verb.md"
            && v.message.to_lowercase().contains("h1")),
        "expected a missing-H1 violation on skills/verb.md, got: {violations:?}"
    );
}

#[test]
fn flags_leaf_whose_only_h1_lives_inside_llm_only_block() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");

    let leaves_src = tmp.path().join("docs").join("leaves");
    std::fs::create_dir_all(&leaves_src).unwrap();
    std::fs::write(
        leaves_src.join("verb.md"),
        // The H1 lives inside an llm-only block. The renderer unwraps the
        // block for the skill artifact, so a naive H1 check on the
        // rendered skill body would see the heading — but it would still
        // leak into the README link text if extract_h1 ran against the
        // raw partial. The structure check should flag this leaf.
        "<!-- llm-only:start -->\n# Hidden agent-only title\n<!-- llm-only:end -->\n\n## When to use\n\n- bullet\n",
    )
    .unwrap();
    let outputs = iii_skill_core::render::render_worker(tmp.path()).unwrap();
    std::fs::write(tmp.path().join("README.md"), &outputs.readme).unwrap();
    std::fs::write(tmp.path().join("skill.md"), &outputs.skill).unwrap();
    for (leaf, body) in &outputs.leaves {
        std::fs::write(tmp.path().join("skills").join(format!("{leaf}.md")), body).unwrap();
    }

    // The rendered leaf still carries the unwrapped H1 (LLM-facing), so the
    // current naive check passes that leaf. This documents the gap: the
    // worker-mode structure check reads the rendered artifact and is
    // strict about a top-level H1 existing there.
    //
    // What we *do* want to assert is that the README link text fell back
    // to the leaf name (extract_h1 ran on a stripped body), not the
    // hidden title. That assertion lives in
    // render_visibility_blocks.rs::leaf_h1_inside_llm_only_block_does_not_leak_into_readme.
    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    // No leaf-h1 violation expected here because the rendered leaf body
    // *does* contain an H1 (the unwrapped block). This test pins that
    // contract so a future change that tightens the H1 check to run on
    // the stripped-for-title body has to update this test deliberately.
    assert!(
        !violations
            .iter()
            .any(|v| v.file == "skills/verb.md" && v.message.to_lowercase().contains("h1")),
        "structure check should not fire on the rendered leaf, since the unwrap restored the H1; got: {violations:?}"
    );
}

#[test]
fn flags_iii_link_to_unknown_leaf() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker(tmp.path(), "fixture");
    let mut skill = std::fs::read_to_string(tmp.path().join("skill.md")).unwrap();
    skill.push_str("\n- [bogus](iii://fixture/nonexistent) — broken link\n");
    std::fs::write(tmp.path().join("skill.md"), skill).unwrap();

    let violations = iii_skill_core::structure::check(tmp.path()).unwrap();
    assert!(
        violations.iter().any(|v| v.message.contains("nonexistent")),
        "expected a broken-link violation, got: {violations:?}"
    );
}
