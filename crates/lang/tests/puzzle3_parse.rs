use puzzle_grid3d::*;
use puzzle_lang::*;
use puzzle_runtime_contract::*;

const PLAYER: ObjectId = ObjectId(1);
const INPUT_LEFT: InputId = InputId(0);
const INPUT_RIGHT: InputId = InputId(1);
const INPUT_UP: InputId = InputId(2);
const INPUT_FORWARD: InputId = InputId(4);
const INPUT_BACKWARD: InputId = InputId(5);
const MICROBAN_FLOOR: ObjectId = ObjectId(1);
const MICROBAN_GOAL: ObjectId = ObjectId(2);
const MICROBAN_PLAYER: ObjectId = ObjectId(3);
const MICROBAN_BOX: ObjectId = ObjectId(4);
const MICROBAN_WALL: ObjectId = ObjectId(5);

fn spec_3d_model_source() -> String {
    let source = include_str!("../../../games/spec_3d.puzzle3");
    [
        source_block(source, "puzzle3 sokoban").as_str(),
        source_block(source, "levels3 microban").as_str(),
        source_block(source, "sprites3 basic").as_str(),
    ]
    .join("\n\n")
}

fn next_level_effect3() -> LifecycleCommand {
    LifecycleCommand::PuzzleNextLevel {
        target: String::new(),
    }
}

fn conditional_win_next_level_effect3() -> LifecycleCommand {
    LifecycleCommand::Conditional {
        condition: puzzle_scene::SceneExpr::Path(vec!["win_conditions".to_string()]),
        effect: Box::new(next_level_effect3()),
    }
}

fn source_block(source: &str, marker: &str) -> String {
    let start = source.find(marker).expect("fixture block marker exists");
    let open = source[start..]
        .find('{')
        .map(|index| start + index)
        .expect("fixture block opens");
    let mut depth = 0_i32;
    for (index, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return source[start..=open + index].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("fixture block closes");
}

fn object_id(parsed: &ParsedPuzzle3, name: &str) -> ObjectId {
    parsed
        .catalog
        .objects
        .iter()
        .find_map(|object| (object.name == name).then_some(object.id))
        .unwrap_or_else(|| panic!("missing object {name}"))
}

fn occupied_cells(state: &State3) -> Vec<(Coord3, Vec<ObjectId>)> {
    let mut cells = Vec::new();
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let position = Coord3 { x, y, z };
                let objects = state
                    .cell_view(position)
                    .expect("scan only visits positions inside the state")
                    .objects;
                if !objects.is_empty() {
                    cells.push((position, objects));
                }
            }
        }
    }
    cells
}

#[test]
fn display_objects_are_carried_by_parsed_puzzle3_not_game3() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 display3 {
layers {
actor = Player
fx = @Dust
}

rules {
input right [ Player | no Player ] -> [ | Player ]
}
}
"#,
    )
    .unwrap();

    let dust = object_id(&parsed, "@Dust");
    assert_eq!(parsed.display_objects, vec![dust]);
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestVoxelSprite3 {
    name: &'static str,
    size: Size3,
    palette: Vec<&'static str>,
    layers: Vec<Vec<&'static str>>,
}

impl TestVoxelSprite3 {
    fn filled_voxel_count(&self) -> usize {
        self.layers
            .iter()
            .flatten()
            .flat_map(|row| row.chars())
            .filter(|ch| *ch != '.' && *ch != ' ')
            .count()
    }
}

fn bottom_voxel_sprite(
    name: &'static str,
    palette: &[&'static str],
    bottom: &[&'static str],
) -> TestVoxelSprite3 {
    assert_eq!(bottom.len(), 5);
    assert!(bottom.iter().all(|row| row.chars().count() == 5));

    let mut layers = vec![bottom.to_vec()];
    for _ in 1..5 {
        layers.push(vec!["....."; 5]);
    }
    TestVoxelSprite3 {
        name,
        size: Size3::new(5, 5, 5),
        palette: palette.to_vec(),
        layers,
    }
}

fn microban_basic_sprites() -> Vec<TestVoxelSprite3> {
    vec![
        bottom_voxel_sprite(
            "@Floor",
            &["#90ee90", "#008000"],
            &["11111", "01111", "11101", "11111", "10111"],
        ),
        bottom_voxel_sprite(
            "Wall",
            &["#a46322", "#493c2b"],
            &["00010", "11111", "01000", "11111", "00010"],
        ),
        bottom_voxel_sprite(
            "Goal",
            &["#00008b"],
            &[".....", ".000.", ".0.0.", ".000.", "....."],
        ),
        bottom_voxel_sprite(
            "Box",
            &["#ffa500", "#ffff00"],
            &["00000", "0...0", "0...0", "0...0", "00000"],
        ),
        bottom_voxel_sprite(
            "Player",
            &["#000000", "#ffa500", "#ffffff", "#0000ff"],
            &[".000.", ".111.", "22222", ".333.", ".3.3."],
        ),
    ]
}

fn microban_basic_model() -> ParsedPuzzle3 {
    parse_puzzle3d(
        r#"
layers {
floor
target
solid
}

objects {
@Floor floor
Goal target
Player solid
Box solid
Wall solid
}

group solid = Player Box Wall

rules {
horizontal [ Player | Box | no solid ] -> [ | Player | Box ]
horizontal [ Player | no solid ] -> [ | Player ]
}
"#,
    )
    .unwrap()
}

fn microban_basic_rules_with_input_guards(rules: &[Rule3]) -> Vec<Rule3> {
    rules
        .iter()
        .cloned()
        .map(|rule| {
            let direction = rule.pattern.cells[1].offset;
            rule.when_input(input_for_microban_offset(direction))
        })
        .collect()
}

fn input_for_microban_offset(offset: Offset3) -> InputId {
    if offset == Direction3::LEFT.offset {
        INPUT_LEFT
    } else if offset == Direction3::RIGHT.offset {
        INPUT_RIGHT
    } else if offset == Direction3::FORWARD.offset {
        INPUT_FORWARD
    } else if offset == Direction3::BACKWARD.offset {
        INPUT_BACKWARD
    } else {
        panic!("Microban Basic only uses horizontal movement, got {offset:?}");
    }
}

fn microban_basic_01_level() -> Level3 {
    microban_basic_level_from_rows(&[
        "####..", "#.G#..", "#..###", "#*P..#", "#..B.#", "#..###", "####..",
    ])
}

fn microban_basic_level_from_rows(rows: &[&str]) -> Level3 {
    let depth = rows.len() as u16;
    let width = rows
        .first()
        .expect("Microban fixture has rows")
        .chars()
        .count() as u16;
    let mut cells = Vec::new();

    for (y, row) in rows.iter().enumerate() {
        assert_eq!(row.chars().count(), usize::from(width));
        for (x, ch) in row.chars().enumerate() {
            let mut objects = vec![MICROBAN_FLOOR];
            match ch {
                '.' => {}
                'G' => objects.push(MICROBAN_GOAL),
                'P' => objects.push(MICROBAN_PLAYER),
                'B' => objects.push(MICROBAN_BOX),
                '#' => objects.push(MICROBAN_WALL),
                '*' => objects.extend([MICROBAN_GOAL, MICROBAN_BOX]),
                '+' => objects.extend([MICROBAN_GOAL, MICROBAN_PLAYER]),
                _ => panic!("unknown Microban Basic cell: {ch}"),
            }
            cells.push(LevelCell3::new(Coord3::new(x as u16, y as u16, 0), objects));
        }
    }

    Level3::new(Size3::new(width, depth, 1), cells)
}

#[test]
fn parser_lowers_minimal_line_rule_with_direction_set_sugar() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor
}

objects {
Player actor
Box actor
Wall actor
}

group solid = Player Box Wall

rules {
horizontal [ Player | no solid ] -> [ | Player ]
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.game.layer_count, 1);
    assert_eq!(parsed.game.objects.len(), 3);
    assert_eq!(parsed.rules.len(), 4);
    assert_eq!(
        parsed.rules[1].pattern.cells[1].offset,
        Direction3::RIGHT.offset
    );
    assert_eq!(
        parsed.rules[1].writes,
        vec![WriteOp3::Move {
            component: 0,
            from_offset: Offset3::ZERO,
            to_offset: Direction3::RIGHT.offset,
            object: ObjectId(1),
        }]
    );
}

