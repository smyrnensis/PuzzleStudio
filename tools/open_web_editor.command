#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

echo "Regenerating PuzzleStudio Pages editor from games/"
tools/generate_web_editor.sh games -o docs/index.html

exec tools/serve_web_editor.sh "$@"
