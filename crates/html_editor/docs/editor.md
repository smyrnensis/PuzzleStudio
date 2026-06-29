# Start Here
PuzzleStudio reads `.puzzle` files. Start with one `puzzle` block, then add the pieces you need: objects, level text, rules, and a win condition.

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

## Where To Go Next
Use the pages on the left by what you are changing:

## File Shape
Read Metadata and Puzzle Block.

## Board Structure
Read Layers, Groups, Tags, Legend, Levels, Level Legend, and Messages.

## Behavior
Read Rewrite Rules, Input Rules, Movement, Guards, and Fix.

## State And Checks
Read Variables, Scratch, Conditions, and Win Conditions.

## Screen Flow
Read Scenes, Scene Layout, Semantic Inputs, Menus, and Lifecycle.

## Presentation
Read Sprites, Display, Theme, and Sounds.

## First Editing Loop
Change the level first, then the legend, then the rules. That keeps each edit visible:

Add a character to the level.

Map that character in `legend`.

Give the object a sprite.

Put it in a layer.

Add rules only after it appears correctly.