#[test]
fn parser_collects_on_level_start_rules_through_shared_block_body() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor
}

objects {
Player actor
Wall actor
}

group solid = Player Wall

on_level_start {
right [ Player
| no solid ]
-> [
| Player ]
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.lifecycle.on_level_start.len(), 1);
    assert_eq!(
        parsed.lifecycle.on_level_start[0].pattern.cells[1].offset,
        Direction3::RIGHT.offset
    );
}

#[test]
fn parser_lowers_3d_line_gap_rules_against_level_extent() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 gap3 {
layers {
floor = Goal
actor = Player
}

rules {
right [ Player | ... | Goal ] -> [ | ... | Player Goal ]
}
}

levels3 basic of gap3 {
legend {
. = empty
P = Player
G = Goal
}

level "start" {
P..G
}
}
"#,
    )
    .unwrap();
    let game = &parsed.game;
    let bundle = parsed.level_bundle.as_ref().unwrap();
    let state = bundle.build_level_state(0).unwrap();

    let next = transition_program(game, &state, &parsed.rules, InputId(0)).unwrap();

    assert!(!next.has_object(game, Coord3::new(0, 0, 0), ObjectId(2)));
    assert!(next.has_object(game, Coord3::new(3, 0, 0), ObjectId(2)));
}

#[test]
fn parser_keeps_render_settings_as_model_owned_display_state() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 camera_test {
render {
      camera {
yaw = 90
pitch = 42
zoom = 1.25
interactive_look = true
interactive_zoom = true
      }
      grid {
type = "occupied_cells"
      }
      pixelate {
enabled = true
scale = 4
smoothing = true
      }
      shade = true
    }

layers {
actor
}

objects {
Player actor
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.settings.camera.yaw_degrees, 90);
    assert_eq!(parsed.settings.camera.pitch_degrees, 42);
    assert_eq!(parsed.settings.camera.zoom_milli, 1250);
    assert!(parsed.settings.camera.interactive_look);
    assert!(parsed.settings.camera.interactive_zoom);
    assert!(parsed.settings.grid.occupied_cells);
    assert!(parsed.settings.sprite.shade);
    assert!(parsed.settings.pixelate.enabled);
    assert_eq!(parsed.settings.pixelate.scale, 4);
    assert!(parsed.settings.pixelate.smoothing);
}

#[test]
fn parser_accepts_boolean_render_setting_assignments() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 camera_test {
render {
  camera {
interactive_look = true
  }
  pixelate {
enabled = true
smoothing = false
  }
  shade = false
}

layers {
actor
}

objects {
Player actor
}
}
"#,
    )
    .unwrap();

    assert!(parsed.settings.camera.interactive_look);
    assert!(parsed.settings.pixelate.enabled);
    assert!(!parsed.settings.pixelate.smoothing);
    assert!(!parsed.settings.sprite.shade);
}

#[test]
fn parser_rejects_old_inline_camera_render_settings() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 camera_test {
render {
  camera yaw=90
}

layers {
actor
}

objects {
Player actor
}
}
"#,
    );

    assert!(parsed.is_err());
}

#[test]
fn parser_rejects_old_bare_grid_render_settings() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 camera_test {
render {
  grid {
occupied_cells
  }
}

layers {
actor
}

objects {
Player actor
}
}
"#,
    );

    assert!(parsed.is_err());
}

#[test]
fn parser_rejects_old_inline_grid_render_settings() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 camera_test {
render {
  grid occupied_cells
}

layers {
actor
}

objects {
Player actor
}
}
"#,
    );

    assert!(parsed.is_err());
}

#[test]
fn parser_rejects_old_inline_pixelate_render_settings() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 camera_test {
render {
  pixelate scale=4
}

layers {
actor
}

objects {
Player actor
}
}
"#,
    );

    assert!(parsed.is_err());
}

#[test]
fn parser_rejects_old_bare_render_settings() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 camera_test {
render {
  shade
}

layers {
actor
}

objects {
Player actor
}
}
"#,
    );

    assert!(parsed.is_err());
}

#[test]
fn parser_rejects_old_camera_setting_value_syntax() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 camera_test {
render {
  camera {
yaw 90
  }
}

layers {
actor
}

objects {
Player actor
}
}
"#,
    );

    assert!(parsed.is_err());
}

#[test]
fn parser_defaults_render_settings() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor
}

objects {
Player actor
}
"#,
    )
    .unwrap();

    assert!(!parsed.settings.camera.interactive_look);
    assert!(!parsed.settings.camera.interactive_zoom);
    assert_eq!(parsed.settings.camera.yaw_degrees, 34);
    assert_eq!(parsed.settings.camera.pitch_degrees, 38);
    assert_eq!(parsed.settings.camera.zoom_milli, 1100);
    assert!(!parsed.settings.grid.occupied_cells);
    assert!(parsed.settings.sprite.shade);
    assert!(!parsed.settings.pixelate.enabled);
    assert_eq!(parsed.settings.pixelate.scale, 4);
    assert!(parsed.settings.pixelate.smoothing);
}

#[test]
fn parser_defaults_3d_zoomscreen_height_to_full() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 viewport_test {
render {
  viewport {
zoomscreen 7 5
focus Player
  }
}

layers {
actor
}

objects {
Player actor
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.settings.viewport.mode, ViewportMode3::Centered);
    assert_eq!(parsed.settings.viewport.follow, ViewportFollow3::Snap);
    assert_eq!(parsed.settings.viewport.focus, "Player");
    assert_eq!(
        parsed.settings.viewport.framing,
        Some(ViewportFraming3 {
            width: 7,
            depth: 5,
            height: ViewportHeight3::Full,
        })
    );
}

