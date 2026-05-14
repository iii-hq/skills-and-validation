//! `llm-only:...` block + inline marker handling.
//!
//! Source partials may contain content that should be visible to the AI /
//! skill consumer but hidden from the published README. The renderer wraps
//! the AI-only sections in matching block markers (`...:start` / `...:end`)
//! or a single inline marker. Two equivalent comment forms are accepted:
//!
//! - HTML form: `<!-- llm-only:start -->`, `<!-- llm-only: inline note -->`
//! - MDX form:  `{/* llm-only:start */}`, `{/* llm-only: inline note */}`
//!
//! `.mdx` files strip HTML comments at render time, so the MDX form is the
//! one that survives in MDX-based docs. `.md` files accept either form.
//!
//! When producing skill.md / skill artifacts, we strip the block markers and
//! expand the inline form to its inner text. README rendering passes the
//! source through unchanged so the comments stay invisible to humans.

/// Strip `llm-only:start` / `llm-only:end` block markers and expand the
/// `llm-only: ...` inline form to its inner text. Both HTML and MDX comment
/// forms are recognised. Use this when rendering for an LLM-facing target
/// (skill.md, skills/*.md, doc skill siblings). For a human-facing target
/// (README.md), pass the source through unchanged.
pub fn unwrap_llm_only(content: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(inner) = strip_comment(trimmed) {
            let inner = inner.trim();
            if inner == "llm-only:start" || inner == "llm-only:end" {
                continue;
            }
            if let Some(payload) = inner.strip_prefix("llm-only:") {
                out.push(payload.trim().to_string());
                continue;
            }
        }
        out.push(line.to_string());
    }
    out.join("\n")
}

/// Drop every `llm-only:start` ... `llm-only:end` block (markers and body
/// included) and every inline `llm-only: ...` line from `content`. Use
/// this to produce the "validator view" of an artifact — the prose a
/// human reader would actually see, with all LLM-targeted content
/// excised. Mirror of [`crate::human_only::strip_human_only`]. Returns
/// the source unchanged when no markers are present.
pub fn strip_llm_only(content: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut inside = false;
    for line in content.lines() {
        if is_llm_only_start(line) {
            inside = true;
            continue;
        }
        if is_llm_only_end(line) {
            inside = false;
            continue;
        }
        if inside {
            continue;
        }
        if is_inline_llm_only(line) {
            continue;
        }
        out.push(line.to_string());
    }
    out.join("\n")
}

/// True iff `line` (trimmed) is an inline `llm-only: payload` comment
/// with no surrounding content. Block start/end markers do not count.
fn is_inline_llm_only(line: &str) -> bool {
    let Some(inner) = strip_comment(line.trim()) else {
        return false;
    };
    let inner = inner.trim();
    if inner == "llm-only:start" || inner == "llm-only:end" {
        return false;
    }
    inner.strip_prefix("llm-only:").is_some()
}

/// True iff `line` (trimmed) is exactly an `llm-only:start` marker in either
/// comment form.
pub fn is_llm_only_start(line: &str) -> bool {
    strip_comment(line.trim()).is_some_and(|inner| inner.trim() == "llm-only:start")
}

/// True iff `line` (trimmed) is exactly an `llm-only:end` marker in either
/// comment form.
pub fn is_llm_only_end(line: &str) -> bool {
    strip_comment(line.trim()).is_some_and(|inner| inner.trim() == "llm-only:end")
}

/// If `trimmed` is a single comment with no surrounding content, return the
/// inner directive text. Handles both `<!-- ... -->` and `{/* ... */}`.
fn strip_comment(trimmed: &str) -> Option<&str> {
    if let Some(rest) = trimmed
        .strip_prefix("<!--")
        .and_then(|r| r.strip_suffix("-->"))
    {
        return Some(rest);
    }
    if let Some(rest) = trimmed
        .strip_prefix("{/*")
        .and_then(|r| r.strip_suffix("*/}"))
    {
        return Some(rest);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwrap_drops_block_markers() {
        let input = "Before\n\n<!-- llm-only:start -->\nInside\n<!-- llm-only:end -->\n\nAfter";
        let expected = "Before\n\nInside\n\nAfter";
        assert_eq!(unwrap_llm_only(input), expected);
    }

    #[test]
    fn unwrap_drops_mdx_block_markers() {
        let input = "Before\n\n{/* llm-only:start */}\nInside\n{/* llm-only:end */}\n\nAfter";
        let expected = "Before\n\nInside\n\nAfter";
        assert_eq!(unwrap_llm_only(input), expected);
    }

    #[test]
    fn unwrap_inline_form_replaces_with_inner() {
        let input = "Before\n<!-- llm-only: short note -->\nAfter";
        let expected = "Before\nshort note\nAfter";
        assert_eq!(unwrap_llm_only(input), expected);
    }

    #[test]
    fn unwrap_mdx_inline_form_replaces_with_inner() {
        let input = "Before\n{/* llm-only: short note */}\nAfter";
        let expected = "Before\nshort note\nAfter";
        assert_eq!(unwrap_llm_only(input), expected);
    }

    #[test]
    fn unwrap_mixed_forms_in_same_file() {
        let input = "<!-- llm-only:start -->\nA\n{/* llm-only:end */}\nB\n{/* llm-only: inline */}";
        let expected = "A\nB\ninline";
        assert_eq!(unwrap_llm_only(input), expected);
    }

    #[test]
    fn unwrap_idempotent_when_no_markers() {
        let input = "Just plain prose\n\nWith no markers.";
        assert_eq!(unwrap_llm_only(input), input);
    }

    #[test]
    fn strip_drops_block_with_body_and_markers() {
        let input =
            "Before\n\n<!-- llm-only:start -->\nLLM-only guidance\nmore guidance\n<!-- llm-only:end -->\n\nAfter";
        let expected = "Before\n\n\nAfter";
        assert_eq!(strip_llm_only(input), expected);
    }

    #[test]
    fn strip_drops_mdx_block_with_body_and_markers() {
        let input =
            "Before\n\n{/* llm-only:start */}\nLLM-only guidance\n{/* llm-only:end */}\n\nAfter";
        let expected = "Before\n\n\nAfter";
        assert_eq!(strip_llm_only(input), expected);
    }

    #[test]
    fn strip_drops_inline_html_and_mdx() {
        let input =
            "Before\n<!-- llm-only: maintainer hint -->\n{/* llm-only: another hint */}\nAfter";
        let expected = "Before\nAfter";
        assert_eq!(strip_llm_only(input), expected);
    }

    #[test]
    fn strip_idempotent_when_no_markers() {
        let input = "Just plain prose.\n\nWith no markers.";
        assert_eq!(strip_llm_only(input), input);
    }

    #[test]
    fn marker_predicates_recognise_both_forms() {
        assert!(is_llm_only_start("<!-- llm-only:start -->"));
        assert!(is_llm_only_start("{/* llm-only:start */}"));
        assert!(is_llm_only_end("<!-- llm-only:end -->"));
        assert!(is_llm_only_end("{/* llm-only:end */}"));
        assert!(!is_llm_only_start("<!-- llm-only:end -->"));
        assert!(!is_llm_only_end("plain text"));
    }
}
