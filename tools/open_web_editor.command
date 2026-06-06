#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

if [[ "${1:-}" == "--pages" ]]; then
  shift
  echo "Regenerating PuzzleStudio generated Pages editor from games/"
  tools/generate_web_editor.sh games -o docs/index.html
  exec tools/serve_web_editor.sh --pages "$@"
fi

echo "Starting PuzzleStudio editor server from games/"
exec tools/serve_web_editor.sh games "$@"
