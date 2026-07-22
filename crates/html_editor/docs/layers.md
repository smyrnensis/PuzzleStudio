# Layers
Use `layers` to declare board objects, state exclusion, and back-to-front
drawing order. There is no separate `objects` or visual-order block.

```puzzle
puzzle sokoban {
layers {
Goal Button
Player Box Wall
}
}
```

Objects on the same state layer cannot occupy the same cell at the same time.

Most layers do not need names. Use a named layer when you also want a selector alias:

```puzzle
layers {
solid = Player Box Wall
}
```

Then a rule can use `solid` to mean `Player Box Wall`. Use `groups` for selector
aliases that should not also define state-layer exclusion. Visual drawing order is
the declaration order in `layers`.

Prefix a visual resource with `!` to place a transient animation without adding
it to state storage. `Box !Box` is valid: the first name is the object and the
second is the animation visual.

Declare directional priority directly in `layers`. Use two distinct axes in 2D
and three in 3D:

```puzzle
layers {
priority = down right
Floor
Box !Push
}
```

`merge` keeps its rows as separate state layers while combining them into one
unordered drawing priority:

```puzzle
layers {
Floor
merge {
actor = Player Box
marker = Goal
effect = !Burst
}
}
```
