# 3D Core Spec Draft

This document fixes the 3D surface for the current extraction work.

Status: draft for the next implementation pass. This is not yet a complete
playable language contract.

## Scope

The 3D core is PuzzleScript with two extra spatial directions.

2D and 3D are the same language and must be implemented in the same form.
Differences are allowed only when they are forced by space:

- `depth`
- `front` / `back`
- 3D coordinate/index/neighbor logic
- 3D rule-frame orientation where spatial pattern interpretation requires it
- 3D rendering

Everything else follows the 2D PuzzleScript contract exactly. Missing support
is an implementation gap, not a design difference.

The 3D surface includes ordinary PuzzleScript-style:

- `OBJECTS`
- `LEGEND`
- `COLLISIONLAYERS`
- `RULES`
- `WINCONDITIONS`
- `SOUNDS`
- tags and mappings where supported by the 2D language
- ordinary commands and session semantics
- `three_dimensions` as the author-facing 3D mode marker
- ordinary `LEVELS` containing 3D slice separators in 3D mode

## Mode Marker

Preferred author-facing syntax uses a prelude flag:

```txt
three_dimensions
```

The section name remains ordinary PuzzleScript `LEVELS`. The flag, not a new
level section name, means that rules, directions, levels, runtime routing, and
diagnostics are interpreted as 3D.

`LEVELS` keeps existing PuzzleScript Next behavior when `three_dimensions` is
absent. Blank lines in normal 2D `LEVELS` remain level separators and must not
be reinterpreted as 3D slice separators.

`LEVELS3` is not an accepted author-facing section. Internal names such as
`levels3` are acceptable as implementation transport names only. Docs, examples,
and parser entry points should use `three_dimensions` plus `LEVELS`.

Existing 2D games must continue to compile and play through the existing 2D
path.

## 3D Levels

In `three_dimensions` mode, `LEVELS` contains one or more anonymous 3D level
grids.

Within one 3D level:

- each non-empty row is a map row
- a line containing only `;` separates depth slices
- `;` inside a map row is an ordinary glyph
- all slices in a level must have the same row count
- all rows in all slices must have the same width

Between 3D levels:

- a blank line separates levels
- `LEVEL`, `SECTION`, `TITLE`, `MESSAGE`, `LINK`, and `INPUT` commands are not
  accepted only when their 2D semantics have been mapped to the same shared
  session contract

Example:

```txt
=======
LEVELS
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

3D `LEVELS` uses the same glyph resolution rule as PuzzleScript Next levels:

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

Relative markers and local frames belong to the spatial rule-frame contract,
not to the renderer.

## Rules

3D rules use the same PuzzleScript rule semantics as 2D rules. The only
extensions are spatial: more directions and 3D pattern interpretation. The
lower core should not expose a separate high-level "push" primitive.

Target example:

```txt
[ > Player | Box ] -> [ > Player | > Box ]
[ > Player | ] -> [ | Player ]
```

The `>` marker in this example means the current expanded direction. In 3D mode,
expansion includes all six core directions when the rule uses a direction set
that asks for all directions.

Until a shared semantic contract exists for a feature, the compiler/runtime
should avoid silently pretending that 3D handles it. Diagnostics must describe
the gap as missing implementation, not as 3D semantics differing from 2D.

## Runtime Boundary

The next implementation step is not a renderer. It is a runtime boundary check:

- a compiled 3D level reaches the play/session layer intentionally
- the runtime either handles it through a named 3D path or refuses it explicitly
- no 3D level is accidentally treated as a 2D `width x height` board

Undo, restart, level advance, win checks, and rendering may be minimal in the
first pass, but they must not corrupt or reinterpret 3D state as 2D state.

Non-spatial PuzzleScript features such as `late`, commands, `again`, `restart`,
`checkpoint`, `gosub`, loops, random choices, tags, mappings, win conditions,
sound, and metadata handling must not be given 3D-specific semantics. They
belong to the same shared contract as 2D.

## Reference Example

The current fixture is:

- `examples/sokoban3d.txt`

It is a target behavior fixture, not confirmed playable PS Next syntax yet.
