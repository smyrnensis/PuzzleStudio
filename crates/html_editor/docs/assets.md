# Assets
Use the top-level `assets` manifest to declare external files used by the game.

```puzzle
assets {
"visuals/player.png"
"audio/push.wav"
}
```

Paths are relative to the game entry. Standalone export embeds declared files;
a file merely being present in the folder does not make it a game asset.
Presentation and behavior belong to the typed `theme`, `visuals`, `sounds`,
scene, and component contracts.

Visual `image` paths must also belong to the workspace.

```puzzle
visuals {
Player {
image "visuals/player.png"
}
}
```
