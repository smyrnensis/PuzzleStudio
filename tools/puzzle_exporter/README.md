# Puzzle HTML Exporter

Dev-only macOS launcher for exporting a `.puzzle` file or game folder to a
standalone `.html` file without typing paths.

Use:

1. Double-click `Puzzle HTML Exporter.command`.
2. Choose `Puzzle File` or `Folder`.
3. Pick the input in the macOS file picker.
4. Pick where to save the exported `.html`.

The launcher calls the existing exporter:

```bash
cargo run -p html-play -- <input> -o <output.html>
```

It is intentionally separate from the editor and is not packaged for release.
