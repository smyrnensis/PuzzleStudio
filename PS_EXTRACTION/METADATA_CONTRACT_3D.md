# 3D Metadata Contract

This document traces how PuzzleScript Next 2D consumes prelude/metadata settings
and maps those settings to 3D contracts.

The purpose is not to decide what the 3D MVP implements. The purpose is to
preserve the same architectural slots as 2D so unsupported settings can be added
later without redesigning the runtime boundary.

## 2D Metadata Pipeline

The 2D path has five distinct metadata stages.

### 1. Parser Acceptance

`src/js/parser.js` classifies prelude terms into:

- flags: `prelude_keywords`
- text parameters: `prelude_param_text`
- number parameters: `prelude_param_number`
- single-value parameters: `prelude_param_single`
- multi-value parameters: `prelude_param_multi`
- explicitly unimplemented terms: `prelude_not_implemented`

3D implication:

- 3D should reuse the same parser acceptance where possible.
- Rejection should happen through a 3D support policy after parsing, not by
  silently removing parser terms.

### 2. Compiler Normalization

`src/js/compiler.js` converts parser metadata pairs into `state.metadata`,
validates and normalizes selected values, and stores `state.default_metadata`.

Trace:

- `twiddleMetaData(state)` converts metadata pairs into an object.
- It normalizes `flickscreen`, `zoomscreen`, `smoothscreen`, `tween_easing`,
  `tween_snap`, and `color_palette`.
- `generateExtraMembersPart2(state)` resolves mouse metadata into object IDs and
  masks.
- compile setup uses `debug`, `verbose_logging`, and `throttle_movement` to set
  global runtime flags.

3D implication:

- 3D needs a metadata normalization contract before runtime construction.
- Mouse object references and color/presentation values should be normalized in
  shared compile/presentation policy, not inside the 3D core transition logic.
- Runtime-mutable metadata must retain defaults so `default` can restore values.

### 3. Core/Session Runtime Extras

`src/js/engine.js` applies metadata to runtime timers and session state through
`twiddleMetadataExtras`.

Trace:

- `realtime_interval` -> `autotickinterval`
- `again_interval` -> `againinterval`
- `tween_length` -> `tweeninterval`
- `key_repeat_interval` -> `repeatinterval`
- `animate_interval` -> `animateinterval`
- colors -> `state.bgcolor`, `state.fgcolor`, `state.author_color`,
  `state.title_color`, `state.keyhint_color`

3D implication:

- 3D needs the same split between core timers, input repeat timers, presentation
  timers, and colors.
- Even if unsupported initially, each setting needs a contract slot.

### 4. Runtime Metadata Mutation

2D supports rule-triggered runtime metadata changes when
`runtime_metadata_twiddling` is enabled.

Trace:

- `engine.js` checks `state.metadata.runtime_metadata_twiddling`.
- rule commands can mutate `state.metadata`.
- `zoomscreen`, `flickscreen`, `smoothscreen`, and `color_palette` have
  immediate side effects such as `twiddleMetaData`, `canvasResize`,
  `initSmoothCamera`, and `regenSpriteImages`.
- all metadata changes call `twiddleMetadataExtras`.

3D implication:

- 3D must not treat metadata as immutable if it wants compatibility.
- The contract needs a mutation policy even if the first implementation rejects
  runtime twiddling.
- Mutation side effects must route to the correct owner: core/session/renderer.

### 5. Direct Consumers

Metadata is consumed directly by multiple 2D modules:

- `engine.js`: title flow, level select, undo/restart, run-on-start,
  realtime/again, require-player-movement, runtime mutation, local storage,
  flick/smooth camera helpers.
- `inputoutput.js`: keyboard/action behavior, pause, level select escape flow,
  mouse input, key repeat, realtime ticks.
- `graphics.js`: palette, sprite/font sizing, status line, flickscreen,
  zoomscreen, smoothscreen, tween animation.
- `compiler.js`: object resolution, mouse IDs, color normalization, debug flags,
  warnings.

3D implication:

- "Upper layer can handle it" is only safe for settings whose 2D consumers are
  already upper/session-level.
- Settings consumed by the 2D core loop need matching 3D core/session hooks.

## Contract Slots

3D should split metadata into these contracts.

```txt
metadata3d
  compiler
  core
  session
  input
  renderer
  mutation
  upper
```

Unsupported settings should still be classified into one of these slots. The
slot may return an unsupported diagnostic, but it should exist.

## Setting Classification

### Compiler Contract

These affect parsing, lowering, object identity, diagnostics, or normalized
metadata before runtime starts.

