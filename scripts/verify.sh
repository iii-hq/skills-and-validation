#!/usr/bin/env bash
# verify.sh — run iii-skill-check against worker manifests matching a glob.
#
# Usage: verify.sh <workers-glob> <layers>
# Env:   INSTALL_DIR  Path to the extracted skills-and-validation bundle
#                     (defaults to $PWD/.skill-check). The binary is at
#                     $INSTALL_DIR/bin/iii-skill-check; bundle-adjacent
#                     content lookup walks up from there.
#
# AI layer is dropped automatically when ANTHROPIC_API_KEY is unset.

set -euo pipefail

WORKERS_GLOB="${1:-*/iii.worker.yaml}"
LAYERS="${2:-structure,vale,ai}"

INSTALL_DIR="${INSTALL_DIR:-$PWD/.skill-check}"
BIN="$INSTALL_DIR/bin/iii-skill-check"
if [ ! -x "$BIN" ]; then
  echo "ERROR: iii-skill-check not found at $BIN — did you run download.sh first?" >&2
  exit 1
fi

fail=0
shopt -s nullglob
matched=0
for manifest in $WORKERS_GLOB; do
  matched=1
  dir="$(dirname "$manifest")"
  if [ ! -d "$dir/docs" ]; then
    echo "::notice::skipping $dir (no docs/ partials yet)"
    continue
  fi
  echo "::group::$dir"
  if ! "$BIN" verify-rendered "$dir"; then
    fail=1
  fi
  effective_layers="$LAYERS"
  if [ -z "${ANTHROPIC_API_KEY:-}" ] && [[ "$LAYERS" == *ai* ]]; then
    effective_layers="$(echo "$LAYERS" | sed -E 's/,?ai,?/,/g; s/^,//; s/,$//')"
    [ -z "$effective_layers" ] && effective_layers="structure"
    echo "::warning::ANTHROPIC_API_KEY not set; running layers=$effective_layers"
  fi
  if ! "$BIN" verify "$dir" --layers "$effective_layers"; then
    fail=1
  fi
  echo "::endgroup::"
done

if [ "$matched" -eq 0 ]; then
  echo "::warning::no manifests matched glob '$WORKERS_GLOB'"
fi

exit $fail
