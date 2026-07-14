# Agent Notes

This crate owns the AI-facing diagnostic and automation facade.

## Authority Boundary

`puzzlestudio` lets AI agents and automation operate the canonical system and
observe errors and execution results through stable exit codes, diagnostics,
and structured output. It is not the owner of parser, lowering, runtime,
session, editor, or browser behavior.

CLI operations and feedback are separate contracts. Commands select an owned
operation; their results expose diagnostics, state snapshots or diffs, traces,
and other typed observations from that owner. Do not reconstruct those results
from source text, renderer state, or human-formatted output inside the CLI.

Interactive human play, raw key handling, and terminal rendering are not CLI
responsibilities. Browser/editor adapters own interactive presentation.

The default build intentionally keeps `check` independent from adapter crates.
Adapter facade commands such as preview, editor, screenshot, and export
require adapter features or owner-local package commands for development checks.

Use `puzzlestudio ...` when the installed facade itself is the thing being
checked, or after explicitly refreshing it with `cargo install --path
crates/cli`. Do not treat a stale debug binary as authoritative for included
CSS/JS/WASM assets.
