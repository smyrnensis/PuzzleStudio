use puzzle_core::{InputId, ObjectId, transition_program, transition_state};
use puzzle_lang::{
    LoadedGame, SceneComponent, SceneEffect, VisualSpriteKind, parse_game2d as parse_game,
    translate_puzzlescript_to_canonical,
};

fn find_choice_by_label<'a>(
    components: &'a [SceneComponent],
    label: &str,
) -> Option<&'a puzzle_lang::SceneButtonDef> {
    for component in components {
        match component {
            SceneComponent::Choice(choice)
                if choice.label == puzzle_lang::SceneExpr::Text(label.to_string()) =>
            {
                return Some(choice);
            }
            SceneComponent::Row(container)
            | SceneComponent::Column(container)
            | SceneComponent::Box(container) => {
                if let Some(choice) = find_choice_by_label(&container.children, label) {
                    return Some(choice);
                }
            }
            SceneComponent::Conditional(conditional) => {
                if let Some(choice) = find_choice_by_label(&conditional.children, label) {
                    return Some(choice);
                }
                if let Some(choice) = find_choice_by_label(&conditional.else_children, label) {
                    return Some(choice);
                }
            }
            SceneComponent::For(for_view) => {
                if let Some(choice) = find_choice_by_label(&for_view.children, label) {
                    return Some(choice);
                }
            }
            _ => {}
        }
    }
    None
}

fn object_id_by_label(loaded: &LoadedGame, label: &str) -> ObjectId {
    loaded
        .object_labels
        .iter()
        .find_map(|(id, existing)| (existing == label).then_some(*id))
        .unwrap_or_else(|| panic!("expected object `{label}`"))
}

fn scene_by_name<'a>(loaded: &'a LoadedGame, name: &str) -> &'a puzzle_lang::SceneDef {
    loaded
        .scenes
        .iter()
        .find(|scene| scene.name == name)
        .unwrap_or_else(|| panic!("expected scene `{name}`"))
}

fn assert_imported_output_uses_current_canonical_surface(source: &str) {
    parse_game(source).expect("imported source should parse");

    assert!(!source.contains('\t'), "generated output must use spaces");
    for forbidden in [
        "\nobjects {",
        "\ncollisionlayers",
        "\ntransitions {",
        "\nmain {",
        "\ninputs {",
        "\nrule ",
        "-> effect ",
        "play_sfx ",
        "board.next_level",
        "board.level.has_next",
        "level imported_",
    ] {
        assert!(
            !source.to_ascii_lowercase().contains(forbidden),
            "generated output contains non-canonical surface `{forbidden}`:\n{source}"
        );
    }

    for line in source.lines() {
        let trimmed = line.trim_start();
        let lowercase = trimmed.to_ascii_lowercase();
        if trimmed.starts_with("message \"") {
            continue;
        }
        assert!(
            !matches!(
                lowercase.split_whitespace().next(),
                Some("objects" | "collisionlayers" | "action" | "emit" | "do" | "random" | "late")
            ),
            "generated output leaked non-canonical line: {line}"
        );
        for leaked in [
            " moving ",
            " stationary ",
            "{moving}",
            "{stationary}",
            "Horizontal [",
            "Vertical [",
            "Orthogonal [",
            "Perpendicular ",
            "Parallel ",
        ] {
            assert!(
                !line.contains(leaked),
                "generated output leaked PuzzleScript token `{leaked}` in line: {line}"
            );
        }
    }
}

#[test]
fn translates_basic_vanilla_puzzlescript_to_canonical_fixture() {
    let source = include_str!("fixtures/puzzlescript/basic_sokoban.ps");
    let expected = include_str!("fixtures/puzzlescript/basic_sokoban.puzzle");

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert_eq!(translated.trim_end(), expected.trim_end());
    assert!(!translated.contains("\nobjects {"));
    assert!(!translated.contains('\t'));
    assert!(!translated.contains("Player {"));
    assert!(!translated.contains("level imported_1 {"));
    assert!(!translated.contains("-> effect "));
    assert!(
        translated
            .contains("if has_progress_save {\n      choice \"Continue\" -> input continue_game")
    );
    assert!(translated.contains("choice \"New Game\" -> input new_game"));
    assert!(translated.contains("Enter Space x -> continue_game"));
    assert!(translated.contains("n -> new_game"));
    assert!(translated.contains("Escape q -> back"));
    assert!(translated.contains("routine continue_game {\n    goto playing"));
    assert!(translated.contains("routine new_game {\n    clear_game_progress"));
    assert!(translated.contains("routine back {\n    goto title"));
    assert!(translated.contains("on_level_clear {\n  wait 0.3s\n  next_level\n}"));
    assert!(!translated.contains("board.next_level"));
    assert!(!translated.contains("board.level.has_next"));
}

