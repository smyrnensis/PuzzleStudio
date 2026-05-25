#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Use cargo as the entry point so include_str! dependencies in editor/html-play
# are fingerprinted before generating the standalone editor HTML.
tools/build_wasm_editor.sh
exec cargo run --release -p html-editor -- "$@"
