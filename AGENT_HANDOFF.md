# Agent Handoff

この文書は、後続エージェントが現在の設計判断と実装状況を素早く把握するための要約である。

## Current Goal

高速で決定論的な transition core を保ちつつ、`.puzzle` 言語処理と play UI を分離して育てている。

```txt
.puzzle file
  -> puzzle-lang parser/compiler
  -> puzzle-core CompiledGame
  -> puzzle-play session/render helpers
  -> transition_state / transition_trace
```

## Run / Test

```bash
cargo test
cargo run -p ascii-play -- games/spec_2d/game.puzzle
cargo run -p puzzlestudio-cli -- check games/spec_2d/game.puzzle
cargo run -p puzzlestudio-cli -- export-html games/spec_2d/game.puzzle -o /tmp/game.html
```

Controls in ASCII play:

- `w/a/s/d` or arrow keys: move
- `r`: sends the standard non-direction `restart` input; it is not an automatic session restart effect
- `n`: next level after clear
- `q`: quit

## Important Design Decisions

### Core Independence

`puzzle-core` must not depend on terminal rendering, file IO, `.puzzle` parsing, game-specific rules, or level formats.

`puzzle-lang` owns `.puzzle` parsing, authoring syntax, validation, and lowering into `puzzle-core` IR.

`puzzle-play` owns loaded-game session mechanics such as undo/restart/level advance and display helpers.

`ascii-play` and `html-play` are adapters. They should not own parser/compiler behavior.

### Editor Host Boundaries

The HTML editor, server mode, and Tauri desktop shell should share the same editor service for source loading, preview compilation, highlighting, export, and saving semantics. Do not fork compile or preview logic for desktop.

Web mode should keep the browser-shaped workflow: import folder or zip into browser/editor state, then download/export files. It should not assume durable direct access to a local folder.

Tauri desktop mode may directly edit files, but only after the user explicitly opens a project folder or project entry. Startup should be empty and must not auto-load `games/`, the repository, or the user's home directory. File reads/writes should be owned by Rust-side host commands bounded by the opened project root; do not expose broad filesystem access to JavaScript unless a concrete feature requires it.

Platform divergence should happen last, at the host adapter/file-access boundary, after the shared editor service behavior is aligned.

Current Tauri shell status: `src-tauri` opens the shared HTML editor from `crates/html_editor/static`, starts with no project loaded, and exposes Rust commands for `load_source`, `open_project`, `compile_preview`, `highlight_source`, and `save_source`. `open_project` uses Tauri's Rust-side native dialog plugin and does not accept frontend-supplied filesystem paths; it resolves the chosen folder through `EditorService::open_game_entry`. `EditorService::open_game_entry` scopes `workspace_root` to the selected project folder or selected entry `.puzzle` parent, while `save_source` and `compile_preview` go through that opened service and reject paths outside the workspace root. `cargo tauri build --bundles app` produces a macOS `PuzzleStudio.app` bundle; signing/notarization and installer-style release packaging are still pending.

### Runtime State

`State` includes `width`, `height`, and `layer_count` for speed.

Cell encoding:

```txt
slot_index = ((y * width + x) * layer_count + layer_id)
slots[slot_index] = object_id | EMPTY
```

`EMPTY = ObjectId(0)`.

### Cell Model

Cell is a finite set of visible objects, constrained by layer slots.

```txt
at most one object per (cell, layer)
```

No invisible multiplicity. If count matters, model it visibly.

### Input

`input` is not canonical state.

It is a transition context value passed into:

```txt
transition(state, input)
```

Rules see it through guards:

```txt
if input == right
if input == d
```

Core IR represents this as:

```rust
Guard::InputIs(InputId)
```

### Surface Syntax

Surface syntax example:

```txt
rules {
for d in directions {
if input == d {
once d [ Player | Box | ] -> [ | Player | Box ]
once d [ Player | ] -> [ | Player ]
}
}
}
```

`directions` is a built-in ordered tag set whose values are `up` / `down` / `left` / `right`. Those four semantic inputs and direction mappings are provided by default. Inputs are the semantic names produced from physical keys or UI events; only inputs with direction mappings can act as rewrite orientations. `direction <alias> <up|down|left|right>` defines a direction/input-context alias such as `east -> right`. Numeric `direction <input_name> <dx> <dy>` is not public syntax.

