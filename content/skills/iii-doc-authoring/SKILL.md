---
title: "iii-doc-authoring"
description: "Skill bundle index for authoring Mintlify-shaped docs that iii-skill-check validates."
type: "reference"
---

# iii-doc-authoring

How to write Mintlify-shaped `.md` / `.mdx` docs that `iii-skill-check` validates and renders into per-doc skill artifacts (`<source>.skill.md` siblings).

Read individual topics directly via `skillkit read iii-doc-authoring/<topic>`.

## Topics

- [`quickstart`](./quickstart.md): minimum viable docs project: `.skill-check.yaml`, frontmatter, the verify command.
- [`frontmatter`](./frontmatter.md): required and optional fields; how `type:` drives voice rules.
- [`types`](./types.md): short summary of which Vale and AI rules each Diataxis category enables.
- [`markers`](./markers.md): `<!-- skill:... -->` HTML-comment markers for opting docs in/out and including/excluding sections.
- [`voice`](./voice.md): pointer to `project-rules/voice.md` plus per-type call-outs.
- [`llm-only-blocks`](./llm-only-blocks.md): when to wrap content visible only to AI consumers.
- [`check`](./check.md): running `iii-skill-check verify` locally on a docs root.

### Authoring guides per Diataxis quadrant

These five files are skillkit-shape skills (their frontmatter uses `name:` + `description:` instead of the Mintlify shape the validator looks for, and they're excluded from the docs-mode pipeline). Load `doc_workflow` once for any docs work, plus the quadrant-specific file that matches the page you're writing.

- [`diataxis/doc_workflow`](./diataxis/doc_workflow.md): global tone, component usage, quadrant integrity, structural rules. Always loaded.
- [`diataxis/doc_tutorial`](./diataxis/doc_tutorial.md): learning-oriented authoring rules.
- [`diataxis/doc_howto`](./diataxis/doc_howto.md): problem-oriented authoring rules.
- [`diataxis/doc_reference`](./diataxis/doc_reference.md): information-oriented authoring rules.
- [`diataxis/doc_explanation`](./diataxis/doc_explanation.md): understanding-oriented authoring rules.

The runtime Vale config is generated per-run from each doc's frontmatter `type:`. Diataxis rules apply to the right artifacts without depending on directory layout.

This bundle is not itself a worker. There is no `iii.worker.yaml`, no rendering step. The markdown files in this directory are the source of truth.
