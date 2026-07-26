use puzzle_lang::{Diagnostic, parse_game_for_path, parse_game2d};

fn first_diagnostic(source: &str) -> Diagnostic {
    parse_game2d(source)
        .expect_err("invalid source must fail")
        .diagnostics()
        .first()
        .expect("compiler diagnostic")
        .clone()
}

fn line_number(source: &str, expected: &str) -> Option<usize> {
    source
        .lines()
        .position(|line| line == expected)
        .map(|line| line + 1)
}

fn first_diagnostic_for_path(source: &str, path: &str) -> Diagnostic {
    parse_game_for_path(source, path)
        .expect_err("invalid source must fail")
        .diagnostics()
        .first()
        .expect("compiler diagnostic")
        .clone()
}

#[test]
fn level_materialization_diagnostic_uses_the_authored_cell_row() {
    let source = r#"const title = "probe"

puzzle main {
layers {
base = Floor
}

rules {
}

levels {
legend {
. = empty
}
level "first"
X
}
}
"#;
    let diagnostic = first_diagnostic(source);
    let span = diagnostic.primary_span.expect("source diagnostic span");

    assert_eq!(diagnostic.message, "unknown level char 'X'");
    assert_eq!(span.line, line_number(source, "X"));
    assert_eq!(span.source_line.as_deref(), Some("X"));
}

#[test]
fn level_shape_diagnostic_uses_the_offending_row() {
    let source = r#"const title = "probe"

puzzle main {
layers {
base = Floor
}

rules {
}

levels {
legend {
. = empty
}
level "first"
..
.
}
}
"#;
    let diagnostic = first_diagnostic(source);
    let span = diagnostic.primary_span.expect("source diagnostic span");

    assert_eq!(diagnostic.message, "level regions must be rectangular");
    assert_eq!(span.line, line_number(source, "."));
    assert_eq!(span.source_line.as_deref(), Some("."));
}

#[test]
fn spatial_level_separator_diagnostic_uses_the_separator_row() {
    let source = r#"const title = "probe"

puzzle main {
dimension = 3
layers {
base = Floor
}
rules {
}
}

levels probe of main {
level "first" {
-
.
}
}
"#;
    let diagnostic = first_diagnostic_for_path(source, "probe.puzzle");
    let span = diagnostic.primary_span.expect("source diagnostic span");

    assert_eq!(
        diagnostic.message,
        "3D level slice separator requires a preceding ASCII slice"
    );
    assert_eq!(span.line, line_number(source, "-"));
    assert_eq!(span.source_line.as_deref(), Some("-"));
}

#[test]
fn spatial_level_shape_diagnostic_uses_the_offending_row() {
    let source = r#"const title = "probe"

puzzle main {
dimension = 3
layers {
base = Floor
}
rules {
}
}

levels probe of main {
level "first" {
..
..
-
..
.
}
}
"#;
    let diagnostic = first_diagnostic_for_path(source, "probe.puzzle");
    let span = diagnostic.primary_span.expect("source diagnostic span");

    assert_eq!(
        diagnostic.message,
        "3D level `first` must be rectangular in every slice"
    );
    assert_eq!(span.line, line_number(source, "."));
    assert_eq!(span.source_line.as_deref(), Some("."));
}

#[test]
fn missing_rules_diagnostic_uses_the_owning_model_declaration() {
    let source = r#"const title = "probe"

puzzle main {
layers {
base = Floor
}

levels {
legend {
. = empty
}
level "first"
.
}
}
"#;
    let diagnostic = first_diagnostic(source);
    let span = diagnostic.primary_span.expect("source diagnostic span");

    assert_eq!(diagnostic.message, "missing puzzle rules");
    assert_eq!(span.line, line_number(source, "puzzle main {"));
    assert_eq!(span.source_line.as_deref(), Some("puzzle main {"));
}

#[test]
fn spatial_visual_materialization_diagnostic_uses_the_visual_declaration() {
    let source = r#"const title = "probe"

puzzle main {
dimension = 3
layers {
base = Floor
}
rules {
}
}

visuals basic of main {
Floor {
image = "floor.png"
}
}
"#;
    let diagnostic = first_diagnostic_for_path(source, "probe.puzzle");
    let span = diagnostic.primary_span.expect("source diagnostic span");

    assert_eq!(
        diagnostic.message,
        "3D voxel renderer cannot materialize image visual `Floor`"
    );
    assert_eq!(span.line, line_number(source, "Floor {"));
    assert_eq!(span.source_line.as_deref(), Some("Floor {"));
}
