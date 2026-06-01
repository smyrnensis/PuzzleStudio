# Inputs
Use `inputs` to map physical keys to named actions.

```puzzle
puzzle sokoban {
  inputs {
    up <- w ArrowUp
    down <- s ArrowDown
    left <- a ArrowLeft
    right <- d ArrowRight
    restart <- r
  }
}
```

The left side is the action name. The right side lists keys.

Rules and scenes use the action name.

