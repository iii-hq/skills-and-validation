#!/usr/bin/env bash
# test-e2e.sh — run every locally-testable phase from README.md.
#
# Phases:
#   A  workspace builds and tests (cargo build + cargo test)
#   B  binaries render and verify the in-tree fixture
#   C  AI layer (skipped unless the env var named by
#      templates/.skill-check.yaml's `api_key_env_var` is set;
#      auto-loaded from .env if present and not already in the env)
#   D  release tarball + extract + run-from-bundle (uses /tmp)
#   E  scripts/verify.sh against the extracted bundle
#
# Phases F (CI on push) and G (cut a real release) are excluded — they
# require network/repo state this script can't fabricate.

set -euo pipefail

print_help() {
  cat <<'HELP'
Usage: scripts/test-e2e.sh [flags]

Flags:
  --clean     run `cargo clean` before phase A (slower but bulletproof
              against a stale CARGO_MANIFEST_DIR after a directory rename)
  --no-ai     skip phase C even when the API key is set
  --keep-tmp  keep /tmp artifacts on success for inspection
  -h, --help  this message

Auth:
  Phase C reads the env-var name from templates/.skill-check.yaml's
  `api_key_env_var` field. The value is taken from the existing
  environment, or — if absent — sourced from a .env file at the repo
  root. Existing env vars are never overridden.
HELP
}

CLEAN=0
NO_AI=0
KEEP_TMP=0
for arg in "$@"; do
  case "$arg" in
    --clean)    CLEAN=1 ;;
    --no-ai)    NO_AI=1 ;;
    --keep-tmp) KEEP_TMP=1 ;;
    -h|--help)  print_help; exit 0 ;;
    *) echo "unknown flag: $arg" >&2; print_help >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# Resolve the env-var name the validator reads (per .skill-check.yaml's
# `api_key_env_var` field) and, if that var isn't already in the
# environment, populate it from .env. Existing env vars win over .env.
KEY_VAR="$(awk '/^[[:space:]]*api_key_env_var:/ {print $2; exit}' templates/.skill-check.yaml 2>/dev/null || true)"
KEY_VAR="${KEY_VAR:-ANTHROPIC_API_KEY}"
if [ -z "${!KEY_VAR:-}" ] && [ -f .env ]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env
  set +a
fi

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)              TRIPLE="aarch64-apple-darwin" ;;
  Darwin-x86_64)             TRIPLE="x86_64-apple-darwin" ;;
  Linux-x86_64)              TRIPLE="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64|Linux-arm64) TRIPLE="aarch64-unknown-linux-gnu" ;;
  *) echo "unsupported host: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

VERSION="0.0.0-e2e"
PKG_NAME="skills-and-validation-${VERSION}-${TRIPLE}"
PKG_DIR="/tmp/${PKG_NAME}"
TARBALL="/tmp/${PKG_NAME}.tar.gz"
INSTALL_DIR="/tmp/skill-check-e2e-install"
STRIP_LOG="/tmp/skill-check-e2e-strip.log"

cleanup() {
  if [ "$KEEP_TMP" -eq 0 ]; then
    rm -rf "$PKG_DIR" "$TARBALL" "$INSTALL_DIR" "$STRIP_LOG"
  fi
}
trap cleanup EXIT

phase() {
  echo
  echo "============================================================"
  echo "Phase $1 — $2"
  echo "============================================================"
}

require() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "ERROR: missing dependency: $1" >&2
    exit 1
  }
}

require cargo
require git
require tar
require curl

# Vale is needed for phase B; surface a clear pointer if it's absent.
if ! command -v vale >/dev/null 2>&1; then
  cat <<'EOF' >&2
ERROR: vale is not on $PATH; phase B's vale layer will fail.
Install Vale: https://docs.vale.sh/topics/installation
EOF
  exit 1
fi

# ----------------------------------------------------------------------
phase A "cargo build + cargo test"
# ----------------------------------------------------------------------
if [ "$CLEAN" -eq 1 ]; then
  echo "+ cargo clean"
  cargo clean
fi
echo "+ cargo build --workspace"
cargo build --workspace
# Live-API tests (ai_check_*) run in phase C with --show-output so the
# model's responses surface; skip them here to avoid double-billing.
echo "+ cargo test --workspace --no-fail-fast (skipping ai_check_*; see phase C)"
cargo test --workspace --no-fail-fast -- --skip ai_check_

# ----------------------------------------------------------------------
phase B "binaries against templates/example-worker"
# ----------------------------------------------------------------------
echo "+ iii-skill-render templates/example-worker (memory-only)"
./target/debug/iii-skill-render templates/example-worker

echo "+ iii-skill-render templates/example-worker --write"
./target/debug/iii-skill-render templates/example-worker --write
if ! git diff --quiet templates/example-worker; then
  echo "ERROR: --write produced a diff against the golden fixture:" >&2
  git --no-pager diff templates/example-worker >&2
  exit 1
fi
echo "  (empty diff — renderer matches the golden fixture)"

echo "+ iii-skill-check verify-rendered templates/example-worker"
./target/debug/iii-skill-check verify-rendered templates/example-worker

echo "+ iii-skill-check verify templates/example-worker --layers structure,vale"
./target/debug/iii-skill-check verify templates/example-worker --layers structure,vale

# Negative case: the deliberately-broken fixture must fail loudly.
echo "+ iii-skill-check verify fixtures/broken-worker --layers structure,vale (should FAIL)"
if ./target/debug/iii-skill-check verify fixtures/broken-worker --layers structure,vale 2>/dev/null; then
  echo "ERROR: verify should have failed against fixtures/broken-worker" >&2
  exit 1
