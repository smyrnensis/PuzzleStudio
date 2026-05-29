# Agent Notes

This project follows the root repository agent rules supplied for
`/Users/rintaromasaoka/Documents/Game/PuzzleBuilder/PS_EXTRACTION`.

When reading implementation files for PS Extraction work, add a short summary
here before moving on. The goal is to avoid re-reading the same core files every
time a design question comes up.

## Read File Notes

## Current 3D Principle

- 3D code must be 2D code with two extra spatial directions. 2D and 3D are
  built in the same form and at the same abstraction level.
- The only acceptable differences are spatial: `depth`, `front` / `back`, 3D
  coordinate/index/neighbor logic, spatial movement resolution, oriented rule
  frames, and rendering.
- Non-spatial PuzzleScript semantics such as `late`, commands, `again`,
  `restart`, `checkpoint`, `gosub`, loops, random choices, tags, mappings, win
  conditions, sound, metadata, undo, and session flow must not become 3D-specific
  design differences.
- Existing 3D modules are prototype/scaffolding code unless they exactly mirror
  the 2D semantic contract. Missing support is an implementation gap, not a
  language design difference.

### `upstream/PuzzleScriptNext/src/js/compiler.js`

- 3D level lowering is present through `levels3ToArray(state)` and
  `level3FromParsedSource(state, parsedLevel)`. `levels3` is an internal
  transport name only; author-facing syntax is `three_dimensions` plus ordinary
  `LEVELS`.
- Lowered 3D levels keep `state.levels3` separate from `state.levels`; do not
  route 3D levels through 2D level loading.
- `generateExtraMembers(state)` assigns object IDs in collision-layer order,
  annotates each concrete object with `object.layer` and `object.id`, and sets
  `state.objectCount`.
- 2D movement bitmap width is compiler-owned: `MOV_BITS = 8`,
  `MOV_MASK = 0xff`, `STRIDE_OBJ = ceil(objectCount / 32)`, and
  `STRIDE_MOV = ceil(layerCount * MOV_BITS / 32)`. The engine later receives
  these through `state.STRIDE_OBJ` and `state.STRIDE_MOV`.
- `generateMasks(state)` builds `state.layerMasks` as one object bitmask per
  collision layer, and `state.objectMasks` for objects, synonyms, and
  properties.
- Object/property/aggregate semantics are compiler-owned before runtime:
  properties are OR groups, aggregates are AND groups, synonyms are resolved
  during compiler setup.
- `concretizePropertyRule` expands `no property` into repeated `no object`
  entries before masks are built.
- `atomizeCellAggregates` expands aggregate references into multiple concrete
  objects, but `no aggregate` is rejected with an error.
- `rulesToMask(state)` lowers LHS cells into `CellPattern` data:
  `objectsPresent`, `objectsMissing`, `anyObjectsPresent`,
  `movementsPresent`, and `movementsMissing`.
- `rulesToMask(state)` lowers RHS cells into `CellReplacement` data:
  `objectsClear`, `objectsSet`, `movementsClear`, `movementsSet`,
  `postMovementsLayerMask`, random entity mask, and random direction mask.
- `three_dimensions` is the explicit prelude flag for 3D rule vocabulary.
  With this flag, rule parsing accepts `front` / `back` direction words and the
  compiler uses six-direction 3D direction aggregates for rule expansion.
  Without it, 2D rule vocabulary remains the default.
- `three_dimensions` should also be treated as the canonical 3D mode marker for
  levels and runtime routing. Do not make `LEVELS3` a public language surface.
- Parser routing now sends ordinary `LEVELS` through the 3D level transport when
  `three_dimensions` is enabled.
- Current `finalizeRulesFor3D(state)` is prototype code and is not the target
  architecture if it marks ordinary 2D semantics as unsupported. Rework it
  toward the same semantic artifacts as 2D with dimension-specific spatial hooks.
- Current 3D finalization does not yet implement counterparts for rigid
  behavior, commands/gosub/loop execution, random/randomdir, or ellipsis. Treat
  these as implementation gaps in the shared 2D/3D contract, not as valid 3D
  design differences.
- `late` is no longer a 3D unsupported feature. The current 3D path lowers late
  rules into `state.rules3d.lateGroups` and `turn3d` applies them after movement
  resolution, matching the 2D phase order.
- Rule `commands` are no longer a 3D unsupported feature. The current 3D path
  preserves `rule.commands` and `turn3d` returns a command queue artifact with
  message/status/gosub/log side data. Browser/session side effects are still a
  shared session integration gap, not a 3D semantic difference.
