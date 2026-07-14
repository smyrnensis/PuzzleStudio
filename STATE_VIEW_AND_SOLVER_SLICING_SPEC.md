# State View And Solver Slicing Spec

This document defines the target contract for making solver state reduction an
explicit analysis. It is a design target, not a description of the current
implementation.

## Goal

The system exposes different state views for different consumers:

- play and editor can work with all authored objects.
- renderer consumes a render view produced by the runtime contract.
- solver consumes a solver key derived by relevance analysis.

The key distinction is whether an object can affect future gameplay observations
that a given consumer is responsible for preserving.

## State Views

`logic_state`

- The ordinary runtime state after applying authored rules.
- It may contain objects that are useful only for drawing, inspection, or editor
  workflows.
- It is the editable state surface for the editor unless a narrower editing mode
  explicitly says otherwise.

`render_state`

- The state or scene view used for drawing.
- It is produced by Rust/runtime-owned APIs.
- Browser/editor JavaScript must not reconstruct solver semantics from source
  syntax, naming conventions, or renderer internals.

`solver_key_state`

- The canonical state used for solver duplicate detection, transition cache
  keys, and search frontier membership.
- It is derived from `logic_state` by solver relevance analysis.
- It may omit objects whose presence cannot affect future solver-visible
  gameplay results.

These views may share storage internally, but their public contracts must stay
separate.

## Authoring Contract

Object spelling must not be the semantic source of solver relevance. A naming
convention alone must not make an object solver-pruned, collision free, or
editor-restricted.

Authored state evolution uses ordinary deterministic rules. Solver pruning
decides whether the result matters for solving.

## Solver Relevance

Solver relevance is backward liveness over compiled rule behavior. A rule read
becomes relevant only when that read can affect a relevant output.

Roots include:

- win and lose conditions, and `solver { deadend <query> }` predicates;
- query values used by gameplay conditions, goals, or solver-visible reports;
- input availability and input-dependent transition differences;
- movement, collision, and layer occupancy that can change relevant objects;
- writes to already relevant objects;
- deterministic gameplay RNG state and counters.

Propagation rules:

- If a rule can write a relevant object or relevant mark, the rule's LHS,
  conditions, selector bindings, and required input dependencies become
  relevant.
- If a rule writes only irrelevant objects, its LHS and conditions do not become
  relevant merely because the rule exists.
- Self-maintaining projection rules such as `[ no Floor ] -> [ Floor ]` do not
  keep `Floor` relevant unless another relevant root reads `Floor` or a rule
  uses it to affect a relevant result.
- Relevance is computed to a fixed point over compiled rules and compiled
  selectors, not over authoring syntax.

Ambiguity must be visible. If the analysis cannot prove that pruning preserves
solver-visible behavior, the object or dependency remains relevant or the
compiler/solver reports a specific unsupported case.

## Editor Contract

The editor should not restrict editing to solver-relevant objects. It may place,
remove, inspect, and save any authored object in `logic_state`.

The editor may surface analysis results such as:

- solver-relevant;
- solver-pruned;
- random-dependent;
- ambiguous or unsupported for solver pruning.

Those labels are explanatory, not object ownership declarations. They must come
from Rust/runtime analysis data, not from editor-side source scanning.

Preview and solver panes may render different views, but the difference must be
named in the API response. A solver preview can display `solver_key_state` or a
solver observation view; an editing preview should display the runtime/render
view.

## Random

Random behavior in this project must remain deterministic.

Gameplay random belongs to the solver-visible deterministic state model. If a
rule consumes gameplay random, that consumption is relevant unless the compiler
can prove that removing the rule cannot change any later gameplay random value.

Cosmetic or render random must use a separate deterministic domain. It should be
derived from stable inputs such as state identity, object identity, position,
turn/frame, and a named render salt. It must not advance gameplay RNG state.

A rule or effect that mixes prunable writes with gameplay RNG
consumption is not silently pruned. It is either kept relevant or rejected with a
specific diagnostic.

## Non-Goals

This contract does not require full automatic semantic inference from visuals,
sprite names, object adjacency, or current sample usage.

It does not promise that every object irrelevant to a particular goal can be
removed from play/editor state. Solver slicing is a solver key optimization and
analysis contract first.

It does not make editor JavaScript a second compiler. The source of truth for
state view classification remains Rust-owned compiled analysis.

## Migration Shape

1. Introduce solver relevance reports without changing editor editing behavior.
2. Use `solver_key_state` for solver cache/frontier keys.
3. Remove naming-based classification only after tests prove relevance reports
   cover the existing cases.
4. Update user-facing docs only after implementation behavior matches the new
   contract.
