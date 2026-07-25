#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
web_root="$repo_root/crates/html_editor/web"

if [[ ! -x "$web_root/node_modules/.bin/esbuild" ]]; then
  echo "editor frontend dependencies are unavailable; run tools/install_editor_frontend_deps.sh" >&2
  exit 1
fi

cd "$web_root"
npm run build
