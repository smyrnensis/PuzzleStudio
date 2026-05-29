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
- `...` ellipsis, except for the spatial scan dimension
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
- ellipsis spatial scan
- movement resolution
- rigid movement rollback over dimension-specific movement
- renderer projection

The hook may differ. The feature semantics may not.

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

## Rebuild Order

1. Audit the current 3D prototype against the 2D compiler and engine.
2. For every unsupported 3D feature, classify it as spatial or non-spatial.
3. Remove unsupported status from non-spatial phase/control features first,
   starting with `late`. `late` has been removed from the 3D unsupported gate
   and is now tested as the same post-movement turn phase. Rule `commands` have
   also been removed from the unsupported gate and are now returned as turn
   command artifacts.
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
| `turn3d.queueCommands` and command/session artifact assembly | Command queuing and command-to-session artifact classification are not spatial. 2D already defines priority, dedupe, `message`, `goto`, `status`, `gosub`, `log`, `win`, `again`, `restart`, and `checkpoint` behavior. | `src/js/command_queue.js` shared helper. | Done for queue and session artifact classification: both 2D `Rule.prototype.queueCommands` / `processOutputCommands` and 3D `turn3d` call the shared helper, with VM-oracle parity tests pinning 2D-visible outputs. 2D-only metadata twiddling remains in `engine.js` as a queued-command side-effect hook. |
| `turn3d.applyRuleGroup` propagation loop | Rule group repetition, `once`, random groups, loop/gosub flow, and command timing are PuzzleScript semantics, not 3D semantics. | Shared rule-group runner or shared rule-group control artifact with dimension-specific match/apply hooks. | In progress: ordinary rule-group propagation now uses `src/js/rule_groups.js` from both 2D and 3D, including 2D-compatible `loopPropagated` return behavior and `once` handling. `turn3d.applyRule` now follows the 2D `Rule.tryApply/applyAt` shape: collect all pattern-row matches, generate tuples, apply tuple replacements with later-tuple recheck, then queue commands once for the whole rule. `random` rule group candidate enumeration and selection uses `src/js/random_rule_groups.js`. These are pinned by 2D VM-oracle parity tests plus 3D turn/E2E tests. 3D session `again` uses explicit object-mask `boardChanged`, not the rule-group return value. Loop/gosub flow remains to share. |
| `turn3d.processTurn` as a full turn runner | Turn phase order and tail processing are shared. The current function is a reduced 2D `processInput`. | Shared turn contract: input seed, normal rules, movement hook, late rules, command/win/again/session artifacts. | In progress: `turn3d` now returns shared session artifacts, and `game_runtime3d` owns the session facade for checkpoint/restart/goto/win/again consumption. Browser/session integration remains outside the board runtime. |
| `game_runtime3d.applySessionArtifacts3D` session tail | Command priority and session side effects are PuzzleScript semantics, not 3D semantics. | Shared session-tail planner plus dimension/session-specific effect hooks. | In progress: 3D now uses `CommandQueue.planSessionTail` for 2D command priority (`undo`, `goto`, `link`, `cancel`, `restart`, `quit`, then `win`/`checkpoint`/`again`) and uses `boardChanged` for `again` eligibility. The planner is pinned against a 2D VM tail oracle in `command_queue_2d_parity.test.js`. Effect hooks for undo/cancel/quit/link are still simplified 3D facade behavior, not complete 2D browser/session parity. |
| `rules3d.validateUnsupportedFeatures` / `assertSupportedRule` | A 3D-only unsupported gate creates a language subset. Missing non-spatial support is an implementation gap. | Shared feature support diagnostics keyed by shared contract gaps. | No non-spatial feature is rejected by a 3D-only gate. |
| `compiler.unsupportedRuleFeatures3d` | Same problem at compile/lowering time. It encodes a reduced 3D semantics set. | Shared lowering capability checks plus spatial hook checks. | Only spatial limitations remain dimension-specific. |
| `compiler.markUnsupportedFinalizationFeatures3D` | Records ordinary 2D counterparts as 3D-missing features. This preserves the wrong architecture. | Shared finalization checklist or no separate finalization feature list. | `unsupportedCounterparts` is gone or contains only spatial hook gaps. |
| `compiler.clear2DRuleFinalizationFor3D` | Deletes shared 2D artifacts (`rules`, `lateRules`, rigid/loop points) instead of moving toward shared artifacts. | Shared lowered artifacts consumed by both dimensions. | 3D no longer clears non-spatial artifacts merely because it is 3D. |
| `compiler.groupRules3D` | Rule grouping is not spatial. | Shared group arrangement/collapse behavior. | Done for group arrangement and discard culling: 2D `arrangeRulesByGroupNumber` and 3D `groupRules3D` both call `src/js/rule_grouping.js`, and 3D lowered rules preserve 2D `discard` metadata. `collapseRules` remains 2D object construction, not a separate 3D grouping behavior. |
| `rules3d.applyCellReplacement` for non-spatial replacement semantics | Object/movement clear/set, random object, randomdir, rigid bookkeeping, and sfx tracking are 2D semantics. Only target cell addressing is spatial. | Shared replacement helper with board/movement/sfx/random/rigid adapters. | In progress: compiler RHS replacement lowering now uses `lowerCellReplacementMasksShared` from both 2D `rulesToMask` and 3D `lowerCellReplacement3D`, with dimension-specific hooks only for movement bit width and direction masks. Runtime object/movement clear-set, random object, randomdir replacement, rigid replacement bookkeeping, rigid rollback/resimulation, create/destroy sfx artifact collection, and movement/cantmove sfx collection use shared helpers or shared artifact contracts with 2D VM-oracle parity and 3D E2E tests. `randomdir` and sound direction masks differ only by spatial direction count/bit table. Broader session artifacts are still pending. |
| `cell_match3d.matchesCell` as a separate copy | Matching predicates are the same mask contract. | `src/js/cell_masks.js` shared helper. | Done for mask matching: 2D `CellPattern.matches` and 3D matching call the same predicate, with black-box parity tests pinning 2D-visible match results. |
| `runtime3d` movement mask helpers that duplicate `BitVec` policy | Bit mask operations are not spatial. | Shared mask/BitVec helpers. | 3D code uses shared mask operations; only index/neighbor differs. |
| `runtime3d.startMovement` / `moveEntitiesAtIndex` non-spatial parts | Input seeding movement masks is shared; only direction table and board scan/indexing differ. | Shared movement seeding helper with dimension-specific direction and board iteration hooks. | 3D owns only direction resolution and board iteration. |
| `slots3d` `supported: false` timer/session/mutation/input/renderer leaves | Session and metadata semantics are not 3D-specific. Marking them unsupported creates a separate language. | Shared metadata/session contract with dimension hooks only for renderer/camera/input direction mapping. | No non-spatial metadata/session feature is disabled only because of 3D. |
| `slots3d.buildRulesSlot` carrying `unsupportedFeatures` as a core rule field | Makes unsupported subset part of 3D rule data. | Shared diagnostics outside rule semantics, or shared capability result. | Rule slot carries semantic artifacts, not 3D-only rejection state. |
| 3D tests that assert non-spatial unsupported behavior | Tests currently freeze the wrong architecture. | Parity tests comparing 2D and 3D artifacts. | No test expects `commands`, `late`, win/session, metadata, tags/mappings, random, loops, or gosub to be different merely because of 3D. |

Keep local 3D code only for spatial hooks:

- 3D level slice parsing
- `coordToIndex` / `indexToCoord`
- `neighbor`
- six-direction direction tables
- relative/oriented spatial rule frames
- ellipsis spatial scan shape
- movement resolution over 3D neighbors
- renderer/camera
