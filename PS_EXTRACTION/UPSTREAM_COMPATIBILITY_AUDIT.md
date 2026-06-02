# Upstream Compatibility Audit

This audit restates the 3D work around its actual goal: maximize compatibility
with PuzzleScriptNext and keep the result plausible for upstream merge.

The 2D/3D isomorphism principle remains useful, but it is not the project goal.
It is a semantic compatibility check. The higher-order goal is that a
PuzzleScriptNext maintainer can accept the work without breaking existing 2D
games, editor flows, standalone export behavior, or the project maintenance
shape.

## Revised Principle

3D should extend PuzzleScriptNext by adding spatial capability while preserving
the existing 2D language, runtime semantics, and browser/editor contracts.

2D/3D isomorphism is therefore a test for semantic drift, not an architectural
mandate. A change is not justified merely because it makes 2D and 3D look more
symmetrical. It must also be compatible with upstream's existing behavior and
review surface.

Practical reading:

- Keep existing 2D behavior stable when `three_dimensions` is absent.
- Reuse existing PuzzleScript semantics for non-spatial behavior.
- Add 3D-specific code only for spatial concerns: depth, front/back, 3D
  coordinates, 3D neighbors, oriented rule frames, 3D movement resolution, and
  rendering.
- Prefer small shared extractions when they clarify an existing boundary and
  remain natural for 2D.
- Avoid turning 3D support into a broad rewrite of the 2D engine, browser shell,
  editor, or build/export system.

## What This Changes

The previous local question was often:

> Does 3D match the 2D semantic contract?

That question is still necessary, but not sufficient. The upstream-facing
question is:

> Can this be accepted by PuzzleScriptNext as a small, 2D-preserving extension
> whose 3D differences are explicit and spatial?

The difference matters. A shared abstraction can improve 2D/3D symmetry while
still being hostile to upstream if it rewrites stable 2D code paths, introduces
review-heavy indirection, or makes browser/editor behavior harder to reason
about for games that do not opt into 3D.

## Compatibility Axes

Use these axes before adding or reviewing 3D work.

### 2D Behavior Preservation

Question:

> What changes when `three_dimensions` is absent?

Required answer:

- Existing 2D games should compile and play the same way.
- Existing 2D editor, title, message, pause, level select, save, share, and
  export flows should remain unchanged unless the change is an independently
  useful 2D cleanup.
- 2D generated output should not change unless regeneration is intentional and
  explained.

Evidence:

- Existing upstream tests.
- Local 2D VM or browser-shell oracles.
- Golden generated artifacts only when the task explicitly touches export
  generation.

### Semantic Carrier Preservation

Question:

> Is 3D extending an existing semantic carrier, or reinterpreting it?

Required answer:

- Existing carriers such as movement bits, rule masks, command queues, session
  artifacts, win checks, sound masks, metadata, level items, and browser loop
  state must keep their 2D meaning.
- 3D may append spatial information or provide spatial hooks.
- 3D must not remap a 2D meaning into a new 3D meaning. For example, `action`
  remains the 2D action bit; `front` and `back` are appended spatial bits.

Evidence:

- Raw carrier comparisons against 2D oracles.
- Shared helper boundaries where 3D supplies only dimension hooks.
- Tests that project away only appended 3D spatial bits.

## Semantic Carrier Ledger

Use this ledger before changing 3D runtime, compiler, browser, renderer, or
shared-helper code. The purpose is not to make every file symmetrical. The
purpose is to keep each PuzzleScript semantic carrier owned by the existing 2D
contract unless 3D is adding only spatial capacity.

Status meanings:

- **Carrier extraction**: safe shared helper when it parameterizes a concrete
  representation carrier such as movement bit width, bitmap storage, direction
  masks, or cell access.
- **2D-owner projection**: acceptable only when the ordinary 2D path uses the
  helper as the owner and 3D supplies dimension hooks.
- **3D spatial extension**: acceptable only under `three_dimensions`, and only
  for depth, front/back, 3D coordinates, 3D neighbors, oriented frames,
  movement resolution over 3D space, renderer, or camera presentation.
- **Frozen semantic mirror**: do not expand. Shrink it, retire it, or prove it
  is a 2D-owner projection.
- **Design interrupt**: stop before implementation and add evidence first.

