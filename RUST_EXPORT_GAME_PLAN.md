# Rust Export Game Plan

## Goal

A standalone exported game must execute all game, presentation, rendering, and
audio behavior in Rust. Browser exports run that Rust as WebAssembly; native
players run the same owned contracts directly.

The browser HTML is only a launch shell. Generated `wasm-bindgen` glue may load
the module and connect browser APIs, but handwritten JavaScript must not own or
reinterpret game behavior.

Completion requires exported games to support:

- scene and component flow, including title screens, menus, focus, and modal
  components;
- 2D and 3D rendering;
- clip, move, tween, and trigger animation;
- deterministic SFX and music generation plus playback control;
- keyboard and pointer input;
- waits, messages, level flow, undo, redo, and restart;
- progress persistence and embedded assets;
- the same typed runtime and presentation semantics on native and WebAssembly.

Passing a Bevy preview or rendering an active puzzle is partial progress, not
evidence that the export goal is complete.

## Governing Principles

### One Semantic Implementation

Every game and presentation rule has one Rust owner. Native and browser players
execute the same implementation instead of maintaining equivalent Rust and
JavaScript behavior.

### Minimal Runtime Interfaces

Crate boundaries remain explicit, but they exchange typed Rust values directly.
An ownership boundary does not require a transport boundary.

The steady-state game loop must avoid:

- serializing a runtime snapshot to JSON and parsing it again for presentation;
- projecting the same scene or render state separately for each backend;
- crossing Rust/JavaScript once per component, object, pixel, voxel, or audio
  event;
- copying a complete snapshot when a borrowed value, stable asset handle, or
  typed change set is sufficient;
- creating an intermediate raster or other adapter representation that the next
  stage immediately expands again.

The remaining interfaces must correspond to real ownership or platform
boundaries: session commands, resolved presentation plans, GPU submission,
audio-device submission, browser input, storage, and asset IO.

### Measured Performance

Removing redundant interfaces is the intended performance mechanism, but the
migration must measure its result. Before replacing the existing export, record
representative 2D and 3D baselines for CPU presentation time,
input-to-browser-frame latency, retained JavaScript heap and Wasm
linear-memory capacity, exported payload bytes, and application-owned
Rust/browser adapter calls. Generated Wasm bindings may cross into browser GPU
and audio APIs at their declared platform boundary; those calls are not
misreported as application transport. The Rust export must remove redundant
application transport costs and must not regress the measured interactive frame
budget.

## Ownership Boundary

The completed data flow is:

```text
authored game
  -> puzzle-lang
  -> Rust runtime/session
  -> typed presentation state and commands by direct Rust calls
  -> Bevy renderer/audio/UI infrastructure
  -> wgpu/browser audio/browser storage
```

Rust semantic owners decide:

- scene identity, component visibility, focus, and modal ordering;
- which menu action is selected and what activating it means;
- object-to-visual resolution, palette resolution, render order, transforms,
  composition, and animation conflicts;
- tween endpoints, interpolation rules, and the current resolved render frame;
- sound identity, seeded synthesis parameters, music score, playback lifecycle,
  and event ordering;
- progress-save data and when it must be read or written.

Backend infrastructure decides:

- Bevy UI node, text, mesh, material, and sprite construction;
- batching, instancing, culling, camera execution, shadows, and GPU buffers;
- final framebuffer rasterization;
- decoded raster texture upload;
- platform audio device and stream execution;
- browser storage and asset IO calls through explicit Rust host contracts.

Renderers and audio backends must not receive authoring expressions, visual
names, palette tokens, raw `SceneEffect` values, sound seeds, or music grammar.

JSON is allowed for authored/exported file formats and explicit persistence
payloads. It is not an internal frame, scene, input, animation, or audio
transport between Rust-owned runtime stages.

## Current State

Implemented foundations:

- `puzzle-presentation` produces typed, source-free 2D and 3D render scenes and
  resolved frames.
- The Bevy 3D backend consumes resolved voxels.
- The Bevy 2D backend consumes logical resolved pixels as direct vertex-colored
  mesh geometry.
