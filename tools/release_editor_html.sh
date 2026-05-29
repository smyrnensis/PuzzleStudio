#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# editor.html is a stable app artifact: generation embeds the latest WASM at
# release time, and the exported file keeps that fixed WASM until regenerated.
tools/build_wasm_editor.sh
tools/build_wasm_game.sh
exec cargo run --release -p html-editor -- "$@"
