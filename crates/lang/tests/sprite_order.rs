use puzzle_lang::{VisualOrderPriorityDef, parse_game2d, parse_puzzle3d};

fn game_with_slots(sprites_body: &str) -> String {
    format!(
        r#"
title = sprite_order

puzzle default {{
slots {{
background = Floor Goal
actor = Player Box
}}
sprites {{
{sprites_body}
sprite {{
selector = Floor
colors = #111111
}}
sprite {{
selector = Goal
colors = #222222
}}
sprite {{
selector = Player
colors = #333333
}}
sprite {{
selector = Box
colors = #444444
}}
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
fn explicit_order_can_split_slots_and_normalizes_plus_to_merge() {
    let source = game_with_slots(
        r#"order {
priority = down right
Floor
Player + Goal
Box
}"#,
    );

    let loaded = parse_game2d(&source).expect("explicit sprite order");

    assert_eq!(loaded.visuals.order.direction_priority, ["down", "right"]);
    assert_eq!(
        loaded.visuals.order.priorities,
        [
            VisualOrderPriorityDef {
                objects: vec!["Floor".to_string()],
                merge: false,
            },
            VisualOrderPriorityDef {
                objects: vec!["Goal".to_string(), "Player".to_string()],
                merge: true,
            },
            VisualOrderPriorityDef {
                objects: vec!["Box".to_string()],
                merge: false,
            },
        ]
    );
}

#[test]
fn omitted_order_is_generated_from_slot_declaration_order() {
    let loaded = parse_game2d(&game_with_slots("")).expect("generated sprite order");

    assert_eq!(loaded.visuals.order.direction_priority, ["down", "right"]);
    assert_eq!(
        loaded.visuals.order.priorities,
        [
            VisualOrderPriorityDef {
                objects: vec!["Floor".to_string(), "Goal".to_string()],
                merge: false,
            },
            VisualOrderPriorityDef {
                objects: vec!["Player".to_string(), "Box".to_string()],
                merge: false,
            },
        ]
    );
}

#[test]
fn direction_priority_requires_each_2d_axis_once() {
    let source = game_with_slots(
        r#"order {
priority = up down
}"#,
    );

    let error = parse_game2d(&source).expect_err("duplicate axis must fail");

    assert!(
        error
            .to_string()
            .contains("must name each coordinate axis exactly once"),
        "{error}"
    );
}

#[test]
fn old_layers_spelling_is_not_a_slots_compatibility_path() {
    let source = game_with_slots("").replacen("slots {", "layers {", 1);

    let error = parse_game2d(&source).expect_err("old spelling must fail");

    assert!(!error.to_string().is_empty());
}

#[test]
fn three_dimensional_direction_priority_requires_and_preserves_three_axes() {
    let source = r#"
sprites {
order {
priority = down right front
Floor
Player
}
}

puzzle board {
dimension = 3
slots {
Floor
Player
}
rules {
}
}

levels demo of board {
legend {
. = empty
P = Player
}
level "start" {
P
}
}
"#;

    let parsed = parse_puzzle3d(source).expect("3D sprite order");

    assert_eq!(
        parsed.visual_order.direction_priority,
        ["down", "right", "front"]
    );
    let fixture = puzzle_lang::export_visual_fixture_json(&parsed).expect("3D fixture");
    let fixture: serde_json::Value = serde_json::from_str(&fixture).expect("fixture JSON");
    assert_eq!(
        fixture["order"]["direction_priority"],
        serde_json::json!(["down", "right", "front"])
    );
}
