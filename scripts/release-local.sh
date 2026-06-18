#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-0.1.0}"
RUN_QUALITY_GATE="${MAESTRO_SKIP_QUALITY_GATE:-0}"

usage() {
  cat <<EOF
Usage: $0 [version]

Build local release artifacts for Debian and Omarchy in one command.

Arguments:
  version   Package version (default: 0.1.0)

Environment:
  MAESTRO_SKIP_QUALITY_GATE=1   Skip ./scripts/quality-gate.sh before packaging
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ ! -x "$ROOT_DIR/scripts/build-deb.sh" ]]; then
  echo "error: missing executable script scripts/build-deb.sh"
  exit 1
fi

if [[ ! -x "$ROOT_DIR/scripts/build-omarchy-pkg.sh" ]]; then
  echo "error: missing executable script scripts/build-omarchy-pkg.sh"
  exit 1
fi

if [[ "$RUN_QUALITY_GATE" != "1" && ! -x "$ROOT_DIR/scripts/quality-gate.sh" ]]; then
  echo "error: missing executable script scripts/quality-gate.sh"
  exit 1
fi

DEBIAN_OK=0
OMARCHY_OK=0

if [[ "$RUN_QUALITY_GATE" != "1" ]]; then
  echo "[0/3] Running quality gate"
  "$ROOT_DIR/scripts/quality-gate.sh"
else
  echo "[0/3] Skipping quality gate (MAESTRO_SKIP_QUALITY_GATE=1)"
fi

echo "[1/3] Building Debian package"
if "$ROOT_DIR/scripts/build-deb.sh" "$VERSION"; then
  DEBIAN_OK=1
else
  echo "warning: Debian build failed"
fi

echo "[2/3] Building Omarchy package"
if "$ROOT_DIR/scripts/build-omarchy-pkg.sh" "$VERSION"; then
  OMARCHY_OK=1
else
  echo "warning: Omarchy build failed"
fi

if command -v dpkg >/dev/null 2>&1; then
  DEB_ARCH="$(dpkg --print-architecture)"
else
  DEB_ARCH="amd64"
fi
DEB_PATH="$ROOT_DIR/target/deb/maestro-ai_${VERSION}_${DEB_ARCH}.deb"

OMARCHY_PATH=""
while IFS= read -r line; do
  OMARCHY_PATH="$line"
  break
done < <(find "$ROOT_DIR/target/omarchy/build" -maxdepth 1 -type f -name "maestro-ai-${VERSION}-1-*.pkg.tar.zst" | sort)

echo "[3/3] Release local completed"
if [[ "$DEBIAN_OK" -eq 1 ]]; then
  echo "Debian package: $DEB_PATH"
else
  echo "Debian package: unavailable"
fi
if [[ -n "$OMARCHY_PATH" ]]; then
  echo "Omarchy package: $OMARCHY_PATH"
else
  echo "warning: Omarchy package artifact not found in target/omarchy/build"
fi

if [[ "$DEBIAN_OK" -ne 1 || "$OMARCHY_OK" -ne 1 ]]; then
  exit 1
fi
