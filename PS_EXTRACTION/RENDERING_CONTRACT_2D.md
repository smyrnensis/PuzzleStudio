# 2D PuzzleScript Rendering Contract

This note records the 2D PuzzleScript rendering conventions that should guide
the engine-to-renderer contract for both 2D and future 3D presentation work.

## Sources Checked

Official PuzzleScript documentation:

- https://www.puzzlescript.net/Documentation/objects.html
- https://www.puzzlescript.net/Documentation/collisionlayers.html
- https://www.puzzlescript.net/Documentation/prelude.html

PuzzleScript Next documentation:

- https://mansoft.nl/puzzlescriptnext/Documentation/collisionlayers.html

Local implementation references:

- `upstream/PuzzleScriptNext/src/js/graphics.js`

## Rendering Rules From PuzzleScript

### Objects Own Visual Definitions

In classic PuzzleScript, an object can be rendered as a single color square or
as a sprite grid. The usual sprite is a 5x5 pixel pattern. Dots are transparent,
and numeric characters index the object's color row.

Rendering implication:

- object identity must be preserved in the render contract
- sprite data belongs to object metadata, not to runtime cell storage
- transparent pixels are part of the object visual, not gameplay state
- a renderer may use a fallback visual only when no sprite metadata exists

### Cells Contain Object Sets

A level glyph can expand to multiple objects through the legend, for example a
crate on a target. Rendering therefore cannot assume one sprite per cell.

Rendering implication:

- each visible cell must expose all non-empty object occupants
- occupants must include their collision layer
- renderer-side composition must stack or otherwise combine occupants in one
  cell without changing game state

### Collision Layer Order Is Draw Order

Official PuzzleScript says collision layer order also determines draw order.
PuzzleScript Next makes this more explicit: objects are drawn back-to-front, so
later layers can hide earlier ones.

Rendering implication:

- layer index is part of the render contract
- default in-cell draw order is ascending layer index
- background/floor must be below ordinary objects
- draw order must not feed back into collision semantics

### Background Layer Is Special

Every game must have a background layer, and every tile must have a background
tile. The engine/compiler may infer a background object for cells that do not
explicitly specify one.

Rendering implication:

- renderer should not invent gameplay background objects
- the snapshot should include the background occupant when it exists in state
- renderer may still have a presentation floor color, but that is only a visual
  fallback and not a state occupant

### Viewport Is Presentation

PuzzleScript prelude options such as `flickscreen` and `zoomscreen` determine
which part of the board is visible. They do not alter the board itself.

Rendering implication:

- viewport belongs in render/screen settings
- board state remains full-size
- renderer receives enough data to frame the view around a focus object or
  explicit viewport

### Effects And Debug Overlays Are Separate

Move tweening, rule animation, visual debugger arrows, grid overlays, and editor
palette highlights are presentation/debug overlays. They inspect state changes
or rule traces but are not board occupants.

Rendering implication:

- animation events should be separate from cell occupants
- movement/debug markers should not be stored as ordinary objects unless the
  author intentionally modeled them as objects
- debug visibility controls are outside the default play contract

## Local Implementation Observations

### PuzzleScript Next Canvas Renderer

The local upstream renderer builds object sprite canvases from compiled objects.
It draws object IDs in collision-layer-group order, then by cell traversal, then
by object/layer order. It also supports sprite offsets, vector sprites, text
sprites, custom cell sizes, scanline effects, smooth/flick/zoom screen framing,
move tweening, and debug overlays.

Important details for the contract:

- object visuals are regenerated from `state.objects`
- cell contents are read from runtime board storage
- `obj.layer` is used for layer filtering and move tween lookup
- movement animation is based on movement/change data, not on renderer-owned
  physics
- viewport and camera offsets are presentation calculations

## Proposed Engine-To-Renderer Contract

The PuzzleScript JS runtime should expose a render snapshot. It should be a
stable presentation-facing contract, not raw internal `Level` storage or an
adapter-specific serialization shape.

