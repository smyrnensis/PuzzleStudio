# Agent Notes

This crate is a temporary 3D authoring/runtime facade.

The deterministic 3D grid core lives in `crates/grid3d`. Do not add new state,
patch, transition, win-condition, level-state construction, coordinate, offset,
direction, or frame mechanics here.

Remaining responsibilities in this crate should migrate toward their normal
owners:

- `.puzzle3` parsing, validation, semantic scanning, and lowering belong in
  `crates/lang`.
- session flow, undo/restart, level navigation, and lifecycle behavior belong in
  `crates/play`.
- visual fixture export and host-facing presentation behavior belong in
  adapters.

3D authoring must stay logically isomorphic with the 2D authoring path. Do not
add an independent parser interpretation for non-spatial syntax. Shared
authoring concepts such as selectors, `no`, scratch blocks, prefix scratch sugar
like `> Player`, inputs, lifecycle names, and scene commands must use shared
authoring helpers or the same lowered meaning as 2D. 3D-specific authoring code
may branch only at the thin spatial boundary: coordinates, offsets,
direction/frame expansion, 3D levels, and 3D rendering metadata.

## 3D Authoring Notes

`levels3` reserves `.` as the empty cell character. Do not require a legend entry
for it, and do not map `.` to real objects such as floor tiles.

Canonical 3D depth inputs are `front` and `back`; compatibility aliases may
exist, but new authoring and tests should prefer canonical names.

3D render/camera options are model-owned renderer metadata, not puzzle state,
solver key state, or win-condition state. Camera view-state updates emitted by
rules are presentation emissions.

3D scene-level key rows use the shared scene shortcut contract:

```txt
<key...> -> <input-or-scene-command>
```

Do not reintroduce `=` rows for scene shortcuts.

## Shared Scene Direction

This crate should continue aligning scene parsing with the shared scene crate.
Typed component payloads can remain distinct, but the scene body parser should
not grow 3D-only structural dispatch lists when a shared handler can own the
common loop.