### Mode And Public Routing

- 2D owner: `parser.js`, `compiler.js`, editor/browser entry points that
  already route ordinary PuzzleScript source.
- 3D projection: `three_dimensions` is the canonical opt-in marker. It routes
  ordinary `LEVELS` and rule vocabulary into 3D carriers without adding another
  public level section.
- Allowed 3D difference: only opt-in routing and front/back vocabulary under
  `three_dimensions`.
- Evidence: `test/e2e3d.test.js`, `test/compiler_host_prepare.test.js`,
  `test/editor_3d_integration.test.js`.
- Danger signal: a second public 3D mode marker, source-string dependency
  detection, or 2D compile behavior changing when `three_dimensions` is absent.

### Level Items And Playable Grids

- 2D owner: compiler level finalization and existing browser level flow.
- 3D projection: keep ordinary level items (`message`, `level`, `section`,
  `goto`, `link`, `input`, `title`) as the same carrier; lower only playable
  grid payloads through 3D slice parsing.
- Allowed 3D difference: playable grid shape has `depth`; index/coordinate
  helpers use the 3D order.
- Evidence: `test/levels3d.test.js`, `test/game_runtime3d_2d_parity.test.js`,
  `test/browser_level_flow.test.js`.
- Danger signal: routing 3D levels through 2D `Level` loading, or creating a 3D
  copy of message/title/section semantics.

### Object, Property, Aggregate, And Layer Masks

- 2D owner: compiler object setup in `compiler.js`.
- 3D projection: use compiler-produced object ids, layer ids, object/property
  masks, layer masks, and aggregate/property expansion.
- Allowed 3D difference: movement carrier width may increase after object and
  layer counts are known.
- Evidence: `test/compiler_lowering_2d_parity.test.js`,
  `test/cell_masks_2d_parity.test.js`,
  `test/semantic_carrier_3d_parity.test.js`.
- Danger signal: a 3D runtime expanding properties, synonyms, or aggregates
  independently of the compiler.

### Movement Bit Layout And Movement Storage

- 2D owner: compiler movement constants plus `engine.js` movement access
  contract.
- 3D projection: preserve the 2D prefix (`up`, `down`, `left`, `right`,
  `action`) and append only `front`/`back`; provide 3D movement storage with
  the same per-cell, per-layer mask contract.
- Allowed 3D difference: `MOV_BITS`, `MOV_MASK`, `STRIDE_MOV`, and direction
  deltas may expand for 3D.
- Evidence: `test/semantic_carrier_3d_parity.test.js`,
  `test/runtime3d.test.js`, `test/resolve_movements_2d_parity.test.js`.
- Danger signal: remapping `action`, changing the 2D bit prefix, or treating
  input as direct object motion instead of movement-mask seeding.

### Rule Lowering Masks

- 2D owner: compiler rule lowering and the `CellPattern` /
  `CellReplacement` mask contract.
- 3D projection: `rule_lowering.js` may lower the same compiler-prepared IR
  with dimension hooks for movement width, direction masks, and 3D row offsets.
- Allowed 3D difference: only appended spatial movement bits and 3D spatial
  offsets.
- Evidence: `test/compiler_lowering_2d_parity.test.js`,
  `test/compiler_lowering_2d_preservation.test.js`.
- Danger signal: reintroducing `unsupportedFeatures` /
  `unsupportedCounterparts`, or handling tags, mappings, prefixes, properties,
  or aggregates in a 3D-only lowering path.

### Rule Grouping, Random Groups, Loops, And Rigid Finalization

- 2D owner: compiler finalization and engine rule sequence behavior.
- 3D projection: helper extraction is acceptable only where the ordinary 2D
  path calls the helper and the helper surface matches the extracted 2D
  boundary.
- Allowed 3D difference: spatial scan hooks and 3D frame expansion.
- Evidence: `test/rule_grouping.test.js`, `test/rule_finalization.test.js`,
  `test/rule_groups_2d_parity.test.js`,
  `test/random_rule_groups_2d_parity.test.js`.
- Danger signal: adding hook-driven rule sequencing behavior for 3D while 2D
  `engine.js` still owns a different implementation.

### Cell Matching And Cell-Local Replacement

