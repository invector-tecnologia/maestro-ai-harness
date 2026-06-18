#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-0.1.0}"
ARCH="$(uname -m)"

echo "🚀 Construindo pacote macOS (.pkg) para a versão $VERSION ($ARCH)..."

# 1. Verifica dependências
if ! command -v pkgbuild >/dev/null 2>&1; then
  echo "❌ erro: 'pkgbuild' não encontrado. Ele está disponível por padrão no macOS ou através do Xcode Command Line Tools."
  exit 1
fi

# 2. Compilação do Rust
echo "🦀 Compilando binário (release)..."
cargo build --release

# 3. Criação dos diretórios de payload (onde o binário será alocado)
PAYLOAD_DIR="target/macos/payload"
PKG_OUT_DIR="target/macos/build"

rm -rf "$PAYLOAD_DIR"
mkdir -p "$PAYLOAD_DIR/usr/local/bin"
mkdir -p "$PKG_OUT_DIR"

# 4. Cópia do binário
echo "📦 Preparando payload em $PAYLOAD_DIR..."
cp target/release/maestro "$PAYLOAD_DIR/usr/local/bin/"
chmod +x "$PAYLOAD_DIR/usr/local/bin/maestro"

# 5. Empacotamento pkg
PKG_NAME="maestro-ai-${VERSION}-macos-${ARCH}.pkg"
PKG_PATH="${PKG_OUT_DIR}/${PKG_NAME}"

echo "🍎 Gerando pacote $PKG_NAME..."
pkgbuild --root "$PAYLOAD_DIR" \
         --identifier "com.invector.maestro-ai" \
         --version "$VERSION" \
         --install-location "/" \
         "$PKG_PATH"

echo "✅ Pacote macOS gerado com sucesso em: $PKG_PATH"