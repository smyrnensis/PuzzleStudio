# Agent Notes

This file gives repository-wide rules for agents changing this project.
Read `DESIGN_PRINCIPLES.md` for the project philosophy and `AGENT_HANDOFF.md`
for the general implementation map before making changes.

For area-specific rules, read the nearest `AGENTS.md` in the directory you are
about to change. Root docs intentionally stay general; crate-, adapter-, sample-,
and generated-artifact-specific guidance belongs beside the owner folder.

## Read First And Stay Honest

Before changing this repository, read this file, `DESIGN_PRINCIPLES.md`, and
`AGENT_HANDOFF.md`. Then read the closest owner-specific `AGENTS.md` for the
files you will touch. Do not claim to have read or verified something unless you
actually did.

Be explicit about capability and uncertainty:

- Do not say a feature works, a test passed, a file was updated, or a behavior
  is supported unless you have checked it.
- Do not promise to do something that the available tools, permissions, time, or
  repository state do not allow. State the blocker and the best safe next step.
- If a change relies on an assumption, name the assumption before implementing
  it or record it in the handoff when appropriate.
- If user intent is ambiguous, confirm the intended behavior with the user or
  propose the smallest concrete interpretation before editing broad surfaces.

Treat implementation as alignment work, not just code production. Restate the
behavior you are about to make true when the request could be interpreted in
multiple ways, especially for syntax, lifecycle behavior, UI defaults, and
cross-crate boundaries.

## No Fallback Paths

This is a context-critical rule for every task in this repository, not an
optional style preference.

Do not add fallback behavior. Fallbacks make bugs harder to see by silently
turning an invalid, stale, unsupported, or miswired path into a different
execution path. When a required capability, generated artifact, command, host
API, feature, version, or backend is unavailable, fail visibly with a specific
error instead of trying an older API, alternate backend, generated artifact,
default behavior, or compatibility path.

If an existing fallback path is encountered during any work, report it
explicitly. When it is in the area being changed, treat it as technical debt to
remove or report rather than a pattern to extend. Compatibility paths are
allowed only when the user explicitly requests a temporary migration bridge and
the code names the migration boundary, the failure mode it preserves, and the
condition for deleting it.

Fallback pressure is a signal to stop and identify the missing required path,
not a reason to make the system keep running by another route. Before coding
around a missing parser case, runtime API, generated artifact, host command, or
adapter capability, name which required contract is absent or broken. If that
contract cannot be repaired in scope, report the blocker instead of adding a
graceful degradation, guessed default, legacy route, or silent compatibility
branch.

## Context Budget And Repository Shape

Before reading broad file contents, get a cheap shape of the repository. Prefer
counts and sizes first:

```bash
git status --short
find . \( -path ./.git -o -path ./target -o -path '*/target' \) -prune -o -type f -print | wc -l
du -sh . .git target 2>/dev/null
find . -maxdepth 2 \( -path ./.git -o -path ./target \) -prune -o -type d -print | sort
```

Treat repository size as a context-selection problem, not a signal to read more.
The largest paths are usually generated artifacts, build output, or history.
Do not inspect them unless the task specifically requires the generated result,
binary artifact, or build cache.

Default read order for implementation work:

1. Root `AGENTS.md`, `DESIGN_PRINCIPLES.md`, and `AGENT_HANDOFF.md`.
2. The nearest owner-specific `AGENTS.md`.
3. The smallest owner crate, adapter, or content folder named by the task.
4. Tests, fixtures, or sample `.puzzle` files that directly exercise that owner.
5. Generated exports only after identifying their source owner and reading that
   owner's guidance.

Default skip list for context gathering:

- version-control internals
- build output directories
- generated standalone exports
- generated documentation exports
- generated WebAssembly binaries
- legacy archives, unless the task is about legacy samples or import behavior

When a generated file looks relevant, first find the source that owns it instead
of reading or patching the generated output. Owner-specific generated-artifact
rules live in the corresponding folder `AGENTS.md`.

## Diagnose Briefly, Then Act

Do not patch only the visible symptom. Before adding a prohibition, syntax case,
runtime default, or UI shortcut, do a short cause check:

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

The goal is better aim, not slower motion. Use abstraction only until it changes
what you will do next.

A non-local correction is not a larger patch. It is a patch that changes the
decision structure that let the visible symptom look acceptable. Before
implementing, be able to state:

- The mechanism that produced the bad state, not only the bad state itself.
- The owner of the missing or broken contract: syntax, lowering, runtime,
  session flow, component behavior, adapter, editor, documentation, or tests.
- One sibling case that the same cause would also affect.
- One boundary case where the proposed principle should not apply.
- The concrete action this diagnosis forces: edit, delete, refuse, ask,
  inspect, test, or document.

