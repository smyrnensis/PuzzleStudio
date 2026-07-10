# Agent Handoff

This document is the general handoff for later agents. It intentionally avoids
deep folder-specific notes. For concrete implementation details, read the nearest
`AGENTS.md` in the folder you are changing.

## Current Goal

The project is growing a lightweight environment for turn-based, grid-based,
rule-driven puzzle games while keeping deterministic transition logic separate
from `.puzzle` language processing, play/session behavior, adapters, and editor
hosts.

```txt
.puzzle file
  -> language parser/compiler
  -> deterministic compiled game/state model
  -> play/session helpers
  -> adapter/editor presentation
```

## Repository Context

This checkout contains large generated artifacts and build outputs. Do not start
by reading raw file contents across the tree. First identify the owner boundary,
then read the smallest source files and that folder's `AGENTS.md`.

Measured on 2026-05-28:

- Whole checkout: about `11G`
- `.git`: about `9.0G`
- root build output: about `1.1G`
- some nested test/build outputs are large enough to distort context selection
- Files excluding version-control and build outputs: `661`

Largest context traps are generated exports, generated documentation,
WebAssembly binaries, legacy archives, and build caches. Do not patch generated
outputs directly. Patch the source owner and regenerate only when that is the
explicit intent.

## Owner Map

Read the corresponding folder `AGENTS.md` before editing these areas:

- `crates/`: shared crate boundaries and package-level commands.
- `crates/core/`: deterministic state, patches, guards, transition application.
- `crates/grid3d/`: deterministic 3D grid primitives, state, patches, levels,
  win checks, and transition application.
- `crates/lang/`: `.puzzle` parsing, validation, authoring syntax, lowering,
  semantic highlighting, and import compatibility.
- `crates/play/`: loaded-game session mechanics, undo/restart/level flow, and
  display helpers.
- `crates/scene/`: shared scene/presentation metadata and component layout
  contracts.
- `crates/html_play/`: browser runtime, standalone export behavior, screenshots,
  themes, audio, and generated HTML runtime surfaces.
- `crates/html_editor/`: browser editor service/UI, preview compilation,
  highlighting, workspace behavior, and editor-owned layout.
- `crates/ascii_play/`: terminal adapter behavior.
- `crates/cli/`: product/automation facade and command routing.
- `src-tauri/`: desktop shell and host filesystem boundary.
- `games/`: sample authoring inputs and generated standalone game exports.
- `docs/`: generated web documentation exports and documentation source policy.
- `archive/`: legacy/reference material.
- `wasm/`: generated WebAssembly artifacts.

## General Run / Test

Use owner-local commands whenever possible. Broad commands are useful only when
their full blast radius is intended.

```bash
cargo test
cargo run -p puzzlestudio -- check games/spec_2d.puzzle
```

For generated web artifacts, use the wrapper that matches the intended audience
instead of calling the crate directly:

```bash
tools/dev_export_html.sh games/fixban_tween.puzzle -o games/fixban_tween.html
tools/generate_web_editor.sh games/fixban_tween.puzzle -o docs/index.html
```

`dev_export_html.sh` rebuilds WASM before game HTML export so local developer
checks see the current Rust implementation. `generate_web_editor.sh` produces
the GitHub Pages editor release as `docs/index.html` plus static JS, CSS, and
WASM assets. Rebuild editor/core/game WASM explicitly with
`tools/build_wasm_editor.sh`, `tools/build_wasm_core.sh`, and
`tools/build_wasm_game.sh` when Rust/WASM changes are meant to appear in the
Pages release.

Use `tools/serve_web_editor.sh` to open the normal local web editor through the
Rust editor server. On macOS, `tools/open_web_editor.command` is the
double-click entry point. Use `tools/serve_web_editor.sh --pages` only when the
intended check is the generated GitHub Pages site under `docs/`; that static
mode does not provide Rust editor API routes such as `/api/highlight`. Do not
use `file://` as a supported editor release surface.

Do not reintroduce a root single-file `editor.html` release path. GitHub Pages is
the web release surface, and it should stay a multi-file static site.

Adapter checks, editor checks, screenshots, and desktop builds have
owner-specific caveats; read the relevant folder `AGENTS.md` before treating a
command as authoritative.

## General Design Boundaries

Core logic must stay deterministic and independent from file IO, parser
concerns, rendering, sound playback, browser timers, terminal rendering, and
game-specific UI behavior.

Language processing owns surface syntax and lowering. Runtime/session layers
should consume checked/compiled structures rather than reinterpreting source
syntax.

Play/session logic owns loaded-game mechanics such as undo, restart, level
advance, screen flow, component dispatch, and post-turn lifecycle behavior.

Adapters own presentation and host integration. They should not become the
source of parser/compiler semantics.

Shared scene metadata is presentation/flow structure, not ownership of 2D or 3D
model internals. Components and model windows define their own behavior behind
clear contracts.

Editor and desktop hosts should share service behavior as much as possible.
Platform divergence should happen at the host adapter/file-access boundary, not
by forking parser, compile, preview, or highlighting logic.

## Documentation Split

Top-level docs are split by audience:

- User-facing docs explain how authors write and run `.puzzle` projects.
- Developer-facing docs explain ownership boundaries, syntax decisions,
  lowering/runtime constraints, and implementation plans.
- Folder `AGENTS.md` files carry operational handoff details for each owner.

The canonical parser/editor source-analysis boundary is specified in
`SOURCE_ANALYSIS_CONTRACT.md`. Read it before changing `SurfaceDocument`,
`SourceAnalysis`, analysis profiles, source offsets, or language-aware editor
integration.

When a feature changes, update the user-facing explanation, developer-facing
principle/spec, or owner-specific agent handoff according to the audience that
needs the information.

## Known General Gaps

- Some language/import compatibility paths are intentionally partial.
- Solver work is still limited compared with parser/runtime/editor work.
- Transition hot paths still have optimization opportunities.
- Trace output is useful but not yet a complete debugging model.
- Several adapter surfaces are still converging on shared contracts.
