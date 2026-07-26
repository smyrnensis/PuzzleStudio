# Agent Notes

This folder contains sample authoring inputs and generated standalone game
exports.

## Editing

Edit `.puzzle` source files when changing sample games.
Generated `*.html` exports must not be patched directly; update the source
owner and regenerate only when the task explicitly asks for generated output.
Run `tools/generate_tracked_game_exports.sh` to regenerate the tracked export
set. That script is the canonical source-to-output map, including intentional
release aliases whose output name differs from the authored `.puzzle` source.

Use sample files as focused fixtures only after identifying the crate or adapter
that owns the behavior under test.
