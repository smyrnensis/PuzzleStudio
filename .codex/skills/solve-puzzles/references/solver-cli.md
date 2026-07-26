# Solver CLI Reference

This reference documents the current agent-facing solver surface and the procedure for detecting drift. Verify the live contract before using the examples.

## Live Contract Calibration

Run from the PuzzleBuilder workspace root.

1. Check the available top-level and agent commands:

   ```sh
   cargo run -q -p puzzlestudio -- --help
   cargo run -q -p puzzlestudio -- agent --help
   ```

2. Start one long-lived JSON-lines process:

   ```sh
   cargo run -q -p puzzlestudio -- agent --stdio
   ```

   Send exactly one JSON request per line and parse exactly one JSON response per line. Keep this process alive: sessions, states, goals, runs, and searches are in-memory handles owned by that process. A new process cannot use handles returned by an earlier process.

   JSONL is the host/process boundary, not the solver's internal data model.
   The compiled game, prepared rule programs, play session, search frontier,
   visited set, and candidate states stay Rust-owned. Requests carry handles and
   typed observations; they do not round-trip compiled artifacts through JSON or
   JavaScript. Browser `WasmSolverService` handles belong to their Web Worker and
   are not interchangeable with agent protocol handles.

   Built-in solving in browser WASM and the native play server is also owned by
   the shared Rust solver runtime. The native server registers its already-loaded
   game and serializes only the final typed result at the HTTP response boundary;
   it does not reparse source or run a host-specific search implementation.

3. Send `compile` as the protocol handshake. Confirm top-level `version` and `ok`, then confirm `data.modelKind`, `data.sessionId`, and `data.initialStates` before constructing later requests. Treat `contract_version_mismatch`, unknown operations, missing required fields, or changed response shapes as contract drift.

4. When the checked-in reference and live behavior disagree, inspect the current contract rather than guessing. Locate it by symbol so file moves do not matter:

   ```sh
   rg -n 'AGENT_PROTOCOL_VERSION|enum AgentCommand|SemanticStateArtifact|SemanticGoalArtifact' crates
   rg -n '"op": "(compile|run|create_search|advance_search)"' crates
   ```

   Treat serde request types as the request source of truth. Response bodies are not all represented by typed response structs; use the response-building implementation, passing protocol tests, and an observed live response together when resolving their shape. Report reference drift during an ordinary solve task. Update this reference only when maintaining the skill; do not add a compatibility fallback.

5. Do not infer PuzzleStudio language syntax from this document. Compile the actual entry and consume the manifest and semantic exports. Use `puzzlestudio check` for source diagnostics.

## Current Entry Points

```text
puzzlestudio check <path> [--json]
puzzlestudio inspect <path>
puzzlestudio agent --stdio
puzzlestudio agent request < requests.jsonl
```

Use `agent --stdio` for an interactive investigation whose later requests depend on returned IDs. `agent request` is suitable for a preconstructed JSONL stream that does not require the caller to interpolate earlier responses.

The current agent protocol version is `2`. Every request has:

```json
{"version":2,"requestId":"optional-correlation-id","op":"operation_name"}
```

Every response has `version`, the optional echoed `requestId`, `ok`, and either `data` or `error`. Branch on `ok`; do not continue by extracting IDs from a failed response.

## Capability Boundary

Protocol v2 currently accepts `puzzle2d` models only. It fails visibly with `unsupported_model_kind` for a selected 3D model and with `ambiguous_model` when a document has multiple models and `compile.model` is omitted.

The unified engine has solver functionality outside this agent protocol, but this CLI reference does not imply that those capabilities are agent-accessible. In particular, protocol v2 exposes semantic-goal search, not a generic built-in-win search operation. Report the missing operation when it is required; do not switch to an undocumented legacy path.

## Compile And Inspect

Compile a game entry:

```json
{"version":2,"requestId":"compile","op":"compile","path":"games/microban/game.puzzle"}
```

