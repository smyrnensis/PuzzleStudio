use super::*;

#[test]
fn virtual_workspace_imports_are_resolved_by_the_language_layer() {
    let documents = vec![
        (
            "games/demo/game.puzzle".to_string(),
            "import \"parts/model.puzzle\"\n".to_string(),
        ),
        (
            "games/demo/parts/model.puzzle".to_string(),
            "title = imported\npuzzle default { objects {} collision_layers {} rules {} levels default of default {} }\n".to_string(),
        ),
    ];
    let expanded = expand_game_imports_from_documents("games/demo/game.puzzle", &documents)
        .expect("virtual import expansion");
    assert!(expanded.contains("title = imported"));
    assert!(!expanded.contains("import \"parts/model.puzzle\""));
}

#[test]
fn virtual_workspace_import_cycles_fail_visibly() {
    let documents = vec![
        (
            "game.puzzle".to_string(),
            "import \"part.puzzle\"\n".to_string(),
        ),
        (
            "part.puzzle".to_string(),
            "import \"game.puzzle\"\n".to_string(),
        ),
    ];
    let error =
        expand_game_imports_from_documents("game.puzzle", &documents).expect_err("cycle must fail");
    assert!(error.to_string().contains("cyclic import"));
}

#[test]
fn source_analysis_returns_typed_import_reference_at_the_path_only() {
    let source = "// import \"ignored.puzzle\"\nimport \"parts/model.puzzle\"\nassets {\n  file \"not-a-link.png\"\n}\n";
    let analysis = analyze_source(source);
    let cursor = source.find("model.puzzle").unwrap();
    let reference = analysis
        .import_reference_at("games/demo/game.puzzle", cursor)
        .expect("import reference");
    assert_eq!(reference.raw_path, "parts/model.puzzle");
    assert_eq!(reference.resolved_path, "games/demo/parts/model.puzzle");
    let asset_cursor = source.find("not-a-link.png").unwrap();
    assert!(
        analysis
            .import_reference_at("games/demo/game.puzzle", asset_cursor)
            .is_none()
    );
}
use puzzle_core::{LocalFrameExtent, RuleStep, State, transition_program, transition_state};

fn parse_game(source: &str) -> Result<LoadedGame, DiagnosticReport> {
    super::parse_game2d(&modernize_test_source(source))
}

#[test]
fn solver_strategy_lowers_query_variable_and_distance_terms() {
    let source = r#"
title = solver_strategy_terms

puzzle default {
layers {
floor = Goal
actor = Player Box
}

var pressure = 0
query boxes_on_goal = count([ Box Goal ])
query player_to_goal = distance(Player, Goal)
query near = distance(Player, Goal) <= 3

solver {
strategy {
maximize boxes_on_goal weight 50
minimize pressure weight 2
prefer near weight 10
avoid player_to_goal weight 3
}
}

rules {
}

levels tiny of default {
legend {
. = empty
P = Player
B = Box
G = Goal
}

level "start" {
PBG
}
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.solver_strategy.terms.len(), 4);
    assert_eq!(loaded.queries.len(), 3);
    assert_eq!(
        loaded.solver_strategy.terms[0].direction,
        SolverStrategyDirection::Maximize
    );
    assert_eq!(loaded.solver_strategy.terms[0].weight, 50);
    assert!(matches!(
        loaded.solver_strategy.terms[0].value,
        QueryExpr::Value(_)
    ));
    assert_eq!(
        loaded.solver_strategy.terms[1].direction,
        SolverStrategyDirection::Minimize
    );
    assert!(matches!(
        loaded.solver_strategy.terms[1].value,
        QueryExpr::Variable(_)
    ));
    assert_eq!(
        loaded.solver_strategy.terms[2].direction,
        SolverStrategyDirection::Prefer
    );
    assert!(matches!(
        loaded.solver_strategy.terms[2].value,
        QueryExpr::Compare { .. }
    ));
    assert!(matches!(
        loaded.solver_strategy.terms[3].value,
        QueryExpr::Distance { .. }
    ));
}

fn modernize_test_source(source: &str) -> String {
    let Ok(lines) = super::source::logical_lines(source) else {
        return source.to_string();
    };
    let mut out = Vec::new();
    let mut i = 0;
    let mut in_keys = false;
    let mut scene_depth = 0usize;
    let mut levels_depth = 0usize;
    let mut pending_level_legend = Vec::<String>::new();
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        let in_scene = scene_depth > 0;
        let in_levels = levels_depth > 0;

        if in_levels {
            out.push(line.clone());
            i += 1;
            if is_block_close_line(line) {
                levels_depth = levels_depth.saturating_sub(1);
            } else if matches!(tokens.as_slice(), ["legend"] | ["{"])
                || (matches!(tokens.as_slice(), ["level", ..]) && line.trim_end().ends_with('{'))
            {
                levels_depth += 1;
            }
            continue;
        }

        if let Some(scene_header) = modern_scene_header(&tokens) {
            scene_depth = 1;
            out.push(scene_header);
            i += 1;
            continue;
        }
        if in_scene && is_block_close_line(line) {
            scene_depth = scene_depth.saturating_sub(1);
            if in_keys {
                in_keys = false;
            }
            out.push(line.clone());
            i += 1;
            continue;
        }

        if in_scene && tokens.as_slice() == ["keys"] {
            in_keys = true;
            scene_depth += 1;
            out.push(line.clone());
            i += 1;
            continue;
        }
        if in_keys {
            out.push(line.clone());
            i += 1;
            continue;
        }

        match tokens.as_slice() {
            ["puzzle", name] if !in_scene && is_identifier(name) => {
                out.push(format!("puzzle {name}"));
            }
            ["render_overlay", rest @ ..] if rest.len() >= 3 => {
                let ch = rest[rest.len() - 1];
                let objects = &rest[..rest.len() - 1];
                pending_level_legend.push(format!("legend {ch} = {}", objects.join(" ")));
            }
            ["legend"] | ["legend", ..] if !in_scene => {
                let (legend, next_i) = collect_test_legend_entry(&lines, i);
                pending_level_legend.extend(legend);
                i = next_i;
                continue;
            }
            ["levels", ..] => {
                levels_depth = 1;
                out.push(line.clone());
                out.append(&mut pending_level_legend);
            }
            ["level", name, ..] => {
                out.push("levels".to_string());
                out.append(&mut pending_level_legend);
                let braced_level = line.trim_end().ends_with('{');
                if braced_level {
                    out.push(line.clone());
                } else {
                    out.push(format!("level {name}"));
                }
                i += 1;
                while i < lines.len() && !is_block_close_line(&lines[i]) {
                    out.push(lines[i].clone());
                    i += 1;
                }
                out.push(BLOCK_CLOSE.to_string());
                if braced_level {
                    out.push(BLOCK_CLOSE.to_string());
                }
            }
            _ => out.push(line.clone()),
        }
        if in_scene && !matches!(tokens.as_slice(), ["keys"]) && test_starts_block(&tokens) {
            scene_depth += 1;
        }
        i += 1;
    }
    out.join("\n")
}

fn collect_test_legend_entry(lines: &[String], start: usize) -> (Vec<String>, usize) {
    if split_header_tokens(&lines[start]).as_slice() == ["legend"] {
        let mut out = vec![lines[start].clone()];
        let mut i = start + 1;
        while i < lines.len() {
            out.push(lines[i].clone());
            if is_block_close_line(&lines[i]) {
                return (out, i + 1);
            }
            i += 1;
        }
        return (out, i);
    }
    (vec![lines[start].clone()], start + 1)
}

fn modern_scene_header(tokens: &[&str]) -> Option<String> {
    match tokens {
        ["scene", "puzzle", name] => Some(format!("scene {name}")),
        ["scene", "puzzle"] => Some("scene puzzle".to_string()),
        ["scene", "menu", name] => Some(format!("scene {name}")),
        ["scene", "menu"] => Some("scene menu".to_string()),
        ["scene", name] => Some(format!("scene {name}")),
        _ => None,
    }
}

fn test_starts_block(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["layers"]
            | ["rules"]
            | ["legend"]
            | ["win_conditions", ..]
            | ["lose_conditions", ..]
            | ["levels", ..]
            | ["resources"]
            | ["main"]
            | ["on_level_start"]
            | ["level_start"]
            | ["on_level_clear"]
            | ["level_clear"]
            | ["routine", ..]
            | ["rule", ..]
            | ["once_all"]
            | ["once_per_level"]
            | ["row", ..]
            | ["column", ..]
            | ["box", ..]
            | ["for", ..]
    )
}

fn object_named(loaded: &LoadedGame, name: &str) -> ObjectId {
    loaded
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == name).then_some(*object))
        .or_else(|| {
            let display_name = format!("@{name}");
            loaded
                .object_labels
                .iter()
                .find_map(|(object, label)| (label == &display_name).then_some(*object))
        })
        .unwrap()
}

fn input_named(loaded: &LoadedGame, name: &str) -> InputId {
    loaded
        .input_labels
        .iter()
        .find_map(|(input, label)| (label == name).then_some(*input))
        .unwrap()
}

fn labels_at(loaded: &LoadedGame, state: &State, x: u16, y: u16) -> Vec<String> {
    state
        .cell_view(x, y)
        .unwrap()
        .objects
        .iter()
        .filter_map(|object| loaded.object_labels.get(object).cloned())
        .collect()
}

#[test]
fn rules_local_frame_limits_main_transition_matching_to_player_frame() {
    let loaded = parse_game(
        r#"
title = local frame
puzzle main {
layers {
actor = Player A B
}
levels {
legend {
. = empty
P = Player
A = A
}
level "one" {
PA.A
}
}
rules local_frame 1 1 {
once_all [ A ] -> [ B ]
}
}
"#,
    )
    .unwrap();
    let a = object_named(&loaded, "A");
    let b = object_named(&loaded, "B");

    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert!(next.has_object(&loaded.game, 1, 0, b));
    assert!(next.has_object(&loaded.game, 3, 0, a));
    assert!(!next.has_object(&loaded.game, 3, 0, b));
}

#[test]
fn rules_local_radius_lowers_to_radius_extent() {
    let loaded = parse_game(
        r#"
title = local radius
puzzle main {
layers {
actor = Player A B
}
levels {
legend {
. = empty
P = Player
A = A
}
level "one" {
PA.A
}
}
rules local_radius 3 {
once_all [ A ] -> [ B ]
}
}
"#,
    )
    .unwrap();

    let Some(RuleStep::LocalFrame { frame, .. }) = loaded.game.program().first() else {
        panic!("expected main rules to be wrapped in a local frame");
    };
    assert_eq!(frame.x, LocalFrameExtent::Radius(3));
    assert_eq!(frame.y, LocalFrameExtent::Radius(3));
    assert_eq!(frame.z, LocalFrameExtent::Full);
}

#[test]
fn section_headers_are_not_canonical_syntax() {
    let source = r#"
title = section_header

puzzle board {
======
LAYERS
======
floor = Goal
actor = Player Box

=======
LEGENDS
=======
. = empty
P = Player
B = Box
* = Goal Box

=====
RULES
=====
once [ Player ] -> [ Player ]

======
LEVELS
======
level "start" {
*
}
}
"#;

    assert!(super::parse_game2d(source).is_err());
}

