---
title: "llm-only blocks"
description: "Wrap context for AI agents that should not appear in the human-facing rendered doc."
type: "how-to"
---

# llm-only blocks

Some context belongs in the skill (visible to AI agents) but not in the published doc (visible to humans). `iii-skill-render` strips the markers but keeps the inner content when it writes the `<source>.skill.md` artifact.

## MDX (`.mdx`) sources

In docs-mode, `.mdx` files are rendered to humans by Mintlify *directly* — the docs site shows the raw source after MDX processing. **Only the single-line inline form actually hides content from humans there.** The block form (`{/* llm-only:start */}` … `{/* llm-only:end */}`) does *not* work in MDX: each marker is its own single-line comment that Mintlify hides, but the prose between them is regular MDX content and renders normally. Mintlify also does not treat a multi-line `{/* … */}` as a single comment, so wrapping a payload across lines does not help.

Use the inline form:

```mdx
The engine reads `config.yaml` from the cwd {/* llm-only: when generating a script that starts iii in a different directory, pass `--config /path/to/config.yaml` explicitly rather than relying on the working directory. */} on startup.
```

The published page shows the surrounding prose with the comment stripped. The `.skill.md` sibling expands the comment to its payload so the LLM sees it.

If you need a multi-line LLM-only payload in docs-mode, move that content into a worker `.md` partial — those *are* run through the renderer and support the block form (see below). There is no clean way to hide a multi-line llm-only block in MDX today.

## Worker `.md` partials

Worker `docs/*.md` partials are not rendered to readers directly — `iii-skill-render` produces both the human-facing `README.md` and the LLM-facing `skill.md` / `skills/*.md` from the same source. Both shapes work:

```markdown
## Configuration

Configure the engine via its `config.yaml`.

<!-- llm-only:start -->
The schema is documented in `crates/iii-engine-config/src/schema.rs`. Field defaults are computed from the `Default` impl in that file. When inlining the defaults into a doc, read them from the source rather than guessing.
<!-- llm-only:end -->
```

The README contains just the surrounding prose; the skill artifact contains the prose plus the inner block content.

## When to use it

- Implementation pointers ("the source of truth is at `<path>`") that humans don't need but agents finding the right file do.
- Caveats that complicate the published doc but matter when an agent is generating code (`this only works on Linux`, `requires PRO_MODE=1` in the env).
- Stale-information markers ("verify against `git log` before quoting this version number"). The AI re-checks; the human reader doesn't need the warning cluttering the page.

Don't use it to hide voice-rule violations. The AI layer reads through the unwrapped content, so flagged phrases inside `llm-only:start`/`end` still flag.

## Balance matters

Every `llm-only:start` needs a matching `llm-only:end`. The structure layer flags imbalances per file across both HTML and MDX forms; if you see `unbalanced llm-only blocks: N start markers, M end markers`, the renderer will produce a malformed skill artifact.
