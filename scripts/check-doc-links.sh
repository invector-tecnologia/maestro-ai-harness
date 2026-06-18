#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DOCS_DIR="$ROOT_DIR/docs"

if [[ ! -d "$DOCS_DIR" ]]; then
  echo "docs directory not found: $DOCS_DIR" >&2
  exit 1
fi

broken=0

check_path() {
  local src_file="$1"
  local raw_target="$2"

  # Drop optional anchor fragment.
  local target="${raw_target%%#*}"

  # Ignore external and empty links.
  if [[ -z "$target" ]]; then
    return 0
  fi
  if [[ "$target" =~ ^(http|https|mailto): ]]; then
    return 0
  fi
  if [[ "$target" =~ ^# ]]; then
    return 0
  fi

  # Decode %20 to space for filesystem resolution.
  target="${target//%20/ }"

  local resolved
  if [[ "$target" == /* ]]; then
    resolved="$ROOT_DIR$target"
  else
    resolved="$(cd "$(dirname "$src_file")" && pwd)/$target"
  fi

  if [[ ! -e "$resolved" ]]; then
    echo "BROKEN LINK | $src_file -> $raw_target" >&2
    broken=$((broken + 1))
  fi
}

while IFS= read -r -d '' file; do
  # Markdown inline links: [text](target)
  while IFS= read -r target; do
    check_path "$file" "$target"
  done < <(grep -oP '\[[^\]]+\]\(\K[^)]+' "$file" || true)

  # Copilot-style anchors: #file:path
  while IFS= read -r anchor; do
    local_path="$(echo "$anchor" | sed 's/[,.;:)]*$//')"
    if [[ "$local_path" == /* ]]; then
      resolved="$ROOT_DIR$local_path"
    else
      resolved="$ROOT_DIR/$local_path"
    fi
    if [[ ! -e "$resolved" ]]; then
      echo "BROKEN #file ANCHOR | $file -> $anchor" >&2
      broken=$((broken + 1))
    fi
  done < <(grep -oP '#file:\K[^\s]+' "$file" || true)
done < <(find "$DOCS_DIR" -type f -name '*.md' -print0)

if [[ "$broken" -gt 0 ]]; then
  echo "Documentation link integrity check failed: $broken broken references found." >&2
  exit 1
fi

echo "Documentation link integrity check passed."
