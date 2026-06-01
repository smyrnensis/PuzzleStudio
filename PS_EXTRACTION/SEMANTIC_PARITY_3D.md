# 2D / 3D Semantic Parity

This document is the reset point for 3D implementation.

## Principle

3D code is 2D code with two extra spatial directions.

2D and 3D must be built in the same form and at the same abstraction level.
The only acceptable differences are spatial:

- `depth`
- `front` and `back`
- 3D coordinate/index/neighbor logic
- spatial movement resolution over six directions
- oriented spatial rule frames
- renderer/camera

Everything else is PuzzleScript semantics and must match 2D exactly.

3D differences should appear as dimension hooks, not feature forks. If 3D code
needs a different neighbor lookup, direction table, rule-frame interpretation,
movement-resolution space, or renderer projection, that difference belongs in a
dimension hook. If 3D code needs a different implementation of commands, `late`,
random choice, win conditions, undo, checkpoint, loops, or gosub, the shared
PuzzleScript contract is missing or has been bypassed.

## Not Design Differences

These features must not be treated as 3D-specific semantics:

- `late`
- commands: `win`, `again`, `restart`, `message`, `goto`, `checkpoint`, `undo`,
  `cancel`, `nosave`, `quit`, `status`, `log`
- `gosub`
- `startloop` / `endloop`
- `random` rule groups
- `random` object replacement
- `randomdir`
- `...` ellipsis, with only the scan axis extended to 3D
- tags and mappings
- properties, aggregates, and synonyms
- collision layer semantics
- win conditions
- sound and sfx event semantics
- metadata and runtime metadata twiddling
- undo, restart, checkpoint, level advance, level select, and links

If one of these is missing in 3D, the correct label is implementation gap.

## Allowed Spatial Hooks

Shared PuzzleScript semantics may call dimension hooks for:

- direction vocabulary and direction aggregates
- relative marker interpretation
- coordinate-to-index and index-to-coordinate
- neighbor lookup
- pattern row offsets
- no remaining 3D-only rule finalization gaps
- movement resolution
- rigid movement rollback over dimension-specific movement
- renderer projection

The hook may differ. The feature semantics may not.

## Thin Adapter Contract

3D modules must be thin adapters over shared PuzzleScript semantics. A 3D file
passes this contract only when every non-spatial PuzzleScript behavior in that
file is one of the following:

1. A direct call to a shared helper that preserves the 2D contract.
2. A hook, adapter, or data projection passed into a shared helper.

If a 3D module implements non-spatial semantics directly, that code is
prototype debt even if black-box tests currently pass. The fix is to extract or
reuse a shared 2D-preserving helper, then make the 3D code call it through
dimension hooks.

3D code may own these responsibilities:

- 3D board shape: `width`, `height`, and `depth`
- coordinate-to-index and index-to-coordinate conversion
- neighbor lookup
- 3D direction tables and append-only direction-bit extensions
- `front` / `back` vocabulary and six-direction aggregates
- relative marker interpretation and 3D rule-frame orientation
- pattern offsets over 3D space
- 3D movement-resolution hooks over 3D neighbors
- renderer, camera, viewport projection, and picking
- board/session adapters needed by shared helpers

3D code must not own these responsibilities:

- command priority, dedupe, or command queue mutation policy
- meanings of `win`, `again`, `checkpoint`, `restart`, `undo`, `cancel`,
  `goto`, `link`, `quit`, `message`, `status`, `log`, or `nosave`
- session-tail planning
- rule-group repetition, `once`, loops, gosub, and subroutine flow
- random rule candidate enumeration or random replacement semantics
- `late` phase semantics
- `global` / `local_radius` semantics
- `require_player_movement` semantics
- `run_rules_on_level_start` semantics
- win-condition semantics
- metadata and runtime metadata twiddling semantics
- sound and sfx event semantics
- unsupported-feature subset definition
- browser lifecycle semantics, except as browser adapter hooks into shared
  lifecycle behavior