- RHS movement markers do not immediately move objects. They set movement masks
  that the engine resolves later.
- LHS movement terms lower into `movementsPresent` or `movementsMissing` per
  layer. `stationary` means "no movement bits in this layer".
- RHS movement terms lower into `movementsSet`; `stationary` lowers into
  `movementsClear`; `randomdir` lowers into `randomDirMask`.
- RHS concrete object placement clears the whole target collision layer before
  setting the object. If an LHS layer disappears from RHS, the replacement
  clears that layer and clears post-movement for that layer.
- `postMovementsLayerMask_r` is a movement-clear mask used by
  `CellPattern.replace` to wipe movement bits from layers whose object content
  was replaced/removed. This is separate from high-level movement resolution.
- `rulesToMask3D(state)` now lowers the post-`rulesToArray` PuzzleScript rule
  IR into `state.rules3d` before 2D `rulesToMask(state)` mutates
  `state.rules`.
- 3D rule lowering mirrors the 2D mask contract for
  `objectsPresent`/`objectsMissing`/`anyObjectsPresent`,
  `movementsPresent`/`movementsMissing`, and cell-local replacement
  clear/set masks.
- 3D lowering differs only at the spatial layer: pattern row offsets use 3D
  direction deltas and 3D movement bits use six logical directions.

### `upstream/PuzzleScriptNext/src/js/engine.js`

- Existing `engine.js` is the 2D core runtime and should not become the 3D
  core by gradual generalization.
- `setGameState` copies compiler-produced `STRIDE_OBJ` and `STRIDE_MOV` into
  engine globals, then `RebuildLevelArrays` allocates movement storage and row /
  column / map summary masks.
- `Level.getCell(index)` returns a copy-like `BitVec` over a cell slice;
  mutating it does not update board storage until `setCell` is called.
- `Level.getCellInto(index, target)` copies cell data into caller-owned storage.
- `Level.setCell(index, vec)` is the storage mutation boundary for cell objects.
- `Level.movements` is a separate `Int32Array` with `n_tiles * STRIDE_MOV`
  entries. Movement access mirrors cell access through `getMovements`,
  `getMovementsInto`, and `setMovements`.
- `setMovements` also updates row/column/map movement summary masks; rule
  matching uses these to skip impossible rows/columns.
- `CellPattern.matches` checks the lowered mask contract:
  all `objectsPresent` bits must be set, no `objectsMissing` bits may be set,
  each `anyObjectsPresent` mask must have at least one matching bit, all
  `movementsPresent` bits must be set, and no `movementsMissing` bits may be
  set.
- `CellPattern.replace` performs pattern replacement by applying
  `objectsClear`/`objectsSet` and movement-mask changes to the matched cell.
- `CellPattern.replace` does not move entities. It mutates the current cell's
  object mask and movement mask, handles `random`/`randomdir`, records rigid
  movement metadata, updates create/destroy SFX masks, and writes the final
  object/movement masks back through `setCell` and `setMovements`.
- Player input is converted into movement masks by `startMovement` /
  `moveEntitiesAtIndex`: the player objects stay in place initially, and their
  layers receive a direction bit in `Level.movements`.
- The runtime does not have a high-level "push" primitive. Sokoban-style push is
  an author/game-level interpretation of rules that set movement masks on
  adjacent objects.
- `resolveMovements` is the special move-resolution phase. It repeatedly calls
  `repositionEntitiesAtCell` / `repositionEntitiesOnLayer` to move entities
  whose movement masks were set by rules or input.
- `repositionEntitiesOnLayer` enforces collision layers: if the target cell has
  an object in the moving layer, that movement does not apply.
- `repositionEntitiesOnLayer` moves only the source objects belonging to the
  requested collision layer. It clears those objects from the source cell,
  ORs them into the target cell, and leaves other layers in the source cell.
- `repositionEntitiesAtCell` iterates each layer's movement field independently.
  Successful movement clears that layer's movement bits from the source cell.
- `resolveMovements` loops until no movement succeeds, then clears all remaining
  movement bits. Rigid failures can request a rollback and resimulation with a
  banned rule group.
- The main turn loop is: seed input movement masks, apply normal rules, resolve
  movement, optionally resimulate rigid failures, apply late rules, then process
  commands / win / undo side effects.

### `upstream/PuzzleScriptNext/src/js/levels3d.js`

