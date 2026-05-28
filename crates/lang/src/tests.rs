use super::*;
use puzzle_core::{
    LocalFrameExtent, RuleStep, transition_program, transition_solver_state, transition_state,
};

fn parse_game(source: &str) -> Result<LoadedGame, AppError> {
    super::parse_game2d(&modernize_test_source(source))
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
    let mut display_aliases = Vec::<(String, String)>::new();
    let mut pending_level_legend = Vec::<String>::new();
    while i < lines.len() {
        let rewritten_line = rewrite_test_display_aliases(&lines[i], &display_aliases);
        let line = &rewritten_line;
        let tokens = split_tokens(line);
        let in_scene = scene_depth > 0;
        let in_levels = levels_depth > 0;

        if in_levels {
            out.push(line.clone());
            i += 1;
            if line == "end" {
                levels_depth = levels_depth.saturating_sub(1);
            } else if matches!(tokens.as_slice(), ["legend"] | ["level", .., "{"] | ["{"]) {
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
        if in_scene && line == "end" {
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
            ["objects"] | ["display_objects"] => {
                let display = tokens[0] == "display_objects";
                let (modern, aliases, next_i) = modernize_test_objects_block(
                    &lines,
                    i,
                    display,
                    test_puzzle_has_following_layers(&lines, i),
                );
                for line in modern {
                    if line.starts_with("legend ") {
                        pending_level_legend.push(line);
                    } else {
                        out.push(line);
                    }
                }
                display_aliases.extend(aliases);
                i = next_i;
                continue;
            }
            ["puzzle", name] if !in_scene && is_identifier(name) => {
                out.push(format!("puzzle {name}"));
            }
            ["render_overlay", rest @ ..] if rest.len() >= 3 => {
                let ch = rest[rest.len() - 1];
                let objects = &rest[..rest.len() - 1];
                pending_level_legend.push(format!("legend {ch} = {}", objects.join(" ")));
            }
            ["legend"] | ["legend", ..] if !in_scene => {
                let (legend, next_i) = collect_test_legend_entry(&lines, i, &display_aliases);
                pending_level_legend.extend(legend);
                i = next_i;
                continue;
            }
            ["transition"] if in_scene => {
                scene_depth += 1;
                out.push("rules".to_string());
            }
            ["levels", ..] => {
                levels_depth = 1;
                out.push(line.clone());
                out.append(&mut pending_level_legend);
            }
            ["level", name, ..] => {
                out.push("levels".to_string());
                out.append(&mut pending_level_legend);
                let braced_level = tokens.contains(&"{");
                if braced_level {
                    out.push(line.clone());
                } else {
                    out.push(format!("level {name}"));
                }
                i += 1;
                while i < lines.len() && lines[i] != "end" {
                    out.push(lines[i].clone());
                    i += 1;
                }
                out.push("end".to_string());
                if braced_level {
                    out.push("end".to_string());
                }
            }
            _ => out.push(line.clone()),
        }
        if in_scene
            && !matches!(tokens.as_slice(), ["keys"] | ["transition"])
            && test_starts_block(&tokens)
        {
            scene_depth += 1;
        }
        i += 1;
    }
    out.join("\n")
}

fn collect_test_legend_entry(
    lines: &[String],
    start: usize,
    aliases: &[(String, String)],
) -> (Vec<String>, usize) {
    if lines[start] == "legend" {
        let mut out = vec!["legend".to_string()];
        let mut i = start + 1;
        while i < lines.len() {
            out.push(rewrite_test_display_aliases(&lines[i], aliases));
            if lines[i] == "end" {
                return (out, i + 1);
            }
            i += 1;
        }
        return (out, i);
    }
    (
        vec![rewrite_test_display_aliases(&lines[start], aliases)],
        start + 1,
    )
}

fn test_puzzle_has_following_layers(lines: &[String], start: usize) -> bool {
    let mut i = start + 1;
    let mut depth = 1usize;
    while i < lines.len() && depth > 0 {
        let tokens = split_tokens(&lines[i]);
        if matches!(
            tokens.as_slice(),
            ["objects"]
                | ["display_objects"]
                | ["legend"]
                | ["group"]
                | ["sprites"]
                | ["rules"]
                | ["levels", ..]
        ) {
            depth += 1;
        } else if lines[i] == "end" {
            depth = depth.saturating_sub(1);
        }
        i += 1;
    }
    while i < lines.len() {
        let tokens = split_tokens(&lines[i]);
        match tokens.as_slice() {
            ["layers"] => return true,
            ["rules"]
            | ["levels", ..]
            | ["level", ..]
            | ["scene", ..]
            | ["model", ..]
            | ["puzzle", ..] => return false,
            _ => i += 1,
        }
    }
    false
}

fn modernize_test_objects_block(
    lines: &[String],
    start: usize,
    display: bool,
    declarations_only: bool,
) -> (Vec<String>, Vec<(String, String)>, usize) {
    let mut object_rows = Vec::<String>::new();
    let mut layer_rows = Vec::<String>::new();
    let mut legend_rows = Vec::<String>::new();
    let mut group_rows = Vec::<String>::new();
    let mut aliases = Vec::<(String, String)>::new();
    let mut i = start + 1;
    let mut layer_index = 0usize;

    while i < lines.len() && lines[i] != "end" {
        let tokens = split_tokens(&lines[i]);
        match tokens.as_slice() {
            ["layer"] | ["layer", _] => {
                let explicit_name = tokens.get(1).copied().map(str::to_string);
                i += 1;
                let mut selectors = Vec::<String>::new();
                while i < lines.len() && lines[i] != "end" {
                    for (selector, legend) in modernize_test_object_row(&lines[i]) {
                        if display {
                            push_test_display_alias(&selector, &mut aliases);
                        }
                        selectors.push(modernize_test_layer_term(&selector, display));
                        if let Some(ch) = legend {
                            legend_rows.push(format!(
                                "legend {ch} = {}",
                                modernize_test_selector_ref(&selector, display)
                            ));
                        }
                    }
                    i += 1;
                }
                let name = explicit_name.unwrap_or_else(|| format!("__test_layer_{layer_index}"));
                layer_rows.push(format!("{name} = {}", selectors.join(" ")));
                layer_index += 1;
                if i < lines.len() && lines[i] == "end" {
                    i += 1;
                }
            }
            ["group"] => {
                i += 1;
                while i < lines.len() && lines[i] != "end" {
                    group_rows.push(lines[i].clone());
                    i += 1;
                }
                if i < lines.len() && lines[i] == "end" {
                    i += 1;
                }
            }
            ["group", ..] => {
                group_rows.push(tokens[1..].join(" "));
                i += 1;
            }
            _ => {
                if tokens.len() == 2 && tokens[1].chars().all(|ch| ch.is_ascii_digit()) {
                    let selector = tokens[0].to_string();
                    if display {
                        push_test_display_alias(&selector, &mut aliases);
                    }
                    if declarations_only {
                        object_rows.push(format!(
                            "object {} {}",
                            modernize_test_selector_ref(&selector, display),
                            tokens[1]
                        ));
                    } else {
                        layer_rows.push(format!(
                            "__test_layer_{} = {}",
                            tokens[1],
                            modernize_test_layer_term(&selector, display)
                        ));
                    }
                    i += 1;
                    continue;
                }
                let mut selectors = Vec::<String>::new();
                for (selector, legend) in modernize_test_object_row(&lines[i]) {
                    if display {
                        push_test_display_alias(&selector, &mut aliases);
                    }
                    if declarations_only {
                        let spec = modernize_test_selector_ref(&selector, display);
                        object_rows.push(format!("object {spec} 0"));
                    } else {
                        selectors.push(modernize_test_layer_term(&selector, display));
                    }
                    if let Some(ch) = legend {
                        legend_rows.push(format!(
                            "legend {ch} = {}",
                            modernize_test_selector_ref(&selector, display)
                        ));
                    }
                }
                if !selectors.is_empty() && !declarations_only {
                    layer_rows.push(format!(
                        "__test_layer_{layer_index} = {}",
                        selectors.join(" ")
                    ));
                    layer_index += 1;
                }
                i += 1;
            }
        }
    }

    let mut out = object_rows;
    if !layer_rows.is_empty() {
        out.push("layers".to_string());
        out.extend(layer_rows);
        out.push("end".to_string());
    }
    if !legend_rows.is_empty() {
        out.extend(legend_rows);
    }
    if !group_rows.is_empty() {
        out.push("group".to_string());
        out.extend(group_rows);
        out.push("end".to_string());
    }
    (out, aliases, i + 1)
}

fn modernize_test_object_row(line: &str) -> Vec<(String, Option<String>)> {
    let tokens = split_tokens(line);
    if tokens.len() == 2 && tokens[0].contains(':') && tokens[1].chars().count() > 1 {
        let family = tokens[0]
            .split_once(':')
            .map_or(tokens[0], |(family, _)| family);
        let mut rows = vec![(tokens[0].to_string(), None)];
        rows.extend(tokens[1].chars().map(|ch| {
            let value = ch.to_ascii_uppercase();
            (format!("{family}:{value}"), Some(ch.to_string()))
        }));
        return rows;
    }
    if tokens.len() == 2
        && tokens[1].chars().count() == 1
        && !tokens[1].chars().all(|ch| ch.is_ascii_digit())
    {
        return vec![(tokens[0].to_string(), Some(tokens[1].to_string()))];
    }
    tokens
        .into_iter()
        .map(|token| (token.to_string(), None))
        .collect()
}

fn modernize_test_layer_term(selector: &str, display: bool) -> String {
    if display && !selector.starts_with('@') {
        format!("display @{selector}")
    } else {
        selector.to_string()
    }
}

fn modernize_test_selector_ref(selector: &str, display: bool) -> String {
    if display && !selector.starts_with('@') {
        format!("@{selector}")
    } else {
        selector.to_string()
    }
}

fn push_test_display_alias(selector: &str, aliases: &mut Vec<(String, String)>) {
    if selector.starts_with('@') || selector.contains(':') {
        return;
    }
    let replacement = format!("@{selector}");
    if !aliases.iter().any(|(from, _)| from == selector) {
        aliases.push((selector.to_string(), replacement));
    }
}

fn rewrite_test_display_aliases(line: &str, aliases: &[(String, String)]) -> String {
    let tokens = split_tokens(line);
    if tokens.is_empty() || aliases.is_empty() {
        return line.to_string();
    }
    let mut rewritten = line.to_string();
    for (from, to) in aliases {
        for (prefix, suffix) in [
            (" ", " "),
            ("[ ", " "),
            (" ", " ]"),
            ("| ", " "),
            (" ", " |"),
            ("= ", ""),
            ("(", ")"),
            ("(", ""),
            ("", ")"),
            ("", "\n"),
        ] {
            rewritten = rewritten.replace(
                &format!("{prefix}{from}{suffix}"),
                &format!("{prefix}{to}{suffix}"),
            );
        }
        if rewritten == *from {
            rewritten = to.clone();
        }
    }
    rewritten
}

fn modern_scene_header(tokens: &[&str]) -> Option<String> {
    match tokens {
        ["scene" | "screen", "puzzle", name] => Some(format!("scene {name}")),
        ["scene" | "screen", "puzzle"] => Some("scene puzzle".to_string()),
        ["scene" | "screen", "level_menu", name] => Some(format!("scene {name}")),
        ["scene" | "screen", "level_menu"] => Some("scene level_menu".to_string()),
        ["scene" | "screen", "title_menu", name] => Some(format!("scene {name}")),
        ["scene" | "screen", "title_menu"] => Some("scene title_menu".to_string()),
        ["scene" | "screen", "menu", name] => Some(format!("scene {name}")),
        ["scene" | "screen", "menu"] => Some("scene menu".to_string()),
        ["scene" | "screen", name] => Some(format!("scene {name}")),
        _ => None,
    }
}

fn test_starts_block(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["layers"]
            | ["rules"]
            | ["transitions"]
            | ["objects"]
            | ["layer", ..]
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
            | ["on_display"]
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

#[test]
fn rules_local_frame_limits_main_transition_matching_to_player_frame() {
    let loaded = parse_game(
        r#"
title local frame
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
level one {
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
title local radius
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
level one {
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
fn section_headers_parse_existing_block_names() {
    let source = r#"
title section_header

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
level start {
*
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let goal = object_named(&loaded, "Goal");
    let box_object = object_named(&loaded, "Box");
    let initial = &loaded.levels[0].initial_state;

    assert!(initial.has_object(&loaded.game, 0, 0, goal));
    assert!(initial.has_object(&loaded.game, 0, 0, box_object));
}

#[test]
fn levels_section_header_preserves_unbraced_level_separators() {
    let source = r#"
title section_header_levels

puzzle board {
======
LAYERS
======
floor = Goal
actor = Player

=======
LEGENDS
=======
. = empty
P = Player
G = Goal

=====
RULES
=====
once [ Player ] -> [ Player ]

======
LEVELS
======
level first
P

level second
G
}
"#;

    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.levels.len(), 2);
    assert_eq!(loaded.levels[0].name, "first");
    assert_eq!(loaded.levels[1].name, "second");
}

#[test]
fn at_display_objects_and_rules_share_object_layers() {
    let source = r#"
title at_display

puzzle board {
layers {
actor = Player
cursor = @Cursor
hint = display @Hint
}

legend {
. = empty
P = Player
}

routine @paint once {
[ Player no @Cursor no display @Hint ] -> [ Player @Cursor display @Hint ]
}

on_display {
@paint
}

rules {
display @paint
}

levels {
level start
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let cursor = object_named(&loaded, "@Cursor");
    let hint = object_named(&loaded, "@Hint");
    let initial = &loaded.levels[0].initial_state;

    assert!(loaded.game.is_visual_object(cursor));
    assert!(loaded.game.is_visual_object(hint));
    assert!(!initial.has_object(&loaded.game, 0, 0, cursor));

    let displayed = transition_program(
        &loaded.game,
        loaded.display_program.as_deref().unwrap(),
        initial,
        InputId(0),
    )
    .unwrap();
    assert!(displayed.has_object(&loaded.game, 0, 0, cursor));
    assert!(displayed.has_object(&loaded.game, 0, 0, hint));
}

#[test]
fn legend_does_not_define_unknown_objects() {
    let source = r#"
title legend_unknown

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
level start
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
title level_legend_unknown

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
level start {
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
fn top_level_sounds_keeps_only_seed_and_settings() {
    let source = r#"
title sounds_game

sounds {
  sfx effect seed=746670 type=jump
  music loop seed=123456 tone=0.62 bpm=104 volume=0.8
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
level one
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.sounds.sfx.len(), 1);
    assert_eq!(loaded.sounds.sfx[0].name, "effect");
    assert_eq!(loaded.sounds.sfx[0].seed, "746670");
    assert_eq!(loaded.sounds.sfx[0].type_target, "jump");
    assert_eq!(loaded.sounds.music.len(), 1);
    assert_eq!(loaded.sounds.music[0].name, "loop");
    assert_eq!(loaded.sounds.music[0].seed, "123456");
    assert_eq!(loaded.sounds.music[0].tone, 0.62);
    assert_eq!(loaded.sounds.music[0].bpm, 104);
    assert_eq!(loaded.sounds.music[0].volume, 0.8);
}

#[test]
fn top_level_audio_block_is_rejected() {
    let source = r#"
title old_sounds_keyword

audio {
  sfx effect seed=746670 type=jump
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("found audio"), "{error}");
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn scene_effects_can_trigger_sounds() {
    let source = r#"
title sounds_effects

sounds {
  sfx click seed=746670 type=jump
  music loop seed=123456
}

scene title_menu {
title game.title
start "Play" -> start_game
action start_game -> {
sfx click
play_music loop
goto playing
}
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
level one
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let title = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == "title")
        .unwrap();
    let start_transition = title
        .transitions
        .iter()
        .find(|transition| {
            matches!(
                &transition.trigger,
                SceneTransitionTrigger::Condition(condition) if condition == "input == start"
            )
        })
        .unwrap();
    let SceneEffect::Input(action) = &start_transition.effect else {
        panic!("expected start transition to emit an input");
    };
    assert_eq!(action, "start_game");

    let start_game_transition = title
        .transitions
        .iter()
        .find(|transition| {
            matches!(
                &transition.trigger,
                SceneTransitionTrigger::Condition(condition) if condition == "input == start_game"
            )
        })
        .unwrap();
    let SceneEffect::Sequence(effects) = &start_game_transition.effect else {
        panic!("expected sounds start effect to lower to a sequence");
    };
    assert!(matches!(
        &effects[0],
        SceneEffect::PlaySfx { name } if name == "click"
    ));
    assert!(matches!(
        &effects[1],
        SceneEffect::PlayMusic { name } if name == "loop"
    ));
    assert!(matches!(
        &effects[2],
        SceneEffect::Goto { scene, .. } if scene == "playing"
    ));
}

#[test]
fn scene_lifecycle_blocks_lower_to_lifecycle_transitions() {
    let source = r#"
title scene_lifecycle_blocks

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
rules {

}
level start {
P
}
}

scene playing {
view {
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
fn scene_message_effect_parses_literal_and_path() {
    let source = r#"
title scene_message_effect
var hint = "Push the box"

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
rules {

}
level start {
P
}
}

scene playing {
view {
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
    let SceneEffect::Sequence(effects) = &scene_start.effect else {
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
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn scene_effect_wait_duration_parses_to_milliseconds() {
    let source = r#"
title scene_wait_effect

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
rules {

}
levels {
P
}
}

scene title {
view {
button "Start" -> input start
}
rules {
start ->
wait
wait 0.1s
wait 1s
wait 25ms
goto playing
end
}
}

scene playing {
view {
text "Playing"
}
}

default_wait_time = 500ms
"#;

    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.default_wait_ms, 500);
    let title = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == "title")
        .unwrap();
    let transition = title
        .transitions
        .iter()
        .find(|transition| {
            matches!(
                &transition.trigger,
                SceneTransitionTrigger::Condition(condition) if condition == "input == start"
            )
        })
        .unwrap();
    let SceneEffect::Sequence(effects) = &transition.effect else {
        panic!("expected wait effects to lower to a sequence");
    };
    assert!(matches!(
        &effects[0],
        SceneEffect::Wait { milliseconds } if *milliseconds == Some(500)
    ));
    assert!(matches!(
        &effects[1],
        SceneEffect::Wait { milliseconds } if *milliseconds == Some(100)
    ));
    assert!(matches!(
        &effects[2],
        SceneEffect::Wait { milliseconds } if *milliseconds == Some(1000)
    ));
    assert!(matches!(
        &effects[3],
        SceneEffect::Wait { milliseconds } if *milliseconds == Some(25)
    ));
    assert!(matches!(
        &effects[4],
        SceneEffect::Goto { scene, .. } if scene == "playing"
    ));
}

#[test]
fn again_interval_parses_to_default_again_milliseconds() {
    let loaded = parse_game(
        r#"
title again_interval_fixture
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
level first
P
}
}
"#,
    )
    .unwrap();

    assert_eq!(loaded.default_again_ms, 75);
}

#[test]
fn puzzlescript_style_again_interval_parses_as_seconds() {
    let loaded = parse_game(
        r#"
title again_interval_seconds_fixture
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
level first
P
}
}
"#,
    )
    .unwrap();

    assert_eq!(loaded.default_again_ms, 100);
}

#[test]
fn top_level_animation_tween_parses_to_game_settings() {
    let loaded = parse_game(
        r#"
title tween_fixture
animation {
tween duration=90ms
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
level first
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
fn animation_tween_rejects_old_enabled_assignment() {
    let source = r#"
title tween_fixture
animation {
tween {
enabled = true
}
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
level first
P
}
}
"#;

    assert!(parse_game(source).is_err());
}

#[test]
fn animation_tween_adds_move_animation_emission_without_sounds() {
    let loaded = parse_game(
        r#"
title tween_emission_fixture
animation {
tween {
duration = 80ms
}
}

puzzle main {
layers {
actor = Player
}
rules {
input directions [ Player ] -> [ Player{>} ]
move
}
levels {
legend {
. = empty
P = Player
}
level first
P.
}
}
"#,
    )
    .unwrap();

    assert!(loaded.rule_emissions.values().any(|emissions| {
        emissions.iter().any(|emission| {
            matches!(
                emission,
                RuleEmission::Animate {
                    trigger: RuleAnimationTrigger::Move,
                    name,
                    objects,
                } if name == "tween" && !objects.is_empty()
            )
        })
    }));
}

#[test]
fn scene_on_level_start_is_rejected() {
    let source = r#"
title scene_level_lifecycle

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
rules {

}
level start {
P
}
}

scene playing {
view {
text "Playing"
}
on_level_start {
message "no"
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("on_level_start belongs inside puzzle"));
}

#[test]
fn scene_current_level_syntax_is_rejected() {
    let source = r#"
title current_level_syntax

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
rules {

}
level start {
P
}
}

scene playing {
view {
board = puzzle current_level
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("current_level is not scene syntax"));
}

#[test]
fn puzzle_presentation_message_parses_literal_and_path() {
    let source = r#"
title rewrite_message_effect

puzzle default {
objects {
layer {
Player P
}
layer {
Goal G
}
}
legend {
. = empty
P = Player
G = Goal
}
rules {

[ Player Goal ] -> message "Found"
[ Player ] -> message hint
}
level start {
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let emissions = loaded
        .rule_emissions
        .values()
        .flat_map(|emissions| emissions.iter())
        .collect::<Vec<_>>();
    assert!(emissions.iter().any(|emission| matches!(
        emission,
        RuleEmission::Message { text, literal: true } if text == "Found"
    )));
    assert!(emissions.iter().any(|emission| matches!(
        emission,
        RuleEmission::Message { text, literal: false } if text == "hint"
    )));
}

#[test]
fn puzzle_presentation_effect_parses_commands() {
    let source = r#"
title puzzle_presentation_effect
default_wait_time = 350ms

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
rules {

[ Player ] -> sfx pushed
wait
wait 25ms
}
level start {
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let emissions = loaded
        .rule_emissions
        .values()
        .flat_map(|emissions| emissions.iter())
        .collect::<Vec<_>>();
    assert!(emissions.iter().any(|emission| {
        matches!(emission, RuleEmission::PlaySfx { name } if name == "pushed")
    }));
    assert!(emissions.iter().any(|emission| {
        matches!(emission, RuleEmission::Wait { milliseconds } if *milliseconds == 350)
    }));
    assert!(emissions.iter().any(|emission| {
        matches!(emission, RuleEmission::Wait { milliseconds } if *milliseconds == 25)
    }));
}

#[test]
fn puzzle_emit_is_rejected() {
    let source = r#"
title puzzle_emit_rejected

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
rules {

emit sfx tick
}
level start {
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
title puzzle_emit_rejects_state_mutation

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
var moved = false
rules {

emit set moved = true
}
level start {
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
title do_statement_rejected

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
rules {

do sfx tick
}
level start {
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
title routine_effect_statements

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
var ready = false
routine ready_feedback once {
sfx tick
message "Ready"
set ready = true
}
rules {

ready_feedback
ready_feedback
}
level start {
P
}
}
"#;

    let loaded = parse_game(source).unwrap();
    let emissions = loaded
        .rule_emissions
        .values()
        .flat_map(|emissions| emissions.iter())
        .collect::<Vec<_>>();
    assert!(
        emissions
            .iter()
            .filter(|emission| matches!(emission, RuleEmission::PlaySfx { name } if name == "tick"))
            .count()
            >= 2
    );
    assert!(
        emissions
            .iter()
            .filter(|emission| matches!(emission, RuleEmission::Message { text, literal: true } if text == "Ready"))
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
            .filter(|effect| matches!(effect, puzzle_core::Effect::UpdateGlobal { .. }))
            .count()
            >= 2
    );
}

#[test]
fn effect_definition_is_rejected() {
    let source = r#"
title effect_definition_rejected

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
effect feedback {
sfx tick
}
level start {
P
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("effect definitions are obsolete"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn scene_lifecycle_arrows_are_rejected() {
    let source = r#"
title lifecycle_arrow

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
rules {

}
level start {
P
}
}

scene playing {
view {
text "Playing"
}
rules {
level_start -> play_music music_name
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("level_start is a puzzle lifecycle block"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn then_effect_sequences_are_rejected() {
    let source = r#"
title no_then

puzzle board {
objects {
  layers {
  background = Player
  }
}

legend {
. = empty
P = Player
}

rules {

}

levels {
level one
P
}
}

scene playing {
rules {
done -> sfx click then goto playing
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("`then` effect sequences are not supported"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn scene_puzzle_can_own_board_model_and_view() {
    let source = r#"
title scene_puzzle

scene puzzle playing {
layers {
actor = Player
}

legend {
. = empty
P = Player
}

win_conditions {
some Player
}

rules {
once right [ Player | no Player ] -> [ | Player ]
}

levels {
level start
P.
}

view {
board = puzzle playing
puzzle board
}

rules {

if input == right -> {
update board
}
}

if win_conditions {
goto level_clear
}
}

scene level_clear {
view {
text "clear"
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let playing = &loaded.scenes[0];
    assert_eq!(playing.name, "playing");
    assert_eq!(playing.state.puzzles[0].name, "board");
    assert_eq!(playing.transitions.len(), 2);

    let SceneTransitionTrigger::Condition(action) = &playing.transitions[0].trigger else {
        panic!("expected input rule to lower to a condition transition");
    };
    assert_eq!(action, "input == right");
    let SceneEffect::Apply { rule, target, .. } = &playing.transitions[0].effect else {
        panic!("expected update board to lower to apply effect");
    };
    assert_eq!(rule, "right");
    assert_eq!(target.as_deref(), Some("board"));

    let SceneTransitionTrigger::Condition(condition) = &playing.transitions[1].trigger else {
        panic!("expected if block to lower to a condition transition");
    };
    assert_eq!(condition, "board.win_conditions");
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn scene_puzzle_uses_explicit_puzzle_slot_as_primary() {
    let source = r#"
title scene_puzzle_custom_slot

scene puzzle playing {
view {
playfield = puzzle playing
}

layers {
actor = Player
}

legend {
. = empty
P = Player
}

win_conditions {
some Player
}

rules {
once right [ Player | no Player ] -> [ | Player ]
}

levels {
level start
P.
}

rules {

if input == right -> {
update playfield
}
}

if win_conditions {
goto level_clear
}
}

scene level_clear {
view {
text "clear"
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let playing = &loaded.scenes[0];
    assert_eq!(playing.state.puzzles.len(), 1);
    assert_eq!(playing.state.puzzles[0].name, "playfield");

    let SceneEffect::Apply { target, .. } = &playing.transitions[0].effect else {
        panic!("expected input handler to apply to explicit puzzle slot");
    };
    assert_eq!(target.as_deref(), Some("playfield"));

    let SceneTransitionTrigger::Condition(condition) = &playing.transitions[1].trigger else {
        panic!("expected unqualified condition to target explicit puzzle slot");
    };
    assert_eq!(condition, "playfield.win_conditions");
}

#[test]
fn scene_input_handler_requires_arrow_block_syntax() {
    let source = r#"
title old_scene_input_handler

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
level start {
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
title old_using_scene

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
level start {
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
fn scene_key_command_assignment_can_feed_input_rule() {
    let source = r#"
title scene_key_command_assignment

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
level start {
P
}
}

scene playing {
inputs {
escape <- q
}
rules {

if input == escape -> {
back
}
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    let SceneEffect::Input(action) = &scene.key_bindings[0].effect else {
        panic!("expected key assignment to emit an input");
    };
    assert_eq!(action, "escape");
    let SceneTransitionTrigger::Condition(condition) = &scene.transitions[0].trigger else {
        panic!("expected input rule to lower to condition transition");
    };
    assert_eq!(condition, "input == escape");
    assert!(matches!(scene.transitions[0].effect, SceneEffect::Back));
}

#[test]
fn scene_rules_accept_input_trigger_sugar() {
    let source = r#"
title input_sugar

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
level start {
P
}
}

scene title {
inputs {
level_select <- q
}
rules {
level_select -> goto level_select
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    let SceneTransitionTrigger::Condition(condition) = &scene.transitions[0].trigger else {
        panic!("expected input sugar to lower to condition transition");
    };
    assert_eq!(condition, "input == level_select");
    assert!(matches!(
        &scene.transitions[0].effect,
        SceneEffect::Goto { scene, .. } if scene == "level_select"
    ));
}

#[test]
fn scene_keys_accept_arrow_to_input_or_effect() {
    let source = r#"
title keys_arrow

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
level start {
P
}
}

scene title {
keys {
q -> level_select
Escape -> goto pause
}
rules {
level_select -> goto level_select
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert!(matches!(
        &scene.key_bindings[0].effect,
        SceneEffect::Input(input) if input == "level_select"
    ));
    assert!(matches!(
        &scene.key_bindings[1].effect,
        SceneEffect::Goto { scene, .. } if scene == "pause"
    ));
}

#[test]
fn scene_keys_reject_equals_assignment() {
    let source = r#"
title keys_equals

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
level start {
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
    assert!(error.contains("keys row must use `->`"));
}

#[test]
fn scene_keys_accept_multiple_keys_per_row() {
    let source = r#"
title keys_multiple

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
level start {
P
}
}

scene title {
keys {
q Escape -> level_select
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert_eq!(scene.key_bindings[0].keys.len(), 2);
    assert!(matches!(
        &scene.key_bindings[0].effect,
        SceneEffect::Input(input) if input == "level_select"
    ));
}

#[test]
fn bare_scene_title_and_subtitle_inherit_game_metadata() {
    let source = r#"
title "Display Title"
subtitle "Display Subtitle"

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
level start {
P
}
}

scene title {
view {
title
subtitle
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert!(matches!(
        &scene.components[0],
        SceneComponent::Title(title)
            if title.content == SceneExpr::Path(vec!["game".to_string(), "title".to_string()])
    ));
    assert!(matches!(
        &scene.components[1],
        SceneComponent::Subtitle(subtitle)
            if subtitle.content == SceneExpr::Path(vec!["game".to_string(), "subtitle".to_string()])
    ));
}

#[test]
fn scene_can_use_model_name_as_default_puzzle_slot() {
    let source = r#"
title default_slot

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
level start {
P
}
}

scene playing {
state {
puzzle sokoban
}
view {
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
fn scene_rules_reject_component_rules_path() {
    let source = r#"
title old_component_rules_path

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
level start {
P
}
}

scene playing {
state {
board = puzzle board
}
view {
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
title frame_slot

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
level start
P
}
}

scene playing {
state {
board = puzzle board
}
view {
frame board
}
}
"#;
    let loaded = parse_game(source).unwrap();
    assert!(matches!(
        &loaded.scenes[0].components[0],
        SceneComponent::Frame(frame) if frame.kind == "frame" && frame.source == "board"
    ));
}

#[test]
fn scene_can_still_name_multiple_puzzle_slots_explicitly() {
    let source = r#"
title named_slots

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
level start {
P
}
}

scene playing {
state {
sokoban1 = puzzle sokoban
sokoban2 = puzzle sokoban
}
view {
sokoban1
sokoban2
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
title command_direction

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
level start {
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
title old_scene_inputs

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
level start {
P
}
}

scene playing {
inputs {
resume = Escape
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("inputs row must be: <input> <- <key...>"));
}

#[test]
fn button_action_assignment_uses_equals() {
    let source = r#"
title button_action_assignment

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
level start {
P
}
}

scene menu {
view {
button "Resume" -> input resume
}
rules {

resume -> {
back
}
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
title scene_box_layout

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
level start {
P
}
}

scene menu {
view size 4 3 {
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
        error.to_string().contains("unknown view directive panel"),
        "expected panel to be rejected, got {error}"
    );
}

#[test]
fn explicit_scene_input_and_component_effect_parse_separately() {
    let source = r#"
title explicit_scene_input_effects

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
level start {
P
}
input right
}

scene playing {
state {
board = puzzle board
}
inputs {
right <- ArrowRight
down <- ArrowDown
restart <- r
}
rules {
down -> component_effect down
restart -> board.restart
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let scene = &loaded.scenes[0];
    assert!(matches!(&scene.key_bindings[0].effect, SceneEffect::Input(input) if input == "right"));
    assert!(
        matches!(&scene.transitions[0].effect, SceneEffect::ComponentEffect(effect) if effect == "down")
    );
    assert!(matches!(
        &scene.transitions[1].effect,
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
title scene_effect_wrapper

puzzle board {
objects {
layer {
Player P
}
}
legend {
. = empty
P = Player
}
rules {

[ Player ] -> [ Player ]
}
level start {
P
}
}

scene playing {
state {
board = puzzle board
}
button "Restart" -> board.restart
}
"#;

    let loaded = parse_game(source).unwrap();
    assert!(matches!(
        &loaded.scenes[0].components[0],
        SceneComponent::Button(button)
            if matches!(&button.effect, SceneEffect::ResetPuzzle { target } if target == "board")
    ));
}

#[test]
fn button_arrow_rejects_plain_action_rhs() {
    let source = r#"
title old_button_action_arrow

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
level start {
P
}
}

scene menu {
view {
button "Resume" -> resume
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("bare scene command aliases were removed"));
}

#[test]
fn view_for_can_project_levels_into_scrollable_column() {
    let source = r#"
title level_projection_view

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
level first {
P
}
level second {
P
}
}
}

scene level_select {
view {
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

    let SceneComponent::For(for_view) = &column.children[0] else {
        panic!("expected level projection");
    };
    assert!(matches!(for_view.source, ForSource::Levels));

    let SceneComponent::Button(button) = &for_view.children[0] else {
        panic!("expected level button");
    };
    assert!(matches!(&button.label, SceneExpr::Call { name, args }
            if name == "join"
                && args.iter().any(|arg| matches!(arg, SceneExpr::Path(path) if path == &vec!["level".to_string(), "solved".to_string()]))));
    assert!(matches!(
        &button.effect,
        SceneEffect::Goto { scene, params }
            if scene == "playing"
                && params.len() == 1
                && params[0].name == "level"
                && matches!(&params[0].value, SceneExpr::Path(path) if path == &vec!["level".to_string()])
    ));
}

#[test]
fn typed_level_menu_scene_accepts_canonical_options() {
    let source = r#"
title typed_level_menu

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
level start {
P
}
}

scene level_menu {
show_index = true
show_solved = true
columns = 4
wrap = true
button "Back" -> back
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
    assert!(matches!(&menu.buttons[0].effect, SceneEffect::Back));
}

#[test]
fn level_menu_rejects_on_off_option_aliases() {
    let source = r#"
title old_level_menu_options

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
level start {
P
}
}

scene level_menu {
index on
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown scene directive index"));
}

#[test]
fn level_menu_rejects_inline_source_and_effect() {
    let source = r#"
title old_level_menu_inline

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
level start {
P
}
}

scene level_select {
view {
level_menu microban -> goto playing(level)
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("level_menu takes no inline source or effect"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn typed_scenes_can_omit_default_names() {
    let source = r#"
title anonymous_typed_scenes

scene puzzle {
objects {
layers {
actor = Player
}
}

legend {
. = empty
P = Player
}

rules {
once right [ Player | no Player ] -> [ | Player ]
}

levels {
level start
P.
}
}

scene level_menu {
show_index = true
columns = 4
wrap = true
button "Back" -> back
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.scenes[0].name, "playing");
    assert_eq!(loaded.scenes[0].state.puzzles[0].name, "playing");
    assert!(matches!(
        &loaded.scenes[0].components[0],
        SceneComponent::Frame(frame) if frame.kind == "puzzle" && frame.source == "playing"
    ));

    let level_select = &loaded.scenes[1];
    assert_eq!(level_select.name, "level_select");
    let SceneComponent::LevelMenu(menu) = &level_select.components[0] else {
        panic!("expected level_menu scene to create a level menu component");
    };
    assert!(menu.show_index);
    assert_eq!(menu.columns, Some(4));
    assert!(menu.wrap);
    assert_eq!(menu.buttons.len(), 1);
}

#[test]
fn title_scene_keeps_buttons_and_rules_explicit() {
    let source = r#"
title title_menu_scene

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

level start {
P.
}
}

scene title {
title
subtitle "A tiny puzzle"
button "Play" -> goto playing
button "Levels" -> goto level_select
}

scene playing {
view {
board = puzzle default
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
title Tiny Metadata Game
subtitle "Small Metadata Puzzle"
author "Puzzle Person"
homepage "https://example.com/puzzle"

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

level start {
P
}
}

scene title {
title game.title
subtitle game.subtitle
text game.author
text game.homepage
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

level start {
P
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("top-level `name` metadata was removed; use `title <text>`"));
}

#[test]
fn occurrence_scratch_supports_multiple_marks_direction_and_int_values() {
    let source = r#"
title scratch_marks

puzzle default {
layers 2
empty .

scratch {
checked
move = directions
count = int
}

object Box 1
object Marker 0
legend B = Box

rules {
once right [ Box ] -> [ Box{checked move=> count=3} ]
once right [ Box{checked move=> count=3} no Marker ] -> [ Box{no checked no move count=2} Marker ]
}

level start {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_scratch().iter().all(Vec::is_empty));
    assert!(moved.cell_scratch().iter().all(Vec::is_empty));
}

#[test]
fn bool_scratch_uses_presence_and_no_syntax() {
    let source = r#"
title bool_scratch

puzzle default {
layers 2
empty .

scratch {
flag = bool
}

object Box 1
object Marker 0
legend B = Box

rules {
once [ Box ] -> [ Box{flag} ]
once [ Box{flag} no Marker ] -> [ Box{no flag} Marker ]
}

level start {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_scratch().iter().all(Vec::is_empty));
}

#[test]
fn colon_scratch_name_does_not_mean_value_assignment() {
    let source = r#"
title scratch_colon

puzzle default {
layers 1
empty .

scratch {
count = int
}

object Box 0
legend B = Box

rules {
once [ Box ] -> [ Box{count:3} ]
}

level start {
B
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown scratch"));
}

#[test]
fn scratch_names_can_use_numeric_colon_parts() {
    let source = r#"
title numeric_qualified_scratch

puzzle default {
layers 2
empty .

scratch {
count:3
}

object Box 1
object Marker 0
legend B = Box

rules {
once [ Box ] -> [ Box{count:3} ]
once [ Box{count:3} no Marker ] -> [ Box Marker ]
}

level start {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_scratch().iter().all(Vec::is_empty));
}

#[test]
fn qualified_scratch_names_can_use_colons() {
    let source = r#"
title qualified_scratch

puzzle default {
layers 2
empty .

scratch {
enter:directions = bool
intent:move = directions
}

object Box 1
object Marker 0
legend B = Box

rules {
once [ Box ] -> [ Box{enter:directions intent:move=right} ]
once [ Box{enter:directions intent:move=right} no Marker ] -> [ Box Marker ]
}

level start {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_scratch().iter().all(Vec::is_empty));
}

#[test]
fn unmentioned_occurrence_scratch_is_preserved_when_same_occurrence_moves() {
    let source = r#"
title moving_scratch

puzzle default {
layers 2
empty .

scratch {
hot
}

object Box 1
object Marker 0
legend B = Box

rules {
once [ Box ] -> [ Box{hot} ]
once right [ Box | ] -> [ | Box ]
once [ Box{hot} no Marker ] -> [ Box Marker ]
}

level start {
B.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 1, 0, marker));
    assert!(moved.slot_scratch().iter().all(Vec::is_empty));
}

#[test]
fn omitted_rhs_scratch_removes_explicit_lhs_scratch_on_moved_occurrence() {
    let source = r#"
title moving_scratch_remove

puzzle default {
layers 2
empty .

scratch {
hot
}

object Box 1
legend B = Box

rules {
once [ Box ] -> [ Box{hot} ]
once right [ Box{hot} | ] -> [ | Box ]
}

level start {
B.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let box_object = object_named(&loaded, "Box");
    assert!(moved.has_object(&loaded.game, 1, 0, box_object));
    assert!(!moved.has_scratch_key(&loaded.game, 1, 0, box_object, puzzle_core::ScratchId(3),));
}

#[test]
fn same_cell_occurrence_is_preserved_before_move_inference() {
    let source = r#"
title same_cell_preserve

puzzle default {
layers 2
empty .

scratch {
hot
}

object Box 1
object Marker 0
legend B = Box

rules {
once [ Box ] -> [ Box{hot} ]
once right [ Box | no Box ] -> [ Box | Box ]
once [ Box{hot} | Box no Marker ] -> [ Box{hot} | Box Marker ]
}

level start {
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
    assert!(moved.slot_scratch().iter().all(Vec::is_empty));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn duplicate_rhs_object_can_add_without_moving_preserved_source() {
    let source = r#"
title duplicate_rhs_add

puzzle default {
layers 2
empty .

kind = A

object Target:kind 0
object Safe:kind 1
object Wall 0
legend t = Target:A

level_start {
mark_safe
}

routine mark_safe once {
repeat [ Safe ] -> []
repeat [ Target:A no Safe:A ] -> [ Target:A Safe:A ]
repeat {
[ no Wall | no Wall no Safe:A | Safe:A ] -> [ | Safe:A | Safe:A ]
}
}

rules {

}

level start {
..t
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let safe = object_named(&loaded, "Safe:A");
    let initial = &loaded.levels[0].initial_state;

    assert!(!initial.has_object(&loaded.game, 0, 0, safe));
    assert!(initial.has_object(&loaded.game, 1, 0, safe));
    assert!(initial.has_object(&loaded.game, 2, 0, safe));
}

#[test]
fn group_selectors_accept_scratch_blocks() {
    let source = r#"
title group_scratch

puzzle default {
layers 2
empty .

scratch {
hot
}

object Box 1
object Crate 1
object Marker 0
group mover = Box Crate
legend B = Box

rules {
once [ Box ] -> [ Box{hot} ]
once [ mover{hot} no Marker ] -> [ mover{hot} Marker ]
}

level start {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_scratch().iter().all(Vec::is_empty));
}

#[test]
fn cell_and_occurrence_scratch_share_names_but_have_distinct_anchors() {
    let source = r#"
title cell_scratch

puzzle default {
layers 2
empty .

scratch {
mark
}

object Box 1
object Marker 0
legend B = Box

rules {
once [ Box ] -> [ Box{mark} ]
once [ Box{mark} ] -> [ Box {mark} ]
once [ Box {mark} no Marker ] -> [ Box Marker ]
once [ Box{mark} {mark} ] -> [ Box ]
}

level start {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_scratch().iter().all(Vec::is_empty));
    assert!(moved.cell_scratch().iter().all(Vec::is_empty));
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
title rhs_layer_conflict

puzzle default {
layers 2
empty .

object Player 0
object Box 0
legend P = Player

rules {
[ Player ] -> [ Player Box ]
}

level start {
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
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn scratch_is_transition_local_and_does_not_need_clear() {
    let source = r#"
title transition_scratch

puzzle default {
layers 2
empty .

scratch {
checked
hot
}

object Box 1
object Marker 0
legend B = Box

input mark m arrow_right
rules {
if input == mark {
once [ Box no Marker ] -> [ Box{checked} Marker{hot} ]
once [ Box{checked} Marker{hot} ] -> [ Box Marker ]
}
}

level start {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let mark = *loaded.controls.keys.get(&b'm').unwrap();
    let marker = object_named(&loaded, "Marker");

    let marked = transition_state(&loaded.game, &loaded.levels[0].initial_state, mark).unwrap();
    assert!(marked.has_object(&loaded.game, 0, 0, marker));
    assert!(marked.slot_scratch().iter().all(Vec::is_empty));
    assert!(marked.cell_scratch().iter().all(Vec::is_empty));

    let unchanged = transition_state(&loaded.game, &marked, mark).unwrap();
    assert_eq!(unchanged, marked);
}

#[test]
fn movement_scratch_prefix_and_legacy_inline_sugar_work_with_transition_local_lifetime() {
    let source = r#"
title anonymous_scratch

puzzle default {
layers 2
empty .

scratch {
checked
}

object Box 1
object Marker 0
legend B = Box

rules {
once right [ Box ] -> [ Box{> checked 7} ]
once right [ > Box{checked 7} no Marker ] -> [ Box Marker ]
once right [ Box Marker ] -> [ 3 Box Marker ]
once right [ 3 Box Marker ] -> [ true Box Marker ]
once right [ true Box Marker ] -> [ false Box Marker ]
once right [ false Box Marker ] -> [ Box ]
}

level start {
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
    assert!(moved.slot_scratch().iter().all(Vec::is_empty));
}

#[test]
fn action_statement_is_rejected() {
    let source = r#"
title action_button

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
level start {
PT
}
}

scene playing {
state {
board = puzzle board
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
fn standard_move_clears_blocked_movement_before_later_rules() {
    let source = r#"
title blocked_move_cleanup

puzzle default {
layers {
floor = Marker
actor = Box Wall
}

legend {
B = Box
W = Wall
. = empty
}

rules {

once right [ Box ] -> [ > Box ]
move
once [ > Box ] -> [ Box Marker ]
}

levels {
level start
BW
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
fn standard_move_resolves_movement_intent_one_cell_per_call() {
    let source = r#"
title standard_move_one_cell

puzzle default {
layers {
actor = Box
}

legend {
B = Box
. = empty
}

rules {

once right [ Box ] -> [ > Box ]
move
}

levels {
level start
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

    assert!(!moved.has_object(&loaded.game, 0, 0, box_object));
    assert!(moved.has_object(&loaded.game, 1, 0, box_object));
    assert!(!moved.has_object(&loaded.game, 2, 0, box_object));
}

#[test]
fn standard_move_moves_same_direction_chains_one_cell() {
    let source = r#"
title standard_move_chain

puzzle default {
layers {
actor = Box
}

legend {
B = Box
. = empty
}

rules {

once_all right [ Box ] -> [ > Box ]
move
}

levels {
level start
BB..
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

    assert!(!moved.has_object(&loaded.game, 0, 0, box_object));
    assert!(moved.has_object(&loaded.game, 1, 0, box_object));
    assert!(moved.has_object(&loaded.game, 2, 0, box_object));
    assert!(!moved.has_object(&loaded.game, 3, 0, box_object));
}

#[test]
fn directions_scratch_sugar_matches_any_movement_value() {
    let source = r#"
title directions_sugar

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
level start
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
fn no_directions_scratch_sugar_forbids_any_movement_value() {
    let source = r#"
title no_directions_sugar

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
level start
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
fn parallel_and_perpendicular_scratch_sets_expand_relative_to_rule_orientation() {
    let source = r#"
title relative_movement_sets

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
level start
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
fn parallel_scratch_prefix_sugar_matches_object_movement_set() {
    let source = r#"
title parallel_prefix_sugar

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
level start
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
fn prefixless_parallel_scratch_pattern_expands_cardinal_directions() {
    let source = r#"
title prefixless_parallel

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
level start
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
fn variant_axis_values_can_define_scratch_without_becoming_value_sets() {
    let source = r#"
title variant_scratch

puzzle default {
layers 2
empty .

tags {
color = red blue
}

scratch {
color
paint = color
}

object Box 1
object Marker 0
legend B = Box

rules {
once [ Box ] -> [ Box{color paint=blue} ]
once [ Box{color paint=blue} no Marker ] -> [ Box Marker ]
}

level start {
B
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(moved.has_object(&loaded.game, 0, 0, marker));
    assert!(moved.slot_scratch().iter().all(Vec::is_empty));
}

#[test]
fn level_start_keeps_raw_initial_state_and_keeps_runtime_program() {
    let source = r#"
title level_start

puzzle default {
layers 2
empty .

object Source 0
object Marker 1
legend S = Source

on_level_start {
[ Source no Marker ] -> [ Source Marker ]
}

rules {

}

level start {
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
fn level_start_rejects_input_dependent_rules() {
    let source = r#"
title level_start_input

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

on_level_start {
input directions [ Player | ] -> [ | Player ]
}

rules {

}

level start {
P.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("on_level_start cannot depend on input"));
}

#[test]
fn display_level_start_keeps_raw_initial_state_and_keeps_runtime_program() {
    let source = r#"
title display_level_start

puzzle default {
layers 2
empty .

objects {
Source 0
}

display_objects {
Marker 1
}

legend S = Source

routine display mark_initial once {
[ Source no Marker ] -> [ Source Marker ]
}

on_level_start {
display mark_initial
}

rules {

}

level start {
S
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let marker = object_named(&loaded, "Marker");

    assert!(loaded.game.is_visual_object(marker));
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
title display_level_start_input

puzzle default {
layers 2
empty .

objects {
Player 0
}

display_objects {
Marker 1
}

legend P = Player

routine display mark_initial once {
input directions [ Player no Marker | ] -> [ Player Marker | ]
}

on_level_start {
display mark_initial
}

rules {

}

level start {
P.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("on_level_start cannot depend on input"));
}

#[test]
fn display_level_start_rejects_main_object_writes() {
    let source = r#"
title display_level_start_main_write

puzzle default {
layers 2
empty .

objects {
Source 0
Marker 1
}

legend S = Source

routine display mark_initial once {
[ Source no Marker ] -> [ Source Marker ]
}

on_level_start {
display mark_initial
}

rules {

}

level start {
S
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(
        error.contains("display block can read main objects but can only write display objects")
    );
}

#[test]
fn level_clear_rejects_input_dependent_rules() {
    let source = r#"
title level_clear_input

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

on_level_clear {
input directions [ Player | ] -> [ | Player ]
}

rules {

}

level start {
P.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("on_level_clear cannot depend on input"));
}

#[test]
fn old_on_level_start_syntax_is_rejected() {
    let source = r#"
title old_on_level_start

puzzle default {
layers 1
empty .
object Player 0
legend P = Player

on level_start {
}

rules {

}

level start {
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown puzzle directive on"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn old_when_transition_syntax_is_rejected() {
    let source = r#"
title old_when_transition

puzzle default {
layers 1
empty .
object Player 0
legend P = Player

win_conditions {
some Player
}

rules {

}

level start {
P
}
}

scene playing {
view {
board = puzzle default
}
rules {
when board.win_conditions -> board.next_level
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("scene transition triggers must be `<input>` or `if <condition>`"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn old_menu_on_handler_syntax_is_rejected() {
    let source = r#"
title old_menu_on_handler

puzzle default {
layers 1
empty .
object Player 0
legend P = Player

rules {

}

level start {
P
}
}

menu selector {
view {
button "Only" value 0
}
rules {
on up -> cursor.prev
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("menu action must be: <input> -> <command>"));
}

#[test]
fn conditional_rule_call_short_form_runs_named_rule_when_pattern_matches() {
    let source = r#"
title conditional_short

puzzle default {
layers 2
empty .

object Player 1
object Wall 1
object Flag 1
legend P = Player
legend W = Wall
legend F = Flag

routine Mark once {
[ Player ] -> [ Flag ]
}

rules {
[ Player | Wall ] -> Mark
}

level start {
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
title conditional_some_none

puzzle default {
layers 2
empty .

object Player 1
object Wall 1
object Flag 1
legend P = Player
legend W = Wall
legend F = Flag

routine Mark once {
[ Player ] -> [ Flag ]
}

rules {
if none([ Player | Wall ]) Mark
}

level start {
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
title pattern_condition_else

puzzle default {
layers 2
empty .

object Player 1
object Box 1
object Flag 1
object Wall 1
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

level start {
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
fn conditional_rule_call_accepts_embedded_puzzlescript_direction_marker() {
    let source = r#"
title conditional_direction_marker

puzzle default {
layers 2
empty .

object Player 1
object Wall 1
object Flag 1
legend P = Player
legend W = Wall
legend F = Flag

routine Mark once {
[ Player ] -> [ Flag ]
}

rules {
[ < Player | Wall ] -> Mark
}

level start {
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
fn unknown_directive_is_rejected() {
    let source = r#"
title old_keyword

puzzle default {
layers 2
empty .

thing Player 1
legend P = Player

rules {

}

level start
P
end
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown puzzle directive thing"));
}

#[test]
fn domain_keyword_is_not_part_of_public_syntax() {
    let source = r#"
title old_domain

puzzle default {
layers 2
empty .

domain color red blue
object Box 1
legend B = Box

rules {

}

level start
B
end
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown puzzle directive domain"));
}

#[test]
fn bare_tag_set_assignment_is_not_canonical_syntax() {
    let source = r#"
title old_tag_assignment

puzzle default {
layers 2
empty .

color = red blue

object Box:color 1

rules {

}

levels {
legend {
. = empty
}
level start
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
title old_directions

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
}
directions
rules {
once input directions [ Player | ] -> [ | Player ]
}
levels {
level start
P.
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown puzzle directive directions"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn top_level_puzzle_directives_are_rejected_publicly() {
    let source = r#"
title old_keyword
layers 2
empty .
object Player 1
legend P = Player
rules {

}
level start
P
end
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains(
        "top-level directive must be title, subtitle, author, homepage, var, const, default_wait_time, again_interval, puzzle, levels, sprites, menu, sounds, theme, or assets; found layers"
    ));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn parses_sample_game() {
    let source = include_str!("../../../games/spec_2d.puzzle");
    let loaded = super::parse_game2d(source).unwrap();

    assert!(loaded.game.object_count() >= 10);
    assert!(!loaded.game.rules().is_empty());
    assert_eq!(loaded.levels.len(), 5);
    assert!(loaded.levels[0].name.ends_with("solve_1"));
    assert!(loaded.levels[1].name.ends_with("solve_2"));
    assert!(loaded.levels[4].name.ends_with("child"));
    assert!(loaded.goal.is_some());
    assert!(!loaded.is_goal_complete(&loaded.levels[0].initial_state));
    assert_eq!(loaded.levels[0].initial_state.height, 3);
}

#[test]
fn parses_declared_assets() {
    let source = r#"
title assets_test

assets {
css "game.css"
script "visuals.js"
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
level one
P
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.assets.entries.len(), 2);
    assert_eq!(loaded.assets.entries[0].kind, AssetKind::Css);
    assert_eq!(loaded.assets.entries[0].path, "game.css");
    assert_eq!(loaded.assets.entries[1].kind, AssetKind::Script);
    assert_eq!(loaded.assets.entries[1].path, "visuals.js");
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn game_file_can_import_puzzle_fragments() {
    let dir = std::env::temp_dir().join(format!("puzzlestudio_import_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("sokoban.puzzle"),
        r#"
puzzle default {
layers 1
empty .

object Player 0
legend P = Player

rules {

}

import "levels.puzzle"
}
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("levels.puzzle"),
        r#"
levels {
level start
P
}
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("level_select.puzzle"),
        r#"
menu level_select {
data {
levels: list<level>
}
view {
column {
button "Start" value 0
}
}
rules {
enter -> emit choose_level(cursor.value)
}
}
"#,
    )
    .unwrap();
    let game_path = dir.join("game.puzzle");
    std::fs::write(
        &game_path,
        r#"
title imported
import "sokoban.puzzle"
import "level_select.puzzle"

scene select {
view {
menu selector = level_select with {
levels = default.levels
}
}
}
"#,
    )
    .unwrap();

    let loaded = super::parse_game2d_file(&game_path).unwrap();

    assert_eq!(loaded.title, "imported");
    assert_eq!(loaded.levels.len(), 1);
    assert_eq!(loaded.menus.len(), 1);
    assert!(matches!(
        &loaded.scenes[0].components[0],
        SceneComponent::Menu(_)
    ));
}

#[test]
fn top_level_levels_and_sprites_are_canonical_resources() {
    let source = r##"
title top_resources

puzzle default {
layers 1
empty .
object Player 0
legend P = Player
rules {

}
}

sprites {
Player #fff
}

levels worldA of default {
level 1
P

level {
P
}
}
"##;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.levels.len(), 2);
    assert_eq!(loaded.levels[0].name, "worldA.1");
    assert_eq!(loaded.levels[0].pack.as_deref(), Some("worldA"));
    assert_eq!(loaded.levels[0].puzzle, "default");
    assert_eq!(loaded.levels[1].name, "worldA.2");
    assert_eq!(loaded.visuals.sprites.len(), 1);
    assert_eq!(loaded.scenes[0].resources.levels, ResourceSelection::All);
}

#[test]
fn scene_resources_can_select_level_and_sprite_sets() {
    let source = r##"
title scene_resources

puzzle default {
layers 1
empty .
object Player 0
object Box 0
legend P = Player
rules {

}
}

sprites {
Player #fff
Box #000
}

levels worldA of default {
level 1
P
}

scene select {
resources {
levels worldA
sprites Player
}
view {
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
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn imported_section_header_closes_at_imported_file_boundary() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_import_section_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("title_view.puzzle"),
        r#"
====
VIEW
====
text "Imported"
"#,
    )
    .unwrap();
    let game_path = dir.join("game.puzzle");
    std::fs::write(
        &game_path,
        r#"
title imported_scene

puzzle default {
layers 1
empty .
object Player 0
legend P = Player
rules {

}
levels {
level start
P
}
}

scene title {
import "title_view.puzzle"
keys {
Enter -> start
}
rules {
start -> goto title
}
}
"#,
    )
    .unwrap();

    let loaded = super::parse_game2d_file(&game_path).unwrap();
    let title = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == "title")
        .unwrap();

    assert_eq!(title.components.len(), 1);
    assert_eq!(title.key_bindings.len(), 1);
    assert_eq!(title.transitions.len(), 1);
}

#[test]
fn game_file_can_import_theme_metadata() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_import_theme_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let game_path = dir.join("game.puzzle");
    std::fs::write(
        &game_path,
        r##"
title themed
import "themes/clean.puzzle"

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
level start {
P
}
}

theme clean {
accent_color #2f7ebc
board_color #edf1f2
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
fn theme_name_can_be_declared_without_block() {
    let loaded = parse_game(
        r##"
title themed
theme pixel
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
level start {
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
fn theme_block_requires_name_in_header() {
    let error = parse_game(
        r##"
title themed
theme {
name pixel
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
level start {
P
}
}
"##,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("theme header must be: theme <theme> or theme <theme> {"));
}

#[test]
fn game_entry_resolution_uses_prelude_puzzle_files() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_entry_resolution_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let game_path = dir.join("game.puzzle");
    let levels_path = dir.join("levels.puzzle");
    std::fs::write(&game_path, "title Entry\n").unwrap();
    std::fs::write(&levels_path, "levels {}\n").unwrap();

    assert_eq!(super::resolve_game_entry(&dir).unwrap(), game_path);
    assert_eq!(
        super::resolve_game_entry(&dir.join("game.puzzle")).unwrap(),
        game_path
    );
    assert_eq!(super::resolve_game_entry(&levels_path).unwrap(), game_path);
}

#[test]
fn game_entry_resolution_allows_non_game_named_prelude_files() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_named_entry_resolution_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(dir.join("fragments")).unwrap();
    let entry_path = dir.join("arcade.puzzle");
    let fragment_path = dir.join("fragments").join("levels.puzzle");
    std::fs::write(&entry_path, "title Arcade\n").unwrap();
    std::fs::write(&fragment_path, "levels {}\n").unwrap();

    assert_eq!(super::resolve_game_entry(&dir).unwrap(), entry_path);
    assert_eq!(
        super::resolve_game_entry(&fragment_path).unwrap(),
        entry_path
    );
}

#[test]
fn folder_without_game_prelude_is_not_auto_resolved() {
    let dir = std::env::temp_dir().join(format!(
        "puzzlestudio_entry_missing_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("only.puzzle"), "levels {}\n").unwrap();

    assert!(super::resolve_game_entry(&dir).is_err());
}

#[test]
fn parses_spec_2d_display_floor_object() {
    let loaded = super::parse_game2d_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../games/spec_2d.puzzle"
    ))
    .unwrap();

    assert!(loaded.object_labels.values().any(|label| label == "@Floor"));
}

#[test]
fn puzzle_sprites_expand_schema_tables() {
    let source = r#"
title sprite_schema

puzzle default {
tags {
kind = A B
}
objects {
layer {
Target:kind ab
}
layer {
Box:kind AB
Wall #
}
}
legend {
. = empty
}
sprites {
colors {
piece_color:kind {
A = #4a4
B = #a4a
}
}
palettes {
piece:kind {
A = piece_color:A transparent
B = piece_color:B transparent
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
Box:kind {
palette piece:kind
shape mark:kind
}
Wall {
#444
0
}
}
rules {
[ Box:A | ] -> [ | Box:A ]
}
levels {
level start
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
fn puzzle_sprites_accept_ps_style_one_off_sprite() {
    let source = r##"
title ps_style_sprite

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
}
sprites {
Player {
#e94f64 #2f80ed
0.
.1
}
}
rules {

}
levels {
level start
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
title line_style_solid_sprite

puzzle default {
objects {
layer {
Box B
}
}
legend {
. = empty
}
sprites {
Box
#aaa
}
rules {

}
levels {
level start
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
fn puzzle_sprites_accept_line_style_solid_color_table_sprite() {
    let source = r##"
title line_style_solid_color_table_sprite

puzzle default {
tags {
kind = A B
}
objects {
layer {
Light:kind L
}
}
legend {
. = empty
}
sprites {
colors {
piece_color:kind {
A = #4a4
B = #a4a
}
}
palettes {
piece:kind {
A = piece_color:A
B = piece_color:B
}
}
Light:kind
palette piece:kind
}
rules {

}
levels {
level start
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
title line_style_ascii_sprite

puzzle default {
objects {
layer {
Box B
Wall W
}
}
legend {
. = empty
}
sprites {
Box
#aaa
00000
00000
00000
00000
00000
Wall
#444
}
rules {

}
levels {
level start
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
fn puzzle_sprites_accept_line_style_palette_and_shape_refs() {
    let source = r##"
title line_style_palette_shape_refs

puzzle default {
objects {
layer {
Box B
}
}
legend {
. = empty
}
sprites {
palettes {
box_palette = #111 #eee
}
shapes {
box_shape {
010
111
010
}
}
Box
box_palette
shape box_shape
}
rules {

}
levels {
level start
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
fn puzzle_sprites_allow_duplicate_palette_color_refs() {
    let source = r##"
title duplicate_palette_color_refs

puzzle default {
tags {
kind = A B
}
objects {
layer {
Box B
}
}
legend {
. = empty
}
sprites {
colors {
shared = #123456
tagged:kind {
A = #abcdef
B = #fedcba
}
}
palettes {
box_palette = shared shared tagged:A tagged:A
}
shapes {
box_shape {
0123
}
}
Box
box_palette
shape box_shape
}
rules {

}
levels {
level start
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
fn puzzle_sprites_accept_image_sprite_refs() {
    let source = r##"
title image_sprite_ref

puzzle default {
objects {
layer {
Box B
}
}
legend {
. = empty
}
sprites {
Box sprites/box.png
}
rules {

}
levels {
level start
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
}

#[test]
fn puzzle_sprites_accept_more_than_ten_ps_style_colors() {
    let source = r##"
title ps_style_many_colors

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
}
sprites {
Player {
#000000 #111111 #222222 #333333 #444444 #555555 #666666 #777777 #888888 #999999 #aaaaaa
a
}
}
rules {

}
levels {
level start
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
title ps_style_alpha_colors

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
}
sprites {
Player {
#ff004d80 #00000000
01
}
}
rules {

}
levels {
level start
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
title leading_alpha_transparent_palette_color

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
}
sprites {
Player {
#00000000 #555555
01.
}
}
rules {

}
levels {
level start
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
title transparent_palette_color

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
}
sprites {
Player {
transparent #555
01
}
}
rules {

}
levels {
level start
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
title sprite_palette_overflow

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
}
sprites {
Player {
transparent
01
}
}
rules {

}
levels {
level start
P
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("sprite pattern references a color outside the color row"));
}

#[test]
fn puzzle_sprites_accept_ps_style_reusable_shape_sprite() {
    let source = r##"
title ps_style_reusable_sprite

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
}
sprites {
palettes {
player = #e94f64 #2f80ed
}
shapes {
player_shape {
0.
.1
}
}

Player {
palette player
shape player_shape
}
}
rules {

}
levels {
level start
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
fn puzzle_sprites_reject_old_ascii_sprite_syntax() {
    let source = r##"
title old_sprite_syntax

puzzle default {
objects {
layer {
Player P
}
}
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
level start
P
}
}
"##;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown sprites directive Player"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn numeric_value_set_values_can_select_object_variants() {
    let source = r#"
title numeric_rank_values

puzzle default {
tags {
rank = 1 2 3
}
objects {
layers {
floor = Portal:rank
actor = Player
}
}
legend {
. = empty
1 = Portal:1
2 = Portal:2
3 = Portal:3
P = Player
}
goal = count([ Player Portal:1 ]) == 0
rules {
once [ Player Portal:1 ] -> [ Player ]
}
levels {
level start
P1
}
}
"#;
    let loaded = parse_game(source).unwrap();

    let labels = loaded
        .object_labels
        .values()
        .cloned()
        .collect::<Vec<String>>();
    assert!(labels.iter().any(|label| label == "Portal:1"));
    assert!(labels.iter().any(|label| label == "Portal:2"));
    assert!(labels.iter().any(|label| label == "Portal:3"));
}

#[test]
fn directions_is_builtin_value_set_for_objects_sprites_and_for() {
    let source = r#"
title directions_value_set

puzzle default {
objects {
layer {
Player P
}
layer {
Boundary:directions
}
}
legend {
. = empty
}
sprites {
palettes {
edge = transparent #555
}
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
Boundary:directions {
palette edge
shape edge:directions
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
level start
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
fn sprite_shape_can_generate_direction_variants_by_rotation() {
    let source = r#"
title rotated_sprites

puzzle default {
map rotate directions {
up -> right
right -> down
down -> left
left -> up
}
objects {
layer {
Boundary:directions
}
}
legend {
. = empty
}
sprites {
palettes {
edge = transparent #555
}
shapes {
edge:directions rotate from up {
111
000
000
}
}
Boundary:directions {
palette edge
shape edge:directions
}
}
rules {

}
levels {
level start
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
fn sprite_ascii_lookup_can_map_selector_axis_values() {
    let source = r#"
title mapped_sprite_lookup

puzzle default {
map rotate directions {
up -> right
right -> down
down -> left
left -> up
}
objects {
layer {
Boundary:directions
}
}
legend {
. = empty
}
sprites {
palettes {
edge = transparent #555
}
shapes {
edge:directions rotate from up {
111
000
000
}
}
Boundary:directions {
palette edge
shape edge:rotate(directions)
}
}
rules {

}
levels {
level start
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
title mapped_sprite_selector

puzzle default {
map rotate directions {
up -> right
right -> down
down -> left
left -> up
}
objects {
layer {
Boundary:directions
}
}
legend {
. = empty
}
sprites {
palettes {
edge = transparent #555
}
shapes {
edge:directions rotate from up {
111
000
000
}
}
Boundary:rotate(directions) {
palette edge
shape edge:directions
}
}
rules {

}
levels {
level start
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
title input_in_directions

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
}
rules {
if input in directions {
once input directions [ Player | ] -> [ | Player ]
}
}
levels {
level start
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
title horizontal_orientation_set

puzzle default {
objects {
layer {
Player P
Wall #
OpenWall O
}
}
legend {
. = empty
}
rules {

once horizontal [ Player | Wall ] -> [ Player | OpenWall ]
}
levels {
level start
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
title directions_orientation_set

puzzle default {
objects {
layer {
Player P
Wall #
OpenWall O
}
}
legend {
. = empty
}
rules {

once directions [ Player | Wall ] -> [ Player | OpenWall ]
}
levels {
level start
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
fn vertical_orientation_set_expands_query_pattern() {
    let source = r#"
title vertical_orientation_set_query

puzzle default {
objects {
layer {
Player P
Wall #
Door D
OpenDoor O
}
}
legend {
. = empty
}
rules {

if some(vertical [ Player | Wall ]) {
once [ Door ] -> [ OpenDoor ]
}
}
levels {
level start
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
title input_horizontal_rewrite

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
}
rules {

once input horizontal [ Player | ] -> [ | Player ]
}
levels {
level start
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
title input_directions_sugar

puzzle default {
objects {
layer {
Player P
}
}
legend {
. = empty
}
rules {
once input [ Player | ] -> [ | Player ]
}
levels {
level start
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
fn input_directions_query_pattern_adds_input_guard_and_expands_orientation() {
    let source = r#"
title input_directions_query

puzzle default {
objects {
layer {
Player P
Wall #
Door D
OpenDoor O
}
}
legend {
. = empty
}
rules {

if some(input directions [ Player | Wall ]) {
once [ Door ] -> [ OpenDoor ]
}
}
levels {
level start
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
fn input_query_pattern_without_set_is_directions_sugar() {
    let source = r#"
title input_query_directions_sugar

puzzle default {
objects {
layer {
Player P
Wall #
Door D
OpenDoor O
}
}
legend {
. = empty
}
rules {
if some(input [ Player | Wall ]) {
once [ Door ] -> [ OpenDoor ]
}
}
levels {
level start
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
title mapped_input_condition

puzzle default {
map rotate directions {
up -> right
right -> down
down -> left
left -> up
}
objects {
layer {
Player P
Marker M
}
}
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
level start
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
fn prefixless_spatial_rewrite_expands_to_cardinal_directions() {
    let source = r#"
title implicit_cardinal_rewrite

puzzle default {
layers 2
empty .

object A 1
legend A = A

rules {
once [ A | ] -> [ | A ]
}

level start {
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
title multiline_rewrite

puzzle default {
layers 2
empty .

object A 1
object B 1
object C 1
legend A = A
legend B = B
legend C = C

rules {
once [ A ]
-> [ B ]
once [ B ] ->
[ C ]
}

level start {
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
title multiline_rewrite_nested_arrow

puzzle default {
layers 2
empty .

object A 1
object B 1
object C 1
legend A = A

rules {
[ A ] ->
[ B ] -> [ C ]
}

level start {
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
title implicit_cardinal_condition

puzzle default {
layers 2
empty .

object Player 1
object Wall 1
object Flag 1
legend P = Player
legend W = Wall
legend F = Flag

routine Mark once {
[ Player ] -> [ Flag ]
}

rules {
[ Player | Wall ] -> Mark
}

level start {
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
title implicit_cardinal_query

puzzle default {
layers 2
empty .

object Player 1
object Wall 1
object Flag 1
legend P = Player
legend W = Wall
legend F = Flag

query blocked = exists([ Player | Wall ])

rules {
if blocked {
once [ Player ] -> [ Flag ]
}
}

level start {
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
fn fix_once_sets_default_rewrite_application_for_nested_lines() {
    let source = r#"
title fix_once

puzzle default {
layers 2
empty .

object A 1
legend A = A

rules
fix once
right [ A | ] -> [ | A ]
end
end

level start
A..
end
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
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn fix_can_contain_application_and_orientation_defaults() {
    let source = r#"
title fix_once_left

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

rules
fix once left
[ Player | ] -> [ | Player ]
end
end

level start
...
.P.
...
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 0, 1, player));
    assert_eq!(loaded.game.rules()[0].application, RuleApplication::Once);
}

#[test]
fn explicit_rewrite_prefix_overrides_fix_default() {
    let source = r#"
title fix_explicit_override

puzzle default {
layers 2
empty .

object A 1
legend A = A

rules
fix once
repeat right [ A | ] -> [ | A ]
end
end

level start
A..
end
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
title once_all_rewrite

puzzle default {
layers 2
empty .

object A 1
object B 1
legend A = A
legend B = B

rules {
once_all [ A ] -> [ B ]
}

level start {
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
fn once_per_level_rewrite_fires_only_once_for_current_level_state() {
    let source = r#"
title once_per_level_rewrite

puzzle default {
layers 2
empty .

var count = 0

object A 1
legend A = A

rules {
once_per_level [ A ] -> [ A ] count += 1
}

level start {
A
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let first =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();
    let second = transition_state(&loaded.game, &first, InputId(0)).unwrap();

    assert_eq!(first.visible_globals(), &[1]);
    assert_eq!(second.visible_globals(), &[1]);
    assert_eq!(
        loaded.game.rules()[0].application,
        RuleApplication::OncePerLevel
    );
}

#[test]
fn fix_default_applies_through_nested_blocks() {
    let source = r#"
title fix_nested_block

puzzle default {
layers 2
empty .

object A 1
legend A = A

rules
fix once
repeat
right [ A | ] -> [ | A ]
end
end
end

level start
A..
end
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
title fix_input

puzzle default {
layers 1
empty .

fix input
lft a arrow_left
rgt d arrow_right
end

rules {

}

level start
.
end
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown puzzle directive fix"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn brace_blocks_are_accepted_for_block_directives() {
    let source = r#"
title brace_blocks

puzzle default {
layers 2
empty .

tags {
axis = left right
}
map flip axis {
left -> right
right -> left
}

object Player 1
legend P = Player

input left a arrow_left
input right d arrow_right

routine move once {
for d in horizontal {
if input == d {
d [ Player | ] -> [ | Player ]
} else {
}
}
}

rules {
repeat {
move
}
fix once right {
[ Player | ] -> [ | Player ]
}
}

level start {
.P.
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert_eq!(loaded.levels.len(), 1);
    assert!(!loaded.game.rules().is_empty());
}

#[test]
fn scene_keys_define_action_bindings_and_puzzle_controls() {
    let source = r#"
title scene_keys

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

rules {
once input directions [ Player | ] -> [ | Player ]
}

level start {
P.
}
}

scene playing {
view {
board = puzzle default
message_visible = false
moves = 0
message = "Push the box"
}
inputs {
right <- d ArrowRight
level_select <- q
menu <- Escape
}
}

scene level_select {
view {
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
    assert_eq!(loaded.scenes[0].state.variables.len(), 3);
    assert_eq!(loaded.scenes[0].state.variables[0].name, "message_visible");
    assert_eq!(
        loaded.scenes[0].state.variables[0].default,
        SceneValue::Bool(false)
    );
    assert_eq!(
        loaded.scenes[0].state.variables[2].default,
        SceneValue::Text("Push the box".to_string())
    );
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
title goto_effects

puzzle default {
layers 1
empty .
object Player 0
legend P = Player

rules {
once [ Player ] -> [ Player ]
}

level start {
P
}
}

scene select {
view {
board = puzzle default
column {
button board.level.label -> playing.goto board.level.name
button "Block" -> playing.goto board.level.index
}
}
inputs {
choose <- Enter
}
rules {
choose -> playing.goto board.level.name
}
}

scene playing {
view {
board = puzzle default
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
        loaded.scenes[0].transitions[0].effect,
        SceneEffect::GotoLevel { .. }
    ));
}

#[test]
fn scene_effects_parse_targeted_restart() {
    let source = r#"
title targeted_restart

puzzle default {
layers 1
empty .
object Player 0
legend P = Player

rules {
once [ Player ] -> [ Player ]
}

level start {
P
}
}

scene playing {
view {
board = puzzle default
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
fn scene_effects_reject_start_level_scene_commands() {
    let source = r#"
title start_level_scene

puzzle default {
layers 1
empty .
object Player 0
legend P = Player

rules {
once [ Player ] -> [ Player ]
}

level first {
P
}
}

scene title {
	button "Play" -> start levels in playing
	inputs {
	start_first <- Enter Space
	}
	rules {
	start_first -> start levels first in playing
	}
	}

scene playing {
view {
board = puzzle default
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("no longer supported"));
    assert!(error.contains("goto <scene>(<level>)"));
}

#[test]
fn scene_effect_parser_retains_semantic_tokens() {
    let line = "goto playing(first)";
    let parsed = parse_scene_effect_with_semantic_tokens(line, line).unwrap();
    assert!(matches!(
        parsed.surface.effect,
        SceneEffect::Goto { ref scene, ref params }
            if scene == "playing" && params.len() == 1 && params[0].name == "level"
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
fn scene_lifecycle_words_parse_by_state_semantics() {
    let resume = parse_scene_effect("resume detail with selected = first", "").unwrap();
    assert!(matches!(
        resume,
        SceneEffect::Goto { ref scene, ref params }
            if scene == "detail" && params.len() == 1 && params[0].name == "selected"
    ));

    let open = parse_scene_effect("open menu", "").unwrap();
    assert!(matches!(
        open,
        SceneEffect::Enter { ref scene, ref params } if scene == "menu" && params.is_empty()
    ));

    let close = parse_scene_effect("close", "").unwrap();
    assert!(matches!(close, SceneEffect::Back));

    let start = parse_scene_effect("start playing(first)", "").unwrap();
    assert!(matches!(
        start,
        SceneEffect::Sequence(ref effects)
            if matches!(effects.as_slice(), [
                SceneEffect::Reset { scene: reset_scene },
                SceneEffect::Goto { scene: goto_scene, params }
            ] if reset_scene == "playing" && goto_scene == "playing" && params.len() == 1)
    ));
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
        &line[token.start..token.end] == "sfx" && token.kind == SemanticKind::Emission
    }));
    assert!(parsed.surface.document.semantic_tokens.iter().any(|token| {
        &line[token.span.start..token.span.end] == "sfx"
            && token.kind == SurfaceSemanticKind::Emission
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
scene title
view
title game.title
end
button "Play" -> goto playing
end

puzzle main
rules
[ Player ] -> [ Player ] sfx bump
end
end
"#;
    let surface = parse_surface_document(source);
    let scene_name_start = source.find("scene title").unwrap() + "scene ".len();
    let component_title_start = source.rfind("title game.title").unwrap();

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
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn persistent_vars_and_clear_undo_history_parse() {
    let source = r#"
title persistent_history_parse

puzzle default {
persistent var cleared = false

objects {
layer {
Player P
}
}

legend {
. = empty
}

rules {
once [ Player ] -> [ Player ] set cleared = true
}

level start {
P
}
}

scene playing {
view {
board = puzzle default
}
rules {
clear -> clear_undo_history
}
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.persistent_vars.len(), 1);
    assert!(matches!(
        loaded.scenes[0].transitions[0].effect,
        SceneEffect::ClearUndoHistory
    ));
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
        parse_scene_effect("set current_level = level", "set current_level = level").unwrap(),
        SceneEffect::SetCurrentLevel { .. }
    ));
    assert!(matches!(
        parse_scene_effect("set level.cleared = true", "set level.cleared = true").unwrap(),
        SceneEffect::SetLevelCleared {
            level: None,
            cleared: true
        }
    ));
    assert!(matches!(
        parse_scene_effect(
            "set level(\"microban.2\").cleared = false",
            "set level(\"microban.2\").cleared = false"
        )
        .unwrap(),
        SceneEffect::SetLevelCleared {
            level: Some(_),
            cleared: false
        }
    ));
}

#[test]
fn var_scopes_parse_by_owner() {
    let source = r#"
title var_scopes
var session_label = "Session Label"
persistent var high_score = 0

puzzle default {
var moved = false
persistent var cleared = false

objects {
layer {
Player P
}
}

legend {
. = empty
P = Player
}

rules {
once [ Player ] -> set moved = true
}

level start {
P
}
}

scene playing {
var message = "Ready"
persistent var last_tab = levels
view {
board = puzzle default
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
    assert_eq!(loaded.global_labels.len(), 2);
    assert!(loaded.global_labels.values().any(|name| name == "moved"));
    assert!(loaded.global_labels.values().any(|name| name == "cleared"));
    assert_eq!(loaded.persistent_vars.len(), 1);
    assert_eq!(loaded.scenes[0].state.variables.len(), 2);
    assert_eq!(loaded.scenes[0].state.variables[1].name, "last_tab");
    assert_eq!(
        loaded.scenes[0].state.variables[1].lifetime,
        SceneStateLifetime::Persistent
    );
}

#[test]
fn objects_and_legend_blocks_define_layers_rendering_groups_and_empty() {
    let source = r#"
title object_blocks

puzzle default {
objects {
layer {
Goal G
}
layer {
Player P
Box B
Wall #
}
group solid = Player Box Wall
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

level start {
#P.BG
}
}

scene playing {
view {
board = puzzle default
}
inputs {
right <- d ArrowRight
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
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn group_block_defines_selector_groups() {
    let source = r#"
title group_block

puzzle default {
layers 2
empty .

object Player 1
object Box 1
object Wall 1

group {
solid = Player Box Wall
}

legend P = Player
legend B = Box
legend # = Wall

input right d arrow_right

rules {
once input directions [ Player | no solid ] -> [ | Player ]
}

level start {
P#.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = input_named(&loaded, "right");
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 0, 0, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn objects_block_rejects_bare_group_rows() {
    let source = r#"
title bare_group_row

puzzle default {
objects {
layers {
actor = Player Box Wall
}
solid = actor
}

legend {
. = empty
P = Player
}

rules {

}

level start {
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("group rows must be inside `group { ... }`"));
}

#[test]
fn level_body_legend_adds_level_local_chars() {
    let source = r#"
title level_local_legend

puzzle default {
objects {
layer {
Goal G
}
layer {
Box B
Player P
}
}

legend {
. = empty
}

rules {

}

levels {
level local
legend {
x = Goal Box
}
x

level plain
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
title level_local_legend_no_leak

puzzle default {
objects {
layer {
Goal
}
layer {
Box
}
}

legend {
. = empty
}

rules {

}

levels {
level first
legend x = Goal Box
x

level second
x
}
}
"#;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("unknown level char 'x'"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn layers_block_can_define_named_layers_and_use_names_as_tags() {
    let source = r#"
title named_layers

puzzle default {
objects {
layers {
floor = Goal Button
actor = Player Box Wall
}
}

legend {
. = empty
P = Player
G = Goal
B = Box
W = Wall
O = Button
}

rules {
once input directions [ Player | no floor ] -> [ | Player ]
}

level start {
PG.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let goal = object_named(&loaded, "Goal");
    let button = object_named(&loaded, "Button");
    let player = object_named(&loaded, "Player");
    let right = input_named(&loaded, "right");

    assert_eq!(loaded.game.object_count(), 5);
    assert_eq!(loaded.game.layer_count, 2);
    assert_eq!(
        loaded.game.object_layer(goal),
        loaded.game.object_layer(button)
    );

    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    assert!(moved.has_object(&loaded.game, 0, 0, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn standard_move_uses_named_layer_groups_without_blocking_other_layers() {
    let source = r#"
title standard_move_named_layers

puzzle default {
objects {
layers {
floor = Goal
actor = Player Box Wall
}
}

legend {
. = empty
P = Player
G = Goal
}

rules {
if input == right {
once right [ Player | no actor ] -> [ > Player | ]
}
move
}

level start {
PG
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = input_named(&loaded, "right");
    let player = object_named(&loaded, "Player");
    let goal = object_named(&loaded, "Goal");

    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    assert!(moved.has_object(&loaded.game, 1, 0, player));
    assert!(moved.has_object(&loaded.game, 1, 0, goal));
}

#[test]
fn standard_move_registers_anonymous_layers_as_internal_groups() {
    let source = r#"
title standard_move_anonymous_layers

puzzle default {
objects {
layer {
Player P
Box B
}
}

legend {
. = empty
}

rules {
if input == right {
once right [ Player | ] -> [ > Player | ]
}
move
}

level start {
P.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = input_named(&loaded, "right");
    let player = object_named(&loaded, "Player");

    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    assert!(moved.has_object(&loaded.game, 1, 0, player));
}

#[test]
fn detects_goal_completion_after_solving_sample_game() {
    let source = r#"
title goal_fixture
puzzle sokoban {
objects {
layer {
Goal G
}
layer {
Player P
Box B
Wall #
}
group {
solid = Player Box Wall
}
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
level first
#######
#P.B.G#
#######

level second
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
title goal_fixture
puzzle sokoban {
objects {
layer {
Goal G
}
layer {
Player P
Box B
Wall #
}
group {
solid = Player Box Wall
}
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
level first
#######
#P.B.G#
#######

level second
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
title lose_fixture
puzzle default {
objects {
layer {
Box B
Wall #
}
group {
solid = Box Wall
}
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
level start
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
title lose_fixture
puzzle default {
objects {
layer {
Box B
Wall #
}
group {
solid = Box Wall
}
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
level start
.#
B#
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_lose_complete(&loaded.levels[0].initial_state));
}

#[test]
fn condition_blocks_accept_explicit_any_combinator() {
    let source = r#"
title condition_any_fixture
puzzle default {
objects {
layer {
Goal G
}
layer {
Box B
Wall #
}
group {
solid = Box Wall
}
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
level start
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
title condition_for_fixture
puzzle default {
tags {
kind = A B
}
objects {
Goal:kind
Box:kind
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
level start
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
title nested_condition_for_fixture
puzzle default {
tags {
kind = A B
}
tags {
direction_side = up down
}
objects {
Box:kind
Edge:direction_side
Found:kind:direction_side
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
level start
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
fn condition_blocks_accept_no_pattern_all_on_and_count_compare() {
    let source = r#"
title condition_fixture
puzzle default {
objects {
layer {
Goal G
}
layer {
Box B
Wall #
}
group {
solid = Box Wall
}
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
level start
*#
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_lose_complete(&loaded.levels[0].initial_state));
}

#[test]
fn condition_blocks_lower_none_function_to_short_circuit_query() {
    let source = r#"
title none_condition_fixture
puzzle default {
objects {
layer {
Goal G
}
layer {
Box B
}
}
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
level start
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
                value: GoalValue::QueryValue(QueryKind::NoneMatches(_)),
                op: ComparisonOp::NotEq,
                expected: 0,
            })
        )),
        "none(pattern) should stay a NoneMatches query, not lower to count(pattern) == 0"
    );
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn low_level_numeric_offsets_rotate_when_guard_uses_dir() {
    let source = r#"
title low_level_direction

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

routine move once
for x in directions
if input == x
once x [ Player | ] -> [ | Player ]
end
end
end

rulesmove
end

level start
...
.P.
...
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let input = *loaded.controls.keys.get(&b'w').unwrap();
    let state = transition_state(&loaded.game, &loaded.levels[0].initial_state, input).unwrap();

    let player = loaded
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "Player").then_some(*object))
        .unwrap();
    assert!(state.has_object(&loaded.game, 1, 0, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn fixed_orientation_rewrite_reads_as_direction_literal() {
    let source = r#"
title fixed_orientation

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

routine move_right once
if input == right
once right [ Player | ] -> [ | Player ]
end
end

rulesmove_right
end

level start
...
.P.
...
end
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.game.rules().len(), 1);

    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = loaded
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "Player").then_some(*object))
        .unwrap();

    assert!(moved.has_object(&loaded.game, 2, 1, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn canonical_rules_block_calls_routine() {
    let source = r#"
title canonical_rules

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

routine move_right once
if input == right
once right [ Player | ] -> [ | Player ]
end
end

rules
move_right
end

level start
...
.P.
...
end
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.game.rules().len(), 1);

    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 2, 1, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn schema_selectors_expand_independently_and_preserve_matched_objects() {
    let source = r#"
title schema_selectors

puzzle default {
layers 2
empty .

tags {
color = red blue
}

object player:color 1
object box:color 1
legend p = player:red
legend q = player:blue
legend a = box:red
legend b = box:blue

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

routine push once
once input directions [ player:color | box:color | ] -> [ | player:color | box:color ]
end

rulespush
end

level start
pb.
end
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.game.object_count(), 4);
    assert_eq!(loaded.game.rules().len(), 16);

    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player_red = object_named(&loaded, "player:red");
    let box_blue = object_named(&loaded, "box:blue");

    assert!(moved.has_object(&loaded.game, 1, 0, player_red));
    assert!(moved.has_object(&loaded.game, 2, 0, box_blue));
}

#[test]
fn schema_selector_tag_can_be_subset_value_set() {
    let source = r#"
title subset_selector

puzzle default {
layers 2
empty .

tags {
kind = A B C D
}
tags {
kindprime = A B C
}

object Target:kind 1
legend a = Target:A
legend b = Target:B
legend c = Target:C
legend d = Target:D

win_conditions = count(Target:kindprime) == 3

rules {

}

level start {
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
title subset_selector_bad_value

puzzle default {
layers 2
empty .

tags {
kind = A B C D
}
tags {
kindprime = A B X
}

object Target:kind 1
legend a = Target:A

win_conditions = count(Target:kindprime) == 0

rules {

}

level start {
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
title subset_selector_ambiguous_tag

puzzle default {
layers 2
empty .

tags {
kind = directions A
}

object Target:kind 1
legend a = Target:A

win_conditions = count(Target:directions) == 0

rules {

}

level start {
a
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("selector tag directions is ambiguous"));
    assert!(error.contains("Target tag slot kind"));
}

#[test]
fn schema_selector_subset_value_sets_are_positional_for_multiple_axes() {
    let source = r#"
title subset_selector_two_axes

puzzle default {
layers 2
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

object Target:kind:state 1
legend a = Target:A:on
legend b = Target:B:on
legend c = Target:C:on
legend x = Target:A:off

win_conditions = count(Target:kindprime:stateprime) == 3

rules {

}

level start {
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
title subset_selector_no_axis_skip

puzzle default {
layers 2
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

object Target:kind:state 1
legend a = Target:A:on

win_conditions = count(Target:stateprime) == 1

rules {

}

level start {
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
title star_selector

puzzle default {
layers 2
empty .

tags {
facing = left right
}

object player:facing 1
legend l = player:left
legend r = player:right

input right direction right

rules {

once input directions [ player:* | ] -> [ | player:* ]
}

level start
r.
end
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
fn underscore_selector_wildcard_is_rejected() {
    let source = r#"
title underscore_selector

puzzle default {
layers 2
empty .

tags {
facing = left right
}

object player:facing 1
legend l = player:left

rules {

once [ player:_ ] -> [ player:_ ]
}

level start
l
end
}

"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("wildcard must use *"));
    assert!(error.contains("_ is reserved for completion"));
}

#[test]
fn bare_schema_family_selector_is_rejected() {
    let source = r#"
title bare_schema_selector

puzzle default {
layers 2
empty .

tags {
facing = left right
}

object player:facing 1
legend l = player:left

rules {

once [ player ] -> [ player ]
}

level start
l
end
}

"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("object selector for variants must use :*"));
}

#[test]
fn star_selector_fills_unconstrained_variant_slots() {
    let source = r#"
title star_selector_slots

puzzle default {
layers 2
empty .

tags {
kind = A B
}
tags {
state = on off
}

object Target:kind:state 1
legend a = Target:A:on
legend b = Target:B:on
legend x = Target:A:off

win_conditions = count(Target:*:on) == 2

rules {

}

level start {
abx
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.is_goal_complete(&loaded.levels[0].initial_state));
}

#[test]
fn object_family_base_cannot_be_a_concrete_object() {
    let source = r#"
title family_shadow

puzzle default {
layers 2
empty .

tags {
color = red blue
}

object marker 1
object marker:color 1

rules {

}

level start
.
end
}

"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("object family name must not shadow an object"));
}

#[test]
fn concrete_object_cannot_shadow_object_family_base() {
    let source = r#"
title family_shadow

puzzle default {
layers 2
empty .

tags {
color = red blue
}

object marker:color 1
object marker 1

rules {

}

level start
.
end
}

"#;
    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("object name must not shadow an object family selector"));
}

#[test]
fn blank_lines_split_level_into_auto_placed_regions() {
    let source = r#"
title region_level

puzzle default {
layers 2
empty .

object Player 1
object Box 1
legend P = Player
legend B = Box

rules {
once input directions [ Player | ] -> [ | Player ]
}

level start {
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
title unbraced_named_levels

puzzle default {
layers 1
empty .
object Player 0
object Box 0
legend P = Player
legend B = Box
rules {

}
levels {
level intro
P

level followup
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
title unnamed_levels

puzzle default {
layers 1
empty .
object Player 0
object Box 0
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
title unnamed_multi_region

puzzle default {
layers 1
empty .
object Player 0
object Box 0
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
fn puzzle_view_parses_flickscreen_viewport_controls() {
    let source = r#"
title frame_view

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

flickscreen 5x3
screen_focus Player

rules {

once input directions [ Player | ] -> [ | Player ]
}

level start {
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
title full_frame

puzzle default {
layers 1
empty .

flickscreen full

rules {

}

level start {
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
title region_frame

puzzle default {
layers 1
empty .

frame_size region

rules {

}

level start {
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
title region_frame

puzzle default {
layers 1
empty .

frame_focus Player

rules {

}

level start {
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
title zoom_view

puzzle default {
layers 1
empty .

zoomscreen 5 3

rules {

}

level start {
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
fn puzzle_render_parses_grid_occupied_cells() {
    let source = r#"
title grid_render

puzzle default {
layers 1
empty .

render {
grid occupied_cells all_cells
}

rules {

}

level start {
.
}
}
"#;
    let loaded = parse_game(source).unwrap();

    assert!(loaded.render.grid.occupied_cells);
    assert!(loaded.render.grid.all_cells);
}

#[test]
fn puzzle_render_rejects_old_boolean_grid_assignments() {
    let source = r#"
title grid_render

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

level start {
.
}
}
"#;

    assert!(parse_game(source).is_err());
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn selector_value_can_use_direction_words_as_tags() {
    let source = r#"
title direction_word_tag

puzzle default {
layers 2
empty .

tags {
facing = left right
}

object player:facing 1
legend l = player:left
legend r = player:right

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

routine move_left_facing_player once
once input directions [ player:left | ] -> [ | player:left ]
end

rulesmove_left_facing_player
end

level start
l.
end
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.game.rules().len(), 4);

    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player_left = object_named(&loaded, "player:left");

    assert!(moved.has_object(&loaded.game, 1, 0, player_left));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn no_selector_forbids_group_members() {
    let source = r#"
title no_group

puzzle default {
layers 2
empty .

object Player 1
object Wall 1
object Goal 0
group blocked = Wall Goal

legend P = Player
legend # = Wall
legend G = Goal

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

routine move once
once input directions [ Player | no blocked ] -> [ | Player ]
end

rulesmove
end

level start
PG.
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 0, 0, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn positive_group_selector_preserves_matched_member() {
    let source = r#"
title positive_group

puzzle default {
layers 2
empty .

object Player 1
object Box 1
object Crate 1
group pushable_objects = Box Crate

legend P = Player
legend B = Box
legend C = Crate

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

routine push once
once input directions [ Player | pushable_objects | ] -> [ | Player | pushable_objects ]
end

rulespush
end

level start
PC.
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");
    let crate_object = object_named(&loaded, "Crate");

    assert!(moved.has_object(&loaded.game, 1, 0, player));
    assert!(moved.has_object(&loaded.game, 2, 0, crate_object));
}

#[test]
fn repeated_group_selector_expands_independently_and_preserves_occurrence_order() {
    let source = r#"
title repeated_group_selector

puzzle default {
layers 2
empty .

object Box 1
object Crate 1
group cargo = Box Crate

legend B = Box
legend C = Crate

rules {
once [ cargo | cargo | ] -> [ | cargo | cargo ]
}

level start
BC.
end
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
title selector_occurrence_labels

puzzle swap {
layers {
actor = Box Crate
}
group {
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
fn selector_occurrence_labels_must_be_unique_in_before_pattern() {
    let source = r#"
title duplicate_selector_occurrence_label

puzzle swap {
layers {
actor = Box Crate
}
group {
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
fn object_occurrence_labels_swap_occurrence_scratch() {
    let source = r#"
title object_occurrence_label_scratch_swap

puzzle swap {
layers {
marker = HotMarker ColdMarker
actor = Box
}
scratch {
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
title repeated_schema_selector

puzzle default {
layers 2
empty .

tags {
color = red blue
}

object box:color 1
legend r = box:red
legend b = box:blue

rules {
once [ box:color | box:color | ] -> [ | box:color | box:color ]
}

level start
rb.
end
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
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn custom_direction_names_use_explicit_direction_declarations() {
    let source = r#"
title custom_direction_names

puzzle default {
layers 2
empty .

tags {
move_dir = north south west east
}

object Player 1
legend P = Player

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

direction north up
direction south down
direction west left
direction east right

routine move once
for x in move_dir
if input == x
once x [ Player | ] -> [ | Player ]
end
end
end

rulesmove
end

level start
...
.P.
...
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let input = *loaded.controls.keys.get(&b'w').unwrap();
    let state = transition_state(&loaded.game, &loaded.levels[0].initial_state, input).unwrap();

    let player = loaded
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "Player").then_some(*object))
        .unwrap();
    assert!(state.has_object(&loaded.game, 1, 0, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn direction_alias_rewrites_to_canonical_direction_input() {
    let source = r#"
title direction_alias

puzzle default {
objects {
layer {
Player P
}
}

legend {
. = empty
}

direction east right

rules {
if input == east {
once east [ Player | ] -> [ | Player ]
}
}

levels {
level start
P.
}
}

scene playing {
view {
board = puzzle default
}
inputs {
east <- d ArrowRight
}
rules {
step board
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let input = *loaded.controls.keys.get(&b'd').unwrap();
    assert_eq!(input, input_named(&loaded, "right"));

    let state = transition_state(&loaded.game, &loaded.levels[0].initial_state, input).unwrap();
    let player = object_named(&loaded, "Player");
    assert!(state.has_object(&loaded.game, 1, 0, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn numeric_direction_vector_syntax_is_rejected() {
    let source = r#"
title old_numeric_direction

puzzle default {
objects {
layers {
actor = Player
}
}

legend {
. = empty
P = Player
}

direction east 1 0

rules {

}

level start {
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("direction must be: direction <alias> <up|down|left|right>"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn horizontal_and_vertical_axes_filter_direction_variants() {
    let source = r#"
title axis_filters

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

routine move_horizontal once
for h in horizontal
if input == h
once h [ Player | ] -> [ | Player ]
end
end
end

routine move_vertical once
for v in vertical
if input == v
once v [ Player | ] -> [ | Player ]
end
end
end

rulesmove_horizontal
move_vertical
end

level start
...
.P.
...
end
}
"#;
    let loaded = parse_game(source).unwrap();

    let up = *loaded.controls.keys.get(&b'w').unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();

    let moved_up = transition_state(&loaded.game, &loaded.levels[0].initial_state, up).unwrap();
    let moved_right =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();

    let player = loaded
        .object_labels
        .iter()
        .find_map(|(object, label)| (label == "Player").then_some(*object))
        .unwrap();
    assert!(moved_up.has_object(&loaded.game, 1, 0, player));
    assert!(moved_right.has_object(&loaded.game, 2, 1, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn rule_header_defaults_rewrites_to_repeat() {
    let source = r#"
title repeat_rule

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

routine slide
input directions [ Player | ] -> [ | Player ]
end

rulesslide
end

level start
P...
end
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.game.rules().len(), 4);
    assert!(
        loaded
            .game
            .rules()
            .iter()
            .all(|rule| rule.application == RuleApplication::UntilStable)
    );

    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 3, 0, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn rule_header_repeat_repeats_the_whole_block() {
    let source = r#"
title repeat_block

puzzle default {
layers 2
empty .

object Fire 1
object Wood 1
legend F = Fire
legend W = Wood

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

routine spread repeat
once right [ Fire | Wood ] -> [ Fire | Fire ]
end

rulesspread
end

level start
FWWW
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let fire = object_named(&loaded, "Fire");

    assert!(moved.has_object(&loaded.game, 0, 0, fire));
    assert!(moved.has_object(&loaded.game, 1, 0, fire));
    assert!(moved.has_object(&loaded.game, 2, 0, fire));
    assert!(moved.has_object(&loaded.game, 3, 0, fire));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn anonymous_inline_rewrite_can_be_once() {
    let source = r#"
title anonymous_once

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

rules
once input directions [ Player | ] -> [ | Player ]
end

level start
P...
end
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.game.rules().len(), 4);
    assert!(
        loaded
            .game
            .rules()
            .iter()
            .all(|rule| rule.application == RuleApplication::Once)
    );

    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 1, 0, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn once_block_can_wrap_expanded_statements() {
    let source = r#"
title once_block_for

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

rules
once
for x in directions
if input == x
once x [ Player | ] -> [ | Player ]
end
end
end
end

level start
P...
end
}
"#;
    let loaded = parse_game(source).unwrap();
    assert!(matches!(
        loaded.game.program()[0],
        puzzle_core::RuleStep::Block {
            application: RuleApplication::Once,
            ..
        }
    ));

    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 1, 0, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn anonymous_inline_rewrite_can_be_explicit_repeat() {
    let source = r#"
title anonymous_repeat

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

rulesrepeat input directions [ Player | ] -> [ | Player ]
end

level start
P...
end
}
"#;
    let loaded = parse_game(source).unwrap();
    assert_eq!(loaded.game.rules().len(), 4);
    assert!(
        loaded
            .game
            .rules()
            .iter()
            .all(|rule| rule.application == RuleApplication::UntilStable)
    );

    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");

    assert!(moved.has_object(&loaded.game, 3, 0, player));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn repeat_statement_block_retries_group_variants_together() {
    let source = r#"
title repeat_group_block

puzzle default {
layers 3
empty .

object Player 1
object Box 1
object Wall 1
object Moment 2
group pushable = Player Box
group solid = Player Box Wall

legend P = Player
legend B = Box
legend # = Wall

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

rules
once input directions [ Player ] -> [ Player Moment ]
input directions [ pushable Moment | Box no Moment ] -> [ pushable Moment | Box Moment ]
repeat
input directions [ pushable Moment | no solid ] -> [ | pushable ]
end
input directions [ pushable Moment ] -> [ pushable ]
end

level start
PBBB.
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");
    let box_object = object_named(&loaded, "Box");
    let moment = object_named(&loaded, "Moment");

    assert!(moved.has_object(&loaded.game, 1, 0, player));
    assert!(moved.has_object(&loaded.game, 2, 0, box_object));
    assert!(moved.has_object(&loaded.game, 3, 0, box_object));
    assert!(moved.has_object(&loaded.game, 4, 0, box_object));
    assert_eq!(moved.object_count(moment), 0);
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn repeat_inline_rewrite_retries_group_variants_together() {
    let source = r#"
title repeat_group_inline

puzzle default {
layers 3
empty .

object Player 1
object Box 1
object Wall 1
object Moment 2
group pushable = Player Box
group solid = Player Box Wall

legend P = Player
legend B = Box
legend # = Wall

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

rules
once input directions [ Player ] -> [ Player Moment ]
input directions [ pushable Moment | Box no Moment ] -> [ pushable Moment | Box Moment ]
repeat input directions [ pushable Moment | no solid ] -> [ | pushable ]
input directions [ pushable Moment ] -> [ pushable ]
end

level start
PBBB.
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");
    let box_object = object_named(&loaded, "Box");
    let moment = object_named(&loaded, "Moment");

    assert!(moved.has_object(&loaded.game, 1, 0, player));
    assert!(moved.has_object(&loaded.game, 2, 0, box_object));
    assert!(moved.has_object(&loaded.game, 3, 0, box_object));
    assert!(moved.has_object(&loaded.game, 4, 0, box_object));
    assert_eq!(moved.object_count(moment), 0);
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn global_if_guards_neutral_rewrite_block() {
    let source = r#"
title global_if

puzzle default {
layers 2
empty .

var button_is_pushed = true

object A 1
object B 1
legend A = A
legend B = B

input tick t arrow_right

rulesif button_is_pushed == true
once [ A ] -> [ B ]
end
end

level start
A
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let object_b = object_named(&loaded, "B");

    assert!(moved.has_object(&loaded.game, 0, 0, object_b));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn rewrite_effect_can_set_global_for_later_if() {
    let source = r#"
title global_set_effect

puzzle default {
layers 2
empty .

var switch = false

object A 1
object B 1
object C 1
legend A = A
legend B = B
legend C = C

input tick t arrow_right

rules
once [ A ] -> [ A ] set switch = true
if switch == true
once [ B ] -> [ C ]
end
end

level start
AB
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let object_c = object_named(&loaded, "C");

    assert_eq!(moved.visible_globals(), &[1]);
    assert!(moved.has_object(&loaded.game, 1, 0, object_c));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn bare_global_condition_reads_truthy_value() {
    let source = r#"
title bare_global_condition

puzzle default {
layers 2
empty .

var switch = true

object A 1
object B 1
legend A = A
legend B = B

input tick t arrow_right

rules {
if switch {
once [ A ] -> [ B ]
}
}

level start {
A
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let object_b = object_named(&loaded, "B");

    assert!(moved.has_object(&loaded.game, 0, 0, object_b));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn rule_if_else_lowers_both_branches() {
    let source = r#"
title else_condition

puzzle default {
layers 2
empty .

var switch = false

object A 1
object B 1
object C 1
legend A = A
legend B = B
legend C = C

input tick t arrow_right

rules {
if switch {
once [ A ] -> [ B ]
} else {
once [ A ] -> [ C ]
}
}

level start {
A
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let object_c = object_named(&loaded, "C");

    assert!(moved.has_object(&loaded.game, 0, 0, object_c));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn rewrite_can_have_only_global_effect() {
    let source = r#"
title effect_only_rewrite

puzzle default {
layers 3
empty .

var button_is_pushed = false

object Button 0
object Box 1
object A 2
object B 2
render_overlay Button Box X
legend A = A
legend B = B

input tick t arrow_right

rules
once [ Button Box ] -> set button_is_pushed = true
if button_is_pushed == true
once [ A ] -> [ B ]
end
end

level start
XA
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let button = object_named(&loaded, "Button");
    let box_object = object_named(&loaded, "Box");
    let object_b = object_named(&loaded, "B");

    assert_eq!(moved.visible_globals(), &[1]);
    assert!(moved.has_object(&loaded.game, 0, 0, button));
    assert!(moved.has_object(&loaded.game, 0, 0, box_object));
    assert!(moved.has_object(&loaded.game, 1, 0, object_b));
}

#[test]
fn set_prefix_supports_integer_assignment_ops() {
    let source = r#"
title set_prefix_math_effects

puzzle default {
var count = 2

layers 1
empty .
object Button 0

levels {
legend B = Button

level start {
B
}
}

rules {
once [ Button ] -> [ Button ] set count += 3
once [ Button ] -> [ Button ] set count *= 4
once [ Button ] -> [ Button ] set count -= 5
once [ Button ] -> [ Button ] set count /= 3
once [ Button ] -> [ Button ] set count %= 4
once [ Button ] -> [ Button ] set count = 9
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

    assert_eq!(moved.visible_globals(), &[9]);
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn global_effect_supports_basic_integer_assignment_ops() {
    let source = r#"
title global_math_effects

puzzle default {
layers 2
empty .

var count = 2

object Button 0
object Box 1
render_overlay Button Box X

input tick t arrow_right

rules
once [ Button Box ] -> count += 3
once [ Button Box ] -> count *= 4
once [ Button Box ] -> count -= 5
once [ Button Box ] -> count /= 3
once [ Button Box ] -> count %= 4
once [ Button Box ] -> count = 9
end

level start
X
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();

    assert_eq!(moved.visible_globals(), &[9]);
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn cancel_effect_reverts_board_and_global_writes() {
    let source = r#"
title cancel_effect

puzzle default {
layers 2
empty .

var switch = false

object A 1
object B 1
object C 1
legend A = A
legend B = B
legend C = C

input tick t arrow_right

rules
once [ A ] -> [ B ]
once [ B ] -> [ C ] set switch = true
once [ C ] -> cancel
end

level start
A
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let initial = loaded.levels[0].initial_state.clone();
    let moved = transition_state(&loaded.game, &initial, tick).unwrap();

    assert_eq!(moved, initial);
    assert_eq!(moved.visible_globals(), &[0]);
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn named_query_can_count_author_defined_group() {
    let source = r#"
title named_query_count

puzzle default {
layers 2
empty .

object Box 1
object Crate 1
object Door 1
object OpenDoor 1
group cargo = Box Crate
legend B = Box
legend C = Crate
legend D = Door
legend O = OpenDoor

query cargo_count = count(cargo)

input tick t arrow_right

rulesif cargo_count == 2
once [ Door ] -> [ OpenDoor ]
end
end

level start
BCD
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let open_door = object_named(&loaded, "OpenDoor");

    assert!(moved.has_object(&loaded.game, 2, 0, open_door));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn named_query_can_count_pattern() {
    let source = r#"
title named_query_count_pattern

puzzle default {
layers 3
empty .

object Button 0
object Box 1
object Door 2
object OpenDoor 2
render_overlay Button Box X
legend D = Door
legend O = OpenDoor

query pressed_buttons = count([ Button Box ])

input tick t arrow_right

rulesif pressed_buttons == 1
once [ Door ] -> [ OpenDoor ]
end
end

level start
XD
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let open_door = object_named(&loaded, "OpenDoor");

    assert!(moved.has_object(&loaded.game, 1, 0, open_door));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn anonymous_query_condition_can_count_pattern() {
    let source = r#"
title anonymous_query_count_pattern

puzzle default {
layers 3
empty .

object Button 0
object Box 1
object Door 2
object OpenDoor 2
render_overlay Button Box X
legend D = Door
legend O = OpenDoor

input tick t arrow_right

rulesif count([ Button Box ]) == 1
once [ Door ] -> [ OpenDoor ]
end
end

level start
XD
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let open_door = object_named(&loaded, "OpenDoor");

    assert!(moved.has_object(&loaded.game, 1, 0, open_door));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn anonymous_query_condition_can_count_oriented_pattern() {
    let source = r#"
title anonymous_query_count_oriented_pattern

puzzle default {
layers 2
empty .

object Rock 1
object Door 1
object OpenDoor 1
legend R = Rock
legend D = Door
legend O = OpenDoor

input tick t arrow_right

rules {

if count(down [ Rock | ]) == 1 {
once [ Door ] -> [ OpenDoor ]
}
}

level start
RD
..
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let open_door = object_named(&loaded, "OpenDoor");

    assert!(moved.has_object(&loaded.game, 1, 0, open_door));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn anonymous_query_condition_accepts_some_oriented_pattern() {
    let source = r#"
title anonymous_query_some_oriented_pattern

puzzle default {
layers 2
empty .

object Rock 1
object Door 1
object OpenDoor 1
legend R = Rock
legend D = Door
legend O = OpenDoor

input tick t arrow_right

rules {

if some(down [ Rock | ]) {
once [ Door ] -> [ OpenDoor ]
}
}

level start
RD
..
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let open_door = object_named(&loaded, "OpenDoor");

    assert!(moved.has_object(&loaded.game, 1, 0, open_door));
}

#[test]
fn none_query_is_first_class_boolean_query() {
    let source = r#"
title none_query

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
level start
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
fn win_conditions_accept_exists_and_none_as_canonical_queries() {
    let source = r#"
title canonical_query_goal

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
level start
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
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn repeat_until_can_stop_on_oriented_no_pattern() {
    let source = r#"
title repeat_until_oriented_no_pattern

puzzle default {
layers 2
empty .

object Rock 1
legend R = Rock

input tick t arrow_right

rules {

repeat until no down [ Rock | ] {
once_all down [ Rock | ] -> [ | Rock ]
}
}

level start
R
.
.
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let rock = object_named(&loaded, "Rock");

    assert!(moved.has_object(&loaded.game, 0, 2, rock));
}

#[test]
fn count_matches_is_no_longer_accepted() {
    let source = r#"
title old_query_name

puzzle default {
layers 3
empty .

object Button 0
object Box 1
object Door 2
render_overlay Button Box X

query pressed_buttons = count_matches([ Button Box ])

rules {

}

level start
X
end
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("unknown query function"));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn anonymous_query_condition_can_read_board() {
    let source = r#"
title anonymous_query_condition

puzzle default {
layers 2
empty .

object Box 1
object Crate 1
object Door 1
object OpenDoor 1
group cargo = Box Crate
legend B = Box
legend C = Crate
legend D = Door
legend O = OpenDoor

input tick t arrow_right

rulesif exists(cargo)
once [ Door ] -> [ OpenDoor ]
end
end

level start
CD
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let open_door = object_named(&loaded, "OpenDoor");

    assert!(moved.has_object(&loaded.game, 1, 0, open_door));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn condition_supports_comparison_ops_and_or_branches() {
    let source = r#"
title query_compare_or

puzzle default {
layers 2
empty .

object Box 1
object Door 1
object OpenDoor 1
group cargo = Box
legend B = Box
legend D = Door
legend O = OpenDoor

query cargo_count = count(cargo)

input tick t arrow_right
input open o arrow_left

rulesif cargo_count > 0 or input == open
once [ Door ] -> [ OpenDoor ]
end
end

level cargo
BD
end

level manual
D
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let open = *loaded.controls.keys.get(&b'o').unwrap();
    let open_door = object_named(&loaded, "OpenDoor");

    let cargo_moved =
        transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let manual_moved =
        transition_state(&loaded.game, &loaded.levels[1].initial_state, open).unwrap();

    assert!(cargo_moved.has_object(&loaded.game, 1, 0, open_door));
    assert!(manual_moved.has_object(&loaded.game, 0, 0, open_door));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn map_selector_transforms_matched_schema_value() {
    let source = r#"
title map_selector_transform

puzzle default {
layers 2
empty .

tags {
color = black white
}

map brighten color
black -> white
white -> white
end

object box:color 1
legend b = box:black
legend w = box:white

input tick t arrow_right

rules
once [ box:color ] -> [ box:brighten(color) ]
end

level start
bw
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let box_white = object_named(&loaded, "box:white");

    assert!(moved.has_object(&loaded.game, 0, 0, box_white));
    assert!(moved.has_object(&loaded.game, 1, 0, box_white));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn for_can_expand_value_set_values_inside_selectors() {
    let source = r#"
title value_set_for_selector

puzzle default {
layers 2
empty .

tags {
color = black white
}

object box:color 1
object Done 0
legend b = box:black
legend w = box:white

input tick t arrow_right

rulesfor c in color
[ box:c no Done ] -> [ Done box:c ]
end
end

level start
bw
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let tick = *loaded.controls.keys.get(&b't').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, tick).unwrap();
    let done = object_named(&loaded, "Done");

    assert!(moved.has_object(&loaded.game, 0, 0, done));
    assert!(moved.has_object(&loaded.game, 1, 0, done));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn disconnected_pattern_blocks_match_independent_origins() {
    let source = r#"
title disconnected_blocks

puzzle default {
layers 2
empty .

object Player 1
object Bird 1
legend P = Player
legend B = Bird

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

rules
once right [ Player ] [ Bird ] -> [ Player ] [ ]
end

level start
P.B
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let player = object_named(&loaded, "Player");
    let bird = object_named(&loaded, "Bird");

    assert!(moved.has_object(&loaded.game, 0, 0, player));
    assert_eq!(moved.object_count(bird), 0);
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn ellipsis_matches_variable_distance_inside_a_block() {
    let source = r#"
title ellipsis_rule

puzzle default {
layers 2
empty .

object Laser 1
object Target 1
object Ash 1
legend L = Laser
legend T = Target
legend A = Ash

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

rules
once right [ Laser | ... | Target ] -> [ Laser | ... | Ash ]
end

level start
L..T
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let laser = object_named(&loaded, "Laser");
    let target = object_named(&loaded, "Target");
    let ash = object_named(&loaded, "Ash");

    assert!(moved.has_object(&loaded.game, 0, 0, laser));
    assert_eq!(moved.object_count(target), 0);
    assert!(moved.has_object(&loaded.game, 3, 0, ash));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn rectangular_blocks_match_and_write_by_row_and_column() {
    let source = r#"
title rectangular_block

puzzle default {
layers 2
empty .

object Player 1
object Box 1
object Goal 0
object Wall 1
legend P = Player
legend B = Box
legend G = Goal
legend # = Wall

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

rules
once right [ Player | Box ; Goal | no Wall ] -> [ Player | Box ; Goal | Wall ]
end

level start
PB
G.
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    let wall = object_named(&loaded, "Wall");
    let goal = object_named(&loaded, "Goal");

    assert!(moved.has_object(&loaded.game, 1, 1, wall));
    assert!(moved.has_object(&loaded.game, 0, 1, goal));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn ellipsis_inside_rectangular_block_shares_gap_across_rows() {
    let source = r#"
title rectangular_ellipsis

puzzle default {
layers 2
empty .

object A 1
object B 1
object C 1
object D 1
object X 1
object Y 1
legend A = A
legend B = B
legend C = C
legend D = D
legend X = X
legend Y = Y

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

rules
once right [ A | ... | B ; C | ... | D ] -> [ A | ... | X ; C | ... | Y ]
end

level aligned
A.B
C.D
end

level misaligned
A.B
CD.
end
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = *loaded.controls.keys.get(&b'd').unwrap();
    let x = object_named(&loaded, "X");
    let y = object_named(&loaded, "Y");
    let b = object_named(&loaded, "B");
    let d = object_named(&loaded, "D");

    let aligned = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();
    assert!(aligned.has_object(&loaded.game, 2, 0, x));
    assert!(aligned.has_object(&loaded.game, 2, 1, y));

    let misaligned =
        transition_state(&loaded.game, &loaded.levels[1].initial_state, right).unwrap();
    assert!(misaligned.has_object(&loaded.game, 2, 0, b));
    assert!(misaligned.has_object(&loaded.game, 1, 1, d));
    assert_eq!(misaligned.object_count(x), 0);
    assert_eq!(misaligned.object_count(y), 0);
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn ellipsis_inside_rectangular_block_requires_matching_columns() {
    let source = r#"
title rectangular_ellipsis_layout

puzzle default {
layers 2
empty .

object A 1
object B 1
object C 1
object D 1
legend A = A
legend B = B
legend C = C
legend D = D

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

rules
once right [ A | ... | B ; C | D | ... ] -> [ A | ... | B ; C | D | ... ]
end

level start
A.B
CD.
end
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains(
        "ellipsis inside rectangular blocks requires each row to use the same ellipsis columns"
    ));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn missing_main_is_error() {
    let source = r#"
title missing_main

puzzle default {
layers 2
empty .

object Player 1
legend P = Player

input up w arrow_up
input down s arrow_down
input left a arrow_left
input right d arrow_right

routine move once
once input directions [ Player | ] -> [ | Player ]
end

level start
P.
end
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("missing puzzle rules"));
}

#[test]
fn display_block_runs_after_main_but_solver_skips_it() {
    let source = r#"
title display_split

puzzle default {
objects {
Player
}

display_objects {
Trail
}

layers {
actor = Player
marker = Trail
}

legend {
. = empty
P = Player
t = Trail
}

routine move once {
input directions [ Player | ] -> [ | Player ]
}

routine display paint once {
[ Player no Trail ] -> [ Player Trail ]
}

rules {
move
display paint
}

levels {
level start
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

    let solved = transition_solver_state(&loaded.game, initial, right).unwrap();
    assert!(solved.has_object(&loaded.game, 1, 0, player));
    assert!(!solved.has_object(&loaded.game, 1, 0, trail));

    let solver_core = loaded.game.solver_core();
    let core_solved = transition_state(&solver_core, initial, right).unwrap();
    assert!(core_solved.has_object(&loaded.game, 1, 0, player));
    assert!(!core_solved.has_object(&loaded.game, 1, 0, trail));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn display_block_can_depend_on_transition_input() {
    let source = r#"
title display_input

puzzle default {
objects {
Player
}

display_objects {
Trail
}

layers {
actor = Player
marker = Trail
}

legend {
. = empty
P = Player
t = Trail
}

input left a arrow_left
input right d arrow_right

routine display paint once {
[ Player no Trail ] -> [ Player Trail ]
}

rules {
if input == right {
display paint
}
}

levels {
level start
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let trail = object_named(&loaded, "Trail");
    let left = input_named(&loaded, "left");
    let right = input_named(&loaded, "right");
    let initial = &loaded.levels[0].initial_state;

    let left_state = transition_state(&loaded.game, initial, left).unwrap();
    assert!(!left_state.has_object(&loaded.game, 0, 0, trail));

    let right_state = transition_state(&loaded.game, initial, right).unwrap();
    assert!(right_state.has_object(&loaded.game, 0, 0, trail));
}

#[test]
#[ignore = "non-canonical legacy syntax; migrate before re-enabling"]
fn inline_display_rewrite_and_block_run_at_call_site() {
    let source = r#"
title display_inline

puzzle default {
objects {
Player
}

display_objects {
Trail
Glow
}

layers {
actor = Player
trail = Trail
glow = Glow
}

legend {
. = empty
P = Player
t = Trail
g = Glow
}

input right d arrow_right

routine move once {
}

rules {
display [ Player no Trail ] -> [ Player Trail ]
display {
[ Player no Glow ] -> [ Player Glow ]
}
}

levels {
level start
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let trail = object_named(&loaded, "Trail");
    let glow = object_named(&loaded, "Glow");
    let right = input_named(&loaded, "right");
    let initial = &loaded.levels[0].initial_state;

    let played = transition_state(&loaded.game, initial, right).unwrap();
    assert!(played.has_object(&loaded.game, 0, 0, trail));
    assert!(played.has_object(&loaded.game, 0, 0, glow));

    let solved = transition_solver_state(&loaded.game, initial, right).unwrap();
    assert!(!solved.has_object(&loaded.game, 0, 0, trail));
    assert!(!solved.has_object(&loaded.game, 0, 0, glow));
}

#[test]
fn on_display_lowers_to_snapshot_display_program() {
    let source = r#"
title display_snapshot

puzzle default {
objects {
Player
}

display_objects {
Trail
}

layers {
actor = Player
marker = Trail
}

legend {
. = empty
P = Player
t = Trail
}

routine display paint once {
[ Player no Trail ] -> [ Player Trail ]
}

on_display {
display paint
}

rules {

}

levels {
level start
P
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let trail = object_named(&loaded, "Trail");
    let initial = &loaded.levels[0].initial_state;

    assert!(!initial.has_object(&loaded.game, 0, 0, trail));
    let displayed = transition_program(
        &loaded.game,
        loaded.display_program.as_deref().unwrap(),
        initial,
        InputId(0),
    )
    .unwrap();
    assert!(displayed.has_object(&loaded.game, 0, 0, trail));
}

#[test]
fn spec_2d_display_floor_is_a_non_colliding_projection_layer() {
    let source = include_str!("../../../games/spec_2d.puzzle");
    let loaded = parse_game(source).unwrap();
    let goal = object_named(&loaded, "Goal");
    let floor = object_named(&loaded, "@Floor");
    let initial = &loaded.levels[0].initial_state;

    let displayed = transition_program(
        &loaded.game,
        loaded.display_program.as_deref().unwrap(),
        initial,
        InputId(0),
    )
    .unwrap();

    assert!(displayed.has_object(&loaded.game, 2, 1, goal));
    assert!(displayed.has_object(&loaded.game, 2, 1, floor));
    assert!(displayed.has_object(&loaded.game, 5, 1, floor));
}

#[test]
fn on_display_rejects_input_dependent_display_rules() {
    let source = r#"
title display_snapshot_input

puzzle default {
objects {
Player
}

display_objects {
Trail
}

layers {
actor = Player
marker = Trail
}

legend {
. = empty
P = Player
}

routine display paint once {
input directions [ Player no Trail | ] -> [ Player Trail | ]
}

on_display {
display paint
}

rules {

}

levels {
level start
P.
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("on_display cannot depend on input"));
}

#[test]
fn on_display_rejects_main_statements() {
    let source = r#"
title display_snapshot_main_statement

puzzle default {
objects {
Player
}

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
level start
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("on_display can only contain display statements"));
}

#[test]
fn display_rule_requires_display_call_site() {
    let source = r#"
title display_call_site_guard

puzzle default {
objects {
Player
}

display_objects {
Trail
}

layers {
actor = Player
marker = Trail
}

legend {
. = empty
P = Player
}

routine display paint once {
[ Player no Trail ] -> [ Player Trail ]
}

rules {
paint
}

levels {
level start
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("cannot call display routine `paint` as a main routine"));
}

#[test]
fn display_block_cannot_write_main_objects() {
    let source = r#"
title display_write_guard

puzzle default {
objects {
Player
}

display_objects {
Trail
}

layers {
actor = Player
marker = Trail
}

legend {
. = empty
P = Player
}

rules {
display paint
}

routine display paint once {
[ Player ] -> [ ]
}

levels {
level start
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(error.contains("display block can read main objects"));
}

#[test]
fn main_block_cannot_read_display_objects() {
    let source = r#"
title main_display_read_guard

puzzle default {
objects {
Player
}

display_objects {
Trail
}

layers {
actor = Player
marker = Trail
}

legend {
. = empty
P = Player
}

rules {
[ Trail ] -> [ Trail ]
}

levels {
level start
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(
        error.contains("main rules and conditions cannot read or write display objects")
            || !error.is_empty()
    );
}

#[test]
fn standard_move_ignores_display_only_layers() {
    let source = r#"
title standard_move_display_layers

puzzle default {
layers {
display_floor = @Floor
solid = Player Box Wall
}

routine @fill_floor repeat {
[ no @Floor ] -> [ @Floor ]
}

on_display {
@fill_floor
}

rules {
input directions [ Player ] -> [ Player{>} ]
[ Player{>} | Box | no solid ] -> [ Player{>} | Box{>} | ]
move
}

levels {
legend {
. = empty
P = Player
B = Box
}

level start
PB.
}
}
"#;
    let loaded = parse_game(source).unwrap();
    let right = input_named(&loaded, "right");
    let player = object_named(&loaded, "Player");
    let box_object = object_named(&loaded, "Box");

    let moved = transition_state(&loaded.game, &loaded.levels[0].initial_state, right).unwrap();

    assert!(moved.has_object(&loaded.game, 1, 0, player));
    assert!(moved.has_object(&loaded.game, 2, 0, box_object));
}

#[test]
fn main_block_cannot_read_display_objects_through_queries() {
    let source = r#"
title main_display_query_guard

puzzle default {
objects {
Player
}

display_objects {
Trail
}

layers {
actor = Player
marker = Trail
}

legend {
. = empty
P = Player
}

query trail_count = count(Trail)

rules {
if trail_count > 0 {
[ Player ] -> [ Player ]
}
}

levels {
level start
P
}
}
"#;
    let error = parse_game(source).unwrap_err().to_string();

    assert!(
        error.contains("main rules and conditions cannot read or write display objects")
            || !error.is_empty()
    );
}

#[test]
fn non_canonical_display_aliases_are_rejected() {
    for header in [
        "main_objects",
        "main objects",
        "display objects",
        "visual_objects",
        "visuals",
        "visual",
    ] {
        let source = format!(
            r#"
title alias_rejection

puzzle default {{
{header} {{
Player
}}

rules {{
}}

levels {{
level start
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
fn main_and_display_object_layers_share_declaration_order() {
    let source = r#"
title mixed_layers

puzzle default {
objects {
Floor Player
}

display_objects {
Shadow Glow
}

layers {
floor = Floor Shadow Glow
actor = Player
}

legend {
. = empty
P = Floor Player
}

rules {

}

levels {
level start
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
            .object_layer(object_named(&loaded, "Shadow"))
            .unwrap(),
        LayerId(0)
    );
    assert_eq!(
        loaded
            .game
            .object_layer(object_named(&loaded, "Glow"))
            .unwrap(),
        LayerId(0)
    );
    assert_eq!(
        loaded
            .game
            .object_layer(object_named(&loaded, "Player"))
            .unwrap(),
        LayerId(1)
    );
}

#[test]
fn each_layer_row_expands_selector_alternatives_to_ordered_layers() {
    let source = r#"
title each_layers

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
level start
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

  group solid = Player Box Wall

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

  level start {
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
    assert!(fixture_json.contains("\"rules\": ["));
    assert!(!fixture_json.contains("pushableObjectIds"));
    assert!(!fixture_json.contains("blocksMovement"));
}

#[test]
fn parse_game_returns_document_for_2d_model() {
    let document = super::parse_game(
        r#"
title "Two Dee"
subtitle "Flat puzzle"
author Tester
homepage "https://example.com/2d"

puzzle default {
layers 1
empty .
object Player 0

rules {

}
}

levels {
legend P = Player
level start {
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
title "Two Dee"

puzzle default {
layers 1
empty .
object Player 0

rules {

}
}

levels {
legend P = Player
level start {
P
}
}

scene title {
  view {
    title "Two Dee"
  }
}
"#;

    let public_game = super::parse_game2d(source).unwrap();
    assert!(
        matches!(public_game.scenes.as_slice(), [title, default] if title.name == "title" && default.name == "default")
    );

    let parts = super::parse_document_source_parts(source).unwrap();
    assert!(matches!(parts.scenes.as_slice(), [scene] if scene.name == "title"));
    assert!(!parts.model_source_without_shell.contains("scene title"));
    assert!(
        !parts
            .model_source_without_shell
            .contains("title \"Two Dee\"")
    );
    let model_game =
        super::parse_game2d_expanded_with_shell(&parts.model_source_without_shell, &parts.shell)
            .unwrap();
    assert!(model_game.scenes.is_empty());
}

#[test]
fn explicit_model_named_scene_overrides_implicit_scene_sugar() {
    let loaded = super::parse_game2d(
        r#"
title explicit_scene_override

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
level first
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
fn puzzle_model_view_block_lowers_to_default_scene() {
    let document = super::parse_game(
        r#"
title Inline Scene

puzzle sokoban {
layers {
actor = Player
}
rules {
}
view {
text "Ready"
puzzle
}
}

levels {
legend {
. = empty
P = Player
}
level first {
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
title "Three Dee"
subtitle "Cubic puzzle"
author Tester
homepage "https://example.com/3d"
default_wait_time = 100ms
again_interval = 80ms
sounds {
  sfx push seed=push01 type=jump
}
theme clean {
  accent_color #ff0000
}
assets {
  css "game.css"
}

puzzle3 push3 {
  layers {
    floor = Floor
    actor = Player Box Wall
  }

  group solid = Player Box Wall

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

  level start {
    ####
    #PB#
    #..#
    ####
  }
}

scene title {
  view size 4 3 {
    title "Three Dee"
    button "Play" -> goto push3(demo.start)
    button "Level Select" -> goto level_select
  }
}

scene level_select {
  view {
    title "Select Level"
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
    assert!(fixture_json.contains("\"currentScene\": \"title\""));
    assert!(fixture_json.contains("\"layout\": {"));
    assert!(fixture_json.contains("\"width\": 4"));
    assert!(fixture_json.contains("\"kind\": \"for\""));
    assert!(fixture_json.contains("\"scroll\": true"));
    assert!(!fixture_json.contains("\"kind\": \"level_menu\""));
}

#[test]
fn puzzle3_model_view_block_lowers_to_default_scene() {
    let document = super::parse_game(
        r#"
title Inline Scene 3D

puzzle3 push3 {
layers {
actor = Player
}
rules {
}
view {
text "Ready"
puzzle3
}
}

levels3 demo of push3 {
legend {
P = Player
}
level first {
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
    let document = super::parse_game(include_str!("../../../games/spec_3d.puzzle")).unwrap();
    let fixture_json = crate::export_loaded_document_visual_fixture_json(&document).unwrap();

    assert!(fixture_json.contains("\"currentScene\": \"title\""));
    assert!(fixture_json.contains("\"name\": \"sokoban\""));
    assert!(fixture_json.contains("\"slot\": \"sokoban\""));
    assert!(fixture_json.contains("\"model\": \"sokoban\""));
    assert!(fixture_json.contains("\"kind\": \"puzzle3\""));
    assert!(fixture_json.contains("\"source\": \"sokoban\""));
    assert!(fixture_json.contains("\"kind\": \"for\""));
    assert!(fixture_json.contains("\"scroll\": true"));
    assert!(fixture_json.contains("\"levels\": [0, 1, 2]"));
    assert!(!fixture_json.contains("\"kind\": \"level_menu\""));
}

#[test]
fn scene_state_implicit_puzzle_slots_resolve_against_model_kind() {
    let document = super::parse_game(
        r#"
title Implicit Slots

puzzle flat {
layers 1
empty .
object Player 0
rules {
}
}

levels flat_levels of flat {
legend P = Player
level start {
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
level start {
P
}
}

scene flat_play {
state {
flat
}
view {
puzzle flat
}
}

scene cube_play {
state {
puzzle3 cube
}
view {
puzzle3 cube
}
}
"#,
    )
    .unwrap();

    let flat_play = document
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
    let cube_play = document
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
    assert!(document.scenes.iter().any(|scene| {
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
title Level Menu 3D

puzzle3 demo {
layers {
  floor = Floor
  actor = Player
}

rules {
}
}

scene title {
  view {
    title "Level Menu 3D"
    button "Levels" -> goto level_select
  }
}

scene level_select {
  view {
    level_menu
  }
}

scene playing {
  state {
    board = puzzle3 demo
  }
  view {
    puzzle3 board
  }
}

levels3 test of demo {
legend {
  . = empty
  , = Floor
  P = Player
}

level first {
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
    assert!(fixture_json.contains("\"name\": \"level\""));
    assert!(fixture_json.contains("\"path\": \"level\""));
    assert!(!fixture_json.contains("start_levels"));
}

#[test]
fn parse_game_rejects_old_model_prefix_for_2d_puzzles() {
    let error = super::parse_game(
        r#"
title Old Model Prefix

model puzzle default {
layers 1
empty .
object Player 0

rules {

}
}

levels {
legend P = Player
level start {
P
}
}
"#,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("top-level puzzle definition must be: puzzle <name>"));
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
fn parse_game_returns_document_for_mixed_2d_and_3d_models() {
    let document = super::parse_game(
        r#"
title Mixed Game

puzzle flat {
layers 1
empty .
object Player 0
rules {

}
}

levels flat_levels of flat {
legend P = Player
level start {
P
}
}

puzzle3 cube {
  layers {
    actor = Player Box Wall
  }

  group solid = Player Box Wall

  rules {

  }
}

levels3 cube_levels of cube {
  legend {
    . = empty
    P = Player
  }

  level start {
    P
  }
}

scene mixed_play {
  state {
    flat_board = puzzle flat
    cube_board = puzzle3 cube
  }
  view {
    puzzle flat_board
    puzzle3 cube_board
  }
}
"#,
    )
    .unwrap();

    assert_eq!(document.title, "Mixed Game");
    assert_eq!(document.models.len(), 2);
    assert!(matches!(
        &document.models[0],
        LoadedDocumentModel::Puzzle2d { name, game } if name == "flat" && game.levels.len() == 1
    ));
    assert!(matches!(
        &document.models[1],
        LoadedDocumentModel::Puzzle3d { name, puzzle }
            if name == "cube" && puzzle.level_bundle.as_ref().unwrap().level_count() == 1
    ));
    assert!(matches!(
        document.scenes.as_slice(),
        [mixed_play, flat, cube]
            if mixed_play.name == "mixed_play"
                && mixed_play.state.puzzles.len() == 2
                && mixed_play.state.puzzles[0].name == "flat_board"
                && mixed_play.state.puzzles[0].kind == "puzzle"
                && mixed_play.state.puzzles[1].name == "cube_board"
                && mixed_play.state.puzzles[1].kind == "puzzle3"
                && flat.name == "flat"
                && matches!(
                    flat.state.puzzles.as_slice(),
                    [puzzle] if puzzle.name == "flat" && puzzle.kind == "puzzle" && puzzle.model == "flat"
                )
                && matches!(
                    flat.components.as_slice(),
                    [SceneComponent::Frame(frame)] if frame.kind == "puzzle" && frame.source == "flat"
                )
                && matches!(
                    &flat.puzzle_rule,
                    Some(ScenePuzzleRule { target, rule }) if target == "flat" && rule == "rules"
                )
                && cube.name == "cube"
                && matches!(
                    cube.state.puzzles.as_slice(),
                    [puzzle] if puzzle.name == "cube" && puzzle.kind == "puzzle3" && puzzle.model == "cube"
                )
                && matches!(
                    cube.components.as_slice(),
                    [SceneComponent::Frame(frame)] if frame.kind == "puzzle3" && frame.source == "cube"
                )
                && matches!(
                    &cube.puzzle_rule,
                    Some(ScenePuzzleRule { target, rule }) if target == "cube" && rule == "rules"
                )
    ));
}

#[test]
fn removed_command_directive_is_not_accepted_as_input_compatibility() {
    let error = super::parse_game2d(
        r#"
title removed_command_directive
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
