use puzzle_core::{InputId, ObjectId, transition_program, transition_state};
use puzzle_lang::{
    SceneComponent, SceneEffect, VisualSpriteKind, parse_game2d as parse_game,
    translate_puzzlescript_to_canonical,
};

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
    assert!(translated.contains("button \"Play\" -> input confirm"));
    assert!(translated.contains("confirm <- Enter Space x"));
    assert!(translated.contains("back <- Escape q"));
    assert!(!translated.contains("\n  keys {"));
    assert!(translated.contains("if board.win_conditions -> {"));
    assert!(!translated.contains("board.level.has_next"));
}

#[test]
fn translated_basic_vanilla_puzzlescript_parses_as_loaded_game() {
    let source = include_str!("fixtures/puzzlescript/basic_sokoban.ps");
    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    let loaded = parse_game(&translated).unwrap();

    assert_eq!(loaded.levels.len(), 1);
    assert_eq!(loaded.levels[0].name, "unnamed level 1");
    assert_eq!(loaded.scenes[0].name, "title");
    assert_eq!(loaded.scenes[1].name, "playing");
    assert!(!loaded.scenes[1].key_bindings.is_empty());
    assert!(
        loaded.scenes[0]
            .components
            .iter()
            .any(|component| matches!(component, SceneComponent::Button(_)))
    );
    let play_button = loaded.scenes[0]
        .components
        .iter()
        .find_map(|component| {
            if let SceneComponent::Button(button) = component {
                Some(button)
            } else {
                None
            }
        })
        .expect("expected title play button");
    assert_eq!(
        play_button.effect,
        SceneEffect::Input("confirm".to_string())
    );
    assert!(loaded.scenes[1].components.iter().any(|component| matches!(
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

    assert!(translated.contains("button \"Play\" -> input confirm"));
    assert!(translated.contains("confirm <- Enter Space x"));
    assert!(
        translated.contains(
            "if input == confirm -> {\n      sfx startgame\n      start levels in playing"
        )
    );

    let loaded = parse_game(&translated).unwrap();
    let button = loaded.scenes[0]
        .components
        .iter()
        .find_map(|component| {
            if let SceneComponent::Button(button) = component {
                Some(button)
            } else {
                None
            }
        })
        .expect("expected title play button");
    assert_eq!(button.effect, SceneEffect::Input("confirm".to_string()));
    assert_eq!(
        loaded.scenes[0].key_bindings[0].effect,
        SceneEffect::Input("confirm".to_string())
    );
    assert!(loaded.scenes[0].transitions.iter().any(|transition| {
        matches!(
            &transition.effect,
            SceneEffect::Sequence(effects)
                if matches!(
                    effects.as_slice(),
                    [
                        SceneEffect::PlaySfx { name },
                        SceneEffect::StartLevel { scene, scope: None }
                    ] if name == "startgame" && scene == "playing"
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

    assert!(translated.contains("group {\n  player = Player_u Player_d\n}"));
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
    assert!(translated.contains("button \"Play\" -> input confirm"));
    assert!(translated.contains("confirm <- Enter Space x"));
    assert!(translated.contains("back <- Escape q"));
    assert!(!translated.contains("\n  keys {"));
    assert!(!translated.contains("if board.win_conditions -> {"));

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

    assert!(translated.contains("group {\n  crate = Crate1 Crate2 Crate3"));
    assert!(translated.contains("  1 = Crate1"));
    assert!(translated.contains("  , = nospawn"));
    assert!(translated.contains("var __ps_again = false"));
    assert!(translated.contains("repeat until __ps_again == false"));
    assert!(translated.contains("[ Player{up} ] -> [ Player{up} slideup ] set __ps_again = true"));
    assert!(translated.contains(
        "[ slideup ] [ crate{no up} ] -> [ slideup ] [ crate{up} ] set __ps_again = true"
    ));
    assert!(translated.contains("[ crate{>} | obs{no directions} ] -> [ crate | obs ]"));
    assert!(
        translated
            .contains("[ Crate1{directions} | Crate1{no directions} ] -> [ Crate1 | Crate1 ]")
    );
    assert!(
        translated.contains("repeat {\n      [ crate{>} | obs{no directions} ] -> [ crate | obs ]")
    );
    assert!(translated.contains("    move\n    [ Target crate ] -> [ Target ]"));
    assert!(!translated.contains(" again"));
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
            .any(|variable| variable.name == "bg" && variable.value == "#000000")
    );
    assert!(
        loaded
            .theme
            .variables
            .iter()
            .any(|variable| variable.name == "ink" && variable.value == "#9CBD0F")
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
