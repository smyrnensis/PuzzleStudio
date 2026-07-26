#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
player_target_dir="${CARGO_TARGET_DIR:-target}"
wasm_profile="${1:-release}"
if (($# > 1)) || [[ "$wasm_profile" != "debug" && "$wasm_profile" != "release" ]]; then
  echo "usage: tools/build_wasm_player.sh [debug|release]" >&2
  exit 2
fi

if [[ "$wasm_profile" == "release" ]]; then
  target_profile_dir="release"
  cargo build --target wasm32-unknown-unknown --release -p puzzle-wasm-player
else
  target_profile_dir="debug"
  cargo build --target wasm32-unknown-unknown -p puzzle-wasm-player
fi

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
player_target_dir="${CARGO_TARGET_DIR:-/private/tmp/puzzlebuilder-bevy-target}"
player_profile="wasm-player-release"
max_player_wasm_bytes=45000000
max_player_glue_bytes=200000
max_audio_worklet_bundle_bytes=1000000
worklet_build_dir="$(mktemp -d)"
trap 'rm -rf "$worklet_build_dir"' EXIT

check_artifact_size() {
  local artifact_path="$1"
  local max_bytes="$2"
  local artifact_name="$3"
  local actual_bytes
  actual_bytes="$(wc -c < "$artifact_path" | tr -d '[:space:]')"
  if (( actual_bytes > max_bytes )); then
    echo "$artifact_name is $actual_bytes bytes, exceeding the $max_bytes-byte export budget" >&2
    exit 1
  fi
  echo "$artifact_name: $actual_bytes bytes (budget $max_bytes)"
}
cargo build \
  --target wasm32-unknown-unknown \
  --target-dir "$player_target_dir" \
  --profile "$player_profile" \
  -p puzzle-audio-worklet
wasm-bindgen \
  --target web \
  --out-dir "$worklet_build_dir" \
  "$player_target_dir/wasm32-unknown-unknown/$player_profile/puzzle_audio_worklet.wasm"
mkdir -p crates/web_audio/generated
node tools/build_audio_worklet_bundle.mjs \
  "$worklet_build_dir/puzzle_audio_worklet.js" \
  "$worklet_build_dir/puzzle_audio_worklet_bg.wasm" \
  crates/audio_worklet/worklet.js \
  crates/web_audio/generated/puzzle_audio_worklet.js
check_artifact_size \
  crates/web_audio/generated/puzzle_audio_worklet.js \
  "$max_audio_worklet_bundle_bytes" \
  "audio worklet bundle"

cargo build \
  --target wasm32-unknown-unknown \
  --target-dir "$player_target_dir" \
  --profile "$player_profile" \
  -p puzzle-wasm-player
=======
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
mkdir -p crates/html_play/static/wasm_player
wasm-bindgen \
  --target web \
  --out-dir crates/html_play/static/wasm_player \
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  "$player_target_dir/wasm32-unknown-unknown/$player_profile/puzzle_wasm_player.wasm"
=======
  "$player_target_dir/wasm32-unknown-unknown/$target_profile_dir/puzzle_wasm_player.wasm"
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544

if ! grep -q "export function startStandalonePlayer" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings are missing the Bevy standalone launcher" >&2
  exit 1
fi

if grep -Eq "WasmStandaloneSession|dispatch\\(action_json\\)|snapshot\\(\\)|presentation_frame|presentation_event_consumed|progress_storage_key|progress_storage_save_version|apply_command_name|apply_input_name|apply_audio_command" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings still expose the legacy JSON session bridge" >&2
  exit 1
fi

if grep -Eq "set_current_state|apply_debug_input_name" \
  crates/html_play/static/wasm_player/puzzle_wasm_player.js \
  crates/html_play/static/wasm_player/puzzle_wasm_player.d.ts \
  crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm.d.ts; then
  echo "generated player WASM bindings expose editor-only state mutation" >&2
  exit 1
fi

if grep -aEq "set_current_state|decode_state_json|apply_debug_input_name" crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm; then
  echo "generated player WASM binary includes editor-only state mutation" >&2
  exit 1
fi

if ! grep -q "resolve_scene_presentation(scene_name, state_json)" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings are missing Rust-owned scene presentation resolution" >&2
  exit 1
fi

if ! grep -q "resolve_render_frame(render_scene_json, elapsed_ms)" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings are missing Rust-owned render-frame resolution" >&2
  exit 1
fi

if ! grep -q "hydrate_render_scene_images(render_scene_json, image_assets_json)" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings are missing decoded-image hydration" >&2
  exit 1
fi

if ! grep -q "resolve_render_moment(render_scene_json, render_moment_json)" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings are missing animation-aware render resolution" >&2
  exit 1
fi

if ! grep -q "project_renderer_state(runtime_export_json, state_json, level_index)" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings are missing typed renderer-state projection" >&2
  exit 1
fi

if grep -Eq "WasmPuzzle3Runtime|fromFixture|solve_state|suggest_source_completions|highlight_source_html|compile_preview|PuzzleStudioSolve|WasmCoreRuntime|WasmCompiledCoreRuntime" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings include non-player exports" >&2
  exit 1
fi

if grep -aEq "/api/debug/input/|apply_debug_input_name_json|apply_traced_input|debug_transition_value|solve_state|puzzle_solver|PuzzleDomain|SearchBudget|suggest_source_completions|highlight_source_html|compile_preview|PuzzleStudioSolve|WasmCoreRuntime|WasmCompiledCoreRuntime|unknown puzzle directive|unknown scene directive|parse_game_for_path|parse_puzzle3d|puzzle source must use" crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm; then
  echo "generated player WASM binary includes editor debug, source parser, editor, solver, or non-player symbols" >&2
  exit 1
fi

check_artifact_size \
  crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm \
  "$max_player_wasm_bytes" \
  "standalone player WASM"
check_artifact_size \
  crates/html_play/static/wasm_player/puzzle_wasm_player.js \
  "$max_player_glue_bytes" \
  "standalone player generated glue"

mkdir -p crates/html_editor/static/wasm_player
cp crates/html_play/static/wasm_player/puzzle_wasm_player.js crates/html_editor/static/wasm_player/puzzle_wasm_player.js
cp crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm crates/html_editor/static/wasm_player/puzzle_wasm_player_bg.wasm
