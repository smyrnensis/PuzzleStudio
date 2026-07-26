# Agent Notes

This crate owns the browser editor service, editor UI, preview compilation,
highlighting integration, workspace behavior, and editor-owned layout.

## Generated Output

`static/renderer.css` and `static/puzzle3_visual_core.js` are generated Tauri
`frontendDist` copies. Their
only source owners are the same-named files under `../html_play/static/`. Never
edit the editor copies directly. Run `tools/sync_static_assets.sh` to regenerate
them, or `tools/sync_static_assets.sh --check` to verify freshness. Adding a new
shared desktop distribution asset requires adding its mapping only to that sync
script; freshness and desktop-boundary checks consume the script's check mode.

`static/editor_authoring_renderer.js` is editor-owned. It renders incomplete
authoring grids and object thumbnails as interactive DOM. Solver observations,
playtest previews, and other valid runtime states must be projected through the
Rust typed render scene and use `../html_play/static/renderer.js`; they must not
be interpreted by the authoring renderer.

`static/editor_codemirror.js` is generated from `web/src/editor_codemirror.js`
and the locked npm dependencies under `web/`. Do not edit the bundle directly.
Install dependencies outside the repository with
`tools/install_editor_frontend_deps.sh`, then regenerate it with
`tools/build_editor_frontend.sh`. The installer keeps `node_modules` under
`/private/tmp` (or `PUZZLE_EDITOR_FRONTEND_CACHE`) and places only a symlink in
the worktree, so disposable dependencies are not synchronized through iCloud.
A missing bundle is an error; do not load CodeMirror from a CDN or fall back to
the old textarea editor.

The web release surface is the GitHub Pages site under `docs/`, not a root
single-file `editor.html`. Patch this crate's source owner, such as `src/` or
`static/`, then regenerate the Pages site through `tools/generate_web_editor.sh`
when the checked-in web release should follow the source.

For local editor checks, prefer serving the editor:

```bash
cargo run -p html-editor -- games/spec_2d.puzzle --serve
```

`tools/generate_web_editor.sh <path> -o docs/index.html` writes the Pages HTML
entry and copies the static JS, CSS, editor WASM, and game runtime WASM assets
beside it. If Rust/WASM changes are meant to appear there,
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

## Editor UI Icons

Use Lucide for general editor UI icons. `static/editor_icons.js` is the single
owner of Lucide SVG geometry. HTML uses `data-editor-icon` placeholders and
JavaScript consumers call the registry helpers; do not copy SVG paths into
feature files. Feature owners still own the semantic mapping from their state or
action to an icon name. Missing registry names must fail visibly rather than
falling back to another icon.

Keep brand marks, favicons, CSS patterns, game-authored SVG assets, and generated
`html_play` copies outside this UI icon registry.

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

CodeMirror highlighting is a range projection of typed Rust spans. Map existing
decorations through edits, remove only decorations intersecting the returned
viewport range, and retain decorations outside that range. Do not replace the
whole decoration set for a range response. UTF-8/UTF-16 offset conversion is an
adapter responsibility and may be cached for one exact source snapshot; it must
not classify source text or infer syntax while constructing that map.

### Source Editor And CodeMirror

CodeMirror owns the generic editing mechanism: the text buffer, selection,
history, viewport, generic editing commands, and arbitration of physical key
bindings. It must not own PuzzleStudio completion policy, inspect completion
items, decide whether a source context is completable, or trigger save, preview,
highlight, and level-builder side effects.

`static/editor_source.js` owns the source-editing workflow above that mechanism:
completion eligibility, request and popup state, candidate selection, commit
validation and replacement, and all editor workflow effects after a commit.

The CodeMirror adapter may translate a physical key into a semantic source
editor command such as `show`, `next`, `previous`, `commit`, or `close`. The
source workflow must explicitly consume that command before CodeMirror suppresses
its normal behavior. When the workflow does not consume it, CodeMirror must keep
the ordinary meaning of the key, including Tab indentation and Enter newline.
Do not duplicate completion state or PuzzleStudio syntax knowledge in the
CodeMirror adapter.
