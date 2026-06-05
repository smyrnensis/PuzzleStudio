# Agent Notes

This folder owns the desktop shell and host filesystem boundary.

## Desktop Boundaries

Desktop mode may directly edit files only after the user explicitly opens a
project folder or project entry, or after the app restores a workspace from the
recorded recent-workspaces list. Startup may restore the most recent recorded
workspace, but must not auto-load sample folders, the repository, or the user's
home directory by convention or default.

When recent-workspace restoration fails, report the failure explicitly and leave
the app in a no-project state. Do not silently substitute a different workspace,
sample, repository path, or generated placeholder.

File reads and writes should be owned by Rust-side host commands bounded by the
opened project root. Do not expose broad filesystem access to JavaScript unless a
concrete feature requires it.

The shell should use the shared HTML editor and editor service behavior for
source loading, preview compilation, highlighting, export, and saving semantics.
Platform divergence belongs at the host adapter/file-access boundary.

Current shell behavior opens the shared HTML editor from the editor crate's
static assets, uses Rust-side native dialogs for project selection, scopes save
and preview operations to the opened workspace root, and preserves app-side
unsaved edits when external file changes conflict.

Build command:

```bash
cargo tauri build --bundles app
```

Signing/notarization and installer-style release packaging are still separate
release tasks.
