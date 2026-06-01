# PS Next Function Map For 3D

This map records the smallest PuzzleScript Next surfaces to inspect or modify
for the next 3D pass. It is based on the local checkout under
`upstream/PuzzleScriptNext`.

Current status: `three_dimensions` plus ordinary `LEVELS` now routes to the
3D level transport. No separate 3D level section is accepted. Runtime
playback still does not exist, but the compiler now has a boundary gate so
unrouted 3D browser playback is refused explicitly instead of falling through
as a missing 2D `LEVELS` section.

Architecture direction is now fixed in `ARCHITECTURE_3D.md`: 3D must be the
same PuzzleScript implementation form as 2D, with only spatial extension points.
The existing engine is the behavioral reference. A separate 3D prototype path
is acceptable only as temporary scaffolding and must not turn missing 2D
semantics into 3D design differences.

Semantic parity is tracked in `SEMANTIC_PARITY_3D.md`. Metadata and runtime
slots are tracked in `METADATA_CONTRACT_3D.md` and `SLOT_DESIGN_3D.md`. The
expected 3D differences are local to frame, depth, front/back directions, 3D
neighbor/indexing, spatial rule-frame expansion, and rendering.

## Already Present

| Area | File | Current role | Status |
| --- | --- | --- | --- |
| 3D helper | `src/js/levels3d.js` | Parses current internal 3D level transport, validates slice shape, maps 3D coordinates to flat indices. | Keep and test; expose only `three_dimensions` plus `LEVELS` as syntax. |
| Parser section | `src/js/parser.js` | Routes `three_dimensions` + ordinary `LEVELS` into `state.levels3`, treating blank lines as 3D level boundaries and standalone `;` as slice boundary. | Keep as the only author-facing 3D level route. |
| Compiler lowering | `src/js/compiler.js` | `levels3ToArray` converts raw `state.levels3`; `level3FromParsedSource` lowers glyphs into a 3D level object. | Keep, then connect to runtime boundary. |
| Runtime gate | `src/js/compiler.js` | `getRuntimeLevelGateMessage` distinguishes missing 2D levels from parsed but unrouted 3D browser playback. | Keep until upper-layer 3D routing replaces it. |
| Engine shape experiment | `src/js/engine.js` | `Level` accepts optional `depth`/`is3d`, clones preserve 3D shape, and `deltaPositionIndex3` computes depth-aware flat offsets. | Do not treat this as the 3D runtime direction; prefer separate 3D runtime modules from here. |
| Tests | `test/levels3d.test.js` | Covers parser storage, current 3D level validation, coordinate round trip, glyph lowering, background fill. | Keep focused on `three_dimensions` + `LEVELS` canonical cases. |
| Dev note | `DEVELOPMENT.md` | May contain older 3D extension notes. | User change already exists; update cautiously if needed. |

## Touch Next

These are the first runtime surfaces to inspect and modify. The goal is not a
separate reduced 3D gameplay subset. The goal is to recover the 2D contract and
add only the spatial extension points.

| Priority | Function / Surface | File | Why it matters | First action |
| --- | --- | --- | --- | --- |
| 1 | 2D semantic parity audit | `src/js/compiler.js`, `src/js/engine.js` | 3D must match 2D except for space. | List every 3D unsupported feature and classify it as spatial or non-spatial. |
| 1 | shared turn contract | new design note / extracted helper | `late`, commands, again, win, restart, checkpoint, loops, gosub, random, and sound are not 3D-specific. | Define the shared artifacts both dimensions consume. |
| 1 | 3D runtime module | `src/js/runtime3d.js` | This is temporary spatial scaffolding unless it exactly mirrors 2D semantics. | Keep only board/index/neighbor/movement spatial differences here. |
| 1 | 3D slot construction | new helper near 3D runtime/facade | Slots must mirror 2D contract ownership. | Build dimension fields without inventing separate semantic ownership. |
| 1 | compile / post-compile state shape | `src/js/compiler.js` | `levels3ToArray(state)` runs, but no upper-layer runtime choice exists yet. | Keep `state.levels3` separate; do not bridge it into `state.levels`. |
| 1 | shared runtime choice | new small facade or narrow hook near compile/session startup | The upper layer must choose dimension hooks while preserving one semantic contract. | Add a mode detector that does not fork PuzzleScript semantics. |
| 2 | existing `setGameState` | `src/js/engine.js` | This is the current 2D session entry point. | Avoid deep edits; only add a narrow routing hook if no cleaner upper-layer facade exists. |
| 2 | existing `loadLevelFromState` / `loadLevelFromLevelDat` | `src/js/engine.js` | These load `state.levels[curLevelNo]` into the 2D `curLevel`. | Do not use them for 3D level transport. |
| 2 | existing `Level` constructor | `src/js/engine.js` | It is part of the 2D core runtime, despite the current depth experiment. | Do not make it the center of 3D behavior. |
| 2 | existing `dirMasks`, `dirMasksDelta`, movement helpers | `src/js/engine.js` | These are the reference movement contract. | Add dimension-aware direction tables only where space requires it. |
| 3 | `redrawCellGrid` | `src/js/graphics.js` | Renderer assumes 2D `width x height`. | First pass can show a clear unsupported message instead of rendering 3D. |
| 3 | input key mapping | `src/js/inputoutput.js` | Keyboard maps only left/up/right/down/action. | Add 3D input only after engine can consume six directions. |
| 3 | standalone build | `src/js/buildStandalone.js` and HTML shells | `levels3d.js` must be included wherever compile/run needs 3D level transport. | Verify inclusion after runtime boundary is chosen. |

## Do Not Fork

Avoid implementing separate 3D semantics for these areas. They must be shared
or exact 2D equivalents unless a spatial difference is proven:

- `solver.js`
- GIF generation
- sound and animation data
- visual level editor
- tags and mappings expansion
- metadata twiddling
- rich camera, shadow, tween, or sprite editor behavior
- feature-specific 3D shortcuts that skip 2D semantics

## Immediate Implementation Gate

Before implementing more 3D movement, add tests for these boundary behaviors:

1. A file with ordinary `LEVELS` still compiles through the 2D path.
2. A file with `three_dimensions` plus ordinary `LEVELS` produces compiled 3D
   level transport with dimensions.
3. Attempting to play a 3D game does not silently fall back to "No levels found"
   or a malformed 2D level.
4. Non-spatial features are not classified as 3D design differences.

Items 3 and 4 are now covered at the compiler/runtime boundary helper level.
The next gate should exercise the full `compile(...)` path once a lightweight
browser/runtime harness exists.

Passing those tests means the project has a real language-to-dimension boundary.
Only then should six-direction movement and rendering continue.

The boundary should now be implemented as one PuzzleScript semantic path with
dimension-specific spatial hooks, not as a reduced 3D runtime.

## First Rebuild Milestone

The first milestone after resetting the premise is:

- load 2D and 3D levels through the same language pipeline shape
- prove `late` is not treated as a 3D unsupported feature
- prove command/win/session artifacts are dimension-neutral
- prove spatial hooks cover only board indexing, neighbors, direction sets,
  movement resolution, oriented rule frames, and renderer
- keep existing 2D behavior covered by tests

This milestone replaces the old reduced 3D playable subset.
