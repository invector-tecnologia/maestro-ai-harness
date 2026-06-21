#!/usr/bin/env bash
set -euo pipefail

PKG_PATH="${1:-}"
if [[ -z "$PKG_PATH" ]]; then
  echo "Usage: $0 <path-to-pkg.tar.zst>"
  exit 1
fi

if [[ ! -f "$PKG_PATH" ]]; then
  echo "Package not found: $PKG_PATH"
  exit 1
fi

if ! command -v pacman >/dev/null 2>&1; then
  echo "error: pacman not found"
  exit 1
fi

if ! command -v sudo >/dev/null 2>&1; then
  echo "error: sudo not found"
  exit 1
fi

echo "[1/6] Installing package"
sudo pacman -U --noconfirm "$PKG_PATH"

echo "[2/6] Verifying binary"
command -v maestro >/dev/null
maestro --help >/dev/null
maestro list-agents >/dev/null

echo "[3/6] Verifying config and dirs"
test -f /etc/maestro/config.yaml
test -d /var/lib/maestro
test -d /var/log/maestro

echo "[4/6] Running doctor"
maestro doctor --config /etc/maestro/config.yaml >/dev/null

echo "[5/6] Removing package"
sudo pacman -R --noconfirm maestro-ai

echo "[6/6] Verifying binary removal"
if command -v maestro >/dev/null 2>&1; then
  echo "maestro binary still present after removal"
  exit 1
fi

echo "Omarchy smoke test completed successfully"