#[test]
fn parser_lowers_3d_local_radius_to_cubic_local_frame() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 local_radius_test {
layers {
actor
}

objects {
Player actor
Box actor
}

rules local_radius 6 {
right [ Player | Box ] -> [ | Player ]
}
}
"#,
    )
    .unwrap();

    let frame = parsed.local_frame.unwrap();
    assert_eq!(frame.x, LocalFrameExtent::Radius(6));
    assert_eq!(frame.y, LocalFrameExtent::Radius(6));
    assert_eq!(frame.z, LocalFrameExtent::Radius(6));
    assert_eq!(frame.focus_objects, vec![PLAYER]);
}

#[test]
fn parser_keeps_smoothscreen_as_smooth_centered_viewport() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 viewport_test {
render {
  viewport {
smoothscreen 9 7 3
focus actor
  }
}

layers {
actor
}

objects {
Player actor
Box actor
}

group actor = Player Box
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.settings.viewport.mode, ViewportMode3::Centered);
    assert_eq!(parsed.settings.viewport.follow, ViewportFollow3::Smooth);
    assert_eq!(parsed.settings.viewport.focus, "actor");
    assert_eq!(
        parsed.settings.viewport.framing,
        Some(ViewportFraming3 {
            width: 9,
            depth: 7,
            height: ViewportHeight3::Size(3),
        })
    );
}

#[test]
fn parser_keeps_flickscreen_as_paged_viewport() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 viewport_test {
render {
  viewport {
flickscreen 9 7 2
focus Player
  }
}

layers {
actor
}

objects {
Player actor
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.settings.viewport.mode, ViewportMode3::Paged);
    assert_eq!(parsed.settings.viewport.follow, ViewportFollow3::Snap);
    assert_eq!(parsed.settings.viewport.focus, "Player");
    assert_eq!(
        parsed.settings.viewport.framing,
        Some(ViewportFraming3 {
            width: 9,
            depth: 7,
            height: ViewportHeight3::Size(2),
        })
    );
}

#[test]
fn visual_fixture_exports_3d_viewport_contract() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 viewport_test {
render {
  viewport {
smoothscreen 7 7
focus actor
  }
}

layers {
actor
floor
}

objects {
Player actor
Box actor
Floor floor
}

group actor = Player Box
}

levels3 test of viewport_test {
legend {
. = empty
P = Player
}

level "one" {
P
}
}
"#,
    )
    .unwrap();
    let fixture = export_visual_fixture_json(&parsed).unwrap();

    assert!(fixture.contains(
        r#""viewport": { "mode": "centered", "follow": "smooth", "focus": "actor", "focusObjects": [1, 2], "framingBox": { "width": 7, "depth": 7, "height": "full" } }"#
    ));
}

#[test]
fn visual_fixture_does_not_assign_implicit_sprites() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 no_sprites {
layers {
actor
}

objects {
Player actor
}
}

levels3 test of no_sprites {
legend {
P = Player
}

level "one" {
P
}
}
"#,
    )
    .unwrap();
    let fixture = export_visual_fixture_json(&parsed).unwrap();

    assert!(fixture.contains(r#""Player": { "id": 1, "name": "Player", "sprite": null"#));
    assert!(fixture.contains(r#"{ "id": 1, "name": "Player", "sprite": null }"#));
}

#[test]
fn parser_rejects_legacy_top_level_camera_settings() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 camera_test {
debug_camera = true
camera_yaw = 90
camera_pitch = 42
camera_zoom = 1.25

layers {
actor
}

objects {
Player actor
}
}
"#,
    );

    assert!(parsed.is_err());
}

#[test]
fn parser_lowers_input_guarded_direction_set_rule() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor = Player Box Wall
}

group solid = Player Box Wall

rules {
input horizontal [ Player | no solid ] -> [ | Player ]
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.rules.len(), 4);
    assert_eq!(parsed.rules[0].guards, vec![Guard3::InputIs(INPUT_LEFT)]);
    assert_eq!(parsed.rules[1].guards, vec![Guard3::InputIs(INPUT_RIGHT)]);
    assert_eq!(parsed.rules[2].guards, vec![Guard3::InputIs(INPUT_FORWARD)]);
    assert_eq!(
        parsed.rules[3].guards,
        vec![Guard3::InputIs(INPUT_BACKWARD)]
    );
}

#[test]
fn parser_lowers_input_rule_without_set_as_directions_sugar() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor = Player Wall
}

group solid = Player Wall

rules {
input [ Player | no solid ] -> [ | Player ]
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.rules.len(), 6);
    let guards = parsed
        .rules
        .iter()
        .map(|rule| rule.guards.as_slice())
        .collect::<Vec<_>>();
    for input in [
        INPUT_LEFT,
        INPUT_RIGHT,
        INPUT_UP,
        InputId(3),
        INPUT_FORWARD,
        INPUT_BACKWARD,
    ] {
        assert!(guards.contains(&[Guard3::InputIs(input)].as_slice()));
    }
}

#[test]
fn parser_expands_3d_horizontal_and_vertical_movement_mark_sets() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor = Box
}

rules {
right [ Box{horizontal} ] -> [ Box{vertical} ]
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.rules.len(), 8);
}

#[test]
fn parser_lowers_input_rule_with_forward_marker_rhs_sugar_as_movement_mark() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor = Player
}

rules {
input [ Player ] -> [ > Player ]
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.rules.len(), 6);
    assert!(parsed.rules.iter().any(|rule| {
        rule.guards == vec![Guard3::InputIs(INPUT_RIGHT)]
            && rule.writes
                == vec![WriteOp3::SetMark {
                    component: 0,
                    object: PLAYER,
                    offset: Offset3::ZERO,
                    mark: MarkId3(0),
                    value: Some(3),
                }]
    }));
}

#[test]
fn parser_lowers_standard_move_step_for_3d_movement_mark() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 push3 {
layers {
actor = Player
}

rules {
input [ Player ] -> [ > Player ]
move
}
}

levels3 demo of push3 {
legend {
. = empty
P = Player
}

level "start" {
P.
}
}
"#,
    )
    .unwrap();
    let level_bundle = parsed.level_bundle.as_ref().unwrap();
    let initial = level_bundle
        .level(0)
        .unwrap()
        .level
        .build_state(&parsed.game)
        .unwrap();

    let moved = transition_program(&parsed.game, &initial, &parsed.rules, INPUT_RIGHT).unwrap();
    let player = object_id(&parsed, "Player");

    assert!(!moved.has_object(&parsed.game, Coord3::new(0, 0, 0), player));
    assert!(moved.has_object(&parsed.game, Coord3::new(1, 0, 0), player));
    assert!(!moved.has_mark(&parsed.game, Coord3::new(1, 0, 0), player, MarkId3(0), None));
}