fi
echo "  (correctly rejected — multiple layer violations)"

# ----------------------------------------------------------------------
phase C "AI layer (live API call)"
# ----------------------------------------------------------------------
if [ "$NO_AI" -eq 1 ]; then
  echo "(skipped: --no-ai flag)"
elif [ -z "${!KEY_VAR:-}" ]; then
  echo "(skipped: $KEY_VAR not set; checked environment and .env)"
else
  # Five lib tests — each one prints the model's full response to stderr.
  # Each FAIL test asserts specific keywords in the response so a wrong-
  # reason FAIL doesn't masquerade as a pass.
  #
  #   ai_check_passes_example_readme            PASS    canary worker
  #   ai_check_fails_marketing_fluff            FAIL    voice keywords
  #   ai_check_fails_broken_fixture             FAIL    voice + broken-link
  #   ai_check_flags_sdk_convention             FAIL    sdks.md violation
  #   ai_check_flags_built_in_concept           FAIL    "built-in" framing
  echo "+ cargo test ai_check_ -- --show-output  (auth via \$$KEY_VAR)"
  cargo test --workspace --no-fail-fast ai_check_ -- --show-output

  # Binary integration: PASS path through verify against the canary template.
  echo "+ iii-skill-check verify templates/example-worker --layers ai"
  ./target/debug/iii-skill-check verify templates/example-worker --layers ai

  # Binary integration: FAIL path against the broken fixture. Captures all
  # three per-artifact AI responses so we can assert per-artifact granular
  # behavior — every artifact must show up flagged, and the response must
  # cite specific seeded violations.
  ai_log="/tmp/skill-check-broken-ai.log"
  echo "+ iii-skill-check verify fixtures/broken-worker --layers ai (should FAIL)"
  if ./target/debug/iii-skill-check verify fixtures/broken-worker --layers ai 2>&1 | tee "$ai_log"; then
    echo "ERROR: expected verify to fail on fixtures/broken-worker AI layer" >&2
    rm -f "$ai_log"
    exit 1
  fi

  # Each broken-worker artifact must be flagged independently.
  for art in README.md skill.md skills/example.md; do
    if ! grep -q "fixtures/broken-worker/$art" "$ai_log"; then
      echo "ERROR: AI layer didn't flag fixtures/broken-worker/$art" >&2
      rm -f "$ai_log"
      exit 1
    fi
  done

  # The response must cite the seeded violations by name.
  if ! grep -qiE "blazing|welcome|revolutionary|magical|marketing" "$ai_log"; then
    echo "ERROR: AI layer didn't cite a marketing/voice violation" >&2
    rm -f "$ai_log"
    exit 1
  fi
  if ! grep -qiE "nonexistent|broken link|iii://" "$ai_log"; then
    echo "ERROR: AI layer didn't cite the broken iii:// link" >&2
    rm -f "$ai_log"
    exit 1
  fi
  rm -f "$ai_log"
  echo "  (3/3 broken-worker artifacts flagged; voice + broken-link cited)"
fi

# ----------------------------------------------------------------------
phase D "release build + tarball + run-from-bundle"
# ----------------------------------------------------------------------
echo "+ cargo build --release --workspace"
cargo build --release --workspace

echo "+ pack ${PKG_NAME}.tar.gz"
rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR/bin" "$PKG_DIR/content" "$PKG_DIR/templates"
cp target/release/iii-skill-check  "$PKG_DIR/bin/"
cp target/release/iii-skill-render "$PKG_DIR/bin/"
cp -r content/.   "$PKG_DIR/content/"
cp -r templates/. "$PKG_DIR/templates/"
echo "$VERSION" > "$PKG_DIR/VERSION"
( cd /tmp && tar -czf "$(basename "$TARBALL")" "$(basename "$PKG_DIR")" )
ls -lh "$TARBALL"

echo "+ extract to $INSTALL_DIR"
rm -rf "$INSTALL_DIR" && mkdir -p "$INSTALL_DIR"
tar -xzf "$TARBALL" -C "$INSTALL_DIR" --strip-components=1

echo "+ extracted iii-skill-check verify-rendered"
"$INSTALL_DIR/bin/iii-skill-check" verify-rendered templates/example-worker

echo "+ extracted iii-skill-check verify --layers structure,vale"
"$INSTALL_DIR/bin/iii-skill-check" verify templates/example-worker --layers structure,vale

# ----------------------------------------------------------------------
phase E "scripts/verify.sh against the extracted bundle"
# ----------------------------------------------------------------------
echo "+ verify.sh structure,vale"
( cd fixtures
  INSTALL_DIR="$INSTALL_DIR" "$REPO_ROOT/scripts/verify.sh" "*/iii.worker.yaml" "structure,vale" )

echo "+ verify.sh structure,vale,ai with no API key (should auto-strip ai)"
# verify.sh's auto-strip checks the env-var name the consumer's config
# specifies; we read the same name here so the test matches whatever
# templates/.skill-check.yaml currently declares.
(
  cd fixtures
  unset "$KEY_VAR"
  INSTALL_DIR="$INSTALL_DIR" "$REPO_ROOT/scripts/verify.sh" "*/iii.worker.yaml" "structure,vale,ai"
) | tee "$STRIP_LOG"
if ! grep -q "$KEY_VAR not set" "$STRIP_LOG"; then
  echo "ERROR: expected the ai-strip warning ($KEY_VAR not set), did not see it" >&2
  exit 1
fi

echo
echo "============================================================"
echo "all phases passed"
echo "============================================================"
