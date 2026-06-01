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
- 3D movement bit layout preserves 2D first: `up=1`, `down=2`, `left=4`,
  `right=8`, and `action=16`. Extra 3D spatial directions are appended after
  that (`front=32`, `back=64`). Do not remap `action` into a 3D spatial
  direction or drop 2D action fade semantics.
- Non-spatial PuzzleScript semantics such as `late`, commands, `again`,
  `restart`, `checkpoint`, `gosub`, loops, random choices, tags, mappings, win
  conditions, sound, metadata, undo, and session flow must not become 3D-specific
  design differences.
- Existing 3D modules are prototype/scaffolding code unless they exactly mirror
  the 2D semantic contract. Missing support is an implementation gap, not a
  language design difference.
- 3D modules should be thin adapters over shared PuzzleScript semantics. A 3D
  file may own spatial hooks such as coordinates, neighbors, direction tables,
  movement resolution over 3D space, rule frames, renderer/camera, and picking.
  It must not own non-spatial semantics such as command priority, session tail
  planning, rule-group control, random choice, `late`, `global`, win conditions,
  metadata twiddling, `again`, `require_player_movement`, or
  `run_rules_on_level_start`.
- If non-spatial PuzzleScript behavior appears in a 3D file, it must be either
  a direct call to a shared 2D-preserving helper or a hook/data adapter passed
  into that helper. Otherwise treat it as prototype debt to extract, even when
  black-box tests pass.

## Design Interrupt Rule

When working under a user-stated governing principle, do not internally convert
a possible principle violation into a local workaround, implementation note, or
renderer/runtime exception.

If an observation suggests that the current work may violate a governing
principle, stop before choosing the next implementation step and surface it as a
design interrupt. This applies especially when words such as "exception",
"collision", "unsupported", "for now", "fallback", "not yet", or "3D-specific"
would explain away a difference.

For the 2D/3D work, a design interrupt is required whenever a local observation
suggests that 3D might be replacing or reinterpreting a 2D semantic carrier
instead of extending it with spatial hooks. Examples include movement bits,
input direction indexes, lowered rule masks, sfx direction masks, command/session
artifacts, metadata side effects, win/session flow, and browser loop state.

A design interrupt should state:

- which governing principle may be violated;
- the local observation that triggered the doubt;
- whether the doubt is about a spatial extension, a missing implementation, or a
  possible non-spatial semantic drift;
- what evidence would decide it, such as a 2D oracle, raw carrier comparison, or
  shared-helper boundary;
- the smallest safe next action, usually a parity audit or test, not a local
  workaround.

Do not proceed to renderer/browser polish, fallback behavior, or feature
completion while a design interrupt touching 2D/3D semantic parity is unresolved.

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
- Movement bitmap width is compiler-owned. 2D uses `MOV_BITS = 5` and
  `MOV_MASK = 0x1f`; 3D preserves that 2D prefix and appends only `front/back`,
  so it uses `MOV_BITS = 7` and `MOV_MASK = 0x7f`. `STRIDE_OBJ =
  ceil(objectCount / 32)` and `STRIDE_MOV = ceil(layerCount * MOV_BITS / 32)`.
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
  levels and runtime routing. Do not add a separate public 3D level section.
- 3D camera syntax is ordinary prelude metadata scoped by `three_dimensions`:
  `orthographic_camera`, `perspective_camera`, `camera_angle <yaw> <pitch>`,
  `camera_zoom <n>`, `camera_distance <cells>`, and
  `camera_view_angle <degrees>`. Do not reintroduce `three_camera` or a
  direction-word camera preset.
- `camera_distance` is measured in level cells. `camera_angle` and
  `camera_view_angle` are measured in degrees.
- Do not expose renderer-internal words such as `projection`, `fov`, or
  low-level scale/preset names as source syntax unless a later design explicitly
  promotes them into PS-style prelude metadata.
- Parser routing now sends ordinary `LEVELS` through the 3D level transport when
  `three_dimensions` is enabled.
- `finalizeRulesFor3D(state)` must not mark ordinary 2D semantics as a 3D
  unsupported subset. It projects shared rule finalization into `state.rules3d`
  with dimension-specific spatial hooks only.
