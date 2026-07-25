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

ensure_wasm_copies_match() {
  local name="$1"
  local build_cmd="$2"
  local source_js="$3"
  local target_js="$4"
  local source_wasm="$5"
  local target_wasm="$6"
  if cmp -s \
    "$source_js" \
    "$target_js" \
    && cmp -s \
      "$source_wasm" \
      "$target_wasm"; then
    return
  fi

  echo "wasm: $name editor copy differs; running $build_cmd" >&2
  "$build_cmd"
  if ! cmp -s \
    "$source_js" \
    "$target_js" \
    || ! cmp -s \
      "$source_wasm" \
      "$target_wasm"; then
    echo "wasm: $name editor copy still differs after $build_cmd" >&2
    exit 1
  fi
}

ensure_static_asset_copies_match() {
  local build_cmd="tools/sync_static_assets.sh"
  if "$build_cmd" --check >/dev/null 2>&1; then
    return
  fi

  echo "static: generated html_editor distribution assets differ; running $build_cmd" >&2
  "$build_cmd"
  if ! "$build_cmd" --check; then
    echo "static: generated html_editor distribution assets still differ after $build_cmd" >&2
    exit 1
  fi
}

workspace_sources=(
  Cargo.toml
  Cargo.lock
)

if ! command -v jq >/dev/null 2>&1; then
  echo "wasm freshness requires jq to read the Cargo workspace dependency graph" >&2
  exit 1
fi

workspace_metadata="$(cargo metadata --format-version 1 --no-deps)"

workspace_dependency_sources() {
  local package_name="$1"
  local manifests
  manifests="$(
    jq -r --arg name "$package_name" '
      . as $workspace
      | ($workspace.packages[] | select(.name == $name) | .manifest_path) as $root
      | def dependencies($manifest):
          $manifest,
          ($workspace.packages[]
            | select(.manifest_path == $manifest)
            | .dependencies[]
            | select(.path != null)
            | (.path + "/Cargo.toml")
            | dependencies(.));
      [dependencies($root)] | unique[]
    ' <<<"$workspace_metadata"
  )"
  if [[ -z "$manifests" ]]; then
    echo "wasm freshness Cargo package is missing: $package_name" >&2
    return 1
  fi

  local manifest crate_root
  while IFS= read -r manifest; do
    printf '%s\n' "$manifest"
    crate_root="${manifest%/Cargo.toml}"
    if [[ -d "$crate_root/src" ]]; then
      printf '%s\n' "$crate_root/src"
    fi
    if [[ -f "$crate_root/build.rs" ]]; then
      printf '%s\n' "$crate_root/build.rs"
    fi
  done <<<"$manifests"
}

wasm_editor_rust_sources=()
while IFS= read -r source; do
  wasm_editor_rust_sources+=("$source")
done < <(workspace_dependency_sources puzzle-wasm)

wasm_game_rust_sources=()
while IFS= read -r source; do
  wasm_game_rust_sources+=("$source")
done < <(workspace_dependency_sources puzzle-wasm-game)

wasm_player_rust_sources=()
while IFS= read -r source; do
  wasm_player_rust_sources+=("$source")
done < <(workspace_dependency_sources puzzle-wasm-player)

html_play_preview_sources=(
  crates/html_play/static/index.html
  crates/html_play/static/app.css
  crates/html_play/static/renderer.css
  crates/html_play/static/visuals.js
  crates/html_play/static/app.js
  crates/html_play/static/renderer.js
  crates/html_play/static/standalone.js
  crates/html_play/static/puzzle3.css
  crates/html_play/static/puzzle3_visual_core.js
  crates/html_play/static/puzzle3_three_renderer.js
  crates/html_play/static/puzzle3_component.js
  crates/html_play/static/vendor/three/three.module.min.js
)

ensure_target_current \
  puzzle_wasm \
  tools/build_wasm_editor.sh \
  crates/html_editor/static/wasm/puzzle_wasm.js \
  crates/html_editor/static/wasm/puzzle_wasm_bg.wasm \
  -- \
  "${workspace_sources[@]}" \
  tools/build_wasm_editor.sh \
  "${html_play_preview_sources[@]}" \
  "${wasm_editor_rust_sources[@]}"

ensure_target_current \
  puzzle_wasm_player \
  tools/build_wasm_player.sh \
  crates/web_audio/generated/puzzle_audio_worklet.js \
  crates/html_play/static/wasm_player/puzzle_wasm_player.js \
  crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm \
  crates/html_editor/static/wasm_player/puzzle_wasm_player.js \
  crates/html_editor/static/wasm_player/puzzle_wasm_player_bg.wasm \
  -- \
  "${workspace_sources[@]}" \
  tools/build_wasm_player.sh \
  tools/build_audio_worklet_bundle.mjs \
  crates/audio_worklet/Cargo.toml \
  crates/audio_worklet/src \
  crates/audio_worklet/worklet.js \
  "${wasm_player_rust_sources[@]}"

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
  "${wasm_game_rust_sources[@]}"

ensure_wasm_copies_match \
  puzzle_wasm_player \
  tools/build_wasm_player.sh \
  crates/html_play/static/wasm_player/puzzle_wasm_player.js \
  crates/html_editor/static/wasm_player/puzzle_wasm_player.js \
  crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm \
  crates/html_editor/static/wasm_player/puzzle_wasm_player_bg.wasm
ensure_wasm_copies_match \
  puzzle_wasm_game \
  tools/build_wasm_game.sh \
  crates/html_play/static/wasm_game/puzzle_wasm_game.js \
  crates/html_editor/static/wasm_game/puzzle_wasm_game.js \
  crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm \
  crates/html_editor/static/wasm_game/puzzle_wasm_game_bg.wasm
ensure_static_asset_copies_match
