#!/usr/bin/env bash
# annotate.sh — convert iii-skill-check / verify.sh output into GitHub
# Actions inline annotations on the PR's "Files changed" tab.
#
# Usage: annotate.sh <log-file>
# Reads `<path>:<line> — <message>` violation lines from the log and
# emits matching `::error file=<path>,line=<line>::<message>` workflow
# commands. Other lines are ignored.

set -euo pipefail

input="${1:?missing log-file arg}"

# The em-dash in the validator's output is U+2014; the regex matches a
# literal em-dash followed by space.
grep -E '^[^[:space:]][^:]+:[0-9]+ — ' "$input" | while IFS= read -r line; do
  head="${line%% — *}"
  msg="${line#* — }"
  path="${head%:*}"
  lineno="${head##*:}"
  printf '::error file=%s,line=%s::%s\n' "$path" "$lineno" "$msg"
done
