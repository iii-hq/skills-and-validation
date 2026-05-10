//! End-to-end tests against `fixtures/example-docs/`.
//!
//! These exercise the docs-mode pipeline (config v2 → enumerate → render
//! → check_rendered) without requiring CI tooling. The fixture covers:
//!   - one of each Diataxis type (tutorial, how-to, reference)
//!   - section-level inclusion (default-include and default-exclude)
//!   - doc-level opt-in (CHANGELOG.md, glob-excluded → opted in)
//!   - doc-level opt-out (draft.mdx, glob-included → opted out)
//!   - llm-only blocks
//!
//! The renderer is its own oracle: re-render and compare against a freshly
//! computed in-memory render. Drift is also exercised explicitly.

use iii_skill_core::config::Mode;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("fixtures/example-docs")
}

fn load_config() -> iii_skill_core::config::Config {
    let path = fixture_root().join(".skill-check.yaml");
    iii_skill_core::config::load(&path).expect("config loads")
}

#[test]
fn config_resolves_to_docs_mode() {
    let config = load_config();
    assert_eq!(config.resolved_mode(), Mode::Docs);
    assert!(config.docs.is_some());
}

#[test]
fn enumerate_picks_up_all_in_scope_docs() {
    let config = load_config();
    let docs = iii_skill_core::docs::enumerate::enumerate(
        &fixture_root(),
        config.docs.as_ref().unwrap(),
    )
    .unwrap();

    let rels: Vec<&str> = docs.iter().map(|d| d.rel.as_str()).collect();
    // CHANGELOG.md is path-excluded; the `skill:include-doc` marker no
    // longer overrides path excludes, so CHANGELOG stays out.
    // draft.mdx is glob-included but has `skill:exclude-doc` → absent.
    let expected = vec![
        "how-to/rotate-credentials.mdx",
        "reference/cli.md",
        "tutorials/intro.mdx",
    ];
    assert_eq!(rels, expected, "scope mismatch");
}

#[test]
fn rendering_strips_excluded_section_in_tutorial() {
    let path = fixture_root().join("tutorials/intro.mdx");
    let rendered = iii_skill_core::docs::render::render_doc(&path).unwrap();
    assert!(rendered.body.contains("Set up the project"));
    assert!(rendered.body.contains("Run it"));
    // The "## Internals" section was opted out via skill:exclude-section.
    assert!(
        !rendered.body.contains("Internals"),
        "expected Internals section dropped, got:\n{}",
        rendered.body
    );
    assert!(!rendered.body.contains("polls for triggers"));
}

#[test]
fn rendering_default_excludes_for_reference_doc() {
    let path = fixture_root().join("reference/cli.md");
    let rendered = iii_skill_core::docs::render::render_doc(&path).unwrap();
    // worker + creds sections are explicitly opted in.
    assert!(rendered.body.contains("iii worker new"));
    assert!(rendered.body.contains("iii creds new"));
    // update section had no marker so the file-level exclude default drops it.
    assert!(
        !rendered.body.contains("Bumps the engine binary"),
        "update section should have been excluded by default"
    );
}

#[test]
fn rendering_unwraps_llm_only_blocks_in_how_to() {
    let path = fixture_root().join("how-to/rotate-credentials.mdx");
    let rendered = iii_skill_core::docs::render::render_doc(&path).unwrap();
    assert!(rendered.body.contains("two-phase commit"));
    assert!(!rendered.body.contains("llm-only:start"));
}

#[test]
fn check_rendered_flags_missing_skill_files() {
    let config = load_config();
    let drift = iii_skill_core::docs::check_rendered::check_rendered(
        &fixture_root(),
        config.docs.as_ref().unwrap(),
    )
    .expect("check_rendered runs");
    // The fixture intentionally ships sources only — every in-scope doc
    // should be flagged as out-of-date. Three docs in scope after the
    // marker-precedence change (CHANGELOG.md no longer overrides the path
    // exclude).
    assert_eq!(drift.len(), 3, "got: {drift:?}");
    for line in &drift {
        assert!(
            line.contains("is out of date"),
            "expected drift line, got: {line}"
        );
    }
}

#[test]
fn vale_config_emits_per_type_blocks() {
    let config = load_config();
    let docs = iii_skill_core::docs::enumerate::enumerate(
        &fixture_root(),
        config.docs.as_ref().unwrap(),
    )
    .unwrap();

    let mut typed: Vec<(PathBuf, iii_skill_core::docs::frontmatter::DocType)> = Vec::new();
    for doc in &docs {
        let body = std::fs::read_to_string(&doc.abs).unwrap();
        let parsed = iii_skill_core::docs::frontmatter::parse(&body).unwrap();
        typed.push((doc.skill_path(), parsed.frontmatter.doc_type));
    }
    let refs: Vec<(&std::path::Path, iii_skill_core::docs::frontmatter::DocType)> =
        typed.iter().map(|(p, ty)| (p.as_path(), *ty)).collect();
    let cfg = iii_skill_core::docs::vale_config::build(&refs, "/styles");

    // Each artifact gets a [path] block.
    for (p, _) in &typed {
        assert!(cfg.contains(&format!("[{}]", p.display())));
    }
    // At least one of each rule shows up — sanity check the type mapping.
    assert!(cfg.contains("Diataxis.Tutorial = YES"));
    assert!(cfg.contains("Diataxis.HowTo = YES"));
    assert!(cfg.contains("Diataxis.Reference = YES"));
}
