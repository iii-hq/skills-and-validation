# Frontmatter

Every in-scope doc starts with a YAML frontmatter block. Anything past the closing `---` is the body.

## Required

- `title` — string. The doc's headline. Used as the rendered `# H1` if the body doesn't already have one.
- `description` — string. One-sentence summary; surfaces in search results and the sticky PR comment when violations land.
- `type` — string, one of `tutorial`, `how-to`, `reference`, `explanation`. See [`types`](./types.md).

Empty strings are rejected. Unknown `type` values fail the structure layer.

## Optional

- `owner` — string. Team or person responsible for keeping the doc accurate.

## Why `type` matters

The validator generates a Vale config per run that maps each doc's path to a Diataxis ruleset based on `type`. A `tutorial` allows phrases that would be marked as voice violations in a `reference` (and vice versa). The AI layer also receives the type as a per-artifact hint so its system prompt applies the right voice expectations.

If you change a doc's category, change its `type:` — never split the difference by leaving the old type and rewording.

## Example

```mdx
---
title: "Configure the iii engine"
description: "Engine options that change how workers are scheduled and how data flows through the bus."
owner: "platform"
type: "reference"
---

The engine reads `~/.iii/config.yaml` on startup …
```

The renderer strips the frontmatter from the rendered `<source>.skill.md` artifact.
