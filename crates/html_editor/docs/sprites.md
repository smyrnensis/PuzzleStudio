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
offset 0 -0.25
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
