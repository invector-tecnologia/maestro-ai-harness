#!/usr/bin/env bash
set -euo pipefail

echo "🚀 Starting Maestro AI installation..."

GITHUB_REPO="invector-tecnologia/maestro-ai-harness"
INSTALL_DIR="/usr/local/bin"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS-$ARCH" in
    linux-x86_64)                  TARGET="maestro-linux-amd64" ;;
    linux-aarch64 | linux-arm64)   TARGET="maestro-linux-arm64" ;;
    darwin-x86_64)                 TARGET="maestro-macos-amd64" ;;
    darwin-arm64 | darwin-aarch64) TARGET="maestro-macos-arm64" ;;
    *)
        echo "❌ Unsupported OS or Architecture: $OS-$ARCH"
        echo "   Build from source manually — see the README 'MANUAL OVERRIDE' section."
        exit 1
        ;;
esac

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

install_file() {
    # Move a built/downloaded binary into INSTALL_DIR, using sudo only if needed.
    local src="$1"
    chmod +x "$src"
    echo "🔨 Installing to $INSTALL_DIR (might require sudo password)..."
    if [ -w "$INSTALL_DIR" ]; then
        mv "$src" "$INSTALL_DIR/maestro"
    else
        sudo mv "$src" "$INSTALL_DIR/maestro"
    fi
}

install_binary() {
    local url="$1"
    local tmp
    tmp="$(mktemp -d)"
    echo "⬇️  Downloading pre-compiled binary ($TARGET)..."
    if ! curl -fSL -o "$tmp/maestro" "$url"; then
        echo "❌ Download failed."
        rm -rf "$tmp"
        return 1
    fi

    # Verify integrity when a matching .sha256 asset is published alongside.
    if curl -fsSL -o "$tmp/maestro.sha256" "${url}.sha256" 2>/dev/null; then
        echo "🔐 Verifying checksum..."
        local expected actual
        expected="$(awk '{print $1}' "$tmp/maestro.sha256")"
        actual="$(sha256_of "$tmp/maestro")"
        if [ "$expected" != "$actual" ]; then
            echo "❌ Checksum mismatch — refusing to install."
            rm -rf "$tmp"
            return 1
        fi
    fi

    install_file "$tmp/maestro"
    rm -rf "$tmp"
}

build_from_source() {
    echo "🛠️  Building Maestro from source..."
    if ! command -v cargo >/dev/null 2>&1; then
        echo "❌ 'cargo' (Rust toolchain) is required to build from source."
        echo "   Install Rust from https://rustup.rs and re-run this installer."
        exit 1
    fi
    if ! command -v git >/dev/null 2>&1; then
        echo "❌ 'git' is required to build from source."
        exit 1
    fi

    local tmp
    tmp="$(mktemp -d)"
    echo "⬇️  Cloning $GITHUB_REPO..."
    git clone --depth 1 "https://github.com/${GITHUB_REPO}.git" "$tmp/src"
    echo "🔨 Compiling release binary (this may take a few minutes)..."
    (cd "$tmp/src" && cargo build --release --locked)
    install_file "$tmp/src/target/release/maestro"
    rm -rf "$tmp"
}

echo "🔎 Looking for a published release for $TARGET..."
API_RESPONSE="$(curl -fsSL "https://api.github.com/repos/$GITHUB_REPO/releases/latest" 2>/dev/null || true)"

if [ -z "$API_RESPONSE" ]; then
    echo "⚠️  Could not reach the GitHub Releases API (offline or rate limited)."
    build_from_source
else
    DOWNLOAD_URL="$(printf '%s' "$API_RESPONSE" | grep "browser_download_url.*${TARGET}\"" | cut -d '"' -f 4 | head -n 1)"
    if [ -z "$DOWNLOAD_URL" ]; then
        echo "ℹ️  No published binary found for $TARGET."
        build_from_source
    elif ! install_binary "$DOWNLOAD_URL"; then
        echo "↩️  Falling back to building from source..."
        build_from_source
    fi
fi

echo "✅ Installation complete! You can now run 'maestro'."
if command -v maestro >/dev/null 2>&1; then
    maestro --version 2>/dev/null || true
fi