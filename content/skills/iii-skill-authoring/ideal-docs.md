---
title: "ideal-docs/project-rules is canonical"
description: "Why the rules under content/project-rules/ are a vendored snapshot of iii-hq/ideal-docs/project-rules/."
type: "explanation"
---

# `ideal-docs/project-rules` is canonical

The rules `iii-skill-check` enforces live under `content/project-rules/` (vendored snapshot) and `content/styles/` (vendored Vale styles). The canonical source for both is `iii-hq/ideal-docs/project-rules/` and `iii-hq/ideal-docs/styles/`.

When the rules conflict with an existing worker README or skill file, the rules win. Never copy phrasing or structure from a sibling worker. There is no guarantee that worker is rules-compliant. Drift accumulates faster than rules-update PRs land.

## When the snapshot is stale

Refresh by hand from a local `ideal-docs/` clone:

```bash
cp /path/to/ideal-docs/project-rules/*.md content/project-rules/
cp -R /path/to/ideal-docs/styles/Diataxis content/styles/
cp -R /path/to/ideal-docs/styles/Terminology content/styles/
```

`content/project-rules/SOURCE.md` records the upstream pointer and the planned migration to a `git`-source mode that pulls directly from `iii-hq/iii` with shallow + sparse checkout.

## When you disagree with a rule

The rules are not the place to push back during a worker PR. Open an issue or PR against `ideal-docs` instead. Once it lands there, the next snapshot refresh propagates the change everywhere.
