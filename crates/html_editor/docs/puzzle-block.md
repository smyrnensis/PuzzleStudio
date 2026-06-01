# Puzzle Block
Use a `puzzle` block for one playable ruleset.

```puzzle
puzzle sokoban {
  layers {
    target = Goal
    solid = Player Box Wall
  }

  rules {
    input [ Player ] -> [ > Player ]
    move
  }

  levels {
    level start
    #####
    #PBG#
    #####
  }
}
```

A puzzle usually contains layers, sprites, rules, win conditions, and levels.

Files can contain more than one puzzle when scenes need to switch between them.

