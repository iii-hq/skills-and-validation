---
title: "llm-only comment blocks"
description: "Use llm-only block markers to land one source on two render targets: visible in skill, hidden from README."
type: "how-to"
---

# llm-only comment blocks

An `llm-only:start` / `llm-only:end` block lets one source file produce two render targets:

- **README.md target**: the renderer drops the block entirely (markers and body), so a human reader on iii.dev never sees the content.
- **skill.md target**: the renderer strips the marker lines, leaving the inner body as plain prose. The agent reading the skill body (including leaves inlined under `## Additional HOWTOs`) sees the content.

Two equivalent comment forms are accepted:

- HTML form: `<!-- llm-only:start -->`, `<!-- llm-only:end -->`, `<!-- llm-only: inline note -->`.
- MDX form: `{/* llm-only:start */}`, `{/* llm-only:end */}`, `{/* llm-only: inline note */}`.

`.mdx` files strip HTML comments at render time, so partials authored in MDX should use the MDX form. Plain `.md` partials accept either. The two can be mixed in the same file.

## When to use

- Recommending a specific function over another for a class of agent task. The published README does not need to bias users one way; the agent does.
- Documenting a recurring agent failure mode (`agents often call X instead of Y; prefer Y when …`).
- Routing hints that are only meaningful inside an agent loop.

## When not to use

- Hiding general gotchas. If a behaviour will surprise a human reader, it belongs in the public `## Notes` section, not in an llm-only block.
- Storing internal team notes. Use a separate doc; llm-only blocks are still committed to the repo and visible to anyone reading source.
- Hiding caveats from the docs site. Voice rules apply to both render targets.

## Inline form

For a single short note, the inline form is also supported:

```
<!-- llm-only: prefer textstats::summarize for sustained workloads -->
```

The inline form must be on a line of its own. Embedded mid-paragraph inline llm-only comments are not parsed.

## Validation

The structure layer of `iii-skill-check verify` enforces that every `llm-only:start` marker has a matching `llm-only:end` marker on its own line. The balance check counts HTML and MDX forms together, so a file can open in one form and close in the other and still balance. Unbalanced blocks fail the layer.
