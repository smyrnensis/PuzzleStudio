# Maps And Expansion
Tags define finite value sets. `for` expands authoring syntax over a set or
numeric range, while `map` translates one bound value into another.

## Expand A Rule
```puzzle
for d in directions {
once d [ Player | Box ] -> [ Player | Box ]
}
```

Prefer bare `input` for normal four-direction movement. Use `for` when the
expanded value is needed in several positions or expressions. `for` is an
authoring-time projection, not a menu or runtime loop.

Numeric ranges are inclusive.

```puzzle
for n in 1...3 {
once [ Gate:n ] -> [ Gate:n ]
}
```

Range endpoints may be integer literals or integer `var` / `const` values whose
initial value is known while authoring is expanded. Updating the variable during
a turn does not change the generated rules.

## Translate Values With Map
```puzzle
map opposite directions {
up -> down
right -> left
down -> up
left -> right
}
```

A map call consumes a value already bound by a selector or `for` expansion.

```puzzle
for d in directions {
once d [ Arrow:d ] -> [ Arrow:opposite(d) ]
}
```

Maps can also drive visual table lookup. They do not create objects, tags, or
runtime variables. The map input and every output must belong to the declared
tag set.

Layout `for` follows the same projection principle, but it only creates layout
children. It does not gain cursor movement, selection, or level-menu behavior.
