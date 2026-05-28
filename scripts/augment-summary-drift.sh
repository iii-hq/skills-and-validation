#!/usr/bin/env bash
# augment-summary-drift.sh — append a drift-status block to the verify
# summary so the sticky PR comment reflects drift outcomes that the
# verify step itself didn't see (drift on artifacts outside the pr-diff
# scope).
#
# Usage: augment-summary-drift.sh <body-file> <write-mode> <commit-outcome> <pr-is-fork>
#
# write-mode:      `true` if the consumer opted into auto-commit, else `false`
# commit-outcome:  `success` / `failure` / `skipped` from the commit step
# pr-is-fork:      `true` for cross-repo PRs (CI cannot push back) else `false`
#
# Four variants in priority order:
#   1. write:true + commit succeeded → CI committed the re-render
#   2. write:true + commit failed    → auto-commit failed
#   3. write:false + fork PR         → can't push to fork; re-render locally
#   4. write:false + same-repo PR    → checkbox to trigger a re-render run
#
# The checkbox label is matched verbatim by the issue_comment listener
# (recheck-on-comment.yml). If you edit it here, edit there too.

set -euo pipefail

body="${1:?missing body-file arg}"
write="${2:-false}"
commit_outcome="${3:-}"
fork="${4:-false}"

[ -f "$body" ] || { echo "augment-summary: body file not found: $body" >&2; exit 0; }

{
  printf '\n'
  printf '> [!CAUTION]\n'
  printf '> **Rendered artifacts are out of date** — sources changed without re-rendering.\n'
} >> "$body"

if [ "$write" = "true" ] && [ "$commit_outcome" = "success" ]; then
  {
    printf '> CI committed the re-render to this branch. **Run `git pull` to sync your local copy** before pushing further commits.\n'
  } >> "$body"
elif [ "$write" = "true" ]; then
  {
    printf '> Auto-commit of the re-render failed — see the workflow logs.\n'
  } >> "$body"
elif [ "$fork" = "true" ]; then
  {
    printf '>\n'
    printf '> **Fork PR:** GitHub does not let base-repo CI push to forks. Pull this branch locally, run `iii-skill-render <target> --write`, and push the resulting commit.\n'
  } >> "$body"
else
  {
    printf '\n'
    printf -- '- [ ] **Re-render this branch and commit rendered artifacts**\n'
    printf '\n'
    printf '> Check the box above to trigger a workflow run that re-renders and pushes a commit with the artifacts.\n'
    printf '> This will add an additional commit on this branch — run `git pull` to sync your local copy before making further changes.\n'
  } >> "$body"
fi
