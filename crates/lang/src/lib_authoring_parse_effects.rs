#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SceneEffectCommandSyntax {
    Plain,
    InputTarget,
    ComponentEffectTarget,
    SceneTarget,
    AssetTarget,
    OptionalAssetTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RewriteEffectCommandSyntax {
    Effect,
    Emission,
}

pub(crate) fn scene_effect_command_syntax(token: &str) -> Option<SceneEffectCommandSyntax> {
    match token {
        "input" => Some(SceneEffectCommandSyntax::InputTarget),
        "component_effect" => Some(SceneEffectCommandSyntax::ComponentEffectTarget),
        "goto" | "start" => Some(SceneEffectCommandSyntax::SceneTarget),
        "sfx" | "play_music" => Some(SceneEffectCommandSyntax::AssetTarget),
        "pause_music" | "resume_music" | "stop_music" => {
            Some(SceneEffectCommandSyntax::OptionalAssetTarget)
        }
        "apply"
        | "clear_history"
        | "clear_undo_history"
        | "clear_game_progress"
        | "clear"
        | "copy"
        | "load"
        | "message"
        | "wait" => Some(SceneEffectCommandSyntax::Plain),
        _ => None,
    }
}

pub(crate) fn scene_effect_semantic_tokens(tokens: &[SourceToken]) -> Vec<semantic::SemanticToken> {
    project_surface_semantic_tokens(&scene_effect_surface_document(tokens).semantic_tokens)
}

fn project_surface_semantic_tokens(
    tokens: &[SurfaceSemanticToken],
) -> Vec<semantic::SemanticToken> {
    tokens
        .iter()
        .map(|token| semantic::SemanticToken {
            start: token.span.start,
            end: token.span.end,
            kind: match token.kind {
                SurfaceSemanticKind::Keyword => semantic::SemanticKind::Keyword,
                SurfaceSemanticKind::Literal => semantic::SemanticKind::Literal,
                SurfaceSemanticKind::Binding => semantic::SemanticKind::Binding,
                SurfaceSemanticKind::Effect => semantic::SemanticKind::Effect,
                SurfaceSemanticKind::Emission => semantic::SemanticKind::Emission,
                SurfaceSemanticKind::Input => semantic::SemanticKind::Input,
                SurfaceSemanticKind::State => semantic::SemanticKind::State,
                SurfaceSemanticKind::Condition => semantic::SemanticKind::Condition,
                SurfaceSemanticKind::Scene => semantic::SemanticKind::Scene,
                SurfaceSemanticKind::Asset => semantic::SemanticKind::Asset,
                SurfaceSemanticKind::Setting => semantic::SemanticKind::Setting,
                SurfaceSemanticKind::Number => semantic::SemanticKind::Number,
                SurfaceSemanticKind::String => semantic::SemanticKind::String,
            },
        })
        .collect()
}

fn scene_effect_surface_document(tokens: &[SourceToken]) -> SurfaceDocument {
    if let Some(parts) = split_scene_effect_token_sequence(tokens) {
        let mut sink = SurfaceSink::default();
        for part in parts {
            sink.extend(scene_effect_surface_document(part));
        }
        return sink.into_document();
    }

    let mut sink = SurfaceSink::default();
    let Some(first) = tokens.first() else {
        return sink.into_document();
    };
    let effect_span = source_tokens_span(tokens);

    if first.text.starts_with("cursor.") {
        add_cursor_scene_effect_token(&mut sink, first);
        return surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span);
    }

    if first.text.contains('.') {
        let mut parts = first.text.split('.');
        if let Some(target) = parts.next() {
            add_scene_effect_token_part(&mut sink, first, target, SurfaceSemanticKind::Scene);
        }
        if let Some(effect) = parts.next() {
            add_scene_effect_token_part(&mut sink, first, effect, SurfaceSemanticKind::Effect);
        }
        return surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span);
    }

    if first.text == "start" && add_level_flow_scene_effect_tokens(tokens, &mut sink) {
        return surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span);
    }

    match scene_effect_command_syntax(&first.text) {
        Some(SceneEffectCommandSyntax::InputTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(input) = tokens.get(1) {
                add_scene_command_token(&mut sink, input);
            }
        }
        Some(SceneEffectCommandSyntax::ComponentEffectTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(effect) = tokens.get(1) {
                add_scene_command_token(&mut sink, effect);
            }
        }
        Some(SceneEffectCommandSyntax::SceneTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(scene) = tokens.get(1) {
                add_scene_effect_token_range(&mut sink, scene, SurfaceSemanticKind::Scene);
            }
        }
        Some(SceneEffectCommandSyntax::AssetTarget) => {
            let kind = scene_effect_command_kind(&first.text);
            add_scene_effect_token_range(&mut sink, first, kind);
            if let Some(asset) = tokens.get(1) {
                add_scene_effect_token_range(&mut sink, asset, SurfaceSemanticKind::Asset);
            }
        }
        Some(SceneEffectCommandSyntax::OptionalAssetTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(asset) = tokens.get(1) {
                add_scene_effect_token_range(&mut sink, asset, SurfaceSemanticKind::Asset);
            }
        }
        Some(SceneEffectCommandSyntax::Plain) => {
            let kind = scene_effect_command_kind(&first.text);
            add_scene_effect_token_range(&mut sink, first, kind);
        }
        None => {}
    }

    surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span)
}

fn source_tokens_span(tokens: &[SourceToken]) -> Option<SourceSpan> {
    let start = tokens.first()?.start;
    let end = tokens.last()?.end;
    (start < end).then_some(SourceSpan { start, end })
}

fn surface_document_with_node(
    mut sink: SurfaceSink,
    kind: SurfaceNodeKind,
    span: Option<SourceSpan>,
) -> SurfaceDocument {
    if sink.has_semantic_tokens()
        && let Some(span) = span
    {
        sink.node(kind, span);
    }
    sink.into_document()
}

fn scene_effect_command_kind(token: &str) -> SurfaceSemanticKind {
    if matches!(
        token,
        "sfx" | "play_music" | "pause_music" | "resume_music" | "stop_music"
    ) {
        return SurfaceSemanticKind::Effect;
    }
    if matches!(
        rewrite_effect_command_syntax(token),
        Some(RewriteEffectCommandSyntax::Emission)
    ) {
        SurfaceSemanticKind::Emission
    } else {
        SurfaceSemanticKind::Effect
    }
}

