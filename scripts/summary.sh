#!/usr/bin/env bash
# summary.sh — emit a markdown report of skill-check results.
#
# Usage: summary.sh <log-file> [mode-label]
# Output goes to stdout — the caller redirects to $GITHUB_STEP_SUMMARY,
# pipes into the PR-comment body, etc. `mode-label` (optional) is
# appended to the title so consumers running the action multiple times
# in one workflow (matrix over modes) can tell their sticky comments
# apart at a glance.

set -euo pipefail

input="${1:?missing log-file arg}"
MODE_LABEL="${2:-}"

# Violation lines are `<path>:~<line>:<severity> — <message>` with
# severity ∈ {error, warning} and an optional `~` prefix on the line
# number tagging it as "approximate" (rendered→source map). Count each
# severity separately so warnings can surface without dragging the
# overall status into "failed".
errors=$(grep -cE '^[^[:space:]][^:]+:~?[0-9]+:error — ' "$input" || true)
warnings=$(grep -cE '^[^[:space:]][^:]+:~?[0-9]+:warning — ' "$input" || true)
total=$((errors + warnings))
counts=$(grep -E '^[0-9]+ verified, [0-9]+ skipped' "$input" | tail -1 || true)
layers=$(grep -E '^layers ran:' "$input" | tail -1 | sed -E 's/^layers ran: *//' || true)

if [ -n "$MODE_LABEL" ]; then
  echo "## skill-check — $MODE_LABEL"
else
  echo "## skill-check"
fi
echo
if [ -n "$counts" ]; then
  echo "$counts."
  echo
fi

if [ "$total" -eq 0 ]; then
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

# Headline: errors first (they fail the run), warnings as context.
header_parts=()
if [ "$errors" -gt 0 ]; then
  header_parts+=("$errors error$([ "$errors" -eq 1 ] || echo s)")
fi
if [ "$warnings" -gt 0 ]; then
  header_parts+=("$warnings warning$([ "$warnings" -eq 1 ] || echo s)")
fi
# IFS only uses its first character to join `[*]` expansions, so join
# manually with ", " to keep "N errors, M warnings" readable.
header=""
for part in "${header_parts[@]}"; do
  if [ -z "$header" ]; then
    header="$part"
  else
    header="$header, $part"
  fi
done
echo "$header across the verified workers."
echo
echo "| File | Approximate line | Severity | Violation |"
echo "| --- | --- | --- | --- |"
grep -E '^[^[:space:]][^:]+:~?[0-9]+:(error|warning) — ' "$input" | while IFS= read -r line; do
  head="${line%% — *}"
  msg="${line#* — }"
  severity="${head##*:}"
  rest="${head%:*}"
  path="${rest%:*}"
  lineno="${rest##*:}"
  # The line number carries a leading `~` (approximate-line tag from
  # source-map translation). Keep it in the cell so a reader sees the
  # framing without needing to recall what "Approximate line" means.
  msg_escaped="${msg//|/\\|}"
  printf '| `%s` | %s | %s | %s |\n' "$path" "$lineno" "$severity" "$msg_escaped"
done