#[test]
fn translated_basic_vanilla_puzzlescript_parses_as_loaded_game() {
    let source = include_str!("fixtures/puzzlescript/basic_sokoban.ps");
    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    let loaded = parse_game(&translated).unwrap();

    assert_eq!(loaded.levels.len(), 1);
    assert_eq!(loaded.levels[0].name, "unnamed level 1");
    let title_scene = scene_by_name(&loaded, "title");
    let playing_scene = scene_by_name(&loaded, "playing");
    assert!(!playing_scene.key_bindings.is_empty());
    assert!(
        title_scene
            .components
            .iter()
            .any(|component| matches!(component, SceneComponent::Choice(_)))
    );
    let continue_button = find_choice_by_label(&title_scene.components, "Continue")
        .expect("expected title continue choice");
    assert_eq!(
        continue_button.effect,
        SceneEffect::Input("continue_game".to_string())
    );
    let new_game_button = find_choice_by_label(&title_scene.components, "New Game")
        .expect("expected title new game choice");
    assert_eq!(
        new_game_button.effect,
        SceneEffect::Input("new_game".to_string())
    );
    assert!(playing_scene.components.iter().any(|component| matches!(
        component,
        SceneComponent::Frame(frame) if frame.kind == "puzzle" && frame.source == "board"
    )));
    assert!(
        loaded
            .visuals
            .sprites
            .iter()
            .any(|sprite| sprite.name == "Player")
    );
    assert!(
        loaded
            .visuals
            .sprites
            .iter()
            .any(|sprite| sprite.name == "Crate")
    );
    assert_eq!(
        loaded.goal.as_ref().map(|goal| goal.description.as_str()),
        Some("all Target on Crate")
    );
}

#[test]
fn puzzlescript_again_interval_lowers_to_canonical_default_again_ms() {
    let source = r#"
Again Interval
again_interval 0.1

OBJECTS
Player
red

LEGEND
P = Player

COLLISIONLAYERS
Player

RULES

WINCONDITIONS
some Player

LEVELS
P
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();
    assert!(translated.contains("again_interval = 0.1s"));

    let loaded = parse_game(&translated).unwrap();
    assert_eq!(loaded.default_again_ms, 100);
}

#[test]
fn group_selector_intersection_filters_impossible_same_layer_tuples() {
    let source = r#"
title "Group Intersection"

puzzle main {
layers {
  floor = Background
  actor = A B
  payload = C
}

groups {
  G = A C
  H = A B
}

levels {
legend {
  . = empty
}
.
}

rules {
[ G H ] -> [ ]
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let a = object_id_by_label(&loaded, "A");
    let b = object_id_by_label(&loaded, "B");
    let c = object_id_by_label(&loaded, "C");

    let requirements = loaded
        .game
        .rules()
        .iter()
        .filter_map(|rule| {
            rule.pattern
                .components
                .first()
                .and_then(|component| component.cells.first())
                .map(|cell| cell.require_objects.clone())
        })
        .collect::<Vec<_>>();

    assert_eq!(requirements.len(), 2);
    assert!(requirements.iter().any(|objects| objects == &[c, a]));
    assert!(requirements.iter().any(|objects| objects == &[c, b]));
    assert!(!requirements.iter().any(|objects| objects == &[a]));
    assert!(!requirements.iter().any(|objects| objects == &[a, b]));
}

#[test]
fn puzzlescript_color_only_object_lowers_to_solid_sprite() {
    let source = r##"
Color Only

OBJECTS

Background
#9CBD0F

Player
#0F380F
00000
00000
00000
00000
00000

LEGEND
. = Background
P = Player

COLLISIONLAYERS
Background
Player

RULES

WINCONDITIONS
some Player

LEVELS
P
"##;
    let translated = translate_puzzlescript_to_canonical(source).unwrap();
    assert!(translated.contains("  Background\n    #9CBD0F\n\n  Player"));
    assert!(!translated.contains("Background\n    #9CBD0F\n    00000"));
    assert!(
        translated.contains("on_level_start {\n  once_all [ no Background ] -> [ Background ]\n}")
    );
    assert!(translated.contains("  P = Player"));
    assert!(!translated.contains("  P = Background Player"));

    let loaded = parse_game(&translated).unwrap();
    let background_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Background")
        .expect("expected imported Background sprite");
    assert!(matches!(
        &background_sprite.kind,
        VisualSpriteKind::Solid(color) if color == "#9CBD0F"
    ));
}

#[test]
fn puzzlescript_object_name_with_digit_after_sprite_is_not_imported_as_sprite_row() {
    let source = r##"
title Digit Object Boundary

OBJECTS

Background
black

pcrate1
purple
00000
0...0
0...0
0...0
00000

pcrate2
yellow
00000
0...0
0...0
0...0
00000

Player
white

LEGEND
. = Background
1 = pcrate1
2 = pcrate2
P = Player

COLLISIONLAYERS
Background
pcrate1, pcrate2
Player

RULES

LEVELS
P12
"##;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains(
        "  pcrate1\n    #800080\n    00000\n    0...0\n    0...0\n    0...0\n    00000\n\n  pcrate2"
    ));
    assert!(!translated.contains("00000\n    pcrate2"));
    parse_game(&translated).unwrap();
}