- Logical pixels are not converted to an intermediate bitmap. Each axis edge is
  computed once, so adjacent pixels reference the exact same mesh coordinate.
- The Bevy player connects the typed runtime session to active 2D and 3D puzzle
  scenes.
- The public player snapshot exposes an evaluated, ordered surface and a
  component-scoped viewport registry. It does not expose authored scene
  definitions, raw scene/session values, raw effects, or index-based choice
  activation.
- Scene buttons, choices, keys, pointer events, waits, and animation events use
  stable typed tokens and `RuntimeViewportSourceId`; keyboard semantics are
  resolved once by the Rust session.
- The Bevy player boots title-only surfaces, renders root/content/overlay and
  modal UI layers, lowers typed layout and theme values, and derives exact
  framebuffer rectangles from viewport UI leaves.
- The player owns separate full-window theme-clear and transparent UI cameras
  around the renderer view cameras. View camera order follows resolved surface
  root/content/overlay/modal stack and tree order, independent of component or
  source identifier sorting.
- UI layout, resolved-frame submission, and both renderer queue consumers share
  one ordered `PostUpdate` contract, so viewport rectangles, frames, and camera
  order reach the render world in the same frame.
- The Bevy 2D and 3D renderers own multiple keyed views; 3D camera and lighting
  state is view-owned rather than plugin-global.
- The runtime snapshot carries the final resolved 2D view and linear RGBA
  theme. Presentation emits typed 2D/3D line decorations and explicit raster
  batches, while decoded RGBA image bytes remain in a typed asset catalog keyed
  by asset ID and revision.
- Standalone startup uses one versioned Rust contract containing the complete
  loaded document, compact encoded visual-image bundle, and progress-storage
  identity. `puzzle-player-bootstrap` strictly validates that contract,
  rejects omitted or stale nested fields, checks the exact referenced asset
  set, decodes images once, and returns the runtime, immutable image catalog,
  and storage identity together.
- Image asset IDs are lowered once by `puzzle-lang` as typed manifests and are
  consumed directly by Rust runtime/bootstrap owners. The WASM player retains
  the decoded catalog and exposes the bundle-owned progress identity; browser
  boot metadata no longer publishes a second save key or version.
- `puzzle-play` owns one typed scene-condition context used by both projection
  and action-token resolution, so restored progress cannot expose a Continue
  action that execution later rejects under a different condition environment.
- The Bevy host exposes typed restore, pending-save, exact request-ID
  acknowledgement, and clear operations while keeping the runtime private and
  refreshing its typed surface and viewport state after persistence changes.
- Official player/game WASM bindings expose the typed session ingress and the
  request-ID persistence protocol without command-name or direct input-name
  compatibility methods. Raw editor-state injection exists only in the
  editor-debug game artifact and is absent from the player artifact and normal
  standalone export. Debug trace input likewise uses an editor-only WASM method
  and is not a player `SessionAction`.
- The topmost visible modal exclusively owns keyboard resolution, including
  unmatched keys, so input cannot fall through to focused scenes or model
  actions behind it.
- Audio events resolve to typed asset IDs; seeded SFX/music synthesis,
  deterministic indexed block rendering, playback lifecycle, native Bevy
  submission, and browser AudioWorklet submission are Rust-owned. The editor
  uses a separate editor-only Rust recipe/audition contract over the same
  runtime and backend rather than exposing recipes in the player snapshot.
- Official standalone HTML contains one Bevy canvas, the versioned runtime
  export, generated WASM loading glue, and explicit fatal diagnostics. The 2D,
  3D, and mixed-document export routes all construct that same launcher
  directly from the complete loaded document, typed visual-image bundle, and
  progress identity.
- Screenshot export boots the same canonical initial surface as the standalone
  player; it has no URL or CLI scene-override path.
- Official player generation uses a dedicated size-oriented Cargo profile with
  LTO, one codegen unit, aborting panics, and stripped symbols. The generator
  enforces byte budgets for the player WASM, generated glue, and audio worklet;
  the current artifacts are 40,555,988 bytes, 123,849 bytes, and 745,170 bytes,
  respectively.
