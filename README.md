# skills-and-validation

Render and validate skill artifacts against project-wide voice, structure, and Diataxis rules. Two modes:

- **Worker mode** — partials under `<worker>/docs/` render into `<worker>/README.md`, `<worker>/skill.md`, and `<worker>/skills/*.md`. The original surface; v1 schemas land here implicitly.
- **Docs mode** — Mintlify-shaped `.md` / `.mdx` sources each render into a sibling `<source>.skill.md`. Heading-level inclusion + per-doc opt-in/out via HTML-comment markers; per-type Vale rules driven by frontmatter `type:`. Opt in by setting `version: 2` and `mode: docs` in `.skill-check.yaml`.

Ships two binaries and a composite GitHub Action. Consumers pin a `version` in `.skill-check.yaml`; the action and the pre-commit hook download a matching release tarball.

---

## Setup

Three things to install, in this order: the authoring skill bundles (so your tooling can read the conventions), the binaries (for local render + validation), and the pre-commit hook (so commits run the validator automatically).

### 1. Install the authoring skill bundles

Two bundles ship from `content/skills/`:

- **`iii-skill-authoring`** — for authoring iii worker partials. Directory layout, renderer slots, voice rules, per-function leaves, llm-only round-trip, running `iii-skill-check` against a worker. Use when you're writing the partials a worker's docs render from.
- **`iii-doc-authoring`** — for authoring Mintlify-shaped `.md` / `.mdx` docs that the docs-mode pipeline validates. Frontmatter shape, Diataxis types, the `<!-- skill:... -->` marker reference, the per-quadrant writing guides under `iii-doc-authoring/diataxis/`. Use when you're writing standalone documentation outside a worker.

Pick the surface that fits your tooling:

**Through skillkit** (installs both bundles):

```bash
cd $HOME && npx skillkit add iii-hq/skills-and-validation/content/skills
```

**Through the iii engine** (after step 2 below puts the bundles on disk under `~/.local/share/skill-check/current/content/skills/`):

```yaml
# in your iii engine config.yaml
skills:
  - ~/.local/share/skill-check/current/content/skills/iii-skill-authoring/**/*.md
  - ~/.local/share/skill-check/current/content/skills/iii-doc-authoring/**/*.md
```

Browse topics with `skillkit read iii-skill-authoring/<topic>` or `skillkit read iii-doc-authoring/<topic>`. The worker bundle covers `quickstart`, `structure`, `skeleton`, `leaves`, `voice`, `llm-only-blocks`, `ideal-docs`, and `check`. The docs bundle covers `quickstart`, `frontmatter`, `types`, `markers`, `voice`, `llm-only-blocks`, `check`, plus the `diataxis/` writing guides — `doc_workflow`, `doc_tutorial`, `doc_howto`, `doc_reference`, and `doc_explanation`.

### 2. Install the binaries

```bash
curl -fsSL https://raw.githubusercontent.com/iii-hq/skills-and-validation/latest/scripts/install.sh | bash
```

Pin to a major.minor line or an exact version when you want reproducibility:

```bash
curl -fsSL https://raw.githubusercontent.com/iii-hq/skills-and-validation/latest/scripts/install.sh | bash -s -- 0.1
# or
curl -fsSL https://raw.githubusercontent.com/iii-hq/skills-and-validation/latest/scripts/install.sh | bash -s -- 0.1.5
```

Layout after install:

```
~/.local/share/skill-check/<version>/      # extracted release tarball
                          /<version>/bin/  # iii-skill-{check,render} binaries
                          /<version>/content/   # bundled rules + Vale styles + skill bundles (iii-skill-authoring, iii-doc-authoring)
                          /<version>/templates/ # .skill-check.yaml + example-worker
                          /<version>/scripts/   # ci-install.sh, verify-workers.sh, verify-docs.sh, pre-commit-hook.sh, install-hook.sh
                          /current          # symlink → <version> (re-pointed on every install)
~/.local/bin/iii-skill-render              # symlink → current/bin/iii-skill-render
~/.local/bin/iii-skill-check               # symlink → current/bin/iii-skill-check
```

Add `~/.local/bin` to your `PATH` if it isn't already. Override either default with `SKV_DIR` / `SKV_BIN` env vars before running install.sh:

| Env var   | Default                      | Purpose                                               |
| --------- | ---------------------------- | ----------------------------------------------------- |
| `SKV_DIR` | `~/.local/share/skill-check` | Where versioned release dirs and `current` symlink go |
| `SKV_BIN` | `~/.local/bin`               | Where the `iii-skill-{check,render}` shims land       |

