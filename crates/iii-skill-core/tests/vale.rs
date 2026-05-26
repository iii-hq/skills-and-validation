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
fn vale_flags_negation_contrast() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("README.md");
    std::fs::write(
        &path,
        "# Test\n\nIt's not a queue, it's a coordination primitive.\n",
    )
    .unwrap();

    let artifacts: Vec<&std::path::Path> = vec![&path];

    let violations = iii_skill_core::vale::run(&artifacts, &vale_config()).unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.message.to_lowercase().contains("contrast")),
        "expected a 'not X, it's Y' contrast violation, got: {violations:?}"
    );
}

#[test]
fn vale_does_not_flag_plain_negation_with_caveat() {
    // A bare negation followed by a "but" caveat is legitimate prose, not
    // the antithesis tic. NegationContrast must leave it alone.
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("README.md");
    std::fs::write(
        &path,
        "# Test\n\nThe flag is not required, but you can pass it.\n",
    )
    .unwrap();

    let artifacts: Vec<&std::path::Path> = vec![&path];

    let violations = iii_skill_core::vale::run(&artifacts, &vale_config()).unwrap();
    assert!(
        !violations
            .iter()
            .any(|v| v.message.to_lowercase().contains("contrast")),
        "plain negation-with-caveat should not be flagged, got: {violations:?}"
    );
}

#[test]
fn vale_flags_hedges_as_warnings() {
    use iii_skill_core::structure::Severity;

    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("README.md");
    std::fs::write(
        &path,
        "# Test\n\nThis is just a thin wrapper that simply forwards calls.\n",
    )
    .unwrap();

    let artifacts: Vec<&std::path::Path> = vec![&path];

    let violations = iii_skill_core::vale::run(&artifacts, &vale_config()).unwrap();
    let hedge_warnings: Vec<_> = violations
        .iter()
        .filter(|v| v.severity == Severity::Warning && v.message.to_lowercase().contains("hedge"))
        .collect();
    assert!(
        hedge_warnings.iter().any(|v| v.message.contains("just")),
        "expected a hedge warning for 'just', got: {violations:?}"
    );
    assert!(
        hedge_warnings.iter().any(|v| v.message.contains("simply")),
        "expected a hedge warning for 'simply', got: {violations:?}"
    );
}

#[test]
fn vale_does_not_flag_just_inside_another_word() {
    // `\bjust\b` must not match "adjust"; the hedge rule is word-bounded.
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("README.md");
    std::fs::write(&path, "# Test\n\nAdjust the timeout to taste.\n").unwrap();

    let artifacts: Vec<&std::path::Path> = vec![&path];

    let violations = iii_skill_core::vale::run(&artifacts, &vale_config()).unwrap();
    assert!(
        !violations
            .iter()
            .any(|v| v.message.to_lowercase().contains("hedge")),
        "'adjust' should not trip the hedge rule, got: {violations:?}"
    );
}

#[test]
fn vale_severity_maps_error_and_warning_distinctly() {
    use iii_skill_core::structure::Severity;

    let tmp = TempDir::new().unwrap();
    // Mix two violations of different severity:
    //   - "Blazing fast" → Terminology.SlopMarketing (level: error)
    //   - "In this tutorial we'll" → Diataxis.CrossContamination (level: warning)
    // skill.md is treated as a how-to in .vale.ini and CrossContamination
    // runs on every Diataxis-scoped doc, so both checks apply here.
    let path = tmp.path().join("skill.md");
    std::fs::write(
        &path,
        "# Test\n\nBlazing fast.\n\nIn this tutorial we'll cover the basics.\n",
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
            .any(|v| v.message.to_lowercase().contains("tutorial")),
        "expected a warning for cross-contamination/tutorial framing, got: {violations:?}"
    );
}