- A separate final-export size gate runs the official CLI for fixed 2D and 3D
  fixtures and checks raw and gzip HTML budgets. Current results are
  60,275,181/14,171,542 bytes for 2D and 54,362,796/14,057,121 bytes for 3D.
- Browser progress restore and exact request-ID save acknowledgement are owned
  by the Rust/WASM host. A committed save acknowledgement cannot be exposed as
  retryable after the runtime consumes it; later projection failure enters the
  visible player-fatal channel. Recoverable storage failures remain on a
  separate visible diagnostic surface with the typed request still pending.
- The progress-storage identity is derived from the compiled game's title,
  model identities, and level content rather than a checkout-local filesystem
  path, so moving an authored game between workspaces preserves its save
  namespace while materially different level sets remain distinct.
- After the renderer queues apply, the Bevy player publishes a read-only typed
  observation containing sequence, snapshot revision, surface focus, and
  viewport count. The WASM adapter projects it to a dedicated hidden browser
  status element; fatal state uses the same operational surface without
  becoming a gameplay acceptance gate.
- Screenshot capture uses the shared Chrome DevTools transport, waits for that
  typed ready observation, rejects fatal diagnostics and browser errors, then
  captures through CDP and validates decoded PNG dimensions. The CLI no longer
  treats file creation as render success.
- Browser audio unlock, independent music-worklet preparation, visibility and
  context wakeups, command submission, feedback routing, and device teardown
  are Rust-owned. Worklet delay or failure is contained to music
  materialization and cannot retain the unlock completion or disable SFX.
- The committed browser contract suite exports representative 2D and 3D games
  and dedicated visibility, persistence, and external-image fixtures. It drives
  title-to-gameplay input through CDP, reloads and clears Rust-owned progress,
  resizes and hides/restores the page, confirms a running Rust-owned browser
  audio backend, rejects browser/fatal diagnostics, and verifies decoded
  screenshot pixels against fixture-owned image shape.
- The browser suite records startup, input latency, Rust presentation CPU,
  steady-state submission intervals, JavaScript heap and Wasm linear-memory
  growth, total heap/memory, exported HTML payload bytes, storage adapter calls,
  and typed host-observation attribute writes. Startup and input latency cross
  two browser animation frames after the typed Rust submission; final decoded
  screenshot validation is measured separately so GPU readback time is not
  mislabeled as game latency. The host-adapter counters are
  deliberately named for the exact browser calls they observe; they are not
  presented as comprehensive instrumentation of every Web API call made by
  generated Wasm bindings. The Rust submission counter proves that observation
  writes remain bounded per submitted frame rather than scaling with visible
  pixels, voxels, components, or sounds. One committed budget file owns common
  interaction limits plus
  fixture-specific memory and payload thresholds. Two serial Chrome
  measurements under the committed 3-second visibility window recorded maxima
  of 1,667.36 ms startup, 1,492.72 ms input-to-browser-frame latency, 8,701
  microseconds presentation CPU p95, 250,000 microseconds submission interval,
  331,808 bytes
  steady JavaScript heap growth, and zero Wasm linear-memory growth. Immediate
  regenerated-release validation additionally observed 5,301- and
  8,701-microsecond presentation CPU p95 values in the representative 2D and
  3D fixtures. The committed limits retain explicit measured headroom rather
  than the former provisional ceilings.
- Release WASM generation remaps repository, user, Cargo, and toolchain paths
  through one shared build environment and rejects generated artifacts that
  still contain a local build path.
- Native tests and `wasm32-unknown-unknown` compilation cover the current Bevy
  renderer and player crates.

Known incomplete boundaries:

- EditorPreview and live development tooling retain their separately owned
  legacy browser presentation implementation and editor-only
  `WasmStandaloneSession`. They are not loaded by, embedded in, or available as
  a fallback to the official player artifact. Replacing those development
  surfaces is a separate migration with different debugging contracts.
- Camera-backed viewport leaves are ordered against other viewport leaves, but
  arbitrary interleaving with overlapping Bevy UI descendants is not yet an
  owned composition contract. If authored overlap requires that behavior,
  viewport output must become a UI-composited render target rather than adding
  more camera-order cases.
