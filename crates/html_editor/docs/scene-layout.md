# Scene Layout
Use `layout` only when the default puzzle screen is not enough.

Most single-board games do not need a layout block. A lone `puzzle sokoban`
gets a playable screen automatically.

```puzzle
puzzle sokoban {
layout {
title
sokoban
}
}
```

The model name, here `sokoban`, displays that puzzle board.

Use an explicit `scene` when you need a title screen, menu, or level select screen.

```puzzle
scene title {
layout {
title
choice "Start" -> goto sokoban
}
}
```
