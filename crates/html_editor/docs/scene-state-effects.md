# Scene State And Effects
A scene owns screen-local state, layout, input dispatch, and navigation. It does
not reinterpret puzzle source or board state.

## State Slots
```puzzle
scene playing {
state {
puzzle sokoban
message = "Push the box"
}
layout {
text message
sokoban
}
rules {
step sokoban
}
}
```

`puzzle sokoban` creates a scene-local model slot with the same name. `step
sokoban` sends the current semantic input to that slot. A bare `sokoban` layout
component displays it.

Scene `var` and `const` values belong to the scene instance. Puzzle variables
belong to their model; top-level values belong to the session. Keep state in the
smallest owner that explains it.

## Navigate Or Reinitialize
```puzzle
scene title {
layout {
choice "Continue" -> goto playing
choice "New Game" -> start playing
}
}
```

`goto` enters a scene while preserving its existing state. `start` initializes
the target scene first. A level scene can receive a level name, such as `goto
sokoban(warehouse)`.

## Explicit State Effects
Scene routines and controls may use target-qualified effects.

```puzzle
scene pause {
layout {
button "Restart" -> playing.restart
button "Next Level" -> playing.next_level
}
keys {
Escape -> goto playing
}
}
```

Use `clear_undo_history` to discard only undo history. `clear_game_progress`
also resets cleared levels, current-level selection, and persistent variables.
More focused operations include `reset persistent_vars`, `reset <variable>`,
`set current_level = <level>`, and `clear current_level`.

These effects are explicit interventions for menus, hubs, and debug controls.
Ordinary clear, advance, and restart behavior remains owned by the puzzle model
and its lifecycle.
