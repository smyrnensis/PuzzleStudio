#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build --target wasm32-unknown-unknown --release -p puzzle-wasm-game
mkdir -p crates/html_play/static/wasm_game
wasm-bindgen \
  --target web \
  --out-dir crates/html_play/static/wasm_game \
  target/wasm32-unknown-unknown/release/puzzle_wasm_game.wasm

if ! grep -q "export class WasmStandaloneSession" crates/html_play/static/wasm_game/puzzle_wasm_game.js; then
  echo "generated game WASM bindings are missing WasmStandaloneSession" >&2
  exit 1
fi

if ! grep -q "dispatch(action_json)" crates/html_play/static/wasm_game/puzzle_wasm_game.js; then
  echo "generated game WASM bindings are missing typed session dispatch" >&2
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

if ! grep -aFq "debug_transition_value" crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm; then
  echo "generated editor game WASM binary is missing the editor debug endpoint" >&2
  exit 1
fi

mkdir -p crates/html_editor/static/wasm_game
cp crates/html_play/static/wasm_game/puzzle_wasm_game.js crates/html_editor/static/wasm_game/puzzle_wasm_game.js
cp crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm crates/html_editor/static/wasm_game/puzzle_wasm_game_bg.wasm
