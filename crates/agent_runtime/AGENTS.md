# Agent Notes

This crate owns the headless, agent-facing 2D experiment session and its typed
JSON protocol.

## Boundaries

- Compile through `puzzle-lang` and execute through `puzzle-play`; do not copy
  parser, lifecycle, or transition semantics.
- Keep sessions symbolic and headless. Do not depend on HTML, WASM, DOM,
  screenshots, editor state, or renderer internals.
- A run consumes an input sequence and produces immutable state/run handles.
  Single-input runs are permitted but are not a privileged interaction model.
- AI-authored intermediate states normally cross the boundary as object-named
  `derive_state` patches over an immutable base handle. A patch replaces the
  complete position set of each named object while preserving unmentioned
  objects and variables. Do not make callers restate board dimensions,
  structural cell stacks, core state slots, or object IDs.
- AI-authored initial arrangements patch the compiled level's authored state
  through `start_level_from_state`, then enter the authoritative play lifecycle
  with level-start materialization enabled exactly once. Assertions observe the
  post-start state. Do not model this lifecycle event as a player input or an
  adapter-owned rule pass.
- Keep the versioned semantic ASCII artifact as the complete-state
  round-trip/export contract. Preserve its base-state provenance and lower it
  with the language-owned level ASCII parser; do not treat its glyph legend as
  the primary state-editing surface.
- Validate derived and imported roots through the authoritative play
  materialization lifecycle before issuing a state handle. Do not defer an
  invalid hypothetical root until its first run or search.
- Intermediate-state patches do not infer rule consequences or re-enter the
  level. Initial-state patches may acquire only the consequences produced by
  the level's owned `on_level_start` lifecycle program.
- Semantic goal `unknown` cells are explicit non-binding don't-cares. Do not
  infer bindings, captures, or equality from repeated legend characters.
- Semantic state legends are concrete `exact` meanings. Semantic goals may use
  typed `exact`, `contains`, `excludes`, and `unknown` cell predicates; do not
  weaken complete-state import into predicate matching.
- Semantic goal search must use the partial goal itself as the stopping
  condition. A solved witness is not accepted until replayed through the
  authoritative play lifecycle into normal run/state handles.
- A resumable search owns the actual frontier, visited keys, node graph, and
  parent actions. Never implement continuation by restarting a one-shot search.
- Search candidates remain search-local until authoritative replay reproduces
  the complete candidate state; only then create normal run/state handles.
- Reject 3D and ambiguous model selection visibly until their contracts are
  implemented.
- Protocol changes require typed serde contracts and versioned tests.

## Commands

```bash
cargo test -p puzzle-agent-runtime
cargo test -p puzzlestudio agent_
```