Add `"model":"authored-model-name"` when the document contains multiple models. A successful response returns `sessionId`, `model`, `modelKind`, `sourceHash`, and `initialStates`. Each initial-state entry provides a level identity and state handle.

Request the compiled manifest:

```json
{"version":2,"op":"manifest","sessionId":"session-1"}
```

The manifest is the typed discovery surface for inputs, objects, variables, queries, goal, lose condition, solver strategy, levels, and rule debug information. Prefer it over reading source syntax heuristically.

Inspect a state:

```json
{"version":2,"op":"inspect_state","sessionId":"session-1","stateId":"state-1"}
```

## Execute And Observe Inputs

Run an input sequence from any state handle in the session:

```json
{"version":2,"op":"run","sessionId":"session-1","fromStateId":"state-1","inputs":["right","up"],"observation":{"mode":"events"}}
```

Observation modes are currently `summary`, `events`, `indices`, and `all`. For `indices`, supply `observation.indices`. The response returns a normal run handle and terminal state handle. Unknown inputs fail the whole request rather than creating a partial run.

One listed input is one complete play-owned semantic action. The authoritative lifecycle performs the input's internal rule work, repeated `again` processing, and completion handling before the next listed input. Do not add synthetic inputs for those internal transitions.

Completion commands may advance the play session, so a solved run's `terminalStateId` can describe the post-completion state rather than the last visible board of the completed level. Use the authoritative `result`, transition summary, and requested level identity instead of inferring victory from terminal board equality alone.

Inspect selected run positions:

```json
{"version":2,"op":"inspect_run","sessionId":"session-1","runId":"run-1","at":[0,2],"includeTrace":true}
```

Compare concrete states:

```json
{"version":2,"op":"compare_states","sessionId":"session-1","leftStateId":"state-1","rightStateId":"state-2"}
```

## Counterfactual Semantic States

Use `start_level_from_state` when the counterfactual is a custom initial
arrangement and its `on_level_start` behavior belongs in the experiment:

```json
{"version":2,"op":"start_level_from_state","sessionId":"session-1","levelIndex":0,"expectedLevelName":"start","setObjectPositions":[{"object":"Player","positions":[[1,0]]}],"setVariables":[],"assert":[{"kind":"contains","position":[1,0],"objects":["Player"]}]}
```

This patches the compiled authored state, then enters the authoritative model
level-start path and runs the level's `on_level_start` program once. Scene and
presentation lifecycle does not materialize. Assertions inspect the post-start
state. The returned `preStartDiff` separates the declared patch from the
lifecycle result; `diff` compares the final result with an ordinary start of the
same level. The handle has `origin: "level_start"` and can be passed to `run` or
search.

Use `derive_state` for an intermediate-state counterfactual. It replaces the
full position set of each named object and inherits every unmentioned object
and variable from the immutable base state:

```json
{"version":2,"op":"derive_state","sessionId":"session-1","baseStateId":"state-1","setObjectPositions":[{"object":"Player","positions":[[1,0]]}],"setVariables":[{"variable":"count","value":3}],"assert":[{"kind":"contains","position":[1,0],"objects":["Player"]}]}
```

Use object names obtained from the manifest. Each object may occur at most once
in `setObjectPositions`; its `positions` array may be empty to remove every
occurrence. Coordinates must be unique and in bounds. Optional
`expectedBaseHash` is a staleness assertion, not required provenance metadata.
`setVariables` is a sparse list of `{"variable": name, "value": integer}`
entries. Variable names may not repeat, and persistent variables may not change.
At least one object-position or variable replacement is required. Unknown
request fields are rejected, so treat `invalid_request` as contract drift or a
misspelled field rather than retrying with a reduced request.

Post-patch `assert` entries support `exact`, `contains`, and `excludes` at one
coordinate. Use them for premises the experiment depends on, such as a button
remaining under a moved crate. The operation is atomic and returns the applied
patch, complete semantic diff, full symbolic state observation, and a
hypothetical state handle with `origin: "derived"`. It validates play
materialization before issuing the handle, so a successful response can be used
immediately by `run` or search. The base session's checkpoint, restart anchor,
lifecycle-started status, persistent values, and scene context remain intact.

