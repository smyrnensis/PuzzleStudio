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

Install or refresh the local CLI:

```bash
cargo install --path crates/cli
```

Run the main validation command:

```bash
puzzlestudio check games/spec_2d/game.puzzle
```

Play the 2D sample in a terminal:

```bash
puzzlestudio play games/spec_2d/game.puzzle
```

Export a standalone HTML build:

```bash
puzzlestudio export-html games/spec_2d/game.puzzle -o /tmp/spec_2d.html
```

Serve the browser player locally:

```bash
puzzlestudio preview games/spec_2d/game.puzzle
```

The server prints a `http://127.0.0.1:<port>` URL after it starts.

## Player Controls

The default 2D sample uses these controls:

- `w/a/s/d` or arrow keys: move
- `r`: send the standard `restart` input
- `q`: quit the terminal player

Input handling is part of the `.puzzle` model. Other games can map keys to
different semantic inputs with an `inputs { ... }` block.

## Editor

Run the browser editor with a local preview server:

```bash
puzzlestudio editor games/spec_2d/game.puzzle
```

By default this serves `http://127.0.0.1:8787/editor.html`. Use `--port` to pick
a different port.

Generate a standalone editor HTML file:

```bash
puzzlestudio export-editor games/spec_2d/game.puzzle -o editor.html
```

If editor JavaScript, WASM, or preview code has changed, refresh the editor WASM
bundle before exporting:

```bash
tools/build_wasm_editor.sh
```

For static web hosting, generate the standalone editor plus WASM preview
fallback:

```bash
tools/generate_web_editor.sh games/spec_2d/game.puzzle -o docs/index.html
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
puzzlestudio play [path]
puzzlestudio preview [path] [--port 7878]
puzzlestudio editor [path] [--port 8787]
puzzlestudio export-html <path> -o <output.html>
puzzlestudio export-editor [path] -o <editor.html>
puzzlestudio import-puzzlescript <source.txt> -o <game.puzzle>
```

`<path>` can be a game folder or a `.puzzle` file. Folder paths resolve to the
best game entry in that folder.

## Game Entries And Imports

A game entry is a `.puzzle` file with top-level game metadata such as:

```txt
title "Microban"
author "David Skinner"
```

When a folder is passed to a tool, PuzzleStudio looks for a prelude-bearing
entry file in that folder. `game.puzzle` is preferred, but it is a convention,
not a requirement.

Files without top-level game metadata are import fragments. They are not loaded
automatically; the entry file must import them explicitly.

## Project Layout

```txt
crates/core/            deterministic transition core
crates/lang/            .puzzle parser, validation, lowering, imports
crates/scene/           shared scene/layout data structures
crates/play/            loaded-game session mechanics and render helpers
crates/solver/          search support for puzzle states
crates/ascii_play/      terminal adapter
crates/html_play/       browser player and standalone HTML export
crates/html_editor/     browser editor and editor export
crates/wasm/            WASM bridge used by editor/player workflows
crates/puzzle3d_model/  prototype 3D model parser/runtime
src-tauri/              desktop shell for the shared editor
games/                  current small specification/sample games
themes/                 built-in HTML theme imports
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
puzzlestudio check games/spec_2d/game.puzzle
```

Export smoke test:

```bash
puzzlestudio export-html games/spec_2d/game.puzzle -o /tmp/spec_2d.html
```
