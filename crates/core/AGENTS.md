# Agent Notes

This crate owns deterministic state and transition mechanics.

## Boundaries

Do not add file IO, parser concerns, rendering, terminal/browser behavior, sound,
timers, or game-specific UI defaults here.

State uses dimensions and layer count for fast slot addressing:

```txt
slot_index = ((y * width + x) * layer_count + layer_id)
slots[slot_index] = object_id | EMPTY
```

`EMPTY` is `ObjectId(0)`. A cell is a finite set of visible objects constrained
by at most one object per `(cell, layer)`. If count matters, model it visibly.

`input` is transition context, not canonical state. Rules see it through guards
such as `Guard::InputIs(InputId)`.

Rules build patches and apply them as a unit. Patch application must keep
derived caches such as object counts coherent.

## Current Gaps

`transition_state` still clones state in some hot paths, and trace output is
minimal compared with the intended debugging model.
