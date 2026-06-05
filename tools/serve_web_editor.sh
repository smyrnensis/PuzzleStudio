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
echo "Starting PuzzleStudio Pages editor at ${url}"
echo "Press Ctrl+C to stop."

python3 -m http.server "$port" --bind 127.0.0.1 -d docs &
server_pid=$!

cleanup() {
  if kill -0 "$server_pid" >/dev/null 2>&1; then
    kill "$server_pid" >/dev/null 2>&1 || true
    wait "$server_pid" 2>/dev/null || true
  fi
}

trap 'cleanup; exit 130' INT
trap 'cleanup; exit 143' TERM
trap cleanup EXIT

ready=0
for _ in {1..50}; do
  if ! kill -0 "$server_pid" >/dev/null 2>&1; then
    wait "$server_pid"
    exit 1
  fi
  if python3 - "$port" <<'PY' >/dev/null 2>&1; then
import socket
import sys

with socket.create_connection(("127.0.0.1", int(sys.argv[1])), timeout=0.2):
    pass
PY
    ready=1
    break
  fi
  sleep 0.1
done

if (( ! ready )); then
  echo "server did not become ready at ${url}" >&2
  exit 1
fi

echo "Serving PuzzleStudio Pages editor at ${url}"

if (( open_browser )); then
  if command -v open >/dev/null 2>&1; then
    open "$url"
  elif command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$url"
  else
    echo "no supported browser opener found for ${url}" >&2
    exit 1
  fi
fi

wait "$server_pid"
