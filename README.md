# PuzzleStudio

PuzzleStudio is an experimental authoring environment for turn-based,
grid-based, rule-driven puzzle games.

The project is intentionally narrow. It is not trying to be a general-purpose
game engine. It is built around a short feedback loop between human intent,
AI-assisted rule editing, deterministic execution, and concrete inspection of
game behavior.

At the moment, the repository provides:

- a `.puzzle` authoring language for 2D and prototype 3D puzzle games
- a deterministic Rust transition core
- a parser/compiler that lowers `.puzzle` files into runtime data
- terminal, browser, standalone HTML, editor, and desktop-shell entry points
- early solver and inspection support for authoring workflows

## Quick Start

Install or refresh the local CLI for source validation:

```bash
cargo install --path crates/cli
```

Install the full CLI façade, including adapter commands such as terminal play,
HTML preview, and editor export:

```bash
cargo install --path crates/cli --features adapters
```

Run the main validation command:

```bash
puzzlestudio check games/spec_2d.puzzle
```

Play the 2D sample in a terminal:

```bash
puzzlestudio play games/spec_2d.puzzle
```

Export a standalone HTML build:

```bash
puzzlestudio export-html games/spec_2d.puzzle -o /tmp/spec_2d.html
```

Capture a browser-rendered PNG screenshot:

```bash
puzzlestudio screenshot games/spec_3d.puzzle -o /tmp/spec_3d.png
```

Screenshot capture uses Chrome/Chromium headless. If it is not auto-detected,
pass `--browser <path>` or set `PUZZLESTUDIO_CHROME`.

Serve the browser player locally:

```bash
puzzlestudio preview games/spec_2d.puzzle
```

The server prints a `http://127.0.0.1:<port>` URL after it starts.

During repository development, prefer `cargo run -p ... -- ...` when checking
changes you just made. The installed `puzzlestudio` command and
`target/debug/puzzlestudio` are build artifacts; static CSS, JS, and WASM are
embedded at Rust build time and can be stale after asset edits.

## Player Controls

The default 2D sample uses these controls:

- `w/a/s/d` or arrow keys: move
- `r`: send the standard `restart` input
- `q`: quit the terminal player

Input handling is part of the `.puzzle` model. Other games can map raw keys to
semantic inputs with a `keys { <key...> -> <input> }` block.

## Editor

Run the browser editor with a local preview server:

```bash
puzzlestudio editor games/spec_2d.puzzle
```

By default this serves `http://127.0.0.1:8787/editor.html`. Use `--port` to pick
a different port.

During editor development, prefer the served editor or the owner-local command:

```bash
cargo run -p html-editor -- games/spec_2d.puzzle --serve
```

This keeps the feedback loop short. The served `editor.html` is the editor app
entry inside the static assets, not the web release artifact.

Generate the GitHub Pages editor release:

```bash
tools/generate_web_editor.sh games/spec_2d.puzzle -o docs/index.html
```

This writes `docs/index.html` plus the JavaScript, CSS, and WASM assets that
GitHub Pages serves as static files. If Rust/WASM code changed, rebuild the
generated WASM artifacts first:

```bash
tools/build_wasm_editor.sh
tools/build_wasm_core.sh
tools/build_wasm_game.sh
```

Open the generated Pages editor locally over HTTP:

```bash
tools/serve_web_editor.sh
```

On macOS, `tools/open_web_editor.command` can be opened from Finder. The Pages
editor is not a `file://` artifact; serve `docs/` over local HTTP for the same
asset-loading shape used by GitHub Pages.

The CLI adapter command exports the same Pages-style editor HTML entry:

```bash
puzzlestudio export-editor games/spec_2d.puzzle -o docs/index.html
```

## 3D Prototype

There is also a prototype 3D `.puzzle` model path:

```bash
puzzlestudio preview games/spec_3d.puzzle
```

The 3D path shares the scene and browser adapter direction, but it is still
more experimental than the 2D rules path.

## Desktop Shell

The Tauri desktop shell hosts the shared HTML editor. If the Tauri CLI is
installed in your Rust environment, run:

```bash
cargo tauri dev
```

The desktop shell starts empty. It should only read or write project files after
the user opens a project folder or game entry.

## CLI Commands

The `puzzlestudio-cli` package installs a `puzzlestudio` binary with:

