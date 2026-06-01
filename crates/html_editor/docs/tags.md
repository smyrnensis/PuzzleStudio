# Tags
Use `tags` to define object variants.

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

`Box:color` expands into objects such as `Box:red` and `Box:blue`.

Built-in tag sets include `directions`, `horizontal`, and `vertical`.

