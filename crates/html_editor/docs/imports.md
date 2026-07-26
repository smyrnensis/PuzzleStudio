# Imports

Each `.puzzle` file is a document module. Declare imports at the source root
with a required alias:

```puzzle
// game.puzzle
import board = "models/sokoban.puzzle"
import screens = "ui/screens.puzzle"

scene title {
layout {
puzzle main = board:sokoban
frame screens:title
}
}
```

Paths are relative to the importing document and must remain inside the
workspace. Missing documents, duplicate aliases, cycles, and root escapes are
errors. Files in the same folder are not imported automatically.

Only declarations from a directly imported module are visible, through
`<alias>:<name>`. Imports are not re-exported transitively; import the owning
document directly when another declaration is needed.

An import never inserts source text. Imported files own complete top-level
declarations such as `puzzle ... {}` or `scene ... {}`. Imports inside a puzzle,
scene, or another owner block are invalid, and files cannot provide partial
`rules`, `levels`, layout, or other owner bodies.
