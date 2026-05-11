---
title: "Run iii-skill-check"
description: "Run iii-skill-check verify locally during authoring; the same binary CI uses on every PR."
type: "how-to"
---

# Running iii-skill-check locally

`iii-skill-check` is the validator that renders, lints, and AI-reviews worker artifacts. It runs on every PR via GitHub Actions and on every commit via the pre-commit hook, but it can also run directly during authoring.

## Render

```bash
iii-skill-render <worker> --write
```

Reads `iii.worker.yaml.name`, `<worker>/config.yaml`, and the partials under `<worker>/docs/`. Writes `<worker>/README.md`, `<worker>/skill.md`, and `<worker>/skills/*.md`.

Drop the `--write` flag to render to memory only, useful for previewing the rendered output without touching the on-disk artifacts.

## Verify all layers

```bash
iii-skill-check verify <worker>
```

Runs three layers in order, accumulating violations:

1. **Structure**: section presence and order in README, install command parity with `iii.worker.yaml.name`, no source-build blocks, llm-only marker balance, every `iii://<name>/<leaf>` link resolves.
2. **Vale**: every rendered artifact lints clean against `styles/Diataxis` (HowTo subset) and `styles/Terminology` (slop, forbidden terms).
3. **AI**: one Claude API call per artifact with the project rules concatenated as context. Requires `ANTHROPIC_API_KEY`.

Subset the layers with `--layers structure,vale` to skip the AI call locally.

## Verify rendered artifacts match source

```bash
iii-skill-check verify-rendered <worker>
```

Re-renders the worker in memory and diffs against the on-disk `README.md`, `skill.md`, and `skills/*.md`. Non-zero exit means an artifact drifted from the partials. Re-run `iii-skill-render <worker> --write` to fix.

## Reading violations

Output format is `<file>:<line>:<severity> — <message>`, where `severity` is `error` or `warning`. Errors fail the run; warnings surface in the same channels but exit code stays 0 when only warnings are present. Structure and Vale layers report inline; AI failures appear under `[AI] <path>` blocks at the end with the model's full violation list.

Vale's per-rule `level` decides severity. Most rules in `styles/Terminology/` use `level: error` (slop, em-dash, forbidden terms). The Diataxis quadrant drift rules use `level: warning` so writers see the drift without the build going red.

## Wire into CI

Adding the GitHub Actions workflow is required to run `iii-skill-check` against pull requests in a consumer repository. The canonical example ships with the install at `content/github_workflows_example.yml`. Copy it into the worker repo and pin the action ref:

```bash
cp ~/.local/share/skill-check/current/content/github_workflows_example.yml \
   .github/workflows/skill-check.yml
```

The bundled file uses `mode: worker` defaults; the action auto-detects mode from `.skill-check.yaml` at the workspace root.
