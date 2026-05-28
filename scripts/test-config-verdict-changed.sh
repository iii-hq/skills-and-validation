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
  # Don't fold non-{0,1} exits into "scope" — an internal SUT error
  # (usage failure, awk syntax error, etc.) should fail the test loudly,
  # not silently pass cases that happen to expect "scope".
  set +e
  "$SUT" "$TMP/base.yaml" "$TMP/head.yaml"
  rc=$?
  set -e
  case "$rc" in
    0) got="bail" ;;
    1) got="scope" ;;
    *) echo "FAIL — $name: SUT exited unexpectedly with rc=$rc"
       fails=$((fails + 1)); return ;;
  esac
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

# Exclude-only edit where the key line carries a trailing `# comment`.
# The block-form regex must still recognise this as the include/exclude
# block so the items below are stripped from comparison.
BASE_TRAILING='version: 2
mode: docs
docs:
  include:
    - "**/*.md"
  exclude: # parked tutorials
    - "**/CHANGELOG.md"
ai_check:
  provider: anthropic
  model: claude-sonnet-4-6'
HEAD_TRAILING="${BASE_TRAILING/    - \"**\/CHANGELOG.md\"/    - \"**/CHANGELOG.md\"
    - \"docs/extra/**\"}"
run "add path under exclude key with trailing comment" scope "$BASE_TRAILING" "$HEAD_TRAILING"

echo
if [ "$fails" -eq 0 ]; then echo "all passed"; else echo "$fails failed"; exit 1; fi
