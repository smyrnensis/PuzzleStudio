# puzzle3D canonical syntax draft

This is the authoring syntax target for the standalone 3D model prototype.
It should stay parallel to the current 2D canonical `.puzzle` syntax unless 3D
requires an explicit difference.

## Scope

This document fixes the minimal 3D surface syntax before adding a parser.
It is not a complete language reference.

The 3D-specific syntax should be as small as possible. The main new surface is
the level body: blank-line separated height slices.

## Objects and layers

Canonical 3D syntax should not require explicit object declarations.

Use the existing canonical layer style:

```txt
layers {
  floor = Goal
  actor = Player Box Wall
}
```

Objects are discovered through canonical authoring constructs such as `layers`
and `legend`, as in the 2D language direction. Avoid this non-canonical form:

```txt
objects {
  Player actor
  Box actor
}
```

## Directions

3D has built-in direction value sets. Authors use the sets, but do not declare
them.

```txt
for d in directions {
  ...
}

for h in horizontal {
  ...
}

for v in vertical {
  ...
}
```

Built-in 3D direction sets:

```txt
directions = up down left right forward backward
horizontal = left right forward backward
vertical = up down
```

Coordinate convention:

```txt
left     = (-1,  0,  0)
right    = ( 1,  0,  0)
forward  = ( 0,  1,  0)
backward = ( 0, -1,  0)
up       = ( 0,  0,  1)
down     = ( 0,  0, -1)
```

`up` and `down` are fixed to the height axis. Camera angle and screen projection
do not redefine directions.

The standard authoring frame is `right:backward:down`. In ordinary 3D text
blocks, columns move to the right, rows written lower in the text move backward,
and blank-line-separated blocks written lower in the text move down.

If more precision is needed later, `lateral = left right` and
`depth = forward backward` can be added as built-in sets. They are not part of
the minimal syntax.

## Levels

3D level size is derived from the body. Authors should not write explicit size
numbers.

Use blank lines to separate height slices:

```txt
levels3 {
  level test {
    .....
    .P.B.
    .....

    .....
    .....
    ..G..
  }
}
```

Lowering convention:

- Each non-empty block is one height slice.
- Slice order follows the standard `down` axis: the first block is the top
  slice and the last block is the bottom slice.
- Row order follows the standard `backward` axis: the first row is the forward
  row and later rows move backward.
- Character index inside a row gives `x`.
- `width` is the row width.
- `depth` is the number of rows per slice.
- `height` is the number of slices.

For the example above:

```txt
P -> (x=1, y=1, z=1)
B -> (x=3, y=1, z=1)
G -> (x=2, y=0, z=0)
size = width 5, depth 3, height 2
```

Validation:

- A level must contain at least one height slice.
- Each slice must contain at least one row.
- Rows must be rectangular within a slice.
- All slices must have the same width and depth.
- Every non-empty character must resolve through `legend`.
- The empty character is handled by the same canonical empty-cell mechanism as
  2D.

## Regions

Normal 3D level bodies reserve blank lines for height slice separation.

If 3D regions are added later, region syntax should be explicit on the region
side rather than changing the meaning of ordinary blank lines.

Possible future shape:

```txt
levels3 {
  level test {
    region room_a {
      .....
      .P.B.
      .....

      .....
      ..G..
      .....
    }

    region room_b {
      ...
    }
  }
}
```

This is only a future direction. Region syntax is not part of the minimal 3D
canonical syntax.

## Render settings

3D render settings belong to the 3D model, not to scene layout or puzzle rules.
Use a `render` block and keep camera settings inside `camera`:

```txt
model puzzle3 push3d {
  render {
    camera {
      yaw 34
      pitch 38
      zoom 1.1
      interactive_look true
      interactive_zoom true
    }
    grid {
      occupied_cells true
    }
    shade true
  }
}
```

`yaw`, `pitch`, and `zoom` are the initial camera view. `interactive_look`
allows pointer drag to change the view direction by changing yaw/pitch.
`interactive_zoom` allows wheel/pinch-style zoom changes.

`interactive_look` is not a semantic input. The parent scene should not treat
click/drag as a special 3D camera command. Raw pointer input is delivered
through the normal focused-scene, layout, and hit-test path. A `puzzle3`
component may capture a pointer drag that starts inside its own display box,
and if `interactive_look` is true, it interprets that gesture as camera
yaw/pitch view-state updates. The drag remains owned by that component until
pointer release/cancel, even if the pointer leaves the component box.

