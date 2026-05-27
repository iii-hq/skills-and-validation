---
title: "Authoring per-function skill leaves"
description: "Per-function `docs/leaves/<leaf>.md` skill bodies. When an agent should read this guidance, plus the canonical leaf shape."
type: "reference"
---

# Authoring per-function skill leaves

Each registered worker function carries one file at `docs/leaves/<leaf>.md`. The leaf name is the function id's suffix after the last `::`. `textstats::analyze` maps to `docs/leaves/analyze.md`. `auth::list_providers` maps to `docs/leaves/list_providers.md`.

Leaves are inlined under the `## Additional HOWTOs` section of both `README.md` and `skill.md`; there is no separate `skills/<leaf>.md` file or `iii://<worker>/<leaf>` address.

## When to use

- Authoring a new worker and writing the per-function HOWTO bodies the renderer inlines under `## Additional HOWTOs`.
- Updating an existing leaf because the function's behaviour or its realistic call sites changed.
- Reviewing a worker PR to judge whether each leaf body follows the canonical shape.

## Notes

- Each leaf body opens with a topical H1 that names the *task*, not the function. `# Sizing text before provider calls`, not `# textstats::analyze`. The H1 is required: it becomes the `### Title` of the inlined section, and the structure layer flags a leaf that lacks one.
- A `## When to use` section is canonical: three to five bullets covering realistic call sites.
- A `## Notes` section is canonical: gotchas, edge cases, behaviour an agent will trip on.
- Optional `<!-- llm-only:start --> ... <!-- llm-only:end -->` blocks hide content from the published README but include it in the agent-facing `skill.md`.
- The function id is not the H1. The auto-gen system publishes the API surface at a separate URI.
- The function signature is not in the leaf body. Same reason.
- The leaf body does not duplicate the `RegisterFunction::new(...).description("...")` text.
- Cross-references between functions go in prose links, not tabular form.
- When inlining, the renderer demotes every heading in the leaf by two levels (`#` → `###`, `##` → `####`) and runs the audience-appropriate visibility passes (README drops llm-only and reveals human-only; skill.md does the inverse). Beyond that, the body is copied verbatim.
