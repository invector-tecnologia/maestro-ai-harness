#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ASSETS_DIR="$ROOT_DIR/docs/assets"

mkdir -p "$ASSETS_DIR"

echo "⬇️ Downloading placeholder images..."
curl -sSL "https://placehold.co/800x400/2a2a2a/daa520.png?text=Maestro+Dream+TUI+Screenshot" -o "$ASSETS_DIR/dream-tui.png"
curl -sSL "https://placehold.co/250x100/2a2a2a/daa520.gif?text=Equalizer+Animation" -o "$ASSETS_DIR/equalizer-animation.gif"

echo "✅ Created $ASSETS_DIR and downloaded placeholder images!"
echo "📸 Remember to replace them with your actual terminal screenshots before committing!"