#[test]
fn puzzlescript_flickscreen_import_keeps_cell_viewport_out_of_scene_layout() {
    let source = r##"
title Flick Fit
flickscreen 13x13

OBJECTS

Background
black

Player
white

LEGEND
. = Background
P = Player

COLLISIONLAYERS
Background
Player

RULES

LEVELS
P
"##;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(
        translated.contains("puzzle main {\nflickscreen 13 13\nscreen_focus Player\n\nlayers {")
    );
    assert!(translated.contains("  layout {\n    puzzle board\n  }"));
    assert!(!translated.contains("layout size 13 13"));
    assert!(!translated.contains("puzzle board size 13 13"));
    assert!(!translated.contains("      title \"Flick Fit\""));
    parse_game(&translated).unwrap();
}

#[test]
fn puzzlescript_startgame_sound_lowers_to_title_start_sequence() {
    let source = r##"
title Start Sound

SOUNDS
startgame 12345

OBJECTS
Background
#000000

Player
#ffffff
00000
00000
00000
00000
00000

LEGEND
. = Background
P = Player

COLLISIONLAYERS
Background
Player

RULES

LEVELS
P
"##;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(
        translated
            .contains("if has_progress_save {\n      choice \"Continue\" -> input continue_game")
    );
    assert!(translated.contains("choice \"New Game\" -> input new_game"));
    assert!(translated.contains("Enter Space x -> continue_game"));
    assert!(translated.contains("n -> new_game"));
    assert!(translated.contains("routine continue_game {\n    sfx startgame\n    goto playing"));
    assert!(translated.contains(
        "routine new_game {\n    sfx startgame\n    clear_game_progress\n    goto playing(0)"
    ));

    let loaded = parse_game(&translated).unwrap();
    let title_scene = scene_by_name(&loaded, "title");
    let button = find_choice_by_label(&title_scene.components, "Continue")
        .expect("expected title continue choice");
    assert_eq!(
        button.effect,
        SceneEffect::Input("continue_game".to_string())
    );
    assert_eq!(
        title_scene.key_bindings[0].effect,
        SceneEffect::RoutineCall("continue_game".to_string())
    );
    assert!(title_scene.routines.iter().any(|routine| {
        matches!(
            &routine.effect,
            SceneEffect::Sequence(effects)
                if matches!(
                    effects.as_slice(),
                    [
                        SceneEffect::PlaySfx { name },
                        SceneEffect::Goto { scene, params }
                    ] if name == "startgame" && scene == "playing" && params.is_empty()
                )
        )
    }));
    assert!(title_scene.routines.iter().any(|routine| {
        matches!(
            &routine.effect,
            SceneEffect::Sequence(effects)
                if matches!(
                    effects.as_slice(),
                    [
                        SceneEffect::PlaySfx { name },
                        SceneEffect::ClearGameProgress,
                        SceneEffect::Goto { scene, params }
                    ] if name == "startgame"
                        && scene == "playing"
                        && params.len() == 1
                )
        )
    }));
}

