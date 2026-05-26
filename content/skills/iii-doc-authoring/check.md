---
title: "Run iii-skill-check"
description: "Run iii-skill-check verify locally to see what passes, what fails, and which layer flagged each finding."
type: "how-to"
---

# check

Run `iii-skill-check verify` against the docs root to see what passes, what fails, and where.

## Local invocation

```bash
iii-skill-check verify <docs-root>                    # all layers
iii-skill-check verify <docs-root> --layers structure # structure only
iii-skill-check verify <docs-root> --layers ai        # AI only
iii-skill-check verify-rendered <docs-root>           # drift check vs siblings
```

The binary walks up from `<docs-root>` to the nearest `.skill-check.yaml`, reads `mode: docs`, and dispatches. Pass a single `.md` / `.mdx` file to verify only that one doc, useful for fast iteration during authoring.

For files that don't carry frontmatter (project READMEs, CHANGELOGs, contributor guides), use `check-file` and pass the type explicitly:

```bash
iii-skill-check check-file README.md --type how-to
```

`check-file` skips the in-scope check, so the file doesn't need to match `docs.include`. Structure in this mode only checks llm-only-block balance.

## What each layer reports

- `structure`: runs against the *source* `.md` / `.mdx` files. Frontmatter validity, llm-only balance, conflicting doc-scope markers.
- `vale`: runs against the rendered `<source>.skill.md` siblings, with a per-run `.vale.ini` that maps each artifact to its frontmatter type's Diataxis ruleset. Flags every phrase that violates the matching style.
- `ai`: one Anthropic API call per artifact, with the doc's type in the per-artifact prompt. Returns `PASS` or a multi-line `FAIL` body explaining the violation.

The AI layer is auto-skipped when its API key env var is unset (default `ANTHROPIC_API_KEY`).

## Drift between source and skill

`iii-skill-check verify-rendered <docs-root>` re-renders every in-scope source and compares to the on-disk `<source>.skill.md`. Any mismatch is one drift line. Orphan `*.skill.md` files (skill artifacts whose source no longer exists or is now out of scope) are also flagged.

Re-render with `iii-skill-render <docs-root> --write` and commit the result.

## Reading violations

Each layer emits one line per finding in the format `path:line:severity — message`, where `severity` is `error` or `warning`. Errors fail the run; warnings surface in the same output but the exit code stays 0 when only warnings are present. The CI action turns these into:

- inline annotations on the PR's "Files changed" tab (always on)
- a markdown table in the run summary (always on)
- a sticky PR comment (opt-in via `pull-requests: write`)

Locally the lines go to stderr. Pipe through your usual `grep` / `less` workflow.

## Common failure shapes

- "frontmatter is missing or invalid: …": fix the frontmatter; the rest of the layers can't run cleanly without the type field.
- "unbalanced llm-only blocks": check for an unclosed `<!-- llm-only:start -->` somewhere.
- "Diataxis.HowTo" / "Diataxis.Tutorial" / etc.: the doc's content drifts toward a different category than its `type:` declares. Either rewrite or change the type.
- "skill.md is out of date": re-render and commit.
