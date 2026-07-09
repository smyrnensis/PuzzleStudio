#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticKind {
    Keyword,
    Literal,
    Binding,
    Effect,
    Emission,
    Object,
    Input,
    State,
    Group,
    Mark,
    Variant,
    Condition,
    Scene,
    Theme,
    Asset,
    Setting,
    Color,
    Number,
    String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticToken {
    pub start: usize,
    pub end: usize,
    pub kind: SemanticKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SemanticCompletionContext {
    pub(crate) replace_start: usize,
    pub(crate) replace_end: usize,
    pub(crate) token_text: String,
    pub(crate) slots: Vec<SemanticCompletionSlot>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SemanticCompletionSlot {
    Keywords(&'static [&'static str]),
    ModelTopLevelKeywords,
    Literals(&'static [&'static str]),
    Objects,
    Groups,
    States,
    Markes,
    ObjectNameAtoms,
    ValueSets,
    Directions,
    DirectionSets,
    Inputs,
    StandardRuleSteps,
    ModelEffects,
    SceneEffects,
    Emissions,
    Routines,
    Conditions,
    Scenes,
    Puzzles,
    SfxAssets,
    MusicAssets,
    Sprites,
    Assets,
    Shapes,
    Themes,
    Colors,
    AuthoringRows(crate::authoring_grammar::AuthoringKind),
    AuthoringChildren(crate::authoring_grammar::AuthoringKind),
    AuthoringContentRows(crate::authoring_grammar::AuthoringContentKind),
    Settings(SettingCompletionSet),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingCompletionSet {
    Static(&'static [&'static str]),
    AuthoringDefinitions(crate::authoring_grammar::AuthoringKind),
}

pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    crate::surface_document_semantics(source).tokens
}

#[cfg(test)]
mod tests {
    use super::{SemanticKind, SemanticToken, semantic_tokens};

    #[test]
    fn classifies_same_word_by_source_scope() {
        let source = r#"
title = semantic_scope
sounds {
sfx clear { seed = clear01; type = jump }
}
scene playing {
rules {
win -> sfx clear
}
}
"#;
        let tokens = semantic_tokens(source);
        let sounds_sfx_start = source.find("sfx clear").unwrap();
        let scene_sfx_start = source.rfind("sfx clear").unwrap();

        assert!(tokens.iter().any(|token| {
            token.start == sounds_sfx_start
                && token.end == sounds_sfx_start + "sfx".len()
                && token.kind == SemanticKind::Keyword
        }));
        assert!(tokens.iter().any(|token| {
            token.start == scene_sfx_start
                && token.end == scene_sfx_start + "sfx".len()
                && token.kind == SemanticKind::Effect
        }));
    }

    #[test]
    fn classifies_authoring_schema_surface_tokens() {
        let source = r#"
title = authoring_schema_semantics
theme = "clean"
puzzle main {
render {
tween = true
tween_duration = 90ms
}
}
sounds {
sfx clear {
seed = clear01
type = jump
}
music loop {
seed = loop01
bars = 4
}
}
"#;
        let tokens = semantic_tokens(source);

        let title_start = source.find("title").unwrap();
        let title_value_start = source.find("authoring_schema_semantics").unwrap();
        let clean_start = source.find("clean").unwrap();
        let tween_duration_start = source.find("tween_duration").unwrap();
        let ms90_start = source.find("90ms").unwrap();
        let sfx_start = source.find("sfx clear").unwrap();
        let clear_start = sfx_start + "sfx ".len();
        let seed_start = source.find("seed = clear01").unwrap();
        let clear_seed_start = source.find("clear01").unwrap();
        let bars_start = source.find("bars = 4").unwrap();
        let four_start = source.find("4").unwrap();

        assert_semantic_token(source, &tokens, title_start, "title", SemanticKind::Keyword);
        assert_semantic_token(
            source,
            &tokens,
            title_value_start,
            "authoring_schema_semantics",
            SemanticKind::String,
        );
        assert_semantic_token(source, &tokens, clean_start, "clean", SemanticKind::Theme);
        assert_semantic_token(
            source,
            &tokens,
            tween_duration_start,
            "tween_duration",
            SemanticKind::Setting,
        );
        assert_semantic_token(source, &tokens, ms90_start, "90ms", SemanticKind::Number);
        assert_semantic_token(source, &tokens, sfx_start, "sfx", SemanticKind::Keyword);
        assert_semantic_token(source, &tokens, clear_start, "clear", SemanticKind::Asset);
        assert_semantic_token(source, &tokens, seed_start, "seed", SemanticKind::Setting);
        assert_semantic_token(
            source,
            &tokens,
            clear_seed_start,
            "clear01",
            SemanticKind::String,
        );
        assert_semantic_token(source, &tokens, bars_start, "bars", SemanticKind::Setting);
        assert_semantic_token(source, &tokens, four_start, "4", SemanticKind::Number);
    }

