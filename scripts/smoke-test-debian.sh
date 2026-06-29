#!/usr/bin/env bash
set -euo pipefail

DEB_PATH="${1:-}"
if [[ -z "$DEB_PATH" ]]; then
  echo "Usage: $0 <path-to-deb>"
  exit 1
fi

if [[ ! -f "$DEB_PATH" ]]; then
  echo "Deb file not found: $DEB_PATH"
  exit 1
fi

if ! command -v dpkg >/dev/null 2>&1; then
  echo "error: dpkg not found"
  exit 1
fi

if ! command -v sudo >/dev/null 2>&1; then
  echo "error: sudo not found"
  exit 1
fi

echo "[1/7] Installing package"
sudo dpkg -i "$DEB_PATH"

echo "[2/7] Verifying binary"
command -v maestro >/dev/null
maestro --help >/dev/null

echo "[3/7] Verifying config and dirs"
test -f /etc/maestro/config.yml
test -d /var/lib/maestro
test -d /var/log/maestro

echo "[4/7] Running doctor"
maestro doctor --config /etc/maestro/config.yml >/dev/null

echo "[5/7] Removing package (preserve mode)"
sudo dpkg -r maestro-ai

echo "[6/7] Ensuring config preserved after remove"
test -f /etc/maestro/config.yml

echo "[7/7] Purging package data"
sudo dpkg -P maestro-ai || true
if [[ -e /etc/maestro || -e /var/lib/maestro || -e /var/log/maestro ]]; then
  echo "Purge did not fully clean directories"
  exit 1
fi

echo "Smoke test completed successfully"
