const RENDERER_JS: &str = include_str!("../static/renderer.js");

#[test]
fn canvas_consumes_only_the_rust_resolved_render_contract() {
    for forbidden in [
        "GameVisuals",
        "resolveVisual(",
        "visualFrames(",
        "paintCanvasMerged",
        "animationProgressForFrame",
        "animationDurationMs",
    ] {
        assert!(
            !RENDERER_JS.contains(forbidden),
            "2D Canvas renderer must not own `{forbidden}` semantics"
        );
    }
    for required in [
        "scene.renderScene",
        "prepareRenderScene",
        "resolveRenderMoment",
        "paintResolvedRenderFrame",
        "batch.pixelGeometry",
    ] {
        assert!(
            RENDERER_JS.contains(required),
            "2D Canvas renderer must consume `{required}`"
        );
    }
}

#[test]
fn logical_pixels_are_painted_directly_and_only_decoded_rasters_use_draw_image() {
    let raster_branch = RENDERER_JS
        .split_once("if (geometry.raster) {")
        .expect("resolved raster branch must be explicit")
        .1;
    let (raster_body, logical_body) = raster_branch
        .split_once("const pixelWidth = drawWidth / width;")
        .expect("logical pixel branch must follow the raster branch");

    assert!(raster_body.contains("context.drawImage(bitmap"));
    assert!(!logical_body.contains("context.drawImage(bitmap"));
    assert!(logical_body.contains("this.fillCanvasRect("));
}
