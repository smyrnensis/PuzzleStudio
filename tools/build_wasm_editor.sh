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
