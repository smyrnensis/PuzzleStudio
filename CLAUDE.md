# Claude Agent Notes

Before changing this repository, read `AGENTS.md`, `DESIGN_PRINCIPLES.md`, and
`AGENT_HANDOFF.md`. Then read the closest owner-specific `AGENTS.md` for the
files you will touch.

## No Fallback Paths

This rule applies to every task in this repository.

Do not add fallback behavior that hides problems by silently turning an invalid,
stale, unsupported, miswired, or unavailable path into a different execution
path. If a required capability, generated artifact, command, host API, feature,
version, or backend is unavailable, fail visibly with a specific error.

If you encounter an existing fallback path during any work, report it
explicitly. Do not rely on it or extend it unless the user explicitly requests a
temporary migration bridge, and the code names the migration boundary, the
failure mode it preserves, and the condition for deleting it.