- This file is a small 3D helper, not a runtime.
- It parses current 3D level transport, validates slice width/height
  consistency, and provides `coordToIndex3` / `indexToCoord3`.
- Its index order is `x * height * depth + y * depth + z`; 3D runtime code
  should stay consistent with that order.

### `upstream/PuzzleScriptNext/src/js/rule_frames3d.js`

- 3D rule-frame helper added locally.
- Standard author-facing frame is `> = right`, `^ = front`, `o = up`; inverse
  markers are `<`, `v`, and `x`.
- Generates 24 proper oriented frames by rotating the standard frame.
- Mirrored/reflected frames are intentionally excluded; this is not a 48-frame
  expansion.

### `upstream/PuzzleScriptNext/src/js/cell_match3d.js`

- 3D cell matcher added locally.
- It intentionally does not import 2D `engine.js`.
- Tests pin it to 2D `CellPattern` semantics for `objectsPresent`,
  `objectsMissing`, `anyObjectsPresent`, `movementsPresent`, and
  `movementsMissing`.

### `upstream/PuzzleScriptNext/src/js/runtime3d.js`

- 3D runtime board helper added locally.
- It receives `slots3d` output and owns 3D board access helpers:
  `coordToIndex`, `indexToCoord`, `neighbor`, `getCell`, `getCellInto`,
  `setCell`, `getMovements`, `getMovementsInto`, `setMovements`, and `clone`.
- Cell access follows the 2D `Level` contract: reads copy, `setCell` mutates.
- Movement access follows the same 2D `Level` contract: reads copy,
  `setMovements` mutates.
- The board now carries `strideMov`, `movementBits`, `movementMask`,
  `directionBits`, `layerMasks`, and `objectLayers` so movement seeding and
  resolution work per collision layer.
- `moveEntitiesAtIndex` / `startMovement` seed layer movement masks without
  directly moving objects, matching the 2D input contract.
- `resolveMovements` applies movement masks through 3D neighbors, moves only
  objects on the requested collision layer, blocks on same-layer occupancy, and
  clears remaining movement masks after resolution.

### `upstream/PuzzleScriptNext/src/js/game_runtime3d.js`

- 3D runtime facade added locally.
- Upper layers should prefer `createRuntimeFromState3D(state)` and
  `processTurn3D(runtime, inputDirection)` instead of manually composing
  `buildSlots3D`, `createRuntime3D`, and `turn3d.processTurn`.
- This facade does not connect browser playback routing, rendering, or session
  flow yet; it only owns the compiler-state-to-runtime and one-turn entry points.
  `compile()` still gates 3D browser playback before `setGameState`; do not describe that
  missing integration as a 3D runtime design difference.

### `upstream/PuzzleScriptNext/src/js/turn3d.js`

- 3D turn runner added locally.
- It follows the 2D turn order for the currently supported subset: seed input
  movement masks for `playerMask`, apply normal rule groups, resolve movement,
  then apply late rule groups.
- Input does not directly move player objects. It calls board `startMovement`,
  and board `resolveMovements` performs layer-based movement afterward.
- Rule groups repeat until no rule changes the board, capped by an iteration
  limit to catch runaway propagation.
- Current compiler-marked unsupported 3D features are prototype gaps. Do not
  preserve them as design differences unless the gap is spatial.

### `upstream/PuzzleScriptNext/src/js/slots3d.js`

- 3D slots wrap compiler-produced `state.levels3[0]` instead of routing it
  through 2D `Level` loading.
- If explicit rules are not passed in options, slots use compiler-produced
  `state.rules3d`.
- `core.board` currently carries occupancy storage, dimensions, stride, layer
  count, movement bitmap constants/storage, layer masks, object-layer mapping,
  player mask, background ids, and source level reference.
- `core.directions` owns six absolute directions plus relative marker metadata.
- Input defaults map `w/a/s/d` to `front/left/back/right`; `up/down` remain
  unbound intents by default.
- Movement support is represented as a PS-compatible mask/layer contract in this
  slot, including default six-direction movement bits and 3D deltas.

### `upstream/PuzzleScriptNext/src/js/rules3d.js`

- 3D rule helper added locally.
- It combines PS-compatible cell matching with absolute offsets and 24-frame
  relative marker expansion.
- `makeCellPattern` carries object and movement match masks. `makeCellReplacement`
  carries object clear/set and movement clear/set masks, and
  `applyCellReplacement` applies those masks cell-locally without directly
  moving entities.
- Unsupported rule features are rejected before execution through an explicit
  feature gate.
