#!/usr/bin/env bash
# augment-summary-drift.sh — append a drift-status block to the verify
# summary so the sticky PR comment reflects drift outcomes that the
# verify step itself didn't see (drift on artifacts outside the pr-diff
# scope).
#
# Usage: augment-summary-drift.sh <body-file> <write-mode> <commit-status> <pr-is-fork>
#
# Positional args:
#   <body-file>       summary markdown file to append to
#   <write-mode>      `true` if the consumer opted into auto-commit, else `false`
#   <commit-status>   explicit signal from the action's commit step:
#                       `success`  → committed and pushed
#                       `noop`     → drift outside the scoped add-set, nothing committed
#                       `failure`  → tried to push, gave up after retries
#                       (empty)    → commit step was skipped (verify failed, or write:false)
#   <pr-is-fork>      `true` for cross-repo PRs (CI cannot push back) else `false`
#
# Env (provided by the action via the augment-summary step):
#   DRIFT_IN_SCOPE_COUNT       integer
#   DRIFT_OUT_OF_SCOPE_COUNT   integer
#   DRIFT_IN_SCOPE_PATHS       path to file listing in-scope dirty paths
#   DRIFT_OUT_OF_SCOPE_PATHS   path to file listing out-of-scope dirty paths
#
# Scope split (the meaningful UX dimension):
#   - in-scope drift  = artifacts under a worker/doc this PR touches.
#                       Treated as a hard error in read-only mode.
#   - out-of-scope drift = pre-existing stale artifacts on main, unrelated
#                          to this PR. Treated as a warning so unrelated
#                          PRs don't block on it.
#
# Branching (write:true cases use the commit-step's `committed` signal,
# write:false cases use the in/out scope split):
#
#   write:true
#     committed=success → CI committed the re-render. Mentions both scope
#                         buckets if both are non-zero.
#     committed=noop    → nothing in scope to commit.
#     committed=failure → push failed.
#     committed empty   → commit step skipped (verify failed first).
#
#   write:false, fork PR     → can't push to forks; re-render locally.
#   write:false, same-repo
#     in-scope > 0 → ERROR block, checkbox to trigger re-render.
#                    Also lists out-of-scope drift inline if present.
#     in-scope = 0, out-of-scope > 0 → NOTE block (not an error), lists
#                                       the stale paths and recommends a
#                                       follow-up chore PR.
#
# The checkbox label is matched verbatim by the issue_comment listener
# (recheck-on-comment.yml). If you edit it here, edit there too.

set -euo pipefail

body="${1:?missing body-file arg}"
write="${2:-false}"
committed="${3:-}"
fork="${4:-false}"

in_n="${DRIFT_IN_SCOPE_COUNT:-0}"
out_n="${DRIFT_OUT_OF_SCOPE_COUNT:-0}"
in_paths="${DRIFT_IN_SCOPE_PATHS:-}"
out_paths="${DRIFT_OUT_OF_SCOPE_PATHS:-}"

[ -f "$body" ] || { echo "augment-summary: body file not found: $body" >&2; exit 0; }

# Render a markdown bullet list from a paths file, truncating after MAX.
# Outputs nothing if the file is empty or missing.
render_paths() {
  local f="$1" max="${2:-12}"
  [ -n "$f" ] && [ -s "$f" ] || return 0
  local n; n=$(wc -l < "$f" | tr -d ' ')
  awk -v max="$max" 'NF{n++; if (n<=max) printf "> - `%s`\n", $0}' "$f"
  if [ "$n" -gt "$max" ]; then
    printf '> - …and %d more (see the workflow logs)\n' "$((n - max))"
  fi
}

# write:true → auto-commit path. The committed status determines copy.
if [ "$write" = "true" ]; then
  {
    printf '\n'
    printf '> [!CAUTION]\n'
    printf '> **Rendered artifacts were out of date** — sources had changed without re-rendering.\n'
  } >> "$body"

  case "$committed" in
    success)
      {
        if [ "$out_n" -gt 0 ] && [ "$in_n" -gt 0 ]; then
          printf '> CI committed the re-render of %d in-scope and %d out-of-scope artifact(s) to this branch. **Run `git pull` to sync your local copy** before pushing further commits.\n' "$in_n" "$out_n"
        elif [ "$out_n" -gt 0 ]; then
          printf '> CI committed the re-render of %d out-of-scope artifact(s) (stale on main, unrelated to this PR) to this branch. **Run `git pull` to sync your local copy** before pushing further commits.\n' "$out_n"
        else
          printf '> CI committed the re-render of %d in-scope artifact(s) to this branch. **Run `git pull` to sync your local copy** before pushing further commits.\n' "$in_n"
        fi
      } >> "$body"
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
  exit 0
fi

# write:false + fork PR — CI can't push to forks.
if [ "$fork" = "true" ]; then
  {
    printf '\n'
    printf '> [!CAUTION]\n'
    printf '> **Rendered artifacts are out of date** — sources changed without re-rendering.\n'
    printf '>\n'
    printf '> **Fork PR:** GitHub does not let base-repo CI push to forks. Pull this branch locally, run `iii-skill-render <target> --write`, and push the resulting commit.\n'
  } >> "$body"
  exit 0
fi

# write:false + same-repo PR. The scope split drives the message.
if [ "$in_n" -gt 0 ]; then
  # Hard error variant: in-scope drift is what blocks the PR.
  {
    printf '\n'
    printf '> [!CAUTION]\n'
    printf '> **%d in-scope rendered artifact(s) are out of date** — sources this PR touches changed without re-rendering.\n' "$in_n"
    render_paths "$in_paths"
    printf '\n'
    printf -- '- [ ] **Re-render this branch and commit rendered artifacts**\n'
    printf '\n'
    printf '> Check the box above to trigger a workflow run that re-renders and pushes a commit with the artifacts.\n'
    printf '> This will add an additional commit on this branch — run `git pull` to sync your local copy before making further changes.\n'
  } >> "$body"

  # If there's also out-of-scope drift, note it (not blocking).
  if [ "$out_n" -gt 0 ]; then
    {
      printf '\n'
      printf '> [!NOTE]\n'
      printf '> Additionally, **%d unrelated stale artifact(s)** exist on main outside this PR'\''s scope. Not blocking this PR; a maintainer should re-render them in a separate chore PR.\n' "$out_n"
      render_paths "$out_paths"
    } >> "$body"
  fi
  exit 0
fi

# write:false + same-repo + out-of-scope only — non-blocking note.
if [ "$out_n" -gt 0 ]; then
  {
    printf '\n'
    printf '> [!NOTE]\n'
    printf '> **%d stale rendered artifact(s) detected on main, unrelated to this PR.** This PR is fine; the drift was already there. A maintainer should open a chore PR to re-render these.\n' "$out_n"
    render_paths "$out_paths"
  } >> "$body"
  exit 0
fi
