#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

port=8891
open_browser=1
entry_path=""

usage() {
  echo "usage: tools/serve_web_editor.sh [path/to/workspace-or-game.puzzle] [--port 8891] [--no-open]"
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
      echo "--pages static editor serving was removed; use the Rust editor server instead" >&2
      exit 1
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
  for _ in {1..600}; do
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

port_accepts_connection() {
  local port_value="$1"
  if python3 - "$port_value" <<'PY' >/dev/null 2>&1; then
import socket
import sys

with socket.create_connection(("127.0.0.1", int(sys.argv[1])), timeout=0.2):
    pass
PY
    return 0
  fi
  return 1
}

listener_field() {
  local pid="$1"
  local field="$2"
  lsof -a -p "$pid" -d cwd "-F${field}" 2>/dev/null | awk -v prefix="$field" '
    index($0, prefix) == 1 { print substr($0, 2); exit }
  '
}

listener_command() {
  local pid="$1"
  lsof -a -p "$pid" "-Fc" 2>/dev/null | awk '
    index($0, "c") == 1 { print substr($0, 2); exit }
  '
}

stop_existing_editor_server_or_fail() {
  local port_value="$1"
  if ! port_accepts_connection "$port_value"; then
    return
  fi

  if ! command -v lsof >/dev/null 2>&1; then
    echo "port ${port_value} is already in use and lsof is unavailable; choose another port with --port" >&2
    exit 1
  fi

  local pids=()
  while IFS= read -r pid; do
    [[ -n "$pid" ]] && pids+=("$pid")
  done < <(lsof -tiTCP:"$port_value" -sTCP:LISTEN 2>/dev/null)

  if ((${#pids[@]} != 1)); then
    echo "port ${port_value} is already in use by ${#pids[@]} listeners; choose another port with --port" >&2
    exit 1
  fi

  local pid="${pids[0]}"
  local command_name
  command_name="$(listener_command "$pid")"
  local cwd
  cwd="$(listener_field "$pid" n)"

  if [[ "$command_name" != "html-editor" || "$cwd" != "$repo_root" ]]; then
    echo "port ${port_value} is already in use by pid ${pid} (${command_name:-unknown}); choose another port with --port" >&2
    exit 1
  fi

  echo "Stopping existing PuzzleStudio editor server on port ${port_value} (pid ${pid})."
  if ! kill "$pid" >/dev/null 2>&1; then
    echo "failed to stop existing PuzzleStudio editor server on port ${port_value} (pid ${pid})" >&2
    exit 1
  fi

  for _ in {1..100}; do
    if ! port_accepts_connection "$port_value"; then
      return
    fi
    sleep 0.1
  done

  echo "existing PuzzleStudio editor server on port ${port_value} did not stop" >&2
  exit 1
}

ensure_port_free() {
  local port_value="$1"
  if port_accepts_connection "$port_value"; then
    echo "port ${port_value} is already in use; choose another port with --port" >&2
    exit 1
  fi
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

  stop_existing_editor_server_or_fail "$port"
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

run_rust_editor_server