#[test]
fn routine_once_does_not_force_inner_rewrites_to_once() {
    let source = r#"
title routine_once_repeat_fixture

puzzle main {
layers {
  layer_1 = A
}

routine spread once {
  [ A | ] -> [ A | A ]
}

rules {
  spread
}

levels {
  legend {
    A = A
    . = empty
  }

  A...
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let object = object_named(&loaded, "A");

    let moved = transition_program(
        &loaded.game,
        loaded.game.program(),
        &loaded.levels[0].initial_state,
        InputId(0),
    )
    .unwrap();

    assert_eq!(moved.object_count(object), 4);
}

#[test]
fn translated_basic_vanilla_puzzlescript_uses_player_move_bridge() {
    let source = include_str!("fixtures/puzzlescript/basic_sokoban.ps");
    let translated = translate_puzzlescript_to_canonical(source).unwrap();
    let loaded = parse_game(&translated).unwrap();
    assert!(!loaded.scenes[1].key_bindings.is_empty());
    let right = input_named(&loaded, "right");
    let player = object_named(&loaded, "Player");

    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();

    assert!(moved.has_object(&loaded.game, 2, 1, player));
}

#[test]
fn translated_basic_vanilla_puzzlescript_uses_prefixless_move_marker_rule() {
    let source = include_str!("fixtures/puzzlescript/basic_sokoban.ps");
    let translated = translate_puzzlescript_to_canonical(source).unwrap();
    assert!(translated.contains("[ Player{>} | Crate ] -> [ Player{>} | Crate{>} ]"));
    assert!(!translated.contains("once_all [ Player{>} | Crate ]"));
    assert!(!translated.contains("directions [ Player{>} | Crate ]"));

    let loaded = parse_game(&translated).unwrap();
    let right = input_named(&loaded, "right");
    let player = object_named(&loaded, "Player");
    let crate_object = object_named(&loaded, "Crate");

    let moved_once =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let moved_twice = transition_state(&loaded.game, &moved_once, right).unwrap();

    assert!(moved_twice.has_object(&loaded.game, 3, 1, player));
    assert!(moved_twice.has_object(&loaded.game, 4, 1, crate_object));
}

#[test]
fn puzzlescript_player_move_bridge_uses_player_alias_when_no_player_object_exists() {
    let source = r#"
title Alias Player

========
OBJECTS
========

Background
green

Player_u
orange

Player_d
orange

=======
LEGEND
=======

player = Player_u or Player_d
. = Background
P = Player_u

================
COLLISIONLAYERS
================

Background
Player_u, Player_d

======
RULES
======

[ > player ] -> [ > player ]

=======
LEVELS
=======

P
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains("groups {\n  player = Player_u Player_d\n}"));
    assert!(translated.contains("input directions [ player ] -> [ player{>} ]"));
    assert!(!translated.contains("input directions [ Player ]"));
    parse_game(&translated).unwrap();
}

#[test]
fn puzzlescript_some_win_condition_resolves_case_insensitive_object_name() {
    let source = r#"
title Some Dog

========
OBJECTS
========

Background
green

Dog
brown

=======
LEGEND
=======

. = Background
p = Dog
Player = Dog

================
COLLISIONLAYERS
================

Background
Dog

==============
WINCONDITIONS
==============

some dog

=======
LEVELS
=======

p
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains("win_conditions {\n  some Dog\n}"));
    parse_game(&translated).unwrap();
}

#[test]
fn puzzlescript_win_effect_advances_without_win_conditions() {
    let source = r#"
title Rule Win

========
OBJECTS
========

Background
black

Player
blue

Exit
green

=======
LEGEND
=======

. = Background
P = Player
X = Player Exit

================
COLLISIONLAYERS
================

Background
Exit
Player

======
RULES
======

late [ Player Exit ] -> win

==============
WINCONDITIONS
==============

=======
LEVELS
=======

X

P
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(!translated.contains("win_conditions {"));
    assert!(translated.contains("on_level_clear {\n  wait 0.3s\n  next_level\n}"));
    let loaded = parse_game(&translated).unwrap();
    assert!(loaded.level_clear_program.is_some());
}

