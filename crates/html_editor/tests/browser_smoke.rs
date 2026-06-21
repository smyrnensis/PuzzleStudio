use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn editor_browser_smoke_flows() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("html-editor crate lives under crates/")
        .to_path_buf();
    let script = repo_root.join("tools/editor_browser_smoke.mjs");
    let editor_bin = std::env::var("CARGO_BIN_EXE_html-editor")
        .map(PathBuf::from)
        .expect("cargo should expose the html-editor binary path to integration tests");

    let status = Command::new("node")
        .arg(script)
        .arg("--editor-bin")
        .arg(editor_bin)
        .current_dir(&repo_root)
        .status()
        .expect("run editor browser smoke test");

    assert!(status.success(), "editor browser smoke test failed");
}
