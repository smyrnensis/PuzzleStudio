# Sprites
Use `sprites` to define how objects look.

```puzzle
puzzle sokoban {
sprites {
Player {
image "sprites/player.png"
}

Box {
#b5651d #f4a261
00000
01110
01110
01110
00000
}
}
}
```

Image sprites use a game-folder relative path. ASCII sprites use a palette row
followed by rows of palette indexes.

Sprites are drawn into one cell by default. Use `contain`, `cover`, or
`stretch` to choose the draw box and scaling:

```puzzle
sprites {
Gate {
image "sprites/gate.svg"
contain 2 2
translate (0, -0.25)
}

Portrait {
image "sprites/portrait.jpg"
cover 1 1
}

Panel {
image "sprites/panel.png"
stretch 2 1
}
}
```

Use `layers` for collision and `legend` for level characters.

Sprite spatial operations use `translate [world|local] <vector>`. A 2D sprite
uses `rotate [world|local] <angle> [from <angle>]`, for example
`rotate directions from up`. A 3D sprite requires an axis:
`rotate local 90deg around up`. The removed `offset`, `rotate using`, and
generic `transform` forms are not accepted.

Both model kinds use `sprites`. In a reusable ASCII shape, `>` starts the next
animation frame and `-` starts the next -Z layer; 2D sprites must have depth 1.
