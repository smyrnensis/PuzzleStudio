use puzzle_core::flattened_rules;
use puzzle_core::{Coord3, Delta3, GridGuard, GridWriteOp, ObjectId, Size3, transition_state};
use puzzle_lang::{
    LoadedDocumentModel, LoadedGridGame, SpatialPresentation, VoxelColor,
    export_visual_fixture_json, parse_game_for_path,
};

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
layers {
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
layers {
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
fn relative_selector_accepts_author_defined_direction_typed_tag_set() {
    let (game, _) = parse_spatial_body(
        r#"
tags {
heading = left right front back
}

layers {
actor = TEN:heading
}

rules {
input [ TEN:heading ] -> [ > TEN:> ]
}
"#,
    );

    let rules = flattened_rules(game.game.program())
        .into_iter()
        .filter(|rule| !rule.guards.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(rules.len(), 16);
    for name in ["left", "right", "front", "back"] {
        let input = game
            .inputs
            .iter()
            .find(|input| input.name == name)
            .unwrap_or_else(|| panic!("missing input {name}"));
        assert!(
            rules
                .iter()
                .any(|rule| rule.guards == vec![GridGuard::<3>::InputIs(input.id)]),
            "missing relative selector rule for {name}"
        );
    }
    for name in ["up", "down"] {
        let input = game
            .inputs
            .iter()
            .find(|input| input.name == name)
            .unwrap_or_else(|| panic!("missing input {name}"));
        assert!(
            rules
                .iter()
                .all(|rule| rule.guards != vec![GridGuard::<3>::InputIs(input.id)]),
            "relative selector must not lower outside its tag domain: {name}"
        );
    }
}

#[test]
fn relative_selector_accepts_cartesian_axis_set() {
    let (game, _) = parse_spatial_body(
        r#"
layers {
actor = TEN:x_axis
}

rules {
input [ TEN:x_axis ] -> [ > TEN:> ]
}
"#,
    );

    let guarded_rules = flattened_rules(game.game.program())
        .into_iter()
        .filter(|rule| !rule.guards.is_empty())
        .count();
    assert_eq!(guarded_rules, 4);
}

#[test]
fn relative_selector_projects_unconstrained_slots_from_each_source_occurrence() {
    let (game, _) = parse_spatial(
        r#"
puzzle test {
dimension = 3
tags {
time = B F
}
layers {
actor = TEN:time:horizontal
}
rules {
input [ TEN:*:* | TEN:*:* ] -> [ TEN:*:> | TEN:*:> ]
}
}

levels demo of test {
legend {
B = TEN:B:left
F = TEN:F:left
}
level "pair" { BF }
}
"#,
    );

    let right = game
        .inputs
        .iter()
        .find(|input| input.name == "right")
        .expect("right input exists")
        .id;
    let ten_b_right = game
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "TEN:B:right").then_some(*object))
        .expect("TEN:B:right exists");
    let ten_f_right = game
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "TEN:F:right").then_some(*object))
        .expect("TEN:F:right exists");

    let next = transition_state(&game.game, &game.levels[0].initial_state, right)
        .expect("relative selector transition succeeds");

    assert!(next.has_object_at(&game.game, Coord3::new(0, 0, 0), ten_b_right));
    assert!(next.has_object_at(&game.game, Coord3::new(1, 0, 0), ten_f_right));
}

#[test]
fn relative_selector_rewrite_sets_fixed_tag_and_projects_relative_tag() {
    let (game, _) = parse_spatial(
        r#"
puzzle test {
dimension = 3
tags {
time = B F
}
layers {
actor = TEN:time:horizontal
}
rules {
input [ TEN:*:* ] -> [ TEN:F:> ]
}
}

levels demo of test {
legend {
B = TEN:B:left
}
level "fixed-and-relative" { B }
}
"#,
    );

    let right = game
        .inputs
        .iter()
        .find(|input| input.name == "right")
        .expect("right input exists")
        .id;
    let ten_f_right = game
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "TEN:F:right").then_some(*object))
        .expect("TEN:F:right exists");

    let next = transition_state(&game.game, &game.levels[0].initial_state, right)
        .expect("relative selector transition succeeds");

    assert!(next.has_object_at(&game.game, Coord3::new(0, 0, 0), ten_f_right));
}

#[test]
fn relative_selector_distinguishes_fixed_sibling_tags_for_push_and_pull() {
    let (game, _) = parse_spatial(
        r#"
puzzle test {
dimension = 3
tags {
time = B F
}
layers {
solid = TEN:time:horizontal Crate
}
rules {
input [ TEN:*:* ] -> [ > TEN:*:* ]
[ > TEN:*:* ] -> [ > TEN:*:> ]
[ > TEN:F:> | Crate ] -> [ > TEN:F:> | > Crate ]
[ < TEN:B:< | Crate ] -> [ < TEN:B:< | < Crate ]
[ > solid | no solid ] -> [ | solid ]
}
}

levels demo of test {
legend {
B = TEN:B:right
F = TEN:F:right
C = Crate
. = empty
}
level "fixed-sibling-tag" { .CB..BC..FC. }
}
"#,
    );

    let right = game
        .inputs
        .iter()
        .find(|input| input.name == "right")
        .expect("right input exists")
        .id;
    let ten_b_right = game
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "TEN:B:right").then_some(*object))
        .expect("TEN:B:right exists");
    let ten_f_right = game
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "TEN:F:right").then_some(*object))
        .expect("TEN:F:right exists");
    let crate_object = game
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "Crate").then_some(*object))
        .expect("Crate exists");

    let next = transition_state(&game.game, &game.levels[0].initial_state, right)
        .expect("movement transition succeeds");

    assert!(next.has_object_at(&game.game, Coord3::new(2, 0, 0), crate_object));
    assert!(next.has_object_at(&game.game, Coord3::new(3, 0, 0), ten_b_right));
    assert!(next.has_object_at(&game.game, Coord3::new(5, 0, 0), ten_b_right));
    assert!(next.has_object_at(&game.game, Coord3::new(6, 0, 0), crate_object));
    assert!(next.has_object_at(&game.game, Coord3::new(10, 0, 0), ten_f_right));
    assert!(next.has_object_at(&game.game, Coord3::new(11, 0, 0), crate_object));
    assert!(!next.has_object_at(&game.game, Coord3::new(1, 0, 0), crate_object));
}

