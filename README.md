# skills-and-validation

Render and validate iii worker docs against the project's voice, structure, and Diataxis rules. You write short markdown partials under `<worker>/docs/`; this project renders them into the worker's `README.md` and a single self-contained `skill.md`, then verifies the result with Vale and an AI pass on every commit and every PR.

It also supports a **docs mode** for standalone (Mintlify, Fumadocs, etc.) `.md` / `.mdx` documentation.

---

## Install

There are a few prerequisites to install; the pre-commit hook is optional.

### 1. Vale

Vale is used to validate prose against static rules.

```bash
brew install vale          # macOS or Linux Homebrew
vale --version             # confirm it's on PATH
```

Other installs: [vale.sh](https://vale.sh) or the [release page](https://github.com/errata-ai/vale/releases). The composite GitHub Action installs Vale on the CI runner for you, so only local setups need this step.

### 2. The `iii-skill-render` and `iii-skill-check` binaries

```bash
curl -fsSL https://raw.githubusercontent.com/iii-hq/skills-and-validation/latest/scripts/install.sh | bash
```

The installer places binaries at `~/.local/bin/iii-skill-{render,check}` and the bundle (rules, Vale styles, skill bundles, templates, scripts) at `~/.local/share/skill-check/current/`. Add `~/.local/bin` to your `PATH` if it isn't already.

### 3. The `iii-skill-authoring` skill bundle

This is the bundle agents (and you) read to know how to write worker partials. Pick whichever surface fits your tooling.

**`skillkit`** — installs both `iii-skill-authoring` and `iii-doc-authoring`:

```bash
cd $HOME && npx skillkit add iii-hq/skills-and-validation/content/skills
```

### 4. The pre-commit hook (optional; per repo)

`cd` into the repo whose workers you're validating, then run the installer. The script refuses to install into this repo (it's only meaningful in consumer repos).

```bash
cd /path/to/your/consumer-repo
~/.local/share/skill-check/current/scripts/install-hook.sh
```

The hook symlinks into `.git/hooks/pre-commit`. On every commit it:

1. Detects staged paths under any worker dir (`<worker>/iii.worker.yaml` + `<worker>/docs/`).
2. Re-renders each affected worker with `iii-skill-render --write`.
3. Re-stages the rendered `README.md` and `skill.md`.
4. Runs `iii-skill-check verify-rendered` + `iii-skill-check verify --layers structure,vale`.
5. Blocks the commit on remaining violations.

The hook deliberately skips the AI layer (slow, costs tokens). CI runs the AI layer on every PR.

### Upgrading

Re-run install steps 2 and 3.

---

## Configure your repo

Two files per consumer repo: `.skill-check.yaml` at the root, and a GitHub Actions workflow.

### `.skill-check.yaml`

Copy from `~/.local/share/skill-check/current/templates/.skill-check.yaml` and edit. Worker-mode minimum:

```yaml
version: 1
ai_check:
  provider: anthropic
  model: claude-opus-4-7
  api_key_env_var: ANTHROPIC_API_KEY
  max_tokens: 6000
```

| Field                      | Purpose                                                                                          |
| -------------------------- | ------------------------------------------------------------------------------------------------ |
| `version`                  | Schema version. `1` = worker mode (implicit). `2` requires the `mode` field below.               |
| `mode`                     | v2 only. `worker` or `docs`. Leave unset on v1 for worker mode.                                  |
| `ai_check.provider`        | Currently only `anthropic`.                                                                      |
| `ai_check.model`           | Anthropic model id, e.g. `claude-opus-4-7`.                                                      |
| `ai_check.api_key_env_var` | Env var holding the API key. Read by the validator, the action, and `scripts/verify-workers.sh`. |
| `ai_check.max_tokens`      | Output token budget per AI call.                                                                 |

### GitHub Actions workflow

Copy `~/.local/share/skill-check/current/content/github_workflows_example.yml` to `.github/workflows/skill-check.yml` and pin the action ref. The minimum:

```yaml
name: skill-check
on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read
  pull-requests: write # opt-in: enables the sticky PR comment
  actions: write # opt-in: persistent AI skip-cache

jobs:
  skill-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: iii-hq/skills-and-validation@v0.3
        with:
          anthropic-api-key: ${{ secrets.ANTHROPIC_API_KEY }}
```

