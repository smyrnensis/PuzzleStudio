# Patterns
Patterns describe cells and spatial relationships on the left and right side of
a rewrite.

## Rows And Orientation
Separate cells in a row with `|`. Prefix a pattern with `left`, `right`, `up`,
or `down` when its orientation is fixed.

```puzzle
right [ Player | Box | no Wall ] -> [ | Player | Box ]
```

Bare `input` uses the current directional input as the orientation.

```puzzle
input [ Player | Box ] -> [ > Player | > Box ]
```

## Rectangles
Separate rows inside one bracket block with `;` to match a rectangle. Every row
must have the same width.

```puzzle
down [ Player | Box ; no Wall | Goal ]
  -> [ | Player ; no Wall | Goal Box ]
```

## Variable Gaps
Use `...` between pattern cells when any non-negative distance may separate the
two sides.

```puzzle
once right [ Laser | ... | Target ] -> [ Laser | ... | Ash ]
```

## Absence And Outside
`no Wall` requires a cell without `Wall`. An empty pattern cell matches a board
cell without a positive object requirement. `null` matches outside the stage;
it is not an object and cannot be written as `no null`.

```puzzle
once right [ Edge | null ] -> [ Edge | ]
```
