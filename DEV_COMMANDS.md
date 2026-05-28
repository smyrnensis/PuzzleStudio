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

## Generate `editor.html`

Root `editor.html` is a generated standalone editor artifact. Do not edit it by
hand. Regenerate it from the editor source and the current WASM runtime.

Before regenerating, check whether the artifact is already dirty:

```bash
git status --short -- editor.html crates/html_editor/static/wasm
```

Pick a seed `.puzzle` file that passes the current checker. The usual sample is:

```bash
cargo run -p puzzlestudio -- check games/spec_2d.puzzle
```

If that fails because the worktree is in the middle of syntax or sample changes,
use another checked sample instead. For example:

```bash
cargo run -p puzzlestudio -- check games/spec_3d.puzzle
```

Regenerate the root standalone editor artifact with the checked seed:

```bash
tools/generate_editor.sh games/spec_2d.puzzle -o editor.html
```

Equivalent explicit release path:

```bash
tools/release_editor_html.sh games/spec_2d.puzzle -o editor.html
```

Both wrapper scripts rebuild the editor WASM before exporting. The generated
`editor.html` embeds that WASM and keeps it fixed until regenerated.

If export fails with a parser or validation error, do not patch `editor.html`.
Fix or choose a valid seed `.puzzle`, then rerun the wrapper. A successful
regeneration should leave changes in `editor.html` and may also update the
embedded editor WASM files under `crates/html_editor/static/wasm`.

After regenerating, inspect the generated-artifact diff:

```bash
git status --short -- editor.html crates/html_editor/static/wasm
git diff --stat -- editor.html crates/html_editor/static/wasm
```

For static web hosting output:

```bash
tools/generate_web_editor.sh games/spec_2d.puzzle -o docs/index.html
```

## Tauri Desktop Shell

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
