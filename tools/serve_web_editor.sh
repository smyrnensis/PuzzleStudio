#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

port=8891
open_browser=1

while (($#)); do
  case "$1" in
    --port)
      if (($# < 2)); then
        echo "--port requires a value" >&2
        exit 1
      fi
      port="$2"
      shift 2
      ;;
    --no-open)
      open_browser=0
      shift
      ;;
    --help|-h)
      echo "usage: tools/serve_web_editor.sh [--port 8891] [--no-open]"
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      echo "usage: tools/serve_web_editor.sh [--port 8891] [--no-open]" >&2
      exit 1
      ;;
  esac
done

if [[ ! -f docs/index.html ]]; then
  echo "docs/index.html is missing; run tools/generate_web_editor.sh first" >&2
  exit 1
fi

url="http://127.0.0.1:${port}/index.html"
echo "Serving PuzzleStudio Pages editor at ${url}"
echo "Press Ctrl+C to stop."

if (( open_browser )); then
  if command -v open >/dev/null 2>&1; then
    open "$url" >/dev/null 2>&1 || true
  elif command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$url" >/dev/null 2>&1 || true
  fi
fi

exec python3 -m http.server "$port" -d docs
