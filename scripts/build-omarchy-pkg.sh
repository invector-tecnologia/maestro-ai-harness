#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PKG_NAME="maestro-ai"
VERSION="${1:-0.1.0}"
WORK_DIR="$ROOT_DIR/target/omarchy"
BUILD_DIR="$WORK_DIR/build"
TARBALL="$BUILD_DIR/${PKG_NAME}-${VERSION}.tar.gz"

if ! command -v makepkg >/dev/null 2>&1; then
  echo "error: makepkg not found. Install base-devel (Arch/Omarchy)."
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found"
  exit 1
fi

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Use working tree content instead of git archive so local packaging assets are included.
tar -C "$ROOT_DIR" \
  --transform "s,^,${PKG_NAME}-${VERSION}/," \
  --exclude=.git \
  --exclude=target \
  --exclude=.vscode \
  -czf "$TARBALL" .

cp "$ROOT_DIR/packaging/omarchy/PKGBUILD" "$BUILD_DIR/PKGBUILD"
sed -i "s/^pkgver=.*/pkgver=${VERSION}/" "$BUILD_DIR/PKGBUILD"

pushd "$BUILD_DIR" >/dev/null
makepkg -f
popd >/dev/null

echo "Package generated in: $BUILD_DIR"
ls -1 "$BUILD_DIR"/*.pkg.tar.zst