#[test]
fn neutral_3d_rewrite_expands_relative_movement_mark_by_direction() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 push3 {
layers {
actor = Player Box
}

rules {
input [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
move
}
}

levels3 demo of push3 {
legend {
. = empty
P = Player
B = Box
}

level "start" {
PB.
}
}
"#,
    )
    .unwrap();
    let initial = parsed
        .level_bundle
        .as_ref()
        .unwrap()
        .level(0)
        .unwrap()
        .level
        .build_state(&parsed.game)
        .unwrap();

    let moved = transition_program(&parsed.game, &initial, &parsed.rules, INPUT_RIGHT).unwrap();
    let player = object_id(&parsed, "Player");
    let box_object = object_id(&parsed, "Box");

    assert!(!moved.has_object(&parsed.game, Coord3::new(0, 0, 0), player));
    assert!(moved.has_object(&parsed.game, Coord3::new(1, 0, 0), player));
    assert!(moved.has_object(&parsed.game, Coord3::new(2, 0, 0), box_object));
}

#[test]
fn parser_accepts_shared_application_prefixed_3d_rule_surface() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 surface3 {
layers {
actor = Player Box
}

rules {
once [ Player ] -> [ Player ]
once_all [ > Player | Box ] -> [ > Player | > Box ]
once_per_level input [ Player ] -> [ > Player ]
random [ Player ] -> [ Player ]
repeat right [ Box | ] -> [ | Box ]
}
}
"#,
    )
    .unwrap();

    assert!(
        parsed
            .rules
            .iter()
            .any(|rule| rule.application == RuleApplication3::Once)
    );
    assert!(
        parsed
            .rules
            .iter()
            .any(|rule| rule.application == RuleApplication3::OnceAll)
    );
    assert!(
        parsed
            .rules
            .iter()
            .any(|rule| rule.application == RuleApplication3::OncePerLevel)
    );
    assert!(
        parsed
            .rules
            .iter()
            .any(|rule| rule.application == RuleApplication3::Random)
    );
    assert!(
        parsed
            .rules
            .iter()
            .any(|rule| rule.application == RuleApplication3::UntilStable)
    );
}

#[test]
fn parser_rejects_shared_statement_surfaces_without_3d_lowering() {
    let routine_call = parse_puzzle3d(
        r#"
puzzle3 no_routine_calls3 {
layers {
actor = Player
}

rules {
push_boxes
}
}
"#,
    )
    .unwrap_err();
    assert!(
        matches!(
            routine_call,
            ParseError3::Message(ref message)
                if message.contains("3D rule blocks do not support routine calls")
        ),
        "{routine_call:?}"
    );

    let application_block = parse_puzzle3d(
        r#"
puzzle3 no_application_blocks3 {
layers {
actor = Player
}

rules {
once
}
}
"#,
    )
    .unwrap_err();
    assert!(
        matches!(
            application_block,
            ParseError3::Message(ref message)
                if message.contains("3D rule blocks do not support nested application blocks")
        ),
        "{application_block:?}"
    );
}

#[test]
fn standard_move_step_blocks_same_layer_destination_in_3d() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 push3 {
layers {
actor = Player Wall
}

rules {
input [ Player ] -> [ > Player ]
move
}
}

levels3 demo of push3 {
legend {
. = empty
P = Player
W = Wall
}

level "start" {
PW
}
}
"#,
    )
    .unwrap();
    let initial = parsed
        .level_bundle
        .as_ref()
        .unwrap()
        .level(0)
        .unwrap()
        .level
        .build_state(&parsed.game)
        .unwrap();

    let blocked = transition_program(&parsed.game, &initial, &parsed.rules, INPUT_RIGHT).unwrap();
    let player = object_id(&parsed, "Player");
    let wall = object_id(&parsed, "Wall");

    assert!(blocked.has_object(&parsed.game, Coord3::new(0, 0, 0), player));
    assert!(blocked.has_object(&parsed.game, Coord3::new(1, 0, 0), wall));
}

#[test]
fn standard_move_step_moves_same_direction_3d_chains_one_cell() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 push3 {
layers {
actor = Box
}

rules {
right [ Box | Box ] -> [ > Box | > Box ]
move
}
}

levels3 demo of push3 {
legend {
. = empty
B = Box
}

level "start" {
BB..
}
}
"#,
    )
    .unwrap();
    let initial = parsed
        .level_bundle
        .as_ref()
        .unwrap()
        .level(0)
        .unwrap()
        .level
        .build_state(&parsed.game)
        .unwrap();

    let moved = transition_program(&parsed.game, &initial, &parsed.rules, INPUT_RIGHT).unwrap();
    let box_object = object_id(&parsed, "Box");

    assert!(!moved.has_object(&parsed.game, Coord3::new(0, 0, 0), box_object));
    assert!(moved.has_object(&parsed.game, Coord3::new(1, 0, 0), box_object));
    assert!(moved.has_object(&parsed.game, Coord3::new(2, 0, 0), box_object));
    assert!(!moved.has_object(&parsed.game, Coord3::new(3, 0, 0), box_object));
}

#[test]
fn parser_lowers_camera_variable_effects_from_rules() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor = Player Wall
}

rules {
right [ Player | no Wall ] -> [ | Player ] set yaw = 100
set zoom = 1.5
reset_camera
}

levels3 test {
legend {
P = Player
}

level "one" {
P
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.rules.len(), 3);
    assert!(parsed.rules.iter().all(|rule| rule.effects.is_empty()));
    assert_eq!(
        parsed.rule_camera_effects[0],
        vec![Puzzle3CameraEffect::SetYaw(100)]
    );
    assert_eq!(
        parsed.rule_camera_effects[1],
        vec![Puzzle3CameraEffect::SetZoom(1500)]
    );
    assert!(parsed.rules[1].pattern.cells.is_empty());
    assert!(parsed.rules[1].writes.is_empty());
    assert_eq!(
        parsed.rule_camera_effects[2],
        vec![Puzzle3CameraEffect::Reset]
    );

    let fixture = export_visual_fixture_json(&parsed).unwrap();
    let contract =
        puzzle3_runtime_model_from_fixture_json(&fixture).expect("runtime contract decodes");
    assert!(contract.rules.iter().all(|rule| rule.effects.is_empty()));
    assert_eq!(contract.rule_camera_effects, parsed.rule_camera_effects);
}

#[test]
fn parser_lowers_variant_selector_assignment() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor
}

objects {
Marker:directions actor
}

rules {
directions [ Marker:* | ] -> [ | Marker:* ]
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.game.objects.len(), 6);
    assert_eq!(parsed.rules.len(), 36);
    assert!(parsed.rules.iter().all(|rule| {
        rule.pattern.cells[0].require_objects[0]
            == match rule.writes[0] {
                WriteOp3::Move { object, .. } => object,
                _ => ObjectId::EMPTY,
            }
    }));
}

