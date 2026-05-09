#!/usr/bin/env bash
# summary.sh — emit a markdown report of skill-check results.
#
# Usage: summary.sh <log-file>
# Output goes to stdout — the caller redirects to $GITHUB_STEP_SUMMARY,
# pipes into the PR-comment body, etc.

set -euo pipefail

input="${1:?missing log-file arg}"

violations=$(grep -cE '^[^[:space:]][^:]+:[0-9]+ — ' "$input" || true)
counts=$(grep -E '^[0-9]+ verified, [0-9]+ skipped' "$input" | tail -1 || true)
layers=$(grep -E '^layers ran:' "$input" | tail -1 | sed -E 's/^layers ran: *//' || true)

echo "## skill-check"
echo
if [ -n "$counts" ]; then
  echo "$counts."
  echo
fi

if [ "$violations" -eq 0 ]; then
  # Per-layer checklist (whichever layers actually ran)
  layer_count=0
  if [ -n "$layers" ]; then
    echo "| Layer     | Result |"
    echo "| --------- | ------ |"
    IFS=',' read -ra layer_arr <<< "$layers"
    for layer in "${layer_arr[@]}"; do
      layer="${layer// /}"
      [ -z "$layer" ] && continue
      printf '| %-9s | ✓      |\n' "$layer"
      layer_count=$((layer_count + 1))
    done
    echo
  fi

  # "Three for three. Nicely done." style closer, scaled to the actual
  # number of layers that ran.
  case "$layer_count" in
    3) echo "Three for three. Nicely done." ;;
    2) echo "Two for two. Nicely done." ;;
    *) echo "Nicely done." ;;
  esac
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
  msg_escaped="${msg//|/\\|}"
  printf '| `%s` | %s | %s |\n' "$path" "$lineno" "$msg_escaped"
done
