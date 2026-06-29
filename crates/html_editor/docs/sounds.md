# Sounds
Use `sounds` to name sound effects and music.

```puzzle
sounds {
sfx step seed=step type=step volume=0.45
sfx splash seed=splash type=water volume=0.7
sfx box_drag seed=box-drag type=drag volume=0.75
sfx clear seed=clear type=jump volume=1
music theme seed=main bpm=105 volume=0.5
}

puzzle sokoban {
sounds {
move Player -> sfx step
move Box -> sfx box_drag
}

on_level_clear {
sfx clear
next_level
}
}
```

Put movement sounds in the puzzle. Put scene music in scene lifecycle. Put clear sounds in level lifecycle.