fn add_level_flow_scene_effect_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) -> bool {
    match tokens {
        [command, levels, in_keyword, scene]
            if levels.text == "levels" && in_keyword.text == "in" =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            add_scene_effect_token_range(sink, levels, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, in_keyword, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, scene, SurfaceSemanticKind::Scene);
            true
        }
        [command, levels, scope, in_keyword, scene]
            if levels.text == "levels" && in_keyword.text == "in" =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            add_scene_effect_token_range(sink, levels, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, scope, SurfaceSemanticKind::Scene);
            add_scene_effect_token_range(sink, in_keyword, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, scene, SurfaceSemanticKind::Scene);
            true
        }
        _ => false,
    }
}

fn add_scene_command_token(sink: &mut SurfaceSink, token: &SourceToken) {
    if let Some(cursor_offset) = token.text.find("cursor.") {
        add_scene_effect_token_subrange(
            sink,
            token,
            cursor_offset,
            cursor_offset + "cursor".len(),
            SurfaceSemanticKind::State,
        );
        let value_start = cursor_offset + "cursor.".len();
        if let Some(value_end) = scene_effect_identifier_end(&token.text, value_start) {
            let value = &token.text[value_start..value_end];
            let kind = if matches!(value, "prev" | "next") {
                SurfaceSemanticKind::Effect
            } else {
                SurfaceSemanticKind::Literal
            };
            add_scene_effect_token_subrange(sink, token, value_start, value_end, kind);
        }
    }

    let Some((first_start, first_end)) = scene_effect_first_identifier_bounds(&token.text) else {
        return;
    };
    let after_first = &token.text[first_end..];
    if after_first.starts_with('.') {
        add_scene_effect_token_subrange(
            sink,
            token,
            first_start,
            first_end,
            SurfaceSemanticKind::Scene,
        );
        let command_start = first_end + 1;
        if let Some(command_end) = scene_effect_identifier_end(&token.text, command_start) {
            add_scene_effect_token_subrange(
                sink,
                token,
                command_start,
                command_end,
                SurfaceSemanticKind::Effect,
            );
        }
    } else {
        add_scene_effect_token_subrange(
            sink,
            token,
            first_start,
            first_end,
            SurfaceSemanticKind::Input,
        );
        if after_first.starts_with(':') {
            let binding_start = first_end + 1;
            if let Some(binding_end) = scene_effect_identifier_end(&token.text, binding_start) {
                add_scene_effect_token_subrange(
                    sink,
                    token,
                    binding_start,
                    binding_end,
                    SurfaceSemanticKind::Binding,
                );
            }
        }
    }
}

fn add_cursor_scene_effect_token(sink: &mut SurfaceSink, token: &SourceToken) {
    add_scene_effect_token_part(sink, token, "cursor", SurfaceSemanticKind::State);
    if let Some((_, tail)) = token.text.split_once('.') {
        let kind = if matches!(tail, "prev" | "next") {
            SurfaceSemanticKind::Effect
        } else {
            SurfaceSemanticKind::Literal
        };
        add_scene_effect_token_part(sink, token, tail, kind);
    }
}

fn add_scene_effect_token_range(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    kind: SurfaceSemanticKind,
) {
    let Some((start, end)) = scene_effect_identifier_bounds(token) else {
        return;
    };
    sink.mark(SourceSpan { start, end }, kind);
}

fn add_scene_effect_token_part(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    part: &str,
    kind: SurfaceSemanticKind,
) {
    if part.is_empty() {
        return;
    }
    if let Some(relative) = token.text.find(part) {
        sink.mark(
            SourceSpan {
                start: token.start + relative,
                end: token.start + relative + part.len(),
            },
            kind,
        );
    }
}

fn add_scene_effect_token_subrange(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    relative_start: usize,
    relative_end: usize,
    kind: SurfaceSemanticKind,
) {
    if relative_start >= relative_end || relative_end > token.text.len() {
        return;
    }
    sink.mark(
        SourceSpan {
            start: token.start + relative_start,
            end: token.start + relative_end,
        },
        kind,
    );
}

fn scene_effect_first_identifier_bounds(value: &str) -> Option<(usize, usize)> {
    let start = value
        .char_indices()
        .find_map(|(index, ch)| scene_effect_is_word_start(ch).then_some(index))?;
    scene_effect_identifier_end(value, start).map(|end| (start, end))
}

fn scene_effect_identifier_end(value: &str, start: usize) -> Option<usize> {
    if start >= value.len() {
        return None;
    }
    let mut end = start;
    for (offset, ch) in value[start..].char_indices() {
        if offset == 0 {
            if !scene_effect_is_word_start(ch) {
                return None;
            }
        } else if !scene_effect_is_word_continue(ch) || matches!(ch, ':' | '.') {
            break;
        }
        end = start + offset + ch.len_utf8();
    }
    (end > start).then_some(end)
}

fn scene_effect_identifier_bounds(token: &SourceToken) -> Option<(usize, usize)> {
    let start_offset = token
        .text
        .char_indices()
        .find_map(|(index, ch)| scene_effect_is_word_start(ch).then_some(index))?;
    let end_offset = token.text.char_indices().rev().find_map(|(index, ch)| {
        scene_effect_is_word_continue(ch).then_some(index + ch.len_utf8())
    })?;
    let start = token.start + start_offset;
    let end = token.start + end_offset;
    debug_assert!(end <= token.end);
    (start < end).then_some((start, end))
}

fn scene_effect_is_word_start(ch: char) -> bool {
    ch == '@' || ch == '_' || ch.is_ascii_alphabetic()
}

fn scene_effect_is_word_continue(ch: char) -> bool {
    ch == '@' || ch == '_' || ch == '-' || ch.is_ascii_alphanumeric()
}

impl EffectAst {
    fn command_syntax(&self) -> RewriteEffectCommandSyntax {
        match self {
            EffectAst::PlaySfx { .. }
            | EffectAst::PlayMusic { .. }
            | EffectAst::PauseMusic { .. }
            | EffectAst::ResumeMusic { .. }
            | EffectAst::StopMusic { .. }
            | EffectAst::Wait { .. }
            | EffectAst::WaitAnimation
            | EffectAst::Message { .. }
            | EffectAst::Scene(_) => RewriteEffectCommandSyntax::Emission,
            EffectAst::Cancel
            | EffectAst::Win
            | EffectAst::Restart
            | EffectAst::NextLevel
            | EffectAst::Again
            | EffectAst::Checkpoint
            | EffectAst::ClearCheckpoint
            | EffectAst::UpdateGlobal { .. } => RewriteEffectCommandSyntax::Effect,
        }
    }
}