- 3D rule lowering no longer carries `unsupportedFeatures` or
  `unsupportedCounterparts`. If a non-spatial rule behavior is missing, track it
  as an implementation gap with parity tests, not as data in the 3D rule
  contract.
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
- `compiler.js` must not own browser 3D session, renderer, or input bridge
  behavior. It only detects a loaded `Puzzle3DPlayHost` capability and delegates
  3D `startCompiledState` to that host. Browser playback details belong to the
  play/input/render adapter layer.
- `compile()` routes through `compileToState -> startCompiledStateAfterHostPreparation`.
  For 2D states this remains synchronous. For states with declared host
  capabilities, compiler delegates preparation to the loaded play host before
  start; it must not inline Three.js or WebGL details.

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
- Author `LEVELS` text lowers each slice as an x/z plane: columns are `x`, rows
  are `z` / front-back, and standalone `;` separators stack slices along `y` /
  up-down. This keeps ordinary 2D row structure planar and reserves vertical
  movement for `o` / up-down.

### `upstream/PuzzleScriptNext/src/js/parser.js`

- In `three_dimensions`, `OBJECTS` sprite authoring stays parallel to 2D:
  ordinary sprite rows keep the same per-character palette / `.` transparent
  meaning, and a standalone `;` line advances to the next 3D sprite slice.
- The parser stores this as `object.sprite3matrix[row][col][slice]`, while also
  preserving the first slice in `object.spritematrix` for existing 2D-owned
  sprite code paths.

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
- `test/resolve_movements_2d_parity.test.js` pins depth-1 3D
  `resolveMovements` against the 2D VM oracle for layer-local movement,
  blocked movement cleanup, repeated scan behavior, movement/cantmove SFX
  timing, and rigid failure group reporting.

### `upstream/PuzzleScriptNext/src/js/game_runtime3d.js`

- 3D runtime facade added locally.
- Upper layers should prefer `createRuntimeFromState3D(state)` and
  `processTurn3D(runtime, inputDirection)` instead of manually composing
  `buildSlots3D`, `createRuntime3D`, and `turn3d.processTurn`.
- `applySessionArtifacts3D` uses the shared 2D session-tail planner. Its
  checkpoint, restart, numeric `goto`, win level-advance, undo, cancel, link,
  and quit effects are pinned by `test/game_runtime3d_2d_parity.test.js`
  against a 2D VM oracle.
- `test/full_turn_2d_parity.test.js` pins a depth-1 3D session turn against a
  2D `processInput` VM oracle for input movement, rule-seeded movement,
  movement resolution, late rules, checkpoint session tail, and wincondition
  tail behavior. The test uses equivalent lowered mask artifacts and preserves
  the raw 2D movement direction prefix while allowing only appended 3D
  `front/back` spatial bits.
- 3D session state carries `backups` and `linkStack` to mirror the 2D session
  side-effect model. `undo` restores and pops the latest backup; `cancel`
  restores the current turn-start source; `restart` pushes the current turn
  source before restoring `restartSource`; `link` pushes a link backup and
  follows the most recent visible matching link object. A `win` while inside a
  link returns to the link source and runs one no-input `dontDoWin` turn, matching
  2D `DoWin` / `returnLink`.
- Numeric `goto` targets follow 2D `gotoLevel` semantics: non-negative values
  are section indexes resolved through `state.sections[index].firstLevel`, and
  negative values encode direct level indexes as `-1 - target`.
- `WINCONDITIONS` are evaluated by `turn3d.processTurn` through the shared
  `WinConditions` helper, matching the 2D `processInput` / `checkWin`
  abstraction level. `game_runtime3d` only consumes the produced win session
  artifact. 3D differs only by board cell access.
- `again` loop control uses the shared `AgainLoop` helper. 3D supplies only
  hooks for turn execution, tail artifact application, and the 2D-style
  no-input dry-run probe that decides whether an actual again turn should run.
  The dry-run probe result interpretation lives in `AgainLoop`; 3D only provides
  a cloned no-input turn, board-change predicate, and session-tail planner hook.
- Browser playback may request deferred `again` handling through the 3D session
  facade. In that mode, one turn is processed, the shared dry-run probe decides
  whether `again` can change state, and `play_host3d` reflects the result into
  the existing 2D browser loop state (`againing` / `timer`). Do not make browser
  `again` timing a separate 3D runtime loop.