```bash
puzzlestudio check <path> [--json]
puzzlestudio import-puzzlescript <source.txt> -o <game.puzzle>
```

Adapter façade commands are included when the CLI is built with
`--features adapters`:

```bash
puzzlestudio play [path]
puzzlestudio preview [path] [--port 7878]
puzzlestudio editor [path] [--port 8787]
puzzlestudio export-html <path> -o <output.html>
puzzlestudio export-editor [path] -o <docs/index.html>
puzzlestudio screenshot <path> -o <output.png> [--scene name] [--width 1280] [--height 720]
```

`<path>` can be a game folder or a `.puzzle` file. Folder paths resolve to the
best game entry in that folder.

The CLI is the stable product / automation façade: command names, exit codes,
diagnostics, JSON output, and explicit output policy belong there. It is not the
only development entry point. The default source build keeps `check` independent
from adapter crates so parser validation is not blocked by terminal or browser
adapter build errors. For adapter-owned work, use the owner-local
commands directly:

```bash
cargo run -p html-play -- games/spec_2d.puzzle --serve
cargo run -p html-play -- games/spec_2d.puzzle -o /tmp/spec_2d.html
cargo run -p html-play -- games/spec_3d.puzzle --screenshot /tmp/spec_3d.png
cargo run -p html-editor -- games/spec_2d.puzzle --serve
cargo run -p ascii-play -- games/spec_2d.puzzle
```

## Game Entries And Imports

A game entry is a `.puzzle` or `.puzzle3` file that declares a top-level puzzle
model, such as:

```txt
puzzle sokoban {
  rules {
  }
}
```

Top-level metadata such as `title`, `author`, and `homepage` is display
metadata; it does not make a source file a game entry by itself. When a folder
is passed to a tool, PuzzleStudio looks for a model-declaring entry file in that
folder. `game.puzzle` is preferred, but it is a convention, not a requirement.

Files without a top-level puzzle model are import fragments. They are not loaded
automatically; the entry file must import them explicitly.

## Project Layout

```txt
crates/core/            deterministic transition core
crates/grid3d/          deterministic 3D grid/state/transition core
crates/lang/            .puzzle parser, validation, lowering, imports
crates/scene/           shared scene/layout data structures
crates/play/            loaded-game session mechanics and render helpers
crates/solver/          search support for puzzle states
crates/ascii_play/      terminal adapter
crates/html_play/       browser player and standalone HTML export
crates/html_editor/     browser editor and editor export
crates/wasm/            WASM bridge used by editor/player workflows
crates/puzzle_3d/       3D authoring/runtime facade pending full layer split
src-tauri/              desktop shell for the shared editor
games/                  current small specification/sample games
archive/games/          older experiments and compatibility samples
tools/                  export scripts and authoring utilities
```

## Architecture

The main data path is:

```txt
.puzzle source
  -> puzzle-lang parser/compiler
  -> puzzle-core CompiledGame
  -> puzzle-play session/render helpers
  -> ascii-play / html-play / html-editor / desktop shell
```

The important boundary is that `puzzle-core` stays deterministic and independent
from file IO, parsing, rendering, terminal input, browser behavior, and
game-specific UI shortcuts. Syntax and validation belong to `puzzle-lang`;
session behavior belongs to `puzzle-play`; adapters present and host the result.

## Documentation

User-facing documentation:

- `AUTHORING_SYNTAX.md`: canonical `.puzzle` authoring reference
- `README.md`: repository entry point, commands, and orientation

Developer-facing documentation:

- `DESIGN_PRINCIPLES.md`: project philosophy and design constraints
- `CURRENT_SPEC.md`: current parser/runtime/adapter behavior
- `AGENT_HANDOFF.md`: compact implementation map and recent design state
- `SOLVER_DESIGN.md`: solver role and design notes
- `EDITOR_TESTING_STRATEGY.md`: editor service and browser testing strategy
- `PUBLICATION_PLAN.md`: release and hosting direction
- `EDITOR_COMPLETION_PLAN.md`: editor completion plan

When changing behavior, keep user-facing syntax guidance separate from
developer-facing implementation rationale.

## Development Checks

Run the full test suite:

```bash
cargo test
```

Run a focused syntax/runtime smoke check:

```bash
puzzlestudio check games/spec_2d.puzzle
```

Export smoke test:

```bash
puzzlestudio export-html games/spec_2d.puzzle -o /tmp/spec_2d.html
```
