# Agent Notes

This crate owns the browser editor service, editor UI, preview compilation,
highlighting integration, workspace behavior, and editor-owned layout.

## Generated Output

Root `editor.html` is a generated artifact and must never be edited directly.
Patch this crate's source owner, such as `src/` or `static/`, then regenerate the
root export through the normal command when the checked-in artifact should follow
the source.

Before regenerating root `editor.html`, run:

```bash
git status --short -- editor.html
```

This status check is for reporting and awareness. Do not hand-edit root
`editor.html`; regenerate it from the source owner instead.

When verifying generated `editor.html` in the Codex in-app browser, do not try
to open it through `file://`. This environment's browser policy blocks local
file URLs even though the browser tool may describe `file://` as generally
supported. Serve the repository or generated file over `http://127.0.0.1:<port>/`
and open that URL instead.

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

Highlighting should use the Rust host/server or WASM `highlight_source_html`
path. JavaScript fallback should be plain escaped text, not an independent
`.puzzle` grammar.
