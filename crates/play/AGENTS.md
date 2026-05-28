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
