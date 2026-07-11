# Agent Notes

This crate owns `.puzzle` surface syntax, validation, compatibility imports,
semantic scanning/highlighting, and lowering.

## Syntax Ownership

Canonical authoring should reuse existing concepts rather than add one-off
mini-languages. Prefer owner-scoped `keys`, routines, lifecycle hooks, scene
commands, and explicit component contracts.

`rules { ... }` is the required puzzle entrypoint. Legacy puzzle
`transitions` / `main` blocks and `rule` declarations are rejected.

Direction inputs `up`, `down`, `left`, and `right` are built in, with standard
direction mappings. Optional direction aliases should map to those semantic
directions rather than expose numeric direction syntax as public authoring.

`for <binding> in directions|horizontal|vertical` is value expansion, not a UI
menu primitive. Do not make generic loops navigable or level-aware.

Use `on_*` only for scoped lifecycle. Puzzle lifecycle hooks include
`on_level_start` and `on_level_clear`; scene lifecycle has its own scope.

`keys { <key...> -> <input-or-routine> }` is owner-scoped. Model keys map raw
keys to model semantic inputs; scene keys map raw keys to scene routines, input
effects, or explicit scene commands. The semantic input concept remains, but
`inputs { <input> <- <key...> }` is not canonical syntax.

## Surface/Highlighting Direction

Highlighting should remain Rust-owned. Browser/editor fallbacks may escape text,
but must not grow a second JavaScript `.puzzle` grammar.

The surface document path is moving toward typed `SurfaceDocument` /
`SurfaceNode` / `SurfaceSink` data. Continue migrating parser, highlight, and
completion behavior toward the shared token/span pipeline instead of duplicating
effect vocabularies.

Sprite body parsing must preserve the distinction between explicit properties and
owner-resolved bare content. A bare row after colors may be inline ASCII or a
declared shape reference; resolve that once through the shared sprite authoring
resolver after the complete `sprites` owner scope is known. Compile, highlight,
completion/source refs, and source-target projection must consume that shared
decision instead of independently reclassifying the row. If a declared shape
name is also valid ASCII for the active palette, fail visibly and require
`shape = <name>` or `shape = { ... }` to disambiguate.

## PuzzleScript Import

PuzzleScript import is intentionally minimal and compatibility-oriented. It
translates supported PuzzleScript subsets into canonical `.puzzle` instead of
widening canonical syntax directly. Keep pinned mapping fixtures under this
crate's tests when expanding the import surface.

## Tests

Use focused crate tests and fixtures first:

```bash
cargo test -p puzzle-lang
cargo run -p puzzlestudio -- check games/spec_2d.puzzle
```