| Setting | 2D trace | 3D contract |
| --- | --- | --- |
| `case_sensitive` | Parser changes token case handling; affects object/glyph lookup. | Compiler. Must be honored because `LEVELS3` glyph parsing already depends on it. |
| `debug` | Compiler enables debug output in IDE. | Compiler/debug. Optional for execution, but slot should exist. |
| `verbose_logging` | Compiler enables runtime verbose logging. | Compiler/session logging. |
| `debug_switch` | Parsed as text and checked by runtime debug traces. | Compiler/session logging. |
| `export_options` | Parsed as text for export behavior. | Upper/export. |
| `color_palette` | Normalized in `twiddleMetaData`; also renderer/presentation. | Compiler normalization plus renderer palette. |
| `mouse_left`, `mouse_drag`, `mouse_up`, `mouse_right`, `mouse_rdrag`, `mouse_rup`, `mouse_obstacle` | Compiler resolves object IDs/masks in `generateExtraMembersPart2`. | Compiler/input. Unsupported initially, but object-reference validation belongs here. |
| `runtime_metadata_twiddling` | Compiler validates twiddled commands and engine applies mutations. | Compiler/mutation. |

### Core Runtime Contract

These affect deterministic turn execution or rule-loop timing.

| Setting | 2D trace | 3D contract |
| --- | --- | --- |
| `again_interval` | `twiddleMetadataExtras` sets `againinterval`; input loop schedules again turns. | Core/session timer. |
| `realtime_interval` | `twiddleMetadataExtras` sets `autotickinterval`; `update()` triggers tick turns. | Core/session timer. |
| `run_rules_on_level_start` | `loadLevelFromLevelDat` calls `processInput(-1, true)` after loading. | Core lifecycle hook. |
| `require_player_movement` | `processInput` cancels turn when player did not move. | Core post-turn validation. |
| `local_radius` | Engine rule matching checks local/global application radius. | Core rule application. |
| `runtime_metadata_twiddling` | Rule commands mutate metadata during execution. | Core command/mutation gate. |

Notes:

- `tween_length` is not core transition semantics, but it affects animation
  timing and sometimes turn pacing. Keep it out of core transition logic but
  give it a renderer/session contract.
- `again_interval` is timing, but the existence of again turns is core/session
  behavior.

### Session Contract

These affect level flow, title/menu/pause, undo/restart availability, save
state, or screen state.

| Setting | 2D trace | 3D contract |
| --- | --- | --- |
| `skip_title_screen` | `setGameState` skips title flow and starts a level/selector. | Session startup. |
| `continue_is_level_select` | title continue routes to level select. | Session/title flow. |
| `enable_pause` | `inputoutput.js` escape key opens pause screen. | Session input/menu. |
| `level_select` | Engine/input route level select screens and progression. | Session level progression. |
| `level_select_lock` | Level select unlock logic. | Session level progression. |
| `level_select_unlocked_ahead` | Level select unlock logic. | Session level progression. |
| `level_select_unlocked_rollover` | Level select unlock logic. | Session level progression. |
| `level_select_solve_symbol` | Level select UI symbol. | Session/UI presentation. |
| `allow_undo_level` | Level load clears backups unless allowed. | Session undo policy. |
| `noundo` | Title/help text and input command behavior. | Session/input capability. |
| `norestart` | Title/help text and restart option behavior. | Session/input capability. |
| `checkpoint` command support | Storage/restart target path uses checkpoint snapshots. | Session save/snapshot policy. |
| `sitelock_origin_whitelist`, `sitelock_hostname_whitelist` | `compile` blocks play in disallowed hosts. | Upper/session host policy. |

### Input Contract

These affect how host input becomes runtime commands.

| Setting | 2D trace | 3D contract |
| --- | --- | --- |
| `noaction` | `inputoutput.js` ignores action input. | Input mapping. |
| `norepeat_action` | Engine/input repeat behavior. | Input repeat policy. |
| `nokeyboard` | Compiler/input require player object or suppress keyboard. | Input source policy. |
| `throttle_movement` | Compiler sets global flag; input repeat loop changes movement repeat timing. | Input repeat/session timer. |
| `key_repeat_interval` | `twiddleMetadataExtras` sets `repeatinterval`. | Input repeat timer. |
| `mouse_clicks` | input routes mouse clicks into actions. | Input source policy. |
| `mouse_*` object settings | compiler resolves IDs; input places mouse objects and starts turns. | Compiler/input policy. |

### Renderer / Presentation Contract

These affect drawing, camera, animation, text rendering, or visual state.

