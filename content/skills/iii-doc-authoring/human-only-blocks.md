---
title: "human-only comment blocks"
description: "Use human-only block markers to hide content from the rendered skill artifact while keeping it visible on the docs site."
type: "how-to"
---

# human-only comment blocks

The inverse of `llm-only`. A `human-only:start` / `human-only:end` block lets one docs source produce two render targets:

- **Published page (Mintlify)**: the `<!--`/`-->` or `{/*`/`*/}` markers are invisible per CommonMark and Mintlify's MDX comment handling, but the prose between them renders normally. A reader on the docs site sees the block as plain text.
- **`<source>.skill.md` artifact**: the renderer drops the entire block, both marker lines and the inner body. The agent loading the skill never sees the prose.

Two shapes, mirroring `llm-only`:

- Block: `<!-- human-only:start -->` ... `<!-- human-only:end -->`.
- Inline: `<!-- human-only: short note -->` on a line of its own.

Both comment forms are accepted on every marker:

- HTML form (for `.md` files).
- MDX form: `{/* human-only:start */}` / `{/* human-only:end */}` / `{/* human-only: short note */}` (preferred for `.mdx` files).

**Inline-form caveat in docs mode:** Mintlify reads the doc source directly. No renderer pass runs between the source and Mintlify, so an inline `<!-- human-only: text -->` stays as an invisible comment on the published page; the payload never expands into visible prose. The block form works as expected. Use the block form when you want the docs-site reader to see the content; reserve the inline form for notes the LLM should not see in the rendered `<source>.skill.md` artifact.

`.mdx` files should prefer the MDX form so Mintlify doesn't strip the markers before they reach the renderer. Plain `.md` files accept either. A block opened in one form can close in the other.

## When to use

- Editorial asides directed at the reader ("we're tracking the legacy auth flow in [LIN-1234]"), where the agent doesn't need the context.
- Migration timelines, deprecation calendars, release dates: useful to a human at scan time, noise to an agent answering a how-to.
- Internal cross-links to runbooks or postmortems that are accessible only to teammates.

## When not to use

- Hiding API details an agent needs. If it affects how the agent should call a function, it belongs in the LLM-facing artifact.
- Replacing the `skill:exclude-section` / `skill:exclude-doc` markers. Those control whether a doc or section enters the skill set at all; `human-only` controls which spans inside an in-scope source survive into the rendered artifact.
- Storing secrets. The block is committed to the source repo.

## Pairing with llm-only

A single doc can carry three audiences cleanly:

- Plain prose: both the published page and the skill artifact include it.
- `llm-only` block: only the skill artifact.
- `human-only` block: only the published page.

## Validation

The docs-mode structure layer of `iii-skill-check verify` enforces that every `human-only:start` marker has a matching `human-only:end` marker on its own line. The balance check counts HTML and MDX forms together, so a file can open in one form and close in the other and still balance. Unbalanced blocks fail the layer.
