# Marks
Use marks for short-lived facts that help a rule chain run.

## Declare Marks
```puzzle
marks {
visited
armed = bool
intent = directions
count = int
}
```

A bare mark name is a flag. A typed mark stores either a boolean, an integer, or one value from a named value set.

## Mark Objects
```puzzle
rules {
[ Box ] -> [ Box{armed} ]
[ Box{armed} ] -> [ Box{no armed} ]
}
```

Marks can be added to objects and removed again inside rules.

## Movement Intent
```puzzle
rules {
input [ Player ] -> [ > Player ]
move
}
```

Directional movement uses a temporary direction mark. `move` applies that direction after the input rule runs.
