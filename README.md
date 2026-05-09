# skills-and-validation

Render and validate worker skill artifacts (`README.md`, `skill.md`, `skills/*.md`) against project-wide voice, structure, and Diataxis rules.

Ships two binaries and a composite GitHub Action. Consumers pin a `version` in `.skill-check.yaml`; the action and the pre-commit hook download a matching release tarball.

---

## Setup

Three things to install, in this order: the authoring skill bundle (so your tooling can read the conventions), the binaries (for local render + validation), and the pre-commit hook (so commits run the validator automatically).

### 1. Install the iii-skill-authoring skill bundle

The bundle is the canonical guide for worker docs — directory layout, renderer slots, voice rules, per-function leaves, llm-only block round-trip, and how to run `iii-skill-check` locally. Pick the surface that fits your tooling:

**Through skillkit:**

```bash
cd $HOME && npx skillkit add iii-hq/skills-and-validation/content/skills
```

**Through the iii engine** (after step 2 below puts the bundle on disk under `~/.local/share/skill-check/current/content/skills/`):

```yaml
# in your iii engine config.yaml
skills:
  - ~/.local/share/skill-check/current/content/skills/iii-skill-authoring/**/*.md
```

Browse topics with `skillkit read iii-skill-authoring/<topic>`. The bundle covers `quickstart`, `structure`, `skeleton`, `leaves`, `voice`, `llm-only-blocks`, `ideal-docs`, and `check`.

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
                          /<version>/content/   # bundled rules + Vale styles + iii-skill-authoring
                          /<version>/templates/ # .skill-check.yaml + example-worker
                          /<version>/scripts/   # ci-install.sh, verify.sh, pre-commit-hook.sh, install-hook.sh
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

Each consumer repo ships one `.skill-check.yaml` at the parent of its worker directories — typically the repo root when workers are top-level (`<repo>/<worker>/iii.worker.yaml`). The validator binary, the composite action, and the pre-commit hook all read it as the single source of truth for which release of `skills-and-validation` to use and how the AI layer authenticates.

See `templates/.skill-check.yaml` for an example file.

| Field                      | Required | Purpose                                                                                                                                                 |
| -------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `version`                  | yes      | Schema version of `.skill-check.yaml` itself (integer). Bumped when the file format changes; current is `1`. Unrelated to the `skills-and-validation` release pin (that lives in your workflow's `uses:` ref or the action's `version:` input). |
| `ai_check.provider`        | yes      | LLM provider for the AI layer. Currently only `anthropic` is supported.                                                                                 |
| `ai_check.model`           | yes      | Anthropic model id (e.g. `claude-opus-4-7`).                                                                                                            |
| `ai_check.api_key_env_var` | yes      | Name of the env var carrying the API key. The validator, the composite action, `scripts/verify.sh`, and `scripts/test-e2e.sh` all read this same field. |
| `ai_check.max_tokens`      | yes      | Output token budget per AI call.                                                                                                                        |
| `rules.path`               | no       | Local override for `project-rules/`. Omit to use the rules bundled with the released validator.                                                         |
| `styles.path`              | no       | Local override for the Vale `styles/` dir. Omit to use the bundled styles.                                                                              |

Pin the release in your workflow file via `uses: iii-hq/skills-and-validation@v0.1` (floats to the latest 0.1.x patch) or `@v0.1.5` (exact). Bump `version` only when the schema itself changes — most consumers leave it alone.

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
| `workers-glob`      | `*/iii.worker.yaml`      | Glob of worker manifests to verify                                                                                                 |
| `layers`            | `structure,vale,ai`      | Comma-separated subset of layers to run                                                                                            |
| `vale-version`      | `3.14.1`                 | Pinned Vale version                                                                                                                |
| `anthropic-api-key` | (none)                   | API key for the AI layer; AI is auto-skipped when unset                                                                            |
| `write`             | `false`                  | Auto-render workers and commit the diff back to the PR branch when sources drift from rendered output. Requires `contents: write`. |

### Auto-fix mode (opt-in)

With `write: true` plus `contents: write`, the action runs `iii-skill-render --write` against every matching worker before verifying. If that produces changes (someone edited `docs/*.md` but forgot to re-render), the action commits them with author `github-actions[bot]` and pushes to the PR branch:

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

The follow-up commit doesn't trigger another workflow run (GitHub's default `GITHUB_TOKEN` doesn't fire downstream `push`/`pull_request` events). Voice / structure violations the renderer can't fix on its own still show up in the verify output.

Forks: write mode only works on PRs opened from the same repository; the consumer's `GITHUB_TOKEN` can't push to a fork. Validation-only mode (`write: false`, the default) works for both.

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
