# Layers
Use `layers` to declare board objects and decide which objects exclude each other.

```puzzle
puzzle sokoban {
layers {
Goal Button
Player Box Wall
@Cursor @Hint
}
}
```

Objects in the same layer cannot occupy the same cell at the same time.

Most layers do not need names. Use a named layer only when you also want a selector alias:

```puzzle
layers {
solid = Player Box Wall
}
```

Then a rule can use `solid` to mean `Player Box Wall`.

Names starting with `@` are display objects. They are useful for cursors, hints, highlights, and other objects drawn on top of play state.
