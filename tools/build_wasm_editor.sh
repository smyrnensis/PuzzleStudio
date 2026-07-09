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

if ! grep -q "export function create_source_analysis_handle" crates/html_editor/static/wasm/puzzle_wasm.js; then
  echo "generated WASM bindings are missing create_source_analysis_handle" >&2
  exit 1
fi

if ! grep -q "export function source_analysis_outline_json" crates/html_editor/static/wasm/puzzle_wasm.js; then
  echo "generated WASM bindings are missing source_analysis_outline_json" >&2
  exit 1
fi

if grep -Eq "export class WasmSourceAnalysis|export function analyze_source|export function highlight_source_html|export function highlight_source_json|export function source_outline_json|export function suggest_source_completions|export function resolve_source_target|export function source_entries_json" crates/html_editor/static/wasm/puzzle_wasm.js; then
  echo "generated editor WASM bindings include old source analysis exports" >&2
  exit 1
fi

if grep -Eq "WasmCoreRuntime|WasmPuzzle3Runtime|WasmStandaloneSession|transition_program_outcome" crates/html_editor/static/wasm/puzzle_wasm.js; then
  echo "generated editor WASM bindings include runtime exports" >&2
  exit 1
fi
