# Agent Notes

This crate owns game-runtime presentation adapters that are not browser hosts.

## Responsibilities

- adapt `puzzle-play` sessions into screen snapshot JSON
- expose runtime bridges used by WASM game exports
- keep legacy screen contracts quarantined from game semantics

## Boundaries

Do not add browser IO, DOM, localStorage, HTML export assembly, editor behavior,
or solver behavior here. Browser hosts consume this crate; they do not define
game meaning here.

The default feature set is the exported player contract. Editor-only request
routes, transition traces, and debug serialization belong behind the explicit
`editor-debug` feature. Player WASM targets must depend on this crate without
that feature; editor hosts may enable it.
