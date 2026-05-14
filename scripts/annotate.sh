#!/usr/bin/env bash
# annotate.sh — convert iii-skill-check / verify.sh output into GitHub
# Actions inline annotations on the PR's "Files changed" tab.
#
# Usage: annotate.sh <log-file>
# Reads `<path>:<line>:<severity> — <message>` violation lines from the
# log and emits matching `::error file=...` or `::warning file=...`
# workflow commands. <severity> is `error` or `warning`. Other lines
# are ignored.

set -euo pipefail

input="${1:?missing log-file arg}"

# The em-dash in the validator's output is U+2014; the regex matches a
# literal em-dash followed by space. Severity (error|warning) sits
# between the line number and the dash. Line numbers carry a leading
# `~` (approximate-line tag from source-map translation) — strip it
# before emitting the annotation since GitHub's `line=` parameter
# expects a bare integer.
grep -E '^[^[:space:]][^:]+:~?[0-9]+:(error|warning) — ' "$input" | while IFS= read -r line; do
  head="${line%% — *}"
  msg="${line#* — }"
  severity="${head##*:}"
  rest="${head%:*}"
  path="${rest%:*}"
  lineno="${rest##*:}"
  # Drop the optional `~` prefix from the line number for the
  # annotation command; the displayed message still indicates the
  # "approximate" framing via the column header in the PR comment.
  lineno="${lineno#~}"
  printf '::%s file=%s,line=%s::%s\n' "$severity" "$path" "$lineno" "$msg"
done
