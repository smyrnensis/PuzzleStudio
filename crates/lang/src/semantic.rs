use crate::source::{SourceScope, SourceToken, scan_source_context};
use crate::{
    LevelPathPartSyntax, MapHeaderTokenSyntax, RewriteEffectCommandSyntax, SceneStateLhsSyntax,
    SoundSettingValueSyntax, level_path_part_syntax, map_header_token_syntax,
    metadata_directive_value_token_index, rewrite_direction_prefix_token_index,
    rewrite_effect_command_syntax, rewrite_effect_semantic_tokens, scene_effect_command_syntax,
    scene_effect_semantic_tokens, scene_state_lhs_syntax, sound_setting_value_syntax,
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
    Query,
    Scene,
    Asset,
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
    Objects,
    Groups,
    States,
    Scratches,
    Variants,
    ValueSets,
    Directions,
    DirectionSets,
    Inputs,
    Commands,
    Effects,
    Emissions,
    Routines,
    Queries,
    Scenes,
    Puzzles,
    Levels,
    SfxAssets,
    MusicAssets,
    Sprites,
    Assets,
}

pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let context = scan_source_context(source);
    let mut tokens = Vec::new();
    for line in &context.lines {
        let line_tokens = source_tokens_as_line_tokens(&line.token_spans);
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
    } else if previous.as_deref() == Some("goto") {
        vec![SemanticCompletionSlot::Scenes]
    } else if previous.as_deref() == Some("of") {
        vec![SemanticCompletionSlot::Puzzles]
    } else if previous.as_deref() == Some("sfx") && !sounds_definition_scope {
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
        default_completion_slots(scope)
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
            SemanticCompletionSlot::Variants,
        ]);
    }

    let line = line_at_cursor(context, token.replace_start)?;
    let line_end = line.start + line.content.len();
    let before = &source[line.start..token.replace_start.min(line_end)];
    let after = &source[token.replace_end.min(line_end)..line_end];
    let previous = previous_completion_token(source, token.replace_start);

    if inside_scratch_selector_attrs(before) {
        return Some(vec![
            SemanticCompletionSlot::Directions,
            SemanticCompletionSlot::DirectionSets,
            SemanticCompletionSlot::Scratches,
        ]);
    }

    if previous.as_deref() == Some("in") {
        return Some(vec![
            SemanticCompletionSlot::ValueSets,
            SemanticCompletionSlot::Groups,
        ]);
    }

    if is_rule_like_scope(scope) && next_non_whitespace_starts_pattern(after) {
        return Some(vec![
            SemanticCompletionSlot::Directions,
            SemanticCompletionSlot::DirectionSets,
        ]);
    }

    None
}

fn line_at_cursor<'a>(
    context: &'a crate::source::SourceContext,
    cursor: usize,
) -> Option<&'a crate::source::SourceContextLine> {
    context.lines.iter().find(|line| {
        let end = line.start + line.content.len();
        cursor >= line.start && cursor <= end
    })
}

fn inside_scratch_selector_attrs(before: &str) -> bool {
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

pub(crate) fn semantic_builtin_effect_commands() -> Vec<(&'static str, SemanticKind)> {
    let commands = [
        "apply",
        "again",
        "back",
        "cancel",
        "clear_history",
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
        "pause_music",
        "play_music",
        "reset",
        "restart",
        "resume_music",
        "set",
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
            let kind = match rewrite_effect_command_syntax(command) {
                Some(RewriteEffectCommandSyntax::Emission) => SemanticKind::Emission,
                Some(RewriteEffectCommandSyntax::Effect) => SemanticKind::Effect,
                None if scene_effect_command_syntax(command).is_some() || command == "restart" => {
                    SemanticKind::Effect
                }
                None => return None,
            };
            Some((command, kind))
        })
        .collect()
}

pub(crate) fn is_completion_keyword(token: &str) -> bool {
    COMPLETION_KEYWORDS.contains(&token)
}