pub(crate) fn rewrite_effect_command_syntax(token: &str) -> Option<RewriteEffectCommandSyntax> {
    let probe = match token {
        "again" | "cancel" | "win" | "restart" | "next_level" | "checkpoint"
        | "clear_checkpoint" | "wait" => token.to_string(),
        "message" => "message \"text\"".to_string(),
        "sfx" => "sfx __highlight_probe".to_string(),
        "play_music" => "play_music __highlight_probe".to_string(),
        "pause_music" => token.to_string(),
        "resume_music" => token.to_string(),
        "stop_music" => token.to_string(),
        _ => return None,
    };
    parse_rewrite_effect_value(&probe, &probe)
        .ok()
        .and_then(|effects| effects.into_iter().next())
        .map(|effect| effect.command_syntax())
}

pub(crate) fn rewrite_effect_semantic_tokens(
    tokens: &[SourceToken],
) -> Vec<semantic::SemanticToken> {
    project_surface_semantic_tokens(&rewrite_effect_surface_document(tokens).semantic_tokens)
}

fn rewrite_effect_surface_document(tokens: &[SourceToken]) -> SurfaceDocument {
    let mut sink = SurfaceSink::default();
    let effect_span = source_tokens_span(tokens);
    add_rewrite_effect_semantic_tokens(tokens, &mut sink);
    surface_document_with_node(sink, SurfaceNodeKind::RewriteEffect, effect_span)
}

fn add_rewrite_effect_semantic_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };

    if first.text == "message" {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Emission);
        if tokens.len() > 1 {
            let text_start = tokens[1].start;
            let text_end = tokens.last().map(|token| token.end).unwrap_or(text_start);
            if text_start < text_end {
                sink.mark(
                    SourceSpan {
                        start: text_start,
                        end: text_end,
                    },
                    SurfaceSemanticKind::String,
                );
            }
        }
        return true;
    }

    if tokens.len() > 2
        && tokens
            .iter()
            .any(|token| is_rewrite_effect_command_token(&token.text))
    {
        return add_simple_rewrite_effect_semantic_tokens(tokens, sink);
    }

    match tokens {
        [command] if command.text == "sfx" => {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            true
        }
        [command]
            if matches_rewrite_effect_command(
                &command.text,
                RewriteEffectCommandSyntax::Effect,
            ) =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            true
        }
        [command]
            if matches_rewrite_effect_command(
                &command.text,
                RewriteEffectCommandSyntax::Emission,
            ) =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Emission);
            true
        }
        [command, duration] if command.text == "wait" => {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Emission);
            add_scene_effect_token_range(sink, duration, SurfaceSemanticKind::Number);
            true
        }
        [command, asset] if command.text == "sfx" => {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            add_scene_effect_token_range(sink, asset, SurfaceSemanticKind::Asset);
            true
        }
        [name, op, value] if is_global_update_operator(&op.text) => {
            add_scene_effect_token_range(sink, name, SurfaceSemanticKind::State);
            add_scene_effect_token_range(sink, value, SurfaceSemanticKind::Number);
            true
        }
        _ => false,
    }
}

fn add_simple_rewrite_effect_semantic_tokens(
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    let mut index = 0usize;
    let mut parsed_any = false;
    while index < tokens.len() {
        match tokens[index].text.to_ascii_lowercase().as_str() {
            "cancel" | "win" | "restart" | "next_level" | "again" | "checkpoint"
            | "clear_checkpoint" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Effect);
                index += 1;
                parsed_any = true;
            }
            "wait" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Emission);
                if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(&tokens[index + 1].text)
                {
                    add_scene_effect_token_range(
                        sink,
                        &tokens[index + 1],
                        SurfaceSemanticKind::Number,
                    );
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            "sfx" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Effect);
                if let Some(asset) = tokens.get(index + 1) {
                    add_scene_effect_token_range(sink, asset, SurfaceSemanticKind::Asset);
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            "play_music" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Emission);
                if let Some(asset) = tokens.get(index + 1) {
                    add_scene_effect_token_range(sink, asset, SurfaceSemanticKind::Asset);
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            "pause_music" | "resume_music" | "stop_music" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Emission);
                if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(&tokens[index + 1].text)
                {
                    add_scene_effect_token_range(
                        sink,
                        &tokens[index + 1],
                        SurfaceSemanticKind::Asset,
                    );
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            _ if index + 2 < tokens.len() && is_global_update_operator(&tokens[index + 1].text) => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::State);
                add_scene_effect_token_range(sink, &tokens[index + 2], SurfaceSemanticKind::Number);
                index += 3;
                parsed_any = true;
            }
            _ => {
                return parsed_any;
            }
        }
    }
    parsed_any
}

