# 3D Slot Design

This document defines the first concrete 3D contract slots. A slot is an
owned config or data boundary handed from the shared upper layer to one lower
layer. The slot name says who is allowed to interpret it.

The design goal is to keep the 3D path close to the existing 2D contracts while
making the true 3D differences explicit.

## Slot Summary

```txt
runtimeInput3d
  compiler
  core
    frame
    directions
    board
    rules
    timers
    lifecycle
  session
  input
    bindings
    repeat
    sources
  renderer
  mutation
  upper
```

Most slots mirror the 2D path. The meaningful 3D differences are:

- `core.frame`: the internal 3D coordinate frame.
- `core.directions`: six logical directions instead of four.
- `core.board`: 3D cell addressing and cell/movement bitmap policy.
- `core.rules`: rule expansion over 3D direction sets and oriented frames.
- `input.bindings`: host keys mapped to six-direction movement intents.

Everything else should stay aligned with the 2D slot contract unless a concrete
2D trace proves that 3D needs a different owner.

## Core Frame Slot

The frame slot defines the internal grid coordinate system. This is not the
renderer camera. It is the runtime's address system for cells and neighbors.

Initial frame:

```js
frame: {
  axes: {
    x: { positive: "right", negative: "left" },
    y: { positive: "down", negative: "up" },
    z: { positive: "back", negative: "front" },
  },
  indexOrder: "z-fastest",
  coordToIndex: "x * height * depth + y * depth + z",
}
```

Consequences:

- 3D `LEVELS` slices are stored as `z` planes.
- Rows inside a slice are `y`.
- Columns inside a row are `x`.
- `front` and `back` move across slices, not screen pixels.
- Renderer world axes may differ later, but that mapping belongs to the
  renderer slot, not the core slot.

The initial direction deltas are:

| Direction | Delta |
| --- | --- |
| `left` | `[-1, 0, 0]` |
| `right` | `[1, 0, 0]` |
| `front` | `[0, 0, -1]` |
| `back` | `[0, 0, 1]` |
| `up` | `[0, -1, 0]` |
| `down` | `[0, 1, 0]` |

This matches the current 3D level lowering helper direction convention, but the
owner should be the 3D runtime module, not the 2D engine.

### Oriented Rule Frames

The core frame above is the fixed storage frame. Rules also need oriented
matching frames. These are not the same thing.

A rule pattern such as:

```txt
[ A | ^B | o C ]
```

is not just a six-direction expansion. It contains relative orientation inside
the rule pattern. In 3D, that means the rule must be expanded over oriented
frames: choose a facing direction, then choose one of the four rotations around
that facing axis.

The standard author-facing rule frame starts from the way a bracket pattern is
drawn on screen:

| Marker | Standard frame direction |
| --- | --- |
| `>` | screen right, game-world `right` |
| `<` | screen left, game-world `left` |
| `^` | screen up, game-world `front` |
| `v` | screen down, game-world `back` |
| `o` | out of the screen, game-world `up` |
| `x` | into the screen, game-world `down` |

The 24-frame expansion rotates this standard frame. In other words, `>`, `^`,
and `o` are directions relative to the standard frame, not fixed global
directions after expansion.

Contract:

```js
frame.ruleFrames = {
  expansion: "proper-orthogonal-frames",
  count: 24,
  includeReflections: false,
}
```

That gives `6 * 4 = 24` rule frames. It intentionally does not include mirrored
frames, so it is not a 48-direction expansion.

Each oriented rule frame should provide a local basis:

```js
{
  screenRight,
  screenUp,
  screenOut,
  screenLeft: -screenRight,
  screenDown: -screenUp,
  screenIn: -screenOut,
}
```

Relative markers in rules are interpreted through this local basis. Absolute
direction names such as `front` or `up` remain tied to the fixed storage frame.

Relative marker mapping:

| Marker | Local frame meaning |
| --- | --- |
| `>` | screenRight |
| `<` | screenLeft |
| `^` | screenUp |
| `v` | screenDown |
| `o` | screenOut |
| `x` | screenIn |

In this context, `o` and `x` are relative screen-depth markers. They are not
glyph object names and they are not fixed global `up` / `down` directions unless
the current oriented frame maps `screenOut` / `screenIn` to the storage `up` /
`down` axis. Their concrete delta is produced by the selected rule frame.

## Core Directions Slot

The directions slot defines logical movement names and direction sets used by
rule expansion.

```js
directions: {
  absolute: ["left", "right", "front", "back", "up", "down"],
  aggregates: {
    horizontal: ["left", "right"],
    depth: ["front", "back"],
    vertical: ["up", "down"],
    planar: ["left", "right", "front", "back"],
    orthogonal: ["left", "right", "front", "back", "up", "down"],
  }
}
```

Relative symbols such as `^`, `v`, `<`, `>`, `o`, and `x` should not be treated
as fixed aliases for absolute 3D directions. They belong to the oriented
rule-frame expansion above.

The important point is that rule expansion gets a direction set from this slot.
It should not hard-code the 2D `up/down/left/right` aggregate tables.

## Core Board Slot

The board slot is the runtime-owned view of a compiled `state.levels3[n]`.

