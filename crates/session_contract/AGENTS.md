# Agent Notes

This crate owns the complete typed runtime snapshot shared by presentation
backends.

## Boundaries

- The snapshot must not contain `serde_json::Value`, JSON maps, source text,
  browser objects, GPU resources, or renderer-specific handles.
- A JavaScript backend converts this snapshot through `presentation_json`.
- A native backend such as Bevy consumes this snapshot directly.
- Keep game/session projection outside serializers. JSON conversion may rename
  fields and encode typed unions, but may not derive missing semantics.

## Tests

```bash
cargo test -p puzzle-session-contract
```
