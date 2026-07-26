# Agent Notes

This crate owns loaded-game session mechanics.

## Responsibilities

- session state
- undo / redo
- restart and level advance
- progress save data for cleared levels, current level, and persistent puzzle vars
- post-turn lifecycle behavior
- display helpers shared by adapters

Host storage belongs to adapters. Core transition logic remains sound-free,
timer-free, browser-free, and terminal-free.

Turn completion is owned here and in equivalent standalone adapter runtime code:
after puzzle rules run, evaluate win conditions on the post-rules/pre-navigation
snapshot, run level-clear lifecycle before navigation when clear, and resolve
queued navigation commands through the owning model window/runtime.

Use `new_headless_before_level_start` when a caller owns an explicit pre-start
state and must materialize one selected level lifecycle. The caller must then
invoke `start_level_from_state(..., true)` exactly once; the ordinary
`new_headless` constructor continues to start the routed initial world.

Use `replace_active_state_snapshot` for an intermediate-state hypothesis. It
replaces only the active model snapshot and clears input history; it preserves
the active attempt's authored initial state, checkpoint, lifecycle-started
status, persistent session values, and scene context.
