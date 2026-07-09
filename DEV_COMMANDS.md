# Development Commands

Run these commands from the repository root unless noted otherwise.

## Local Servers

Serve a game in the browser player:

```bash
cargo run -p html-play -- games/spec_2d.puzzle --serve
```

Serve the browser editor with a local preview server:

```bash
cargo run -p html-editor -- games/spec_2d.puzzle --serve
```

Use the installed CLI facade when you want the product command instead of a
crate-local development command:

```bash
puzzlestudio preview games/spec_2d.puzzle
puzzlestudio editor games/spec_2d.puzzle
```

The server command prints the local `http://127.0.0.1:<port>` URL after it
starts. Use `--port <port>` with the CLI commands when you need a fixed port.

## Generate Standalone Game HTML

For local development exports, use the wrapper so the WASM bundle is rebuilt
before the HTML is generated:

```bash
tools/dev_export_html.sh games/fixban_tween.puzzle -o games/fixban_tween.html
```

Direct crate command, useful when you intentionally do not want the wrapper:

```bash
cargo run -p html-play -- games/spec_2d.puzzle -o /tmp/spec_2d.html
```

## Generate Web Editor

The editor web release is the generated Pages site under `docs/`, not a root
standalone `editor.html` artifact. Regenerate the Pages entry and adjacent
static assets through the wrapper:

```bash
tools/generate_web_editor.sh games/spec_2d.puzzle -o docs/index.html
```

## Tauri Desktop Shell

The editor frontend keeps runtime renderer assets as generated distribution
copies. After editing `crates/html_play/static/renderer.js` or
`crates/html_play/static/renderer.css`, sync the Tauri static copy:

```bash
tools/sync_static_assets.sh
```

Run the desktop shell in development mode:

```bash
cargo tauri dev
```

Build the macOS app bundle:

```bash
cargo tauri build --bundles app
```

The Tauri CLI must be installed in the Rust environment for these commands.
Signing, notarization, and installer packaging are separate release steps.

## Quick Validation

Check a `.puzzle` file through the CLI facade:

```bash
puzzlestudio check games/spec_2d.puzzle
```

Check through Cargo without relying on an installed CLI binary:

```bash
cargo run -p puzzlestudio -- check games/spec_2d.puzzle
```
