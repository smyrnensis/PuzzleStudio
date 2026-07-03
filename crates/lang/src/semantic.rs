use crate::source::{SourceScope, SourceToken, scan_source_context};
use crate::syntax::{ExpectedCompletionValue, PUZZLE_COMPLETION_KEYWORDS, PUZZLE_LIFECYCLE_BLOCKS};
use crate::{
    ANIMATION_BLOCK_OPTIONS, ANIMATION_TWEEN_OPTIONS, LEVEL_MENU_OPTIONS, LevelPathPartSyntax,
    MUSIC_SOUND_SETTING_OPTIONS, MapHeaderTokenSyntax, PUZZLE_RENDER_BLOCK_OPTIONS,
    PUZZLE_RENDER_GRID_OPTIONS, RewriteEffectCommandSyntax, SFX_SOUND_SETTING_OPTIONS,
    SceneStateLhsSyntax, SoundSettingValueSyntax, SurfaceOptionBlock, THEME_SETTING_SPECS,
    level_path_part_syntax, map_header_token_syntax, metadata_directive_value_token_index,
    rewrite_effect_command_syntax, rewrite_effect_semantic_tokens, scene_effect_command_syntax,
    scene_effect_semantic_tokens, scene_state_lhs_syntax, sound_setting_value_syntax,
    surface_option_block_before_line,
};

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
    Variant,
    Condition,
    Scene,
    Theme,
    Asset,
    Setting,
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
    Settings(SettingCompletionSet),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingCompletionSet {
    Static(&'static [&'static str]),
    Theme,
}

pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let context = scan_source_context(source);
    let mut tokens = Vec::new();
    for (line_index, line) in context.lines.iter().enumerate() {
        let line_tokens = source_tokens_as_line_tokens(&line.token_spans);
        scan_visual_semantic_line(&context, line_index, &line_tokens, &mut tokens);
        scan_semantic_line(line.scope, &line_tokens, &mut tokens);
    }
    tokens.extend(crate::surface_document_semantic_tokens(source));
    tokens
}

pub(crate) fn semantic_completion_context(
    source: &str,
    cursor_offset: usize,
) -> SemanticCompletionContext {
    let cursor = cursor_offset.min(source.len());
    let token = completion_token_at_cursor(source, cursor);
    let previous = previous_completion_token(source, token.replace_start);
    let context = scan_source_context(source);
    let scope = scope_at_cursor(&context, cursor);
    let sounds_definition_scope = scope == Some(SourceScope::Sounds);

    let contextual_slots = contextual_completion_slots(source, &context, &token, scope);
    let slots = if let Some(slots) = contextual_slots {
        slots
    } else if matches!(
        previous.as_deref(),
        Some("goto" | "resume" | "enter" | "open" | "start")
    ) && scene_effect_scope(scope)
    {
        vec![SemanticCompletionSlot::Scenes]
    } else if previous.as_deref() == Some("input") && scene_effect_scope(scope) {
        vec![SemanticCompletionSlot::Inputs]
    } else if previous.as_deref() == Some("of") {
        vec![SemanticCompletionSlot::Puzzles]
    } else if previous.as_deref() == Some("theme") {
        vec![SemanticCompletionSlot::Themes]
    } else if previous.as_deref() == Some("sfx")
        && (!sounds_definition_scope
            || sounds_operation_sfx_target_context(source, token.replace_start))
    {
        vec![SemanticCompletionSlot::SfxAssets]
    } else if matches!(
        previous.as_deref(),
        Some("play_music" | "pause_music" | "resume_music" | "stop_music")
    ) && !sounds_definition_scope
    {
        vec![SemanticCompletionSlot::MusicAssets]
    } else if matches!(previous.as_deref(), Some("puzzle" | "puzzle3")) {
        vec![SemanticCompletionSlot::Puzzles]
    } else {
        fallback_completion_slots(scope)
    };

    SemanticCompletionContext {
        replace_start: token.replace_start,
        replace_end: token.replace_end,
        token_text: token.text,
        slots,
    }
}

fn contextual_completion_slots(
    source: &str,
    context: &crate::source::SourceContext,
    token: &CompletionTokenAtCursor,
    scope: Option<SourceScope>,
) -> Option<Vec<SemanticCompletionSlot>> {
    if token.text.contains(':') {
        return Some(vec![
            SemanticCompletionSlot::ValueSets,
            SemanticCompletionSlot::ObjectNameAtoms,
        ]);
    }

    let (line_index, line) = line_at_cursor(context, token.replace_start)?;
    let line_end = line.start + line.content.len();
    let before = &source[line.start..token.replace_start.min(line_end)];
    let after = &source[token.replace_end.min(line_end)..line_end];
    let previous = previous_completion_token(source, token.replace_start);

    if cursor_is_after_effect_arrow(before) {
        return Some(arrow_rhs_completion_slots(scope));
    }

    if let Some(options) = sound_setting_completion_slots(line.scope, before) {
        return Some(options);
    }

    if let Some(options) = option_completion_slots(context, line_index, before) {
        return Some(options);
    }

    if let Some(slots) = visual_completion_slots(line.scope, before) {
        return Some(slots);
    }

    if let Some(slots) = grammar_completion_slots(scope, before) {
        return Some(slots);
    }

    if inside_mark_selector_attrs(before) {
        return Some(vec![
            SemanticCompletionSlot::Directions,
            SemanticCompletionSlot::DirectionSets,
            SemanticCompletionSlot::Markes,
        ]);
    }

    if previous.as_deref() == Some("in") {
        return Some(match scope {
            Some(
                SourceScope::Scene
                | SourceScope::SceneLayout
                | SourceScope::SceneTransitions
                | SourceScope::LevelMenu,
            ) => vec![
                SemanticCompletionSlot::Keywords(SCENE_FOR_SOURCE_COMPLETION_KEYWORDS),
                SemanticCompletionSlot::States,
            ],
            _ => vec![
                SemanticCompletionSlot::ValueSets,
                SemanticCompletionSlot::Groups,
            ],
        });
    }

    if is_rule_like_scope(scope) && next_non_whitespace_starts_pattern(after) {
        return Some(vec![
            SemanticCompletionSlot::Directions,
            SemanticCompletionSlot::DirectionSets,
        ]);
    }

    if before.trim().is_empty() && (token.text.is_empty() || !symbol_definition_scope(scope)) {
        return Some(line_head_completion_slots(context, line_index, scope));
    }

    None
}

fn grammar_completion_slots(
    scope: Option<SourceScope>,
    before: &str,
) -> Option<Vec<SemanticCompletionSlot>> {
    let tokens = split_completion_line_tokens(before);
    let classes = grammar_completion_value_classes(scope, &tokens)?;
    Some(completion_slots_for_value_classes(classes))
}

fn grammar_completion_value_classes(
    scope: Option<SourceScope>,
    tokens: &[&str],
) -> Option<&'static [ExpectedCompletionValue]> {
    let syntax = match scope {
        Some(SourceScope::Legend) => crate::syntax::legend_block_row_syntax(tokens, false),
        Some(SourceScope::Puzzle) => crate::syntax::legend_directive_syntax(tokens, false),
        Some(SourceScope::Level | SourceScope::UnbracedLevel) => {
            crate::syntax::level_legend_directive_syntax(tokens, false)
        }
        Some(SourceScope::Group) => crate::syntax::named_selector_assignment_syntax(tokens, false),
        Some(SourceScope::Layers) => crate::syntax::named_selector_assignment_syntax(tokens, false),
        _ => None,
    }?;
    Some(syntax.expected_completion_values)
}

