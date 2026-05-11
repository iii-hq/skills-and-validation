//! `skill:...` HTML / MDX comment marker scanning.
//!
//! Two equivalent comment forms are accepted on every marker:
//!   - HTML form: `<!-- skill:include-doc -->`
//!   - MDX form:  `{/* skill:include-doc */}`
//!
//! `.mdx` files strip HTML comments at render time, so the MDX form is
//! what survives in MDX-based docs. `.md` files accept either form.
//!
//! Five markers, two scopes:
//!
//! Doc-level (top-of-file overrides, beat the include/exclude globs in
//! `.skill-check.yaml`):
//!   `skill:include-doc`     opt this doc into the skill set
//!   `skill:exclude-doc`     opt this doc out of the skill set
//!
//! File-level default for sections (anywhere in the file; first wins):
//!   `skill:include-sections-by-default`
//!   `skill:exclude-sections-by-default`
//!
//! Heading-level (per-heading override of the file-level default; appears
//! on the same line as a heading or anywhere in that heading's section
//! before the next heading):
//!   `skill:include-section`
//!   `skill:exclude-section`

/// Doc-level scope override, derived from a top-of-file marker. `None`
/// means no marker present and the include/exclude globs decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocScope {
    Include,
    Exclude,
}

/// File-level default for section inclusion. Defaults to
/// `IncludeByDefault` when no marker is present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionDefault {
    IncludeByDefault,
    ExcludeByDefault,
}

impl Default for SectionDefault {
    fn default() -> Self {
        SectionDefault::IncludeByDefault
    }
}

/// Per-heading override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionOverride {
    Include,
    Exclude,
}

// Marker directive names (the inner text of the comment, sans wrapper).
const D_INCLUDE_DOC: &str = "skill:include-doc";
const D_EXCLUDE_DOC: &str = "skill:exclude-doc";
const D_INCLUDE_BY_DEFAULT: &str = "skill:include-sections-by-default";
const D_EXCLUDE_BY_DEFAULT: &str = "skill:exclude-sections-by-default";
const D_INCLUDE_SECTION: &str = "skill:include-section";
const D_EXCLUDE_SECTION: &str = "skill:exclude-section";

/// If `trimmed` is a single comment with no surrounding content, return
/// the inner directive text (trimmed). Handles both `<!-- ... -->` and
/// `{/* ... */}` forms.
fn parse_comment_line(trimmed: &str) -> Option<&str> {
    if let Some(rest) = trimmed
        .strip_prefix("<!--")
        .and_then(|r| r.strip_suffix("-->"))
    {
        return Some(rest.trim());
    }
    if let Some(rest) = trimmed
        .strip_prefix("{/*")
        .and_then(|r| r.strip_suffix("*/}"))
    {
        return Some(rest.trim());
    }
    None
}

/// Parse a doc-scope marker from a single line. Returns `Some(scope)` if
/// the line is exactly an include/exclude doc marker in either form.
pub fn parse_doc_scope_line(line: &str) -> Option<DocScope> {
    match parse_comment_line(line.trim())? {
        D_INCLUDE_DOC => Some(DocScope::Include),
        D_EXCLUDE_DOC => Some(DocScope::Exclude),
        _ => None,
    }
}

/// Scan the doc body (post-frontmatter) for the doc-scope marker. The
/// marker can appear anywhere in the file but conventionally is near
/// the top. First marker wins; conflicting markers are detected by the
/// docs-mode structure check.
pub fn scan_doc_scope(body: &str) -> Option<DocScope> {
    body.lines().find_map(parse_doc_scope_line)
}

/// Scan the doc body for the file-level section default. Returns the
/// default if found; otherwise `IncludeByDefault`.
pub fn scan_section_default(body: &str) -> SectionDefault {
    for line in body.lines() {
        if let Some(inner) = parse_comment_line(line.trim()) {
            match inner {
                D_INCLUDE_BY_DEFAULT => return SectionDefault::IncludeByDefault,
                D_EXCLUDE_BY_DEFAULT => return SectionDefault::ExcludeByDefault,
                _ => {}
            }
        }
    }
    SectionDefault::default()
}

/// True iff a single line is exactly any recognised `skill:` marker (in
/// either comment form). Used by the renderer to drop marker lines from
/// rendered output and by the structure layer to detect orphan tokens.
pub fn is_marker_line(line: &str) -> bool {
    match parse_comment_line(line.trim()) {
        Some(inner) => matches!(
            inner,
            D_INCLUDE_DOC
                | D_EXCLUDE_DOC
                | D_INCLUDE_BY_DEFAULT
                | D_EXCLUDE_BY_DEFAULT
                | D_INCLUDE_SECTION
                | D_EXCLUDE_SECTION
        ),
        None => false,
    }
}

