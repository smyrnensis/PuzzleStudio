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
"$repo_root/tools/build_editor_frontend.sh"
if ((${#args[@]})); then
  cargo run -p html-editor -- "${args[@]}"
else
  cargo run -p html-editor --
fi
