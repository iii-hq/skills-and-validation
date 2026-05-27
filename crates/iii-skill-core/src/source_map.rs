//! Translate violations from rendered artifacts back to the source files
//! a human actually edits.
//!
//! The renderer composes `.skill.md` artifacts from one or more source
//! partials, but those rendered files are read-only-by-convention (they
//! carry `DO NOT EDIT` headers). Violations the layers emit cite the
//! rendered path + the rendered line — useful for inspection, useless
//! for editing. This module maps both back to the source.
//!
//! ## Layout
//!
//! Two pieces:
//!
//! 1. [`candidates`] enumerates the source path(s) that compose a given
//!    rendered artifact:
//!    - Docs mode: `<src>.mdx.skill.md` → `<src>.mdx` (1:1)
//!    - Worker README/skill.md: composed from `<worker>/docs/{intro,quickstart,companions}.md` plus every `docs/leaves/*.md` (inlined under ## Additional HOWTOs) (1:N)
//!
//! 2. [`translate`] reads the offending line text from the rendered file,
//!    grep-equivalents it against each candidate source, and returns the
//!    first exact match it finds. The renderer copies partial bodies
//!    verbatim, so an exact-line match is almost always unambiguous in
//!    practice. Blank lines and lines shorter than three non-whitespace
//!    characters are skipped (too ambiguous to anchor).
//!
//! ## "Approximate"
//!
//! Even when [`translate`] finds an exact match, the returned line is
//! labeled approximate at the presentation layer (`~N`). Two reasons:
//!
//! - The renderer may concatenate partials in an order that places
//!   duplicate-text instances near each other; we pick the first match
//!   deterministically.
//! - If no source match exists (renderer-inserted header, mangled line),
//!   we still surface the source path but reuse the rendered line as a
//!   best-guess anchor; the user gets a path to open and a nearby line
//!   to scan from.

use std::path::{Path, PathBuf};

/// Source-file candidates that could have produced a given rendered
/// artifact. Returns paths in the order they're searched (intro,
/// quickstart, companions, then leaves alphabetically).
pub fn candidates(rendered: &Path) -> Vec<PathBuf> {
    let s = rendered.to_string_lossy();

    // Docs mode: <src>.mdx.skill.md → <src>.mdx
    //            <src>.md.skill.md  → <src>.md
    if let Some(stripped) = s.strip_suffix(".skill.md") {
        return vec![PathBuf::from(stripped.to_string())];
    }

    // Worker README/skill.md: composed from docs/{intro,quickstart,companions}.md
    // plus every docs/leaves/*.md (leaves are inlined under ## Additional
    // HOWTOs). Order matters — return partials in the same order the renderer
    // emits them so the first match is also the most natural one.
    //
    // Gate this on the parent dir actually being a worker (has an
    // iii.worker.yaml manifest OR a docs/intro.md partial). Without
    // this gate, the repo's root-level README.md would be misidentified
    // as a worker README and remapped to a non-existent docs/intro.md.
    if let Some(file_name) = rendered.file_name().and_then(|n| n.to_str()) {
        if matches!(file_name, "README.md" | "skill.md") {
            if let Some(worker) = rendered.parent() {
                let manifest = worker.join("iii.worker.yaml");
                let intro = worker.join("docs").join("intro.md");
                if manifest.exists() || intro.exists() {
                    let docs = worker.join("docs");
                    let mut cands = vec![
                        docs.join("intro.md"),
                        docs.join("quickstart.md"),
                        docs.join("companions.md"),
                    ];
                    let leaves = docs.join("leaves");
                    if let Ok(entries) = std::fs::read_dir(&leaves) {
                        let mut leaf_paths: Vec<PathBuf> =
                            entries.flatten().map(|e| e.path()).collect();
                        leaf_paths.sort();
                        cands.extend(leaf_paths);
                    }
                    return cands;
                }
            }
        }
    }

    Vec::new()
}

/// Translate `(rendered, rendered_line)` to `(source_path, source_line)`.
///
/// Algorithm:
///
/// 1. Determine [`candidates`] from the rendered path. No candidates →
///    return `None` (caller should display the rendered path unchanged).
/// 2. Read line `rendered_line` from `rendered`. If the file or line is
///    unreadable, return `(first_candidate, rendered_line)` so at least
///    the source path is surfaced.
/// 3. If the target line is blank or too short to anchor (<3 non-ws
///    chars), return `(first_candidate, rendered_line)` — we can't
///    reliably anchor, but the user still gets the source path.
/// 4. Search candidates in order for an exact-line match against the
///    target line's trimmed content. First match wins.
/// 5. No match → `(first_candidate, rendered_line)` (best-effort path,
///    rendered line as a scan anchor).
pub fn translate(rendered: &Path, rendered_line: usize) -> Option<(PathBuf, usize)> {
    let cands = candidates(rendered);
    if cands.is_empty() {
        return None;
    }
    let first = cands[0].clone();

    let rendered_text = match std::fs::read_to_string(rendered) {
        Ok(s) => s,
        Err(_) => return Some((first, rendered_line)),
    };
    let lines: Vec<&str> = rendered_text.lines().collect();
    if rendered_line == 0 || rendered_line > lines.len() {
        return Some((first, rendered_line));
    }
    let target = lines[rendered_line - 1].trim_end();
    if target.trim().len() < 3 {
        // Blank lines, lone punctuation, single chars — too ambiguous.
        return Some((first, rendered_line));
    }

    for cand in &cands {
        if let Ok(src) = std::fs::read_to_string(cand) {
            for (i, src_line) in src.lines().enumerate() {
                if src_line.trim_end() == target {
                    return Some((cand.clone(), i + 1));
                }
            }
        }
    }

    Some((first, rendered_line))
}