Set `ANTHROPIC_API_KEY` as a repo secret so the AI layer runs in CI. Without it, CI runs structure + Vale only and emits a workflow warning.

`workers-glob` defaults to `*/iii.worker.yaml`. Override it (or set `config-path`) for non-default layouts. Full input reference under [Action inputs](#action-inputs).

---

## Author a worker's README and SKILLs

A worker that uses this validator keeps its SKILL.md and README.md source material under `docs/`. You edit short partials; the renderer produces three artifacts. Never edit the rendered files by hand.

### Use the iii-skill-authoring skill

The bundle in `content/skills/iii-skill-authoring/` is the canonical reference. After step 3 of [Install](#install), read individual topics:

```bash
skillkit read iii-skill-authoring/quickstart       # what makes a good ## Quickstart
skillkit read iii-skill-authoring/structure        # the slot order in each rendered artifact
skillkit read iii-skill-authoring/skeleton         # copy-paste starter for a new worker
skillkit read iii-skill-authoring/voice            # voice rules (slop lists, banned phrasing)
skillkit read iii-skill-authoring/leaves           # per-function `docs/leaves/<leaf>.md` bodies
skillkit read iii-skill-authoring/llm-only-blocks  # spans visible to agents, hidden from humans
skillkit read iii-skill-authoring/check            # running the validator locally
```

The same files are on disk under `~/.local/share/skill-check/current/content/skills/iii-skill-authoring/` and the engine surfaces them through `iii://` once the `skills:` glob is set. Coding agents in the consumer repo should read these topics before editing partials.

### What to write (inputs)

```
<worker>/
├── iii.worker.yaml          # name + description + tags feed the searchable frontmatter (all required); plus build characteristics
├── config.yaml              # worker runtime config; inlined verbatim under ## Configuration
└── docs/
    ├── intro.md             # intro paragraph(s) inserted after the title (README + skill.md)
    ├── quickstart.md        # body of ## Quickstart (README only)
    ├── companions.md        # appended inside "## Install" and covers common workers that a user
                             # or agent may want to use alongside this worker (optional).
    ├── migration.md         # body of ## Migration notes (optional, README only)
                             # Used to document any breaking changes that require migration steps.
    └── leaves/
        └── <leaf>.md        # inlined under ## Additional HOWTOs in both README and skill.md; write any number
                             # they cover specifics that individual commands that an agent needs to
                             # know how to do (ex: enqueue.md, chmod.md).
                             # They DO NOT cover function signatures, outputs, or other API-level documentation.
                             # They ARE (like the skills) intended to be a how-to do something.
                             # They CAN be multi-step common actions like `check-queue-success.md`.
```

Notes on the inputs:

- **`docs/intro.md`** — one or two short paragraphs: what the worker does, who calls it, the single most important thing it gives you. Can contain `<!-- llm-only:start --> ... <!-- llm-only:end -->` blocks for agent-only routing hints.
- **`docs/quickstart.md`** — the meat of the README. Show one fenced code block per SDK language (Rust `iii_sdk`, TypeScript `iii-sdk`, Python `iii`), ≤ 30 lines each, with a realistic payload and the expected output/result. One to three functions, chosen for introductory value, not breadth.
- **`docs/leaves/<leaf>.md`** — the leaf name is the suffix of the function id after the last `::`. `textstats::analyze` → `docs/leaves/analyze.md`. H1 is a topical phrase (`# Sizing text before provider calls`), never the function id. Canonical sections: `## When to use` (three to five bullets of realistic call sites) and `## Notes` (gotchas, edge cases an agent will trip on).
- **`config.yaml`** — runtime config for the worker itself. The renderer inlines it verbatim under `## Configuration`. It preserves comments and all `config.yaml` entries should contain accompanying comments on usage.

Function signatures, payload schemas, and `RegisterFunction::new("…").description("…")` text are generated by a separate auto-gen system (iii-directory worker). Don't duplicate them in the partials.

### Outputs: what gets rendered

```
<worker>/
├── README.md       # published on iii.dev (human-facing)
└── skill.md        # single agent-facing file for iii://<worker> (leaves inlined)
```

Two files, no `skills/` directory: each leaf is inlined into both artifacts under `## Additional HOWTOs`. Both files open with an identical YAML frontmatter block — `name`, `description`, `tags`, all sourced from `iii.worker.yaml` — that makes the worker searchable in the registry. Below the frontmatter, each artifact starts with a generated banner comment; everything after that is derived from your partials. Rendered files should never be hand-edited (the structure layer will flag drift and re-renders blow away your edits anyway).

**README.md slot order:**

1. Frontmatter block (`name` / `description` / `tags`).
2. Generated banner.
3. `# <name>` (from `iii.worker.yaml.name`).
4. `intro.md`.
5. `## Install` + `iii worker add <name>` boilerplate, optionally followed by `companions.md`.
6. `## Quickstart` + `quickstart.md`.
7. `## Configuration` + code blocked `config.yaml`.
8. `## Migration notes` + `migration.md` (only if present).
9. `## Additional HOWTOs`: each leaf inlined, its `# Title` demoted to `### Title` (omitted when `docs/leaves/` is empty).

**skill.md slot order** (llm-only blocks unwrapped, human-only dropped):

1. Frontmatter block (identical to README's).
2. Generated banner.
3. `# <name>`.
4. `intro.md`.
5. `companions.md`.
6. `## Additional HOWTOs`: same inlined leaves as the README.

**Inlined leaves:** each `docs/leaves/<leaf>.md` is appended under `## Additional HOWTOs` with its headings demoted by two levels (`#` → `###`, `##` → `####`). A top-level H1 is required per leaf (the structure layer flags its absence).

### Render and verify locally

```bash
iii-skill-render <worker> --write              # produces README.md + skill.md (removes any stale skills/ dir)
iii-skill-check verify <worker>                # all three layers (structure + Vale + AI)
iii-skill-check verify <worker> --layers structure,vale   # offline; no API key needed
iii-skill-check verify <worker> --layers ai    # AI only — fastest signal after a tweak
iii-skill-check verify-rendered <worker>       # confirm rendered artifacts match partials
```

`verify-rendered` re-renders in memory and diffs against the on-disk files. Non-zero exit means an artifact drifted; re-run `iii-skill-render <worker> --write`. The pre-commit hook does this for you on every commit.

For the AI layer, export the key named in your `.skill-check.yaml`:

```bash
export ANTHROPIC_API_KEY=sk-ant-…
```

---

## `llm-only` and `human-only` blocks

A single source partial can carry three audiences cleanly: plain prose (everyone sees it), `llm-only` spans (only the agent-facing artifact gets them), and `human-only` spans (only the human-facing render gets them). Both block (`:start`/`:end`) and inline forms are implemented for both directives.

| Shape  | HTML form (worker `.md`)                                | MDX form (`.mdx`)                                     |
| ------ | ------------------------------------------------------- | ----------------------------------------------------- |
| Block  | `<!-- llm-only:start -->` … `<!-- llm-only:end -->`     | `{/* llm-only:start */}` … `{/* llm-only:end */}`     |
|        | `<!-- human-only:start -->` … `<!-- human-only:end -->` | `{/* human-only:start */}` … `{/* human-only:end */}` |
| Inline | `<!-- llm-only: short note -->`                         | `{/* llm-only: short note */}`                        |
|        | `<!-- human-only: short note -->`                       | `{/* human-only: short note */}`                      |

Marker lines must each sit on their own line. The structure layer balance-checks both block types per file; an unclosed `:start` fails verify. HTML and MDX forms count together, so a block opened in one form and closed in the other still balances.

### Worker `.md` partials — every form works

Worker partials are never rendered to humans directly. The renderer produces `README.md` (humans) and `skill.md` (agents) from the same source, so block and inline forms both work fully for both `llm-only` and `human-only`. Use whichever fits the payload.

```markdown
## Quickstart

This worker does X. Use `worker::get` and `worker::set`.

<!-- llm-only:start -->

Prefer `get` over `set` for read-only flows; `set` invalidates the cache.

<!-- llm-only:end -->

<!-- human-only:start -->

**Heads-up for maintainers:** the legacy `foo` cli wrapper is under
`tools/legacy/foo.sh`. It's deprecated.

<!-- human-only:end -->
```

In the rendered README the `llm-only` block is invisible and the `human-only` block reads as a normal paragraph. In `skill.md` the inverse: `llm-only` body is expanded, `human-only` block is dropped entirely.

Inline form: each comment must be on a line of its own. The renderer expands it to its payload for the visible side and drops it on the hidden side. Embedded mid-paragraph inline comments are not parsed and pass through verbatim.

```markdown
The worker exposes `worker::set_token`.
<!-- llm-only: call this before any other op; tokens cache for 60s -->

Add the worker.
<!-- human-only: maintainers, this image is rebuilt nightly; bump the tag in iii.lock. -->
```

### `.mdx` docs sources exception: Do not use mutli-line tags.

Mintlify treats MDX comments as line-by-line

MDX files in docs mode are rendered to humans directly by Mintlify. Mintlify treats `{/* … */}` as an invisible single-line comment per line; it does not collapse a multi-line `{/* … */}` span into one comment. That asymmetry decides which form works for each directive:

- **`llm-only` in `.mdx`** — only the inline form hides from humans, and the comment must be on a line of its own (mid-paragraph inline comments are not parsed):

  ```mdx
  The worker exposes `set_token`.
  {/* llm-only: call this before any other op; tokens cache for 60s */}
  ```

  The block form `{/* llm-only:start */}` … `{/* llm-only:end */}` is still parsed by the renderer (so `skill.md` is correct), but the prose between the markers is regular MDX content that Mintlify renders to readers. The `:start`/`:end` lines themselves are invisible; the body between them leaks. There is no way to hide a multi-line `llm-only` span in MDX today — keep the payload to a short inline comment, or move it to a worker `.md` partial.

- **`human-only` in `.mdx`** — use the block form when you want humans to see the payload:

  ```mdx
  {/* human-only:start */}
  Maintainers: this image is rebuilt nightly. Bump the tag in `iii.lock` when you upgrade.
  {/* human-only:end */}
  ```

  Mintlify treats both marker lines as invisible comments and renders the body in between as ordinary prose, which is exactly what `human-only` wants for the human-facing side. The renderer then strips the entire block from `<source>.skill.md` so the agent never sees it.

  The inline form `{/* human-only: … */}` does _not_ work in `.mdx` — Mintlify drops the whole comment, so the payload is invisible to readers too. No render pass runs between the source and Mintlify to expand it. Reserve inline `human-only` for worker `.md` partials and for maintainer notes the agent should never see (the dropped-from-`skill.md` behavior, with no human surface).

Full authoring guides ship under `content/skills/iii-skill-authoring/llm-only-blocks.md` (worker mode) and `content/skills/iii-doc-authoring/llm-only-blocks.md` (docs mode); `human-only-blocks.md` exists in both bundles. Read via `skillkit read iii-skill-authoring/llm-only-blocks` etc.

### Relationship to `skill:...` markers

The `<!-- skill:... -->` markers (docs mode only) decide _whether_ a doc or section enters the skill set at all. `llm-only` and `human-only` decide _which spans_ of an in-scope source go to which audience. They compose: a marker block inside an `<!-- skill:exclude-section -->` section is dropped regardless, because the whole section is excluded.

---

## Fix errors the validator finds

The validator emits one line per violation in this format:

```
<file>:<line>:<severity> — <message>
```

`<severity>` is `error` (fails the run) or `warning` (surfaces but doesn't fail). AI failures appear under `[AI] <path>` blocks at the end with the model's full violation list.

In CI the same violations surface three ways: inline annotations on the Files diff, a markdown table in the Checks tab run summary, and (with `pull-requests: write`) a sticky PR comment that updates in place on each push.

### Common error categories and how to fix them

**Vale slop / marketing language** (`Terminology.SlopMarketing`, `Terminology.SlopMagic`, `Terminology.SlopEase`, `Terminology.SlopConnection`, `Terminology.SlopFlow`)

```
textstats/docs/intro.md:3:error — Avoid marketing/anthropomorphic phrasing 'blazing fast'. Use concrete, technical language.
```

Rewrite with concrete technical language. `blazing fast` → `sub-millisecond at p99`. `effortless` → drop the adjective and show the call. `wire up X to Y` → `register X; Y invokes it`. The full token lists live in `~/.local/share/skill-check/current/content/styles/Terminology/`.

**Em dashes** (`Terminology.EmDash`)

Rewrite with commas, parentheses, periods, or colons. The rule is at error level.

**Forbidden terms** (`Terminology.ForbiddenTerms`, `Terminology.BackendSoftware`)

Includes the bare term `telemetry` (disambiguate: `OpenTelemetry` / `observability` for traces/metrics/logs, or `iii-telemetry` for usage analytics) and a handful of backend-software terms with project-specific alternates.

**Diataxis voice drift in a how-to** (`Diataxis.HowTo`, `Diataxis.CrossContamination`)

Worker partials are how-tos. Phrases like `in this guide you will learn` or `step 1`, `step 2` are tutorial framing and get flagged. Rewrite as direct instructions: `Run X.`, `Then Y.`. `CrossContamination` warnings are advisory; the others are errors.

**Structure violations** (structure layer)

Missing or out-of-order sections in `README.md`, install command doesn't match `iii.worker.yaml.name`, source-build instructions in the rendered output, an `iii://<name>/<leaf>` link that doesn't resolve, or an unbalanced llm-only marker. Most of these mean a rendered file was hand-edited or a partial drifted from the slot order in [Outputs](#outputs-what-gets-rendered). Re-render and fix the source partial.

**Unbalanced llm-only block** (structure layer)

Every `llm-only:start` needs a matching `llm-only:end` on its own line. HTML and MDX forms count together, so you can open with one form and close with the other.

**AI layer failure** (`[AI] <path>`)

The AI layer reads the project rules in `content/project-rules/` (voice, general, workers, sdks, …) and reviews each rendered artifact against them. Failures cite specific passages. Treat the model's feedback as a peer review: the surfaced issue is usually real, but the specific rewrite it suggests is a starting point, not the only fix.

**Render drift** (`verify-rendered` non-zero exit)

The on-disk artifact doesn't match what the partials would produce. Run `iii-skill-render <worker> --write`, stage the result, retry.

### When auto-fix mode is on

With `write: true` plus `contents: write` on the action, the bot commits rendered output back to the PR branch (only after `verify` passed). If the bot's `chore: auto-render skill artifacts` commit lands before your next local push, rebase preferring the upstream side (the bot's render is the authoritative one):

```bash
git fetch origin
git rebase -X ours origin/<branch>
```

`-X ours` during a rebase resolves conflicts in favour of the side being rebased _onto_. Double-check non-artifact files with `git diff @{u}..` before pushing.

### Bypassing

- A single commit: `git commit --no-verify`.
- Re-running only the AI layer after a small tweak: `iii-skill-check verify <worker> --layers ai`.
- Letting an old binary keep running while you update: pass `--allow-old-version`.
- Suppressing the update check (offline, CI, batch scripts): `export SKV_NO_UPDATE_CHECK=1`.

---

## Docs mode

The same render → verify → optional auto-commit pipeline also runs against standalone Mintlify-shaped `.md` / `.mdx` documentation. Opt in with `version: 2` and `mode: docs` in `.skill-check.yaml`; each in-scope doc renders into a sibling `<source>.skill.md`. Diataxis ruleset is selected per doc by the frontmatter `type:` field (`tutorial`, `how-to`, `reference`, `explanation`).

Full guide:

```bash
skillkit read iii-doc-authoring/quickstart
skillkit read iii-doc-authoring/frontmatter
skillkit read iii-doc-authoring/types
skillkit read iii-doc-authoring/markers          # <!-- skill:include-doc --> etc.
skillkit read iii-doc-authoring/llm-only-blocks  # MDX caveats
```

Worker and docs modes share the validator binary; what differs is the unit and the rendered artifacts:

|                            | Worker mode                                                       | Docs mode                                            |
| -------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------- |
| Unit                       | A worker dir (one `iii.worker.yaml`)                              | One `.md` / `.mdx` doc                               |
| Sources                    | `<worker>/docs/*.md`, `iii.worker.yaml`, `config.yaml`            | The doc itself, plus its YAML frontmatter            |
| Rendered artifacts         | `<worker>/README.md`, `<worker>/skill.md` (leaves inlined) | `<source>.skill.md` sibling next to each doc         |
| Action input scope         | `workers-glob` (default `*/iii.worker.yaml`)                      | `docs-glob` (default `**/*.md **/*.mdx`)             |
| `.skill-check.yaml` schema | v1 (no `mode`) or v2 with `mode: worker`                          | v2 with `mode: docs` + `docs.include`/`docs.exclude` |

---

## Action inputs

| Input               | Default                  | Description                                                                                                                                                                                                                                                                                                         |
| ------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `version`           | from `.skill-check.yaml` | Pinned validator version, without the `v` prefix                                                                                                                                                                                                                                                                    |
| `config-path`       | `.skill-check.yaml`      | Path to the `.skill-check.yaml` controlling this run. Override per matrix entry to validate multiple modes in one repo.                                                                                                                                                                                             |
| `workers-glob`      | `*/iii.worker.yaml`      | Worker mode: glob of worker manifests to verify. Ignored in docs mode.                                                                                                                                                                                                                                              |
| `docs-glob`         | `**/*.md **/*.mdx`       | Docs mode: glob(s) of doc files to verify (space-separated). Filtered per-file against `docs.include`/`docs.exclude`.                                                                                                                                                                                               |
| `layers`            | `structure,vale,ai`      | Comma-separated subset of layers to run                                                                                                                                                                                                                                                                             |
| `vale-version`      | `3.14.1`                 | Pinned Vale version                                                                                                                                                                                                                                                                                                 |
| `anthropic-api-key` | (none)                   | API key for the AI layer; AI is auto-skipped when unset                                                                                                                                                                                                                                                             |
| `write`             | `false`                  | Auto-render and commit the diff back to the PR branch when sources drift from rendered output. Requires `contents: write`.                                                                                                                                                                                          |
| `scope`             | `all`                    | `all` validates every artifact matching the glob; `pr-diff` (PR events only) restricts to files changed against the merge base. A diff that touches any `.skill-check.yaml` falls back to full scan. In worker mode, a changed file under `<worker-dir>/` validates the whole worker since rendering is per-worker. |

The action auto-detects mode by reading the `.skill-check.yaml` named by `config-path`. Repos that mix worker dirs and docs run the action multiple times via a matrix strategy with one entry per controlling config; the sticky PR comment is keyed off `config-path` so each entry gets its own comment.

---

## Troubleshooting

**`vale: command not found`.** Vale is required for local runs; see [Install → 1. Vale](#1-vale).

**`verify-rendered` fails after editing a `docs/` partial.** Re-render the worker: `iii-skill-render <worker> --write`. The hook does this automatically on commit.

**Bot keeps auto-committing on top of my pushes.** See [When auto-fix mode is on](#when-auto-fix-mode-is-on). If you'd rather re-render locally than trust the bot, disable `write: true` in your workflow.

**Bundle lookup misses on a moved binary.** `bundle::find_content_root` walks up from the binary looking for `content/` with `project-rules/` and `.vale.ini`. Pass `--rules-dir` and `--vale-config` explicitly to `iii-skill-check verify` if you've relocated the layout.

**Anonymous download returns 404 on a public repo.** Confirm the asset name matches `skills-and-validation-{version}-{target}.tar.gz` exactly. The git tag has the `v` prefix; the asset filename does not.

**`cargo test` fails on path-dependent tests after renaming the repo dir.** `CARGO_MANIFEST_DIR` is baked into the test binary at compile time. Run `cargo clean` to force a rebuild.

---

## Layout (for contributors to this repo)

```
crates/iii-skill-core    — shared lib (render, structure, vale, ai, config, bundle)
crates/iii-skill-render  — render-only binary (no network deps)
crates/iii-skill-check   — verify + verify-rendered binary (Vale + AI)
content/                 — project-rules, styles, skills/, .vale.ini
templates/               — .skill-check.yaml + example-worker the consumer copies
fixtures/                — intentionally broken/targeted workers used by tests
scripts/                 — shared between the composite action and pre-commit hook
action.yml               — composite action consumed via `uses: iii-hq/skills-and-validation@v0.3`
```

Local end-to-end check:

```bash
./scripts/test-e2e.sh                            # offline phases (build, fixtures, scripts)
ANTHROPIC_API_KEY=sk-ant-… ./scripts/test-e2e.sh # also exercises the live AI layer
```

Pass `--clean` if you've renamed the repo directory (`CARGO_MANIFEST_DIR` is baked into test binaries at compile time).
