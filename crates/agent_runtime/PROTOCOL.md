# Agent Runtime Protocol v2

`puzzlestudio agent --stdio` reads one JSON request per line and writes one JSON
response per line. The process owns compiled sessions, immutable state handles,
and run trajectories. It does not use the editor, browser, HTML export, WASM, or
renderer paths.

Every request includes `version: 2`. `requestId` is optional and is echoed when
present. Errors use `ok: false` and a structured `error.code`; unsupported or
ambiguous requests never select a fallback path.

## Compile Once

```json
{"version":2,"requestId":"c1","op":"compile","path":"games/microban/game.puzzle"}
```

Use `model` when a document has more than one model. Protocol v2 accepts only a
selected 2D model. A successful response contains a process-local `sessionId`
and one immutable initial state handle per level.

## Inspect The Symbolic Contract

```json
{"version":2,"op":"manifest","sessionId":"session-1"}
```

The manifest names inputs, objects, variables, queries, levels, goals, and
source-mapped rules. Object state is represented symbolically by names and grid
coordinates; the protocol does not return raster images.

## Run An Input Sequence

```json
{"version":2,"op":"run","sessionId":"session-1","fromStateId":"state-1","inputs":["up","left","down"],"observation":{"mode":"events"}}
```

`run` validates the complete input list before execution, replays the source
state through the authoritative play lifecycle, and creates a new terminal
state without mutating the source state. When an input completes a level,
semantic goal matching and the corresponding run point use the committed
post-rules/pre-clear completion observation; `terminalStateId` still identifies
the post-lifecycle continuation state. Observation modes are:

- `summary`: terminal summary only (default);
- `events`: summary plus deterministic transition events when explicitly requested;
- `indices`: return only `observation.indices`;
- `all`: return every point for explicit debugging.

The complete trajectory remains inside the session even when it is omitted from
the response.

## Inspect Selected Points

```json
{"version":2,"op":"inspect_run","sessionId":"session-1","runId":"run-1","at":[0,12,30],"includeTrace":true}
```

State and run handles can also be queried with `inspect_state`, compared with
`compare_states`, and released with `close`. Handles are valid only in the
session and process that created them.

## Start A Level From A Declared Initial State

To test a custom initial arrangement, patch the authored pre-start state and
enter the authoritative model level-start lifecycle:

```json
{"version":2,"op":"start_level_from_state","sessionId":"session-1","levelIndex":0,"expectedLevelName":"start","setObjectPositions":[{"object":"Player","positions":[[1,0]]}],"setVariables":[{"variable":"count","value":3}],"assert":[{"kind":"contains","position":[1,0],"objects":["Player","Started"]}]}
```

The patch is applied to the compiled level's authored `initial_state`, before
runtime level-start processing. The play owner then enters that state with
level-start materialization enabled, so the level's `on_level_start` program
runs exactly once. Routines referenced by that program run through the normal
model execution path; scene and presentation lifecycle does not materialize.
The protocol provides no selector for additional routines or player-input
rules. Persistent variables cannot be changed by the patch.

`assert` predicates inspect the state after level start. The response includes
`preStartDiff` from the authored state to the patched pre-start state, `diff`
from the ordinary started level to the custom started state, the final symbolic
state, and `hypothetical`/`level_start` provenance. The resulting handle can be
used immediately by `run` or semantic search. A non-empty patch is required.

## Derive A Hypothetical Intermediate State

For normal AI-authored experiments, derive a state from an immutable base
handle by replacing only the position sets that matter:

```json
{"version":2,"op":"derive_state","sessionId":"session-1","baseStateId":"state-1","expectedBaseHash":"optional-staleness-check","setObjectPositions":[{"object":"Player","positions":[[1,0]]}],"setVariables":[{"variable":"count","value":3}],"assert":[{"kind":"contains","position":[1,0],"objects":["Player"]}]}
```

