use puzzle_core::flattened_rules;
use puzzle_core::{Delta3, GridGuard, GridWriteOp, ObjectId, Size3};
use puzzle_lang::{
    LoadedDocumentModel, LoadedGridGame, SpatialPresentation, VoxelColor,
    export_visual_fixture_json, parse_game_for_path,
};
use puzzle_runtime_contract::{CameraEffect, GridRuntimeModel, runtime_contract_from_fixture_json};

fn parse_spatial(source: &str) -> (LoadedGridGame<3, Size3>, SpatialPresentation) {
    let document = parse_game_for_path(source, "test.puzzle").expect("canonical document parses");
    let mut models = document.models.into_iter();
    let model = models.next().expect("document contains a model");
    assert!(
        models.next().is_none(),
        "test source must contain one model"
    );
    match model {
        LoadedDocumentModel::Puzzle3d {
            game, presentation, ..
        } => (game, presentation),
        LoadedDocumentModel::Puzzle2d { .. } => panic!("expected a spatial model"),
    }
}

fn parse_spatial_body(body: &str) -> (LoadedGridGame<3, Size3>, SpatialPresentation) {
    parse_spatial(&format!("puzzle test {{\ndimension = 3\n{body}\n}}"))
}

#[test]
fn canonical_document_lowers_spatial_line_rules() {
    let (game, _) = parse_spatial_body(
        r#"
slots {
Player Box Wall
}

groups {
solid = Player Box Wall
}

rules {
horizontal [ Player | no solid ] -> [ | Player ]
}
"#,
    );

    let rules = flattened_rules(game.game.program());
    assert_eq!(game.game.layer_count, 1);
    assert_eq!(game.game.objects().len(), 3);
    assert_eq!(rules.len(), 5);
    assert_eq!(
        rules[1].pattern.cells()[1].offset,
        Delta3::new(1, 0, 0).into()
    );
    assert_eq!(
        rules[1].writes,
        vec![GridWriteOp::<3>::Move {
            component: 0,
            from_offset: Delta3::ZERO.into(),
            to_offset: Delta3::new(1, 0, 0).into(),
            object: ObjectId(1),
        }]
    );
}

#[test]
fn direction_without_an_axis_set_expands_over_the_spatial_input_domain() {
    let (game, _) = parse_spatial_body(
        r#"
slots {
actor = Player Wall
}

groups {
solid = Player Wall
}

rules {
input [ Player | no solid ] -> [ | Player ]
}
"#,
    );

    let rules = flattened_rules(game.game.program())
        .into_iter()
        .filter(|rule| !rule.guards.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(rules.len(), 6);
    for name in ["left", "right", "up", "down", "front", "back"] {
        let input = game
            .inputs
            .iter()
            .find(|input| input.name == name)
            .unwrap_or_else(|| panic!("missing input {name}"));
        assert!(
            rules
                .iter()
                .any(|rule| { rule.guards == vec![GridGuard::<3>::InputIs(input.id)] })
        );
    }
}

#[test]
fn comma_separates_frame_axes_and_colon_does_not() {
    let (game, _) = parse_spatial_body(
        r#"
slots {
Player Box
}

rules {
right, up [ Player | Box | ] -> [ | Player | Box ]
}
"#,
    );
    assert_eq!(flattened_rules(game.game.program()).len(), 2);

    let error = parse_game_for_path(
        r#"
puzzle invalid {
dimension = 3
slots { Player }
rules {
right:up [ Player ] -> [ Player ]
}
}
"#,
        "test.puzzle",
    )
    .expect_err("colon is not frame-axis syntax")
    .to_string();
    assert!(error.contains("unknown orientation: right:up"), "{error}");
}

#[test]
fn spatial_levels_use_the_shared_loaded_game_container() {
    let (game, _) = parse_spatial(
        r#"
puzzle board {
dimension = 3
slots {
floor = Goal
actor = Player Box Wall
}
rules {
}
}

levels demo of board {
legend {
. = empty
P = Player
B = Box
# = Wall
G = Goal
}
level "stacked" {
...
.G.
...

###
#PB
###
}
}
"#,
    );

    assert_eq!(game.game.layer_count, 2);
    assert_eq!(game.levels.len(), 1);
    assert_eq!(game.levels[0].name, "stacked");
    assert_eq!(game.levels[0].initial_state.size, Size3::new(3, 3, 2));
}

#[test]
fn spatial_sprite_materialization_derives_from_shared_visuals() {
    let (game, presentation) = parse_spatial_body(
        r##"
slots {
floor = Floor
}
rules {
}

sprites basic {
Floor {
colors = #90ee90 #008000 transparent
shape = {
.....
..1..
.....
-
00000
0...0
00000
}
}
}
"##,
    );

    let sprites = presentation.sprite_set.as_ref().expect("sprite set exists");
    let floor = sprites.sprite("Floor").expect("Floor sprite exists");
    assert_eq!(
        floor.palette.get(&'0'),
        Some(&VoxelColor::Hex("#90ee90".to_string()))
    );
    assert_eq!(floor.palette.get(&'2'), Some(&VoxelColor::Transparent));
    assert_eq!(floor.first_frame().size, Size3::new(5, 3, 2));
    assert_eq!(game.visuals.sprites.len(), 1);
}

#[test]
fn visual_fixture_round_trips_the_shared_runtime_model() {
    let (game, presentation) = parse_spatial(
        r#"
puzzle board {
dimension = 3
slots { Player }
rules {
}
}
levels demo of board {
legend { P = Player }
level "start" { P }
}
"#,
    );

    let fixture = export_visual_fixture_json(&game, &presentation).expect("fixture exports");
    let contract =
        runtime_contract_from_fixture_json::<GridRuntimeModel<3, Size3, CameraEffect>>(&fixture)
            .expect("shared runtime contract decodes");

    assert_eq!(contract.model.game, game.game);
    assert_eq!(contract.model.level_bundle.level_count(), 1);
}
