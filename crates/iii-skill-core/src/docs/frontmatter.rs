//! YAML frontmatter parsing for `.md` / `.mdx` doc sources.

use anyhow::{anyhow, Context};
use serde::Deserialize;
use std::fmt;

/// Diataxis doc category. Drives type-aware Vale rule selection and the
/// AI-layer system prompt's per-artifact context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocType {
    Tutorial,
    HowTo,
    Reference,
    Explanation,
}

impl DocType {
    /// Stable kebab-case identifier — used for path slugs in the runtime
    /// Vale config and for the AI prompt's type hint.
    pub fn slug(self) -> &'static str {
        match self {
            DocType::Tutorial => "tutorial",
            DocType::HowTo => "how-to",
            DocType::Reference => "reference",
            DocType::Explanation => "explanation",
        }
    }
}

impl fmt::Display for DocType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

/// Required frontmatter fields. `owner` is optional — some docs don't have
/// a single team responsible.
#[derive(Debug, Deserialize)]
pub struct Frontmatter {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(rename = "type")]
    pub doc_type: DocType,
}

/// Result of splitting frontmatter from body. The body still contains any
/// HTML-comment markers (`<!-- skill:... -->`) — the marker scanner runs
/// over the body separately.
#[derive(Debug)]
pub struct ParsedDoc {
    pub frontmatter: Frontmatter,
    pub body: String,
}

/// Split `--- yaml --- body` into a structured `Frontmatter` + the
/// remaining body. Errors when the header is missing, malformed, or any
/// required field is absent.
///
/// Mintlify accepts both `---` and `---\n...\n---` framing; we require the
/// closing `---` on its own line for unambiguous extraction.
pub fn parse(content: &str) -> anyhow::Result<ParsedDoc> {
    let trimmed_start = content.trim_start_matches('\u{FEFF}'); // strip BOM if present
    if !trimmed_start.starts_with("---") {
        anyhow::bail!("missing frontmatter header (expected leading `---`)");
    }
    // Skip the opening fence (and the newline after it).
    let after_open = trimmed_start
        .strip_prefix("---")
        .ok_or_else(|| anyhow!("frontmatter prefix mismatch"))?
        .trim_start_matches('\n');

    // Find the closing fence: a line containing only `---`.
    let mut yaml_end: Option<usize> = None;
    for (idx, line) in after_open.split_inclusive('\n').scan(0usize, |off, l| {
        let cur = *off;
        *off += l.len();
        Some((cur, l))
    }) {
        if line.trim_end_matches('\n').trim() == "---" {
            yaml_end = Some(idx);
            break;
        }
    }
    let yaml_end = yaml_end
        .ok_or_else(|| anyhow!("frontmatter close `---` not found"))?;

    let yaml_body = &after_open[..yaml_end];
    let frontmatter: Frontmatter = serde_yaml::from_str(yaml_body)
        .context("parsing frontmatter YAML")?;

    if frontmatter.title.trim().is_empty() {
        anyhow::bail!("frontmatter `title` must not be empty");
    }
    if frontmatter.description.trim().is_empty() {
        anyhow::bail!("frontmatter `description` must not be empty");
    }

    // Body = everything past the closing `---` line (skip the line itself).
    let after_yaml = &after_open[yaml_end..];
    let body = after_yaml
        .splitn(2, '\n')
        .nth(1)
        .unwrap_or("")
        .to_string();

    Ok(ParsedDoc { frontmatter, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_frontmatter() {
        let input = "---\ntitle: \"Foo\"\ndescription: \"A foo doc\"\nowner: \"devrel\"\ntype: \"how-to\"\n---\n\nBody starts here.\n";
        let parsed = parse(input).unwrap();
        assert_eq!(parsed.frontmatter.title, "Foo");
        assert_eq!(parsed.frontmatter.description, "A foo doc");
        assert_eq!(parsed.frontmatter.owner.as_deref(), Some("devrel"));
        assert_eq!(parsed.frontmatter.doc_type, DocType::HowTo);
        assert!(parsed.body.starts_with("\nBody starts here."));
    }

    #[test]
    fn owner_is_optional() {
        let input = "---\ntitle: t\ndescription: d\ntype: tutorial\n---\nbody\n";
        let parsed = parse(input).unwrap();
        assert!(parsed.frontmatter.owner.is_none());
        assert_eq!(parsed.frontmatter.doc_type, DocType::Tutorial);
    }

    #[test]
    fn rejects_missing_required_field() {
        let input = "---\ntitle: t\ntype: reference\n---\n";
        let result = parse(input);
        assert!(result.is_err(), "missing description should error");
    }

    #[test]
    fn rejects_unknown_type() {
        let input = "---\ntitle: t\ndescription: d\ntype: walkthrough\n---\n";
        let result = parse(input);
        assert!(result.is_err(), "unknown type should error");
    }

    #[test]
    fn rejects_empty_required_string() {
        let input = "---\ntitle: \"\"\ndescription: d\ntype: explanation\n---\n";
        let result = parse(input);
        assert!(result.is_err(), "empty title should error");
    }

    #[test]
    fn rejects_missing_frontmatter_header() {
        let input = "# Just markdown, no frontmatter\n";
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unclosed_frontmatter() {
        let input = "---\ntitle: t\ndescription: d\ntype: reference\nno closing fence\n";
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn slug_round_trips() {
        for (s, ty) in [
            ("tutorial", DocType::Tutorial),
            ("how-to", DocType::HowTo),
            ("reference", DocType::Reference),
            ("explanation", DocType::Explanation),
        ] {
            assert_eq!(ty.slug(), s);
        }
    }
}
