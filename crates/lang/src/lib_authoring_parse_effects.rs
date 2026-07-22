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
        | "next_level"
        | "previous_level"
        | "restart"
        | "wait" => Some(SceneEffectCommandSyntax::Plain),
        _ => None,
    }
}

fn project_surface_semantic_tokens(
    tokens: &[SurfaceSemanticToken],
) -> Vec<semantic::SemanticToken> {
    tokens
        .iter()
        .map(|token| semantic::SemanticToken {
            start: token.span.start,
            end: token.span.end,
            kind: project_surface_semantic_kind(token.kind),
        })
        .collect()
}

fn project_surface_semantic_kind(kind: SurfaceSemanticKind) -> semantic::SemanticKind {
    match kind {
        SurfaceSemanticKind::Keyword => semantic::SemanticKind::Keyword,
        SurfaceSemanticKind::Literal => semantic::SemanticKind::Literal,
        SurfaceSemanticKind::Binding => semantic::SemanticKind::Binding,
        SurfaceSemanticKind::Effect => semantic::SemanticKind::Effect,
        SurfaceSemanticKind::Emission => semantic::SemanticKind::Emission,
        SurfaceSemanticKind::Object => semantic::SemanticKind::Object,
        SurfaceSemanticKind::Input => semantic::SemanticKind::Input,
        SurfaceSemanticKind::State => semantic::SemanticKind::State,
        SurfaceSemanticKind::Group => semantic::SemanticKind::Group,
        SurfaceSemanticKind::Mark => semantic::SemanticKind::Mark,
        SurfaceSemanticKind::Variant => semantic::SemanticKind::Variant,
        SurfaceSemanticKind::Condition => semantic::SemanticKind::Condition,
        SurfaceSemanticKind::Scene => semantic::SemanticKind::Scene,
        SurfaceSemanticKind::Theme => semantic::SemanticKind::Theme,
        SurfaceSemanticKind::Asset => semantic::SemanticKind::Asset,
        SurfaceSemanticKind::Setting => semantic::SemanticKind::Setting,
        SurfaceSemanticKind::Color => semantic::SemanticKind::Color,
        SurfaceSemanticKind::Number => semantic::SemanticKind::Number,
        SurfaceSemanticKind::String => semantic::SemanticKind::String,
    }
}

fn scene_effect_parser_recognition(tokens: &[SourceToken]) -> crate::surface::ParserRecognition {
    if let Some(parts) = split_scene_effect_token_sequence(tokens) {
        let mut recognition = crate::surface::ParserRecognition::default();
        for part in parts {
            recognition.merge(scene_effect_parser_recognition(part));
        }
        return recognition.finish();
    }

    let mut recognition = crate::surface::ParserRecognition::default();
    let Some(first) = tokens.first() else {
        return recognition;
    };
    let effect_span = source_tokens_span(tokens);

    if first.text.starts_with("cursor.") {
        add_cursor_scene_effect_token(&mut recognition, first);
        return parser_recognition_with_node(
            recognition,
            SurfaceNodeKind::SceneEffect,
            effect_span,
        );
    }

    if first.text.contains('.') {
        let mut parts = first.text.split('.');
        if let Some(target) = parts.next() {
            add_scene_effect_token_part(
                &mut recognition,
                first,
                target,
                SurfaceSemanticKind::Scene,
            );
        }
        if let Some(effect) = parts.next() {
            add_scene_effect_token_part(
                &mut recognition,
                first,
                effect,
                SurfaceSemanticKind::Effect,
            );
        }
        for argument in tokens.iter().skip(1) {
            add_scene_effect_token_range(&mut recognition, argument, SurfaceSemanticKind::State);
        }
        return parser_recognition_with_node(
            recognition,
            SurfaceNodeKind::SceneEffect,
            effect_span,
        );
    }

    if first.text == "start" && add_level_flow_scene_effect_tokens(tokens, &mut recognition) {
        return parser_recognition_with_node(
            recognition,
            SurfaceNodeKind::SceneEffect,
            effect_span,
        );
    }

    let command_syntax = scene_effect_command_syntax(&first.text);
    match command_syntax {
        Some(SceneEffectCommandSyntax::InputTarget) => {
            add_scene_effect_token_range(&mut recognition, first, SurfaceSemanticKind::Effect);
            for input in tokens
                .iter()
                .skip(1)
                .filter(|token| !matches!(token.text.as_str(), "=" | ":"))
            {
                add_scene_command_token(&mut recognition, input);
            }
        }
        Some(SceneEffectCommandSyntax::ComponentEffectTarget) => {
            add_scene_effect_token_range(&mut recognition, first, SurfaceSemanticKind::Effect);
            if let Some(effect) = tokens.get(1) {
                add_scene_command_token(&mut recognition, effect);
            }
        }
        Some(SceneEffectCommandSyntax::SceneTarget) => {
            add_scene_effect_token_range(&mut recognition, first, SurfaceSemanticKind::Effect);
            if let Some(scene) = tokens.get(1) {
                add_scene_effect_token_range(&mut recognition, scene, SurfaceSemanticKind::Scene);
            }
        }
        Some(SceneEffectCommandSyntax::AssetTarget) => {
            let kind = scene_effect_command_kind(&first.text);
            add_scene_effect_token_range(&mut recognition, first, kind);
            if let Some(asset) = tokens.get(1) {
                add_scene_effect_token_range(&mut recognition, asset, SurfaceSemanticKind::Asset);
            }
        }
        Some(SceneEffectCommandSyntax::OptionalAssetTarget) => {
            add_scene_effect_token_range(&mut recognition, first, SurfaceSemanticKind::Effect);
            if let Some(asset) = tokens.get(1) {
                add_scene_effect_token_range(&mut recognition, asset, SurfaceSemanticKind::Asset);
            }
        }
        Some(SceneEffectCommandSyntax::Plain) => {
            let kind = scene_effect_command_kind(&first.text);
            add_scene_effect_token_range(&mut recognition, first, kind);
        }
        None if tokens.len() == 1 && puzzle_authoring::is_identifier(&first.text) => {
            add_scene_effect_token_range(&mut recognition, first, SurfaceSemanticKind::Effect);
        }
        None => {}
    }

    if command_syntax.is_some() {
        for token in tokens.iter().skip(1).filter(|token| {
            matches!(command_syntax, Some(SceneEffectCommandSyntax::Plain))
                || token
                    .text
                    .chars()
                    .any(|ch| matches!(ch, '(' | ')' | '.' | '[' | ']'))
        }) {
            recognition.mark(
                SourceSpan {
                    start: token.start,
                    end: token.end,
                },
                if token.text.starts_with('"') || token.text.starts_with('\'') {
                    SurfaceSemanticKind::String
                } else {
                    SurfaceSemanticKind::State
                },
            );
        }
    }

    parser_recognition_with_node(recognition, SurfaceNodeKind::SceneEffect, effect_span)
}