### 3. Install the pre-commit hook

The hook installs into whatever git repo you're currently in, so `cd` into the consumer repo first — running the script from somewhere else (including a clone of `skills-and-validation` itself) is almost never what you want, and the script will refuse if it detects it's being run from this repo.

```bash
cd /path/to/your/consumer-repo
~/.local/share/skill-check/current/scripts/install-hook.sh
```

The script symlinks `pre-commit-hook.sh` into `.git/hooks/pre-commit`. On every commit, the hook:

1. Detects staged paths under any worker dir (`<worker>/iii.worker.yaml` + `<worker>/docs/`).
2. Re-renders each affected worker with `iii-skill-render --write`.
3. Re-stages the rendered `README.md`, `skill.md`, `skills/*.md`.
4. Runs `iii-skill-check verify-rendered` + `iii-skill-check verify --layers structure,vale`.
5. Blocks the commit on remaining violations.

**The hook deliberately skips the AI layer** — it's slow and costs API tokens, so commits stay fast. CI runs the AI layer on every PR.

To bypass the hook for a single commit: `git commit --no-verify`.

### Running the AI layer manually

The pre-commit hook never asks for an API key, and the offline `--layers structure,vale` path works without one. To run the AI layer at the terminal, set the env var named in your repo's `.skill-check.yaml` (defaults to `ANTHROPIC_API_KEY`):

```bash
export ANTHROPIC_API_KEY=sk-ant-…
iii-skill-check verify <worker>                    # all three layers
iii-skill-check verify <worker> --layers ai        # AI only — fastest signal
```

CI invokes the AI layer automatically. Set `ANTHROPIC_API_KEY` as a repo secret in your consumer repo for the CI run to use it; without the secret, CI runs structure + vale only and emits a workflow warning.

### Upgrading

Re-run `install.sh` whenever a new release lands — the `latest` tag floats to the most recent stable release, so the same one-liner installs the new version and re-points the symlinks.

Both binaries check at runtime whether a newer release is available (via `https://api.github.com/repos/iii-hq/skills-and-validation/releases/latest`, cached for 24h at `~/.cache/skill-check/update-check.json`). When out of date, the binary prints the install command and exits with code 2 — pass `--allow-old-version` to proceed on the older binary anyway:

```bash
iii-skill-check verify <worker> --allow-old-version
```

To suppress the check entirely (offline runs, CI environments, scripted batch invocations), set `SKV_NO_UPDATE_CHECK=1`. The composite action and `scripts/test-e2e.sh` set this automatically — only interactive local runs hit the API.

---

## Layout

```bash
crates/iii-skill-core    — shared lib (render, structure, vale, ai, config, bundle)
crates/iii-skill-render  — render-only binary (no network deps)
crates/iii-skill-check   — verify + verify-rendered binary (Vale + AI)
content/                 — project-rules, styles, skills/, .vale.ini
templates/               — .skill-check.yaml + example-worker the consumer copies
fixtures/                — intentionally broken/targeted workers used by tests
scripts/                 — shared between the composite action and pre-commit hook
action.yml               — composite action consumed via `uses: iii-hq/skills-and-validation@v1`
```

---

## Configuration: `.skill-check.yaml`

Each consumer repo ships one `.skill-check.yaml` at the root that contains the targets it validates. For worker mode that's the parent of the worker directories; for docs mode it's the docs root. The validator binary, the composite action, and the pre-commit hook all read it as the single source of truth for which release of `skills-and-validation` to use, which mode applies, and how the AI layer authenticates.

See `templates/.skill-check.yaml` for an example file.

| Field                      | Required | Purpose                                                                                                                                                 |
| -------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `version`                  | yes      | Schema version of `.skill-check.yaml` itself (integer). `1` = implicit worker mode. `2` = the `mode` field below is required.                          |
| `mode`                     | v2 only  | `worker` or `docs`. v1 schemas don't carry this field and resolve to `worker`.                                                                          |
| `ai_check.provider`        | yes      | LLM provider for the AI layer. Currently only `anthropic` is supported.                                                                                 |
| `ai_check.model`           | yes      | Anthropic model id (e.g. `claude-opus-4-7`).                                                                                                            |
| `ai_check.api_key_env_var` | yes      | Name of the env var carrying the API key. The validator, the composite action, `scripts/verify-workers.sh` / `scripts/verify-docs.sh`, and `scripts/test-e2e.sh` all read this same field. |
| `ai_check.max_tokens`      | yes      | Output token budget per AI call.                                                                                                                        |
| `docs.include`             | docs     | List of glob patterns for sources to include (relative to the docs root). Required when `mode: docs`.                                                   |
| `docs.exclude`             | no       | List of glob patterns evaluated after `docs.include` to drop matches.                                                                                   |
| `rules.path`               | no       | Local override for `project-rules/`. Omit to use the rules bundled with the released validator.                                                         |
| `styles.path`              | no       | Local override for the Vale `styles/` dir. Omit to use the bundled styles.                                                                              |