    #[test]
    fn classifies_rewrite_effects_from_parser_owned_tokens() {
        let source = r#"
title = rewrite_effect_semantics
puzzle default {
rules {
once [ Player ] -> [ Player ] sfx clear
score = 1
}
}
"#;
        let tokens = semantic_tokens(source);
        let sfx_start = source.find("sfx clear").unwrap();
        let clear_start = source.find("clear").unwrap();
        let score_start = source.find("score").unwrap();

        assert!(tokens.iter().any(|token| {
            token.start == sfx_start
                && token.end == sfx_start + "sfx".len()
                && token.kind == SemanticKind::Effect
        }));
        assert!(tokens.iter().any(|token| {
            token.start == clear_start
                && token.end == clear_start + "clear".len()
                && token.kind == SemanticKind::Asset
        }));
        assert!(tokens.iter().any(|token| {
            token.start == score_start
                && token.end == score_start + "score".len()
                && token.kind == SemanticKind::State
        }));
    }

    #[test]
    fn classifies_rewrite_pattern_selectors_from_parser_owned_tokens() {
        let source = r#"
title = rewrite_selector_semantics
puzzle board {
layers {
actor = Player Box
}
rules {
[ Player | Box ] -> [ Box | Player ]
}
}
"#;
        let tokens = semantic_tokens(source);
        let player_start = source.find("Player |").unwrap();
        let box_start = source.find("Box ]").unwrap();

        assert!(tokens.iter().any(|token| {
            token.start == player_start
                && token.end == player_start + "Player".len()
                && token.kind == SemanticKind::Object
        }));
        assert!(tokens.iter().any(|token| {
            token.start == box_start
                && token.end == box_start + "Box".len()
                && token.kind == SemanticKind::Object
        }));
    }

    #[test]
    fn classifies_implicit_level_event_sugar_from_parser_owned_tokens() {
        let source = r#"
title = implicit_level_event_semantics

puzzle board {
layers {
actor = Player
}
legend P = Player
rules {
}
}

levels {
legend {
. = empty
P = Player
}

message "one"
message "two"
P
}
"#;
        let tokens = semantic_tokens(source);
        let first_message_start = source.find("message \"one\"").unwrap();
        let second_message_start = source.find("message \"two\"").unwrap();

        for start in [first_message_start, second_message_start] {
            assert!(tokens.iter().any(|token| {
                token.start == start
                    && token.end == start + "message".len()
                    && token.kind == SemanticKind::Emission
            }));
        }
    }

    #[test]
    fn classifies_standard_move_call_as_routine_effect() {
        let source = r#"
title = standard_move_semantics

puzzle board {
layers {
actor = Player
}
rules {
move
}
}
"#;
        let tokens = semantic_tokens(source);
        let move_start = source.find("move\n").unwrap();

        assert!(tokens.iter().any(|token| {
            token.start == move_start
                && token.end == move_start + "move".len()
                && token.kind == SemanticKind::Effect
        }));
    }

    #[test]
    fn classifies_parser_owned_rewrite_prefixes_as_keywords() {
        let source = r#"
title = rewrite_prefix_semantics

puzzle board {
layers {
actor = Player Wall
}
rules {
input [ Player | ] -> [ | Player ]
input directions [ Player | ] -> [ | Player ]
once input [ Player | ] -> [ | Player ]
once input directions [ Player | ] -> [ | Player ]
input horizontal [ Player | ] -> [ | Player ]
input [ Player | Wall ] -> push_player
input directions [ Player | Wall ] -> push_player
if some(input directions [ Player | Wall ]) {
[ Player ] -> [ Player ]
}
if some(input [ Player | Wall ]) {
[ Player ] -> [ Player ]
}
random left [ Player | ] -> [ | Player ]
routine push_player {
[ Player ] -> [ Player ]
}
}
}
"#;
        let tokens = semantic_tokens(source);
        let input_start = source.find("input [").unwrap();
        let input_directions_start = source.find("input directions").unwrap();
        let once_input_start = source.find("once input").unwrap() + "once ".len();
        let once_input_directions_start =
            source.find("once input directions").unwrap() + "once ".len();
        let input_horizontal_start = source.find("input horizontal").unwrap();
        let conditional_call_input_start = source.find("input [ Player | Wall ]").unwrap();
        let conditional_call_input_directions_start =
            source.find("input directions [ Player | Wall ]").unwrap();
        let condition_input_start = source.rfind("input directions [ Player | Wall ]").unwrap();
        let condition_directions_start = condition_input_start + "input ".len();
        let condition_shorthand_input_start = source.rfind("input [ Player | Wall ]").unwrap();
        let random_left_start = source.find("random left").unwrap() + "random ".len();

        for (start, text) in [
            (input_start, "input"),
            (input_directions_start, "input"),
            (once_input_start, "input"),
            (once_input_directions_start, "input"),
            (input_horizontal_start, "input"),
            (conditional_call_input_start, "input"),
            (conditional_call_input_directions_start, "input"),
            (condition_input_start, "input"),
            (condition_directions_start, "directions"),
            (condition_shorthand_input_start, "input"),
            (random_left_start, "left"),
        ] {
            assert!(tokens.iter().any(|token| {
                token.start == start
                    && token.end == start + text.len()
                    && token.kind == SemanticKind::Keyword
            }));
        }
    }

