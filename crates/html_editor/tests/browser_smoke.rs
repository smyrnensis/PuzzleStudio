use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

static BROWSER_SMOKE_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn editor_browser_smoke_flows() {
    run_editor_browser_smoke(&[]);
}

#[test]
fn initial_preview_starts_without_waiting_for_source_analysis() {
    run_editor_browser_smoke(&["--initial-preview-only"]);
}

#[test]
fn solver_observations_render_through_bevy() {
    run_editor_browser_smoke(&["--solver-bevy-only"]);
}

#[test]
fn retired_runtime_generation_cannot_mutate_current_controller() {
    run_editor_browser_smoke(&["--stale-runtime-generation-only"]);
}

#[test]
fn source_click_discards_stale_selection_anchor() {
    run_editor_browser_smoke(&["--source-selection-only"]);
}

#[test]
fn visual_selection_waits_for_current_revision_entries() {
    run_editor_browser_smoke(&["--visual-selection-revision-only"]);
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
fn preview_iframe_uses_its_final_raster_geometry() {
    run_editor_browser_smoke(&["--preview-final-raster-only"]);
}

#[test]
fn level_selection_waits_for_current_revision_entries() {
    run_editor_browser_smoke(&["--level-selection-revision-only"]);
}

fn run_editor_browser_smoke(extra_args: &[&str]) {
    let _browser_smoke_guard = BROWSER_SMOKE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
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
