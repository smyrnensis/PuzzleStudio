#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

copy_asset() {
  local source="$1"
  local target="$2"
  if [[ ! -f "$source" ]]; then
    echo "static asset source is missing: $source" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$target")"
  cp "$source" "$target"
}

copy_asset crates/html_play/static/renderer.js crates/html_editor/static/renderer.js
copy_asset crates/html_play/static/renderer.css crates/html_editor/static/renderer.css

echo "static assets synced"
