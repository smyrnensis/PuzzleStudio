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

fn diagnostic_contains(report: &DiagnosticReport, needle: &str) -> bool {
    report
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.message.contains(needle))
}

fn parse_puzzle_body3(body: &str) -> Result<ParsedPuzzle3, DiagnosticReport> {
    parse_puzzle3d(&format!("puzzle test {{\ndimension = 3\n{body}\n}}"))
}

fn spec_3d_model_source() -> String {
    let source = include_str!("fixtures/spec_3d_full.puzzle3");
    [
        source_block(source, "puzzle sokoban").as_str(),
        source_block(source, "levels microban").as_str(),
        source_block(source, "sprites basic").as_str(),
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
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == name).then_some(*object))
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
fn at_prefixed_layer_objects_are_normal_objects() {
    let parsed = parse_puzzle3d(
        r#"
puzzle display3 {
dimension = 3
slots {
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

    object_id(&parsed, "@Dust");
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
    parse_puzzle_body3(
        r#"
slots {
@Floor
Goal
Player Box Wall
}

groups {
solid = Player Box Wall
}

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
            let direction = rule.pattern.cells()[1].offset.clone();
            rule.when_input(input_for_microban_offset(direction))
        })
        .collect()
}

fn input_for_microban_offset(offset: Offset3) -> InputId {
    if offset == Direction3::LEFT.offset.into() {
        INPUT_LEFT
    } else if offset == Direction3::RIGHT.offset.into() {
        INPUT_RIGHT
    } else if offset == Direction3::FORWARD.offset.into() {
        INPUT_FORWARD
    } else if offset == Direction3::BACKWARD.offset.into() {
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
    let parsed = parse_puzzle_body3(
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
    )
    .unwrap();

    assert_eq!(parsed.game.layer_count, 1);
    assert_eq!(parsed.game.objects().len(), 3);
    assert_eq!(flattened_rules(parsed.game.program()).len(), 4);
    assert_eq!(
        flattened_rules(parsed.game.program())[1].pattern.cells()[1].offset,
        Direction3::RIGHT.offset.into()
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[1].writes,
        vec![WriteOp3::Move {
            component: 0,
            from_offset: Delta3::ZERO.into(),
            to_offset: Direction3::RIGHT.offset.into(),
            object: ObjectId(1),
        }]
    );
}

#[test]
fn parser_collects_on_level_start_rules_through_shared_block_body() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
Player Wall
}

groups {
solid = Player Wall
}

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
        flattened_rules(&parsed.lifecycle.on_level_start)[0]
            .pattern
            .cells()[1]
            .offset,
        Direction3::RIGHT.offset.into()
    );
}

#[test]
fn plain_3d_lifecycle_rewrite_uses_shared_repeat_statement_boundary() {
    let parsed = parse_puzzle3d(
        r#"
puzzle fill_floor {
dimension = 3
slots {
floor = Floor
}
on_level_start {
[ no Floor ] -> [ Floor ]
}
rules {
}
}

levels basic of fill_floor {
legend {
. = empty
}
level "start" {
..
..

..
..
}
}
"#,
    )
    .unwrap();
    let bundle = parsed.level_bundle.as_ref().unwrap();
    let initial = bundle.build_level_state(0).unwrap();
    let next =
        transition_program_without_input(&parsed.game, &initial, &parsed.lifecycle.on_level_start)
            .unwrap();
    let floor = ObjectId(1);
    let occupied = (0..2)
        .flat_map(|z| (0..2).flat_map(move |y| (0..2).map(move |x| Coord3::new(x, y, z))))
        .filter(|position| next.has_object(&parsed.game, *position, floor))
        .count();

    assert_eq!(occupied, 8);
    assert!(matches!(
        parsed.lifecycle.on_level_start.as_slice(),
        [RuleStep3::Block {
            application: RuleApplication3::UntilStable,
            ..
        }]
    ));
}

#[test]
fn once_3d_neutral_rewrite_fires_once_across_direction_alternatives() {
    let parsed = parse_puzzle3d(
        r#"
puzzle fill_one {
dimension = 3
slots {
floor = Floor
}
on_level_start {
once [ no Floor ] -> [ Floor ]
}
rules {
}
}

levels basic of fill_one {
legend {
. = empty
}
level "start" {
...
...
}
}
"#,
    )
    .unwrap();
    let bundle = parsed.level_bundle.as_ref().unwrap();
    let initial = bundle.build_level_state(0).unwrap();
    let next =
        transition_program_without_input(&parsed.game, &initial, &parsed.lifecycle.on_level_start)
            .unwrap();
    let floor = ObjectId(1);
    let occupied = (0..2)
        .flat_map(|y| (0..3).map(move |x| Coord3::new(x, y, 0)))
        .filter(|position| next.has_object(&parsed.game, *position, floor))
        .count();

    assert_eq!(occupied, 1);
}

#[test]
fn once_3d_input_statement_selects_the_matching_guarded_direction() {
    let parsed = parse_puzzle3d(
        r#"
puzzle move_once {
dimension = 3
slots {
actor = Player
}
rules {
once input [ Player | no Player ] -> [ | Player ]
}
}

levels basic of move_once {
legend {
. = empty
P = Player
}
level "start" {
...
.P.
...
}
}
"#,
    )
    .unwrap();
    let initial = parsed
        .level_bundle
        .as_ref()
        .unwrap()
        .build_level_state(0)
        .unwrap();
    let next =
        transition_program(&parsed.game, &initial, parsed.game.program(), INPUT_RIGHT).unwrap();
    let player = object_id(&parsed, "Player");

    assert!(!next.has_object(&parsed.game, Coord3::new(1, 1, 0), player));
    assert!(next.has_object(&parsed.game, Coord3::new(2, 1, 0), player));
}

