# Rule Effects
Effects request work outside a board-cell replacement. Write an effect by
itself or after a matching rewrite.

## Finish Or Control A Turn
```puzzle
rules {
once [ Player Trap ] -> cancel
once [ Player Goal ] -> win
once [ Conveyor Player ] -> [ Conveyor > Player ] again
}
```

`cancel` restores the state from the start of the current turn, including board,
mark, and variable writes. `win` clears the level through the normal
`on_level_clear` lifecycle. `again` commits the current turn and requests a
follow-up turn without repeating the previous input.

## Emit Feedback
```puzzle
rules {
once [ Player Coin ] -> [ Player ] sfx pickup
once [ Player Goal ] -> message "Goal reached"
}
```

`sfx` emits a named sound declared in `sounds`. `message` accepts quoted text or
a text value. These are observable turn results; they are not board objects.

## Level And Scene Effects
```puzzle
rules {
restart -> restart
if win_conditions -> next_level
}
```

Puzzle rules can emit `restart`, `next_level`, `goto`, or `start`. Normal level
clear should use `win_conditions` or `win`; use explicit navigation for an
intentional flow change.

Several effects may be clearer as separate lines inside `if`, lifecycle, or
routine blocks. Do not use an `effect` wrapper or `then` separator.
