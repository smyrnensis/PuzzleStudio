# Legend
Use `legend` to map level characters to objects.

```puzzle
levels {
legend {
. = empty
# = Wall
P = Player
B = Box
G = Goal
* = Goal Box
+ = Goal Player
}
}
```

Each character used in a level map should have a legend entry.

One character can place multiple objects in the same cell, such as `Goal Box`.

