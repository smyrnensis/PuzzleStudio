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
fn visual_clip_clipboard_text_is_an_unnamed_visual_body() {
    let clipboard_text = function_body(
        EDITOR_VISUAL_SOURCE,
        "function visualClipboardTextForRect(rect) {",
        "async function copyVisualEditRegion()",
    );

    assert!(clipboard_text.contains("`colors = ${visualPaletteSourceTokens().join(\" \")}`"));
    assert!(clipboard_text.contains("\"shape = {\""));
    assert!(!clipboard_text.contains("VisualClip"));
}

#[test]
fn visual3d_clip_clipboard_text_is_an_unnamed_visual_body() {
    let clipboard_text = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function visual3dClipboardSourceText(clipboard) {",
        "async function copyVisual3dEditRegion()",
    );

    assert!(clipboard_text.contains("`colors = ${visual3dPaletteSourceTokens().join(\" \")}`"));
    assert!(clipboard_text.contains("\"shape = {\""));
    assert!(!clipboard_text.contains("Visual3dClip"));
}
