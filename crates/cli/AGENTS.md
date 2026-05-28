# Agent Notes

This crate owns the product and automation facade.

## Authority Boundary

`puzzlestudio` is the facade for stable commands, exit codes, diagnostics, and
JSON output. It is not the owner of parser/runtime/editor/browser behavior.

The default build intentionally keeps `check` independent from adapter crates.
Adapter facade commands such as play, preview, editor, screenshot, and export
require adapter features or owner-local package commands for development checks.

Use `puzzlestudio ...` when the installed facade itself is the thing being
checked, or after explicitly refreshing it with `cargo install --path
crates/cli`. Do not treat a stale debug binary as authoritative for included
CSS/JS/WASM assets.