#[test]
fn parser_lowers_bare_star_selector_assignment() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 bare_star {
layers {
actor
}

objects {
Player actor
Box actor
}

rules {
right [ * | no * ] -> [ | * ]
}
}

levels3 basic of bare_star {
legend {
. = empty
P = Player
B = Box
}

level "start" {
PB.
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.rules.len(), 2);
    let state = parsed
        .level_bundle
        .as_ref()
        .unwrap()
        .build_level_state(0)
        .unwrap();
    let next = transition_program_without_input(&parsed.game, &state, &parsed.rules).unwrap();

    assert!(next.has_object(&parsed.game, Coord3::new(0, 0, 0), ObjectId(1)));
    assert!(!next.has_object(&parsed.game, Coord3::new(1, 0, 0), ObjectId(2)));
    assert!(next.has_object(&parsed.game, Coord3::new(2, 0, 0), ObjectId(2)));
}

#[test]
fn parser_lowers_group_selector_to_runtime_object_set_matcher() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 group_move {
layers {
actor
}

objects {
Box actor
Crate actor
}

group solid = Box Crate

rules {
right [ solid | ] -> [ | solid ]
}
}

levels3 basic of group_move {
legend {
. = empty
B = Box
C = Crate
}

level "start" {
B.C
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.rules.len(), 1);
    let rule = &parsed.rules[0];
    assert!(rule.pattern.cells[0].require_objects.is_empty());
    assert_eq!(rule.pattern.cells[0].require_object_sets.len(), 1);
    assert_eq!(
        rule.pattern.cells[0].require_object_sets[0].objects,
        vec![ObjectId(1), ObjectId(2)]
    );
    assert!(matches!(
        rule.writes.as_slice(),
        [WriteOp3::MoveObjectSet { binding: 0, .. }]
    ));

    let state = parsed
        .level_bundle
        .as_ref()
        .unwrap()
        .build_level_state(0)
        .unwrap();
    let next = transition_program_without_input(&parsed.game, &state, &parsed.rules).unwrap();
    assert!(!next.has_object(&parsed.game, Coord3::new(0, 0, 0), ObjectId(1)));
    assert!(next.has_object(&parsed.game, Coord3::new(1, 0, 0), ObjectId(1)));
}

#[test]
fn parser_lowers_selector_occurrence_labels_for_group_swap() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor = Box Crate
}

group solid = Box Crate

rules {
right [ solid#1 | solid#2 ] -> [ solid#2 | solid#1 ]
}
"#,
    )
    .unwrap();

    assert!(parsed.rules.iter().any(|rule| {
        rule.pattern.cells[0].require_objects == vec![ObjectId(1)]
            && rule.pattern.cells[1].require_objects == vec![ObjectId(2)]
            && rule.writes.contains(&WriteOp3::Move {
                component: 0,
                from_offset: Offset3::ZERO,
                to_offset: Direction3::RIGHT.offset,
                object: ObjectId(1),
            })
            && rule.writes.contains(&WriteOp3::Move {
                component: 0,
                from_offset: Direction3::RIGHT.offset,
                to_offset: Offset3::ZERO,
                object: ObjectId(2),
            })
    }));
}

#[test]
fn parser_rejects_duplicate_selector_occurrence_labels() {
    let err = parse_puzzle3d(
        r#"
layers {
actor = Box Crate
}

group solid = Box Crate

rules {
right [ solid#1 | solid#1 ] -> [ solid#1 | solid#1 ]
}
"#,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        ParseError3::Message(message)
            if message.contains("DuplicateSelectorOccurrenceLabel")
    ));
}

#[test]
fn parser_lowers_dense_frame_rule() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor
}

objects {
Player actor
Box actor
}

rules {
right:up [ Player | Box ] -> [ | Player | Box ]
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.rules.len(), 1);
    assert_eq!(
        parsed.rules[0].pattern.cells[1].offset,
        Direction3::RIGHT.offset
    );
    assert_eq!(
        parsed.rules[0].writes[0],
        WriteOp3::Move {
            component: 0,
            from_offset: Offset3::ZERO,
            to_offset: Direction3::RIGHT.offset,
            object: ObjectId(1),
        }
    );
}

#[test]
fn parser_lowers_dense_frame_rule_with_pattern_line_breaks() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor
}

objects {
Player actor
Box actor
}

rules {
right:up [ Player
Box ] -> [ Player
Box ]
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.rules.len(), 1);
    assert_eq!(
        parsed.rules[0].pattern.cells[1].offset,
        Direction3::UP.offset
    );
}

#[test]
fn parser_accepts_inline_braced_blocks_with_semicolon_rows() {
    let parsed = parse_puzzle3d(
        r#"
layers { actor }
objects { Player actor; Box actor }
groups { solid = Box }
rules { right [ Player | no solid ] -> [ | Player ] }
"#,
    )
    .unwrap();

    assert_eq!(parsed.game.objects.len(), 2);
    assert_eq!(parsed.rules.len(), 1);
}

