---
title: "Diataxis types"
description: "Reference for the four Diataxis categories the type field selects, with the rule sets each one enables and a pointer to the quadrant-specific authoring guide."
type: "reference"
---

# Diataxis types

`type:` in your frontmatter selects a Diataxis category. The validator turns that into a per-artifact Vale ruleset and a per-artifact AI prompt; the writer's job is to keep the prose inside its category. Each quadrant has a dedicated authoring guide under [`diataxis/`](./diataxis/) — load `doc_workflow` plus the matching quadrant file for any docs work.

| `type:` | Quadrant | Authoring guide | Diataxis rules enabled | Cross-contamination check |
| --- | --- | --- | --- | --- |
| `tutorial` | learning-oriented | [`diataxis/doc_tutorial`](./diataxis/doc_tutorial.md) | `Tutorial`, `TutorialExplanation`, `TutorialAbstraction`, `TutorialReferenceLists` | disabled (tutorial framing is expected) |
| `how-to` | problem-oriented | [`diataxis/doc_howto`](./diataxis/doc_howto.md) | `HowTo`, `HowToBackground` | enabled |
| `reference` | information-oriented | [`diataxis/doc_reference`](./diataxis/doc_reference.md) | `Reference`, `ReferenceOpinion`, `ReferenceTeaching` | enabled |
| `explanation` | understanding-oriented | [`diataxis/doc_explanation`](./diataxis/doc_explanation.md) | `Explanation`, `ExplanationImperatives` | enabled |

The cross-contamination check is the Vale rule that flags tutorial-specific framing phrases (`in this tutorial`, `this tutorial will`, `by the end of this tutorial`, `you have successfully completed`, `congratulations you have`) appearing in non-tutorial docs. It's disabled for `type: tutorial` so tutorials may use those phrases freely. The deeper drift-detection rules — `Tutorial`, `TutorialExplanation`, `TutorialAbstraction`, `TutorialReferenceLists` for tutorial drift; `HowTo`, `HowToBackground`, `Reference`, `ReferenceTeaching`, `ReferenceOpinion`, `Explanation`, `ExplanationImperatives` for the others — stay enabled per quadrant and catch drift in both directions regardless of `CrossContamination`.

## Picking the right type

A single page that reads naturally as several categories at once should be split. The validator doesn't try to detect the misfit — the writer does, with help from the authoring guides.

A common split:

- A `tutorial` for the first end-to-end build.
- A `how-to` per recurring task ("add a worker", "rotate credentials").
- A `reference` page per CLI command, config field, or SDK module.
- One or two `explanation` pages for the load-bearing concepts.

If you change a doc's category, change its `type:` — never split the difference by leaving the old type and rewording.
