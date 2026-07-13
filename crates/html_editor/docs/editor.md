# Start Here
PuzzleStudio reads `.puzzle` files for 2D models and `.puzzle3` files for 3D
models. Start with one `puzzle` or `puzzle` block. A model owns its objects,
levels, rules, lifecycle, and presentation.

## Smallest Shape
```puzzle
puzzle my_game {
layers {
Goal
Player Wall
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
Player {
#2d6cdf
.000.
.0.0.
.000.
..0..
.0.0.
}
Wall {
#6c757d
00000
00000
00000
00000
00000
}
}

win_conditions {
all Goal on Player
}

rules {
input [ Player ] -> [ > Player ]
move
}

levels {
legend {
. = empty
G = Goal
# = Wall
P = Player
}

level start
#####
#P.G#
#####
}
}
```

`layers` is the only place that declares board objects. `legend` only maps
level characters to objects that already exist. `rules` is the required
gameplay entry point.

## Basic Path
Read the Basic pages in order when making your first game.

Metadata and Puzzle Block define the game and its playable model. Layers,
Legend, and Levels make a board that the editor can load. Rewrite Rules, Input
Rules, and Movement make it respond to input. Win Conditions make a level
finish, and Sprites make declared objects visible.

## Advanced Path
Read Advanced pages when the basic game needs another capability.

Advanced is organized by authoring task: Rules & Patterns, State & Lifecycle,
Objects & Selectors, Levels, Project Structure, Scenes & UI, Visuals, 3D, and
Assets & Sound. Choose the chapter for the task you are doing instead of reading
Advanced as one long sequence.

## First Editing Loop
Change the level first, then the legend, then the rules. That keeps each edit
visible.

Add a character to the level.

Map that character in `legend`.

Put it in a layer.

Give the object a sprite if it should be visible. A 3D object without a sprite
is not given an implicit cube.

Add rules only after it appears correctly.
