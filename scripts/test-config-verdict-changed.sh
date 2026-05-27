#!/usr/bin/env bash
# test-config-verdict-changed.sh — unit tests for the pr-diff config bailout
# decision. Pure (no network/gh): feeds base/head yaml pairs to
# config-verdict-changed.sh and asserts bail (exit 0) vs stay-scoped (exit 1).

set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SUT="$HERE/config-verdict-changed.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
fails=0

# run <name> <expected: bail|scope> <base-content> <head-content>
run() {
  local name="$1" expect="$2" base="$3" head="$4"
  printf '%s' "$base" > "$TMP/base.yaml"
  printf '%s' "$head" > "$TMP/head.yaml"
  if "$SUT" "$TMP/base.yaml" "$TMP/head.yaml"; then got="bail"; else got="scope"; fi
  if [ "$got" = "$expect" ]; then
    echo "ok   — $name ($got)"
  else
    echo "FAIL — $name: expected $expect, got $got"; fails=$((fails + 1))
  fi
}

BASE='version: 2
mode: docs
docs:
  include:
    - "docs/**/*.md"
    - "docs/**/*.mdx"
  exclude:
    - "docs/0-10-0/**"
    - "docs/changelog/**"
ai_check:
  provider: anthropic
  model: claude-sonnet-4-6
  max_tokens: 6000'

# PR #1701: a single added exclude path. Must stay scoped.
run "add one exclude path" scope "$BASE" "${BASE/  exclude:
    - \"docs\/0-10-0\/**\"/  exclude:
    - \"docs/tutorials/linkly/_workspace/**\"
    - \"docs/0-10-0/**\"}"

# Add an include path. Must stay scoped.
run "add include path" scope "$BASE" "${BASE/    - \"docs\/**\/*.mdx\"/    - \"docs/**/*.mdx\"
    - \"docs/**/*.markdown\"}"

# Change the model. Must bail.
run "change model" bail "$BASE" "${BASE/claude-sonnet-4-6/claude-opus-4-7}"

# Change max_tokens. Must bail.
run "change max_tokens" bail "$BASE" "${BASE/6000/8000}"

# Flip mode. Must bail.
run "change mode" bail "$BASE" "${BASE/mode: docs/mode: worker}"

# No-op (identical). Confined-change path => stay scoped.
run "identical content" scope "$BASE" "$BASE"

# Brand-new config (empty base). Establishes rules => bail.
run "new config file" bail "" "$BASE"

# Deleted config (empty head). Removing rules => bail.
run "deleted config file" bail "$BASE" ""

# Both exclude edit AND model edit. Verdict key wins => bail.
run "scope + verdict change" bail "$BASE" "${BASE/claude-sonnet-4-6/claude-opus-4-7}
    - \"docs/extra/**\""

echo
if [ "$fails" -eq 0 ]; then echo "all passed"; else echo "$fails failed"; exit 1; fi
