# Levels
Use `levels` to store playable maps.

```puzzle
levels {
level first
#####
#PBG#
#####

level second
#######
#P.BG#
#######
}
```

Each non-empty row after `level <name>` is read as level text until a blank line, another `level`, or the end of the block.

2D and 3D models both use `levels`. A 3D level separates Z layers with a line
containing only `-`; the removed `levels3` block is not accepted.
In both dimensions, `.` is the reserved empty cell and needs no legend entry.

A level can own a local legend and turn-rule additions. `rules before` runs
before the puzzle rules, while `rules` (or `rules after`) runs afterward.

```puzzle
level tutorial {
legend {
x = Goal Box
}
rules before {
once [ Box ] -> [ Box{armed} ]
}
P.x
}
```

Use `next_level` from rules or lifecycle hooks to advance.
