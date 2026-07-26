# PuzzleStudio Bevy Player

This crate is the native host between PuzzleStudio's typed Rust runtime and the
Bevy renderer. It does not serialize a runtime snapshot to JSON.

The host owns the following adapter responsibilities:

- loading a native workspace into a compiled document and `RuntimeSession`;
- mapping native keyboard presses to typed `SessionAction` values;
- preserving presentation-event order and scheduling authored waits;
- resolving animation time with `puzzle-presentation`;
- routing `RuntimeResolvedRenderFrame` values to the active 2D or 3D backend;
- synchronizing the resolved 2D view or authored 3D camera with the Bevy camera.

Visual lookup, palette resolution, transforms, animation conflicts, render
priority, merge groups, and color composition remain in the Rust presentation
owner. Meshes, materials, entity reconciliation, camera execution, shadows,
and GPU work remain in the Bevy renderer.

Run the current 3D game fixture from the worktree root:

```bash
cargo run -p puzzle-bevy-player -- games/TENETEN3D.puzzle
```

Run a directly playable 2D fixture:

```bash
cargo run -p puzzle-bevy-player -- games/animation_test.puzzle
```

The player uses the puzzle's declared key bindings. For TENETEN3D, arrow keys
or WASD move horizontally, E/Space moves up, and Q moves down. The host also
provides Z for undo, Y for redo, R for restart, N for next level, and P for the
previous level when the typed snapshot declares those actions available.

Audio presentation events remain ordered with waits and animations inside
`PuzzleBevyPlayerHost`. The host resolves them through `puzzle_audio::AudioRuntime`
into typed device commands, and the native plugin executes those commands with
Bevy audio sources for finite SFX and streaming looped music.

The 2D host consumes the runtime-resolved view origin and size and submits typed
line decorations with each resolved frame. External images are decoded by the
native asset adapter into a `DecodedVisualImageCatalog`; presentation and the
renderer share that catalog by asset ID and revision without putting image
bytes in the player snapshot.

Progress persistence stays behind typed host operations. A platform resource
may restore the runtime-owned save payload, inspect the pending
`RuntimeProgressSaveRequest`, execute its typed write or delete operation, and
acknowledge only its exact request ID after success. Each mutation refreshes
the host snapshot and projected viewports; storage-device IO remains outside
this crate.
