#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
web_root="$repo_root/crates/html_editor/web"
dependency_root="${PUZZLE_EDITOR_FRONTEND_CACHE:-/private/tmp/puzzlebuilder-editor-frontend}"
link_path="$web_root/node_modules"

mkdir -p "$dependency_root"
cp "$web_root/package.json" "$dependency_root/package.json"
cp "$web_root/package-lock.json" "$dependency_root/package-lock.json"
npm ci --prefix "$dependency_root"

if [[ -L "$link_path" ]]; then
  current_target="$(readlink "$link_path")"
  if [[ "$current_target" != "$dependency_root/node_modules" ]]; then
    echo "editor dependency link points to an unexpected cache: $current_target" >&2
    exit 1
  fi
elif [[ -e "$link_path" ]]; then
  echo "editor dependencies must not be stored inside the repository: $link_path" >&2
  exit 1
else
  ln -s "$dependency_root/node_modules" "$link_path"
fi