The contract should be based on the actual PuzzleScript Next renderer inputs,
not on a generic grid renderer shape. The important source structures are:

- `state.objects`: object visual metadata and compiler-assigned `id` / `layer`
- `state.idDict`: object ID to object name
- `state.collisionLayerGroups`: grouped draw order and cell traversal order
- `curLevel`: board storage exposed through `getCell` / `getCellInto`
- `state.metadata`: viewport, sprite size, palette, tween, and screen settings
- animation globals such as `currentMovedEntities` and `seedsToAnimate`

The renderer should not need to inspect compiler internals directly. A small JS
adapter should translate those structures into an explicit render frame.

```txt
PuzzleScriptNext state + current Level + render globals
  -> buildPuzzleRenderFrame2D(...)
  -> renderer
```

## Concrete PuzzleScript Next Contract

Use a frame object, not a persistent game model object, as the renderer input.
The frame is a snapshot for one draw pass.

```ts
type PuzzleRenderFrame2D = {
  model: "psnext-grid2"
  size: {
    width: number
    height: number
    layerCount: number
  }
  spriteGrid: {
    width: number
    height: number
    pixelSize?: number
  }
  objects: RenderObject2D[]
  drawPlan: RenderDrawGroup2D[]
  cells: RenderCell2D[]
  viewport: RenderViewport2D
  palette: RenderPalette2D
  overlays?: RenderOverlays2D
  animation?: RenderAnimationState2D
}

type RenderObject2D = {
  id: number
  name: string
  layer: number
  visual: RenderObjectVisual2D
}

type RenderObjectVisual2D =
  | {
      kind: "matrix"
      colors: string[]
      matrix: number[][]
      offset: { x: number; y: number }
      scale?: number
    }
  | {
      kind: "text"
      colors: string[]
      text: string
      offset: { x: number; y: number }
      scale?: number
    }
  | {
      kind: "vector"
      vectorType: "canvas" | "svg"
      data: unknown
      widthCells?: number
      heightCells?: number
      angle?: number
      flipX?: boolean
      flipY?: boolean
      offset: { x: number; y: number }
    }

type RenderDrawGroup2D = {
  firstObjectId: number
  objectCount: number
  dirFirst: "left" | "right" | "up" | "down"
  dirSecond: "left" | "right" | "up" | "down"
}

type RenderCell2D = {
  index: number
  x: number
  y: number
  objectIds: number[]
}
```

`drawPlan` is required because PuzzleScript Next does not only sort objects by
layer. Its renderer iterates `state.collisionLayerGroups`; each group carries
both an object ID range and a two-axis cell traversal order. A faithful contract
must preserve that plan.

The renderer then draws like this:

```ts
for (const group of frame.drawPlan) {
  for (const index of positionIndexesForGroup(group, frame.viewport, frame.size)) {
    const cell = frame.cells[index]
    for (
      let objectId = group.firstObjectId;
      objectId < group.firstObjectId + group.objectCount;
      objectId++
    ) {
      if (cell.objectIds.includes(objectId)) {
        drawObject(frame.objects[objectId], cell, frame)
      }
    }
  }
}
```

This mirrors `graphics.js`: group first, position traversal second, object ID
inside the group third.

## Frame Builder

The contract should be built by a JS adapter near the PuzzleScript Next runtime,
for example `render_frame2d.js`.

```js
function buildPuzzleRenderFrame2D(state, level, renderState = {}) {
  return {
    model: "psnext-grid2",
    size: {
      width: level.width,
      height: level.height,
      layerCount: level.layerCount,
    },
    spriteGrid: {
      width: state.sprite_size || 5,
      height: state.cell_height || state.sprite_size || 5,
    },
    objects: buildRenderObjects2D(state),
    drawPlan: buildDrawPlan2D(state),
    cells: buildRenderCells2D(state, level),
    viewport: buildViewport2D(state, level, renderState),
    palette: buildRenderPalette2D(state),
    overlays: buildRenderOverlays2D(state, renderState),
    animation: buildAnimationState2D(state, renderState),
  };
}
```

