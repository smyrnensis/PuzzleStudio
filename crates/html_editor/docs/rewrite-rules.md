# Rewrite Rules
Rules describe local state changes.

```puzzle
rules {
[ Player | Box ] -> [ Player | Crate ]
}
```

A rewrite has a left side, an arrow, and a right side.

Cells inside a row pattern are separated by `|`. A selector such as `Player` or `Box` matches an object in that cell.

Use `no <selector>` when a board cell must not contain an object. Use `null` when the pattern cell itself must be outside the stage.

```puzzle
rules {
once right [ no Edge | null ] -> [ Edge | ]
}
```

`null` is only an outside-board pattern atom. It is not an object, cannot be combined with other cell tokens, and is not written as `no null`.
