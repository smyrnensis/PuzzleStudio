use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=static/editor.html");
    println!("cargo:rerun-if-changed=static/editor_boot.js");
    println!("cargo:rerun-if-changed=static/editor_theme_imports.js");
    println!("cargo:rerun-if-changed=static/editor_dom.js");
    println!("cargo:rerun-if-changed=static/editor_workspace.js");
    println!("cargo:rerun-if-changed=static/editor_source.js");
    println!("cargo:rerun-if-changed=static/editor_level3d.js");
    println!("cargo:rerun-if-changed=static/editor_workbench.js");
    println!("cargo:rerun-if-changed=static/editor.js");
    println!("cargo:rerun-if-changed=static/editor_sprite.js");
    println!("cargo:rerun-if-changed=../html_play/static/puzzle3_visual_core.js");
    println!("cargo:rerun-if-changed=static/editor_sprite3d.js");
    println!("cargo:rerun-if-changed=static/editor_sounds.js");
    println!("cargo:rerun-if-changed=static/editor.css");
    println!("cargo:rerun-if-changed=static/wasm/puzzle_wasm.js");
    println!("cargo:rerun-if-changed=static/wasm/puzzle_wasm_bg.wasm");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let editor_js =
        fs::read_to_string(manifest_dir.join("static/editor.js")).expect("read static/editor.js");
    let editor_css =
        fs::read_to_string(manifest_dir.join("static/editor.css")).expect("read static/editor.css");

    let mut failures = Vec::new();
    if editor_js.contains("ResizeObserver(syncPreviewViewportScale)") {
        failures.push("preview ResizeObserver reintroduces iframe sizing feedback");
    }
    if editor_js.contains("updatePreviewFrameLayout(event.data.layout)")
        || editor_js.contains("updatePreviewFrameLayout(event.data)")
    {
        failures.push("PuzzleStudioPreviewLayout must not drive editor preview sizing");
    }
    if editor_js.contains("puzzle-studio-editor-preview-layout-script") {
        failures.push("preview layout injection script must stay removed");
    }
    if !editor_js.contains("function fitPreviewViewportSize(")
        || !editor_js.contains("previewAspectForScene(")
    {
        failures.push("editor preview viewport must preserve the compiled game scene aspect");
    }
    if editor_js.contains("const viewportWidth = availableWidth || previewVirtualWidth") {
        failures.push("editor preview iframe must not inherit the pane aspect");
    }
    if !editor_js.contains("previousFrame.removeAttribute(\"id\")") {
        failures
            .push("preview iframe swap must show the loaded frame before removing the old frame");
    }
    if !editor_css.contains("transform: scale(var(--preview-scale))") {
        failures.push("editor preview must fit the fixed iframe viewport as a whole");
    }
    if editor_js.contains(
        "previewFrameWrap.style.setProperty(\"--preview-virtual-width\", `${viewportWidth}px`)",
    ) {
        failures.push("editor preview virtual size must not inherit the pane size");
    }
    if editor_css.contains("transform: scale(var(--board-scale))") {
        failures.push("editor level/solver boards must not use fractional CSS scaling");
    }

    if !failures.is_empty() {
        panic!(
            "html-editor preview flicker regression:\n- {}",
            failures.join("\n- ")
        );
    }
}
