#!/usr/bin/env bash
# config-verdict-changed.sh — decide whether a change to a `.skill-check.yaml`
# affects validation *verdicts* (so pr-diff must fall back to a full scan) or
# is confined to scope keys that only widen/narrow the file set.
#
# Usage: config-verdict-changed.sh <base-yaml> <head-yaml>
# Exit:  0 = verdict-affecting change — caller should bail to full scan
#        1 = change confined to docs.include / docs.exclude — stay scoped
#
# Why this split exists: rules, model, system_prompt, ai_check settings and
# mode change what the validator *decides* about a file, so editing them can
# flip the verdict of files the PR never touched — those must be re-checked.
# But docs.include / docs.exclude only add or remove paths from scope; they
# can never change the verdict of a file that stays in scope. Bailing to a
# full corpus scan on an exclude-path tweak is what made unrelated,
# pre-existing docs get flagged nondeterministically by the AI layer.
#
# Strategy: strip the docs.include and docs.exclude blocks from both files,
# then compare the remainder. Identical remainder => only scope changed.
# A missing file is treated as empty content (covers add/delete of a config).

set -euo pipefail

base="${1:?usage: config-verdict-changed.sh <base-yaml> <head-yaml>}"
head="${2:?usage: config-verdict-changed.sh <base-yaml> <head-yaml>}"

# Drop the `include:` / `exclude:` keys nested under `docs:` along with their
# list items (block form) or inline flow value, keeping every other line
# verbatim. Indentation-aware so a key at the same or shallower indent than
# the block ends it.
strip_scope() {
  # `cat` tolerates a missing path by reading nothing when we pass /dev/null
  # for absent files (see callers below).
  awk '
    {
      match($0, /^[ ]*/); ind = RLENGTH
      if (inblock) {
        if ($0 ~ /^[ ]*$/) next          # blank line inside the block: drop
        if (ind > blockind) next         # list item / nested mapping: drop
        inblock = 0                      # dedented: block ended, fall through
      }
      if ($0 ~ /^[ ]+(include|exclude):[ ]*$/) { inblock = 1; blockind = ind; next }
      if ($0 ~ /^[ ]+(include|exclude):[ ]*\[/) next   # inline flow list on one line
      print
    }
  ' "$1"
}

# Resolve each side to a real file (empty when absent) so awk never errors.
base_src="$base"; [ -f "$base" ] || base_src=/dev/null
head_src="$head"; [ -f "$head" ] || head_src=/dev/null

if diff -q <(strip_scope "$base_src") <(strip_scope "$head_src") >/dev/null; then
  exit 1   # only scope (include/exclude) changed — safe to stay scoped
fi
exit 0     # something verdict-affecting changed — bail to full scan
