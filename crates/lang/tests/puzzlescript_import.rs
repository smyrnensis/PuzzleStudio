use puzzle_core::{InputId, ObjectId, transition_program, transition_state};
use puzzle_lang::{
    LoadedGame, ModelOperationSound, ModelOperationSoundDef, SceneComponent, SceneEffect,
    VisualSpriteKind, parse_game2d as parse_game, translate_puzzlescript_to_canonical,
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

    assert!(
        !source.contains('\t'),
        "generated output must not contain tabs"
    );
    for line in source.lines() {
        assert_eq!(
            line,
            line.trim_start(),
            "generated output must not indent lines: {line}"
        );
    }
    for forbidden in [
        "\nobjects {",
        "\nsprite {",
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
        "w ArrowUp -> up",
        "s ArrowDown -> down",
        "a ArrowLeft -> left",
        "d ArrowRight -> right",
        "r -> restart",
        "choice \"Continue\" -> input",
        "choice \"New Game\" -> input",
        "Enter Space x -> continue_game",
        "n -> new_game",
        "layer_1 =",
        "\nscene =",
        "rotate from ",
        "__ps_",
    ] {
        let forbidden_lowercase = forbidden.to_ascii_lowercase();
        assert!(
            !source.to_ascii_lowercase().contains(&forbidden_lowercase),
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
        translated.contains("if has_progress_save {\nchoice \"Continue\" -> {\ngoto playing\n}")
    );
    assert!(translated.contains("choice \"New Game\" -> {\nclear_game_progress\nstart playing\n}"));
    assert!(!translated.contains("choice \"Continue\" -> input"));
    assert!(!translated.contains("choice \"New Game\" -> input"));
    assert!(!translated.contains("Enter Space x -> continue_game"));
    assert!(!translated.contains("n -> new_game"));
    assert!(!translated.contains("w ArrowUp -> up"));
    assert!(!translated.contains("r -> restart"));
    assert!(
        translated
            .contains("layout {\nrow {\nheading \"Basic PS Sokoban\"\n}\npuzzle board = main\n}")
    );
    assert!(translated.contains("rules {\nstep board\n}"));
    assert!(translated.contains("Escape q -> goto title"));
    assert!(!translated.contains("routine continue_game"));
    assert!(!translated.contains("routine new_game"));
    assert!(!translated.contains("routine back"));
    assert!(translated.contains("on_level_clear {\nwait 0.3s\nnext_level\n}"));
    assert!(!translated.contains("board.next_level"));
    assert!(!translated.contains("board.level.has_next"));
}

#[test]
fn puzzlescript_import_adds_background_to_every_level_legend_entry_without_empty() {
    let source = r#"
title Background Legend

OBJECTS

Background
black

Player
blue

LEGEND

. = Background
P = Background and Player

COLLISIONLAYERS

Background
Player

LEVELS

.P
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(
        translated.contains("levels {\nlegend {\n. = Background\nP = Player Background\n}"),
        "{translated}"
    );
    assert!(!translated.contains("= empty"));
    assert!(!translated.contains("once_all [ no Background ] -> [ Background ]"));
    parse_game(&translated).expect("background-legended imported game should parse");
}

#[test]
fn puzzlescript_import_does_not_inject_an_empty_legend_entry() {
    let source = r#"
title Explicit Legend Only

OBJECTS

Wall
gray

Player
blue

LEGEND

_ = Wall
P = Player

COLLISIONLAYERS

Wall
Player

LEVELS

_
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(
        translated.contains("legend {\n_ = Wall\nP = Player\n}"),
        "{translated}"
    );
    assert!(!translated.contains("= empty"), "{translated}");
    parse_game(&translated)
        .expect("explicit underscore legend should parse without injected empty");
}

#[test]
fn puzzlescript_import_remaps_semicolon_legend_chars() {
    let source = r#"
title Semicolon Legend

OBJECTS

Wall
gray

Player
blue

LEGEND

; = Wall
P = Player

COLLISIONLAYERS

Wall
Player

LEVELS

;;
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(!translated.contains("; = Wall"), "{translated}");
    assert!(translated.contains("A = Wall"), "{translated}");
    assert!(translated.contains("\nAA\n"), "{translated}");
    parse_game(&translated).expect("semicolon-remapped imported game should parse");
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
    assert!(matches!(
        &continue_button.effect,
        SceneEffect::Goto { scene, params } if scene == "playing" && params.is_empty()
    ));
    let new_game_button = find_choice_by_label(&title_scene.components, "New Game")
        .expect("expected title new game choice");
    assert!(matches!(
        &new_game_button.effect,
        SceneEffect::Sequence { effects } if matches!(
            effects.as_slice(),
            [
                SceneEffect::ClearGameProgress,
                SceneEffect::Sequence { effects: navigation }
            ] if matches!(
                navigation.as_slice(),
                [
                    SceneEffect::Reset { scene: reset_scene },
                    SceneEffect::Goto { scene: goto_scene, params }
                ] if reset_scene == "playing" && goto_scene == "playing" && params.is_empty()
            )
        )
    ));
    assert!(playing_scene.components.iter().any(|component| matches!(
        component,
        SceneComponent::Frame(frame) if frame.kind == "puzzle" && frame.source == "board"
    )));
    assert!(
        playing_scene
            .state
            .puzzles
            .iter()
            .any(|puzzle| { puzzle.name == "board" && puzzle.model == "main" })
    );
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
fn puzzlescript_level_select_prelude_generates_level_menu_scene() {
    let source = r#"
title Level Menu Import
level_select

OBJECTS
Background
#000000
.....
.....
.....
.....
.....

Player
#ffffff
00000
00000
00000
00000
00000

Goal
#00ff00
.....
.....
.....
.....
.....

LEGEND
. = Background
P = Player
G = Goal

COLLISIONLAYERS
Background
Player
Goal

RULES

WINCONDITIONS
all Goal on Player

LEVELS
P

G
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains("choice \"Level Select\" -> goto level_select"));
    assert!(translated.contains("Escape q -> goto level_select"));
    assert!(!translated.contains("Escape q -> goto title"));
    assert!(translated.contains("scene level_select {\nlayout {\nlevel_menu {"));
    assert!(translated.contains("show_index = true"));
    assert!(translated.contains("show_solved = true"));
    assert_imported_output_uses_current_canonical_surface(&translated);

    let loaded = parse_game(&translated).unwrap();
    let title_scene = scene_by_name(&loaded, "title");
    let level_select_scene = scene_by_name(&loaded, "level_select");
    let level_select_choice = find_choice_by_label(&title_scene.components, "Level Select")
        .expect("expected level select choice");
    assert!(matches!(
        &level_select_choice.effect,
        SceneEffect::Goto { scene, params } if scene == "level_select" && params.is_empty()
    ));
    assert!(
        level_select_scene
            .components
            .iter()
            .any(|component| matches!(component, SceneComponent::LevelMenu(_)))
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
title = "Group Intersection"

puzzle main {
slots {
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
    assert!(translated.contains("Background\n#9CBD0F"));
    assert!(!translated.contains("Background\n#9CBD0F\nshape ="));
    assert!(!translated.contains("once_all [ no Background ] -> [ Background ]"));
    assert!(translated.contains("P = Player Background"));
    assert!(!translated.contains("= empty"));

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

    assert!(translated.contains("pcrate1\n#800080\n00000\n0...0\n0...0\n0...0\n00000"));
    assert!(translated.contains("pcrate2\n#ffff00"));
    assert!(!translated.contains("00000\npcrate2"));
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
    assert!(translated.contains("layout {\npuzzle board = main\n}"));
    assert!(translated.contains("rules {\nstep board\n}"));
    assert!(!translated.contains("layout {\nmain\n}"));
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

    assert!(translated.contains(
        "if has_progress_save {\nchoice \"Continue\" -> {\nsfx startgame\ngoto playing\n}"
    ));
    assert!(translated.contains(
        "choice \"New Game\" -> {\nsfx startgame\nclear_game_progress\nstart playing\n}"
    ));
    assert!(!translated.contains("choice \"Continue\" -> input"));
    assert!(!translated.contains("choice \"New Game\" -> input"));
    assert!(!translated.contains("Enter Space x -> continue_game"));
    assert!(!translated.contains("n -> new_game"));
    assert!(!translated.contains("routine continue_game"));
    assert!(!translated.contains("routine new_game"));

    let loaded = parse_game(&translated).unwrap();
    let title_scene = scene_by_name(&loaded, "title");
    let button = find_choice_by_label(&title_scene.components, "Continue")
        .expect("expected title continue choice");
    assert!(matches!(
        &button.effect,
        SceneEffect::Sequence { effects } if matches!(
            effects.as_slice(),
            [SceneEffect::PlaySfx { name }, SceneEffect::Goto { scene, params }]
                if name == "startgame" && scene == "playing" && params.is_empty()
        )
    ));
    assert!(title_scene.key_bindings.is_empty());
    assert!(title_scene.routines.is_empty());
}

#[test]
fn puzzlescript_event_and_operation_sounds_lower_at_owned_boundaries() {
    let source = r##"
title Sound Operations

OBJECTS
Background
#000000
.....
.....
.....
.....
.....

Player
#ffffff
00000
00000
00000
00000
00000

Wall
#888888
00000
00000
00000
00000
00000

SOUNDS
player move 111
player cantmove 444
endlevel 555
undo 222
restart 333

LEGEND
. = Background
P = Player

COLLISIONLAYERS
Background
Player, Wall

RULES
[ > Player | no Wall ] -> [ > Player | ]

LEVELS
P..
...
...
"##;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains("sfx player_move { seed = 111; type = puzzlescript }"));
    assert!(translated.contains("sfx player_cantmove { seed = 444; type = puzzlescript }"));
    assert!(translated.contains("sfx endlevel { seed = 555; type = puzzlescript }"));
    assert!(translated.contains("sfx undo { seed = 222; type = puzzlescript }"));
    assert!(translated.contains("sfx restart { seed = 333; type = puzzlescript }"));
    assert!(translated.contains(
        "sounds {\nmove Player -> sfx player_move\nundo -> sfx undo\nrestart -> sfx restart\n}"
    ));
    assert!(!translated.contains("cantmove Player -> sfx"));
    assert!(translated.contains("once [ > Player | | < "));
    assert!(translated.contains("| | < Player ] -> sfx player_cantmove"));
    assert!(translated.contains("] -> sfx player_cantmove"));
    assert!(translated.contains("once [ > Player | ; | ^ "));
    assert!(translated.contains("| ; | ^ Player ] -> sfx player_cantmove"));
    assert!(translated.contains("once [ > Player ] -> sfx player_cantmove"));
    assert!(translated.contains("on_level_clear {\nsfx endlevel\nwait 0.3s\nnext_level\n}"));

    let loaded = parse_game(&translated).unwrap();
    assert_eq!(
        loaded.model_operation_sounds,
        vec![
            ModelOperationSoundDef {
                operation: ModelOperationSound::Undo,
                sfx_name: "undo".to_string(),
            },
            ModelOperationSoundDef {
                operation: ModelOperationSound::Restart,
                sfx_name: "restart".to_string(),
            },
        ]
    );
}

#[test]
fn routine_once_does_not_force_inner_rewrites_to_once() {
    let source = r#"
title = routine_once_repeat_fixture

puzzle main {
slots {
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
    assert!(translated.contains("routine move {\nrepeat {"));
    assert!(translated.contains("layer1 = Background"));
    assert!(translated.contains("for l in layer1 layer2 layer3 {"));
    let loaded = parse_game(&translated).unwrap();
    assert!(
        loaded
            .scenes
            .iter()
            .any(|scene| !scene.key_bindings.is_empty())
    );
    let right = input_named(&loaded, "right");
    let player = object_named(&loaded, "Player");

    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();

    assert!(moved.has_object(&loaded.game, 2, 1, player));
}

#[test]
fn translated_basic_vanilla_puzzlescript_uses_prefixless_move_marker_rule() {
    let source = include_str!("fixtures/puzzlescript/basic_sokoban.ps");
    let translated = translate_puzzlescript_to_canonical(source).unwrap();
    assert!(translated.contains("[ > Player | Crate ] -> [ > Player | > Crate ]"));
    assert!(!translated.contains("once_all [ > Player | Crate ]"));
    assert!(!translated.contains("directions [ > Player | Crate ]"));

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

    assert!(translated.contains("groups {\nplayer = Player_u Player_d\n}"));
    assert!(translated.contains("input [ player ] -> [ > player ]"));
    assert!(!translated.contains("input [ Player ]"));
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

    assert!(translated.contains("win_conditions {\nsome Dog\n}"));
    parse_game(&translated).unwrap();
}

#[test]
fn puzzlescript_case_sensitive_names_preserve_group_object_case_distinction() {
    let source = r#"
case_sensitive
title Case Sensitive Groups

========
OBJECTS
========

Background
black

Crate
brown

CrateW
blue

TargetCrate
yellow

Player
white

=======
LEGEND
=======

. = Background
C = Crate
W = CrateW
t = TargetCrate
P = Player
crate = Crate or CrateW

================
COLLISIONLAYERS
================

Background
crate
TargetCrate
Player

======
RULES
======

[ TargetCrate crate ] -> [ TargetCrate crate ]

==============
WINCONDITIONS
==============

all TargetCrate on crate

=======
LEVELS
=======

PtC
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(translated.contains("groups {\ncrate = Crate CrateW\n}"));
    assert!(translated.contains("all TargetCrate on crate"));
    assert!(translated.contains("[ TargetCrate crate ] -> [ TargetCrate crate ]"));
    assert!(!translated.contains("all TargetCrate on Crate"));
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
    assert!(translated.contains("on_level_clear {\nwait 0.3s\nnext_level\n}"));
    let loaded = parse_game(&translated).unwrap();
    assert!(loaded.level_clear_program.is_some());
}

#[test]
fn puzzlescript_comments_are_preserved_without_becoming_rules() {
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

[ Player ] -> [ Player ] // ordinary movement
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

    assert!(
        translated.contains("// [RULES line 32] ordinary movement"),
        "{translated}"
    );
    assert!(
        translated.contains("// [RULES line 33] [ Player ] -> cancel"),
        "{translated}"
    );
    assert!(
        translated.contains("// [LEVELS line 41] choose 1 [ ] -> [ Player ]"),
        "{translated}"
    );
    assert!(!translated.contains("( [ Player ] -> cancel )"));
    parse_game(&translated).unwrap();
}

#[test]
fn translates_official_sumo_demo_with_disconnected_pattern() {
    let source = include_str!("fixtures/puzzlescript/official_sumo.ps");
    let expected = include_str!("fixtures/puzzlescript/official_sumo.puzzle");

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert_eq!(translated.trim_end(), expected.trim_end());
    assert!(translated.contains("[ > Player ] [ Sumo ] -> [ > Player ] [ > Sumo ]"));
    assert!(!translated.contains("win_conditions {"));
    assert!(!translated.contains("-> effect "));
    assert!(
        translated.contains("if has_progress_save {\nchoice \"Continue\" -> {\ngoto playing\n}")
    );
    assert!(translated.contains("choice \"New Game\" -> {\nclear_game_progress\nstart playing\n}"));
    assert!(!translated.contains("choice \"Continue\" -> input"));
    assert!(!translated.contains("choice \"New Game\" -> input"));
    assert!(!translated.contains("Enter Space x -> continue_game"));
    assert!(!translated.contains("n -> new_game"));
    assert!(translated.contains("Escape q -> goto title"));
    assert!(!translated.contains("routine continue_game"));
    assert!(!translated.contains("routine new_game"));
    assert!(!translated.contains("routine back"));
    assert!(!translated.contains("if board.win_conditions -> {"));
    assert!(translated.contains("on_level_clear {\nwait 0.3s\nnext_level\n}"));

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

    assert!(translated.contains("groups {\ncrate = Crate1 Crate2 Crate3"));
    assert!(translated.contains("1 = Crate1"));
    assert!(translated.contains(", = nospawn"));
    assert!(!translated.contains("var __ps_again"));
    assert!(!translated.contains("repeat until __ps_again"));
    assert!(translated.contains("[ up Player ] -> [ up Player slideup ] again"));
    assert!(translated.contains("[ slideup ] [ crate ] -> [ slideup ] [ up crate ] again"));
    assert!(translated.contains("[ > crate | obs{no directions} ] -> [ crate | obs ]"));
    assert!(
        translated.contains("[ directions Crate1 | Crate1{no directions} ] -> [ Crate1 | Crate1 ]")
    );
    assert!(translated.contains("repeat {\n[ > crate | obs{no directions} ] -> [ crate | obs ]"));
    assert!(translated.contains("move\n[ Target crate ] -> [ Target ]"));
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
fn imports_puzzlescript_next_teneten_sample_as_current_canonical_syntax() {
    let source = include_str!("fixtures/puzzlescript/teneten_next.txt");

    let translated = translate_puzzlescript_to_canonical(source)
        .expect("TENETEN should import from PuzzleScript Next syntax");

    assert!(translated.contains("title = \"TENETEN\""));
    assert!(translated.contains("slots {\nlayer1 = Background\n"));
    assert!(translated.contains("all TargetCrate on crate"));
    assert!(translated.contains("level \"1\""));
    assert!(translated.lines().any(|line| line == "message \"1\""));
    assert!(translated.contains("message \"Thank you for playing!\""));
    assert!(translated.contains("[ TargetCrate crate ] -> [ TargetCrate crate Satisfied ]"));
    assert!(!translated.contains("[ TargetCrate Crate ] -> [ TargetCrate Crate Satisfied ]"));
    assert!(
        translated.contains(
            "[ You:D#1 Count:N#2 no Checked ] -> [ You:D_rev(D#1) Count:Nm(N#2) Checked ]"
        )
    );
    assert!(!translated.contains("[ You:F Count:0 no Checked ] -> [ You:B Count:3 Checked ]"));
    assert!(translated.contains("shape_You_F {"));
    assert!(translated.contains("You:B\n#000 #fff #00000015\nshape_You_F"));
    assert!(!translated.contains("You:B {"));
    assert!(translated.contains("rotate (directions - up)"));
    assert!(!translated.contains("rotate from up"));
    assert!(translated.contains("choice \"Level Select\" -> goto level_select"));
    assert!(translated.contains("scene level_select {\nlayout {\nlevel_menu {"));
    assert_imported_output_uses_current_canonical_surface(&translated);

    let loaded = parse_game(&translated).expect("translated TENETEN should parse as canonical");
    assert_eq!(loaded.title, "TENETEN");
    assert_eq!(loaded.author.as_deref(), Some("smyrnensis"));
    assert_eq!(loaded.homepage.as_deref(), Some("smyrnensis.itch.io"));
    assert!(
        loaded
            .scenes
            .iter()
            .any(|scene| scene.name == "level_select")
    );
}

#[test]
fn puzzlescript_import_rejects_canonical_metadata_assignment_in_prelude() {
    let error = translate_puzzlescript_to_canonical("title = Not PuzzleScript\n")
        .expect_err("PuzzleScript metadata must not use canonical assignment syntax")
        .to_string();

    assert!(error.contains("PuzzleScript title metadata must use `title <text>`"));
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

    assert!(translated.contains("routine main once {\n[ Wall ] -> [ Player ]\nmove\n}"));
    assert!(translated.contains("on_level_start {\n"));
    assert!(translated.contains("main\n}"));
    assert!(translated.contains("rules {\ninput [ Player ] -> [ > Player ]\nmain\n}"));
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
fn puzzlescript_import_rejects_subroutine_conflict_with_generated_main() {
    let source = r#"
title Generated Routine Conflict
run_rules_on_level_start

OBJECTS

Background
black

Player
blue

LEGEND

. = Background
P = Player

COLLISIONLAYERS

Background
Player

RULES

subroutine main
[ Player ] -> [ Player ]

LEVELS

P
"#;

    let error = translate_puzzlescript_to_canonical(source)
        .expect_err("generated main routine conflict should be rejected")
        .to_string();

    assert!(
        error.contains("PuzzleScript subroutine `main` conflicts with importer-generated routine"),
        "{error}"
    );
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

    assert!(translated.contains("routine main once {\nmove\nright [ Emitter | no Beam ]"));
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

    assert!(translated.contains("[ > victimSolo | pushBlock ] -> [ > victimSolo | > pushBlock ]"));
    assert!(!translated.contains("v ictimSolo"));
    parse_game(&translated).unwrap();
}

#[test]
fn puzzlescript_imports_compact_directional_rule_with_moving_qualifiers() {
    let source = r#"
title Compact Moving Rule

========
OBJECTS
========

Background
black

Player
white

edge
blue

push
red

=======
LEGEND
=======

. = Background
P = Player
e = edge
p = push

================
COLLISIONLAYERS
================

Background
Player
edge
push

======
RULES
======

down[edge     | moving push] -> [moving edge     |moving push]

=======
LEVELS
=======

Pep
"#;

    let translated = translate_puzzlescript_to_canonical(source).unwrap();

    assert!(
        translated
            .contains("down [ edge | directions push ] -> [ directions edge | directions push ]")
    );
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

    assert!(translated.contains("sfx sfx1 { seed = 26; type = puzzlescript }"));
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
background black
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

    assert!(translated.contains("background_color = #000000"));
    assert!(translated.contains("text_color = #9CBD0F"));
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
