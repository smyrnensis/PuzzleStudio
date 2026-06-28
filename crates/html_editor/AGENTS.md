# Agent Notes

This crate owns the browser editor service, editor UI, preview compilation,
highlighting integration, workspace behavior, and editor-owned layout.

## Generated Output

The web release surface is the GitHub Pages site under `docs/`, not a root
single-file `editor.html`. Patch this crate's source owner, such as `src/` or
`static/`, then regenerate the Pages site through `tools/generate_web_editor.sh`
when the checked-in web release should follow the source.

For local editor checks, prefer serving the editor:

```bash
cargo run -p html-editor -- games/spec_2d.puzzle --serve
```

`tools/generate_web_editor.sh <path> -o docs/index.html` writes the Pages HTML
entry and copies the static JS, CSS, editor WASM, core runtime WASM, and game
runtime WASM assets beside it. If Rust/WASM changes are meant to appear there,
rebuild the generated WASM artifacts explicitly before generating the Pages site.
Do not add a portable single-file HTML release path or a compatibility alias for
the removed root artifact.

Use `tools/serve_web_editor.sh` or `tools/open_web_editor.command` for the
normal local web editor. These start the Rust editor server so `/api/highlight`,
preview compilation, save, and other editor backend routes are available.

Do not use a static Pages mode for editor development. The supported local
entrypoint is the Rust editor server, because preview compilation,
highlighting, save, and workspace APIs are server-owned instead of browser
fallback behavior.

For visual feedback, prefer the shortest human-visible loop: open the served
editor for the user or capture a screenshot of the relevant viewport. DOM-only
inspection is useful for state checks, but it is a poor substitute for checking
layout, spacing, rendering, and first-load visual behavior.

## Editor Boundaries

The HTML editor, server mode, and desktop shell should share the same editor
service for source loading, preview compilation, highlighting, export, and save
semantics. Do not fork compile or preview logic for desktop.

Web mode keeps the browser-shaped workflow: import folder or zip into
browser/editor state, then download/export files. Durable local filesystem access
belongs to host adapters.

Editor frame geometry is owned in one place. The workbench owns pane columns and
splitter width through `syncWorkbenchGridLayout` plus the
`--workbench-splitter-width` CSS variable. Tool panes must not rederive pane
widths. Preview iframe fitting, level board fitting, and similar surfaces should
ask shared frame helpers in `static/editor.js` for the available rectangle, then
apply only their own aspect ratio, virtual size, or tool chrome.

3D level editor preview must use a public runtime control contract, not runtime
fixture internals. Editor-originated changes should go through explicit preview
update/render APIs with named `level`, `resources`, `camera`, `view`, and
`settings` fields. Keep whole-fixture replacement as compatibility/debug surface
only.

Highlighting should use the Rust host/server path. If highlighting is
unavailable, show plain escaped text and surface the reason visibly enough for
debugging. Do not fall back to a second highlighter path such as WASM or a
JavaScript `.puzzle` grammar.
