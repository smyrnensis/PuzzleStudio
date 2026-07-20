use std::{env, fs, path::PathBuf};

fn main() {
    let embedded_assets = env::var_os("CARGO_FEATURE_EMBEDDED_ASSETS").is_some();
    if !embedded_assets {
        println!("cargo:rerun-if-changed=build.rs");
        if env::var_os("CARGO_FEATURE_SOUND_TOOLS").is_some() {
            println!("cargo:rerun-if-changed=../../tools/music_generator/seeded_sfx.mjs");
            println!("cargo:rerun-if-changed=../../tools/music_generator/seeded_music.mjs");
            println!("cargo:rerun-if-changed=../../tools/music_generator/seeded_music_player.mjs");
            println!("cargo:rerun-if-changed=../../tools/music_generator/seeded_timbre_fields.mjs");
            println!("cargo:rerun-if-changed=../../tools/music_generator/audio_export.mjs");
        }
        return;
    }

    println!("cargo:rerun-if-changed=static/editor.html");
    println!("cargo:rerun-if-changed=docs/editor.md");
    println!("cargo:rerun-if-changed=docs/metadata.md");
    println!("cargo:rerun-if-changed=docs/puzzle-block.md");
    println!("cargo:rerun-if-changed=docs/slots.md");
    println!("cargo:rerun-if-changed=docs/groups.md");
    println!("cargo:rerun-if-changed=docs/tags.md");
    println!("cargo:rerun-if-changed=docs/legend.md");
    println!("cargo:rerun-if-changed=docs/levels.md");
    println!("cargo:rerun-if-changed=docs/level-local-legend.md");
    println!("cargo:rerun-if-changed=docs/messages.md");
    println!("cargo:rerun-if-changed=docs/rewrite-rules.md");
    println!("cargo:rerun-if-changed=docs/input-rules.md");
    println!("cargo:rerun-if-changed=docs/movement.md");
    println!("cargo:rerun-if-changed=docs/guards.md");
    println!("cargo:rerun-if-changed=docs/fix.md");
    println!("cargo:rerun-if-changed=docs/variables.md");
    println!("cargo:rerun-if-changed=docs/mark.md");
    println!("cargo:rerun-if-changed=docs/conditions.md");
    println!("cargo:rerun-if-changed=docs/win-conditions.md");
    println!("cargo:rerun-if-changed=docs/scenes.md");
    println!("cargo:rerun-if-changed=docs/scene-layout.md");
    println!("cargo:rerun-if-changed=docs/semantic-inputs.md");
    println!("cargo:rerun-if-changed=docs/menus.md");
    println!("cargo:rerun-if-changed=docs/lifecycle.md");
    println!("cargo:rerun-if-changed=docs/visuals.md");
    println!("cargo:rerun-if-changed=docs/display.md");
    println!("cargo:rerun-if-changed=docs/theme.md");
    println!("cargo:rerun-if-changed=docs/sounds.md");
    println!("cargo:rerun-if-changed=docs/routines.md");
    println!("cargo:rerun-if-changed=docs/rule-application.md");
    println!("cargo:rerun-if-changed=docs/patterns.md");
    println!("cargo:rerun-if-changed=docs/imports.md");
    println!("cargo:rerun-if-changed=docs/rendering.md");
    println!("cargo:rerun-if-changed=docs/3d.md");
    println!("cargo:rerun-if-changed=docs/assets.md");
    println!("cargo:rerun-if-changed=docs/rule-effects.md");
    println!("cargo:rerun-if-changed=docs/visual-shapes.md");
    println!("cargo:rerun-if-changed=docs/scene-state-effects.md");
    println!("cargo:rerun-if-changed=docs/maps-expansion.md");
    println!("cargo:rerun-if-changed=static/editor_boot.js");
    println!("cargo:rerun-if-changed=static/editor_icons.js");
    println!("cargo:rerun-if-changed=static/editor_codemirror.js");
    println!("cargo:rerun-if-changed=static/editor_runtime.js");
    println!("cargo:rerun-if-changed=static/editor_analysis_worker.js");
    println!("cargo:rerun-if-changed=static/editor_dom.js");
    println!("cargo:rerun-if-changed=static/editor_workspace.js");
    println!("cargo:rerun-if-changed=static/editor_source.js");
    println!("cargo:rerun-if-changed=static/editor_level3d.js");
    println!("cargo:rerun-if-changed=static/editor_workbench.js");
    println!("cargo:rerun-if-changed=static/editor_import_export.js");
    println!("cargo:rerun-if-changed=static/editor.js");
    println!("cargo:rerun-if-changed=static/editor_visual.js");
    println!("cargo:rerun-if-changed=../html_play/static/puzzle3_visual_core.js");
    println!("cargo:rerun-if-changed=static/editor_visual3d.js");
    println!("cargo:rerun-if-changed=static/editor_sounds.js");
    println!("cargo:rerun-if-changed=static/editor.css");
    println!("cargo:rerun-if-changed=static/wasm/puzzle_wasm.js");
    println!("cargo:rerun-if-changed=static/wasm/puzzle_wasm_bg.wasm");
    println!("cargo:rerun-if-changed=../html_play/static/wasm_game/puzzle_wasm_game.js");
    println!("cargo:rerun-if-changed=../html_play/static/wasm_game/puzzle_wasm_game_bg.wasm");
    println!("cargo:rerun-if-changed=../html_play/static/wasm_player/puzzle_wasm_player.js");
    println!("cargo:rerun-if-changed=../html_play/static/wasm_player/puzzle_wasm_player_bg.wasm");
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