`setObjectPositions` is object-centric and atomic. Each entry replaces every
existing occurrence of that named object with the declared coordinate set.
Objects and variables omitted from the request are inherited from the base.
At least one object-position or variable replacement is required. The request
rejects unknown fields so a misspelled mutation or assertion cannot become a
successful no-op.
The runtime rejects duplicate object entries, duplicate or out-of-range
positions, duplicate variable entries, unknown names, collision-layer conflicts,
and changes to persistent variables. `setVariables` is a sparse list of typed
`{"variable": name, "value": integer}` replacements. `expectedBaseHash` is
optional because state handles are immutable;
when supplied, it is an explicit staleness assertion.

Post-patch `assert` entries use typed `exact`, `contains`, or `excludes`
predicates at one coordinate. A failed assertion rejects the whole patch and
does not create a partial state. The response returns the applied patch,
complete semantic diff, canonical symbolic state observation,
`hypothetical`/`derived` provenance, and a new immutable state handle.

The runtime validates authoritative play materialization before returning the
handle. Derivation does not execute a player input, re-enter the level, or run
`on_level_start`. It preserves the base session's checkpoint, restart anchor,
lifecycle-started status, persistent values, and scene context. Declare every
changed object in an intermediate-state patch. Use `start_level_from_state`
when the hypothesis is a custom initial arrangement whose level-start rules
should produce dependent state.

## Import A Complete Hypothetical State

For exact fixtures, external storage, and lossless round trips, export a solver
state into a meaning-preserving ASCII artifact:

```json
{"version":2,"op":"export_semantic_state","sessionId":"session-1","stateId":"state-1"}
```

The artifact contains an explicit object-name legend, ASCII rows, named
variables, and the exact base state identity. Importing it replaces the complete
state rather than applying a sparse patch:

```json
{"version":2,"op":"import_semantic_state","sessionId":"session-1","artifact":{"version":1,"kind":"puzzle2d-semantic-state","baseStateId":"state-1","baseStateHash":"...","levelIndex":0,"levelName":"start","width":3,"height":1,"empty":".","legend":{"G":{"kind":"exact","objects":["Goal"]},"P":{"kind":"exact","objects":["Player"]}},"lines":[".PG"],"variables":{}}}
```

Import validates dimensions, object names, collision layers, variables, level
identity, and the base hash through the language-owned ASCII state parser. It
returns a semantic object/variable diff and a new state whose provenance is
`hypothetical`. Hidden once-per-level state is preserved from the base; the
artifact does not pretend that the hypothetical board is reachable.

Persistent variables belong to the play session rather than an isolated puzzle
board. Semantic state artifact version 1 preserves their base values and rejects
imports that try to change them; it does not silently reset or reinterpret them.

## Declare A Non-Binding Unknown Goal Cell

A partial goal uses the same ASCII surface with typed legend meanings. In the
AI-facing notation, the declaration is conceptually:

```text
? = unknown
P = Player
```

The JSON contract preserves that distinction without treating `unknown` as an
object name:

```json
{"version":2,"op":"import_semantic_goal","sessionId":"session-1","artifact":{"version":1,"kind":"puzzle2d-semantic-goal","baseStateId":"state-1","baseStateHash":"...","levelIndex":0,"levelName":"start","width":3,"height":1,"empty":".","legend":{"?":{"kind":"unknown"},"P":{"kind":"contains","objects":["Player"]}},"lines":["?P?"]}}
```

Each `unknown` cell is an independent don't-care. It creates no variable,
capture, equality relation, or binding, including when the same character is
used more than once. `import_semantic_goal` returns `bindingCount: 0`; use
`evaluate_semantic_goal` with the returned `goalId` to evaluate a state.

Goal legend meanings are explicit predicates:

- `exact`: the cell's complete object set must equal `objects`;
- `contains`: every named object must be present, while additional objects are allowed;
- `excludes`: none of the named objects may be present;
- `unknown`: the cell is not checked.

