const EDITOR_VISUAL_SOURCE: &str = include_str!("../static/editor_visual.js");

fn function_body<'a>(source: &'a str, signature: &str, next_signature: &str) -> &'a str {
    source
        .split_once(signature)
        .and_then(|(_, tail)| tail.split_once(next_signature))
        .map(|(body, _)| body)
        .expect("function body")
}

#[test]
fn playback_controller_reads_frames_from_restored_animation_state() {
    let controller = function_body(
        EDITOR_VISUAL_SOURCE,
        "function sharedVisualAnimationController(dimension = currentVisualPaneMode) {",
        "function selectSharedVisualAnimationFrame(dimension, index) {",
    );

    assert!(controller.contains("get frames()"));
    assert!(controller.contains("return is3d ? state.frames : state.animationFrames;"));
    assert!(!controller.contains("frames: is3d ? state.frames : state.animationFrames"));
}
