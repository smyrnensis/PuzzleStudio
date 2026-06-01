# Layers
Use `layers` to declare board objects and decide which objects exclude each other.

```puzzle
puzzle sokoban {
  layers {
    floor = Goal Button
    solid = Player Box Wall
    @overlay = @Cursor @Hint
  }
}
```

Objects in the same layer cannot occupy the same cell at the same time.

Layer names also become selectors. A rule can use `solid` to mean `Player Box Wall`.

Names starting with `@` are display objects. They are useful for cursors, hints, highlights, and other objects drawn on top of play state.

