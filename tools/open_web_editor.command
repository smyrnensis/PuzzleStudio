#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

if (($#)); then
  echo "Starting PuzzleStudio editor server"
  exec tools/serve_web_editor.sh "$@"
fi

echo "Starting PuzzleStudio editor server from games/"
exec tools/serve_web_editor.sh games
