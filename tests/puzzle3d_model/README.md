# puzzle3D model test crate

This standalone crate tests the 3D puzzle-core shape before merging anything
into the main crates.

See `CANONICAL_SYNTAX.md` for the draft 3D `.puzzle3` authoring syntax.

## Core conventions

- Coordinates are absolute grid coordinates: `(x, y, z)`.
- `z` is the height axis.
- `up = (0, 0, 1)` and `down = (0, 0, -1)`.
- The horizontal plane is X/Y:
  - `left = (-1, 0, 0)`
  - `right = (1, 0, 0)`
  - `forward = (0, 1, 0)`
  - `backward = (0, -1, 0)`
- The standard authoring text frame is `right:backward:down`. Text columns move
  right, later rows move backward, and later blank-line-separated blocks move
  down.
- Camera angle and screen projection must not redefine core directions.

## Responsibility split

- `State3`, `Patch3`, and `transition_*` own deterministic puzzle state
  changes.
- `Game3` owns definition validation for layers, objects, and semantic inputs.
- `Level3` is initial board data. It builds a `State3`, but it is not runtime
  state and does not own play/session behavior.
- Semantic input is a transition context value. It is not part of canonical
  state.
- `BoardSnapshot3` is the core-to-display handoff: a discrete list of occupied
  cells and object ids.
- Sprite names, palettes, and model-owned `render` metadata are display-facing.
  Camera settings are parsed with the model but interpreted by visual adapters.

## 3D input remapping policy

The core model should receive absolute semantic directions:

```txt
left / right / forward / backward / up / down
```

Physical keys should not be baked directly into those absolute directions for
camera-driven 3D play. A key press should first become a screen intent, then
the 3D board/view adapter should map that intent through the current camera
yaw into the nearest absolute horizontal core input.

```txt
physical key
  -> screen intent
  -> camera-relative board/view adapter
  -> absolute core input
```

Default horizontal controls should be camera-relative:

```txt
W / ArrowUp     -> screen_forward
S / ArrowDown   -> screen_backward
A / ArrowLeft   -> screen_left
D / ArrowRight  -> screen_right
```

The adapter maps those four screen intents to `left` / `right` / `forward` /
`backward` based on the current camera yaw. The model and rules still only see
the absolute semantic direction.

Vertical movement should stay world-absolute because screen-up is ambiguous in
3D grid play:

```txt
Space / E -> up
Shift / Q -> down
```

Longer-term ownership:

- model: defines the absolute input vocabulary.
- scene: routes keys to the focused component and owns scene-wide shortcuts.
- component/view adapter: owns camera-relative interpretation because it owns
  camera state.
