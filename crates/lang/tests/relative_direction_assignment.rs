use puzzle_core::{InputId, transition_state};
use puzzle_lang::{LoadedGame, parse_game2d};

fn direction_assignment_game(forward: &str, backward: &str) -> LoadedGame {
    parse_game2d(&format!(
        r#"
const title = relative_direction_assignment

puzzle default {{
tags {{
time = B F
}}
layers {{
anchor = Start
actor = TEN:time:directions
edge = NotEdge:directions
}}
empty .

rules {{
once right [ Start | TEN:*:* | Start ] -> [ Start NotEdge:{forward} | TEN:*:{forward} | Start NotEdge:{backward} ]
}}

level "start" {{
legend {{
S = Start
B = TEN:B:left
}}
SBS
}}
}}
"#
    ))
    .expect("direction assignments should compile")
}

fn object_named(game: &LoadedGame, name: &str) -> puzzle_core::ObjectId {
    game.object_labels
        .iter()
        .find_map(|(object, label)| (label == name).then_some(*object))
        .unwrap_or_else(|| panic!("missing object {name}"))
}

#[test]
fn relative_direction_is_an_absolute_value_before_rhs_selector_binding() {
    let relative = direction_assignment_game(">", "<");
    let absolute = direction_assignment_game("right", "left");

    let relative_state = transition_state(
        &relative.game,
        &relative.levels[0].initial_state,
        InputId(0),
    )
    .expect("relative direction assignment should run");
    let absolute_state = transition_state(
        &absolute.game,
        &absolute.levels[0].initial_state,
        InputId(0),
    )
    .expect("absolute direction assignment should run");

    for (x, name) in [
        (0, "NotEdge:right"),
        (1, "TEN:B:right"),
        (2, "NotEdge:left"),
    ] {
        assert!(
            relative_state.has_object(&relative.game, x, 0, object_named(&relative, name)),
            "relative assignment should produce {name} at x={x}"
        );
        assert!(
            absolute_state.has_object(&absolute.game, x, 0, object_named(&absolute, name)),
            "absolute assignment should produce {name} at x={x}"
        );
    }
}

#[test]
fn direction_assignment_with_an_unassigned_slot_requires_a_source_occurrence() {
    let error = parse_game2d(
        r#"
const title = unbound_direction_assignment

puzzle default {
tags {
time = B F
}
layers {
anchor = Start
actor = TEN:time:directions
}
empty .

rules {
once right [ Start ] -> [ Start TEN:*:right ]
}

level "start" {
legend {
S = Start
}
S
}
}
"#,
    )
    .expect_err("the unassigned time slot requires a matching LHS occurrence")
    .to_string();

    assert!(
        error.contains("selector assignment source must appear in before"),
        "{error}"
    );
}
