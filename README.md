# skills-and-validation

Render and validate worker skill artifacts (`README.md`, `skill.md`, `skills/*.md`) against project-wide voice, structure, and Diataxis rules.

Ships two binaries and a composite GitHub Action. Consumers pin a `version` in `.skill-check.yaml`; the action and the pre-commit hook download a matching release tarball.

---

## Layout

```bash
crates/iii-skill-core    — shared lib (render, structure, vale, ai, config, bundle)
crates/iii-skill-render  — render-only binary (no network deps)
crates/iii-skill-check   — verify + verify-rendered binary (Vale + AI)
content/                 — project-rules, styles, iii-skill-authoring, .vale.ini
templates/               — .skill-check.yaml the consumer copies into their repo
fixtures/example-worker  — golden render fixture used by tests + dogfood
scripts/                 — shared between the composite action and the workers' pre-commit hook
action.yml               — composite action consumed via `uses: iii-hq/skills-and-validation@v1`
```

---

## End-to-end test plan

The whole system can be exercised locally before any tag is pushed. To run phases A–E in one shot:

```bash
./scripts/test-e2e.sh                    # add --clean if you just renamed the repo dir
```

The script reads the env-var name from `templates/.skill-check.yaml`'s `api_key_env_var` field (default `ANTHROPIC_API_KEY`). The value is taken from your shell environment, or — if unset — sourced from `.env` at the repo root. So for the AI layer to run, either:

```bash
echo "ANTHROPIC_API_KEY=sk-ant-…" > .env       # gitignored
./scripts/test-e2e.sh
```

or:

```bash
ANTHROPIC_API_KEY=sk-ant-… ./scripts/test-e2e.sh
```

Or step through phases manually below — each is independent.

| Phase | What it validates                           | Network | Secrets             |
| ----- | ------------------------------------------- | ------- | ------------------- |
| A     | Rust workspace builds and tests             | none    | none                |
| B     | Binaries render and verify the fixture      | none    | none                |
| C     | AI layer hits Anthropic and parses a PASS   | yes     | `ANTHROPIC_API_KEY` |
| D     | Release tarball layout + bundle lookup      | none    | none                |
| E     | Action scripts against a hand-built install | partial | none                |
| F     | CI workflows on push                        | yes     | optional            |
| G     | Composite action against a real release     | yes     | optional            |

---

### Phase A — workspace builds and tests

```bash
cargo clean
cargo build --workspace
cargo test --workspace --no-fail-fast
```

The AI-layer test is gated on `ANTHROPIC_API_KEY` and prints a skip message when unset, so phase A is offline-safe.

---

### Phase B — binaries against the fixture (ie. test/example worker)

```bash
# render the fixture into memory (does not write)
./target/debug/iii-skill-render fixtures/example-worker
# -> rendered fixtures/example-worker (readme 2169 bytes, skill 616 bytes, 3 leaves)

# write rendered artifacts to disk (idempotent — should produce no diff)
./target/debug/iii-skill-render fixtures/example-worker --write
git diff fixtures/example-worker
# -> empty diff confirms the renderer is in sync with the golden fixture

# drift check: re-renders, byte-compares against on-disk artifacts
./target/debug/iii-skill-check verify-rendered fixtures/example-worker
# -> rendered artifacts match fixtures/example-worker

# structure + vale layers (no API key needed)
./target/debug/iii-skill-check verify fixtures/example-worker --layers structure,vale
# -> verify clean across [structure,vale] for fixtures/example-worker
```

`vale` must be on `$PATH` for the vale layer. If you don't already have it, follow the upstream install instructions: https://docs.vale.sh/topics/installation

---

### Phase C — AI layer (live API call)

Two complementary library tests run only when `ANTHROPIC_API_KEY` (or whatever `api_key_env_var` resolves to) is set:

- `ai_check_passes_example_readme_when_key_present` — the clean fixture must PASS.
- `ai_check_fails_marketing_fluff_when_key_present` — a synthetic README full of marketing fluff, tutorial-speak, and hedging must FAIL.

