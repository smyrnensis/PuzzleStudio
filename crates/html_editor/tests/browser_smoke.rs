use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn editor_browser_smoke_flows() {
    run_editor_browser_smoke(&[]);
}

#[test]
fn initial_preview_starts_without_waiting_for_source_analysis() {
    run_editor_browser_smoke(&["--initial-preview-only"]);
}

#[test]
fn source_click_discards_stale_selection_anchor() {
    run_editor_browser_smoke(&["--source-selection-only"]);
}

#[test]
fn visual_palette_mouse_click_preserves_pane_scroll() {
    run_editor_browser_smoke(&["--visual-palette-only"]);
}

#[test]
fn visual_bucket_fill_respects_active_clip() {
    run_editor_browser_smoke(&["--visual-clip-fill-only"]);
}

#[test]
fn compact_index_controls_stay_vertically_centered() {
    run_editor_browser_smoke(&["--index-control-layout-only"]);
}

#[test]
fn level_selection_waits_for_current_revision_entries() {
    run_editor_browser_smoke(&["--level-selection-revision-only"]);
}

fn run_editor_browser_smoke(extra_args: &[&str]) {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("html-editor crate lives under crates/")
        .to_path_buf();
    let script = repo_root.join("tools/editor_browser_smoke.mjs");
    let editor_bin = std::env::var("CARGO_BIN_EXE_html-editor")
        .map(PathBuf::from)
        .expect("cargo should expose the html-editor binary path to integration tests");

    let mut command = Command::new("node");
    command
        .arg(script)
        .arg("--editor-bin")
        .arg(editor_bin)
        .args(extra_args)
        .current_dir(&repo_root);
    let status = command.status().expect("run editor browser smoke test");

    assert!(status.success(), "editor browser smoke test failed");
}