#[test]
fn parser_lowers_3d_line_gap_rules_against_level_extent() {
    let parsed = parse_puzzle3d(
        r#"
puzzle gap3 {
dimension = 3
slots {
floor = Goal
actor = Player
}

rules {
right [ Player | ... | Goal ] -> [ | ... | Player Goal ]
}
}

levels basic of gap3 {
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
    let rules = flattened_rules(game.program());
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].pattern.components[0].gap_count, 1);
    assert!(matches!(
        rules[0].pattern.components[0].cells[1].offset,
        puzzle_grid3d::Offset3::Variable { .. }
    ));
    let bundle = parsed.level_bundle.as_ref().unwrap();
    let state = bundle.build_level_state(0).unwrap();

    let next = transition_program(game, &state, parsed.game.program(), InputId(0)).unwrap();

    assert!(!next.has_object(game, Coord3::new(0, 0, 0), ObjectId(2)));
    assert!(next.has_object(game, Coord3::new(3, 0, 0), ObjectId(2)));
}

#[test]
fn parser_materializes_named_occurrence_cell_marks_and_core_effects() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
actor = Player
}

marks {
checked
cellflag
}

rules {
once [ Player{checked} {cellflag} ] -> [ Player{checked} {cellflag} ] win
}
"#,
    )
    .unwrap();
    let rules = flattened_rules(parsed.game.program());
    assert!(!rules.is_empty());
    assert_eq!(parsed.game.mark().len(), 7);
    let cell = &rules[0].pattern.components[0].cells[0];
    assert!(cell.require_mark.iter().any(|mark| !mark.object.is_empty()));
    assert!(cell.require_mark.iter().any(|mark| mark.object.is_empty()));
    assert!(
        rules
            .iter()
            .all(|rule| matches!(rule.effects.as_slice(), [RuleEffect3::Win]))
    );
}

