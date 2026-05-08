#!/usr/bin/env bash
# download.sh — fetch + unpack a skills-and-validation release tarball.
#
# Usage: download.sh <version> <dest-dir>
#
# Tries anonymous GitHub Releases download first. Only when the request
# fails for an authentication-shaped reason (HTTP 401/403/404) does it
# fall back to `gh` or $GITHUB_TOKEN. Other failures (5xx, network,
# DNS, timeout) error out without attempting auth.
#
# After a successful download, the tarball is extracted to <dest-dir>:
#   <dest-dir>/bin/iii-skill-check
#   <dest-dir>/bin/iii-skill-render
#   <dest-dir>/content/{project-rules,styles,iii-skill-authoring,.vale.ini}
#   <dest-dir>/templates/.skill-check.yaml
#   <dest-dir>/VERSION

set -euo pipefail

VERSION="${1:?missing version arg}"
DEST="${2:?missing dest-dir arg}"
REPO="iii-hq/skills-and-validation"

# --- detect target triple --------------------------------------------------
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)
    TRIPLE="aarch64-apple-darwin" ;;
  Darwin-x86_64)
    TRIPLE="x86_64-apple-darwin" ;;
  Linux-x86_64)
    if ldd --version 2>&1 | grep -qi musl; then
      TRIPLE="x86_64-unknown-linux-musl"
    else
      TRIPLE="x86_64-unknown-linux-gnu"
    fi ;;
  Linux-aarch64|Linux-arm64)
    if ldd --version 2>&1 | grep -qi musl; then
      TRIPLE="aarch64-unknown-linux-musl"
    else
      TRIPLE="aarch64-unknown-linux-gnu"
    fi ;;
  *)
    echo "unsupported platform: $(uname -s)-$(uname -m)" >&2
    exit 1 ;;
esac

ASSET="skills-and-validation-${VERSION}-${TRIPLE}.tar.gz"
URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# --- step 1: anonymous download -------------------------------------------
http=$(curl -sL -o "$TMP/$ASSET" -w '%{http_code}' "$URL" || echo "000")

if [ "$http" = "200" ]; then
  : # success — auth path skipped entirely
elif [ "$http" = "401" ] || [ "$http" = "403" ] || [ "$http" = "404" ]; then
  # Authentication-shaped failure. Repo is probably still private.
  echo "Anonymous download returned HTTP $http; trying authenticated download..." >&2
  authed=0
  if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    if gh release download "v${VERSION}" \
         --repo "$REPO" \
         --pattern "$ASSET" \
         --output "$TMP/$ASSET"; then
      authed=1
    fi
  fi
  if [ "$authed" -eq 0 ] && [ -n "${GITHUB_TOKEN:-}" ]; then
    if curl -fsSL -H "Authorization: Bearer ${GITHUB_TOKEN}" -o "$TMP/$ASSET" "$URL"; then
      authed=1
    fi
  fi
  if [ "$authed" -eq 0 ]; then
    cat >&2 <<EOF
ERROR: Couldn't download $ASSET from $REPO (HTTP $http).
The repo may be private. To authenticate:
  gh auth login                        # easiest — uses GitHub CLI
  OR
  export GITHUB_TOKEN=…                # token with read access to $REPO
EOF
    exit 1
  fi
else
  echo "ERROR: download failed (HTTP $http or network error). URL: $URL" >&2
  exit 1
fi

# --- step 2: extract ------------------------------------------------------
mkdir -p "$DEST"
tar -xzf "$TMP/$ASSET" -C "$DEST" --strip-components=1
chmod +x "$DEST/bin/iii-skill-check" "$DEST/bin/iii-skill-render"
echo "skills-and-validation $VERSION installed to $DEST" >&2
