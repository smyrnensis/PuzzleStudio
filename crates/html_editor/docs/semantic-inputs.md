# Semantic Inputs
Use owner-scoped `keys` to map physical keys to semantic inputs, scene routines,
or explicit scene effects.

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

The left side lists raw keys. In a puzzle the right side is a semantic input. In
a scene it may instead name a scene-local routine or a direct effect such as
`goto title`. The old `inputs { input <- keys }` form is not canonical syntax.

Direction inputs `up`, `down`, `left`, and `right` have standard built-in key
mappings; add `keys` only to override or extend them.
