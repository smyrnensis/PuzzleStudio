# Legend
Use `legend` to map level characters to objects.

```puzzle
levels {
legend {
# = Wall
P = Player
B = Box
G = Goal
* = Goal Box
+ = Goal Player
}
}
```

Each non-empty character used in a level map should have a legend entry. `.` is
reserved as empty in both 2D and 3D, so it needs no entry. `. = empty` may be
written explicitly, but `.` cannot be remapped and another character cannot be
assigned to `empty`.

The right side must resolve to objects or selectors already declared by
`slots`; a legend entry never creates an object.

One character can place multiple objects in the same cell, such as `Goal Box`.