- A reproducible pre-cutover browser-performance baseline was not recorded.
  EditorPreview is a different contract and cannot substitute for it; the
  first complete Rust-player measurements are therefore the committed
  regression baseline rather than a historical before/after comparison.

## Work Packages

### A. Resolved Scene And Menu Actions

Owners: `puzzle-scene`, `puzzle-play`, `puzzle-game-runtime`, and
`puzzle-session-contract`.

Replace renderer-visible scene effects with stable typed action tokens.

The resolved surface must contain:

- a visible, ordered component tree;
- resolved text and layout values;
- focused component identity;
- selected state attached to each selectable node;
- stable action tokens for buttons, choices, keys, and pointer events;
- viewport leaves that reference a typed 2D or 3D presentation source;
- explicit presentation errors at the owning component boundary.

Activating a token returns it to the Rust session. The session resolves and
executes the effect; Bevy must not interpret `SceneEffect`.

Primary acceptance case:

- `TENETEN.puzzle` boots into its title scene in Bevy, moves menu focus, activates
  a choice, and enters the playable 2D scene without JavaScript scene handling.

### B. Bevy Scene UI

Owner: the Bevy player/presentation adapter.

Implement the resolved scene tree with Bevy UI:

- row, column, box, text, heading, caption, button, and choice nodes;
- fit/fill, alignment, distribution, gap, aspect ratio, and scrolling;
- focused and selected visual states;
- keyboard and pointer activation through typed action tokens;
- modal ordering and viewport composition;
- 2D and 3D viewport leaves using the existing renderer plugins.

Style values must arrive as resolved typed theme data. The Bevy UI layer may
provide backend defaults, but it must not parse theme strings or presets.

### C. Complete Animation Path

Owners: `puzzle-play` and `puzzle-presentation`.

Unify clip, move, tween, and trigger timing in the input to
`resolve_render_moment`. The result submitted to Bevy must already contain the
current transform, opacity, frame, render order, and composition result.

Tests must cover:

- endpoint values at time zero and completion;
- interpolation at a controlled intermediate time;
- competing animation channels;
- interruption, wait, undo, restart, and scene transition boundaries;
- both 2D pixels and 3D voxels.

The renderer may apply resolved per-frame values but must not select tween rules
or reinterpret animation channels.

### D. Rust SFX And Music

Owners: a Rust audio-presentation/synthesis crate plus the session contract.

Port seeded SFX and music generation from the current JavaScript modules. Split
semantic generation from device playback:

```text
sound definition + seed
  -> deterministic Rust clip/score
  -> typed audio asset
  -> typed play/pause/resume/stop command
  -> Bevy/platform audio backend
```

Required properties:

- identical inputs produce identical samples or score events;
- SFX deduplication and presentation-event order stay session-owned;
- named and unnamed pause/resume/stop behavior is explicit;
- volume is resolved before backend submission;
- native and WebAssembly use the same generated audio representation;
- unavailable audio output fails in the audio consumer without invalidating an
  otherwise valid game session.

Compatibility tests should pin representative PuzzleScript, typed SFX, and
music outputs before removing the JavaScript generators.

### E. Presentation Contracts

Owners: `puzzle-presentation`, `puzzle-session-contract`, and asset adapters.

Implemented backend-neutral data required by export:

- final resolved 2D view origin and size;
- typed grid/line decoration draw plans;
- resolved linear RGBA background and UI theme values;
- decoded external-image RGBA plus dimensions, sampling, and fit mode;
- explicit raster batches distinct from logical ASCII/solid pixels.

Logical visuals continue to use direct mesh geometry. Already-raster external
images use texture upload and must not be routed through the logical-pixel path.
Decoded image bytes remain in the typed asset catalog rather than the public
player snapshot. Native loading and standalone browser bootstrap now supply
that catalog through the same validated asset contract. Bevy/WASM texture
upload and browser-host ownership remain in Package F.

### F. Rust Browser Host

Owners: the WebAssembly player and browser adapter.

