# Agent Notes

This folder owns the desktop shell and host filesystem boundary.

## Desktop Boundaries

Desktop mode may directly edit files only after the user explicitly opens a
project folder or project entry, or after the app restores workspaces from the
recorded loaded-workspaces list. Startup may restore the workspace set that was
loaded in the previous desktop session, but must not auto-load sample folders,
the repository, the user's home directory, or the whole recent-workspace history
by convention or default.

When loaded-workspace restoration fails, report the failure explicitly. If no
workspace restores successfully, leave the app in a no-project state. Do not
silently substitute a different workspace, sample, repository path, or generated
placeholder.

File reads and writes should be owned by Rust-side host commands bounded by the
opened project root. Do not expose broad filesystem access to JavaScript unless a
concrete feature requires it.

The shell should use the shared HTML editor and editor service behavior for
source loading, preview compilation, highlighting, export, and saving semantics.
Platform divergence belongs at the host adapter/file-access boundary.

Current shell behavior opens the shared HTML editor from the editor crate's
static assets, uses Rust-side native dialogs for project selection, scopes save
operations to the opened workspace root, and preserves app-side unsaved edits
when external file changes conflict. Browser preview/highlight behavior belongs
to the editor runtime assets that Tauri serves through `frontendDist`.

Before changing the desktop/editor asset boundary, run:

```bash
node tools/check_desktop_asset_boundary.mjs
```

Before shipping a desktop asset boundary change, run the same guard in enforcing
mode:

```bash
node tools/check_desktop_asset_boundary.mjs --enforce
```

Do not treat a prose note as sufficient safety for this refactor. The executable
guard must fail if `src-tauri` enables editor asset/export features or if
desktop-owned Rust code embeds web assets through `include_str!` or
`include_bytes!`.

Build command:

```bash
cargo tauri build --bundles app
```

Signing/notarization and installer-style release packaging are still separate
release tasks.
