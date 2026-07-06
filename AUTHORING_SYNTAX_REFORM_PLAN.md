# Authoring Syntax Reform Plan

This document defines the long-term requirements for reforming the
PuzzleStudio authoring parser. The goal is not to make large files smaller for
its own sake. The goal is to make syntax changes, owner-specific semantics, and
editor feedback easier to develop without duplicating the same surface rules in
many parser branches.

## Objective

Improve development ergonomics by making authoring syntax a first-class
implementation concept.

A future change should usually answer these questions without searching through
unrelated owner parsers:

- Which surface syntax primitive is being used?
- Which owner is allowed to interpret it?
- Which layer rejects unsupported use?
- Which parser result feeds diagnostics, highlighting, completion, and
  lowering?

The reform assumes a long-lived codebase. Short-term patch size, local file
size reduction, and one-off bug fixes are not success criteria unless they also
move the parser toward a stable syntax/owner boundary.

## Problem

The current parser shows signs that syntax is not uniformly modeled.

Visible symptoms include:

- `->` rows are parsed separately for keys, buttons, rewrite suffixes, and
  puzzle rules.
- Removed scene transition rows still appear as implementation/test vocabulary,
  which makes old authoring forms look like migration candidates instead of
  deletion targets.
- `=` assignment rows are parsed separately for state, theme, variables,
  options, and resource-like declarations.
- call syntax and top-level delimiter scanning are implemented near scene
  expressions, even though calls and delimited argument lists are not inherently
  scene-specific.
- block collection logic differs across levels, statement blocks, effects,
  sprites, visual tables, and raw ASCII bodies.
- highlighting and completion often need to rediscover the same surface roles
  that parsing already knows.

The deeper issue is not duplication alone. The implementation often lets an
owner-specific parser own the surface syntax itself. That makes the system easy
to extend locally but hard to keep coherent globally.

## Core Principle

Common authoring syntax must be parsed once as a surface form. Owners then
interpret, validate, or reject that surface form.

In other words:

```txt
source text
  -> universal surface syntax
  -> owner adapter
  -> validated owner meaning
  -> lowering/runtime/editor outputs
```

The surface layer owns questions such as:

- Where is the arrow in `lhs -> rhs`?
- Is the left side empty?
- Where are the spans for diagnostics and semantic tokens?
- How are top-level commas split without entering strings, calls, braces, or
  bracketed patterns?
- Is this body structured, or intentionally raw owner content?

The owner adapter owns questions such as:

- Is this row valid in a model, scene, level, visual, or rule body?
- Does the right side mean an input, an effect, a state initializer, or a
  rewrite payload?
- Which names resolve in this scope?
- Which forms are rejected even though the surface syntax is well-formed?

## Requirements

### 1. Developer Predictability

Adding or changing syntax must have a predictable edit path.

Expected shape:

- Add or modify a surface primitive in the syntax layer.
- Add or modify one or more owner adapters.
- Add focused tests for surface parsing, owner acceptance, owner rejection, and
  diagnostics.

A change should not require rediscovering every ad hoc parser that happens to
split the same token sequence.

### 2. Surface Syntax Is Owner-Neutral

Syntax primitives must not encode one current owner unless the syntax is truly
owned by that construct.

Examples of owner-neutral primitives:

- arrow row: `<lhs> -> <rhs>`
- assignment row: `<name> = <value>`
- call expression: `<name>(<args...>)`
- identifier path: `a.b.c`
- braced block body
- line-style body
- raw body
- top-level delimiter splitting

Examples of owner-specific interpretation:

- model keys map key triggers to model inputs.
- scene keys map key triggers to scene routines, scene effects, or model input
  dispatch.
- a level body may treat rows as ASCII map content.
- a sprite body may treat rows as pixel content.
- a rule statement may treat `->` as rewrite syntax.

### 3. Owners Interpret, They Do Not Re-Parse Surface Rules

Owner adapters should receive typed surface nodes or shared surface parse
results. They should not manually split `->`, `=`, call args, or top-level
commas unless they are defining a genuinely new syntax primitive.

