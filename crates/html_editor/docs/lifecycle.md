# Lifecycle
Use `on_*` blocks for named moments in a puzzle or scene.

```puzzle
scene title {
on_scene_start {
stop_music theme
}
}
```

`on_scene_start` belongs to a scene. `on_level_start` and `on_level_clear`
belong to a puzzle model; do not move model setup through a fake player input.

Put setup, music, messages, and level advancement in the hook that names when they should happen.