fn completion_slots_for_value_classes(
    classes: &[ExpectedCompletionValue],
) -> Vec<SemanticCompletionSlot> {
    let mut slots = Vec::new();
    for class in classes {
        match class {
            ExpectedCompletionValue::Selector | ExpectedCompletionValue::SpriteSelector => {
                slots.push(SemanticCompletionSlot::Objects);
                slots.push(SemanticCompletionSlot::Groups);
            }
            ExpectedCompletionValue::LegendEmpty => {
                slots.push(SemanticCompletionSlot::Keywords(LEGEND_COMPLETION_KEYWORDS));
            }
            ExpectedCompletionValue::VisualDirective => {
                slots.push(SemanticCompletionSlot::Keywords(VISUAL_COMPLETION_KEYWORDS));
            }
        }
    }
    slots
}

fn cursor_is_after_effect_arrow(before: &str) -> bool {
    let Some(arrow) = before.rfind("->") else {
        return false;
    };
    let suffix = &before[arrow + 2..];
    let tokens = suffix
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    tokens.is_empty()
        || (tokens.len() == 1 && suffix.chars().next_back().is_some_and(is_completion_char))
}

fn arrow_rhs_completion_slots(scope: Option<SourceScope>) -> Vec<SemanticCompletionSlot> {
    match scope {
        Some(SourceScope::Keys) => vec![
            SemanticCompletionSlot::Inputs,
            SemanticCompletionSlot::Directions,
        ],
        Some(SourceScope::SceneKeys) => vec![
            SemanticCompletionSlot::SceneEffects,
            SemanticCompletionSlot::Routines,
        ],
        Some(
            SourceScope::Scene
            | SourceScope::SceneLayout
            | SourceScope::SceneTransitions
            | SourceScope::LevelMenu,
        ) => vec![
            SemanticCompletionSlot::SceneEffects,
            SemanticCompletionSlot::Routines,
        ],
        Some(SourceScope::Sounds) => vec![],
        _ => vec![
            SemanticCompletionSlot::ModelEffects,
            SemanticCompletionSlot::Emissions,
        ],
    }
}

fn line_head_completion_slots(
    context: &crate::source::SourceContext,
    line_index: usize,
    scope: Option<SourceScope>,
) -> Vec<SemanticCompletionSlot> {
    match scope {
        None => vec![SemanticCompletionSlot::ModelTopLevelKeywords],
        Some(
            SourceScope::Puzzle
            | SourceScope::Sounds
            | SourceScope::Assets
            | SourceScope::Scene
            | SourceScope::SceneLayout
            | SourceScope::SceneState
            | SourceScope::SceneKeys
            | SourceScope::SceneTransitions
            | SourceScope::LevelMenu
            | SourceScope::Tags
            | SourceScope::Group
            | SourceScope::Layers
            | SourceScope::Mark
            | SourceScope::Keys
            | SourceScope::Legend
            | SourceScope::Levels
            | SourceScope::Level
            | SourceScope::UnbracedLevel
            | SourceScope::VisualShapeTable
            | SourceScope::VisualShapeEntry
            | SourceScope::VisualColorTable,
        ) => vec![SemanticCompletionSlot::Keywords(
            completion_keywords_for_scope(scope),
        )],
        Some(SourceScope::Visuals) => completion_slots_for_value_classes(
            crate::syntax::visual_line_head_expected_completion_values(),
        ),
        Some(SourceScope::Other) if current_statement_block_before_line(context, line_index) => {
            vec![
                SemanticCompletionSlot::Keywords(puzzle_authoring::RULE_STATEMENT_HEAD_KEYWORDS),
                SemanticCompletionSlot::Routines,
                SemanticCompletionSlot::StandardRuleSteps,
                SemanticCompletionSlot::Directions,
                SemanticCompletionSlot::DirectionSets,
                SemanticCompletionSlot::ModelEffects,
                SemanticCompletionSlot::Emissions,
            ]
        }
        Some(SourceScope::Other) => fallback_completion_slots(scope),
    }
}

fn current_statement_block_before_line(
    context: &crate::source::SourceContext,
    line_index: usize,
) -> bool {
    let mut stack = Vec::<CompletionBlockKind>::new();
    for line in context.lines.iter().take(line_index) {
        update_completion_block_stack(line.content.trim(), &mut stack);
    }
    stack.last() == Some(&CompletionBlockKind::Statement)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletionBlockKind {
    Statement,
    Other,
}

fn update_completion_block_stack(line: &str, stack: &mut Vec<CompletionBlockKind>) {
    let mut current = line;
    while let Some(rest) = current.strip_prefix('}') {
        stack.pop();
        current = rest.trim_start();
        if current.is_empty() {
            return;
        }
    }
    if !current.ends_with('{') {
        return;
    }
    let parent_is_statement_block = stack.last() == Some(&CompletionBlockKind::Statement);
    let kind = if puzzle_authoring::rule_statement_block_surface(current, parent_is_statement_block)
        .is_some()
    {
        CompletionBlockKind::Statement
    } else {
        CompletionBlockKind::Other
    };
    stack.push(kind);
}

fn sound_setting_completion_slots(
    scope: Option<SourceScope>,
    before: &str,
) -> Option<Vec<SemanticCompletionSlot>> {
    if scope != Some(SourceScope::Sounds) {
        return None;
    }
    let tokens = before
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}' | ',' | ';'))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    match tokens.as_slice() {
        ["sfx", _, ..] => Some(vec![SemanticCompletionSlot::Settings(
            SettingCompletionSet::Static(SFX_SOUND_SETTING_OPTIONS),
        )]),
        ["music", _, ..] => Some(vec![SemanticCompletionSlot::Settings(
            SettingCompletionSet::Static(MUSIC_SOUND_SETTING_OPTIONS),
        )]),
        _ => None,
    }
}

fn line_at_cursor<'a>(
    context: &'a crate::source::SourceContext,
    cursor: usize,
) -> Option<(usize, &'a crate::source::SourceContextLine)> {
    context.lines.iter().enumerate().find(|(_, line)| {
        let end = line.start + line.content.len();
        cursor >= line.start && cursor <= end
    })
}

fn option_completion_slots(
    context: &crate::source::SourceContext,
    line_index: usize,
    before: &str,
) -> Option<Vec<SemanticCompletionSlot>> {
    let block = surface_option_block_before_line(&context.lines, line_index);
    let tokens_before = before
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}' | ',' | ';'))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let first = tokens_before.first().copied();

    let option_names = match (block, first) {
        (Some(SurfaceOptionBlock::Render3), Some("camera")) => puzzle_3d::CAMERA_OPTIONS3,
        (Some(SurfaceOptionBlock::Render3), Some("grid")) => puzzle_3d::GRID_BARE_OPTIONS3,
        (Some(SurfaceOptionBlock::Render3), Some("pixelate")) => puzzle_3d::PIXELATE_OPTIONS3,
        (Some(SurfaceOptionBlock::Render2), Some("grid")) => PUZZLE_RENDER_GRID_OPTIONS,
        (Some(SurfaceOptionBlock::Animation), Some("tween")) => ANIMATION_TWEEN_OPTIONS,
        (Some(SurfaceOptionBlock::Camera3), _) => puzzle_3d::CAMERA_OPTIONS3,
        (Some(SurfaceOptionBlock::Grid3), _) => puzzle_3d::GRID_BARE_OPTIONS3,
        (Some(SurfaceOptionBlock::Pixelate3), _) => puzzle_3d::PIXELATE_OPTIONS3,
        (Some(SurfaceOptionBlock::Grid2), _) => PUZZLE_RENDER_GRID_OPTIONS,
        (Some(SurfaceOptionBlock::Tween), _) => ANIMATION_TWEEN_OPTIONS,
        (Some(SurfaceOptionBlock::LevelMenu), _) => LEVEL_MENU_OPTIONS,
        (Some(SurfaceOptionBlock::Theme), _) => {
            if theme_setting_value_is_before_cursor(before) {
                return Some(vec![SemanticCompletionSlot::Colors]);
            }
            return Some(vec![SemanticCompletionSlot::Settings(
                SettingCompletionSet::Theme,
            )]);
        }
        (Some(SurfaceOptionBlock::Render3), _) => puzzle_3d::RENDER_OPTIONS3,
        (Some(SurfaceOptionBlock::Render2), _) => PUZZLE_RENDER_BLOCK_OPTIONS,
        (Some(SurfaceOptionBlock::Animation), _) => ANIMATION_BLOCK_OPTIONS,
        _ => return None,
    };

    Some(vec![SemanticCompletionSlot::Settings(
        SettingCompletionSet::Static(option_names),
    )])
}

