#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

mode="${1:-desktop}"
if (($# > 1)); then
  echo "usage: tools/prepare_desktop_build.sh [desktop|desktop-dev]" >&2
  exit 2
fi

tools/ensure_cargo_target_budget.sh
exec tools/ensure_wasm_fresh.sh "$mode"
