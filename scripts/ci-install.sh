#!/usr/bin/env bash
# ci-install.sh — fetch + unpack a skills-and-validation release tarball.
#
# Usage: ci-install.sh <version> <dest-dir>
#
# Tries anonymous GitHub Releases download first. Only when the request
# fails for an authentication-shaped reason (HTTP 401/403/404) does it
# fall back to `gh` or $GITHUB_TOKEN. Other failures (5xx, network,
# DNS, timeout) error out without attempting auth.
#
# After a successful download, the tarball is extracted to <dest-dir>:
#   <dest-dir>/bin/iii-skill-check
#   <dest-dir>/bin/iii-skill-render
#   <dest-dir>/content/{project-rules,styles,skills,.vale.ini}
#   <dest-dir>/templates/.skill-check.yaml
#   <dest-dir>/VERSION

set -euo pipefail

VERSION="${1:?missing version arg}"
DEST="${2:?missing dest-dir arg}"
REPO="iii-hq/skills-and-validation"

# --- resolve floating refs to a concrete X.Y.Z -----------------------------
# Consumers can pass:
#   "latest"  → resolve via /releases/latest (the most recent stable release)
#   "0"       → highest 0.*.* release
#   "0.1"     → highest 0.1.* release
#   "0.1.5"   → used as-is (matches the X.Y.Z regex below)
# Anything matching X.Y.Z (with optional pre-release suffix) is used as-is.
if [ "$VERSION" = "latest" ]; then
  echo "resolving 'latest' from $REPO releases..." >&2
  api="https://api.github.com/repos/${REPO}/releases/latest"
  http=$(curl -sL -o /tmp/skv-releases.json -w '%{http_code}' "$api" 2>/dev/null || echo "000")
  if [ "$http" = "401" ] || [ "$http" = "403" ] || [ "$http" = "404" ]; then
    if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
      gh api "repos/${REPO}/releases/latest" > /tmp/skv-releases.json
      http=200
    elif [ -n "${GITHUB_TOKEN:-}" ]; then
      http=$(curl -sL -o /tmp/skv-releases.json -w '%{http_code}' \
        -H "Authorization: Bearer ${GITHUB_TOKEN}" "$api")
    fi
  fi
  if [ "$http" != "200" ]; then
    echo "ERROR: GitHub API returned $http resolving 'latest' from $REPO" >&2
    exit 1
  fi
  resolved=$(grep -E '"tag_name"' /tmp/skv-releases.json \
    | sed -E 's/.*"tag_name": *"v([0-9]+\.[0-9]+\.[0-9]+)".*/\1/' \
    | head -1)
  rm -f /tmp/skv-releases.json
  if [ -z "$resolved" ]; then
    echo "ERROR: couldn't parse tag_name from /releases/latest response" >&2
    exit 1
  fi
  echo "resolved to $resolved" >&2
  VERSION="$resolved"
elif [[ "$VERSION" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
  echo "resolving '$VERSION' to latest patch in $REPO..." >&2
  api="https://api.github.com/repos/${REPO}/releases?per_page=100"
  http=$(curl -sL -o /tmp/skv-releases.json -w '%{http_code}' "$api" 2>/dev/null || echo "000")
  if [ "$http" = "401" ] || [ "$http" = "403" ] || [ "$http" = "404" ]; then
    if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
      gh api "repos/${REPO}/releases?per_page=100" > /tmp/skv-releases.json
      http=200
    elif [ -n "${GITHUB_TOKEN:-}" ]; then
      http=$(curl -sL -o /tmp/skv-releases.json -w '%{http_code}' \
        -H "Authorization: Bearer ${GITHUB_TOKEN}" "$api")
    fi
  fi
  if [ "$http" != "200" ]; then
    echo "ERROR: GitHub API returned $http while listing releases of $REPO" >&2
    exit 1
  fi
  resolved=$(grep -E '"tag_name"' /tmp/skv-releases.json \
    | sed -E 's/.*"tag_name": *"v([0-9]+\.[0-9]+\.[0-9]+)".*/\1/' \
    | grep -E "^${VERSION}\\.[0-9]+$" \
    | sort -V \
    | tail -1)
  rm -f /tmp/skv-releases.json
  if [ -z "$resolved" ]; then
    echo "ERROR: no release matching v${VERSION}.* in $REPO" >&2
    exit 1
  fi
  echo "resolved to $resolved" >&2
  VERSION="$resolved"
fi

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