#[test]
fn puzzlescript_parenthetical_comment_lines_are_not_imported_as_rules() {
    let source = r#"
title Parenthetical Comment

========
OBJECTS
========

Background
green

Player
orange

=======
LEGEND
=======

. = Background
P = Player

================
COLLISIONLAYERS
================

Background
Player

======
RULES
======

( [ Player ] -> cancel )
[ Player ] -> [ Player ]

=======
LEVELS
=======

P
(
choose 1 [ ] -> [ Player ]
)
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(!translated.contains("( [ Player ] -> cancel )"));
    assert!(!translated.contains("choose 1"));
    parse_game(&translated).unwrap();
}

#[test]
fn translates_official_sumo_demo_with_disconnected_pattern() {
    let source = include_str!("fixtures/puzzlescript/official_sumo.ps");
    let expected = include_str!("fixtures/puzzlescript/official_sumo.puzzle");

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert_eq!(translated.trim_end(), expected.trim_end());
    assert!(translated.contains("[ Player{>} ] [ Sumo ] -> [ Player{>} ] [ Sumo{>} ]"));
    assert!(!translated.contains("win_conditions {"));
    assert!(!translated.contains("-> effect "));
    assert!(
        translated
            .contains("if has_progress_save {\n      choice \"Continue\" -> input continue_game")
    );
    assert!(translated.contains("choice \"New Game\" -> input new_game"));
    assert!(translated.contains("Enter Space x -> continue_game"));
    assert!(translated.contains("n -> new_game"));
    assert!(translated.contains("Escape q -> back"));
    assert!(translated.contains("routine continue_game {\n    goto playing"));
    assert!(translated.contains("routine new_game {\n    clear_game_progress"));
    assert!(translated.contains("routine back {\n    goto title"));
    assert!(!translated.contains("if board.win_conditions -> {"));
    assert!(translated.contains("on_level_clear {\n  wait 0.3s\n  next_level\n}"));

    let loaded = parse_game(&translated).unwrap();
    let right = input_named(&loaded, "right");
    let player = object_named(&loaded, "Player");
    let sumo = object_named(&loaded, "Sumo");

    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();

    assert!(moved.has_object(&loaded.game, 3, 3, player));
    assert!(moved.has_object(&loaded.game, 6, 3, sumo));
}

#[test]
fn translates_official_simple_block_sliding_with_groups_and_again_effects() {
    let source = include_str!("fixtures/puzzlescript/official_simple_block_sliding.ps");

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains("groups {\n  crate = Crate1 Crate2 Crate3"));
    assert!(translated.contains("  1 = Crate1"));
    assert!(translated.contains("  , = nospawn"));
    assert!(!translated.contains("var __ps_again"));
    assert!(!translated.contains("repeat until __ps_again"));
    assert!(translated.contains("[ Player{up} ] -> [ Player{up} slideup ] again"));
    assert!(translated.contains("[ slideup ] [ crate ] -> [ slideup ] [ crate{up} ] again"));
    assert!(translated.contains("[ crate{>} | obs{no directions} ] -> [ crate | obs ]"));
    assert!(
        translated
            .contains("[ Crate1{directions} | Crate1{no directions} ] -> [ Crate1 | Crate1 ]")
    );
    assert!(
        translated.contains("repeat {\n    [ crate{>} | obs{no directions} ] -> [ crate | obs ]")
    );
    assert!(translated.contains("  move\n  [ Target crate ] -> [ Target ]"));
    assert!(translated.contains(" again"));
    assert!(!translated.contains("level imported_"));
    assert!(!translated.contains("\nobjects {"));
    assert!(!translated.contains("late ["));
    assert!(!translated.contains(" moving "));
    assert!(!translated.contains("{moving}"));
    assert!(!translated.contains("stationary"));

    let loaded = parse_game(&translated).unwrap();
    assert!(loaded.levels.len() > 1);
    assert!(loaded.object_labels.values().any(|label| label == "Crate3"));

    let down = input_named(&loaded, "down");
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, down).unwrap();
    assert_ne!(moved, loaded.levels[0].initial_state);
}