fn theme_setting_value_is_before_cursor(before: &str) -> bool {
    let tokens = before
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}' | ',' | ';'))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let Some(first) = tokens.first().copied() else {
        return false;
    };
    if !THEME_SETTING_SPECS
        .iter()
        .any(|spec| spec.canonical == first || spec.aliases.contains(&first))
    {
        return false;
    }
    tokens.len() > 1 || before.chars().next_back().is_some_and(char::is_whitespace)
}

fn visual_completion_slots(
    scope: Option<SourceScope>,
    before: &str,
) -> Option<Vec<SemanticCompletionSlot>> {
    match scope {
        Some(SourceScope::VisualColorTable) => {
            before.rfind('=')?;
            Some(vec![SemanticCompletionSlot::Colors])
        }
        Some(SourceScope::Visuals) => {
            let tokens = before
                .split(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}' | ',' | ';'))
                .filter(|token| !token.is_empty())
                .collect::<Vec<_>>();
            match tokens.as_slice() {
                ["colors", ..] => Some(vec![SemanticCompletionSlot::Colors]),
                ["shape", ..] => Some(vec![SemanticCompletionSlot::Shapes]),
                [first, ..] if !matches!(*first, "shape" | "sprite" | "colors") => Some(vec![
                    SemanticCompletionSlot::Colors,
                    SemanticCompletionSlot::Assets,
                ]),
                _ => None,
            }
        }
        _ => None,
    }
}

fn inside_mark_selector_attrs(before: &str) -> bool {
    let Some(open) = before.rfind('{') else {
        return false;
    };
    if before[open + 1..].contains('}') {
        return false;
    }
    if open == 0 {
        return true;
    }
    let before_open = &before[..open];
    let Some(previous) = before_open.chars().next_back() else {
        return true;
    };
    if !previous.is_whitespace() {
        return true;
    }
    let trimmed = before_open.trim_end();
    trimmed.ends_with('[') || trimmed.ends_with('|')
}

fn next_non_whitespace_starts_pattern(after: &str) -> bool {
    after.trim_start().starts_with('[')
}

fn is_rule_like_scope(scope: Option<SourceScope>) -> bool {
    matches!(scope, Some(SourceScope::Puzzle | SourceScope::Other))
}

fn symbol_definition_scope(scope: Option<SourceScope>) -> bool {
    matches!(
        scope,
        Some(
            SourceScope::Group
                | SourceScope::Layers
                | SourceScope::Tags
                | SourceScope::Mark
                | SourceScope::Keys
                | SourceScope::SceneKeys
                | SourceScope::SceneState
        )
    )
}

fn scene_effect_scope(scope: Option<SourceScope>) -> bool {
    matches!(
        scope,
        Some(
            SourceScope::Scene
                | SourceScope::SceneLayout
                | SourceScope::SceneKeys
                | SourceScope::SceneTransitions
                | SourceScope::LevelMenu
        )
    )
}

pub(crate) fn semantic_builtin_effect_commands() -> Vec<(&'static str, SemanticKind)> {
    let commands = [
        "apply",
        "again",
        "back",
        "cancel",
        "checkpoint",
        "clear_checkpoint",
        "close",
        "clear_history",
        "clear_undo_history",
        "clear_game_progress",
        "continue",
        "copy",
        "create",
        "delete",
        "enter",
        "focus",
        "goto",
        "hide",
        "input",
        "component_effect",
        "load",
        "message",
        "next_level",
        "open",
        "pause_music",
        "play_music",
        "reset",
        "restart",
        "resume",
        "resume_music",
        "sfx",
        "show",
        "start",
        "stop_music",
        "toggle",
        "wait",
        "win",
    ];
    commands
        .into_iter()
        .filter_map(|command| {
            let kind = if command == "sfx" {
                SemanticKind::Effect
            } else {
                match rewrite_effect_command_syntax(command) {
                    Some(RewriteEffectCommandSyntax::Emission) => SemanticKind::Emission,
                    Some(RewriteEffectCommandSyntax::Effect) => SemanticKind::Effect,
                    None if scene_effect_command_syntax(command).is_some()
                        || command == "restart" =>
                    {
                        SemanticKind::Effect
                    }
                    None => return None,
                }
            };
            Some((command, kind))
        })
        .collect()
}

pub(crate) fn semantic_model_effect_commands() -> Vec<(&'static str, SemanticKind)> {
    [
        "again",
        "cancel",
        "checkpoint",
        "clear_checkpoint",
        "message",
        "next_level",
        "restart",
        "sfx",
        "wait",
        "win",
    ]
    .into_iter()
    .filter_map(|command| {
        let kind = if command == "sfx" {
            SemanticKind::Effect
        } else {
            match rewrite_effect_command_syntax(command)? {
                RewriteEffectCommandSyntax::Emission => SemanticKind::Emission,
                RewriteEffectCommandSyntax::Effect => SemanticKind::Effect,
            }
        };
        Some((command, kind))
    })
    .collect()
}

pub(crate) fn semantic_scene_effect_commands() -> Vec<(&'static str, SemanticKind)> {
    [
        "apply",
        "clear",
        "clear_game_progress",
        "clear_history",
        "clear_undo_history",
        "component_effect",
        "copy",
        "goto",
        "input",
        "load",
        "message",
        "pause_music",
        "play_music",
        "resume_music",
        "sfx",
        "start",
        "stop_music",
        "wait",
    ]
    .into_iter()
    .filter_map(|command| {
        scene_effect_command_syntax(command)?;
        Some((command, SemanticKind::Effect))
    })
    .collect()
}

pub(crate) fn is_completion_keyword(token: &str) -> bool {
    COMPLETION_KEYWORDS.contains(&token)
}

