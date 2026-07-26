# Development Commands

Run these commands from the repository root unless noted otherwise.

## Local Servers

Serve a game in the browser player:

```bash
cargo run -p html-play -- games/animation_test.puzzle --serve
```

Serve the browser editor with a local preview server:

```bash
cargo run -p html-editor -- games/animation_test.puzzle --serve
```

Use the installed CLI facade when you want the product command instead of a
crate-local development command:

```bash
puzzlestudio preview games/animation_test.puzzle
puzzlestudio editor games/animation_test.puzzle
```

The server command prints the local `http://127.0.0.1:<port>` URL after it
starts. Use `--port <port>` with the CLI commands when you need a fixed port.

## Generate Standalone Game HTML

For local development exports, use the wrapper so the WASM bundle is rebuilt
before the HTML is generated:

```bash
tools/dev_export_html.sh games/fixban_tween.puzzle -o games/fixban_tween.html
```

Regenerate every tracked standalone export, including the declared alias
outputs, through the canonical mapping:

```bash
tools/generate_tracked_game_exports.sh
```

Direct crate command, useful when you intentionally do not want the wrapper:

```bash
cargo run -p html-play -- games/animation_test.puzzle -o /tmp/animation_test.html
```

## Generate Web Editor

The editor web release is the generated Pages site under `docs/`, not a root
standalone `editor.html` artifact. Regenerate the Pages entry and adjacent
static assets through the wrapper:

```bash
tools/install_editor_frontend_deps.sh
tools/generate_web_editor.sh -o docs/index.html
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
puzzlestudio check games/animation_test.puzzle
```

Check through Cargo without relying on an installed CLI binary:

```bash
cargo run -p puzzlestudio -- check games/animation_test.puzzle
```

## Standalone Player Acceptance

Regenerate the release WASM artifacts before validating an official export:

```bash
CARGO_TARGET_DIR=/private/tmp/puzzlebuilder-bevy-target tools/ensure_wasm_fresh.sh desktop
```

Run the committed browser acceptance suite for representative 2D and 3D
exports plus visibility, progress persistence, external-image rendering, and
missing-asset diagnostics:

```bash
CARGO_TARGET_DIR=/private/tmp/puzzlebuilder-bevy-target tools/check_standalone_browser_contracts.sh
```

The suite reads its performance thresholds from
`tools/standalone_player_browser_baseline.json`; do not duplicate those limits
in another runner.
