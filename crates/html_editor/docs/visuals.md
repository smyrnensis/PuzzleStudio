# Visuals
Use `visuals` to define how objects look.

```puzzle
puzzle sokoban {
visuals {
Player {
image "visuals/player.png"
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

Image visuals use a game-folder relative path. ASCII visuals use a palette row
followed by rows of palette indexes.

Visuals are drawn into one cell by default. Use `contain`, `cover`, or
`stretch` to choose the draw box and scaling:

```puzzle
visuals {
Gate {
image "visuals/gate.svg"
contain 2 2
translate (0, -0.25)
}

Portrait {
image "visuals/portrait.jpg"
cover 1 1
}

Panel {
image "visuals/panel.png"
stretch 2 1
}
}
```

Use `slots` for collision and `legend` for level characters.

Visual spatial operations use `translate [world|local] <vector>`. A 2D visual
uses `rotate [world|local] <angle> [from <angle>]`, for example
`rotate directions from up`. A 3D visual may use the same `from` and axis-less
forms; an omitted axis defaults to +Z (`up`). Use `around <axis>` for another
axis, for example `rotate local facing from 0deg around right`. The removed
`offset`, `rotate using`, and generic `transform` forms are not accepted.
For four-way 3D variants authored facing front, use
`Arrow:horizontal { rotate horizontal from front }`.

Both model kinds use `visuals`. In a reusable ASCII shape, `>` starts the next
animation frame and `-` starts the next -Z layer; 2D visuals must have depth 1.

Drawing order belongs inside `visuals`:

```puzzle
visuals {
order {
priority = down right
Floor
Player + Goal
Box
}
}
```

Order rows run back to front and may name slots or objects. `A + B` is sugar
for canonical `merge { A; B }`; merge has no internal order. Overlapping
pixels or voxels average their non-transparent RGBA channels. Direction
priority compares owner-cell coordinates lexicographically. Use two distinct
axes in 2D and three in 3D (`priority = down right front`, for example).