The same rule applies to compiler, runtime, and tests. 3D-specific tests may
exercise spatial hooks, but they must not define a reduced non-spatial 3D
language. When a test needs a non-spatial PuzzleScript behavior, it should
prefer a shared-helper test or a 2D oracle parity test.

## Parity Verification

Parity must be proved against the 2D behavior, not asserted from parallel 3D
tests alone.

For non-spatial semantics, the 2D compiler/runtime is the oracle until a shared
contract has been extracted. Tests should compare the contract shape and
observable artifacts that both dimensions produce:

- lowered rule groups and late rule groups
- command queues and command priority/dedupe behavior
- win-condition query artifacts
- random object and random direction decisions, with direction count as the
  dimension hook
- loop/gosub control artifacts
- restart, checkpoint, undo, message, status, and level-flow requests
- sound and sfx event artifacts
- metadata/session requests after normalization

3D-only tests may cover spatial behavior, but they must not freeze a reduced
3D subset for non-spatial PuzzleScript semantics. A test that expects a
non-spatial feature to be unsupported only in 3D is evidence of an
implementation gap, not a design decision.

## Semantic Carrier Audit

Some implementation details are not incidental: they carry PuzzleScript
semantics across compiler, runtime, session, and renderer boundaries. These are
semantic carriers. 3D must not redefine them locally and then translate back to
2D meanings later, because that hides non-spatial semantic drift behind a
spatial adapter.

Examples of semantic carriers:

- input direction indexes, including `action`
- movement direction bits and movement field width
- raw movement masks stored per collision layer
- rule LHS/RHS movement mask artifacts
- randomdir candidate masks
- movement and cantmove sfx direction masks
- `movedEntities` direction values used by tween rendering
- command queues and session-tail artifacts
- metadata keys and runtime metadata twiddling artifacts
- level, section, and `goto` target encodings

The audit rule is stricter than ordinary black-box parity:

1. Identify the 2D carrier and record its raw values.
2. Require 3D to preserve the 2D carrier as a prefix or exact artifact.
3. Allow only spatial extension fields after the preserved 2D carrier.
4. Compare raw carrier values in tests before normalizing labels.
5. Treat a 3D-local carrier definition as suspicious until it is proved to be a
   spatial extension or a thin projection of the 2D carrier.

For movement bits, the canonical contract is:

| Meaning | 2D raw bit | 3D raw bit |
| --- | ---: | ---: |
| `up` | `1` | `1` |
| `down` | `2` | `2` |
| `left` | `4` | `4` |
| `right` | `8` | `8` |
| `action` | `16` | `16` |
| `front` | n/a | `32` |
| `back` | n/a | `64` |

Therefore 3D movement fields use `MOV_BITS = 7` and `MOV_MASK = 0x7f` unless a
future 2D carrier change first updates the shared contract. `front` and `back`
are spatial append-only bits; they must not occupy any 2D prefix bit.

Depth-1 oracle tests that compare only final board state are insufficient for
this class of bug. Carrier parity tests must also compare raw movement masks,
raw sfx direction masks, randomdir masks, rule movement artifacts, and
`movedEntities` values.

## Rebuild Order

1. Audit the current 3D prototype against the 2D compiler and engine.
2. For every 3D gap, classify it as spatial or non-spatial before changing the
   contract.
3. Do not encode non-spatial gaps as 3D unsupported rule data. `late`,
   commands, loops/gosub, random groups, random/randomdir replacement, ellipsis,
   rigid bookkeeping, metadata twiddling, and browser/session command tails are
   either implemented through shared helpers or tracked as parity work, not as a
   reduced 3D language subset.
4. Define shared artifacts for both dimensions:
   lowered rules, late rule groups, command queue, win result, sound events,
   random decisions, loop/gosub control, restart/checkpoint requests, and
   metadata/session requests.
5. Re-implement 3D by adding spatial hooks to that shared contract.

## First Parity Tests

The first tests should prove structure, not rendering:

- The same simple 2D game and its `three_dimensions` equivalent pass through
  the same compile stages.
