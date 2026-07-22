const EDITOR_CSS: &str = include_str!("../static/editor.css");

#[test]
fn non_empty_3d_layer_cells_do_not_add_a_theme_background_overlay() {
    let selector = ".level3d-layer-board.is-grid-board .level3d-layer-cell:not(.is-empty)";
    let Some(rule_start) = EDITOR_CSS.find(&format!("{selector} {{")) else {
        return;
    };
    let rule = &EDITOR_CSS[rule_start..];
    let rule = rule
        .split_once('}')
        .map(|(body, _)| body)
        .expect("3D layer cell CSS rule should be closed");

    assert!(
        !rule.contains("background"),
        "3D layer cells must leave transparent visual pixels transparent"
    );
}
