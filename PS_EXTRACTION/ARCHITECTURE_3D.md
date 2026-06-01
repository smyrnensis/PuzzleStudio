# 3D Runtime Architecture

This document fixes the architecture direction for 3D inside the local
PuzzleScript Next extraction checkout.

## Decision

New guiding principle:

```txt
3D code is 2D code with two extra spatial directions.
```

2D and 3D must be built in the same form and at the same abstraction level.
The only acceptable semantic differences are those forced by space itself:
extra axis/depth, two extra directions, 3D neighbor/indexing, and any rule-frame
orientation needed to interpret spatial patterns.

Do not turn missing work into a 3D design difference. If a PuzzleScript feature
has no spatial reason to differ, then it must be shared or duplicated exactly
with the same contract.

The intended shape is therefore:

```txt
source text
  -> parser / compiler front door
  -> same PuzzleScript semantic pipeline
  -> dimension parameter: 2D or 3D
  -> same rule/session/turn contracts
       -> board/index/neighbors differ by dimension
       -> renderer differs by dimension
```

The old "small separate 3D runtime beside 2D" MVP framing is no longer the
target architecture. It was useful for experiments, but it allowed feature
omissions to look like design differences.

## Why

PuzzleScript semantics are not 2D-specific unless they depend on board
adjacency, direction sets, spatial scans, or rendering. Features such as
`late`, commands, loops, gosub, tags, mappings, win conditions, random rule
choice, checkpoint, undo/restart policy, sound events, and metadata handling do
not become different merely because the board has depth.

The previous prototype matched some 2D low-level masks but skipped many higher
level PuzzleScript semantics. That is not acceptable as the long-term design.
Going forward, 3D work must start from the 2D contract and add only the spatial
extension points. See `SEMANTIC_PARITY_3D.md`.

## Layer Responsibilities

### Shared Upper Layer

The shared layer owns every PuzzleScript semantic that is not forced to differ
by dimension:

- compile source text
- detect whether the compiled game is 2D or 3D
- normalize metadata
- expand tags, mappings, properties, aggregates, synonyms, and rule groups
- lower rules to the same contract shape
- apply turn phase order: input, normal rules, movement, late rules, commands
- handle shared commands such as restart, undo, again, message, goto, checkpoint,
  win, and level advance through one session contract
- collect shared artifacts such as command queues, win results, sound events,
  restart/checkpoint requests, and messages
- route dimension-specific board and renderer operations through explicit
  dimension hooks

The shared layer may not silently drop a 2D feature in 3D. Unsupported means
"not implemented yet against the shared contract", not "3D has different
semantics".

Metadata is not raw shared state. The upper layer owns normalization and routing,
but each metadata setting must be assigned to the same kind of contract slot as
the 2D implementation. See `METADATA_CONTRACT_3D.md` and
`SLOT_DESIGN_3D.md`.

### 2D Core Runtime

The existing engine remains the behavioral reference.

It owns:

- existing `LEVELS`
- existing `Level` behavior
- current 2D movement masks
- existing rule execution order and advanced PS Next features
- existing undo/restart/win behavior
- existing 2D renderer contract

3D work should read this path as the source contract. Extractions from this path
must preserve 2D behavior exactly.

### 3D Core Runtime

The 3D core must mirror the 2D core form.

It owns:

- `three_dimensions` mode level data lowered from ordinary `LEVELS`
- 3D board state: `width`, `height`, `depth`, flat objects array
- internal frame: `x` left/right, `y` up/down rows, `z` front/back slices
- oriented rule frames: 24 rotations, not mirrored 48-frame expansion
- coordinate/index helpers
- six directions: `left`, `right`, `front`, `back`, `up`, `down`
- the same cell/movement mask contract as 2D
- the same rule, movement, late, command, win, undo/restart, checkpoint, random,
  loop, gosub, and sound semantics as 2D unless a spatial proof requires a
  dimension hook

The 3D core may implement spatial operations separately, but not semantic
features separately. For example, `late` is not a 3D feature; it is the same
turn phase split. `rigid` may need 3D movement/rollback mechanics, but its
contract must match 2D.

### Renderers

Rendering is below the shared upper layer and beside the matching core runtime:

- 2D runtime feeds the existing 2D renderer.
- 3D runtime eventually feeds a 3D renderer.

The first 3D path does not need a polished renderer. A debug renderer or
explicit unsupported-renderer state is acceptable after the shared semantic path
is real.

## Data Flow

The intended 3D path is:

```txt
parser/compiler
  -> 3D mode detection from three_dimensions
  -> state.levels3 or equivalent internal 3D level transport
  -> same lowered PuzzleScript semantic artifacts as 2D
  -> dimension-aware board/index/neighbors
  -> same turn/session contract
  -> 3D renderer
```

The intended 2D path remains:

```txt
parser/compiler
  -> state.levels
  -> same lowered PuzzleScript semantic artifacts
  -> 2D board/index/neighbors
  -> same turn/session contract
  -> existing renderer
```

Internal 3D level transport such as `state.levels3` must not be shoved into
`state.levels` to reuse the 2D loading path. That would let 2D caches,
movement, undo, and rendering reinterpret 3D state as a malformed 2D board.

There is no separate author-facing 3D level section. The stable boundary is
`three_dimensions` mode plus ordinary `LEVELS` syntax.

## Implication For Current Work

The existing 3D modules are prototype code. They are useful for tests and for
understanding spatial requirements, but they are not the final architecture if
they omit ordinary PuzzleScript semantics.

Before adding more 3D-only feature code, re-audit each difference from 2D:

- If the difference is not caused by `z`, `front/back`, 3D neighbor/indexing, or
  oriented spatial matching, remove the difference.
- If the 3D code marks a 2D semantic such as `late` unsupported, treat that as a
  missing shared contract, not as a valid 3D limitation.
- If the 3D code has a custom rule/session path, compare it line by line against
  the 2D phase order and artifacts.

## Next Implementation Steps

1. Build a 2D-vs-3D semantic parity checklist from the existing 2D engine.
2. Mark each current 3D unsupported feature as either spatial or non-spatial.
3. Remove non-spatial unsupported status starting with phase-only features such
   as `late`.
4. Define the shared lowered artifacts consumed by both 2D and 3D:
   rules, late rules, command queues, win results, sound events, checkpoints,
   random choices, loop/gosub control, and metadata/session requests.
5. Only then continue spatial work: 3D board indexing, neighbors, movement
   resolution, rigid rollback over 3D movement, oriented rule frames, and
   renderer.

## Non-Goals

- accepting feature omissions as 3D design differences
- separate 3D semantics for non-spatial PuzzleScript features
- polished 3D renderer before shared semantics exist
- implicit fallback from 3D level transport to the 2D `LEVELS` runtime path
