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

Use `choice` for items owned by the standard selection cursor and `button` for
auxiliary click/tap or explicitly keyed actions. A `level_menu { ... }`
component owns level-list cursor movement and selection; generic `for` and
`column` blocks do not become menus.
