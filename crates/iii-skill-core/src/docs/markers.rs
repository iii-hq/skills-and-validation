//! `<!-- skill:... -->` HTML-comment marker scanning.
//!
//! Five markers, two scopes:
//!
//! Doc-level (top-of-file overrides — beat the include/exclude globs in
//! `.skill-check.yaml`):
//!   `<!-- skill:include-doc -->`     opt this doc into the skill set
//!   `<!-- skill:exclude-doc -->`     opt this doc out of the skill set
//!
//! File-level default for sections (anywhere in the file; first wins):
//!   `<!-- skill:include-sections-by-default -->`
//!   `<!-- skill:exclude-sections-by-default -->`
//!
//! Heading-level (per-heading override of the file-level default; appears
//! on the same line as a heading or anywhere in that heading's section
//! before the next heading):
//!   `<!-- skill:include-section -->`
//!   `<!-- skill:exclude-section -->`

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

const M_INCLUDE_DOC: &str = "<!-- skill:include-doc -->";
const M_EXCLUDE_DOC: &str = "<!-- skill:exclude-doc -->";
const M_INCLUDE_BY_DEFAULT: &str = "<!-- skill:include-sections-by-default -->";
const M_EXCLUDE_BY_DEFAULT: &str = "<!-- skill:exclude-sections-by-default -->";
const M_INCLUDE_SECTION: &str = "<!-- skill:include-section -->";
const M_EXCLUDE_SECTION: &str = "<!-- skill:exclude-section -->";

/// Scan the doc body (post-frontmatter) for the doc-scope marker. The
/// marker can appear anywhere in the file but conventionally lives near
/// the top. First marker wins; conflicting markers are reported by
/// [`scan_markers_with_diagnostics`] but [`scan_doc_scope`] returns the
/// first one and ignores the rest.
pub fn scan_doc_scope(body: &str) -> Option<DocScope> {
    for line in body.lines() {
        let t = line.trim();
        if t == M_INCLUDE_DOC {
            return Some(DocScope::Include);
        }
        if t == M_EXCLUDE_DOC {
            return Some(DocScope::Exclude);
        }
    }
    None
}

/// Scan the doc body for the file-level section default. Returns the
/// default if found; otherwise `IncludeByDefault`.
pub fn scan_section_default(body: &str) -> SectionDefault {
    for line in body.lines() {
        let t = line.trim();
        if t == M_INCLUDE_BY_DEFAULT {
            return SectionDefault::IncludeByDefault;
        }
        if t == M_EXCLUDE_BY_DEFAULT {
            return SectionDefault::ExcludeByDefault;
        }
    }
    SectionDefault::default()
}

/// True iff a single line is exactly any recognised `skill:` marker.
/// Used by the renderer to drop marker lines from rendered output and by
/// the structure layer to detect orphan tokens.
pub fn is_marker_line(line: &str) -> bool {
    matches!(
        line.trim(),
        M_INCLUDE_DOC
            | M_EXCLUDE_DOC
            | M_INCLUDE_BY_DEFAULT
            | M_EXCLUDE_BY_DEFAULT
            | M_INCLUDE_SECTION
            | M_EXCLUDE_SECTION
    )
}

/// Parse a per-heading override appearing on `line`. Two acceptable forms:
///   - `## Heading <!-- skill:exclude-section -->`  (trailing on heading)
///   - `<!-- skill:exclude-section -->` on its own line
pub fn parse_section_override(line: &str) -> Option<SectionOverride> {
    let t = line.trim();
    if t.contains(M_INCLUDE_SECTION) {
        return Some(SectionOverride::Include);
    }
    if t.contains(M_EXCLUDE_SECTION) {
        return Some(SectionOverride::Exclude);
    }
    None
}

/// Strip an inline section override comment from a heading line so the
/// rendered heading stays clean. `## Heading <!-- skill:exclude-section -->`
/// becomes `## Heading`.
pub fn strip_section_override(line: &str) -> String {
    line.replace(M_INCLUDE_SECTION, "")
        .replace(M_EXCLUDE_SECTION, "")
        .trim_end()
        .to_string()
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
    fn section_override_on_own_line() {
        assert_eq!(
            parse_section_override("<!-- skill:include-section -->"),
            Some(SectionOverride::Include)
        );
    }

    #[test]
    fn marker_recognition() {
        assert!(is_marker_line("<!-- skill:include-doc -->"));
        assert!(is_marker_line("  <!-- skill:exclude-section -->"));
        assert!(!is_marker_line("# Heading"));
        assert!(!is_marker_line("<!-- llm-only:start -->"));
    }
}
