//! `<!-- llm-only:... -->` block + inline marker handling.
//!
//! Source partials may contain content that should be visible to the AI /
//! skill consumer but hidden from the published README. The renderer wraps
//! the AI-only sections in `<!-- llm-only:start -->` / `<!-- llm-only:end -->`
//! pairs (block form) or `<!-- llm-only: ... -->` (inline form). When
//! producing skill.md / skill artifacts, we strip the block markers and
//! expand the inline form to its inner text. README rendering passes the
//! source through unchanged so the comments stay invisible to humans.

/// Strip `<!-- llm-only:start -->` / `<!-- llm-only:end -->` block markers and
/// expand `<!-- llm-only: ... -->` inline form to its inner text. Use this
/// when rendering for an LLM-facing target (skill.md, skills/*.md, doc skill
/// siblings). For a human-facing target (README.md), pass the source through
/// unchanged.
pub fn unwrap_llm_only(content: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "<!-- llm-only:start -->" || trimmed == "<!-- llm-only:end -->" {
            continue;
        }
        if let Some(inner) = parse_inline_llm_only(trimmed) {
            out.push(inner);
            continue;
        }
        out.push(line.to_string());
    }
    out.join("\n")
}

fn parse_inline_llm_only(trimmed: &str) -> Option<String> {
    let prefix = "<!-- llm-only:";
    let suffix = "-->";
    if !trimmed.starts_with(prefix) || !trimmed.ends_with(suffix) {
        return None;
    }
    if trimmed == "<!-- llm-only:start -->" || trimmed == "<!-- llm-only:end -->" {
        return None;
    }
    let inner = &trimmed[prefix.len()..trimmed.len() - suffix.len()];
    Some(inner.trim().to_string())
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
    fn unwrap_inline_form_replaces_with_inner() {
        let input = "Before\n<!-- llm-only: short note -->\nAfter";
        let expected = "Before\nshort note\nAfter";
        assert_eq!(unwrap_llm_only(input), expected);
    }

    #[test]
    fn unwrap_idempotent_when_no_markers() {
        let input = "Just plain prose\n\nWith no markers.";
        assert_eq!(unwrap_llm_only(input), input);
    }
}
