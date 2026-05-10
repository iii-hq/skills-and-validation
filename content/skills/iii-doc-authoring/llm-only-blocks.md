---
title: "llm-only blocks"
description: "Wrap context for AI agents that should not appear in the human-facing rendered doc."
type: "how-to"
---

# llm-only blocks

Some context belongs in the skill (visible to AI agents) but not in the published doc (visible to humans). Wrap that content in `<!-- llm-only:start -->` / `<!-- llm-only:end -->` markers. The Mintlify renderer ignores HTML comments; `iii-skill-render` strips the markers but keeps the inner content when it writes the `<source>.skill.md` artifact.

## Block form

```mdx
## Configuration

Configure the engine via `~/.iii/config.yaml`.

<!-- llm-only:start -->
The schema is documented in `crates/iii-engine-config/src/schema.rs`. Field defaults are computed from the `Default` impl in that file — when inlining the defaults into a doc, read them from the source rather than guessing.
<!-- llm-only:end -->

…
```

The published page shows just the prose. The skill artifact contains the prose plus the inner block content (without the markers).

## Inline form

```mdx
The CLI reads `$XDG_CONFIG_HOME/iii/config.yaml` <!-- llm-only: which is `~/.config/iii/config.yaml` on Linux and `~/Library/Application Support/iii/config.yaml` on macOS --> when one of the search paths is set.
```

The published page shows the surrounding prose plus the comment (which Mintlify hides). The skill artifact reads `… config.yaml which is ~/.config/… on Linux and …`.

Use the inline form when the AI-only content is one short clause; use the block form for multi-line additions.

## When to use it

- Implementation pointers ("the source of truth is at `<path>`") that humans don't need but agents finding the right file do.
- Caveats that complicate the published doc but matter when an agent is generating code (`this only works on Linux`, `requires PRO_MODE=1` in the env).
- Stale-information markers ("verify against `git log` before quoting this version number") — the AI re-checks; the human reader doesn't need the warning cluttering the page.

Don't use it to hide voice-rule violations. The AI layer reads through the unwrapped content, so flagged phrases inside `llm-only:start`/`end` still flag.

## Balance matters

Every `<!-- llm-only:start -->` needs a matching `<!-- llm-only:end -->`. The structure layer flags imbalances per file; if you see `unbalanced llm-only blocks: N start markers, M end markers`, the renderer will produce a malformed skill artifact.
