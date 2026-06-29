#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
  echo "Usage: $0 <tag-version>"
  echo "Example: $0 v0.1.0"
  exit 1
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "❌ error: GitHub CLI (gh) not found. Please install it to publish."
  exit 1
fi

echo "🔨 Building release binary locally..."
cargo build --release

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS-$ARCH" in
    linux-x86_64)                  ASSET_NAME="maestro-linux-amd64" ;;
    linux-aarch64 | linux-arm64)   ASSET_NAME="maestro-linux-arm64" ;;
    darwin-x86_64)                 ASSET_NAME="maestro-macos-amd64" ;;
    darwin-arm64 | darwin-aarch64) ASSET_NAME="maestro-macos-arm64" ;;
    *)                             ASSET_NAME="maestro-${OS}-${ARCH}" ;;
esac

echo "📦 Preparing asset: $ASSET_NAME"
cp target/release/maestro "target/release/$ASSET_NAME"

# Emit a checksum so the installer can verify download integrity.
(
    cd target/release
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$ASSET_NAME" >"$ASSET_NAME.sha256"
    else
        shasum -a 256 "$ASSET_NAME" >"$ASSET_NAME.sha256"
    fi
)

echo "🚀 Publishing compiled binary to GitHub Releases ($VERSION)..."
if ! gh release view "$VERSION" >/dev/null 2>&1; then
  echo "Creating new release $VERSION..."
  gh release create "$VERSION" --title "Release $VERSION" --notes "Local build of $VERSION"
fi

gh release upload "$VERSION" \
  "target/release/$ASSET_NAME" \
  "target/release/$ASSET_NAME.sha256" \
  --clobber

echo "✅ Published $ASSET_NAME to GitHub!"