- Browser playback may also request deferred `win` handling. In that mode, a
  win request must not immediately advance the 3D session level; it marks the
  session as won and lets `play_host3d` reflect the result into the existing 2D
  browser loop state (`winning` / `timer`). The delayed advance remains owned by
  `inputoutput.update()`, just like 2D `DoWin()`.
- `run_rules_on_level_start` is handled as a lifecycle turn at the same
  abstraction level as 2D level creation/restart/goto/win advance. The session
  creates/restores a pre-start `restartSource`, then runs a no-input turn with
  `dontDoWin`, rather than modeling level start as a fake player input. Command
  tail planning receives `dontDoWin`, so a level-start `win` command is suppressed
  like 2D `checkWin(dontDoWin)`.
- This facade owns compiler-state-to-runtime and one-turn/session entry points.
  Browser playback routing, canvas ownership, keyboard focus, and renderer
  invocation are adapter concerns owned by `play_host3d.js` / `inputoutput.js`,
  not by `game_runtime3d.js`.

### `upstream/PuzzleScriptNext/src/js/play_host3d.js`

- 3D browser playback host added locally.
- This file is an adapter layer, not a semantic runtime. It receives a compiled
  3D state, creates a `game_runtime3d` session, asks `render_frame3d` for render
  frames, invokes `three_renderer3d`, and installs the browser `processInput`
  bridge.
- `canStart()` requires the runtime, render-frame builder, Three renderer, and
  `THREE`. Three.js is a declared host capability, not something detected by
  source-string heuristics or replaced by a 2D canvas fallback.
- `prepareCompiledState()` / `prepareCapabilities()` own browser capability
  preparation for compiled 3D states, including Three.js module loading and
  WebGL context checks. Page shells and editor/standalone compile callers should
  reach this through the shared compiler start path instead of carrying renderer
  dependency logic inline.
- The bridge preserves the 2D `processInput(inputDirection, dontDoWin,
  dontModify, bak, coord)` entry shape. It forwards only dimension hooks:
  direction-index normalization and 2D browser `up` / `down` to 3D
  `front` / `back`.
- The bridge asks `game_runtime3d` for deferred `again` handling and maps the
  result to the existing browser `againing` / `timer` variables. `inputoutput.js`
  remains the owner of the delayed `processInput(-1)` tick and `autotick`
  scheduling.
- The bridge asks `game_runtime3d` for deferred `win` handling and maps the
  result to the existing browser `winning` / `timer` variables. `inputoutput.js`
  remains the owner of the delayed level-advance timing; `play_host3d` only
  handles the resulting `winAdvance` screen command for 3D sessions.
- Browser command side effects are centralized in small host helpers. Status
  text, simple sound commands, command messages, deferred quit, and terminal
  command sounds must not be reimplemented at individual call sites. Add to the
  shared host helpers first, then call them from keyboard/session/rule-command
  paths.
- Browser title and level-select screens remain 2D-owned UI shells. The 3D host
  installs a `handleScreenCommand` adapter so those shells can start a 3D
  session or jump to a 3D section without calling 2D `nextLevel` / `gotoLevel`.
  The 3D host may remove/hide the 3D canvas while these text shells are active,
  but it must not fork their key policy.
- Browser level-flow semantics go through `BrowserLevelFlow`, which is now used
  by 2D `engine.js` itself for `nextLevel`, `gotoLevel`,
  `loadLevelFromLevelDat`, level target loading, local storage, section solving,
  and clear-storage behavior. 3D must use the same helper and supply only 3D
  hooks for `levels3`, session creation/resume, render canvas removal, and
  level target resolution.
- `test/browser_level_flow.test.js` includes a 2D `engine.js` VM oracle for
  playable next-level advance, section-index `gotoLevel`, and level-message
  close. Extend that oracle before changing level-flow semantics.
- Browser pause remains the same 2D-owned shell. `engine.getPauseScreen` may
  read a current title from either `state.levels` or `state.levels3`, while
  pause selection is committed through the same `handleScreenCommand` adapter
  so resume / restart / level-select / title affect the 3D session instead of
  calling 2D runtime side effects directly.
- Browser message screens remain the same 2D-owned shell. Command messages
  should be shown with the existing 2D message UI and close back into the
  current 3D session. Message levels should display through that shell and then
  continue to the next 3D playable level. The 3D host owns only the session
  continuation hook; it must not fork message key policy or text layout.
