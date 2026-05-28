#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build --target wasm32-unknown-unknown --release -p puzzle-wasm-core
mkdir -p crates/wasm_core/static
wasm-bindgen \
  --target web \
  --out-dir crates/wasm_core/static \
  target/wasm32-unknown-unknown/release/puzzle_core_wasm.wasm

if ! grep -q "WasmCompiledCoreRuntime" crates/wasm_core/static/puzzle_core_wasm.js; then
  echo "generated core WASM bindings are missing WasmCompiledCoreRuntime" >&2
  exit 1
fi
