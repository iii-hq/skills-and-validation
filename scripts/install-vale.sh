#!/usr/bin/env bash
# install-vale.sh — install a pinned Vale binary into a prefix dir.
#
# Usage: install-vale.sh <version> [prefix]
# Default prefix: /usr/local/bin (uses sudo if not writable).

set -euo pipefail

VERSION="${1:?missing vale version arg}"
PREFIX="${2:-/usr/local/bin}"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    ASSET="vale_${VERSION}_Linux_64-bit.tar.gz" ;;
  Linux-arm64|Linux-aarch64)
    ASSET="vale_${VERSION}_Linux_arm64.tar.gz" ;;
  Darwin-arm64)
    ASSET="vale_${VERSION}_macOS_arm64.tar.gz" ;;
  Darwin-x86_64)
    ASSET="vale_${VERSION}_macOS_64-bit.tar.gz" ;;
  *)
    echo "unsupported platform: $(uname -s)-$(uname -m)" >&2
    exit 1 ;;
esac

URL="https://github.com/errata-ai/vale/releases/download/v${VERSION}/${ASSET}"

if [ -w "$PREFIX" ]; then
  curl -sL "$URL" | tar -xz -C "$PREFIX" vale
elif command -v sudo >/dev/null 2>&1; then
  curl -sL "$URL" | sudo tar -xz -C "$PREFIX" vale
else
  echo "ERROR: $PREFIX is not writable and sudo is unavailable" >&2
  exit 1
fi

"$PREFIX/vale" --version
