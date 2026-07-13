# Sprite Shapes And Animation
Use `shapes` for reusable visual patterns and `sprites` to bind those patterns,
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

sprites {
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
sprites {
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

2D sprites must remain one voxel layer deep. Both 2D and 3D use the same
`sprites`, palette, shape, and frame vocabulary.

## Spatial Operations
```puzzle
sprites {
Arrow:directions {
#ffffff
shape = arrow
rotate directions from up
translate local (0, -0.25)
}
}
```

2D rotation is `rotate [world|local] <angle> [from <angle>]`. 3D rotation
requires `around <axis>`. Operations run in source order; put a transform on the
sprite reference that needs it rather than deriving rotated shapes globally.