#[test]
fn parser_keeps_render_settings_as_model_owned_display_state() {
    let parsed = parse_puzzle3d(
        r#"
puzzle camera_test {
dimension = 3
render {
      camera {
yaw = 90
pitch = 42
roll = 15
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
      shadow = true
    }

slots {
Player
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.settings.camera.yaw_degrees, 90);
    assert_eq!(parsed.settings.camera.pitch_degrees, 42);
    assert_eq!(parsed.settings.camera.roll_degrees, 15);
    assert_eq!(parsed.settings.camera.zoom_milli, 1250);
    assert!(parsed.settings.camera.interactive_look);
    assert!(parsed.settings.camera.interactive_zoom);
    assert!(parsed.settings.grid.occupied_cells);
    assert!(parsed.settings.sprite.shade);
    assert!(parsed.settings.shadow);
    assert!(parsed.settings.pixelate.enabled);
    assert_eq!(parsed.settings.pixelate.scale, 4);
    assert!(parsed.settings.pixelate.smoothing);
}

#[test]
fn parser_accepts_boolean_render_setting_assignments() {
    let parsed = parse_puzzle3d(
        r#"
puzzle camera_test {
dimension = 3
render {
  camera {
interactive_look = true
  }
  pixelate {
enabled = true
smoothing = false
  }
  shade = false
  shadow = false
}

slots {
Player
}
}
"#,
    )
    .unwrap();

    assert!(parsed.settings.camera.interactive_look);
    assert!(parsed.settings.pixelate.enabled);
    assert!(!parsed.settings.pixelate.smoothing);
    assert!(!parsed.settings.sprite.shade);
    assert!(!parsed.settings.shadow);
}

#[test]
fn parser_rejects_old_inline_camera_render_settings() {
    let parsed = parse_puzzle3d(
        r#"
puzzle camera_test {
dimension = 3
render {
  camera yaw=90
}

slots {
Player
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
puzzle camera_test {
dimension = 3
render {
  grid {
occupied_cells
  }
}

slots {
Player
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
puzzle camera_test {
dimension = 3
render {
  grid occupied_cells
}

slots {
Player
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
puzzle camera_test {
dimension = 3
render {
  pixelate scale=4
}

slots {
Player
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
puzzle camera_test {
dimension = 3
render {
  shade
}

slots {
Player
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
puzzle camera_test {
dimension = 3
render {
  camera {
yaw 90
  }
}

slots {
Player
}
}
"#,
    );

    assert!(parsed.is_err());
}

#[test]
fn parser_defaults_render_settings() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
Player
}
"#,
    )
    .unwrap();

    assert!(!parsed.settings.camera.interactive_look);
    assert!(!parsed.settings.camera.interactive_zoom);
    assert_eq!(parsed.settings.camera.yaw_degrees, 0);
    assert_eq!(parsed.settings.camera.pitch_degrees, 90);
    assert_eq!(parsed.settings.camera.roll_degrees, 0);
    assert_eq!(parsed.settings.camera.zoom_milli, 1000);
    assert!(!parsed.settings.grid.occupied_cells);
    assert!(parsed.settings.sprite.shade);
    assert!(!parsed.settings.shadow);
    assert!(!parsed.settings.pixelate.enabled);
    assert_eq!(parsed.settings.pixelate.scale, 4);
    assert!(parsed.settings.pixelate.smoothing);
}

#[test]
fn parser_defaults_3d_zoomscreen_height_to_full() {
    let parsed = parse_puzzle3d(
        r#"
puzzle viewport_test {
dimension = 3
render {
  viewport {
zoomscreen 7 5
focus Player
  }
}

slots {
Player
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
puzzle local_radius_test {
dimension = 3
slots {
Player Box
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
puzzle viewport_test {
dimension = 3
render {
  viewport {
smoothscreen 9 7 3
focus actor
  }
}

slots {
Player Box
}

groups {
actor = Player Box
}
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
puzzle viewport_test {
dimension = 3
render {
  viewport {
flickscreen 9 7 2
focus Player
  }
}

slots {
Player
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
puzzle viewport_test {
dimension = 3
render {
  viewport {
smoothscreen 7 7
focus actor
  }
}

slots {
Player Box
Floor
}

groups {
actor = Player Box
}
}

levels test of viewport_test {
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
puzzle no_sprites {
dimension = 3
slots {
Player
}
}

levels test of no_sprites {
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
puzzle camera_test {
dimension = 3
debug_camera = true
camera_yaw = 90
camera_pitch = 42
camera_zoom = 1.25

slots {
Player
}
}
"#,
    );

    assert!(parsed.is_err());
}

#[test]
fn parser_lowers_input_guarded_direction_set_rule() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
actor = Player Box Wall
}

groups {
solid = Player Box Wall
}

rules {
input horizontal [ Player | no solid ] -> [ | Player ]
}
"#,
    )
    .unwrap();

    assert_eq!(flattened_rules(parsed.game.program()).len(), 4);
    assert_eq!(
        flattened_rules(parsed.game.program())[0].guards,
        vec![Guard3::InputIs(INPUT_LEFT)]
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[1].guards,
        vec![Guard3::InputIs(INPUT_RIGHT)]
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[2].guards,
        vec![Guard3::InputIs(INPUT_FORWARD)]
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[3].guards,
        vec![Guard3::InputIs(INPUT_BACKWARD)]
    );
}

#[test]
fn parser_lowers_input_rule_without_set_as_directions_sugar() {
    let parsed = parse_puzzle_body3(
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
    )
    .unwrap();

    assert_eq!(flattened_rules(parsed.game.program()).len(), 6);
    let rules = flattened_rules(parsed.game.program());
    let guards = rules
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
    let parsed = parse_puzzle_body3(
        r#"
slots {
actor = Box
}

rules {
right [ Box{horizontal} ] -> [ Box{vertical} ]
}
"#,
    )
    .unwrap();

    assert_eq!(flattened_rules(parsed.game.program()).len(), 8);
}

#[test]
fn parser_lowers_input_rule_with_forward_marker_rhs_sugar_as_movement_mark() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
actor = Player
}

rules {
input [ Player ] -> [ > Player ]
}
"#,
    )
    .unwrap();

    assert_eq!(flattened_rules(parsed.game.program()).len(), 6);
    assert!(flattened_rules(parsed.game.program()).iter().any(|rule| {
        rule.guards == vec![Guard3::InputIs(INPUT_RIGHT)]
            && rule.writes
                == vec![WriteOp3::SetMark {
                    component: 0,
                    object: PLAYER,
                    offset: Delta3::ZERO.into(),
                    mark: MarkId3(0),
                    value: Some(3),
                }]
    }));
}

#[test]
fn default_repeat_rewrite_stops_after_rhs_removes_movement_mark_in_3d() {
    let parsed = parse_puzzle3d(
        r#"
puzzle move_once3 {
dimension = 3
slots {
actor = Player
}

rules {
input [ Player ] -> [ > Player ]
[ > Player | ] -> [ | Player ]
}
}

levels demo of move_once3 {
legend {
. = empty
P = Player
}

level "start" {
P....
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

    let moved =
        transition_program(&parsed.game, &initial, parsed.game.program(), INPUT_RIGHT).unwrap();
    let player = object_id(&parsed, "Player");

    assert!(moved.has_object(&parsed.game, Coord3::new(1, 0, 0), player));
    assert!(!moved.has_object(&parsed.game, Coord3::new(4, 0, 0), player));
    assert!(!moved.has_mark(&parsed.game, Coord3::new(1, 0, 0), player, MarkId3(0), None,));
}

#[test]
fn bare_move_is_not_a_3d_builtin_step() {
    let error = parse_puzzle3d(
        r#"
puzzle push3 {
dimension = 3
slots {
actor = Player
}

rules {
move
}
}
"#,
    )
    .unwrap_err();
    assert!(diagnostic_contains(
        &error,
        "3D rule blocks do not support routine calls"
    ));
}

#[test]
fn neutral_3d_rewrite_expands_relative_movement_mark_by_direction() {
    let parsed = parse_puzzle3d(
        r#"
puzzle push3 {
dimension = 3
slots {
actor = Player Box
}

rules {
input [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
right [ > Player | > Box | no Player no Box ] -> [ | Player | Box ]
}
}

levels demo of push3 {
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

    let moved =
        transition_program(&parsed.game, &initial, parsed.game.program(), INPUT_RIGHT).unwrap();
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
puzzle surface3 {
dimension = 3
slots {
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

    let rules = flattened_rules(parsed.game.program());
    assert!(
        rules
            .iter()
            .any(|rule| rule.application == RuleApplication3::Once)
    );
    assert!(parsed.game.program().iter().any(|step| matches!(
        step,
        RuleStep3::Block {
            application: RuleApplication3::OnceAll,
            ..
        }
    )));
    assert!(
        rules
            .iter()
            .any(|rule| rule.application == RuleApplication3::OncePerLevel)
    );
    for application in [RuleApplication3::Random, RuleApplication3::UntilStable] {
        assert!(parsed.game.program().iter().any(|step| matches!(
            step,
            RuleStep3::Block {
                application: actual,
                ..
            } if *actual == application
        )));
    }
}

#[test]
fn parser_rejects_shared_statement_surfaces_without_3d_lowering() {
    let routine_call = parse_puzzle3d(
        r#"
puzzle no_routine_calls3 {
dimension = 3
slots {
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
        diagnostic_contains(&routine_call, "3D rule blocks do not support routine calls"),
        "{routine_call:?}"
    );

    let application_block = parse_puzzle3d(
        r#"
puzzle no_application_blocks3 {
dimension = 3
slots {
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
        diagnostic_contains(
            &application_block,
            "3D rule blocks do not support nested application blocks"
        ),
        "{application_block:?}"
    );
}

#[test]
fn parser_lowers_camera_variable_effects_from_rules() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
actor = Player Wall
}

rules {
right [ Player | no Wall ] -> [ | Player ] set yaw = 100
set roll = 25
set zoom = 1.5
reset_camera
}

levels test {
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

    assert_eq!(flattened_rules(parsed.game.program()).len(), 4);
    assert!(
        flattened_rules(parsed.game.program())
            .iter()
            .all(|rule| rule.effects.is_empty())
    );
    assert_eq!(
        parsed.rule_camera_effects[0],
        vec![Puzzle3CameraEffect::SetYaw(100)]
    );
    assert_eq!(
        parsed.rule_camera_effects[1],
        vec![Puzzle3CameraEffect::SetRoll(25)]
    );
    assert_eq!(
        parsed.rule_camera_effects[2],
        vec![Puzzle3CameraEffect::SetZoom(1500)]
    );
    assert!(
        flattened_rules(parsed.game.program())[2]
            .pattern
            .cells()
            .is_empty()
    );
    assert!(flattened_rules(parsed.game.program())[2].writes.is_empty());
    assert_eq!(
        parsed.rule_camera_effects[3],
        vec![Puzzle3CameraEffect::Reset]
    );

    let fixture = export_visual_fixture_json(&parsed).unwrap();
    let contract =
        puzzle3_runtime_model_from_fixture_json(&fixture).expect("runtime contract decodes");
    assert!(
        flattened_rules(contract.game.program())
            .iter()
            .all(|rule| rule.effects.is_empty())
    );
    assert_eq!(contract.rule_camera_effects, parsed.rule_camera_effects);
}

#[test]
fn parser_lowers_variant_selector_assignment() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
Marker:directions
}

rules {
directions [ Marker:* | ] -> [ | Marker:* ]
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.game.objects().len(), 6);
    assert_eq!(flattened_rules(parsed.game.program()).len(), 36);
    assert!(flattened_rules(parsed.game.program()).iter().all(|rule| {
        rule.pattern.cells()[0].require_objects[0]
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
puzzle bare_star {
dimension = 3
slots {
Player Box
}

rules {
once right [ * | no * ] -> [ | * ]
}
}

levels basic of bare_star {
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

    assert_eq!(flattened_rules(parsed.game.program()).len(), 2);
    let state = parsed
        .level_bundle
        .as_ref()
        .unwrap()
        .build_level_state(0)
        .unwrap();
    let next =
        transition_program_without_input(&parsed.game, &state, parsed.game.program()).unwrap();

    assert!(next.has_object(&parsed.game, Coord3::new(0, 0, 0), ObjectId(1)));
    assert!(!next.has_object(&parsed.game, Coord3::new(1, 0, 0), ObjectId(2)));
    assert!(next.has_object(&parsed.game, Coord3::new(2, 0, 0), ObjectId(2)));
}

#[test]
fn parser_lowers_group_selector_to_runtime_object_set_matcher() {
    let parsed = parse_puzzle3d(
        r#"
puzzle group_move {
dimension = 3
slots {
Box Crate
}

groups {
solid = Box Crate
}

rules {
once right [ solid | ] -> [ | solid ]
}
}

levels basic of group_move {
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

    assert_eq!(flattened_rules(parsed.game.program()).len(), 1);
    let rule = &flattened_rules(parsed.game.program())[0];
    assert!(rule.pattern.cells()[0].require_objects.is_empty());
    assert_eq!(rule.pattern.cells()[0].require_object_sets.len(), 1);
    assert_eq!(
        rule.pattern.cells()[0].require_object_sets[0].objects,
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
    let next =
        transition_program_without_input(&parsed.game, &state, parsed.game.program()).unwrap();
    assert!(!next.has_object(&parsed.game, Coord3::new(0, 0, 0), ObjectId(1)));
    assert!(next.has_object(&parsed.game, Coord3::new(1, 0, 0), ObjectId(1)));
}

#[test]
fn parser_lowers_selector_occurrence_labels_for_group_swap() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
actor = Box Crate
}

groups {
solid = Box Crate
}

rules {
right [ solid#1 | solid#2 ] -> [ solid#2 | solid#1 ]
}
"#,
    )
    .unwrap();

    assert!(flattened_rules(parsed.game.program()).iter().any(|rule| {
        rule.pattern.cells()[0].require_objects == vec![ObjectId(1)]
            && rule.pattern.cells()[1].require_objects == vec![ObjectId(2)]
            && rule.writes.contains(&WriteOp3::Move {
                component: 0,
                from_offset: Delta3::ZERO.into(),
                to_offset: Direction3::RIGHT.offset.into(),
                object: ObjectId(1),
            })
            && rule.writes.contains(&WriteOp3::Move {
                component: 0,
                from_offset: Direction3::RIGHT.offset.into(),
                to_offset: Delta3::ZERO.into(),
                object: ObjectId(2),
            })
    }));
}

#[test]
fn parser_rejects_duplicate_selector_occurrence_labels() {
    let err = parse_puzzle_body3(
        r#"
slots {
actor = Box Crate
}

groups {
solid = Box Crate
}

rules {
right [ solid#1 | solid#1 ] -> [ solid#1 | solid#1 ]
}
"#,
    )
    .unwrap_err();

    assert!(diagnostic_contains(
        &err,
        "DuplicateSelectorOccurrenceLabel"
    ));
}

#[test]
fn parser_lowers_dense_frame_rule() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
Player Box
}

rules {
(right, up) [ Player | Box ] -> [ | Player | Box ]
}
"#,
    )
    .unwrap();

    assert_eq!(flattened_rules(parsed.game.program()).len(), 1);
    assert_eq!(
        flattened_rules(parsed.game.program())[0].pattern.cells()[1].offset,
        Direction3::RIGHT.offset.into()
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[0].writes[0],
        WriteOp3::Move {
            component: 0,
            from_offset: Delta3::ZERO.into(),
            to_offset: Direction3::RIGHT.offset.into(),
            object: ObjectId(1),
        }
    );
}

#[test]
fn parser_lowers_dense_frame_rule_with_pattern_line_breaks() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
Player Box
}

rules {
right, up [ Player
Box ] -> [ Player
Box ]
}
"#,
    )
    .unwrap();

    assert_eq!(flattened_rules(parsed.game.program()).len(), 1);
    assert_eq!(
        flattened_rules(parsed.game.program())[0].pattern.cells()[1].offset,
        Direction3::UP.offset.into()
    );
}

#[test]
fn parser_binds_frame3_domain_values_to_parenthesized_object_slots() {
    let parsed = parse_puzzle_body3(
        r#"
tags {
pose = right, front front, left
}

slots {
Die:pose
}

legend {
d = Die:(right, front)
}

rules {
right [ Die:(right, front) ] -> [ Die:(right, front) ]
}

levels test {
level "one" {
d
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.game.objects().len(), 2);
    assert_eq!(flattened_rules(parsed.game.program()).len(), 1);
}

#[test]
fn parser_rejects_colon_frame_orientation_syntax() {
    let error = parse_puzzle_body3(
        r#"
slots {
Player
}

rules {
right:up [ Player ] -> [ Player ]
}
"#,
    )
    .unwrap_err();

    assert!(diagnostic_contains(&error, "unknown direction set"));
}

#[test]
fn parser_rejects_unparenthesized_frame3_object_slots() {
    let error = parse_puzzle_body3(
        r#"
tags {
pose = right, front
}

slots {
Die:pose
}

rules {
right [ Die:right,front ] -> [ Die:right,front ]
}
"#,
    )
    .unwrap_err();

    assert!(diagnostic_contains(
        &error,
        "frame3 object slot must be parenthesized"
    ));
}

#[test]
fn parser_accepts_inline_braced_blocks_with_semicolon_rows() {
    let parsed = parse_puzzle_body3(
        r#"
slots { Player Box }
groups { solid = Box }
rules { right [ Player | no solid ] -> [ | Player ] }
"#,
    )
    .unwrap();

    assert_eq!(parsed.game.objects().len(), 2);
    assert_eq!(flattened_rules(parsed.game.program()).len(), 1);
}

#[test]
fn parser_lowers_layers_legend_and_levels_to_level_bundle() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
floor = Goal
actor = Player Box Wall
}

groups {
solid = Player Box Wall
}

rules {
horizontal [ Player | no solid ] -> [ | Player ]
}

levels {
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
    assert_eq!(parsed.game.objects().len(), 4);

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
fn parser_uses_shared_anonymous_named_and_each_layer_rows() {
    let parsed = parse_puzzle3d(
        r#"
puzzle shared_layers {
dimension = 3
slots {
Floor
solid = Player Box
each Marker Goal
for object in Crate Wall {
object
}
}
rules {
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.game.layer_count, 6);
    for name in ["Floor", "Player", "Box", "Marker", "Goal", "Crate", "Wall"] {
        assert!(
            parsed.object_labels.iter().any(|(_, label)| label == name),
            "missing shared layer-row object {name}"
        );
    }
}

#[test]
fn parser_uses_shared_tags_block_and_resolves_groups_without_source_order() {
    let parsed = parse_puzzle3d(
        r#"
puzzle shared_catalog {
dimension = 3
tags {
state = open closed
}
slots {
Door:state Key
}
groups {
interactive = lockable Key
lockable = Door:closed
}
rules {
right [ interactive ] -> [ interactive ]
}
}
"#,
    )
    .unwrap();

    assert_eq!(flattened_rules(parsed.game.program()).len(), 1);
}

#[test]
fn parser_expands_forward_groups_before_declaring_layer_objects() {
    let parsed = parse_puzzle3d(
        r#"
puzzle forward_layer_groups {
dimension = 3
slots {
solid
}
groups {
solid = pushable Wall
pushable = Box Crate
}
rules {
right [ solid ] -> [ solid ]
}
}
"#,
    )
    .unwrap();

    for name in ["Box", "Crate", "Wall"] {
        assert!(parsed.object_labels.iter().any(|(_, label)| label == name));
    }
    assert_eq!(flattened_rules(parsed.game.program()).len(), 1);
}

#[test]
fn parser_uses_shared_tag_domain_normalization() {
    let parsed = parse_puzzle3d(
        r#"
puzzle shared_tag_domains {
dimension = 3
tags {
angle = 0deg 90deg
count = 1...3
pose = right, front front, left
}
slots {
Rotor:angle
Counter:count
Die:pose
}
rules {
}
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.game.objects().len(), 7);

    let duplicate = parse_puzzle3d(
        r#"
puzzle duplicate_tag_values {
dimension = 3
tags {
state = open open
}
slots {
Door:state
}
rules {
}
}
"#,
    )
    .unwrap_err();
    assert!(duplicate.to_string().contains("duplicate value"));
}

#[test]
fn parser_rejects_bare_tag_sets_and_cyclic_groups() {
    let bare = parse_puzzle3d(
        r#"
puzzle bare_tags {
dimension = 3
state = open closed
slots {
Door:state
}
rules {
}
}
"#,
    )
    .unwrap_err();
    assert!(
        bare.to_string()
            .contains("object schema tag slot must name a tag set")
    );

    let cycle = parse_puzzle3d(
        r#"
puzzle cyclic_groups {
dimension = 3
slots {
Player
}
groups {
a = b
b = a
}
rules {
}
}
"#,
    )
    .unwrap_err();
    assert!(cycle.to_string().contains("cannot be cyclic"));
}

#[test]
fn catalog_validation_reports_tag_errors_before_selector_namespace_conflicts_in_2d_and_3d() {
    let source2d = r#"
puzzle board {
dimension = 3
tags {
bad-name = open closed
}
slots {
solid = Player
}
groups {
solid = Player
}
rules {
}
}
"#;
    let source3d = source2d.to_string();

    let error2d = parse_game(source2d).unwrap_err();
    let error3d = parse_puzzle3d(&source3d).unwrap_err();

    assert!(diagnostic_contains(
        &error2d,
        "tag set name must be an identifier"
    ));
    assert!(diagnostic_contains(
        &error3d,
        "tag set name must be an identifier"
    ));

    let conflict2d = source2d.replace("bad-name", "state");
    let conflict3d = conflict2d.clone();
    let error2d = parse_game(&conflict2d).unwrap_err();
    let error3d = parse_puzzle3d(&conflict3d).unwrap_err();
    assert!(
        diagnostic_contains(&error2d, "group name must not shadow another selector"),
        "{error2d}"
    );
    assert!(diagnostic_contains(
        &error3d,
        "group name must not shadow another selector"
    ));
}

#[test]
fn parser_rejects_removed_levels3_header() {
    let error = parse_puzzle3d(
        r#"
puzzle board {
dimension = 3
slots {
Player
}
rules {
}
}
levels3 demo of board {
level "one" {
P
}
}
"#,
    )
    .unwrap_err();

    assert!(diagnostic_contains(
        &error,
        "`levels3` was removed; use `levels`"
    ));
}

#[test]
fn parser_rejects_unquoted_3d_level_header_names() {
    let err = parse_puzzle_body3(
        r#"
slots {
actor = Player
}

levels {
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

    assert!(diagnostic_contains(
        &err,
        "level header must be: level \"<id>\""
    ));
}

#[test]
fn parser_uses_dot_as_default_empty_char_for_3d_levels() {
    let parsed = parse_puzzle_body3(
        r#"
slots {
actor = Player
}

levels {
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
fn parser_rejects_non_dot_empty_char_for_levels() {
    let err = parse_puzzle_body3(
        r#"
slots {
actor = Player
}

levels {
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

    assert!(diagnostic_contains(&err, "levels use `.` for empty"));
}

#[test]
fn parser_lowers_canonical_3d_sprite_entries() {
    let parsed = parse_puzzle_body3(
        r##"
slots {
floor = Floor
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
    assert_eq!(floor.first_frame().size, Size3::new(5, 3, 2));
}

#[test]
fn parser_lowers_canonical_3d_sprite_shape_refs() {
    let parsed = parse_puzzle_body3(
        r##"
slots {
floor = Floor
}

sprites basic {
shapes {
flat {
.....
..1..
.....
-
00000
0...0
00000
}
}

Floor {
colors = #90ee90 #008000
shape = flat
}
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
    assert_eq!(floor.first_frame().size, Size3::new(5, 3, 2));
}

#[test]
fn parser_lowers_color_only_3d_sprite_entry_to_filled_cube() {
    let parsed = parse_puzzle_body3(
        r##"
slots {
floor = Floor
target = Goal
}

sprites basic {
Floor {
colors = #90ee90
}

Goal {
colors = #00008b
}
}
"##,
    )
    .unwrap();

    let sprites = parsed.sprite_set.as_ref().expect("sprite set exists");
    let floor = sprites.sprite("Floor").expect("Floor sprite exists");
    let goal = sprites.sprite("Goal").expect("Goal sprite exists");

    assert_eq!(floor.first_frame().size, Size3::new(1, 1, 1));
    assert_eq!(
        floor.first_frame().slices.as_slice(),
        &[vec!["0".to_string()]]
    );
    assert_eq!(goal.first_frame().size, Size3::new(1, 1, 1));
}

#[test]
fn parser_rejects_blank_lines_in_sprite_shape() {
    let err = parse_puzzle_body3(
        r##"
slots {
floor = Floor
}

sprites basic {
Floor {
colors = #90ee90
shape = {
0

0
}
}
}
"##,
    )
    .unwrap_err();

    assert!(diagnostic_contains(&err, "cannot contain blank lines"));
}

#[test]
fn parser_rejects_legacy_sprites3_blocks() {
    let err = parse_puzzle_body3(
        r##"
slots {
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

    assert!(diagnostic_contains(
        &err,
        "unknown model directive: sprites3"
    ));
}

#[test]
fn parser_lowers_world_and_local_3d_sprite_spatial_ops_in_source_order() {
    let source = include_str!("fixtures/spec_3d_full.puzzle3").replacen(
        "Floor {\ncolors =",
        "Floor {\ntranslate local (1, 0, -1/2)\nrotate 45deg around (1, 1, 0)\nrotate local 90deg around up\ncolors =",
        1,
    );
    let parsed = parse_puzzle3d(&source).unwrap();
    let sprite = parsed.sprite_set.as_ref().unwrap().sprite("Floor").unwrap();
    assert_eq!(sprite.spatial_ops.len(), 3);
    assert_eq!(
        sprite.spatial_ops[0],
        SpriteSpatialOp3::Translate {
            space: SpriteSpace3::Local,
            value: [1.0, 0.0, -0.5]
        }
    );
    assert!(matches!(
        sprite.spatial_ops[1],
        SpriteSpatialOp3::Rotate {
            space: SpriteSpace3::World,
            ..
        }
    ));
    assert_eq!(
        sprite.spatial_ops[2],
        SpriteSpatialOp3::Rotate {
            space: SpriteSpace3::Local,
            axis: [0.0, 0.0, 1.0],
            degrees: 90.0
        }
    );
}

#[test]
fn parser_rejects_removed_shape_rotation_derivation() {
    let source = include_str!("fixtures/spec_3d_full.puzzle3").replacen(
        "Floor {\ncolors =",
        "Floor {\nrotate from up\ncolors =",
        1,
    );
    let error = parse_puzzle3d(&source).unwrap_err();
    assert!(
        diagnostic_contains(&error, "removed sprite rotation syntax"),
        "{error:?}"
    );
}

#[test]
fn parser_rejects_unknown_level_legend_char() {
    let err = parse_puzzle_body3(
        r#"
slots {
actor = Player
}

levels {
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

    assert!(diagnostic_contains(&err, "unknown legend char: X"));
}

#[test]
fn parser_lowers_model_wrapped_win_conditions_and_named_level_pack() {
    let parsed = parse_puzzle3d(
        r#"
puzzle push3d {
dimension = 3
slots {
floor = Goal
actor = Player Box
}

win_conditions {
some Goal
no down [ no Box | Goal ]
}
}

levels basic of push3d {
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
puzzle push3d {
dimension = 3
slots {
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

    assert!(diagnostic_contains(&err, "unknown win condition"), "{err}");
}

#[test]
fn parser_preserves_all_on_as_covered_object_condition() {
    let parsed = parse_puzzle3d(
        r#"
puzzle push3d {
dimension = 3
slots {
floor = Goal
actor = Box
}

win_conditions {
all Goal on Box
}
}
"#,
    )
    .unwrap();
    let goal = object_id(&parsed, "Goal");
    let box_object = object_id(&parsed, "Box");
    let WinCondition3::AllObjectsCoveredByPattern {
        object,
        cover_pattern,
    } = parsed.win_condition.as_ref().unwrap()
    else {
        panic!("all X on Y should preserve its covered-object semantics");
    };

    assert_eq!(*object, goal);
    assert_eq!(cover_pattern.cells().len(), 1);
    assert!(cover_pattern.cells()[0].require_objects.contains(&goal));
    assert!(
        cover_pattern.cells()[0]
            .require_objects
            .contains(&box_object)
    );
}

#[test]
fn parser_accepts_function_style_3d_win_conditions() {
    let parsed = parse_puzzle3d(
        r#"
puzzle push3d {
dimension = 3
slots {
floor = Goal
actor = Box
}

win_conditions {
exists(Goal)
none(down [ no Box | Goal ])
}
}

levels basic of push3d {
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
fn parser_accepts_last_level_clear_lifecycle() {
    let parsed = parse_puzzle3d(
        r#"
puzzle lifecycle {
dimension = 3
slots {
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
puzzle lifecycle {
dimension = 3
slots {
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
        sprites.sprite("Floor").unwrap().first_frame().size,
        Size3::new(5, 5, 5)
    );
    assert_eq!(
        sprites.sprite("Box").unwrap().first_frame().size,
        Size3::new(5, 5, 5)
    );
    assert_eq!(
        sprites.sprite("Player").unwrap().first_frame().size,
        Size3::new(5, 5, 5)
    );
    assert_eq!(
        sprites.sprite("Wall").unwrap().first_frame().size,
        Size3::new(5, 5, 5)
    );
    let fixture_json = export_visual_fixture_json(&parsed).unwrap();
    assert!(fixture_json.contains("\"shade\": true"));
    assert!(fixture_json.contains("\"shadow\": false"));
    let contract =
        puzzle3_runtime_model_from_fixture_json(&fixture_json).expect("runtime contract decodes");
    assert!(!flattened_rules(contract.game.program()).is_empty());
    assert_eq!(
        contract.lifecycle.on_level_clear,
        vec![conditional_win_next_level_effect3()]
    );
    assert!(contract.win_condition.is_some());
    assert!(fixture_json.contains("\"Box\": {"));
    assert!(fixture_json.contains("\"frames\": ["));
    assert!(fixture_json.contains("\"layers\": ["));

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

    assert!(!flattened_rules(parsed.game.program()).is_empty());
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
        sprites.sprite("Floor").unwrap().first_frame().size,
        Size3::new(5, 5, 5)
    );
    assert_eq!(
        sprites.sprite("Box").unwrap().first_frame().size,
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
    let rules = microban_basic_rules_with_input_guards(&flattened_rules(parsed.game.program()));
    let program = rules.into_iter().map(RuleStep3::Rule).collect::<Vec<_>>();
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

    let moved_down = transition_program(&parsed.game, &state, &program, INPUT_FORWARD).unwrap();
    let pushed_right =
        transition_program(&parsed.game, &moved_down, &program, INPUT_RIGHT).unwrap();

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
puzzle scoped_inputs {
dimension = 3
slots {
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

    assert_eq!(parsed.inputs.len(), 2);
    let right = parsed.input_by_name("right").unwrap();
    assert_eq!(right.id, INPUT_RIGHT);
    assert_eq!(right.direction, Some(Direction3::RIGHT));
    assert_eq!(right.keys, vec!["d", "ArrowRight"]);
    let restart = parsed.input_by_name("restart").unwrap();
    assert_eq!(restart.direction, None);
    assert_eq!(restart.keys, vec!["r"]);
}

#[test]
fn parser_rejects_non_arrow_3d_key_rows_through_shared_surface() {
    let err = parse_puzzle3d(
        r#"
puzzle scoped_inputs {
dimension = 3
slots {
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
        diagnostic_contains(&err, "keys row must use `->`"),
        "{err:?}"
    );
    assert_eq!(
        err.diagnostics()[0]
            .primary_span
            .as_ref()
            .and_then(|span| span.source_line.as_deref()),
        Some("d ArrowRight = right")
    );
}

#[test]
fn parser_accepts_front_back_as_canonical_3d_directions() {
    let parsed = parse_puzzle3d(
        r#"
puzzle front_back {
dimension = 3
slots {
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
        parsed.input_by_name("front").map(|input| input.id),
        Some(INPUT_FORWARD)
    );
    assert_eq!(
        parsed.input_by_name("back").map(|input| input.id),
        Some(INPUT_BACKWARD)
    );
    assert_eq!(flattened_rules(parsed.game.program()).len(), 4);
    assert_eq!(
        flattened_rules(parsed.game.program())[0].pattern.cells()[0].offset,
        Delta3::ZERO.into()
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[0].pattern.cells()[1].offset,
        Direction3::FORWARD.offset.into()
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[1].pattern.cells()[1].offset,
        Direction3::BACKWARD.offset.into()
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[2].guards,
        vec![Guard3::InputIs(INPUT_FORWARD)]
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[3].guards,
        vec![Guard3::InputIs(INPUT_BACKWARD)]
    );
}

#[test]
fn parser_keeps_forward_backward_as_3d_direction_aliases() {
    let parsed = parse_puzzle3d(
        r#"
puzzle legacy_forward_backward {
dimension = 3
slots {
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

    assert!(parsed.input_by_name("forward").is_none());
    assert!(parsed.input_by_name("backward").is_none());
    assert_eq!(
        parsed
            .input_by_name("front")
            .map(|input| input.keys.clone()),
        Some(vec!["w".to_string(), "ArrowUp".to_string()])
    );
    assert_eq!(
        parsed.input_by_name("back").map(|input| input.keys.clone()),
        Some(vec!["s".to_string(), "ArrowDown".to_string()])
    );
    assert_eq!(flattened_rules(parsed.game.program()).len(), 4);
    assert_eq!(
        flattened_rules(parsed.game.program())[0].pattern.cells()[1].offset,
        Direction3::FORWARD.offset.into()
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[1].pattern.cells()[1].offset,
        Direction3::BACKWARD.offset.into()
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[2].guards,
        vec![Guard3::InputIs(INPUT_FORWARD)]
    );
    assert_eq!(
        flattened_rules(parsed.game.program())[3].guards,
        vec![Guard3::InputIs(INPUT_BACKWARD)]
    );
}

#[test]
fn teneten3d_direction_input_changes_the_level_state() {
    let source = include_str!("fixtures/ten_horizontal_3d.puzzle3");
    let document = parse_game_for_path(source, "ten_horizontal_3d.puzzle3").unwrap();
    let Some(LoadedDocumentModel::Puzzle3d { puzzle, .. }) = document.single_model() else {
        panic!("TENETEN3D must compile as a 3D puzzle");
    };
    let bundle = puzzle.level_bundle.as_ref().unwrap();
    let initial = bundle.build_level_state(0).unwrap();
    let ten_objects = puzzle
        .object_labels
        .iter()
        .filter_map(|(object, label)| label.starts_with("TEN:").then_some(*object))
        .collect::<Vec<_>>();
    let ten_positions = |state: &State3| {
        occupied_cells(state)
            .into_iter()
            .filter_map(|(position, objects)| {
                objects
                    .iter()
                    .any(|object| ten_objects.contains(object))
                    .then_some(position)
            })
            .collect::<Vec<_>>()
    };

    // The 3D host maps screen ArrowUp/ArrowDown to front/back. The up/down
    // input names are the physical Z axis, not screen-relative controls.
    for input_name in ["left", "right", "front", "back"] {
        let input = puzzle.input_by_name(input_name).unwrap().id;
        let next =
            transition_program(&puzzle.game, &initial, puzzle.game.program(), input).unwrap();
        assert_ne!(
            ten_positions(&next),
            ten_positions(&initial),
            "{input_name} input must change TEN's position"
        );
    }
}