Allowed owner behavior:

- resolve names against owner scope
- enforce owner-specific arity and shape
- reject unsupported but syntactically valid surface nodes
- lower surface nodes into owner-specific AST or runtime data

Disallowed owner behavior:

- silently reinterpreting malformed surface syntax
- implementing a local fallback parser for a shared primitive
- accepting a legacy variant because it is convenient in one owner
- duplicating span logic needed by diagnostics, highlighting, or completion

### 4. Unified Syntax Does Not Mean Uniform Semantics

The same surface primitive may have different owner meanings.

For example, `keys { <key...> -> <rhs> }` should share row parsing. The `rhs`
meaning may still differ:

- in a model, `rhs` is a model input name
- in a scene, `rhs` may be an input dispatch, routine call, or effect

Uniform syntax requires common parsing and diagnostics for the row shape. It
does not require every owner to accept the same right-hand side.

### 5. Raw Owner Content Is Explicit

Some bodies should not be parsed as generic statement rows.

Examples:

- level ASCII rows
- sprite pixel rows
- visual shape tables

These should be represented as explicit raw owner content, not accidental
exceptions in a generic tree walker. The parser should make the boundary visible
in type names, function names, and tests.

### 6. Diagnostics, Highlighting, And Completion Share Surface Evidence

Parser behavior, semantic highlighting, completion, and source-target lookup
should converge on the same surface parse results.

This does not require one giant AST. It does require that grammar facts are not
re-implemented independently in browser/editor code or unrelated Rust scanners.

When a token is classified as an effect command, state name, object selector,
asset reference, or raw pixel row, that classification should be traceable to a
surface/owner rule rather than a duplicated keyword list.

### 7. No Fallback Paths

If the surface layer cannot parse a shared primitive, owner adapters must not
rescue the text with a nearby local interpretation.

The correct choices are:

- extend the surface primitive
- reject the syntax with an owner-specific diagnostic
- mark a temporary migration bridge explicitly, including its deletion
  condition

## Non-Goals

This reform is not:

- a rewrite of the whole parser at once
- a new grammar framework requirement
- a promise that all authoring forms become uniform
- a reason to erase owner-specific rules
- a cleanup pass that only moves functions between files
- a compatibility layer for removed syntax

The reform should reduce accidental diversity, not legitimate ownership
boundaries.

## Target Architecture

### Surface Syntax Layer

Proposed owner: `crates/lang/src/lib_authoring_parse_syntax.rs`, or a future
module with the same responsibility.

Responsibilities:

- parse universal row forms
- preserve source spans where needed
- split top-level delimiters
- parse identifier/path/call shells
- classify structured body versus raw owner content
- provide small typed surface nodes

Candidate types:

```rust
struct ArrowRow<'a> {
    lhs: &'a str,
    rhs: &'a str,
}

struct AssignmentRow<'a> {
    name: &'a str,
    value: &'a str,
}

struct CallSurface<'a> {
    name: &'a str,
    args: Vec<&'a str>,
}

struct KeysSurfaceRow<'a> {
    keys: Vec<&'a str>,
    target: &'a str,
}

enum BodySurface<'a> {
    Structured(Vec<SurfaceEntry<'a>>),
    RawLines(Vec<&'a str>),
}
```

These sketches are not final APIs. The requirement is the direction: surface
syntax should be typed before owner interpretation.

### Owner Adapters

Owner adapters convert surface nodes into owned meaning.

Examples:

```txt
KeysSurfaceRow
  -> model keys adapter
  -> Controls input binding

KeysSurfaceRow
  -> scene keys adapter
  -> KeyBinding with SceneEffect

ArrowRow
  -> rule statement adapter
  -> rewrite AST

AssignmentRow
  -> scene state adapter
  -> SceneVarDef or ScenePuzzleDef
```

Each adapter should be small enough to answer:

- what surface form it consumes
- what scope it resolves against
- what it emits
- what it rejects

### Lowering And Runtime

Lowering and runtime should consume validated owner meaning, not source syntax.

They should not parse authoring constructs, recover missing syntax, or infer
semantics from presentation results.

