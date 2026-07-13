# Scene Layout
Use `layout` only when the default puzzle screen is not enough.

Most single-board games do not need a layout block. A lone `puzzle sokoban`
gets a playable screen automatically.

```puzzle
puzzle sokoban {
layout {
heading title
sokoban
}
}
```

The model name, here `sokoban`, displays that puzzle board.

Use an explicit `scene` when you need a title screen, menu, or level select screen.

```puzzle
scene title {
layout {
heading title
choice "Start" -> goto sokoban
}
}
```

`heading`, `subheading`, `text`, and `caption` are text roles. `row`, `column`,
and `box` are containers. Layout allocation uses `space fit` or
`space fill <weight>`; `aspect <w> <h>` controls ratio. Layout does not use CSS
sizes or the removed `size <w> <h>` form.