If the cause cannot yet be stated at that level, do not continue into
implementation as a way to discover it. Gather targeted evidence, read the
owner-specific guidance, construct a minimal example, or ask the user one narrow
question. Broad implementation is not a substitute for a missing diagnosis.

Do not preserve an existing implementation, plan, section, generated artifact,
or wording merely because effort has already gone into it. Existing work has no
authority by itself. Keep it only if it still belongs to the identified owner
and satisfies the corrected contract. When the diagnosis shows the current shape
is at the wrong layer, delete, replace, or move it instead of polishing it in
place.

## Boundary Discipline

Many bugs in this project come from putting a useful default in the wrong scope.
Before adding syntax, runtime behavior, or UI convenience, identify the owner of
the behavior.

- Generic constructs must stay generic. They must not gain behavior merely
  because one current use happens to be a menu, level list, editor affordance, or
  adapter shortcut.
- Component-specific behavior belongs to the component. Defaults should not leak
  into screens, generic loops, or unrelated action handling.
- Screen behavior belongs to the screen only when it is truly screen-wide:
  navigation, entering/leaving screens, modal flow, and explicit transitions.
- Engine lifecycle behavior belongs to the puzzle/game lifecycle. Lifecycle
  setup should not be modeled as a fake player input.

When a feature feels convenient, ask: "Would this behavior still be correct if
the same syntax appeared in a different component or screen?" If not, the
behavior is scoped too broadly.

Boundary problems must be named before they are fixed. State which actor or
layer has authority to decide the behavior, which layer is only consuming a
contract, and where the behavior should fail if that contract is absent. Do not
let an adapter, sample, current UI, or generated output become the de facto
owner of semantics because it is the easiest place to patch.

## Defaults

Defaults are welcome when they reduce authoring noise, but defaults must be
owned by the smallest construct that can explain them.

Good defaults:

- A domain-specific component can work with no configuration when its purpose
  explicitly owns the default behavior.
- A scoped lifecycle handler can run at the lifecycle point named by the event.
- Standard semantic inputs can have built-in meanings when those meanings are
  stable across the project.

Bad defaults:

- A generic loop or container automatically becoming an interactive widget.
- A semantic action becoming globally meaningful because one screen happens to
  render content that can use it.
- Runtime setup running through gameplay rules via a sentinel or fake input.

## Reserved Words And Surface Syntax

Keep reserved words scarce. Prefer author-chosen action names and explicit
payload bindings over special words tied to one widget or adapter.

Use `on_*` only for scoped lifecycle handlers. The scope must determine what the
event means. Do not make `on` a grab bag for unrelated command shortcuts.

## PuzzleStudio Syntax Shape

PuzzleStudio syntax should optimize for stable author intent rather than parser
convenience. When adding or changing syntax, preserve these surface principles:

- When exactly one object of a requested type exists in the current scope, the
  type name may identify that object. If the surrounding syntax already requires
  that type, the reference may be omitted entirely. This shorthand is valid only
  when uniqueness is checked and ambiguity fails visibly.
- Nested blocks form a tree of owner-scoped declarations or content. A structure
  such as `a { b { } c { } }` expresses containment and ownership first; source
  order is meaningful only when the owner syntax defines an ordered sequence,
  such as levels, layout children, or rule statements.
- Compilation must not use sibling source order as an implicit declaration
  availability rule. References should resolve against the complete owning
  scope after its declarations have been collected, while ordered constructs
  still preserve their authored item order for runtime or presentation meaning.
- A block body may contain child nodes and owner-scoped special forms, such as
  replacement rules or `ascii` content. Special forms must be interpreted by the
  owner of the containing block, not by a generic tree walker guessing from
  appearance.

## Generated Artifacts

Generated artifacts must not be edited directly. Patch the source owner and
regenerate through the normal command only when regeneration is explicitly
intended. Before regenerating a tracked generated artifact, check whether the
output path is dirty and avoid overwriting unrelated user work without clear
intent.

Generated-artifact details, including which paths are generated and which source
folders own them, belong in the nearest folder-specific `AGENTS.md`.

## Layer Separation

Preserve the major package boundaries:

- deterministic state and transition logic own no file IO, parser concerns,
  rendering, or game-specific UI behavior.
- language processing owns `.puzzle` parsing, validation, authoring syntax, and
  lowering.
- play/session logic owns undo, restart, level advance, screen flow, and
  component behavior.
- adapters and editors own presentation, host IO, browser/terminal behavior, and
  export surfaces.

If a behavior is duplicated across runtime/adapters, update each owned copy or
explicitly document why one side is intentionally different.
