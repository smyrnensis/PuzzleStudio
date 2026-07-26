#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

target_dir="${CARGO_TARGET_DIR:-/private/tmp/puzzlebuilder-bevy-target}"

CARGO_TARGET_DIR="$target_dir" tools/ensure_wasm_fresh.sh desktop

export_game() {
  local source="$1"
  local output="$2"
  cargo run --quiet --target-dir "$target_dir" -p html-play -- "$source" -o "$output"
}

export_game games/fixban_tween.puzzle games/fixban_tween.html
export_game games/fixban_tween.puzzle games/fixban_tween_latest.html
export_game games/TPGJ6/locked.puzzle games/locked.html
export_game games/TPGJ6/locked.puzzle games/stuck_room.html
export_game games/move_collision_test.puzzle games/move_collision_test.html
export_game games/spec_3d.puzzle games/spec_3d.html
export_game games/wide_floor_3d.puzzle games/wide_floor_3d.html

echo "tracked standalone game exports regenerated"
