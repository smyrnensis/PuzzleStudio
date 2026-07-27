#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
source tools/wasm_build_environment.sh
configure_reproducible_wasm_build "$repo_root"
player_target_dir="${CARGO_TARGET_DIR:-target}"
wasm_profile="${1:-release}"
if (($# > 1)) || [[ "$wasm_profile" != "debug" && "$wasm_profile" != "release" ]]; then
  echo "usage: tools/build_wasm_player.sh [debug|release]" >&2
  exit 2
fi

if [[ "$wasm_profile" == "release" ]]; then
  player_cargo_profile_args=(--profile wasm-player-release)
  player_target_profile_dir="wasm-player-release"
else
  player_cargo_profile_args=()
  player_target_profile_dir="debug"
fi
max_player_wasm_bytes=45000000
max_player_glue_bytes=200000
max_audio_worklet_bundle_bytes=1000000
worklet_build_dir="$(mktemp -d)"
trap 'rm -rf "$worklet_build_dir"' EXIT

if cargo tree \
  --package puzzle-wasm-player \
  --target wasm32-unknown-unknown \
  --edges normal,build |
  grep -q "puzzle-presentation-json"; then
  echo "official player dependency graph includes the development JSON projection adapter" >&2
  exit 1
fi

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
  "${player_cargo_profile_args[@]}" \
  -p puzzle-audio-worklet
wasm-bindgen \
  --target web \
  --out-dir "$worklet_build_dir" \
  "$player_target_dir/wasm32-unknown-unknown/$player_target_profile_dir/puzzle_audio_worklet.wasm"
mkdir -p crates/web_audio/generated
node tools/build_audio_worklet_bundle.mjs \
  "$worklet_build_dir/puzzle_audio_worklet.js" \
  "$worklet_build_dir/puzzle_audio_worklet_bg.wasm" \
  crates/audio_worklet/worklet.js \
  crates/web_audio/generated/puzzle_audio_worklet.js
if [[ "$wasm_profile" == "release" ]]; then
  check_artifact_size \
    crates/web_audio/generated/puzzle_audio_worklet.js \
    "$max_audio_worklet_bundle_bytes" \
    "audio worklet bundle"
fi

cargo build \
  --target wasm32-unknown-unknown \
  "${player_cargo_profile_args[@]}" \
  -p puzzle-wasm-player
mkdir -p crates/html_play/static/wasm_player
wasm-bindgen \
  --target web \
  --out-dir crates/html_play/static/wasm_player \
  "$player_target_dir/wasm32-unknown-unknown/$player_target_profile_dir/puzzle_wasm_player.wasm"

if ! grep -q "export function startStandalonePlayer" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings are missing the Bevy standalone launcher" >&2
  exit 1
fi

if grep -Eq "WasmStandaloneSession|startEditorPreview|dispatchEditorPreviewCommand|dispatch\\(action_json\\)|snapshot\\(\\)|presentation_frame|presentation_event_consumed|progress_storage_key|progress_storage_save_version|apply_command_name|apply_input_name|apply_audio_command" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
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

if grep -Eq "WasmPuzzle3Runtime|fromFixture|solve_state|suggest_source_completions|highlight_source_html|compile_preview|PuzzleStudioSolve|WasmCoreRuntime|WasmCompiledCoreRuntime" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings include non-player exports" >&2
  exit 1
fi

if grep -aEq "/api/debug/input/|apply_debug_input_name_json|apply_traced_input|debug_transition_value|solve_state|puzzle_solver|PuzzleDomain|SearchBudget|suggest_source_completions|highlight_source_html|compile_preview|PuzzleStudioSolve|WasmCoreRuntime|WasmCompiledCoreRuntime|unknown puzzle directive|unknown scene directive|parse_game_for_path|parse_puzzle3d|puzzle source must use" crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm; then
  echo "generated player WASM binary includes editor debug, source parser, editor, solver, or non-player symbols" >&2
  exit 1
fi

if [[ "$wasm_profile" == "release" ]]; then
  check_artifact_size \
    crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm \
    "$max_player_wasm_bytes" \
    "standalone player WASM"
  check_artifact_size \
    crates/html_play/static/wasm_player/puzzle_wasm_player.js \
    "$max_player_glue_bytes" \
    "standalone player generated glue"
fi

verify_wasm_artifacts_have_no_local_paths \
  crates/web_audio/generated/puzzle_audio_worklet.js \
  crates/html_play/static/wasm_player/puzzle_wasm_player.js \
  crates/html_play/static/wasm_player/puzzle_wasm_player.d.ts \
  crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm \
  crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm.d.ts

mkdir -p crates/html_editor/static/wasm_player
cp crates/html_play/static/wasm_player/puzzle_wasm_player.js crates/html_editor/static/wasm_player/puzzle_wasm_player.js
cp crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm crates/html_editor/static/wasm_player/puzzle_wasm_player_bg.wasm