## Migration Strategy

Migrate vertically by syntax primitive, not horizontally by file.

Each migration should include:

- one surface primitive
- at least two owner call sites when possible
- focused tests for both shared syntax and owner-specific interpretation
- no behavior change unless explicitly documented

### Package 1: Keys Rows

Goal: establish the first surface-to-owner adapter pattern.

Tasks:

- represent `keys { <key...> -> <target> }` as a typed surface row
- keep model keys and scene keys as separate owner adapters
- verify model input binding and scene effect/input binding both use the shared
  row parser
- preserve diagnostics for missing `->`, empty key list, duplicate model key,
  and scene `=` rejection

Validation:

```bash
cargo test -p puzzle-lang scene_keys
cargo test -p puzzle-lang model_sounds_parse_undo_and_restart_sfx_operations
```

The second command is not about keys directly; it is a cheap guard that the
split authoring parser still reaches model-owned blocks correctly.

### Package 2: Assignment Rows

Goal: remove duplicated `name = value` parsing.

Candidate owners:

- scene state variables
- top-level variables
- theme settings
- resource/state declarations where applicable
- model settings that already use assignment shape

Rules:

- surface layer parses the assignment shape
- owner adapter decides accepted names, mutability, value type, and diagnostics
- compatibility assignment forms must be named as compatibility, not silently
  folded into canonical syntax

### Package 3: Arrow Rows

Goal: make `->` a shared surface primitive while preserving owner meaning.

Candidate owners:

- keys
- button effect rows
- rule statement arrows
- rewrite rows

Boundary:

Pattern rewrite parsing may need richer surface structure than a simple
`ArrowRow`, because bracketed pattern sides have their own grammar. The first
step should still share arrow location, span, and missing-arrow diagnostics
where the primitive is the same.

Removed scene transition rows are not part of this package. Scene-owned
conditions use scene-level `if <condition> { ... }`, lifecycle uses
`on_scene_start { ... }`, and input dispatch uses `keys` plus routines/effects.

### Package 4: Call And Argument Surfaces

Goal: move call-shell parsing out of scene-specific naming.

Candidate owners:

- scene expressions
- level selectors
- rule calls
- effect calls
- future macro/routine calls

Rules:

- surface layer parses name, top-level argument slices, and delimiter errors
- owner adapter parses argument values according to owner scope
- top-level comma and delimiter scanning must be shared

### Package 5: Body Collection And Raw Boundaries

Goal: make block traversal and raw content ownership explicit.

Candidate bodies:

- statement blocks
- effect blocks
- level bodies
- sprite bodies
- visual tables
- scene component trees

Rules:

- common body collection tracks braces and owner boundaries
- raw owner bodies are declared as raw, not accidental parser exceptions
- source line numbers and spans are preserved for diagnostics

## Acceptance Criteria

The reform is making real progress when:

- adding a new owner using `->` does not require a new manual `split_once("->")`
  path
- adding a new assignment-style declaration reuses assignment surface parsing
- diagnostics for the same surface error are consistent across owners unless an
  owner intentionally overrides the message
- highlighting/completion can point to the same surface role parser uses
- owner adapters are smaller than the surface primitives they consume
- tests name both the shared surface rule and the owner-specific interpretation

The reform is not making progress when:

- files get smaller but syntax decisions remain duplicated
- a helper only wraps `split_once` without carrying ownership, diagnostics, or
  spans forward
- owner adapters still accept malformed syntax through local fallbacks
- a feature works only because one current owner special-cases it
- generated/editor/highlight surfaces grow their own parallel grammar

## Immediate Next Step

Before broad implementation, finish the `keys` vertical slice cleanly:

1. Define a typed `KeysSurfaceRow`.
2. Lower it through separate model and scene key adapters.
3. Run focused `puzzle-lang` key tests.
4. Confirm broad `puzzle-lang` failures, if any, are unrelated to the slice.
5. Only then use the same pattern for assignment rows.

This keeps the work aligned with the long-term requirement while avoiding a
large, risky parser rewrite.