- 2D owner: `CellPattern.matches`, `CellPattern.replace`, and their lowered
  mask semantics.
- 3D projection: evaluate the same object/movement predicates and replacement
  masks against 3D board cell access.
- Allowed 3D difference: 3D neighbor/offset lookup and 3D scan bounds.
- Evidence: `test/cell_masks_2d_parity.test.js`,
  `test/cell_match3d.test.js`, `test/rules3d.test.js`.
- Danger signal: treating replacement as movement, or applying collision-layer
  defaults outside the lowered mask contract.

### Movement Resolution

- 2D owner: `engine.js` movement-resolution phase.
- 3D projection: resolve movement masks per collision layer over 3D neighbors,
  with the same blocking, clearing, rigid failure, SFX, and tween carrier
  meanings.
- Allowed 3D difference: target-neighbor calculation and six-direction spatial
  deltas.
- Evidence: `test/resolve_movements_2d_parity.test.js`,
  `test/full_turn_2d_parity.test.js`.
- Danger signal: adding a high-level push primitive, moving all layers together,
  or clearing movement masks with a 3D-only policy.

### Turn Order And Rule Application

- 2D owner: `engine.js` `processInput` / `procInp`.
- 3D projection: current `turn_runtime.js`, `turn3d.js`, and
  `rule_application.js` are frozen semantic mirrors unless converted into a
  true 2D-owner projection or reduced to carrier extraction.
- Allowed 3D difference: board hooks for cell access, movement resolution,
  spatial scanning, and 3D player position lookup.
- Evidence: `test/turn_runtime.test.js`, `test/turn3d.test.js`,
  `test/full_turn_2d_parity.test.js`.
- Danger signal: adding new non-spatial turn semantics to the hook-driven
  runtime, especially command priority, `late`, `global`, loops,
  `require_player_movement`, or rigid retry behavior.

### Command Queue And Session Artifacts

- 2D owner: engine command queue handling and command session tail semantics.
- 3D projection: preserve command artifacts and consume them through the shared
  command/session planning contract.
- Allowed 3D difference: adapter hooks for restoring/loading 3D board state and
  following visible 3D link targets.
- Evidence: `test/command_queue_2d_parity.test.js`,
  `test/game_runtime3d_2d_parity.test.js`,
  `test/full_turn_2d_parity.test.js`.
- Danger signal: command side effects applied directly inside rule matching,
  renderer code, or browser key handlers instead of through the command/session
  artifact boundary.

### Runtime Metadata Twiddling

- 2D owner: `engine.js` command-time metadata twiddling and metadata-derived
  browser/runtime state.
- 3D projection: apply the same `set` / `default` / `wipe` semantics and then
  refresh metadata-derived 3D slots.
- Allowed 3D difference: 3D slots may project metadata into camera, renderer,
  lifecycle, and input adapter state.
- Evidence: `test/runtime_metadata_twiddling_2d_parity.test.js`,
  `test/ps_metadata_slots.test.js`, `test/turn3d.test.js`.
- Danger signal: keeping a separate 3D metadata copy, or treating a metadata
  command as unsupported because the renderer/session hook has not caught up.

### Win Conditions And Session Tail

- 2D owner: engine win-condition evaluation and `DoWin` / session tail flow.
- 3D projection: evaluate win conditions over board-access hooks and apply tail
  artifacts through the same checkpoint, restart, undo, goto, link, cancel,
  quit, and win-advance semantics.
- Allowed 3D difference: load/restore hooks for 3D board sources and 3D visible
  link lookup.
- Evidence: `test/win_conditions_2d_parity.test.js`,
  `test/game_runtime3d_2d_parity.test.js`,
  `test/full_turn_2d_parity.test.js`.
- Danger signal: immediate 3D win advance in browser mode when 2D delays through
  `winning` / `timer`, or a separate 3D interpretation of numeric `goto`.

### Again Loop And Browser Loop State

- 2D owner: engine/inputoutput browser loop timing and the existing
  `againing` / `timer` state.
- 3D projection: 3D may request deferred again handling and map the result into
  existing browser loop state.
- Allowed 3D difference: no-input probe runs against a cloned 3D runtime.
- Evidence: `test/again_loop.test.js`, `test/play_host3d.test.js`,
  `test/inputoutput3d.test.js`.
