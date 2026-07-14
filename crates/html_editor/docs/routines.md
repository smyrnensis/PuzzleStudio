# Routines
Use a routine to name and reuse a list of rule statements.

```puzzle
routine push {
[ > Player | Box ] -> [ > Player | > Box ]
}

rules {
input [ Player ] -> [ > Player ]
push
move
}
```

Defining a routine does not run it. Write its name in `rules` or another
routine to call it.

## Routine Application
A plain routine runs its statement list once. Add `repeat` when the whole list
must run again until it stops changing.

```puzzle
routine spread repeat {
once [ Fire | Wood ] -> [ Fire | Fire ]
once [ Fire | Grass ] -> [ Fire | Fire ]
}
```

Use `routine name random` to choose one currently applicable statement with a
deterministic choice. It does not introduce hidden randomness.
