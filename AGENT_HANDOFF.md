# Agent Handoff

This is a routing map for agents that do not yet know which part of the
repository owns a change. It is not a status report, command reference, or
second copy of design rules. After locating the owner, read the nearest
`AGENTS.md` and continue from there.

## System Flow

```txt
.puzzle source
  -> language parsing, validation, and lowering
  -> deterministic model and transition logic
  -> play/session and runtime contracts
  -> browser, WASM, CLI, and desktop adapters
```

Presentation and editor hosts consume explicit contracts from these owners.
They do not reinterpret source syntax or deterministic model internals.

## Owner Routing

- Rust package boundaries and the current crate map: `crates/AGENTS.md`
- `.puzzle` syntax, validation, source analysis, and lowering:
  `crates/lang/AGENTS.md`
- Deterministic dimension-generic state and transitions: `crates/core/AGENTS.md`
- Session lifecycle, undo, restart, and level flow: `crates/play/AGENTS.md`
- Shared scene and component contracts: `crates/scene/AGENTS.md`
- Source-free adapter/runtime schemas: `crates/runtime_contract/AGENTS.md`
- Browser player and standalone export: `crates/html_play/AGENTS.md`
- Browser editor, preview, and generated Pages assets:
  `crates/html_editor/AGENTS.md`
- CLI and agent-facing facades: `crates/cli/AGENTS.md` and
  `crates/agent_runtime/AGENTS.md`
- Desktop host and filesystem boundary: `src-tauri/AGENTS.md`
- Samples and generated game exports: `games/AGENTS.md`
- Generated documentation release: `docs/AGENTS.md`
- Generated WebAssembly outputs: `wasm/AGENTS.md`

If a folder has no local `AGENTS.md`, use the closest ancestor guidance. Do not
turn this list into an exhaustive inventory of every crate; the owner-level maps
are authoritative for their subtree.

## Reference Routing

- Long-lived product and design principles: `DESIGN_PRINCIPLES.md`
- Canonical authoring syntax: `AUTHORING_SYNTAX.md`
- Current execution semantics: `CURRENT_SPEC.md`
- Parser/editor source-analysis boundary: `SOURCE_ANALYSIS_CONTRACT.md`
- Development and generation commands: `DEV_COMMANDS.md`

Read only the references whose contract the task can change. In particular,
read `SOURCE_ANALYSIS_CONTRACT.md` before changing `SurfaceDocument`,
`SourceAnalysis`, analysis profiles, source offsets, or language-aware editor
integration.

## Context Traps

This checkout may contain large version-control history, build outputs,
generated exports, documentation releases, WebAssembly binaries, and legacy
archives. Locate the source owner before opening a generated artifact. Patch and
regenerate generated output only when regeneration is explicitly in scope.

Use owner-local tests and commands first. Use repository-wide commands only when
their full blast radius is part of the intended verification.