- Danger signal: a separate 3D browser loop, or expanding `again_loop.js` as a
  new semantic owner while 2D `engine.js` still owns the original behavior.

### SFX, Animation, And Tween Artifacts

- 2D owner: engine SFX collection and graphics/tween semantics.
- 3D projection: collect the same SFX artifact shapes and project movement
  tween data into render frames.
- Allowed 3D difference: 3D direction deltas and 3D renderer interpolation.
- Evidence: `test/sfx_artifacts_2d_parity.test.js`,
  `test/resolve_movements_2d_parity.test.js`,
  `test/render_frame3d.test.js`.
- Danger signal: playing sounds from rule/session code directly, or making
  tween semantics depend on renderer internals instead of the artifact carrier.

### Input, Keyboard, Mouse, And Screen Commands

- 2D owner: `inputoutput.js` key policy, repeat suppression, title/message/
  pause/level-select shell behavior, and mouse metadata behavior.
- 3D projection: provide focus/session/screen command adapter hooks that let the
  existing browser shell start or resume a 3D session.
- Allowed 3D difference: direction-index normalization and canvas focus testing.
- Evidence: `test/inputoutput3d.test.js`, `test/play_host3d.test.js`,
  `test/browser_level_flow.test.js`.
- Danger signal: a 3D-specific key policy, menu policy, message layout, or
  mouse-command semantics path.

### Renderer Frames And Camera Metadata

- 2D owner: source metadata and existing sprite/graphics contracts remain the
  semantic source; 3D render frames own only presentation snapshots.
- 3D projection: render frames consume normalized state/session data and expose
  renderer-facing `frame.view` / `frame.effects` fields.
- Allowed 3D difference: camera projection, yaw/pitch, zoom, distance, view
  angle, visibility, slice, 3D object instances, and WebGL rendering.
- Evidence: `test/render_frame3d.test.js`,
  `test/editor_3d_playable_contract.test.js`,
  `test/play_host3d.test.js`.
- Danger signal: renderers reading PuzzleScript source, compiler state, or live
  sessions directly; compiler/browser code silently falling back to 2D canvas
  when 3D host capability is missing.
- Design interrupt: this audit and `AGENTS.md` list `camera_distance <cells>`
  as 3D camera metadata, but current compiler behavior rejects
  `CAMERA_DISTANCE`. Resolve that mismatch with a focused syntax/metadata test
  before relying on either behavior.

### Upstream Review Surface

Question:

> Can this be reviewed as a narrow change, or does it force the reviewer to
> accept a new architecture first?

Required answer:

- Prefer changes that are separable by upstream concern: parser/lowering,
  semantic helper extraction, runtime/session integration, browser host,
  renderer, export.
- Avoid bundling unrelated 2D cleanup with 3D feature work.
- Keep adapter code at adapter boundaries; keep compiler code about language
  and lowering; keep runtime code about gameplay state.

Evidence:

- Small files or small patches with clear owner boundaries.
- Tests attached to the owner that changed.
- No generated-artifact churn unless explicitly intended.

### Public Surface Minimality

Question:

> Is this new source syntax, metadata, command, dependency, or browser behavior
> necessary for author-facing 3D?

Required answer:

- Prefer one canonical mode marker: `three_dimensions`.
- Keep camera syntax as PuzzleScript-style metadata, not renderer internals.
- Do not expose implementation terms such as projection/fov presets unless they
  are explicitly promoted into the language design.
- Keep reserved words scarce.

Evidence:

- Parser tests for accepted/rejected syntax.
- Documentation or samples showing the minimum author-facing contract.

### Dependency And Host Impact

Question:

> Does 3D add a dependency or browser capability requirement to ordinary 2D
> play?

Required answer:

- 2D play must not require Three.js or WebGL.
- 3D host capability preparation belongs in the play host, not the compiler
  core or generic browser shell.
- Missing 3D capability should be a 3D host preparation failure, not a 2D
  fallback behavior that hides the failure.

Evidence:

- Host preparation tests.
- Browser tests for 2D paths without 3D capability.
- No source-string heuristics for dependency detection.

## Classification For Future Changes

Classify every proposed change before implementing it.

### 2D-Preserving Shared Cleanup

