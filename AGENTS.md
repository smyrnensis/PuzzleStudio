# Agent Notes

This file gives repository-wide rules for agents changing this project. Read it
before making changes, then read the nearest `AGENTS.md` for every area you will
touch. Area-specific guidance belongs beside its owner rather than in root docs.

Consult `DESIGN_PRINCIPLES.md` when work changes product semantics, authoring
concepts, or a boundary shared by multiple owners. Consult `AGENT_HANDOFF.md`
when you need the repository map or do not yet know which owner to inspect.
Neither document is a mandatory pre-read for an unrelated local change.

## Read First And Stay Honest

Before changing this repository, read this file and the closest owner-specific
`AGENTS.md` for the files you will touch. Read the additional documents routed
above when the task crosses their scope. Do not claim to have read or verified
something unless you actually did.

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

## Reproduce Before Fixing

Do not invent a problem in order to solve it. When a task is prompted by a bug,
failed behavior, regression, visual defect, or mismatch between intent and
observed output, first reproduce or directly observe the problem before
designing the fix.

Reproduction can be a failing test, a command output, a trace, a minimal
`.puzzle` case, a local UI run, or another concrete observation that shows the
bad state. If the issue cannot be reproduced with the available tools, say that
explicitly and ask for the missing artifact or narrow the work to the evidence
that is actually present. Do not proceed by filling the gap with a guessed
failure mode.

For visual issues, a user-provided screenshot counts as direct observation of
the visual state shown in that screenshot. Treat it as evidence, not as
permission to infer unseen states. Do not claim a visual problem is present,
fixed, or verified from code intuition alone. If the required visual state is
not visible in the screenshot, reproduce it with a local render, screenshot, or
browser check before acting on that part of the claim.

When the user is asking for speculative design, a new feature, or an instruction
change rather than a reported failure, name that as design work and do not
pretend there is a reproduced bug. The same honesty rule applies: separate
observed evidence from assumptions, proposed behavior, and implementation
judgment.

## No Fallback Paths

This is a context-critical rule for engineering decisions in every task in this
repository, not an optional style preference. Its purpose is to keep broken
technical contracts diagnosable. It does not make every internal failure a
valid reason to fail a user operation.

Do not add fallback behavior. Fallbacks make bugs harder to see by silently
turning an invalid, stale, unsupported, or miswired path into a different
execution path. When a required capability, generated artifact, command, host
API, feature, version, or backend is unavailable, fail visibly at the boundary
that owns that requirement, with a specific diagnostic for the developer or
operator who can repair it, instead of trying an older API, alternate backend,
generated artifact, default behavior, or compatibility path.

Contain the failure to the operations whose own contracts require the missing
capability. Before making an error block compilation, preview, export, saving,
gameplay, or another user outcome, identify the declared dependency that makes
the failed contract necessary for that outcome. Presentation, editing
assistance, observability, and diagnostic consumers must not become acceptance
gates for otherwise valid language or runtime operations. If an auxiliary
consumer fails while the requested operation's contracts still hold, report
that consumer's failure through its own channel and run the requested operation
through its normal owner path. Continuing an independently valid operation is
failure containment, not a fallback.

User-facing recovery is part of the product contract when the owning feature
defines it: preserving work, explaining the failed operation, and offering
retry, correction, or cancellation are not technical fallback paths. Such
behavior must not suppress the underlying diagnostic or substitute a different
implementation for the failed operation.

If an existing fallback path is encountered during any work, report it
explicitly. When it is in the area being changed, treat it as technical debt to
remove rather than a pattern to extend. Migration support is valid only when
coexistence or conversion is itself part of the final supported product
contract. A request to preserve the current implementation temporarily does not
justify a compatibility path.

Fallback pressure is a signal to stop and identify the missing required path,
not a reason to make the system keep running by another route. Before coding
around a missing parser case, runtime API, generated artifact, host command, or
adapter capability, name which required contract is absent or broken, which
owner must repair it, and which dependent operations it actually invalidates.
If that contract cannot be repaired in scope, report the blocker for those
operations instead of adding a graceful degradation, guessed default, legacy
route, or silent compatibility branch.

## Implement The Final Structure

Technical debt is not an implementation strategy. A locally cheaper detour
usually defers the same design decision until more callers, states, and data
depend on the wrong boundary, multiplying the eventual repair cost. Optimize
for the final ownership model and dependency structure, not for the smallest
immediate diff or preservation of the current shape.

Before implementing, identify the owner, contract, data flow, and lifecycle that
the completed system requires, then change those owners directly. Do not add a
temporary wrapper, parallel path, duplicated contract, transitional adapter,
old/new branch, or cleanup TODO whose only purpose is to avoid making the
structural change now. Existing behavior and compatibility have authority only
when they are explicit requirements of the final product, not merely because
they already exist or make the current patch easier.

