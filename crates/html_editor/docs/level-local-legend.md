# Level Local Legend
Use a level-local `legend` when one level needs extra characters.

```puzzle
levels {
legend {
. = empty
P = Player
}

level warehouse
legend {
x = Goal Box
}

P.x
}
```

The level-local legend only affects that level.