#[test]
fn parser_lowers_layers_legend_and_levels_to_level_bundle() {
    let parsed = parse_puzzle3d(
        r#"
layers {
floor = Goal
actor = Player Box Wall
}

group solid = Player Box Wall

rules {
horizontal [ Player | no solid ] -> [ | Player ]
}

levels3 {
legend {
P = Player
B = Box
# = Wall
G = Goal
* = Goal Box
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
    )
    .unwrap();

    assert_eq!(parsed.game.layer_count, 2);
    assert_eq!(parsed.game.objects.len(), 4);

    let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
    assert_eq!(bundle.level_count(), 1);
    assert_eq!(bundle.level(0).unwrap().name, "stacked");
    assert_eq!(bundle.level(0).unwrap().level.size, Size3::new(3, 3, 2));

    let state = bundle.build_level_state(0).unwrap();

    assert!(state.has_object(&bundle.game, Coord3::new(1, 1, 1), ObjectId(1)));
    assert!(state.has_object(&bundle.game, Coord3::new(1, 1, 0), ObjectId(2)));
    assert!(state.has_object(&bundle.game, Coord3::new(2, 1, 0), ObjectId(3)));
    assert!(state.has_object(&bundle.game, Coord3::new(0, 2, 0), ObjectId(4)));
}

#[test]
fn parser_rejects_unquoted_3d_level_header_names() {
    let err = parse_puzzle3d(
        r#"
layers {
actor = Player
}

levels3 {
legend {
P = Player
}

level old_style {
P
}
}
"#,
    )
    .unwrap_err();

    assert!(
        matches!(err, ParseError3::Message(message) if message.contains("level header must be: level \"<id>\""))
    );
}

#[test]
fn parser_uses_dot_as_default_empty_char_for_3d_levels() {
    let parsed = parse_puzzle3d(
        r#"
layers {
actor = Player
}

levels3 {
legend {
P = Player
}

level "default_dot" {
P.
}
}
"#,
    )
    .unwrap();

    let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
    let state = bundle.build_level_state(0).unwrap();

    assert!(state.has_object(&bundle.game, Coord3::new(0, 0, 0), ObjectId(1)));
    assert_eq!(
        state.cell_view(Coord3::new(1, 0, 0)).unwrap().objects,
        Vec::<ObjectId>::new()
    );
}

#[test]
fn parser_rejects_non_dot_empty_char_for_3d_levels() {
    let err = parse_puzzle3d(
        r#"
layers {
actor = Player
}

levels3 {
legend {
_ = empty
P = Player
}

level "override_empty" {
P.
}
}
"#,
    )
    .unwrap_err();

    assert!(
        matches!(err, ParseError3::Message(message) if message.contains("3D levels use `.` for empty"))
    );
}

#[test]
fn parser_lowers_canonical_sprites3_entries() {
    let parsed = parse_puzzle3d(
        r##"
layers {
floor = Floor
}

sprites3 basic {
Floor
#90ee90 #008000 transparent
.....
..1..
.....

00000
0...0
00000
}
"##,
    )
    .unwrap();

    let sprites = parsed.sprite_set.as_ref().expect("sprite set exists");
    let floor = sprites.sprite("Floor").expect("Floor sprite exists");

    assert_eq!(sprites.name, "basic");
    assert_eq!(
        floor.palette.get(&'0'),
        Some(&SpriteColor3::Hex("#90ee90".to_string()))
    );
    assert_eq!(
        floor.palette.get(&'1'),
        Some(&SpriteColor3::Hex("#008000".to_string()))
    );
    assert_eq!(floor.palette.get(&'2'), Some(&SpriteColor3::Transparent));
    assert_eq!(floor.voxels.size, Size3::new(5, 3, 2));
}

#[test]
fn parser_lowers_canonical_sprites3_shape_refs() {
    let parsed = parse_puzzle3d(
        r##"
layers {
floor = Floor
}

sprites3 basic {
shape flat {
.....
..1..
.....

00000
0...0
00000
}

Floor
#90ee90 #008000
flat
}
"##,
    )
    .unwrap();

    let sprites = parsed.sprite_set.as_ref().expect("sprite set exists");
    let floor = sprites.sprite("Floor").expect("Floor sprite exists");

    assert_eq!(
        floor.palette.get(&'0'),
        Some(&SpriteColor3::Hex("#90ee90".to_string()))
    );
    assert_eq!(
        floor.palette.get(&'1'),
        Some(&SpriteColor3::Hex("#008000".to_string()))
    );
    assert_eq!(floor.voxels.size, Size3::new(5, 3, 2));
}

#[test]
fn parser_lowers_color_only_sprites3_entry_to_filled_cube() {
    let parsed = parse_puzzle3d(
        r##"
layers {
floor = Floor
target = Goal
}

sprites3 basic {
Floor
#90ee90

Goal
#00008b
}
"##,
    )
    .unwrap();

    let sprites = parsed.sprite_set.as_ref().expect("sprite set exists");
    let floor = sprites.sprite("Floor").expect("Floor sprite exists");
    let goal = sprites.sprite("Goal").expect("Goal sprite exists");

    assert_eq!(floor.voxels.size, Size3::new(1, 1, 1));
    assert_eq!(floor.voxels.slices.as_slice(), &[vec!["0".to_string()]]);
    assert_eq!(goal.voxels.size, Size3::new(1, 1, 1));
}

#[test]
fn parser_rejects_prefixed_sprites3_shape_refs() {
    let err = parse_puzzle3d(
        r##"
layers {
floor = Floor
}

sprites3 basic {
shape flat {
0
}

Floor
#90ee90
shape flat
}
"##,
    )
    .unwrap_err();

    assert!(
        matches!(err, ParseError3::Message(message) if message.contains("shape refs are bare"))
    );
}

#[test]
fn parser_rejects_legacy_sprites3_blocks() {
    let err = parse_puzzle3d(
        r##"
layers {
floor = Floor
}

sprites3 basic {
sprite Floor {
colors {
0 = #90ee90
}

voxels {
0
}
}
}
"##,
    )
    .unwrap_err();

    assert!(matches!(err, ParseError3::Message(message) if message.contains("canonical form")));
}

#[test]
fn parser_rejects_unknown_level_legend_char() {
    let err = parse_puzzle3d(
        r#"
layers {
actor = Player
}

levels3 {
legend {
. = empty
P = Player
}

level "bad" {
PX
}
}
"#,
    )
    .unwrap_err();

    assert!(
        matches!(err, ParseError3::Message(message) if message.contains("unknown legend char: X"))
    );
}

#[test]
fn parser_lowers_model_wrapped_win_conditions_and_named_level_pack() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 push3d {
layers {
floor = Goal
actor = Player Box
}

win_conditions {
some Goal
no down [ no Box | Goal ]
}
}

levels3 basic of push3d {
legend {
. = empty
G = Goal
B = Box
}

level "solved" {
...
.B.
...

...
.G.
...
}

level "unsolved" {
...
..B
...

...
.G.
...
}
}

"#,
    )
    .unwrap();

    let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
    let win = parsed.win_condition.as_ref().expect("win condition exists");

    assert_eq!(bundle.level_count(), 2);
    assert_eq!(
        parsed.level_packs,
        vec![Some("basic".to_string()), Some("basic".to_string())]
    );

    let solved = bundle.build_level_state(0).unwrap();
    let unsolved = bundle.build_level_state(1).unwrap();

    assert!(win.is_met(&bundle.game, &solved));
    assert!(!win.is_met(&bundle.game, &unsolved));
}

#[test]
fn parser_rejects_all_on_oriented_win_pattern() {
    let err = parse_puzzle3d(
        r#"
puzzle3 push3d {
layers {
floor = Goal
actor = Box
}

win_conditions {
some Goal
all Goal on down [ Box | Goal ]
}
}
"#,
    )
    .unwrap_err();

    assert!(
        matches!(err, ParseError3::Message(message) if message.contains("all <selector> on <pattern> is not valid"))
    );
}

#[test]
fn parser_accepts_function_style_3d_win_conditions() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 push3d {
layers {
floor = Goal
actor = Box
}

win_conditions {
exists(Goal)
none(down [ no Box | Goal ])
}
}

levels3 basic of push3d {
legend {
. = empty
G = Goal
B = Box
}

level "solved" {
...
.B.
...

...
.G.
...
}
}
"#,
    )
    .unwrap();

    let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
    let win = parsed.win_condition.as_ref().expect("win condition exists");
    let solved = bundle.build_level_state(0).unwrap();

    assert!(win.is_met(&bundle.game, &solved));
}

#[test]
fn parser_rejects_2d_model_keyword_in_3d_parser() {
    let err = parse_puzzle3d(
        r#"
puzzle push3d {
layers {
actor = Player
}
}
"#,
    )
    .unwrap_err();

    assert!(
        matches!(err, ParseError3::Message(message) if message.contains("unknown 3D puzzle directive"))
    );
}

