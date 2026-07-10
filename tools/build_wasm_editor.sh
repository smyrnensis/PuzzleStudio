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

if ! grep -q "export function activate_source_analysis" crates/html_editor/static/wasm/puzzle_wasm.js; then
  echo "generated WASM bindings are missing activate_source_analysis" >&2
  exit 1
fi

if ! grep -q "export function active_source_analysis_outline_json" crates/html_editor/static/wasm/puzzle_wasm.js; then
  echo "generated WASM bindings are missing active_source_analysis_outline_json" >&2
  exit 1
fi

if ! grep -q "export function active_source_analysis_level_editor_manifest_json" crates/html_editor/static/wasm/puzzle_wasm.js; then
  echo "generated WASM bindings are missing active_source_analysis_level_editor_manifest_json" >&2
  exit 1
fi

if ! grep -q "export function active_source_analysis_level_editor_level_slots" crates/html_editor/static/wasm/puzzle_wasm.js; then
  echo "generated WASM bindings are missing active_source_analysis_level_editor_level_slots" >&2
  exit 1
fi

if ! grep -q "export function active_source_analysis_level_editor_sprite_json" crates/html_editor/static/wasm/puzzle_wasm.js; then
  echo "generated WASM bindings are missing active_source_analysis_level_editor_sprite_json" >&2
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

node tools/check_wasm_editor_preview.mjs
