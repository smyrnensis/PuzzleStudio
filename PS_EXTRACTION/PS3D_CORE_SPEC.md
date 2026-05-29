# PS3D Core Spec Draft

This document fixes the smallest PS3D surface that the current extraction work
should support first. It is intentionally narrower than the long-term 3D plan.

Status: draft for the next implementation pass. This is not yet a complete
playable language contract.

## Scope

The first PS3D core is a PuzzleScript Next extension mode for simple
grid-replacement games in 3D.

It should prove one thing before anything else: a PuzzleScript-style author can
write a small 3D Sokoban in a single source file without learning the wider
Puzzle Studio `.puzzle` language.

Core includes:

- ordinary PuzzleScript-style `OBJECTS`, `LEGEND`, `COLLISIONLAYERS`, `RULES`,
  `WINCONDITIONS`
- `LEVELS3`
- 3D dimensions: `width`, `height`, `depth`
- six semantic directions: `left`, `right`, `front`, `back`, `up`, `down`
- explicit unsupported-feature errors for PS Next features outside the MVP

Core does not include:

- dense 3D rule patterns
- frame syntax
- tags and mappings in 3D mode
- random, late, rigid, gosub, checkpoint, solver, GIF, sound, metadata
  twiddling, links, or level-select behavior in 3D mode
- 3D sprite editor, visual level editor, shadows, tweening, or advanced camera
  behavior

## Mode Marker

`LEVELS3` is the first explicit 3D mode marker.

`LEVELS` keeps existing PuzzleScript Next behavior. Blank lines in normal
`LEVELS` remain level separators and must not be reinterpreted as 3D slice
separators.

The compiler may treat a game containing `LEVELS3` as 3D only for the 3D runtime
path. Existing 2D games must continue to compile and play through the existing
2D path.

## LEVELS3

`LEVELS3` contains one or more anonymous 3D level grids.

Within one 3D level:

- each non-empty row is a map row
- a line containing only `;` separates depth slices
- `;` inside a map row is an ordinary glyph
- all slices in a level must have the same row count
- all rows in all slices must have the same width

Between 3D levels:

- a blank line separates levels
- `LEVEL`, `SECTION`, `TITLE`, `MESSAGE`, `LINK`, and `INPUT` commands are not
  part of the first core pass

Example:

```txt
=======
LEVELS3
=======

#####
#P B#
#####
;
.....
..G..
.....
```

The example has:

- `width = 5`
- `height = 3`
- `depth = 2`

## Glyphs And Background

`LEVELS3` uses the same glyph resolution rule as PuzzleScript Next levels:

- every glyph used in a map must be defined by `OBJECTS` or `LEGEND`
- case sensitivity follows the existing `case_sensitive` prelude behavior
- property glyphs defined with `or` are ambiguous in maps and must be rejected
- aggregate glyphs that resolve to concrete object masks may be supported only
  when they follow the existing 2D lowering rule

`.` is not a magic empty cell in the implementation path. It is legal only when
the game defines it, usually as:

```txt
. = Background
```

Missing background-layer content is filled by the same principle as 2D
PuzzleScript Next: use the first explicit background cell in the level when
available, otherwise use the default background object.

## Coordinate Order

The 3D board exposes coordinates as:

```txt
x: left/right axis
y: row axis
z: slice/depth axis
```

The current extraction helper indexes cells as:

```txt
index = x * height * depth + y * depth + z
```

This is an implementation detail for the current checkout, but tests should lock
round-trip behavior until the runtime boundary is deliberately changed.

## Directions

Core direction names:

```txt
left right front back up down
```

Recommended direction sets:

```txt
horizontal = left right front back
vertical = up down
directions = left right front back up down
```

MVP gameplay should begin with basic directional replacement only. Advanced
relative markers and local frames are not part of core.

## Rules

The first playable target is ordinary PuzzleScript-style directional rules that
can express 3D Sokoban movement and pushing.

Target example:

```txt
[ > Player | Box ] -> [ > Player | > Box ]
[ > Player | ] -> [ | Player ]
```

The `>` marker in this example means the current expanded direction. In 3D mode,
expansion should eventually include all six core directions when the rule uses a
direction set that asks for all directions.

Until engine support exists, the compiler should avoid silently pretending that
2D rule execution handles 3D. Unsupported 3D rule features should fail with a
source-facing diagnostic.

## Runtime Boundary

The next implementation step is not a renderer. It is a runtime boundary check:

- a compiled `LEVELS3` level reaches the play/session layer intentionally
- the runtime either handles it through a named 3D path or refuses it explicitly
- no 3D level is accidentally treated as a 2D `width x height` board

Undo, restart, level advance, win checks, and rendering may be minimal in the
first pass, but they must not corrupt or reinterpret 3D state as 2D state.

## Reference Example

The current fixture is:

- `examples/sokoban3d.txt`

It is a target behavior fixture, not confirmed playable PS Next syntax yet.