Both print the model's full response either way. Cargo captures stdout/stderr by default; pass `--show-output` to see responses on passing tests:

```bash
ANTHROPIC_API_KEY=sk-ant-… cargo test --workspace --no-fail-fast ai_check_ -- --show-output
```

Or run the binary directly against the clean fixture:

```bash
export ANTHROPIC_API_KEY=sk-ant-…
./target/debug/iii-skill-check verify fixtures/example-worker --layers structure,vale,ai
# -> verify clean across [structure,vale,ai] for fixtures/example-worker
```

---

### Phase D — release tarball + bundle-adjacent lookup

This simulates what `.github/workflows/release.yml` will produce on `v*` tag pushes.

```bash
# 1. release-mode build (with strip = true)
cargo build --release --workspace
ls -lh target/release/iii-skill-{check,render}
# -> ~3.9M and ~1.0M

# 2. pack a tarball with the release layout
TARGET="aarch64-apple-darwin"   # adjust to your host
VERSION="0.1.0"
NAME="skills-and-validation-${VERSION}-${TARGET}"
mkdir -p "/tmp/$NAME"/{bin,content,templates}
cp target/release/iii-skill-check  "/tmp/$NAME/bin/"
cp target/release/iii-skill-render "/tmp/$NAME/bin/"
cp -r content/.   "/tmp/$NAME/content/"
cp -r templates/. "/tmp/$NAME/templates/"
echo "$VERSION" > "/tmp/$NAME/VERSION"
( cd /tmp && tar -czf "$NAME.tar.gz" "$NAME" )
ls -lh /tmp/$NAME.tar.gz
# -> ~2.3M

# 3. extract somewhere clean and run from there
rm -rf /tmp/install && mkdir /tmp/install
tar -xzf "/tmp/$NAME.tar.gz" -C /tmp/install --strip-components=1
ls /tmp/install
# -> bin  content  templates  VERSION

# 4. confirm bundle::find_content_root walks up from bin/ to sibling content/
/tmp/install/bin/iii-skill-check verify-rendered fixtures/example-worker
/tmp/install/bin/iii-skill-check verify fixtures/example-worker --layers structure,vale
# both should print "verify clean"
```

---

### Phase E — scripts against the hand-built install

`scripts/verify.sh` is what the composite action calls. It expects an `INSTALL_DIR` produced by `scripts/download.sh` (or, here, the manual one from phase D).

```bash
# verify the fixture using the install from phase D
( cd fixtures
  INSTALL_DIR=/tmp/install ../scripts/verify.sh "*/iii.worker.yaml" "structure,vale" )

# AI-layer auto-strip when the key is unset
( cd fixtures
  unset ANTHROPIC_API_KEY
  INSTALL_DIR=/tmp/install ../scripts/verify.sh "*/iii.worker.yaml" "structure,vale,ai" )
# -> ::warning::ANTHROPIC_API_KEY not set; running layers=structure,vale
# -> verify clean across [structure,vale]
```

`scripts/download.sh` cannot be tested end-to-end until phase G (a release exists). You can dry-run its triple detection:

```bash
bash -n scripts/download.sh && echo "syntax ok"
# inspect the triple it would request:
sh -c 'case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) echo aarch64-apple-darwin ;;
  Darwin-x86_64) echo x86_64-apple-darwin ;;
  Linux-x86_64) ldd --version 2>&1 | grep -qi musl && echo x86_64-unknown-linux-musl || echo x86_64-unknown-linux-gnu ;;
  Linux-aarch64|Linux-arm64) ldd --version 2>&1 | grep -qi musl && echo aarch64-unknown-linux-musl || echo aarch64-unknown-linux-gnu ;;
esac'
```

---

### Phase F — CI workflows on push

After the first push to `origin/main`, three workflows fire:

| Trigger             | Workflow      | What runs                                                                      |
| ------------------- | ------------- | ------------------------------------------------------------------------------ |
| PR / push to main   | `ci.yml`      | `cargo test --workspace`                                                       |
| PR / push to main   | `dogfood.yml` | release build + `verify-rendered` + `verify` against `fixtures/example-worker` |
| `v*` tag / dispatch | `release.yml` | 6-target build matrix → tarballs → GitHub Release                              |

Watch them at `https://github.com/iii-hq/skills-and-validation/actions`.

```bash
# push commits and watch CI:
git push origin main
gh run watch --repo iii-hq/skills-and-validation         # blocks until current run finishes
gh run list --repo iii-hq/skills-and-validation --limit 5
```

The dogfood job needs `ANTHROPIC_API_KEY` set as a repo secret to run the AI layer; without it, the layer is skipped with a workflow warning. Set it once:

```bash
gh secret set ANTHROPIC_API_KEY --repo iii-hq/skills-and-validation
```

---

### Phase G — cut the first release and exercise the composite action

```bash
# 1. tag and push (triggers release.yml)
git tag v0.1.0
git push origin v0.1.0
gh run watch --repo iii-hq/skills-and-validation

# 2. confirm the matrix produced 6 tarballs
gh release view v0.1.0 --repo iii-hq/skills-and-validation
# -> should list 6 .tar.gz + 6 .tar.gz.sha256 assets

# 3. test the auth-on-failure-only download from a clean machine
rm -rf /tmp/skill-check-install
./scripts/download.sh 0.1.0 /tmp/skill-check-install
# -> on a public repo: anonymous succeeds, no auth code path runs
# -> on a private repo without `gh auth login` and without GITHUB_TOKEN:
#    "Anonymous download returned HTTP 404; trying authenticated download..."
#    "ERROR: Couldn't download ... gh auth login OR export GITHUB_TOKEN=..."

# 4. test the action from a sister repo (or from workers once cut over)
#    add this to .github/workflows/skill-check.yml in the consumer repo:
#
#    jobs:
#      verify:
#        runs-on: ubuntu-latest
#        steps:
#          - uses: actions/checkout@v4
#          - uses: iii-hq/skills-and-validation@v0.1.0
#            with:
#              anthropic-api-key: ${{ secrets.ANTHROPIC_API_KEY }}
```

---

## Pre-tag checklist

Before pushing `v0.1.0`:

- [ ] `./scripts/test-e2e.sh --clean` exits 0 (covers phases A–E)
- [ ] `ANTHROPIC_API_KEY=sk-ant-… ./scripts/test-e2e.sh` exits 0 (also covers phase C)
- [ ] `git push origin main` is green on `ci.yml` + `dogfood.yml`
- [ ] `ANTHROPIC_API_KEY` secret is set on the repo (otherwise dogfood AI layer is skipped silently in CI)

Tagging triggers `release.yml` and is irreversible (well, `gh release delete v0.1.0` is possible, but consumers may already have pinned it).

---

## Troubleshooting

**`cargo test` fails on path-dependent tests after renaming the repo dir.**
`CARGO_MANIFEST_DIR` is baked into the test binary at compile time. After a parent-directory rename, run `cargo clean` to force a rebuild with the new path.

**Vale layer fails with "vale: command not found".**
Install Vale per the upstream docs: https://docs.vale.sh/topics/installation

**`cross install --locked` fails in CI.**
cross-rs occasionally lags behind cargo updates. Two fallback options:

1. Pin a known-good version: `cargo install cross --version 0.2.5 --locked`.
2. Replace cross with `cargo-zigbuild` in `release.yml` (no Docker, single runner can build all four Linux targets).

**Anonymous download returns 404 on a public repo.**
Confirm the asset name matches `skills-and-validation-{version}-{target}.tar.gz` exactly (no `v` prefix on the version inside the asset name; the tag has the `v`, the asset doesn't).

**Bundle lookup misses on local builds.**
`bundle::find_content_root` walks up from the running binary looking for a `content/` dir with both `project-rules/` and `.vale.ini`. If you've moved the binary outside its bundle layout, pass `--rules-dir` and `--vale-config` explicitly to `iii-skill-check verify`.
