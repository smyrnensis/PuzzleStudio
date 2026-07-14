# Groups
Use `groups` to create selector aliases in one block.

```puzzle
groups {
pushable = Box Crate
}
```

Groups keep rules readable when several objects behave the same way.

```puzzle
rules {
[ > Player | pushable ] -> [ > Player | > pushable ]
}
```

Use a group name where you would otherwise list the same objects repeatedly.