Allowed when:

- The extracted helper describes a real existing 2D boundary.
- 3D supplies only hooks or data to that helper.
- 2D behavior is pinned before and after extraction.

Examples:

- Shared command/session tail planning.
- Shared rule lowering that accepts dimension hooks.
- Shared win-condition evaluation over board access hooks.

Risk:

- Over-extracting until the 2D path becomes harder for upstream to maintain.

### 3D-Only Spatial Extension

Allowed when:

- The behavior is genuinely spatial.
- It is activated only for `three_dimensions`.
- It does not alter existing 2D carriers except by appending spatial capacity.

Examples:

- 3D level depth.
- `front` / `back` direction words.
- 3D neighbor lookup.
- Oriented 3D rule frames.
- Three.js renderer host.

Risk:

- Smuggling non-spatial semantics into 3D files because the current code path
  happens to need them.

### 3D Syntax Or Metadata Surface

Allowed when:

- The syntax is author-facing and stable.
- It is scoped by `three_dimensions`.
- It avoids renderer-internal vocabulary.

Examples:

- `orthographic_camera`
- `perspective_camera`
- `camera_angle <yaw> <pitch>`
- `camera_zoom <n>`
- `camera_distance <cells>`
- `camera_view_angle <degrees>`

Risk:

- Adding syntax as a shortcut for one renderer or one sample game.

### Browser Adapter Gap

Allowed temporarily when:

- The gap is documented as an implementation gap, not a final semantic
  difference.
- The adapter is localized and does not fork browser key policy, screen flow, or
  command semantics.
- There is a path to delete or shrink the adapter by making the shared browser
  play contract dimension-neutral.

Examples:

- A 3D play host that owns canvas creation and render invocation.
- A bridge that preserves the existing `processInput` entry shape while
  forwarding only dimension hooks.

Risk:

- Normalizing global replacement, duplicated screen flow, or 3D-owned keyboard
  policy as supported architecture.

### Upstream-Hostile Risk

Treat a change as upstream-hostile until proven otherwise when it:

- Changes 2D behavior for games without `three_dimensions`.
- Rewrites stable browser/editor/export paths primarily for 3D symmetry.
- Adds global state or dependency requirements that affect 2D play.
- Creates a second implementation of non-spatial PuzzleScript semantics.
- Explains a parity gap as a supported 3D difference instead of a missing
  implementation.
- Edits generated artifacts directly.

The smallest safe action is usually an audit or parity test, not a local
workaround.

## Shared Helper Admission Rule

The current 2D/3D work should minimize forks, but "shared helper" is not itself
a virtue. A helper is allowed only when it removes a concrete 2D carrier
assumption from logic that must remain semantically identical in 2D and 3D.

Allowed shared helpers parameterize deep carriers such as:

- movement bit width and masks, for example `MOV_BITS`, `MOV_MASK`, and
  `STRIDE_MOV`;
- object and movement bitmap storage, for example `BitVec` versus `Int32Array`;
- direction masks where 3D appends spatial bits without remapping the 2D
  prefix;
- board cell access where the same mask predicate is evaluated over 2D or 3D
  storage.

Do not create or expand a shared helper merely to make 2D and 3D look
architecturally symmetrical. Non-spatial PuzzleScript behavior should have one
owner. If a helper reimplements turn order, session tail behavior, browser loop
timing, command priority, or rule sequencing while the original 2D owner still
exists, it is semantic duplication, not fork reduction.

### Helper Classes

Use this classification before touching an existing helper or adding a new one.

#### Carrier Extraction

Status: keep.

The helper exists because original 2D code had a deep representation assumption
that 3D must extend without changing 2D semantics.

Current examples:

- `src/js/rule_lowering.js`: owns mask lowering over object masks, movement mask
  width, and direction masks. 2D passes the original five-bit movement carrier;
  3D passes the seven-bit carrier with appended `front` / `back`.
- `src/js/cell_masks.js`: owns cell-pattern predicates and cell-local
  replacement mask arithmetic over object and movement bitmap storage.

Required evidence:

- 2D parity or preservation tests against the original carrier behavior.
- Raw carrier comparison where 3D is allowed to differ only by appended spatial
  bits.

#### 2D Owner Projection

Status: allowed only when the 2D path actually uses the helper as its owner.

