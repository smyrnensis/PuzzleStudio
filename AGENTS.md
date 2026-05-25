# Agent Notes

This file gives working rules for agents changing this repository.
Read `DESIGN_PRINCIPLES.md` for the project philosophy and `AGENT_HANDOFF.md`
for the current implementation map.

## Read First And Stay Honest

Before changing this repository, read this file, `DESIGN_PRINCIPLES.md`, and
`AGENT_HANDOFF.md`. Do not claim to have read or verified something unless you
actually did.

Be explicit about capability and uncertainty:

- Do not say a feature works, a test passed, a file was updated, or a behavior
  is supported unless you have checked it.
- Do not promise to do something that the available tools, permissions, time, or
  repository state do not allow. State the blocker and the best safe fallback.
- If a change relies on an assumption, name the assumption before implementing
  it or record it in the handoff when appropriate.
- If user intent is ambiguous, confirm the intended behavior with the user or
  propose the smallest concrete interpretation before editing broad surfaces.

Treat implementation as alignment work, not just code production. Restate the
behavior you are about to make true when the request could be interpreted in
multiple ways, especially for syntax, lifecycle behavior, UI defaults, and
cross-crate boundaries.

## Diagnose Briefly, Then Act

Do not patch only the visible symptom. Before adding a prohibition, syntax
case, runtime default, or UI shortcut, do a short cause check:

- Treat the symptom as evidence. Ask what missing distinction, ownership
  boundary, validation gap, or feedback loop let the bad state look acceptable.
- Challenge one premise before broadening the system. Is the issue really
  syntax, runtime behavior, component ownership, lifecycle, documentation, or
  tests?
- State the smallest principle that would change the current case and close
  sibling cases. If it does not force a concrete implementation, refusal,
  question, test, or documentation update, it is too vague.
- Deduce one or two concrete consequences, then return to implementation. Do
  not keep abstracting once the next scoped action is clear.

The goal is better aim, not slower motion. Use abstraction only until it
changes what you will do next.

## Boundary Discipline

Many bugs in this project come from putting a useful default in the wrong
scope. Before adding syntax, runtime behavior, or UI convenience, identify the
owner of the behavior.

- Generic constructs must stay generic. `for level in levels` is a data loop,
  not a level-select menu. It must not gain cursor movement, confirm behavior,
  or screen-level shortcuts merely because its source is `levels`.
- Component-specific behavior belongs to the component. A `level_menu` may own
  selected-level state, next/previous movement, confirm, and its default start
  behavior. Those defaults should not leak into `screen`, `for`, or unrelated
  action handling.
- Screen behavior belongs to the screen only when it is truly screen-wide:
  navigation, entering/leaving screens, modal flow, and explicit transitions.
- Engine lifecycle behavior belongs to the puzzle/game lifecycle. For example,
  `on_level_start` should initialize a raw level into its starting state; it
  should not be modeled as a fake player input.

When a feature feels convenient, ask: "Would this behavior still be correct if
the same syntax appeared in a different component or screen?" If not, the
behavior is scoped too broadly.

## Defaults

Defaults are welcome when they reduce authoring noise, but defaults must be
owned by the smallest construct that can explain them.

Good defaults:

- `level_menu` can work with no configuration because it is explicitly a
  levels-specific widget.
- `on_level_start` can run once per level because the event name states the
  lifecycle point.
- Cardinal directions can be inferred because `up` / `down` / `left` / `right`
  are standard semantic inputs with built-in direction mappings.

Bad defaults:

- Any `for level in levels` loop automatically becoming navigable.
- `menu_up`, `menu_down`, or `confirm` being globally meaningful because a
  screen happens to render levels.
- A setup rule running through puzzle `rules` with a sentinel input such as
  `InputId(0)`.

## Reserved Words And Surface Syntax

Keep reserved words scarce. Prefer author-chosen action names and explicit
payload bindings over special words such as `selected` or `start_level`.

For example, prefer a component emitting an ordinary action with a payload:

```txt
level_menu {
action choose_level
}

transitions {
choose_level:level -> goto playing with level = level
}
```

over a menu-specific command language:

```txt
on confirm start_level selected
```

Use `on_*` only for scoped lifecycle handlers. The scope must determine what
the event means:

- `puzzle { on_level_start { ... } }` is a puzzle lifecycle event.
- `level_menu { on_choose -> ... }`, if added, is a component event.

Do not make `on` a grab bag for unrelated command shortcuts.

## Layer Separation

Preserve the major package boundaries:

- `puzzle-core`: deterministic state, rules, patches, transitions. No file IO,
  parser concerns, rendering, or game-specific UI behavior.
- `puzzle-lang`: `.puzzle` parsing, validation, authoring syntax, and lowering.
- `puzzle-play`: session mechanics such as undo, restart, level advance, screen
  flow, and component behavior.
- `html-play`, `ascii-play`, editors: adapters and presentation.

If a behavior is duplicated in Rust runtime and standalone JavaScript, update
both or explicitly document why one side is intentionally different.
