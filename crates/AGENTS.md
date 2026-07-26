# Agent Notes

This folder owns Rust packages. Read this file before changing crate code, then
read the more specific crate `AGENTS.md` when present.

## Crate Boundaries

- `core`: deterministic dimension-generic state, rules, patches, levels, goals,
  and transitions. No file IO, parser concerns, rendering, sound, timers, or
  game-specific UI behavior.
- `lang`: `.puzzle` parsing, validation, authoring syntax, compatibility
  imports, semantic surface data, and lowering into compiled model structures.
- `play`: loaded-game session mechanics such as undo, redo, restart, level
  advance, screen flow, progress save data, and display helpers.
- `presentation`: renderer-neutral visual ordering, composition, and animation
  planning over compiled, source-free contracts. No browser, GPU, scene graph,
  file IO, or parser behavior.
- `session_contract`: the complete typed runtime snapshot consumed by native
  presentation backends.
- `presentation_json`: the browser JSON transport adapter for that snapshot;
  it owns wire conversion but no game or renderer semantics.
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
- `audio_contract`: typed audio asset, voice, capability, and device-command
  contracts. It owns no synthesis or platform API.
- `audio`: deterministic seeded SFX/music generation, canonical block
  rendering, catalog resolution, and playback lifecycle.
- `audio_worklet`: the dedicated browser audio-thread Rust renderer. Its
  adjacent JavaScript is generated binding/processor transport only.
- `web_audio`: WebAudio device submission and capability feedback over resolved
  audio assets and typed commands.
=======
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
- `scene`: shared presentation/flow metadata and layout/component contracts.
- `html_play`, `html_editor`, `cli`, `wasm`: adapters or facades.
  They should not own parser/compiler semantics.
- Spatial authoring/runtime responsibilities are split across their normal owners:
  `core` for deterministic grid mechanics, `lang` for shared `.puzzle`
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
cargo test -p puzzle-lang
cargo test -p puzzle-play
cargo test -p puzzle-scene
```

Use full `cargo test` only when the broader workspace result matters.
