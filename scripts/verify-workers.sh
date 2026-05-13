#!/usr/bin/env bash
# verify-workers.sh — run iii-skill-check against worker manifests matching a glob.
#
# Usage: verify-workers.sh <workers-glob> <layers>
# Env:   INSTALL_DIR  Path to the extracted skills-and-validation bundle
#                     (defaults to $PWD/.skill-check). The binary is at
#                     $INSTALL_DIR/bin/iii-skill-check; bundle-adjacent
#                     content lookup walks up from there.
#
# The AI layer is dropped automatically when the env var named by
# .skill-check.yaml's `api_key_env_var` field is unset (default
# ANTHROPIC_API_KEY).
#
# globstar is enabled so consumers can use `**/iii.worker.yaml` to walk
# arbitrary subdirectory depth.

set -euo pipefail
shopt -s nullglob
# globstar (the `**` recursive glob) needs bash 4+; macOS still ships
# bash 3.2 by default. Enable it where available; on older bash the
# script falls back to single-level globbing — consumers with deeper
# layouts can pass multiple space-separated patterns instead.
if (( BASH_VERSINFO[0] >= 4 )); then
  shopt -s globstar
fi

WORKERS_GLOB="${1:-*/iii.worker.yaml}"
LAYERS="${2:-structure,vale,ai}"

INSTALL_DIR="${INSTALL_DIR:-$PWD/.skill-check}"
BIN="$INSTALL_DIR/bin/iii-skill-check"
if [ ! -x "$BIN" ]; then
  echo "ERROR: iii-skill-check not found at $BIN — did you run ci-install.sh first?" >&2
  exit 1
fi

# Resolve which env var carries the API key. Honors `api_key_env_var` in
# the consumer's .skill-check.yaml (in CWD); falls back to ANTHROPIC_API_KEY.
KEY_VAR="$(awk '/^[[:space:]]*api_key_env_var:/ {print $2; exit}' .skill-check.yaml 2>/dev/null || true)"
KEY_VAR="${KEY_VAR:-ANTHROPIC_API_KEY}"

# Compute the effective layer set once: drop the AI layer up front when its
# env var is unset, so per-worker output and the summary line stay in sync.
effective_layers="$LAYERS"
if [ -z "${!KEY_VAR:-}" ] && [[ "$LAYERS" == *ai* ]]; then
  effective_layers="$(echo "$LAYERS" | sed -E 's/,?ai,?/,/g; s/^,//; s/,$//')"
  [ -z "$effective_layers" ] && effective_layers="structure"
  echo "::warning::$KEY_VAR not set; running layers=$effective_layers"
fi

fail=0
verified=0
skipped=0
# pr-diff scope: when set and the file is non-empty, only validate
# workers whose dir contains at least one changed file. The render
# step still runs against every matched worker (cheap, local-only),
# so rendered-artifact drift detection isn't affected.
if [ -n "${SKV_CHANGED_FILES:-}" ] && [ ! -s "${SKV_CHANGED_FILES:-/nonexistent}" ]; then
  unset SKV_CHANGED_FILES
fi
for manifest in $WORKERS_GLOB; do
  dir="$(dirname "$manifest")"
  # `docs/intro.md` is the minimum required partial for the renderer.
  # A worker that ships only skill bundles or hand-authored docs (e.g.
  # `harness/docs/iii-skill.md`) without `docs/intro.md` has opted out
  # of the rendered-partials pattern — skip it rather than fail to
  # render.
  if [ ! -f "$dir/docs/intro.md" ]; then
    echo "::notice::skipping $dir (no docs/intro.md — worker isn't using the renderer)"
    skipped=$((skipped + 1))
    continue
  fi
  if [ -n "${SKV_CHANGED_FILES:-}" ]; then
    if ! grep -q "^$dir/" "$SKV_CHANGED_FILES"; then
      echo "::notice::pr-diff scope: skipping $dir (no changed files under it)"
      skipped=$((skipped + 1))
      continue
    fi
  fi
  echo "::group::$dir"
  if ! "$BIN" verify-rendered "$dir"; then
    fail=1
  fi
  if ! "$BIN" verify "$dir" --layers "$effective_layers"; then
    fail=1
  fi
  echo "::endgroup::"
  verified=$((verified + 1))
done

total=$((verified + skipped))
if [ "$total" -eq 0 ]; then
  echo "::warning::no manifests matched glob '$WORKERS_GLOB'"
else
  echo
  echo "$verified verified, $skipped skipped (no docs/)"
  echo "layers ran: $effective_layers"
fi

exit $fail
