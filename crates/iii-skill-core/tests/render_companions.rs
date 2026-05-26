use std::path::Path;
use tempfile::TempDir;

/// Build a minimal worker with an optional `docs/companions.md` partial.
fn write_minimal_worker_with_companions(dir: &Path, name: &str, companions: Option<&str>) {
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
    if let Some(c) = companions {
        std::fs::write(dir.join("docs").join("companions.md"), c).unwrap();
    }
}

const COMPANION_BODY: &str = "To surface every registered skill, add the [skills](../skills) worker:\n\n```bash\niii worker add skills\n```\n";

#[test]
fn companions_partial_appears_in_readme_install_section() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker_with_companions(tmp.path(), "fixture", Some(COMPANION_BODY));

    let outputs = iii_skill_core::render::render_worker(tmp.path()).unwrap();

    let readme = &outputs.readme;
    assert!(
        readme.contains("To surface every registered skill"),
        "companions framing absent from README: {readme}"
    );
    assert!(
        readme.contains("iii worker add skills"),
        "companions install command absent from README: {readme}"
    );

    // The companion content must sit inside the ## Install section, i.e. after
    // the install-section header and before the next H2.
    let install_idx = readme.find("## Install").expect("## Install present");
    let companion_idx = readme.find("To surface every").expect("companion present");
    let quickstart_idx = readme.find("## Quickstart").expect("## Quickstart present");
    assert!(
        install_idx < companion_idx && companion_idx < quickstart_idx,
        "companion content not positioned inside ## Install"
    );
}

#[test]
fn companions_partial_appears_in_skill_md() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker_with_companions(tmp.path(), "fixture", Some(COMPANION_BODY));

    let outputs = iii_skill_core::render::render_worker(tmp.path()).unwrap();
    assert!(
        outputs.skill.contains("To surface every registered skill"),
        "companions framing absent from skill.md: {}",
        outputs.skill
    );
    assert!(
        outputs.skill.contains("iii worker add skills"),
        "companions install command absent from skill.md: {}",
        outputs.skill
    );
}

#[test]
fn companions_llm_only_block_dropped_in_readme_unwrapped_in_skill() {
    let tmp = TempDir::new().unwrap();
    let body = "Visible companion line.\n\n<!-- llm-only:start -->\nLLM-only guidance for agents.\n<!-- llm-only:end -->\n";
    write_minimal_worker_with_companions(tmp.path(), "fixture", Some(body));

    let outputs = iii_skill_core::render::render_worker(tmp.path()).unwrap();

    // skill.md: markers stripped, the inner text is plain prose.
    assert!(
        outputs.skill.contains("LLM-only guidance for agents"),
        "llm-only body absent from skill.md: {}",
        outputs.skill
    );
    assert!(
        !outputs.skill.contains("llm-only:start"),
        "llm-only marker leaked into skill.md: {}",
        outputs.skill
    );

    // README: the marker lines are invisible HTML comments, but the prose
    // between them is regular markdown that would render to humans on
    // iii.dev / GitHub. The renderer therefore drops both markers AND body.
    assert!(
        !outputs.readme.contains("LLM-only guidance for agents"),
        "llm-only body leaked into README: {}",
        outputs.readme
    );
    assert!(
        !outputs.readme.contains("llm-only:start"),
        "llm-only marker leaked into README: {}",
        outputs.readme
    );
    // Sanity: the surrounding non-llm-only prose still made it through.
    assert!(
        outputs.readme.contains("Visible companion line."),
        "visible companion prose absent from README: {}",
        outputs.readme
    );
}

#[test]
fn no_companions_partial_means_install_section_is_just_boilerplate() {
    let tmp = TempDir::new().unwrap();
    write_minimal_worker_with_companions(tmp.path(), "fixture", None);

    let outputs = iii_skill_core::render::render_worker(tmp.path()).unwrap();
    // Without companions, the only `iii worker add <name>` invocation is the
    // worker's own install command (the boilerplate sentence has the literal
    // `iii worker add` in inline code but not the trailing worker name).
    assert_eq!(
        outputs.readme.matches("iii worker add fixture").count(),
        1,
        "expected exactly one `iii worker add fixture` line in README without companions"
    );
    assert!(
        !outputs.readme.contains("iii worker add skills"),
        "expected no companion install line, got: {}",
        outputs.readme
    );
}