fn default_completion_slots(scope: Option<SourceScope>) -> Vec<SemanticCompletionSlot> {
    let sounds_definition_scope = scope == Some(SourceScope::Sounds);
    let mut slots = vec![
        SemanticCompletionSlot::Keywords(completion_keywords_for_scope(scope)),
        SemanticCompletionSlot::Objects,
        SemanticCompletionSlot::Groups,
        SemanticCompletionSlot::States,
        SemanticCompletionSlot::Scratches,
        SemanticCompletionSlot::Variants,
        SemanticCompletionSlot::ValueSets,
        SemanticCompletionSlot::Directions,
        SemanticCompletionSlot::Inputs,
        SemanticCompletionSlot::Commands,
    ];
    if !sounds_definition_scope {
        slots.push(SemanticCompletionSlot::Effects);
        slots.push(SemanticCompletionSlot::Emissions);
    }
    slots.extend([
        SemanticCompletionSlot::Routines,
        SemanticCompletionSlot::Queries,
        SemanticCompletionSlot::Scenes,
        SemanticCompletionSlot::Puzzles,
        SemanticCompletionSlot::Levels,
    ]);
    if !sounds_definition_scope {
        slots.push(SemanticCompletionSlot::SfxAssets);
        slots.push(SemanticCompletionSlot::MusicAssets);
    }
    slots.extend([
        SemanticCompletionSlot::Sprites,
        SemanticCompletionSlot::Assets,
    ]);
    slots
}

