# State View And Solver Slicing Spec

This document defines the state boundary shared by play, editor, and solver.

## State Views

`real_state`

- The complete model state observed and edited by play/editor.
- It may contain authored objects that have no effect on the selected solver
  goal or on any transition that can affect that goal.
- Runtime state handles and authoritative replays use this view.

`solver_logical_state`

- The relevance-projected model state stored in solver nodes.
- It is the only state admitted to the frontier, visited table, duplicate key,
  heuristic, dead-end predicate, and candidate record.
- It contains no checkpoint, navigation wait, editor presentation state,
  lifecycle bookkeeping, provenance, witness history, or other state outside
  the compiled logical transition needed by the search.

`materialized_observation`

- A `real_state` reconstructed for an explicitly selected logical candidate.
- Reconstruction starts from the candidate search's authoritative real root and
  replays its witness inputs through `puzzle-play`.
- The runtime projects the reconstructed result and verifies equality with the
  candidate's `solver_logical_state` before exposing a reusable state handle.
- It is never attached to every search node or used as a solver key.

`render_state`

- A runtime-owned view of a real or materialized state for drawing.
- Browser/editor JavaScript consumes this view and does not infer solver or
  language semantics from source text, names, or sprites.

## Search-State Contract

A stored search node consists of:

- one `solver_logical_state`;
- goal-completion metadata required by the search algorithm;
- parent/action linkage owned by the search machine.

The witness input sequence is reconstructed from parent/action linkage. Run
provenance, editor history, a real board snapshot, and replay/session state are
owned outside the node.

Solver inputs are compiled logical inputs. Session operations such as undo,
restart, level selection, and navigation are not search actions. A compiled
effect that requires unsupported session semantics fails at the solver
transition boundary; it does not switch the search to a full session state.

## Solver Relevance

Relevance is backward liveness over compiled rules. A read becomes relevant
only when it can affect a relevant output.

Roots include:

- the active goal and lose conditions;
- `solver { deadend <query> }` and solver strategy predicates;
- query values used by the selected goal or heuristic;
- movement, collision, layers, and writes that can change a rooted object;
- deterministic gameplay random state when its consumption can change a
  relevant transition.

Propagation rules:

- If a rule can write a relevant object or mark, its LHS, conditions, selector
  bindings, and input dependencies become relevant.
- A rule that writes only irrelevant objects does not keep its reads relevant
  merely because it exists.
- Relevance reaches a fixed point over compiled rules and selectors, not source
  spelling.
- Object names and rendering roles never determine relevance.

The projection must be closed under the logical transition used by the solver.
If analysis cannot establish that closure, compilation or search reports the
unsupported dependency. It must not retain the complete real state as a silent
fallback.

## Materialization Contract

Logical-to-real reconstruction is an explicit runtime operation:

```txt
authoritative real root + witness inputs
  -> puzzle-play replay
  -> reconstructed real state
  -> solver projection
  -> equality check against logical candidate
```

`puzzle-solver-runtime` owns this orchestration. `puzzle-play` owns game
initialization and replay semantics. The editor decides which candidate to
observe, but does not implement reconstruction in JavaScript.

Materialization cost is proportional to the selected witness. Search expansion
does not pay that cost and does not retain the result. A UI that wants previews
requests them for selected candidates or progress checkpoints, not for every
frontier node.

## Editor Contract

The editor can place, remove, inspect, and save every authored object in
`real_state`, including solver-pruned objects. It may display runtime-provided
labels such as solver-relevant, solver-pruned, random-dependent, or unsupported.

A solver preview must identify whether it displays a logical projection or a
materialized real observation. An editing preview displays the real/render
view. Both views come from typed Rust-owned contracts.

## Random

Gameplay random remains deterministic and solver-visible whenever its
consumption can affect a relevant future transition. Cosmetic random uses a
separate deterministic render domain and never advances gameplay random state.

A transition that mixes prunable writes with relevant gameplay RNG consumption
is retained or rejected with a specific diagnostic.

## Non-Goals

- Solver slicing does not restrict what the editor can author.
- Visual appearance, sprite names, and object adjacency are not semantic
  relevance declarations.
- Materialization is not a second search representation.
- Provenance categories such as reachable and counterfactual do not alter the
  contents of an internal solver node.

## Verification

Tests must establish that:

1. irrelevant authored objects are absent from every inspected search
   candidate state;
2. duplicate keys are computed from the projected logical state;
3. a selected witness reconstructs a real state containing preserved
   solver-irrelevant objects;
4. projecting that real state exactly reproduces the logical candidate;
5. agent and editor adapters use the same runtime search and materialization
   implementation.
