#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
source tools/wasm_build_environment.sh
configure_reproducible_wasm_build "$repo_root"
game_target_dir="${CARGO_TARGET_DIR:-target}"
wasm_profile="${1:-release}"
if (($# > 1)) || [[ "$wasm_profile" != "debug" && "$wasm_profile" != "release" ]]; then
  echo "usage: tools/build_wasm_game.sh [debug|release]" >&2
  exit 2
fi

if [[ "$wasm_profile" == "release" ]]; then
  target_profile_dir="wasm-player-release"
  cargo build --target wasm32-unknown-unknown --profile wasm-player-release -p puzzle-wasm-game
else
  target_profile_dir="debug"
  cargo build --target wasm32-unknown-unknown -p puzzle-wasm-game
fi

mkdir -p crates/html_play/static/wasm_game
wasm-bindgen \
  --target web \
  --out-dir crates/html_play/static/wasm_game \
  "$game_target_dir/wasm32-unknown-unknown/$target_profile_dir/puzzle_wasm_game.wasm"

if ! grep -q "startEditorPreview" crates/html_play/static/wasm_game/puzzle_wasm_game.js; then
  echo "generated editor preview WASM bindings are missing startEditorPreview" >&2
  exit 1
fi

if ! grep -q "dispatchEditorPreviewCommand" crates/html_play/static/wasm_game/puzzle_wasm_game.js; then
  echo "generated editor preview WASM bindings are missing the typed editor command ingress" >&2
  exit 1
fi

if grep -Eq "WasmStandaloneSession|dispatch\\(action_json\\)|apply_debug_input_name|apply_command_name|apply_input_name|takeEditorPreviewControlResponses|submitEditorPreviewControl" crates/html_play/static/wasm_game/puzzle_wasm_game.js; then
  echo "generated editor preview WASM bindings expose a legacy or parallel session ingress" >&2
  exit 1
fi

if grep -Eq "WasmPuzzle3Runtime|fromFixture|solve_state|suggest_source_completions|highlight_source_html|compile_preview|PuzzleStudioSolve|WasmCoreRuntime|WasmCompiledCoreRuntime" crates/html_play/static/wasm_game/puzzle_wasm_game.js; then
  echo "generated game WASM bindings include editor or solver exports" >&2
  exit 1
fi

if grep -aEq "solve_state|puzzle_solver|PuzzleDomain|SearchBudget|suggest_source_completions|highlight_source_html|compile_preview|PuzzleStudioSolve|WasmCoreRuntime|WasmCompiledCoreRuntime" crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm; then
  echo "generated game WASM binary includes editor, solver, or core-runtime symbols" >&2
  exit 1
fi

verify_wasm_artifacts_have_no_local_paths \
  crates/html_play/static/wasm_game/puzzle_wasm_game.js \
  crates/html_play/static/wasm_game/puzzle_wasm_game.d.ts \
  crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm \
  crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm.d.ts

mkdir -p crates/html_editor/static/wasm_game
cp crates/html_play/static/wasm_game/puzzle_wasm_game.js crates/html_editor/static/wasm_game/puzzle_wasm_game.js
cp crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm crates/html_editor/static/wasm_game/puzzle_wasm_game_bg.wasm
