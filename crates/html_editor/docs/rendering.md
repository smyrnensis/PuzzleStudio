# Rendering
Model-owned rendering options belong in `render`. They affect presentation, not
board state, collision, win conditions, or solver state.

## Grid And Tween
```puzzle
puzzle sokoban {
render {
grid occupied_cells
tween = true
tween_duration = 160ms
}
}
```

`grid occupied_cells` outlines cells containing objects. `grid all_cells`
includes empty cells. `wait animation` commits the current rule segment and
pauses the turn until that segment's visual animations finish. The remaining
rules resume in the same turn.