| Setting | 2D trace | 3D contract |
| --- | --- | --- |
| `flickscreen` | Normalized in compiler; engine initializes `oldflickscreendat`; graphics changes viewport size. | Renderer viewport/camera. |
| `zoomscreen` | Normalized in compiler; graphics changes viewport. | Renderer viewport/camera. |
| `smoothscreen` | Normalized to object; engine initializes and updates camera target; graphics animates camera. | Renderer camera with session timing. |
| `smoothscreen_debug` | Normalized into smoothscreen debug flag; graphics draws debug. | Renderer debug. |
| `tween_length` | `twiddleMetadataExtras` sets `tweeninterval`; graphics animates moves. | Renderer animation plus session timer. |
| `tween_easing` | Compiler validates easing; graphics reads it. | Renderer animation. |
| `tween_snap` | Compiler normalizes snap; graphics reads it. | Renderer animation. |
| `animate_interval` | `twiddleMetadataExtras` sets `animateinterval`. | Renderer animation timer. |
| `sprite_size` | Graphics uses for cell/sprite scaling. | Renderer sprite layout. |
| `font_size` | Graphics uses for text rendering. | Renderer text layout. |
| `custom_font` | Engine loads font; graphics uses it. | Renderer asset/text. |
| `load_images` | Engine loads images and regenerates images. | Renderer asset loading. |
| `scanline` | Graphics effect. | Renderer effect. |
| `status_line` | Graphics reserves status line height. | Renderer HUD layout. |
| `background_color`, `text_color`, `author_color`, `title_color`, `keyhint_color` | `twiddleMetadataExtras` converts to display colors. | Renderer/theme. |
| `text_controls`, `text_message_continue`, `message_text_align` | Engine text/title/message screens. | Upper/session UI, not core. |

### Upper Metadata Only

These do not need to reach lower core runtimes except as labels or UI metadata.

| Setting | 2D trace | 3D contract |
| --- | --- | --- |
| `title` | title screen/document title. | Upper/session UI. |
| `author` | title screen. | Upper/session UI. |
| `homepage` | compiler formats URL. | Upper/export/UI. |
| `puzzlescript`, `youtube` | parsed multi metadata, presentation/docs-style metadata. | Upper/export/UI unless a runtime consumer is introduced. |

## Initial 3D Runtime Config Shape

3D runtime construction should receive a filtered config rather than raw
`state.metadata`.

```js
{
  compiler: {
    caseSensitive,
    debug,
    verboseLogging,
  },
  core: {
    again: { supported, intervalMs },
    realtime: { supported, intervalMs },
    runRulesOnLevelStart: { supported },
    requirePlayerMovement: { supported },
    localRadius: { supported, value },
    metadataMutation: { supported },
  },
  session: {
    undo: { enabled, allowLevelUndo },
    restart: { enabled },
    levelSelect: { supported },
    pause: { supported },
    titleFlow: { skipTitleScreen, continueIsLevelSelect },
    checkpoint: { supported },
  },
  input: {
    keyboard: { enabled },
    action: { enabled, noRepeat },
    movementRepeat: { throttle, repeatMs },
    mouse: { supported },
  },
  renderer: {
    viewport: { flickscreen, zoomscreen, smoothscreen },
    tween: { supported, lengthMs, easing, snap },
    palette,
    text,
    assets,
  },
  upper: {
    title,
    author,
    homepage,
    exportOptions,
  }
}
```

Each leaf may be supported or unsupported. The important contract rule is that
the slot exists and has a single owner.

The detailed 3D slot shape is defined in `SLOT_DESIGN_3D.md`. The short
version is that most metadata slots mirror 2D. The true 3D differences are
owned by `core.frame`, `core.directions`, `core.board`, `core.rules`, and
`input.bindings`.

## Implementation Rules

1. Do not pass raw `state.metadata` directly into 3D core transition logic.
2. Normalize metadata once, then build explicit 3D config slots.
3. If 2D has a core/session/renderer branch for a setting, 3D must classify it
   into the same kind of slot even when unsupported.
4. Unsupported settings should fail at 3D runtime construction or compile-time
   support validation, not halfway through a turn.
5. Renderer settings must not mutate core puzzle state.
6. Runtime metadata mutation must dispatch side effects to the owning slot:
   core timers, session policy, renderer camera, renderer palette, or assets.
7. Do not let shared upper-layer convenience hide a setting whose 2D consumer is
   actually in the core turn loop.

## Highest-Risk Settings

These need explicit tests before being accepted in 3D:

- `runtime_metadata_twiddling`
- `again_interval` and `again`
- `realtime_interval`
- `run_rules_on_level_start`
- `require_player_movement`
- `flickscreen`, `zoomscreen`, `smoothscreen`
- `tween_length`
- `mouse_*`
- `level_select*`
- `allow_undo_level`, `noundo`, `norestart`, checkpoint behavior
