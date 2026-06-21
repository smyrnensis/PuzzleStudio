# Agent Notes

This crate owns deterministic 3D grid mechanics.

## Boundaries

Keep this crate focused on 3D spatial primitives, level state construction,
state mutation, patches, win checks, and transition execution.

Do not add `.puzzle3` parser concerns, authoring syntax, scene parsing,
session flow, rendering, visual fixture export, browser/terminal behavior,
sound, timers, or host IO here. Those belong to language, play/session, scene,
or adapter owners.

3D-specific branching belongs here only when it is spatial: coordinates,
offsets, directions, frame expansion used by deterministic matching, level
dimensions, and layer-constrained cell state.
