# Assets
Use the top-level `assets` manifest to declare external files used by the game.

```puzzle
assets {
css "game.css"
script "visuals.js"
file "visuals/player.png"
}
```

Paths are relative to the game folder. Only declared CSS and scripts are loaded,
and standalone export embeds only declared files. A file merely being present
in the folder does not make it a game asset.

Visual `image` paths must also belong to the game folder.

```puzzle
visuals {
Player {
image "visuals/player.png"
}
}
```

Asset scripts may derive additional presentation from rendered scene snapshots.
They do not own puzzle rules and must not directly mutate puzzle state, undo
history, or level progression.
