mod common;

use common::RenderedTemplate;

/// Smoke test: rendering `templates/example-worker/` produces a non-empty
/// README, skill, and the three known leaves. The byte-stability of the
/// renderer is exercised separately by `check_rendered.rs` (re-rendering
/// must produce identical output).
#[test]
fn render_example_succeeds_in_place() {
    let rendered = RenderedTemplate::lock();
    let worker = rendered.worker();

    let readme = std::fs::read_to_string(worker.join("README.md")).unwrap();
    assert!(readme.contains("# textstats"), "README missing worker name heading");

    let skill = std::fs::read_to_string(worker.join("skill.md")).unwrap();
    assert!(skill.contains("# textstats"), "skill.md missing worker name heading");

    for leaf in ["analyze", "diff", "summarize"] {
        let path = worker.join("skills").join(format!("{leaf}.md"));
        let body = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("rendered leaf missing: {}", path.display()));
        assert!(
            body.contains("DO NOT EDIT"),
            "leaf {leaf} missing generated-file warning"
        );
    }
}
