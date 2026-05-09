#!/usr/bin/env bash
# install.sh — install the skills-and-validation binaries locally.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/iii-hq/skills-and-validation/latest/scripts/install.sh | bash
#   ./scripts/install.sh                 # install latest
#   ./scripts/install.sh 0.1              # install latest 0.1.x
#   ./scripts/install.sh 0.1.3            # install exactly 0.1.3
#
# Layout:
#   $SKV_DIR / <version> / { bin/, content/, templates/, VERSION }
#   $SKV_DIR / current   -> <version>     (symlink, repointed on every install)
#   $SKV_BIN / iii-skill-{check,render}   -> $SKV_DIR/current/bin/*
#
# Defaults:
#   SKV_DIR  ~/.local/share/skill-check
#   SKV_BIN  ~/.local/bin

set -euo pipefail

VERSION="${1:-latest}"
REPO="iii-hq/skills-and-validation"
SKV_DIR="${SKV_DIR:-$HOME/.local/share/skill-check}"
SKV_BIN="${SKV_BIN:-$HOME/.local/bin}"

# --- detect target triple --------------------------------------------------
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)
    TRIPLE="aarch64-apple-darwin" ;;
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
    echo "ERROR: unsupported platform $(uname -s)-$(uname -m)" >&2
    exit 1 ;;
esac

# --- resolve version -------------------------------------------------------
if [ "$VERSION" = "latest" ]; then
  echo "resolving 'latest'..." >&2
  VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | head -1 \
    | sed -E 's/.*"tag_name": *"v([0-9]+\.[0-9]+\.[0-9]+)".*/\1/')
  if [ -z "$VERSION" ]; then
    echo "ERROR: could not resolve 'latest' from $REPO" >&2
    exit 1
  fi
elif [[ "$VERSION" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
  echo "resolving '$VERSION' to latest patch..." >&2
  VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases?per_page=100" \
    | grep '"tag_name"' \
    | sed -E 's/.*"tag_name": *"v([0-9]+\.[0-9]+\.[0-9]+)".*/\1/' \
    | grep -E "^${VERSION}\\.[0-9]+$" \
    | sort -V \
    | tail -1)
  if [ -z "$VERSION" ]; then
    echo "ERROR: no release matching v${1}.* in $REPO" >&2
    exit 1
  fi
fi
echo "installing skills-and-validation v${VERSION} for ${TRIPLE}..." >&2

# --- download and extract --------------------------------------------------
ASSET="skills-and-validation-${VERSION}-${TRIPLE}.tar.gz"
URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET}"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

curl -fsSL "$URL" -o "$TMP/$ASSET" || {
  echo "ERROR: download failed: $URL" >&2
  exit 1
}

mkdir -p "$SKV_DIR/$VERSION"
tar -xzf "$TMP/$ASSET" -C "$SKV_DIR/$VERSION" --strip-components=1
chmod +x "$SKV_DIR/$VERSION/bin/iii-skill-check" "$SKV_DIR/$VERSION/bin/iii-skill-render"

# --- repoint `current` symlink + bin shims --------------------------------
rm -f "$SKV_DIR/current"
ln -s "$VERSION" "$SKV_DIR/current"

mkdir -p "$SKV_BIN"
ln -sf "$SKV_DIR/current/bin/iii-skill-check"  "$SKV_BIN/iii-skill-check"
ln -sf "$SKV_DIR/current/bin/iii-skill-render" "$SKV_BIN/iii-skill-render"

# Invalidate the runtime update-check cache so the next run hits the GitHub
# API rather than reporting the previously-installed version as "latest".
CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/skill-check"
rm -f "$CACHE_DIR/update-check.json"

echo "" >&2
echo "Installed to $SKV_DIR/$VERSION" >&2
echo "Symlinks: $SKV_BIN/iii-skill-{check,render}" >&2
echo "" >&2
case ":$PATH:" in
  *:"$SKV_BIN":*) ;;
  *)
    echo "Add $SKV_BIN to your PATH:" >&2
    echo "  export PATH=\"$SKV_BIN:\$PATH\"" >&2
    echo "" >&2 ;;
esac
echo "Next: install the pre-commit hook in your worker repo:" >&2
echo "  $SKV_DIR/current/scripts/install-hook.sh" >&2