fn completion_keywords_for_scope(scope: Option<SourceScope>) -> &'static [&'static str] {
    match scope {
        None => TOP_LEVEL_COMPLETION_KEYWORDS,
        Some(SourceScope::Sounds) => SOUNDS_COMPLETION_KEYWORDS,
        Some(SourceScope::Assets) => ASSET_COMPLETION_KEYWORDS,
        Some(SourceScope::Puzzle) => PUZZLE_COMPLETION_KEYWORDS,
        Some(SourceScope::Objects) => OBJECT_COMPLETION_KEYWORDS,
        Some(SourceScope::Tags) => TAG_COMPLETION_KEYWORDS,
        Some(SourceScope::Group) => GROUP_COMPLETION_KEYWORDS,
        Some(SourceScope::Layers) => LAYER_COMPLETION_KEYWORDS,
        Some(SourceScope::Scratch) => SCRATCH_COMPLETION_KEYWORDS,
        Some(SourceScope::Keys) | Some(SourceScope::SceneKeys) => KEY_COMPLETION_KEYWORDS,
        Some(SourceScope::Legend) => LEGEND_COMPLETION_KEYWORDS,
        Some(SourceScope::Levels) | Some(SourceScope::Level) | Some(SourceScope::UnbracedLevel) => {
            LEVEL_COMPLETION_KEYWORDS
        }
        Some(
            SourceScope::Scene
            | SourceScope::SceneView
            | SourceScope::SceneState
            | SourceScope::SceneTransitions
            | SourceScope::LevelMenu,
        ) => SCENE_COMPLETION_KEYWORDS,
        Some(
            SourceScope::Visuals
            | SourceScope::VisualShapeTable
            | SourceScope::VisualShapeEntry
            | SourceScope::VisualColorTable
            | SourceScope::VisualPaletteTable,
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

const TOP_LEVEL_COMPLETION_KEYWORDS: &[&str] = &[
    "assets",
    "author",
    "sounds",
    "const",
    "global",
    "homepage",
    "import",
    "levels",
    "levels3",
    "music",
    "puzzle",
    "puzzle3",
    "resources",
    "scene",
    "sfx",
    "sprites",
    "sprites3",
    "subtitle",
    "theme",
    "title",
    "var",
];

const SOUNDS_COMPLETION_KEYWORDS: &[&str] = &["music", "sfx"];

const ASSET_COMPLETION_KEYWORDS: &[&str] = &["css"];

const PUZZLE_COMPLETION_KEYWORDS: &[&str] = &[
    "collision_layers",
    "command",
    "const",
    "display_objects",
    "for",
    "global",
    "group",
    "if",
    "input",
    "inputs",
    "keys",
    "layer",
    "layers",
    "legend",
    "level",
    "levels",
    "levels3",
    "lose_conditions",
    "objects",
    "on_display",
    "on_level_clear",
    "on_level_start",
    "once",
    "once_all",
    "once_per_level",
    "persistent",
    "query",
    "repeat",
    "resources",
    "render",
    "routine",
    "rule",
    "rules",
    "scratch",
    "state",
    "tags",
    "var",
    "win_conditions",
];

const OBJECT_COMPLETION_KEYWORDS: &[&str] = &["display", "each"];
const TAG_COMPLETION_KEYWORDS: &[&str] = &[];
const GROUP_COMPLETION_KEYWORDS: &[&str] = &["display", "each"];
const LAYER_COMPLETION_KEYWORDS: &[&str] = &["display", "each"];
const SCRATCH_COMPLETION_KEYWORDS: &[&str] = &["const", "global", "persistent", "var"];
const KEY_COMPLETION_KEYWORDS: &[&str] = &["command", "direction", "input"];
const LEGEND_COMPLETION_KEYWORDS: &[&str] = &["empty"];
const LEVEL_COMPLETION_KEYWORDS: &[&str] = &["legend", "level", "of"];

const SCENE_COMPLETION_KEYWORDS: &[&str] = &[
    "button",
    "column",
    "const",
    "else",
    "for",
    "if",
    "inputs",
    "keys",
    "level_menu",
    "menu",
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
    "view",
    "with",
];

const VISUAL_COMPLETION_KEYWORDS: &[&str] = &["colors", "shape", "sprite"];

const COMPLETION_KEYWORDS: &[&str] = &[
    "assets",
    "author",
    "sounds",
    "button",
    "column",
    "command",
    "const",
    "colors",
    "collision_layers",
    "component_effect",
    "css",
    "direction",
    "display",
    "display_objects",
    "each",
    "else",
    "for",
    "from",
    "global",
    "group",
    "homepage",
    "if",
    "import",
    "input",
    "inputs",
    "interactive_look",
    "interactive_zoom",
    "keys",
    "layer",
    "layers",
    "legend",
    "level",
    "level_menu",
    "levels",
    "levels3",
    "lose_conditions",
    "map",
    "music",
    "objects",
    "of",
    "on_display",
    "on_level_clear",
    "on_level_start",
    "once",
    "once_all",
    "once_per_level",
    "box",
    "persistent",
    "puzzle",
    "puzzle3",
    "query",
    "repeat",
    "resources",
    "render",
    "row",
    "routine",
    "rule",
    "rules",
    "scene",
    "scratch",
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
    "view",
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
    scan_rewrite_direction_prefix(tokens, ranges);
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
                | SourceScope::SceneView
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
        _ => {}
    }
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
    add_token_subrange(ranges, token, 0, key.len(), SemanticKind::Keyword);
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
                LevelPathPartSyntax::LevelRef => SemanticKind::Keyword,
                LevelPathPartSyntax::TextProperty => SemanticKind::String,
                LevelPathPartSyntax::NumberProperty => SemanticKind::Number,
                LevelPathPartSyntax::ConditionProperty => SemanticKind::Query,
            };
            add_token_subrange(ranges, token, offset, offset + part.len(), kind);
        }
        offset += part.len() + 1;
    }
}

fn scan_key_semantic_line(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    let Some(separator) = tokens
        .iter()
        .position(|token| matches!(token.text, "=" | "->" | "<-"))
    else {
        return;
    };
    for key in &tokens[..separator] {
        add_token_range(ranges, *key, SemanticKind::Input);
    }
    if tokens[separator].text == "<-" {
        for key in &tokens[separator + 1..] {
            add_token_range(ranges, *key, SemanticKind::Input);
        }
        return;
    }
    scan_scene_effect_tokens(&tokens[separator + 1..], ranges);
}

