use std::path::Path;
use tempfile::TempDir;

/// Build a minimal worker dir with a custom intro body. The intro is the
/// partial most authors actually reach for when stashing agent-only notes,
/// so it's the right partial to regression-test against.
fn write_worker_with_intro(dir: &Path, name: &str, intro_body: &str) {
    std::fs::write(
        dir.join("iii.worker.yaml"),
        format!(
            "iii: v1\nname: {name}\nlanguage: rust\ndeploy: binary\nmanifest: Cargo.toml\nbin: {name}\ndescription: A small fixture worker for tests.\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("config.yaml"), "# Fixture config.\nkey: value\n").unwrap();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs").join("intro.md"), intro_body).unwrap();
    std::fs::write(
        dir.join("docs").join("quickstart.md"),
        "```rust\nfn main() {}\n```\n",
    )
    .unwrap();
}

/// Regression: an `llm-only` block in `docs/intro.md` must drop entirely
/// from the rendered README. The marker lines are HTML comments and are
/// invisible to a markdown renderer, but the prose between them is plain
/// markdown that would otherwise render to humans on iii.dev / GitHub.
#[test]
fn intro_llm_only_block_dropped_in_readme() {
    let tmp = TempDir::new().unwrap();
    let intro = "Visible intro paragraph.\n\n<!-- llm-only:start -->\nAgent-only routing hint that should never reach humans.\n<!-- llm-only:end -->\n";
    write_worker_with_intro(tmp.path(), "fixture", intro);

    let outputs = iii_skill_core::render::render_worker(tmp.path()).unwrap();

    assert!(
        !outputs
            .readme
            .contains("Agent-only routing hint that should never reach humans"),
        "llm-only body leaked into README: {}",
        outputs.readme
    );
    assert!(
        !outputs.readme.contains("llm-only:start"),
        "llm-only marker leaked into README: {}",
        outputs.readme
    );
    assert!(
        outputs.readme.contains("Visible intro paragraph."),
        "visible intro prose absent from README: {}",
        outputs.readme
    );

    // skill.md is the agent-facing artifact: the body must survive, markers
    // must not.
    assert!(
        outputs
            .skill
            .contains("Agent-only routing hint that should never reach humans"),
        "llm-only body absent from skill.md: {}",
        outputs.skill
    );
    assert!(
        !outputs.skill.contains("llm-only:start"),
        "llm-only marker leaked into skill.md: {}",
        outputs.skill
    );
}

/// Inline `llm-only` comments must also drop from the README (markers AND
/// payload). The inline form is recognised only when the comment occupies
/// the entire line (see llm_only::is_inline_llm_only); embedded
/// mid-paragraph comments are not parsed and pass through verbatim.
#[test]
fn intro_llm_only_inline_dropped_in_readme() {
    let tmp = TempDir::new().unwrap();
    let intro = "Plain prose.\n<!-- llm-only: agents should prefer foo over bar -->\nNext sentence.\n";
    write_worker_with_intro(tmp.path(), "fixture", intro);

    let outputs = iii_skill_core::render::render_worker(tmp.path()).unwrap();

    assert!(
        !outputs
            .readme
            .contains("agents should prefer foo over bar"),
        "inline llm-only payload leaked into README: {}",
        outputs.readme
    );
    assert!(
        outputs.skill.contains("agents should prefer foo over bar"),
        "inline llm-only payload absent from skill.md: {}",
        outputs.skill
    );
}

/// Inverse direction: a `human-only` block in `docs/intro.md` renders as
/// visible prose in the README and is excised entirely from skill.md.
#[test]
fn intro_human_only_block_kept_in_readme_dropped_in_skill() {
    let tmp = TempDir::new().unwrap();
    let intro = "Shared prose.\n\n<!-- human-only:start -->\nMaintainer-facing note about the legacy wrapper.\n<!-- human-only:end -->\n";
    write_worker_with_intro(tmp.path(), "fixture", intro);

    let outputs = iii_skill_core::render::render_worker(tmp.path()).unwrap();

    assert!(
        outputs
            .readme
            .contains("Maintainer-facing note about the legacy wrapper"),
        "human-only body absent from README: {}",
        outputs.readme
    );
    assert!(
        !outputs.readme.contains("human-only:start"),
        "human-only marker leaked into README: {}",
        outputs.readme
    );
    assert!(
        !outputs
            .skill
            .contains("Maintainer-facing note about the legacy wrapper"),
        "human-only body leaked into skill.md: {}",
        outputs.skill
    );
}

/// A leaf whose H1 lives inside an `llm-only` block must not surface that
/// H1 as the README `## Additional Resources` link text. The renderer
/// falls back to the leaf name; the structure layer is the place that
/// flags the missing H1 (covered in tests/structure.rs).
#[test]
fn leaf_h1_inside_llm_only_block_does_not_leak_into_readme() {
    let tmp = TempDir::new().unwrap();
    write_worker_with_intro(tmp.path(), "fixture", "Intro.\n");

    let leaves = tmp.path().join("docs").join("leaves");
    std::fs::create_dir_all(&leaves).unwrap();
    std::fs::write(
        leaves.join("verb.md"),
        "<!-- llm-only:start -->\n# Hidden agent-only title\n<!-- llm-only:end -->\n\nVisible body.\n",
    )
    .unwrap();

    let outputs = iii_skill_core::render::render_worker(tmp.path()).unwrap();

    assert!(
        !outputs.readme.contains("Hidden agent-only title"),
        "leaf H1 inside llm-only block leaked into README link text: {}",
        outputs.readme
    );
    // Fallback: link text is the bare leaf name when no H1 is available.
    assert!(
        outputs.readme.contains("- [verb](skills/verb.md)"),
        "expected leaf-name fallback link text in README: {}",
        outputs.readme
    );
}