- `late` rules are accepted in 3D and run after movement resolution.
- Command-producing rules produce the same command artifact in 2D and 3D.
- `random` rule groups are accepted in 3D and use the shared 2D-preserving
  candidate enumeration / random-choice helper.
- Win condition lowering produces the same query shape in 2D and 3D.
- Unsupported diagnostics are reserved for actual missing implementation, not
  for non-spatial semantic differences.

## 3D Local Semantic Implementations To Remove

These are local 3D implementations of semantics that belong to shared
PuzzleScript behavior. They should not be extended. Remove them by first
extracting or defining a shared 2D-preserving helper, then routing both 2D and
3D through that helper.

| 3D local code | Why it must go | Replacement | Removal condition |
| --- | --- | --- | --- |
| `turn3d.queueCommands` and command/session artifact assembly | Command queuing and command-to-session artifact classification are not spatial. 2D already defines priority, dedupe, `message`, `goto`, `status`, `gosub`, `log`, `win`, `again`, `restart`, and `checkpoint` behavior. | `src/js/command_queue.js` plus `src/js/runtime_metadata_twiddling.js` shared helpers. | Done for queue and session artifact classification: both 2D `Rule.prototype.queueCommands` / `processOutputCommands` and 3D `turn3d` use the shared command helper, with VM-oracle parity tests pinning 2D-visible outputs. Runtime metadata twiddling is now a shared queued-command helper pinned against the 2D `handleQueuedCommand2D` oracle; 3D invokes it from the same enqueue point and refreshes metadata-derived slots. |
| `turn3d.applyRuleGroup` propagation loop | Rule group repetition, `once`, random groups, `global`, ellipsis, loop/gosub flow, tuple rechecks, and command timing are PuzzleScript semantics, not 3D semantics. | Shared rule-group / rule-application helpers with dimension-specific match/apply hooks. | Done: ordinary rule-group propagation uses `src/js/rule_groups.js` from both 2D and 3D, including 2D-compatible `loopPropagated` return behavior and `once` handling. Rule application hook wiring lives in `RuleApplication.buildRuleApplicationHooks`; `turn3d` supplies only 3D pattern matching, match validity, cell replacement, and metadata-command hooks. `turn3d.applyRule` follows the 2D `Rule.tryApply/applyAt` shape through shared code: collect all pattern-row matches, generate tuples, apply tuple replacements with later-tuple recheck, then queue commands once for the whole rule. `global` has the 2D `local_radius` meaning: ordinary rules scan within the player-local box, while global rules bypass that limit. Directional scan order is passed through the match hook so x-axis rules match 2D horizontal order. `...` ellipsis preserves the 2D wildcard tuple contract for one and two ellipses, extending only the scan axis to x/y/z. `startloop` / `endloop` and `gosub` use shared `RuleGroups.applyRuleSequence` from both 2D `applyRules` and 3D `turn3d`, with 3D compiler lowering loop points and gosub targets into `state.rules3d`. `random` rule group candidate enumeration and selection uses `src/js/random_rule_groups.js`. These are pinned by 2D VM-oracle parity tests plus 3D turn/E2E tests. 3D session `again` uses explicit object-mask `boardChanged`, not the rule-group return value. |
| `turn3d.processTurn` as a full turn runner | Turn phase order and tail processing are shared. The current function is a reduced 2D `processInput`. | Shared turn contract: input seed, normal rules, movement hook, late rules, command/win/again/session artifacts. | Done: `src/js/turn_runtime.js` owns the shared turn pipeline and `turn3d.processTurn` is a thin hook adapter. The shared pipeline runs input seeding, normal groups, rigid movement retry/rollback, late groups, `require_player_movement`, SFX collection, command/session artifact collection, win-condition evaluation, and board-change reporting. `test/turn_runtime.test.js` pins the shared phase order and rollback/bypass semantics. `test/full_turn_2d_parity.test.js` pins depth-1 3D session turns against a 2D `processInput` VM oracle for input movement, rule-seeded movement, movement resolution, late rules, checkpoint session tail, wincondition tail behavior, `require_player_movement` rollback, and `run_rules_on_level_start` lifecycle turns, with only movement bit layout translated as a spatial hook. Browser/session integration remains outside the board runtime by design. |
| `game_runtime3d.applySessionArtifacts3D` session tail | Command priority and session side effects are PuzzleScript semantics, not 3D semantics. | Shared session-tail planner plus dimension/session-specific effect hooks. | Done: 3D uses `CommandQueue.planSessionTail` for 2D command priority (`undo`, `goto`, `link`, `cancel`, `restart`, `quit`, then `win`/`checkpoint`/`again`) and uses `boardChanged` for `again` eligibility. The planner is pinned against a 2D VM tail oracle in `command_queue_2d_parity.test.js`, including `dontDoWin` suppression of a queued `win`. `again` loop control and dry-run result interpretation are factored through `src/js/again_loop.js`; 3D supplies thin hooks for turn execution, tail artifact application, cloned no-input probe execution, board-change detection, and session-tail planning. Browser playback can request deferred `again` so one turn returns to the host, the shared dry-run probe sets `againScheduled`, and `play_host3d` maps that to 2D browser `againing` / `timer`; non-browser session processing can still run the full shared again loop. Browser playback can also request deferred `win` so session tail does not advance the level immediately; `play_host3d` maps `winDeferred` to 2D browser `winning` / `timer`, and `inputoutput.update()` owns the delayed advance through the `winAdvance` screen hook. `run_rules_on_level_start` is a level lifecycle pass with `dontDoWin`, run after level creation/restart/goto/win advance from the pre-start restart source; its `win` and `checkpoint` command tail behavior is pinned by `test/full_turn_2d_parity.test.js`. Checkpoint/restart, numeric `goto`, win level-advance, undo, cancel, link, link-return-on-win, and quit side effects are pinned against a 2D VM oracle in `game_runtime3d_2d_parity.test.js`; numeric `goto` follows 2D section/negative-level target semantics. |
| `WINCONDITIONS` evaluation | `no` / `some` / `all` checks are board-mask predicates, not spatial semantics. Only board iteration and cell access differ. | `src/js/win_conditions.js` shared evaluator with board adapter. | Done for runtime evaluation: the shared evaluator is pinned against the 2D `checkWin` VM oracle for `no`, `some`, `all`, multi-condition, and aggregate-mask cases. 3D compiler copies processed `state.winconditions` into `state.rules3d.winConditions`, and `turn3d.processTurn` evaluates them at the same abstraction level as 2D `processInput` / `checkWin`. `game_runtime3d` only consumes the resulting win session artifact. |
| `rules3d.validateUnsupportedFeatures` / `assertSupportedRule` | A 3D-only unsupported gate creates a language subset. Missing non-spatial support is an implementation gap. | No 3D-only unsupported gate in rule application. Shared helpers implement the semantics; missing behavior must be exposed by parity tests. | Removed from the 3D rule/replacement contract. No non-spatial feature is rejected by a 3D-only runtime gate. |
| `compiler.unsupportedRuleFeatures3d` | Same problem at compile/lowering time. It encodes a reduced 3D semantics set. | Shared lowering with spatial hook checks only. | Removed. Lowered 3D rules do not carry `unsupportedFeatures`. |
| `compiler.markUnsupportedFinalizationFeatures3D` | Records ordinary 2D counterparts as 3D-missing features. This preserves the wrong architecture. | Shared finalization checklist or no separate finalization feature list. | Removed. `state.rules3d.finalization` no longer carries `unsupportedCounterparts`; ellipsis, loop, and gosub are lowered into `state.rules3d` instead of being recorded as 3D-missing counterparts. |
| `compiler.clear2DRuleFinalizationFor3D` | Deletes shared 2D artifacts (`rules`, `lateRules`, rigid/loop points) instead of moving toward shared artifacts. | Shared lowered artifacts consumed by both dimensions. | 3D no longer clears non-spatial artifacts merely because it is 3D. |
| `compiler.groupRules3D` | Rule grouping is not spatial. | Shared group arrangement/collapse behavior. | Done for group arrangement and discard culling: 2D `arrangeRulesByGroupNumber` and 3D `groupRules3D` both call `src/js/rule_grouping.js`, and 3D lowered rules preserve 2D `discard` metadata. `collapseRules` remains 2D object construction, not a separate 3D grouping behavior. |
| `rules3d.applyCellReplacement` for non-spatial replacement semantics | Object/movement clear/set, random object, randomdir, rigid bookkeeping, and sfx tracking are 2D semantics. Only target cell addressing is spatial. | Shared replacement helper with board/movement/sfx/random/rigid adapters. | Done: compiler RHS replacement lowering uses `lowerCellReplacementMasksShared` from both 2D `rulesToMask` and 3D `lowerCellReplacement3D`, with dimension-specific hooks only for movement bit width and direction masks. Runtime object/movement clear-set, random object, randomdir replacement, rigid replacement bookkeeping, rigid rollback/resimulation, create/destroy SFX artifact collection, and movement/cantmove SFX collection use shared helpers or shared artifact contracts with 2D VM-oracle parity and 3D E2E tests. `randomdir` and sound direction masks differ only by spatial direction count/bit table. Session artifacts are owned by `turn_runtime.js`, `command_queue.js`, and `game_runtime3d.js`, not by cell replacement. |
| `cell_match3d.matchesCell` as a separate copy | Matching predicates are the same mask contract. | `src/js/cell_masks.js` shared helper. | Done for mask matching: 2D `CellPattern.matches` and 3D matching call the same predicate, with black-box parity tests pinning 2D-visible match results. |
| `runtime3d` movement mask helpers that duplicate `BitVec` policy | Bit mask operations are not spatial. | Shared mask/BitVec helpers. | 3D code uses shared mask operations; only index/neighbor differs. |
| `runtime3d.startMovement` / `moveEntitiesAtIndex` non-spatial parts | Input seeding movement masks is shared; only direction table and board scan/indexing differ. | Shared movement seeding helper with dimension-specific direction and board iteration hooks. | 3D owns only direction resolution and board iteration. `test/resolve_movements_2d_parity.test.js` now pins depth-1 movement resolution against the 2D VM oracle for movement application, blocked cleanup, repeated scan behavior, movement/cantmove SFX timing, and rigid failure group reporting. |
| `slots3d` timer/session/mutation/input/renderer capability leaves | Session and metadata semantics are not 3D-specific. Marking them unsupported creates a separate language. | Shared metadata/session contract with dimension hooks only for renderer/camera/input direction mapping. | Replaced by `semantic.owner` / `semantic.implemented` entries that name the corresponding 2D owner layer, such as `engine.processInput.checkpoint`, `inputoutput.realtime_interval`, and `graphics.tween`. Implemented session semantics such as checkpoint are marked implemented; browser/input/render adapter gaps are marked unimplemented under their 2D owner rather than as 3D design differences. |
| `slots3d.buildRulesSlot` carrying `unsupportedFeatures` as a core rule field | Makes unsupported subset part of 3D rule data. | Shared diagnostics outside rule semantics, or shared capability result. | Removed. Rule slot carries semantic artifacts, not 3D-only rejection state. |
| `compiler.js` / page shells owning browser 3D playback details | Compiler lowering and page shell code are not the owners of browser session, canvas, renderer, keyboard focus, Three.js loading, or WebGL checks. Keeping those details outside the host adapter makes adapter gaps look like language/compiler semantics. | `src/js/play_host3d.js` as the browser adapter, with `compiler.js` only detecting/delegating to the host capability before start. Page shells call the same compile/start entry and do not carry renderer dependency logic. | Done for the current browser path: `compile()` routes through `compileToState -> startCompiledStateAfterHostPreparation`; 2D remains synchronous, while states with declared host capabilities call `Puzzle3DPlayHost.prepareCompiledState()` before `startCompiledState`. The host loads declared Three/WebGL capabilities, creates the 3D session, renders frames, installs `processInput`, and registers the 3D canvas focus adapter. |
| Keyboard undo/restart bypassing the 3D session | In 2D, `inputoutput.js` handles `Z` / `R` directly through `DoUndo` / `DoRestart`, not through `processInput`. Leaving that unhooked would make browser input semantics differ in 3D even though undo/restart are non-spatial session features. | Keep key mapping, repeat suppression, `pushInput`, and `prevent` in shared `inputoutput.js`; add a focused adapter side-effect hook only when the active game keyboard target is the 3D canvas. | Done for current browser input: `inputoutput.js` asks `Puzzle3DInputAdapter.handleSessionCommand()` before falling back to 2D `DoUndo` / `DoRestart`; `play_host3d` maps those commands to shared 3D session artifacts and rerenders. Tests pin 2D canvas fallback, 3D session hook use, duplicate key suppression, action path, and non-game target behavior. |
| Browser `again` / `autotick` / `win` timing bypassing 2D loop state | `again`, realtime ticks, and win advance delay are browser lifecycle timing, not 3D runtime semantics. If 3D runs every `again` synchronously in the browser bridge, advances on `win` immediately, or invents separate timers, message blocking, delayed no-input turns, autotick gating, and win delay diverge from 2D. | Keep `inputoutput.js` as owner of `againing`, `againinterval`, `autotickinterval`, `winning`, `timer`, message blocking, delayed `processInput(-1)`, and delayed level advance. Let the 3D session facade expose deferred-again and deferred-win results for browser playback. | Done for current browser path: `play_host3d` calls `processSessionTurn3D` with deferred `again` and `win`, maps `againScheduled` to `againing=true` / `timer=0`, maps `winDeferred` to `DoWin`-style browser side effects (`againing=false`, end-level sound, `winning=true`, `timer=0`), and leaves `inputoutput.update()` to fire delayed no-input turns, realtime ticks, and delayed win advance through existing paths. Tests cover deferred session return, bridge state mapping, again update tick, autotick update tick, deferred win, and `winAdvance` hook routing. |
| Browser command output side effects bypassing 2D output handling | `message`, `status`, `sfxN`, restart/undo/cancel sounds, and `quit` shell routing are browser side effects. If 3D only exposes render-frame effects or completes the session internally, it diverges from 2D `processOutputCommands`, `DoRestart`, `DoUndo`, `cancel`, and `quit` behavior. | Centralize host-side command output handling in `play_host3d` helpers. The shared runtime produces artifacts; the browser host maps them to the existing 2D globals/functions (`statusText`, `showTempMessage`, `tryPlaySimpleSound`, `tryPlay*Sound`, pause/level-select/title shell functions). | Done for the current browser host: `play_host3d` maps status text, simple sound commands, command messages, win sound, keyboard/rule restart/undo/cancel sounds, and deferred quit routing through common host helpers. Command message enters the 2D message shell before 3D rerender, matching 2D output-command ordering. `test/play_host3d.test.js` pins command message, status, simple SFX, terminal restart/cancel sounds, deferred win sound, and quit shell routing. |
| Browser title / level-select / pause / message shell bypassing 3D session start | Title, level-select, pause, message screens, and level advancement are browser UI flow, not 3D runtime semantics. If their commit paths call a simplified 3D `nextLevel`, `gotoLevel`, `DoRestart`, `goToTitleScreen`, or message close implementation, 3D playback diverges from 2D level lifecycle side effects. | Keep 2D shell key/selection/message-close behavior in `inputoutput.js`; route level lifecycle through `BrowserLevelFlow`, a shared helper used by 2D `engine.js` itself for `nextLevel` / `gotoLevel` / `loadLevelFromLevelDat`, with 3D hooks only for `levels3`, session creation/resume, canvas removal, title/level-select shell exits, and target resolution. | Done for the current browser level-flow surface: 2D `engine.js` delegates level-flow entry points to `BrowserLevelFlow`, and `play_host3d` uses the same helper for start/title commit/goto/win advance/message close. The helper covers section-solved bookkeeping, local storage/checkpoint keys, message and target levels, start/end sounds, `initSmoothCamera`, transition tail calls (`canvasResize`, `processLevelInput`, `clearInputHistory` where applicable), title exit, and level-select exit. `test/browser_level_flow.test.js` exercises the 2D engine path through that helper and compares 3D-hooked flow against it; `test/play_host3d.test.js` pins 3D title, level-select, pause, command-message, level-message, quit, and win-advance hook wiring. |
| Renderer consuming runtime/compiler internals | Rendering is a presentation adapter concern. If the renderer can read compiled state, live session/runtime objects, or raw source, render gaps become another path for accidental 3D-only semantics. | Runtime/session creates a normalized render frame; renderer consumes only that frame plus canvas/options. The frame schema is explicit and shared by builder and renderer. | Done for the current Three path: `render_frame_contract3d.js` owns `model: psnext-grid3`, `schemaVersion: 1`, size, spriteGrid, objects, drawPlan, cells, session, effects, and view. `render_frame3d` validates generated frames and no longer leaks raw source. `three_renderer3d` validates the same contract in `buildInstances` / `render` and rejects unexpected `state` / runtime-like fields or incomplete draw/cell data instead of falling back. |
| 3D camera syntax leaking renderer internals | Source syntax should follow PuzzleScript Next prelude forms, not renderer-internal presets. Raw words like `projection`, `fov`, `distanceScale`, or direction-word presets describe the adapter implementation rather than the author-facing camera. | Add PS-style prelude metadata for the camera choices authors reasonably control; lower it to internal `frame.view`. Keep low-level renderer details internal. | Done for the current camera surface: `three_dimensions` scopes `orthographic_camera`, `perspective_camera`, `camera_angle <yaw> <pitch>`, `camera_zoom <n>`, `camera_distance <cells>`, and `camera_view_angle <degrees>`. The compiler validates/normalizes those metadata fields, and `render_frame3d` lowers them to `frame.view` fields: `projection`, `yaw`, `pitch`, `cameraZoom`, `cameraDistance`, and `cameraViewAngle`. `three_camera` / `cameraPreset` was removed. |
| 3D sprite display pressuring 2D sprite code | 2D `graphics.js` already defines the PuzzleScript sprite contract: `spritematrix`, `colors`, palette conversion, and transparent cells. 3D rendering should consume that contract, not modify it. | Treat 2D sprite rendering as read-only oracle; project the existing compiled sprite data into 3D render-frame visuals. | Done for normal object sprites: `render_frame3d` converts non-background `spritematrix` into `visual.kind: "spritematrix"` voxel presentation using the same visible-cell rule (`col >= 0`), palette colors, `transparent`, and `spriteoffset` data. `three_renderer3d` consumes those voxels without implicit padding. 2D `graphics.js` was not changed. |
| `three_renderer3d` missing-Three canvas message path | A 2D canvas fallback makes a missing host capability look like a valid 3D renderer mode. That is an adapter workaround, not 2D/3D semantic parity. | Capability loading before playback; Three renderer fails explicitly if `THREE` is absent. | Done: `Puzzle3DPlayHost.canStart()` requires `THREE`, and `Puzzle3DThreeRenderer.render()` throws on missing `THREE` instead of drawing a 2D fallback message. |
| 3D tests that assert non-spatial unsupported behavior | Tests currently freeze the wrong architecture. | Parity tests comparing 2D and 3D artifacts. | No test expects `commands`, `late`, win/session, metadata, tags/mappings, random, loops, or gosub to be different merely because of 3D. |

Keep local 3D code only for spatial hooks:

- 3D level slice parsing
- `coordToIndex` / `indexToCoord`
- `neighbor`
- six-direction direction tables
- relative/oriented spatial rule frames
- movement resolution over 3D neighbors
- renderer/camera
