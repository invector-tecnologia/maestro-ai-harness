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

if [ "$OS" = "linux" ] && [ "$ARCH" = "x86_64" ]; then
    ASSET_NAME="maestro-linux-amd64"
elif [ "$OS" = "darwin" ] && [ "$ARCH" = "x86_64" ]; then
    ASSET_NAME="maestro-macos-amd64"
elif [ "$OS" = "darwin" ] && { [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; }; then
    ASSET_NAME="maestro-macos-arm64"
else
    ASSET_NAME="maestro-${OS}-${ARCH}"
fi

echo "📦 Preparing asset: $ASSET_NAME"
cp target/release/maestro "target/release/$ASSET_NAME"

echo "🚀 Publishing compiled binary to GitHub Releases ($VERSION)..."
if ! gh release view "$VERSION" >/dev/null 2>&1; then
  echo "Creating new release $VERSION..."
  gh release create "$VERSION" --title "Release $VERSION" --notes "Local build of $VERSION"
fi

gh release upload "$VERSION" "target/release/$ASSET_NAME" --clobber

echo "✅ Published $ASSET_NAME to GitHub!"