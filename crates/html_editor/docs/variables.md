# Variables
Use variables for values that can change while the puzzle runs.

## Basic Variables
```puzzle
var button_is_pushed = false
var moves = 0
```

`var` creates a named value. Rules and lifecycle blocks can update it.

## Constants
```puzzle
const target_moves = 12
const door_color = "blue"
```

`const` creates a named value that stays the same.

## Persistent Variables
```puzzle
persistent var cleared = false
```

`persistent var` keeps its value across normal restart and level load inside the current session.