```js
board: {
  width,
  height,
  depth,
  cellCount: width * height * depth,
  layerCount,
  strideObj,
  strideMov,
  movementBits,
  movementMask,
  cells,
  movements,
  layerMasks,
  objectLayers,
  background,
  getCell(index),
  getCellInto(index, target),
  setCell(index, mask),
  getMovements(index),
  getMovementsInto(index, target),
  setMovements(index, mask),
  neighbor(index, direction),
  coordToIndex(x, y, z),
  indexToCoord(index),
  clone(),
}
```

The compiler-produced `state.levels3[n]` may keep using the existing object-mask
bitmap shape as its lowered transport format. The 3D runtime should wrap or
copy that into its own board object at construction time.

Cell occupancy and movement are separate concerns:

- Cell occupancy can initially reuse the compiled object bitmap shape.
- Cell reads follow the 2D `Level` contract: `getCell` returns a copy-like cell,
  `getCellInto` copies into a caller-owned target, and only `setCell` mutates
  board storage.
- Movement reads follow the same 2D `Level` contract: `getMovements` returns a
  copy-like movement mask, `getMovementsInto` copies into a caller-owned target,
  and only `setMovements` mutates movement storage.
- Cell matching is independently implemented in the 3D path, but its tests
  pin the 2D `CellPattern` semantics: all `objectsPresent` bits must be set, no
  `objectsMissing` bits may be set, each `anyObjectsPresent` mask must have at
  least one bit in the cell, all `movementsPresent` bits must be set, and no
  `movementsMissing` bits may be set.
- Movement bitmap constants preserve the 2D prefix:
  `up=1`, `down=2`, `left=4`, `right=8`, and `action=16`. 3D appends only
  `front=32` and `back=64`, so the default 3D shape is `movementBits = 7`,
  `movementMask = 0x7f`, and
  `strideMov = ceil(layerCount * movementBits / 32)` unless the compiler
  supplies explicit values.
- `layerMasks` and `objectLayers` belong in the board slot because movement
  resolution is layer-based. A 3D board cannot resolve movement correctly
  from object occupancy alone.

That separation keeps the low-level object masks familiar while preventing the
2D movement mask table from becoming the accidental 3D core.

## Core Rules Slot

The rules slot owns lowered rule data after direction expansion.

```js
rules: {
  directionSet,
  ruleFrames,
  groups,
  lateGroups,
  winConditions,
  unsupportedFeatures,
}
```

For the first runtime milestone, this slot can be intentionally narrow:

- absolute six-direction rules
- relative rule-frame expansion over 24 proper orientations where relative
  markers are admitted
- PS-compatible cell matching through `objectsPresent`, `objectsMissing`, and
  `anyObjectsPresent`
- basic replacement patterns
- movement-mask replacement and movement resolution
- basic win conditions

Unsupported 2D rule features should still be detected before play starts. The
slot exists so those features can later be admitted without moving ownership.

## Input Slot

The input slot maps host input to runtime movement intents. It does not define
the board frame; it only says which logical direction the player asked for.

Initial keyboard policy:

```js
input: {
  bindings: {
    keyboard: {
      keyToIntent: {
        w: "front",
        a: "left",
        s: "back",
        d: "right",
      },
      unboundIntents: ["up", "down"],
    }
  },
  repeat: {
    throttle,
    repeatMs,
  },
  sources: {
    keyboard: { enabled },
    action: { enabled, noRepeat },
    mouse: { supported: false },
  }
}
```

`up` and `down` are optional movement intents and should be unbound by default
until the host/UI decides which keys or controls should emit them. The runtime
must still support the logical directions from the start.

This keeps the early keyboard contract simple:

- `W/A/S/D` cover the horizontal 3D plane: `front/left/back/right`.
- No source-level syntax is needed to choose up/down keys yet.
- A future host can bind up/down without changing the core movement contract.

## Renderer Slot

The renderer slot receives runtime state plus presentation settings. It maps the
core frame into a camera/world frame, but it does not alter core coordinates.

```js
renderer: {
  frameMapping,
  viewport,
  camera,
  tween,
  palette,
  assets,
  text,
}
```

Renderer work can remain last. The only early requirement is that the contract
does not force renderer concepts into core state.

## Mutation Slot

Runtime metadata twiddling remains a separate slot even if 3D rejects it
initially.

```js
mutation: {
  supported: false,
  dispatch(settingName, value) {
    // core timer, session policy, input binding, renderer camera/palette, etc.
  }
}
```

The key contract is ownership: a twiddled setting must be dispatched to its
owning slot. For example, a future camera setting goes to `renderer`, while
`require_player_movement` goes to `core.lifecycle`.

## First Construction Target

The next concrete implementation should construct this shape from
`state.levels3[0]`:

```js
const runtime = create3DRuntime({
  level: state.levels3[0],
  metadata: metadata3d,
  rules: rules3d,
});
```

Minimum acceptance:

- preserves `width`, `height`, `depth`, and `cellCount`
- exposes frame-aware `coordToIndex`, `indexToCoord`, and `neighbor`
- exposes `getCell`, `setCell`, and `clone`
- accepts movement intents for all six logical directions
- leaves `up/down` unbound in default keyboard input
- rejects unsupported rules/settings before the first turn
