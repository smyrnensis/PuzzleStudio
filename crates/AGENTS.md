# Agent Notes

This folder owns Rust packages. Read this file before changing crate code, then
read the more specific crate `AGENTS.md` when present.

## Crate Boundaries

- `core`: deterministic state, rules, patches, transitions. No file IO, parser
  concerns, rendering, sound, timers, or game-specific UI behavior.
- `lang`: `.puzzle` parsing, validation, authoring syntax, compatibility
  imports, semantic surface data, and lowering into compiled model structures.
- `play`: loaded-game session mechanics such as undo, redo, restart, level
  advance, screen flow, progress save data, and display helpers.
- `scene`: shared presentation/flow metadata and layout/component contracts.
- `html_play`, `ascii_play`, `html_editor`, `cli`, `wasm`: adapters or facades.
  They should not own parser/compiler semantics.
- `puzzle3d_model`: 3D parser/runtime experiment. Keep shared scene and document
  contracts aligned with the 2D language path.

## Commands

Prefer owner-local commands while developing:

```bash
cargo test -p puzzle-core
cargo test -p puzzle-lang
cargo test -p puzzle-play
cargo test -p puzzle-scene
```

Use full `cargo test` only when the broader workspace result matters.
