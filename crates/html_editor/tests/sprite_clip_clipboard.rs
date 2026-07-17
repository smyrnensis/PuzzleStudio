const EDITOR_SPRITE_SOURCE: &str = include_str!("../static/editor_sprite.js");
const EDITOR_SPRITE3D_SOURCE: &str = include_str!("../static/editor_sprite3d.js");

fn function_body<'a>(source: &'a str, signature: &str, next_signature: &str) -> &'a str {
    source
        .split_once(signature)
        .and_then(|(_, tail)| tail.split_once(next_signature))
        .map(|(body, _)| body)
        .expect("function body")
}

#[test]
fn sprite_clip_clipboard_text_is_an_unnamed_sprite_body() {
    let clipboard_text = function_body(
        EDITOR_SPRITE_SOURCE,
        "function spriteClipboardTextForRect(rect) {",
        "async function copySpriteEditRegion()",
    );

    assert!(clipboard_text.contains("`colors = ${spritePaletteSourceTokens().join(\" \")}`"));
    assert!(clipboard_text.contains("\"shape = {\""));
    assert!(!clipboard_text.contains("SpriteClip"));
}

#[test]
fn sprite3d_clip_clipboard_text_is_an_unnamed_sprite_body() {
    let clipboard_text = function_body(
        EDITOR_SPRITE3D_SOURCE,
        "function sprite3dClipboardSourceText(clipboard) {",
        "async function copySprite3dEditRegion()",
    );

    assert!(clipboard_text.contains("`colors = ${sprite3dPaletteSourceTokens().join(\" \")}`"));
    assert!(clipboard_text.contains("\"shape = {\""));
    assert!(!clipboard_text.contains("Sprite3dClip"));
}
