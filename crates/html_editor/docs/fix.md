# Fix
Use `fix` when a group of rules should repeat or be limited together.

```puzzle
rules {
fix once {
[ Player | Coin ] -> [ Player | ]
[ Player | Key ] -> [ Player | ]
}
}
```

`fix once` gives each contained rewrite the `once` application mode. It fixes
the enclosed rules' defaults; it does not make the whole block a new routine or
repeat the block as a unit.
