#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v gzip >/dev/null 2>&1; then
  echo "standalone export size check requires gzip" >&2
  exit 1
fi

export_target_dir="${CARGO_TARGET_DIR:-/private/tmp/puzzlebuilder-bevy-target}"
export_temp_dir="$(mktemp -d)"
trap 'rm -rf "$export_temp_dir"' EXIT
max_raw_html_bytes=70000000
max_gzip_html_bytes=16000000

check_export_size() {
  local source_path="$1"
  local output_name="$2"
  local output_path="$export_temp_dir/$output_name.html"
  local raw_bytes
  local gzip_bytes

  cargo run \
    --quiet \
    --target-dir "$export_target_dir" \
    -p html-play \
    -- \
    "$source_path" \
    -o "$output_path"

  raw_bytes="$(wc -c < "$output_path" | tr -d '[:space:]')"
  gzip_bytes="$(gzip -c "$output_path" | wc -c | tr -d '[:space:]')"
  if (( raw_bytes > max_raw_html_bytes )); then
    echo "$source_path export is $raw_bytes bytes, exceeding the $max_raw_html_bytes-byte raw HTML budget" >&2
    exit 1
  fi
  if (( gzip_bytes > max_gzip_html_bytes )); then
    echo "$source_path export is $gzip_bytes gzip bytes, exceeding the $max_gzip_html_bytes-byte transfer budget" >&2
    exit 1
  fi
  echo "$source_path: raw=$raw_bytes gzip=$gzip_bytes"
}

check_export_size games/TENETEN.puzzle teneten-2d
check_export_size games/TENETEN3D.puzzle3 teneten-3d
