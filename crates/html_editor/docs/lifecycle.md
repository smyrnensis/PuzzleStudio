# Lifecycle
Use `on_*` blocks for named moments in a puzzle or scene.

```puzzle
scene title {
  on_scene_start {
    stop_music theme
  }
}
```

Common lifecycle hooks include `on_scene_start`, `on_level_start`, and `on_level_clear`.

Put setup, music, messages, and level advancement in the hook that names when they should happen.

