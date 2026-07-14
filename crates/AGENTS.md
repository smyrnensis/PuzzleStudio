# Agent Notes

This folder owns Rust packages. Read this file before changing crate code, then
read the more specific crate `AGENTS.md` when present.

## Crate Boundaries

- `core`: deterministic state, rules, patches, transitions. No file IO, parser
  concerns, rendering, sound, timers, or game-specific UI behavior.
- `grid3d`: deterministic 3D grid primitives, state, rules, patches, levels,
  win checks, and transitions. No parser, session, rendering, sound, timers, or
  host behavior.
- `lang`: `.puzzle` parsing, validation, authoring syntax, compatibility
  imports, semantic surface data, and lowering into compiled model structures.
- `play`: loaded-game session mechanics such as undo, redo, restart, level
  advance, screen flow, progress save data, and display helpers.
- `scene`: shared presentation/flow metadata and layout/component contracts.
- `html_play`, `html_editor`, `cli`, `wasm`: adapters or facades.
  They should not own parser/compiler semantics.
- 3D authoring/runtime responsibilities are split across their normal owners:
  `grid3d` for deterministic spatial mechanics, `lang` for shared `.puzzle`
  syntax/lowering and final spatial materialization, `play` for session flow,
  `runtime_contract` for source-free runtime schemas, and adapters for
  export/presentation behavior. 3D must not independently reinterpret
  non-spatial authoring syntax; share the 2D/authoring helpers and branch only
  for spatial concerns.

## 2D / 3D Drift Guard

Do not introduce new `*3` semantic types when the difference is only dimensional.
Rules, pattern application, write operations, marks, win conditions, session
lifecycle, and export runtime contracts should move toward shared kernel,
language, play, or adapter-owned contracts. `*3` names are acceptable for true
spatial/rendering boundaries such as coordinates, directions, frames, camera,
3D levels, and temporary migration shims that name their deletion boundary.

When a shared contract exposes a variant that 3D cannot support yet, reject it
visibly at the owning boundary instead of mapping it to a nearby supported
behavior.

## Commands

Prefer owner-local commands while developing:

```bash
cargo test -p puzzle-core
cargo test -p puzzle-grid3d
cargo test -p puzzle-lang
cargo test -p puzzle-play
cargo test -p puzzle-scene
```

Use full `cargo test` only when the broader workspace result matters.
