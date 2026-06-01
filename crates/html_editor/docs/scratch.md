# Scratch
Use scratch for short-lived marks that help one step of rules run.

## Declare Scratch
```puzzle
scratch {
  visited
  armed = bool
}
```

A bare scratch name is a flag. A typed scratch name can store a small value.

## Mark Objects
```puzzle
rules {
  [ Box ] -> [ Box{armed} ]
  [ Box{armed} ] -> [ Box{no armed} ]
}
```

Scratch marks can be added to objects and removed again inside rules.

## Movement Intent
```puzzle
rules {
  input [ Player ] -> [ > Player ]
  move
}
```

Directional movement uses a temporary direction mark. `move` applies that direction after the input rule runs.

