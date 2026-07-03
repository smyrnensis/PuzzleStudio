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

A puzzle usually contains layers, sprites, rules, win conditions, and levels.

Files can contain more than one puzzle when scenes need to switch between them.