Derivation does not execute an input or re-enter the level. For an intermediate
state, declare every object changed by the intended snapshot. For a custom
initial arrangement, use `start_level_from_state` so only the authored
`on_level_start` lifecycle produces dependent objects. Do not simulate a route
merely to avoid declaring the intended state.

Use the complete ASCII artifact only for lossless round trips, exact fixtures,
or cases that genuinely replace most of the board. Export a live state first:

```json
{"version":2,"op":"export_semantic_state","sessionId":"session-1","stateId":"state-1"}
```

Use the returned artifact as the template. Preserve its version, base-state provenance, dimensions, level identity, empty marker, and meanings for unchanged legend characters. Change only the intended `lines`, concrete `exact` legend meanings, or permitted variable values. Keep the variable-name set exactly equal to the compiled manifest. Protocol v2 requires persistent variables to retain their base values; only non-persistent variables may be changed by a counterfactual artifact.

Import the edited artifact:

```json
{"version":2,"op":"import_semantic_state","sessionId":"session-1","artifact":{"...":"the edited exported artifact"}}
```

Imported semantic states have hypothetical provenance. Semantic states require concrete `exact` cells; `unknown`, `contains`, and `excludes` are goal predicates and are rejected in state artifacts. A base hash mismatch is also rejected.

Running inputs or starting semantic search from a hypothetical state is useful for experiments, but it does not make the hypothetical root reachable from the authored initial state. The runtime reconstructs its authoritative session from the base provenance and verifies that lifecycle initialization does not change the imported semantic state before the first input.

## Semantic Goals

Construct a goal from a freshly exported semantic-state artifact:

1. Change `kind` from `puzzle2d-semantic-state` to `puzzle2d-semantic-goal`.
2. Construct exactly the current goal artifact shape and remove every state-only field absent from it; protocol v2 removes `variables`.
3. Add goal legend cells with the current predicate forms.
4. Keep provenance, level, dimensions, and unchanged legend meanings intact.

Protocol v2 does not reject every unknown JSON field. Successful deserialization is therefore not evidence that a leftover field belongs to the goal contract.

Protocol v2 goal meanings are:

```json
{"kind":"exact","objects":["Player"]}
{"kind":"contains","objects":["Player"]}
{"kind":"excludes","objects":["Player"]}
{"kind":"unknown"}
```

Import and evaluate a goal:

```json
{"version":2,"op":"import_semantic_goal","sessionId":"session-1","artifact":{"...":"semantic goal artifact"}}
{"version":2,"op":"evaluate_semantic_goal","sessionId":"session-1","goalId":"goal-1","stateId":"state-1"}
```

Use `unknown` for non-binding cells. Use `contains` or `excludes` when other objects in a cell are irrelevant. Prefer the weakest predicate that captures the hypothesis being tested.

## One-Shot Semantic Search

```json
{"version":2,"op":"solve_semantic_goal","sessionId":"session-1","goalId":"goal-1","fromStateId":"state-1","algorithm":"best_first","budget":{"maxDepth":128,"maxStoredNodes":1000,"maxMillis":5000}}
```

Algorithms are currently `bfs` and `best_first`. Protocol v2 `best_first` scores the number of mismatched constrained goal cells. It does not consume an arbitrary heuristic expression from this request.

`maxStoredNodes` limits unique nodes retained in the visited set and node graph,
including the start node. It does not limit expanded nodes or transition attempts.
Expanding one node may try every available semantic input; duplicate and no-op
successors still execute a transition but do not consume another stored-node slot.

Before search, the runtime reconstructs the source state from its authoritative
provenance and initializes the selected level through the play lifecycle. That
validated state is then projected into the sliced logical search model. Each
search edge applies one complete semantic input directly to that deterministic
model, including internal `again` processing and logical completion detection;
it does not run a play session per edge. `solutionDepth` counts semantic inputs,
not internal rule passes. A solved candidate is accepted only after its witness
has been replayed through the authoritative play lifecycle and the resulting
complete state has been verified.

