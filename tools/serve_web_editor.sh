#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

port=8891
open_browser=1
pages=0
entry_path=""

usage() {
  echo "usage: tools/serve_web_editor.sh [path/to/game-folder-or-game.puzzle-or-game.puzzle3] [--port 8891] [--no-open] [--pages]"
}

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
    --pages)
      pages=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    --*)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
    *)
      if [[ -n "$entry_path" ]]; then
        echo "only one editor entry path can be provided" >&2
        usage >&2
        exit 1
      fi
      entry_path="$1"
      shift
      ;;
  esac
done

open_url() {
  local url="$1"
  if (( ! open_browser )); then
    return
  fi
  if command -v open >/dev/null 2>&1; then
    open "$url"
  elif command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$url"
  else
    echo "no supported browser opener found for ${url}" >&2
    exit 1
  fi
}

wait_for_port() {
  local port_value="$1"
  local pid="$2"
  local ready=0
  for _ in {1..100}; do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      wait "$pid"
      exit 1
    fi
    if python3 - "$port_value" <<'PY' >/dev/null 2>&1; then
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
    echo "server did not become ready on port ${port_value}" >&2
    exit 1
  fi
}

ensure_port_free() {
  local port_value="$1"
  if python3 - "$port_value" <<'PY' >/dev/null 2>&1; then
import socket
import sys

with socket.create_connection(("127.0.0.1", int(sys.argv[1])), timeout=0.2):
    pass
PY
    echo "port ${port_value} is already in use; choose another port with --port" >&2
    exit 1
  fi
}

run_pages_server() {
  if [[ -n "$entry_path" ]]; then
    echo "--pages serves the generated docs/ site and does not accept an entry path" >&2
    exit 1
  fi
  if [[ ! -f docs/index.html ]]; then
    echo "docs/index.html is missing; run tools/generate_web_editor.sh first" >&2
    exit 1
  fi

  local url="http://127.0.0.1:${port}/index.html"
  echo "Starting PuzzleStudio generated Pages editor at ${url}"
  echo "This static server does not provide Rust editor APIs such as /api/highlight."
  echo "Press Ctrl+C to stop."

  ensure_port_free "$port"
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

  wait_for_port "$port" "$server_pid"
  echo "Serving PuzzleStudio generated Pages editor at ${url}"
  open_url "$url"
  wait "$server_pid"
}

run_rust_editor_server() {
  local url="http://127.0.0.1:${port}/editor"
  local args=()
  if [[ -n "$entry_path" ]]; then
    args+=("$entry_path")
  fi
  args+=(--serve --port "$port")

  echo "Starting PuzzleStudio editor server at ${url}"
  echo "This server provides Rust editor APIs such as /api/highlight."
  echo "Press Ctrl+C to stop."

  ensure_port_free "$port"
  cargo run -p html-editor -- "${args[@]}" &
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

  wait_for_port "$port" "$server_pid"
  echo "Serving PuzzleStudio editor server at ${url}"
  open_url "$url"
  wait "$server_pid"
}

if (( pages )); then
  run_pages_server
else
  run_rust_editor_server
fi