fn matches_rewrite_effect_command(token: &str, syntax: RewriteEffectCommandSyntax) -> bool {
    rewrite_effect_command_syntax(&token.to_ascii_lowercase()) == Some(syntax)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SoundSettingValueSyntax {
    String,
    Number,
}

pub(crate) fn sound_setting_value_syntax(key: &str) -> Option<SoundSettingValueSyntax> {
    match key {
        "seed" | "type" => Some(SoundSettingValueSyntax::String),
        "height" | "tone" | "bars" | "bpm" | "volume" => Some(SoundSettingValueSyntax::Number),
        _ => None,
    }
}

pub(crate) const SFX_SOUND_SETTING_OPTIONS: &[&str] = &["seed", "type", "volume"];
pub(crate) const MUSIC_SOUND_SETTING_OPTIONS: &[&str] =
    &["seed", "height", "bars", "bpm", "volume"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MapHeaderTokenSyntax {
    Keyword,
    Name,
    Axis,
}

pub(crate) fn map_header_token_syntax(
    tokens: &[&str],
    index: usize,
) -> Option<MapHeaderTokenSyntax> {
    if !matches!(tokens, ["map", _, _]) {
        return None;
    }
    match index {
        0 => Some(MapHeaderTokenSyntax::Keyword),
        1 => Some(MapHeaderTokenSyntax::Name),
        2 => Some(MapHeaderTokenSyntax::Axis),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SceneStateLhsSyntax {
    PuzzleSlot,
    Variable,
}

pub(crate) fn scene_state_lhs_syntax(tokens: &[&str]) -> Option<(usize, SceneStateLhsSyntax)> {
    match tokens {
        [name, "=", "puzzle", ..] if is_identifier(name) => {
            Some((0, SceneStateLhsSyntax::PuzzleSlot))
        }
        ["var" | "const", name, "=", ..] if is_identifier(name) => {
            Some((1, SceneStateLhsSyntax::Variable))
        }
        ["persistent", "var" | "const", name, "=", ..] if is_identifier(name) => {
            Some((2, SceneStateLhsSyntax::Variable))
        }
        ["persistent", name, "=", ..] if is_identifier(name) => {
            Some((1, SceneStateLhsSyntax::Variable))
        }
        [name, "=", ..] if is_identifier(name) => Some((0, SceneStateLhsSyntax::Variable)),
        _ => None,
    }
}

pub(crate) fn metadata_directive_value_token_index(tokens: &[&str]) -> Option<usize> {
    matches!(
        tokens,
        ["title" | "subtitle" | "author" | "homepage", _, ..]
    )
    .then_some(1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LevelPathPartSyntax {
    Owner,
    TextProperty,
    NumberProperty,
    ConditionProperty,
}

pub(crate) fn level_path_part_syntax(parts: &[&str], index: usize) -> Option<LevelPathPartSyntax> {
    match parts {
        ["level", property] => match index {
            0 => None,
            1 => level_property_syntax(property),
            _ => None,
        },
        [_, "level", property] => match index {
            0 => Some(LevelPathPartSyntax::Owner),
            1 => None,
            2 => level_property_syntax(property),
            _ => None,
        },
        _ => None,
    }
}

fn level_property_syntax(property: &str) -> Option<LevelPathPartSyntax> {
    match property {
        "name" | "label" | "title" => Some(LevelPathPartSyntax::TextProperty),
        "index" | "num" => Some(LevelPathPartSyntax::NumberProperty),
        "cleared" | "solved" | "last" | "has_next" => Some(LevelPathPartSyntax::ConditionProperty),
        _ => None,
    }
}

fn parse_scene_effect_with_optional_block(
    value: &str,
    lines: &[String],
    start: usize,
) -> Result<(SceneEffect, usize), DiagnosticReport> {
    let line = &lines[start];
    if value.is_empty() {
        return Err(parse_error(
            line,
            "effect block must use `{ ... }`; `end` effect blocks were removed",
        ));
    }
    if value == "{" {
        let block_end = matching_effect_block_end(lines, start, lines.len())?;
        let body = lines[start + 1..block_end].to_vec();
        if body.is_empty() {
            return Err(parse_error(
                line,
                "effect block requires at least one effect",
            ));
        }
        return Ok((parse_scene_handler_effects(&body, line)?, block_end + 1));
    }

    Ok((parse_scene_effect(value, line)?, start + 1))
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedSceneEffect {
    pub(crate) surface: SurfaceSceneEffect,
    pub(crate) semantic_tokens: Vec<semantic::SemanticToken>,
}

fn parse_scene_effect(value: &str, line: &str) -> Result<SceneEffect, DiagnosticReport> {
    let parsed = parse_scene_effect_with_semantic_tokens(value, line)?;
    debug_assert!(
        parsed
            .semantic_tokens
            .iter()
            .all(|token| token.start < token.end)
    );
    Ok(parsed.surface.effect)
}

fn parse_scene_effect_with_semantic_tokens(
    value: &str,
    line: &str,
) -> Result<ParsedSceneEffect, DiagnosticReport> {
    let surface = parse_surface_scene_effect(value, line)?;
    let semantic_tokens = project_surface_semantic_tokens(&surface.document.semantic_tokens);
    Ok(ParsedSceneEffect {
        surface,
        semantic_tokens,
    })
}

fn parse_surface_scene_effect(
    value: &str,
    line: &str,
) -> Result<SurfaceSceneEffect, DiagnosticReport> {
    let tokens = source_line_tokens(strip_line_comment(value), 0);
    let document = scene_effect_surface_document(&tokens);
    let effect = parse_scene_effect_value(value, line)?;
    Ok(SurfaceSceneEffect { effect, document })
}

fn parse_scene_effect_value(value: &str, line: &str) -> Result<SceneEffect, DiagnosticReport> {
    if value.contains(" then ") {
        return Err(parse_error(
            line,
            "`then` effect sequences are not supported; use an effect block with one effect per line",
        ));
    }

    if let Some(parts) = split_scene_effect_sequence(value) {
        let mut effects = Vec::new();
        for part in parts {
            effects.push(parse_scene_effect_value(part, line)?);
        }
        return match effects.len() {
            0 => unreachable!("scene effect sequence splitter returned no effects"),
            1 => Ok(effects.remove(0)),
            _ => Ok(SceneEffect::Sequence(effects)),
        };
    }

    if let Some(text) = value.strip_prefix("message ") {
        return Ok(SceneEffect::Message {
            text: parse_scene_expr(text.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("current_level = ") {
        return Ok(SceneEffect::SetCurrentLevel {
            level: parse_scene_level_expr(rest.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("level.cleared = ") {
        return Ok(SceneEffect::SetLevelCleared {
            level: None,
            cleared: parse_scene_effect_bool(rest.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("level(") {
        if let Some((level, cleared)) = rest.split_once(").cleared = ") {
            return Ok(SceneEffect::SetLevelCleared {
                level: Some(parse_scene_level_expr(level.trim(), line)?),
                cleared: parse_scene_effect_bool(cleared.trim(), line)?,
            });
        }
    }
    if let Some((name, rhs)) = parse_scene_variable_assignment(value) {
        return Ok(SceneEffect::SetVariable {
            name: name.to_string(),
            value: parse_scene_expr(rhs, line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("goto ") {
        let (scene, params) = parse_scene_target_params(rest, line)?;
        return Ok(SceneEffect::Goto { scene, params });
    }
    if let Some(rest) = value.strip_prefix("start ") {
        if rest.starts_with("levels ") || rest.contains(" in ") {
            return Err(legacy_start_levels_error(line));
        }
        let (scene, params) = parse_scene_target_params(rest, line)?;
        return Ok(SceneEffect::Sequence(vec![
            SceneEffect::Reset {
                scene: scene.clone(),
            },
            SceneEffect::Goto { scene, params },
        ]));
    }

    let tokens = split_header_tokens(value);
    match tokens.as_slice() {
        ["input", input] => Ok(SceneEffect::Input(
            parse_input_name(input, line)?.to_string(),
        )),
        ["component_effect", effect] => Ok(SceneEffect::ComponentEffect(
            parse_scene_signal_name(effect, line, "component effect")?.to_string(),
        )),
        ["apply", call, "to", target] => {
            validate_target_path(target, line, "apply target")?;
            let (rule, args) = parse_rule_call_expr(call, line)?;
            Ok(SceneEffect::Apply {
                rule,
                args,
                target: Some((*target).to_string()),
            })
        }
        ["apply", call] => {
            let (rule, args) = parse_rule_call_expr(call, line)?;
            Ok(SceneEffect::Apply {
                rule,
                args,
                target: None,
            })
        }
        ["copy", source, "to", target] => {
            validate_target_path(source, line, "copy source")?;
            validate_target_path(target, line, "copy target")?;
            Ok(SceneEffect::Copy {
                source: (*source).to_string(),
                target: (*target).to_string(),
            })
        }
        ["load", target, "from", source] => {
            validate_target_path(target, line, "load target")?;
            Ok(SceneEffect::LoadPuzzle {
                target: (*target).to_string(),
                source: (*source).to_string(),
            })
        }
        ["wait"] => Ok(SceneEffect::Wait { milliseconds: None }),
        ["wait", duration] => Ok(SceneEffect::Wait {
            milliseconds: Some(parse_wait_duration_ms(duration, line)?),
        }),
        ["clear_undo_history"] | ["clear_history"] => Ok(SceneEffect::ClearUndoHistory),
        ["clear_game_progress"] => Ok(SceneEffect::ClearGameProgress),
        ["clear", "current_level"] => Ok(SceneEffect::ClearCurrentLevel),
        ["reset", "persistent_vars"] => Ok(SceneEffect::ResetPersistentVars),
        ["sfx", name] => {
            validate_qualified_identifier(name, line, "sfx sounds name")?;
            Ok(SceneEffect::PlaySfx {
                name: (*name).to_string(),
            })
        }
        ["play_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::PlayMusic {
                name: (*name).to_string(),
            })
        }
        ["pause_music"] => Ok(SceneEffect::PauseMusic { name: None }),
        ["pause_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::PauseMusic {
                name: Some((*name).to_string()),
            })
        }
        ["resume_music"] => Ok(SceneEffect::ResumeMusic { name: None }),
        ["resume_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::ResumeMusic {
                name: Some((*name).to_string()),
            })
        }
        ["stop_music"] => Ok(SceneEffect::StopMusic { name: None }),
        ["stop_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::StopMusic {
                name: Some((*name).to_string()),
            })
        }
        ["reset", target] if target.contains('.') => {
            validate_target_path(target, line, "reset target")?;
            Ok(SceneEffect::ResetPuzzle {
                target: (*target).to_string(),
            })
        }
        ["start", "levels", ..] | ["start", _, "in", _] | ["continue", "levels", ..] => {
            Err(legacy_start_levels_error(line))
        }
        [target_command, level] => {
            if let Some((target, command)) = parse_puzzle_command(target_command, line)? {
                if command == "goto" {
                    return Ok(SceneEffect::GotoLevel {
                        target,
                        level: parse_scene_level_expr(level, line)?,
                    });
                }
            }
            Err(parse_error(
                line,
                "effect must be: input <name> | component_effect <name> | goto <scene> | goto <scene>(<level>) | start <scene> | start <scene>(<level>) | clear_undo_history | clear_game_progress | message <text> | wait <duration> | sfx <name> | play_music <name> | pause_music [name] | resume_music [name] | stop_music [name] | <scene>.goto <level> | copy <puzzle> to <puzzle>",
            ))
        }
        ["input"] => Err(parse_error(line, "input effect must name an input")),
        ["component_effect"] => Err(parse_error(
            line,
            "component_effect must name a component effect",
        )),
        [command_text] => {
            if let Some((target, command)) = parse_puzzle_command(command_text, line)? {
                if command == "next_level" {
                    return Ok(SceneEffect::PuzzleNextLevel { target });
                }
                if command == "previous_level" {
                    return Ok(SceneEffect::PuzzlePreviousLevel { target });
                }
                if command == "restart" {
                    return Ok(SceneEffect::ResetPuzzle { target });
                }
            }
            if is_identifier(command_text) {
                return Ok(SceneEffect::RoutineCall((*command_text).to_string()));
            }
            Err(parse_error(
                line,
                "bare scene effect aliases were removed; use `input <name>`, `component_effect <name>`, a scene routine, or an explicit scene effect",
            ))
        }
        _ => Err(parse_error(
            line,
            "effect must be: input <name> | component_effect <name> | goto <scene> | goto <scene>(<level>) | start <scene> | start <scene>(<level>) | clear_undo_history | clear_game_progress | message <text> | wait <duration> | sfx <name> | play_music <name> | pause_music [name] | resume_music [name] | stop_music [name] | copy <puzzle> to <puzzle>",
        )),
    }
}

fn parse_scene_variable_assignment(value: &str) -> Option<(&str, &str)> {
    let (name, value) = parse_assignment_row(value)?;
    if value.is_empty() || !is_identifier(name) || reserved_scene_assignment_target(name) {
        return None;
    }
    Some((name, value))
}

fn reserved_scene_assignment_target(name: &str) -> bool {
    matches!(name, "current_level" | "level")
}

fn split_scene_effect_sequence(value: &str) -> Option<Vec<&str>> {
    let stripped = strip_line_comment(value);
    let tokens = source_line_tokens(stripped, 0);
    let parts = split_scene_effect_token_sequence(&tokens)?;
    Some(
        parts
            .into_iter()
            .map(|part| stripped[part.first().unwrap().start..part.last().unwrap().end].trim())
            .collect(),
    )
}

fn split_scene_effect_token_sequence(tokens: &[SourceToken]) -> Option<Vec<&[SourceToken]>> {
    let mut parts = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let length = scene_effect_token_length(&tokens[index..])?;
        parts.push(&tokens[index..index + length]);
        index += length;
    }
    (parts.len() > 1).then_some(parts)
}

fn scene_effect_token_length(tokens: &[SourceToken]) -> Option<usize> {
    let first = tokens.first()?.text.as_str();
    match first {
        "input" | "component_effect" | "sfx" | "play_music" => (tokens.len() >= 2).then_some(2),
        "pause_music" | "resume_music" | "stop_music" => {
            if tokens
                .get(1)
                .is_some_and(|token| !is_scene_effect_command_start(&token.text))
            {
                Some(2)
            } else {
                Some(1)
            }
        }
        "wait" => {
            if tokens
                .get(1)
                .is_some_and(|token| !is_scene_effect_command_start(&token.text))
            {
                Some(2)
            } else {
                Some(1)
            }
        }
        "clear_undo_history" | "clear_history" | "clear_game_progress" => Some(1),
        "clear" => (tokens.get(1)?.text == "current_level").then_some(2),
        "reset"
            if tokens
                .get(1)
                .is_some_and(|token| token.text == "persistent_vars") =>
        {
            Some(2)
        }
        "reset" if tokens.get(1).is_some_and(|token| token.text.contains('.')) => Some(2),
        "goto" | "start" => {
            if tokens.get(2).is_some_and(|token| token.text == "with") {
                None
            } else {
                (tokens.len() >= 2).then_some(2)
            }
        }
        _ if first.contains('.') => {
            let command = first.rsplit_once('.').map(|(_, command)| command)?;
            match command {
                "goto" | "goto_level" => (tokens.len() >= 2).then_some(2),
                "next_level" | "previous_level" | "restart" => Some(1),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_scene_effect_command_start(token: &str) -> bool {
    scene_effect_command_syntax(token).is_some()
        || token.rsplit_once('.').is_some_and(|(_, command)| {
            matches!(
                command,
                "goto" | "goto_level" | "next_level" | "previous_level" | "restart"
            )
        })
}

fn parse_scene_effect_bool(value: &str, line: &str) -> Result<bool, DiagnosticReport> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(parse_error(
            line,
            "boolean progress value must be true or false",
        )),
    }
}

fn legacy_start_levels_error(line: &str) -> DiagnosticReport {
    parse_error(
        line,
        "`start levels ... in <scene>` and `continue levels ... in <scene>` are no longer supported; use `goto <puzzle>` for the default playable scene, `goto <puzzle>(<level>)` for a specific level, or `goto <scene>(<level>)` for an explicit level scene",
    )
}

const DEFAULT_WAIT_MS: u64 = 200;
const DEFAULT_AGAIN_MS: u64 = 120;

fn resolve_default_wait_in_scenes(scenes: &mut [SceneDef], default_wait_ms: u64) {
    for scene in scenes {
        for component in &mut scene.components {
            resolve_default_wait_in_component(component, default_wait_ms);
        }
        for binding in &mut scene.key_bindings {
            resolve_default_wait_in_effect(&mut binding.effect, default_wait_ms);
        }
        for routine in &mut scene.routines {
            resolve_default_wait_in_effect(&mut routine.effect, default_wait_ms);
        }
        for transition in &mut scene.transitions {
            resolve_default_wait_in_effect(&mut transition.effect, default_wait_ms);
        }
    }
}

fn resolve_default_wait_in_component(component: &mut SceneComponent, default_wait_ms: u64) {
    match component {
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            resolve_default_wait_in_effect(&mut button.effect, default_wait_ms);
        }
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => {
            for child in &mut container.children {
                resolve_default_wait_in_component(child, default_wait_ms);
            }
        }
        SceneComponent::Conditional(conditional) => {
            for child in &mut conditional.children {
                resolve_default_wait_in_component(child, default_wait_ms);
            }
        }
        SceneComponent::For(for_view) => {
            for child in &mut for_view.children {
                resolve_default_wait_in_component(child, default_wait_ms);
            }
        }
        SceneComponent::LevelMenu(menu) => {
            for button in &mut menu.buttons {
                resolve_default_wait_in_effect(&mut button.effect, default_wait_ms);
            }
        }
        SceneComponent::Frame(_)
        | SceneComponent::Title(_)
        | SceneComponent::Subtitle(_)
        | SceneComponent::Text(_) => {}
    }
}

fn resolve_default_wait_in_effect(effect: &mut SceneEffect, default_wait_ms: u64) {
    match effect {
        SceneEffect::Wait { milliseconds } => {
            if milliseconds.is_none() {
                *milliseconds = Some(default_wait_ms);
            }
        }
        SceneEffect::Conditional { effect, .. } => {
            resolve_default_wait_in_effect(effect, default_wait_ms);
        }
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                resolve_default_wait_in_effect(effect, default_wait_ms);
            }
        }
        SceneEffect::Input(_)
        | SceneEffect::ComponentEffect(_)
        | SceneEffect::RoutineCall(_)
        | SceneEffect::Message { .. }
        | SceneEffect::PlaySfx { .. }
        | SceneEffect::PlayMusic { .. }
        | SceneEffect::PauseMusic { .. }
        | SceneEffect::ResumeMusic { .. }
        | SceneEffect::StopMusic { .. }
        | SceneEffect::Goto { .. }
        | SceneEffect::Enter { .. }
        | SceneEffect::Back
        | SceneEffect::Create { .. }
        | SceneEffect::Reset { .. }
        | SceneEffect::Delete { .. }
        | SceneEffect::Show { .. }
        | SceneEffect::Hide { .. }
        | SceneEffect::Toggle { .. }
        | SceneEffect::Focus { .. }
        | SceneEffect::PuzzleNextLevel { .. }
        | SceneEffect::PuzzlePreviousLevel { .. }
        | SceneEffect::GotoLevel { .. }
        | SceneEffect::ResetPuzzle { .. }
        | SceneEffect::LoadPuzzle { .. }
        | SceneEffect::Apply { .. }
        | SceneEffect::Copy { .. }
        | SceneEffect::SetVariable { .. }
        | SceneEffect::ClearUndoHistory
        | SceneEffect::ClearGameProgress
        | SceneEffect::SetCurrentLevel { .. }
        | SceneEffect::ClearCurrentLevel
        | SceneEffect::SetLevelCleared { .. }
        | SceneEffect::ResetPersistentVars => {}
    }
}

fn parse_wait_duration_ms(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    if let Some(milliseconds) = value.strip_suffix("ms") {
        return parse_whole_milliseconds(milliseconds, line);
    }
    if let Some(seconds) = value.strip_suffix('s') {
        return parse_seconds_duration_ms(seconds, line);
    }
    Err(parse_error(
        line,
        "wait duration must use seconds or milliseconds, for example `wait 0.1s` or `wait 100ms`",
    ))
}

fn parse_whole_milliseconds(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait milliseconds must be a whole number",
        ));
    }
    value
        .parse::<u64>()
        .map_err(|_| parse_error(line, "wait duration is too large"))
}

fn parse_seconds_duration_ms(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    let value = value.trim();
    let has_decimal = value.contains('.');
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty() || !whole.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait seconds must be a non-negative number",
        ));
    }
    if (has_decimal && fraction.is_empty()) || !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait seconds must be a non-negative number",
        ));
    }
    if fraction.len() > 3 {
        return Err(parse_error(
            line,
            "wait seconds can use at most millisecond precision",
        ));
    }
    let whole_ms = whole
        .parse::<u64>()
        .map_err(|_| parse_error(line, "wait duration is too large"))?
        .checked_mul(1000)
        .ok_or_else(|| parse_error(line, "wait duration is too large"))?;
    let fraction_ms = if fraction.is_empty() {
        0
    } else {
        let padded = format!("{fraction:0<3}");
        padded
            .parse::<u64>()
            .map_err(|_| parse_error(line, "wait duration is too large"))?
    };
    whole_ms
        .checked_add(fraction_ms)
        .ok_or_else(|| parse_error(line, "wait duration is too large"))
}

fn parse_rule_call_expr(
    value: &str,
    line: &str,
) -> Result<(String, Vec<SceneExpr>), DiagnosticReport> {
    let value = value.trim();
    let Some(call) = parse_complete_call_surface(
        value,
        line,
        "rule call args must end with )",
        "rule call expression must not have trailing text",
    )?
    else {
        validate_qualified_identifier(value, line, "rule name")?;
        return Ok((value.to_string(), Vec::new()));
    };
    validate_qualified_identifier(call.name, line, "rule name")?;
    let args = parse_scene_call_arg_surfaces(&call.args, line)?;
    Ok((call.name.to_string(), args))
}

fn parse_scene_call_params(
    value: &str,
    line: &str,
) -> Result<Option<(String, Vec<SceneEffectParam>)>, DiagnosticReport> {
    let Some((call, suffix)) =
        parse_optional_call_surface_with_suffix(value, line, "scene call must close with `)`")?
    else {
        return Ok(None);
    };
    if !suffix.is_empty() {
        return Err(parse_error(line, "scene call must close with `)`"));
    }
    validate_qualified_identifier(call.name, line, "scene name")?;
    if call.args.is_empty() {
        return Ok(Some((call.name.to_string(), Vec::new())));
    }

    let params = if call.args.len() == 1 && parse_assignment_row(call.args[0]).is_none() {
        vec![SceneEffectParam::Level(parse_scene_level_expr(
            call.args[0],
            line,
        )?)]
    } else {
        parse_scene_named_params(&call.args, line)?
    };
    Ok(Some((call.name.to_string(), params)))
}

fn parse_scene_target_params(
    value: &str,
    line: &str,
) -> Result<(String, Vec<SceneEffectParam>), DiagnosticReport> {
    let value = value.trim();
    if let Some((scene, params)) = value.split_once(" with ") {
        let scene = scene.trim();
        validate_qualified_identifier(scene, line, "scene name")?;
        let parts = parse_call_argument_surfaces(params);
        return Ok((scene.to_string(), parse_scene_named_params(&parts, line)?));
    }
    if let Some((scene, params)) = parse_scene_call_params(value, line)? {
        return Ok((scene, params));
    }
    validate_qualified_identifier(value, line, "scene name")?;
    Ok((value.to_string(), Vec::new()))
}

fn parse_scene_named_params(
    parts: &[&str],
    line: &str,
) -> Result<Vec<SceneEffectParam>, DiagnosticReport> {
    let mut params = Vec::new();
    for part in parts {
        let (name, value) =
            require_assignment_row(part, "scene params must be named `<name> = <expr>`")?;
        validate_identifier(name, line, "scene param name")?;
        params.push(SceneEffectParam::Named {
            name: name.to_string(),
            value: parse_scene_expr(value, line)?,
        });
    }
    Ok(params)
}

pub fn parse_scene_expression(value: &str) -> Result<SceneExpr, DiagnosticReport> {
    parse_scene_expr(value, value)
}

pub fn parse_scene_expression_args(value: &str) -> Result<Vec<SceneExpr>, DiagnosticReport> {
    parse_scene_call_args(value.trim(), value)
}

pub fn parse_scene_effect_params(value: &str) -> Result<Vec<SceneEffectParam>, DiagnosticReport> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(Vec::new());
    }
    parse_call_argument_surfaces(value)
        .into_iter()
        .map(|param| {
            if let Some((name, value)) = parse_assignment_row(param) {
                validate_identifier(name, param, "scene effect parameter name")?;
                return Ok(SceneEffectParam::Named {
                    name: name.to_string(),
                    value: parse_scene_expr(value, param)?,
                });
            }
            Ok(SceneEffectParam::Level(parse_scene_level_expr(
                param, param,
            )?))
        })
        .collect()
}

fn parse_scene_level_expr(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    if let Some(expr) = parse_level_selector_expr(value, line)? {
        return Ok(expr);
    }
    match parse_scene_expr(value, line) {
        Ok(expr) => Ok(expr),
        Err(error) => Err(error),
    }
}

fn parse_scene_expr(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    let value = value.trim();
    if value.is_empty() {
        return Err(parse_error(line, "expression must not be empty"));
    }
    if let Some(expr) = parse_scene_if_expr(value, line)? {
        return Ok(expr);
    }
    if let Some((left, right)) = split_top_level_keyword_once(value, "and") {
        return Ok(SceneExpr::Binary {
            op: SceneBinaryOp::And,
            left: Box::new(parse_scene_expr(left.trim(), line)?),
            right: Box::new(parse_scene_expr(right.trim(), line)?),
        });
    }
    if let Some((left, right)) = split_top_level_operator_once(value, "==") {
        return Ok(SceneExpr::Binary {
            op: SceneBinaryOp::Eq,
            left: Box::new(parse_scene_expr(left.trim(), line)?),
            right: Box::new(parse_scene_expr(right.trim(), line)?),
        });
    }
    if let Some((left, right)) = split_top_level_operator_once(value, "!=") {
        return Ok(SceneExpr::Binary {
            op: SceneBinaryOp::NotEq,
            left: Box::new(parse_scene_expr(left.trim(), line)?),
            right: Box::new(parse_scene_expr(right.trim(), line)?),
        });
    }
    if value == "true" {
        return Ok(SceneExpr::Bool(true));
    }
    if value == "false" {
        return Ok(SceneExpr::Bool(false));
    }
    if let Ok(number) = value.parse::<i64>() {
        return Ok(SceneExpr::Int(number));
    }
    if let Some(text) = parse_quoted_text(value) {
        return Ok(SceneExpr::Text(text));
    }
    if let Some(expr) = parse_level_selector_expr(value, line)? {
        return Ok(expr);
    }
    if value.contains('(') {
        let (name, args) = parse_rule_call_expr(value, line)?;
        return Ok(SceneExpr::Call { name, args });
    }
    if value.starts_with("join ") {
        return Err(parse_error(
            line,
            "`join` scene expression is not supported",
        ));
    }
    if let Some(path) = parse_view_path(value) {
        return Ok(SceneExpr::Path(path));
    }
    Err(parse_error(
        line,
        "expression must be true, false, integer, quoted text, path, call, or if expression",
    ))
}

fn parse_scene_if_expr(value: &str, line: &str) -> Result<Option<SceneExpr>, DiagnosticReport> {
    let Some(rest) = value.strip_prefix("if ") else {
        return Ok(None);
    };
    let rest = rest.trim_start();
    let Some(open) = find_top_level_char(rest, '{') else {
        return Err(parse_error(
            line,
            "scene if expression must be: if <bool> { <value> } else { <value> }",
        ));
    };
    let condition = rest[..open].trim();
    if condition.is_empty() {
        return Err(parse_error(
            line,
            "scene if expression requires a condition",
        ));
    }
    let close = matching_delimiter(rest, open, '{', '}')
        .ok_or_else(|| parse_error(line, "scene if expression branch must close with `}`"))?;
    let then_branch = rest[open + 1..close].trim();
    let after_then = rest[close + 1..].trim_start();
    let Some(after_else) = after_then.strip_prefix("else") else {
        return Err(parse_error(
            line,
            "scene if expression requires an else branch",
        ));
    };
    let after_else = after_else.trim_start();
    if !after_else.starts_with('{') {
        return Err(parse_error(
            line,
            "scene if expression else branch must start with `{`",
        ));
    }
    let else_close = matching_delimiter(after_else, 0, '{', '}')
        .ok_or_else(|| parse_error(line, "scene if expression else branch must close with `}`"))?;
    if !after_else[else_close + 1..].trim().is_empty() {
        return Err(parse_error(
            line,
            "scene if expression must not have trailing text after else branch",
        ));
    }
    let else_branch = after_else[1..else_close].trim();
    Ok(Some(SceneExpr::If {
        condition: Box::new(parse_scene_expr(condition, line)?),
        then_branch: Box::new(parse_scene_expr(then_branch, line)?),
        else_branch: Box::new(parse_scene_expr(else_branch, line)?),
    }))
}

fn parse_level_selector_expr(
    value: &str,
    line: &str,
) -> Result<Option<SceneExpr>, DiagnosticReport> {
    let value = value.trim();
    if let Some(expr) = parse_level_call_selector_expr(value, line)? {
        return Ok(Some(expr));
    }
    parse_pack_level_selector_expr(value, line)
}

fn parse_level_call_selector_expr(
    value: &str,
    line: &str,
) -> Result<Option<SceneExpr>, DiagnosticReport> {
    let Some((call, suffix)) =
        parse_optional_call_surface_with_suffix(value, line, "level selector must close with `)`")?
    else {
        return Ok(None);
    };
    if call.name != "level" {
        return Ok(None);
    }
    let args = parse_scene_call_arg_surfaces(&call.args, line)?;
    let name = if suffix.is_empty() {
        "level".to_string()
    } else if let Some(field) = suffix.strip_prefix('.') {
        validate_identifier(field, line, "level property")?;
        format!("level.{field}")
    } else {
        return Err(parse_error(
            line,
            "level selector suffix must be empty or `.property`",
        ));
    };
    Ok(Some(SceneExpr::Call { name, args }))
}

fn parse_pack_level_selector_expr(
    value: &str,
    line: &str,
) -> Result<Option<SceneExpr>, DiagnosticReport> {
    let Some(open) = value.find('[') else {
        return Ok(None);
    };
    let Some(close) = value[open + 1..].find(']').map(|offset| open + 1 + offset) else {
        return Err(parse_error(line, "level pack selector must close with `]`"));
    };
    let pack = value[..open].trim();
    if pack.is_empty() || !is_qualified_identifier(pack) {
        return Ok(None);
    }
    let key = value[open + 1..close].trim();
    let suffix = value[close + 1..].trim();
    let mut args = vec![SceneExpr::Path(vec![pack.to_string()])];
    let base = if let Some(id) = parse_quoted_text(key) {
        args.push(SceneExpr::Text(id));
        "level_in"
    } else if let Ok(index) = key.parse::<i64>() {
        args.push(SceneExpr::Int(index));
        "level_at"
    } else {
        return Err(parse_error(
            line,
            "level pack selector key must be a quoted id or integer index",
        ));
    };
    let name = if suffix.is_empty() {
        base.to_string()
    } else if let Some(field) = suffix.strip_prefix('.') {
        validate_identifier(field, line, "level property")?;
        format!("{base}.{field}")
    } else {
        return Err(parse_error(
            line,
            "level pack selector suffix must be empty or `.property`",
        ));
    };
    Ok(Some(SceneExpr::Call { name, args }))
}

fn parse_scene_call_args(value: &str, line: &str) -> Result<Vec<SceneExpr>, DiagnosticReport> {
    let args = parse_call_argument_surfaces(value);
    parse_scene_call_arg_surfaces(&args, line)
}

fn parse_scene_call_arg_surfaces(
    args: &[&str],
    line: &str,
) -> Result<Vec<SceneExpr>, DiagnosticReport> {
    args.iter()
        .map(|arg| parse_scene_expr(arg.trim(), line))
        .collect()
}

fn parse_input_name<'a>(value: &'a str, line: &str) -> Result<&'a str, DiagnosticReport> {
    validate_identifier(value, line, "input name")?;
    Ok(value)
}