The builder may use `Level.getCellInto` and bit vectors internally, but the
renderer should receive plain object IDs, not bit vectors.

## Viewport Contract

PuzzleScript Next viewport behavior is presentation-owned but state-dependent.
The frame should expose a resolved viewport rather than asking the renderer to
reinterpret all metadata.

```ts
type RenderViewport2D = {
  x: number
  y: number
  width: number
  height: number
  mode: "full" | "flickscreen" | "zoomscreen" | "smoothscreen" | "editor"
  cameraOffset?: { x: number; y: number }
  renderBorderSize?: number
  clip?: boolean
}
```

The frame builder owns the translation from:

- `state.metadata.flickscreen`
- `state.metadata.zoomscreen`
- `state.metadata.smoothscreen`
- player position
- old flickscreen data
- smooth camera position/target

into the resolved viewport fields above.

## Animation Contract

PuzzleScript Next rendering uses animation data that is separate from board
occupants:

- move tweening reads `currentMovedEntities["p<pos>-l<layer>"]`
- AFX animation reads `seedsToAnimate`
- create/destroy/cantmove information is collected from SFX artifacts

The contract should keep this separate:

```ts
type RenderAnimationState2D = {
  moveTween?: {
    progress: number
    snap: number
    movedLayers: Array<{
      index: number
      layer: number
      directionMask: number
    }>
  }
  objectAnimations?: Array<{
    index: number
    objectId: number
    kind: "move" | "cant" | "hit" | "create" | "destroy"
    seed: string
    directionMask?: number
  }>
}
```

Animation may change draw position, alpha, scale, or rotation. It must not
change `cells[].objectIds`.

## Contract Rules

The JS compiler/runtime side must provide:

- board dimensions
- object definitions with stable IDs, names, layers, and PS Next visual data
- `collisionLayerGroups` lowered into an explicit draw plan
- explicit cell occupants by coordinate and object ID
- resolved viewport fields derived from metadata and render state
- animation/debug events as separate presentation artifacts

The renderer must provide:

- sprite realization from the visual data in `objects`
- PS Next draw-plan execution
- viewport framing
- pixel/canvas/WebGL realization
- optional animation and debug overlays
- picking back to coordinates/object IDs when needed by editor/debug UI

The renderer must not provide:

- collision semantics
- inferred adjacency semantics
- rule execution meaning
- hidden gameplay state through animation state
- implicit object occupants based on missing visuals

## 3D Consequence

The 3D contract should preserve the same split:

```txt
PuzzleScript JS runtime state
  -> render snapshot with object IDs, layers, coordinates, viewport intent
  -> renderer-specific realization
```

The 3D version should add only spatial fields:

- `model: "grid3"`
- `size: { width, height, depth }`
- cell coordinate `{ x, y, z }`
- a 3D draw plan that replaces 2D cell traversal with camera/view-dependent
  back-to-front traversal
- 3D viewport/camera intent
- optional spatial visibility mode such as all/slice/ghost-front

It should not replace object/layer occupants with raw voxel geometry. Voxel
geometry belongs to `RenderVisualDef` or a renderer asset table. The board
snapshot remains a symbolic PuzzleScript state snapshot.

For 3D, the key extension point is the draw plan:

```ts
type RenderDrawPlan3D = {
  objectGroups: Array<{
    firstObjectId: number
    objectCount: number
  }>
  cellOrder: number[]
}
```

In 2D, PuzzleScript Next can encode draw traversal through
`collisionLayerGroups.dirFirst` and `dirSecond`. In 3D, traversal depends on the
current projection/camera preset and visibility mode. The renderer should still
receive an explicit order or enough declarative camera information to derive
one deterministically.