#[test]
fn imports_supported_official_gallery_samples_as_current_canonical_syntax() {
    for (name, source) in [
        (
            "2d_whale_world",
            include_str!("fixtures/puzzlescript/gallery/2d_whale_world.ps"),
        ),
        (
            "icecrates",
            include_str!("fixtures/puzzlescript/gallery/icecrates.ps"),
        ),
        (
            "lime_rick",
            include_str!("fixtures/puzzlescript/gallery/lime_rick.ps"),
        ),
        (
            "mad_queens",
            include_str!("fixtures/puzzlescript/gallery/mad_queens.ps"),
        ),
        (
            "microban",
            include_str!("fixtures/puzzlescript/gallery/microban.ps"),
        ),
        (
            "midas",
            include_str!("fixtures/puzzlescript/gallery/midas.ps"),
        ),
        (
            "quantum_rails",
            include_str!("fixtures/puzzlescript/gallery/quantum_rails.ps"),
        ),
        (
            "transition",
            include_str!("fixtures/puzzlescript/gallery/transition.ps"),
        ),
    ] {
        let translated = translate_puzzlescript_to_canonical(source)
            .unwrap_or_else(|err| panic!("{name} should translate: {err}"));
        assert_imported_output_uses_current_canonical_surface(&translated);
    }
}

#[test]
fn puzzlescript_random_rules_fail_at_import_boundary() {
    let source = include_str!("fixtures/puzzlescript/gallery/collapse.ps");

    let error = translate_puzzlescript_to_canonical(source)
        .expect_err("collapse uses random rules and should be rejected")
        .to_string();

    assert!(
        error.contains("PuzzleScript random rules are not supported by this importer"),
        "{error}"
    );
}

#[test]
fn translates_run_rules_on_level_start_to_shared_routine_and_lifecycle() {
    let source = r#"
title Level Start Setup
author increpare
run_rules_on_level_start

========
OBJECTS
========

Background
black

Player
blue

Wall
gray

=======
LEGEND
=======

. = Background
P = Player
# = Wall

================
COLLISIONLAYERS
================

Background
Player, Wall

======
RULES
======

[ Wall ] -> [ Player ]

=======
LEVELS
=======

#
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains("routine __ps_main once {\n  [ Wall ] -> [ Player ]\n  move\n}"));
    assert!(translated.contains("on_level_start {\n"));
    assert!(translated.contains("  __ps_main\n}"));
    assert!(
        translated
            .contains("rules {\n  input directions [ Player ] -> [ Player{>} ]\n  __ps_main\n}")
    );
    assert!(!translated.contains("run_rules_on_level_start"));

    let loaded = parse_game(&translated).unwrap();
    let player = object_named(&loaded, "Player");
    let wall = object_named(&loaded, "Wall");
    let started = transition_program(
        &loaded.game,
        loaded.level_start_program.as_deref().unwrap(),
        &loaded.levels[0].initial_state,
        InputId(0),
    )
    .unwrap();

    assert!(started.has_object(&loaded.game, 0, 0, player));
    assert!(!started.has_object(&loaded.game, 0, 0, wall));
}

#[test]
fn run_rules_on_level_start_keeps_ps_rules_repeating_and_late_after_move() {
    let source = r#"
title Laser Setup
run_rules_on_level_start

========
OBJECTS
========

Background
black

Player
blue

Emitter
red

Beam
yellow

=======
LEGEND
=======

. = Background
P = Player
E = Emitter
B = Beam

================
COLLISIONLAYERS
================

Background
Player, Emitter
Beam

======
RULES
======

late right [ Emitter | no Beam ] -> [ Emitter | Beam ]
late right [ Beam | no Beam ] -> [ Beam | Beam ]

=======
LEVELS
=======

E..
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains("routine __ps_main once {\n  move\n  right [ Emitter | no Beam ]"));
    let loaded = parse_game(&translated).unwrap();
    let beam = object_named(&loaded, "Beam");
    let started = transition_program(
        &loaded.game,
        loaded.level_start_program.as_deref().unwrap(),
        &loaded.levels[0].initial_state,
        InputId(0),
    )
    .unwrap();

    assert!(started.has_object(&loaded.game, 1, 0, beam));
    assert!(started.has_object(&loaded.game, 2, 0, beam));
}

