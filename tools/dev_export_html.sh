#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Dev game exports should reflect the current Rust/WASM implementation.
# Rebuilding is usually a cheap no-op when Cargo's cache is already fresh.
tools/build_wasm_game.sh
tools/build_wasm_core.sh
exec cargo run -p html-play -- "$@"
