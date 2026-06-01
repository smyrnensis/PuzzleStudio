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

Use `goto` to move to another scene.

