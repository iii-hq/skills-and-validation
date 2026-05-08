#!/usr/bin/env bash
# test-e2e.sh — run every locally-testable phase from README.md.
#
# Phases:
#   A  workspace builds and tests (cargo build + cargo test)
#   B  binaries render and verify the in-tree fixture
#   C  AI layer (skipped unless ANTHROPIC_API_KEY is set)
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
  --no-ai     skip phase C even if ANTHROPIC_API_KEY is set
  --keep-tmp  keep /tmp artifacts on success for inspection
  -h, --help  this message
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
echo "+ cargo test --workspace --no-fail-fast"
cargo test --workspace --no-fail-fast

# ----------------------------------------------------------------------
phase B "binaries against fixtures/example-worker"
# ----------------------------------------------------------------------
echo "+ iii-skill-render fixtures/example-worker (memory-only)"
./target/debug/iii-skill-render fixtures/example-worker

echo "+ iii-skill-render fixtures/example-worker --write"
./target/debug/iii-skill-render fixtures/example-worker --write
if ! git diff --quiet fixtures/example-worker; then
  echo "ERROR: --write produced a diff against the golden fixture:" >&2
  git --no-pager diff fixtures/example-worker >&2
  exit 1
fi
echo "  (empty diff — renderer matches the golden fixture)"

echo "+ iii-skill-check verify-rendered fixtures/example-worker"
./target/debug/iii-skill-check verify-rendered fixtures/example-worker

echo "+ iii-skill-check verify fixtures/example-worker --layers structure,vale"
./target/debug/iii-skill-check verify fixtures/example-worker --layers structure,vale

# ----------------------------------------------------------------------
phase C "AI layer (live API call)"
# ----------------------------------------------------------------------
if [ "$NO_AI" -eq 1 ]; then
  echo "(skipped: --no-ai flag)"
elif [ -z "${ANTHROPIC_API_KEY:-}" ]; then
  echo "(skipped: ANTHROPIC_API_KEY not set)"
else
  echo "+ iii-skill-check verify fixtures/example-worker --layers structure,vale,ai"
  ./target/debug/iii-skill-check verify fixtures/example-worker --layers structure,vale,ai
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
"$INSTALL_DIR/bin/iii-skill-check" verify-rendered fixtures/example-worker

echo "+ extracted iii-skill-check verify --layers structure,vale"
"$INSTALL_DIR/bin/iii-skill-check" verify fixtures/example-worker --layers structure,vale

# ----------------------------------------------------------------------
phase E "scripts/verify.sh against the extracted bundle"
# ----------------------------------------------------------------------
echo "+ verify.sh structure,vale"
( cd fixtures
  INSTALL_DIR="$INSTALL_DIR" "$REPO_ROOT/scripts/verify.sh" "*/iii.worker.yaml" "structure,vale" )

echo "+ verify.sh structure,vale,ai with no API key (should auto-strip ai)"
(
  cd fixtures
  unset ANTHROPIC_API_KEY
  INSTALL_DIR="$INSTALL_DIR" "$REPO_ROOT/scripts/verify.sh" "*/iii.worker.yaml" "structure,vale,ai"
) | tee "$STRIP_LOG"
if ! grep -q "ANTHROPIC_API_KEY not set" "$STRIP_LOG"; then
  echo "ERROR: expected the ai-strip warning, did not see it in verify.sh output" >&2
  exit 1
fi

echo
echo "============================================================"
echo "all phases passed"
echo "============================================================"