fn fallback_completion_slots(scope: Option<SourceScope>) -> Vec<SemanticCompletionSlot> {
    match scope {
        None => vec![SemanticCompletionSlot::ModelTopLevelKeywords],
        Some(SourceScope::Sounds | SourceScope::Assets) => {
            vec![SemanticCompletionSlot::Keywords(
                completion_keywords_for_scope(scope),
            )]
        }
        Some(SourceScope::Tags) => vec![],
        Some(SourceScope::Group | SourceScope::Layers) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Objects,
            SemanticCompletionSlot::Groups,
        ],
        Some(SourceScope::Mark) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::ObjectNameAtoms,
            SemanticCompletionSlot::Directions,
            SemanticCompletionSlot::DirectionSets,
        ],
        Some(SourceScope::Keys) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Inputs,
            SemanticCompletionSlot::Directions,
        ],
        Some(SourceScope::SceneKeys) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::SceneEffects,
            SemanticCompletionSlot::Routines,
        ],
        Some(SourceScope::SceneState) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Literals(COMPLETION_LITERALS),
            SemanticCompletionSlot::States,
            SemanticCompletionSlot::Puzzles,
        ],
        Some(SourceScope::Puzzle) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Literals(COMPLETION_LITERALS),
            SemanticCompletionSlot::States,
            SemanticCompletionSlot::Conditions,
            SemanticCompletionSlot::Inputs,
            SemanticCompletionSlot::Directions,
            SemanticCompletionSlot::DirectionSets,
        ],
        Some(
            SourceScope::Scene
            | SourceScope::SceneLayout
            | SourceScope::SceneTransitions
            | SourceScope::LevelMenu,
        ) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Literals(COMPLETION_LITERALS),
            SemanticCompletionSlot::States,
            SemanticCompletionSlot::Routines,
            SemanticCompletionSlot::Conditions,
            SemanticCompletionSlot::Inputs,
            SemanticCompletionSlot::SceneEffects,
        ],
        Some(
            SourceScope::Legend
            | SourceScope::Levels
            | SourceScope::Level
            | SourceScope::UnbracedLevel,
        ) => vec![SemanticCompletionSlot::Keywords(
            completion_keywords_for_scope(scope),
        )],
        Some(
            SourceScope::Visuals
            | SourceScope::VisualShapeTable
            | SourceScope::VisualShapeEntry
            | SourceScope::VisualColorTable,
        ) => vec![
            SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
            SemanticCompletionSlot::Sprites,
            SemanticCompletionSlot::Assets,
            SemanticCompletionSlot::Shapes,
            SemanticCompletionSlot::Colors,
        ],
        Some(SourceScope::Other) => rule_expression_completion_slots(),
    }
}

fn rule_expression_completion_slots() -> Vec<SemanticCompletionSlot> {
    vec![
        SemanticCompletionSlot::Literals(COMPLETION_LITERALS),
        SemanticCompletionSlot::Objects,
        SemanticCompletionSlot::Groups,
        SemanticCompletionSlot::States,
        SemanticCompletionSlot::Markes,
        SemanticCompletionSlot::ObjectNameAtoms,
        SemanticCompletionSlot::Directions,
        SemanticCompletionSlot::DirectionSets,
        SemanticCompletionSlot::Inputs,
        SemanticCompletionSlot::ModelEffects,
        SemanticCompletionSlot::Emissions,
        SemanticCompletionSlot::Routines,
        SemanticCompletionSlot::Conditions,
    ]
}

fn completion_keywords_for_scope(scope: Option<SourceScope>) -> &'static [&'static str] {
    match scope {
        None => &[],
        Some(SourceScope::Sounds) => SOUNDS_COMPLETION_KEYWORDS,
        Some(SourceScope::Assets) => ASSET_COMPLETION_KEYWORDS,
        Some(SourceScope::Puzzle) => PUZZLE_COMPLETION_KEYWORDS,
        Some(SourceScope::Tags) => TAG_COMPLETION_KEYWORDS,
        Some(SourceScope::Group) => GROUP_COMPLETION_KEYWORDS,
        Some(SourceScope::Layers) => LAYER_COMPLETION_KEYWORDS,
        Some(SourceScope::Mark) => MARK_COMPLETION_KEYWORDS,
        Some(SourceScope::Keys) | Some(SourceScope::SceneKeys) => KEY_COMPLETION_KEYWORDS,
        Some(SourceScope::Legend) => LEGEND_COMPLETION_KEYWORDS,
        Some(SourceScope::Levels) | Some(SourceScope::Level) | Some(SourceScope::UnbracedLevel) => {
            LEVEL_COMPLETION_KEYWORDS
        }
        Some(
            SourceScope::Scene
            | SourceScope::SceneLayout
            | SourceScope::SceneState
            | SourceScope::SceneTransitions
            | SourceScope::LevelMenu,
        ) => SCENE_COMPLETION_KEYWORDS,
        Some(
            SourceScope::Visuals
            | SourceScope::VisualShapeTable
            | SourceScope::VisualShapeEntry
            | SourceScope::VisualColorTable,
        ) => VISUAL_COMPLETION_KEYWORDS,
        Some(SourceScope::Other) => COMPLETION_KEYWORDS,
    }
}

fn scope_at_cursor(context: &crate::source::SourceContext, cursor: usize) -> Option<SourceScope> {
    let mut previous = None;
    for line in &context.lines {
        let end = line.start + line.content.len();
        if cursor >= line.start && cursor <= end {
            return line.scope;
        }
        if line.start <= cursor {
            previous = line.scope;
        } else {
            break;
        }
    }
    previous
}

struct CompletionTokenAtCursor {
    text: String,
    replace_start: usize,
    replace_end: usize,
}

fn completion_token_at_cursor(source: &str, cursor: usize) -> CompletionTokenAtCursor {
    let mut start = cursor;
    while start > 0 {
        let Some(ch) = source[..start].chars().next_back() else {
            break;
        };
        if !is_completion_char(ch) {
            break;
        }
        start -= ch.len_utf8();
    }
    let mut end = cursor;
    while end < source.len() {
        let Some(ch) = source[end..].chars().next() else {
            break;
        };
        if !is_completion_char(ch) {
            break;
        }
        end += ch.len_utf8();
    }
    CompletionTokenAtCursor {
        text: source[start..cursor].to_string(),
        replace_start: start,
        replace_end: end,
    }
}

fn previous_completion_token(source: &str, before: usize) -> Option<String> {
    let mut index = before;
    while index > 0 {
        let ch = source[..index].chars().next_back()?;
        if !ch.is_whitespace() {
            break;
        }
        index -= ch.len_utf8();
    }
    let end = index;
    while index > 0 {
        let ch = source[..index].chars().next_back()?;
        if !is_completion_char(ch) {
            break;
        }
        index -= ch.len_utf8();
    }
    (index < end).then(|| source[index..end].to_string())
}

fn is_completion_char(ch: char) -> bool {
    ch == '@' || ch == '_' || ch == ':' || ch == '.' || ch == '-' || ch.is_ascii_alphanumeric()
}

const SOUNDS_COMPLETION_KEYWORDS: &[&str] = &["music", "restart", "sfx", "undo"];

const ASSET_COMPLETION_KEYWORDS: &[&str] = &["css", "file", "script"];

const TAG_COMPLETION_KEYWORDS: &[&str] = &[];
const GROUP_COMPLETION_KEYWORDS: &[&str] = &["each"];
const LAYER_COMPLETION_KEYWORDS: &[&str] = &["each"];
const MARK_COMPLETION_KEYWORDS: &[&str] = &["const", "persistent", "var"];
const KEY_COMPLETION_KEYWORDS: &[&str] = &["direction", "input"];
const LEGEND_COMPLETION_KEYWORDS: &[&str] = &["empty"];
const LEVEL_COMPLETION_KEYWORDS: &[&str] = &["legend", "level", "of"];
const SCENE_FOR_SOURCE_COMPLETION_KEYWORDS: &[&str] = &["levels"];