Pin the release in your workflow file via `uses: iii-hq/skills-and-validation@v0.1` (floats to the latest 0.1.x patch) or `@v0.1.5` (exact). Bump `version` only when the schema itself changes — most consumers leave it alone.

### Modes

Worker and docs are the same architecture pointed at different content. Both:

- Are configured by `.skill-check.yaml` at the relevant root.
- Are iterated by globs at the action level (`workers-glob` / `docs-glob`).
- Run the same render → verify → optional auto-commit pipeline.
- Use the same structure / Vale / AI layers (the rule sets adjust per mode + per Diataxis type).

What differs is the unit and the rendered artifacts:

| | Worker mode | Docs mode |
| --- | --- | --- |
| Unit | A worker dir (one `iii.worker.yaml`) | One `.md` / `.mdx` doc |
| Sources | `<worker>/docs/*.md`, `iii.worker.yaml`, `config.yaml` | The doc itself, plus its YAML frontmatter |
| Rendered artifacts | `<worker>/README.md`, `<worker>/skill.md`, `<worker>/skills/*.md` | `<source>.skill.md` sibling next to each doc |
| Action input scope | `workers-glob` (default `*/iii.worker.yaml`) | `docs-glob` (default `**/*.md **/*.mdx`) |
| `.skill-check.yaml` schema | v1 (no `mode`) or v2 with `mode: worker` | v2 with `mode: docs` + `docs.include`/`docs.exclude` |

#### Docs mode specifics

Each in-scope doc starts with YAML frontmatter:

```mdx
---
title: "Build a real-time todo app"
description: "Step-by-step build of a real-time todo using iii streams."
owner: "devrel"
type: "tutorial"
---
```

`type` selects the Diataxis ruleset (`tutorial`, `how-to`, `reference`, `explanation`) and is fed into the AI layer's per-artifact prompt. Required: `title`, `description`, `type`. Optional: `owner`.

`.skill-check.yaml` shape:

```yaml
version: 2
mode: docs
docs:
  include:
    - "**/*.md"
    - "**/*.mdx"
  exclude:
    - "**/CHANGELOG.md"
ai_check:
  provider: anthropic
  model: claude-opus-4-7
  api_key_env_var: ANTHROPIC_API_KEY
  max_tokens: 6000
```

Markers (`<!-- skill:... -->` HTML comments) override the globs at the doc level or filter sections at the heading level:

| Marker                                        | Scope     | Effect                                                                                                            |
| --------------------------------------------- | --------- | ----------------------------------------------------------------------------------------------------------------- |
| `<!-- skill:include-doc -->`                  | doc       | Pulls this doc in when `docs.include` missed it. `docs.exclude` still wins.                                       |
| `<!-- skill:exclude-doc -->`                  | doc       | Drops this doc from the skill set (overrides `docs.include`).                                                     |
| `<!-- skill:include-sections-by-default -->`  | file      | Default if absent. Every section is in the skill unless excluded.                                                 |
| `<!-- skill:exclude-sections-by-default -->`  | file      | No section is in the skill unless explicitly included.                                                            |
| `<!-- skill:include-section -->`              | heading   | Keeps this heading's section regardless of the file-level default.                                                |
| `<!-- skill:exclude-section -->`              | heading   | Drops this heading's section regardless of the file-level default.                                                |

Section markers can sit on the heading line (`## Internals <!-- skill:exclude-section -->`) or on their own line within the section. The renderer drops every recognised marker line from the rendered output; heading text stays.

The full authoring guide is in `content/skills/iii-doc-authoring/`. Browse via `skillkit read iii-doc-authoring/<topic>` after installing the bundle.

---

## Use it in your repo

Add this to a workflow file in the consumer repo, e.g. `.github/workflows/skill-check.yml`:

```yaml
name: skill-check

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read
  pull-requests:
    write # opt-in: enables the sticky PR comment.
    # Omit to keep just inline annotations + the run summary.

jobs:
  skill-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: iii-hq/skills-and-validation@v0.1.0
        with:
          anthropic-api-key: ${{ secrets.ANTHROPIC_API_KEY }}
```

### What the consumer's PR shows

Three layers of feedback, all driven by the validator's existing per-violation output:

| Surface                         | Permission needed      | What appears                                                |
| ------------------------------- | ---------------------- | ----------------------------------------------------------- |
| Inline annotations (Files diff) | none (always-on)       | red squiggle on each `path:line` the validator flagged      |
| Run summary (Checks tab)        | none (always-on)       | markdown table of every violation + `N verified, M skipped` |
| Sticky PR comment               | `pull-requests: write` | same markdown table, updated in place on each push          |

Annotations and step summary are processed by the runner itself — no token, no API call, no opt-in. The PR-comment step uses the consumer's default `GITHUB_TOKEN` and runs only on `pull_request` events; without `pull-requests: write` it no-ops via `continue-on-error: true` rather than failing the run.

### Action inputs

| Input               | Default                  | Description                                                                                                                        |
| ------------------- | ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| `version`           | from `.skill-check.yaml` | Pinned validator version, without the `v` prefix                                                                                   |
| `config-path`       | `.skill-check.yaml`      | Path to the `.skill-check.yaml` controlling this run. Override per matrix entry to validate multiple modes in one repo.            |
| `workers-glob`      | `*/iii.worker.yaml`      | Worker mode: glob of worker manifests to verify. Ignored in docs mode.                                                             |
| `docs-glob`         | `**/*.md **/*.mdx`       | Docs mode: glob(s) of doc files to verify (space-separated). The binary additionally filters per-file against `docs.include`/`docs.exclude`, so a permissive glob can't slip non-doc files into the renderer. |
| `layers`            | `structure,vale,ai`      | Comma-separated subset of layers to run                                                                                            |
| `vale-version`      | `3.14.1`                 | Pinned Vale version                                                                                                                |
| `anthropic-api-key` | (none)                   | API key for the AI layer; AI is auto-skipped when unset                                                                            |
| `write`             | `false`                  | Auto-render and commit the diff back to the PR branch when sources drift from rendered output. Requires `contents: write`.         |

The action auto-detects the mode by reading the `.skill-check.yaml` named by `config-path`. Worker-mode consumers leave `docs-glob` at its default (and it's ignored); docs-mode consumers leave `workers-glob` at its default (also ignored).

#### Validating both modes in one repo (matrix)

Repos that mix worker dirs and docs run the action multiple times via a matrix strategy — one entry per controlling config. The sticky PR comment is keyed off `config-path`, so each matrix run gets its own comment instead of clobbering the previous one.

```yaml
jobs:
  skill-check:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        include:
          - config-path: .skill-check.yaml             # workers at the repo root
          - config-path: docs/.skill-check.yaml         # docs under docs/
            docs-glob: docs/**/*.md docs/**/*.mdx
    steps:
      - uses: actions/checkout@v5
      - uses: iii-hq/skills-and-validation@v0.2
        with:
          config-path: ${{ matrix.config-path }}
          docs-glob: ${{ matrix.docs-glob || '**/*.md **/*.mdx' }}
          anthropic-api-key: ${{ secrets.ANTHROPIC_API_KEY }}
```

Each matrix entry produces an independent status check, so branch-protection rules can require both to pass.

### Render-then-verify ordering

The action always re-renders worker docs in the CI workspace *before* running `verify`. Without this, an out-of-sync `README.md` could mask voice or structure violations that exist in `docs/` but haven't been propagated to the rendered artifacts yet — verify would happily pass on the stale README while real errors sat unflagged in `docs/intro.md`. Rendering first means verify always operates on artifacts that reflect the current `docs/` content.

This is independent of `write:` — the in-tree render runs in both modes. What `write` controls is whether the rendered diff gets committed back to the PR branch.

### Auto-fix mode (opt-in)

With `write: true` plus `contents: write`, the action commits the rendered diff back to the PR branch — but only when `verify` passed first. The bot never pushes content the action hasn't validated, so a `chore: auto-render worker docs` commit on the branch is always known-good output.

```yaml
permissions:
  contents: write # required for auto-fix
  pull-requests: write # required for the sticky PR comment

jobs:
  skill-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
        with:
          ref:
            ${{ github.head_ref }} # check out the PR branch directly
            # (not the merge commit) so push-back lands
      - uses: iii-hq/skills-and-validation@v0.1
        with:
          write: true
          anthropic-api-key: ${{ secrets.ANTHROPIC_API_KEY }}
```

The follow-up commit doesn't trigger another workflow run (GitHub's default `GITHUB_TOKEN` doesn't fire downstream `push`/`pull_request` events) — that's fine because the action already validated the content before pushing.

Forks: write mode only works on PRs opened from the same repository; the consumer's `GITHUB_TOKEN` can't push to a fork. Validation-only mode (`write: false`, the default) works for both. In read-only mode, drift between `docs/` and rendered artifacts is reported as a workflow failure so the consumer knows to re-render locally and push.

#### When the bot auto-commits while you're working

If you push a source change and the bot's `chore: auto-render skill artifacts` commit lands before your next push, your local branch is one commit behind. Plain `git pull --rebase` works as long as your local commits didn't touch the rendered artifacts (`README.md`, `skill.md`, `skills/*.md`, or `*.skill.md`). When they did — typically because you ran the renderer locally before committing — rebase will conflict on those files.

The safe one-liner is to rebase preferring the upstream side, since the bot's render is authoritative (it ran on the head commit's sources):