Input-gated movement should spell out both parts: `if input == d` checks the transition input name, and `d [ ... ] -> ...` gives the rewrite orientation. `directions [ ... ]`, `horizontal [ ... ]`, and `vertical [ ... ]` are orientation set prefixes that expand to their member directions without adding input guards. `input directions [ ... ]`, `input horizontal [ ... ]`, and `input vertical [ ... ]` are input-guarded orientation-set sugar, equivalent to expanding the set and adding `if input == d` for each member. Legacy `input directions [ ... ]` still parses with the previous input-relative meaning but should not be used as the new mental model.

Puzzle `rules` is required. `routine` is a named statement list and does not run until called from puzzle `rules` or another routine. Legacy puzzle `transitions` / `main` blocks and `rule` declarations are rejected.

`for d in directions { ... }` expands the statement list per direction. `for d in directions ... end` is still accepted as legacy syntax. Inside `for`, the rewrite itself should be oriented, e.g. `d [ ... ] -> [ ... ]`; the mental model is oriented rewrites such as `left [ ... ] -> [ ... ]`.

Prefixless spatial patterns are PuzzleScript-compatible cardinal patterns. A prefixless pattern with multiple cells, multiple rows, ellipsis, or relative direction movement scratch lowers to `up` / `down` / `left` / `right` variants. This applies uniformly to rewrites, pattern conditions, and query patterns such as `exists([ Player | Wall ])`. Prefixless single-cell patterns remain neutral.

### Patch

Rules do not mutate state directly.

Transition builds a patch and applies it as a single unit.

Patch updates derived cache such as `object_counts`.

## Implemented Files

Top-level docs are split by audience:

User-facing docs:

- `README.md`
- `AUTHORING_SYNTAX.md`

Developer-facing docs:

- `DESIGN_PRINCIPLES.md`
- `CURRENT_SPEC.md`
- `IMPLEMENTATION_PLAN.md`
- `CLI_IMPLEMENTATION_PLAN.md`
- `SOLVER_DESIGN.md`
- `AGENT_HANDOFF.md`

Core:

- `crates/core/src/ids.rs`
- `crates/core/src/compiled_game.rs`
- `crates/core/src/state.rs`
- `crates/core/src/patch.rs`
- `crates/core/src/transition.rs`

Language:

- `crates/lang/src/lib.rs`

Play helpers:

- `crates/play/src/lib.rs`

Adapters:

- `crates/ascii_play/src/main.rs`
- `crates/ascii_play/src/lib.rs`
- `crates/html_play/src/main.rs`
- `crates/cli/src/main.rs`
- `games/spec_2d/game.puzzle`

## Current Implemented Capabilities

`puzzle-core`:

- ID newtypes
- layer-slot state
- object count derived cache
- rule guards
- required / forbidden object matching
- add / remove / replace writes
- all-or-nothing patch apply
- `transition_state`
- `transition_trace`

`puzzle-lang`:

