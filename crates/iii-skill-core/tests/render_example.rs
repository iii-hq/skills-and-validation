use std::path::PathBuf;

/// TDD spec for the renderer.
///
/// Asserts that running `render_worker` against `fixtures/example-worker`
/// produces, byte-for-byte, the checked-in `README.md`, `skill.md`, and
/// `skills/*.md` artifacts.
///
/// The example-worker fixture is the single source of truth for how the
/// renderer must behave; if the fixture is updated, this test is what will
/// detect it.
#[test]
fn render_example_matches_golden() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf();
    let example = repo_root.join("fixtures/example-worker");

    let outputs = iii_skill_core::render::render_worker(&example)
        .expect("render_worker should succeed against the example-worker fixture");

    let expected_readme = std::fs::read_to_string(example.join("README.md")).unwrap();
    similar_asserts::assert_eq!(expected_readme, outputs.readme);

    let expected_skill = std::fs::read_to_string(example.join("skill.md")).unwrap();
    similar_asserts::assert_eq!(expected_skill, outputs.skill);

    for leaf in ["analyze", "diff", "summarize"] {
        let expected =
            std::fs::read_to_string(example.join("skills").join(format!("{leaf}.md"))).unwrap();
        let actual = outputs
            .leaves
            .get(leaf)
            .unwrap_or_else(|| panic!("missing leaf '{leaf}' in render output"));
        similar_asserts::assert_eq!(expected, *actual);
    }
}
