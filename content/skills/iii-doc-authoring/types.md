# Diataxis types

`type:` in your frontmatter selects a Diataxis category. Each category enables one rule set in `Diataxis.*` and disables the others, plus the cross-contamination check that flags content drifting toward the wrong category.

## tutorial

Learning-oriented. The reader is following along; you're guaranteeing they end up somewhere predictable. Imperatives are expected (`run`, `add`, `open`); explanation is acceptable inline. Avoid full reference tables.

Rules enabled: `Tutorial`, `TutorialExplanation`, `TutorialAbstraction`, `TutorialReferenceLists`. Cross-contamination is disabled — tutorial-specific framing won't trip a how-to lint.

## how-to

Problem-oriented. The reader has a goal; you're showing the path. Background is allowed but kept brief and contextual. No first-time onboarding.

Rules enabled: `HowTo`, `HowToBackground`. Cross-contamination is enabled — tutorial-style "you'll learn how to" framing or reference-style flag tables will flag.

## reference

Information-oriented. The reader is looking something up; you're listing facts. No tutorials, no opinion, no teaching. Rule per parameter / function / option.

Rules enabled: `Reference`, `ReferenceOpinion`, `ReferenceTeaching`. Cross-contamination is enabled.

## explanation

Understanding-oriented. The reader wants to know *why*. Imperatives ("do this") are out of place; reasoning, history, trade-offs are the substance.

Rules enabled: `Explanation`, `ExplanationImperatives`. Cross-contamination is enabled.

## Picking the right type

If the page reads naturally as several categories at once, that's a sign it should be split. The renderer doesn't try to detect this — the writer does. A common split:

- A `tutorial` for the first end-to-end build.
- A `how-to` per recurring task ("add a worker", "rotate credentials").
- A `reference` page per CLI command, config field, or SDK module.
- One or two `explanation` pages for the load-bearing concepts.
