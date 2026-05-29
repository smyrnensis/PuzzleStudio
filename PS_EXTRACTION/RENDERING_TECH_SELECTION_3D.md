# 3D Rendering Technology Selection

This note records the initial rendering technology direction for 3D
PuzzleScript work in this checkout.

## Decision

Use Three.js as the first 3D renderer foundation.

The initial implementation should use:

- `THREE.WebGLRenderer`
- `THREE.InstancedMesh` for repeated cell/object geometry
- `THREE.OrthographicCamera` as the default play camera
- `THREE.PerspectiveCamera` as an optional finite-distance camera mode
- flat or toon-like materials that preserve object identity
- fixed camera presets rather than unrestricted free camera movement

This is a browser-facing renderer choice. It does not change PuzzleScript
runtime semantics.

## PuzzleScript Constraints

Official PuzzleScript is organized around tile/cell state, object definitions,
legend expansion, collision layers, rules, win conditions, and levels.

The important rendering consequence is that PuzzleScript is not primarily a
continuous 3D world model. It is a grid-based symbolic state model:

- a cell can contain multiple objects
- legend entries can expand one level character into several objects
- collision layers determine which objects can coexist in one cell
- rules are pattern replacements over cells and movement markers
- movement markers are resolved after normal rules, before late rules

Therefore the 3D renderer should be designed as a 3D cell-state viewer, not as a
general voxel terrain engine.

Reference documentation:

- https://www.puzzlescript.net/Documentation/documentation.html
- https://www.puzzlescript.net/Documentation/objects.html
- https://www.puzzlescript.net/Documentation/legend.html
- https://www.puzzlescript.net/Documentation/collisionlayers.html
- https://www.puzzlescript.net/Documentation/rules101.html
- https://www.puzzlescript.net/Documentation/rules.html

## Why Three.js

Three.js is the best first choice because it gives enough 3D control without
making rendering infrastructure the main project.

It provides:

- orthographic and perspective cameras
- depth testing and occlusion
- transparency for ghosted front slices
- instancing for repeated cubes, markers, and cell glyphs
- picking support for editor/debug interaction
- stable browser deployment through WebGL
- a small enough integration surface for adapter-owned rendering

The main renderer problem is not raw voxel throughput. It is preserving
PuzzleScript readability when board state has depth. Three.js is sufficient for
that problem and leaves room for later optimization.

## Fit With PuzzleScript Technology

Three.js is a good fit only if it is kept as an adapter-level presentation
library. It should not become part of the PuzzleScript semantic model.

The fit is strongest in these areas:

- PuzzleScript board state is discrete, so each cell/object representation can
  be mapped to stable transforms and instance slots.
- PuzzleScript object identity is explicit, so renderer material and shape
  lookup can be driven by compiled object IDs rather than scene inspection.
- Collision layers already define a stable ordering concept, which can inform
  visual stacking or draw grouping without changing coexistence rules.
- Turn-based updates mean the renderer can update snapshots after committed
  state transitions rather than streaming continuous physics.
- Orthographic projection maps well to PuzzleScript's symbolic tile reading.
- Three.js picking can support editor/debug inspection of cells without adding
  renderer knowledge to the runtime.

The fit is weakest if the renderer is allowed to infer behavior:

- Three.js scene hierarchy must not imply gameplay ownership or grouping.
- Mesh adjacency must not imply PuzzleScript adjacency; board coordinates do.
- Draw order must not become collision-layer semantics.
- Animation state must not become hidden game state.
- Camera occlusion must not hide information that the game expects players to
  reason about.
- Object visibility filters must stay editor/debug tools unless a game
  explicitly authors them as part of presentation.

This means the renderer should be treated as a projection of PuzzleScriptNext
compiled/runtime state:

```txt
PuzzleScriptNext JS runtime state
  -> explicit 3D view snapshot
  -> Three.js scene/instances/materials
```

The reverse direction should be narrow:

```txt
Three.js pick result
  -> cell coordinate / object id for editor or debug UI
```

No gameplay rule should depend on the Three.js object tree, material, camera,
depth buffer, animation mixer, or rendered visibility.

## Reconsidered Decision

The Three.js decision still stands, but for a narrower reason than "voxel
rendering".

The reason to choose Three.js is that it can be used as a thin browser adapter
for a symbolic, discrete, turn-based board while still providing the projection,
depth, instancing, transparency, and picking tools needed for a readable 3D
view.

The decision would change if the first renderer had to be one of these:

- a renderer whose scene graph owns gameplay state
- a dense voxel engine that treats the board as terrain
- a physics-oriented 3D engine where movement is continuous
- a renderer that cannot preserve stable cell coordinates and object identity
- a system where camera visibility becomes the primary way to understand state

Under the current PuzzleScriptNext extraction direction, those are not the
target. Three.js is acceptable because the JS runtime remains deterministic and
grid-owned, while Three.js remains replaceable presentation infrastructure.

## Rejected Initial Directions

### Custom WebGL

Custom WebGL may eventually be useful for specialized rendering, but it is too
low-level for the first renderer. It would force early decisions about shader
pipelines, buffers, picking, and text/icon overlays before the 3D presentation
contract is stable.

### Raymarching Voxel Renderer

Raymarching treats the scene like a dense spatial volume. PuzzleScript state is
sparser and more symbolic: one cell can contain multiple logical objects, and
those objects must remain readable as rule entities. A raymarched volume is a
poor first fit for that model.

### Chunk Meshing / Minecraft-Style Terrain

Chunk meshing is optimized for large terrain-like volumes with many adjacent
solid blocks. PuzzleScript levels are small to medium symbolic boards with
turn-by-turn logical changes. Chunk meshing is likely premature and can obscure
cell/object identity.

### CSS 3D

CSS 3D can prototype a small board, but depth ordering, picking, transparency,
and frequent state updates become awkward. It is not a good implementation
target for the real renderer.

## Camera Modes

The renderer should support both orthographic and finite-distance perspective
projection, but they should have different roles.

### Orthographic Camera

Orthographic projection should be the default play mode.

It behaves like an infinite-distance camera for practical purposes: objects do
not shrink with distance. This preserves the symbolic readability of
PuzzleScript boards and makes grid relationships easier to inspect.

Use it for:

- default play
- puzzle-state inspection
- screenshots intended to communicate logic
- stable camera presets such as isometric or dimetric views

### Perspective Camera

Perspective projection should be available, but conservative.

Use a finite-distance camera when a game or editor preview benefits from depth
feeling, but avoid wide-angle views that distort cell relationships.

Suggested starting values:

- field of view: 25 to 35 degrees
- target: board center
- distance: proportional to the board diagonal
- controls: fixed presets and 90-degree rotations first
- free orbit: editor/debug option, not default play behavior

Perspective mode is a presentation option, not a different game model.

## Visibility Controls

Player-facing visibility controls should be spatial, not object-semantic.

Good first controls:

- all: show the full board
- slice: show one selected spatial slice
- ghost front: make camera-front slices translucent
- rotate: snap the camera to readable preset angles

Object or collision-layer visibility controls should be debug/editor features,
not default play UI. Showing only players, targets, walls, or movable objects
can help diagnose rules, but it decomposes the authored game state and is too
advanced for normal play.

## Scene Contract

The renderer should consume a presentation snapshot rather than runtime internals
or source syntax.

Example shape:

```ts
type Scene3D = {
  size: { x: number; y: number; z: number }
  cells: Array<{
    x: number
    y: number
    z: number
    objects: string[]
  }>
}
```

The PuzzleScriptNext compiler/runtime path owns rules, movement, collision, win
state, undo, and turn order. The renderer owns only presentation:

- mapping object IDs to material/shape/icon choices
- placing visible object representations in 3D cells
- applying camera and spatial visibility controls
- handling view picking when the adapter/editor needs it

Do not infer gameplay behavior from visual adjacency, object shape, stacking, or
rendered grouping.

## Initial Visual Grammar

Start with a restrained symbolic grammar:

- background or floor: low, pale base tile
- solid objects: cube or inset block
- important actors: distinct color plus optional simple marker shape
- multiple objects in one cell: stacked, inset, or billboard overlay treatment
- movement/debug markers: editor/debug overlay only

The renderer should prioritize readable object identity over physical realism.
PBR materials, dramatic shadows, strong perspective, and decorative camera
motion are lower priority than cell readability.

## Implementation Order

1. Build a 3D scene snapshot from compiled/runtime 3D board state.
2. Render occupied cells with Three.js instancing.
3. Add orthographic fixed camera presets.
4. Add object color/material mapping.
5. Add spatial slice and ghost-front display modes.
6. Add optional conservative perspective projection.
7. Add editor/debug picking and object/layer visibility only after play display
   is stable.

## Open Questions

- How should multiple visible objects in the same 3D cell be arranged by
  default?
- Which object metadata should control 3D shape selection, if any?
- Should 3D display reuse 2D sprite colors directly, or define a separate
  renderer metadata surface?
- What is the smallest public view-control API that the editor needs without
  reaching into renderer internals?
