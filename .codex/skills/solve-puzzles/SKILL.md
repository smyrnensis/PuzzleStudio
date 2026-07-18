---
name: solve-puzzles
description: Solve difficult PuzzleStudio levels by combining causal reasoning, controlled experiments, counterfactual intermediate states, semantic goals, heuristics, and bounded solver searches. Use when Codex is asked to solve or investigate the solvability of a .puzzle or .puzzle3 game, especially when unguided enumeration is impractical.
---

# Solve Puzzles

Read [references/solver-cli.md](references/solver-cli.md) completely before operating the solver. Calibrate against the live CLI contract described there instead of relying on remembered commands, JSON fields, PuzzleStudio source syntax, or old examples.

## Objective

Produce an input sequence that wins from the authoritative initial state. Treat the solver as an experimental instrument inside a model-based investigation of the puzzle. Use search to test and execute an explanation of the puzzle, not to replace one.

Keep these state classes distinct:

- **reachable**: produced by authoritative runtime inputs from a reachable state;
- **counterfactual**: imported to isolate a mechanism or test an endgame hypothesis;
- **candidate**: retained inside a search until explicitly materialized;
- **goal**: a predicate over states, not a state itself.

Only reachable states and replay-verified input sequences may contribute to the final solution.

Treat a solver action as one complete play-owned semantic input. Let the authoritative lifecycle process internal rule activity, including repeated `again` work and completion commands. Do not model those internal transitions as extra player inputs. Do not assume that visually identical boards are interchangeable when hidden lifecycle or session state can affect later behavior.

## Investigate Before Scaling Search

Compile the target, obtain its manifest, and inspect the selected level's initial state. Derive a working causal model from the compiled rules, objects, variables, goal, lose condition, solver strategy, and controlled transitions.

Identify:

- actions and state changes that enable later progress;
- irreversible commitments and recoverable moves;
- spatial, ordering, resource, orientation, or access constraints;
- states that look advanced but destroy necessary freedom;
- plausible phases and endgame preconditions;
- observations that would distinguish competing explanations.

Run a modest direct search as a baseline when the live CLI supports the required goal. A failed baseline is evidence about the current search formulation, not a conclusion about solvability.

## Hypothesis Loop

Maintain a small portfolio of competing hypotheses. For each hypothesis, record:

- the claimed mechanism;
- a predicted observable consequence;
- the smallest experiment that can distinguish it;
- a counterfactual state or semantic goal when useful;
- invariants that a valid solution must preserve;
- evidence that would falsify it;
- the heuristic consequence if supported.

Prefer controlled experiments over broad exploration. Change one meaningful factor at a time when practical. Compare resulting states, traces, mobility, goal differences, and recovery options.

Use counterfactual states to test endgames, preconditions, deadends, ordering constraints, and heuristic discrimination. Derive counterfactual artifacts from a live exported semantic state and change only the intended semantic content. Never invent a state schema or PuzzleStudio source syntax from memory.

Use successful experiments to define minimally constrained semantic goals. Avoid exact full-board goals when only a few properties matter. Ground useful counterfactuals by searching for their necessary conditions from reachable states.

## Design And Test Heuristics

Derive heuristic features from the causal model. Favor features that predict future solvability, such as preserved mobility, access, useful alignment, phase completion, and satisfied preconditions. Penalize irreversible loss, violated invariants, and supported deadend conditions.

Test a heuristic against contrasting states before trusting a large search. It should rank known productive states ahead of superficially advanced dead states and explain why. Revise the feature model before merely tuning weights when the ordering is wrong.

Use the heuristic surfaces actually exposed by the live CLI. When the current CLI cannot accept a desired heuristic directly, use the heuristic at the agent level to choose experiments, semantic goals, and candidates. Do not claim that the engine used a heuristic it did not receive.

## Allocate Search Deliberately

Begin experiments with a modest state budget, commonly around 1000 states. Choose each budget from the hypothesis, expected solution depth, branching factor, and predicted observation. This is a calibration default, not a hard limit.

Increase a budget when observations support the current model, the frontier progresses in the predicted direction, and additional search has a specific evidential or solution value. Redirect the investigation when expansion stops distinguishing hypotheses, discovering structure, improving relevant features, or approaching a predicted condition.

Large searches are acceptable when guided by a validated model and producing interpretable progress. Do not use repeated budget increases as the only response to failure.

## Ground And Verify The Solution

Materialize promising search candidates before treating them as states. Re-run important semantic-goal checks on materialized states. Preserve the authoritative input sequence for every reachable transition used in the solution.

Concatenate the selected input sequences and replay them from the original initial state. Accept completion only when the authoritative runtime reports the actual win condition. Report the game, exact model and level, inputs, final result, search evidence, and any remaining uncertainty.

Classify failure precisely:

- `exhausted` applies only to the exact bounded search that exhausted its space;
- `budget_exceeded`, `paused`, and `resource_limit` are inconclusive;
- protocol, compile, transition, and unsupported-capability errors are visible failures;
- an ungrounded counterfactual is a hypothesis, never a solution segment.
