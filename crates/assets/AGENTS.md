# Agent Notes

This crate owns source-independent decoding and validation of immutable game
assets.

## Boundaries

- Consume typed asset formats and encoded bytes supplied by a host.
- Do not read files, fetch URLs, inspect `.puzzle` source, or own renderer/GPU
  resources.
- Decode the same bytes through the same Rust implementation on native and
  Wasm targets.
- Reject unsupported formats, format mismatches, corrupt payloads, and invalid
  dimensions explicitly.

## Commands

```bash
cargo test -p puzzle-assets
cargo check -p puzzle-assets --target wasm32-unknown-unknown
```