- `.puzzle` file loading
- game folder entry resolution is prelude-based: a `.puzzle` with top-level `title` / `subtitle` / `author` / `homepage` metadata is a game entry. Folder paths resolve to the best prelude-bearing `.puzzle` in that folder, preferring `game.puzzle`, `<folder>.puzzle`, then `main.puzzle`. Prelude-less fragments resolve by searching the same folder and then parent folders for a game entry.
- initial vanilla PuzzleScript import via `translate_puzzlescript_to_canonical`; it translates a small Sokoban-oriented PS subset into canonical `.puzzle` instead of widening canonical syntax directly. The `ps_to_puzzle` helper binary writes translated files. The first pinned mapping lives in `crates/lang/tests/fixtures/puzzlescript/`.
- objects / layers
- top-level `title <text>` and optional `subtitle <text>` / `author <text>` / `homepage <text>` metadata. Top-level `name <text>` is intentionally rejected to avoid confusion with scene/model/level names. Scene/display expressions can read `game.title`, `game.subtitle`, `game.author`, and `game.homepage`.
- compact named layer declarations with `layers { floor = Goal Button }`
- named layers usable as selector tags
- `legend <char> = <selector...>` for display, level chars, and overlays belongs directly inside `levels { ... }`; model-level `legend` is rejected
- level bodies can contain level-local `legend` directives/blocks for parse-only chars scoped to that level. Braced `level { ... }` bodies can also contain level-local `on_level_start { ... }` / `on_level_clear { ... }` statement blocks. As sugar, `message` / `sfx` / `wait` before the first ASCII row lower to that level's `on_level_start`, and the same commands after the ASCII rows lower to that level's `on_level_clear`.
- canonical `levels { ... }` entries use `level <name>` followed by rows, with blank lines separating unbraced levels; unnamed row chunks are accepted as unnamed levels, and braced `level <name> { ... }` / `{ ... }` forms are for multi-region levels that need blank lines inside the body
- finite ordered tag sets declared with `tags { color = red blue }`
- object schemas such as `object player:color 1`
- schema selectors such as `player:*`, `player:red`, `player:color`, `player:left`; bare schema family names such as `player` are not all-variant selectors
- render overlays
- input key and arrow mappings
- `win_conditions = exists(A) and count([ A no B ]) == 0`
- multiple levels
- default cardinal directions from `up/down/left/right`
- optional `direction <alias> <up|down|left|right>` for direction/input-context aliases
- required puzzle `rules { ... }` entrypoint; legacy `transitions { ... }` / `main { ... }` are rejected
- `routine display <name>` declares a display routine. `display <name>`, `display <rewrite>`, and statement-local `display { ... }` run display behavior at that exact point in puzzle `rules` / lifecycle blocks.
- `on_display { display <routine> }` is a display-only snapshot hook for renderer/editor projection. It runs without input, can only contain display statements, and must not be used for gameplay state.
- display-only state objects use `@Name` declarations in `layers`; parser compatibility for `display_objects { ... }` has been removed.
- `scratch { ... }` for transition-local facts; `{mark}` is cell-anchored, `Object{mark}` is occurrence-anchored, and all scratch is cleared before returned state / solver keys.
- movement sugar such as `> Box` lowers to builtin occurrence scratch `__move`; `parallel` / `perpendicular` are relative movement scratch set sugar and expand to `<` / `>` or `^` / `v` alternatives during oriented lowering.
- main and display objects share one layer namespace/order; canonical syntax declares object roles separately from puzzle-level `layers`
- `layers { each A:tag_set }` expands selector alternatives into separate ordinary layers, preserving display order
- display routines can read main objects and call-site transition input, but can only write display objects and cannot use effects
- main routines and gameplay conditions cannot read or write display objects
- sprite reuse is owned by `sprites` sub-blocks: `colors { ... }`, `palettes { ... }`, and `shapes { ... }`. Sprite entries reference them with `palette <ref>` and `shape <ref>`, and `shape <name>:<tag_set> rotate from <value>` inside `shapes` expands rotated ASCII variants via `map rotate <tag_set>`.
- named statement lists with `routine <name> { ... }`
- routine block application is `repeat` by default; use `routine <name> once` for single block pass
- rewrite line application is also `repeat` by default; use `once <direction> ...` for first-match application, `once_all ...` for one pass over all current matches, and `once_per_level ...` for a rule that can fire once in the current level state
- `repeat until <condition> { ... }` is a pre-check block loop; condition uses the same var / named condition / query / pattern condition language as rule `if`
- `repeat`/until-stable cycle detection is progress-based: a sweep or rule application that leaves the exact state unchanged is stable/no progress, not a cycle. Reaching an earlier exact state after crossing at least one distinct state is a cycle; the runtime keeps the revisited current state and ends that repeat instead of rolling back to the repeat boundary. Non-cycling divergent repeats stop at the internal repeat limit of 200 and keep the last reached state. `cancel` still takes priority over repeat cycle / limit handling. Browser exports log a warning for cycle / limit diagnosis.
- routine calls from puzzle `rules` or another routine
- `for <binding> in directions|horizontal|vertical`
- `if input == <binding>`
- rule conditions and query pattern args can carry explicit orientation, e.g. `no down [ Rock | ]`, `some(down [ Rock | ])`, `count(down [ Rock | ])`, set-oriented `some(horizontal [ Rock | ])`, and input-guarded set-oriented `some(input horizontal [ Rock | ])`; legacy input-relative patterns such as `some(input directions [ Rock | ])` still make non-direction inputs false
- puzzle screen viewport directives are author-facing `flickscreen <w> <h>` for paged movement and `zoomscreen <w> <h>` for centered follow movement. `screen_focus <selector>` sets the focus object. Internal parser/export names use `viewport_*`; removed `frame_*` directives are not canonical.
- `inputs { <input> <- <key...> }` is owner-scoped. Model `inputs` maps raw keys to puzzle/model semantic inputs; scene `inputs` maps raw keys to scene-wide semantic inputs such as title `confirm` or playing `back`. Scene-level `keys` is rejected. Supported named key tokens include arrows plus `Enter`, `Space`, `Escape`, `Tab`, and `Backspace`. Prefer the mental model `raw key/button -> owner inputs -> semantic input -> rules/component behavior`; do not reintroduce author-facing `action` syntax.
- `if` is the condition guard in both routine statement lists and scene transitions; `when` is not accepted
- `var` / `const` are scoped by owner: top-level session value, scene instance value, or puzzle state slot; puzzle `global` is rejected
- puzzle `const` can be read by guards but cannot be updated by rewrite effects; scene `const` cannot be overwritten by scene params
- `persistent var` preserves values across that owner's normal reinitialization; legacy puzzle `persistent <name> = ...` is rejected
- rule `if` accepts bare puzzle vars as truthy checks, and `else` lowers to negated guards
- puzzle rule effects are written directly as statements at that statement position; accepted effects are the same rule effects as rewrite suffixes, not scene effects. Legacy `do <effect>` is obsolete and rejected.
- scene `<slot>.<name>` conditions resolve named conditions first, then fall back to truthy puzzle vars on that puzzle slot
- lifecycle hooks are `on_level_start { ... }` / `on_level_clear { ... }`; legacy `level_start { ... }` / `level_clear { ... }` are still accepted, while two-word `on level_start` / `on level_clear` are not accepted. `on_level_start` is runtime lifecycle, not parser materialization: raw `Level.initial_state` remains the parsed map, and play runtimes apply the hook on level entry/restart/navigation while collecting rule emissions such as `message` and `sfx`.
- component behavior is owned by the component; `level_menu` owns cursor movement and enter, so canonical authoring syntax does not expose `cursor.*`, `emit`, `selected`, or menu-specific action commands
- `level_menu` starts the selected level on enter by default; use ordinary `button ... -> <scene-command>` entries only for extra commands such as Back
- scene layout primitives are `row`, `column`, and `box`. `box` is a pure transparent layout rectangle with no default border/background. `panel` was removed as a scene/layout primitive because its styled-container meaning conflicts with the ownership boundary; do not reintroduce it as compatibility syntax.
- Scene is shared presentation/flow metadata, not a 2D or 3D model owner. The generic scene component keywords are `title`, `subtitle`, `text`, `button`, `row`, `column`, `box`, `for`, `level_menu`, and `menu`; model window components are `puzzle` for a 2D puzzle slot and `puzzle3` for a 3D puzzle slot. `view` is the scene root layout block, not a component.
- Shared scene layout metadata lives in `puzzle-scene`, not separately in the 2D and 3D parsers. `view`, `row`, `column`, and `box` can carry the same header attributes such as `size <w> <h>`, `gap <n>`, and `align <x> [y]`; `view size 720 540 { ... }` is the canonical root layout form for scene-sized layout, independent of whether the scene contains a 2D `puzzle` or 3D `puzzle3` model window.
- The 2D scene component tree now uses the shared `puzzle-scene` component shape with 2D-specific `SceneEffect`, `SceneExpr`, and text content payloads. A 2D board component is represented as a shared `ModelWindow` with `ModelKind::Puzzle2d`, not as a separate 2D-only `PuzzleState` component variant.
- Component embed mode is a model-window contract, not nested scene playback: when a model is embedded as a component, the parent scene owns the outer screen size and input dispatch, and the embedded runtime exposes only the model window inside that host area. The child runtime must not reinterpret its own scene `row` / `column` / `box` layout inside the component iframe.
- In the component input model, the game has one raw input stream, raw input enters the focused scene, and mapping keys/buttons to named inputs is owned by the component/model. A `puzzle3` component may declare `inputs { forward <- w ArrowUp }`; compact authoring aliases lower to standard key values such as `w` / `ArrowUp`. The 2D HTML component path also accepts raw `PuzzleStudioKey { key, code }` messages and resolves them through its own loaded input table. Scene-level 3D `controls { ... }` and `rules { board.rules with input { ... } }` still exist as experimental compatibility syntax, but the current design direction is default focused-scene broadcast plus component-owned input interpretation. Undo is session-level. Restart is both a semantic model input and a model effect; scene handling must use explicit target commands such as `board.restart`.
- 3D scene-level `keys { ... }` is compatibility/prototype drift, not canonical syntax. New examples and migrations should use owner-scoped `inputs { <input> <- <key...> }` so 2D and 3D scenes share the same input contract.
- 3D camera/render options are model-owned renderer metadata. Canonical syntax wraps them as `render { camera { yaw <deg> pitch <deg> zoom <n> interactive_look <bool> interactive_zoom <bool> } grid { occupied_cells <bool> } shade <bool> }` inside `model puzzle3`; legacy top-level `debug_camera`, `camera_yaw`, `camera_pitch`, and `camera_zoom` are compatibility only. `render { shade false }` disables face-light shading for 3D sprite voxels without changing sprite data or puzzle state. `interactive_look` is not a semantic input: raw pointer input reaches `puzzle3` through normal focused-scene component dispatch, and a pointer drag that starts inside the component box may be captured by that component and used only for camera yaw/pitch view-state updates.
- scene visibility effects `show <scene>`, `hide <scene>`, and `toggle <scene>` remain canonical scene-level effects
- level lifecycle commands are target-qualified: `start levels in playing` / `start levels <scope> in playing` for starting from the first accepted level, `continue levels in playing` / `continue levels <scope> in playing` for resuming the selected or restored current level, `playing.restart`, `playing.next_level`, `playing.previous_level`, `playing.goto <level>`, and slot-targeted forms such as `board.restart`; bare menu-level `restart` / `restart_level` / `next_level` are legacy-style and should not be used in new examples. Normal clear / advance / restart belongs to the model window component and puzzle lifecycle; scene target commands are for explicit intervention such as buttons, menus, hubs, debug, or exceptional flow.
- scene conditions can read current level context: `level.name == <name>`, `level.name != <name>`, `level.label == <label>`, `level.label != <label>`, `level.last`, and `level.has_next`. Prefer `level.name` for authoring; do not introduce index/number level conditions as canonical syntax. Do not make scene conditions the standard owner of level progression.
- top-level `sounds { ... }` defines named `sfx` and `music`; scene/component effects can emit `sfx <name>`, `play_music <name>`, `pause_music [name]`, `resume_music [name]`, `stop_music [name]`, popup `message <expr>`, and presentation wait `wait [duration]` such as `wait`, `wait 0.1s`, `wait 1s`, or `wait 100ms`; `play_sfx <name>` is rejected. Bare `wait` defaults to `0.2s` and top-level `default_wait_time = 500ms` can change that default. Scene-level lifecycle is `on_scene_start { ... }`; `on_level_start { ... }` is puzzle lifecycle only and is rejected in scenes. Browser adapters play these events; `puzzle-core` remains sound-playback-free, timer-free, and message-state-free.
- top-level `assets { ... }` declares external HTML build inputs with `css "..."` and `script "..."` entries. CSS and scripts are loaded only when declared here; same-folder `game.css` / `visuals.js` are no longer implicit. Asset paths are game-folder relative. Scripts are display helpers over rendered scene snapshots, not gameplay extensions.
- top-level `theme <theme>` / `theme <theme> { ... }` declares HTML display theme metadata. Theme identity belongs to HTML CSS presets; `.puzzle` theme declarations select the preset name and, in the braced form, expose only a small override API (`accent_color`, `background_color`, `text_color`, `muted_text_color`, `line_color`, `board_color`, `ui_font`, `title_font`, `control_radius`, `panel_radius`). These lower to CSS custom properties in HTML adapters only; theme is not core state. The default theme name is `clean` when no theme declaration is provided.
- Built-in theme imports currently include `clean`, `terminal`, `paper`, `pixel`, `puzzlescript`, `candy`, `blueprint`, and `noir`.
- puzzle rule effects can emit `win`, `restart`, `next_level`, `message`, and `sfx`; `win` is a clear outcome command that treats `win_conditions` as true for that turn, roughly like `set win_conditions = true` sugar without mutating the condition definition or board objects. `restart -> restart` is the canonical input/effect sugar and is added implicitly when no `restart` input handler is written. `[ Goal Box ] -> next_level` and `if win_conditions -> next_level` produce a core transition command that the owning model window component / runtime converts into level advance. `[ Player Goal ] -> message "Found"` and `[ Player Box ] -> message hint` produce a presentation command that `puzzle-play` / standalone HTML convert into a popup. `[ Player | Box | ] -> [ | Player | Box ] sfx push` produces a named SFX command when the rule matches. `win_conditions` remains metadata, but a defined named condition can be referenced from puzzle rule `if`.
- turn completion is owned by `puzzle-play` / standalone HTML, not `puzzle-core`: after puzzle rules run, runtime evaluates `win_conditions` on the post-rules/pre-navigation snapshot, runs model `on_level_clear` before navigation when clear, and resolves queued level navigation commands through the owning model window component/runtime. Scene condition transitions are for overlay, menu, hub, debug, or exceptional flow intervention, not the default owner of level progression.
- scene command sequences use block form with one command per line; inline `then` is not accepted
- rectangular inline rewrite blocks: `[ A | B ; C | D ] -> [ A | B ; C | E ]`
- disconnected inline rewrite blocks: `[ A ] [ B ] -> [ A ] [ ]`
- row ellipsis inside rewrite blocks: `[ A | ... | B ] -> [ A | ... | C ]`
- application-prefixed inline rewrite: `once right [ A | B ] -> [ C | D ]`, `once_all [ A ] -> [ B ]`, `once_per_level [ Door ] -> [ OpenDoor ]`, `repeat right [ A | B ] -> [ C | D ]`
- oriented inline rewrite: `<direction-or-binding> [ A | B ] -> [ C | D ]`
- selector alternatives lower to concrete low-level rule variants

