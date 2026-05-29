# PS Next Function Map For PS3D

This map records the smallest PuzzleScript Next surfaces to inspect or modify
for the next PS3D pass. It is based on the local checkout under
`upstream/PuzzleScriptNext`.

Current status: `LEVELS3` parsing and lowering exist, but playable runtime
integration does not.

## Already Present

| Area | File | Current role | Status |
| --- | --- | --- | --- |
| 3D helper | `src/js/ps3d.js` | Parses `LEVELS3`, validates slice shape, maps 3D coordinates to flat indices. | Keep and test. |
| Parser section | `src/js/parser.js` | Recognizes `levels3`, stores raw level lines, treats blank lines as 3D level boundaries and standalone `;` as slice boundary. | Keep narrow. |
| Compiler lowering | `src/js/compiler.js` | `levels3ToArray` converts raw `state.levels3`; `level3FromParsedSource` lowers glyphs into a 3D level object. | Keep, then connect to runtime boundary. |
| Tests | `test/ps3d.test.js` | Covers parser storage, `LEVELS3` validation, coordinate round trip, glyph lowering, background fill. | Extend before runtime changes. |
| Dev note | `DEVELOPMENT.md` | Records `LEVELS3` extension notes. | User change already exists; do not overwrite casually. |

## Touch Next

These are the first runtime files to inspect and modify. The goal is not full
3D gameplay yet; the goal is preventing `LEVELS3` from remaining compiler-only
or being misread as a 2D board.

| Priority | Function / Surface | File | Why it matters | First action |
| --- | --- | --- | --- | --- |
| 1 | `compile` / post-compile state shape | `src/js/compiler.js` | `levels3ToArray(state)` runs, but the playable engine still consumes `state.levels`. | Decide whether 3D mode bridges `state.levels3` into a named runtime entry or refuses play explicitly. |
| 1 | `setGameState` | `src/js/engine.js` | This is where compiled state becomes live play state. | Add an explicit `state.levels3` guard or dispatch before normal 2D level loading. |
| 1 | `loadLevelFromState` / `loadLevelFromLevelDat` | `src/js/engine.js` | These load `state.levels[curLevelNo]` into `curLevel`. | Ensure 3D levels are not loaded through 2D assumptions. |
| 1 | `Level` constructor | `src/js/engine.js` | Current `Level` stores `width`, `height`, and `n_tiles = width * height`. | Add a deliberate 3D shape path or keep 3D out with a clear error. |
| 1 | `restoreLevel` / backups | `src/js/engine.js` | Undo/restart snapshots currently store 2D dimensions. | Block or extend snapshots for `depth` before allowing play. |
| 2 | `dirMasks`, `dirMasksDelta`, `deltaPositionIndex` | `src/js/engine.js` | Movement is 2D: `deltaPositionIndex(level, positionIndex, x, y)`. | Do not add `front/back/up/down` globally until 3D-only dispatch exists. |
| 2 | `repositionEntitiesOnLayer` / `repositionEntitiesAtCell` | `src/js/engine.js` | Collision and movement bounds are 2D. | Extend only inside the 3D runtime path. |
| 2 | rule matching helpers using `d` | `src/js/engine.js` | Rule matching compiles directional offsets into flat deltas. | Map 3D direction deltas only after the `Level` shape is dimension-aware. |
| 3 | `redrawCellGrid` | `src/js/graphics.js` | Renderer assumes 2D `width x height`. | First pass can show a clear unsupported message instead of rendering 3D. |
| 3 | input key mapping | `src/js/inputoutput.js` | Keyboard maps only left/up/right/down/action. | Add 3D input only after engine can consume six directions. |
| 3 | standalone build | `src/js/buildStandalone.js` and HTML shells | `ps3d.js` must be included wherever compile/run needs `LEVELS3`. | Verify inclusion after runtime boundary is chosen. |

## Do Not Touch Yet

Avoid these areas until the runtime boundary is explicit:

- `solver.js`
- GIF generation
- sound and animation data
- visual level editor
- tags and mappings expansion
- metadata twiddling
- rich camera, shadow, tween, or sprite editor behavior

## Immediate Implementation Gate

Before implementing 3D movement, add tests for these boundary behaviors:

1. A file with ordinary `LEVELS` still compiles through the 2D path.
2. A file with `LEVELS3` produces a compiled `state.levels3` with dimensions.
3. Attempting to play a `LEVELS3` game does not silently fall back to "No levels
   found" or a malformed 2D level.
4. The runtime emits one explicit unsupported diagnostic if 3D play is not
   implemented yet.

Passing those tests means the project has a real PS3D runtime boundary. Only
then should six-direction movement and rendering start.

## First Playable Milestone

The smallest playable milestone after the boundary gate is:

- load exactly one anonymous `LEVELS3` level
- no sections, links, title screen, level select, solver, sound, or GIF
- no advanced PS Next rules
- one 3D Sokoban rule pair
- undo and restart either work for `depth` or are explicitly disabled with a
  source-facing/runtime-facing diagnostic
- rendering may be debug-only, but must make `width`, `height`, and `depth`
  visible

This milestone should be treated as a 3D runtime path, not as an extension of
the current 2D `Level` by accidental optional fields.