These settings affect only renderer view state; puzzle rules, levels, win
conditions, undo, restart, and transition state must not read them.

`grid { occupied_cells true }` shows exterior edges of cells that contain
objects. It is a renderer/debug readability setting, not a floor, volume, level
object, collision rule, or gameplay fact. Omitted grid settings default to off.

`render { shade false }` disables per-face light shading for 3D sprite voxels.
It is a renderer readability setting and does not change sprite voxel data,
rules, collision, or win conditions. Omitted sprite shade defaults to on.

Legacy top-level `debug_camera`, `camera_yaw`, `camera_pitch`, and
`camera_zoom` are compatibility syntax and should not be used in new examples.

## Raw input and component inputs

The game receives one raw input stream: keys, buttons, pointer events, and
similar device events. Raw input first enters the focused scene. The focused
scene decides which component routines run; by default, interactive components
in that scene receive the raw input context.

Scene syntax is shared with the 2D language. A 3D scene is not a separate scene
kind; it is an ordinary scene that contains a `puzzle3` model window component.
The root scene size is written with the same layout header as 2D:

```txt
scene playing3d {
  state {
    board = puzzle3 push3d
  }

  view size 720 540 {
    column gap 12 align center top {
      puzzle3 board
      row gap 8 {
        button "Restart" -> board.restart
        button "Levels" -> goto level_select
      }
    }
  }
}
```

The shared scene/layout keywords are `view`, `row`, `column`, and `box`; they all
accept `size <w> <h>`, `gap <n>`, and `align <x> [y]` header attributes. Generic
component keywords such as `title`, `subtitle`, `text`, `button`, `row`,
`column`, `box`, `for`, `level_menu`, and `menu` have the same meaning for 2D
and 3D scenes. The model-specific leaf is `puzzle3 <slot>`.

Mapping raw keys to named inputs is owned by the component, not by the scene.
The component-local block is plural:

```txt
scene playing {
  state {
    board = puzzle3 push3d
  }

  rules {
    board.rules
  }

  view {
    puzzle3 board {
      inputs {
        forward <- w ArrowUp
        backward <- s ArrowDown
        left <- a ArrowLeft
        right <- d ArrowRight
      }
    }
  }
}
```

`board.rules` means "step the `board` component with the current raw input
context". The component then interprets that raw input through its `inputs`
block and calls the model with the resulting local input.

Key tokens in source use the shared authoring tokens such as `w`, `ArrowUp`,
`Escape`, `Enter`, and `Space`. They lower to standard browser key/code values
in the visual fixture/runtime.

Most model components have default input adapters, so ordinary WASD/arrow
movement does not need to be written. For example, a 3D puzzle component reads
`w` / `ArrowUp` as `forward`, while a 2D puzzle component reads the same raw keys
as `up`.

Undo and restart are not component inputs. They are stronger play/session
operations owned by the parent runtime or scene.

Prototype-only scene `keys { ... }` blocks are compatibility syntax and should
not appear in new canonical examples. The migration target is
`inputs { <input> <- <key...> }`, with model-specific interpretation owned by the
`puzzle3` component or 3D model runtime.

## Minimal example

```txt
model puzzle3 push3d {
  layers {
    floor = Goal
    solid = Player Box Wall
  }

  rules {
    for d in horizontal {
      if input == d {
        once d [ Player | Box | no solid ] -> [ | Player | Box ]
        once d [ Player | no solid ] -> [ | Player ]
      }
    }
  }
}

levels3 basic of push3d {
  legend {
    . = empty
    P = Player
    B = Box
    # = Wall
    G = Goal
  }

  level push3d_01 {
    .....
    .P.B.
    .....

    .....
    .....
    ..G..
  }
}

sprites3 basic of push3d {
  Floor
  #90ee90 #008000
  .....
  .....
  .....
  .....
  .....

  .....
  .....
  .....
  .....
  .....

  .....
  .....
  .....
  .....
  .....

  .....
  .....
  .....
  .....
  .....

  11111
  01111
  11101
  11111
  10111
}
```

## Parser prototype target

The parser prototype lowers the independent 3D surface into:

- `Game3`
- `SelectorCatalog3`
- `Vec<Rule3>`
- optional `LevelBundle3`
- optional `WinCondition3`

`sprites3` uses the same canonical surface shape as 2D sprites: a sprite name
line, a whitespace-separated palette row, then the voxel bitmap. Palette entries
map to `0`, `1`, `2`, ... in order. Blank lines separate voxel slices.
