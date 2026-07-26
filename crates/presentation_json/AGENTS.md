# Agent Notes

This crate owns JSON transport for the explicit development snapshot.

## Boundaries

- Convert `RuntimeDevelopmentSessionSnapshot` into the established JavaScript
  development wire shape.
- Do not make the public player snapshot serializable here. Native and browser
  players consume its typed Rust contract directly.
- Do not derive game, scene, visual, ordering, lifecycle, or animation meaning.
- Do not contain browser APIs, renderer code, source parsing, or session logic.
- Bevy and other native backends consume `RuntimeSessionSnapshot` directly and
  do not depend on this crate.

## Tests

```bash
cargo test -p puzzle-presentation-json
```
