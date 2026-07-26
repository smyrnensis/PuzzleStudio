use puzzle_core::transition_trace;
use puzzle_lang::parse_game2d;

#[test]
fn once_input_wildcard_rotation_fires_only_one_lowered_alternative() {
    let loaded = parse_game2d(
        r#"
const title = once_input_wildcard_rotation

puzzle default {
layers {
actor = Player:directions
wall = Wall
}
empty .

rules {
once input [ Player:* | no Wall ] -> [ | Player:> ]
}

level "start" {
legend {
U = Player:up
D = Player:down
}
U.D.
}
}
"#,
    )
    .unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();

    let outcome = transition_trace(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();

    let source_lines = outcome
        .firings
        .iter()
        .filter_map(|firing| {
            loaded
                .rule_debug_info
                .get(&firing.rule)
                .map(|info| (firing.rule, info.source_line.clone()))
        })
        .collect::<Vec<_>>();
    let source_firings = source_lines
        .iter()
        .filter(|(_, line)| line.starts_with("once input"))
        .count();
    assert_eq!(source_firings, 1, "{source_lines:?}");
    assert_eq!(loaded.game.program().len(), 2);
}

#[test]
fn explicit_direction_loop_keeps_one_once_boundary_per_expanded_statement() {
    let loaded = parse_game2d(
        r#"
const title = explicit_once_direction_boundaries

puzzle default {
layers {
actor = Player:directions
wall = Wall
}
empty .

rules {
for d in directions {
once input d [ Player:* | no Wall ] -> [ | Player:> ]
}
}

level "start" {
legend {
U = Player:up
}
U.
}
}
"#,
    )
    .unwrap();

    assert_eq!(loaded.game.program().len(), 5);
}
