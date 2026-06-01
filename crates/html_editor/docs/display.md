# Display
Use display objects for rendered helpers such as floors, cursors, hints, and highlights.

```puzzle
puzzle sokoban {
  layers {
    @floor = @Floor
    solid = Player Box Wall
  }

  routine @fill_floor repeat {
    [ no @Floor ] -> [ @Floor ]
  }

  on_display {
    @fill_floor
  }
}
```

Names starting with `@` are display objects.

`on_display` can produce visual helpers without changing saved play state.