- `gameCanvas3D` focus registration lives here through `Puzzle3DInputAdapter`;
  `inputoutput.js` remains the owner of the keyboard gate. The adapter may
  handle session commands such as keyboard undo/restart by delegating to
  `game_runtime3d` session artifacts; it must not define separate key policy.
- Do not move browser session/render/input bridge code back into `compiler.js`
  or `game_runtime3d.js`.

### `upstream/PuzzleScriptNext/src/js/inputoutput.js`

- 2D remains the owner of browser keyboard routing. 3D may only add an adapter
  that tells the existing gate which 3D canvas is a game input target.
- `Puzzle3DInputAdapter.isKeyboardFocusTarget(target)` is a focus hook, not a
  separate 3D input policy.
- `Puzzle3DInputAdapter.handleSessionCommand(command, context)` is a side-effect
  hook for commands whose 2D keyboard handling bypasses `processInput`, such as
  `undo` and `restart`. The key mapping, repeat suppression, `pushInput`, and
  `prevent` behavior remain owned by `inputoutput.js`.
- `Puzzle3DInputAdapter.handleScreenCommand(command, context)` is the screen-flow
  hook used when existing 2D title / level-select UI commits to starting or
  jumping into gameplay, or when existing 2D pause/message shells commit their
  close/selection side effects. It is not a separate 3D menu/message policy; the
  2D shell still owns selection, text layout, and keyboard behavior.

### Browser Smoke Testing

- In this Codex desktop environment, Browser automation currently cannot open
  the local PuzzleScriptNext play URLs used for this project:
  `http://127.0.0.1:8765/...`, `http://localhost:8765/...`, or equivalent
  `file://.../play.html` URLs. Attempts have repeatedly failed with
  `ERR_BLOCKED_BY_CLIENT` or Browser Use URL-policy rejection.
- Do not spend tokens retrying those same Browser automation navigations unless
  the user explicitly says the Browser automation policy/configuration has been
  changed. Prefer Node/unit/e2e tests plus user-visible manual smoke through the
  already-open in-app browser.
- If browser smoke is needed, provide the exact URL and observable checks for
  the user to run manually, or add a small repo fixture/demo that makes manual
  smoke unambiguous.

### `upstream/PuzzleScriptNext/src/js/render_frame3d.js`

- Render frames are presentation data snapshots built from a runtime/session and
  compiled state. They must not own browser DOM, canvas, WebGL, input routing,
  or session side effects.
- The render-frame contract is renderer-agnostic: board size, compiled object
  visuals, draw order, cells, session snapshot, and turn effects.
- The public frame schema is owned by
  `upstream/PuzzleScriptNext/src/js/render_frame_contract3d.js`. The frame
  builder validates generated frames against it, and renderers must validate the
  same schema before drawing.
- Render frames must not carry raw PuzzleScript source, compiler state, runtime
  board objects, or live session objects. Those are upstream owners; the
  renderer boundary receives only normalized presentation fields.
- 2D sprite rendering remains the oracle and must not be changed to accommodate
  3D. `render_frame3d.js` may project existing `spritematrix` / `colors` /
  `spriteoffset` data into 3D presentation voxels, but the 2D `graphics.js`
  sprite contract is read-only for this work.
- `frame.view` is internal renderer presentation state. It may contain
  `projection`, `yaw`, `pitch`, `cameraZoom`, `cameraDistance`,
  `cameraViewAngle`, `shade`, `visibility`, and `slice`, but PS source reaches
  it only through PS-style camera prelude metadata. Renderers must read
  `frame.view`, not raw metadata.

### `upstream/PuzzleScriptNext/src/js/three_renderer3d.js`

- This file is the Three/WebGL adapter for 3D render frames. It consumes only a
  render frame plus canvas/options and must not inspect PuzzleScript source,
  compiler state, runtime session objects, or browser input state directly.
- Missing `THREE` is a host capability failure. Do not reintroduce a 2D canvas
  fallback renderer here.
- `buildInstances`, `render`, and `renderToCanvas` are renderer consumers, not
  runtime adapters. They should fail on malformed frames instead of silently
  synthesizing fallback `cells`, `drawPlan`, or object data from missing fields.

### `upstream/PuzzleScriptNext/src/js/turn3d.js`

