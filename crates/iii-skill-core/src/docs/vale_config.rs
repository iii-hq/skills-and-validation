//! Build a runtime `.vale.ini` for docs-mode validation.
//!
//! Each in-scope `<source>.skill.md` artifact gets a `[<exact-path>]`
//! block whose `Diataxis.*` toggles match its frontmatter `type:`. The
//! Diataxis style files themselves live in the bundled
//! `content/styles/Diataxis/`; we just point at them via `StylesPath` and
//! switch which rules apply per file.
//!
//! Path-based rules (the approach `ideal-docs` uses today) work but
//! force callers to lay out their docs by directory. Frontmatter-driven
//! application means a doc can live anywhere and still get the right
//! ruleset.

use crate::docs::frontmatter::DocType;
use std::path::Path;

/// Render a complete `.vale.ini` covering every artifact passed in.
///
/// `styles_path` should point at the `content/styles/` directory in the
/// installed bundle (or a local override). It's emitted verbatim into
/// the `StylesPath = ...` line, so it can be relative to wherever the
/// generated config lives.
pub fn build(artifacts: &[(&Path, DocType)], styles_path: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("StylesPath = {styles_path}\n"));
    out.push_str("MinAlertLevel = suggestion\n\n");
    out.push_str("[formats]\n");
    out.push_str("mdx = md\n\n");

    for (path, ty) in artifacts {
        let p = path.to_string_lossy();
        out.push_str(&format!("[{p}]\n"));
        out.push_str("BasedOnStyles = Diataxis, Terminology\n");
        out.push_str(rules_for(*ty));
        out.push('\n');
    }

    out
}

/// Per-type Diataxis rule block, modeled after `ideal-docs/.vale.ini`.
fn rules_for(ty: DocType) -> &'static str {
    match ty {
        DocType::Tutorial => concat!(
            "Diataxis.Tutorial = YES\n",
            "Diataxis.TutorialExplanation = YES\n",
            "Diataxis.TutorialAbstraction = YES\n",
            "Diataxis.TutorialReferenceLists = YES\n",
            "Diataxis.HowTo = NO\n",
            "Diataxis.HowToBackground = NO\n",
            "Diataxis.Reference = NO\n",
            "Diataxis.ReferenceOpinion = NO\n",
            "Diataxis.ReferenceTeaching = NO\n",
            "Diataxis.Explanation = NO\n",
            "Diataxis.ExplanationImperatives = NO\n",
            "Diataxis.CrossContamination = NO\n",
        ),
        DocType::HowTo => concat!(
            "Diataxis.Tutorial = NO\n",
            "Diataxis.TutorialExplanation = NO\n",
            "Diataxis.TutorialAbstraction = NO\n",
            "Diataxis.TutorialReferenceLists = NO\n",
            "Diataxis.HowTo = YES\n",
            "Diataxis.HowToBackground = YES\n",
            "Diataxis.Reference = NO\n",
            "Diataxis.ReferenceOpinion = NO\n",
            "Diataxis.ReferenceTeaching = NO\n",
            "Diataxis.Explanation = NO\n",
            "Diataxis.ExplanationImperatives = NO\n",
            "Diataxis.CrossContamination = YES\n",
        ),
        DocType::Reference => concat!(
            "Diataxis.Tutorial = NO\n",
            "Diataxis.TutorialExplanation = NO\n",
            "Diataxis.TutorialAbstraction = NO\n",
            "Diataxis.TutorialReferenceLists = NO\n",
            "Diataxis.HowTo = NO\n",
            "Diataxis.HowToBackground = NO\n",
            "Diataxis.Reference = YES\n",
            "Diataxis.ReferenceOpinion = YES\n",
            "Diataxis.ReferenceTeaching = YES\n",
            "Diataxis.Explanation = NO\n",
            "Diataxis.ExplanationImperatives = NO\n",
            "Diataxis.CrossContamination = YES\n",
        ),
        DocType::Explanation => concat!(
            "Diataxis.Tutorial = NO\n",
            "Diataxis.TutorialExplanation = NO\n",
            "Diataxis.TutorialAbstraction = NO\n",
            "Diataxis.TutorialReferenceLists = NO\n",
            "Diataxis.HowTo = NO\n",
            "Diataxis.HowToBackground = NO\n",
            "Diataxis.Reference = NO\n",
            "Diataxis.ReferenceOpinion = NO\n",
            "Diataxis.ReferenceTeaching = NO\n",
            "Diataxis.Explanation = YES\n",
            "Diataxis.ExplanationImperatives = YES\n",
            "Diataxis.CrossContamination = YES\n",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn emits_per_artifact_blocks_with_type_rules() {
        let p1 = PathBuf::from("/x/tutorials/intro.mdx.skill.md");
        let p2 = PathBuf::from("/x/how-to/foo.mdx.skill.md");
        let cfg = build(
            &[(&p1, DocType::Tutorial), (&p2, DocType::HowTo)],
            "/styles",
        );
        assert!(cfg.contains("StylesPath = /styles"));
        assert!(cfg.contains("[/x/tutorials/intro.mdx.skill.md]"));
        assert!(cfg.contains("Diataxis.Tutorial = YES"));
        assert!(cfg.contains("[/x/how-to/foo.mdx.skill.md]"));
        assert!(cfg.contains("Diataxis.HowTo = YES"));
        // Ensure formats line is present so .mdx is treated as markdown.
        assert!(cfg.contains("[formats]"));
    }

    #[test]
    fn handles_empty_artifact_list() {
        let cfg = build(&[], "styles");
        // Still valid: header but no per-file blocks. Vale on this config
        // would just no-op.
        assert!(cfg.contains("StylesPath = styles"));
        assert!(!cfg.contains("BasedOnStyles"));
    }
}
