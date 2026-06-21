# Menus
Use choices and commands to build menus.

```puzzle
scene menu {
  layout {
    text "Menu"
    column {
      choice "Resume" -> input resume
      choice "Restart" -> playing.restart
      choice "Level Select" -> goto level_select
    }
  }

  keys {
    Escape Enter Space -> resume
  }

  routine resume {
    goto playing
  }
}
```

Use target-qualified commands such as `playing.restart` or `board.restart` when a command should operate on a specific target.
