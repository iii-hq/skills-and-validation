#!/usr/bin/env bash
# pr-comment.sh — post or update a sticky PR comment with the verify report.
#
# Usage: pr-comment.sh <pr-number> <body-file>
# Env:   GH_TOKEN              — passed to gh; the action sets this from
#                                 ${{ github.token }}.
#        GITHUB_REPOSITORY     — provided automatically by the runner.
#
# A hidden HTML marker pins our previous comment so subsequent runs edit
# it in place instead of stacking new comments. Skipped silently when
# the token lacks pull-requests:write — the caller pairs this script
# with `continue-on-error: true`.

set -euo pipefail

PR="${1:?missing PR number}"
BODY_FILE="${2:?missing body-file arg}"
MARKER="<!-- skill-check-status-comment -->"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI not available; skipping PR comment" >&2
  exit 0
fi

# Compose the body with the marker pinned to the top.
final=$(mktemp)
{ echo "$MARKER"; cat "$BODY_FILE"; } > "$final"

# Look up an existing skill-check comment on this PR.
existing=$(gh api \
  "repos/${GITHUB_REPOSITORY}/issues/${PR}/comments" \
  --jq ".[] | select(.body | contains(\"$MARKER\")) | .id" \
  | head -1 || true)

if [ -n "$existing" ]; then
  gh api \
    "repos/${GITHUB_REPOSITORY}/issues/comments/${existing}" \
    --method PATCH \
    -f body="$(cat "$final")" \
    >/dev/null
  echo "updated PR comment ${existing}" >&2
else
  gh api \
    "repos/${GITHUB_REPOSITORY}/issues/${PR}/comments" \
    --method POST \
    -f body="$(cat "$final")" \
    >/dev/null
  echo "created new PR comment on #${PR}" >&2
fi
