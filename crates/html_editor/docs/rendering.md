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
includes empty cells. `wait animation` adds presentation pacing based on the
turn's visual animation duration. The logical turn always reaches its stable
state before the adapter schedules that wait.
