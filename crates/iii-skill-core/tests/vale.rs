mod common;

use common::{repo_root, RenderedTemplate};
use std::path::PathBuf;
use tempfile::TempDir;

fn vale_config() -> PathBuf {
    repo_root().join("content/.vale.ini")
}

#[test]
fn vale_passes_example_artifacts() {
    let rendered = RenderedTemplate::lock();
    let example = rendered.worker();

    let readme = example.join("README.md");
    let skill = example.join("skill.md");
    let analyze = example.join("skills").join("analyze.md");
    let diff = example.join("skills").join("diff.md");
    let summarize = example.join("skills").join("summarize.md");
    let artifacts: Vec<&std::path::Path> = vec![&readme, &skill, &analyze, &diff, &summarize];

    let violations =
        iii_skill_core::vale::run(&artifacts, &vale_config()).expect("vale should run");
    assert!(
        violations.is_empty(),
        "expected zero Vale violations on example-worker, got: {violations:?}"
    );
}

#[test]
fn vale_flags_marketing_fluff_in_a_readme() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("README.md");
    std::fs::write(
        &path,
        "# Test\n\nBlazing fast and revolutionary.\n",
    )
    .unwrap();

    let artifacts: Vec<&std::path::Path> = vec![&path];

    let violations = iii_skill_core::vale::run(&artifacts, &vale_config()).unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.message.to_lowercase().contains("blazing")),
        "expected 'blazing fast' to be flagged, got: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.message.to_lowercase().contains("revolutionary")),
        "expected 'revolutionary' to be flagged, got: {violations:?}"
    );
}

#[test]
fn vale_flags_howto_teaching_phrase_in_skill() {
    let tmp = TempDir::new().unwrap();
    // skill.md is treated as a how-to in .vale.ini — Diataxis.HowTo applies.
    let path = tmp.path().join("skill.md");
    std::fs::write(
        &path,
        "# Test\n\nIn this guide you will learn how to use this worker.\n",
    )
    .unwrap();

    let artifacts: Vec<&std::path::Path> = vec![&path];

    let violations = iii_skill_core::vale::run(&artifacts, &vale_config()).unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.message.to_lowercase().contains("teaching")
                || v.message.to_lowercase().contains("how-to")),
        "expected a how-to/teaching violation, got: {violations:?}"
    );
}

#[test]
fn vale_severity_maps_error_and_warning_distinctly() {
    use iii_skill_core::structure::Severity;

    let tmp = TempDir::new().unwrap();
    // Mix two violations of different severity:
    //   - "Blazing fast" → Terminology.SlopMarketing (level: error)
    //   - "In this guide you will learn" → Diataxis.HowTo (level: warning)
    // skill.md is treated as a how-to in .vale.ini, so both checks apply.
    let path = tmp.path().join("skill.md");
    std::fs::write(
        &path,
        "# Test\n\nBlazing fast.\n\nIn this guide you will learn how to use this worker.\n",
    )
    .unwrap();
    let artifacts: Vec<&std::path::Path> = vec![&path];

    let violations = iii_skill_core::vale::run(&artifacts, &vale_config()).unwrap();

    let errors: Vec<_> = violations
        .iter()
        .filter(|v| v.severity == Severity::Error)
        .collect();
    let warnings: Vec<_> = violations
        .iter()
        .filter(|v| v.severity == Severity::Warning)
        .collect();

    assert!(
        errors
            .iter()
            .any(|v| v.message.to_lowercase().contains("blazing")),
        "expected an error for 'Blazing fast', got: {violations:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|v| v.message.to_lowercase().contains("teaching")
                || v.message.to_lowercase().contains("how-to")),
        "expected a warning for how-to/teaching framing, got: {violations:?}"
    );
}