fn parse_scene_signal_name<'a>(
    value: &'a str,
    line: &str,
    label: &str,
) -> Result<&'a str, DiagnosticReport> {
    validate_qualified_identifier(value, line, label)?;
    Ok(value)
}

fn parse_puzzle_command<'a>(
    value: &'a str,
    line: &str,
) -> Result<Option<(String, &'a str)>, DiagnosticReport> {
    let Some((target, command)) = value.split_once('.') else {
        return Ok(None);
    };
    validate_qualified_identifier(target, line, "puzzle target")?;
    validate_identifier(command, line, "puzzle command")?;
    Ok(Some((target.to_string(), command)))
}

fn validate_target_path(value: &str, line: &str, label: &str) -> Result<(), DiagnosticReport> {
    if parse_view_path(value).is_some() {
        Ok(())
    } else {
        Err(parse_error(
            line,
            &format!("{label} must be an identifier path"),
        ))
    }
}

#[derive(Clone, Copy)]
enum NameClass {
    Identifier,
    Qualified,
}

fn validate_name(
    value: &str,
    class: NameClass,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    let valid = match class {
        NameClass::Identifier => is_identifier(value),
        NameClass::Qualified => is_qualified_identifier(value),
    };
    if valid {
        Ok(())
    } else {
        let expected = match class {
            NameClass::Identifier => "an identifier",
            NameClass::Qualified => "a qualified identifier",
        };
        Err(parse_error(line, &format!("{label} must be {expected}")))
    }
}

fn validate_identifier(value: &str, line: &str, label: &str) -> Result<(), DiagnosticReport> {
    validate_name(value, NameClass::Identifier, line, label)
}

fn validate_qualified_identifier(
    value: &str,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    validate_name(value, NameClass::Qualified, line, label)
}
