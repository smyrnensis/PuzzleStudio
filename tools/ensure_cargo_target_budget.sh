#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$repo_root"

if [[ -n "${CARGO_TARGET_DIR+x}" ]]; then
  echo "cargo cache budget: CARGO_TARGET_DIR is externally managed; skipping project target cleanup"
  exit 0
fi

max_gib_raw="${PUZZLE_CARGO_TARGET_MAX_GIB:-25}"
if [[ ! "$max_gib_raw" =~ ^[1-9][0-9]{0,3}$ ]] \
  || ((10#$max_gib_raw > 1024)); then
  echo "cargo cache budget must be an integer from 1 to 1024 GiB: $max_gib_raw" >&2
  exit 2
fi
max_gib="$((10#$max_gib_raw))"

case "$(uname -s)" in
  Darwin)
    cache_root="${HOME:?HOME is required}/Library/Caches/PuzzleBuilder"
    repo_key="$(md5 -qs "$repo_root")"
    ;;
  Linux)
    cache_root="${XDG_CACHE_HOME:-${HOME:?HOME is required}/.cache}/PuzzleBuilder"
    if [[ "$cache_root" != /* ]]; then
      echo "managed Cargo cache root must be absolute: $cache_root" >&2
      exit 1
    fi
    repo_key="$(sha256sum <<<"$repo_root")"
    repo_key="${repo_key%% *}"
    ;;
  *)
    echo "unsupported platform for managed Cargo cache; set CARGO_TARGET_DIR explicitly" >&2
    exit 1
    ;;
esac

mkdir -p "$cache_root/workspaces"
physical_cache_root="$(cd "$cache_root" && pwd -P)"
if [[ "$physical_cache_root" != "$cache_root" ]]; then
  echo "managed Cargo cache root resolves through an unexpected symlink: $physical_cache_root" >&2
  exit 1
fi
physical_workspaces_root="$(cd "$cache_root/workspaces" && pwd -P)"
if [[ "$physical_workspaces_root" != "$cache_root/workspaces" ]]; then
  echo "managed Cargo workspaces root resolves through an unexpected symlink: $physical_workspaces_root" >&2
  exit 1
fi

expected_target_dir="$cache_root/workspaces/$repo_key/target"
expected_feedback_target_dir="$expected_target_dir/puzzle_feedback"
owner_marker="$expected_target_dir/.puzzlestudio-cargo-target-owner"
target_path="$repo_root/target"
feedback_target_path="$repo_root/tools/puzzle_feedback/target"

create_owner_marker() {
  if ! (set -o noclobber; printf '%s\n' "$repo_root" >"$owner_marker") 2>/dev/null; then
    echo "refusing to overwrite Cargo target ownership marker: $owner_marker" >&2
    exit 1
  fi
}

if [[ ! -e "$expected_target_dir" && ! -L "$expected_target_dir" ]]; then
  mkdir -p "$expected_target_dir"
fi
if [[ ! -d "$expected_target_dir" || -L "$expected_target_dir" ]]; then
  echo "managed Cargo target must be a real directory: $expected_target_dir" >&2
  exit 1
fi
physical_target_dir="$(cd "$expected_target_dir" && pwd -P)"
if [[ "$physical_target_dir" != "$expected_target_dir" ]]; then
  echo "managed Cargo target resolves outside its owned path: $physical_target_dir" >&2
  exit 1
fi
if [[ -L "$owner_marker" ]]; then
  echo "Cargo target ownership marker must not be a symlink: $owner_marker" >&2
  exit 1
fi
if [[ ! -e "$owner_marker" ]]; then
  if find "$expected_target_dir" -mindepth 1 -maxdepth 1 -print -quit | grep -q .; then
    echo "managed Cargo target is non-empty and missing its ownership marker: $expected_target_dir" >&2
    exit 1
  fi
  create_owner_marker
fi
if [[ ! -f "$owner_marker" || "$(<"$owner_marker")" != "$repo_root" ]]; then
  echo "managed Cargo target belongs to a different workspace: $expected_target_dir" >&2
  exit 1
fi

ensure_managed_link() {
  local link_path="$1"
  local expected_path="$2"
  local label="$3"
  if [[ ! -e "$link_path" && ! -L "$link_path" ]]; then
    ln -s "$expected_path" "$link_path"
  fi
  if [[ ! -L "$link_path" ]]; then
    echo "$label must be a symlink to $expected_path: $link_path" >&2
    exit 1
  fi
  local link_target
  link_target="$(readlink "$link_path")"
  if [[ "$link_target" != "$expected_path" ]]; then
    echo "$label points to an unexpected path: $link_target" >&2
    echo "expected: $expected_path" >&2
    exit 1
  fi
}

mkdir -p "$expected_feedback_target_dir"
if [[ ! -d "$expected_feedback_target_dir" || -L "$expected_feedback_target_dir" ]]; then
  echo "managed puzzle_feedback target must be a real directory: $expected_feedback_target_dir" >&2
  exit 1
fi
physical_feedback_target_dir="$(cd "$expected_feedback_target_dir" && pwd -P)"
if [[ "$physical_feedback_target_dir" != "$expected_feedback_target_dir" ]]; then
  echo "managed puzzle_feedback target resolves outside its owned path: $physical_feedback_target_dir" >&2
  exit 1
fi
ensure_managed_link "$target_path" "$expected_target_dir" "managed Cargo target"
ensure_managed_link \
  "$feedback_target_path" \
  "$expected_feedback_target_dir" \
  "managed puzzle_feedback target"

size_kib="$(du -sk "$expected_target_dir" | awk '{print $1}')"
max_kib="$((max_gib * 1024 * 1024))"
if ((size_kib <= max_kib)); then
  size_mib="$((size_kib / 1024))"
  echo "cargo cache budget: ${size_mib} MiB / ${max_gib} GiB"
  exit 0
fi

size_gib="$((size_kib / 1024 / 1024))"
echo "cargo cache budget exceeded: ${size_gib} GiB / ${max_gib} GiB; cleaning $expected_target_dir"
if [[ "$(cd "$expected_target_dir" && pwd -P)" != "$expected_target_dir" ]] \
  || [[ -L "$expected_feedback_target_dir" ]] \
  || [[ "$(cd "$expected_feedback_target_dir" && pwd -P)" != "$expected_feedback_target_dir" ]] \
  || [[ -L "$owner_marker" ]] \
  || [[ ! -f "$owner_marker" ]] \
  || [[ "$(<"$owner_marker")" != "$repo_root" ]] \
  || [[ "$(readlink "$target_path")" != "$expected_target_dir" ]] \
  || [[ "$(readlink "$feedback_target_path")" != "$expected_feedback_target_dir" ]]; then
  echo "managed Cargo target ownership changed during the budget check" >&2
  exit 1
fi
cargo clean --target-dir "$expected_target_dir"
mkdir -p "$expected_feedback_target_dir"
create_owner_marker
