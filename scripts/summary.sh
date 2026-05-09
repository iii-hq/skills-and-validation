#!/usr/bin/env bash
# summary.sh — emit a markdown report of skill-check violations.
#
# Usage: summary.sh <log-file>
# Output goes to stdout — the caller redirects to $GITHUB_STEP_SUMMARY,
# pipes into the PR-comment body, etc.

set -euo pipefail

input="${1:?missing log-file arg}"

violations=$(grep -cE '^[^[:space:]][^:]+:[0-9]+ — ' "$input" || true)

# Verified / skipped totals come from verify.sh's final summary line.
counts=$(grep -E '^[0-9]+ verified, [0-9]+ skipped' "$input" | tail -1 || true)

echo "## skill-check"
echo
if [ -n "$counts" ]; then
  echo "$counts."
  echo
fi

if [ "$violations" -eq 0 ]; then
  echo "All verified workers passed every layer."
  exit 0
fi

echo "$violations violation$([ "$violations" -eq 1 ] || echo s) across the verified workers."
echo
echo "| File | Line | Violation |"
echo "| --- | --- | --- |"
grep -E '^[^[:space:]][^:]+:[0-9]+ — ' "$input" | while IFS= read -r line; do
  head="${line%% — *}"
  msg="${line#* — }"
  path="${head%:*}"
  lineno="${head##*:}"
  # Escape pipes so they don't break the table cell.
  msg_escaped="${msg//|/\\|}"
  printf '| `%s` | %s | %s |\n' "$path" "$lineno" "$msg_escaped"
done
