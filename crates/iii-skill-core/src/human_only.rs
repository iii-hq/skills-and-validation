//! `human-only:...` block + inline marker handling.
//!
//! Inverse of [`crate::llm_only`]: spans that should be visible to the
//! human reader (worker `README.md`, Mintlify-rendered docs source) but
//! hidden from every LLM-facing artifact (`skill.md`, `skills/*.md`,
//! `<source>.skill.md`). Two equivalent comment forms are accepted on
//! every marker:
//!
//! - HTML form: `<!-- human-only:start -->`, `<!-- human-only:end -->`,
//!   `<!-- human-only: inline note -->`.
//! - MDX form: `{/* human-only:start */}`, `{/* human-only:end */}`,
//!   `{/* human-only: inline note */}`.
//!
//! `.mdx` files strip HTML comments at render time, so partials authored
//! in MDX should use the MDX form. Plain `.md` partials accept either.
//! Both forms can coexist in the same file; a block opened in one form
//! can close in the other and still parse.
//!
//! ## Authoring pattern (MDX)
//!
//! For `human-only`, use the **two-comment form** in MDX:
//!
//! ```mdx
//! {/* human-only:start */}
//! Maintainer-facing note that should render to humans.
//! {/* human-only:end */}
//! ```
//!
//! Mintlify treats each `{/* ... */}` as an invisible comment and renders
//! the prose between them — exactly what `human-only` wants. The
//! multi-line **wrapping form** (`{/* human-only:start ... end */}`) makes
//! Mintlify hide the entire block from humans too, which defeats the
//! tag's purpose. The parser still accepts the wrapping form so the
//! LLM-render path can drop it defensively if an author uses it by
//! mistake, but authors should prefer the two-comment form in MDX.
//!
//! Two complementary passes:
//!
//! - [`unwrap_human_only`]: used when producing the human-facing render
//!   (worker `README.md`). Strips block markers (keeping the inner body)
//!   and expands the inline form to its payload so humans see the prose.
//! - [`strip_human_only`]: used when producing the LLM-facing artifact.
//!   Drops every `human-only:start ... :end` block (markers and body)
//!   and every inline `human-only: ...` line.