const SCENE_COMPLETION_KEYWORDS: &[&str] = &[
    "button",
    "column",
    "const",
    "else",
    "for",
    "if",
    "keys",
    "level_menu",
    "message",
    "on_scene_start",
    "box",
    "puzzle",
    "puzzle3",
    "row",
    "rules",
    "scene",
    "state",
    "text",
    "title",
    "layout",
    "with",
];

const VISUAL_COMPLETION_KEYWORDS: &[&str] = &[
    "colors",
    "offset",
    "pixels_per_cell",
    "rotate",
    "shape",
    "shapes",
    "sprite",
];
const COMPLETION_LITERALS: &[&str] = &["false", "true"];

const COMPLETION_KEYWORDS: &[&str] = &[
    "again_interval",
    "assets",
    "author",
    "sounds",
    "button",
    "column",
    "const",
    "colors",
    "collision_layers",
    "component_effect",
    "css",
    "direction",
    "each",
    "file",
    "else",
    "for",
    "from",
    "groups",
    "homepage",
    "if",
    "import",
    "input",
    "interactive_look",
    "interactive_zoom",
    "keys",
    "layers",
    "legend",
    "level",
    "level_menu",
    "levels",
    "levels3",
    "lose_conditions",
    "map",
    "music",
    "of",
    "on_display",
    PUZZLE_LIFECYCLE_BLOCKS[0],
    PUZZLE_LIFECYCLE_BLOCKS[1],
    PUZZLE_LIFECYCLE_BLOCKS[2],
    "once",
    "once_all",
    "once_per_level",
    "box",
    "persistent",
    "puzzle",
    "puzzle3",
    "condition",
    "repeat",
    "resources",
    "render",
    "row",
    "routine",
    "rule",
    "rules",
    "scene",
    "marks",
    "sfx",
    "shape",
    "show_index",
    "show_solved",
    "sprite",
    "sprites3",
    "state",
    "subtitle",
    "text",
    "theme",
    "title",
    "var",
    "layout",
    "win_conditions",
    "with",
];

#[derive(Clone, Copy, Debug)]
struct LineToken<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

