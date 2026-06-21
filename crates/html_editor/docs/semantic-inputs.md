# Semantic Inputs
Use `keys` to map physical keys to named semantic inputs.

```puzzle
puzzle sokoban {
  keys {
    w ArrowUp -> up
    s ArrowDown -> down
    a ArrowLeft -> left
    d ArrowRight -> right
    r -> restart
  }
}
```

The left side lists keys. The right side is the semantic input.

Rules and scenes use the action name.
