#!/usr/bin/env bash
set -euo pipefail

echo "🚀 Starting Maestro AI installation..."

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

if [ "$OS" = "linux" ] && [ "$ARCH" = "x86_64" ]; then
    TARGET="maestro-linux-amd64"
elif [ "$OS" = "darwin" ] && [ "$ARCH" = "x86_64" ]; then
    TARGET="maestro-macos-amd64"
elif [ "$OS" = "darwin" ] && { [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; }; then
    TARGET="maestro-macos-arm64"
else
    echo "❌ Unsupported OS or Architecture: $OS-$ARCH"
    exit 1
fi

# TODO: Replace 'your-username' with your actual GitHub username/repo
GITHUB_REPO="invector-tecnologia/maestro-multi-agents"
INSTALL_DIR="/usr/local/bin"

echo "⬇️  Downloading pre-compiled binary for $OS-$ARCH..."
DOWNLOAD_URL=$(curl -s "https://api.github.com/repos/$GITHUB_REPO/releases/latest" | grep "browser_download_url.*$TARGET" | cut -d '"' -f 4 | head -n 1)

if [ -z "$DOWNLOAD_URL" ]; then
    echo "❌ Could not find a release binary for $TARGET."
    echo "Make sure there's a published release with assets on GitHub."
    exit 1
fi

curl -L -o maestro "$DOWNLOAD_URL"
chmod +x maestro

echo "🔨 Installing to $INSTALL_DIR (might require sudo password)..."
if [ -w "$INSTALL_DIR" ]; then
    mv maestro "$INSTALL_DIR/maestro"
else
    sudo mv maestro "$INSTALL_DIR/maestro"
fi

echo "✅ Installation complete! You can now run 'maestro'."