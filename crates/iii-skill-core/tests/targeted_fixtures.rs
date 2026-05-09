use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

/// Each targeted fixture deliberately ships exactly one rule violation that
/// the AI layer should catch. The render byte-exact tests here keep their
/// rendered artifacts in sync with the sources, so the AI tests in
/// `tests/ai.rs` don't end up evaluating stale content.

#[test]
fn render_bad_sdk_worker_matches_golden() {
    let dir = repo_root().join("fixtures/bad-sdk-worker");
    let outputs =
        iii_skill_core::render::render_worker(&dir).expect("render bad-sdk-worker");

    let expected_readme = std::fs::read_to_string(dir.join("README.md")).unwrap();
    similar_asserts::assert_eq!(expected_readme, outputs.readme);

    let expected_skill = std::fs::read_to_string(dir.join("skill.md")).unwrap();
    similar_asserts::assert_eq!(expected_skill, outputs.skill);
}

#[test]
fn render_bad_concept_worker_matches_golden() {
    let dir = repo_root().join("fixtures/bad-concept-worker");
    let outputs =
        iii_skill_core::render::render_worker(&dir).expect("render bad-concept-worker");

    let expected_readme = std::fs::read_to_string(dir.join("README.md")).unwrap();
    similar_asserts::assert_eq!(expected_readme, outputs.readme);

    let expected_skill = std::fs::read_to_string(dir.join("skill.md")).unwrap();
    similar_asserts::assert_eq!(expected_skill, outputs.skill);
}
