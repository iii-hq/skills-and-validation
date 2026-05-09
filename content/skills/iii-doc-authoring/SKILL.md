# iii-doc-authoring

How to write Mintlify-shaped `.md` / `.mdx` docs that `iii-skill-check` validates and renders into per-doc skill artifacts (`<source>.skill.md` siblings).

Read individual topics directly via `skillkit read iii-doc-authoring/<topic>`.

## Topics

- [`quickstart`](./quickstart.md) — minimum viable docs project: `.skill-check.yaml`, frontmatter, the verify command.
- [`frontmatter`](./frontmatter.md) — required and optional fields; how `type:` drives voice rules.
- [`types`](./types.md) — Diataxis categories (tutorial, how-to, reference, explanation) and what each one expects.
- [`markers`](./markers.md) — `<!-- skill:... -->` HTML-comment markers for opting docs in/out and including/excluding sections.
- [`voice`](./voice.md) — pointer to `project-rules/voice.md` plus per-type call-outs.
- [`llm-only-blocks`](./llm-only-blocks.md) — when to wrap content visible only to AI consumers.
- [`check`](./check.md) — running `iii-skill-check verify` locally on a docs root.

The runtime Vale config is generated per-run from each doc's frontmatter `type:` — Diataxis rules apply to the right artifacts without depending on directory layout.

This bundle is not itself a worker — there is no `iii.worker.yaml`, no rendering step. The markdown files in this directory are the source of truth.