    #[test]
    fn preserves_all_parser_surface_semantic_tokens() {
        let source = r#"
title = surface_semantic_projection

puzzle board {
layers {
actor = Player Wall
}
rules {
input [ Player | ] -> [ | Player ]
input directions [ Player | ] -> [ | Player ]
once input [ Player | ] -> [ | Player ]
once input directions [ Player | ] -> [ | Player ]
input [ Player | Wall ] -> push_player
input directions [ Player | Wall ] -> push_player
if some(input [ Player | Wall ]) {
[ Player ] -> [ Player ]
}
if some(input directions [ Player | Wall ]) {
[ Player ] -> [ Player ]
}
routine push_player {
[ Player ] -> [ Player ]
}
}
}
"#;
        let surface_tokens = crate::surface_document_semantics(source).tokens;
        let semantic_tokens = semantic_tokens(source);
        assert!(
            !surface_tokens.is_empty(),
            "fixture must exercise parser-owned surface tokens"
        );
        for surface_token in surface_tokens {
            assert!(
                semantic_tokens.contains(&surface_token),
                "semantic tokens must preserve parser-owned surface token {surface_token:?}"
            );
        }
    }

    #[test]
    fn classifies_anonymous_layer_entries_as_objects() {
        let source = r#"
title = anonymous_layer_semantics

puzzle board {
layers {
Floor
Goal
solid = Player Box Wall
}
}
"#;
        let tokens = semantic_tokens(source);
        let floor_start = source.find("Floor").unwrap();
        let goal_start = source.find("Goal").unwrap();
        let solid_start = source.find("solid =").unwrap();
        let player_start = source.find("Player").unwrap();

        assert!(tokens.iter().any(|token| {
            token.start == floor_start
                && token.end == floor_start + "Floor".len()
                && token.kind == SemanticKind::Object
        }));
        assert!(tokens.iter().any(|token| {
            token.start == goal_start
                && token.end == goal_start + "Goal".len()
                && token.kind == SemanticKind::Object
        }));
        assert!(tokens.iter().any(|token| {
            token.start == solid_start
                && token.end == solid_start + "solid".len()
                && token.kind == SemanticKind::Group
        }));
        assert!(tokens.iter().any(|token| {
            token.start == player_start
                && token.end == player_start + "Player".len()
                && token.kind == SemanticKind::Object
        }));
    }

    #[test]
    fn classifies_tag_set_definitions_as_groups_and_variants() {
        let source = r#"
title = tag_semantics

puzzle board {
tags {
color = red blue
facing = left right
}
}
"#;
        let tokens = semantic_tokens(source);
        let color_start = source.find("color =").unwrap();
        let red_start = source.find("red").unwrap();
        let left_start = source.find("left").unwrap();

        assert!(tokens.iter().any(|token| {
            token.start == color_start
                && token.end == color_start + "color".len()
                && token.kind == SemanticKind::Group
        }));
        assert!(tokens.iter().any(|token| {
            token.start == red_start
                && token.end == red_start + "red".len()
                && token.kind == SemanticKind::Variant
        }));
        assert!(tokens.iter().any(|token| {
            token.start == left_start
                && token.end == left_start + "left".len()
                && token.kind == SemanticKind::Variant
        }));
    }

    #[test]
    fn classifies_theme_state_and_condition_contexts() {
        let source = r#"
title = semantic_contexts
theme = "clean"
var count = 1

scene playing {
if win_conditions -> goto title
if board.win_conditions -> goto title
}
"#;
        let tokens = semantic_tokens(source);
        let theme_start = source.find("\"clean\"").unwrap() + 1;
        let count_start = source.find("count").unwrap();
        let win_start = source.find("win_conditions").unwrap();
        let path_win_start = source.rfind("win_conditions").unwrap();

        assert!(tokens.iter().any(|token| {
            token.start == theme_start
                && token.end == theme_start + "clean".len()
                && token.kind == SemanticKind::Theme
        }));
        assert!(tokens.iter().any(|token| {
            token.start == count_start
                && token.end == count_start + "count".len()
                && token.kind == SemanticKind::State
        }));
        assert!(tokens.iter().any(|token| {
            token.start == win_start
                && token.end == win_start + "win_conditions".len()
                && token.kind == SemanticKind::Condition
        }));
        assert!(tokens.iter().any(|token| {
            token.start == path_win_start
                && token.end == path_win_start + "win_conditions".len()
                && token.kind == SemanticKind::Condition
        }));
    }