`puzzle-play`:

- session state
- undo / redo
- restart / level advance
- progress save data for cleared levels, current level, and persistent puzzle vars; host storage belongs to adapters
- ASCII legend rendering helper

`ascii-play`:

- terminal file selection
- terminal key reading
- terminal screen refresh

## Known Gaps

- PuzzleScript import is intentionally minimal: it currently covers title/author metadata, PS `background_color` / `text_color` lowered to canonical `theme` overrides, PS special `Background` as a real object plus an `on_level_start` background fill (`once_all [ no Background ] -> [ Background ]`), PS `run_rules_on_level_start` lowered to canonical `routine __ps_main once` plus `on_level_start`, basic `OBJECTS` including PS-style color/pattern sprites and one-character object shorthand, `LEGEND` rows including property aliases lowered to `group`, `COLLISIONLAYERS`, directional `RULES`, `+` continuation rows lowered to a repeated rule group, canonical `again` rule effects, `late` rules emitted after `move`, PS `sfx0`-style rule suffixes lowered to canonical `sfx sfx0`, simple PS `SOUNDS` named seed rows lowered to `sounds { sfx <name> seed=<seed> type=puzzlescript }`, `WINCONDITIONS`, and blank-line-separated `LEVELS`. Runtime handles `again` as a no-input follow-up turn after the current turn commits, capped at 256 automatic turns per input. `again` does not resend the previous key or semantic input; it reruns the same puzzle target's rule entrypoint with no input, so input guards are false in automatic turns. Standalone HTML schedules those turns at `defaultAgainMs` (currently 120ms) and publishes each auto-turn as its own snapshot so `sfx` emissions play once per `again` turn. When PS rules are moved into the `__ps_main once` routine, individual rewrite rows are explicitly prefixed with `repeat` so rule application semantics are not accidentally weakened to `once`. PS `message ...` entries inside `LEVELS` are emitted as canonical level-local start messages on the following imported level. It emits a canonical `scene title` first, with default title/subtitle and `button "Play" -> start levels in playing`, plus `scene playing` with `state { board = puzzle main }` and `view { puzzle board }`. It relies on canonical prefixless `>` implicit cardinal expansion and the built-in `move` routine rather than generating ad hoc movement-resolution rules. `moving` / `stationary` qualifiers are currently approximated by movement scratch where possible. Synonyms, aggregates, event-based sounds such as object movement/create sounds, random rules, and checkpoints are still future work.
- Rewrite blank cells are unspecified unless paired with before-side objects to remove; absence checks use `no <selector>`.
- `group <name> = <selector...>` is supported for rule selectors.
- No property matching beyond explicit groups yet.
- No `for c in color` value binding yet.
- No phases.
- No visible var writes beyond the current `var` effect support.
- No event emission.
- No solver crate yet.
- `transition_state` still clones state; hot path is not optimized.
- Trace is minimal.
