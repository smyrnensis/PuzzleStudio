# Agent Notes

This crate owns the 3D model parser/runtime experiment.

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