#[test]
fn parser_accepts_last_level_clear_lifecycle() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 lifecycle {
layers {
actor = Player
}

on_level_clear {
next_level
}

on_last_level_clear {
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.lifecycle.on_level_clear, vec![next_level_effect3()]);
    assert_eq!(parsed.lifecycle.on_last_level_clear, Some(Vec::new()));
}

#[test]
fn parser_uses_shared_scene_effect_for_last_level_message() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 lifecycle {
layers {
actor = Player
}

on_last_level_clear {
message "END"
}
}
"#,
    )
    .unwrap();

    assert_eq!(
        parsed.lifecycle.on_last_level_clear,
        Some(vec![LifecycleCommand::Message {
            text: puzzle_scene::SceneExpr::Text("END".to_string())
        }])
    );
}

#[test]
fn spec_3d_recreates_microban_level_1() {
    let source = spec_3d_model_source();
    let parsed = parse_puzzle3d(&source).unwrap();
    let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
    let win = parsed.win_condition.as_ref().expect("win condition exists");

    assert_eq!(bundle.level_count(), 3);
    assert_eq!(bundle.level(0).unwrap().name, "microban 1");
    assert_eq!(bundle.level(1).unwrap().name, "microban 2");
    assert_eq!(bundle.level(2).unwrap().name, "microban 3");
    assert_eq!(bundle.level(0).unwrap().level.size, Size3::new(6, 7, 2));
    assert_eq!(bundle.level(1).unwrap().level.size, Size3::new(6, 7, 2));
    assert_eq!(bundle.level(2).unwrap().level.size, Size3::new(9, 6, 2));
    assert_eq!(
        parsed.lifecycle.on_level_clear,
        vec![conditional_win_next_level_effect3()]
    );
    let sprites = parsed.sprite_set.as_ref().expect("sprite set exists");
    assert_eq!(sprites.name, "basic");
    assert_eq!(sprites.model.as_deref(), Some("sokoban"));
    assert_eq!(sprites.sprites.len(), 5);
    assert_eq!(
        sprites.sprite("Floor").unwrap().voxels.size,
        Size3::new(5, 5, 5)
    );
    assert_eq!(
        sprites.sprite("Box").unwrap().voxels.size,
        Size3::new(5, 5, 5)
    );
    assert_eq!(
        sprites.sprite("Player").unwrap().voxels.size,
        Size3::new(5, 5, 5)
    );
    assert_eq!(
        sprites.sprite("Wall").unwrap().voxels.size,
        Size3::new(5, 5, 5)
    );
    let fixture_json = export_visual_fixture_json(&parsed).unwrap();
    assert!(fixture_json.contains("\"shade\": true"));
    let contract =
        puzzle3_runtime_model_from_fixture_json(&fixture_json).expect("runtime contract decodes");
    assert!(!contract.rules.is_empty());
    assert_eq!(
        contract.lifecycle.on_level_clear,
        vec![conditional_win_next_level_effect3()]
    );
    assert!(contract.win_condition.is_some());
    assert!(fixture_json.contains("\"Box\": {"));
    assert!(fixture_json.contains("\"bitmap\": ["));

    let initial = bundle.build_level_state(0).unwrap();
    let floor_cells = occupied_cells(&initial)
        .into_iter()
        .filter(|(position, objects)| position.z == 0 && objects.contains(&ObjectId(1)))
        .count();
    assert_eq!(floor_cells, 42);
    assert!(initial.has_object(&bundle.game, Coord3::new(0, 0, 0), ObjectId(1)));
    assert!(initial.has_object(&bundle.game, Coord3::new(5, 0, 0), ObjectId(1)));
    assert!(initial.has_object(&bundle.game, Coord3::new(2, 3, 0), ObjectId(1)));
    assert!(initial.has_object(&bundle.game, Coord3::new(2, 5, 0), ObjectId(2)));
    assert!(initial.has_object(&bundle.game, Coord3::new(1, 3, 0), ObjectId(2)));
    assert!(initial.has_object(&bundle.game, Coord3::new(2, 3, 1), ObjectId(3)));
    assert!(initial.has_object(&bundle.game, Coord3::new(1, 3, 1), ObjectId(4)));
    assert!(initial.has_object(&bundle.game, Coord3::new(3, 2, 1), ObjectId(4)));
    assert!(!win.is_met(&bundle.game, &initial));

    let second_initial = bundle.build_level_state(1).unwrap();
    assert!(second_initial.has_object(&bundle.game, Coord3::new(3, 4, 1), ObjectId(3)));
    assert!(second_initial.has_object(&bundle.game, Coord3::new(2, 3, 1), ObjectId(4)));
    assert!(second_initial.has_object(&bundle.game, Coord3::new(3, 3, 1), ObjectId(4)));
    assert!(second_initial.has_object(&bundle.game, Coord3::new(3, 2, 1), ObjectId(4)));
    assert!(second_initial.has_object(&bundle.game, Coord3::new(3, 3, 0), ObjectId(2)));
    assert!(second_initial.has_object(&bundle.game, Coord3::new(2, 2, 0), ObjectId(2)));
    assert!(second_initial.has_object(&bundle.game, Coord3::new(3, 2, 0), ObjectId(2)));

    assert!(!parsed.rules.is_empty());
}

#[test]
fn spec_3d_sokoban_can_be_authored_from_puzzle_file() {
    let source = spec_3d_model_source();
    let parsed = parse_puzzle3d(&source).unwrap();
    let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
    assert!(parsed.win_condition.is_some());
    let sprites = parsed.sprite_set.as_ref().expect("sprite set exists");

    assert_eq!(bundle.level_count(), 3);
    assert_eq!(bundle.level(0).unwrap().name, "microban 1");
    assert_eq!(bundle.level(1).unwrap().name, "microban 2");
    assert_eq!(bundle.level(0).unwrap().level.size, Size3::new(6, 7, 2));
    assert_eq!(
        sprites.sprite("Floor").unwrap().voxels.size,
        Size3::new(5, 5, 5)
    );
    assert_eq!(
        sprites.sprite("Box").unwrap().voxels.size,
        Size3::new(5, 5, 5)
    );

    let fixture_json = export_visual_fixture_json(&parsed).unwrap();
    assert!(fixture_json.contains("\"kind\": \"puzzle3\""));
    assert!(fixture_json.contains("\"levels\""));

    assert_eq!(
        parsed.lifecycle.on_level_clear,
        vec![conditional_win_next_level_effect3()]
    );
}

