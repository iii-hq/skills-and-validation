//! `llm-only:...` block + inline marker handling.
//!
//! Source partials may contain content that should be visible to the AI /
//! skill consumer but hidden from the published doc. Two equivalent comment
//! forms are accepted in either Markdown or MDX:
//!
//! - HTML form: `<!-- ... -->`
//! - MDX form:  `{/* ... */}`
//!
//! ## Authoring patterns
//!
//! ### Wrapping form (recommended for MDX, required when humans render the source)
//!
//! Place `llm-only:start` and `llm-only:end` inside **one multi-line comment**
//! that wraps the payload. The whole block is then a single comment that the
//! human-facing renderer (Mintlify, GitHub-flavored Markdown, etc.) strips
//! entirely:
//!
//! ```mdx
//! {/* llm-only:start
//! Always verify the claim above; it may have changed since this was written.
//! llm-only:end */}
//! ```
//!
//! ### Two-comment form (legacy; works for `.md` partials rendered through this
//! renderer, but NOT for `.mdx` rendered by Mintlify)
//!
//! ```md
//! <!-- llm-only:start -->
//! payload
//! <!-- llm-only:end -->
//! ```
//!
//! The two markers are each their own single-line comment. The renderer
//! recognizes them and strips both; the payload between is the LLM-visible
//! body. **Do not use this form in `.mdx` files** — the markers are stripped
//! by Mintlify but the payload between them renders as plain prose visible
//! to readers. Use the wrapping form there instead.
//!
//! ### Inline form
//!
//! `<!-- llm-only: short note -->` / `{/* llm-only: short note */}` expand
//! to the trailing `short note` text at LLM render time, and stay invisible
//! at human render time.
//!
//! ## LLM render vs human render
//!
//! [`unwrap_llm_only`] handles every accepted form and is called when
//! emitting the LLM-facing artifact (skill.md, skills/*.md, `<src>.skill.md`).
//! Human-facing rendering passes the source through unchanged — invisibility
//! comes from the source itself being inside comments.

/// Strip `llm-only:start` / `llm-only:end` block markers and expand the
/// `llm-only: ...` inline form to its inner text. Recognizes:
///
/// - Single-line markers (`<!-- llm-only:start -->`, `{/* llm-only:end */}`)
/// - The inline form (`<!-- llm-only: note -->`)
/// - The multi-line wrapping form where `llm-only:start` and `llm-only:end`
///   appear on the opener and closer lines of one comment that wraps the
///   payload (see module docs)
///
/// Use when rendering for an LLM-facing target. For a human-facing target,
/// pass the source through unchanged.
pub fn unwrap_llm_only(content: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut in_wrapping_block = false;
    for line in content.lines() {
        let trimmed = line.trim();

        if in_wrapping_block {
            // Inside a multi-line wrapping comment. Look for the closer:
            // a line containing `llm-only:end` immediately followed by the
            // comment-close token (`*/}` for MDX, `-->` for HTML). The
            // closer is dropped from the output; everything else inside
            // the block becomes visible LLM-facing payload.
            if line_closes_wrapping_block(trimmed) {
                in_wrapping_block = false;
                continue;
            }
            out.push(line.to_string());
            continue;
        }

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

        if line_opens_wrapping_block(trimmed) {
            in_wrapping_block = true;
            continue;
        }

        out.push(line.to_string());
    }
    out.join("\n")
}

/// True iff `line` (trimmed) is exactly an `llm-only:start` marker in
/// single-line comment form. The multi-line wrapping form opens differently
/// (`{/* llm-only:start` with no closer on the line) and is not matched
/// here — callers that care about opener detection should consume the line
/// or use [`unwrap_llm_only`] directly.
pub fn is_llm_only_start(line: &str) -> bool {
    strip_comment(line.trim()).is_some_and(|inner| inner.trim() == "llm-only:start")
}

/// True iff `line` (trimmed) is exactly an `llm-only:end` marker in
/// single-line comment form. See [`is_llm_only_start`].
pub fn is_llm_only_end(line: &str) -> bool {
    strip_comment(line.trim()).is_some_and(|inner| inner.trim() == "llm-only:end")
}

/// True iff the trimmed line opens a multi-line wrapping comment whose
/// opener contains `llm-only:start` and which is *not* closed on the same
/// line. MDX opener `{/*` or HTML opener `<!--` qualify.
fn line_opens_wrapping_block(trimmed: &str) -> bool {
    if trimmed.starts_with("{/*") && !trimmed.contains("*/}") {
        return trimmed.contains("llm-only:start");
    }
    if trimmed.starts_with("<!--") && !trimmed.contains("-->") {
        return trimmed.contains("llm-only:start");
    }
    false
}

/// True iff the trimmed line closes a multi-line wrapping comment whose
/// closer contains `llm-only:end` followed by `*/}` (MDX) or `-->` (HTML).
fn line_closes_wrapping_block(trimmed: &str) -> bool {
    if !trimmed.contains("llm-only:end") {
        return false;
    }
    trimmed.ends_with("*/}") || trimmed.ends_with("-->")
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
    fn unwrap_drops_mdx_wrapping_block() {
        // The MDX wrapping form puts start + payload + end inside ONE
        // multi-line comment. This is the form that's actually invisible
        // to Mintlify-style MDX rendering.
        let input = "\
Before

{/* llm-only:start
Inside payload
spread across lines
llm-only:end */}

After";
        let expected = "\
Before

Inside payload
spread across lines

After";
        assert_eq!(unwrap_llm_only(input), expected);
    }

    #[test]
    fn unwrap_drops_html_wrapping_block() {
        let input = "\
Before

<!-- llm-only:start
Inside payload
spread across lines
llm-only:end -->

After";
        let expected = "\
Before

Inside payload
spread across lines

After";
        assert_eq!(unwrap_llm_only(input), expected);
    }

    #[test]
    fn unwrap_handles_wrapping_block_followed_by_two_comment_form() {
        // The two patterns coexist in one file. Both get unwrapped.
        let input = "\
{/* llm-only:start
wrapped payload
llm-only:end */}

<!-- llm-only:start -->
two-comment payload
<!-- llm-only:end -->";
        let expected = "\
wrapped payload

two-comment payload";
        assert_eq!(unwrap_llm_only(input), expected);
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
