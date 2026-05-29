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
