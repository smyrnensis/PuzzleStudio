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

Use `next_level` from rules or lifecycle hooks to advance.

