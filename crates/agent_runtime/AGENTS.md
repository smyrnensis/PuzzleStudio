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
- AI-authored intermediate states cross the boundary through the versioned
  semantic ASCII artifact. Preserve its base-state provenance and lower it with
  the language-owned level ASCII parser; do not make callers author core state
  slots or object IDs.
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
