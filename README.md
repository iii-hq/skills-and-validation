# skills-and-validation

> [!TIP]
> **Authoring worker docs?** The release bundle ships an `iii-skill-authoring` skill bundle at `.skill-check/content/iii-skill-authoring/` after `scripts/download.sh` (or after the action runs). Browse with `skillkit read iii-skill-authoring/<topic>` — topics include `quickstart`, `structure`, `skeleton`, `leaves`, `voice`, `llm-only-blocks`, `ideal-docs`, and `check`. To surface the bundle through the iii engine, add `.skill-check/content/iii-skill-authoring/skills/**/*.md` to the engine `config.yaml`'s `skills:` glob.

Render and validate worker skill artifacts (`README.md`, `skill.md`, `skills/*.md`) against project-wide voice, structure, and Diataxis rules.

Ships two binaries and a composite GitHub Action. Consumers pin a `version` in `.skill-check.yaml`; the action and the pre-commit hook download a matching release tarball.

---

## Layout

```bash
crates/iii-skill-core    — shared lib (render, structure, vale, ai, config, bundle)
crates/iii-skill-render  — render-only binary (no network deps)
crates/iii-skill-check   — verify + verify-rendered binary (Vale + AI)
content/                 — project-rules, styles, iii-skill-authoring, .vale.ini
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
| `version`                  | yes      | Pinned release tag (without the `v` prefix). The downloader fetches the matching tarball; the action picks up the same value.                           |
| `ai_check.provider`        | yes      | LLM provider for the AI layer. Currently only `anthropic` is supported.                                                                                 |
| `ai_check.model`           | yes      | Anthropic model id (e.g. `claude-opus-4-7`).                                                                                                            |
| `ai_check.api_key_env_var` | yes      | Name of the env var carrying the API key. The validator, the composite action, `scripts/verify.sh`, and `scripts/test-e2e.sh` all read this same field. |
| `ai_check.max_tokens`      | yes      | Output token budget per AI call.                                                                                                                        |
| `rules.path`               | no       | Local override for `project-rules/`. Omit to use the rules bundled with the released validator.                                                         |
| `styles.path`              | no       | Local override for the Vale `styles/` dir. Omit to use the bundled styles.                                                                              |

Bump `version` whenever you want a newer release of the validator; everything else is wiring you usually leave alone.

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

| Input               | Default                  | Description                                             |
| ------------------- | ------------------------ | ------------------------------------------------------- |
| `version`           | from `.skill-check.yaml` | Pinned validator version, without the `v` prefix        |
| `workers-glob`      | `*/iii.worker.yaml`      | Glob of worker manifests to verify                      |
| `layers`            | `structure,vale,ai`      | Comma-separated subset of layers to run                 |
| `vale-version`      | `3.14.1`                 | Pinned Vale version                                     |
| `anthropic-api-key` | (none)                   | API key for the AI layer; AI is auto-skipped when unset |

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
