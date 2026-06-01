# Movement
Use `move` after rules have marked objects with direction.

```puzzle
rules {
  input [ Player ] -> [ > Player ]
  [ > Player | Box ] -> [ > Player | > Box ]
  move
}
```

The first rule marks the player. The second rule passes that direction to a box.

`move` applies the marked movement and blocks movement into occupied cells in the same layer.