/// Strip `human-only:start` / `human-only:end` block markers and expand
/// the `human-only: ...` inline form to its inner text. Recognizes:
///
/// - Single-line markers (two-comment form): markers vanish, body kept.
/// - Inline form (`<!-- human-only: note -->`): expands to the trailing
///   text.
/// - Multi-line wrapping form (one comment containing start + body + end):
///   markers vanish, body kept. Note this form is wrong-by-intent for
///   `human-only` in MDX (Mintlify renders the whole comment as nothing),
///   but the parser accepts it for symmetry with [`strip_human_only`].
///
/// Use when rendering for a human-facing target.
pub fn unwrap_human_only(content: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut in_wrapping_block = false;
    for line in content.lines() {
        let trimmed = line.trim();

        if in_wrapping_block {
            if line_closes_wrapping_block(trimmed) {
                in_wrapping_block = false;
                continue;
            }
            out.push(line.to_string());
            continue;
        }

        if let Some(inner) = strip_comment(trimmed) {
            let inner = inner.trim();
            if inner == "human-only:start" || inner == "human-only:end" {
                continue;
            }
            if let Some(payload) = inner.strip_prefix("human-only:") {
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

/// Drop every `human-only:start` ... `human-only:end` block (markers and
/// body included) and every inline `human-only: ...` line from `content`.
/// Recognizes both the two-comment form and the multi-line wrapping form;
/// either way, LLM consumers never see the payload. Returns the source
/// unchanged when no markers are present.
pub fn strip_human_only(content: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut inside_two_comment = false;
    let mut inside_wrapping = false;
    for line in content.lines() {
        let trimmed = line.trim();

        if inside_wrapping {
            if line_closes_wrapping_block(trimmed) {
                inside_wrapping = false;
            }
            continue;
        }
        if inside_two_comment {
            if is_human_only_end(line) {
                inside_two_comment = false;
            }
            continue;
        }
        if is_human_only_start(line) {
            inside_two_comment = true;
            continue;
        }
        if line_opens_wrapping_block(trimmed) {
            inside_wrapping = true;
            continue;
        }
        if is_inline_human_only(line) {
            continue;
        }
        out.push(line.to_string());
    }
    out.join("\n")
}

/// True iff the trimmed line opens a multi-line wrapping comment whose
/// opener contains `human-only:start` and which is *not* closed on the
/// same line.
fn line_opens_wrapping_block(trimmed: &str) -> bool {
    if trimmed.starts_with("{/*") && !trimmed.contains("*/}") {
        return trimmed.contains("human-only:start");
    }
    if trimmed.starts_with("<!--") && !trimmed.contains("-->") {
        return trimmed.contains("human-only:start");
    }
    false
}

/// True iff the trimmed line closes a multi-line wrapping comment whose
/// closer contains `human-only:end` followed by `*/}` (MDX) or `-->` (HTML).
fn line_closes_wrapping_block(trimmed: &str) -> bool {
    if !trimmed.contains("human-only:end") {
        return false;
    }
    trimmed.ends_with("*/}") || trimmed.ends_with("-->")
}

/// True iff `line` (trimmed) is an inline `human-only: payload` comment
/// with no surrounding content. Block start/end markers do not count.
fn is_inline_human_only(line: &str) -> bool {
    let Some(inner) = strip_comment(line.trim()) else {
        return false;
    };
    let inner = inner.trim();
    if inner == "human-only:start" || inner == "human-only:end" {
        return false;
    }
    inner.strip_prefix("human-only:").is_some()
}

/// True iff `line` (trimmed) is exactly a `human-only:start` marker in
/// either comment form.
pub fn is_human_only_start(line: &str) -> bool {
    strip_comment(line.trim()).is_some_and(|inner| inner.trim() == "human-only:start")
}

/// True iff `line` (trimmed) is exactly a `human-only:end` marker in
/// either comment form.
pub fn is_human_only_end(line: &str) -> bool {
    strip_comment(line.trim()).is_some_and(|inner| inner.trim() == "human-only:end")
}

/// If `trimmed` is a single comment with no surrounding content, return
/// the inner directive text. Handles both `<!-- ... -->` and `{/* ... */}`.
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
    fn strip_drops_html_block() {
        let input =
            "Before\n\n<!-- human-only:start -->\nHumans only.\n<!-- human-only:end -->\n\nAfter";
        let expected = "Before\n\n\nAfter";
        assert_eq!(strip_human_only(input), expected);
    }

    #[test]
    fn strip_drops_mdx_block() {
        let input =
            "Before\n\n{/* human-only:start */}\nHumans only.\n{/* human-only:end */}\n\nAfter";
        let expected = "Before\n\n\nAfter";
        assert_eq!(strip_human_only(input), expected);
    }

    #[test]
    fn strip_handles_mixed_open_close_forms() {
        let input =
            "Before\n<!-- human-only:start -->\nA\nB\n{/* human-only:end */}\nAfter";
        let expected = "Before\nAfter";
        assert_eq!(strip_human_only(input), expected);
    }

    #[test]
    fn strip_handles_multiple_blocks() {
        let input = "<!-- human-only:start -->\nA\n<!-- human-only:end -->\nmid\n{/* human-only:start */}\nB\n{/* human-only:end */}";
        let expected = "mid";
        assert_eq!(strip_human_only(input), expected);
    }

    #[test]
    fn strip_idempotent_when_no_markers() {
        let input = "Just plain prose\n\nWith no markers.";
        assert_eq!(strip_human_only(input), input);
    }

    #[test]
    fn strip_drops_inline_html() {
        let input = "Before\n<!-- human-only: maintainer note -->\nAfter";
        let expected = "Before\nAfter";
        assert_eq!(strip_human_only(input), expected);
    }

    #[test]
    fn strip_drops_inline_mdx() {
        let input = "Before\n{/* human-only: maintainer note */}\nAfter";
        let expected = "Before\nAfter";
        assert_eq!(strip_human_only(input), expected);
    }

    #[test]
    fn unwrap_drops_block_markers_keeps_body() {
        let input = "Before\n<!-- human-only:start -->\nInside.\n<!-- human-only:end -->\nAfter";
        let expected = "Before\nInside.\nAfter";
        assert_eq!(unwrap_human_only(input), expected);
    }

    #[test]
    fn unwrap_drops_mdx_block_markers_keeps_body() {
        let input = "Before\n{/* human-only:start */}\nInside.\n{/* human-only:end */}\nAfter";
        let expected = "Before\nInside.\nAfter";
        assert_eq!(unwrap_human_only(input), expected);
    }

    #[test]
    fn unwrap_inline_form_replaces_with_inner() {
        let input = "Before\n<!-- human-only: short note -->\nAfter";
        let expected = "Before\nshort note\nAfter";
        assert_eq!(unwrap_human_only(input), expected);
    }

    #[test]
    fn unwrap_mdx_inline_form_replaces_with_inner() {
        let input = "Before\n{/* human-only: short note */}\nAfter";
        let expected = "Before\nshort note\nAfter";
        assert_eq!(unwrap_human_only(input), expected);
    }

    #[test]
    fn unwrap_idempotent_when_no_markers() {
        let input = "Just plain prose\n\nWith no markers.";
        assert_eq!(unwrap_human_only(input), input);
    }

    #[test]
    fn strip_drops_mdx_wrapping_block() {
        // Wrapping form for human-only is not the recommended pattern
        // (Mintlify would hide it from humans too), but the LLM strip path
        // must still drop it so the payload never reaches the model.
        let input = "\
Before

{/* human-only:start
Maintainer note that should never reach the LLM
llm-trap line
human-only:end */}

After";
        let expected = "Before\n\n\nAfter";
        assert_eq!(strip_human_only(input), expected);
    }

    #[test]
    fn strip_drops_html_wrapping_block() {
        let input = "\
Before

<!-- human-only:start
Maintainer note
human-only:end -->

After";
        let expected = "Before\n\n\nAfter";
        assert_eq!(strip_human_only(input), expected);
    }

    #[test]
    fn unwrap_drops_mdx_wrapping_block_markers_keeps_body() {
        let input = "\
Before

{/* human-only:start
Inside payload
human-only:end */}

After";
        let expected = "\
Before

Inside payload

After";
        assert_eq!(unwrap_human_only(input), expected);
    }

    #[test]
    fn marker_predicates_recognise_both_forms() {
        assert!(is_human_only_start("<!-- human-only:start -->"));
        assert!(is_human_only_start("{/* human-only:start */}"));
        assert!(is_human_only_end("<!-- human-only:end -->"));
        assert!(is_human_only_end("{/* human-only:end */}"));
        assert!(!is_human_only_start("<!-- human-only:end -->"));
        assert!(!is_human_only_end("plain text"));
        assert!(!is_human_only_start("<!-- llm-only:start -->"));
    }
}