- 3D turn runner added locally as a thin adapter over `src/js/turn_runtime.js`.
- `turn_runtime.js` owns the shared turn order: seed input movement masks for
  `playerMask`, apply normal rule groups, resolve movement with rigid retry /
  rollback, apply late rule groups, validate `require_player_movement`, collect
  SFX, collect command/session artifacts, evaluate win conditions, and report
  board changes. `turn3d.js` supplies only 3D board/rule hooks.
- Input does not directly move player objects. It calls board `startMovement`,
  and board `resolveMovements` performs layer-based movement afterward.
- Rule groups repeat until no rule changes the board, capped by an iteration
  limit to catch runaway propagation.
- `global` is not a separate 3D rule kind. It preserves the 2D
  `local_radius` contract: ordinary rules scan inside the first-player local
  box, while global rules scan the full board. Directional scan order is passed
  into the 3D matcher so x-axis rules follow 2D horizontal match order.
- `...` ellipsis is supported by preserving the 2D wildcard tuple contract:
  matches carry origin plus one or two gap lengths, and later tuple rechecks use
  the concrete matched cells. 3D only extends the scan axis to x/y/z.
- Rule group sequence control is shared: 2D `applyRules` and 3D `turn3d` both
  use `RuleGroups.applyRuleSequence` for `startloop` / `endloop`, subroutine
  boundaries, gosub jumps, and returns.
- Rule application control is shared through `RuleApplication.buildRuleApplicationHooks`.
  `turn3d` supplies only 3D match/replacement hooks and command metadata hooks;
  tuple generation, random group application, later-tuple rechecks, command
  queue timing, and rule/group looping belong to shared helpers.
- Runtime metadata twiddling is not a 3D-specific unsupported feature. 3D
  command enqueue uses the shared `RuntimeMetadataTwiddling` helper, matching
  2D `handleQueuedCommand2D` for `set` / `default` / `wipe` behavior and then
  refreshing metadata-derived 3D slots.
- `require_player_movement` is a turn validation step, not a session facade
  rule. 3D records player positions at turn start, runs the normal turn phases,
  and if an input turn leaves player objects in all start cells it restores the
  turn-start board and drops command artifacts, matching 2D `processInput`.
- Compiler-marked unsupported 3D rule features are not part of the contract.
  Do not reintroduce `unsupportedFeatures`, `unsupportedCounterparts`, or a
  3D-only runtime rejection gate for non-spatial PuzzleScript behavior.

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
- `local_radius` is passed as a supported lifecycle/matching parameter; it is
  consumed by rule scan hooks rather than treated as a 3D unsupported feature.
- `runRulesOnLevelStart` and `requirePlayerMovement` lifecycle slots are
  represented with 2D owner names:
  `engine.loadLevel.run_rules_on_level_start` and
  `engine.processInput.require_player_movement`. They differ from 2D only
  through board dimension hooks used by the turn/session runtime.
- Slot capability metadata uses `semantic.owner` and `semantic.implemented`
  fields that name the corresponding 2D owner layer (`engine.processInput`,
  `engine.titleFlow`, `inputoutput`, `graphics`). Do not reintroduce a flat
  `supported:false` flag for non-spatial PuzzleScript semantics.
- Runtime metadata twiddling is represented as
  `engine.runtime_metadata_twiddling`. The slot keeps references to
  current/default metadata so command-time twiddles update the same semantic
  state rather than a separate 3D copy.
- Movement support is represented as a PS-compatible mask/layer contract in this
  slot, including default six-direction movement bits and 3D deltas.

### `upstream/PuzzleScriptNext/src/js/rules3d.js`

- 3D rule helper added locally.
- It combines PS-compatible cell matching with absolute offsets and 24-frame
  relative marker expansion.
- It accepts rule-scan hooks for `local_radius`, `global`, player positions,
  and direction-axis order; these are 2D rule semantics with 3D coordinate
  bounds, not 3D-specific language behavior.
- `makeCellPattern` carries object and movement match masks. `makeCellReplacement`
  carries object clear/set and movement clear/set masks, and
  `applyCellReplacement` applies those masks cell-locally without directly
  moving entities.
- No non-spatial rule feature should be rejected by this file merely because
  execution is 3D. Unsupported diagnostics are for real remaining gaps, not for
  ordinary PuzzleScript semantics.
