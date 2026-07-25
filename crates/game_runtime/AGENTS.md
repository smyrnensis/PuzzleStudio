# Agent Notes

This crate owns the typed projection from game/session state into the complete
runtime presentation snapshot.

## Responsibilities

- adapt `puzzle-play` sessions into `RuntimeSessionSnapshot`
- expose runtime bridges used by WASM game exports
- expose typed snapshot and dispatch paths for native backends

## Boundaries

Do not add browser IO, DOM, localStorage, HTML export assembly, editor behavior,
or solver behavior here. JSON wire naming and encoding belong to
`puzzle-presentation-json`; native renderers consume the typed snapshot without
passing through JSON.

The default feature set is the exported player contract. Editor-only request
routes, transition traces, and debug serialization belong behind the explicit
`editor-debug` feature. Player WASM targets must depend on this crate without
that feature; editor hosts may enable it.
