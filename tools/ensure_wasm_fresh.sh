#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

mode="${1:-desktop}"
if (($# > 1)); then
  echo "usage: tools/ensure_wasm_fresh.sh [desktop|desktop-dev]" >&2
  exit 2
fi
case "$mode" in
  desktop)
    wasm_profile="release"
    ;;
  desktop-dev)
    wasm_profile="debug"
    ;;
  *)
  echo "unknown wasm freshness mode: $mode" >&2
  exit 2
    ;;
esac

provenance_root="${CARGO_TARGET_DIR:-target}/puzzlestudio-wasm-provenance"

artifacts_exist() {
  local path
  for path in "$@"; do
    if [[ ! -f "$path" ]]; then
      return 1
    fi
  done
}

source_set_digest() {
  local build_profile="$1"
  local build_cmd="$2"
  shift 2
  local sources=("$@")
  local path
  if ((${#sources[@]} == 0)); then
    echo "wasm provenance source set is empty" >&2
    return 2
  fi
  for path in "${sources[@]}"; do
    if [[ ! -f "$path" && ! -d "$path" ]]; then
      echo "wasm provenance source path is missing: $path" >&2
      return 2
    fi
  done
  {
    printf 'schema=1\nprofile=%s\nbuilder=%s\n' "$build_profile" "$build_cmd"
    rustc -Vv
    cargo -V
    wasm-bindgen --version
    node --version
    for path in "${sources[@]}"; do
      if [[ -f "$path" ]]; then
        printf 'file=%s\n' "${path#"$repo_root"/}"
        printf 'sha256='
        shasum -a 256 "$path" | awk '{print $1}'
      else
        local file
        while IFS= read -r file; do
          printf 'file=%s\n' "${file#"$repo_root"/}"
          printf 'sha256='
          shasum -a 256 "$file" | awk '{print $1}'
        done < <(find "$path" -type f -print | LC_ALL=C sort)
      fi
    done
  } | shasum -a 256 | awk '{print $1}'
}

record_provenance() {
  local name="$1"
  local digest="$2"
  local manifest="$provenance_root/$name.sha256"
  mkdir -p "$provenance_root"
  local temporary
  temporary="$(mktemp "$provenance_root/$name.XXXXXX")"
  printf '%s\n' "$digest" >"$temporary"
  mv "$temporary" "$manifest"
}

ensure_target_current() {
  local name="$1"
  local build_profile="$2"
  local build_cmd="$3"
  shift 3
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

  local manifest="$provenance_root/$name.sha256"
  local expected_digest recorded_digest=""
  expected_digest="$(source_set_digest "$build_profile" "$build_cmd" "${sources[@]}")"
  if [[ -f "$manifest" ]]; then
    recorded_digest="$(<"$manifest")"
  fi
  if artifacts_exist "${artifacts[@]}" && [[ "$recorded_digest" == "$expected_digest" ]]; then
    echo "wasm: $name $build_profile current"
    return
  fi

  echo "wasm: $name $build_profile inputs changed; running $build_cmd" >&2
  "$build_cmd" "$build_profile"
  if ! artifacts_exist "${artifacts[@]}"; then
    echo "wasm: $name artifacts are missing after build" >&2
    return 1
  fi
  record_provenance "$name" "$expected_digest"
}

sync_wasm_copies() {
  local name="$1"
  local source_js="$2"
  local target_js="$3"
  local source_wasm="$4"
  local target_wasm="$5"
  if cmp -s \
    "$source_js" \
    "$target_js" \
    && cmp -s \
      "$source_wasm" \
      "$target_wasm"; then
    return
  fi

  echo "wasm: synchronizing $name editor copy from its canonical artifact" >&2
  cp "$source_js" "$target_js"
  cp "$source_wasm" "$target_wasm"
  if ! cmp -s \
    "$source_js" \
    "$target_js" \
    || ! cmp -s \
      "$source_wasm" \
      "$target_wasm"; then
    echo "wasm: $name editor copy still differs after synchronization" >&2
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
  tools/wasm_build_environment.sh
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

ensure_target_current \
  puzzle_wasm \
  "$wasm_profile" \
  tools/build_wasm_editor.sh \
  crates/html_editor/static/wasm/puzzle_wasm.js \
  crates/html_editor/static/wasm/puzzle_wasm_bg.wasm \
  -- \
  "${workspace_sources[@]}" \
  tools/build_wasm_editor.sh \
  "${wasm_editor_rust_sources[@]}"

ensure_target_current \
  puzzle_wasm_player \
  "$wasm_profile" \
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
  "$wasm_profile" \
  tools/build_wasm_game.sh \
  crates/html_play/static/wasm_game/puzzle_wasm_game.js \
  crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm \
  crates/html_editor/static/wasm_game/puzzle_wasm_game.js \
  crates/html_editor/static/wasm_game/puzzle_wasm_game_bg.wasm \
  -- \
  "${workspace_sources[@]}" \
  tools/build_wasm_game.sh \
  "${wasm_game_rust_sources[@]}"

sync_wasm_copies \
  puzzle_wasm_player \
  crates/html_play/static/wasm_player/puzzle_wasm_player.js \
  crates/html_editor/static/wasm_player/puzzle_wasm_player.js \
  crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm \
  crates/html_editor/static/wasm_player/puzzle_wasm_player_bg.wasm
sync_wasm_copies \
  puzzle_wasm_game \
  crates/html_play/static/wasm_game/puzzle_wasm_game.js \
  crates/html_editor/static/wasm_game/puzzle_wasm_game.js \
  crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm \
  crates/html_editor/static/wasm_game/puzzle_wasm_game_bg.wasm
ensure_static_asset_copies_match
