use puzzle_lang::{VisualKind, parse_game2d};

fn visual_pattern(source: &str) -> Vec<String> {
    let loaded = parse_game2d(source).expect("visual source should compile");
    let visual = loaded
        .visuals
        .entries
        .iter()
        .find(|visual| visual.name == "You")
        .expect("visual should be present");
    match &visual.kind {
        VisualKind::Ascii { .. } => visual.frames[0].planes[0].clone(),
        _ => panic!("You should have an ASCII visual"),
    }
}

fn source_with_visual_body(visual_body: &str, shapes: &str) -> String {
    format!(
        r##"
title = visual shape resolution

puzzle default {{
slots {{
actors = You
}}
visuals {{
{visual_body}
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
    let source = source_with_visual_body(
        "shapes {\nshape_You_F {\n01\n10\n}\n}\nYou\n#000 #fff\nshape_You_F",
        "",
    );
    assert_eq!(visual_pattern(&source), ["01", "10"]);
}

#[test]
fn bare_shape_reference_resolves_against_later_declaration_in_same_scope() {
    let source = source_with_visual_body(
        "You\n#000 #fff\nshape_You_F",
        "\nshapes {\nshape_You_F {\n01\n10\n}\n}",
    );
    assert_eq!(visual_pattern(&source), ["01", "10"]);
}

#[test]
fn unknown_bare_shape_name_reports_shape_error() {
    let source = source_with_visual_body("You\n#000 #fff\nmissing_shape", "");
    let error = parse_game2d(&source).unwrap_err().to_string();
    assert!(
        error.contains("unknown visual shape `missing_shape`"),
        "{error}"
    );
}

#[test]
fn bare_row_that_is_both_shape_name_and_ascii_requires_explicit_syntax() {
    let source = source_with_visual_body(
        "shapes {\na {\n0\n}\n}\nYou\n#000 #111 #222 #333 #444 #555 #666 #777 #888 #999 #aaa\na",
        "",
    );
    let error = parse_game2d(&source).unwrap_err().to_string();
    assert!(
        error.contains("bare visual row is both a declared shape name and valid ASCII"),
        "{error}"
    );
}

#[test]
fn explicit_shape_reference_resolves_an_ambiguous_bare_spelling() {
    let source = source_with_visual_body(
        "shapes {\na {\n01\n10\n}\n}\nYou\n#000 #111 #222 #333 #444 #555 #666 #777 #888 #999 #aaa\nshape = a",
        "",
    );
    assert_eq!(visual_pattern(&source), ["01", "10"]);
}

#[test]
fn valid_single_ascii_row_remains_inline_shape() {
    let source = source_with_visual_body("You\n#000\n0", "");
    assert_eq!(visual_pattern(&source), ["0"]);
}
