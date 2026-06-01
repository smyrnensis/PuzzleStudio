# Rewrite Rules
Rules describe local state changes.

```puzzle
rules {
  [ Player | Box ] -> [ Player | Crate ]
}
```

A rewrite has a left side, an arrow, and a right side.

Cells inside a row pattern are separated by `|`. A selector such as `Player` or `Box` matches an object in that cell.

