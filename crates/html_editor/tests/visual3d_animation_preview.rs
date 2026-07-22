const EDITOR_CSS: &str = include_str!("../static/editor.css");
const EDITOR_VISUAL_SOURCE: &str = include_str!("../static/editor_visual.js");
const EDITOR_VISUAL3D_SOURCE: &str = include_str!("../static/editor_visual3d.js");

fn function_body<'a>(source: &'a str, signature: &str, next_signature: &str) -> &'a str {
    source
        .split_once(signature)
        .and_then(|(_, tail)| tail.split_once(next_signature))
        .map(|(body, _)| body)
        .expect("function body")
}

#[test]
fn visual3d_frame_surfaces_render_the_cells_passed_to_them() {
    let merged_faces = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function visual3dMergedVoxelFaces(occupied, view) {",
        "function visual3dUnitFaceRects(cells) {",
    );

    assert!(merged_faces.contains("[...occupied.values()]"));
    assert!(!merged_faces.contains("visual3d.cells"));
}

#[test]
fn visual3d_thumbnail_projection_uses_the_thumbnail_surface() {
    let render_preview = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function renderVisual3dPreviewCanvas(canvas, cells, options = {}) {",
        "function visual3dPreviewView(width, height, options = {}) {",
    );
    let preview_view = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function visual3dPreviewView(width, height, options = {}) {",
        "function visual3dBoundsCorners() {",
    );

    assert!(render_preview.contains("reserveOverlaySpace: options.overlays !== false"));
    assert!(preview_view.contains("const contentPadding = reserveOverlaySpace ? padding : 3;"));
    assert!(preview_view.contains("const safeTop = reserveOverlaySpace ? overlaySafeInset : 0;"));
    assert!(
        preview_view.contains("const safeBottom = reserveOverlaySpace ? overlaySafeInset : 0;")
    );
}

#[test]
fn visual3d_frame_canvases_fill_their_button_content_box() {
    let mount_animation_ui = function_body(
        EDITOR_VISUAL_SOURCE,
        "function mountSharedVisualAnimationUi(dimension) {",
        "function syncVisualAnimationInputValues(options = {}) {",
    );

    assert!(mount_animation_ui.contains("panel.classList.add(\"visual3d-animation-panel\")"));
    assert!(mount_animation_ui.contains("panel.classList.remove(\"visual3d-animation-panel\")"));
    assert!(EDITOR_CSS.contains(
        ".visual3d-animation-panel .visual-animation-frame-button {\n  display: block;\n}"
    ));
}