#[test]
fn inline_braced_blocks_accept_semicolon_rows() {
    let source = r#"
title = inline_blocks

puzzle board {
layers { actor = Player Box Wall; floor = Goal }
groups { solid = Box Wall; pushable = Box }
legend { . = empty; P = Player; B = Box; W = Wall; G = Goal }
rules { once [ Player | ] -> [ | Player ] }
levels {
level "start" {
P.G
}
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let player = object_named(&loaded, "Player");
    let right = input_named(&loaded, "right");
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();

    assert!(moved.has_object(&loaded.game, 1, 0, player));
}

#[test]
fn null_pattern_matches_outside_board_cell() {
    let source = r#"
title = null_pattern

puzzle board {
layers { mark = Edge }
legend { . = empty; E = Edge }
rules { once right [ no Edge | null ] -> [ Edge | ] }
levels {
level "start" {
.
}
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let edge = object_named(&loaded, "Edge");
    let right = input_named(&loaded, "right");
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();

    assert!(moved.has_object(&loaded.game, 0, 0, edge));
}

#[test]
fn no_null_pattern_is_rejected() {
    let source = r#"
title = no_null_pattern

puzzle board {
layers { mark = Edge }
legend { . = empty; E = Edge }
rules { once right [ no null | null ] -> [ Edge | ] }
levels {
level "start" {
.
}
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("`no null` is not a valid cell pattern"));
}

#[test]
fn rhs_only_null_pattern_is_rejected() {
    let source = r#"
title = rhs_only_null_pattern

puzzle board {
layers { mark = Edge }
legend { . = empty; E = Edge }
rules { once right [ no Edge | ] -> [ Edge | null ] }
levels {
level "start" {
.
}
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("`null` can only be matched on the before side"));
}

#[test]
fn pattern_rows_accept_physical_line_breaks() {
    let source = r#"
title = pattern_newlines

puzzle board {
layers { actor = A B C }
legend { . = empty; A = A; B = B; C = C }
rules {
once [ A
B ] -> [ C
C ]
}
levels {
level "start" {
A
B
}
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let object_c = object_named(&loaded, "C");
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert!(moved.has_object(&loaded.game, 0, 0, object_c));
    assert!(moved.has_object(&loaded.game, 0, 1, object_c));
}

#[test]
fn at_prefixed_objects_and_routines_use_normal_gameplay_semantics() {
    let source = r#"
title = at_display

puzzle board {
layers {
actor = Player
@cursor = @Cursor
@hint = @Hint
}

legend {
. = empty
P = Player
}

routine @paint once {
[ Player no @Cursor no @Hint ] -> [ Player @Cursor @Hint ]
}

rules {
@paint
}

levels {
level "start"
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let cursor = object_named(&loaded, "@Cursor");
    let hint = object_named(&loaded, "@Hint");
    let initial = &loaded.levels[0].initial_state;

    assert!(!loaded.is_display_object(cursor));
    assert!(!loaded.is_display_object(hint));
    assert!(!initial.has_object(&loaded.game, 0, 0, cursor));

    let stepped = transition_state(&loaded.game, initial, InputId(0)).unwrap();
    assert!(stepped.has_object(&loaded.game, 0, 0, cursor));
    assert!(stepped.has_object(&loaded.game, 0, 0, hint));
}

#[test]
fn legend_does_not_define_unknown_objects() {
    let source = r#"
title = legend_unknown

puzzle board {
layers {
actor = Player
}

legend {
. = empty
P = Player
G = Ghost
}

rules {

}

levels {
level "start"
P
}
}
"#;

    let err = parse_game(source).unwrap_err();

    assert!(err.to_string().contains("unknown object selector"), "{err}");
}

#[test]
fn level_local_legend_does_not_define_unknown_objects() {
    let source = r#"
title = level_legend_unknown

puzzle board {
layers {
actor = Player
}

legend {
. = empty
P = Player
}

rules {

}

levels {
level "start" {
legend {
G = Ghost
}

P
}
}
}
"#;

    let err = parse_game(source).unwrap_err();

    assert!(err.to_string().contains("unknown object selector"), "{err}");
}

#[test]
fn layers_define_objects_even_when_sprites_are_omitted() {
    let source = r#"
title = sprite_omitted_is_transparent

puzzle board {
layers {
actor = Player Hidden
}

levels {
legend {
. = empty
P = Player
H = Hidden
}

level "start" {
PH
}
}

rules {
[ Hidden ] -> [ Hidden ]
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let hidden = object_named(&loaded, "Hidden");

    assert!(loaded.game.object(hidden).is_some());
    assert!(
        loaded.levels[0]
            .initial_state
            .has_object(&loaded.game, 1, 0, hidden)
    );
    assert!(loaded.visuals.sprites.is_empty());
    assert!(loaded.visuals.aliases.is_empty());
}

#[test]
fn layers_schema_terms_can_use_later_tag_sets() {
    let source = r#"
title = layers_schema_later_tags

puzzle board {
layers {
solid = Wall Alien Crab:state
}

tags {
state = norm poss
}

levels {
legend {
. = empty
C = Crab:norm
}

level "start" {
C
}
}

rules {
[ Crab:norm ] -> [ Crab:poss ]
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let crab_norm = object_named(&loaded, "Crab:norm");
    let crab_poss = object_named(&loaded, "Crab:poss");

    assert!(
        loaded.levels[0]
            .initial_state
            .has_object(&loaded.game, 0, 0, crab_norm)
    );
    assert!(loaded.game.object(crab_poss).is_some());
}

#[test]
fn top_level_sounds_keeps_only_seed_and_settings() {
    let source = r#"
title = sounds_game

sounds {
sfx effect { seed = 746670; type = jump; volume = 0.35 }
music loop { seed = 123456; bars = 16; height = 0.62; bpm = 104; volume = 0.8 }
}

puzzle board {
layers {
background = Player
}

legend {
. = empty
P = Player
}

rules {

}

levels {
level "one"
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.sounds.sfx.len(), 1);
    assert_eq!(loaded.sounds.sfx[0].name, "effect");
    assert_eq!(loaded.sounds.sfx[0].seed, "746670");
    assert_eq!(loaded.sounds.sfx[0].type_target, "jump");
    assert_eq!(loaded.sounds.sfx[0].volume, 0.35);
    assert_eq!(loaded.sounds.music.len(), 1);
    assert_eq!(loaded.sounds.music[0].name, "loop");
    assert_eq!(loaded.sounds.music[0].seed, "123456");
    assert_eq!(loaded.sounds.music[0].height, 0.62);
    assert_eq!(loaded.sounds.music[0].bars, 16);
    assert_eq!(loaded.sounds.music[0].bpm, 104);
    assert_eq!(loaded.sounds.music[0].volume, 0.8);
}

#[test]
fn top_level_sfx_volume_defaults_to_existing_full_gain() {
    let source = r#"
title = sounds_game

sounds {
sfx effect { seed = 746670; type = jump }
}

puzzle board {
layers {
background = Player
}

legend {
. = empty
P = Player
}

rules {

}

levels {
level "one"
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.sounds.sfx[0].volume, 1.0);
}

#[test]
fn top_level_sounds_allow_volume_above_full_gain() {
    let source = r#"
title = sounds_game

sounds {
sfx effect { seed = 746670; type = jump; volume = 1.5 }
music loop { seed = 123456; bars = 16; height = 0.62; bpm = 104; volume = 1.25 }
}

puzzle board {
layers {
background = Player
}

legend {
. = empty
P = Player
}

rules {

}

levels {
level "one"
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.sounds.sfx[0].volume, 1.5);
    assert_eq!(loaded.sounds.music[0].volume, 1.25);
}

#[test]
fn top_level_sounds_reject_negative_volume() {
    let sfx_source = r#"
title = sounds_game

sounds {
sfx effect { seed = 746670; type = jump; volume = -0.1 }
}

puzzle board {
layers {
background = Player
}

legend {
. = empty
P = Player
}

rules {

}

levels {
level "one"
P
}
}
"#;

    let error = parse_game(sfx_source).unwrap_err().to_string();
    assert!(
        error.contains("sfx volume must be zero or greater"),
        "{error}"
    );

    let music_source = r#"
title = sounds_game

sounds {
music loop { seed = 123456; bars = 16; height = 0.62; bpm = 104; volume = -0.1 }
}

puzzle board {
layers {
background = Player
}

legend {
. = empty
P = Player
}

rules {

}

levels {
level "one"
P
}
}
"#;

    let error = parse_game(music_source).unwrap_err().to_string();
    assert!(
        error.contains("music volume must be zero or greater"),
        "{error}"
    );
}

#[test]
fn model_sounds_resolve_against_whole_puzzle_scope() {
    let source = r#"
title = scoped_model_sounds

sounds {
sfx push { seed = push01; type = jump }
}

puzzle sokoban {
sounds {
move Box -> sfx push
cantmove Box -> sfx push
}

layers {
actor = Player Box
}

levels {
legend {
. = empty
P = Player
B = Box
}

level "start"
PB
}

rules {
input directions [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
}
}
"#;

    parse_game(source).unwrap();
}

#[test]
fn model_sounds_parse_undo_and_restart_sfx_operations() {
    let source = r#"
title = operation_sounds

sounds {
sfx back { seed = back01; type = hit }
sfx reset { seed = reset01; type = jump }
}

puzzle sokoban {
sounds {
undo -> sfx back
restart -> sfx reset
}

layers {
actor = Player
}

levels {
legend {
. = empty
P = Player
}

level "start"
P
}

rules {

}
}
"#;

    let loaded = parse_game(source).unwrap();
    assert_eq!(
        loaded.model_operation_sounds,
        vec![
            ModelOperationSoundDef {
                operation: ModelOperationSound::Undo,
                sfx_name: "back".to_string(),
            },
            ModelOperationSoundDef {
                operation: ModelOperationSound::Restart,
                sfx_name: "reset".to_string(),
            },
        ]
    );
}

#[test]
fn model_sounds_report_selector_errors_at_sound_entry() {
    let source = r#"
title = bad_model_sound_selector

sounds {
sfx push { seed = push01; type = jump }
}

puzzle sokoban {
sounds {
move Ghost -> sfx push
}

layers {
actor = Player Box
}

levels {
legend {
. = empty
P = Player
}

level "start"
P
}

rules {
input directions [ Player ] -> [ > Player ]
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();

    assert!(
        error.contains("unknown model sound trigger object selector `Ghost`"),
        "{error}"
    );
}

#[test]
fn top_level_audio_block_is_rejected() {
    let source = r#"
title = old_sounds_keyword

audio {
  sfx effect { seed = 746670; type = jump }
}
"#;

    let error = parse_game_document(source).unwrap_err().to_string();
    assert!(
        error.contains("unknown top-level directive `audio`"),
        "{error}"
    );
    assert!(error.contains("content ("), "{error}");
    assert!(error.contains("`sounds`"), "{error}");
}

#[test]
fn model_top_level_expected_message_uses_parser_alternatives() {
    let message = model_top_level_expected_directives_message();

    for alternative in MODEL_TOP_LEVEL_ALTERNATIVES {
        let label = format!("`{}`", alternative.label);
        if alternative.expected_group.is_some() {
            assert!(
                message.contains(&label),
                "expected parser alternative {label} to appear in: {message}"
            );
            assert_ne!(
                classify_model_top_level_directive(&[alternative.trigger]),
                ModelTopLevelDirective::Unknown,
                "expected parser alternative {label} to classify as known"
            );
        } else {
            assert!(
                !message.contains(&label),
                "non-expected parser alternative {label} leaked into: {message}"
            );
        }
    }
}

#[test]
fn scene_lifecycle_blocks_lower_to_lifecycle_transitions() {
    let source = r#"
title = scene_lifecycle_blocks

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
rules {

}
level "start" {
P
}
}

scene playing {
layout {
text "Playing"
}
on_scene_start {
stop_music music_name
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    let scene_start = scene
        .transitions
        .iter()
        .find(|transition| transition.trigger == SceneTransitionTrigger::SceneStart)
        .unwrap();
    assert!(matches!(
        &scene_start.effect,
        SceneEffect::StopMusic { name } if name.as_deref() == Some("music_name")
    ));
}

#[test]
fn scene_lifecycle_accepts_bare_next_level_effect() {
    let source = r#"
title = scene_next_level_effect

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
rules {
}
level "start" {
P
}
}

scene playing {
layout {
text "Playing"
}
on_scene_start {
next_level
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let scene_start = loaded.scenes[0]
        .transitions
        .iter()
        .find(|transition| transition.trigger == SceneTransitionTrigger::SceneStart)
        .unwrap();
    assert!(matches!(
        &scene_start.effect,
        SceneEffect::PuzzleNextLevel { target } if target.is_empty()
    ));
}

#[test]
fn scene_message_effect_parses_literal_and_path() {
    let source = r#"
title = scene_message_effect
var hint = "Push the box"

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
rules {

}
level "start" {
P
}
}

scene playing {
layout {
text "Playing"
}
on_scene_start {
message "Welcome"
message hint
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let scene_start = loaded.scenes[0]
        .transitions
        .iter()
        .find(|transition| transition.trigger == SceneTransitionTrigger::SceneStart)
        .unwrap();
    let SceneEffect::Sequence { effects } = &scene_start.effect else {
        panic!("expected message effects to lower to a sequence");
    };
    assert!(matches!(
        &effects[0],
        SceneEffect::Message { text: SceneExpr::Text(value) } if value == "Welcome"
    ));
    assert!(matches!(
        &effects[1],
        SceneEffect::Message { text: SceneExpr::Path(path) } if path == &vec!["hint".to_string()]
    ));
}

#[test]
fn again_interval_parses_to_default_again_milliseconds() {
    let loaded = parse_game(
        r#"
title = again_interval_fixture
again_interval = 75ms

puzzle main {
layers {
actor = Player
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}
"#,
    )
    .unwrap();

    assert_eq!(loaded.default_again_ms, 75);
}

#[test]
fn puzzlescript_style_again_interval_is_not_canonical_syntax() {
    let err = parse_game(
        r#"
title = again_interval_seconds_fixture
again_interval 0.1

puzzle main {
layers {
actor = Player
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}
"#,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("again_interval must be: again_interval = <duration>"));
}

#[test]
fn top_level_metadata_requires_assignment_syntax() {
    let err = parse_game(
        r#"
title "Old Metadata"

puzzle main {
layers {
actor = Player
}
rules {
}
levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}
"#,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("title metadata must be: title = <text>"));
}

#[test]
fn puzzle_render_tween_parses_to_game_settings() {
    let loaded = parse_game(
        r#"
title = tween_fixture

puzzle main {
render {
tween = true
tween_duration = 90ms
}
layers {
actor = Player
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}
"#,
    )
    .unwrap();

    assert!(loaded.animation.tween.enabled);
    assert_eq!(loaded.animation.tween.interval_ms, 90);
}

#[test]
fn puzzle_render_tween_duration_requires_enabled_tween() {
    let error = parse_game(
        r#"
title = tween_fixture

puzzle main {
render {
tween_duration = 90ms
}
layers {
actor = Player
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}
"#,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("tween_duration requires tween = true"));
}

#[test]
fn puzzle_render_rejects_old_inline_tween_node() {
    let error = parse_game(
        r#"
title = tween_fixture

puzzle main {
render {
tween duration 90ms
}
layers {
actor = Player
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}
"#,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("tween must have one value"));
}

#[test]
fn top_level_render_tween_is_not_shell_syntax() {
    let error = parse_game(
        r#"
title = tween_fixture
render {
tween = true
tween_duration = 90ms
}

puzzle main {
layers {
actor = Player
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}
"#,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("unknown top-level directive `render`"));
}

#[test]
fn top_level_input_buffer_parses_to_game_settings() {
    let loaded = parse_game(
        r#"
title = input_buffer_fixture
input_buffer {
queue_during_wait = false
fast_forward_wait = true
min_wait = 75ms
}

puzzle main {
layers {
actor = Player
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}
"#,
    )
    .unwrap();

    assert!(!loaded.input_buffer.queue_during_wait);
    assert!(loaded.input_buffer.fast_forward_wait);
    assert_eq!(loaded.input_buffer.min_wait_ms, 75);
}

#[test]
fn render_tween_rejects_old_block_form() {
    let source = r#"
title = tween_fixture

puzzle main {
render {
tween {
duration = 80ms
}
}
layers {
actor = Player
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("tween must have one value"));
}

#[test]
fn render_tween_adds_move_rule_animation_without_sounds() {
    let loaded = parse_game(
        r#"
title = tween_animation_fixture

puzzle main {
render {
tween = true
tween_duration = 80ms
}
layers {
actor = Player
}
rules {
input directions [ Player ] -> [ Player{>} ]
input directions [ Player | no Player ] -> [ | Player ]
}
levels {
legend {
. = empty
P = Player
}
level "first"
P.
}
}
"#,
    )
    .unwrap();

    assert!(loaded.rule_animations.values().any(|animations| {
        animations.iter().any(|animation| {
            animation.trigger == RuleAnimationTrigger::Move
                && animation.name == "tween"
                && !animation.objects.is_empty()
        })
    }));
}

#[test]
fn scene_on_level_start_is_rejected() {
    let source = r#"
title = scene_level_lifecycle

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
rules {

}
level "start" {
P
}
}

scene playing {
layout {
text "Playing"
}
on_level_start {
message "no"
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("level lifecycle blocks belong inside puzzle"));
}

#[test]
fn scene_current_level_syntax_is_rejected() {
    let source = r#"
title = current_level_syntax

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
rules {

}
level "start" {
P
}
}

scene playing {
layout {
puzzle board = current_level
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("current_level is not scene syntax"));
}

#[test]
fn puzzle_presentation_message_parses_literal_and_path() {
    let source = r#"
title = rewrite_message_effect

puzzle default {
layers {
__legacy_layer_0 = Player
__legacy_layer_1 = Goal
}
legend P = Player
legend G = Goal
legend {
. = empty
P = Player
G = Goal
}
rules {

[ Player Goal ] -> message "Found"
[ Player ] -> message hint
}
level "start" {
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let effects = loaded
        .rule_effects
        .values()
        .flat_map(|effects| effects.iter())
        .collect::<Vec<_>>();
    assert!(effects.iter().any(|effect| matches!(
        effect,
        RuleEffect::Message { text, literal: true } if text == "Found"
    )));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        RuleEffect::Message { text, literal: false } if text == "hint"
    )));
}

#[test]
fn puzzle_presentation_effect_parses_commands() {
    let source = r#"
title = puzzle_presentation_effect
default_wait_time = 350ms

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
rules {

[ Player ] -> sfx pushed
wait
wait 25ms
}
level "start" {
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let effects = loaded
        .rule_effects
        .values()
        .flat_map(|effects| effects.iter())
        .collect::<Vec<_>>();
    assert!(
        effects
            .iter()
            .any(|effect| { matches!(effect, RuleEffect::PlaySfx { name } if name == "pushed") })
    );
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, RuleEffect::WaitAnimation))
    );
    assert!(effects.iter().any(|effect| {
        matches!(effect, RuleEffect::Wait { milliseconds } if *milliseconds == 25)
    }));
}

#[test]
fn rewrite_suffix_wait_lowers_to_after_triggered_animation_barrier() {
    let source = r#"
title = rewrite_wait_suffix

puzzle default {
layers {
__legacy_layer_0 = Player Goal
}
legend {
. = empty
P = Player
G = Goal
}
rules {
[ Player ] -> [ Goal ] wait
}
level "start" {
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    assert!(loaded.game.program().iter().any(|step| {
        matches!(
            step,
            puzzle_core::RuleStep::AfterTriggered { then_steps, .. }
                if then_steps.iter().any(|then_step| {
                    matches!(
                        then_step,
                        puzzle_core::RuleStep::Rule(rule)
                            if loaded
                                .rule_effects
                                .get(&rule.id)
                                .is_some_and(|effects| effects.iter().any(|effect| {
                                    matches!(effect, RuleEffect::WaitAnimation)
                                }))
                    )
                })
        )
    }));
}

#[test]
fn rule_condition_can_emit_win_effect() {
    let source = r#"
title = rule_condition_win_effect

puzzle default {
layers {
actor = Player
}
input clear
rules {
if input == clear -> win
}
levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let effects = loaded
        .rule_effects
        .values()
        .flat_map(|effects| effects.iter())
        .collect::<Vec<_>>();

    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, RuleEffect::Win))
    );
}

#[test]
fn puzzle_rule_effect_accepts_goto_scene() {
    let source = r#"
title = puzzle_rule_goto_effect

puzzle default {
layers {
__legacy_layer_0 = Player
}
input open
rules {
if input == open -> goto menu
}
levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}
}

scene menu {
layout {
text "Menu"
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let effects = loaded
        .rule_effects
        .values()
        .flat_map(|effects| effects.iter())
        .collect::<Vec<_>>();
    assert!(effects.iter().any(|effect| matches!(
        effect,
        RuleEffect::Scene(SceneEffect::Goto { scene, params }) if scene == "menu" && params.is_empty()
    )));
}

#[test]
fn puzzle_wait_animation_lowers_to_ordered_boundary_effect() {
    let source = r#"
title = puzzle_wait_animation_effect

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
rules {
wait animation
}
level "start" {
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    assert!(loaded.rule_effects.values().any(|effects| {
        effects
            .iter()
            .any(|effect| matches!(effect, RuleEffect::WaitAnimation))
    }));
}

#[test]
fn puzzle_emit_is_rejected() {
    let source = r#"
title = puzzle_emit_rejected

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
rules {

emit sfx tick
}
level "start" {
P
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("`emit` is obsolete"));
}

#[test]
fn puzzle_emit_is_rejected_for_state_mutating_effects() {
    let source = r#"
title = puzzle_emit_rejects_state_mutation

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
var moved = false
rules {

emit moved = true
}
level "start" {
P
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("`emit` is obsolete"));
}

#[test]
fn do_statement_is_rejected() {
    let source = r#"
title = do_statement_rejected

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
rules {

do sfx tick
}
level "start" {
P
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("`do` is obsolete"));
}

#[test]
fn routine_can_group_effect_statements() {
    let source = r#"
title = routine_effect_statements

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
var ready = false
routine ready_feedback once {
sfx tick
message "Ready"
ready = true
}
rules {

ready_feedback
ready_feedback
}
level "start" {
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let ordered_effects = loaded
        .rule_effects
        .values()
        .flat_map(|effects| effects.iter())
        .collect::<Vec<_>>();
    assert!(
        ordered_effects
            .iter()
            .filter(|effect| matches!(effect, RuleEffect::PlaySfx { name } if name == "tick"))
            .count()
            >= 2
    );
    assert!(
        ordered_effects
            .iter()
            .filter(|effect| matches!(effect, RuleEffect::Message { text, literal: true } if text == "Ready"))
            .count()
            >= 2
    );
    let effects = loaded
        .game
        .rules()
        .iter()
        .flat_map(|rule| rule.effects.iter())
        .collect::<Vec<_>>();
    assert!(
        effects
            .iter()
            .filter(|effect| matches!(effect, puzzle_core::Effect::UpdateVariable { .. }))
            .count()
            >= 2
    );
}

#[test]
fn effect_definition_is_rejected() {
    let source = r#"
title = effect_definition_rejected

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
effect feedback {
sfx tick
}
level "start" {
P
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("effect definitions are obsolete"));
}

#[test]
fn scene_input_handler_requires_arrow_block_syntax() {
    let source = r#"
title = old_scene_input_handler

puzzle board {
layers {
actor = Player
}
rules {

[ Player ] -> [ Player ]
}
}

levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}

scene playing {
input resume {
back
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("scene input handlers are removed"));
}

#[test]
fn scene_template_rejects_using_keyword() {
    let source = r#"
title = old_using_scene

puzzle board {
layers {
actor = Player
}
rules {

[ Player ] -> [ Player ]
}
}

levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}

scene menu using menu {
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("scene header must be: scene <name>"));
}

#[test]
fn scene_header_rejects_assignment_form() {
    let source = r#"
title = scene_assignment_header

puzzle board {
layers {
actor = Player
}
rules {
}
}

scene = title {
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("scene header must be: scene <name>"));
}

#[test]
fn scene_key_command_assignment_can_feed_input_rule() {
    let source = r#"
title = scene_key_command_assignment

puzzle board {
layers {
actor = Player
}
input escape
legend {
. = empty
P = Player
}
rules {

[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene playing {
keys {
q -> input escape
}
routine escape {
goto title
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    let SceneEffect::Input(action) = &scene.key_bindings[0].effect else {
        panic!("expected key assignment to emit an input action");
    };
    assert_eq!(action, "escape");
    assert!(matches!(
        scene.routines[0].effect,
        SceneEffect::Goto { ref scene, ref params } if scene == "title" && params.is_empty()
    ));
}

#[test]
fn scene_keys_accept_routine_target() {
    let source = r#"
title = input_sugar

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
keys {
q -> level_select
}
routine level_select {
goto level_select
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert!(
        matches!(&scene.key_bindings[0].effect, SceneEffect::RoutineCall(name) if name == "level_select")
    );
    assert!(matches!(
        &scene.routines[0].effect,
        SceneEffect::Goto { scene, .. } if scene == "level_select"
    ));
}

#[test]
fn scene_rules_accept_condition_arrow_effect_rows() {
    let source = r#"
title = scene_condition_block_arrow

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
keys {
Enter -> input confirm
}
rules {
if has_progress_save == false -> goto playing
}
}

scene playing {
layout {
text "Playing"
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    let SceneTransitionTrigger::Condition(condition) = &scene.transitions[0].trigger else {
        panic!("expected rules row to lower to condition transition");
    };
    assert!(matches!(
        condition,
        SceneExpr::Binary {
            op: SceneBinaryOp::Eq,
            left,
            right,
        } if matches!(left.as_ref(), SceneExpr::Path(path) if path == &vec!["has_progress_save".to_string()])
            && matches!(right.as_ref(), SceneExpr::Bool(false))
    ));
    assert!(matches!(
        &scene.transitions[0].effect,
        SceneEffect::Goto { scene, .. } if scene == "playing"
    ));
}

#[test]
fn scene_if_block_lowers_to_condition_transition() {
    let source = r#"
title = scene_if_block

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
if has_progress_save == false {
goto playing
}
}

scene playing {
layout {
text "Playing"
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    let SceneTransitionTrigger::Condition(condition) = &scene.transitions[0].trigger else {
        panic!("expected condition block to lower to condition transition");
    };
    assert!(matches!(
        condition,
        SceneExpr::Binary {
            op: SceneBinaryOp::Eq,
            left,
            right,
        } if matches!(left.as_ref(), SceneExpr::Path(path) if path == &vec!["has_progress_save".to_string()])
            && matches!(right.as_ref(), SceneExpr::Bool(false))
    ));
    assert!(matches!(
        &scene.transitions[0].effect,
        SceneEffect::Goto { scene, .. } if scene == "playing"
    ));
}

#[test]
fn layout_if_keeps_structural_block_syntax() {
    let source = r#"
title = layout_if_block

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
layout {
if has_progress_save == true {
button "Continue" -> input continue_game
} else {
button "New Game" -> input new_game
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    let Some(SceneComponent::Conditional(conditional)) = scene.components.first() else {
        panic!("expected layout if to lower to a conditional component");
    };
    assert!(matches!(
        &conditional.condition,
        SceneExpr::Binary {
            op: SceneBinaryOp::Eq,
            ..
        }
    ));
    assert_eq!(conditional.children.len(), 1);
    assert_eq!(conditional.else_children.len(), 1);
}

#[test]
fn scene_keys_accept_arrow_to_input_or_effect() {
    let source = r#"
title = keys_arrow

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
keys {
q -> level_select
Escape -> goto pause
Escape q -> goto title
}
routine level_select {
goto level_select
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert!(matches!(
        &scene.key_bindings[0].effect,
        SceneEffect::RoutineCall(input) if input == "level_select"
    ));
    assert!(matches!(
        &scene.key_bindings[1].effect,
        SceneEffect::Goto { scene, .. } if scene == "pause"
    ));
    assert_eq!(scene.key_bindings[2].keys.len(), 2);
    assert!(matches!(
        &scene.key_bindings[2].effect,
        SceneEffect::Goto { scene, .. } if scene == "title"
    ));
}

#[test]
fn scene_keys_accept_multiline_effect_block() {
    let source = r#"
title = keys_effect_block

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
keys {
q -> {
clear_game_progress
goto playing
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert!(matches!(
        &scene.key_bindings[0].effect,
        SceneEffect::Sequence { effects }
            if matches!(
                effects.as_slice(),
                [
                    SceneEffect::ClearGameProgress,
                    SceneEffect::Goto { scene, params }
                ] if scene == "playing" && params.is_empty()
            )
    ));
}

#[test]
fn scene_effect_blocks_share_nested_if_parsing() {
    let source = r#"
title = keys_nested_effect_block

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
keys {
q -> {
if has_progress_save {
goto playing
}
goto title
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    let SceneEffect::Sequence { effects } = &scene.key_bindings[0].effect else {
        panic!("expected key effect block to parse as sequence");
    };
    assert!(matches!(
        &effects[0],
        SceneEffect::Conditional { condition, effect }
            if matches!(condition, SceneExpr::Path(path) if path == &vec!["has_progress_save".to_string()])
                && matches!(effect.as_ref(), SceneEffect::Goto { scene, params }
                    if scene == "playing" && params.is_empty())
    ));
    assert!(matches!(
        &effects[1],
        SceneEffect::Goto { scene, params } if scene == "title" && params.is_empty()
    ));
}

#[test]
fn scene_keys_reject_equals_assignment() {
    let source = r#"
title = keys_equals

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
keys {
q = goto level_select
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(
        error.contains("keys row must be: <key...> -> <scene effect-or-input>"),
        "{error}"
    );
}

#[test]
fn puzzle_default_scene_keys_accept_scene_effects_without_stealing_model_inputs() {
    let source = r#"
title = model_keys_scene_effect

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
keys {
w ArrowUp -> up
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
keys {
Escape q -> goto title
}
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(
        loaded.controls.keys.get(&b'w'),
        loaded.controls.arrows.get(&ArrowKey::Up)
    );
    let board = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == "board")
        .unwrap();
    assert!(board.key_bindings.is_empty());
    let title = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == "title")
        .unwrap();
    assert!(
        title.key_bindings.len() == 1
            && title.key_bindings[0].keys.len() == 2
            && matches!(
                &title.key_bindings[0].effect,
                SceneEffect::Goto { scene, params } if scene == "title" && params.is_empty()
            )
    );
}

#[test]
fn bare_scene_key_action_rejects_input_routine_ambiguity() {
    let source = r#"
title = ambiguous_key_action

puzzle board {
layers {
actor = Player
}
input open
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
keys {
Enter -> open
}
routine open {
goto title
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("ambiguous scene action `open`"));
    assert!(error.contains("write `input open`"));
}

#[test]
fn scene_keys_accept_multiple_keys_per_row() {
    let source = r#"
title = keys_multiple

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
keys {
q Escape -> level_select
}
routine level_select {
goto level_select
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert_eq!(scene.key_bindings[0].keys.len(), 2);
    assert!(matches!(
        &scene.key_bindings[0].effect,
        SceneEffect::RoutineCall(input) if input == "level_select"
    ));
}

#[test]
fn explicit_scene_title_and_subtitle_can_reference_game_metadata() {
    let source = r#"
title = "Display Title"
subtitle = "Display Subtitle"

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene title {
layout {
title = title
subtitle = subtitle
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert!(matches!(
        &scene.components[0],
        SceneComponent::Title(title)
            if title.content == SceneExpr::Path(vec!["title".to_string()])
    ));
    assert!(matches!(
        &scene.components[1],
        SceneComponent::Subtitle(subtitle)
            if subtitle.content == SceneExpr::Path(vec!["subtitle".to_string()])
    ));
}

#[test]
fn scene_can_use_model_name_as_default_puzzle_slot() {
    let source = r#"
title = default_slot

puzzle sokoban {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene playing {
layout {
sokoban
}
rules {
step sokoban
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert_eq!(scene.state.puzzles[0].name, "sokoban");
    assert!(matches!(
        &scene.components[0],
        SceneComponent::Frame(frame) if frame.kind == "puzzle" && frame.source == "sokoban"
    ));
    assert!(matches!(
        &scene.puzzle_rule,
        Some(ScenePuzzleRule { target, rule }) if target == "sokoban" && rule == "rules"
    ));
}

#[test]
fn scene_signal_input_handler_can_step_puzzle_for_direction_set() {
    let source = r#"
title = signal_handler_step

puzzle board {
input right
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
right [ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene playing {
var input = signal none
layout {
puzzle board = board
}
keys {
ArrowRight -> input = right
}
on input in directions {
step board
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert!(scene.state.variables.iter().any(|variable| {
        variable.name == "input"
            && variable.kind == SceneVarKind::Signal
            && variable.default == SceneValue::Symbol("none".to_string())
    }));
    assert!(matches!(
        &scene.key_bindings[0].effect,
        SceneEffect::SetVariable { name, value }
            if name == "input" && matches!(value, SceneExpr::Path(path) if path == &vec!["right".to_string()])
    ));
    assert!(matches!(
        &scene.transitions[0].trigger,
        SceneTransitionTrigger::Signal(SceneExpr::Binary {
            op: SceneBinaryOp::In,
            ..
        })
    ));
    assert!(matches!(
        &scene.transitions[0].effect,
        SceneEffect::Apply { rule, target, args }
            if rule == "rules"
                && target.as_deref() == Some("board")
                && matches!(args.as_slice(), [SceneExpr::Path(path)] if path == &vec!["input".to_string()])
    ));
}

#[test]
fn scene_rules_reject_component_rules_path() {
    let source = r#"
title = old_component_rules_path

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene playing {
layout {
board
}
rules {
board.rules
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("scene rules do not call component rules by path; use `step <puzzle>`"));
}

#[test]
fn scene_frame_component_places_content_slot_without_model_kind() {
    let source = r#"
title = frame_slot

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {

}
levels {
level "start"
P
}
}

scene playing {
layout {
puzzle board = board
frame board
}
}
"#;
    let loaded = parse_game(source).unwrap();
    assert!(matches!(
        &loaded.scenes[0].components[1],
        SceneComponent::Frame(frame) if frame.kind == "frame" && frame.source == "board"
    ));
}

#[test]
fn scene_rejects_old_rhs_puzzle_slot_declaration() {
    let source = r#"
title = old_rhs_puzzle_slot

puzzle default {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
}
level "start" {
P
}
}

scene playing {
layout {
board = puzzle default
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(
        error.contains("scene puzzle declaration must be: puzzle <slot> = <model>"),
        "{error}"
    );
}

#[test]
fn scene_can_still_name_multiple_puzzle_slots_explicitly() {
    let source = r#"
title = named_slots

puzzle sokoban {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene playing {
layout {
puzzle sokoban1 = sokoban
puzzle sokoban2 = sokoban
}
rules {
step sokoban1
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert_eq!(scene.state.puzzles[0].name, "sokoban1");
    assert_eq!(scene.state.puzzles[1].name, "sokoban2");
    assert!(matches!(
        &scene.puzzle_rule,
        Some(ScenePuzzleRule { target, rule }) if target == "sokoban1" && rule == "rules"
    ));
}

#[test]
fn input_declaration_defines_direction_input() {
    let source = r#"
title = command_direction

puzzle board {
layers {
actor = Player
}
input right direction right
legend {
. = empty
P = Player
}
rules {

input directions [ Player | no Player ] -> [ | Player ]
}
level "start" {
P.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    assert!(loaded.input_labels.values().any(|label| label == "right"));
}

#[test]
fn scene_inputs_reject_equals_assignment_syntax() {
    let source = r#"
title = old_scene_inputs

puzzle board {
layers {
actor = Player
}
rules {

[ Player ] -> [ Player ]
}
}

levels default of board {
legend {
. = empty
P = Player
}
level "start" {
P
}
}

scene playing {
keys {
resume = Escape
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(
        error.contains("keys row must be: <key...> -> <scene effect-or-input>"),
        "{error}"
    );
}

#[test]
fn button_action_assignment_uses_equals() {
    let source = r#"
title = button_action_assignment

puzzle board {
layers {
actor = Player
}
rules {

[ Player ] -> [ Player ]
}
}

levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}

scene menu {
layout {
button "Resume" -> input resume
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    let SceneComponent::Button(button) = &scene.components[0] else {
        panic!("expected button component");
    };
    assert!(matches!(&button.effect, SceneEffect::Input(action) if action == "resume"));
}

#[test]
fn scene_box_is_layout_container_and_panel_is_not_scene_syntax() {
    let source = r#"
title = scene_box_layout

puzzle board {
layers {
actor = Player
}
rules {

[ Player ] -> [ Player ]
}
}

levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}

scene menu {
layout size 4 3 {
box size 3 2 gap 1 align left top {
text "Ready"
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.scenes[0].layout.size.unwrap().width, 4);
    assert_eq!(loaded.scenes[0].layout.size.unwrap().height, 3);
    assert!(matches!(
        &loaded.scenes[0].components[0],
        SceneComponent::Box(container)
            if container.layout.size.unwrap().width == 3
                && container.layout.size.unwrap().height == 2
                && container.layout.gap == Some(1)
                && matches!(&container.children[0], SceneComponent::Text(_))
    ));

    let rejected = source.replace("box size 3 2 gap 1 align left top {", "panel {");
    let error = parse_game(&rejected).unwrap_err();
    assert!(
        error.to_string().contains("unknown layout directive panel"),
        "expected panel to be rejected, got {error}"
    );
}

#[test]
fn explicit_scene_input_and_component_effect_parse_separately() {
    let source = r#"
title = explicit_scene_input_effects

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {

[ Player ] -> [ Player ]
}
level "start" {
P
}
input right
}

scene playing {
layout {
board
}
keys {
ArrowRight -> input right
ArrowDown -> component_effect down
r -> board.restart
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert!(matches!(&scene.key_bindings[0].effect, SceneEffect::Input(input) if input == "right"));
    assert!(
        matches!(&scene.key_bindings[1].effect, SceneEffect::ComponentEffect(effect) if effect == "down")
    );
    assert!(matches!(
        &scene.key_bindings[2].effect,
        SceneEffect::ResetPuzzle { target } if target == "board"
    ));
    assert_eq!(
        loaded.controls.arrows.get(&ArrowKey::Right).copied(),
        Some(input_named(&loaded, "right"))
    );
}

#[test]
fn scene_effect_wrapper_marks_scene_commands_explicitly() {
    let source = r#"
title = scene_effect_wrapper

puzzle board {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
P = Player
}
rules {

[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene playing {
layout {
board
button "Restart" -> board.restart
}
}
"#;

    let loaded = parse_game(source).unwrap();
    assert!(loaded.scenes[0].components.iter().any(|component| matches!(
        component,
        SceneComponent::Button(button)
            if matches!(&button.effect, SceneEffect::ResetPuzzle { target } if target == "board")
    )));
}

#[test]
fn button_arrow_rejects_plain_action_rhs() {
    let source = r#"
title = old_button_action_arrow

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {

[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene menu {
layout {
button "Resume" -> resume
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown scene action: resume"), "{error}");
}

#[test]
fn layout_for_can_project_levels_into_scrollable_column() {
    let source = r#"
title = level_projection_view

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {

[ Player ] -> [ Player ]
}
levels {
level "first" {
P
}
level "second" {
P
}
}
}

scene level_select {
layout {
column scroll=true {
for level in levels {
button join(level.num, ". ", level.title, " ", level.solved) -> goto playing(level)
}
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let SceneComponent::Column(column) = &loaded.scenes[0].components[0] else {
        panic!("expected scrollable column");
    };
    assert!(column.layout.scroll);

    assert_eq!(column.children.len(), 2);
    let SceneComponent::Button(button) = &column.children[0] else {
        panic!("expected level button");
    };
    assert!(matches!(&button.label, SceneExpr::Call { name, args }
            if name == "join"
                && args.iter().any(|arg| matches!(arg, SceneExpr::Int(1)))
                && args.iter().any(|arg| matches!(arg, SceneExpr::LevelSelector { collection, key: SceneLevelKey::Index(0), property: Some(property) }
                    if collection == "levels" && property == "title"))
                && args.iter().any(|arg| matches!(arg, SceneExpr::LevelSelector { collection, key: SceneLevelKey::Index(0), property: Some(property) }
                    if collection == "levels" && property == "solved"))));
    assert!(matches!(
        &button.effect,
        SceneEffect::Goto { scene, params }
            if scene == "playing"
                && params.len() == 1
                && matches!(&params[0], SceneEffectParam::Level(SceneExpr::LevelSelector { collection, key: SceneLevelKey::Index(0), property: None })
                    if collection == "levels")
    ));
}

#[test]
fn layout_for_can_project_levels_with_author_chosen_binding_name() {
    let source = r#"
title = level_projection_binding

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
levels {
level "first" {
P
}
}
}

scene level_select {
layout {
for l in levels {
button join(l.num, ". ", l.title) -> goto playing(l)
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let SceneComponent::Button(button) = &loaded.scenes[0].components[0] else {
        panic!("expected level button");
    };
    assert!(matches!(&button.label, SceneExpr::Call { name, args }
            if name == "join"
                && args.iter().any(|arg| matches!(arg, SceneExpr::Int(1)))
                && args.iter().any(|arg| matches!(arg, SceneExpr::LevelSelector { collection, key: SceneLevelKey::Index(0), property: Some(property) }
                    if collection == "levels" && property == "title"))));
    assert!(matches!(
        &button.effect,
        SceneEffect::Goto { scene, params }
            if scene == "playing"
                && params.len() == 1
                && matches!(&params[0], SceneEffectParam::Level(SceneExpr::LevelSelector { collection, key: SceneLevelKey::Index(0), property: None })
                    if collection == "levels")
    ));
}

#[test]
fn layout_for_levels_respects_scene_resources_declared_after_layout() {
    let source = r#"
title = level_projection_resources

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
levels world of board {
level "first" {
P
}
level "second" {
P
}
}
}

scene level_select {
layout {
for level in levels {
button join(level.num, ". ", level.title) -> goto playing(level)
}
}
resources {
levels second
}
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.scenes[0].components.len(), 1);
    let SceneComponent::Button(button) = &loaded.scenes[0].components[0] else {
        panic!("expected one expanded level button");
    };
    assert!(matches!(&button.label, SceneExpr::Call { name, args }
        if name == "join"
            && args.iter().any(|arg| matches!(arg, SceneExpr::Int(2)))
            && args.iter().any(|arg| matches!(arg, SceneExpr::LevelSelector { collection, key: SceneLevelKey::Index(1), property: Some(property) }
                if collection == "levels" && property == "title"))));
}

#[test]
fn layout_for_does_not_replace_binding_inside_quoted_text() {
    let source = r#"
title = level_projection_quotes

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "first" {
P
}
}

scene level_select {
layout {
for level in levels {
button "level.title" -> goto playing(level)
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let SceneComponent::Button(button) = &loaded.scenes[0].components[0] else {
        panic!("expected expanded level button");
    };
    assert!(matches!(&button.label, SceneExpr::Text(value) if value == "level.title"));
    assert!(matches!(
        &button.effect,
        SceneEffect::Goto { params, .. }
            if matches!(&params[0], SceneEffectParam::Level(SceneExpr::LevelSelector { collection, key: SceneLevelKey::Index(0), property: None })
                if collection == "levels")
    ));
}

#[test]
fn level_menu_component_accepts_canonical_options() {
    let source = r#"
title = typed_level_menu

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {

[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene level_select {
layout {
level_menu {
button "Title" -> goto title
show_index = true
show_solved = true
columns = 4
wrap = true
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert_eq!(scene.name, "level_select");
    let SceneComponent::LevelMenu(menu) = &scene.components[0] else {
        panic!("expected level menu component");
    };
    assert!(menu.show_index);
    assert!(menu.show_cleared);
    assert_eq!(menu.columns, Some(4));
    assert!(menu.wrap);
    assert!(matches!(
        &menu.buttons[0].effect,
        SceneEffect::Goto { scene, params } if scene == "title" && params.is_empty()
    ));
}

#[test]
fn bare_level_menu_does_not_capture_following_button() {
    let source = r#"
title = bare_level_menu_button_boundary

puzzle board {
layers {
actor = Player
}
empty .
legend {
P = Player
}
rules {
}
level "start" {
P
}
}

scene level_select {
layout {
level_menu
button "Back" -> goto title
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    let [
        SceneComponent::LevelMenu(menu),
        SceneComponent::Button(button),
    ] = scene.components.as_slice()
    else {
        panic!("expected bare level_menu followed by top-level button");
    };
    assert!(menu.buttons.is_empty());
    assert!(matches!(
        &button.effect,
        SceneEffect::Goto { scene, params } if scene == "title" && params.is_empty()
    ));
}

#[test]
fn typed_level_menu_scene_is_rejected() {
    let source = r#"
title = old_level_menu_scene

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {

[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene level_menu level_select {
level_menu {
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("scene level_menu template is not supported"));
}

#[test]
fn level_menu_rejects_on_off_option_aliases() {
    let source = r#"
title = old_level_menu_options

puzzle board {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {

[ Player ] -> [ Player ]
}
level "start" {
P
}
}

scene level_select {
layout {
level_menu {
index on
}
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("level_menu option must be"));
}

#[test]
fn level_menu_rejects_inline_source_and_effect() {
    let source = r#"
title = old_level_menu_inline

puzzle sokoban {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {

[ Player ] -> [ Player ]
}
}

levels microban of sokoban {
level "start" {
P
}
}

scene level_select {
layout {
level_menu microban -> goto playing(level)
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("level_menu takes no inline source or effect"));
}

#[test]
fn scene_root_rejects_layout_components() {
    let source = r#"
title = title_scene

scene title {
title = title
choice "Play" -> goto playing
on_scene_start {
stop_music locked_room
}
}

scene playing {
layout {
text "Playing"
}
}
"#;
    let error = super::parse_game2d(source).unwrap_err().to_string();
    assert!(error.contains("scene layout components must be inside `layout { ... }`"));
}

#[test]
fn title_scene_keeps_layout_buttons_and_rules_explicit() {
    let source = r#"
title = title_scene

puzzle default {
layers {
actor = Player
}

legend {
. = empty
P = Player
}

rules {

once right [ Player | no Player ] -> [ | Player ]
}

level "start" {
P.
}
}

scene title {
layout {
title = title
subtitle = "A tiny puzzle"
button "Play" -> goto playing
button "Levels" -> goto level_select
}
}

scene playing {
layout {
puzzle board = default
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let title = &loaded.scenes[0];
    assert_eq!(title.name, "title");
    assert_eq!(title.components.len(), 4);
    assert!(title.key_bindings.is_empty());
    assert!(title.transitions.is_empty());
}

#[test]
fn top_level_metadata_is_available_to_scenes() {
    let source = r#"
title = Tiny Metadata Game
subtitle = "Small Metadata Puzzle"
author = "Puzzle Person"
homepage = "https://example.com/puzzle"

puzzle default {
layers {
actor = Player
}

legend {
. = empty
P = Player
}

rules {

}

level "start" {
P
}
}

scene title {
layout {
title = title
subtitle = subtitle
text author
text homepage
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.title, "Tiny Metadata Game");
    assert_eq!(loaded.subtitle.as_deref(), Some("Small Metadata Puzzle"));
    assert_eq!(loaded.author.as_deref(), Some("Puzzle Person"));
    assert_eq!(
        loaded.homepage.as_deref(),
        Some("https://example.com/puzzle")
    );
}

#[test]
fn top_level_name_metadata_is_rejected() {
    let source = r#"
name Old Metadata

puzzle default {
layers {
actor = Player
}

legend {
. = empty
P = Player
}

rules {

}

level "start" {
P
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("top-level `name` metadata was removed; use `title = <text>`"));
}

#[test]
fn top_level_lifecycle_blocks_point_to_puzzle_scope() {
    let source = r#"
title = lifecycle_scope

on_level_clear {
next_level
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(
        error.contains(
            "on_level_clear is a puzzle lifecycle block; put it inside `puzzle <name> { ... }`"
        ),
        "{error}"
    );
    assert!(!error.contains("top-level directive must be"), "{error}");
}

#[test]
fn all_top_level_puzzle_lifecycle_blocks_share_scope_diagnostic() {
    for lifecycle in ["on_level_start", "on_level_clear", "on_last_level_clear"] {
        let source = format!(
            r#"
title = lifecycle_scope

{lifecycle} {{
next_level
}}
"#
        );

        let error = parse_game(&source).unwrap_err().to_string();
        assert!(
            error.contains(&format!("{lifecycle} is a puzzle lifecycle block")),
            "{error}"
        );
    }
}

#[test]
fn occurrence_mark_supports_multiple_marks_direction_and_int_values() {
    let source = r#"
title = mark_marks

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

marks {
checked
move = directions
count = int
}

legend B = Box

rules {
once right [ Box ] -> [ Box{checked move=> count=3} ]
once right [ Box{checked move=> count=3} no Marker ] -> [ Box{no checked no move count=2} Marker ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
    assert!(moved.cell_mark().iter().all(Vec::is_empty));
}

#[test]
fn bool_mark_uses_presence_and_no_syntax() {
    let source = r#"
title = bool_mark

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

marks {
flag = bool
}

legend B = Box

rules {
once [ Box ] -> [ Box{flag} ]
once [ Box{flag} no Marker ] -> [ Box{no flag} Marker ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    eprintln!("rules: {:?}", loaded.game.rules());
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
}

#[test]
fn colon_mark_name_does_not_mean_value_assignment() {
    let source = r#"
title = mark_colon

puzzle default {
layers {
__legacy_layer_0 = Box
}
empty .

marks {
count = int
}

legend B = Box

rules {
once [ Box ] -> [ Box{count:3} ]
}

level "start" {
B
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown mark"));
}

#[test]
fn mark_names_can_use_numeric_colon_parts() {
    let source = r#"
title = numeric_qualified_mark

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

marks {
count:3
}

legend B = Box

rules {
once [ Box ] -> [ Box{count:3} ]
once [ Box{count:3} no Marker ] -> [ Box Marker ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
}

#[test]
fn mark_names_can_use_direction_glyph_colon_parts() {
    let source = r#"
title = glyph_qualified_mark

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

marks {
push:>
pull:<
rise:^
fall:v
}

legend B = Box

rules {
once [ Box ] -> [ Box{push:> pull:< rise:^ fall:v} ]
once [ Box{push:> pull:< rise:^ fall:v} no Marker ] -> [ Box Marker ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
}

#[test]
fn qualified_mark_names_can_use_colons() {
    let source = r#"
title = qualified_mark

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

marks {
enter:directions = bool
intent:move = directions
}

legend B = Box

rules {
once [ Box ] -> [ Box{enter:directions intent:move=right} ]
once [ Box{enter:directions intent:move=right} no Marker ] -> [ Box Marker ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
}

#[test]
fn unmentioned_occurrence_mark_is_preserved_when_same_occurrence_moves() {
    let source = r#"
title = moving_mark

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

marks {
hot
}

legend B = Box

rules {
once [ Box ] -> [ Box{hot} ]
once right [ Box | ] -> [ | Box ]
once [ Box{hot} no Marker ] -> [ Box Marker ]
}

level "start" {
B.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 1, 0, marker));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
}

#[test]
fn omitted_rhs_mark_removes_explicit_lhs_mark_on_moved_occurrence() {
    let source = r#"
title = moving_mark_remove

puzzle default {
layers {
__legacy_layer_1 = Box
}
empty .

marks {
hot
}

legend B = Box

rules {
once [ Box ] -> [ Box{hot} ]
once right [ Box{hot} | ] -> [ | Box ]
}

level "start" {
B.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let box_object = object_named(&loaded, "Box");
    assert!(moved.has_object(&loaded.game, 1, 0, box_object));
    assert!(!moved.has_mark_key(&loaded.game, 1, 0, box_object, puzzle_core::MarkId(3),));
}

#[test]
fn same_cell_occurrence_is_preserved_before_move_inference() {
    let source = r#"
title = same_cell_preserve

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

marks {
hot
}

legend B = Box

rules {
once [ Box ] -> [ Box{hot} ]
once right [ Box | no Box ] -> [ Box | Box ]
once [ Box{hot} | Box no Marker ] -> [ Box{hot} | Box Marker ]
}

level "start" {
B.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let box_object = object_named(&loaded, "Box");
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, box_object));
    assert!(moved.has_object(&loaded.game, 1, 0, box_object));
    assert!(moved.has_object(&loaded.game, 1, 0, marker));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
}

#[test]
fn group_selectors_accept_mark_blocks() {
    let source = r#"
title = group_mark

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box Crate
}
empty .

marks {
hot
}

groups {
mover = Box Crate
}
legend B = Box

rules {
once [ Box ] -> [ Box{hot} ]
once [ mover{hot} no Marker ] -> [ mover{hot} Marker ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
}

#[test]
fn group_selector_removal_also_removes_movement_mark() {
    let source = r#"
title = group_remove_movement_mark

puzzle default {
layers {
  floor = Background
  actor = Player Key Lock
}
empty .

groups {
key = Key
lock = Lock
pushable = Key
}
legend P = Player
legend K = Key
legend L = Lock

rules {
  once [ pushable ] -> [ pushable{>} ]
  [ key{>} | lock ] -> [ | ]
}

level "start" {
PKL
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let player = object_named(&loaded, "Player");
    let key = object_named(&loaded, "Key");
    let lock = object_named(&loaded, "Lock");

    assert!(moved.has_object(&loaded.game, 0, 0, player));
    assert!(!moved.has_object(&loaded.game, 1, 0, key));
    assert!(!moved.has_object(&loaded.game, 2, 0, lock));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
}

#[test]
fn cell_and_occurrence_mark_share_names_but_have_distinct_anchors() {
    let source = r#"
title = cell_mark

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

marks {
mark
}

legend B = Box

rules {
once [ Box ] -> [ Box{mark} ]
once [ Box{mark} ] -> [ Box {mark} ]
once [ Box {mark} no Marker ] -> [ Box Marker ]
once [ Box{mark} {mark} ] -> [ Box ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
    assert!(moved.cell_mark().iter().all(Vec::is_empty));
    assert!(
        loaded
            .warnings
            .iter()
            .any(|warning| warning.contains("changes anchor"))
    );
    assert!(
        loaded
            .warnings
            .iter()
            .any(|warning| warning.contains("both a cell and an object occurrence"))
    );
}

#[test]
fn rewrite_rejects_same_layer_rhs_cell_conflict_with_author_message() {
    let source = r#"
title = rhs_layer_conflict

puzzle default {
layers {
__legacy_layer_0 = Player Box
}
empty .

legend P = Player

rules {
[ Player ] -> [ Player Box ]
}

level "start" {
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains(
        "cell pattern cannot contain both `Player` and `Box` because they are in the same collision layer"
    ));
    assert!(error.contains("[ Player ] -> [ Player Box ]"));
}

#[test]
fn movement_mark_prefix_and_legacy_inline_sugar_work_with_transition_local_lifetime() {
    let source = r#"
title = anonymous_mark

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

marks {
checked
}

legend B = Box

rules {
once right [ Box ] -> [ Box{> checked 7} ]
once right [ > Box{checked 7} no Marker ] -> [ Box Marker ]
once right [ Box Marker ] -> [ 3 Box Marker ]
once right [ 3 Box Marker ] -> [ true Box Marker ]
once right [ true Box Marker ] -> [ false Box Marker ]
once right [ false Box Marker ] -> [ Box ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let box_object = object_named(&loaded, "Box");
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, box_object));
    assert!(!moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
}

#[test]
fn action_statement_is_rejected() {
    let source = r#"
title = action_button

puzzle board {
layers {
floor = Target Open
actor = Player
}
legend {
. = empty
P = Player
T = Target
O = Open
}
rules {

action Player
once [ Player{__action} | Target ] -> [ Player | Open ]
}
level "start" {
PT
}
}

scene playing {
layout {
board
}
keys {
x -> input action
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("`action` statements were removed"));
}

#[test]
fn move_call_without_explicit_routine_reports_unknown_routine() {
    let source = r#"
title = move_requires_explicit_routine

puzzle default {
layers {
actor = Box
marker = Marker
}

legend {
B = Box
. = empty
}

rules {
move
}

levels {
level "start"
B
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown routine call: move"));
}

#[test]
fn explicit_move_routine_remains_callable() {
    let source = r#"
title = explicit_move_routine

puzzle default {
layers {
actor = Box
marker = Marker
}

legend {
B = Box
. = empty
}

routine move {
[ Box ] -> [ Box Marker ]
}

rules {
move
}

levels {
level "start"
B..
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved = transition_state(
        &loaded.game,
        &loaded.levels[0].initial_state,
        input_named(&loaded, "right"),
    )
    .unwrap();
    let box_object = object_named(&loaded, "Box");
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, box_object));
    assert!(moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn directions_mark_sugar_matches_any_movement_value() {
    let source = r#"
title = directions_sugar

puzzle default {
layers {
actor = Box
floor = Marker
}

legend {
B = Box
. = empty
}

rules {

once right [ Box ] -> [ > Box ]
once [ Box{directions} ] -> [ Box Marker ]
}

levels {
legend {
. = empty
}
level "start"
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved = transition_state(
        &loaded.game,
        &loaded.levels[0].initial_state,
        input_named(&loaded, "right"),
    )
    .unwrap();
    let box_object = object_named(&loaded, "Box");
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, box_object));
    assert!(moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn prefix_movement_mark_sugar_matches_braced_selector_mark() {
    let source = r#"
title = prefix_movement_mark_sugar

puzzle default {
layers {
actor = Player
floor = Marker
}

legend {
P = Player
. = empty
}

rules {

once right [ Player ] -> [ right Player ]
once [ right Player ] -> [ Player Marker ]
}

levels {
level "start"
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved = transition_state(
        &loaded.game,
        &loaded.levels[0].initial_state,
        input_named(&loaded, "right"),
    )
    .unwrap();
    let player = object_named(&loaded, "Player");
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, player));
    assert!(moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn prefix_directions_mark_sugar_matches_any_movement_value() {
    let source = r#"
title = prefix_directions_mark_sugar

puzzle default {
layers {
actor = Player
floor = Marker
}

legend {
P = Player
. = empty
}

rules {

once right [ Player ] -> [ right Player ]
once [ directions Player ] -> [ Player Marker ]
}

levels {
level "start"
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved = transition_state(
        &loaded.game,
        &loaded.levels[0].initial_state,
        input_named(&loaded, "right"),
    )
    .unwrap();
    let player = object_named(&loaded, "Player");
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, player));
    assert!(moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn no_directions_mark_sugar_forbids_any_movement_value() {
    let source = r#"
title = no_directions_sugar

puzzle default {
layers {
actor = Box
floor = Marker
}

legend {
B = Box
. = empty
}

rules {

once right [ Box ] -> [ > Box ]
once [ Box{no directions} ] -> [ Box Marker ]
}

levels {
level "start"
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved = transition_state(
        &loaded.game,
        &loaded.levels[0].initial_state,
        input_named(&loaded, "right"),
    )
    .unwrap();
    let box_object = object_named(&loaded, "Box");
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, box_object));
    assert!(!moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn parallel_and_perpendicular_mark_sets_expand_relative_to_rule_orientation() {
    let source = r#"
title = relative_movement_sets

puzzle default {
layers {
actor = Box Crate
floor = ParallelMarker PerpendicularMarker
}

legend {
B = Box
C = Crate
. = empty
}

rules {
once right [ Box ] -> [ > Box ]
once right [ Crate ] -> [ ^ Crate ]
once right [ Box{parallel} ] -> [ Box ParallelMarker ]
once right [ Crate{parallel} ] -> [ Crate ParallelMarker ]
once right [ Crate{perpendicular} ] -> [ Crate PerpendicularMarker ]
}

levels {
level "start"
BC
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved = transition_state(
        &loaded.game,
        &loaded.levels[0].initial_state,
        input_named(&loaded, "right"),
    )
    .unwrap();
    let parallel_marker = object_named(&loaded, "ParallelMarker");
    let perpendicular_marker = object_named(&loaded, "PerpendicularMarker");

    assert!(moved.has_object(&loaded.game, 0, 0, parallel_marker));
    assert!(!moved.has_object(&loaded.game, 1, 0, parallel_marker));
    assert!(moved.has_object(&loaded.game, 1, 0, perpendicular_marker));
}

#[test]
fn parallel_mark_prefix_sugar_matches_object_movement_set() {
    let source = r#"
title = parallel_prefix_sugar

puzzle default {
layers {
actor = Box
floor = Marker
}

legend {
B = Box
. = empty
}

rules {
once right [ Box ] -> [ < Box ]
once right [ parallel Box ] -> [ Box Marker ]
}

levels {
level "start"
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved = transition_state(
        &loaded.game,
        &loaded.levels[0].initial_state,
        input_named(&loaded, "right"),
    )
    .unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn prefixless_parallel_mark_pattern_expands_cardinal_directions() {
    let source = r#"
title = prefixless_parallel

puzzle default {
layers {
actor = Box
floor = Marker
}

legend {
B = Box
. = empty
}

rules {
once right [ Box ] -> [ > Box ]
once [ Box{parallel} ] -> [ Box Marker ]
}

levels {
level "start"
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved = transition_state(
        &loaded.game,
        &loaded.levels[0].initial_state,
        input_named(&loaded, "right"),
    )
    .unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn variant_axis_values_can_define_mark_without_becoming_value_sets() {
    let source = r#"
title = variant_mark

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

tags {
color = red blue
}

marks {
color
paint = color
}

legend B = Box

rules {
once [ Box ] -> [ Box{color paint=blue} ]
once [ Box{color paint=blue} no Marker ] -> [ Box Marker ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_mark().iter().all(Vec::is_empty));
}

#[test]
fn level_start_keeps_raw_initial_state_and_keeps_runtime_program() {
    let source = r#"
title = level_start

puzzle default {
layers {
__legacy_layer_0 = Source
__legacy_layer_1 = Marker
}
empty .

legend S = Source

on_level_start {
[ Source no Marker ] -> [ Source Marker ]
}

rules {

}

level "start" {
S
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(
        !loaded.levels[0]
            .initial_state
            .has_object(&loaded.game, 0, 0, marker)
    );
    assert!(loaded.level_start_program.is_some());
}

#[test]
fn rules_block_accepts_scope_local_routine() {
    let source = r#"
title = local_rules_routine

puzzle default {
layers {
base = Player
marker = Marker
}
empty .

legend P = Player

rules {
mark

routine mark {
[ Player no Marker ] -> [ Player Marker ]
}
}

level "start" {
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let marker = object_named(&loaded, "Marker");
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn level_start_accepts_scope_local_routine() {
    let source = r#"
title = local_level_start_routine

puzzle default {
layers {
base = Source
marker = Marker
}
empty .

legend S = Source

on_level_start {
mark_initial

routine mark_initial {
[ Source no Marker ] -> [ Source Marker ]
}
}

rules {
}

level "start" {
S
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let marker = object_named(&loaded, "Marker");
    let initial = &loaded.levels[0].initial_state;

    assert!(!initial.has_object(&loaded.game, 0, 0, marker));

    let started = transition_program(
        &loaded.game,
        loaded.level_start_program.as_deref().unwrap(),
        initial,
        InputId(0),
    )
    .unwrap();
    assert!(started.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn scope_local_routine_does_not_leak_to_lifecycle_block() {
    let source = r#"
title = local_routine_no_leak

puzzle default {
layers {
base = Source
marker = Marker
}
empty .

legend S = Source

rules {
routine mark_initial {
[ Source no Marker ] -> [ Source Marker ]
}
}

on_level_start {
mark_initial
}

level "start" {
S
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown routine call: mark_initial"));
}

#[test]
fn variable_routine_does_not_capture_caller_local_routine() {
    let source = r#"
title = local_routine_lexical_scope

puzzle default {
layers {
base = Source
marker = Marker
}
empty .

legend S = Source

routine variable_mark {
mark_initial
}

rules {
variable_mark

routine mark_initial {
[ Source no Marker ] -> [ Source Marker ]
}
}

level "start" {
S
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown routine call: mark_initial"));
}

#[test]
fn level_start_rejects_input_dependent_rules() {
    let source = r#"
title = level_start_input

puzzle default {
layers {
actor = Player
}
layers {
__legacy_layer_0 = Player actor
}
legend {
. = empty
P = Player
}

on_level_start {
input directions [ Player | ] -> [ | Player ]
}

rules {

}

level "start" {
P.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("on_level_start cannot depend on input"));
}

#[test]
fn level_start_rejects_input_dependent_local_routine() {
    let source = r#"
title = level_start_local_input

puzzle default {
layers {
actor = Player
}
layers {
__legacy_layer_0 = Player actor
}
legend {
. = empty
P = Player
}

on_level_start {
mark_initial

routine mark_initial {
input directions [ Player | ] -> [ | Player ]
}
}

rules {

}

level "start" {
P.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("on_level_start cannot depend on input"));
}

#[test]
fn at_prefixed_level_start_routine_uses_normal_runtime_program() {
    let source = r#"
title = display_level_start

puzzle default {
layers {
@__legacy_layer_1 = @Marker
}
empty .

layers {
__legacy_layer_0 = Source
}


legend S = Source

routine @mark_initial once {
[ Source no @Marker ] -> [ Source @Marker ]
}

on_level_start {
@mark_initial
}

rules {

}

level "start" {
S
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(!loaded.is_display_object(marker));
    assert!(
        !loaded.levels[0]
            .initial_state
            .has_object(&loaded.game, 0, 0, marker)
    );
    assert!(loaded.level_start_program.is_some());
}

#[test]
fn display_level_start_rejects_input_dependent_rules() {
    let source = r#"
title = display_level_start_input

puzzle default {
layers {
@__legacy_layer_1 = @Marker
}
empty .

layers {
__legacy_layer_0 = Player
}


legend P = Player

routine @mark_initial once {
input directions [ Player no @Marker | ] -> [ Player @Marker | ]
}

on_level_start {
@mark_initial
}

rules {

}

level "start" {
P.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("on_level_start cannot depend on input"));
}

#[test]
fn at_prefixed_level_start_routine_accepts_normal_object_writes() {
    let source = r#"
title = display_level_start_main_write

puzzle default {
layers {
__legacy_layer_1 = Player
}
empty .

layers {
__legacy_layer_0 = Source
__legacy_layer_1 = Marker
}

legend S = Source

routine @mark_initial once {
[ Source no Marker ] -> [ Source Marker ]
}

on_level_start {
@mark_initial
}

rules {

}

level "start" {
S
}
}
"#;
    parse_game(source).unwrap();
}

#[test]
fn level_clear_rejects_input_dependent_rules() {
    let source = r#"
title = level_clear_input

puzzle default {
layers {
actor = Player
}
layers {
__legacy_layer_0 = Player actor
}
legend {
. = empty
P = Player
}

on_level_clear {
input directions [ Player | ] -> [ | Player ]
}

rules {

}

level "start" {
P.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("on_level_clear cannot depend on input"));
}

#[test]
fn independent_lifecycle_lowering_errors_are_reported_together() {
    let source = r#"
title = multiple_lifecycle_errors

puzzle default {
layers {
actor = Player
}
layers {
__legacy_layer_0 = Player actor
}
legend {
. = empty
P = Player
}

on_level_start {
input directions [ Player | ] -> [ | Player ]
}

on_level_clear {
input directions [ Player | ] -> [ | Player ]
}

rules {

}

level "start" {
P.
}
}
"#;
    let report = parse_game(source).unwrap_err();
    let messages = report
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();

    assert!(
        messages.contains(&"on_level_start cannot depend on input"),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"on_level_clear cannot depend on input"),
        "{messages:?}"
    );
}

#[test]
fn independent_statement_parse_errors_are_reported_together() {
    let source = r#"
title = multiple_statement_parse_errors

puzzle default {
layers {
__legacy_layer_0 = Player
}

rules {
action push
do win
banana split
}

levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}
}
"#;
    let report = parse_game(source).unwrap_err();
    let messages = report
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();

    assert!(
        messages
            .contains(&"`action` statements were removed; use explicit input guards and rewrites"),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"`do` is obsolete; write the effect statement directly"),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"unknown statement directive banana"),
        "{messages:?}"
    );
}

#[test]
fn sibling_statement_blocks_are_parsed_after_inner_errors() {
    let source = r#"
title = sibling_statement_block_errors

puzzle default {
layers {
__legacy_layer_0 = Player
}

rules {
once {
action push
}

repeat {
do win
}

banana split
}

levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}
}
"#;
    let report = parse_game(source).unwrap_err();
    let messages = report
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();

    assert!(
        messages
            .contains(&"`action` statements were removed; use explicit input guards and rewrites"),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"`do` is obsolete; write the effect statement directly"),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"unknown statement directive banana"),
        "{messages:?}"
    );
}

#[test]
fn old_on_level_start_syntax_is_rejected() {
    let source = r#"
title = old_on_level_start

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
legend P = Player

on level_start {
}

rules {

}

level "start" {
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown puzzle directive on"));
}

#[test]
fn conditional_rule_call_short_form_runs_named_rule_when_pattern_matches() {
    let source = r#"
title = conditional_short

puzzle default {
layers {
__legacy_layer_1 = Player Wall Flag
}
empty .

legend P = Player
legend W = Wall
legend F = Flag

routine Mark once {
[ Player ] -> [ Flag ]
}

rules {
[ Player | Wall ] -> Mark
}

level "start" {
PW
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let flag = object_named(&loaded, "Flag");

    assert!(moved.has_object(&loaded.game, 0, 0, flag));
}

#[test]
fn conditional_rule_call_accepts_some_and_none_forms() {
    let source = r#"
title = conditional_some_none

puzzle default {
layers {
__legacy_layer_1 = Player Wall Flag
}
empty .

legend P = Player
legend W = Wall
legend F = Flag

routine Mark once {
[ Player ] -> [ Flag ]
}

rules {
if none([ Player | Wall ]) Mark
}

level "start" {
P.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let flag = object_named(&loaded, "Flag");

    assert!(moved.has_object(&loaded.game, 0, 0, flag));
}

#[test]
fn pattern_condition_block_accepts_else() {
    let source = r#"
title = pattern_condition_else

puzzle default {
layers {
__legacy_layer_1 = Player Box Flag Wall
}
empty .

legend P = Player
legend B = Box
legend F = Flag
legend W = Wall

rules {
if some([ Player ]) {
once [ Box ] -> [ Flag ]
} else {
once [ Box ] -> [ Wall ]
}
}

level "start" {
B.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let wall = object_named(&loaded, "Wall");

    assert!(moved.has_object(&loaded.game, 0, 0, wall));
}

#[test]
fn unknown_directive_is_rejected() {
    let source = r#"
title = old_keyword

puzzle default {
layers {
__legacy_layer_1 = Player
}
thing Player 1

rules {

}
}

levels default_levels of default {
legend {
. = empty
P = Player
}
level "start" {
P
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown puzzle directive thing"), "{error}");
}

#[test]
fn singular_group_block_is_rejected() {
    let source = r#"
title = old_group_block

puzzle default {
layers {
actor = Player Wall
}
group {
solid = Wall
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("`group { ... }` was removed; use `groups { ... }`"));
}

#[test]
fn singular_group_directive_is_rejected() {
    let source = r#"
title = old_group_directive

puzzle default {
layers {
actor = Player Wall
}
group solid = Wall
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("`group <name> = ...` was removed; use `groups { <name> = ... }`"));
}

#[test]
fn group_aliases_named_like_layout_keywords_stay_in_group_scope() {
    let source = r#"
title = group_alias_layout_keywords

puzzle main {
layers {
Player Box
}

groups {
box = Box Player
row = Player
column = Player
level_menu = Player
}

rules {
}
}

levels {
legend {
. = empty
P = Player
}

level "start"
P
}
"#;

    let loaded = parse_game(source).unwrap();
    assert!(loaded.object_groups.contains_key("box"));
    assert!(loaded.object_groups.contains_key("row"));
    assert!(loaded.object_groups.contains_key("column"));
    assert!(loaded.object_groups.contains_key("level_menu"));
}

#[test]
fn domain_keyword_is_not_part_of_public_syntax() {
    let source = r#"
title = old_domain

puzzle default {
layers {
__legacy_layer_1 = Box
}
empty .

domain color red blue
legend B = Box

rules {

}

level "start"
B
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown puzzle directive domain"));
}

#[test]
fn end_is_allowed_as_user_defined_object_name() {
    let source = r#"
title = end_name

puzzle default {
layers {
end
}
rules {
}
}

levels test of default {
legend {
. = empty
E = end
}
E
}
"#;

    let loaded = super::parse_game2d(source).unwrap();
    let end = object_named(&loaded, "end");
    assert!(
        loaded.levels[0]
            .initial_state
            .has_object(&loaded.game, 0, 0, end)
    );
}

#[test]
fn layer_is_allowed_as_user_defined_object_name() {
    let source = r#"
title = layer_name

puzzle default {
layers {
floor = layer
}
rules {
}
}

levels test of default {
legend {
. = empty
L = layer
}
L
}
"#;

    let loaded = super::parse_game2d(source).unwrap();
    let layer = object_named(&loaded, "layer");
    assert!(
        loaded.levels[0]
            .initial_state
            .has_object(&loaded.game, 0, 0, layer)
    );
}

#[test]
fn bare_tag_set_assignment_is_not_canonical_syntax() {
    let source = r#"
title = old_tag_assignment

puzzle default {
layers {
__legacy_layer_1 = Box
}
empty .

color = red blue


rules {

}

levels {
legend {
. = empty
}
level "start"
.
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("tag sets must be declared inside `tags { ... }`"));
}

#[test]
fn directions_directive_is_not_part_of_public_syntax() {
    let source = r#"
title = old_directions

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
directions
rules {
once input directions [ Player | ] -> [ | Player ]
}
levels {
level "start"
P.
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown puzzle directive directions"));
}

#[test]
fn parses_declared_assets() {
    let source = r#"
title = assets_test

assets {
"game.css"
"visuals.js"
"sprites/player.png"
}

puzzle sokoban {
layers {
solid = Player
}
legend {
. = empty
P = Player
}
rules {

}
levels {
level "one"
P
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.assets.entries.len(), 3);
    assert_eq!(loaded.assets.entries[0].kind, AssetKind::Css);
    assert_eq!(loaded.assets.entries[0].path, "game.css");
    assert_eq!(loaded.assets.entries[1].kind, AssetKind::Script);
    assert_eq!(loaded.assets.entries[1].path, "visuals.js");
    assert_eq!(loaded.assets.entries[2].kind, AssetKind::File);
    assert_eq!(loaded.assets.entries[2].path, "sprites/player.png");
}

#[test]
fn assets_reject_typed_entry_syntax() {
    let source = r#"
title = assets_old_syntax

assets {
css "game.css"
}

puzzle sokoban {
layers {
solid = Player
}
legend {
. = empty
P = Player
}
rules {
}
levels {
level "one"
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("css row must be: <string>"));
}

#[test]
fn top_level_levels_and_sprites_are_canonical_resources() {
    let source = r##"
title = top_resources

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
legend P = Player
rules {

}
}

sprites {
sprite {
selector = Player
colors = #fff
}
}

levels worldA of default {
level "1"
P

level {
P
}
}
"##;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.levels.len(), 2);
    assert_eq!(loaded.levels[0].name, "1");
    assert_eq!(loaded.levels[0].pack.as_deref(), Some("worldA"));
    assert_eq!(loaded.levels[0].puzzle, "default");
    assert_eq!(loaded.levels[1].name, "worldA.2");
    assert_eq!(loaded.visuals.sprites.len(), 1);
    assert_eq!(loaded.scenes[0].resources.levels, ResourceSelection::All);
}

#[test]
fn top_level_sprites_with_nested_tables_do_not_leak_after_prior_model_error() {
    let source = r##"
title = recovered_sprites_scope

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
legend P = Player
lose_conditions {
no [ Missing ]
}
rules {

}
}

sprites {
palette {
white = #ffffff
black = #000000
}
shapes {
marks {
0
}
}
Player {
white black
shape mark
}
}

levels worldA of default {
legend {
. = empty
P = Player
}
level "1"
P
}
"##;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(
        error.contains("unknown object selector: [ Missing ]"),
        "{error}"
    );
    assert!(
        !error.contains("unknown top-level directive `white`"),
        "{error}"
    );
    assert!(
        !error.contains("unknown top-level directive `Player`"),
        "{error}"
    );
    assert!(
        !error.contains("unknown top-level directive `shape`"),
        "{error}"
    );
}

#[test]
fn model_error_recovery_keeps_scope_after_prior_if_else_block() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_error_recovery_scope_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let game_path = dir.join("game.puzzle");
    std::fs::write(
        &game_path,
        r##"
title = recovered_after_if_else

puzzle default {
tags {
state = open close
}
layers {
__legacy_layer_0 = Player Gate:state Box:state Goal:state
}
rules {
if some([ Gate:open ]) {
[ Gate:open ] -> [ Gate:close ]
} else {
[ Gate:close ] -> [ Gate:open ]
}
}
lose_conditions {
no [ Missing ]
}
}

levels worldA of default {
legend {
. = empty
P = Player
}
level "1"
P
}

sprites {
palette {
white = #ffffff
}
sprite {
selector = Player
colors = white
shape = {
0
}
}
}
"##,
    )
    .unwrap();

    let error = super::parse_game_file(&game_path).unwrap_err().to_string();

    assert!(
        error.contains("unknown object selector: [ Missing ]"),
        "{error}"
    );
    assert!(
        !error.contains("unknown top-level directive `lose_conditions`"),
        "{error}"
    );
    assert!(
        !error.contains("unknown top-level directive `Player`"),
        "{error}"
    );
}

#[test]
fn scene_resources_can_select_level_and_sprite_sets() {
    let source = r##"
title = scene_resources

puzzle default {
layers {
__legacy_layer_0 = Player Box
}
empty .
legend P = Player
rules {

}
}

sprites {
sprite {
selector = Player
colors = #fff
}
sprite {
selector = Box
colors = #000
}
}

levels worldA of default {
level "1"
P
}

scene select {
resources {
levels worldA
sprites Player
}
layout {
level_menu
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let scene = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == "select")
        .unwrap();

    assert_eq!(
        scene.resources.levels,
        ResourceSelection::Named(vec!["worldA".to_string()])
    );
    assert_eq!(
        scene.resources.sprites,
        ResourceSelection::Named(vec!["Player".to_string()])
    );
}

#[test]
fn game_file_can_declare_theme_metadata() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_import_theme_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let game_path = dir.join("game.puzzle");
    std::fs::write(
        &game_path,
        r##"
title = themed

puzzle default {
layers {
actor = Player
}
rules {

[ Player ] -> [ Player ]
}
}

levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}

theme {
preset = "clean"
accent_color = #2f7ebc
}
"##,
    )
    .unwrap();

    let loaded = super::parse_game2d_file(&game_path).unwrap();

    assert_eq!(loaded.theme.name.as_deref(), Some("clean"));
    assert_eq!(
        loaded
            .theme
            .variables
            .iter()
            .find(|variable| variable.name == "accent")
            .map(|variable| variable.value.as_str()),
        Some("#2f7ebc")
    );
    assert_eq!(loaded.levels.len(), 1);
}

#[test]
fn game_file_import_expansion_preserves_top_level_resource_braces() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_import_resource_scope_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let game_path = dir.join("game.puzzle");
    let source = r##"
title = file_resources

puzzle default {
layers {
__legacy_layer_0 = Player
}
rules {

}
}

sprites {
palette {
white = #ffffff
}
sprite {
selector = Player
colors = white
shape = {
0
}
}
}

levels worldA of default {
legend {
. = empty
P = Player
}
level "1"
P
}
"##;
    std::fs::write(&game_path, source).unwrap();

    assert_eq!(
        super::expand_game_imports_for_file(source, &game_path).unwrap(),
        source
    );
    let parsed_from_source = super::parse_game(source).unwrap();
    let Some(LoadedDocumentModel::Puzzle2d { game, .. }) = parsed_from_source.single_model() else {
        panic!("expected 2D model");
    };
    assert_eq!(game.visuals.sprites.len(), 1);

    let document = super::parse_game_file(&game_path).unwrap();

    let Some(LoadedDocumentModel::Puzzle2d { game, .. }) = document.single_model() else {
        panic!("expected 2D model");
    };
    assert_eq!(game.visuals.sprites.len(), 1);
    assert_eq!(game.levels.len(), 1);
}

#[test]
fn theme_background_alias_sets_background_variable() {
    let loaded = parse_game(
        r##"
title = themed
theme {
preset = "puzzlescript"
background = #123456
}
puzzle default {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}
"##,
    )
    .unwrap();

    assert_eq!(loaded.theme.name.as_deref(), Some("puzzlescript"));
    assert_eq!(
        loaded
            .theme
            .variables
            .iter()
            .find(|variable| variable.name == "background")
            .map(|variable| variable.value.as_str()),
        Some("#123456")
    );
}

#[test]
fn puzzlescript_import_accepts_background_theme_alias() {
    let canonical = translate_puzzlescript_to_canonical(
        r##"
title themed
background #123456
=======
OBJECTS
=======

Background
#000000

======
LEGEND
======
. = Background

================
COLLISIONLAYERS
================
Background

======
LEVELS
======
.
"##,
    )
    .unwrap();

    assert!(
        canonical.contains("theme {\npreset = \"puzzlescript\"\nbackground_color = #123456\n}")
    );
}

#[test]
fn theme_preset_can_be_selected_without_block() {
    let loaded = parse_game(
        r##"
title = themed
theme = "pixel"
puzzle default {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}
"##,
    )
    .unwrap();

    assert_eq!(loaded.theme.name.as_deref(), Some("pixel"));
    assert!(loaded.theme.variables.is_empty());
}

#[test]
fn theme_setting_accepts_assignment_syntax() {
    let loaded = parse_game(
        r##"
title = themed
theme {
preset = "clean"
background_color = #123456
accent_color = #abcdef
}
puzzle default {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}
"##,
    )
    .unwrap();

    assert_eq!(loaded.theme.name.as_deref(), Some("clean"));
    assert_eq!(
        loaded
            .theme
            .variables
            .iter()
            .find(|variable| variable.name == "background")
            .map(|variable| variable.value.as_str()),
        Some("#123456")
    );
    assert_eq!(
        loaded
            .theme
            .variables
            .iter()
            .find(|variable| variable.name == "accent")
            .map(|variable| variable.value.as_str()),
        Some("#abcdef")
    );
}

#[test]
fn theme_rejects_non_public_color_settings() {
    let error = parse_game(
        r##"
title = themed
theme {
preset = "clean"
board_color = #edf1f2
}
puzzle default {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}
"##,
    )
    .unwrap_err();

    assert!(error.to_string().contains("background_color"));
    assert!(error.to_string().contains("text_color"));
    assert!(error.to_string().contains("accent_color"));
    assert!(!error.to_string().contains("ui_font"));
}

#[test]
fn theme_rejects_non_color_style_settings() {
    let error = parse_game(
        r##"
title = themed
theme {
preset = "clean"
ui_font = Inter
}
puzzle default {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}
"##,
    )
    .unwrap_err();

    assert!(error.to_string().contains("accent_color"));
    assert!(error.to_string().contains("background_color"));
    assert!(error.to_string().contains("text_color"));
    assert!(
        error
            .to_string()
            .contains("theme setting must be one of: accent_color, background_color, text_color")
    );
}

#[test]
fn theme_block_requires_quoted_preset_value() {
    let error = parse_game(
        r##"
title = themed
theme {
preset = pixel
}
puzzle default {
layers {
actor = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
level "start" {
P
}
}
"##,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("preset must be a quoted string"));
}

#[test]
fn game_entry_resolution_uses_declared_puzzle_models() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_entry_resolution_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let game_path = dir.join("game.puzzle");
    let levels_path = dir.join("levels.puzzle");
    std::fs::write(&game_path, "puzzle entry {}\n").unwrap();
    std::fs::write(&levels_path, "levels {}\n").unwrap();

    assert_eq!(super::resolve_game_entry(&dir).unwrap(), game_path);
    assert_eq!(
        super::resolve_game_entry(&dir.join("game.puzzle")).unwrap(),
        game_path
    );
    assert_eq!(super::resolve_game_entry(&levels_path).unwrap(), game_path);
}

#[test]
fn game_entry_resolution_allows_non_game_named_model_files() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_named_entry_resolution_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(dir.join("fragments")).unwrap();
    let entry_path = dir.join("arcade.puzzle");
    let fragment_path = dir.join("fragments").join("levels.puzzle");
    std::fs::write(&entry_path, "puzzle arcade {}\n").unwrap();
    std::fs::write(&fragment_path, "levels {}\n").unwrap();

    assert_eq!(super::resolve_game_entry(&dir).unwrap(), entry_path);
    assert_eq!(
        super::resolve_game_entry(&fragment_path).unwrap(),
        entry_path
    );
}

#[test]
fn game_entry_resolution_accepts_puzzle3_extension() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_puzzle3_entry_resolution_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let game_path = dir.join("game.puzzle3");
    std::fs::write(
        &game_path,
        r#"
title = "3D Entry"

puzzle3 cube {
  layers {
    actor = Player
  }
  rules {
  }
}
"#,
    )
    .unwrap();

    assert_eq!(super::resolve_game_entry(&dir).unwrap(), game_path);
}

#[test]
fn parse_game_file_rejects_puzzle3_sections_in_puzzle_file() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_puzzle_profile_mismatch_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let game_path = dir.join("game.puzzle");
    std::fs::write(
        &game_path,
        r#"
title = "Wrong Extension"

puzzle3 cube {
  layers {
    actor = Player
  }
  rules {
  }
}
"#,
    )
    .unwrap();

    let error = super::parse_game_file(&game_path).unwrap_err().to_string();
    assert!(error.contains(".puzzle files cannot contain 3D"));
    assert!(error.contains("use .puzzle3"));
}

#[test]
fn parse_game_file_rejects_puzzle_sections_in_puzzle3_file() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_puzzle3_profile_mismatch_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let game_path = dir.join("game.puzzle3");
    std::fs::write(
        &game_path,
        r#"
title = "Wrong Extension"

puzzle board {
  layers {
    actor = Player
  }
  legend {
    P = Player
  }
  rules {
  }
  level "start" {
    P
  }
}
"#,
    )
    .unwrap();

    let error = super::parse_game_file(&game_path).unwrap_err().to_string();
    assert!(error.contains(".puzzle3 files must contain 3D"));
}

#[test]
fn folder_without_puzzle_model_is_not_auto_resolved() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_entry_missing_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("only.puzzle"), "levels {}\n").unwrap();

    assert!(super::resolve_game_entry(&dir).is_err());
}

#[test]
fn parses_display_floor_projection_object() {
    let loaded = parse_game(display_floor_projection_source()).unwrap();

    assert!(loaded.object_labels.values().any(|label| label == "@Floor"));
}

#[test]
fn puzzle_sprites_expand_schema_tables() {
    let source = r#"
title = sprite_schema

puzzle default {
tags {
kind = A B
}
layers {
__legacy_layer_0 = Target:kind Target:A Target:B
__legacy_layer_1 = Box:kind Box:A Box:B Wall
}
legend a = Target:A
legend b = Target:B
legend A = Box:A
legend B = Box:B
legend # = Wall
legend {
. = empty
}
sprites {
palette {
piece_color:kind {
A = #4a4
B = #a4a
}
}
shapes {
mark:kind {
A {
01
10
}
B {
11
00
}
}
}
sprite {
selector = Box:kind
colors = piece_color:kind transparent
shape = mark:kind
}
sprite {
selector = Wall
colors = #444
shape = {
0
}
}
}
rules {
[ Box:A | ] -> [ | Box:A ]
}
levels {
level "start"
A.
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.visuals.aliases.len(), 3);
    assert!(
        loaded
            .visuals
            .aliases
            .iter()
            .any(|alias| { alias.object == "Box:A" && alias.sprite == "Box-A" })
    );
    let box_b = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box-B")
        .unwrap();
    match &box_b.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(
                pattern.as_slice(),
                ["11".to_string(), "00".to_string()].as_slice()
            );
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '0' && color.color == "#a4a" })
            );
        }
        _ => panic!("Box-B should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_braced_inline_ascii_sprite() {
    let source = r##"
title = braced_inline_ascii_sprite

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = #e94f64 #2f80ed
shape = {
0.
.1
}
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(
                pattern.as_slice(),
                ["0.".to_string(), ".1".to_string()].as_slice()
            );
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '0' && color.color == "#e94f64" })
            );
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '1' && color.color == "#2f80ed" })
            );
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_line_style_solid_sprite() {
    let source = r##"
title = line_style_solid_sprite

puzzle default {
layers {
__legacy_layer_0 = Box
}
legend B = Box
legend {
. = empty
}
sprites {
sprite {
selector = Box
colors = #aaa
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Solid(color) => {
            assert_eq!(color, "#aaa");
        }
        _ => panic!("Box should be a solid sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_display_object_single_color_solid_sprite() {
    let source = r##"
title = display_object_single_color_solid_sprite

puzzle default {
layers {
@display_floor = @Floor
}
legend {
. = empty
}
sprites {
sprite {
selector = @Floor
colors = #eeeeee
}
}
rules {

}
levels {
level "start"
.
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Floor")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Solid(color) => {
            assert_eq!(color, "#eeeeee");
        }
        _ => panic!("@Floor should be a solid sprite"),
    }
}

#[test]
fn puzzle_sprites_can_reference_layers_declared_later() {
    let source = r##"
title = sprites_before_layers

puzzle default {
sprites {
sprite {
selector = @Floor
colors = #eeeeee
}
}
layers {
@display_floor = @Floor
}
legend {
. = empty
}
rules {

}
levels {
level "start"
.
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Floor")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Solid(color) => assert_eq!(color, "#eeeeee"),
        _ => panic!("@Floor should be a solid sprite"),
    }
}

#[test]
fn puzzle_levels_can_reference_layers_declared_later_and_keep_order() {
    let source = r##"
title = levels_before_layers

puzzle default {
levels {
legend {
. = empty
P = Player
B = Box
}
level "first"
P

level "second"
B
}
layers {
actor = Player Box
}
rules {

}
}
"##;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.levels.len(), 2);
    assert_eq!(loaded.levels[0].name, "first");
    assert_eq!(loaded.levels[1].name, "second");
    assert!(loaded.levels[0].initial_state.has_object(
        &loaded.game,
        0,
        0,
        object_named(&loaded, "Player")
    ));
    assert!(loaded.levels[1].initial_state.has_object(
        &loaded.game,
        0,
        0,
        object_named(&loaded, "Box")
    ));
}

#[test]
fn top_level_levels_can_reference_puzzle_declared_later_and_keep_order() {
    let source = r##"
title = top_level_levels_before_puzzle

levels of default {
legend {
. = empty
P = Player
B = Box
}
level "first"
P

level "second"
B
}

puzzle default {
layers {
actor = Player Box
}
rules {

}
}
"##;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.levels.len(), 2);
    assert_eq!(loaded.levels[0].name, "first");
    assert_eq!(loaded.levels[1].name, "second");
    assert!(loaded.levels[0].initial_state.has_object(
        &loaded.game,
        0,
        0,
        object_named(&loaded, "Player")
    ));
    assert!(loaded.levels[1].initial_state.has_object(
        &loaded.game,
        0,
        0,
        object_named(&loaded, "Box")
    ));
}

#[test]
fn puzzle_sprites_accept_display_object_after_another_line_style_sprite() {
    let source = r##"
title = display_object_after_sprite

puzzle default {
layers {
solid = Player
@display_floor = @Floor
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = #ff0000
}
sprite {
selector = @Floor
colors = #eeeeee
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let player = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    let floor = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Floor")
        .unwrap();

    match &player.kind {
        VisualSpriteKind::Solid(color) => assert_eq!(color, "#ff0000"),
        _ => panic!("Player should be a solid sprite"),
    }
    match &floor.kind {
        VisualSpriteKind::Solid(color) => assert_eq!(color, "#eeeeee"),
        _ => panic!("@Floor should be a solid sprite"),
    }
}

#[test]
fn puzzle_sprites_report_unknown_display_selector_instead_of_shape_error() {
    let source = r##"
title = unknown_display_sprite_selector

puzzle default {
layers {
solid = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = #ff0000
}
sprite {
selector = @Floor
colors = #eeeeee
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown visual object selector"), "{error}");
    assert!(
        !error.contains("visual shape rows must be equal-width ascii"),
        "{error}"
    );
}

#[test]
fn puzzle_sprites_accept_sprite_names_that_are_css_color_names() {
    let source = r##"
title = color_named_sprites

puzzle default {
layers {
__legacy_layer_0 = red blue
}
legend r = red
legend b = blue
legend {
. = empty
}
sprites {
sprite {
selector = red
colors = #ff0000
}
sprite {
selector = blue
colors = #0000ff
}
}
rules {

}
levels {
level "start"
rb
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let red = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "red")
        .unwrap();
    let blue = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "blue")
        .unwrap();

    match &red.kind {
        VisualSpriteKind::Solid(color) => assert_eq!(color, "#ff0000"),
        _ => panic!("red should be a solid sprite"),
    }
    match &blue.kind {
        VisualSpriteKind::Solid(color) => assert_eq!(color, "#0000ff"),
        _ => panic!("blue should be a solid sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_line_style_solid_color_table_sprite() {
    let source = r##"
title = line_style_solid_color_table_sprite

puzzle default {
tags {
kind = A B
}
layers {
__legacy_layer_0 = Light:kind
}
legend L = Light:kind
legend {
. = empty
}
sprites {
palette {
piece_color:kind {
A = #4a4
B = #a4a
}
}
sprite {
selector = Light:kind
colors = piece_color:kind
}
}
rules {

}
levels {
level "start"
.
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Light-B")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Solid(color) => {
            assert_eq!(color, "#a4a");
        }
        _ => panic!("Light-B should be a solid sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_line_style_ascii_sprite() {
    let source = r##"
title = line_style_ascii_sprite

puzzle default {
layers {
__legacy_layer_0 = Box Wall
}
legend B = Box
legend W = Wall
legend {
. = empty
}
sprites {
sprite {
selector = Box
colors = #aaa
shape = {
00000
00000
00000
00000
00000
}
}
sprite {
selector = Wall
colors = #444
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let box_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box")
        .unwrap();
    match &box_sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(
                pattern.as_slice(),
                [
                    "00000".to_string(),
                    "00000".to_string(),
                    "00000".to_string(),
                    "00000".to_string(),
                    "00000".to_string(),
                ]
                .as_slice()
            );
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '0' && color.color == "#aaa" })
            );
        }
        _ => panic!("Box should be an ascii sprite"),
    }
    let wall_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Wall")
        .unwrap();
    match &wall_sprite.kind {
        VisualSpriteKind::Solid(color) => assert_eq!(color, "#444"),
        _ => panic!("Wall should be a solid sprite"),
    }
}

#[test]
fn puzzle_sprites_keep_solid_entry_before_line_style_shape_sprite() {
    let source = r##"
title = solid_before_line_style_shape_sprite

puzzle default {
tags {
state = open close
}
layers {
__legacy_layer_0 = Hole Box:state
}
legend H = Hole
legend B = Box:open
legend {
. = empty
}
sprites {
shapes {
Box {
01
10
}
}

sprite {
selector = Hole
colors = #000
}

sprite {
selector = Box:open
colors = #45667d #2f485d
shape = Box
}

sprite {
selector = Box:close
colors = #34444e #262f38
shape = Box
}
}
rules {

}
levels {
level "start"
HB
}
}
"##;
    let loaded = parse_game(source).unwrap();

    let hole_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Hole")
        .unwrap();
    match &hole_sprite.kind {
        VisualSpriteKind::Solid(color) => assert_eq!(color, "#000"),
        _ => panic!("Hole should be a solid sprite"),
    }

    let box_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box-open")
        .unwrap();
    match &box_sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(pattern.as_slice(), ["01".to_string(), "10".to_string()]);
            assert!(
                colors
                    .iter()
                    .any(|color| color.token == '0' && color.color == "#45667d")
            );
            assert!(
                colors
                    .iter()
                    .any(|color| color.token == '1' && color.color == "#2f485d")
            );
        }
        _ => panic!("Box:open should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_warn_when_generated_sprite_key_is_overwritten() {
    let source = r##"
title = duplicate_sprite_visual

puzzle default {
layers {
__legacy_layer_0 = Crack
}
legend C = Crack
legend {
. = empty
}
sprites {
sprite {
selector = Crack
colors = #2cc511
shape = {
.....
..0..
.000.
..0..
.....
}
}

sprite {
selector = Crack
colors = #000
shape = {
0
}
}
}
rules {

}
levels {
level "start"
C
}
}
"##;
    let loaded = parse_game(source).unwrap();

    assert_eq!(
        loaded
            .visuals
            .sprites
            .iter()
            .filter(|sprite| sprite.name == "Crack")
            .count(),
        2
    );
    assert!(loaded.warnings.iter().any(|warning| {
        warning.contains("visual sprite `Crack` is defined more than once")
            && warning.contains("later definition overwrites earlier sprite")
    }));
}

#[test]
fn puzzle_sprites_warn_when_sprite_grid_does_not_divide_largest_grid() {
    let source = r##"
title = sprite_grid_warning

puzzle default {
layers {
__legacy_layer_0 = Box Pull
}
legend B = Box
legend {
. = empty
}
sprites {
sprite {
selector = Box
colors = #aaa
shape = {
0000
0000
0000
0000
}
}

sprite {
selector = Pull
colors = #bbb
shape = {
000
000
000
}
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.warnings.iter().any(|warning| {
        warning.contains("visual sprite `Pull` uses a 3x3 cell grid")
            && warning.contains("does not divide the largest sprite grid 4")
    }));
}

#[test]
fn puzzle_sprites_do_not_warn_when_sprite_grid_divides_largest_grid() {
    let source = r##"
title = sprite_grid_divides

puzzle default {
layers {
__legacy_layer_0 = Box Pull
}
legend B = Box
legend {
. = empty
}
sprites {
sprite {
selector = Box
colors = #aaa
shape = {
0000
0000
0000
0000
}
}

sprite {
selector = Pull
colors = #bbb
shape = {
00
00
}
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();

    assert!(
        !loaded
            .warnings
            .iter()
            .any(|warning| warning.contains("largest sprite grid"))
    );
}

#[test]
fn puzzle_sprites_accept_line_style_tagged_ascii_sprite_after_pattern() {
    let source = r##"
title = line_style_tagged_ascii_sprite

puzzle default {
tags {
state = base movable
}
layers {
__legacy_layer_0 = Box:state
}
legend B = Box:base
legend {
. = empty
}
sprites {
sprite {
selector = Box:base
colors = #aaa
shape = {
0
}
}
sprite {
selector = Box:movable
colors = #bbb
shape = {
0
}
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let box_movable = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box-movable")
        .unwrap();
    match &box_movable.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(pattern.as_slice(), ["0".to_string()].as_slice());
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '0' && color.color == "#bbb" })
            );
        }
        _ => panic!("Box:movable should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_schema_sprite_with_color_alias_row() {
    let source = r##"
title = schema_sprite_color_alias_row

puzzle default {
tags {
num = 1 2
}
layers {
__legacy_layer_0 = Gate:num
}
legend 1 = Gate:1
legend {
. = empty
}
sprites {
palette {
Gate_color_1 = #111111
Gate_color_2 = #222222
}
sprite {
selector = Gate:num
colors = Gate_color_1 Gate_color_2
shape = {
01
10
}
}
}
rules {

}
levels {
level "start"
1
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Gate-1")
        .unwrap();

    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(pattern.as_slice(), ["01".to_string(), "10".to_string()]);
            assert!(
                colors
                    .iter()
                    .any(|color| color.token == '0' && color.color == "#111111")
            );
            assert!(
                colors
                    .iter()
                    .any(|color| color.token == '1' && color.color == "#222222")
            );
        }
        _ => panic!("Gate:1 should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_do_not_parse_tagged_entry_header_as_transform() {
    let source = r##"
title = tagged_sprite_header_not_transform

puzzle default {
tags {
state = base
}
layers {
__legacy_layer_0 = Box:state
}
legend B = Box:base
legend {
. = empty
}
sprites {
Box:base
#aaa
0
Box:movable
#bbb
0
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(
        !error.contains("translate sprite transforms were removed"),
        "{error}"
    );
}

#[test]
fn puzzle_sprites_reject_braces_in_ascii_rows() {
    let source = r##"
title = sprite_ascii_braces

puzzle default {
layers {
__legacy_layer_0 = Box
}
legend B = Box
legend {
. = empty
}
sprites {
Box
#aaa
00{00
00000
00000
00000
00000
}
rules {
}
levels {
level "start"
B
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("ASCII rows cannot contain braces"));
}

#[test]
fn puzzle_sprites_reject_translate_transform_offset() {
    let source = r##"
title = translated_sprite_removed

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = #fff
shape = {
00000
00000
00000
00000
00000
translate:right:2 translate:up:1
}
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(
        error.contains("visual shape rows must be equal-width ascii")
            || error.contains("ASCII rows cannot contain braces")
            || error.contains("sprite ASCII row must be a single token row"),
        "{error}"
    );
}

#[test]
fn puzzle_sprites_reject_malformed_translate_transform() {
    let source = r##"
title = malformed_translated_sprite

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = #fff
shape = {
0
translate:right
}
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(
        error.contains("visual shape rows must be equal-width ascii"),
        "{error}"
    );
}

#[test]
fn puzzle_sprites_accept_line_style_color_and_shape_refs() {
    let source = r##"
title = line_style_color_shape_refs

puzzle default {
layers {
__legacy_layer_0 = Box
}
legend B = Box
legend {
. = empty
}
sprites {
shapes {
box_shape {
010
111
010
}
}
sprite {
selector = Box
colors = #111 #eee
shape = box_shape
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let box_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box")
        .unwrap();
    match &box_sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(
                pattern.as_slice(),
                ["010".to_string(), "111".to_string(), "010".to_string()].as_slice()
            );
            assert_eq!(colors[0].color, "#111");
            assert_eq!(colors[1].color, "#eee");
        }
        _ => panic!("Box should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_bare_shape_reference_after_colors() {
    let source = r##"
title = bare_shape_reference

puzzle default {
layers {
__legacy_layer_0 = Box
}
sprites {
shapes {
box_shape {
01
10
}
}
sprite {
selector = Box
colors = #111 #eee
shape = box_shape
}
}
rules {

}
levels {
legend {
. = empty
B = Box
}
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let box_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box")
        .unwrap();
    match &box_sprite.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(
                pattern.as_slice(),
                ["01".to_string(), "10".to_string()].as_slice()
            );
        }
        _ => panic!("Box should be an ascii sprite"),
    }
}

#[test]
fn unbraced_sprite_attachment_colors_property_does_not_enter_palette() {
    let source = r##"
title = unbraced_sprite_colors_property

puzzle default {
layers {
__legacy_layer_0 = Player
}
sprites {
shapes {
player_shape {
00
11
}
}
Player
colors #fff #000
shape player_shape
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(pattern.as_slice(), ["00".to_string(), "11".to_string()]);
            assert_eq!(colors[0].color, "#fff");
            assert_eq!(colors[1].color, "#000");
            assert!(!colors.iter().any(|color| color.color == "colors"));
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_unbraced_shorthand_animation_body() {
    let source = r##"
title = shorthand_animation_sprite

puzzle default {
layers {
__legacy_layer_0 = Background
}

legend {
. = empty
B = Background
}
sprites {
Background
#90ee90 #008000
500ms
11111
01111
11101
11111
10111
>
10111
11111
01111
11101
11111
>
11111
10111
11111
01111
11101
}
rules {
}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Background")
        .unwrap();
    assert_eq!(
        sprite
            .loop_animation
            .as_ref()
            .map(|animation| animation.duration_ms),
        Some(500)
    );
    assert_eq!(
        sprite
            .loop_animation
            .as_ref()
            .map(|animation| animation.frames.len()),
        Some(3)
    );
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(pattern[0], "11111");
            assert_eq!(colors[0].color, "#90ee90");
            assert_eq!(colors[1].color, "#008000");
        }
        _ => panic!("Background should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_explicit_braced_inline_shape() {
    let source = r##"
title = explicit_braced_inline_shape

puzzle default {
layers {
__legacy_layer_0 = Player
}
sprites {
sprite {
selector = Player
colors = #fff #000
shape = {
000
010
000
}
}
}
rules {
}
levels {
legend {
. = empty
P = Player
}
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(pattern, &["000", "010", "000"]);
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_reject_legacy_unbraced_shape_marker() {
    let source = r##"
title = reject_legacy_unbraced_shape_marker

puzzle default {
layers {
__legacy_layer_0 = Player
}
sprites {
sprite {
selector = Player
colors = #fff #000
shape =
000
010
000
}
}
rules {
}
levels {
legend {
. = empty
P = Player
}
level "start"
P
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(
        error.contains("inline sprite shape must be `shape = { ... }` or bare ASCII rows"),
        "{error}"
    );
}

#[test]
fn puzzle_sprites_accept_frame_duration_for_animation_body() {
    let source = r##"
title = frame_duration_animation_sprite

puzzle default {
layers {
__legacy_layer_0 = Background
}
legend {
. = empty
B = Background
}
sprites {
Background
#90ee90 #008000
frame_duration 100ms
11111
01111
11101
11111
10111
>
10111
11111
01111
11101
11111
>
11111
10111
11111
01111
11101
}
rules {
}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Background")
        .unwrap();
    assert_eq!(
        sprite
            .loop_animation
            .as_ref()
            .map(|animation| animation.duration_ms),
        Some(300)
    );
}

#[test]
fn puzzle_sprites_reject_conflicting_duration_and_frame_duration() {
    let source = r##"
title = conflicting_duration_animation_sprite

puzzle default {
layers {
__legacy_layer_0 = Background
}
legend {
. = empty
B = Background
}
sprites {
Background
#90ee90 #008000
duration 500ms
frame_duration 100ms
11111
01111
11101
11111
10111
>
10111
11111
01111
11101
11111
>
11111
10111
11111
01111
11101
}
rules {
}
levels {
level "start"
B
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("sprite duration must equal frame_duration multiplied by frame count"));
}

#[test]
fn puzzle_sprites_accept_unbraced_shape_table_values_and_bare_refs() {
    let source = r##"
title = unbraced_shape_table_values

puzzle default {
tags {
kind = A B
}
layers {
__legacy_layer_0 = Box:kind
each @Floor
}
legend B = Box:B
legend {
. = empty
}
sprites {
palette {
piece_color:kind {
A = #4a4
B = #a4a
}
}
shapes {
mark:kind {
A
01
10
B
11
00
}
floor
0
}
sprite {
selector = Box:kind
colors = piece_color:kind transparent
shape = mark:kind
}
sprite {
selector = @Floor
colors = #111 #eee
shape = floor
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let box_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box-B")
        .unwrap();
    match &box_sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(
                pattern.as_slice(),
                ["11".to_string(), "00".to_string()].as_slice()
            );
            assert_eq!(colors[0].color, "#a4a");
            assert_eq!(colors[1].color, "transparent");
        }
        _ => panic!("Box:B should be an ascii sprite"),
    }

    let floor_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Floor")
        .unwrap();
    match &floor_sprite.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(pattern.as_slice(), ["0".to_string()].as_slice());
        }
        _ => panic!("@Floor should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_individual_shape_table_values() {
    let source = r##"
title = individual_shape_table_values

puzzle default {
tags {
kind = A B
}
layers {
__legacy_layer_0 = Box:kind
}
legend B = Box:B
legend {
. = empty
}
sprites {
shapes {
mark:A
01
10

mark:B
11
00
}
sprite {
selector = Box:kind
colors = #111 #eee
shape = mark:kind
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let box_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box-B")
        .unwrap();
    match &box_sprite.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(
                pattern.as_slice(),
                ["11".to_string(), "00".to_string()].as_slice()
            );
        }
        _ => panic!("Box:B should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_terminal_unbraced_shape_block_before_colors() {
    let source = r##"
title = terminal_unbraced_shape_block_before_colors

puzzle default {
layers {
__legacy_layer_0 = Box
}
legend B = Box
legend {
. = empty
}
sprites {
shapes {
box_shape
010
111
010
}

palette {
box_color = #eee
}

sprite {
selector = Box
colors = box_color #111
shape = box_shape
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let box_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box")
        .unwrap();
    match &box_sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(
                pattern.as_slice(),
                ["010".to_string(), "111".to_string(), "010".to_string()].as_slice()
            );
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '0' && color.color == "#eee" })
            );
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '1' && color.color == "#111" })
            );
        }
        _ => panic!("Box should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_multiple_unbraced_shapes_in_one_shapes_block() {
    let source = r##"
title = multiple_unbraced_shapes

puzzle default {
layers {
__legacy_layer_0 = Box Pull
}
legend B = Box
legend P = Pull
legend {
. = empty
}
sprites {
shapes {
Box
010
111
010

Pull
000
010
000
}

sprite {
selector = Box
colors = #111 #eee
shape = Box
}

sprite {
selector = Pull
colors = #222 #0f0
shape = Pull
}
}
rules {

}
levels {
level "start"
BP
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let box_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box")
        .unwrap();
    match &box_sprite.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(
                pattern.as_slice(),
                ["010".to_string(), "111".to_string(), "010".to_string()].as_slice()
            );
        }
        _ => panic!("Box should be an ascii sprite"),
    }

    let pull_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Pull")
        .unwrap();
    match &pull_sprite.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(
                pattern.as_slice(),
                ["000".to_string(), "010".to_string(), "000".to_string()].as_slice()
            );
        }
        _ => panic!("Pull should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_do_not_extend_unbraced_shape_by_row_width() {
    let source = r##"
title = unbraced_shape_boundary

puzzle default {
layers {
__legacy_layer_0 = Box Pad
}
legend B = Box
legend P = Pad
legend {
. = empty
}
sprites {
shapes {
Box
010
111
010

Pad
0
}

sprite {
selector = Box
colors = #111 #eee
shape = Box
}

sprite {
selector = Pad
colors = #222
shape = Pad
}
}
rules {

}
levels {
level "start"
BP
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let box_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box")
        .unwrap();
    match &box_sprite.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(
                pattern.as_slice(),
                ["010".to_string(), "111".to_string(), "010".to_string()].as_slice()
            );
        }
        _ => panic!("Box should be an ascii sprite"),
    }

    let pad_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Pad")
        .unwrap();
    match &pad_sprite.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(pattern.as_slice(), ["0".to_string()].as_slice());
        }
        _ => panic!("Pad should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_allow_duplicate_color_refs() {
    let source = r##"
title = duplicate_color_refs

puzzle default {
tags {
kind = A B
}
layers {
__legacy_layer_0 = Box
}
legend B = Box
legend {
. = empty
}
sprites {
palette {
shared = #123456
tagged:kind {
A = #abcdef
B = #fedcba
}
}
shapes {
box_shape {
0123
}
}
sprite {
selector = Box
colors = shared shared tagged:A tagged:A
shape = box_shape
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let box_sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box")
        .unwrap();
    match &box_sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(pattern.as_slice(), ["0123".to_string()].as_slice());
            assert_eq!(colors.len(), 4);
            assert_eq!(colors[0].token, '0');
            assert_eq!(colors[1].token, '1');
            assert_eq!(colors[0].color, "#123456");
            assert_eq!(colors[1].color, "#123456");
            assert_eq!(colors[2].color, "#abcdef");
            assert_eq!(colors[3].color, "#abcdef");
        }
        _ => panic!("Box should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_blank_separated_sprite_attachment() {
    let source = r##"
title = blank_separated_sprite_attachment

puzzle default {
layers {
__legacy_layer_0 = Box
}
legend B = Box
legend {
. = empty
}
sprites {
Box
#123456
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Solid(color) => {
            assert_eq!(color, "#123456");
        }
        _ => panic!("Box should be a solid sprite"),
    }
}

#[test]
fn puzzle_sprites_reject_same_line_sprite_attachment_body() {
    let source = r##"
title = image_sprite_ref

puzzle default {
layers {
__legacy_layer_0 = Box
}
legend B = Box
legend {
. = empty
}
sprites {
Box sprites/box.png
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("sprite entry missing selector"), "{error}");
}

#[test]
fn puzzle_sprites_accept_braced_sprite_attachment_properties() {
    let source = r##"
title = braced_sprite_attachment

puzzle default {
layers {
__legacy_layer_0 = Box
}
legend B = Box
legend {
. = empty
}
sprites {
Box {
image = "sprites/box.png"
translate (0, -1/4)
sampling = smooth
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Image { source } => {
            assert_eq!(source, "sprites/box.png");
        }
        _ => panic!("Box should be an image sprite"),
    }
    assert_eq!(
        sprite.transforms,
        [VisualSpriteTransform::Translate { x: 0.0, y: -0.25 }]
    );
    assert_eq!(sprite.sampling, Some(VisualSpriteSampling::Smooth));
}

#[test]
fn puzzle_sprites_accept_sprite_node_image_properties() {
    let source = r##"
title = sprite_node_image_ref

puzzle default {
layers {
__legacy_layer_0 = Box
}
legend B = Box
legend {
. = empty
}
sprites {
sprite {
selector = Box
image = "sprites/box.png"
translate (0, -1/4)
sampling = smooth
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Box")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Image { source } => {
            assert_eq!(source, "sprites/box.png");
        }
        _ => panic!("Box should be an image sprite"),
    }
    assert_eq!(sprite.fit, VisualSpriteFit::default());
    assert_eq!(
        sprite.transforms,
        [VisualSpriteTransform::Translate { x: 0.0, y: -0.25 }]
    );
    assert_eq!(sprite.sampling, Some(VisualSpriteSampling::Smooth));
}

#[test]
fn puzzle_sprites_reject_removed_offset_property() {
    let source = r##"
title = removed_sprite_offset

puzzle default {
layers {
actor = Box
}
sprites {
Box {
image = "sprites/box.png"
offset 0.5 0
}
}
rules {
}
level "start" {
.
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("sprite offset was replaced by translate (<x>, <y>)"));
}

#[test]
fn puzzle_sprites_reject_gif_image_sprite_refs() {
    let source = r##"
title = image_sprite_ref

puzzle default {
layers {
__legacy_layer_0 = Box
}
legend B = Box
legend {
. = empty
}
sprites {
sprite {
selector = Box
image = "sprites/box.gif"
}
}
rules {

}
levels {
level "start"
B
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("sprite image must use .png, .jpg, .jpeg, or .svg"));
}

#[test]
fn puzzle_sprites_accept_more_than_ten_inline_colors() {
    let source = r##"
title = inline_sprite_many_colors

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = #000000 #111111 #222222 #333333 #444444 #555555 #666666 #777777 #888888 #999999 #aaaaaa
shape = {
a
}
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(pattern.as_slice(), ["a".to_string()].as_slice());
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == 'a' && color.color == "#aaaaaa" })
            );
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_accept_alpha_hex_colors() {
    let source = r##"
title = inline_sprite_alpha_colors

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = #ff004d80 #00000000
shape = {
01
}
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(pattern.as_slice(), ["01".to_string()].as_slice());
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '0' && color.color == "#ff004d80" })
            );
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '1' && color.color == "#00000000" })
            );
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_count_leading_alpha_hex_transparent_as_palette_color() {
    let source = r##"
title = leading_alpha_transparent_palette_color

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = #00000000 #555555
shape = {
01.
}
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(pattern.as_slice(), ["01.".to_string()].as_slice());
            assert_eq!(colors.len(), 2);
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '0' && color.color == "#00000000" })
            );
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '1' && color.color == "#555555" })
            );
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_count_transparent_as_palette_color() {
    let source = r##"
title = transparent_palette_color

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = transparent #555
shape = {
01
}
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(pattern.as_slice(), ["01".to_string()].as_slice());
            assert_eq!(colors.len(), 2);
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '0' && color.color == "transparent" })
            );
            assert!(
                colors
                    .iter()
                    .any(|color| { color.token == '1' && color.color == "#555" })
            );
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_reject_pattern_colors_outside_palette() {
    let source = r##"
title = sprite_palette_overflow

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = transparent
shape = {
01
}
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("sprite pattern references a color outside the color row"));
}

#[test]
fn puzzle_sprites_accept_bare_reusable_shape_ref() {
    let source = r##"
title = bare_reusable_shape_ref

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
shapes {
player_shape {
0.
.1
}
}

sprite {
selector = Player
colors = #e94f64 #2f80ed
shape = player_shape
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(
                pattern.as_slice(),
                ["0.".to_string(), ".1".to_string()].as_slice()
            );
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn puzzle_sprites_reject_old_ascii_sprite_syntax() {
    let source = r##"
title = old_sprite_syntax

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
shapes {
player_shape {
0.
.1
}
}

Player = ascii player_shape {
0 = #e94f64
1 = #2f80ed
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(
        error.contains("sprite ASCII row must be a single token row"),
        "{error}"
    );
}

#[test]
fn puzzle_sprites_reject_legacy_palettes_block() {
    let source = r##"
title = legacy_palettes_block

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
palettes {
player = #e94f64 #2f80ed
}
Player
colors #e94f64 #2f80ed
01
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(
        error.contains("palettes block was renamed to palette"),
        "{error}"
    );
}

#[test]
fn puzzle_sprites_reject_legacy_colors_block_for_palette_defs() {
    let source = r##"
title = legacy_colors_block

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
colors {
player = #e94f64
}
Player
colors #e94f64
0
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(
        error.contains("colors block was renamed to palette; sprite color rows still use colors"),
        "{error}"
    );
}

#[test]
fn directions_is_builtin_value_set_for_objects_sprites_and_for() {
    let source = r#"
title = directions_value_set

puzzle default {
layers {
__legacy_layer_0 = Player
__legacy_layer_1 = Boundary:directions
}
legend P = Player
legend {
. = empty
}
sprites {
shapes {
edge:directions {
up {
11
00
}
down {
00
11
}
left {
10
10
}
right {
01
01
}
}
}
sprite {
selector = Boundary:directions
colors = transparent #555
shape = edge:directions
}
}
rules {
for d in directions {
if input == d {
once d [ Player | ] -> [ | Player ]
}
}
}
levels {
level "start"
.P.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 2, 0, player));
    assert!(
        loaded
            .visuals
            .aliases
            .iter()
            .any(|alias| alias.object == "Boundary:right")
    );
    let boundary_right = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Boundary-right")
        .unwrap();
    match &boundary_right.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(
                pattern.as_slice(),
                ["01".to_string(), "01".to_string()].as_slice()
            );
        }
        _ => panic!("Boundary-right should be an ascii sprite"),
    }
}

#[test]
fn tag_sets_expand_inclusive_numeric_ranges() {
    let source = r#"
title = numeric_tag_range

puzzle default {
tags {
count = 1...10
}
layers {
__legacy_layer_0 = Box:count
}
legend B = Box:10

rules {
}

levels {
legend {
. = empty
}
level "start"
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let tenth = object_named(&loaded, "Box:10");

    assert!(loaded.object_labels.values().any(|label| label == "Box:1"));
    assert!(loaded.object_labels.values().any(|label| label == "Box:10"));
    assert!(!loaded.object_labels.values().any(|label| label == "Box:11"));
    assert!(
        loaded
            .levels
            .first()
            .unwrap()
            .initial_state
            .has_object(&loaded.game, 0, 0, tenth)
    );
}

#[test]
fn sprite_shape_can_generate_direction_variants_by_rotation() {
    let source = r#"
title = rotated_sprites

puzzle default {
map rotate directions {
up -> right
right -> down
down -> left
left -> up
}
layers {
__legacy_layer_0 = Boundary:directions
}
legend {
. = empty
}
sprites {
shapes {
edge:directions rotate from up {
111
000
000
}
}
sprite {
selector = Boundary:directions
colors = transparent #555
shape = edge:directions
}
}
rules {

}
levels {
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let expected = [
        ("Boundary-up", vec!["111", "000", "000"]),
        ("Boundary-right", vec!["001", "001", "001"]),
        ("Boundary-down", vec!["000", "000", "111"]),
        ("Boundary-left", vec!["100", "100", "100"]),
    ];

    for (name, pattern) in expected {
        let sprite = loaded
            .visuals
            .sprites
            .iter()
            .find(|sprite| sprite.name == name)
            .unwrap();
        match &sprite.kind {
            VisualSpriteKind::Ascii {
                pattern: actual, ..
            } => {
                let expected = pattern.into_iter().map(str::to_string).collect::<Vec<_>>();
                assert_eq!(actual.as_slice(), expected.as_slice());
            }
            _ => panic!("{name} should be an ascii sprite"),
        }
    }
}

#[test]
fn unbraced_sprite_entry_can_generate_direction_variants_by_rotation() {
    let source = r#"
title = unbraced_rotated_sprite

puzzle default {
map rotate directions {
up -> right
right -> down
down -> left
left -> up
}
layers {
__legacy_layer_0 = Boundary:directions
}
legend {
. = empty
}
sprites {
shapes {
edge:directions rotate from up {
111
000
000
}
}
sprite {
selector = Boundary:directions
colors = transparent #555
shape = edge:directions
}
}
rules {

}
levels {
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let expected = [
        ("Boundary-up", vec!["111", "000", "000"]),
        ("Boundary-right", vec!["001", "001", "001"]),
        ("Boundary-down", vec!["000", "000", "111"]),
        ("Boundary-left", vec!["100", "100", "100"]),
    ];

    for (name, pattern) in expected {
        let sprite = loaded
            .visuals
            .sprites
            .iter()
            .find(|sprite| sprite.name == name)
            .unwrap();
        match &sprite.kind {
            VisualSpriteKind::Ascii {
                pattern: actual, ..
            } => {
                let expected = pattern.into_iter().map(str::to_string).collect::<Vec<_>>();
                assert_eq!(actual.as_slice(), expected.as_slice());
            }
            _ => panic!("{name} should be an ascii sprite"),
        }
    }
}

#[test]
fn unbraced_display_sprite_entry_can_put_rotation_on_selector_line() {
    let source = r#"
title = unbraced_display_rotated_sprite_header

puzzle default {
layers {
each @WallFrame:directions
}
sprites {
shapes {
wall_frame:directions rotate from up {
0000000
.......
.......
.......
.......
.......
.......
}
}
sprite {
selector = @WallFrame:directions
colors = #585858
shape = wall_frame:directions
}
}
rules {

}
levels {
legend {
. = empty
}
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let expected = [
        (
            "WallFrame-up",
            vec![
                "0000000", ".......", ".......", ".......", ".......", ".......", ".......",
            ],
        ),
        (
            "WallFrame-right",
            vec![
                "......0", "......0", "......0", "......0", "......0", "......0", "......0",
            ],
        ),
        (
            "WallFrame-down",
            vec![
                ".......", ".......", ".......", ".......", ".......", ".......", "0000000",
            ],
        ),
        (
            "WallFrame-left",
            vec![
                "0......", "0......", "0......", "0......", "0......", "0......", "0......",
            ],
        ),
    ];

    for (name, pattern) in expected {
        let sprite = loaded
            .visuals
            .sprites
            .iter()
            .find(|sprite| sprite.name == name)
            .unwrap();
        match &sprite.kind {
            VisualSpriteKind::Ascii {
                pattern: actual, ..
            } => {
                let expected = pattern.into_iter().map(str::to_string).collect::<Vec<_>>();
                assert_eq!(actual.as_slice(), expected.as_slice());
            }
            _ => panic!("{name} should be an ascii sprite"),
        }
    }
}

#[test]
fn consecutive_unbraced_display_sprite_entries_can_use_rotation_and_shape_metadata() {
    let source = r#"
title = unbraced_display_rotated_sprites

puzzle default {
map rotate directions {
up -> right
right -> down
down -> left
left -> up
}
tags {
state = open close
}
layers {
each @Boundary:directions
each @Corner:directions
actor = Goal:state
}
legend {
. = empty
}
sprites {
shapes {
Flag
11
00

boundary:directions rotate from up {
100
000
000
}
corner:directions rotate from up {
010
000
000
}
}
sprite {
selector = @Boundary:directions
colors = #000 #fff
shape = boundary:directions
}
sprite {
selector = @Corner:directions
colors = #111 #fff
shape = corner:directions
}
sprite {
selector = Goal:state
colors = #222 #333
shape = Flag
}
}
rules {

}
levels {
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let sprite_names = loaded
        .visuals
        .sprites
        .iter()
        .map(|sprite| sprite.name.as_str())
        .collect::<Vec<_>>();

    for expected in [
        "Boundary-up",
        "Boundary-right",
        "Corner-up",
        "Corner-right",
        "Goal-open",
        "Goal-close",
    ] {
        assert!(
            sprite_names.contains(&expected),
            "missing sprite {expected}; got {sprite_names:?}"
        );
    }
}

#[test]
fn unbraced_shape_sprite_entry_can_be_followed_by_braced_rotated_display_sprite() {
    let source = r#"
title = shape_before_braced_rotated_display_sprite

puzzle default {
map rotate directions {
up -> right
right -> down
down -> left
left -> up
}
tags {
state = open close
}
layers {
actor = Goal:state
each @LockedFrame:directions
}
legend {
. = empty
}
sprites {
shapes {
Flag
11
00

locked_frame:directions rotate from up {
100
000
000
}
}
sprite {
selector = Goal:state
colors = #222 #333
shape = Flag
}
sprite {
selector = @LockedFrame:directions
colors = #000 #fff
shape = locked_frame:directions
}
}
rules {

}
levels {
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let sprite_names = loaded
        .visuals
        .sprites
        .iter()
        .map(|sprite| sprite.name.as_str())
        .collect::<Vec<_>>();

    for expected in ["Goal-open", "LockedFrame-up", "LockedFrame-right"] {
        assert!(
            sprite_names.contains(&expected),
            "missing sprite {expected}; got {sprite_names:?}"
        );
    }
}

#[test]
fn sprite_shape_rotation_can_use_named_map_directive() {
    let source = r#"
title = rotated_sprites_named_map

puzzle default {
map clockwise directions {
up -> right
right -> down
down -> left
left -> up
}
layers {
__legacy_layer_0 = Boundary:directions
}
legend {
. = empty
}
sprites {
shapes {
edge:directions rotate using clockwise from up {
111
000
000
}
}
sprite {
selector = Boundary:directions
colors = transparent #555
shape = edge:directions
}
}
rules {

}
levels {
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let boundary_right = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Boundary-right")
        .unwrap();

    match &boundary_right.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(
                pattern.as_slice(),
                ["001".to_string(), "001".to_string(), "001".to_string()].as_slice()
            );
        }
        _ => panic!("Boundary-right should be an ascii sprite"),
    }
}

#[test]
fn sprite_entry_accepts_canonical_metadata_colors_and_ascii_order() {
    let source = r##"
title = canonical_sprite_metadata

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
translate (0.5, -1/4)
sampling = smooth
colors = #e94f64 #2f80ed
shape = {
........
..00....
..01....
........
}
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    assert_eq!(
        sprite.transforms,
        [VisualSpriteTransform::Translate { x: 0.5, y: -0.25 }]
    );
    assert_eq!(sprite.fit, VisualSpriteFit::default());
    assert_eq!(sprite.sampling, Some(VisualSpriteSampling::Smooth));
    assert!(sprite.pixels_per_cell.is_none());
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(
                pattern.as_slice(),
                [
                    "........".to_string(),
                    "..00....".to_string(),
                    "..01....".to_string(),
                    "........".to_string(),
                ]
                .as_slice()
            );
            assert_eq!(colors[0].color, "#e94f64");
            assert_eq!(colors[1].color, "#2f80ed");
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn sprite_entry_accepts_canonical_selector_block() {
    let source = r##"
title = canonical_sprite_selector

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = #e94f64 #2f80ed
shape = {
........
..00....
..01....
........
}
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(
                pattern.as_slice(),
                [
                    "........".to_string(),
                    "..00....".to_string(),
                    "..01....".to_string(),
                    "........".to_string(),
                ]
                .as_slice()
            );
            assert_eq!(colors[0].color, "#e94f64");
            assert_eq!(colors[1].color, "#2f80ed");
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn sprite_entry_accepts_canonical_property_shape_block() {
    let source = r##"
title = canonical_sprite_property_shape

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
sprite {
selector = Player
colors = #e94f64 #2f80ed
shape = {
........
..00....
..01....
........
}
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(
                pattern.as_slice(),
                [
                    "........".to_string(),
                    "..00....".to_string(),
                    "..01....".to_string(),
                    "........".to_string(),
                ]
                .as_slice()
            );
            assert_eq!(colors[0].color, "#e94f64");
            assert_eq!(colors[1].color, "#2f80ed");
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn sprite_entry_accepts_explicit_shape_reference_property() {
    let source = r##"
title = canonical_sprite_shape_ref

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
sprites {
shapes {
BoxShape {
00
01
}
}
sprite {
selector = Player
colors = #e94f64 #2f80ed
shape = BoxShape
}
}
rules {

}
levels {
level "start"
P
}
}
"##;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player")
        .unwrap();
    match &sprite.kind {
        VisualSpriteKind::Ascii { pattern, colors } => {
            assert_eq!(
                pattern.as_slice(),
                ["00".to_string(), "01".to_string()].as_slice()
            );
            assert_eq!(colors[0].color, "#e94f64");
            assert_eq!(colors[1].color, "#2f80ed");
        }
        _ => panic!("Player should be an ascii sprite"),
    }
}

#[test]
fn sprite_entry_can_rotate_inline_ascii_from_selector_axis() {
    let source = r#"
title = inline_rotated_sprite

puzzle default {
layers {
__legacy_layer_0 = Boundary:directions
}
legend {
. = empty
}
sprites {
shapes {
edge:directions rotate from up {
111
000
000
}
}
sprite {
selector = Boundary:directions
colors = transparent #555
shape = edge:directions
}
}
rules {

}
levels {
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let expected = [
        ("Boundary-up", vec!["111", "000", "000"]),
        ("Boundary-right", vec!["001", "001", "001"]),
        ("Boundary-down", vec!["000", "000", "111"]),
        ("Boundary-left", vec!["100", "100", "100"]),
    ];

    for (name, pattern) in expected {
        let sprite = loaded
            .visuals
            .sprites
            .iter()
            .find(|sprite| sprite.name == name)
            .unwrap();
        match &sprite.kind {
            VisualSpriteKind::Ascii {
                pattern: actual, ..
            } => {
                let expected = pattern.into_iter().map(str::to_string).collect::<Vec<_>>();
                assert_eq!(actual.as_slice(), expected.as_slice());
            }
            _ => panic!("{name} should be an ascii sprite"),
        }
    }
}

#[test]
fn sprite_rotation_does_not_depend_on_user_map_named_rotate() {
    let source = r#"
title = inline_rotated_sprite_with_unrelated_rotate_map

puzzle default {
tags {
state = open close
}
map rotate state {
open -> close
close -> open
}
layers {
__legacy_layer_0 = Boundary:directions
}
legend {
. = empty
}
sprites {
shapes {
edge:directions rotate from up {
111
000
000
}
}
sprite {
selector = Boundary:directions
colors = transparent #555
shape = edge:directions
}
}
rules {

}
levels {
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let boundary_right = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Boundary-right")
        .unwrap();

    match &boundary_right.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(
                pattern.as_slice(),
                ["001".to_string(), "001".to_string(), "001".to_string()].as_slice()
            );
        }
        _ => panic!("Boundary-right should be an ascii sprite"),
    }
}

#[test]
fn sprite_rotation_ignores_user_map_named_rotate_on_same_axis() {
    let source = r#"
title = inline_rotated_sprite_with_same_axis_rotate_map

puzzle default {
map rotate directions {
up -> left
left -> down
down -> right
right -> up
}
layers {
__legacy_layer_0 = Boundary:directions
}
legend {
. = empty
}
sprites {
shapes {
edge:directions rotate from up {
111
000
000
}
}
sprite {
selector = Boundary:directions
colors = transparent #555
shape = edge:directions
}
}
rules {

}
levels {
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let boundary_right = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Boundary-right")
        .unwrap();

    match &boundary_right.kind {
        VisualSpriteKind::Ascii { pattern, .. } => {
            assert_eq!(
                pattern.as_slice(),
                ["001".to_string(), "001".to_string(), "001".to_string()].as_slice()
            );
        }
        _ => panic!("Boundary-right should be an ascii sprite"),
    }
}

#[test]
fn sprite_ascii_lookup_can_map_selector_axis_values() {
    let source = r#"
title = mapped_sprite_lookup

puzzle default {
map rotate directions {
up -> right
right -> down
down -> left
left -> up
}
layers {
__legacy_layer_0 = Boundary:directions
}
legend {
. = empty
}
sprites {
shapes {
edge:directions rotate from up {
111
000
000
}
}
sprite {
selector = Boundary:directions
colors = transparent #555
shape = edge:rotate(directions)
}
}
rules {

}
levels {
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let expected = [
        ("Boundary-up", vec!["001", "001", "001"]),
        ("Boundary-right", vec!["000", "000", "111"]),
        ("Boundary-down", vec!["100", "100", "100"]),
        ("Boundary-left", vec!["111", "000", "000"]),
    ];

    for (name, pattern) in expected {
        let sprite = loaded
            .visuals
            .sprites
            .iter()
            .find(|sprite| sprite.name == name)
            .unwrap();
        match &sprite.kind {
            VisualSpriteKind::Ascii {
                pattern: actual, ..
            } => {
                let expected = pattern.into_iter().map(str::to_string).collect::<Vec<_>>();
                assert_eq!(actual.as_slice(), expected.as_slice());
            }
            _ => panic!("{name} should be an ascii sprite"),
        }
    }
}

#[test]
fn sprite_visual_selector_can_map_axis_values() {
    let source = r#"
title = mapped_sprite_selector

puzzle default {
map rotate directions {
up -> right
right -> down
down -> left
left -> up
}
layers {
__legacy_layer_0 = Boundary:directions
}
legend {
. = empty
}
sprites {
shapes {
edge:directions rotate from up {
111
000
000
}
}
sprite {
selector = Boundary:rotate(directions)
colors = transparent #555
shape = edge:directions
}
}
rules {

}
levels {
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let expected = [
        ("Boundary-up", vec!["100", "100", "100"]),
        ("Boundary-right", vec!["111", "000", "000"]),
        ("Boundary-down", vec!["001", "001", "001"]),
        ("Boundary-left", vec!["000", "000", "111"]),
    ];

    for (name, pattern) in expected {
        let sprite = loaded
            .visuals
            .sprites
            .iter()
            .find(|sprite| sprite.name == name)
            .unwrap();
        match &sprite.kind {
            VisualSpriteKind::Ascii {
                pattern: actual, ..
            } => {
                let expected = pattern.into_iter().map(str::to_string).collect::<Vec<_>>();
                assert_eq!(actual.as_slice(), expected.as_slice());
            }
            _ => panic!("{name} should be an ascii sprite"),
        }
    }
}

#[test]
fn input_in_directions_scopes_input_oriented_rewrite() {
    let source = r#"
title = input_in_directions

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
rules {
if input in directions {
once input directions [ Player | ] -> [ | Player ]
}
}
levels {
level "start"
.P.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 2, 0, player));
}

#[test]
fn horizontal_orientation_set_expands_rewrite() {
    let source = r#"
title = horizontal_orientation_set

puzzle default {
layers {
__legacy_layer_0 = Player Wall OpenWall
}
legend P = Player
legend # = Wall
legend O = OpenWall
legend {
. = empty
}
rules {

once horizontal [ Player | Wall ] -> [ Player | OpenWall ]
}
levels {
level "start"
.P#.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.game.rules().len(), 3);

    let up = *loaded.controls.keys.get(&b'w').unwrap();
    let moved_up = transition_state(&loaded.game, &loaded.levels[0].initial_state, up).unwrap();
    let open_wall = object_named(&loaded, "OpenWall");

    assert!(moved_up.has_object(&loaded.game, 2, 0, open_wall));
}

#[test]
fn directions_orientation_set_expands_rewrite() {
    let source = r#"
title = directions_orientation_set

puzzle default {
layers {
__legacy_layer_0 = Player Wall OpenWall
}
legend P = Player
legend # = Wall
legend O = OpenWall
legend {
. = empty
}
rules {

once directions [ Player | Wall ] -> [ Player | OpenWall ]
}
levels {
level "start"
.P#.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.game.rules().len(), 5);

    let up = *loaded.controls.keys.get(&b'w').unwrap();
    let moved_up = transition_state(&loaded.game, &loaded.levels[0].initial_state, up).unwrap();
    let open_wall = object_named(&loaded, "OpenWall");

    assert!(moved_up.has_object(&loaded.game, 2, 0, open_wall));
}

#[test]
fn vertical_orientation_set_expands_condition_pattern() {
    let source = r#"
title = vertical_orientation_set_condition

puzzle default {
layers {
__legacy_layer_0 = Player Wall Door OpenDoor
}
legend P = Player
legend # = Wall
legend D = Door
legend O = OpenDoor
legend {
. = empty
}
rules {

if some(vertical [ Player | Wall ]) {
once [ Door ] -> [ OpenDoor ]
}
}
levels {
level "start"
PD
#.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let open_door = object_named(&loaded, "OpenDoor");

    assert!(moved.has_object(&loaded.game, 1, 0, open_door));
}

#[test]
fn input_horizontal_rewrite_adds_input_guard_and_expands_orientation() {
    let source = r#"
title = input_horizontal_rewrite

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
rules {

once input horizontal [ Player | ] -> [ | Player ]
}
levels {
level "start"
...
.P.
...
}
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.game.rules().len(), 3);

    let up = *loaded.controls.keys.get(&b'w').unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved_up = transition_state(&loaded.game, &loaded.levels[0].initial_state, up).unwrap();
    let moved_right =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved_up.has_object(&loaded.game, 1, 1, player));
    assert!(moved_right.has_object(&loaded.game, 2, 1, player));
}

#[test]
fn input_prefix_without_set_is_directions_sugar() {
    let source = r#"
title = input_directions_sugar

puzzle default {
layers {
__legacy_layer_0 = Player
}
legend P = Player
legend {
. = empty
}
rules {
once input [ Player | ] -> [ | Player ]
}
levels {
level "start"
.P.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 2, 0, player));
}

#[test]
fn input_directions_condition_pattern_adds_input_guard_and_expands_orientation() {
    let source = r#"
title = input_directions_condition

puzzle default {
layers {
__legacy_layer_0 = Player Wall Door OpenDoor
}
legend P = Player
legend # = Wall
legend D = Door
legend O = OpenDoor
legend {
. = empty
}
rules {

if some(input directions [ Player | Wall ]) {
once [ Door ] -> [ OpenDoor ]
}
}
levels {
level "start"
P#D
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let up = *loaded.controls.keys.get(&b'w').unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved_up = transition_state(&loaded.game, &loaded.levels[0].initial_state, up).unwrap();
    let moved_right =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let door = object_named(&loaded, "Door");
    let open_door = object_named(&loaded, "OpenDoor");

    assert!(moved_up.has_object(&loaded.game, 2, 0, door));
    assert!(moved_right.has_object(&loaded.game, 2, 0, open_door));
}

#[test]
fn input_condition_pattern_without_set_is_directions_sugar() {
    let source = r#"
title = input_condition_directions_sugar

puzzle default {
layers {
__legacy_layer_0 = Player Wall Door OpenDoor
}
legend P = Player
legend # = Wall
legend D = Door
legend O = OpenDoor
legend {
. = empty
}
rules {
if some(input [ Player | Wall ]) {
once [ Door ] -> [ OpenDoor ]
}
}
levels {
level "start"
P#D
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let up = *loaded.controls.keys.get(&b'w').unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved_up = transition_state(&loaded.game, &loaded.levels[0].initial_state, up).unwrap();
    let moved_right =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let open_door = object_named(&loaded, "OpenDoor");

    assert!(!moved_up.has_object(&loaded.game, 2, 0, open_door));
    assert!(moved_right.has_object(&loaded.game, 2, 0, open_door));
}

#[test]
fn input_condition_can_use_map_call_inside_for_expansion() {
    let source = r#"
title = mapped_input_condition

puzzle default {
map rotate directions {
up -> right
right -> down
down -> left
left -> up
}
layers {
__legacy_layer_0 = Player Marker
}
legend P = Player
legend M = Marker
legend {
. = empty
}
rules {
for d in directions {
if input == rotate(d) {
once d [ Player | ] -> [ | Player ]
}
}
}
levels {
level "start"
...
.P.
...
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 1, 0, player));
    assert!(!moved.has_object(&loaded.game, 1, 1, player));
}

#[test]
fn object_selector_map_call_can_use_tag_set_argument() {
    let source = r#"
title = mapped_tag_set_selector

puzzle default {
tags {
tag = a b c
tags = a b
}
map flip tag {
a -> b
b -> a
c -> c
}
layers {
__legacy_layer_0 = Obj:tag
}
legend a = Obj:a
legend b = Obj:b
legend c = Obj:c

rules {
once [ Obj:tags ] -> [ Obj:flip(tags) ]
}

levels {
legend {
. = empty
}
level "start"
bc
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let obj_a = object_named(&loaded, "Obj:a");
    let obj_c = object_named(&loaded, "Obj:c");

    assert!(next.has_object(&loaded.game, 0, 0, obj_a));
    assert!(next.has_object(&loaded.game, 1, 0, obj_c));
}

#[test]
fn prefixless_spatial_rewrite_expands_to_cardinal_directions() {
    let source = r#"
title = implicit_cardinal_rewrite

puzzle default {
layers {
__legacy_layer_1 = A
}
empty .

legend A = A

rules {
once [ A | ] -> [ | A ]
}

level "start" {
.A.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let mut offsets = loaded
        .game
        .rules()
        .iter()
        .filter_map(|rule| {
            rule.writes.iter().find_map(|write| match write {
                WriteOp::Move {
                    to_offset: Offset::Fixed { dx, dy },
                    ..
                } => Some((*dx, *dy)),
                _ => None,
            })
        })
        .collect::<Vec<_>>();
    offsets.sort();

    assert_eq!(offsets, vec![(-1, 0), (0, -1), (0, 1), (1, 0)]);
}

#[test]
fn rewrite_allows_lhs_and_rhs_pattern_line_breaks() {
    let source = r#"
title = multiline_rewrite

puzzle default {
layers {
__legacy_layer_1 = A B C
}
empty .

legend A = A
legend B = B
legend C = C

rules {
once [ A ]
-> [ B ]
once [ B ] ->
[ C ]
}

level "start" {
A
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object_c = object_named(&loaded, "C");

    assert!(moved.has_object(&loaded.game, 0, 0, object_c));
}

#[test]
fn multiline_rewrite_rejects_rhs_with_nested_arrow() {
    let source = r#"
title = multiline_rewrite_nested_arrow

puzzle default {
layers {
__legacy_layer_1 = A B C
}
empty .

legend A = A

rules {
[ A ] ->
[ B ] -> [ C ]
}

level "start" {
A
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("rewrite continuation rhs cannot contain another ->"));
}

#[test]
fn prefixless_pattern_condition_expands_to_cardinal_directions() {
    let source = r#"
title = implicit_cardinal_condition

puzzle default {
layers {
__legacy_layer_1 = Player Wall Flag
}
empty .

legend P = Player
legend W = Wall
legend F = Flag

routine Mark once {
[ Player ] -> [ Flag ]
}

rules {
[ Player | Wall ] -> Mark
}

level "start" {
WP
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let flag = object_named(&loaded, "Flag");

    assert!(moved.has_object(&loaded.game, 1, 0, flag));
}

#[test]
fn named_query_patterns_expand_to_cardinal_directions() {
    let source = r#"
title = implicit_cardinal_condition

puzzle default {
layers {
__legacy_layer_1 = Player Wall Flag
}
empty .

legend P = Player
legend W = Wall
legend F = Flag

query blocked = exists([ Player | Wall ])

rules {
if blocked {
once [ Player ] -> [ Flag ]
}
}

level "start" {
WP
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let flag = object_named(&loaded, "Flag");

    assert!(moved.has_object(&loaded.game, 1, 0, flag));
}

#[test]
fn condition_declaration_is_rejected() {
    let source = r#"
title = old_condition_declaration

puzzle default {
layers {
__legacy_layer_0 = Player Wall
}
empty .
legend P = Player

condition blocked = exists([ Player | Wall ])

rules {
}

level "start" {
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("`condition` declarations were removed; use `query`"));
}

#[test]
fn if_condition_block_arrow_accepts_mixed_rule_body() {
    let source = r#"
title = if_condition_block_arrow

puzzle default {
layers {
__legacy_layer_1 = Player Wall
__legacy_layer_2 = Flag
}
empty .

legend P = Player
legend F = Flag

rules {
if {
exists(Player)
none(Wall)
} -> {
once [ Player ] -> [ Player Flag ]
checkpoint
}
}

level "start" {
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let flag = object_named(&loaded, "Flag");

    assert!(moved.has_object(&loaded.game, 0, 0, flag));
}

#[test]
fn if_condition_arrow_block_accepts_rule_body() {
    let source = r#"
title = if_condition_arrow_block

puzzle default {
layers {
__legacy_layer_1 = Player
__legacy_layer_2 = Flag
}
empty .

legend P = Player
legend F = Flag

rules {
if exists(Player) -> {
once [ Player ] -> [ Player Flag ]
}
}

level "start" {
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let flag = object_named(&loaded, "Flag");

    assert!(moved.has_object(&loaded.game, 0, 0, flag));
}

#[test]
fn fix_once_sets_default_rewrite_application_for_nested_lines() {
    let source = r#"
title = fix_once

puzzle default {
layers {
__legacy_layer_1 = A
}
empty .

legend A = A

rules {
fix once {
right [ A | ] -> [ | A ]
}
}

level "start" {
A..
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object = object_named(&loaded, "A");

    assert!(moved.has_object(&loaded.game, 1, 0, object));
    assert!(!moved.has_object(&loaded.game, 2, 0, object));
    assert_eq!(loaded.game.rules()[0].application, RuleApplication::Once);
}

#[test]
fn explicit_rewrite_prefix_overrides_fix_default() {
    let source = r#"
title = fix_explicit_override

puzzle default {
layers {
__legacy_layer_1 = A
}
empty .

legend A = A

rules {
fix once {
repeat right [ A | ] -> [ | A ]
}
}

level "start" {
A..
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object = object_named(&loaded, "A");

    assert!(moved.has_object(&loaded.game, 2, 0, object));
    assert_eq!(
        loaded.game.rules()[0].application,
        RuleApplication::UntilStable
    );
}

#[test]
fn once_all_rewrite_applies_to_all_current_matches() {
    let source = r#"
title = once_all_rewrite

puzzle default {
layers {
__legacy_layer_1 = A B
}
empty .

legend A = A
legend B = B

rules {
once_all [ A ] -> [ B ]
}

level "start" {
AAA
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object_b = object_named(&loaded, "B");

    assert!(moved.has_object(&loaded.game, 0, 0, object_b));
    assert!(moved.has_object(&loaded.game, 1, 0, object_b));
    assert!(moved.has_object(&loaded.game, 2, 0, object_b));
    assert_eq!(loaded.game.rules()[0].application, RuleApplication::OnceAll);
}

#[test]
fn random_rewrite_applies_to_one_current_match() {
    let source = r#"
title = random_rewrite

puzzle default {
layers {
actor = A B
}
empty .

legend A = A
legend B = B

rules {
random [ A ] -> [ B ]
}

level "start" {
AAA
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object_a = object_named(&loaded, "A");
    let object_b = object_named(&loaded, "B");

    assert_eq!(moved.object_count(object_b), 1);
    assert_eq!(moved.object_count(object_a), 2);
    assert_eq!(loaded.game.rules()[0].application, RuleApplication::Random);
}

#[test]
fn random_block_applies_one_firing_statement() {
    let source = r#"
title = random_block

puzzle default {
layers {
actor = A B
}
empty .

legend A = A
legend B = B

rules {
random {
[ A | A ] -> [ B | A ]
[ A | A ] -> [ A | B ]
}
}

level "start" {
AA
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object_a = object_named(&loaded, "A");
    let object_b = object_named(&loaded, "B");

    assert_eq!(moved.object_count(object_b), 1);
    assert_eq!(moved.object_count(object_a), 1);
}

#[test]
fn random_routine_applies_one_firing_statement() {
    let source = r#"
title = random_routine

puzzle default {
layers {
actor = A B
}
empty .

legend A = A
legend B = B

routine choose random {
[ A | A ] -> [ B | A ]
[ A | A ] -> [ A | B ]
}

rules {
choose
}

level "start" {
AA
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object_a = object_named(&loaded, "A");
    let object_b = object_named(&loaded, "B");

    assert_eq!(moved.object_count(object_b), 1);
    assert_eq!(moved.object_count(object_a), 1);
}

#[test]
fn once_per_level_rewrite_fires_only_once_for_current_level_state() {
    let source = r#"
title = once_per_level_rewrite

puzzle default {
layers {
__legacy_layer_1 = A
}
empty .

var count = 0

legend A = A

rules {
once_per_level [ A ] -> [ A ] count += 1
}

level "start" {
A
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let first =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let second = transition_state(&loaded.game, &first, InputId(0)).unwrap();

    assert_eq!(first.visible_variables(), &[1]);
    assert_eq!(second.visible_variables(), &[1]);
    assert_eq!(
        loaded.game.rules()[0].application,
        RuleApplication::OncePerLevel
    );
}

#[test]
fn routine_default_application_runs_effect_statement_once() {
    let source = r#"
title = routine_default_once

puzzle default {
layers {
__legacy_layer_1 = A
}
empty .

var count = 0

legend A = A

routine bump {
count += 1
}

rules {
bump
}

level "start" {
A
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert_eq!(next.visible_variables(), &[1]);
}

#[test]
fn routine_default_application_runs_statement_list_once() {
    let source = r#"
title = routine_default_statement_list_once

puzzle default {
layers {
__legacy_layer_1 = A B C
}
empty .

legend A = A
legend B = B
legend C = C

routine advance {
once [ B ] -> [ C ]
once [ A ] -> [ B ]
}

rules {
advance
}

level "start" {
A
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object_b = object_named(&loaded, "B");
    let object_c = object_named(&loaded, "C");

    assert!(next.has_object(&loaded.game, 0, 0, object_b));
    assert!(!next.has_object(&loaded.game, 0, 0, object_c));
}

#[test]
fn explicit_routine_repeat_runs_block_until_stable() {
    let source = r#"
title = explicit_routine_repeat

puzzle default {
layers {
__legacy_layer_1 = A B C
}
empty .

legend A = A
legend B = B
legend C = C

routine spread repeat {
once right [ A | B ] -> [ A | A ]
once right [ A | C ] -> [ A | B ]
}

rules {
spread
}

level "start" {
AC
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object_a = object_named(&loaded, "A");

    assert!(next.has_object(&loaded.game, 1, 0, object_a));
}

#[test]
fn rewrite_suffix_calls_routine_after_rewrite_statement_triggers() {
    let source = r#"
title = rewrite_suffix_after_call

puzzle default {
layers {
__legacy_layer_0 = A B C D
}
empty .

legend A = A
legend B = B
legend C = C
legend D = D

routine feedback once {
[ C ] -> [ D ]
}

rules {
[ A ] -> [ B ] feedback
}

level "start" {
AC
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object_b = object_named(&loaded, "B");
    let object_d = object_named(&loaded, "D");

    assert!(next.has_object(&loaded.game, 0, 0, object_b));
    assert!(next.has_object(&loaded.game, 1, 0, object_d));
}

#[test]
fn rewrite_suffix_after_call_uses_lhs_match_not_rhs_change() {
    let source = r#"
title = rewrite_suffix_after_lhs_match

puzzle default {
layers {
__legacy_layer_0 = A
}
empty .

var count = 0

legend A = A

routine feedback once {
count += 1
}

rules {
[ A ] -> [ A ] feedback
}

level "start" {
A
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert_eq!(next.visible_variables(), &[1]);
}

#[test]
fn directional_condition_call_binds_move_mark_and_offset_to_same_direction() {
    let source = r#"
title = directional_condition_call

puzzle main {
layers {
Background
Player Wall
Marker
}

groups {
player = Player
object = player Wall
}

legend P = Player
legend b = Background
legend # = Wall

routine CancelTurn once {
[ Player ] -> [ Player Marker ]
}

routine move {
repeat {
for d in directions {
d [ d object | no object no {__move_collision} ] -> [ | object{no directions} ]
}
for d in directions {
once_all d [ d object ] -> [ object ]
}
once_all [ {__move_collision} ] -> [ ]
}
}

rules {
input directions [ player ] -> [ > player ]
[ > object | Wall ] -> CancelTurn
move
}

levels {
legend {
_ = empty
}

level "start" {
b_
P#
__
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let up = input_named(&loaded, "up");
    let right = input_named(&loaded, "right");
    let player = object_named(&loaded, "Player");
    let marker = object_named(&loaded, "Marker");

    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, up).unwrap();

    assert!(next.has_object(&loaded.game, 0, 0, player));
    assert_eq!(next.object_count(marker), 0);

    let blocked = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();

    assert_eq!(blocked.object_count(marker), 1);
}

#[test]
fn pattern_condition_preserves_embedded_move_mark_as_mark() {
    let source = r#"
title = directional_pattern_condition

puzzle main {
layers {
Background
Player Wall
Marker
}

groups {
player = Player
object = player Wall
}

legend P = Player
legend b = Background
legend # = Wall

routine move {
repeat {
for d in directions {
d [ d object | no object no {__move_collision} ] -> [ | object{no directions} ]
}
for d in directions {
once_all d [ d object ] -> [ object ]
}
once_all [ {__move_collision} ] -> [ ]
}
}

rules {
input directions [ player ] -> [ > player ]
if some([ > object | Wall ]) {
[ Player ] -> [ Player Marker ]
}
move
}

levels {
legend {
_ = empty
}

level "start" {
b_
P#
__
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let up = input_named(&loaded, "up");
    let right = input_named(&loaded, "right");
    let player = object_named(&loaded, "Player");
    let marker = object_named(&loaded, "Marker");

    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, up).unwrap();

    assert!(next.has_object(&loaded.game, 0, 0, player));
    assert_eq!(next.object_count(marker), 0);

    let blocked = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();

    assert_eq!(blocked.object_count(marker), 1);
}

#[test]
fn embedded_move_mark_condition_uses_relative_direction() {
    let source = r#"
title = relative_embedded_move_mark_condition

puzzle main {
layers {
Background
Player Wall
Marker
}

groups {
player = Player
object = player Wall
}

legend P = Player
legend b = Background
legend # = Wall

routine move {
repeat {
for d in directions {
d [ d object | no object no {__move_collision} ] -> [ | object{no directions} ]
}
for d in directions {
once_all d [ d object ] -> [ object ]
}
once_all [ {__move_collision} ] -> [ ]
}
}

rules {
input directions [ player ] -> [ > player ]
if some([ > object | Wall ]) {
[ Player ] -> [ Player Marker ]
}
move
}

levels {
legend {
_ = empty
}

level "start" {
#_
P_
__
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let up = input_named(&loaded, "up");
    let right = input_named(&loaded, "right");
    let marker = object_named(&loaded, "Marker");

    let blocked_up = transition_state(&loaded.game, &loaded.levels[0].initial_state, up).unwrap();
    let moved_right =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();

    assert_eq!(blocked_up.object_count(marker), 1);
    assert_eq!(moved_right.object_count(marker), 0);
}

#[test]
fn rhs_keep_marker_preserves_matching_cell() {
    let source = r#"
title = rhs_keep_marker

puzzle default {
layers {
__legacy_layer_1 = A B C
}
empty .

legend A = A
legend B = B
legend C = C

rules {
once [ A | B ] -> [ = | C ]
}

level "start" {
AB
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object_a = object_named(&loaded, "A");
    let object_b = object_named(&loaded, "B");
    let object_c = object_named(&loaded, "C");

    assert!(next.has_object(&loaded.game, 0, 0, object_a));
    assert!(!next.has_object(&loaded.game, 1, 0, object_b));
    assert!(next.has_object(&loaded.game, 1, 0, object_c));
}

#[test]
fn keep_marker_is_only_valid_as_whole_rhs_cell() {
    let lhs_source = r#"
title = rhs_keep_marker_lhs_reject

puzzle default {
layers {
__legacy_layer_1 = A B
}
empty .

rules {
[ = | B ] -> [ A | B ]
}

level "start" {
AB
}
}
"#;
    let error = parse_game(lhs_source).unwrap_err().to_string();
    assert!(error.contains("`=` is only valid as a RHS cell"));

    let mixed_rhs_source = r#"
title = rhs_keep_marker_mixed_reject

puzzle default {
layers {
__legacy_layer_1 = A B
}
empty .

rules {
[ A ] -> [ = B ]
}

level "start" {
A
}
}
"#;
    let error = parse_game(mixed_rhs_source).unwrap_err().to_string();
    assert!(error.contains("`=` RHS cell cannot contain other tokens"));
}

#[test]
fn fix_default_applies_through_nested_blocks() {
    let source = r#"
title = fix_nested_block

puzzle default {
layers {
__legacy_layer_1 = A
}
empty .

legend A = A

rules {
fix once {
repeat {
right [ A | ] -> [ | A ]
}
}
}

level "start" {
A..
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let object = object_named(&loaded, "A");

    assert!(moved.has_object(&loaded.game, 2, 0, object));
    assert_eq!(loaded.game.rules()[0].application, RuleApplication::Once);
}

#[test]
fn fix_does_not_prefix_top_level_directives() {
    let source = r#"
title = fix_input

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

fix input {
lft a arrow_left
rgt d arrow_right
}

rules {

}

level "start" {
.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown puzzle directive fix"));
}

#[test]
fn scene_keys_define_action_bindings_and_puzzle_controls() {
    let source = r#"
title = scene_keys

puzzle default {
layers {
actor = Player
}
layers {
__legacy_layer_0 = Player actor
}
legend {
. = empty
P = Player
}

rules {
once input directions [ Player | ] -> [ | Player ]
}

level "start" {
P.
}
}

scene playing {
layout {
puzzle board = default
message_visible = false
moves = 0
message = "Push the box"
}
keys {
d ArrowRight -> input right
q -> level_select
Escape -> menu
}
routine level_select {
goto level_select
}
routine menu {
goto level_select
}
}

scene level_select {
layout {
level_menu {
show_index = true
show_solved = true
}
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(
        matches!(loaded.scenes.as_slice(), [playing, level_select, default]
            if playing.name == "playing"
                && level_select.name == "level_select"
                && default.name == "default")
    );
    assert_eq!(loaded.scenes[0].name, "playing");
    assert_eq!(loaded.scenes[0].state.puzzles.len(), 1);
    assert_eq!(loaded.scenes[0].state.puzzles[0].name, "board");
    assert_eq!(loaded.scenes[0].state.variables.len(), 4);
    assert_eq!(loaded.scenes[0].state.variables[0].name, "message_visible");
    assert_eq!(
        loaded.scenes[0].state.variables[0].default,
        SceneValue::Bool(false)
    );
    assert_eq!(
        loaded.scenes[0].state.variables[2].default,
        SceneValue::Text("Push the box".to_string())
    );
    assert!(loaded.scenes[0].state.variables.iter().any(|variable| {
        variable.name == "input"
            && variable.kind == SceneVarKind::Signal
            && variable.default == SceneValue::Symbol("none".to_string())
    }));
    assert_eq!(loaded.scenes[0].key_bindings[0].keys.len(), 2);
    assert_eq!(
        loaded.controls.keys.get(&b'd'),
        loaded.controls.arrows.get(&ArrowKey::Right)
    );
    assert!(loaded.controls.keys.get(&b'q').is_none());

    let SceneComponent::LevelMenu(menu) = &loaded.scenes[1].components[0] else {
        panic!("expected level menu component");
    };
    assert!(menu.show_index);
    assert!(menu.show_cleared);
}

#[test]
fn scene_effects_parse_targeted_goto_level_paths() {
    let source = r#"
title = goto_effects

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
legend P = Player

rules {
once [ Player ] -> [ Player ]
}

level "start" {
P
}
}

scene select {
layout {
puzzle board = default
column {
button board.level.label -> playing.goto board.level.name
button "Block" -> playing.goto board.level.index
}
}
keys {
Enter -> choose
}
routine choose {
playing.goto board.level.name
}
}

scene playing {
layout {
puzzle board = default
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let column = loaded.scenes[0]
        .components
        .iter()
        .find_map(|component| match component {
            SceneComponent::Column(column) => Some(column),
            _ => None,
        })
        .expect("expected column");
    let SceneComponent::Button(inline_button) = &column.children[0] else {
        panic!("expected inline button");
    };
    let SceneExpr::Path(label_path) = &inline_button.label else {
        panic!("expected path label");
    };
    assert_eq!(
        label_path,
        &vec![
            "board".to_string(),
            "level".to_string(),
            "label".to_string()
        ]
    );
    let SceneEffect::GotoLevel { target, level } = &inline_button.effect else {
        panic!("expected targeted goto effect");
    };
    assert_eq!(target, "playing");
    assert!(matches!(level, SceneExpr::Path(_)));

    let SceneComponent::Button(block_button) = &column.children[1] else {
        panic!("expected second button");
    };
    let SceneEffect::GotoLevel { level, .. } = &block_button.effect else {
        panic!("expected second targeted goto effect");
    };
    assert!(matches!(level, SceneExpr::Path(_)));
    assert!(matches!(
        loaded.scenes[0].routines[0].effect,
        SceneEffect::GotoLevel { .. }
    ));
}

#[test]
fn scene_effects_parse_targeted_restart() {
    let source = r#"
title = targeted_restart

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
legend P = Player

rules {
once [ Player ] -> [ Player ]
}

level "start" {
P
}
}

scene playing {
layout {
puzzle board = default
button "Restart Scene" -> playing.restart
button "Restart Board" -> board.restart
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let SceneComponent::Button(scene_button) = &loaded.scenes[0].components[1] else {
        panic!("expected scene restart button");
    };
    assert!(matches!(
        &scene_button.effect,
        SceneEffect::ResetPuzzle { target } if target == "playing"
    ));

    let SceneComponent::Button(board_button) = &loaded.scenes[0].components[2] else {
        panic!("expected board restart button");
    };
    assert!(matches!(
        &board_button.effect,
        SceneEffect::ResetPuzzle { target } if target == "board"
    ));
}

#[test]
fn scene_effects_parse_inline_sequences_by_effect_vocabulary() {
    let source = r#"
title = inline_effect_sequence

sounds {
music music { seed = 123456 }
sfx click { seed = 746670; type = jump }
}

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
legend P = Player

rules {
once [ Player ] -> [ Player ]
}

level "start" {
P
}
}

scene title {
layout {
button "New Game" -> goto playing play_music music
button "Continue" -> sfx click wait 100ms goto playing
}
routine start {
goto playing play_music music
}
}

scene playing {
layout {
puzzle board = default
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let SceneComponent::Button(new_game) = &loaded.scenes[0].components[0] else {
        panic!("expected new game button");
    };
    assert!(matches!(
        &new_game.effect,
        SceneEffect::Sequence { effects }
            if matches!(effects.as_slice(), [
                SceneEffect::Goto { scene, .. },
                SceneEffect::PlayMusic { name },
            ] if scene == "playing" && name == "music")
    ));

    let SceneComponent::Button(continue_button) = &loaded.scenes[0].components[1] else {
        panic!("expected continue button");
    };
    assert!(matches!(
        &continue_button.effect,
        SceneEffect::Sequence { effects }
            if matches!(effects.as_slice(), [
                SceneEffect::PlaySfx { name: sfx },
                SceneEffect::Wait { milliseconds: Some(100) },
                SceneEffect::Goto { scene, .. },
            ] if sfx == "click" && scene == "playing")
    ));

    assert!(matches!(
        &loaded.scenes[0].routines[0].effect,
        SceneEffect::Sequence { effects }
            if matches!(effects.as_slice(), [
                SceneEffect::Goto { scene, .. },
                SceneEffect::PlayMusic { name },
            ] if scene == "playing" && name == "music")
    ));
}

#[test]
fn scene_button_effect_blocks_reject_end_form() {
    let source = r#"
title = old_button_effect_block

scene title {
layout {
button "New Game" ->
goto playing
play_music music
end
}
}

scene playing {
layout {
puzzle board = default
}
}

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
legend P = Player

rules {
once [ Player ] -> [ Player ]
}

level "start" {
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(
        error.contains("effect block must use `{ ... }`"),
        "unexpected error: {error}"
    );
}

#[test]
fn scene_effects_reject_start_level_scene_commands() {
    let source = r#"
title = start_level_scene

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
legend P = Player

rules {
once [ Player ] -> [ Player ]
}

level "first" {
P
}
}

scene title {
	layout {
	button "Play" -> start levels in playing
	}
	keys {
	Enter Space -> start_first
	}
	routine start_first {
	start levels first in playing
	}
	}

scene playing {
layout {
puzzle board = default
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("no longer supported"));
    assert!(error.contains("goto <scene>(<level>)"));
}

#[test]
fn scene_effect_parser_retains_semantic_tokens() {
    let line = "goto playing(\"first\")";
    let parsed = parse_scene_effect_with_semantic_tokens(line, line).unwrap();
    assert!(matches!(
        parsed.surface.effect,
        SceneEffect::Goto { ref scene, ref params }
            if scene == "playing"
                && matches!(params.as_slice(), [SceneEffectParam::Level(SceneExpr::Text(level))] if level == "first")
    ));
    assert!(parsed.semantic_tokens.iter().any(|token| {
        &line[token.start..token.end] == "goto" && token.kind == SemanticKind::Effect
    }));
    assert!(parsed.surface.document.semantic_tokens.iter().any(|token| {
        &line[token.span.start..token.span.end] == "goto"
            && token.kind == SurfaceSemanticKind::Effect
    }));
    assert!(parsed.surface.document.nodes.iter().any(|node| {
        node.kind == SurfaceNodeKind::SceneEffect && &line[node.span.start..node.span.end] == line
    }));
}

#[test]
fn scene_effect_level_call_accepts_quoted_id_and_selectors() {
    let line = "goto playing(\"microban.1\")";
    let parsed = parse_scene_effect_with_semantic_tokens(line, line).unwrap();
    assert!(matches!(
        parsed.surface.effect,
        SceneEffect::Goto { ref scene, ref params }
            if scene == "playing"
                && matches!(params.as_slice(), [SceneEffectParam::Level(SceneExpr::Text(level))] if level == "microban.1")
    ));

    let line = "goto playing(levels[\"first state\"])";
    let parsed = parse_scene_effect_with_semantic_tokens(line, line).unwrap();
    assert!(matches!(
        parsed.surface.effect,
        SceneEffect::Goto { ref scene, ref params }
            if scene == "playing"
                && matches!(params.as_slice(), [SceneEffectParam::Level(SceneExpr::LevelSelector { collection, key, property: None })]
                    if collection == "levels" && matches!(key, SceneLevelKey::Id(level) if level == "first state"))
    ));

    let line = "goto playing(level(\"first state\"))";
    let error = parse_scene_effect_with_semantic_tokens(line, line)
        .unwrap_err()
        .to_string();
    assert!(error.contains("`level(...)` was removed"));
}

#[test]
fn scene_call_surface_splits_nested_arguments_for_multiple_owners() {
    let line = r#"goto playing(selected = levels["a,b"], label = join("x,y", levels[0].title))"#;
    let parsed = parse_scene_effect_with_semantic_tokens(line, line).unwrap();
    assert!(matches!(
        parsed.surface.effect,
        SceneEffect::Goto { ref scene, ref params }
            if scene == "playing"
                && matches!(
                    params.as_slice(),
                    [
                        SceneEffectParam::Named {
                            name: selected,
                            value: SceneExpr::LevelSelector { collection: selected_collection, key: selected_key, property: None }
                        },
                        SceneEffectParam::Named {
                            name: label,
                            value: SceneExpr::Call { name: join_name, args: join_args }
                        },
                    ] if selected == "selected"
                        && selected_collection == "levels"
                        && matches!(selected_key, SceneLevelKey::Id(text) if text == "a,b")
                        && label == "label"
                        && join_name == "join"
                        && matches!(join_args.as_slice(), [
                            SceneExpr::Text(text),
                            SceneExpr::LevelSelector { collection, key, property: Some(property) }
                        ] if text == "x,y"
                            && collection == "levels"
                            && matches!(key, SceneLevelKey::Index(0))
                            && property == "title")
                )
    ));

    let line = r#"apply sync(levels["a,b"],join("x,y",levels[0]))"#;
    let parsed = parse_scene_effect_with_semantic_tokens(line, line).unwrap();
    assert!(matches!(
        parsed.surface.effect,
        SceneEffect::Apply { ref rule, ref args, target: None }
            if rule == "sync"
                && matches!(args.as_slice(), [
                    SceneExpr::LevelSelector { collection: level_collection, key: level_key, property: None },
                    SceneExpr::Call { name: join_name, args: join_args },
                ] if level_collection == "levels"
                    && matches!(level_key, SceneLevelKey::Id(text) if text == "a,b")
                    && join_name == "join"
                    && matches!(join_args.as_slice(), [
                        SceneExpr::Text(text),
                        SceneExpr::LevelSelector { collection, key, property: None }
                    ] if text == "x,y"
                        && collection == "levels"
                        && matches!(key, SceneLevelKey::Index(0))))
    ));
}

#[test]
fn scene_effect_level_call_rejects_legacy_dotted_level_atom() {
    let error = parse_scene_effect("goto playing(microban.1)", "goto playing(microban.1)")
        .unwrap_err()
        .to_string();

    assert!(
        error.contains("expression must be"),
        "unexpected error: {error}"
    );
}

#[test]
fn scene_effect_sequence_parser_retains_semantic_tokens() {
    let line = "goto playing play_music music";
    let parsed = parse_scene_effect_with_semantic_tokens(line, line).unwrap();
    assert!(matches!(
        parsed.surface.effect,
        SceneEffect::Sequence { effects }
            if matches!(effects.as_slice(), [
                SceneEffect::Goto { scene, .. },
                SceneEffect::PlayMusic { name },
            ] if scene == "playing" && name == "music")
    ));
    assert!(parsed.semantic_tokens.iter().any(|token| {
        &line[token.start..token.end] == "goto" && token.kind == SemanticKind::Effect
    }));
    assert!(parsed.semantic_tokens.iter().any(|token| {
        &line[token.start..token.end] == "play_music" && token.kind == SemanticKind::Effect
    }));
    assert!(parsed.surface.document.nodes.iter().any(|node| {
        node.kind == SurfaceNodeKind::SceneEffect
            && &line[node.span.start..node.span.end] == "goto playing"
    }));
    assert!(parsed.surface.document.nodes.iter().any(|node| {
        node.kind == SurfaceNodeKind::SceneEffect
            && &line[node.span.start..node.span.end] == "play_music music"
    }));
}

#[test]
fn scene_navigation_words_parse_by_state_semantics() {
    let goto = parse_scene_effect("goto detail with selected = first", "").unwrap();
    assert!(matches!(
        goto,
        SceneEffect::Goto { ref scene, ref params }
            if scene == "detail"
                && matches!(params.as_slice(), [SceneEffectParam::Named { name, .. }] if name == "selected")
    ));

    let start = parse_scene_effect("start playing(first)", "").unwrap();
    assert!(matches!(
        start,
        SceneEffect::Sequence { effects }
            if matches!(effects.as_slice(), [
                SceneEffect::Reset { scene: reset_scene },
                SceneEffect::Goto { scene: goto_scene, params }
            ] if reset_scene == "playing" && goto_scene == "playing" && params.len() == 1)
    ));

    assert!(matches!(
        parse_scene_effect("back", "back").unwrap(),
        SceneEffect::RoutineCall(name) if name == "back"
    ));

    assert!(matches!(
        parse_scene_effect("close", "close").unwrap(),
        SceneEffect::RoutineCall(name) if name == "close"
    ));

    for old in ["resume detail", "open menu", "enter menu"] {
        assert!(
            parse_scene_effect(old, old).is_err(),
            "{old} should not be accepted as canonical scene navigation"
        );
    }
}

#[test]
fn rewrite_effect_parser_retains_semantic_tokens() {
    let line = "sfx clear";
    let parsed = parse_rewrite_effect_with_semantic_tokens(line, line).unwrap();
    assert!(matches!(
        parsed.surface.effects.as_slice(),
        [EffectAst::PlaySfx { name }] if name == "clear"
    ));
    assert!(parsed.semantic_tokens.iter().any(|token| {
        &line[token.start..token.end] == "sfx" && token.kind == SemanticKind::Effect
    }));
    assert!(parsed.surface.document.semantic_tokens.iter().any(|token| {
        &line[token.span.start..token.span.end] == "sfx"
            && token.kind == SurfaceSemanticKind::Effect
    }));
    assert!(parsed.surface.document.nodes.iter().any(|node| {
        node.kind == SurfaceNodeKind::RewriteEffect && &line[node.span.start..node.span.end] == line
    }));
    assert!(parsed.semantic_tokens.iter().any(|token| {
        &line[token.start..token.end] == "clear" && token.kind == SemanticKind::Asset
    }));
}

#[test]
fn surface_document_collects_parser_owned_effect_nodes() {
    let source = r#"
scene title {
layout {
title = title
button "Play" -> goto playing
}
}

puzzle main {
rules {
[ Player ] -> [ Player ] sfx bump
}
}
"#;
    let surface = parse_surface_document(source);
    let scene_name_start = source.find("scene title").unwrap() + "scene ".len();
    let component_title_start = source.rfind("title = title").unwrap();

    assert!(surface.semantic_tokens.iter().any(|token| {
        &source[token.span.start..token.span.end] == "scene"
            && token.kind == SurfaceSemanticKind::Keyword
    }));
    assert!(surface.semantic_tokens.iter().any(|token| {
        token.span.start == scene_name_start
            && &source[token.span.start..token.span.end] == "title"
            && token.kind == SurfaceSemanticKind::Scene
    }));
    assert!(surface.semantic_tokens.iter().any(|token| {
        token.span.start == component_title_start
            && &source[token.span.start..token.span.end] == "title"
            && token.kind == SurfaceSemanticKind::Keyword
    }));
    assert!(surface.nodes.iter().any(|node| {
        node.kind == SurfaceNodeKind::SceneEffect
            && &source[node.span.start..node.span.end] == "goto playing"
    }));
    assert!(surface.nodes.iter().any(|node| {
        node.kind == SurfaceNodeKind::RewriteEffect
            && &source[node.span.start..node.span.end] == "sfx bump"
    }));
    assert!(surface.semantic_tokens.iter().any(|token| {
        &source[token.span.start..token.span.end] == "goto"
            && token.kind == SurfaceSemanticKind::Effect
    }));
    assert!(surface.semantic_tokens.iter().any(|token| {
        &source[token.span.start..token.span.end] == "bump"
            && token.kind == SurfaceSemanticKind::Asset
    }));
}

#[test]
fn progress_scene_effects_parse() {
    assert!(matches!(
        parse_scene_effect("clear_undo_history", "clear_undo_history").unwrap(),
        SceneEffect::ClearUndoHistory
    ));
    assert!(matches!(
        parse_scene_effect("clear_game_progress", "clear_game_progress").unwrap(),
        SceneEffect::ClearGameProgress
    ));
    assert!(matches!(
        parse_scene_effect("clear current_level", "clear current_level").unwrap(),
        SceneEffect::ClearCurrentLevel
    ));
    assert!(matches!(
        parse_scene_effect("reset persistent_vars", "reset persistent_vars").unwrap(),
        SceneEffect::ResetPersistentVars
    ));
    assert!(matches!(
        parse_scene_effect("current_level = level", "current_level = level").unwrap(),
        SceneEffect::SetCurrentLevel { .. }
    ));
    assert!(matches!(
        parse_scene_effect("level.cleared = true", "level.cleared = true").unwrap(),
        SceneEffect::SetLevelCleared {
            level: None,
            cleared: true
        }
    ));
    assert!(matches!(
        parse_scene_effect(
            "levels[\"microban.2\"].cleared = false",
            "levels[\"microban.2\"].cleared = false"
        )
        .unwrap(),
        SceneEffect::SetLevelCleared {
            level: Some(_),
            cleared: false
        }
    ));
}

#[test]
fn scene_variable_assignment_effect_parses_path_rhs() {
    assert!(matches!(
        parse_scene_effect("num = num_run", "num = num_run").unwrap(),
        SceneEffect::SetVariable { name, value }
            if name == "num" && value == SceneExpr::Path(vec!["num_run".to_string()])
    ));
    assert!(parse_scene_effect("set num = num_run", "set num = num_run").is_err());
}

#[test]
fn var_scopes_parse_by_owner() {
    let source = r#"
title = var_scopes
var session_label = "Session Label"
persistent var high_score = 0

puzzle default {
var moved = false
persistent var cleared = false

layers {
__legacy_layer_0 = Player
}
legend P = Player

legend {
. = empty
P = Player
}

rules {
once [ Player ] -> moved = true
}

level "start" {
P
}
}

scene playing {
var message = "Ready"
persistent var last_tab = levels
layout {
puzzle board = default
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.variables.len(), 2);
    assert_eq!(loaded.variables[0].name, "session_label");
    assert_eq!(
        loaded.variables[0].default,
        SceneValue::Text("Session Label".to_string())
    );
    assert_eq!(loaded.variables[1].lifetime, SceneStateLifetime::Persistent);
    assert_eq!(loaded.variable_labels.len(), 2);
    assert!(loaded.variable_labels.values().any(|name| name == "moved"));
    assert!(
        loaded
            .variable_labels
            .values()
            .any(|name| name == "cleared")
    );
    assert_eq!(loaded.persistent_vars.len(), 1);
    assert_eq!(loaded.scenes[0].state.variables.len(), 2);
    assert_eq!(loaded.scenes[0].state.variables[1].name, "last_tab");
    assert_eq!(
        loaded.scenes[0].state.variables[1].lifetime,
        SceneStateLifetime::Persistent
    );
}

#[test]
fn layers_and_legend_define_rendering_groups_and_empty() {
    let source = r#"
title = object_blocks

puzzle default {
layers {
__legacy_layer_0 = Goal
__legacy_layer_1 = Player Box Wall
}
legend G = Goal
legend P = Player
legend B = Box
legend # = Wall
groups {
solid = Player Box Wall
}

legend {
. = empty
* = Goal Box
+ = Goal Player
}

rules {
once input directions [ Player | Box | no solid ] -> [ | Player | Box ]
once input directions [ Player | no solid ] -> [ | Player ]
}

level "start" {
#P.BG
}
}

scene playing {
layout {
puzzle board = default
}
keys {
d ArrowRight -> input right
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.game.layer_count, 2);
    assert_eq!(loaded.game.object_count(), 4);
    assert_eq!(loaded.legend.char_for_cell(&[]), '.');
    assert_eq!(loaded.levels[0].initial_state.width, 5);
    assert!(loaded.controls.keys.get(&b'd').is_some());
}

#[test]
fn level_body_legend_adds_level_local_chars() {
    let source = r#"
title = level_local_legend

puzzle default {
layers {
__legacy_layer_0 = Goal
__legacy_layer_1 = Box Player
}
legend G = Goal
legend B = Box
legend P = Player

legend {
. = empty
}

rules {

}

levels {
level "local"
legend {
x = Goal Box
}
x

level "plain"
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let goal = object_named(&loaded, "Goal");
    let box_object = object_named(&loaded, "Box");
    let player = object_named(&loaded, "Player");

    assert!(
        loaded.levels[0]
            .initial_state
            .has_object(&loaded.game, 0, 0, goal)
    );
    assert!(
        loaded.levels[0]
            .initial_state
            .has_object(&loaded.game, 0, 0, box_object)
    );
    assert!(
        loaded.levels[1]
            .initial_state
            .has_object(&loaded.game, 0, 0, player)
    );
}

#[test]
fn level_body_legend_does_not_leak_to_other_levels() {
    let source = r#"
title = level_local_legend_no_leak

puzzle default {
layers {
__legacy_layer_0 = Goal
__legacy_layer_1 = Box
}

legend {
. = empty
}

rules {

}

levels {
level "first"
legend x = Goal Box
x

level "second"
x
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown level char 'x'"));
}

#[test]
fn detects_goal_completion_after_solving_sample_game() {
    let source = r#"
title = goal_fixture
puzzle sokoban {
layers {
__legacy_layer_0 = Goal
__legacy_layer_1 = Player Box Wall
}
legend G = Goal
legend P = Player
legend B = Box
legend # = Wall
groups {
solid = Player Box Wall
}
legend {
. = empty
* = Goal Box
}
win_conditions {
some Goal
all Goal on Box
}
rules {
once input directions [ Player | Box | no solid ] -> [ | Player | Box ]
once input directions [ Player | no solid ] -> [ | Player ]
}
levels {
level "first"
#######
#P.B.G#
#######

level "second"
#######
#P.B.G#
#######
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let mut state = loaded.levels[0].initial_state.clone();

    for key in "ddd".bytes() {
        let input = *loaded.controls.keys.get(&key).unwrap();
        state = transition_state(&loaded.game, &state, input).unwrap();
    }

    assert!(loaded.is_goal_complete(&state));
}

#[test]
fn detects_goal_completion_on_second_stage() {
    let source = r#"
title = goal_fixture
puzzle sokoban {
layers {
__legacy_layer_0 = Goal
__legacy_layer_1 = Player Box Wall
}
legend G = Goal
legend P = Player
legend B = Box
legend # = Wall
groups {
solid = Player Box Wall
}
legend {
. = empty
* = Goal Box
}
win_conditions {
some Goal
all Goal on Box
}
rules {
once input directions [ Player | Box | no solid ] -> [ | Player | Box ]
once input directions [ Player | no solid ] -> [ | Player ]
}
levels {
level "first"
#######
#P.B.G#
#######

level "second"
#######
#P.B.G#
#######
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let mut state = loaded.levels[1].initial_state.clone();

    for key in "ddd".bytes() {
        let input = *loaded.controls.keys.get(&key).unwrap();
        state = transition_state(&loaded.game, &state, input).unwrap();
    }

    assert!(loaded.is_goal_complete(&state));
}

#[test]
fn parses_lose_conditions_with_some_pattern_row() {
    let source = r#"
title = lose_fixture
puzzle default {
layers {
__legacy_layer_0 = Box Wall
}
legend B = Box
legend # = Wall
groups {
solid = Box Wall
}
legend {
. = empty
}
lose_conditions {
some [ | Wall ; Box | Wall ]
}
rules {

}
levels {
level "start"
.#
B#
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let state = &loaded.levels[0].initial_state;

    assert!(loaded.lose.is_some());
    assert!(loaded.is_lose_complete(state));
    assert!(!loaded.is_condition_true("lose_conditions", state));
}

#[test]
fn parses_lose_conditions_with_exists_pattern_expr() {
    let source = r#"
title = lose_fixture
puzzle default {
layers {
__legacy_layer_0 = Box Wall
}
legend B = Box
legend # = Wall
groups {
solid = Box Wall
}
legend {
. = empty
}
lose_conditions {
exists([ | Wall ; Box | Wall ])
}
rules {

}
levels {
level "start"
.#
B#
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_lose_complete(&loaded.levels[0].initial_state));
}

#[test]
fn condition_prefix_patterns_accept_wrapping_parentheses() {
    let source = r#"
title = condition_prefix_pattern_parens
puzzle default {
layers {
__legacy_layer_0 = Goal
__legacy_layer_1 = Box
}
legend * = Goal Box
legend {
. = empty
}
win_conditions {
no ([ Box no Goal ])
some ([ Box ] [ Goal ])
}
rules {

}
levels {
level "start"
*
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn condition_patterns_accept_family_wildcard_tag_selector() {
    let source = r#"
title = condition_family_wildcard_tag
puzzle default {
tags {
state = open close
}
layers {
__legacy_layer_0 = Door:state Switch:state
}
legend d = Door:open
legend {
. = empty
}
win_conditions {
some [ *:open ]
}
rules {

}
levels {
level "start"
d
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn no_function_alias_accepts_pattern_conditions() {
    let source = r#"
title = no_function_pattern_alias
puzzle default {
layers {
__legacy_layer_0 = Goal
}
legend {
. = empty
}
win_conditions = no([ Goal ])
rules {

}
levels {
level "start"
.
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn condition_blocks_accept_explicit_any_combinator() {
    let source = r#"
title = condition_any_fixture
puzzle default {
layers {
__legacy_layer_0 = Goal
__legacy_layer_1 = Box Wall
}
legend G = Goal
legend B = Box
legend # = Wall
groups {
solid = Box Wall
}
legend {
. = empty
* = Goal Box
}
lose_conditions any {
count(Box) == 0
all Box on Goal
}
rules {

}
levels {
level "start"
*#
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let lose = loaded.lose.as_ref().unwrap();

    assert_eq!(lose.description, "count(Box) == 0 or all Box on Goal");
    assert!(loaded.is_lose_complete(&loaded.levels[0].initial_state));
}

#[test]
fn condition_blocks_expand_for_value_sets() {
    let source = r#"
title = condition_for_fixture
puzzle default {
tags {
kind = A B
}
layers {
__legacy_layer_0 = Goal:kind
__legacy_layer_1 = Box:kind
}
legend {
. = empty
A = Goal:A Box:A
B = Goal:B Box:B
}
win_conditions all {
for k in kind {
all Goal:k on Box:k
}
}
rules {

}
levels {
level "start"
AB
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let goal = loaded.goal.as_ref().unwrap();

    assert_eq!(
        goal.description,
        "all Goal:A on Box:A and all Goal:B on Box:B"
    );
    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn condition_blocks_expand_nested_for_value_sets() {
    let source = r#"
title = nested_condition_for_fixture
puzzle default {
tags {
kind = A B
}
tags {
direction_side = up down
}
layers {
__legacy_layer_0 = Box:kind
__legacy_layer_1 = Edge:direction_side
__legacy_layer_2 = Found:kind:direction_side
}
legend {
. = empty
A = Box:A Edge:up Found:A:up
}
lose_conditions any {
for k in kind {
for d in direction_side {
exists([ Box:k Edge:d no Found:k:d ])
}
}
}
rules {

}
levels {
level "start"
A
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let lose = loaded.lose.as_ref().unwrap();

    assert!(lose.description.contains("Box:A Edge:down"));
    assert!(!loaded.is_lose_complete(&loaded.levels[0].initial_state));
}

#[test]
fn rules_expand_for_in_inclusive_numeric_range() {
    let source = r#"
title = numeric_for_range

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

marks {
count = int
}

legend B = Box

rules {
for i in 1...3 {
once [ Box ] -> [ Box{count=i} ]
}
once [ Box{count=3} no Marker ] -> [ Box Marker ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn rules_expand_for_in_numeric_range_with_integer_var_endpoint() {
    let source = r#"
title = numeric_for_var_range

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box
}
empty .

var L = 3

marks {
count = int
}

legend B = Box

rules {
for i in 1...L {
once [ Box ] -> [ Box{count=i} ]
}
once [ Box{count=3} no Marker ] -> [ Box Marker ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn rules_expand_for_inline_value_list_as_object_tokens() {
    let source = r#"
title = inline_for_objects

puzzle default {
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box Wall Player
}
empty .

legend W = Wall

rules {
for object in Box Wall Player {
once [ object no Marker ] -> [ object Marker ]
}
}

levels {
legend {
. = empty
}
level "start"
W
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn rules_expand_for_inline_value_list_as_tag_tokens() {
    let source = r#"
title = inline_for_tags

puzzle default {
tags {
kind = tag_1 tag_2 tag_3 tag_4
}
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box:kind
}
empty .

legend B = Box:tag_3

rules {
for tag in tag_1 tag_2 tag_3 tag_4 {
once [ Box:tag no Marker ] -> [ Box:tag Marker ]
}
}

levels {
legend {
. = empty
}
level "start"
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
}

#[test]
fn for_statement_body_uses_balanced_brace_depth() {
    let source = r#"
title = for_if_else_checked_mark

puzzle default {
tags {
gate_no = 1...5
}
marks {
checked
}
var locked_room_count = 1
layers {
gate = Gate:gate_no
}
empty .

levels {
legend 1 = Gate:1

level "start" {
1
}
}

rules {
for n in 1...5 {
if some([ Gate:n{checked} ]) {
if locked_room_count > n {
locked_room_count -= n
[ Gate:n{checked} ] -> [  ]
} else {
[ Gate:n{checked} ] -> [ Gate:n ]
}
}
}
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let _ = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
}

#[test]
fn inline_if_condition_accepts_inclusive_numeric_comparisons() {
    let source = r#"
title = inclusive_compare_if

puzzle default {
var count = 2
layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Flag
__legacy_layer_2 = Box
}
empty .
legend B = Box

rules {
if count >= 2 {
[ Box no Marker ] -> [ Box Marker ]
}
if count <= 2 {
[ Box no Flag ] -> [ Box Flag ]
}
}

levels {
level "start"
B
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");
    let flag = object_named(&loaded, "Flag");

    assert!(next.has_object(&loaded.game, 0, 0, marker));
    assert!(next.has_object(&loaded.game, 0, 0, flag));
}

#[test]
fn routine_for_condition_accepts_inclusive_loop_binding_comparison() {
    let source = r#"
title = routine_inclusive_loop_compare

sounds {
sfx bump { seed = 746670; type = jump }
}

puzzle default {
tags {
gate_no = 1...5
}
marks {
checked
}
var locked_room_count = 1
layers {
gate = Gate:gate_no
}
empty .

routine open_gate {
for n in 1...5 {
if some([ Gate:n{checked}]) {
sfx bump
if locked_room_count >= n {
locked_room_count -= n
[ Gate:n{checked} ] -> [  ]
} else {
[ Gate:n{checked} ] -> [ Gate:n ]
}
}
}
}

rules {
once [ Gate:1 ] -> [ Gate:1{checked} ]
open_gate
}

levels {
legend 1 = Gate:1

level "start" {
1
}
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let gate = object_named(&loaded, "Gate:1");

    assert_eq!(next.visible_variables(), &[0]);
    assert!(!next.has_object(&loaded.game, 0, 0, gate));
}

#[test]
fn routine_if_without_else_uses_one_condition_snapshot_for_all_then_statements() {
    let source = r#"
title = routine_if_without_else_condition_snapshot

puzzle default {
tags {
gate_no = 1...5
count_value = 0 1
}
marks {
checked
}
var locked_room_count = 1
layers {
gate = Gate:gate_no
@count = @Count:count_value
}
empty .

routine open_gate {
for n in 1...5 {
if some([ Gate:n{checked} ]) {
if locked_room_count >= n {
locked_room_count -= n
[ @Count:* ] -> [ @Count:locked_room_count ]
[ Gate:n{checked} ] -> [  ]
}
[ Gate:n{checked} ] -> [ Gate:n ]
}
}
}

rules {
once [ Gate:1 ] -> [ Gate:1{checked} ]
open_gate
}

levels {
legend 1 = Gate:1
legend c = @Count:1

level "start" {
1
c
}
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let gate = object_named(&loaded, "Gate:1");
    let count_zero = object_named(&loaded, "@Count:0");

    assert_eq!(next.visible_variables(), &[0]);
    assert!(!next.has_object(&loaded.game, 0, 0, gate));
    assert!(next.has_object(&loaded.game, 0, 1, count_zero));
}

#[test]
fn routine_if_without_else_uses_updated_variable_for_later_dynamic_display_write() {
    let source = r#"
title = routine_if_without_else_dynamic_display_update

puzzle default {
tags {
gate_no = 1...5
count_value = 0 1 2
}
marks {
checked
}
var locked_room_count = 2
layers {
gate = Gate:gate_no
@count = @Count:count_value
}
empty .

routine open_gate {
for n in 1...5 {
if some([ Gate:n{checked} ]) {
if locked_room_count >= n {
locked_room_count -= n
[ @Count:* ] -> [ @Count:locked_room_count ]
[ Gate:n{checked} ] -> [  ]
}
[ Gate:n{checked} ] -> [ Gate:n ]
}
}
}

rules {
once [ Gate:1 ] -> [ Gate:1{checked} ]
open_gate
}

levels {
legend 1 = Gate:1
legend c = @Count:2

level "start" {
1
c
}
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let gate = object_named(&loaded, "Gate:1");
    let count_one = object_named(&loaded, "@Count:1");

    assert_eq!(next.visible_variables(), &[1]);
    assert!(!next.has_object(&loaded.game, 0, 0, gate));
    assert!(next.has_object(&loaded.game, 0, 1, count_one));
}

#[test]
fn routine_for_condition_runs_inclusive_loop_binding_else_branch() {
    let source = r#"
title = routine_inclusive_loop_compare_else

sounds {
sfx bump { seed = 746670; type = jump }
}

puzzle default {
tags {
gate_no = 1...5
}
marks {
checked
}
var locked_room_count = 0
layers {
gate = Gate:gate_no
}
empty .

routine open_gate {
for n in 1...5 {
if some([ Gate:n{checked} ]) {
sfx bump
if locked_room_count >= n {
locked_room_count -= n
[ Gate:n{checked} ] -> [  ]
} else {
[ Gate:n{checked} ] -> [ Gate:n ]
}
}
}
}

rules {
once [ Gate:1 ] -> [ Gate:1{checked} ]
open_gate
}

levels {
legend 1 = Gate:1

level "start" {
1
}
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let gate = object_named(&loaded, "Gate:1");

    assert_eq!(next.visible_variables(), &[0]);
    assert!(next.has_object(&loaded.game, 0, 0, gate));
    assert!(next.slot_mark().iter().all(Vec::is_empty));
}

#[test]
fn schema_selector_can_read_current_integer_var_tag_value() {
    let source = r#"
title = dynamic_selector_var

puzzle default {
var count = 2

tags {
num = 1 2 3
}

layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box:num
}
empty .

legend {
. = empty
B = Box:2
}

rules {
once [ Box:count no Marker ] -> [ Box:count Marker ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(loaded.warnings.iter().any(|warning| {
        warning.contains("dynamic selector `Box:count` uses mutable var `count`")
    }));
}

#[test]
fn schema_tag_slot_capture_updates_var_from_matched_variant() {
    let source = r#"
title = schema_tag_slot_capture

puzzle default {
var captured = 0

tags {
kind = 1 2 3
}

layers {
floor = Detector
actor = Obj:kind
}
empty .

legend {
. = empty
X = Detector Obj:2
}

rules {
once [ Obj:kind Detector ] -> captured = kind
}

level "start" {
X
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert_eq!(next.visible_variables(), &[2]);
}

#[test]
fn schema_tag_slot_labeled_capture_updates_var_from_matched_variant() {
    let source = r#"
title = schema_tag_slot_labeled_capture

puzzle default {
var captured = 0

tags {
kind = 1 2 3
}

layers {
actor = Obj:kind
}
empty .

legend {
. = empty
X = Obj:3
}

rules {
once [ Obj:kind#1 ] -> captured = kind#1
}

level "start" {
X
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert_eq!(next.visible_variables(), &[3]);
}

#[test]
fn schema_tag_slot_labeled_capture_can_feed_map_call_selector() {
    let source = r#"
title = schema_tag_capture_map_call

puzzle default {
tags {
kind = 1 2
}
map flip kind {
1 -> 2
2 -> 1
}
layers {
actor = Obj:kind
}
empty .

legend {
. = empty
X = Obj:2
}

rules {
once [ Obj:kind#1 ] -> [ Obj:flip(kind#1) ]
}

level "start" {
X
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let obj_one = object_named(&loaded, "Obj:1");

    assert!(next.has_object(&loaded.game, 0, 0, obj_one));
}

#[test]
fn schema_wildcard_capture_updates_var_when_single_tag_slot_is_unambiguous() {
    let source = r#"
title = schema_wildcard_capture

puzzle default {
var captured = 0

tags {
kind = 1 2 3
}

layers {
actor = Obj:kind
}
empty .

legend {
. = empty
X = Obj:1
}

rules {
once [ Obj:* ] -> captured = *
}

level "start" {
X
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert_eq!(next.visible_variables(), &[1]);
}

#[test]
fn schema_wildcard_labeled_capture_updates_var() {
    let source = r#"
title = schema_wildcard_labeled_capture

puzzle default {
var captured = 0

tags {
kind = 1 2 3
}

layers {
actor = Obj:kind
}
empty .

legend {
. = empty
X = Obj:2
}

rules {
once [ Obj:*#1 ] -> captured = *#1
}

level "start" {
X
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert_eq!(next.visible_variables(), &[2]);
}

#[test]
fn schema_tag_capture_reference_is_rejected_when_ambiguous() {
    let source = r#"
title = schema_tag_capture_ambiguous

puzzle default {
var captured = 0

tags {
kind = 1 2
}

layers {
a = A:kind
b = B:kind
}
empty .

legend {
. = empty
1 = A:1
2 = B:2
}

rules {
once [ A:kind | B:kind ] -> captured = kind
}

level "start" {
12
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("tag capture reference `kind` is ambiguous"));
}

#[test]
fn schema_tag_capture_reference_requires_matching_lhs_binding() {
    let source = r#"
title = schema_tag_capture_missing

puzzle default {
var captured = 0

tags {
kind = 1 2
}

layers {
actor = Obj:kind
}
empty .

legend {
. = empty
X = Obj:1
}

rules {
once [ Obj:kind ] -> captured = kind#1
}

level "start" {
X
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown tag capture reference: kind#1"));
}

#[test]
fn schema_tag_capture_var_update_requires_numeric_tag_value() {
    let source = r#"
title = schema_tag_capture_non_numeric

puzzle default {
var captured = 0

tags {
color = red blue
}

layers {
actor = Obj:color
}
empty .

legend {
. = empty
R = Obj:red
}

rules {
once [ Obj:color ] -> captured = color
}

level "start" {
R
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("tag capture values used in var updates"));
}

#[test]
fn schema_selector_tracks_var_value_on_later_turns() {
    let source = r#"
title = dynamic_selector_updates

puzzle default {
var count = 2

tags {
num = 1 2 3
}

layers {
__legacy_layer_0 = Flag
__legacy_layer_1 = Box:num
}
empty .

legend {
. = empty
B = Box:2
C = Box:3
}

rules {
if input == right {
count = 3
}
if input == up {
once [ Box:count no Flag ] -> [ Box:count Flag ]
}
}

level "start" {
BC
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let flag = object_named(&loaded, "Flag");
    let right = input_named(&loaded, "right");
    let up = input_named(&loaded, "up");
    let first = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let second = transition_state(&loaded.game, &first, up).unwrap();

    assert!(!first.has_object(&loaded.game, 0, 0, flag));
    assert!(!first.has_object(&loaded.game, 1, 0, flag));
    assert!(second.has_object(&loaded.game, 1, 0, flag));
}

#[test]
fn schema_selector_reads_var_updated_by_previous_statement_in_same_turn() {
    let source = r#"
title = dynamic_selector_same_turn_update

puzzle default {
var count = 0

tags {
num = 0 1 2
}

layers {
__legacy_layer_1 = Count:num
}
empty .

legend {
. = empty
0 = Count:0
}

rules {
count += 1
once [ Count:* ] -> [ Count:count ]
}

level "start" {
0
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let count_0 = object_named(&loaded, "Count:0");
    let count_1 = object_named(&loaded, "Count:1");

    assert_eq!(next.visible_variables(), &[1]);
    assert!(!next.has_object(&loaded.game, 0, 0, count_0));
    assert!(next.has_object(&loaded.game, 0, 0, count_1));
}

#[test]
fn repeated_schema_selector_reads_var_updated_by_previous_statement_in_same_turn() {
    let source = r#"
title = dynamic_selector_same_turn_update_repeated

puzzle default {
var count = 0

tags {
num = 0 1 2
}

layers {
__legacy_layer_1 = Count:num
}
empty .

legend {
. = empty
0 = Count:0
}

rules {
count += 1
[ Count:* ] -> [ Count:count ]
}

level "start" {
0
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let count_0 = object_named(&loaded, "Count:0");
    let count_1 = object_named(&loaded, "Count:1");

    assert_eq!(next.visible_variables(), &[1]);
    assert!(!next.has_object(&loaded.game, 0, 0, count_0));
    assert!(next.has_object(&loaded.game, 0, 0, count_1));
}

#[test]
fn schema_selector_rhs_uses_current_var_value() {
    let source = r#"
title = dynamic_selector_rhs_current_value

puzzle default {
var count = 1

tags {
num = 0 1 2
}

layers {
__legacy_layer_1 = Count:num
}
empty .

legend {
. = empty
0 = Count:0
}

rules {
once [ Count:* ] -> [ Count:count ]
}

level "start" {
0
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let count_0 = object_named(&loaded, "Count:0");
    let count_1 = object_named(&loaded, "Count:1");

    assert_eq!(next.visible_variables(), &[1]);
    assert!(!next.has_object(&loaded.game, 0, 0, count_0));
    assert!(next.has_object(&loaded.game, 0, 0, count_1));
}

#[test]
fn schema_selector_rhs_mutable_var_tag_does_not_warn() {
    let source = r#"
title = dynamic_selector_rhs_no_warning

puzzle default {
var count = 1

tags {
num = 0 1 2
}

layers {
__legacy_layer_1 = Count:num
}
empty .

legend {
. = empty
0 = Count:0
}

rules {
once [ Count:* ] -> [ Count:count ]
}

level "start" {
0
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(
        !loaded
            .warnings
            .iter()
            .any(|warning| warning.contains("dynamic selector `Count:count` uses mutable var"))
    );
}

#[test]
fn schema_selector_rhs_lowers_to_guarded_concrete_write() {
    let source = r#"
title = dynamic_selector_rhs_lowering

puzzle default {
var count = 1

tags {
num = 0 1 2
}

layers {
__legacy_layer_1 = Count:num
}
empty .

legend {
. = empty
0 = Count:0
}

rules {
once [ Count:* ] -> [ Count:count ]
}

level "start" {
0
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let count_variable = loaded
        .variable_labels
        .iter()
        .find_map(|(variable, label)| (label == "count").then_some(*variable))
        .unwrap();
    let count_1 = object_named(&loaded, "Count:1");

    let rules = loaded.game.rules();
    assert!(rules.iter().any(|rule| {
        rule.guards.iter().any(|guard| {
            matches!(
                guard,
                Guard::VariableEquals {
                    variable,
                    value: 1
                } if *variable == count_variable
            )
        }) && rule.writes.iter().any(|write| {
            matches!(
                write,
                WriteOp::Add {
                    object,
                    ..
                } if *object == count_1
            )
        })
    }));
}

#[test]
fn dynamic_selector_suffix_update_runs_once_after_rewrite_triggers() {
    let source = r#"
title = dynamic_selector_same_rewrite_effect_order

puzzle default {
var count = 0

tags {
num = 0 1 2
}

layers {
__legacy_layer_1 = Count:num
}
empty .

legend {
. = empty
0 = Count:0
}

rules {
[ Count:* ] -> [ Count:count ] count += 1
}

level "start" {
0
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let count_0 = object_named(&loaded, "Count:0");
    let count_1 = object_named(&loaded, "Count:1");

    assert_eq!(next.visible_variables(), &[1]);
    assert!(next.has_object(&loaded.game, 0, 0, count_0));
    assert!(!next.has_object(&loaded.game, 0, 0, count_1));
}

#[test]
fn schema_selector_out_of_domain_var_value_does_not_match() {
    let source = r#"
title = dynamic_selector_out_of_domain

puzzle default {
var count = 2

tags {
num = 1 2 3
}

layers {
__legacy_layer_0 = Flag
__legacy_layer_1 = Box:num
}
empty .

legend {
. = empty
B = Box:2
C = Box:3
}

rules {
once [ Box:count no Flag ] -> [ Box:count Flag ]
count = 4
}

level "start" {
BC
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let flag = object_named(&loaded, "Flag");
    let first =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let second = transition_state(&loaded.game, &first, InputId(0)).unwrap();

    assert!(first.has_object(&loaded.game, 0, 0, flag));
    assert!(!second.has_object(&loaded.game, 1, 0, flag));
}

#[test]
fn const_backed_schema_selector_does_not_warn() {
    let source = r#"
title = dynamic_selector_const

puzzle default {
const count = 2

tags {
num = 1 2 3
}

layers {
__legacy_layer_0 = Marker
__legacy_layer_1 = Box:num
}
empty .

legend {
. = empty
B = Box:2
}

rules {
once [ Box:count no Marker ] -> [ Box:count Marker ]
}

level "start" {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(!loaded.warnings.iter().any(|warning| {
        warning.contains("dynamic selector `Box:count` uses mutable var `count`")
    }));
}

#[test]
fn dynamic_schema_selector_rejects_var_and_tag_set_name_collision() {
    let source = r#"
title = dynamic_selector_tag_set_collision

puzzle default {
var count = 2

tags {
count = 1 2 3
}

layers {
__legacy_layer_1 = Box:count
}
empty .

legend {
. = empty
B = Box:2
}

rules {
once [ Box:count ] -> [ Box:count ]
}

level "start" {
B
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("selector tag count is ambiguous"));
}

#[test]
fn dynamic_schema_selector_rejects_var_and_tag_value_name_collision() {
    let source = r#"
title = dynamic_selector_tag_value_collision

puzzle default {
var count = 2

tags {
num = count 2 3
}

layers {
__legacy_layer_1 = Box:num
}
empty .

legend {
. = empty
B = Box:2
}

rules {
once [ Box:count ] -> [ Box:count ]
}

level "start" {
B
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("selector tag count is ambiguous"));
}

#[test]
fn condition_blocks_accept_no_pattern_all_on_and_count_compare() {
    let source = r#"
title = condition_fixture
puzzle default {
layers {
__legacy_layer_0 = Goal
__legacy_layer_1 = Box Wall
}
legend G = Goal
legend B = Box
legend # = Wall
groups {
solid = Box Wall
}
legend {
. = empty
* = Goal Box
}
lose_conditions all {
no [ Box | Box ]
all Box on Goal
count(Box) == 1
}
rules {

}
levels {
level "start"
*#
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_lose_complete(&loaded.levels[0].initial_state));
}

#[test]
fn condition_blocks_lower_none_function_to_short_circuit_condition_def() {
    let source = r#"
title = none_condition_fixture
puzzle default {
layers {
__legacy_layer_0 = Goal
__legacy_layer_1 = Box
}
legend G = Goal
legend B = Box
legend {
. = empty
* = Goal Box
}
win_conditions {
exists(Goal)
none([ Goal no Box ])
}
rules {

}
levels {
level "start"
*
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let goal = loaded.goal.as_ref().unwrap();

    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
    let GoalExpr::All(exprs) = &goal.expr else {
        panic!("win_conditions with two rows should lower to all expression");
    };
    assert!(
        exprs.iter().any(|expr| matches!(
            expr,
            GoalExpr::Clause(GoalClause {
                value: GoalValue::InlineConditionValue(ConditionValueKind::NoneMatches(_)),
                op: ComparisonOp::NotEq,
                expected: 0,
            })
        )),
        "none(pattern) should stay a NoneMatches condition, not lower to count(pattern) == 0"
    );
}

#[test]
fn all_on_lowers_to_generic_goal_and_generates_solver_strategy() {
    let source = r#"
title = all_on_semantics
puzzle default {
empty .
layers {
floor = Goal
actor = Box
}
legend G = Goal
legend B = Box
win_conditions {
all Goal on Box
}
rules {
}
level "start" {
GB
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let goal = loaded.goal.as_ref().unwrap();
    assert!(matches!(
        &goal.expr,
        GoalExpr::Clause(GoalClause {
            value: GoalValue::InlineConditionValue(ConditionValueKind::NoneMatches(_)),
            op: ComparisonOp::NotEq,
            expected: 0,
        })
    ));
    let QueryExpr::AllOnDistance { subjects, covers } = &loaded.solver_strategy.terms[0].value
    else {
        panic!("all X on Y should generate an all-on solver strategy");
    };
    assert_eq!(loaded.object_labels.get(&subjects[0]).unwrap(), "Goal");
    assert_eq!(loaded.object_labels.get(&covers[0]).unwrap(), "Box");
    assert!(!loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn schema_selector_tag_can_be_subset_value_set() {
    let source = r#"
title = subset_selector

puzzle default {
empty .

tags {
kind = A B C D
}
tags {
kindprime = A B C
}

layers {
__legacy_layer_1 = Target:kind
}

legend a = Target:A
legend b = Target:B
legend c = Target:C
legend d = Target:D

win_conditions = count(Target:kindprime) == 3

rules {

}

level "start" {
abcd
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn schema_selector_subset_value_set_must_fit_axis_values() {
    let source = r#"
title = subset_selector_bad_value

puzzle default {
empty .

tags {
kind = A B C D
}
tags {
kindprime = A B X
}

layers {
__legacy_layer_1 = Target:kind
}

legend a = Target:A

win_conditions = count(Target:kindprime) == 0

rules {

}

level "start" {
a
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("tag set kindprime contains value X"));
    assert!(error.contains("Target tag slot kind"));
}

#[test]
fn schema_selector_tag_cannot_be_both_value_and_value_set() {
    let source = r#"
title = subset_selector_ambiguous_tag

puzzle default {
empty .

tags {
kind = directions A
}

layers {
__legacy_layer_1 = Target:kind
}

legend a = Target:A

win_conditions = count(Target:directions) == 0

rules {

}

level "start" {
a
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("selector tag directions is ambiguous"));
    assert!(error.contains("Target tag slot kind"));
}

#[test]
fn schema_selector_direction_symbols_are_relative_to_rule_orientation() {
    let source = r#"
title = relative_direction_selector

puzzle default {
empty .

layers {
actor = Marker:directions
}

legend {
r = Marker:right
}

rules {
once right [ Marker:> ] -> [ Marker:v ]
}

levels {
level "start"
r
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let marker_down = object_named(&loaded, "Marker:down");
    let state =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert!(state.has_object(&loaded.game, 0, 0, marker_down));
}

#[test]
fn schema_selector_subset_value_sets_are_positional_for_multiple_axes() {
    let source = r#"
title = subset_selector_two_axes

puzzle default {
empty .

tags {
kind = A B C D
}
tags {
kindprime = A B C
}
tags {
state = on off
}
tags {
stateprime = on
}

layers {
__legacy_layer_1 = Target:kind:state
}

legend a = Target:A:on
legend b = Target:B:on
legend c = Target:C:on
legend x = Target:A:off

win_conditions = count(Target:kindprime:stateprime) == 3

rules {

}

level "start" {
abcx
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn schema_selector_subset_value_sets_do_not_skip_axes() {
    let source = r#"
title = subset_selector_no_axis_skip

puzzle default {
empty .

tags {
kind = A B C D
}
tags {
state = on off
}
tags {
stateprime = on
}

layers {
__legacy_layer_1 = Target:kind:state
}

legend a = Target:A:on

win_conditions = count(Target:stateprime) == 1

rules {

}

level "start" {
a
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("object selector must name every variant slot"));
    assert!(error.contains("use * for unconstrained slots"));
}

#[test]
fn star_selector_matches_all_schema_variants() {
    let source = r#"
title = star_selector

puzzle default {
empty .

tags {
facing = left right
}

layers {
__legacy_layer_1 = player:facing
}

legend l = player:left
legend r = player:right

input right direction right

rules {

once input directions [ player:* | ] -> [ | player:* ]
}

level "start" {
r.
}
}

"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.game.rules().len(), 9);

    let right = input_named(&loaded, "right");
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player_right = object_named(&loaded, "player:right");

    assert!(moved.has_object(&loaded.game, 1, 0, player_right));
}

#[test]
fn layers_declare_new_variant_tag_values() {
    let source = r#"
title = layer_declares_new_variant_tag

puzzle default {
empty .

tags {
kind = A B
}

layers {
target = Target:kind Target:Z
}

legend z = Target:Z

win_conditions = count(Target:kind) == 1

rules {

}

level "start" {
z
}
}

"#;
    let loaded = parse_game(source).unwrap();

    object_named(&loaded, "Target:Z");
    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn layers_reject_undeclared_tag_set_schema_slots() {
    let source = r#"
title = layer_rejects_undeclared_tag_set

puzzle default {
empty .

layers {
target = Target:missing_axis
}

rules {

}

level "start" {
.
}
}

"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("object schema tag slot must name a tag set"));
}

#[test]
fn layers_can_use_later_groups_to_declare_variant_values() {
    let source = r#"
title = later_group_declares_layer_objects

puzzle default {
empty .

tags {
kind = A B
}

layers {
target = specials Target:kind
}

groups {
specials = Target:Z
}

legend z = Target:Z

win_conditions = count(specials) == 1

rules {

}

level "start" {
z
}
}

"#;
    let loaded = parse_game(source).unwrap();

    object_named(&loaded, "Target:Z");
    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn groups_do_not_declare_variant_values_outside_layers() {
    let source = r#"
title = group_does_not_declare_variant

puzzle default {
empty .

tags {
kind = A B
}

layers {
target = Target:kind
}

groups {
specials = Target:Z
}

legend a = Target:A

rules {

}

level "start" {
a
}
}

"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(
        error.contains("object selector matched no objects"),
        "{error}"
    );
}

#[test]
fn bare_star_selector_matches_any_concrete_object() {
    let source = r#"
title = bare_star_selector

puzzle default {
empty .

tags {
state = on off
}

layers {
__legacy_layer_1 = Player Box:state
}

legend P = Player
legend b = Box:on

input right direction right
win_conditions = count(*) == 2

rules {
input right [ * | no * ] -> [ | * ]
}

level "start" {
Pb.
}
}

"#;
    let loaded = parse_game(source).unwrap();
    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));

    let right = input_named(&loaded, "right");
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let box_on = object_named(&loaded, "Box:on");

    assert!(moved.has_object(&loaded.game, 2, 0, box_on));
}

#[test]
fn typed_tags_declare_angle_and_vec2_variant_domains() {
    let source = r#"
title = geometric_axes

puzzle default {
empty .

tags {
color = red blue
facing = 0deg..<360deg step 90deg
offset = (0..<1 step 1/2, 0..<1 step 1/2)
}

layers {
__legacy_layer_1 = Box:facing:color Ball:offset:color
}

legend b = Box:0deg:red
legend o = Ball:(0, 0):red

rules {

}

level "start" {
bo
}
}
"#;
    let loaded = parse_game(source).unwrap();

    object_named(&loaded, "Box:270deg:blue");
    object_named(&loaded, "Ball:(1/2,1/2):blue");
}

#[test]
fn frame3_tags_accept_domain_sugar_and_require_parenthesized_object_slots() {
    let source = r#"
title = frame3_axis

puzzle default {
empty .

tags {
pose = right, front front, left
}

layers {
actors = Die:pose
}

legend d = Die:(right, front)

rules {

}

level "start" {
d
}
}
"#;
    let loaded = parse_game(source).unwrap();

    object_named(&loaded, "Die:(right,front)");
    object_named(&loaded, "Die:(front,left)");
}

#[test]
fn frame3_object_slots_reject_unparenthesized_values() {
    let source = r#"
title = frame3_slot_requires_parentheses

puzzle default {
empty .

tags {
pose = right, front
}

layers {
actors = Die:pose
}

legend d = Die:right,front

rules {

}

level "start" {
d
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("frame3 value must be parenthesized"));
}

#[test]
fn vec2_domain_expansion_accepts_independent_component_domains_internally() {
    let x = [Rational::ZERO, Rational::new(1, 2).unwrap()];
    let y = [Rational::integer(-1), Rational::integer(1)];

    assert_eq!(
        expand_vec2_domain(&x, &y),
        ["(0,-1)", "(0,1)", "(1/2,-1)", "(1/2,1)"]
    );
}

#[test]
fn sprite_transforms_bind_typed_slots_and_preserve_source_order() {
    let source = r#"
title = typed_sprite_transforms

puzzle default {
empty .

tags {
hor = 0..<1 step 0.5
}

layers {
actors = Player:directions:hor
}

legend p = Player:right:0.5

sprites {
Player:directions:hor {
colors = #fff
translate (hor, 0)
rotate directions from up
0
}
}

rules {

}

level "start" {
p
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let sprite = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player-right-1-2")
        .unwrap();

    assert_eq!(
        sprite.transforms,
        [
            VisualSpriteTransform::Translate { x: 0.5, y: 0.0 },
            VisualSpriteTransform::Rotate { degrees: -90.0 },
        ]
    );
    let up = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player-up-1-2")
        .unwrap();
    assert_eq!(
        up.transforms,
        [
            VisualSpriteTransform::Translate { x: 0.5, y: 0.0 },
            VisualSpriteTransform::Rotate { degrees: 0.0 },
        ]
    );
    let down = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player-down-1-2")
        .unwrap();
    assert_eq!(
        down.transforms,
        [
            VisualSpriteTransform::Translate { x: 0.5, y: 0.0 },
            VisualSpriteTransform::Rotate { degrees: -180.0 },
        ]
    );
}

#[test]
fn sprite_direction_minus_angle_matches_rotate_from_sugar() {
    let bindings = HashMap::from([("directions".to_string(), "up".to_string())]);
    let explicit =
        eval_sprite_angle_expr("directions - 90deg", &bindings, "rotate expression").unwrap();
    let from_sugar = eval_sprite_angle_expr("directions", &bindings, "rotate expression")
        .unwrap()
        .sub(eval_sprite_angle_expr("up", &bindings, "rotate expression").unwrap());

    assert_eq!(explicit, from_sugar);
    assert_eq!(explicit, Rational::ZERO);
}

#[test]
fn sprite_flip_binds_boolean_tag_values() {
    let source = r#"
title = sprite_flip

puzzle default {
empty .

tags {
reversed = false true
}

layers {
actors = Player:reversed
}

legend p = Player:true

sprites {
Player:reversed {
colors = #fff
flip reversed
0
}
}

rules {

}

level "start" {
p
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let flipped = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player-true")
        .unwrap();
    let unflipped = loaded
        .visuals
        .sprites
        .iter()
        .find(|sprite| sprite.name == "Player-false")
        .unwrap();

    assert_eq!(
        flipped.transforms,
        [VisualSpriteTransform::Flip { enabled: true }]
    );
    assert_eq!(
        unflipped.transforms,
        [VisualSpriteTransform::Flip { enabled: false }]
    );
}

#[test]
fn vec2_domain_accepts_literal_and_component_range_items() {
    let source = r#"
title = vec2_domain_items

puzzle default {
empty .

tags {
offset = (0, -1) (0..<1 step 0.5, 0..<1 step 1/2)
}

layers {
actors = Ball:offset
}

legend b = Ball:(0.5, 0.5)

rules {

}

level "start" {
b
}
}
"#;
    let loaded = parse_game(source).unwrap();

    object_named(&loaded, "Ball:(0,-1)");
    object_named(&loaded, "Ball:(1/2,1/2)");
}

#[test]
fn computed_rotation_axis_replacement_uses_captured_axis() {
    let source = r#"
title = computed_rotation

puzzle default {
empty .

tags {
color = red blue
facing = 0deg..<360deg step 90deg
}

layers {
__legacy_layer_1 = Box:facing:color
}

legend b = Box:0deg:red

rules {
once [ Box:facing:red ] -> [ Box:(facing + 90deg):red ]
}

level "start" {
b
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert!(
        moved.has_object(&loaded.game, 0, 0, object_named(&loaded, "Box:90deg:red")),
        "objects at 0,0: {:?}",
        labels_at(&loaded, &moved, 0, 0)
    );
}

#[test]
fn computed_translation_axis_accepts_coordinate_and_direction_sum() {
    let source = r#"
title = computed_translation

puzzle default {
empty .

tags {
offset = (0..<1 step 1/2, 0..<1 step 1/2)
facing = 0deg..<360deg step 90deg
}

layers {
__legacy_layer_1 = Ball:offset Arrow:facing:offset
}

legend b = Ball:(0, 0)
legend a = Arrow:0deg:(1/2, 1/2)

rules {
once [ Ball:offset ] -> [ Ball:(offset + (0, 0)) ]
once [ Arrow:facing:offset ] -> [ Arrow:facing:(offset + 1/2 < + 1/2 >) ]
}

level "start" {
ba
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert!(moved.has_object(&loaded.game, 0, 0, object_named(&loaded, "Ball:(0,0)")));
    assert!(moved.has_object(
        &loaded.game,
        1,
        0,
        object_named(&loaded, "Arrow:0deg:(1/2,1/2)")
    ));
}

#[test]
fn computed_translation_rejects_undeclared_offset_target() {
    let source = r#"
title = computed_translation_error

puzzle default {
empty .

tags {
offset = (0...1/2 step 1/2, 0...1/2 step 1/2)
}

layers {
__legacy_layer_1 = Ball:offset
}

legend b = Ball:(1/2, 0)

rules {
once [ Ball:offset ] -> [ Ball:(offset + (1/2, 0)) ]
}

level "start" {
b
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("vec2 computed selector target is not declared"));
}

#[test]
fn angle_tag_domains_accept_literal_lists() {
    let source = r#"
title = typed_tag_range_required

puzzle default {
empty .

tags {
facing = 0deg 90deg
}

layers {
actor = Player:facing
}

rules {

}

legend p = Player:90deg

level "start" {
p
}
}
"#;
    let loaded = parse_game(source).unwrap();
    object_named(&loaded, "Player:90deg");
}

#[test]
fn legacy_geometric_axis_kinds_fail_visibly() {
    let source = r#"
title = legacy_geometric_type

puzzle default {
empty .

tags {
facing = rotation step 90deg
}

layers {
actor = Player:facing
}

rules {

}

level "start" {
.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("tag value types are inferred from literals"));
}

#[test]
fn vec2_tag_values_require_parentheses() {
    let source = r#"
title = vec2_parentheses

puzzle default {
empty .

tags {
offset = (0..<1 step 1/2, 0..<1 step 1/2)
}

layers {
actor = Ball:offset
}

legend b = Ball:0,0

rules {

}

level "start" {
b
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("vec2 value must be parenthesized"));
}

#[test]
fn underscore_selector_wildcard_is_rejected() {
    let source = r#"
title = underscore_selector

puzzle default {
empty .

tags {
facing = left right
}

layers {
__legacy_layer_1 = player:facing
}

legend l = player:left

rules {

once [ player:_ ] -> [ player:_ ]
}

level "start"
l
}
}

"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("wildcard must use *"));
    assert!(error.contains("_ is reserved for completion"));
}

#[test]
fn bare_schema_family_selector_is_rejected() {
    let source = r#"
title = bare_schema_selector

puzzle default {
empty .

tags {
facing = left right
}

layers {
__legacy_layer_1 = player:facing
}

legend l = player:left

rules {

once [ player ] -> [ player ]
}

level "start"
l
}
}

"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("object selector for variants must use :*"));
}

#[test]
fn star_selector_fills_unconstrained_variant_slots() {
    let source = r#"
title = star_selector_slots

puzzle default {
empty .

tags {
kind = A B
}
tags {
state = on off
}

layers {
__legacy_layer_1 = Target:kind:state
}

legend a = Target:A:on
legend b = Target:B:on
legend x = Target:A:off

win_conditions = count(Target:*:on) == 2

rules {

}

level "start" {
abx
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn family_wildcard_selector_matches_tag_across_schema_families() {
    let source = r#"
title = family_wildcard_selector

puzzle default {
empty .

tags {
state = on off
}

layers {
__legacy_layer_1 = Door:state Switch:state
}

legend d = Door:on
legend s = Switch:on
legend x = Door:off

win_conditions = count(*:on) == 2

rules {

}

level "start" {
dsx
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn family_wildcard_selector_maps_matching_family_on_rhs() {
    let source = r#"
title = family_wildcard_rewrite

puzzle default {
empty .

tags {
state = A B
}

layers {
__legacy_layer_1 = Door:state Switch:state
}

legend d = Door:A
legend s = Switch:A

rules {
once [ *:A ] -> [ *:B ]
}

level "start" {
ds
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let door_b = object_named(&loaded, "Door:B");
    let switch_b = object_named(&loaded, "Switch:B");

    assert!(next.has_object(&loaded.game, 0, 0, door_b));
    assert!(next.has_object(&loaded.game, 1, 0, switch_b));
}

#[test]
fn family_wildcard_rhs_allows_tag_set_and_group_name_overlap() {
    let source = r#"
title = family_wildcard_group_tag_overlap

puzzle default {
empty .

tags {
state = stack movable
}

layers {
__legacy_layer_1 = Crate:state
}

groups {
movable = Crate:movable
}

legend c = Crate:stack

rules {
once [ *:stack ] -> [ *:movable ]
}

level "start" {
c
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let crate_movable = object_named(&loaded, "Crate:movable");

    assert!(next.has_object(&loaded.game, 0, 0, crate_movable));
}

#[test]
fn qualified_tag_selector_expands_object_name_atoms_mechanically() {
    let source = r#"
title = qualified_tag_selector

puzzle default {
empty .

tags {
kind = a b
pair = A B
}

layers {
__legacy_layer_1 = A:kind B:kind C:kind
}

legend a = A:a
legend b = B:a
legend c = C:a

win_conditions = count(pair:a) == 2

rules {

}

level "start" {
abc
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
    assert!(!loaded.object_groups.contains_key("pair"));
}

#[test]
fn group_rows_reject_bare_family_terms_instead_of_deferring_them() {
    let source = r#"
title = bare_family_group_rejected

puzzle default {
empty .

tags {
kind = a b
}

layers {
__legacy_layer_1 = A:kind B:kind
}

groups {
pair = A B
}

legend a = A:a

win_conditions = count(pair) == 0

rules {

}

level "start" {
a
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("object selector for variants must use :*"));
}

#[test]
fn qualified_tag_selector_rhs_maps_matching_object_name_atoms() {
    let source = r#"
title = qualified_tag_selector_rhs

puzzle default {
empty .

tags {
kind = a b
pair = A B
}

layers {
__legacy_layer_1 = A:kind B:kind C:kind
}

legend x = A:a
legend y = B:a
legend z = C:a

rules {
once [ pair:a ] -> [ pair:b ]
}

level "start" {
xyz
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let a_b = object_named(&loaded, "A:b");
    let b_b = object_named(&loaded, "B:b");
    let c_a = object_named(&loaded, "C:a");

    assert!(next.has_object(&loaded.game, 0, 0, a_b));
    assert!(next.has_object(&loaded.game, 1, 0, b_b));
    assert!(next.has_object(&loaded.game, 2, 0, c_a));
}

#[test]
fn qualified_tag_selector_occurrence_labels_attach_before_suffix() {
    let source = r#"
title = qualified_tag_selector_occurrence_labels

puzzle default {
empty .

tags {
kind = a b
pair = A B
}

layers {
__legacy_layer_1 = A:kind B:kind
}

legend x = A:a
legend y = B:a

rules {
once [ pair#1:a | pair#2:a ] -> [ pair#2:b | pair#1:b ]
}

level "start" {
xy
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let next = transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let a_b = object_named(&loaded, "A:b");
    let b_b = object_named(&loaded, "B:b");

    assert!(next.has_object(&loaded.game, 0, 0, b_b));
    assert!(next.has_object(&loaded.game, 1, 0, a_b));
}

#[test]
fn qualified_tag_selector_errors_when_an_atom_cannot_take_the_suffix() {
    let source = r#"
title = qualified_tag_selector_bad_atom

puzzle default {
empty .

tags {
kind = a b
mixed = A Wall
}

layers {
__legacy_layer_1 = A:kind Wall
}

legend a = A:a
legend # = Wall

win_conditions = count(mixed:a) == 1

rules {

}

level "start" {
a#
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown object selector"));
}

#[test]
fn group_selector_suffix_is_not_a_group_feature() {
    let source = r#"
title = group_suffix_rejected

puzzle default {
empty .

tags {
kind = a b
}

layers {
__legacy_layer_1 = A:kind B:kind
}

groups {
pair = A:a B:a
}

legend a = A:a

win_conditions = count(pair:b) == 0

rules {

}

level "start" {
a
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown object selector"));
}

#[test]
fn object_family_base_can_also_be_a_concrete_object() {
    let source = r#"
title = family_exact_object

puzzle main {
tags {
state = open close
}

layers {
room = Room Room:state
marker = Marker:state Marker
}

rules {
[ Room ] -> [ Room:open ]
[ Marker ] -> [ Marker:open ]
}

levels {
legend {
. = empty
R = Room
M = Marker
O = Room:open
K = Marker:open
}

level "start"
RM
}
}
"#;
    let loaded = super::parse_game2d(source).unwrap();
    let room = object_named(&loaded, "Room");
    let room_open = object_named(&loaded, "Room:open");
    let marker = object_named(&loaded, "Marker");
    let marker_open = object_named(&loaded, "Marker:open");

    let initial = &loaded.levels[0].initial_state;
    assert!(initial.has_object(&loaded.game, 0, 0, room));
    assert!(!initial.has_object(&loaded.game, 0, 0, room_open));
    assert!(initial.has_object(&loaded.game, 1, 0, marker));
    assert!(!initial.has_object(&loaded.game, 1, 0, marker_open));

    let next = transition_state(&loaded.game, initial, InputId(0)).unwrap();
    assert!(next.has_object(&loaded.game, 0, 0, room_open));
    assert!(!next.has_object(&loaded.game, 0, 0, room));
    assert!(next.has_object(&loaded.game, 1, 0, marker_open));
    assert!(!next.has_object(&loaded.game, 1, 0, marker));
}

#[test]
fn bare_schema_family_selector_without_exact_object_is_rejected() {
    let source = r#"
title = family_base_without_exact_object

puzzle main {
tags {
color = red blue
}

layers {
base = marker:color
}

rules {
[ marker ] -> [ marker:red ]
}

levels {
legend {
. = empty
r = marker:red
}

level "start"
r
}
}
"#;
    let error = super::parse_game2d(source).unwrap_err().to_string();
    assert!(error.contains("object selector for variants must use :*"));
}

#[test]
fn blank_lines_split_level_into_auto_placed_regions() {
    let source = r#"
title = region_level

puzzle default {
layers {
__legacy_layer_1 = Player Box
}
empty .

legend P = Player
legend B = Box

rules {
once input directions [ Player | ] -> [ | Player ]
}

level "start" {
P.
..

.B
..
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let stage = &loaded.levels[0];

    assert_eq!(stage.initial_state.width, 6);
    assert_eq!(stage.initial_state.height, 2);
    assert_eq!(stage.regions.len(), 2);
    assert_eq!(stage.regions[0].x, 0);
    assert_eq!(stage.regions[0].width, 2);
    assert_eq!(stage.regions[1].x, 4);
    assert_eq!(stage.regions[1].width, 2);

    let player = object_named(&loaded, "Player");
    let box_object = object_named(&loaded, "Box");
    assert!(stage.initial_state.has_object(&loaded.game, 0, 0, player));
    assert!(
        stage
            .initial_state
            .has_object(&loaded.game, 5, 0, box_object)
    );
}

#[test]
fn levels_block_accepts_unbraced_named_levels_split_by_blank_lines() {
    let source = r#"
title = unbraced_named_levels

puzzle default {
layers {
__legacy_layer_0 = Player Box
}
empty .
legend P = Player
legend B = Box
rules {

}
levels {
level "intro"
P

level "followup"
B
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.levels.len(), 2);
    assert_eq!(loaded.levels[0].name, "intro");
    assert_eq!(loaded.levels[1].name, "followup");
}

#[test]
fn levels_block_accepts_unnamed_levels_split_by_blank_lines() {
    let source = r#"
title = unnamed_levels

puzzle default {
layers {
__legacy_layer_0 = Player Box
}
empty .
legend P = Player
legend B = Box
rules {

}
levels {
P

B
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.levels.len(), 2);
    assert_eq!(loaded.levels[0].name, "unnamed level 1");
    assert_eq!(loaded.levels[1].name, "unnamed level 2");
}

#[test]
fn levels_block_accepts_braced_unnamed_multi_region_level() {
    let source = r#"
title = unnamed_multi_region

puzzle default {
layers {
__legacy_layer_0 = Player Box
}
empty .
legend P = Player
legend B = Box
rules {

}
levels {
{
P.

.B
}
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.levels.len(), 1);
    assert_eq!(loaded.levels[0].name, "unnamed level 1");
    assert_eq!(loaded.levels[0].regions.len(), 2);
}

#[test]
fn levels_block_accepts_canonical_level_name_definition() {
    let source = r#"
title = canonical_level_name

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
legend P = Player
rules {

}
levels {
level {
name = "intro"
P
}
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.levels.len(), 1);
    assert_eq!(loaded.levels[0].name, "intro");
}

#[test]
fn levels_block_rejects_legacy_braced_name_without_level_keyword() {
    let source = r#"
title = legacy_braced_level_name

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
legend P = Player
rules {
}
levels {
intro {
P
}
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("braced level header must be `level <name> {`"));
}

#[test]
fn levels_block_rejects_braces_in_ascii_rows() {
    let source = r#"
title = level_ascii_braces

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
legend P = Player
rules {
}
levels {
level "start" {
P{
}
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("brace"), "{error}");
}

#[test]
fn level_ascii_layers_overlay_empty_cells_as_transparent() {
    let source = r#"
title = level_ascii_layers

puzzle default {
layers {
terrain = Floor
actor = Player
}
empty .
legend f = Floor
legend P = Player
rules {
}
levels {
level "start" {
fff
fff
fff
+
...
.P.
...
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let floor = object_named(&loaded, "Floor");
    let player = object_named(&loaded, "Player");
    let initial = &loaded.levels[0].initial_state;

    assert_eq!(initial.width, 3);
    assert_eq!(initial.height, 3);
    assert!(initial.has_object(&loaded.game, 1, 1, floor));
    assert!(initial.has_object(&loaded.game, 1, 1, player));
    assert!(initial.has_object(&loaded.game, 0, 0, floor));
    assert!(!initial.has_object(&loaded.game, 0, 0, player));
}

#[test]
fn level_ascii_layers_reject_different_sizes_in_same_region() {
    let source = r#"
title = level_ascii_layer_size

puzzle default {
layers {
actor = Player
}
empty .
legend P = Player
rules {
}
levels {
level "start" {
PP
+
P
}
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("level ASCII layers in the same region must have the same size"));
}

#[test]
fn level_ascii_layers_reject_separator_without_following_layer() {
    let source = r#"
title = level_ascii_layer_separator

puzzle default {
layers {
actor = Player
}
empty .
legend P = Player
rules {
}
levels {
level "start" {
P
+
}
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("level layer separator requires a following ASCII layer"));
}

#[test]
fn level_ascii_layers_preserve_blank_line_region_split() {
    let source = r#"
title = level_ascii_layer_regions

puzzle default {
layers {
terrain = Floor
actor = Player Box
}
empty .
legend f = Floor
legend P = Player
legend B = Box
rules {
}
levels {
level "start" {
ff
ff
+
P.
..

ff
ff
+
.B
..
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let floor = object_named(&loaded, "Floor");
    let player = object_named(&loaded, "Player");
    let box_object = object_named(&loaded, "Box");
    let stage = &loaded.levels[0];

    assert_eq!(stage.initial_state.width, 6);
    assert_eq!(stage.initial_state.height, 2);
    assert_eq!(stage.regions.len(), 2);
    assert!(stage.initial_state.has_object(&loaded.game, 0, 0, floor));
    assert!(stage.initial_state.has_object(&loaded.game, 0, 0, player));
    assert!(
        stage
            .initial_state
            .has_object(&loaded.game, 5, 0, box_object)
    );
}

#[test]
fn level_ascii_layers_prefer_upper_object_on_same_core_layer() {
    let source = r#"
title = level_ascii_layer_priority

puzzle default {
layers {
actor = Player Box
}
empty .
legend P = Player
legend B = Box
rules {
}
levels {
level "start" {
P
+
B
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let player = object_named(&loaded, "Player");
    let box_object = object_named(&loaded, "Box");
    let initial = &loaded.levels[0].initial_state;

    assert!(!initial.has_object(&loaded.game, 0, 0, player));
    assert!(initial.has_object(&loaded.game, 0, 0, box_object));
}

#[test]
fn puzzle_view_parses_flickscreen_viewport_controls() {
    let source = r#"
title = frame_view

puzzle default {
layers {
__legacy_layer_1 = Player
}
empty .

legend P = Player

flickscreen 5x3
screen_focus Player

rules {

once input directions [ Player | ] -> [ | Player ]
}

level "start" {
P..
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(
        loaded.screen.viewport_size,
        ViewportSizeDef::Size {
            width: 5,
            height: 3
        }
    );
    assert_eq!(loaded.screen.viewport_focus, "Player");
    assert_eq!(loaded.screen.viewport_mode, ViewportModeDef::Paged);
}

#[test]
fn puzzle_view_parses_full_flickscreen() {
    let full_source = r#"
title = full_frame

puzzle default {
layers 1
empty .

flickscreen full

rules {

}

level "start" {
.
}
}
"#;
    let full = parse_game(full_source).unwrap();
    assert_eq!(full.screen.viewport_size, ViewportSizeDef::Full);
}

#[test]
fn puzzle_view_rejects_removed_frame_size_syntax() {
    let source = r#"
title = region_frame

puzzle default {
layers 1
empty .

frame_size region

rules {

}

level "start" {
.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("were removed; use `flickscreen`, `zoomscreen`, or `screen_focus`"));
}

#[test]
fn puzzle_view_rejects_removed_frame_focus_syntax() {
    let source = r#"
title = region_frame

puzzle default {
layers 1
empty .

frame_focus Player

rules {

}

level "start" {
.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("were removed; use `flickscreen`, `zoomscreen`, or `screen_focus`"));
}

#[test]
fn puzzle_view_parses_zoomscreen_as_centered_viewport() {
    let source = r#"
title = zoom_view

puzzle default {
layers 1
empty .

zoomscreen 5 3

rules {

}

level "start" {
.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(
        loaded.screen.viewport_size,
        ViewportSizeDef::Size {
            width: 5,
            height: 3
        }
    );
    assert_eq!(loaded.screen.viewport_mode, ViewportModeDef::Centered);
}

#[test]
fn puzzle_render_parses_grid_type_all_cells() {
    let source = r#"
title = grid_render

puzzle default {
layers 1
empty .

render {
grid {
type = "all_cells"
}
}

rules {

}

level "start" {
.
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(!loaded.render.grid.occupied_cells);
    assert!(loaded.render.grid.all_cells);
}

#[test]
fn puzzle_render_parses_grid_type_occupied_cells() {
    let source = r#"
title = grid_render

puzzle default {
layers 1
empty .

render {
grid {
type = "occupied_cells"
}
}

rules {

}

level "start" {
.
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.render.grid.occupied_cells);
    assert!(!loaded.render.grid.all_cells);
}

#[test]
fn puzzle_render_parses_cell_size() {
    let source = r#"
title = cell_size_render

puzzle default {
layers 1
empty .

render {
cell_size = 64
}

rules {

}

level "start" {
.
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.render.cell_size, Some(64));
}

#[test]
fn puzzle_render_rejects_invalid_cell_size() {
    let source = r#"
title = cell_size_render

puzzle default {
layers 1
empty .

render {
cell_size = 0
}

rules {

}

level "start" {
.
}
}
"#;
    let err = parse_game(source).unwrap_err().to_string();

    assert!(err.contains("cell_size must be an integer from 1 to 256"));
}

#[test]
fn puzzle_render_rejects_old_cell_size_value_syntax() {
    let source = r#"
title = cell_size_render

puzzle default {
layers 1
empty .

render {
cell_size 64
}

rules {

}

level "start" {
.
}
}
"#;
    let err = parse_game(source).unwrap_err().to_string();

    assert!(err.contains("cell_size directive must be: cell_size = <pixels>"));
}

#[test]
fn puzzle_render_rejects_old_boolean_grid_assignments() {
    let source = r#"
title = grid_render

puzzle default {
layers 1
empty .

render {
grid {
occupied_cells = true
}
}

rules {

}

level "start" {
.
}
}
"#;

    assert!(parse_game(source).is_err());
}

#[test]
fn puzzle_render_rejects_old_bare_grid_type_rows() {
    let source = r#"
title = grid_render

puzzle default {
layers 1
empty .

render {
grid {
occupied_cells
}
}

rules {

}

level "start" {
.
}
}
"#;

    assert!(parse_game(source).is_err());
}

#[test]
fn repeated_group_selector_expands_independently_and_preserves_occurrence_order() {
    let source = r#"
title = repeated_group_selector

puzzle default {
layers {
__legacy_layer_1 = Box Crate
}
empty .

groups {
cargo = Box Crate
}

legend B = Box
legend C = Crate

rules {
once [ cargo | cargo | ] -> [ | cargo | cargo ]
}

level "start"
BC.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let box_object = object_named(&loaded, "Box");
    let crate_object = object_named(&loaded, "Crate");

    assert!(moved.has_object(&loaded.game, 1, 0, box_object));
    assert!(moved.has_object(&loaded.game, 2, 0, crate_object));
    assert!(!moved.has_object(&loaded.game, 0, 0, box_object));
}

#[test]
fn selector_occurrence_labels_can_swap_group_members() {
    let source = r#"
title = selector_occurrence_labels

puzzle swap {
layers {
actor = Box Crate
}
groups {
solid = Box Crate
}
rules {
once [ solid#1 | solid#2 ] -> [ solid#2 | solid#1 ]
}
}

levels basic of swap {
legend {
. = empty
B = Box
C = Crate
}
BC
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let box_object = object_named(&loaded, "Box");
    let crate_object = object_named(&loaded, "Crate");

    assert!(moved.has_object(&loaded.game, 0, 0, crate_object));
    assert!(moved.has_object(&loaded.game, 1, 0, box_object));
}

#[test]
fn selector_occurrence_labels_can_duplicate_group_members_on_rhs() {
    let source = r#"
title = duplicate_selector_occurrence_label_rhs

puzzle copy {
layers {
actor = Box Crate
}
groups {
solid = Box Crate
}
rules {
once right [ solid#1 | solid#2 ] -> [ solid#1 | solid#1 ]
}
}

levels basic of copy {
legend {
. = empty
B = Box
C = Crate
}
BC
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let box_object = object_named(&loaded, "Box");
    let crate_object = object_named(&loaded, "Crate");

    assert!(moved.has_object(&loaded.game, 0, 0, box_object));
    assert!(moved.has_object(&loaded.game, 1, 0, box_object));
    assert!(!moved.has_object(&loaded.game, 1, 0, crate_object));
}

#[test]
fn single_group_occurrence_duplicates_to_multiple_rhs_cells() {
    let source = r#"
title = duplicate_single_group_occurrence_rhs

puzzle copy {
layers {
actor = Box Crate
}
groups {
solid = Box Crate
}
rules {
once right [ solid | ] -> [ solid | solid ]
}
}

levels basic of copy {
legend {
. = empty
B = Box
C = Crate
}
B.
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let box_object = object_named(&loaded, "Box");
    let crate_object = object_named(&loaded, "Crate");

    assert!(moved.has_object(&loaded.game, 0, 0, box_object));
    assert!(moved.has_object(&loaded.game, 1, 0, box_object));
    assert!(!moved.has_object(&loaded.game, 1, 0, crate_object));
}

#[test]
fn repeated_group_occurrences_do_not_allow_extra_unlabeled_rhs_copy() {
    let source = r#"
title = reject_ambiguous_extra_group_rhs

puzzle copy {
layers {
actor = Box Crate
}
groups {
solid = Box Crate
}
rules {
once right [ solid | solid | ] -> [ solid | solid | solid ]
}
}

levels basic of copy {
legend {
. = empty
B = Box
C = Crate
}
BC.
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("after selector with alternatives must also appear in before"));
}

#[test]
fn selector_occurrence_labels_must_be_unique_in_before_pattern() {
    let source = r#"
title = duplicate_selector_occurrence_label

puzzle swap {
layers {
actor = Box Crate
}
groups {
solid = Box Crate
}
rules {
once [ solid#1 | solid#1 ] -> [ solid#1 | solid#1 ]
}
}

levels basic of swap {
legend {
. = empty
B = Box
C = Crate
}
BC
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("selector occurrence label must be unique"));
}

#[test]
fn object_occurrence_labels_swap_occurrence_mark() {
    let source = r#"
title = object_occurrence_label_mark_swap

puzzle swap {
layers {
marker = HotMarker ColdMarker
actor = Box
}
marks {
hot
cold
}
rules {
once [ Box#1 | Box#2 ] -> [ Box#1{hot} | Box#2{cold} ]
once [ Box#1{hot} | Box#2{cold} ] -> [ Box#2{cold} | Box#1{hot} ]
once [ Box{hot} ] -> [ Box{hot} HotMarker ]
once [ Box{cold} ] -> [ Box{cold} ColdMarker ]
}
}

levels basic of swap {
legend {
. = empty
B = Box
}
BB
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let hot_marker = object_named(&loaded, "HotMarker");
    let cold_marker = object_named(&loaded, "ColdMarker");

    assert!(moved.has_object(&loaded.game, 1, 0, hot_marker));
    assert!(moved.has_object(&loaded.game, 0, 0, cold_marker));
}

#[test]
fn repeated_schema_selector_expands_independently_and_preserves_occurrence_order() {
    let source = r#"
title = repeated_schema_selector

puzzle default {
empty .

tags {
color = red blue
}

layers {
__legacy_layer_1 = box:color
}

legend r = box:red
legend b = box:blue

rules {
once [ box:color | box:color | ] -> [ | box:color | box:color ]
}

level "start"
rb.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let box_red = object_named(&loaded, "box:red");
    let box_blue = object_named(&loaded, "box:blue");

    assert!(moved.has_object(&loaded.game, 1, 0, box_red));
    assert!(moved.has_object(&loaded.game, 2, 0, box_blue));
    assert!(!moved.has_object(&loaded.game, 0, 0, box_red));
}

#[test]
fn set_prefix_supports_integer_assignment_ops() {
    let source = r#"
title = set_prefix_math_effects

puzzle default {
var count = 2

layers {
__legacy_layer_0 = Button
}
empty .

levels {
legend B = Button

level "start" {
B
}
}

rules {
once [ Button ] -> [ Button ] count += 3
once [ Button ] -> [ Button ] count *= 4
once [ Button ] -> [ Button ] count -= 5
once [ Button ] -> [ Button ] count /= 3
once [ Button ] -> [ Button ] count %= 4
once [ Button ] -> [ Button ] count = 9
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert_eq!(moved.visible_variables(), &[9]);
}

#[test]
fn none_query_is_first_class_boolean_guard() {
    let source = r#"
title = none_condition

puzzle default {
layers {
floor = Button
solid = Box Door OpenDoor
}
legend {
. = empty
B = Button
D = Door
O = OpenDoor
}

query no_pressed_buttons = none([ Button Box ])

rules {
if no_pressed_buttons {
once [ Door ] -> [ OpenDoor ]
}
}

levels {
level "start"
BD
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let open_door = object_named(&loaded, "OpenDoor");

    assert!(moved.has_object(&loaded.game, 1, 0, open_door));
}

#[test]
fn win_conditions_accept_exists_and_none_as_canonical_query_functions() {
    let source = r#"
title = canonical_condition_goal

puzzle default {
layers {
target = Goal
solid = Box
}
legend {
. = empty
* = Goal Box
G = Goal
}
win_conditions {
exists(Goal)
none([ Goal no Box ])
}
rules {

}
levels {
level "start"
*
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let goal = loaded.goal.as_ref().unwrap();

    assert_eq!(goal.description, "exists(Goal) and none([ Goal no Box ])");
    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn count_matches_is_no_longer_accepted() {
    let source = r#"
title = old_condition_name

puzzle default {
layers {
__legacy_layer_0 = Button
__legacy_layer_1 = Box
__legacy_layer_2 = Door
}
empty .

render_overlay Button Box X

query pressed_buttons = count_matches([ Button Box ])

rules {

}

level "start"
X
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown query function"));
}

#[test]
fn at_prefixed_routine_is_part_of_the_normal_game_program() {
    let source = r#"
title = display_split

puzzle default {
layers {
__legacy_layer_0 = Player
}


layers {
actor = Player
@marker = @Trail
}

legend {
. = empty
P = Player
t = @Trail
}

routine move once {
input directions [ Player | ] -> [ | Player ]
}

routine @paint once {
[ Player no @Trail ] -> [ Player @Trail ]
}

rules {
move
@paint
}

levels {
level "start"
P.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let player = object_named(&loaded, "Player");
    let trail = object_named(&loaded, "Trail");
    let right = input_named(&loaded, "right");
    let initial = &loaded.levels[0].initial_state;

    let played = transition_state(&loaded.game, initial, right).unwrap();
    assert!(played.has_object(&loaded.game, 1, 0, player));
    assert!(played.has_object(&loaded.game, 1, 0, trail));

    let solver_game = loaded.solver_game();
    let solved = transition_state(&solver_game, &loaded.solver_state(initial), right).unwrap();
    assert!(solved.has_object(&loaded.game, 1, 0, player));
    assert!(solved.has_object(&loaded.game, 1, 0, trail));

    let core_solved = transition_state(&solver_game, &loaded.solver_state(initial), right).unwrap();
    assert!(core_solved.has_object(&loaded.game, 1, 0, player));
    assert!(core_solved.has_object(&loaded.game, 1, 0, trail));
}

#[test]
fn at_prefixed_layer_objects_are_not_display_objects() {
    let source = r#"
title = unified_objects

puzzle default {

layers {
actor = Player
@marker = @Trail
}

levels {
legend {
. = empty
P = Player
}

level "start"
P
}

rules {

}
}
"#;
    let loaded = super::parse_game2d(source).unwrap();
    let player = object_named(&loaded, "Player");
    let trail = object_named(&loaded, "Trail");

    assert!(!loaded.is_display_object(player));
    assert!(!loaded.is_display_object(trail));
}

#[test]
fn on_display_is_removed() {
    let source = r#"
title = display_removed

puzzle default {
layers {
actor = Player
}

legend {
. = empty
P = Player
}

on_display {
[ Player ] -> [ Player ]
}

rules {

}

levels {
level "start"
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("`on_display` was removed"));
}

#[test]
fn at_prefixed_projection_object_is_a_normal_object() {
    let loaded = parse_game(display_floor_projection_source()).unwrap();
    let floor = object_named(&loaded, "Floor");

    assert!(!loaded.is_display_object(floor));
    assert!(loaded.display_program.is_none());
}

fn display_floor_projection_source() -> &'static str {
    r#"
title = display_floor_projection

puzzle default {
layers {
@display_floor = @Floor
target = Goal
}

legend {
. = empty
G = Goal
}

rules {

}

levels {
level "start" {
......
..G...
}
}
}
"#
}

#[test]
fn removed_on_display_rejects_input_dependent_display_rules_before_lowering() {
    let source = r#"
title = display_snapshot_input

puzzle default {
layers {
__legacy_layer_0 = Player
}


layers {
actor = Player
@marker = @Trail
}

legend {
. = empty
P = Player
}

routine @paint once {
input directions [ Player no @Trail | ] -> [ Player @Trail | ]
}

on_display {
@paint
}

rules {

}

levels {
level "start"
P.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("`on_display` was removed"));
}

#[test]
fn removed_on_display_rejects_main_statements_before_lowering() {
    let source = r#"
title = display_snapshot_main_statement

puzzle default {

layers {
actor = Player
}

legend {
. = empty
P = Player
}

on_display {
[ Player ] -> [ ]
}

rules {

}

levels {
level "start"
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("`on_display` was removed"));
}

#[test]
fn main_program_can_call_at_prefixed_routine() {
    let source = r#"
title = display_call_site_guard

puzzle default {
layers {
__legacy_layer_0 = Player
}


layers {
actor = Player
@marker = @Trail
}

legend {
. = empty
P = Player
}

routine @paint once {
[ Player no @Trail ] -> [ Player @Trail ]
}

rules {
@paint
}

levels {
level "start"
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let trail = object_named(&loaded, "Trail");
    let right = input_named(&loaded, "right");
    let initial = &loaded.levels[0].initial_state;

    let played = transition_state(&loaded.game, initial, right).unwrap();
    assert!(played.has_object(&loaded.game, 0, 0, trail));

    let solver_game = loaded.solver_game();
    let solved = transition_state(&solver_game, &loaded.solver_state(initial), right).unwrap();
    assert!(solved.has_object(&loaded.game, 0, 0, trail));
}

#[test]
fn at_prefixed_routine_can_write_any_normal_object() {
    let source = r#"
title = display_write_guard

puzzle default {
layers {
__legacy_layer_0 = Player
}


layers {
actor = Player
@marker = @Trail
}

legend {
. = empty
P = Player
}

rules {
@paint
}

routine @paint once {
[ Player ] -> [ ]
}

levels {
level "start"
P
}
}
"#;
    parse_game(source).unwrap();
}

#[test]
fn at_prefixed_object_match_can_change_other_objects() {
    let source = r#"
title = main_display_read_guard

puzzle default {
layers {
__legacy_layer_0 = Player
}


layers {
actor = Player
@marker = @Trail
}

legend {
. = empty
P = Player
}

rules {
[ @Trail ] -> [ @Trail Player ]
}

levels {
level "start"
P
}
}
"#;
    parse_game(source).unwrap();
}

#[test]
fn display_match_can_emit_sfx_without_rhs_block() {
    let source = r#"
title = display_match_sfx

puzzle default {
layers {
actor = Player
@ui = @Check
}

groups {
group = Player
}

legend {
. = empty
P = Player
}

rules {
[ @Check no group ] -> sfx x
[ @Check no group ] -> [ = ] sfx y
}

levels {
level "start"
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let effects = loaded
        .rule_effects
        .values()
        .flat_map(|effects| effects.iter())
        .collect::<Vec<_>>();

    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, RuleEffect::PlaySfx { name } if name == "x"))
    );
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, RuleEffect::PlaySfx { name } if name == "y"))
    );
}

#[test]
fn at_prefixed_object_match_can_emit_gameplay_effect_without_rhs_block() {
    let source = r#"
title = display_match_gameplay_effect_guard

puzzle default {
layers {
actor = Player
@ui = @Check
}

groups {
group = Player
}

legend {
. = empty
P = Player
}

rules {
[ @Check no group ] -> win
}

levels {
level "start"
P
}
}
"#;
    parse_game(source).unwrap();
}

#[test]
fn display_match_can_write_display_group_movement_mark() {
    let source = r#"
title = display_group_movement_mark

puzzle default {
tags {
kind = A B
}

layers {
@light = @LightBase @Light:kind
solid = Box:kind
}

legend {
. = empty
A = Box:A
}

rules {
input [ Box:A ] -> [ > Box:A ]
[ > Box:* @light ] -> [ > Box:* > @light ]
}

levels {
level "start"
A
}
}
"#;

    parse_game(source).unwrap();
}

#[test]
fn normal_routine_can_write_at_prefixed_object() {
    let source = r#"
title = bare_display_rule

puzzle default {
layers {
__legacy_layer_0 = Player
}


layers {
actor = Player
@marker = @Trail
}

legend {
. = empty
P = Player
}

routine paint once {
[ Player no @Trail ] -> [ Player @Trail ]
}

rules {
paint
}

levels {
level "start"
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let trail = object_named(&loaded, "Trail");
    let right = input_named(&loaded, "right");
    let initial = &loaded.levels[0].initial_state;

    let played = transition_state(&loaded.game, initial, right).unwrap();
    assert!(played.has_object(&loaded.game, 0, 0, trail));

    let solver_game = loaded.solver_game();
    let solved = transition_state(&solver_game, &loaded.solver_state(initial), right).unwrap();
    assert!(solved.has_object(&loaded.game, 0, 0, trail));
}

#[test]
fn normal_rule_can_write_at_prefixed_object() {
    let source = r#"
title = composite_display_effect

puzzle default {
layers {
__legacy_layer_0 = Player
}


layers {
actor = Player
@marker = @Trail
}

legend {
. = empty
P = Player
}

input right direction right

routine move once {
input right [ Player | ] -> [ | Player @Trail ]
}

rules {
move
}

levels {
level "start"
P.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let player = object_named(&loaded, "Player");
    let trail = object_named(&loaded, "Trail");
    let right = input_named(&loaded, "right");
    let initial = &loaded.levels[0].initial_state;

    let played = transition_state(&loaded.game, initial, right).unwrap();
    assert!(played.has_object(&loaded.game, 1, 0, player));
    assert!(played.has_object(&loaded.game, 1, 0, trail));

    let solver_game = loaded.solver_game();
    let solved = transition_state(&solver_game, &loaded.solver_state(initial), right).unwrap();
    assert!(solved.has_object(&loaded.game, 1, 0, player));
    assert!(solved.has_object(&loaded.game, 1, 0, trail));
}

#[test]
fn at_prefixed_routine_accepts_composite_normal_rule() {
    let source = r#"
title = display_routine_composite_guard

puzzle default {
layers {
__legacy_layer_0 = Player
}


layers {
actor = Player
@marker = @Trail
}

legend {
. = empty
P = Player
}

routine @paint once {
[ Player | ] -> [ | Player @Trail ]
}

rules {
@paint
}

levels {
level "start"
P.
}
}
"#;
    parse_game(source).unwrap();
}

#[test]
fn main_block_can_read_at_prefixed_objects_through_query_defs() {
    let source = r#"
title = main_display_condition_guard

puzzle default {
layers {
__legacy_layer_0 = Player
}


layers {
actor = Player
@marker = @Trail
}

legend {
. = empty
P = Player
}

query trail_count = count(@Trail)

rules {
if trail_count > 0 {
[ Player ] -> [ Player ]
}
}

levels {
level "start"
P
}
}
"#;
    parse_game(source).unwrap();
}

#[test]
fn non_canonical_display_aliases_are_rejected() {
    for header in [
        "main_objects",
        "main objects",
        "objects",
        "visual_objects",
        "visuals",
        "visual",
    ] {
        let source = format!(
            r#"
title = alias_rejection

puzzle default {{
{header} {{
Player
}}

rules {{
}}

levels {{
level "start"
.
}}
}}
"#
        );

        assert!(
            parse_game(&source).is_err(),
            "{header} should not be accepted as canonical syntax"
        );
    }
}

#[test]
fn legacy_display_keyword_syntax_is_rejected() {
    for legacy in [
        "layers {\nactor = Player\n@marker = @Trail\n}\nroutine display paint once {\n[ Player no @Trail ] -> [ Player @Trail ]\n}\nrules {\n}",
        "layers {\nactor = Player\n@marker = @Trail\n}\nroutine @paint once {\n[ Player no @Trail ] -> [ Player @Trail ]\n}\nrules {\ndisplay @paint\n}",
        "layers {\nactor = Player\n@marker = @Trail\n}\nrules {\ndisplay [ Player no @Trail ] -> [ Player @Trail ]\n}",
    ] {
        let source = format!(
            r#"
title = legacy_display_keyword_rejection

puzzle default {{
{legacy}

legend {{
. = empty
P = Player
}}

levels {{
level "start"
P
}}
}}
"#
        );

        assert!(
            parse_game(&source).is_err(),
            "{legacy} should not be accepted as canonical syntax"
        );
    }
}

#[test]
fn main_and_display_object_layers_keep_separate_storage_slots() {
    let source = r#"
title = mixed_layers

puzzle default {
layers {
floor = Floor
@floor_visual = @Shadow @Glow
actor = Player
}

legend {
. = empty
P = Floor Player
}

rules {

}

levels {
level "start"
P
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(
        loaded
            .game
            .object_layer(object_named(&loaded, "Floor"))
            .unwrap(),
        LayerId(0)
    );
    assert_eq!(
        loaded
            .game
            .object_layer(object_named(&loaded, "@Shadow"))
            .unwrap(),
        LayerId(1)
    );
    assert_eq!(
        loaded
            .game
            .object_layer(object_named(&loaded, "@Glow"))
            .unwrap(),
        LayerId(1)
    );
    assert_eq!(
        loaded
            .game
            .object_layer(object_named(&loaded, "Player"))
            .unwrap(),
        LayerId(2)
    );
}

#[test]
fn layer_can_mix_prefixed_and_unprefixed_objects() {
    let source = r#"
title = mixed_layer_rejected

puzzle default {
layers {
floor = Floor @Shadow
actor = Player
}

legend {
. = empty
P = Floor Player
}

rules {

}

levels {
level "start"
P
}
}
"#;
    parse_game(source).unwrap();
}

#[test]
fn display_groups_can_use_at_names() {
    let source = r#"
title = display_group

puzzle default {
layers {
floor = Floor
@marks = @Shadow @Glow
actor = Player
}

groups {
@paint = @Shadow @Glow
}

legend {
. = empty
P = Floor Player
}

rules {

}

levels {
level "start"
P
}
}
"#;

    parse_game(source).unwrap();
}

#[test]
fn at_prefixed_layer_name_can_contain_any_objects() {
    let source = r#"
title = display_layer_rejected

puzzle default {
layers {
@marks = Player @Shadow
}

legend {
. = empty
P = Player
}

rules {

}

levels {
level "start"
P
}
}
"#;
    parse_game(source).unwrap();
}

#[test]
fn unprefixed_layer_name_can_contain_at_prefixed_objects() {
    let source = r#"
title = main_layer_rejected

puzzle default {
layers {
marks = @Shadow
actor = Player
}

legend {
. = empty
P = Player
}

rules {

}

levels {
level "start"
P
}
}
"#;
    parse_game(source).unwrap();
}

#[test]
fn at_prefixed_group_name_can_contain_any_objects() {
    let source = r#"
title = display_group_rejected

puzzle default {
layers {
floor = Floor
@marks = @Shadow
actor = Player
}

groups {
@paint = Player @Shadow
}

legend {
. = empty
P = Floor Player
}

rules {

}

levels {
level "start"
P
}
}
"#;
    parse_game(source).unwrap();
}

#[test]
fn unprefixed_group_name_can_contain_at_prefixed_objects() {
    let source = r#"
title = main_group_rejected

puzzle default {
layers {
floor = Floor
@marks = @Shadow
actor = Player
}

groups {
paint = @Shadow
}

legend {
. = empty
P = Floor Player
}

rules {

}

levels {
level "start"
P
}
}
"#;
    parse_game(source).unwrap();
}

#[test]
fn each_layer_row_expands_selector_alternatives_to_ordered_layers() {
    let source = r#"
title = each_layers

puzzle default {
layers {
actor = Wall
each @Boundary:directions
each @Corner:directions
}

legend {
. = empty
# = Wall
}

rules {

}

levels {
level "start"
#
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(
        loaded
            .game
            .object_layer(object_named(&loaded, "@Boundary:up"))
            .unwrap(),
        LayerId(1)
    );
    assert_eq!(
        loaded
            .game
            .object_layer(object_named(&loaded, "@Boundary:down"))
            .unwrap(),
        LayerId(2)
    );
    assert_eq!(
        loaded
            .game
            .object_layer(object_named(&loaded, "@Corner:up"))
            .unwrap(),
        LayerId(5)
    );
}

#[test]
fn puzzle3_parser_is_available_through_lang_crate() {
    let parsed = crate::parse_puzzle3d(
        r#"
puzzle3 push3 {
  layers {
    floor = Floor
    actor = Player Box Wall
  }

  groups {
    solid = Player Box Wall
  }

  rules {

    input horizontal [ Player | Box | no solid ] -> [ | Player | Box ]
    input horizontal [ Player | no solid ] -> [ | Player ]
  }
}

levels3 demo of push3 {
  legend {
    . = empty
    P = Player
    B = Box
    # = Wall
  }

  level "start" {
    ####
    #PB#
    #..#
    ####
  }
}
"#,
    )
    .unwrap();

    assert_eq!(parsed.rules.len(), 8);
    assert_eq!(parsed.level_bundle.as_ref().unwrap().level_count(), 1);
    let fixture_json = crate::export_visual_fixture_json(&parsed).unwrap();
    let contract =
        puzzle_runtime_contract::puzzle3_runtime_model_from_fixture_json(&fixture_json).unwrap();
    assert_eq!(contract.rules.len(), parsed.rules.len());
    assert!(!fixture_json.contains("pushableObjectIds"));
    assert!(!fixture_json.contains("blocksMovement"));
}

#[test]
fn parse_game_returns_document_for_2d_model() {
    let document = super::parse_game(
        r#"
title = "Two Dee"
subtitle = "Flat puzzle"
author = Tester
homepage = "https://example.com/2d"

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .

rules {

}
}

levels {
legend P = Player
level "start" {
P
}
}
"#,
    )
    .unwrap();

    let Some(LoadedDocumentModel::Puzzle2d { name, game }) = document.single_model() else {
        panic!("expected one 2D puzzle model");
    };
    assert_eq!(document.title, "Two Dee");
    assert_eq!(document.subtitle.as_deref(), Some("Flat puzzle"));
    assert_eq!(document.author.as_deref(), Some("Tester"));
    assert_eq!(document.homepage.as_deref(), Some("https://example.com/2d"));
    assert!(
        matches!(document.scenes.as_slice(), [scene] if scene.name == "default")
            && matches!(
                document.scenes[0].state.puzzles.as_slice(),
                [puzzle] if puzzle.name == "default" && puzzle.model == "default"
            )
            && matches!(
                document.scenes[0].components.as_slice(),
                [SceneComponent::Frame(frame)] if frame.kind == "puzzle" && frame.source == "default"
            )
            && matches!(
                &document.scenes[0].puzzle_rule,
                Some(ScenePuzzleRule { target, rule }) if target == "default" && rule == "rules"
            )
    );
    assert_eq!(name, "default");
    assert_eq!(game.levels.len(), 1);
}

#[test]
fn parse_game2d_document_owns_scene_blocks() {
    let source = r#"
title = "Two Dee"

sounds {
  sfx push { seed = push01; type = hit }
}

puzzle default {
render {
  tween = true
  tween_duration = 30ms
}
layers {
__legacy_layer_0 = Player
}
empty .

rules {

}
}

levels {
legend P = Player
level "start" {
P
}
}

scene title {
  layout {
    title = "Two Dee"
  }
}
"#;

    let public_game = super::parse_game2d(source).unwrap();
    assert!(
        matches!(public_game.scenes.as_slice(), [title, default] if title.name == "title" && default.name == "default")
    );

    let parts = super::parse_document_source_parts(source).unwrap();
    assert!(matches!(parts.scenes.as_slice(), [scene] if scene.name == "title"));
    assert!(!parts.model_source.contains("scene title"));
    assert!(!parts.model_source.contains("title \"Two Dee\""));
    assert!(!parts.model_source.contains("sfx push"));
    let model_game =
        super::parse_game2d_expanded_lines_with_shell(parts.model_lines, &parts.shell).unwrap();
    assert!(model_game.scenes.is_empty());
}

#[test]
fn explicit_model_named_scene_overrides_implicit_scene_sugar() {
    let loaded = super::parse_game2d(
        r#"
title = explicit_scene_override

puzzle sokoban {
layers {
actor = Player
}
rules {
}
levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}

scene sokoban {
}
"#,
    )
    .unwrap();

    assert!(matches!(loaded.scenes.as_slice(), [scene] if scene.name == "sokoban"));
    let scene = &loaded.scenes[0];
    assert!(scene.state.puzzles.is_empty());
    assert!(scene.components.is_empty());
    assert!(scene.puzzle_rule.is_none());
}

#[test]
fn puzzle_model_layout_block_lowers_to_default_scene() {
    let document = super::parse_game(
        r#"
title = Inline Scene

puzzle sokoban {
layers {
actor = Player
}
rules {
}
layout {
text "Ready"
puzzle
}
}

levels {
legend {
. = empty
P = Player
}
level "first" {
P
}
}
"#,
    )
    .unwrap();

    assert!(matches!(
        document.scenes.as_slice(),
        [scene]
            if scene.name == "sokoban"
                && matches!(
                    scene.state.puzzles.as_slice(),
                    [puzzle] if puzzle.name == "sokoban" && puzzle.kind == "puzzle" && puzzle.model == "sokoban"
                )
                && matches!(
                    scene.components.as_slice(),
                    [SceneComponent::Text(text), SceneComponent::Frame(frame)]
                        if matches!(&text.content, SceneTextContent::Literal(value) if value == "Ready")
                            && frame.kind == "puzzle"
                            && frame.source == "sokoban"
                )
                && matches!(
                    &scene.puzzle_rule,
                    Some(ScenePuzzleRule { target, rule }) if target == "sokoban" && rule == "rules"
                )
    ));
}

#[test]
fn parse_game_returns_document_for_3d_model() {
    let document = super::parse_game(
        r#"
title = "Three Dee"
subtitle = "Cubic puzzle"
author = Tester
homepage = "https://example.com/3d"
default_wait_time = 100ms
again_interval = 80ms
sounds {
  sfx push { seed = push01; type = jump }
}
theme {
preset = "clean"
  accent_color = #ff0000
}
assets {
  "game.css"
}

puzzle3 push3 {
  layers {
    floor = Floor
    actor = Player Box Wall
  }

  groups {
    solid = Player Box Wall
  }

  rules {

    input horizontal [ Player | Box | no solid ] -> [ | Player | Box ]
    input horizontal [ Player | no solid ] -> [ | Player ]
  }
}

levels3 demo of push3 {
  legend {
    . = empty
    P = Player
    B = Box
    # = Wall
  }

  level "start" {
    ####
    #PB#
    #..#
    ####
  }
}

scene title {
  layout size 4 3 {
    title = "Three Dee"
    button "Play" -> goto push3(demo.start)
    button "Level Select" -> goto level_select
  }
}

scene level_select {
  layout {
    title = "Select Level"
    column scroll=true {
      for level in levels {
        button join(level.num, ". ", level.title) -> goto push3(level)
      }
    }
  }
}
"#,
    )
    .unwrap();

    let Some(LoadedDocumentModel::Puzzle3d { name, puzzle }) = document.single_model() else {
        panic!("expected one 3D puzzle model");
    };
    assert_eq!(document.title, "Three Dee");
    assert_eq!(document.subtitle.as_deref(), Some("Cubic puzzle"));
    assert_eq!(document.author.as_deref(), Some("Tester"));
    assert_eq!(document.homepage.as_deref(), Some("https://example.com/3d"));
    assert_eq!(document.default_wait_ms, 100);
    assert_eq!(document.default_again_ms, 80);
    assert_eq!(document.sounds.sfx[0].name, "push");
    assert_eq!(document.theme.name.as_deref(), Some("clean"));
    assert_eq!(document.assets.entries[0].path, "game.css");
    assert!(matches!(
        document.scenes.as_slice(),
        [title, level_select, push3]
            if title.name == "title"
                && title.layout.size.unwrap().width == 4
                && title.layout.size.unwrap().height == 3
                && level_select.name == "level_select"
                && push3.name == "push3"
                && matches!(push3.state.puzzles.as_slice(), [puzzle] if puzzle.name == "push3" && puzzle.kind == "puzzle3" && puzzle.model == "push3")
                && matches!(push3.components.as_slice(), [SceneComponent::Frame(frame)] if frame.kind == "puzzle3" && frame.source == "push3")
                && matches!(&push3.puzzle_rule, Some(ScenePuzzleRule { target, rule }) if target == "push3" && rule == "rules")
    ));
    assert_eq!(name, "push3");
    assert_eq!(puzzle.rules.len(), 8);
    assert_eq!(puzzle.level_bundle.as_ref().unwrap().level_count(), 1);
    let fixture_json = crate::export_loaded_document_visual_fixture_json(&document).unwrap();
    assert!(fixture_json.contains("\"title\": \"Three Dee\""));
    assert!(fixture_json.contains("\"theme\": {\"name\":\"clean\""));
    assert!(fixture_json.contains("\"variables\":[{\"name\":\"accent\",\"value\":\"#ff0000\"}]"));
    assert!(fixture_json.contains("\"currentScene\": \"title\""));
    assert!(fixture_json.contains("\"layout\": {"));
    assert!(fixture_json.contains("\"width\": 4"));
    assert!(!fixture_json.contains("\"kind\": \"for\""));
    assert!(fixture_json.contains("\"kind\": \"button\""));
    assert!(fixture_json.contains("\"scroll\": true"));
    assert!(!fixture_json.contains("\"kind\": \"level_menu\""));
}

#[test]
fn parse_game_accepts_3d_input_rule_without_orientation_set() {
    let document = super::parse_game(
        r#"
title = "Bare 3D Input"

puzzle3 push3 {
  layers {
    actor = Player
  }

  rules {
    input [ Player ] -> [ > Player ]
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

    let Some(LoadedDocumentModel::Puzzle3d { puzzle, .. }) = document.single_model() else {
        panic!("expected one 3D puzzle model");
    };
    assert_eq!(puzzle.rules.len(), 6);
}

#[test]
fn theme_preset_statement_before_puzzle3_does_not_capture_model_block() {
    let document = super::parse_game(
        r#"
title = "Themed 3D"
theme = "puzzlescript"

puzzle3 push3 {
  layers {
    actor = Player
  }
  rules {
  }
}

levels3 demo of push3 {
  legend {
    P = Player
  }
  level "start" {
    P
  }
}
"#,
    )
    .unwrap();

    assert_eq!(document.theme.name.as_deref(), Some("puzzlescript"));
    assert!(matches!(
        document.models.as_slice(),
        [LoadedDocumentModel::Puzzle3d { name, puzzle }]
            if name == "push3" && puzzle.level_bundle.as_ref().unwrap().level_count() == 1
    ));
}

#[test]
fn document_runtime_sources_keep_model_sources_separate_from_document_scenes() {
    let source = r#"
title = "Mixed Runtime"
theme = "puzzlescript"

puzzle flat {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
}
}

levels flat_levels of flat {
legend P = Player
level "start" {
P
}
}

puzzle3 cube {
layers {
actor = Player
}
rules {
}
}

levels3 cube_levels of cube {
legend {
P = Player
}
level "start" {
P
}
}

scene mixed_play {
layout {
row {
puzzle flat_board = flat
puzzle3 cube_board = cube
}
}
}
"#;

    let sources = super::split_document_runtime_sources(source).unwrap();

    assert!(sources.model_2d.contains("puzzle flat"));
    assert!(sources.model_2d.contains("levels flat_levels"));
    assert!(!sources.model_2d.contains("scene mixed_play"));
    assert!(!sources.model_2d.contains("puzzle3 cube_board"));
    assert!(!sources.model_2d.contains("puzzle3 cube {"));
    assert!(!sources.model_2d.contains("levels3 cube_levels"));
    assert!(sources.model_3d.contains("puzzle3 cube {"));
    assert!(sources.model_3d.contains("levels3 cube_levels"));
    assert!(!sources.model_3d.contains("puzzle flat"));
    assert!(!sources.model_3d.contains("scene mixed_play"));
    assert!(!sources.model_3d.contains("theme = \"puzzlescript\""));
}

#[test]
fn puzzle3_model_layout_block_lowers_to_default_scene() {
    let document = super::parse_game(
        r#"
title = Inline Scene 3D

puzzle3 push3 {
layers {
actor = Player
}
rules {
}
layout {
text "Ready"
puzzle3
}
}

levels3 demo of push3 {
legend {
P = Player
}
level "first" {
P
}
}
"#,
    )
    .unwrap();

    assert!(matches!(
        document.scenes.as_slice(),
        [scene]
            if scene.name == "push3"
                && matches!(
                    scene.state.puzzles.as_slice(),
                    [puzzle] if puzzle.name == "push3" && puzzle.kind == "puzzle3" && puzzle.model == "push3"
                )
                && matches!(
                    scene.components.as_slice(),
                    [SceneComponent::Text(text), SceneComponent::Frame(frame)]
                        if matches!(&text.content, SceneTextContent::Literal(value) if value == "Ready")
                            && frame.kind == "puzzle3"
                            && frame.source == "push3"
                )
                && matches!(
                    &scene.puzzle_rule,
                    Some(ScenePuzzleRule { target, rule }) if target == "push3" && rule == "rules"
                )
    ));
    assert!(matches!(
        &document.models[0],
        LoadedDocumentModel::Puzzle3d { name, puzzle }
            if name == "push3" && puzzle.level_bundle.as_ref().unwrap().level_count() == 1
    ));
}

#[test]
fn spec_3d_exports_playable_puzzle_scene() {
    let document =
        super::parse_game(include_str!("../tests/fixtures/spec_3d_full.puzzle3")).unwrap();
    let fixture_json = crate::export_loaded_document_visual_fixture_json(&document).unwrap();

    assert!(fixture_json.contains("\"currentScene\": \"title\""));
    assert!(fixture_json.contains("\"name\": \"sokoban\""));
    assert!(fixture_json.contains("\"slot\": \"sokoban\""));
    assert!(fixture_json.contains("\"model\": \"sokoban\""));
    assert!(fixture_json.contains("\"kind\": \"puzzle3\""));
    assert!(fixture_json.contains("\"source\": \"sokoban\""));
    assert!(!fixture_json.contains("\"kind\": \"for\""));
    assert!(fixture_json.contains("\"kind\": \"button\""));
    assert!(fixture_json.contains("\"scroll\": true"));
    assert!(fixture_json.contains("\"levels\": [0, 1, 2]"));
    assert!(!fixture_json.contains("\"kind\": \"level_menu\""));
}

#[test]
fn puzzle3_lifecycle_diagnostic_uses_shared_source_line_mapping() {
    let source = r#"title = = "Line Probe"

puzzle3 lifecycle {
layers {
actor = Player
}

on_last_level_clear {
messag "END"
}
}
"#;
    let report = super::parse_game(source).unwrap_err();
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message.contains("effect must be"))
        .expect("shared scene effect diagnostic");
    let expected_line = source
        .lines()
        .position(|line| line.trim() == r#"messag "END""#)
        .map(|line| line + 1);

    assert_eq!(
        diagnostic
            .primary_span
            .as_ref()
            .and_then(|span| span.source_line.as_deref()),
        Some(r#"messag "END""#)
    );
    assert_eq!(
        diagnostic.primary_span.as_ref().and_then(|span| span.line),
        expected_line
    );
}

#[test]
fn scene_state_implicit_puzzle_slots_resolve_against_model_kind() {
    let flat_document = super::parse_game(
        r#"
title = Implicit Flat Slot

puzzle flat {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
}
}

levels flat_levels of flat {
legend P = Player
level "start" {
P
}
}

scene flat_play {
layout {
flat
}
}
"#,
    )
    .unwrap();

    let flat_play = flat_document
        .scenes
        .iter()
        .find(|scene| scene.name == "flat_play")
        .unwrap();
    assert!(matches!(
        flat_play.state.puzzles.as_slice(),
        [flat]
            if flat.name == "flat"
                && flat.kind == "puzzle"
                && flat.model == "flat"
    ));

    let cube_document = super::parse_game(
        r#"
title = Implicit Cube Slot

puzzle3 cube {
layers {
actor = Player
}
rules {
}
}

levels3 cube_levels of cube {
legend {
P = Player
}
level "start" {
P
}
}

scene cube_play {
layout {
cube
}
}
"#,
    )
    .unwrap();

    let cube_play = cube_document
        .scenes
        .iter()
        .find(|scene| scene.name == "cube_play")
        .unwrap();
    assert!(matches!(
        cube_play.state.puzzles.as_slice(),
        [cube]
            if cube.name == "cube"
                && cube.kind == "puzzle3"
                && cube.model == "cube"
    ));
    assert!(cube_document.scenes.iter().any(|scene| {
        scene.name == "cube"
            && matches!(
                scene.state.puzzles.as_slice(),
                [puzzle] if puzzle.name == "cube" && puzzle.kind == "puzzle3" && puzzle.model == "cube"
            )
            && matches!(
                scene.components.as_slice(),
                [SceneComponent::Frame(frame)] if frame.kind == "puzzle3" && frame.source == "cube"
            )
    }));
}

#[test]
fn puzzle3_level_menu_fixture_uses_goto_level_action_not_start_levels() {
    let document = super::parse_game(
        r#"
title = Level Menu 3D

puzzle3 demo {
layers {
  floor = Floor
  actor = Player
}

rules {
}
}

scene title {
  layout {
    title = "Level Menu 3D"
    button "Levels" -> goto level_select
  }
}

scene level_select {
  layout {
    level_menu
  }
}

scene playing {
  layout {
    puzzle3 board = demo
  }
}

levels3 test of demo {
legend {
  . = empty
  , = Floor
  P = Player
}

level "first" {
P

,
}
}
"#,
    )
    .unwrap();
    let fixture_json = crate::export_loaded_document_visual_fixture_json(&document).unwrap();

    assert!(fixture_json.contains("\"kind\": \"level_menu\""));
    assert!(fixture_json.contains("\"kind\": \"goto\""));
    assert!(fixture_json.contains("\"scene\": \"playing\""));
    assert!(fixture_json.contains("\"kind\": \"level\""));
    assert!(fixture_json.contains("\"path\": \"level\""));
    assert!(!fixture_json.contains("start_levels"));
}

#[test]
fn puzzle3_fixture_serializes_shared_scene_effects() {
    let document = super::parse_game(
        r#"
title = Shared Effects 3D

puzzle3 demo {
layers {
  actor = Player
}

rules {
}
}

scene title {
  layout {
    button "Inspect" -> sfx click wait 100ms goto title
  }
}

levels3 test of demo {
legend {
  P = Player
}

level "first" {
P
}
}
"#,
    )
    .unwrap();
    let fixture_json = crate::export_loaded_document_visual_fixture_json(&document).unwrap();

    assert!(fixture_json.contains("\"kind\": \"button\""));
    assert!(fixture_json.contains("\"effect\":"));
    assert!(fixture_json.contains("\"kind\": \"sequence\""));
    assert!(fixture_json.contains("\"kind\": \"play_sfx\""));
    assert!(fixture_json.contains("\"name\": \"click\""));
    assert!(fixture_json.contains("\"kind\": \"wait\""));
    assert!(fixture_json.contains("\"milliseconds\": 100"));
    assert!(fixture_json.contains("\"kind\": \"goto\""));
    assert!(!fixture_json.contains("\"action\""));
}

#[test]
fn parse_game_rejects_old_model_prefix_for_2d_puzzles() {
    let error = super::parse_game(
        r#"
title = Old Model Prefix

model puzzle default {
layers {
__legacy_layer_0 = Player
}

rules {

}
}
"#,
    )
    .unwrap_err()
    .to_string();

    assert!(
        error.contains("top-level puzzle definition must be: puzzle <name>"),
        "{error}"
    );
}

#[test]
fn puzzle3_parser_rejects_old_model_prefix() {
    let error = crate::parse_puzzle3d(
        r#"
model puzzle3 push3 {
  layers {
    actor = Player
  }

  rules {
  }
}
"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        ParseError3::Message(message)
            if message.contains("top-level 3D puzzle definition must be: puzzle3 <name>")
    ));
}

#[test]
fn parse_game_rejects_mixed_2d_and_3d_models() {
    let error = super::parse_game(
        r#"
title = Mixed Game

puzzle flat {
layers {
__legacy_layer_0 = Player
}
empty .
rules {

}
}

levels flat_levels of flat {
legend P = Player
level "start" {
P
}
}

puzzle3 cube {
  layers {
    actor = Player Box Wall
  }

  groups {
    solid = Player Box Wall
  }

  rules {

  }
}

levels3 cube_levels of cube {
  legend {
    . = empty
    P = Player
  }

  level "start" {
    P
  }
}

scene mixed_play {
  layout {
    row {
      puzzle flat_board = flat
      puzzle3 cube_board = cube
    }
  }
}
"#,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("mixed 2D/3D documents are no longer supported"));
}

#[test]
fn removed_command_directive_is_not_accepted_as_input_compatibility() {
    let error = super::parse_game2d(
        r#"
title = removed_command_directive
puzzle board {
command jump
rules {
}
}
"#,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("unknown puzzle directive command"));
}

#[test]
fn music_effect_in_puzzle_statement_lowers_to_rule_effect() {
    let loaded = super::parse_game2d(
        r#"
title = music_effect_in_puzzle_statement

puzzle main {
layers {
base = Player
}
rules {
stop_music locked_room
}
levels {
legend {
. = empty
P = Player
}
level "start"
P
}
}
"#,
    )
    .unwrap();

    assert!(
        loaded
            .rule_effects
            .values()
            .any(|effects| effects.iter().any(|effect| matches!(
                effect,
                RuleEffect::StopMusic { name } if name.as_deref() == Some("locked_room")
            )))
    );
}

#[test]
fn parse_game_reports_sibling_unknown_routine_calls() {
    let report = super::parse_game2d(
        r#"
title = "Multi Error Probe"

puzzle main {
layers {
base = Floor
}

sprites {
}

rules {
unknown_statement_one
unknown_statement_two
}

levels {
legend {
. = empty
}
level "first"
.
}
}
"#,
    )
    .unwrap_err();
    let diagnostics = report.diagnostics();

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(
        diagnostics[0].message,
        "unknown routine call: unknown_statement_one"
    );
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .and_then(|span| span.source_line.as_deref()),
        Some("unknown_statement_one")
    );
    assert_eq!(
        diagnostics[1].message,
        "unknown routine call: unknown_statement_two"
    );
    assert_eq!(
        diagnostics[1]
            .primary_span
            .as_ref()
            .and_then(|span| span.source_line.as_deref()),
        Some("unknown_statement_two")
    );
}

#[test]
fn diagnostic_source_location_resolves_split_structural_line() {
    let source = r#"title = probe

puzzle main {
layers {
base = Floor
}

rules {
}

on_last_level_clear {r
}

levels {
legend {
. = empty
}
level "first"
.
}
}
"#;
    let report = super::parse_game2d(source).unwrap_err();
    let diagnostics = report.diagnostics();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message, "unknown routine call: r");
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .and_then(|span| span.source_line.as_deref()),
        Some("r")
    );
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .and_then(|span| span.line),
        Some(11)
    );
}

#[test]
fn split_rewrite_selector_diagnostic_keeps_source_line_and_dedupes_calls() {
    let source = r#"title = split_selector_diagnostic

puzzle main {
layers {
actor = Box Crate
}
groups {
solid = Box Crate
}
rules {
bad
bad
}
routine bad {
[ Box ]
-> [ solid ]
}
levels {
legend {
. = empty
B = Box
}
level "first"
B
}
}
"#;
    let report = super::parse_game2d(source).unwrap_err();
    let diagnostics = report.diagnostics();
    let expected_line = source
        .lines()
        .position(|line| line.trim() == "[ Box ]")
        .map(|line| line + 1);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message,
        "after selector with alternatives must also appear in before"
    );
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .and_then(|span| span.line),
        expected_line
    );
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .and_then(|span| span.source_line.as_deref()),
        Some("[ Box ] -> [ solid ]")
    );
}

#[test]
fn diagnostic_source_location_uses_statement_line_for_duplicate_lines() {
    let source = r#"title = probe

puzzle main {
layers {
base = Floor
}

rules {
missing
missing
}

levels {
legend {
. = empty
}
level "first"
.
}
}
"#;
    let report = super::parse_game2d(source).unwrap_err();
    let diagnostics = report.diagnostics();
    let expected_lines = source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| (line.trim() == "missing").then_some(index + 1))
        .collect::<Vec<_>>();

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.primary_span.as_ref().and_then(|span| span.line))
            .collect::<Vec<_>>(),
        expected_lines.into_iter().map(Some).collect::<Vec<_>>()
    );
}

#[test]
fn statement_parse_diagnostic_carries_source_line_number() {
    let source = r#"title = probe

puzzle main {
layers {
base = Floor
}

rules {
action jump
}

levels {
legend {
. = empty
}
level "first"
.
}
}
"#;
    let report = super::parse_game2d(source).unwrap_err();
    let diagnostic = report.diagnostics().first().expect("statement diagnostic");
    let expected_line = source
        .lines()
        .position(|line| line.trim() == "action jump")
        .map(|line| line + 1);

    assert_eq!(
        diagnostic.message,
        "`action` statements were removed; use explicit input guards and rewrites"
    );
    assert_eq!(
        diagnostic
            .primary_span
            .as_ref()
            .and_then(|span| span.source_line.as_deref()),
        Some("action jump")
    );
    assert_eq!(
        diagnostic.primary_span.as_ref().and_then(|span| span.line),
        expected_line
    );
}

#[test]
fn parser_boundary_resolves_source_line_only_diagnostic_line_number() {
    let source = r#"title = probe

unknown_top_level

puzzle main {
layers {
base = Floor
}

rules {
}

levels {
legend {
. = empty
}
level "first"
.
}
}
"#;
    let report = super::parse_game2d(source).unwrap_err();
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message.contains("unknown"))
        .expect("unknown top-level diagnostic");
    let expected_line = source
        .lines()
        .position(|line| line.trim() == "unknown_top_level")
        .map(|line| line + 1);

    assert_eq!(
        diagnostic
            .primary_span
            .as_ref()
            .and_then(|span| span.source_line.as_deref()),
        Some("unknown_top_level")
    );
    assert_eq!(
        diagnostic.primary_span.as_ref().and_then(|span| span.line),
        expected_line
    );
}
