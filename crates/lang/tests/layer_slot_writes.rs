use puzzle_core::{InputId, transition_state};
use puzzle_lang::parse_game2d;

#[test]
fn rhs_add_sets_an_occupied_collision_layer_slot() {
    let loaded = parse_game2d(
        r#"
const title = layer_slot_write

puzzle default {
layers {
state = StateA StateB
control = Control
}
empty .

rules {
once [ Control ] -> [ Control StateB ]
}

level "start" {
legend {
X = Control StateA
}
X
}
}
"#,
    )
    .unwrap();
    let state_b = loaded
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "StateB").then_some(*object))
        .unwrap();

    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert!(next.has_object(&loaded.game, 0, 0, state_b));
}

#[test]
fn later_routine_rule_sees_tag_variant_written_by_previous_rule() {
    let loaded = parse_game2d(
        r#"
const title = sequential_tag_write

puzzle default {
tags {
D = F B
}
map D_rev D {
F -> B
B -> F
}
layers {
actor = You:D
ink = Ink:D
time = Time:D
computer = Computer
}
empty .

routine flip once {
[ You:D#1 Ink:D_rev(D#1) ] -> [ You:D_rev(D#1) Ink:D_rev(D#1) ]
[ Computer ] [ You:D#1 ] -> [ Computer Time:D#1 ] [ You:D#1 ]
}

rules {
flip
}

level "start" {
legend {
C = Computer Time:F
P = You:F Ink:B
}
CP
}
}
"#,
    )
    .unwrap();
    let you_b = loaded
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "You:B").then_some(*object))
        .unwrap();
    let time_b = loaded
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "Time:B").then_some(*object))
        .unwrap();

    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert!(next.has_object(&loaded.game, 1, 0, you_b));
    assert!(next.has_object(&loaded.game, 0, 0, time_b));
}
