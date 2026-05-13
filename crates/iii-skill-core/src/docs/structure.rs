//! Docs-mode structure check.
//!
//! Runs against the *source* `.md` / `.mdx` (not the rendered skill
//! sibling) and reports:
//!   - Frontmatter missing / malformed / required field empty
//!   - Unknown `type:` value
//!   - Unbalanced `llm-only:start` / `llm-only:end` (HTML or MDX form)
//!   - Unbalanced `human-only:start` / `human-only:end` (HTML or MDX form)
//!   - Both `skill:include-doc` AND `skill:exclude-doc` declared in the
//!     same file (a strong signal something's confused)

use crate::docs::frontmatter::parse;
use crate::structure::Violation;
use std::path::Path;

/// Run the docs-mode structure check against one source file. Returns
/// zero or more violations; never errors.
pub fn check_source(path: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    let rel = path.to_string_lossy().into_owned();

    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            violations.push(Violation::error(
                rel,
                None,
                format!("could not read source: {e}"),
            ));
            return violations;
        }
    };

    if let Err(e) = parse(&content) {
        violations.push(Violation::error(
            rel.clone(),
            None,
            format!("frontmatter is missing or invalid: {e}"),
        ));
        // Without frontmatter we still want to keep checking the body
        // markers, so fall through.
    }

    violations.extend(check_llm_only_balance(&rel, &content));
    violations.extend(check_human_only_balance(&rel, &content));
    violations.extend(check_conflicting_doc_scope(&rel, &content));
    violations
}

fn check_llm_only_balance(file: &str, content: &str) -> Vec<Violation> {
    // Line-exact match (via the marker predicates) so an inline backtick
    // example inside prose doesn't count as a real marker. Both HTML and
    // MDX comment forms are recognised.
    let starts = content
        .lines()
        .filter(|l| crate::llm_only::is_llm_only_start(l))
        .count();
    let ends = content
        .lines()
        .filter(|l| crate::llm_only::is_llm_only_end(l))
        .count();
    if starts == ends {
        return Vec::new();
    }
    vec![Violation::error(
        file,
        None,
        format!("unbalanced llm-only blocks: {starts} start markers, {ends} end markers"),
    )]
}

fn check_human_only_balance(file: &str, content: &str) -> Vec<Violation> {
    let starts = content
        .lines()
        .filter(|l| crate::human_only::is_human_only_start(l))
        .count();
    let ends = content
        .lines()
        .filter(|l| crate::human_only::is_human_only_end(l))
        .count();
    if starts == ends {
        return Vec::new();
    }
    vec![Violation::error(
        file,
        None,
        format!("unbalanced human-only blocks: {starts} start markers, {ends} end markers"),
    )]
}

fn check_conflicting_doc_scope(file: &str, content: &str) -> Vec<Violation> {
    use crate::docs::markers::DocScope;
    let mut has_include = false;
    let mut has_exclude = false;
    for line in content.lines() {
        match crate::docs::markers::parse_doc_scope_line(line) {
            Some(DocScope::Include) => has_include = true,
            Some(DocScope::Exclude) => has_exclude = true,
            None => {}
        }
    }
    if has_include && has_exclude {
        return vec![Violation::error(
            file,
            None,
            "doc declares both `skill:include-doc` and `skill:exclude-doc`: pick one",
        )];
    }
    Vec::new()
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(rel: &str, content: &str) -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
        (tmp, path)
    }

    #[test]
    fn passes_clean_source() {
        let content = "---\ntitle: t\ndescription: d\ntype: how-to\n---\n\n# Heading\n\nBody.\n";
        let (_t, p) = write("foo.mdx", content);
        let violations = check_source(&p);
        assert!(violations.is_empty(), "expected clean, got {violations:?}");
    }

    #[test]
    fn flags_missing_frontmatter() {
        let (_t, p) = write("foo.mdx", "# No frontmatter\n");
        let violations = check_source(&p);
        assert!(violations.iter().any(|v| v.message.contains("frontmatter")));
    }

    #[test]
    fn flags_unknown_type() {
        let content = "---\ntitle: t\ndescription: d\ntype: walkthrough\n---\nbody\n";
        let (_t, p) = write("foo.mdx", content);
        let violations = check_source(&p);
        assert!(violations.iter().any(|v| v.message.contains("frontmatter")));
    }

    #[test]
    fn flags_unbalanced_llm_only() {
        let content = "---\ntitle: t\ndescription: d\ntype: how-to\n---\n\n<!-- llm-only:start -->\nA\n";
        let (_t, p) = write("foo.mdx", content);
        let violations = check_source(&p);
        assert!(violations.iter().any(|v| v.message.contains("unbalanced llm-only")));
    }

    #[test]
    fn flags_conflicting_doc_scope() {
        let content = "---\ntitle: t\ndescription: d\ntype: how-to\n---\n\n<!-- skill:include-doc -->\n<!-- skill:exclude-doc -->\n";
        let (_t, p) = write("foo.mdx", content);
        let violations = check_source(&p);
        assert!(violations.iter().any(|v| v.message.contains("both `skill:include-doc`")));
    }
}
