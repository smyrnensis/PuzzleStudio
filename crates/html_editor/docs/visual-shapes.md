# Visual Shapes And Animation
Use `shapes` for reusable visual patterns and `visuals` to bind those patterns,
colors, images, and transforms to object selectors.

## Reusable Shapes
```puzzle
shapes {
arrow {
..0..
.000.
..0..
..0..
.....
}
}

visuals {
Player {
#2d6cdf
shape = arrow
}
}
```

`shapes` owns visual data only. Object identity and collision still come from
`layers`. Use `shape = <name>` when a bare row could also be valid ASCII and
would therefore be ambiguous.

## Frames And Voxel Layers
Inside a shape, a line containing only `>` starts the next animation frame. In a
3D shape, a line containing only `-` starts the next -Z voxel layer of the same
frame. Do not put blank lines around these separators.

```puzzle
visuals {
Beacon {
duration 400ms
#ffd166 #ef476f
000
010
000
>
111
101
111
}
}
```

2D visuals must remain one voxel layer deep. Both 2D and 3D use the same
`visuals`, palette, shape, and frame vocabulary.

## Spatial Operations
```puzzle
visuals {
Arrow:directions {
#ffffff
shape = arrow
rotate directions from up
translate local (0, -0.25)
}
}
```

Rotation is `rotate [world|local] <angle> [from <angle>]`. In 3D, the same
axis-less form rotates around +Z (`up`); append `around <axis>` to select
another axis. Operations run in source order; put a transform on the visual
reference that needs it rather than deriving rotated shapes globally.
For a four-way 3D visual authored facing front,
`Arrow:horizontal { rotate horizontal from front }` expands the variants around
Z with front as the zero rotation.