```bash
git fetch origin
git rebase -X ours origin/<branch>
```

`-X ours` during a rebase resolves conflicts in favour of the side being rebased *onto* (the bot's commit), which is the opposite of the same flag during a merge. If you'd rather not memorise that, the equivalent merge form is:

```bash
git pull --no-rebase -X theirs
```

Either way, double-check the result with `git diff @{u}..` before pushing — `-X ours`/`-X theirs` is path-blind, so any conflicts in *non*-artifact files would also be silently resolved that way.

If you'd rather re-render than trust the bot's commit:

```bash
git pull --rebase
iii-skill-render <target> --write
git add <rendered paths>
git commit --amend --no-edit
git push --force-with-lease
```

---

## Local end-to-end check

```bash
./scripts/test-e2e.sh                            # offline phases (build, fixtures, scripts)
ANTHROPIC_API_KEY=sk-ant-… ./scripts/test-e2e.sh # also exercises the live AI layer
```

The script reads `api_key_env_var` from `templates/.skill-check.yaml` and auto-loads a matching `.env` file at the repo root if the value isn't already in the shell environment. `.env` is gitignored.

Pass `--clean` if you've just renamed the repo directory (`CARGO_MANIFEST_DIR` is baked into test binaries at compile time and a stale `target/` cache will fail path-dependent tests until rebuilt).

---

## Pre-tag checklist

Before pushing `v0.1.0`:

- [ ] `./scripts/test-e2e.sh --clean` exits 0
- [ ] `ANTHROPIC_API_KEY=sk-ant-… ./scripts/test-e2e.sh` exits 0 (exercises the AI layer)
- [ ] `git push origin main` is green on `ci.yml` + `dogfood.yml`
- [ ] `ANTHROPIC_API_KEY` is set as a repo secret (otherwise dogfood's AI layer is silently skipped in CI)

Tagging triggers `release.yml` and is effectively irreversible once consumers pin to the tag.

---

## Troubleshooting

**`cargo test` fails on path-dependent tests after renaming the repo dir.**
`CARGO_MANIFEST_DIR` is baked into the test binary at compile time. After a parent-directory rename, run `cargo clean` to force a rebuild with the new path.

**Vale layer fails with `vale: command not found`.**
Install Vale per the upstream docs: https://docs.vale.sh/topics/installation

**`cross install --locked` fails in CI.**
cross-rs occasionally lags behind cargo updates. Two fallback options:

1. Pin a known-good version: `cargo install cross --version 0.2.5 --locked`.
2. Replace cross with `cargo-zigbuild` in `release.yml` (no Docker, single runner builds all four Linux targets).

**Anonymous download returns 404 on a public repo.**
Confirm the asset name matches `skills-and-validation-{version}-{target}.tar.gz` exactly. The git tag has the `v` prefix; the asset filename does not.

**Bundle lookup misses on local builds.**
`bundle::find_content_root` walks up from the running binary looking for a `content/` dir with both `project-rules/` and `.vale.ini`. If you've moved the binary outside its bundle layout, pass `--rules-dir` and `--vale-config` explicitly to `iii-skill-check verify`.
