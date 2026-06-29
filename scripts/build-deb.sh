#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-0.1.0}"

if command -v dpkg >/dev/null 2>&1; then
	ARCH="$(dpkg --print-architecture)"
else
	ARCH="amd64"
fi

PKG_NAME="maestro-ai"
PKG_DIR="$ROOT_DIR/target/deb/${PKG_NAME}_${VERSION}_${ARCH}"
DEBIAN_DIR="$PKG_DIR/DEBIAN"

if ! command -v cargo >/dev/null 2>&1; then
	echo "error: cargo not found"
	exit 1
fi

if ! command -v dpkg-deb >/dev/null 2>&1; then
	echo "error: dpkg-deb not found. Install dpkg tools to build .deb packages."
	exit 1
fi

rm -rf "$PKG_DIR"
mkdir -p "$DEBIAN_DIR" "$PKG_DIR/usr/bin"

pushd "$ROOT_DIR" >/dev/null
cargo build --release
popd >/dev/null

cp "$ROOT_DIR/target/release/maestro" "$PKG_DIR/usr/bin/maestro"
chmod 0755 "$PKG_DIR/usr/bin/maestro"

sed "s/{{ARCH}}/${ARCH}/g" "$ROOT_DIR/packaging/debian/control" > "$DEBIAN_DIR/control"
cp "$ROOT_DIR/packaging/debian/postinst" "$DEBIAN_DIR/postinst"
cp "$ROOT_DIR/packaging/debian/prerm" "$DEBIAN_DIR/prerm"
cp "$ROOT_DIR/packaging/debian/postrm" "$DEBIAN_DIR/postrm"

chmod 0755 "$DEBIAN_DIR/postinst" "$DEBIAN_DIR/prerm" "$DEBIAN_DIR/postrm"

# Mark config as conffile so dpkg preserves local edits on upgrades/removal.
mkdir -p "$PKG_DIR/etc/maestro"
cat > "$DEBIAN_DIR/conffiles" <<'EOF'
/etc/maestro/config.yml
EOF

dpkg-deb --build "$PKG_DIR"

echo "Package generated: ${PKG_DIR}.deb"