fn scan_rewrite_direction_prefix(tokens: &[LineToken<'_>], ranges: &mut Vec<SemanticToken>) {
    let token_texts = tokens.iter().map(|token| token.text).collect::<Vec<_>>();
    let Some(index) = rewrite_direction_prefix_token_index(&token_texts)
        .or_else(|| rewrite3_prefix_token_index(&token_texts))
    else {
        return;
    };
    add_frame_orientation_token(ranges, tokens[index]);
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

fn rewrite3_prefix_token_index(tokens: &[&str]) -> Option<usize> {
    let mut index = 0usize;
    while tokens.get(index).is_some_and(|token| {
        matches!(
            *token,
            "fix" | "once" | "once_all" | "once_per_level" | "repeat"
        )
    }) {
        index += 1;
    }
    let direction = tokens.get(index).copied()?;
    if !rewrite3_orientation_word(direction) {
        return None;
    }
    tokens
        .get(index + 1)
        .is_some_and(|token| matches!(*token, "[" | "{"))
        .then_some(index)
}

fn rewrite3_orientation_word(value: &str) -> bool {
    matches!(
        value,
        "forward" | "backward" | "frames" | "canonical" | "mirrored"
    ) || frame_orientation_word(value)
}

fn frame_orientation_word(value: &str) -> bool {
    let parts = value.split(':').collect::<Vec<_>>();
    matches!(parts.len(), 2 | 3) && parts.into_iter().all(frame_slot_word)
}

fn frame_slot_word(value: &str) -> bool {
    matches!(
        value,
        "_" | "up"
            | "down"
            | "left"
            | "right"
            | "forward"
            | "backward"
            | "directions"
            | "horizontal"
            | "vertical"
    )
}

fn add_frame_orientation_token(ranges: &mut Vec<SemanticToken>, token: LineToken<'_>) {
    if !token.text.contains(':') {
        add_token_range(ranges, token, SemanticKind::Keyword);
        return;
    }

    let mut offset = 0usize;
    for part in token.text.split(':') {
        if !part.is_empty() {
            add_token_subrange(
                ranges,
                token,
                offset,
                offset + part.len(),
                SemanticKind::Keyword,
            );
        }
        offset += part.len() + 1;
    }
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

fn first_identifier_bounds(value: &str) -> Option<(usize, usize)> {
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
                && token.kind == SemanticKind::Emission
        }));
    }

    #[test]
    fn classifies_rewrite_effects_from_parser_owned_tokens() {
        let source = r#"
title rewrite_effect_semantics
puzzle default {
rules {
once [ Player ] -> [ Player ] sfx clear
set score = 1
}
}
"#;
        let tokens = semantic_tokens(source);
        let sfx_start = source.find("sfx clear").unwrap();
        let clear_start = source.find("clear").unwrap();
        let set_start = source.find("set score").unwrap();
        let score_start = source.find("score").unwrap();

        assert!(tokens.iter().any(|token| {
            token.start == sfx_start
                && token.end == sfx_start + "sfx".len()
                && token.kind == SemanticKind::Emission
        }));
        assert!(tokens.iter().any(|token| {
            token.start == clear_start
                && token.end == clear_start + "clear".len()
                && token.kind == SemanticKind::Asset
        }));
        assert!(tokens.iter().any(|token| {
            token.start == set_start
                && token.end == set_start + "set".len()
                && token.kind == SemanticKind::Effect
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
view {
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
                .contains(&SemanticCompletionSlot::Emissions)
        );
        assert!(!scene_context.slots.iter().any(|slot| {
            matches!(slot, SemanticCompletionSlot::Keywords(keywords) if keywords.contains(&"sfx"))
        }));
    }
}