Initialization failures are visible. Current errors include `semantic_search_session_failed` when an authoritative search session cannot be created and replay/provenance errors such as `replay_mismatch`, `hypothetical_state_changed`, or `transition_failed` when the source state cannot be reconstructed faithfully.

All budget fields must be positive. Choose values for the experiment rather than copying the example mechanically.

Outcomes:

- `solved`: returns a replay-verified run, terminal state, inputs, and solution depth;
- `exhausted`: the specified search exhausted its reachable bounded space;
- `budget_exceeded`: the specified budget ended first and the result is inconclusive;
- a failed transition or contract error returns `ok: false` with an error code.

## Resumable Search And Candidates

Create a persistent search frontier:

```json
{"version":2,"op":"create_search","sessionId":"session-1","goalId":"goal-1","fromStateId":"state-1","algorithm":"best_first","limits":{"maxDepth":128,"maxStoredNodes":1000}}
```

Advance it with an experimental allowance:

```json
{"version":2,"op":"advance_search","sessionId":"session-1","searchId":"search-1","allowance":{"maxExpandedNodes":100,"maxMillis":1000}}
```

The start state, goal snapshot, algorithm, input set, heuristic, maximum depth, and stored-node limit are fixed for the search lifetime. An advance allowance adds work to the existing frontier; it is not a cumulative total and does not restart the search. Node-count allowances are deterministic. Duration allowances preserve work but may pause at different positions on different machines.

`maxStoredNodes` is the lifetime capacity for unique retained nodes.
`maxExpandedNodes` is the additional expansion work permitted by one
`advance_search` request. Neither field counts transition attempts.

Inspect the best retained search candidates without promoting them to normal states:

```json
{"version":2,"op":"inspect_search","sessionId":"session-1","searchId":"search-1","candidateLimit":10}
```

Candidates currently expose their ID, score, depth, discovery order, input sequence, state hash, and semantic goal difference. The returned pool is ranked from all retained search nodes, not restricted to the live frontier. Use these observations to assess a hypothesis and choose informative candidates.

Materialize a candidate when its concrete state is needed:

```json
{"version":2,"op":"materialize_search_candidate","sessionId":"session-1","searchId":"search-1","candidateId":"candidate-1"}
```

Materialization replays the candidate through the authoritative lifecycle and returns a normal run and terminal state handle while retaining the search root's provenance. A materialized candidate is a reachable checkpoint only when its search was rooted in a reachable state; one rooted in a hypothetical state remains hypothetical.

Current search statuses include `ready`, `paused`, `solved`, `exhausted`, `resource_limit`, and `failed`. `paused` and `resource_limit` are not claims of unsolvability. A terminal search cannot be advanced again.

Close a search after use:

```json
{"version":2,"op":"close_search","sessionId":"session-1","searchId":"search-1"}
```

## Close The Session

```json
{"version":2,"op":"close","sessionId":"session-1"}
```

Closing releases every state, run, goal, and search handle owned by the session.

## Drift-Resistant Operating Rules

- Discover command availability through the live executable before use.
- Handshake with `compile` and honor the returned protocol version and model kind.
- Obtain names and IDs from successful live responses; do not derive them from source text.
- Use `start_level_from_state` for hypotheses about an authored pre-start level arrangement, and use `derive_state` for changes to an intermediate runtime snapshot. Create complete semantic artifacts by minimally editing a fresh export only when the complete-state contract is required.
- Treat examples in this reference as protocol-v2 examples, not timeless syntax.
- On a schema or capability mismatch, inspect current request types, response builders, passing protocol tests, and live responses. Fail visibly until the required operation is understood; update this reference only as an explicit skill-maintenance change.
- Never add a silent compatibility route, older command, filename heuristic, or source parser as a fallback.
