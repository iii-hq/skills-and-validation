mod common;

use common::RenderedTemplate;

/// Smoke test: rendering `templates/example-worker/` produces a README and
/// skill.md that each open with the searchable frontmatter block and inline
/// every leaf under `## Additional HOWTOs`. There is no longer a skills/ dir.
/// Byte-stability of the renderer is exercised separately by check_rendered.rs.
#[test]
fn render_example_succeeds_in_place() {
    let rendered = RenderedTemplate::lock();
    let worker = rendered.worker();

    let readme = std::fs::read_to_string(worker.join("README.md")).unwrap();
    let skill = std::fs::read_to_string(worker.join("skill.md")).unwrap();

    for (label, body) in [("README.md", &readme), ("skill.md", &skill)] {
        assert!(
            body.starts_with("---\nname: textstats\n"),
            "{label} missing leading frontmatter with name"
        );
        assert!(body.contains("description:"), "{label} frontmatter missing description");
        assert!(body.contains("tags:"), "{label} frontmatter missing tags");
        assert!(body.contains("# textstats"), "{label} missing worker name heading");
        assert!(
            body.contains("## Additional HOWTOs"),
            "{label} missing Additional HOWTOs section"
        );
        // Leaves inlined as demoted H3 titles, not links.
        assert!(
            body.contains("### Sizing text before provider calls"),
            "{label} missing inlined leaf title"
        );
        assert!(
            !body.contains("](skills/"),
            "{label} should not link to skills/ files"
        );
    }

    // The single-file model: no skills/ directory.
    assert!(
        !worker.join("skills").exists(),
        "skills/ directory should not be produced"
    );
}