Incremental work is acceptable only when every landed increment belongs to the
final structure and needs no later removal, bypass, or ownership transfer. When
the final structure cannot be completed within scope, expose the missing
prerequisite and stop at that boundary instead of landing a detour. If the area
being changed already contains a structural workaround, remove it as part of
the change; do not build the new behavior on top of it or report the task as
complete while required cleanup remains.

Report every existing structural debt encountered during the work, whether or
not it can be removed within the current scope. The report must identify its
location, the detour or misplaced responsibility, the final owner or contract
it should use, and any prerequisite that prevents removal. Discovery creates a
reporting obligation; do not silently leave a known workaround undocumented.

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

## Concurrent Sessions And Worktree Ownership

Prefer separate git worktrees for parallel agent sessions. A shared dirty
worktree is not a safe coordination mechanism: the filesystem does not record
which session owns a change, and repeatedly repairing drift in the same file can
silently overwrite another session's work.

Before editing any file that is dirty, staged, untracked, or likely to be touched
by another session, record its current content hash after reading it. Immediately
before applying a patch, check the hash again. If the hash changed, treat that as
evidence of concurrent ownership, not as an automatic reason to stop. Re-read the
latest file and its diff, identify whether the concurrent change overlaps the
lines or behavior contract you intend to change, and reconstruct your smallest
patch against the latest content while preserving all unrelated work.

Do not revert, overwrite, normalize, or otherwise repair concurrent changes merely
to restore the state you first inspected. Do not alter authored inputs or generator
implementation owned by another session to make a test pass, reduce the diff,
satisfy formatting, or force a particular generated result. Run verification
against the combined current authored state, and regenerate derived outputs from
that state when required. Report failures that belong to concurrent work without
trying to absorb or conceal them.
Ask the user only when the concurrent change overlaps the same lines or semantic
contract, makes ownership impossible to determine, or prevents a safe minimal
patch. Treat repeated patch context mismatches, content that reverts between
commands, or a change that disappears after a test/build as evidence of another
owner or generator and narrow the work accordingly.

Do not edit an `AM`, staged, or otherwise externally owned file unless the user
explicitly assigns that file to the current session or you can identify the
existing changes as your own. If shared-worktree editing is unavoidable, use the
hash-and-overlap check above as the minimum safety gate; separate worktrees remain
the preferred structure. Apply this ownership check to authored inputs, not to a
known generated artifact as though it were an independently edited source file.
Generated outputs follow the regeneration rules below.

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

## DRY Check Gate

Before adding or copying implementation, documentation, tests, configuration,
or UI behavior, perform a targeted repository search for the same responsibility
and nearby variants. Identify the existing owner before writing a second version.

- If the behavior is already owned, extend or parameterize that owner instead of
  duplicating it at the new call site.
- If two copies must remain because their contracts genuinely differ, state the
  distinction, keep their ownership separate, and add a focused test or
  documentation note that makes the divergence intentional.
- Do not extract a shared abstraction merely because text is similar. Extract
  only when the behavior, lifecycle, and owner contract are actually shared;
  otherwise the abstraction is a new cross-boundary coupling.
- Before finalizing, inspect the changed area for copied branches, parallel
  constants, duplicated validation, and adapter-specific reimplementations of
  language, runtime, or session semantics. Consolidate them or report the
  intentional boundary.

This is a gate, not a suggestion: a change that adds a second implementation
without checking the first is incomplete. Prefer a visible request for the
missing shared contract over copying behavior while its proper owner is unknown.

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
regenerate through the normal command whenever the source change or requested
verification requires the generated result to be current. A dirty generated
output is derivation state, not evidence of an independent manual edit: assume
known generated artifacts are changed through their generator, and do not stop
or ask for permission solely because an output path is dirty.

Before regenerating, identify the source owner, generator, and expected output
set. Apply dirty-file and concurrent-ownership checks to the authored inputs,
then run the generator against the combined current source state. Inspect the
resulting diff and stop only when the command would modify authored files,
consume an unresolved source conflict, or write outside its declared generated
output set.

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

JavaScript adapters and editors must not recognize or interpret PuzzleStudio
source syntax. They may consume only explicit, typed contracts produced by the
language-processing owner. In particular, JavaScript must not tokenize source,
identify declarations or blocks, resolve names, combine legends, levels, or
sprites, or infer language semantics from source text. If an editor needs
information that its current contract does not provide, extend the language
contract and fail visibly until that contract is available; do not add a
JavaScript parser, regex recognizer, heuristic, or fallback source path.

If a behavior is duplicated across runtime/adapters, update each owned copy or
explicitly document why one side is intentionally different.
