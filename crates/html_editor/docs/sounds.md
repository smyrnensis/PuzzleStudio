# Sounds
Use `sounds` to name sound effects and music.

```puzzle
sounds {
  sfx push seed=push type=hit
  sfx clear seed=clear type=jump
  music theme seed=main bpm=105 volume=0.5
}

puzzle sokoban {
  sounds {
    move Box -> sfx push
  }

  on_level_clear {
    sfx clear
    next_level
  }
}
```

Put movement sounds in the puzzle. Put scene music in scene lifecycle. Put clear sounds in level lifecycle.

