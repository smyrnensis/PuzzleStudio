# Agent Notes

This crate owns `.puzzle` surface syntax, validation, compatibility imports,
semantic scanning/highlighting, and lowering.

## Syntax Ownership

Canonical authoring should reuse existing concepts rather than add one-off
mini-languages. Prefer owner-scoped `inputs`, `if` guards, routines, lifecycle
hooks, scene commands, and explicit component contracts.

`rules { ... }` is the required puzzle entrypoint. Legacy puzzle
`transitions` / `main` blocks and `rule` declarations are rejected.

Direction inputs `up`, `down`, `left`, and `right` are built in, with standard
direction mappings. Optional direction aliases should map to those semantic
directions rather than expose numeric direction syntax as public authoring.

`for <binding> in directions|horizontal|vertical` is value expansion, not a UI
menu primitive. Do not make generic loops navigable or level-aware.

Use `on_*` only for scoped lifecycle. Puzzle lifecycle hooks include
`on_level_start` and `on_level_clear`; scene lifecycle has its own scope.

`inputs { <input> <- <key...> }` is owner-scoped. Model inputs map raw keys to
model semantic inputs; scene inputs/keys map raw keys to scene semantic inputs
or explicit scene commands.

## Surface/Highlighting Direction

Highlighting should remain Rust-owned. Browser/editor fallbacks may escape text,
but must not grow a second JavaScript `.puzzle` grammar.

The surface document path is moving toward typed `SurfaceDocument` /
`SurfaceNode` / `SurfaceSink` data. Continue migrating parser, highlight, and
completion behavior toward the shared token/span pipeline instead of duplicating
effect vocabularies.

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
