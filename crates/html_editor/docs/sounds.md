# Sounds
Use `sounds` to name sound effects and music.

```puzzle
sounds {
  sfx box_drag seed=box-drag type=drag volume=0.75
  sfx clear seed=clear type=jump volume=1
  music theme seed=main bpm=105 volume=0.5
}

puzzle sokoban {
  sounds {
    move Box -> sfx box_drag
  }

  on_level_clear {
    sfx clear
    next_level
  }
}
```

Put movement sounds in the puzzle. Put scene music in scene lifecycle. Put clear sounds in level lifecycle.
