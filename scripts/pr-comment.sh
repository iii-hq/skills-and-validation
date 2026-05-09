#!/usr/bin/env bash
# pr-comment.sh — post or update a sticky PR comment with the verify report.
#
# Usage: pr-comment.sh <pr-number> <body-file> [config-path]
# Env:   GH_TOKEN              — passed to gh; the action sets this from
#                                 ${{ github.token }}.
#        GITHUB_REPOSITORY     — provided automatically by the runner.
#
# A hidden HTML marker pins our previous comment so subsequent runs edit
# it in place instead of stacking new comments. The marker is keyed by
# `config-path` so consumers running the action multiple times in one
# workflow (e.g. via matrix to validate worker + docs side-by-side) get
# one sticky comment per config rather than the runs clobbering each
# other. Skipped silently when the token lacks pull-requests:write — the
# caller pairs this script with `continue-on-error: true`.

set -euo pipefail

PR="${1:?missing PR number}"
BODY_FILE="${2:?missing body-file arg}"
CONFIG_PATH="${3:-.skill-check.yaml}"
MARKER="<!-- skill-check-status-comment:${CONFIG_PATH} -->"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI not available; skipping PR comment" >&2
  exit 0
fi

final=$(mktemp)
{ echo "$MARKER"; cat "$BODY_FILE"; } > "$final"

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
