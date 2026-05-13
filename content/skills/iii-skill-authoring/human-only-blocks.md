---
title: "human-only comment blocks"
description: "Use human-only block markers to hide content from skill.md while keeping it visible in the README."
type: "how-to"
---

# human-only comment blocks

The inverse of `llm-only`. A `human-only:start` / `human-only:end` block lets one source file produce two render targets:

- **README.md target**: the `<!--`/`-->` markers are invisible per CommonMark, but the prose between them renders normally. A human reader on GitHub or iii.dev sees the block as plain text.
- **skill.md / skills/*.md target**: the renderer drops the entire block, both marker lines and the inner body. The agent reading the skill never sees the prose.

Two shapes, mirroring `llm-only`:

- Block: `<!-- human-only:start -->` ... `<!-- human-only:end -->`.
- Inline: `<!-- human-only: short note -->` on a line of its own.

Both comment forms are accepted on every marker:

- HTML form (preferred for `.md` partials).
- MDX form: `{/* human-only:start */}` / `{/* human-only:end */}` / `{/* human-only: short note */}`.

`.mdx` partials should prefer the MDX form so Mintlify doesn't strip the markers before they reach the renderer. Plain `.md` partials accept either. A block opened in one form can close in the other.

## When to use

- Maintainer or contributor heads-up that doesn't help an agent execute the task ("we're removing this wrapper in v3", "ping #infra before changing the worker's image").
- Migration notes that humans need at upgrade time but that would only bloat the agent's context.
- Editorial commentary about why a default is what it is: useful for a reviewer, irrelevant to an agent that just needs to use the API.

## When not to use

- Hiding behaviour that would surprise the agent. If it changes how the agent should call a function, it belongs in the LLM-facing artifact, not a human-only block.
- Storing internal team policy or secrets. The block is still committed to the repo and visible to anyone reading source.
- Replacing voice or structure rules. Both apply to the human-facing render too; a human-only block doesn't suppress validation.

## Pairing with llm-only

A single source can carry three audiences cleanly:

- Plain prose: both the README and the skill artifact include it.
- `llm-only` block: only the skill artifact.
- `human-only` block: only the README.

The two block types compose: a `human-only` block inside an `llm-only` block (or vice versa) still gets dropped by the inner rule, because each pass operates over the same line stream.

## Inline form

`<!-- human-only: short note -->` on its own line expands to its payload in the rendered README and is dropped entirely from `skill.md` / `skills/*.md`. Worker mode runs the README through `unwrap_human_only`, so the payload becomes visible prose; the skill render path drops the line.

The inline form must sit on a line of its own. Embedded mid-paragraph human-only comments are not parsed.

## Validation

The structure layer of `iii-skill-check verify` enforces that every `human-only:start` marker has a matching `human-only:end` marker on its own line. The balance check counts HTML and MDX forms together, so a file can open in one form and close in the other and still balance. Unbalanced blocks fail the layer.
