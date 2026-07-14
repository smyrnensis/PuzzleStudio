# Agent Notes

This crate owns `.puzzle` surface syntax, validation, compatibility imports,
semantic scanning/highlighting, and lowering.

## Syntax Ownership

Canonical authoring should reuse existing concepts rather than add one-off
mini-languages. Prefer owner-scoped `keys`, routines, lifecycle hooks, scene
commands, and explicit component contracts.

`rules { ... }` is the required puzzle entrypoint. Legacy puzzle
`transitions` / `main` blocks and `rule` declarations are rejected.

Levels may attach turn rules with `rules before { ... }`, `rules after { ... }`,
or `rules { ... }` as the `after` shorthand. Language lowering owns the
effective per-level program in the fixed order `before`, puzzle `rules`,
`after`; runtime and solver consumers must select that same compiled program by
level rather than recomposing or interpreting source syntax.

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

Scene text syntax lowers `heading`, `subheading`, `text`, and `caption` through
one text-component path. Top-scope `title`, `subtitle`, `author`, and `homepage`
remain content values and must not select a component kind. Scene layout syntax
lowers allocation, ratio, cross-axis alignment, and main-axis distribution into
the typed `puzzle-scene` contract; it must not encode CSS or adapter sizing.

## Surface/Highlighting Direction

Highlighting should remain Rust-owned. Browser/editor fallbacks may escape text,
but must not grow a second JavaScript `.puzzle` grammar.

`highlight.rs` is a projection boundary, not a lexer or tolerant parser. It may
construct a profile-aware `SourceAnalysis` and request a typed full or range
product. It must not inspect source characters, split tokens, recognize comments
or quotes, match braces, infer selector punctuation, or classify owner-specific
leaf content.

The parser frontend owns one lossless `ParseSnapshot` per source revision. Its
lexer may classify only context-free spelling. The structural and owner parsers
that accept contextual syntax must attach token dispositions, diagnostics, and
display facts during that same parse operation. Strict compile and editor
products consume this same snapshot; neither may parse source text again.

`source_lexical_product.rs` may exhaustively map parser-owned dispositions,
semantic facts, and owner-produced display facts to colors. Source text and raw
token payloads are deliberately absent from its function signatures. A viewport
request must use binary range windows; it must not build or filter a
whole-document highlight product first.

If highlighting lacks a fact, extend the grammar owner that already accepts the
syntax. Do not add a recognizer, source string, regex, syntax word list, or
fallback to `highlight.rs`, `source_lexical_product.rs`, a surface scanner, or a
parser-named highlight lexer. New disposition or semantic variants must make the
display mapping fail to compile until mapping is explicit.

Public and host highlighting entrypoints require an explicit `.puzzle` or
`.puzzle3` profile. Do not restore a profile-free overload or infer a default
dimension from source text.

`source_outline.rs` is likewise a projection and wire-format boundary. It may
create a profile-aware `SourceAnalysis`, clone the revision-local cached outline
items, and serialize them. It must not inspect `SurfaceDocument` blocks, headers,
lines, source characters, authoring grammar, scope names, or fixed syntax-word
lists. Canonical block construction must attach typed outline kind, label, and
child-suppression facts to each `SurfaceStructuralBlock` during the parser's
existing structural pass. The parser-owned `source::outline_product` module in
`source_outline_product.rs` may only assemble those typed facts into items; it
must not inspect headers or line content either. Keep its builder call confined
to the lazy `SourceAnalysis` outline cache. This laziness is required: viewport
highlighting and other queries that do not request outline must not construct the
outline product or walk the whole structural tree. Public and host outline
entrypoints require an explicit source profile, and the editor must query the
already-active analysis revision rather than parse a second source snapshot.

`SurfaceDocument`, `SurfaceSourceScan`, and `SurfaceSink` are projections, not
grammar authorities. Establish the lossless `ParseProduct<T>` contract before
migrating owner behavior: every accepted logical piece retains original token
identity and returns dispositions and display facts with its semantic value.
Do not run an owner parser over `Vec<String>` and reconstruct positions later.
Do not keep an owner-by-owner surface recognizer beside the new contract.
`record_*_surface`, `scan_*_surface_ranges`, raw-source recognition, and any
renamed equivalent must be deleted when the common contract is introduced.
Missing facts must remain visible until the accepting parser emits them; a
temporary legacy projector is not an allowed migration bridge.

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