#[test]
fn microban_basic_01_is_a_single_layer_3d_level() {
    let parsed = microban_basic_model();
    let rules = microban_basic_rules_with_input_guards(&parsed.rules);
    let level = microban_basic_01_level();

    assert_eq!(level.size, Size3::new(6, 7, 1));

    let state = level.build_state(&parsed.game).unwrap();
    let cells = occupied_cells(&state);

    assert_eq!(state.size, Size3::new(6, 7, 1));
    assert!(cells.iter().all(|(position, _)| position.z == 0));
    assert_eq!(cells.len(), 42);
    assert!(state.has_object(&parsed.game, Coord3::new(1, 3, 0), MICROBAN_GOAL));
    assert!(state.has_object(&parsed.game, Coord3::new(1, 3, 0), MICROBAN_BOX));
    assert!(state.has_object(&parsed.game, Coord3::new(2, 3, 0), MICROBAN_PLAYER));
    assert!(state.has_object(&parsed.game, Coord3::new(3, 4, 0), MICROBAN_BOX));

    let moved_down = transition_program(&parsed.game, &state, &rules, INPUT_FORWARD).unwrap();
    let pushed_right = transition_program(&parsed.game, &moved_down, &rules, INPUT_RIGHT).unwrap();

    assert!(pushed_right.has_object(&parsed.game, Coord3::new(3, 4, 0), MICROBAN_PLAYER));
    assert!(pushed_right.has_object(&parsed.game, Coord3::new(4, 4, 0), MICROBAN_BOX));
    assert!(!pushed_right.has_object(&parsed.game, Coord3::new(3, 4, 0), MICROBAN_BOX));
    assert!(pushed_right.has_object(&parsed.game, Coord3::new(1, 3, 0), MICROBAN_GOAL));
    assert!(pushed_right.has_object(&parsed.game, Coord3::new(1, 3, 0), MICROBAN_BOX));
}

#[test]
fn microban_basic_sprites_are_flat_bottom_5x5x1_voxel_slices() {
    let sprites = microban_basic_sprites();

    assert_eq!(sprites.len(), 5);
    for sprite in &sprites {
        assert_eq!(sprite.size, Size3::new(5, 5, 5));
        assert_eq!(sprite.layers.len(), 5);
        assert!(sprite.layers.iter().all(|layer| layer.len() == 5));
        assert!(
            sprite
                .layers
                .iter()
                .all(|layer| layer.iter().all(|row| row.chars().count() == 5))
        );
        assert!(
            sprite.layers[1..]
                .iter()
                .flatten()
                .all(|row| *row == ".....")
        );
    }

    let player = sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .expect("Microban Basic player sprite exists");
    assert_eq!(
        player.layers[0],
        vec![".000.", ".111.", "22222", ".333.", ".3.3."]
    );
    assert_eq!(
        player.palette,
        vec!["#000000", "#ffa500", "#ffffff", "#0000ff"]
    );

    let goal = sprites
        .iter()
        .find(|sprite| sprite.name == "Goal")
        .expect("Microban Basic goal sprite exists");
    assert_eq!(goal.filled_voxel_count(), 8);
}

#[test]
fn parser_accepts_owner_scoped_keys_for_3d_models() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 scoped_inputs {
layers {
solid = Player
}

keys {
d ArrowRight -> right
r -> restart
}

rules {
input right [ Player | ] -> [ | Player ]
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.game.inputs.len(), 2);
    let right = parsed.game.input_by_name("right").unwrap();
    assert_eq!(right.id, INPUT_RIGHT);
    assert_eq!(right.direction, Some(Direction3::RIGHT));
    assert_eq!(right.keys, vec!["d", "ArrowRight"]);
    let restart = parsed.game.input_by_name("restart").unwrap();
    assert_eq!(restart.direction, None);
    assert_eq!(restart.keys, vec!["r"]);
}

#[test]
fn parser_rejects_non_arrow_3d_key_rows_through_shared_surface() {
    let err = parse_puzzle3d(
        r#"
puzzle3 scoped_inputs {
layers {
solid = Player
}

keys {
d ArrowRight = right
}
}
"#,
    )
    .unwrap_err();

    assert!(
        matches!(
            err,
            ParseError3::Message(ref message)
                if message.contains("keys row must use `->`")
        ),
        "{err:?}"
    );
}

#[test]
fn parser_accepts_front_back_as_canonical_3d_directions() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 front_back {
layers {
solid = Player
}

keys {
w ArrowUp -> front
s ArrowDown -> back
}

rules {
front [ Player | ] -> [ | Player ]
back [ Player | ] -> [ | Player ]
input front [ Player | ] -> [ | Player ]
input back [ Player | ] -> [ | Player ]
}
}
"#,
    )
    .unwrap();

    assert_eq!(
        parsed.game.input_by_name("front").map(|input| input.id),
        Some(INPUT_FORWARD)
    );
    assert_eq!(
        parsed.game.input_by_name("back").map(|input| input.id),
        Some(INPUT_BACKWARD)
    );
    assert_eq!(parsed.rules.len(), 4);
    assert_eq!(parsed.rules[0].pattern.cells[0].offset, Offset3::ZERO);
    assert_eq!(
        parsed.rules[0].pattern.cells[1].offset,
        Direction3::FORWARD.offset
    );
    assert_eq!(
        parsed.rules[1].pattern.cells[1].offset,
        Direction3::BACKWARD.offset
    );
    assert_eq!(parsed.rules[2].guards, vec![Guard3::InputIs(INPUT_FORWARD)]);
    assert_eq!(
        parsed.rules[3].guards,
        vec![Guard3::InputIs(INPUT_BACKWARD)]
    );
}

#[test]
fn parser_keeps_forward_backward_as_3d_direction_aliases() {
    let parsed = parse_puzzle3d(
        r#"
puzzle3 legacy_forward_backward {
layers {
solid = Player
}

keys {
w ArrowUp -> forward
s ArrowDown -> backward
}

rules {
forward [ Player | ] -> [ | Player ]
backward [ Player | ] -> [ | Player ]
input forward [ Player | ] -> [ | Player ]
input backward [ Player | ] -> [ | Player ]
}
}
"#,
    )
    .unwrap();

    assert!(parsed.game.input_by_name("forward").is_none());
    assert!(parsed.game.input_by_name("backward").is_none());
    assert_eq!(
        parsed
            .game
            .input_by_name("front")
            .map(|input| input.keys.clone()),
        Some(vec!["w".to_string(), "ArrowUp".to_string()])
    );
    assert_eq!(
        parsed
            .game
            .input_by_name("back")
            .map(|input| input.keys.clone()),
        Some(vec!["s".to_string(), "ArrowDown".to_string()])
    );
    assert_eq!(parsed.rules.len(), 4);
    assert_eq!(
        parsed.rules[0].pattern.cells[1].offset,
        Direction3::FORWARD.offset
    );
    assert_eq!(
        parsed.rules[1].pattern.cells[1].offset,
        Direction3::BACKWARD.offset
    );
    assert_eq!(parsed.rules[2].guards, vec![Guard3::InputIs(INPUT_FORWARD)]);
    assert_eq!(
        parsed.rules[3].guards,
        vec![Guard3::InputIs(INPUT_BACKWARD)]
    );
}
