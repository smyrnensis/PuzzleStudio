# Tags
Use `tags` to define ordered sets of object-name atoms for schema axes.

```puzzle
tags {
  color = red blue
}

puzzle color_boxes {
  layers {
    solid = Player Box:color Wall
  }
}
```

`Box:color` expands into object identities such as `Box:red` and `Box:blue`.

When a tag set contains object-name atoms, a qualified selector appends the
suffix to each atom. For example, `pair:a` with `pair = A B` resolves as
`A:a B:a`.

Built-in tag sets include `directions`, `horizontal`, and `vertical`.
