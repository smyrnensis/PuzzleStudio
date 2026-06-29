# Input Rules
Use `input` for rules that respond to directional input.

```puzzle
rules {
input [ Player ] -> [ > Player ]
}
```

`input` runs for `up`, `down`, `left`, and `right`.

`>` marks the direction on the matched object.

