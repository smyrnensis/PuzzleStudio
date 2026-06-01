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

  inputs {
    resume <- Escape Enter Space
  }

  rules {
    resume -> back
  }
}
```

Use target-qualified commands such as `playing.restart` or `board.restart` when a command should operate on a specific target.