The helper may stay when it is a direct extraction of existing 2D behavior and
the 2D implementation is reduced to a thin call into that helper. This is not a
license to invent a new architecture for 3D.

Current examples to review narrowly:

- `src/js/rule_groups.js`
- `src/js/command_queue.js`
- `src/js/random_rule_groups.js`
- `src/js/rule_grouping.js`
- `src/js/rule_finalization.js`

Required evidence:

- The 2D engine/compiler calls the helper on the ordinary 2D path.
- The helper's public surface matches the extracted 2D boundary, not a future
  dimension-neutral runtime design.
- A parity test pins the extracted behavior against the original 2D oracle.

#### Semantic Reimplementation

Status: freeze, then retire or shrink.

The helper is suspect when it creates a new hook-driven pipeline for
non-spatial semantics that are still owned by 2D `engine.js` or browser code.
This may help local 3D progress, but it increases the number of behavioral
owners.

Current freeze list:

- `src/js/turn_runtime.js`
- `src/js/session_runtime.js`
- `src/js/again_loop.js`
- `src/js/rule_application.js`, except where it can be proven to be a direct
  2D-owner projection rather than a 3D-only rule runner

Freeze means:

- Do not add new semantics or special cases to these helpers.
- Do not cite them as the desired final shared architecture.
- Any bug found here should first be checked against the 2D owner and oracle.
- Prefer shrinking 3D dependence on these helpers, or converting a helper into a
  true 2D-owner projection, over expanding the hook surface.

Required evidence before unfreezing:

- The ordinary 2D path has moved to the helper as the single implementation, or
  the helper has been reduced to carrier extraction.
- The migration does not make upstream review accept a broad 2D runtime rewrite
  as a prerequisite for 3D.

### Decision Test

Before adding a shared helper, answer this in the change description:

> Which concrete 2D carrier assumption is being parameterized?

If the answer is not a carrier such as bit width, bitmap storage, direction mask,
cell accessor, coordinate/index conversion, or another deep representation
assumption, do not add the helper. Keep the behavior with its existing owner and
give 3D only the spatial adapter hooks it needs.

## Design Interrupt: Upstreamability

In addition to the existing 2D/3D semantic parity interrupt, stop and surface an
upstreamability interrupt when an observation suggests that a change may improve
local 3D progress while reducing upstream compatibility.

Use this form:

- Governing goal at risk: upstream compatibility / mergeability.
- Local observation: the concrete code, test, syntax, dependency, or browser
  flow that creates doubt.
- Drift type: 2D behavior change, broad architecture rewrite, public surface
  expansion, dependency impact, generated artifact churn, or non-spatial
  semantic duplication.
- Evidence needed: 2D oracle, generated output diff, browser/editor flow test,
  host capability test, or upstream patch split.
- Smallest safe next action: usually classify the change, add a focused test, or
  split the patch before implementing more.

Do not proceed to renderer polish, fallback behavior, or broad cleanup while an
upstreamability interrupt is unresolved.

## Merge-Oriented Patch Order

A plausible upstream path should be staged from least invasive to most visible.

1. 2D-preserving tests and helper extractions.
2. Parser/compiler changes for `three_dimensions` and 3D level/rule lowering.
3. 3D runtime/session modules that call shared semantic helpers.
4. Browser play host capability preparation and 3D session bridge.
5. Renderer and camera metadata.
6. Samples and documentation.
7. Generated standalone/export artifacts, only when explicitly regenerated.

This order is not a required implementation sequence for local experiments. It
is the order to keep in mind when deciding whether a change is upstreamable.

## Current Read Of The Project

The project is still broadly aligned with upstream compatibility because it has
kept the most important non-spatial PuzzleScript semantics tied to 2D-preserving
contracts: rule lowering, movement masks, commands, `late`, `again`, win/session
flow, and browser loop timing.

The main risk is no longer that 3D becomes a separate language. The main risk is
that preserving 2D/3D isomorphism becomes a reason to redesign upstream's 2D
browser/editor architecture before the upstream reviewer has accepted the
smaller 3D extension.

The next design posture should be:

> Use isomorphism to detect semantic drift. Use upstream compatibility to decide
> whether the resulting implementation shape is acceptable.
