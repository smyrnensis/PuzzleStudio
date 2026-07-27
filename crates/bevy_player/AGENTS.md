# Agent Notes

This crate owns the native Bevy player host.

## Boundaries

- Drive `puzzle-game-runtime` only through typed `SessionAction` and
  `RuntimeSessionSnapshot` APIs.
- Resolve presentation time through `puzzle-presentation`, then submit the
  resulting typed frame to `puzzle-bevy-renderer` without JSON.
- Own native keyboard mapping, presentation wait scheduling, and Bevy app
  lifecycle. Load native source trees through `puzzle-workspace::FileWorkspace`,
  then construct the runtime from the compiled `LoadedDocument`.
- Do not recreate visual, palette, animation, priority, or composition
  semantics.
- Route typed resolved frames to the active 2D or 3D backend. Missing active
  scenes and unresolved backend-neutral presentation contracts fail explicitly.
- Editor pointer press, move, release, and leave gestures resolve only against
  the committed typed authoring frame. Keep hit selection and highlight state
  here rather than reconstructing renderer geometry in a browser host.

## Commands

```bash
cargo test -p puzzle-bevy-player
cargo check -p puzzle-bevy-player --bin puzzle-bevy-player
cargo run -p puzzle-bevy-player -- games/TENETEN3D.puzzle
cargo check -p puzzle-bevy-player --lib --target wasm32-unknown-unknown
```
