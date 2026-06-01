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

`fix once` applies each contained rewrite as a single match instead of repeatedly applying it.

