# Minimal Sokoban
Start with one model, one level, and the two rules that make pushing work.

## Copy this into a `.puzzle` file
```puzzle
title "Minimal Sokoban"
puzzle sokoban {
  layers {
    target = Goal
    solid = Player Box Wall
  }
  sprites {
    Goal {
      #ffd166
      .....
      .000.
      .0.0.
      .000.
      .....
    }
    Wall {
      #6c757d
      00000
      00000
      00000
      00000
      00000
    }
    Box {
      #b5651d #f4a261
      00000
      01110
      01110
      01110
      00000
    }
    Player {
      #2d6cdf
      .000.
      .0.0.
      .000.
      ..0..
      .0.0.
    }
  }
  win_conditions {
    all Goal on Box
  }
  rules {
    input directions [ Player ] -> [ Player{>} ]
    [ Player{>} | Box ] -> [ Player{>} | Box{>} ]
    move
  }
  levels {
    legend {
      . = empty
      # = Wall
      P = Player
      B = Box
      G = Goal
      * = Goal Box
      + = Goal Player
    }
    level start
    #####
    #PBG#
    #####
    message "WIN"
  }
}
```

## What matters first
`layers` defines the objects and collision layers. `sprites` gives the visible objects a small 5x5 pixel shape. `legend` maps level characters to those objects.

The first rule marks the player with movement intent. The second rule passes that intent to a box. `move` commits valid movement and blocks walls or occupied solid cells.

The level clears when every `Goal` has a `Box` on it, then shows `WIN`.
