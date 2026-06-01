# Conditions
Use conditions for named checks derived from the current board.

## Count Objects
```puzzle
condition cargo_count = count(Box)
```

`count(Object)` returns how many matching objects are on the board.

## Check Presence
```puzzle
condition any_goal = exists(Goal)
condition no_open_goals = none([ Goal no Box ])
condition some_box_on_goal = some([ Goal Box ])
```

`exists`, `none`, and `some` describe whether matching objects or patterns are present.

## Use A Condition
```puzzle
rules {
  if no_open_goals {
    next_level
  }
}
```

Use a condition name inside `if` when the same check should stay readable or be reused.

