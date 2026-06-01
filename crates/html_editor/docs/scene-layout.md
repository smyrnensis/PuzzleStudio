# Scene Layout
Use `layout` to describe what a scene shows.

```puzzle
scene playing {
  state {
    board = puzzle sokoban
  }

  layout {
    title
    subtitle level.label
    board
  }

  rules {
    step board
  }
}
```

The scene state names a puzzle slot. The layout displays that slot.

`step board` sends gameplay input into the puzzle.