Move browser services behind Rust-owned contracts:

- animation clock and frame scheduling;
- keyboard, pointer, resize, and focus events;
- progress-save serialization and browser storage;
- embedded asset lookup and image decoding;
- audio-context lifecycle;
- canvas creation and resize.

JavaScript generated by the binding tool may expose browser calls, but no
handwritten JavaScript state machine or presentation projection may remain.
Browser events should enter Rust in bounded batches, and render/audio updates
should leave Rust as backend-ready buffers or handles rather than object-wise
callbacks.

### G. Export Replacement

Owner: `html-play` standalone export.

Replace the current embedded runtime scripts with the Rust/Bevy WebAssembly
player. Remove export dependencies on:

- `renderer.js`;
- `standalone.js`;
- `visual_tween_core.js`;
- `puzzle3_visual_core.js`;
- `puzzle3_three_renderer.js`;
- `puzzle3_component.js`;
- Three.js;
- seeded SFX/music JavaScript modules.

Do not preserve the old renderer as a fallback. The new export must fail with a
specific diagnostic when its required WASM or asset contract is absent.

## Verification Gates

### Owner Tests

- Scene action tokens execute only through the Rust session owner.
- Resolved UI trees contain selected/focused state without consumer traversal
  heuristics.
- Adjacent logical pixels share exact edge coordinates.
- Raster images cannot enter the logical-pixel mesh path.
- Tween results are deterministic under controlled clocks.
- Seeded audio generation is deterministic.
- Pause/resume/stop state transitions are typed and ordered.

### Integration Tests

- Native Bevy title/menu to 2D gameplay.
- Native Bevy title/menu to 3D gameplay.
- Pointer and keyboard menu activation.
- Tween and trigger animation in both dimensions.
- SFX and music command execution.
- Save, reload, and level continuation.

### Export Tests

- Exported 2D and 3D games boot in a browser from their title scene.
- Browser screenshots cover scene UI, 2D, 3D, tween, and external images.
- Audio tests verify generated buffers or score traces without depending on a
  physical audio device.
- Exported HTML contains no handwritten game-runtime, renderer, tween, or audio
  JavaScript.
- The export contains the player WASM artifact and only the required generated
  loading glue.
- Missing WASM, asset, storage, and audio capabilities produce specific owner
  diagnostics rather than alternate execution paths.
- Steady-state gameplay performs no JSON serialization or parsing between the
  session, presentation, UI, renderer, and audio stages.
- Instrumented exports report bounded application-owned host-adapter calls per
  submitted frame rather than calls proportional to visible pixels, voxels,
  components, or sounds.

### Performance Tests

- Record the existing export baseline on fixed 2D and 3D scenes with controlled
  window size, animation clock, and input sequence.
- Compare Rust presentation CPU, input-to-browser-frame latency, retained
  JavaScript heap and Wasm linear-memory capacity, exported payload bytes, and
  application-owned host-adapter call counts.
- Separate startup compilation and asset decode costs from steady-state
  gameplay costs.
- Retain benchmark fixtures with the owning crates so later adapter changes
  cannot silently reintroduce serialization or per-object boundary traffic.

## Completion Criteria

The migration is complete only when all of the following are true:

1. A representative 2D game and 3D game can be exported and played from title
   screen through gameplay in the browser.
2. Scene/menu, rendering, tween, sound, music, persistence, and lifecycle
   behavior is implemented in Rust/WASM.
3. The native Bevy player and browser export consume the same typed session and
   presentation contracts.
4. Standalone export no longer embeds or calls the handwritten JavaScript
   runtime, renderers, tween core, Three.js renderer, or audio generators.
5. Logical ASCII/solid visuals reach the final framebuffer without an
   intermediate raster boundary.
6. Unsupported or missing capabilities fail visibly at their owning boundary;
   there is no legacy renderer or JavaScript fallback.
7. Session, scene, animation, render, and audio data moves through typed Rust
   calls without a JSON or duplicate-projection boundary during gameplay.
8. Performance measurements demonstrate removal of the redundant transport
   costs and no regression of the representative interactive frame budget.
