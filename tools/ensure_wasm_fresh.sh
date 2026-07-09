#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

mode="${1:-desktop}"
if (($# > 1)); then
  echo "usage: tools/ensure_wasm_fresh.sh [desktop]" >&2
  exit 2
fi
if [[ "$mode" != "desktop" ]]; then
  echo "unknown wasm freshness mode: $mode" >&2
  exit 2
fi

oldest_artifact_file() {
  local oldest=""
  local path
  for path in "$@"; do
    if [[ ! -f "$path" ]]; then
      return 1
    fi
    if [[ -z "$oldest" || "$path" -ot "$oldest" ]]; then
      oldest="$path"
    fi
  done
  printf '%s\n' "$oldest"
}

newest_source_file() {
  local newest=""
  local found=0
  local path
  for path in "$@"; do
    if [[ -f "$path" ]]; then
      found=1
      if [[ -z "$newest" || "$path" -nt "$newest" ]]; then
        newest="$path"
      fi
    elif [[ -d "$path" ]]; then
      local file
      while IFS= read -r -d '' file; do
        found=1
        if [[ -z "$newest" || "$file" -nt "$newest" ]]; then
          newest="$file"
        fi
      done < <(find "$path" -type f -print0)
    else
      echo "wasm freshness source path is missing: $path" >&2
      return 2
    fi
  done
  if ((found == 0)); then
    echo "wasm freshness source set is empty" >&2
    return 2
  fi
  printf '%s\n' "$newest"
}

verify_target_current() {
  local name="$1"
  shift
  local artifacts=()
  while (($#)); do
    if [[ "$1" == "--" ]]; then
      shift
      break
    fi
    artifacts+=("$1")
    shift
  done
  local sources=("$@")

  local oldest_artifact newest_source
  if ! oldest_artifact="$(oldest_artifact_file "${artifacts[@]}")"; then
    echo "wasm: $name artifacts are missing after build" >&2
    return 1
  fi
  if ! newest_source="$(newest_source_file "${sources[@]}")"; then
    return 1
  fi
  if [[ "$newest_source" -nt "$oldest_artifact" ]]; then
    echo "wasm: $name is still stale after build" >&2
    echo "wasm: newest source: $newest_source" >&2
    echo "wasm: oldest artifact: $oldest_artifact" >&2
    return 1
  fi
}

ensure_target_current() {
  local name="$1"
  local build_cmd="$2"
  shift 2
  local artifacts=()
  while (($#)); do
    if [[ "$1" == "--" ]]; then
      shift
      break
    fi
    artifacts+=("$1")
    shift
  done
  local sources=("$@")

  local oldest_artifact newest_source
  if ! oldest_artifact="$(oldest_artifact_file "${artifacts[@]}")"; then
    echo "wasm: $name artifacts missing; running $build_cmd" >&2
    "$build_cmd"
    verify_target_current "$name" "${artifacts[@]}" -- "${sources[@]}"
    return
  fi
  if ! newest_source="$(newest_source_file "${sources[@]}")"; then
    exit 1
  fi
  if [[ "$newest_source" -nt "$oldest_artifact" ]]; then
    echo "wasm: $name stale; running $build_cmd" >&2
    echo "wasm: newest source: $newest_source" >&2
    echo "wasm: oldest artifact: $oldest_artifact" >&2
    "$build_cmd"
    verify_target_current "$name" "${artifacts[@]}" -- "${sources[@]}"
    return
  fi
  echo "wasm: $name current"
}

ensure_game_wasm_copies_match() {
  local build_cmd="tools/build_wasm_game.sh"
  if cmp -s \
    crates/html_play/static/wasm_game/puzzle_wasm_game.js \
    crates/html_editor/static/wasm_game/puzzle_wasm_game.js \
    && cmp -s \
      crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm \
      crates/html_editor/static/wasm_game/puzzle_wasm_game_bg.wasm; then
    return
  fi

  echo "wasm: puzzle_wasm_game editor copy differs; running $build_cmd" >&2
  "$build_cmd"
  if ! cmp -s \
    crates/html_play/static/wasm_game/puzzle_wasm_game.js \
    crates/html_editor/static/wasm_game/puzzle_wasm_game.js \
    || ! cmp -s \
      crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm \
      crates/html_editor/static/wasm_game/puzzle_wasm_game_bg.wasm; then
    echo "wasm: puzzle_wasm_game editor copy still differs after $build_cmd" >&2
    exit 1
  fi
}

ensure_static_asset_copies_match() {
  local build_cmd="tools/sync_static_assets.sh"
  if cmp -s \
    crates/html_play/static/renderer.js \
    crates/html_editor/static/renderer.js \
    && cmp -s \
      crates/html_play/static/renderer.css \
      crates/html_editor/static/renderer.css; then
    return
  fi

  echo "static: html_editor renderer copy differs; running $build_cmd" >&2
  "$build_cmd"
  if ! cmp -s \
    crates/html_play/static/renderer.js \
    crates/html_editor/static/renderer.js \
    || ! cmp -s \
      crates/html_play/static/renderer.css \
      crates/html_editor/static/renderer.css; then
    echo "static: html_editor renderer copy still differs after $build_cmd" >&2
    exit 1
  fi
}

workspace_sources=(
  Cargo.toml
  Cargo.lock
)

authoring_sources=(crates/authoring/Cargo.toml crates/authoring/src)
core_sources=(crates/core/Cargo.toml crates/core/src)
grid3d_sources=(crates/grid3d/Cargo.toml crates/grid3d/src)
grid3d_authoring_sources=(crates/grid3d_authoring/Cargo.toml crates/grid3d_authoring/src)
kernel_sources=(crates/kernel/Cargo.toml crates/kernel/src)
lang_sources=(crates/lang/Cargo.toml crates/lang/src)
play_sources=(crates/play/Cargo.toml crates/play/src)
runtime_contract_sources=(crates/runtime_contract/Cargo.toml crates/runtime_contract/src)
puzzle3_sources=("${grid3d_sources[@]}" "${grid3d_authoring_sources[@]}" "${lang_sources[@]}" "${runtime_contract_sources[@]}")
scene_sources=(crates/scene/Cargo.toml crates/scene/src)
solver_sources=(crates/solver/Cargo.toml crates/solver/src)

ensure_target_current \
  puzzle_wasm \
  tools/build_wasm_editor.sh \
  crates/html_editor/static/wasm/puzzle_wasm.js \
  crates/html_editor/static/wasm/puzzle_wasm_bg.wasm \
  -- \
  "${workspace_sources[@]}" \
  tools/build_wasm_editor.sh \
  crates/wasm/Cargo.toml \
  crates/wasm/src \
  crates/html_play/Cargo.toml \
  crates/html_play/src \
  crates/wasm_core/Cargo.toml \
  crates/wasm_core/src \
  "${authoring_sources[@]}" \
  "${core_sources[@]}" \
  "${grid3d_sources[@]}" \
  "${kernel_sources[@]}" \
  "${lang_sources[@]}" \
  "${play_sources[@]}" \
  "${puzzle3_sources[@]}" \
  "${scene_sources[@]}" \
  "${solver_sources[@]}"

ensure_target_current \
  puzzle_wasm_game \
  tools/build_wasm_game.sh \
  crates/html_play/static/wasm_game/puzzle_wasm_game.js \
  crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm \
  crates/html_editor/static/wasm_game/puzzle_wasm_game.js \
  crates/html_editor/static/wasm_game/puzzle_wasm_game_bg.wasm \
  -- \
  "${workspace_sources[@]}" \
  tools/build_wasm_game.sh \
  crates/wasm_game/Cargo.toml \
  crates/wasm_game/src \
  crates/game_runtime/Cargo.toml \
  crates/game_runtime/src \
  "${authoring_sources[@]}" \
  "${core_sources[@]}" \
  "${grid3d_sources[@]}" \
  "${kernel_sources[@]}" \
  "${lang_sources[@]}" \
  "${play_sources[@]}" \
  "${puzzle3_sources[@]}" \
  "${scene_sources[@]}"

ensure_game_wasm_copies_match
ensure_static_asset_copies_match
