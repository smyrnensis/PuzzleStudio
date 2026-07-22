const EDITOR_SOURCE: &str = include_str!("../static/editor.js");
const EDITOR_VISUAL_SOURCE: &str = include_str!("../static/editor_visual.js");
const EDITOR_VISUAL3D_SOURCE: &str = include_str!("../static/editor_visual3d.js");

fn function_body<'a>(source: &'a str, signature: &str, next_signature: &str) -> &'a str {
    source
        .split_once(signature)
        .and_then(|(_, tail)| tail.split_once(next_signature))
        .map(|(body, _)| body)
        .expect("function body")
}

fn assert_render_then_top_bar_sync(body: &str, render_call: &str) {
    let render = body.find(render_call).expect("visual builder render");
    let sync = body
        .find("syncPreviewModeButtonState();")
        .expect("top bar state sync");
    assert!(
        render < sync,
        "top bar must reflect the rendered source state"
    );
}

#[test]
fn source_visual_hydration_syncs_the_animate_button_for_2d_and_3d() {
    let top_bar_sync = function_body(
        EDITOR_SOURCE,
        "function syncPreviewModeButtonState() {",
        "function setPreviewMode(mode, options = {}) {",
    );
    let visual_loader = function_body(
        EDITOR_VISUAL_SOURCE,
        "function loadVisualSourceTarget(target, options = {}) {",
        "function applyIncompleteVisualSourceTarget(name, target) {",
    );
    let incomplete_visual = function_body(
        EDITOR_VISUAL_SOURCE,
        "function applyIncompleteVisualSourceTarget(name, target) {",
        "function parseVisualDefinitionSource(contract, selectorName = \"\") {",
    );
    let incomplete_visual3d = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function applyIncompleteVisual3dSourceTarget(name, target) {",
        "function applyLoadedVisual3d(name, loaded) {",
    );
    let visual3d_loader = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function applyLoadedVisual3d(name, loaded) {",
        "function resetVisual3dCamera() {",
    );

    assert_render_then_top_bar_sync(visual_loader, "renderVisualBuilder();");
    assert_render_then_top_bar_sync(incomplete_visual, "renderVisualBuilder();");
    assert_render_then_top_bar_sync(incomplete_visual3d, "renderVisual3dBuilder();");
    assert_render_then_top_bar_sync(visual3d_loader, "renderVisual3dBuilder();");
    assert!(top_bar_sync.contains(
        "currentVisualPaneMode === \"visual3d\" ? Boolean(visual3d.animationMode) : Boolean(visual.animationMode)"
    ));
    assert!(top_bar_sync.contains(
        "visualAnimateModeButton?.classList.toggle(\"is-active\", visualPaneVisible && visualAnimationActive);"
    ));
    assert!(top_bar_sync.contains(
        "visualAnimateModeButton?.setAttribute(\"aria-pressed\", String(visualPaneVisible && visualAnimationActive));"
    ));
}
