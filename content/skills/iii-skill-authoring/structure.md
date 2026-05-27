---
title: 'Worker docs source layout'
description: 'Reference for the docs/ partials a worker keeps and how each one feeds the rendered README and skill.md.'
type: 'reference'
---

# Worker docs source layout

A worker that uses `iii-skill-check` keeps narrative source under `docs/`. The renderer combines those partials with `iii.worker.yaml` (name, description, tags) and `config.yaml` to produce two rendered artifacts: `README.md` and a single self-contained `skill.md`.

## Files the author edits

```
<worker>/
├── iii.worker.yaml          # worker manifest: name (required) + description + tags (recommended) feed the frontmatter; plus the deploy stanza
├── config.yaml              # worker runtime config, rendered verbatim under ## Configuration. Distinct from the engine's config.yaml (located wherever the engine project is).
├── docs/
│   ├── intro.md             # paragraph(s) shown after the H1 in README and skill.md
│   ├── quickstart.md        # body of ## Quickstart in README only
│   ├── companions.md        # appended inside ## Install when this worker pairs with a sibling (optional, README only)
│   ├── migration.md         # body of ## Migration notes (optional, README only)
│   └── leaves/
│       └── <leaf>.md        # inlined under ## Additional HOWTOs in both README and skill.md
```

## Files the renderer produces

```
<worker>/
├── README.md                # published to the registry, rendered on iii.dev
└── skill.md                 # single agent-facing file for iii://<worker> (leaves inlined)
```

There is no `skills/` directory: each leaf's content is inlined into both artifacts under `## Additional HOWTOs`. Always run `iii-skill-render <worker> --write` before committing. The rendered files carry a generated banner and should not be hand-edited.

## Searchable frontmatter

Both `README.md` and `skill.md` open with an identical YAML frontmatter block, sourced entirely from `iii.worker.yaml`. `name` is always present; `description` and `tags` are emitted when set and omitted when absent (a missing one is a structure-layer warning, not an error):

```yaml
---
name: textstats
description: 'Text analysis on the iii bus: word and character counts…'
tags: 'text, analysis, nlp'
---
```

`name`, `description`, and `tags` make the worker indexable and searchable in the registry. The block carries no `llm-only` / `human-only` content (it's identical in both files).

## Slot order in README.md

1. Frontmatter block.
2. Generated banner.
3. `# <name>` (from `iii.worker.yaml.name`).
4. `intro.md`
5. `## Install` + `iii worker add <name>` boilerplate, optionally followed by `companions.md` (no new H2).
6. `## Quickstart` + `quickstart.md`.
7. `## Configuration` + fenced `config.yaml`.
8. `## Migration notes` + `migration.md` (only if present).
9. `## Additional HOWTOs`: each leaf inlined (see below). Omitted when `docs/leaves/` is empty.

## Slot order in skill.md

llm-only blocks are unwrapped; human-only blocks are dropped.

1. Frontmatter block.
2. Generated banner.
3. `# <name>`.
4. `intro.md`
5. `companions.md`
6. `## Additional HOWTOs`: same inlined leaves as the README. Omitted when `docs/leaves/` is empty.

## Inlined leaves under `## Additional HOWTOs`

Leaves are for additional topics, not specific functions. Each `docs/leaves/<leaf>.md` is inlined under the `## Additional HOWTOs` H2, with its headings demoted by two levels so the leaf's own `# Title` becomes an `### Title` and its `## When to use` / `## Notes` become `#### …`.

The leaf author chooses the H1, typically a topical phrase like `# Sizing text before provider calls`, not the function id. A top-level H1 is required (the structure layer flags a leaf without one) because it becomes the `### Title` of the inlined section.
