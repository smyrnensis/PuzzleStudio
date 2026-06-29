# Desktop Asset Boundary Safety

この文書は、Tauri desktop build を軽くするために editor/web asset の埋め込み境界を直す前の安全条件を固定する。

Executable guard:

```bash
node tools/check_desktop_asset_boundary.mjs
```

Before shipping a desktop asset boundary change, the guard must be run in enforcing mode:

```bash
node tools/check_desktop_asset_boundary.mjs --enforce
```

目的は、`src-tauri` から不要な static asset 埋め込みを外すことであって、editor service、browser preview、save、highlight、workspace 操作、standalone export、Pages export の挙動を変えることではない。

## Problem Statement

Tauri desktop は `tauri.conf.json` の `frontendDist` で `crates/html_editor/static` を配れる。したがって desktop shell の通常起動に必要な HTML、JS、CSS、WASM は Tauri asset として扱える。

一方で、現在の `src-tauri` は `html-editor` crate に依存している。その crate は editor service だけでなく、web server、Pages export、single-file export のために多数の `include_str!` / `include_bytes!` を持つ。結果として、desktop が直接必要としない static asset 埋め込みまで Rust dependency graph に入る。

修正対象はこの所有境界である。asset が不要なのではなく、desktop host が asset-embedding owner に依存していることが問題である。

## Contracts To Preserve

Desktop host must preserve these Rust-side behaviors:

- Opening a project folder or puzzle file through the Tauri dialog.
- Restoring previously loaded workspaces without substituting fallback folders.
- Returning `source_json` payloads with workspace root, active puzzle path, source, CSS, documents, and recent workspaces.
- Creating the new puzzle source from the authoring template.
- Saving, creating, renaming, and deleting workspace entries under the opened workspace root only.
- Exporting user-provided HTML through the native save dialog.
- Watching opened workspace files and surfacing watcher errors explicitly.

Web/editor/export must preserve these asset-owned behaviors:

- Browser editor preview compilation and source highlighting run through the browser editor runtime assets, including Tauri desktop's static frontend.
- Browser editor preview diagnostics stay structured instead of being flattened into a generic error.
- The standalone `html-editor` server can serve editor HTML, JS, CSS, WASM, sound tools, and preview routes.
- Pages editor export can still write a self-contained static editor release under `docs/`.
- Standalone game export can still embed the runtime assets it intentionally owns.
- Editor sound tools remain available to desktop and web editor flows.
- WASM editor compile/highlight assets remain available to browser editor flows.

## Required Separation

The safe shape is:

```txt
src-tauri
  -> editor workspace/service contract
  -> Tauri frontendDist assets

html-editor binary/server/export
  -> editor workspace/service contract
  -> editor asset embedding

html-play export/server
  -> play/export logic
  -> game runtime asset embedding
```

`src-tauri` must not depend on an asset-embedding crate merely to get editor service behavior. If desktop needs an asset-like payload, such as sound tools, that dependency must be named as a desktop host contract rather than inherited accidentally through the full web exporter.

## Safety Gates

Before claiming the boundary refactor is correct, all applicable gates must pass.

1. Build graph gate:

   ```bash
   cargo tree -p puzzlestudio-desktop -e normal,build --locked --offline
   ```

   The tree must show the intended service crate boundary. It must not pull in an editor asset/export crate unless the dependency is explicitly justified.

2. Asset embedding gate:

   ```bash
   node tools/check_desktop_asset_boundary.mjs --enforce
   ```

   This fails if desktop-owned code embeds static web assets, if `src-tauri` enables `html-editor`'s `embedded-assets` or `native-preview` features, if `src-tauri` directly depends on `html-play`, or if Tauri no longer points at the expected `frontendDist`.

3. Service behavior gate:

   ```bash
   cargo test -p html-editor --lib
   ```

   Existing service tests around open, source JSON, preview compilation, diagnostics, save, workspace mutation, generated file exclusion, and sound tools must still pass, or move with the service owner and pass there.

4. Desktop host gate:

   ```bash
   cargo check -p puzzlestudio-desktop --locked --offline
   ```

   This must pass before claiming the desktop refactor works.

5. Browser/editor gate:

   ```bash
   cargo test -p html-editor --test browser_smoke
   ```

   If the environment cannot run the browser smoke, record that limitation and run the owner-local served editor manually before shipping the refactor.

6. Export gate:

   ```bash
   cargo run -p html-editor -- games/spec_2d.puzzle -o /tmp/puzzlestudio-editor.html
   cargo run -p html-play -- games/spec_2d.puzzle -o /tmp/puzzlestudio-game.html
   ```

   These verify that removing desktop's dependency on embedding did not remove the web/export owners' embedding behavior.

## Current Verification Notes

`cargo check -p puzzlestudio-desktop --locked --offline` is a required passing gate for this boundary. If it fails, do not claim the desktop boundary refactor is verified.

## Non-Goals

- Do not remove asset embedding from web/server/export owners just because desktop should not depend on it.
- Do not add runtime fallback paths from Tauri assets to embedded Rust assets.
- Do not silently substitute WASM, JS, CSS, or generated artifacts from another location when the required owner asset is missing.
- Do not change parser, preview, save, workspace, or export semantics while moving the dependency boundary.
