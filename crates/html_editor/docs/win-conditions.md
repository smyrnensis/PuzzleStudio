# Win Conditions
Use `win_conditions` for checks that clear the current level.

## Object Goals
```puzzle
win_conditions {
  some Goal
  all Goal on Box
}
```

This level clears when there is at least one `Goal` and every `Goal` has a `Box` on it.

## Expression Checks
```puzzle
win_conditions {
  exists(Goal)
  none([ Goal no Box ])
}
```

Expression checks are useful when the clear condition is easier to read as board queries.

## Multiple Lines
```puzzle
win_conditions {
  all Goal on Box
  no Enemy
}
```

Every line in `win_conditions` must be true before the level clears.

