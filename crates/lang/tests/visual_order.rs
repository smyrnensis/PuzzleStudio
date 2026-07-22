use puzzle_lang::{LoadedDocumentModel, VisualOrderPriorityDef, parse_game_for_path, parse_game2d};

fn game_with_layers(layers_body: &str, visuals_extra: &str) -> String {
    format!(
        r#"
title = visual_order

puzzle default {{
layers {{
{layers_body}
}}
visuals {{
visual Floor {{
colors = #111111
}}
visual Goal {{
colors = #222222
}}
visual Player {{
colors = #333333
}}
visual Box {{
colors = #444444
}}
{visuals_extra}
}}
rules {{
}}
levels {{
legend {{
. = empty
P = Player
}}
level "start"
P
}}
}}
"#
    )
}

#[test]
fn layers_own_state_storage_render_order_animations_and_directional_priority() {
    let loaded = parse_game2d(&game_with_layers(
        r#"priority = down right
background = Floor Goal
actor = Player Box !Box
!Burst"#,
        "visual !Burst {\nduration = 90ms\ncolors = #ffffff\n0\n}",
    ))
    .expect("unified layers");

    assert_eq!(loaded.game.layer_count, 2);
    assert_eq!(loaded.visuals.order.direction_priority, ["down", "right"]);
    assert_eq!(
        loaded.visuals.order.priorities,
        [
            VisualOrderPriorityDef {
                objects: vec!["Floor".to_string(), "Goal".to_string()],
                animations: Vec::new(),
                merge: false,
            },
            VisualOrderPriorityDef {
                objects: vec!["Player".to_string(), "Box".to_string()],
                animations: vec!["Box".to_string()],
                merge: false,
            },
            VisualOrderPriorityDef {
                objects: Vec::new(),
                animations: vec!["Burst".to_string()],
                merge: false,
            },
        ]
    );
}

#[test]
fn merge_keeps_separate_state_layers_and_builds_one_unordered_render_priority() {
    let loaded = parse_game2d(&game_with_layers(
        r#"background = Floor
merge {
actor = Player Box
goal = Goal
effect = !Burst
}"#,
        "visual !Burst {\nduration = 90ms\ncolors = #ffffff\n0\n}",
    ))
    .expect("merged layers");

    assert_eq!(loaded.game.layer_count, 3);
    assert_eq!(
        loaded.visuals.order.priorities,
        [
            VisualOrderPriorityDef {
                objects: vec!["Floor".to_string()],
                animations: Vec::new(),
                merge: false,
            },
            VisualOrderPriorityDef {
                objects: vec!["Box".to_string(), "Goal".to_string(), "Player".to_string()],
                animations: vec!["Burst".to_string()],
                merge: true,
            },
        ]
    );
}

#[test]
fn visuals_order_is_rejected() {
    let source = game_with_layers(
        "background = Floor Goal\nactor = Player Box",
        "order { Floor; Player; Goal; Box }",
    );

    let error = parse_game2d(&source).expect_err("visuals order must fail");
    assert!(!error.to_string().is_empty(), "{error}");
}

#[test]
fn slots_spelling_is_rejected() {
    let source = game_with_layers("background = Floor Goal\nactor = Player Box", "")
        .replacen("layers {", "slots {", 1);

    let error = parse_game2d(&source).expect_err("removed spelling must fail");
    assert!(
        error
            .to_string()
            .contains("`slots` was removed; use `layers { ... }`"),
        "{error}"
    );
}

#[test]
fn three_dimensional_layers_require_and_preserve_three_direction_axes() {
    let source = r#"
puzzle board {
dimension = 3
layers {
priority = down right front
Floor
Player
}
rules {
}
levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}
}
"#;

    let document = parse_game_for_path(source, "test.puzzle").expect("3D visual order");
    let Some(LoadedDocumentModel::Puzzle3d {
        game, presentation, ..
    }) = document.single_model()
    else {
        panic!("expected one spatial model");
    };

    assert_eq!(
        presentation.visual_order.direction_priority,
        ["down", "right", "front"]
    );
    let fixture = puzzle_lang::export_visual_fixture_json(game, presentation).expect("3D fixture");
    let fixture: serde_json::Value = serde_json::from_str(&fixture).expect("fixture JSON");
    assert_eq!(
        fixture["order"]["direction_priority"],
        serde_json::json!(["down", "right", "front"])
    );
}

#[test]
fn directional_priority_rejects_repeating_one_axis() {
    let error = parse_game2d(&game_with_layers(
        "priority = up down\nbackground = Floor Goal\nactor = Player Box",
        "",
    ))
    .expect_err("duplicate direction axis must fail");

    assert!(
        error
            .to_string()
            .contains("must name each coordinate axis exactly once"),
        "{error}"
    );
}

#[test]
fn animation_layer_reference_requires_a_visual_resource() {
    let error = parse_game2d(&game_with_layers(
        "background = Floor Goal\nactor = Player Box\n!Missing",
        "",
    ))
    .expect_err("unknown animation visual must fail");

    assert!(
        error
            .to_string()
            .contains("unknown animation visual in layers: !Missing"),
        "{error}"
    );
}