fn source_tokens_span(tokens: &[SourceToken]) -> Option<SourceSpan> {
    let start = tokens.first()?.start;
    let end = tokens.last()?.end;
    (start < end).then_some(SourceSpan { start, end })
}

fn parser_recognition_with_node(
    mut recognition: crate::surface::ParserRecognition,
    kind: SurfaceNodeKind,
    span: Option<SourceSpan>,
) -> crate::surface::ParserRecognition {
    if !recognition.token_dispositions.is_empty()
        && let Some(span) = span
    {
        recognition.node(kind, span);
    }
    recognition.finish()
}

fn parser_recognition_surface_document(
    recognition: &crate::surface::ParserRecognition,
) -> SurfaceDocument {
    let mut sink = SurfaceSink::default();
    sink.project_parser_recognition(recognition);
    sink.into_document()
}

trait SemanticDispositionSink {
    fn mark(&mut self, span: SourceSpan, kind: SurfaceSemanticKind);
}

impl SemanticDispositionSink for crate::surface::ParserRecognition {
    fn mark(&mut self, span: SourceSpan, kind: SurfaceSemanticKind) {
        crate::surface::ParserRecognition::mark(self, span, kind);
    }
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

fn add_level_flow_scene_effect_tokens(
    tokens: &[SourceToken],
    sink: &mut impl SemanticDispositionSink,
) -> bool {
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

fn add_scene_command_token(sink: &mut impl SemanticDispositionSink, token: &SourceToken) {
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

fn add_cursor_scene_effect_token(sink: &mut impl SemanticDispositionSink, token: &SourceToken) {
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
    sink: &mut impl SemanticDispositionSink,
    token: &SourceToken,
    kind: SurfaceSemanticKind,
) {
    let Some((start, end)) = scene_effect_identifier_bounds(token) else {
        return;
    };
    sink.mark(SourceSpan { start, end }, kind);
}

fn add_scene_effect_token_part(
    sink: &mut impl SemanticDispositionSink,
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
    sink: &mut impl SemanticDispositionSink,
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
            | EffectAst::EmitVisual { .. }
            | EffectAst::PresentComponent { .. }
            | EffectAst::Scene(_) => RewriteEffectCommandSyntax::Emission,
            EffectAst::Cancel
            | EffectAst::Win
            | EffectAst::Restart
            | EffectAst::NextLevel
            | EffectAst::Again
            | EffectAst::Checkpoint
            | EffectAst::ClearCheckpoint
            | EffectAst::UpdateVariable { .. } => RewriteEffectCommandSyntax::Effect,
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

pub(crate) fn is_level_event_sugar(trimmed: &str, tokens: &[&str]) -> bool {
    let Some(command) = tokens.first().copied() else {
        return false;
    };
    if rewrite_effect_command_syntax(command) != Some(RewriteEffectCommandSyntax::Emission) {
        return false;
    }
    match tokens {
        ["message", ..] => trimmed.strip_prefix("message ").is_some(),
        ["sfx", _] | ["wait"] | ["wait", _] => true,
        _ => false,
    }
}

fn rewrite_effect_parser_recognition(tokens: &[SourceToken]) -> crate::surface::ParserRecognition {
    if tokens.first().is_some_and(|token| {
        matches!(
            token.text.as_str(),
            "goto" | "start" | "play_music" | "pause_music" | "resume_music" | "stop_music"
        )
    }) {
        return scene_effect_parser_recognition(tokens);
    }
    let mut recognition = crate::surface::ParserRecognition::default();
    let effect_span = source_tokens_span(tokens);
    add_rewrite_effect_surface_tokens(tokens, &mut recognition);
    parser_recognition_with_node(recognition, SurfaceNodeKind::RewriteEffect, effect_span)
}

fn add_rewrite_effect_surface_tokens(
    tokens: &[SourceToken],
    sink: &mut impl SemanticDispositionSink,
) -> bool {
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
        return add_simple_rewrite_effect_surface_tokens(tokens, sink);
    }

    match tokens {
        [visual] if puzzle_authoring::is_visual_emission_name(&visual.text) => {
            add_scene_effect_token_range(sink, visual, SurfaceSemanticKind::Asset);
            true
        }
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
        [name, op, value] if is_variable_update_operator(&op.text) => {
            add_scene_effect_token_range(sink, name, SurfaceSemanticKind::State);
            add_scene_effect_token_range(sink, value, SurfaceSemanticKind::Number);
            true
        }
        _ => false,
    }
}

fn add_simple_rewrite_effect_surface_tokens(
    tokens: &[SourceToken],
    sink: &mut impl SemanticDispositionSink,
) -> bool {
    let mut index = 0usize;
    let mut parsed_any = false;
    while index < tokens.len() {
        if puzzle_authoring::is_visual_emission_name(&tokens[index].text) {
            add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Asset);
            index += 1;
            parsed_any = true;
            continue;
        }
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
            _ if index + 2 < tokens.len()
                && is_variable_update_operator(&tokens[index + 1].text) =>
            {
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

fn parse_scene_effect_with_optional_block(
    value: &str,
    lines: &[source::LogicalLine],
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
    let effect = parse_scene_effect_value(value, line)?;
    let recognition = scene_effect_parser_recognition(&tokens);
    let document = parser_recognition_surface_document(&recognition);
    Ok(SurfaceSceneEffect { effect, document })
}

fn parse_scene_effect_value(value: &str, line: &str) -> Result<SceneEffect, DiagnosticReport> {
    puzzle_scene::parse_scene_effect_at(value, line).map_err(scene_parse_error)
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

const DEFAULT_WAIT_MS: u64 = 200;

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
        SceneComponent::Viewport(_) | SceneComponent::Frame(_) | SceneComponent::Text(_) => {}
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
        SceneEffect::Sequence { effects } => {
            for effect in effects {
                resolve_default_wait_in_effect(effect, default_wait_ms);
            }
        }
        SceneEffect::Input(_)
        | SceneEffect::ComponentEffect(_)
        | SceneEffect::RoutineCall(_)
        | SceneEffect::PresentComponent { .. }
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
        | SceneEffect::Move { .. }
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
    puzzle_scene::parse_wait_duration_ms_at(value, line).map_err(scene_parse_error)
}

pub fn parse_scene_expression(value: &str) -> Result<SceneExpr, DiagnosticReport> {
    puzzle_scene::parse_scene_expression(value).map_err(scene_parse_error)
}

pub fn parse_scene_expression_args(value: &str) -> Result<Vec<SceneExpr>, DiagnosticReport> {
    puzzle_scene::parse_scene_expression_args(value).map_err(scene_parse_error)
}

pub fn parse_scene_effect_params(value: &str) -> Result<Vec<SceneEffectParam>, DiagnosticReport> {
    puzzle_scene::parse_scene_effect_params(value).map_err(scene_parse_error)
}

fn parse_scene_expr(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    puzzle_scene::parse_scene_expression_at(value, line).map_err(scene_parse_error)
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

fn scene_parse_error(error: puzzle_scene::SceneParseError) -> DiagnosticReport {
    parse_error(error.source_line(), error.message())
}
