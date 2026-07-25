# Agent Notes

This crate owns the native Bevy player host.

## Boundaries

- Drive `puzzle-game-runtime` only through typed `SessionAction` and
  `RuntimeSessionSnapshot` APIs.
- Resolve presentation time through `puzzle-presentation`, then submit the
  resulting typed frame to `puzzle-bevy-renderer` without JSON.
- Own native keyboard mapping, presentation wait scheduling, and Bevy app
  lifecycle. Do not parse PuzzleStudio authoring syntax except through
  `RuntimeSession::from_source` at the file-loading boundary.
- Do not recreate visual, palette, animation, priority, or composition
  semantics.
- Route typed resolved frames to the active 2D or 3D backend. Missing active
  scenes and unresolved backend-neutral presentation contracts fail explicitly.

## Commands

```bash
cargo test -p puzzle-bevy-player
cargo check -p puzzle-bevy-player --bin puzzle-bevy-player
cargo run -p puzzle-bevy-player -- games/TENETEN3D.puzzle3
cargo check -p puzzle-bevy-player --lib --target wasm32-unknown-unknown
```