fn scan_semantic_line(
    scope: Option<SourceScope>,
    tokens: &[LineToken<'_>],
    ranges: &mut Vec<SemanticToken>,
) {
    let Some(first) = tokens.first().copied() else {
        return;
    };
    if scope == Some(SourceScope::Sounds) {
        scan_sounds_semantic_line(tokens, ranges);
        return;
    }
    if scope == Some(SourceScope::SceneKeys) {
        scan_key_semantic_line(tokens, ranges);
        return;
    }
    scan_authoring_semantic_line(scope, tokens, ranges);
    if !is_scene_semantic_scope(scope) && first.text != "scene" {
        scan_rewrite_effect_line(scope, tokens, ranges);
        return;
    }
    scan_scene_semantic_line(scope, tokens, ranges);
}

fn is_scene_semantic_scope(scope: Option<SourceScope>) -> bool {
    matches!(
        scope,
        Some(
            SourceScope::Scene
                | SourceScope::SceneLayout
                | SourceScope::SceneState
                | SourceScope::SceneKeys
                | SourceScope::SceneTransitions
                | SourceScope::LevelMenu
        )
    )
}

fn scan_sounds_semantic_line(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    match tokens {
        [keyword, name, ..] if matches!(keyword.text, "sfx" | "music") => {
            add_token_range(ranges, *keyword, SemanticKind::Keyword);
            add_token_range(ranges, *name, SemanticKind::Asset);
            for setting in &tokens[2..] {
                scan_sounds_setting_token(*setting, ranges);
            }
        }
        [operation, arrow, sfx, name]
            if matches!(operation.text, "undo" | "restart")
                && arrow.text == "->"
                && sfx.text == "sfx" =>
        {
            add_token_range(ranges, *operation, SemanticKind::Keyword);
            add_token_range(ranges, *sfx, SemanticKind::Keyword);
            add_token_range(ranges, *name, SemanticKind::Asset);
        }
        _ => {}
    }
}

fn sounds_operation_sfx_target_context(source: &str, cursor: usize) -> bool {
    let line_start = source[..cursor].rfind('\n').map_or(0, |index| index + 1);
    let before = source[line_start..cursor].trim();
    matches!(
        split_completion_line_tokens(before).as_slice(),
        ["undo", "->", "sfx"] | ["restart", "->", "sfx"]
    )
}

fn split_completion_line_tokens(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

fn scan_sounds_setting_token(token: LineToken<'_>, ranges: &mut Vec<SemanticToken>) {
    let Some((key, value)) = token.text.split_once('=') else {
        return;
    };
    let Some(value_syntax) = sound_setting_value_syntax(key) else {
        return;
    };
    if key.is_empty() || value.is_empty() {
        return;
    }
    add_token_subrange(ranges, token, 0, key.len(), SemanticKind::Setting);
    let value_start = key.len() + 1;
    let value_kind = match value_syntax {
        SoundSettingValueSyntax::String => SemanticKind::String,
        SoundSettingValueSyntax::Number => SemanticKind::Number,
    };
    add_token_subrange(ranges, token, value_start, token.text.len(), value_kind);
}

fn scan_authoring_semantic_line(
    scope: Option<SourceScope>,
    tokens: &[LineToken<'_>],
    ranges: &mut Vec<SemanticToken>,
) {
    let token_texts = tokens.iter().map(|token| token.text).collect::<Vec<_>>();
    if let Some(index) = metadata_directive_value_token_index(&token_texts)
        && let Some(value) = tokens.get(index).copied()
    {
        add_token_range(ranges, value, SemanticKind::String);
    }
    for token in tokens {
        scan_level_path_token(*token, ranges);
    }

    for index in 0..tokens.len() {
        match map_header_token_syntax(&token_texts, index) {
            Some(MapHeaderTokenSyntax::Keyword) => {
                add_token_range(ranges, tokens[index], SemanticKind::Keyword)
            }
            Some(MapHeaderTokenSyntax::Name) => {
                add_token_range(ranges, tokens[index], SemanticKind::Effect)
            }
            Some(MapHeaderTokenSyntax::Axis) => {
                add_token_range(ranges, tokens[index], SemanticKind::Group)
            }
            None => {}
        }
    }

    scan_levels_header(tokens, ranges);
    scan_level_header(tokens, ranges);
    scan_theme_header(tokens, ranges);
    scan_routine_header(tokens, ranges);
    scan_condition_reference_tokens(tokens, ranges);
    scan_state_declaration_line(tokens, ranges);
    scan_tag_set_line(scope, tokens, ranges);
    scan_standard_move_call_line(scope, tokens, ranges);
    scan_layer_assignment_line(scope, tokens, ranges);
    scan_render_setting_line(scope, tokens, ranges);

    if is_scene_semantic_scope(scope)
        && let Some((index, syntax)) = scene_state_lhs_syntax(&token_texts)
        && let Some(name) = tokens.get(index).copied()
    {
        let kind = match syntax {
            SceneStateLhsSyntax::PuzzleSlot | SceneStateLhsSyntax::Variable => SemanticKind::State,
        };
        add_token_range(ranges, name, kind);
    }
}

fn scan_visual_semantic_line(
    context: &crate::source::SourceContext,
    line_index: usize,
    tokens: &[LineToken<'_>],
    ranges: &mut Vec<SemanticToken>,
) {
    let Some(line) = context.lines.get(line_index) else {
        return;
    };
    if visual_semantic_closing_line(line) {
        return;
    }
    let token_texts = tokens.iter().map(|token| token.text).collect::<Vec<_>>();
    match line.scope {
        Some(SourceScope::Visuals) => match token_texts.as_slice() {
            ["shape", shape_ref, ..] => {
                add_token_range(ranges, tokens[0], SemanticKind::Keyword);
                add_visual_table_expr_token(ranges, tokens[1], shape_ref);
            }
            ["colors", ..] | ["pixels_per_cell", ..] | ["offset", ..] | ["rotate", ..] => {
                add_token_range(ranges, tokens[0], SemanticKind::Keyword);
                add_visual_rotation_keywords(ranges, tokens);
            }
            _ => {}
        },
        Some(SourceScope::VisualShapeTable) => {
            let Some(first) = tokens.first().copied() else {
                return;
            };
            match token_texts.as_slice() {
                ["rotate", ..] => {
                    add_token_range(ranges, first, SemanticKind::Keyword);
                    add_visual_rotation_keywords(ranges, tokens);
                }
                ["shape" | "shapes" | "colors", ..] => {
                    add_token_range(ranges, first, SemanticKind::Keyword);
                }
                _ => add_visual_table_ref_token(ranges, first),
            }
        }
        Some(SourceScope::VisualColorTable) => {
            let Some(first) = tokens.first().copied() else {
                return;
            };
            if token_texts
                .as_slice()
                .get(1)
                .is_some_and(|token| *token == "=")
            {
                return;
            }
            match first.text {
                "shape" | "shapes" | "colors" | "rotate" => {
                    add_token_range(ranges, first, SemanticKind::Keyword);
                }
                _ => add_visual_table_ref_token(ranges, first),
            }
        }
        Some(SourceScope::VisualShapeEntry) => match token_texts.as_slice() {
            ["shape", shape_ref, ..] => {
                add_token_range(ranges, tokens[0], SemanticKind::Keyword);
                add_visual_table_expr_token(ranges, tokens[1], shape_ref);
            }
            ["rotate" | "colors", ..] => {
                add_token_range(ranges, tokens[0], SemanticKind::Keyword);
                add_visual_rotation_keywords(ranges, tokens);
            }
            _ => {}
        },
        _ => {}
    }
}

fn visual_semantic_closing_line(line: &crate::source::SourceContextLine) -> bool {
    crate::source::strip_line_comment(&line.content).trim() == "}"
}

fn add_visual_table_ref_token(ranges: &mut Vec<SemanticToken>, token: LineToken<'_>) {
    let Some((start, end)) = first_identifier_bounds(token.text) else {
        return;
    };
    add_token_subrange(ranges, token, start, end, SemanticKind::Asset);
    let suffix = &token.text[end..];
    if let Some(axis_start) = suffix.find(':') {
        let axis_start = end + axis_start + 1;
        if let Some(axis_end) = identifier_end(token.text, axis_start) {
            add_token_subrange(ranges, token, axis_start, axis_end, SemanticKind::Group);
        }
    }
}

fn add_visual_table_expr_token(ranges: &mut Vec<SemanticToken>, token: LineToken<'_>, text: &str) {
    let Some((start, end)) = first_identifier_bounds(text) else {
        return;
    };
    add_token_subrange(ranges, token, start, end, SemanticKind::Asset);
    let suffix = &text[end..];
    if let Some(value_start) = suffix.find(':') {
        let value_start = end + value_start + 1;
        if let Some(value_end) = identifier_end(text, value_start) {
            add_token_subrange(ranges, token, value_start, value_end, SemanticKind::Variant);
        }
    }
}

fn add_visual_rotation_keywords(ranges: &mut Vec<SemanticToken>, tokens: &[LineToken<'_>]) {
    for token in tokens.iter().skip(1) {
        if matches!(token.text, "from" | "using") {
            add_token_range(ranges, *token, SemanticKind::Keyword);
        }
    }
}

fn scan_theme_header(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    if !tokens.first().is_some_and(|token| token.text == "theme") {
        return;
    }
    let tokens = trim_trailing_block_markers(tokens);
    if let Some(name) = tokens.get(1).copied() {
        add_token_range(ranges, name, SemanticKind::Theme);
    }
}

fn scan_state_declaration_line(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    let name = match tokens {
        [keyword, name, ..] if matches!(keyword.text, "var" | "const") => Some(*name),
        [persistent, keyword, name, ..]
            if persistent.text == "persistent" && matches!(keyword.text, "var" | "const") =>
        {
            Some(*name)
        }
        [persistent, name, ..] if persistent.text == "persistent" => Some(*name),
        _ => None,
    };
    if let Some(name) = name {
        add_token_range(ranges, name, SemanticKind::State);
    }
}

fn scan_routine_header(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    let Some(kind) = tokens.first().copied() else {
        return;
    };
    if !matches!(kind.text, "routine" | "rule") {
        return;
    }

    add_token_range(ranges, kind, SemanticKind::Keyword);
    let name_index = 1usize;
    if let Some(name) = tokens.get(name_index).copied() {
        add_token_range(ranges, name, SemanticKind::Effect);
    }
    if let Some(application) = tokens.get(name_index + 1).copied()
        && rule_application_keyword(application.text)
    {
        add_token_range(ranges, application, SemanticKind::Keyword);
    }
}

fn rule_application_keyword(token: &str) -> bool {
    matches!(token, "once" | "once_all" | "once_per_level" | "repeat")
}

fn scan_tag_set_line(
    scope: Option<SourceScope>,
    tokens: &[LineToken<'_>],
    ranges: &mut Vec<SemanticToken>,
) {
    if scope != Some(SourceScope::Tags) {
        return;
    }
    let [name, separator, values @ ..] = tokens else {
        return;
    };
    if separator.text != "=" || values.is_empty() {
        return;
    }
    add_token_range(ranges, *name, SemanticKind::Group);
    for value in values {
        add_token_range(ranges, *value, SemanticKind::Object);
    }
}

fn scan_condition_reference_tokens(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    for token in tokens {
        let mut offset = 0usize;
        for part in token.text.split('.') {
            if is_builtin_condition_name(part) {
                add_token_subrange(
                    ranges,
                    *token,
                    offset,
                    offset + part.len(),
                    SemanticKind::Condition,
                );
            }
            offset += part.len() + 1;
        }
    }
}

fn is_builtin_condition_name(value: &str) -> bool {
    matches!(value, "win_conditions" | "lose_conditions")
}

fn scan_standard_move_call_line(
    scope: Option<SourceScope>,
    tokens: &[LineToken<'_>],
    ranges: &mut Vec<SemanticToken>,
) {
    if scope != Some(SourceScope::Other) {
        return;
    }
    if let [call] = tokens
        && call.text == "move"
    {
        add_token_range(ranges, *call, SemanticKind::Effect);
    }
}

fn scan_layer_assignment_line(
    scope: Option<SourceScope>,
    tokens: &[LineToken<'_>],
    ranges: &mut Vec<SemanticToken>,
) {
    if scope != Some(SourceScope::Layers) {
        return;
    }
    let Some(separator) = tokens.iter().position(|token| token.text == "=") else {
        let selector_start = usize::from(tokens.first().is_some_and(|token| token.text == "each"));
        if selector_start == 1 {
            add_token_range(ranges, tokens[0], SemanticKind::Keyword);
        }
        if tokens
            .first()
            .is_some_and(|token| matches!(token.text, "for" | "}"))
        {
            return;
        }
        for object in &tokens[selector_start..] {
            add_selector_object_token(ranges, *object);
        }
        return;
    };
    if separator > 0 {
        add_token_range(ranges, tokens[0], SemanticKind::Group);
    }
    for object in &tokens[separator + 1..] {
        add_selector_object_token(ranges, *object);
    }
}

fn add_selector_object_token(ranges: &mut Vec<SemanticToken>, token: LineToken<'_>) {
    let head = token
        .text
        .trim_matches(|ch: char| matches!(ch, '[' | ']' | '(' | ')' | '|'));
    let head_len = head.find([':', '{']).unwrap_or(head.len());
    if head_len == 0 {
        return;
    }
    let Some(relative_start) = token.text.find(&head[..head_len]) else {
        return;
    };
    add_token_subrange(
        ranges,
        token,
        relative_start,
        relative_start + head_len,
        SemanticKind::Object,
    );
}

fn scan_render_setting_line(
    scope: Option<SourceScope>,
    tokens: &[LineToken<'_>],
    ranges: &mut Vec<SemanticToken>,
) {
    if scope != Some(SourceScope::Other) {
        return;
    }
    if let Some(first) = tokens.first().copied()
        && first.text == "shade"
    {
        add_token_range(ranges, first, SemanticKind::Keyword);
    }
}

fn scan_levels_header(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    let Some(first) = tokens.first().copied() else {
        return;
    };
    if !matches!(first.text, "levels" | "levels3") {
        return;
    }
    let tokens = trim_trailing_block_markers(tokens);
    match tokens {
        [_, of, puzzle] if of.text == "of" => {
            add_token_range(ranges, *of, SemanticKind::Keyword);
            add_token_range(ranges, *puzzle, SemanticKind::Scene);
        }
        [_, pack, of, puzzle] if of.text == "of" => {
            add_token_range(ranges, *pack, SemanticKind::Scene);
            add_token_range(ranges, *of, SemanticKind::Keyword);
            add_token_range(ranges, *puzzle, SemanticKind::Scene);
        }
        _ => {}
    }
}

fn scan_level_header(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    let Some(first) = tokens.first().copied() else {
        return;
    };
    if first.text != "level" {
        return;
    }
    add_token_range(ranges, first, SemanticKind::Keyword);
    let tokens = trim_trailing_block_markers(tokens);
    for name in tokens.iter().skip(1) {
        add_token_range(ranges, *name, SemanticKind::Scene);
    }
}

fn trim_trailing_block_markers<'a>(tokens: &'a [LineToken<'a>]) -> &'a [LineToken<'a>] {
    let mut end = tokens.len();
    while end > 0 && matches!(tokens[end - 1].text, "{" | "}") {
        end -= 1;
    }
    &tokens[..end]
}

fn scan_level_path_token(token: LineToken<'_>, ranges: &mut Vec<SemanticToken>) {
    if !token.text.contains('.') {
        return;
    }
    let parts = token.text.split('.').collect::<Vec<_>>();
    let mut offset = 0usize;
    for (index, part) in parts.iter().enumerate() {
        if let Some(syntax) = level_path_part_syntax(&parts, index) {
            let kind = match syntax {
                LevelPathPartSyntax::Owner => SemanticKind::State,
                LevelPathPartSyntax::TextProperty => SemanticKind::String,
                LevelPathPartSyntax::NumberProperty => SemanticKind::Number,
                LevelPathPartSyntax::ConditionProperty => SemanticKind::Condition,
            };
            add_token_subrange(ranges, token, offset, offset + part.len(), kind);
        }
        offset += part.len() + 1;
    }
}

fn scan_key_semantic_line(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    let Some(separator) = tokens
        .iter()
        .position(|token| matches!(token.text, "=" | "->"))
    else {
        return;
    };
    for key in &tokens[..separator] {
        add_token_range(ranges, *key, SemanticKind::Input);
    }
    if matches!(tokens[separator].text, "->")
        && tokens[separator + 1..].len() == 1
        && is_semantic_identifier(tokens[separator + 1].text)
        && scene_effect_command_syntax(tokens[separator + 1].text).is_none()
        && rewrite_effect_command_syntax(tokens[separator + 1].text).is_none()
    {
        add_token_range(ranges, tokens[separator + 1], SemanticKind::Input);
        return;
    }
    scan_scene_effect_tokens(&tokens[separator + 1..], ranges);
}

fn is_semantic_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn scan_rewrite_effect_line(
    scope: Option<SourceScope>,
    tokens: &[LineToken<'_>],
    ranges: &mut Vec<SemanticToken>,
) {
    if let Some(arrow) = tokens.iter().position(|token| token.text == "->") {
        let rhs = &tokens[arrow + 1..];
        let effect_start = rhs
            .iter()
            .rposition(|token| token.text.contains(']'))
            .map_or(0, |index| index + 1);
        if effect_start < rhs.len() {
            scan_rewrite_effect_tokens(&rhs[effect_start..], ranges);
        }
        return;
    }

    if scope != Some(SourceScope::Other) {
        return;
    }
    scan_rewrite_effect_tokens(tokens, ranges);
}

fn scan_rewrite_effect_tokens(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    let source_tokens = line_tokens_as_source_tokens(tokens);
    ranges.extend(rewrite_effect_semantic_tokens(&source_tokens));
}

fn scan_scene_semantic_line(
    scope: Option<SourceScope>,
    tokens: &[LineToken<'_>],
    ranges: &mut Vec<SemanticToken>,
) {
    let Some(first) = tokens.first().copied() else {
        return;
    };
    match first.text {
        "scene" => {
            scan_scene_header(tokens, ranges);
            return;
        }
        "step" if scope == Some(SourceScope::SceneTransitions) => {
            scan_scene_step_line(tokens, ranges);
            return;
        }
        "button" => {
            scan_button_line(tokens, ranges);
        }
        _ => {}
    }

    if let Some(arrow) = tokens.iter().position(|token| token.text == "->") {
        if is_puzzle_rewrite_line(tokens) {
            return;
        }
        scan_scene_condition_prefix(&tokens[..arrow], ranges);
        scan_scene_effect_tokens(&tokens[arrow + 1..], ranges);
        return;
    }

    scan_scene_effect_tokens(tokens, ranges);
}

fn scan_scene_step_line(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    let [step, target] = tokens else {
        return;
    };
    add_token_range(ranges, *step, SemanticKind::Keyword);
    add_token_range(ranges, *target, SemanticKind::State);
}

fn scan_scene_condition_prefix(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    if let Some(first) = tokens.first().copied() {
        if first.text == "if" {
            add_token_range(ranges, first, SemanticKind::Keyword);
        }
    }
}

fn scan_scene_header(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    let Some(name) = tokens.get(1).copied() else {
        return;
    };
    add_token_range(ranges, name, SemanticKind::Scene);
}

fn scan_button_line(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    if let Some(value_index) = tokens.iter().position(|token| token.text == "value") {
        add_token_range(ranges, tokens[value_index], SemanticKind::Keyword);
    }
    if let Some(assign) = tokens.iter().position(|token| token.text == "=") {
        if let Some(command) = tokens.get(assign + 1).copied() {
            add_command_token(ranges, command);
        }
        return;
    }
    if let Some(arrow) = tokens.iter().position(|token| token.text == "->") {
        scan_scene_effect_tokens(&tokens[arrow + 1..], ranges);
    }
}

fn is_puzzle_rewrite_line(tokens: &[LineToken<'_>]) -> bool {
    tokens
        .iter()
        .any(|token| token.text.contains('[') || token.text.contains(']'))
}

fn add_command_token(ranges: &mut Vec<SemanticToken>, token: LineToken<'_>) {
    if let Some(cursor_offset) = token.text.find("cursor.") {
        add_token_subrange(
            ranges,
            token,
            cursor_offset,
            cursor_offset + "cursor".len(),
            SemanticKind::State,
        );
        let value_start = cursor_offset + "cursor.".len();
        if let Some(value_end) = identifier_end(token.text, value_start) {
            let value = &token.text[value_start..value_end];
            let kind = if matches!(value, "prev" | "next") {
                SemanticKind::Effect
            } else {
                SemanticKind::Literal
            };
            add_token_subrange(ranges, token, value_start, value_end, kind);
        }
    }

    let Some((first_start, first_end)) = first_identifier_bounds(token.text) else {
        return;
    };
    let after_first = &token.text[first_end..];
    if after_first.starts_with('.') {
        add_token_subrange(ranges, token, first_start, first_end, SemanticKind::Scene);
        let command_start = first_end + 1;
        if let Some(command_end) = identifier_end(token.text, command_start) {
            add_token_subrange(
                ranges,
                token,
                command_start,
                command_end,
                SemanticKind::Effect,
            );
        }
    } else {
        add_token_subrange(ranges, token, first_start, first_end, SemanticKind::Input);
        if after_first.starts_with(':') {
            let binding_start = first_end + 1;
            if let Some(binding_end) = identifier_end(token.text, binding_start) {
                add_token_subrange(
                    ranges,
                    token,
                    binding_start,
                    binding_end,
                    SemanticKind::Binding,
                );
            }
        }
    }
}

fn scan_scene_effect_tokens(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    let source_tokens = line_tokens_as_source_tokens(tokens);
    ranges.extend(scene_effect_semantic_tokens(&source_tokens));
}

fn line_tokens_as_source_tokens(tokens: &[LineToken<'_>]) -> Vec<SourceToken> {
    tokens
        .iter()
        .map(|token| SourceToken {
            text: token.text.to_string(),
            start: token.start,
            end: token.end,
        })
        .collect()
}

fn add_token_range(ranges: &mut Vec<SemanticToken>, token: LineToken<'_>, kind: SemanticKind) {
    let Some((start, end)) = identifier_bounds(token) else {
        return;
    };
    ranges.push(SemanticToken { start, end, kind });
}

fn add_token_subrange(
    ranges: &mut Vec<SemanticToken>,
    token: LineToken<'_>,
    relative_start: usize,
    relative_end: usize,
    kind: SemanticKind,
) {
    if relative_start >= relative_end || relative_end > token.text.len() {
        return;
    }
    ranges.push(SemanticToken {
        start: token.start + relative_start,
        end: token.start + relative_end,
        kind,
    });
}

pub(crate) fn first_identifier_bounds(value: &str) -> Option<(usize, usize)> {
    let start = value
        .char_indices()
        .find_map(|(index, ch)| is_word_start(ch).then_some(index))?;
    identifier_end(value, start).map(|end| (start, end))
}

fn identifier_end(value: &str, start: usize) -> Option<usize> {
    if start >= value.len() {
        return None;
    }
    let mut end = start;
    for (offset, ch) in value[start..].char_indices() {
        if offset == 0 {
            if !is_word_start(ch) {
                return None;
            }
        } else if !is_word_continue(ch) || matches!(ch, ':' | '.') {
            break;
        }
        end = start + offset + ch.len_utf8();
    }
    (end > start).then_some(end)
}

fn identifier_bounds(token: LineToken<'_>) -> Option<(usize, usize)> {
    let start_offset = token
        .text
        .char_indices()
        .find_map(|(index, ch)| is_word_start(ch).then_some(index))?;
    let end_offset = token
        .text
        .char_indices()
        .rev()
        .find_map(|(index, ch)| is_word_continue(ch).then_some(index + ch.len_utf8()))?;
    let start = token.start + start_offset;
    let end = token.start + end_offset;
    debug_assert!(end <= token.end);
    (start < end).then_some((start, end))
}

fn source_tokens_as_line_tokens(tokens: &[SourceToken]) -> Vec<LineToken<'_>> {
    tokens
        .iter()
        .map(|token| LineToken {
            text: token.text.as_str(),
            start: token.start,
            end: token.end,
        })
        .collect()
}

fn is_word_start(ch: char) -> bool {
    ch == '@' || ch == '_' || ch.is_ascii_alphabetic()
}

fn is_word_continue(ch: char) -> bool {
    ch == '@' || ch == '_' || ch == '-' || ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::{
        SemanticCompletionSlot, SemanticKind, semantic_completion_context, semantic_tokens,
    };

    #[test]
    fn classifies_same_word_by_source_scope() {
        let source = r#"
title semantic_scope
sounds {
sfx clear seed=clear01 type=jump
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
    fn classifies_rewrite_effects_from_parser_owned_tokens() {
        let source = r#"
title rewrite_effect_semantics
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
    fn classifies_standard_move_call_as_routine_effect() {
        let source = r#"
title standard_move_semantics

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
title rewrite_prefix_semantics

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
title surface_semantic_projection

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
        let surface_tokens = crate::surface_document_semantic_tokens(source);
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
title anonymous_layer_semantics

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
    fn classifies_tag_set_definitions_as_groups_and_object_name_atoms() {
        let source = r#"
title tag_semantics

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
                && token.kind == SemanticKind::Object
        }));
        assert!(tokens.iter().any(|token| {
            token.start == left_start
                && token.end == left_start + "left".len()
                && token.kind == SemanticKind::Object
        }));
    }

    #[test]
    fn classifies_theme_state_and_condition_contexts() {
        let source = r#"
title semantic_contexts
theme clean
var count = 1

scene playing {
if win_conditions -> goto title
if board.win_conditions -> goto title
}
"#;
        let tokens = semantic_tokens(source);
        let theme_start = source.find("clean").unwrap();
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
    fn classifies_scene_step_rule_as_scene_transition_directive() {
        let source = r#"
title scene_step_semantics

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
title semantic_surface_roles

scene title {
layout {
title
}
}
"#;
        let tokens = semantic_tokens(source);
        let metadata_title_start = source.find("title semantic_surface_roles").unwrap();
        let scene_title_start = source.find("scene title").unwrap() + "scene ".len();
        let component_title_start = source.rfind("title\n").unwrap();

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
title visual_shape_semantics

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
    fn completion_context_uses_source_scope_for_same_word() {
        let sounds_source = r#"
title completion_sounds_scope
sounds {
s
}
"#;
        let sounds_cursor = sounds_source.rfind("\ns\n").unwrap() + "\ns".len();
        let sounds_context = semantic_completion_context(sounds_source, sounds_cursor);
        assert!(sounds_context.slots.iter().any(|slot| {
            matches!(slot, SemanticCompletionSlot::Keywords(keywords) if keywords.contains(&"sfx"))
        }));
        assert!(
            !sounds_context
                .slots
                .contains(&SemanticCompletionSlot::Emissions)
        );

        let scene_source = r#"
title completion_scene_scope
scene playing {
rules {
win -> s
}
}
"#;
        let scene_cursor = scene_source.find("win -> s").unwrap() + "win -> s".len();
        let scene_context = semantic_completion_context(scene_source, scene_cursor);
        assert!(
            scene_context
                .slots
                .contains(&SemanticCompletionSlot::SceneEffects)
        );
        assert!(!scene_context.slots.iter().any(|slot| {
            matches!(slot, SemanticCompletionSlot::Keywords(keywords) if keywords.contains(&"sfx"))
        }));
    }
}
