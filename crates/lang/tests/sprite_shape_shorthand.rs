use puzzle_lang::{VisualSpriteKind, parse_game2d};

fn sprite_pattern(source: &str) -> Vec<String> {
    let loaded = parse_game2d(source).expect("sprite source should compile");
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "You")
        .expect("sprite should be present");
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, .. } => pattern.clone(),
        _ => panic!("You should have an ASCII sprite"),
    }
}

fn source_with_sprite_body(sprite_body: &str, shapes: &str) -> String {
    format!(
        r##"
title = sprite shape resolution

puzzle default {{
layers {{
actors = You
}}
sprites {{
{sprite_body}
{shapes}
}}
rules {{
}}
levels {{
legend {{
. = empty
Y = You
}}
level "start"
Y
}}
}}
"##
    )
}

#[test]
fn declared_shape_name_after_colors_is_a_bare_reference() {
    let source = source_with_sprite_body(
        "shapes {\nshape_You_F {\n01\n10\n}\n}\nYou\n#000 #fff\nshape_You_F",
        "",
    );
    assert_eq!(sprite_pattern(&source), ["01", "10"]);
}

#[test]
fn bare_shape_reference_resolves_against_later_declaration_in_same_scope() {
    let source = source_with_sprite_body(
        "You\n#000 #fff\nshape_You_F",
        "\nshapes {\nshape_You_F {\n01\n10\n}\n}",
    );
    assert_eq!(sprite_pattern(&source), ["01", "10"]);
}

#[test]
fn unknown_bare_shape_name_reports_shape_error() {
    let source = source_with_sprite_body("You\n#000 #fff\nmissing_shape", "");
    let error = parse_game2d(&source).unwrap_err().to_string();
    assert!(
        error.contains("unknown sprite shape `missing_shape`"),
        "{error}"
    );
}

#[test]
fn bare_row_that_is_both_shape_name_and_ascii_requires_explicit_syntax() {
    let source = source_with_sprite_body(
        "shapes {\na {\n0\n}\n}\nYou\n#000 #111 #222 #333 #444 #555 #666 #777 #888 #999 #aaa\na",
        "",
    );
    let error = parse_game2d(&source).unwrap_err().to_string();
    assert!(
        error.contains("bare sprite row is both a declared shape name and valid ASCII"),
        "{error}"
    );
}

#[test]
fn explicit_shape_reference_resolves_an_ambiguous_bare_spelling() {
    let source = source_with_sprite_body(
        "shapes {\na {\n01\n10\n}\n}\nYou\n#000 #111 #222 #333 #444 #555 #666 #777 #888 #999 #aaa\nshape = a",
        "",
    );
    assert_eq!(sprite_pattern(&source), ["01", "10"]);
}

#[test]
fn valid_single_ascii_row_remains_inline_shape() {
    let source = source_with_sprite_body("You\n#000\n0", "");
    assert_eq!(sprite_pattern(&source), ["0"]);
}
