# Puzzle Block
Use a `puzzle` block for one playable ruleset.

```puzzle
puzzle sokoban {
layers {
Goal
Player Box Wall
}

win_conditions {
all Goal on Box
}

rules {
input [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
move
}

levels {
legend {
. = empty
# = Wall
P = Player
B = Box
G = Goal
}

level start
#####
#PBG#
#####
}
}
```

A puzzle usually contains layers, visuals, rules, win conditions, and levels.
`rules { ... }` is the required gameplay entry point; the removed
`transitions`, `main`, and `rule` forms are not alternatives.

Files can contain more than one puzzle when scenes need to switch between them.
Use `puzzle` for a 3D model. Both model kinds use `levels { ... }`; `levels3`
is no longer syntax.