#[test]
fn cartesian_plane_set_is_shared_by_schema_and_orientation_lowering() {
    let (game, _) = parse_spatial_body(
        r#"
layers {
actor = Player
marker = Marker:yz_plane
}

rules {
input yz_plane [ Player ] -> [ > Player ]
}
"#,
    );

    for name in ["up", "down", "front", "back"] {
        assert!(
            game.object_labels
                .values()
                .any(|label| label == &format!("Marker:{name}")),
            "missing schema variant Marker:{name}"
        );
        let input = game
            .inputs
            .iter()
            .find(|input| input.name == name)
            .unwrap_or_else(|| panic!("missing input {name}"));
        assert!(
            flattened_rules(game.game.program())
                .into_iter()
                .any(|rule| rule.guards == vec![GridGuard::<3>::InputIs(input.id)])
        );
    }
}

#[test]
fn unavailable_cartesian_direction_sets_fail_in_two_dimensions() {
    let error = parse_game_for_path(
        r#"
puzzle test {
dimension = 2
layers {
actor = Player
}
rules {
z_axis [ Player ] -> [ Player ]
}
}
"#,
        "test.puzzle",
    )
    .expect_err("z_axis is not available in two dimensions")
    .to_string();

    assert!(error.contains("unknown orientation: z_axis"), "{error}");
}

#[test]
fn cartesian_direction_set_names_cannot_be_redefined() {
    let error = parse_game_for_path(
        r#"
puzzle test {
dimension = 2
tags {
z_axis = left right
}
layers {
actor = Player
}
rules {
}
}
"#,
        "test.puzzle",
    )
    .expect_err("cartesian direction set names are built in")
    .to_string();

    assert!(
        error.contains("built-in tag set cannot be redefined"),
        "{error}"
    );
}

#[test]
fn relative_selector_rejects_nominal_tag_set() {
    let error = parse_game_for_path(
        r#"
puzzle test {
dimension = 3
tags {
mood = calm alert
}
layers {
actor = Token:mood
}
rules {
right [ Token:> ] -> [ Token:> ]
}
}
"#,
        "test.puzzle",
    )
    .expect_err("relative selectors require a direction-typed tag set")
    .to_string();

    assert!(
        error.contains("relative direction selector tag requires a direction-typed tag slot"),
        "{error}"
    );
}

#[test]
fn comma_separates_frame_axes_and_colon_does_not() {
    let (game, _) = parse_spatial_body(
        r#"
layers {
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
layers { Player }
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
layers {
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
fn spatial_visual_materialization_derives_from_shared_visuals() {
    let (game, presentation) = parse_spatial_body(
        r##"
layers {
floor = Floor
}
rules {
}

visuals basic {
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

    let visuals = presentation.visual_set.as_ref().expect("visual set exists");
    let floor = visuals.visual("Floor").expect("Floor visual exists");
    assert_eq!(
        floor.palette.get(&'0'),
        Some(&VoxelColor::Hex("#90ee90".to_string()))
    );
    assert_eq!(floor.palette.get(&'2'), Some(&VoxelColor::Transparent));
    assert_eq!(floor.first_frame().size, Size3::new(5, 3, 2));
    assert_eq!(game.visuals.entries.len(), 1);
}

#[test]
fn visual_fixture_contains_no_session_runtime_model() {
    let (game, presentation) = parse_spatial(
        r#"
puzzle board {
dimension = 3
layers { Player }
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
    let fixture: serde_json::Value = serde_json::from_str(&fixture).expect("fixture decodes");

    assert_eq!(
        fixture["size"],
        serde_json::json!({"width": 1, "depth": 1, "height": 1})
    );
    assert!(fixture["cells"].is_array());
    assert!(fixture.get("runtimeContract").is_none());
    assert!(fixture.get("runtimeContractVersion").is_none());
}

#[test]
fn inline_spatial_rows_can_span_multiple_cells_and_levels() {
    let (game, _) = parse_spatial(
        r#"
puzzle board {
dimension = 3
layers { Player Goal }
rules {
}
}
levels demo of board {
legend {
P = Player
G = Goal
}
level "one" { PG }
level "two" { PG }
}
"#,
    );

    assert_eq!(game.levels.len(), 2);
    assert_eq!(game.levels[0].initial_state.size, Size3::new(2, 1, 1));
    assert_eq!(game.levels[1].initial_state.size, Size3::new(2, 1, 1));
}

#[test]
fn starter_06_3d_compiles_with_its_level_clear_surface_flow() {
    parse_game_for_path(
        include_str!("../../html_editor/starter/06-3d.puzzle"),
        "starter/06-3d.puzzle",
    )
    .expect("06-3d starter should compile");
}
