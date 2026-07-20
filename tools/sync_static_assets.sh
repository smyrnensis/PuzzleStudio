#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

mode="${1:-sync}"
if [[ "$mode" != "sync" && "$mode" != "--check" ]]; then
  echo "usage: tools/sync_static_assets.sh [--check]" >&2
  exit 2
fi

sync_asset() {
  local source="$1"
  local target="$2"
  if [[ ! -f "$source" ]]; then
    echo "static asset source is missing: $source" >&2
    exit 1
  fi
  if [[ "$mode" == "--check" ]]; then
    if [[ ! -f "$target" ]] || ! cmp -s "$source" "$target"; then
      echo "$target must be the generated distribution copy of $source; run tools/sync_static_assets.sh" >&2
      return 1
    fi
    return
  fi
  mkdir -p "$(dirname "$target")"
  cp "$source" "$target"
}

status=0
sync_asset crates/html_play/static/renderer.js crates/html_editor/static/renderer.js || status=1
sync_asset crates/html_play/static/renderer.css crates/html_editor/static/renderer.css || status=1
sync_asset crates/html_play/static/visual_tween_core.js crates/html_editor/static/visual_tween_core.js || status=1
sync_asset crates/html_play/static/puzzle3_visual_core.js crates/html_editor/static/puzzle3_visual_core.js || status=1

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

if [[ "$mode" == "sync" ]]; then
  echo "generated editor distribution assets synced"
fi
