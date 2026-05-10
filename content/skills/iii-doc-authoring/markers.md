---
title: "Markers"
description: "Reference for the skill: HTML-comment markers that override doc-level scope and filter sections."
type: "reference"
---

# Markers

`<!-- skill:... -->` HTML comments override the include/exclude globs in `.skill-check.yaml` (doc-level) or filter what makes it into the rendered skill (section-level).

## Doc-level

`<!-- skill:include-doc -->`

Pulls this doc into the skill set when the `docs.include` list missed it. Useful for one-off docs in a directory the include patterns don't cover. **Path-based excludes still win** — if `docs.exclude` matches the path, this marker has no effect.

`<!-- skill:exclude-doc -->`

Drops this doc from the skill set even if it matched the include list. Useful for drafts, experimental pages, or pages that aren't ready for AI consumption.

Precedence:

1. `docs.exclude` (path-based) — hard out, beats everything else.
2. `<!-- skill:exclude-doc -->` — beats include-doc + the include list.
3. `<!-- skill:include-doc -->` — beats an include-list miss.
4. `docs.include` — default in/out signal.

Either marker can appear anywhere in the file but conventionally lives near the top. If both appear in the same file the structure layer flags it — pick one.

## Section-level

`<!-- skill:include-section -->`

Includes this heading's section in the rendered skill, regardless of the file-level default.

`<!-- skill:exclude-section -->`

Excludes this heading's section.

A section runs from one heading line to (but not including) the next heading line. The marker can sit on the heading itself or anywhere inside the section before the next heading.

```mdx
## Public API <!-- skill:include-section -->

Goes into the skill.

## Internals <!-- skill:exclude-section -->

Skipped — too implementation-specific for AI consumers.
```

## File-level default

`<!-- skill:include-sections-by-default -->`

(Default if neither marker is present.) Every section is in the skill unless an `exclude-section` marker drops it. Use this when you want most of a doc visible and only a few headings hidden.

`<!-- skill:exclude-sections-by-default -->`

No section is in the skill unless an `include-section` marker keeps it. Use this for reference docs that have lots of internal scaffolding and only a few headings worth surfacing.

## Inside a kept section

The section body still goes through `<!-- llm-only:start -->` / `<!-- llm-only:end -->` unwrapping, so you can keep AI-only context within a section without leaking it to readers of the source.

The renderer drops every `skill:` marker line from the rendered output, including markers that appear on a heading line — the heading text itself stays.
