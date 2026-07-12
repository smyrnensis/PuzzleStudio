# Agent Notes

This crate owns typed runtime/export contracts.

## Boundaries

- Own source-free runtime contract schemas shared by exporters and runtime
  consumers.
- Do not parse `.puzzle` source, run sessions, render, access host APIs, or
  assemble HTML.
- Prefer typed Rust structures plus serde over hand-written JSON writer/parser
  pairs.
- Reject missing or unsupported contract fields visibly. Do not read visual
  fixture fields as a fallback for runtime semantics.

Contract structures may reference deterministic model types from `grid3d` and
shared kernel types. Adapters and temporary facades may produce or consume these
contracts, but they must not define an alternate semantic schema.

Scene presentation transport must preserve the shared text role and layout
contract. Serialize one text component with an explicit role, and serialize
`fit` / weighted `fill`, aspect ratio, alignment, and distribution directly;
do not recreate title/subtitle-specific runtime variants in exporters.
