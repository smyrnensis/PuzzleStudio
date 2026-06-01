#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build --target wasm32-unknown-unknown --release -p puzzle-wasm
mkdir -p crates/html_editor/static/wasm
wasm-bindgen \
  --target web \
  --out-dir crates/html_editor/static/wasm \
  target/wasm32-unknown-unknown/release/puzzle_wasm.wasm

if ! grep -q "export function suggest_source_completions" crates/html_editor/static/wasm/puzzle_wasm.js; then
  echo "generated WASM bindings are missing suggest_source_completions" >&2
  exit 1
fi

if grep -Eq "WasmCoreRuntime|WasmPuzzle3Runtime|WasmStandaloneSession|transition_program_outcome" crates/html_editor/static/wasm/puzzle_wasm.js; then
  echo "generated editor WASM bindings include runtime exports" >&2
  exit 1
fi
