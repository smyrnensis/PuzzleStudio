use std::collections::BTreeSet;

use puzzle_core::{GridConditionValueKind, GridWriteOp, Size3, WriteOp, flattened_rules};
use puzzle_kernel::SpatialOffset;
use puzzle_lang::{LoadedDocumentModel, LoadedGame, LoadedGridGame, parse_game_for_path};

fn parse_2d_rules(rules: &str) -> LoadedGame {
    let source = format!(
        r#"
puzzle test {{
layers {{
actor = A
}}
rules {{
{rules}
}}
}}
"#
    );
    let document = parse_game_for_path(&source, "implicit_2d_rule_orientation.puzzle")
        .expect("2D rule source should compile");
    let model = document
        .models
        .into_iter()
        .next()
        .expect("source should contain one model");
    match model {
        LoadedDocumentModel::Puzzle2d { game, .. } => game,
        LoadedDocumentModel::Puzzle3d { .. } => panic!("expected a 2D model"),
    }
}

fn parse_3d_body(body: &str) -> LoadedGridGame<3, Size3> {
    let source = format!(
        r#"
puzzle test {{
dimension = 3
layers {{
actor = A
}}
{body}
}}
"#
    );
    let document = parse_game_for_path(&source, "implicit_3d_rule_orientation.puzzle3")
        .expect("3D rule source should compile");
    let model = document
        .models
        .into_iter()
        .next()
        .expect("source should contain one model");
    match model {
        LoadedDocumentModel::Puzzle3d { game, .. } => game,
        LoadedDocumentModel::Puzzle2d { .. } => panic!("expected a 3D model"),
    }
}

fn parse_3d_rules(rules: &str) -> LoadedGridGame<3, Size3> {
    parse_3d_body(&format!("rules {{\n{rules}\n}}"))
}

fn move_offsets(game: &LoadedGridGame<3, Size3>) -> Vec<[i16; 3]> {
    let mut offsets = flattened_rules(game.game.program())
        .into_iter()
        .flat_map(|rule| rule.writes.into_iter())
        .filter_map(|write| match write {
            GridWriteOp::Move {
                to_offset: SpatialOffset::Fixed { delta },
                ..
            } => Some(delta.axes()),
            _ => None,
        })
        .collect::<Vec<_>>();
    offsets.sort_unstable();
    offsets
}

#[test]
fn prefixless_spatial_rule_defaults_to_the_3d_horizontal_plane() {
    let game = parse_3d_rules("[ A | ] -> [ | A ]");

    assert_eq!(
        move_offsets(&game),
        vec![[-1, 0, 0], [0, -1, 0], [0, 1, 0], [1, 0, 0]]
    );
}

#[test]
fn prefixless_spatial_rule_keeps_all_four_2d_directions() {
    let game = parse_2d_rules("[ A | ] -> [ | A ]");
    let move_count = flattened_rules(game.game.program())
        .into_iter()
        .flat_map(|rule| rule.writes.into_iter())
        .filter(|write| matches!(write, WriteOp::Move { .. }))
        .count();

    assert_eq!(move_count, 4);
}

#[test]
fn prefixless_3d_condition_pattern_uses_the_same_horizontal_default() {
    let game = parse_3d_body(
        r#"
query adjacent = exists([ A | A ])
rules {
}
"#,
    );
    let condition = game
        .game
        .condition_defs()
        .first()
        .expect("query should produce one condition definition");
    let GridConditionValueKind::ExistsMatches(patterns) = &condition.kind else {
        panic!("query should lower to exists-matches patterns");
    };

    let offsets = patterns
        .iter()
        .flat_map(|pattern| pattern.cells())
        .filter_map(|cell| match cell.offset {
            SpatialOffset::Fixed { delta } if delta.axes() != [0, 0, 0] => Some(delta.axes()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(
        offsets,
        BTreeSet::from([[-1, 0, 0], [0, -1, 0], [0, 1, 0], [1, 0, 0]])
    );
}

#[test]
fn explicit_directions_still_expands_over_all_six_3d_directions() {
    let game = parse_3d_rules("directions [ A | ] -> [ | A ]");

    assert_eq!(
        move_offsets(&game),
        vec![
            [-1, 0, 0],
            [0, -1, 0],
            [0, 0, -1],
            [0, 0, 1],
            [0, 1, 0],
            [1, 0, 0],
        ]
    );
}