`contains` and `excludes` require at least one object. Complete semantic states
accept `exact` only because they describe a concrete state, not a predicate.

`?` is reserved from generated state legends. Complete semantic state import
rejects `unknown`, even when the declared character does not appear in its
ASCII rows. Only semantic goals accept it.

## Search Directly For A Semantic Goal

Use an imported semantic goal as the search stopping condition from any
compatible state handle:

```json
{"version":2,"op":"solve_semantic_goal","sessionId":"session-1","goalId":"goal-1","fromStateId":"state-1","algorithm":"best_first","budget":{"maxDepth":80,"maxStoredNodes":1000,"maxMillis":5000}}
```

`algorithm` is explicitly `bfs` or `best_first`, and every budget field is
required and must be greater than zero. Both algorithms test the partial goal
directly; they do not substitute the game's built-in win condition. Best-first
orders states by the number of mismatching checked cell predicates, while
unknown cells contribute neither a mismatch nor a binding.

`maxStoredNodes` limits the unique search nodes retained in the visited set and
node graph, including the start node. It does not limit expanded nodes or
transition attempts. Each expanded node may try every available semantic input;
duplicate and no-op successors still perform a transition but are not stored.

Search initialization reconstructs the source state through its authoritative
provenance and the play lifecycle, then projects it into the sliced logical
model. Search edges apply semantic inputs directly to that deterministic model,
including internal `again` processing and logical completion detection. A play
session is not executed per edge. A solved witness is accepted only after
authoritative replay reproduces and verifies its complete state.

A solved search replays the witness through the authoritative play lifecycle
and returns normal immutable `runId` and `terminalStateId` handles together with
`searchOutcome: "solved"`. The search and replay both test a level-completing
input against its completion observation, even when `on_level_clear` navigates
to another level atomically. Exhausted and budget-limited searches return
search statistics without manufacturing a terminal state.

## Keep A Search Session Alive

A resumable search owns its actual frontier, visited keys, node graph, and
parent actions. Creating it performs validation but no search work:

```json
{"version":2,"op":"create_search","sessionId":"session-1","fromStateId":"state-1","goalId":"goal-1","algorithm":"best_first","limits":{"maxDepth":200,"maxStoredNodes":1000}}
```

The start state, goal snapshot, algorithm, input set, heuristic, maximum depth,
and stored-node limit are immutable for the search lifetime. Advance it by an
additional allowance; an allowance is not a cumulative budget:

```json
{"version":2,"op":"advance_search","sessionId":"session-1","searchId":"search-1","allowance":{"maxExpandedNodes":1000,"maxMillis":2000}}
```

Node or duration allowance exhaustion produces `paused` and preserves the same
frontier. Terminal statuses are `solved`, `exhausted`, `resource_limit`, and
`failed`; a terminal search cannot be advanced again. Node-count allowances are
deterministic. Duration allowances preserve work but do not promise the same
pause position across machines.

`maxStoredNodes` is the lifetime capacity for unique retained nodes.
`maxExpandedNodes` is the additional expansion work allowed by one
`advance_search` request. Neither field counts transition attempts.

Inspecting is read-only and requires an explicit candidate count:

```json
{"version":2,"op":"inspect_search","sessionId":"session-1","searchId":"search-1","candidateLimit":20}
```

Candidates are search-local solver nodes with stable `candidateId` values,
witness inputs, scores, hashes, and semantic goal diffs. They are not normal
state handles. Promote one only by authoritative replay:

```json
{"version":2,"op":"materialize_search_candidate","sessionId":"session-1","searchId":"search-1","candidateId":"candidate-42"}
```

Materialization creates ordinary `runId` and `terminalStateId` handles only if
the complete replayed state equals the solver candidate. `close_search` releases
the frontier, visited set, node graph, and candidates; already materialized
run/state handles remain. Closing the compiled session also releases all child
search sessions. Protocol v2 does not provide search forking or mutation of an
existing search's goal or heuristic.
