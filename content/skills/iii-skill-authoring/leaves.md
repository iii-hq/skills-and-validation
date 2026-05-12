---
title: "Authoring per-function skill leaves"
description: "Per-function `docs/leaves/<leaf>.md` skill bodies. When an agent should read this guidance, plus the canonical leaf shape."
type: "reference"
---

# Authoring per-function skill leaves

Each registered worker function carries one file at `docs/leaves/<leaf>.md`. The leaf name is the function id's suffix after the last `::`. `textstats::analyze` maps to `docs/leaves/analyze.md`. `auth::list_providers` maps to `docs/leaves/list_providers.md`.

## When to use

- Authoring a new worker and writing the per-function skill bodies the renderer publishes as `skills/<leaf>.md`.
- Updating an existing leaf because the function's behaviour or its realistic call sites changed.
- Reviewing a worker PR to judge whether each leaf body follows the canonical shape.

## Notes

- Each leaf body opens with a topical H1 (optional) that names the *task*, not the function. `# Sizing text before provider calls`, not `# textstats::analyze`.
- A `## When to use` section is canonical: three to five bullets covering realistic call sites.
- A `## Notes` section is canonical: gotchas, edge cases, behaviour an agent will trip on.
- Optional `<!-- llm-only:start --> ... <!-- llm-only:end -->` blocks hide content from the published page but include it in the agent-facing skill artifact.
- The function id is not the H1. The auto-gen system publishes the API surface at a separate URI.
- The function signature is not in the leaf body. Same reason.
- The leaf body does not duplicate the `RegisterFunction::new(...).description("...")` text.
- Cross-references between functions go in prose links, not tabular form.
- The renderer prepends a generated banner comment to each leaf and unwraps any llm-only blocks before writing `skills/<leaf>.md`. Beyond that, the file is copied verbatim.