    #[test]
    fn classifies_authoring_schema_projection_tokens() {
        let source = r##"
title = authoring_schema_semantics

theme {
preset = "clean"
background_color = #112233
}

sounds {
sfx clear {
seed = clear01
volume = 0.5
}
undo -> sfx clear
}

input_buffer {
queue_during_wait = false
}

assets {
"game.css"
}

puzzle board {
layers {
actor = Player
}
render {
cell_size = 64
grid {
type = "all_cells"
}
}
}

puzzle3 board3 {
render {
shade = true
camera {
yaw = 90
interactive_look = true
}
grid {
type = "occupied_cells"
}
pixelate {
enabled = true
scale = 4
smoothing = false
}
}
}
"##;
        let tokens = semantic_tokens(source);
        let has = |needle: &str, kind: SemanticKind| {
            let start = source.find(needle).unwrap();
            tokens.iter().any(|token| {
                token.start == start && token.end == start + needle.len() && token.kind == kind
            })
        };

        assert!(has("preset", SemanticKind::Setting));
        assert!(has("clean", SemanticKind::Theme));
        assert!(has("sfx", SemanticKind::Keyword));
        assert!(has("clear", SemanticKind::Asset));
        let undo_start = source.find("undo ->").unwrap();
        let undo_sfx_start = source.rfind("sfx clear").unwrap();
        let undo_clear_start = undo_sfx_start + "sfx ".len();
        assert_semantic_token(source, &tokens, undo_start, "undo", SemanticKind::Keyword);
        assert_semantic_token(
            source,
            &tokens,
            undo_sfx_start,
            "sfx",
            SemanticKind::Keyword,
        );
        assert_semantic_token(
            source,
            &tokens,
            undo_clear_start,
            "clear",
            SemanticKind::Asset,
        );
        assert!(has("seed", SemanticKind::Setting));
        assert!(has("clear01", SemanticKind::String));
        assert!(has("volume", SemanticKind::Setting));
        assert!(has("0.5", SemanticKind::Number));
        assert!(has("queue_during_wait", SemanticKind::Setting));
        assert!(has("false", SemanticKind::Literal));
        assert!(has("game.css", SemanticKind::String));
        assert!(has("cell_size", SemanticKind::Setting));
        assert!(has("64", SemanticKind::Number));
        assert!(has("type", SemanticKind::Setting));
        assert!(has("all_cells", SemanticKind::Literal));
        assert!(has("shade", SemanticKind::Setting));
        assert!(has("true", SemanticKind::Literal));
        assert!(has("yaw", SemanticKind::Setting));
        assert!(has("interactive_look", SemanticKind::Setting));
        assert!(has("occupied_cells", SemanticKind::Literal));
        assert!(has("enabled", SemanticKind::Setting));
        assert!(has("scale", SemanticKind::Setting));
        assert!(has("smoothing", SemanticKind::Setting));
        assert!(has("false", SemanticKind::Literal));
        assert!(has("#112233", SemanticKind::Color));
    }

    #[test]
    fn classifies_scene_step_rule_target() {
        let source = r#"
title = scene_step_semantics

scene playing {
rules {
step board
}
}
"#;
        let tokens = semantic_tokens(source);
        let step_start = source.find("step board").unwrap();
        let board_start = step_start + "step ".len();

        assert!(tokens.iter().any(|token| {
            token.start == step_start
                && token.end == step_start + "step".len()
                && token.kind == SemanticKind::Keyword
        }));
        assert!(tokens.iter().any(|token| {
            token.start == board_start
                && token.end == board_start + "board".len()
                && token.kind == SemanticKind::State
        }));
    }

