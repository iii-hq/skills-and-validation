//! Docs-mode validation surface.
//!
//! Where worker mode renders a fixed bundle of partials into
//! `README.md` + `skill.md` + `skills/*.md`, docs mode walks a docs root
//! (typically a Mintlify project) and produces one `<source>.skill.md`
//! sibling per in-scope `.md` / `.mdx` doc. Each artifact is a stripped-
//! down version of the source: frontmatter removed, llm-only blocks
//! unwrapped, sections kept or dropped per the `<!-- skill:... -->`
//! HTML-comment markers in the source.
//!
//! The split between `frontmatter`, `markers`, `render`, `enumerate`, and
//! `check_rendered` mirrors the pipeline order.

pub mod check_rendered;
pub mod enumerate;
pub mod frontmatter;
pub mod markers;
pub mod render;
pub mod structure;
pub mod vale_config;
