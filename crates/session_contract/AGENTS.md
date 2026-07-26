# Agent Notes

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
This crate owns the typed player snapshot shared by presentation backends and
the separate development snapshot used by editor/debug tooling.
=======
This crate owns the complete typed runtime snapshot shared by presentation
backends.
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544

## Boundaries

- The snapshot must not contain `serde_json::Value`, JSON maps, source text,
  browser objects, GPU resources, or renderer-specific handles.
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
- Player backends consume `RuntimeSessionSnapshot`. It contains only resolved
  presentation, persistence, and input-routing state needed to run the player.
- Authored names, editable projections, solver state, and other inspection data
  belong to `RuntimeDevelopmentSessionSnapshot`.
- JavaScript development tooling converts the development snapshot through
  `presentation_json`.
=======
- A JavaScript backend converts this snapshot through `presentation_json`.
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
- A native backend such as Bevy consumes this snapshot directly.
- Keep game/session projection outside serializers. JSON conversion may rename
  fields and encode typed unions, but may not derive missing semantics.

## Tests

```bash
cargo test -p puzzle-session-contract
```
