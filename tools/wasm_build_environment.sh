#!/usr/bin/env bash

configure_reproducible_wasm_build() {
  local repository_root="$1"
  local cargo_binary cargo_home_path local_user_path rust_sysroot remap_flags

  cargo_binary="$(command -v cargo)"
  if [[ -n "${CARGO_HOME:-}" ]]; then
    cargo_home_path="$(cd "$CARGO_HOME" && pwd -P)"
  else
    cargo_home_path="$(cd "$(dirname "$cargo_binary")/.." && pwd -P)"
  fi
  local_user_path="$(dirname "$cargo_home_path")"
  rust_sysroot="$(rustc --print sysroot)"
  remap_flags="--remap-path-prefix=$repository_root=/workspace"
  remap_flags+=" --remap-path-prefix=$local_user_path=/local-user"
  remap_flags+=" --remap-path-prefix=$rust_sysroot=/rust-toolchain"
  export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }$remap_flags"
  wasm_forbidden_local_paths=(
    "$repository_root"
    "$local_user_path"
    "$rust_sysroot"
  )
}

verify_wasm_artifacts_have_no_local_paths() {
  local artifact local_path
  for artifact in "$@"; do
    for local_path in "${wasm_forbidden_local_paths[@]}"; do
      if grep -aFq "$local_path" "$artifact"; then
        echo "generated WASM artifact exposes local build path $local_path: $artifact" >&2
        return 1
      fi
    done
  done
}
