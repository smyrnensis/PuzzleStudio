#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

output="docs/index.html"
args=()

while (($#)); do
  case "$1" in
    -o|--output)
      if (($# < 2)); then
        echo "$1 requires a value" >&2
        exit 1
      fi
      output="$2"
      args+=("$1" "$2")
      shift 2
      ;;
    *)
      args+=("$1")
      shift
      ;;
  esac
done

output_dir="$(dirname "$output")"
mkdir -p "$output_dir"
tools/generate_editor.sh "${args[@]}"

mkdir -p "$output_dir/wasm"
cp crates/html_editor/static/wasm/puzzle_wasm.js "$output_dir/wasm/"
cp crates/html_editor/static/wasm/puzzle_wasm_bg.wasm "$output_dir/wasm/"
