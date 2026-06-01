# Groups
Use `group` to create selector aliases.

```puzzle
group {
  solid = Player Box Wall
  pushable = Box Crate
  @hints = @Cursor @Hint
}
```

Groups keep rules readable when several objects behave the same way.

```puzzle
rules {
  [ > Player | pushable ] -> [ > Player | > pushable ]
}
```

Use a group name where you would otherwise list the same objects repeatedly.

