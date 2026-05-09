#!/usr/bin/env bash
# verify-docs.sh — run iii-skill-check against doc files matching a glob.
#
# Usage: verify-docs.sh <docs-glob> <layers>
# Env:   INSTALL_DIR  Path to the extracted skills-and-validation bundle
#                     (defaults to $PWD/.skill-check). The binary is at
#                     $INSTALL_DIR/bin/iii-skill-check; bundle-adjacent
#                     content lookup walks up from there.
#
# Mirror of scripts/verify-workers.sh: iterate the matching files and
# call `iii-skill-check verify <file>` per match. The binary walks up
# from each file to find the controlling .skill-check.yaml and reads
# its `mode` field. The action's `docs-glob` input controls scope at
# the workflow level; .skill-check.yaml's `docs.include`/`docs.exclude`
# control scope at the repo level. The two compose — both must accept
# a doc for it to be checked.
#
# globstar is enabled so consumers can use `**/*.mdx` to walk arbitrary
# subdirectory depth.

set -euo pipefail
shopt -s nullglob
if (( BASH_VERSINFO[0] >= 4 )); then
  shopt -s globstar
fi

DOCS_GLOB="${1:-**/*.md **/*.mdx}"
LAYERS="${2:-structure,vale,ai}"

INSTALL_DIR="${INSTALL_DIR:-$PWD/.skill-check}"
BIN="$INSTALL_DIR/bin/iii-skill-check"
if [ ! -x "$BIN" ]; then
  echo "ERROR: iii-skill-check not found at $BIN — did you run ci-install.sh first?" >&2
  exit 1
fi

# Honour `api_key_env_var` from the nearest .skill-check.yaml. We don't
# know the docs root up front, so look in the cwd; the binary itself
# walks up if needed.
KEY_VAR="$(awk '/^[[:space:]]*api_key_env_var:/ {print $2; exit}' .skill-check.yaml 2>/dev/null || true)"
KEY_VAR="${KEY_VAR:-ANTHROPIC_API_KEY}"

effective_layers="$LAYERS"
if [ -z "${!KEY_VAR:-}" ] && [[ "$LAYERS" == *ai* ]]; then
  effective_layers="$(echo "$LAYERS" | sed -E 's/,?ai,?/,/g; s/^,//; s/,$//')"
  [ -z "$effective_layers" ] && effective_layers="structure"
  echo "::warning::$KEY_VAR not set; running layers=$effective_layers"
fi

fail=0
verified=0
skipped=0
for doc in $DOCS_GLOB; do
  # Skip rendered skill outputs.
  case "$doc" in
    *.skill.md) continue ;;
  esac
  [ -f "$doc" ] || continue

  echo "::group::$doc"
  if ! "$BIN" verify-rendered "$doc"; then
    fail=1
  fi
  if ! "$BIN" verify "$doc" --layers "$effective_layers"; then
    fail=1
  fi
  echo "::endgroup::"
  verified=$((verified + 1))
done

total=$((verified + skipped))
if [ "$total" -eq 0 ]; then
  echo "::warning::no docs matched glob '$DOCS_GLOB'"
else
  echo
  echo "$verified verified, $skipped skipped"
  echo "layers ran: $effective_layers"
fi

exit $fail
