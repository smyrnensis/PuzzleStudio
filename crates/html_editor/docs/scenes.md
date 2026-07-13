# Scenes
Use `scene` blocks for screen flow outside the board.

```puzzle
scene title {
layout {
title
choice "Start" -> goto playing
}
}
```

Scenes can show text, menus, puzzle slots, and other layout components.

Use `goto` to move to another scene while preserving its state. Use `start` when
the target scene should be initialized before entry. A top-level model gets an
automatic same-named playable scene unless an explicit scene overrides it.