#[test]
fn puzzlescript_object_names_starting_with_v_are_not_direction_markers() {
    let source = r#"
title V Object

========
OBJECTS
========

Background
black

Player
blue

victimSolo
green

pushBlock
gray

=======
LEGEND
=======

. = Background
P = Player
v = victimSolo
B = pushBlock

================
COLLISIONLAYERS
================

Background
Player, victimSolo, pushBlock

======
RULES
======

[ > victimSolo | pushBlock ] -> [ > victimSolo | > pushBlock ]

=======
LEVELS
=======

vB.
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(
        translated.contains("[ victimSolo{>} | pushBlock ] -> [ victimSolo{>} | pushBlock{>} ]")
    );
    assert!(!translated.contains("v ictimSolo"));
    parse_game(&translated).unwrap();
}

#[test]
fn puzzlescript_sfx_suffix_lowers_to_canonical_sfx_effect() {
    let source = r#"
title SFX Suffix

========
OBJECTS
========

Background
black

Player
blue

Wall
gray

=======
LEGEND
=======

. = Background
P = Player
# = Wall

================
COLLISIONLAYERS
================

Background
Player, Wall

======
RULES
======

[ Player | Wall ] -> [ Player | ] SFX1

=======
LEVELS
=======

P#
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains("[ Player | Wall ] -> [ Player | ] sfx sfx1"));
    parse_game(&translated).unwrap();
}

#[test]
fn puzzlescript_sounds_named_seed_lowers_to_puzzlescript_sounds() {
    let source = r#"
title SFX Seed

========
OBJECTS
========

Background
black

Player
blue

Wall
gray

=======
LEGEND
=======

. = Background
P = Player
# = Wall

======
SOUNDS
======

sfx1 26 (heart)

================
COLLISIONLAYERS
================

Background
Player, Wall

======
RULES
======

[ Player | Wall ] -> [ Player | ] SFX1

=======
LEVELS
=======

P#
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains("sounds {\n  sfx sfx1 seed=26 type=puzzlescript\n}"));
    assert!(translated.contains("[ Player | Wall ] -> [ Player | ] sfx sfx1"));
    let parsed = parse_game(&translated).unwrap();
    assert_eq!(parsed.sounds.sfx.len(), 1);
    assert_eq!(parsed.sounds.sfx[0].name, "sfx1");
    assert_eq!(parsed.sounds.sfx[0].seed, "26");
    assert_eq!(parsed.sounds.sfx[0].type_target, "puzzlescript");
}

#[test]
fn puzzlescript_level_messages_are_not_imported_as_map_rows() {
    let source = r#"
title Level Messages

========
OBJECTS
========

Background
black

Player
blue

=======
LEGEND
=======

. = Background
P = Player

================
COLLISIONLAYERS
================

Background
Player

======
RULES
======

=======
LEVELS
=======

message hello
P

message goodbye
P
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains("message \"hello\""));
    assert!(translated.contains("message \"goodbye\""));
    let loaded = parse_game(&translated).unwrap();
    assert_eq!(loaded.levels.len(), 2);
    assert!(loaded.levels[0].level_start_program.is_some());
    assert!(loaded.levels[0].level_clear_program.is_none());
    assert!(loaded.levels[1].level_start_program.is_some());
}

#[test]
fn puzzlescript_prelude_colors_lower_to_theme_overrides() {
    let source = r#"
title Theme Colors
background_color black
text_color #9CBD0F

========
OBJECTS
========

Background
black

Player
blue

=======
LEGEND
=======

. = Background
P = Player

================
COLLISIONLAYERS
================

Background
Player

======
RULES
======

=======
LEVELS
=======

P
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(
        translated
            .contains("theme puzzlescript {\n  background_color #000000\n  text_color #9CBD0F\n}")
    );
    let loaded = parse_game(&translated).unwrap();
    assert_eq!(loaded.theme.name.as_deref(), Some("puzzlescript"));
    assert!(
        loaded
            .theme
            .variables
            .iter()
            .any(|variable| variable.name == "background" && variable.value == "#000000")
    );
    assert!(
        loaded
            .theme
            .variables
            .iter()
            .any(|variable| variable.name == "text" && variable.value == "#9CBD0F")
    );
}

fn input_named(loaded: &puzzle_lang::LoadedGame, name: &str) -> InputId {
    loaded
        .input_labels
        .iter()
        .find_map(|(input, label)| (label == name).then_some(*input))
        .unwrap()
}

fn object_named(loaded: &puzzle_lang::LoadedGame, name: &str) -> ObjectId {
    loaded
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == name).then_some(*object))
        .unwrap()
}
