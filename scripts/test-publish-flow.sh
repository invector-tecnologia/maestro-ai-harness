#!/usr/bin/env bash
set -euo pipefail

VERSION="v0.0.0-test"

echo "🚀 Starting automated End-to-End test for publish and install..."

# 1. Check dependencies
if ! command -v gh >/dev/null 2>&1; then
  echo "❌ error: GitHub CLI (gh) not found. Please install it to run this test."
  exit 1
fi

# 2. Publish test release
echo "📦 Publishing test release $VERSION..."
./scripts/publish-github.sh "$VERSION"

# 3. Run install script
echo "⬇️ Running install script..."
./scripts/install.sh

# 4. Verify installation
echo "✅ Verifying installation..."
if command -v maestro >/dev/null 2>&1; then
    maestro --help >/dev/null
    echo "🎉 maestro binary successfully installed and executed!"
else
    echo "❌ maestro binary not found in PATH."
    exit 1
fi

# 5. Clean up
echo "🧹 Cleaning up test release from GitHub..."
gh release delete "$VERSION" -y --cleanup-tag || true

echo "🧹 Removing maestro binary from /usr/local/bin..."
sudo rm -f /usr/local/bin/maestro

echo "✅ End-to-End test completed successfully!"