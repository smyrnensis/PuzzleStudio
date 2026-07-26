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
const title = "Level Select"
```

`const` creates a named value that stays the same. Names such as `title`,
`author`, or `homepage` have no special declaration or compile behavior; a scene
may display any constant through a text component.

## Persistent Variables
```puzzle
persistent var cleared = false
```

`persistent var` keeps its value across normal restart and level load inside the current session.
