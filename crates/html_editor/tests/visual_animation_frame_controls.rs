const EDITOR_HTML: &str = include_str!("../static/editor.html");
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
fn frame_buttons_operate_on_the_current_frame_immediately() {
    let insert_current = function_body(
        EDITOR_VISUAL_SOURCE,
        "function insertSharedVisualAnimationFrameAfterCurrent(dimension = currentVisualPaneMode) {",
        "function removeSharedVisualAnimationCurrentFrame(dimension = currentVisualPaneMode) {",
    );
    let remove_current = function_body(
        EDITOR_VISUAL_SOURCE,
        "function removeSharedVisualAnimationCurrentFrame(dimension = currentVisualPaneMode) {",
        "function setVisualAnimationFrame(index) {",
    );

    assert!(insert_current.contains("context.state.animationFrameIndex + 1"));
    assert!(insert_current.contains("insertSharedVisualAnimationFrameAt("));
    assert!(remove_current.contains("context.state.animationFrameIndex,"));
    assert!(remove_current.contains("removeSharedVisualAnimationFrameAt("));
    assert!(EDITOR_VISUAL_SOURCE.contains(
        "visualAnimationInsertFrameButton?.addEventListener(\"click\", () => insertSharedVisualAnimationFrameAfterCurrent());"
    ));
    assert!(EDITOR_VISUAL_SOURCE.contains(
        "visualAnimationRemoveFrameButton?.addEventListener(\"click\", () => removeSharedVisualAnimationCurrentFrame());"
    ));
}

#[test]
fn frame_strip_remains_a_selection_surface_for_2d_and_3d() {
    let frame_strip = function_body(
        EDITOR_VISUAL_SOURCE,
        "function renderVisualAnimationFrameStripView(options) {",
        "function sharedVisualAnimationController(dimension = currentVisualPaneMode) {",
    );

    assert!(
        frame_strip.contains("button.addEventListener(\"click\", () => options.onSelect(index));")
    );
    assert!(!frame_strip.contains("onRemove"));
    assert!(!EDITOR_VISUAL_SOURCE.contains("visualAnimationInsertMode"));
    assert!(!EDITOR_VISUAL_SOURCE.contains("visualAnimationRemoveMode"));
    assert!(!EDITOR_VISUAL_SOURCE.contains("toggleSharedVisualAnimationEditMode"));
    assert!(!EDITOR_VISUAL3D_SOURCE.contains("showInsertTargets"));
    assert!(!EDITOR_VISUAL3D_SOURCE.contains("showRemoveTargets"));
    assert!(!EDITOR_CSS.contains(".visual-animation-insert-target"));
    assert!(!EDITOR_CSS.contains(".visual-animation-frame-strip.is-insert-mode"));
    assert!(!EDITOR_CSS.contains(".visual-animation-frame-strip.is-remove-mode"));
}

#[test]
fn frame_action_buttons_expose_command_semantics() {
    let insert_button = EDITOR_HTML
        .split_once(r#"id="visualAnimationInsertFrameButton""#)
        .and_then(|(_, tail)| tail.split_once("</button>"))
        .map(|(button, _)| button)
        .expect("insert frame button");
    let remove_button = EDITOR_HTML
        .split_once(r#"id="visualAnimationRemoveFrameButton""#)
        .and_then(|(_, tail)| tail.split_once("</button>"))
        .map(|(button, _)| button)
        .expect("remove frame button");

    assert!(insert_button.contains(r#"aria-label="Add animation frame after current frame""#));
    assert!(remove_button.contains(r#"aria-label="Remove current animation frame""#));
    assert!(!insert_button.contains("aria-pressed"));
    assert!(!remove_button.contains("aria-pressed"));
}
