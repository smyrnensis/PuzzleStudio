# Rule Application
Application keywords decide how many matches a rewrite applies.

## Repeat
A rewrite without an application keyword uses `repeat`: it keeps applying the
same concrete rewrite until no match remains.

```puzzle
[ Fire | Wood ] -> [ Fire | Fire ]
```

## Once
`once` applies only the first match in board order.

```puzzle
once [ Coin ] -> [ ]
```

`once_all` collects all matches from the state at the start of the rule and
applies each collected match at most once. New matches created by those writes
are not included.

```puzzle
once_all [ Ice ] -> [ Water ]
```

`once_per_level` lets a concrete rule fire once until restart or level change.

```puzzle
once_per_level [ Player Goal ] -> message "Goal reached"
```

## Random
`random` applies one available match. The choice is deterministic for the same
game state and input, so it remains visible to replay and solver behavior.

```puzzle
random [ Spark | Wire ] -> [ | Spark ]
```

Use `fix once { ... }` when several enclosed rewrites should share the same
default. An explicit keyword on a contained rewrite overrides the `fix` value.