/// Parse a per-heading override appearing on `line`. Two acceptable
/// positions, two acceptable comment forms:
///   - trailing on heading: `## Heading <!-- skill:exclude-section -->`
///   - own line:            `{/* skill:exclude-section */}`
pub fn parse_section_override(line: &str) -> Option<SectionOverride> {
    let t = line.trim();
    if contains_directive(t, D_INCLUDE_SECTION) {
        return Some(SectionOverride::Include);
    }
    if contains_directive(t, D_EXCLUDE_SECTION) {
        return Some(SectionOverride::Exclude);
    }
    None
}

/// Strip both comment forms of an inline section override from a heading
/// line so the rendered heading stays clean. `## Heading {/* skill:exclude-section */}`
/// becomes `## Heading`.
pub fn strip_section_override(line: &str) -> String {
    let mut s = line.to_string();
    for directive in [D_INCLUDE_SECTION, D_EXCLUDE_SECTION] {
        for (open, close) in [("<!--", "-->"), ("{/*", "*/}")] {
            let pat = format!("{open} {directive} {close}");
            s = s.replace(&pat, "");
        }
    }
    s.trim_end().to_string()
}

/// True iff `haystack` contains the directive wrapped in either comment
/// form. Whitespace between the wrapper and the directive is fixed at one
/// space, mirroring the canonical form the renderer emits.
fn contains_directive(haystack: &str, directive: &str) -> bool {
    haystack.contains(&format!("<!-- {directive} -->"))
        || haystack.contains(&format!("{{/* {directive} */}}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_scope_include() {
        let body = "intro\n<!-- skill:include-doc -->\nrest";
        assert_eq!(scan_doc_scope(body), Some(DocScope::Include));
    }

    #[test]
    fn doc_scope_include_mdx() {
        let body = "intro\n{/* skill:include-doc */}\nrest";
        assert_eq!(scan_doc_scope(body), Some(DocScope::Include));
    }

    #[test]
    fn doc_scope_exclude_first_wins() {
        let body = "<!-- skill:exclude-doc -->\n<!-- skill:include-doc -->";
        assert_eq!(scan_doc_scope(body), Some(DocScope::Exclude));
    }

    #[test]
    fn doc_scope_absent() {
        assert_eq!(scan_doc_scope("plain prose"), None);
    }

    #[test]
    fn section_default_include() {
        let body = "<!-- skill:include-sections-by-default -->\n# Heading";
        assert_eq!(scan_section_default(body), SectionDefault::IncludeByDefault);
    }

    #[test]
    fn section_default_exclude() {
        let body = "<!-- skill:exclude-sections-by-default -->\n# Heading";
        assert_eq!(scan_section_default(body), SectionDefault::ExcludeByDefault);
    }

    #[test]
    fn section_default_exclude_mdx() {
        let body = "{/* skill:exclude-sections-by-default */}\n# Heading";
        assert_eq!(scan_section_default(body), SectionDefault::ExcludeByDefault);
    }

    #[test]
    fn section_default_falls_back_to_include() {
        assert_eq!(scan_section_default(""), SectionDefault::IncludeByDefault);
    }

    #[test]
    fn section_override_inline_on_heading() {
        let line = "## Internals <!-- skill:exclude-section -->";
        assert_eq!(parse_section_override(line), Some(SectionOverride::Exclude));
        assert_eq!(strip_section_override(line), "## Internals");
    }

    #[test]
    fn section_override_inline_on_heading_mdx() {
        let line = "## Internals {/* skill:exclude-section */}";
        assert_eq!(parse_section_override(line), Some(SectionOverride::Exclude));
        assert_eq!(strip_section_override(line), "## Internals");
    }

    #[test]
    fn section_override_on_own_line() {
        assert_eq!(
            parse_section_override("<!-- skill:include-section -->"),
            Some(SectionOverride::Include)
        );
    }

    #[test]
    fn section_override_on_own_line_mdx() {
        assert_eq!(
            parse_section_override("{/* skill:include-section */}"),
            Some(SectionOverride::Include)
        );
    }

    #[test]
    fn marker_recognition() {
        assert!(is_marker_line("<!-- skill:include-doc -->"));
        assert!(is_marker_line("  <!-- skill:exclude-section -->"));
        assert!(is_marker_line("{/* skill:include-doc */}"));
        assert!(is_marker_line("  {/* skill:exclude-section */}"));
        assert!(!is_marker_line("# Heading"));
        assert!(!is_marker_line("<!-- llm-only:start -->"));
        assert!(!is_marker_line("{/* llm-only:start */}"));
    }
}
