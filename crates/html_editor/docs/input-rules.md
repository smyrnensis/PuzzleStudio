# Input Rules
Use `input` for a rule oriented by the current directional input.

```puzzle
rules {
input [ Player ] -> [ > Player ]
}
```

Bare `input` is the recommended form for the built-in `up`, `down`, `left`, and
`right` inputs. It uses the current direction as the rule orientation.

The explicit set form, such as `input horizontal` or `input directions`, is for
a rule that intentionally restricts or exposes its accepted direction set.
Prefer bare `input` for ordinary four-direction movement.

`>` marks the direction on the matched object.
