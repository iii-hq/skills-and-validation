# Voice

The voice rules in `project-rules/voice.md` apply to every doc regardless of `type`. The Diataxis ruleset on top of them adjusts which patterns are *expected* vs flagged for a given category — `voice.md` is the always-on baseline.

## Reading the rules

Pull the rule sheet from the bundle the validator installs:

```
~/.local/share/skill-check/current/content/project-rules/voice.md
```

The headlines:

- No tutorial-speak in published docs that aren't tutorials. "Welcome!", "Let's get started!", "You're going to love this" all flag.
- No marketing fluff. "Blazing fast", "magical", "powerful", "the best", "seamless" — drop them.
- No hedging. "You might want to consider", "perhaps", "kind of" — say it plainly or don't say it.
- No exclamation salad. One exclamation per page is one too many in most contexts.

## Per-type expectations

The Diataxis ruleset adjusts which voice patterns are flagged based on `type`:

- `tutorial` — second-person address, imperatives, "you'll do X next" framing are expected. Cross-contamination is disabled, so tutorial phrases won't trip how-to or reference rules.
- `how-to` — second-person and imperatives are still allowed, but "you'll learn" and onboarding framing flag. Background is okay if minimal.
- `reference` — third-person facts, no opinion, no teaching. "You should" framing flags.
- `explanation` — narrative, reasoning, trade-offs. "Run this command" framing flags as misplaced imperatives.

## When the AI flags something

The AI layer reads each rendered skill artifact with the doc's `type:` baked into the per-artifact prompt. If the AI flags a phrase that you believe is correct for the category:

1. Re-check whether the doc's `type:` matches what you actually wrote. A page tagged `reference` that reads like a tutorial will get flagged for everything.
2. If `type:` is right, the wording probably *is* drifting. Rephrase to fit the category.
3. If you're sure the AI is wrong, that's a model-judgement disagreement — the structure + Vale layers don't catch this, so a one-off override isn't supported. File an issue against the rule the AI cited.
