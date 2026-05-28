#!/usr/bin/env bash
# augment-summary-drift.sh — append a drift-status block to the verify
# summary so the sticky PR comment reflects drift outcomes that the
# verify step itself didn't see (drift on artifacts outside the pr-diff
# scope).
#
# Usage: augment-summary-drift.sh <body-file> <write-mode> <commit-status> <pr-is-fork>
#
# write-mode:     `true` if the consumer opted into auto-commit, else `false`
# commit-status:  explicit signal from the action's commit step:
#                   `success`  → committed and pushed
#                   `noop`     → drift outside the scoped add-set, nothing committed
#                   `failure`  → tried to push, gave up after retries
#                   (empty)    → commit step was skipped (verify failed, or write:false)
# pr-is-fork:     `true` for cross-repo PRs (CI cannot push back) else `false`
#
# Branching, in priority order:
#   1. write:true + committed=success → CI committed; pull locally to sync
#   2. write:true + committed=noop    → drift outside scope; nothing pushed
#   3. write:true + committed=failure → auto-commit attempted but push failed
#   4. write:true + committed empty   → commit step was skipped (verify
#                                       failed first); fix verify and the
#                                       next push will re-render
#   5. write:false + fork PR          → can't push to forks; re-render locally
#   6. write:false + same-repo PR     → checkbox to trigger a re-render run
#
# The checkbox label is matched verbatim by the issue_comment listener
# (recheck-on-comment.yml). If you edit it here, edit there too.

set -euo pipefail

body="${1:?missing body-file arg}"
write="${2:-false}"
committed="${3:-}"
fork="${4:-false}"

[ -f "$body" ] || { echo "augment-summary: body file not found: $body" >&2; exit 0; }

{
  printf '\n'
  printf '> [!CAUTION]\n'
  printf '> **Rendered artifacts are out of date** — sources changed without re-rendering.\n'
} >> "$body"

if [ "$write" = "true" ]; then
  case "$committed" in
    success)
      printf '> CI committed the re-render to this branch. **Run `git pull` to sync your local copy** before pushing further commits.\n' >> "$body"
      ;;
    noop)
      printf '> Drift was outside the scoped auto-commit paths (typically untracked files) — nothing was pushed. See the workflow logs.\n' >> "$body"
      ;;
    failure)
      printf '> Auto-commit was attempted but the push failed — see the workflow logs.\n' >> "$body"
      ;;
    "")
      printf '> Auto-commit was skipped because verify failed first. Fix the errors above and the next push will re-render.\n' >> "$body"
      ;;
    *)
      printf '> Unknown commit status (`%s`) — see the workflow logs.\n' "$committed" >> "$body"
      ;;
  esac
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
