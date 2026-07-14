# Slots
Use `slots` to declare board objects and decide which objects exclude each
other. There is no separate `objects` block.

```puzzle
puzzle sokoban {
slots {
Goal Button
Player Box Wall
}
}
```

Objects in the same slot cannot occupy the same cell at the same time.

Most slots do not need names. Use a named slot only when you also want a selector alias:

```puzzle
slots {
solid = Player Box Wall
}
```

Then a rule can use `solid` to mean `Player Box Wall`. Use `groups` for selector
aliases that should not also define storage-slot exclusion. Sprite drawing order is
defined separately by `sprites { order { ... } }`.
