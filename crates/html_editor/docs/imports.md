# Imports
Use imports to split one game into files without changing ownership or scope.

```puzzle
// game.puzzle
puzzle sokoban {
slots {
Goal
Player Box Wall
}
import "rules.puzzle"
import "levels.puzzle"
}
```

An import expands the referenced file at the location of the `import` line. A
fragment therefore contains the syntax valid at that exact owner position.

```puzzle
// rules.puzzle
rules {
input [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
move
}
```

Paths are relative to the file containing the import. Nested imports resolve
relative to their own files. Files in the same folder are not loaded
automatically.

A runnable game folder still needs an entry file declaring a top-level
`puzzle` or `puzzle`. Imported fragments use `.puzzle` too; the extension does
not create an implicit wrapper.
