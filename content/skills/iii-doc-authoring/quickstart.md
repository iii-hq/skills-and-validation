---
title: "Quickstart"
description: "Get from a Mintlify docs site to a validated skill set."
type: "how-to"
---

# Quickstart

Get from a Mintlify docs site to a validated skill set.

## 1. Add `.skill-check.yaml` at the docs root

```yaml
version: 2
mode: docs
docs:
  include:
    - "**/*.md"
    - "**/*.mdx"
  exclude:
    - "**/CHANGELOG.md"
    - "**/draft.*"
ai_check:
  provider: anthropic
  model: claude-opus-4-7
  api_key_env_var: ANTHROPIC_API_KEY
  max_tokens: 6000
```

`include` and `exclude` are evaluated against paths relative to the docs root. `**` matches any depth; brace expansion is not supported, so list `**/*.md` and `**/*.mdx` separately.

## 2. Add frontmatter to every in-scope doc

```mdx
---
title: "Build a real-time todo app"
description: "Step-by-step build of a real-time todo using iii streams."
owner: "devrel"
type: "tutorial"
---

Lead paragraph.

# Build a real-time todo app

…
```

Required fields: `title`, `description`, `type`. Optional: `owner`. `type` must be one of `tutorial`, `how-to`, `reference`, `explanation` (see [`types`](./types.md)).

## 3. Render and validate

```bash
iii-skill-render <docs-root> --write     # writes <source>.skill.md per in-scope doc
iii-skill-check verify <docs-root>       # structure + vale + ai
```

The renderer writes one `<source>.skill.md` sibling per matched doc. Commit those alongside the sources; CI's `verify-rendered` flags drift between them and the source.

## 4. Plug into CI

Adding the GitHub Actions workflow is required to wire this validation into a consumer repository. The canonical example ships with the install at `content/github_workflows_example.yml`. Copy it into the consumer repo:

```bash
cp ~/.local/share/skill-check/current/content/github_workflows_example.yml \
   .github/workflows/skill-check.yml
```

Adjust the `uses:` ref to a pinned tag, then add overrides as needed:

```yaml
- uses: iii-hq/skills-and-validation@v0.2
  with:
    docs-glob: "**/*.md **/*.mdx"
    write: true
    anthropic-api-key: ${{ secrets.ANTHROPIC_API_KEY }}
```

The action auto-detects the mode from `.skill-check.yaml` at the workspace root (override the path with `config-path` for repos that keep their config elsewhere). Worker-mode consumers leave `docs-glob` at its default; docs-mode consumers leave `workers-glob` at its default. The unused input is ignored either way.

Repos that mix worker dirs with docs run the action twice via a matrix strategy, one entry per `config-path`. Each run posts a sticky PR comment keyed off its config so they don't clobber each other.