    #[test]
    fn classifies_same_spelling_by_surface_role() {
        let source = r#"
title = semantic_surface_roles

scene title {
layout {
title = title
}
"#;
        let tokens = semantic_tokens(source);
        let metadata_title_start = source.find("title = semantic_surface_roles").unwrap();
        let scene_title_start = source.find("scene title").unwrap() + "scene ".len();
        let component_title_start = source.rfind("title = title").unwrap();

        assert!(tokens.iter().any(|token| {
            token.start == metadata_title_start
                && token.end == metadata_title_start + "title".len()
                && token.kind == SemanticKind::Keyword
        }));
        assert!(tokens.iter().any(|token| {
            token.start == scene_title_start
                && token.end == scene_title_start + "title".len()
                && token.kind == SemanticKind::Scene
        }));
        assert!(tokens.iter().any(|token| {
            token.start == component_title_start
                && token.end == component_title_start + "title".len()
                && token.kind == SemanticKind::Keyword
        }));
    }

    #[test]
    fn classifies_visual_shape_refs_by_visual_grammar_slots() {
        let source = r#"
title = visual_shape_semantics

puzzle board {
tags {
kind = A B
}
layers {
actor = Block:kind
}
sprites {
shapes {
Block:kind {
A {
0
}
B {
0
}
}
}
Block:kind {
#111
shape Block:kind
}
}
}
"#;
        let tokens = semantic_tokens(source);
        let shape_table_start = source.find("shapes {\nBlock:kind").unwrap() + "shapes {\n".len();
        let shape_ref_start = source.rfind("Block:kind").unwrap();
        let shape_value_start = shape_ref_start + "Block:".len();

        assert!(tokens.iter().any(|token| {
            token.start == shape_table_start
                && token.end == shape_table_start + "Block".len()
                && token.kind == SemanticKind::Asset
        }));
        assert!(tokens.iter().any(|token| {
            token.start == shape_table_start + "Block:".len()
                && token.end == shape_table_start + "Block:kind".len()
                && token.kind == SemanticKind::Group
        }));
        assert!(tokens.iter().any(|token| {
            token.start == shape_ref_start
                && token.end == shape_ref_start + "Block".len()
                && token.kind == SemanticKind::Asset
        }));
        assert!(tokens.iter().any(|token| {
            token.start == shape_value_start
                && token.end == shape_value_start + "kind".len()
                && token.kind == SemanticKind::Variant
        }));
    }

    #[test]
    fn classifies_rule_selectors_from_parser_resolved_surface_tokens() {
        let source = r#"
title = selector_parser_resolved_surface_tokens

puzzle board {
layers {
each A:directions
}
groups {
movers = A:left
}
rules {
once [ movers | A:directions ] -> [ A:left | movers ]
}
levels {
legend {
. = empty
L = A:left
}
level "start" {
.
}
}
}
"#;
        crate::parse_game2d(source).unwrap();
        let tokens = semantic_tokens(source);
        let rule_start = source.find("once [ movers").unwrap();
        let movers_start = rule_start + "once [ ".len();
        let axis_selector_start = source[rule_start..].find("A:directions").unwrap() + rule_start;
        let axis_start = axis_selector_start + "A:".len();
        let concrete_selector_start = source[rule_start..].find("A:left").unwrap() + rule_start;
        let concrete_value_start = concrete_selector_start + "A:".len();

        assert_semantic_token(source, &tokens, movers_start, "movers", SemanticKind::Group);
        assert_semantic_token(
            source,
            &tokens,
            axis_selector_start,
            "A",
            SemanticKind::Object,
        );
        assert_semantic_token(
            source,
            &tokens,
            axis_start,
            "directions",
            SemanticKind::Group,
        );
        assert_semantic_token(
            source,
            &tokens,
            concrete_selector_start,
            "A",
            SemanticKind::Object,
        );
        assert_semantic_token(
            source,
            &tokens,
            concrete_value_start,
            "left",
            SemanticKind::Variant,
        );
    }

    #[test]
    fn classifies_map_row_values_from_parser_resolved_surface_tokens() {
        let source = r#"
title = map_row_semantics

puzzle board {
tags {
N = 0 1
D = F B
}
map Nm N {
0 -> 1
1 -> 0
}
map D_rev D {
F -> B
B -> F
}
layers {
You:D Count:N
}
rules {
}
levels {
legend {
. = empty
Y = You:F Count:0
}
level "start" {
.
}
}
}
"#;
        crate::parse_game2d(source).unwrap();
        let tokens = semantic_tokens(source);
        let nm_start = source.find("map Nm N").unwrap();
        let zero_from_start = source[nm_start..].find("0 -> 1").unwrap() + nm_start;
        let one_to_start = zero_from_start + "0 -> ".len();
        let d_rev_start = source.find("map D_rev D").unwrap();
        let f_from_start = source[d_rev_start..].find("F -> B").unwrap() + d_rev_start;
        let b_to_start = f_from_start + "F -> ".len();

        assert_semantic_token(source, &tokens, zero_from_start, "0", SemanticKind::Variant);
        assert_semantic_token(source, &tokens, one_to_start, "1", SemanticKind::Variant);
        assert_semantic_token(source, &tokens, f_from_start, "F", SemanticKind::Variant);
        assert_semantic_token(source, &tokens, b_to_start, "B", SemanticKind::Variant);
    }

    #[test]
    fn classifies_group_rhs_selectors_from_parser_resolved_surface_tokens() {
        let source = r#"
title = group_rhs_selector_semantics

puzzle board {
tags {
D = F B
}
layers {
You:D Crate
}
groups {
player = You:D
object = player Crate
}
rules {
}
levels {
legend {
. = empty
P = You:F
}
level "start" {
.
}
}
}
"#;
        crate::parse_game2d(source).unwrap();
        let tokens = semantic_tokens(source);
        let groups_start = source.find("groups {").unwrap();
        let you_rhs_start = source[groups_start..].find("You:D").unwrap() + groups_start;
        let d_rhs_start = you_rhs_start + "You:".len();
        let player_rhs_start = source[groups_start..].find("player Crate").unwrap() + groups_start;
        let crate_rhs_start = source[player_rhs_start..].find("Crate").unwrap() + player_rhs_start;

        assert_semantic_token(source, &tokens, you_rhs_start, "You", SemanticKind::Object);
        assert_semantic_token(source, &tokens, d_rhs_start, "D", SemanticKind::Group);
        assert_semantic_token(
            source,
            &tokens,
            player_rhs_start,
            "player",
            SemanticKind::Group,
        );
        assert_semantic_token(
            source,
            &tokens,
            crate_rhs_start,
            "Crate",
            SemanticKind::Object,
        );
    }

    #[test]
    fn classifies_legend_rhs_selectors_from_parser_resolved_surface_tokens() {
        let source = r#"
title = legend_rhs_selector_semantics

puzzle board {
tags {
D = F B
}
layers {
You:D Crate
}
groups {
actors = You:D
}
rules {
}
levels {
legend {
. = empty
P = You:F
C = Crate
A = actors
}
level "start" {
.
}
}
}
"#;
        crate::parse_game2d(source).unwrap();
        let tokens = semantic_tokens(source);
        let legend_start = source.find("legend {").unwrap();
        let you_rhs_start = source[legend_start..].find("You:F").unwrap() + legend_start;
        let f_rhs_start = you_rhs_start + "You:".len();
        let crate_rhs_start = source[legend_start..].find("Crate").unwrap() + legend_start;
        let actors_rhs_start = source[legend_start..].find("actors").unwrap() + legend_start;

        assert_semantic_token(source, &tokens, you_rhs_start, "You", SemanticKind::Object);
        assert_semantic_token(source, &tokens, f_rhs_start, "F", SemanticKind::Variant);
        assert_semantic_token(
            source,
            &tokens,
            crate_rhs_start,
            "Crate",
            SemanticKind::Object,
        );
        assert_semantic_token(
            source,
            &tokens,
            actors_rhs_start,
            "actors",
            SemanticKind::Group,
        );
    }

    #[test]
    fn classifies_teneten_group_rhs_selectors_from_parser_resolved_surface_tokens() {
        let source = include_str!("../../../games/TPGJ6/TENETEN.puzzle");
        let tokens = semantic_tokens(source);
        let groups_start = source.find("groups {").unwrap();
        let object_rhs_start =
            source[groups_start..].find("player Crate Ball").unwrap() + groups_start;
        let crate_rhs_start = source[object_rhs_start..].find("Crate").unwrap() + object_rhs_start;
        let time_machine_rhs_start =
            source[object_rhs_start..].find("TimeMachine:D").unwrap() + object_rhs_start;
        let d_rhs_start = time_machine_rhs_start + "TimeMachine:".len();

        assert_semantic_token(
            source,
            &tokens,
            object_rhs_start,
            "player",
            SemanticKind::Group,
        );
        assert_semantic_token(
            source,
            &tokens,
            crate_rhs_start,
            "Crate",
            SemanticKind::Object,
        );
        assert_semantic_token(
            source,
            &tokens,
            time_machine_rhs_start,
            "TimeMachine",
            SemanticKind::Object,
        );
        assert_semantic_token(source, &tokens, d_rhs_start, "D", SemanticKind::Group);
    }

    #[test]
    fn classifies_structural_headers_from_parser_surface_events() {
        let source = r#"
title = structural_header_semantics

puzzle board {
tags {
T = A
}
layers {
Player
}
groups {
movers = Player
}
rules {
on_level_start {
once [ Player ] -> [ Player ]
}
}
levels {
legend {
P = Player
}
level "start" {
P
}
}
}

scene title {
layout {
row {
title = "Title"
}
}
}
"#;
        let tokens = semantic_tokens(source);

        for (needle, text) in [
            ("puzzle board", "puzzle"),
            ("tags {", "tags"),
            ("layers {", "layers"),
            ("groups {", "groups"),
            ("rules {", "rules"),
            ("on_level_start {", "on_level_start"),
            ("levels {", "levels"),
            ("legend {", "legend"),
            ("level \"start\"", "level"),
            ("scene title", "scene"),
            ("layout {", "layout"),
            ("row {", "row"),
        ] {
            let start = source.find(needle).unwrap();
            assert_semantic_token(source, &tokens, start, text, SemanticKind::Keyword);
        }
    }

    #[test]
    fn classifies_marks_from_parser_resolved_surface_tokens() {
        let source = r#"
title = mark_parser_resolved_surface_tokens

puzzle board {
tags {
color = red blue
}
marks {
flag = bool
tint = color
}
layers {
actor = Player
}
rules {
[ Player{flag} ] -> [ Player{tint=red} ]
}
levels {
legend {
. = empty
P = Player
}
level "start" {
.
}
}
}
"#;
        crate::parse_game2d(source).unwrap();
        let tokens = semantic_tokens(source);
        let flag_def_start = source.find("flag").unwrap();
        let tint_def_start = source.find("tint = color").unwrap();
        let flag_selector_start = source.rfind("flag").unwrap();
        let tint_selector_start = source.rfind("tint=red").unwrap();
        let red_selector_start = tint_selector_start + "tint=".len();

        assert_semantic_token(source, &tokens, flag_def_start, "flag", SemanticKind::Mark);
        assert_semantic_token(source, &tokens, tint_def_start, "tint", SemanticKind::Mark);
        assert_semantic_token(
            source,
            &tokens,
            flag_selector_start,
            "flag",
            SemanticKind::Mark,
        );
        assert_semantic_token(
            source,
            &tokens,
            tint_selector_start,
            "tint",
            SemanticKind::Mark,
        );
        assert_semantic_token(
            source,
            &tokens,
            red_selector_start,
            "red",
            SemanticKind::Variant,
        );
    }

    #[test]
    fn classifies_compact_rule_selector_mark_from_rule_attachment_surface() {
        let source = r#"
title = compact_rule_selector_mark

puzzle board {
marks {
mark = bool
}
layers {
actor = Player
}
rules {
[ > Player{mark} ] -> [ Player ]
}
levels {
legend {
. = empty
P = Player
}
level "start" {
.
}
}
}
"#;
        crate::parse_game2d(source).unwrap();
        let tokens = semantic_tokens(source);
        let direction_start = source.find("> Player").unwrap();
        let player_start = source.find("Player{mark}").unwrap();
        let mark_start = player_start + "Player{".len();

        assert_semantic_token(source, &tokens, direction_start, ">", SemanticKind::Keyword);
        assert_semantic_token(
            source,
            &tokens,
            player_start,
            "Player",
            SemanticKind::Object,
        );
        assert_semantic_token(source, &tokens, mark_start, "mark", SemanticKind::Mark);
    }

    #[test]
    fn semantic_highlight_does_not_resolve_selector_identity() {
        assert_production_source_omits_fragments(
            "semantic.rs",
            include_str!("semantic.rs"),
            &[
                &["use crate::", "source"],
                &["scan_source", "_context"],
                &["ExpectedCompletion", "Value"],
                &["completion", "_slots("],
                &["completion", "_keywords_for_scope"],
                &["COMPLETION", "_KEYWORDS"],
                &["semantic_completion", "_context("],
                &["semantic_builtin", "_effect_commands"],
                &["is_completion", "_keyword("],
                &["struct ", "LineToken"],
                &["fn scan_", "semantic_line"],
                &["fn scan_visual", "_semantic_line"],
                &["fn scan_authoring", "_semantic_line"],
                &["fn add_token", "_range"],
                &["fn add_token", "_subrange"],
                &["source_tokens", "_as_line_tokens"],
                &["scene_effect", "_semantic_tokens"],
                &["rewrite_effect", "_semantic_tokens"],
                &["struct ", "SemanticSymbols"],
                &["add_tag", "_definition_line"],
                &["add_layer", "_definition_line"],
                &["add_group", "_definition_line"],
                &["scan_rule", "_pattern_selector_line"],
                &["scan_layer", "_assignment_line"],
                &["add_selector", "_symbol_token"],
                &["SourceScope::", "Tags) => self"],
                &["SourceScope::", "Layers) => self"],
                &["SourceScope::", "Group) => self"],
                &["SemanticKind::", "Selector"],
            ],
            "preserve parser-emitted surface tokens",
        );
    }

    #[test]
    fn surface_document_does_not_own_authoring_declaration_classification() {
        let source = include_str!("lib_surface_doc.rs");
        let forbidden_fragments = [
            ["SourceScope::", "Tags"],
            ["SourceScope::", "Layers"],
            ["SourceScope::", "Group"],
            ["SourceScope::", "Mark"],
            ["record_tag", "_surface_symbols"],
            ["record_layer", "_surface_symbols"],
            ["record_group", "_surface_symbols"],
            ["record_mark", "_surface_symbols"],
        ];
        for parts in forbidden_fragments {
            let forbidden = parts.concat();
            assert!(
                !source.contains(&forbidden),
                "lib_surface_doc.rs must call parser-owned surface emitters, not classify authoring declarations via {forbidden}"
            );
        }
    }

    #[test]
    fn surface_document_does_not_own_assets_content_classification() {
        let source = include_str!("lib_surface_doc.rs");
        for forbidden in ["\"css\"", "\"script\"", "\"file\""] {
            assert!(
                !source.contains(forbidden),
                "lib_surface_doc.rs must call the content surface projector, not classify assets rows via {forbidden}"
            );
        }
    }

    #[test]
    fn surface_completion_does_not_own_authoring_content_keywords() {
        assert_production_source_omits_fragments(
            "surface_completion.rs",
            include_str!("surface_completion.rs"),
            &[
                &["\"", "css", "\""],
                &["\"", "script", "\""],
                &["\"", "file", "\""],
                &["ASSET", "_COMPLETION", "_KEYWORDS"],
            ],
            "derive asset content completions from authoring schema",
        );
    }

    #[test]
    fn surface_completion_does_not_own_authoring_sound_children() {
        assert_production_source_omits_fragments(
            "surface_completion.rs",
            include_str!("surface_completion.rs"),
            &[
                &["SOUNDS", "_COMPLETION", "_KEYWORDS"],
                &["Some(SourceScope::Sounds) => SOUNDS"],
            ],
            "derive sounds children from authoring schema",
        );
    }

    #[test]
    fn authoring_catalog_does_not_own_sound_symbol_exports() {
        assert_production_source_omits_fragments(
            "lib_authoring_parse_catalog.rs",
            include_str!("lib_authoring_parse_catalog.rs"),
            &[
                &["AuthoringKind::", "SfxSoundConfig"],
                &["AuthoringKind::", "MusicSoundConfig"],
                &["symbols_mut().", "sfx, &name"],
                &["symbols_mut().", "music, &name"],
            ],
            "read authoring symbol exports from schema",
        );
    }

    #[test]
    fn completion_items_do_not_own_authoring_setting_vocabularies() {
        assert_production_source_omits_fragments(
            "completion.rs",
            include_str!("completion.rs"),
            &[
                &["THEME", "_SETTING", "_SPECS"],
                &["ASSET", "_COMPLETION", "_KEYWORDS"],
                &["SOUNDS", "_COMPLETION", "_KEYWORDS"],
            ],
            "render schema-provided completion slots",
        );
    }

    fn assert_production_source_omits_fragments(
        file_name: &str,
        source: &str,
        forbidden_fragments: &[&[&str]],
        required_owner: &str,
    ) {
        let source = source
            .split("#[cfg(test)]")
            .next()
            .expect("source file has production section");
        for parts in forbidden_fragments {
            let forbidden = parts.concat();
            assert!(
                !source.contains(&forbidden),
                "{file_name} must {required_owner}, not own {forbidden}"
            );
        }
    }

    #[test]
    fn surface_document_emitters_do_not_accept_legacy_source_scan_products() {
        for source in [
            include_str!("lib_surface_doc.rs"),
            include_str!("lib_authoring_parse_catalog.rs"),
        ] {
            let forbidden_fragments = [
                ["crate::source::", "SourceContext"],
                ["crate::source::", "SourceContextLine"],
                ["&Source", "Context"],
                ["&Source", "ContextLine"],
            ];
            for parts in forbidden_fragments {
                let forbidden = parts.concat();
                assert!(
                    !source.contains(&forbidden),
                    "surface document emitters must consume SurfaceDocument scan products, not legacy source scanner products via {forbidden}"
                );
            }
        }
    }

    #[test]
    fn legacy_source_scanner_is_not_available_to_surface_consumers() {
        let forbidden_fragments: &[&[&str]] = &[
            &["scan_source", "_context"],
            &["scan_surface", "_source"],
            &["Source", "Context"],
            &["Source", "ContextLine"],
            &["Surface", "SourceScan"],
            &["Surface", "SourceLine"],
            &["SourceStructure", "Event"],
            &["SourceBlock", "Role"],
        ];
        for (name, source) in [
            ("lib.rs", include_str!("lib.rs")),
            ("highlight.rs", include_str!("highlight.rs")),
            ("completion.rs", include_str!("completion.rs")),
            (
                "surface_completion.rs",
                include_str!("surface_completion.rs"),
            ),
            ("source_target.rs", include_str!("source_target.rs")),
            ("source_outline.rs", include_str!("source_outline.rs")),
            ("lib_document.rs", include_str!("lib_document.rs")),
        ] {
            for parts in forbidden_fragments {
                let forbidden = parts.concat();
                assert!(
                    !source.contains(&forbidden),
                    "{name} must consume parser-owned surface products, not legacy source scanner products via {forbidden}"
                );
            }
        }
    }

    fn assert_semantic_token(
        source: &str,
        tokens: &[SemanticToken],
        start: usize,
        text: &str,
        kind: SemanticKind,
    ) {
        assert_eq!(&source[start..start + text.len()], text);
        assert!(
            tokens.iter().any(|token| {
                token.start == start && token.end == start + text.len() && token.kind == kind
            }),
            "missing {kind:?} token for {text:?} at {start}"
        );
    }
}
