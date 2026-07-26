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

mkdir -p crates/html_play/static/wasm_player
wasm-bindgen \
  --target web \
  --out-dir crates/html_play/static/wasm_player \
  "$player_target_dir/wasm32-unknown-unknown/$target_profile_dir/puzzle_wasm_player.wasm"

if ! grep -q "export class WasmStandaloneSession" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings are missing WasmStandaloneSession" >&2
  exit 1
fi

if ! grep -q "dispatch(action_json)" crates/html_play/static/wasm_player/puzzle_wasm_player.js; then
  echo "generated player WASM bindings are missing typed session dispatch" >&2
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

mkdir -p crates/html_editor/static/wasm_player
cp crates/html_play/static/wasm_player/puzzle_wasm_player.js crates/html_editor/static/wasm_player/puzzle_wasm_player.js
cp crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm crates/html_editor/static/wasm_player/puzzle_wasm_player_bg.wasm
