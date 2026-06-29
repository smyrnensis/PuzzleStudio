pub fn parse_game(source: &str) -> Result<LoadedDocument, DiagnosticReport> {
    parse_game_document(source)
}

pub fn parse_game_for_path(
    source: &str,
    path: impl AsRef<Path>,
) -> Result<LoadedDocument, DiagnosticReport> {
    validate_source_profile_for_path(source, path)?;
    parse_game_document(source)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentRuntimeSources {
    pub model_2d: String,
    pub model_3d: String,
}

pub fn split_document_runtime_sources(
    source: &str,
) -> Result<DocumentRuntimeSources, DiagnosticReport> {
    let mixed_sources = split_mixed_game_document_source(source)?;
    Ok(DocumentRuntimeSources {
        model_2d: strip_document_shell_source(&mixed_sources.puzzle2d)?,
        model_3d: strip_document_shell_source_raw(&mixed_sources.puzzle3d),
    })
}

pub fn parse_game2d(source: &str) -> Result<LoadedGame, DiagnosticReport> {
    let _surface = parse_surface_document(source);
    parse_game2d_document(source)
}

pub(crate) fn parse_surface_document(source: &str) -> SurfaceDocument {
    let context = scan_source_context(source);
    let mut sink = SurfaceSink::default();
    let mut option_stack = Vec::<SurfaceOptionBlock>::new();
    for line in &context.lines {
        let option_block = active_surface_option_block(&option_stack);
        record_surface_document_line(option_block, line.scope, &line.token_spans, &mut sink);
        update_surface_option_block_stack(line, &mut option_stack);
    }
    sink.into_document()
}

pub(crate) fn surface_document_semantic_tokens(source: &str) -> Vec<semantic::SemanticToken> {
    project_surface_semantic_tokens(&parse_surface_document(source).semantic_tokens)
}

fn record_surface_document_line(
    option_block: Option<SurfaceOptionBlock>,
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    if record_document_prelude_surface_line(scope, tokens, sink) {
        return;
    }
    if record_option_surface_line(option_block, scope, tokens, sink) {
        return;
    }
    if tokens
        .first()
        .is_some_and(|token| token.text.as_str() == "scene")
    {
        record_scene_surface_line(scope, tokens, sink);
        return;
    }
    if is_scene_surface_scope(scope) {
        record_scene_surface_line(scope, tokens, sink);
        return;
    }
    record_rewrite_surface_line(scope, tokens, sink);
}

fn record_document_prelude_surface_line(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    if scope.is_some() {
        return false;
    }
    let Some(first) = tokens.first() else {
        return false;
    };
    if let Some(action) = model_top_level_surface_directive(&first.text) {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
        match action {
            ModelTopLevelDirective::Title
            | ModelTopLevelDirective::Subtitle
            | ModelTopLevelDirective::Author
            | ModelTopLevelDirective::Homepage => {
                if let Some(value) = tokens.get(1) {
                    add_scene_effect_token_range(sink, value, SurfaceSemanticKind::String);
                }
            }
            ModelTopLevelDirective::Puzzle | ModelTopLevelDirective::Scene => {
                if let Some(name) = tokens.get(1) {
                    add_scene_effect_token_range(sink, name, SurfaceSemanticKind::Scene);
                }
            }
            _ => {}
        }
        return true;
    }
    if classify_known_mixed_document_section(
        &tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>(),
    )
    .is_some()
    {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
        if matches!(first.text.as_str(), "puzzle3")
            && let Some(name) = tokens.get(1)
        {
            add_scene_effect_token_range(sink, name, SurfaceSemanticKind::Scene);
        }
        return true;
    }
    false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceOptionBlock {
    Puzzle2,
    Puzzle3,
    Render2,
    Render3,
    Camera3,
    Grid2,
    Grid3,
    Pixelate3,
    Animation,
    Tween,
    LevelMenu,
    Theme,
    Other,
}

pub(crate) fn surface_option_block_before_line(
    lines: &[crate::source::SourceContextLine],
    line_index: usize,
) -> Option<SurfaceOptionBlock> {
    let mut stack = Vec::<SurfaceOptionBlock>::new();
    for line in lines.iter().take(line_index) {
        update_surface_option_block_stack(line, &mut stack);
    }
    active_surface_option_block(&stack)
}

fn active_surface_option_block(stack: &[SurfaceOptionBlock]) -> Option<SurfaceOptionBlock> {
    stack.iter().rev().copied().find(|block| {
        matches!(
            block,
            SurfaceOptionBlock::Render2
                | SurfaceOptionBlock::Render3
                | SurfaceOptionBlock::Camera3
                | SurfaceOptionBlock::Grid2
                | SurfaceOptionBlock::Grid3
                | SurfaceOptionBlock::Pixelate3
                | SurfaceOptionBlock::Animation
                | SurfaceOptionBlock::Tween
                | SurfaceOptionBlock::LevelMenu
                | SurfaceOptionBlock::Theme
        )
    })
}

fn update_surface_option_block_stack(
    line: &crate::source::SourceContextLine,
    stack: &mut Vec<SurfaceOptionBlock>,
) {
    let trimmed = line.content.trim();
    if trimmed == "}" {
        stack.pop();
        return;
    }
    if !surface_line_opens_option_block(line) {
        return;
    }
    let block = surface_option_block_for_opening(&line.tokens, stack);
    stack.push(block);
}

fn surface_line_opens_option_block(line: &crate::source::SourceContextLine) -> bool {
    line.content.trim_end().ends_with('{')
        || matches!(
            line.tokens.as_slice(),
            [name] if matches!(
                name.as_str(),
                "puzzle" | "puzzle3" | "render" | "camera" | "grid" | "pixelate"
                    | "animation" | "tween" | "level_menu" | "theme"
            )
        )
}

fn surface_option_block_for_opening(
    tokens: &[String],
    stack: &[SurfaceOptionBlock],
) -> SurfaceOptionBlock {
    let Some(first) = tokens.first().map(String::as_str) else {
        return SurfaceOptionBlock::Other;
    };
    match first {
        "puzzle3" => SurfaceOptionBlock::Puzzle3,
        "puzzle" => SurfaceOptionBlock::Puzzle2,
        "render" if stack.contains(&SurfaceOptionBlock::Puzzle3) => SurfaceOptionBlock::Render3,
        "render" => SurfaceOptionBlock::Render2,
        "camera" if stack.last() == Some(&SurfaceOptionBlock::Render3) => {
            SurfaceOptionBlock::Camera3
        }
        "grid" if stack.last() == Some(&SurfaceOptionBlock::Render3) => SurfaceOptionBlock::Grid3,
        "grid" if stack.last() == Some(&SurfaceOptionBlock::Render2) => SurfaceOptionBlock::Grid2,
        "pixelate" if stack.last() == Some(&SurfaceOptionBlock::Render3) => {
            SurfaceOptionBlock::Pixelate3
        }
        "animation" => SurfaceOptionBlock::Animation,
        "tween" if stack.last() == Some(&SurfaceOptionBlock::Animation) => {
            SurfaceOptionBlock::Tween
        }
        "level_menu" => SurfaceOptionBlock::LevelMenu,
        "theme" => SurfaceOptionBlock::Theme,
        _ => SurfaceOptionBlock::Other,
    }
}

fn record_option_surface_line(
    block: Option<SurfaceOptionBlock>,
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };
    match block {
        Some(SurfaceOptionBlock::Puzzle3) if first.text == "render" => {
            add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
            mark_option_surface_tokens(&tokens[1..], puzzle_3d::RENDER_OPTIONS3, sink);
            true
        }
        Some(SurfaceOptionBlock::Puzzle2) if first.text == "render" => {
            add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
            mark_option_surface_tokens(&tokens[1..], PUZZLE_RENDER_BLOCK_OPTIONS, sink);
            true
        }
        _ if scope == Some(SourceScope::Puzzle) && first.text == "render" => {
            add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
            true
        }
        Some(SurfaceOptionBlock::Puzzle2 | SurfaceOptionBlock::Puzzle3)
            if first.text == "animation" =>
        {
            add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
            mark_option_surface_tokens(&tokens[1..], ANIMATION_BLOCK_OPTIONS, sink);
            true
        }
        _ if scope == Some(SourceScope::Puzzle) && first.text == "animation" => {
            add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
            mark_option_surface_tokens(&tokens[1..], ANIMATION_BLOCK_OPTIONS, sink);
            true
        }
        Some(SurfaceOptionBlock::Render3)
            if puzzle_3d::RENDER_OPTIONS3.contains(&first.text.as_str()) =>
        {
            add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Setting);
            match first.text.as_str() {
                "camera" => {
                    mark_option_surface_tokens(&tokens[1..], puzzle_3d::CAMERA_OPTIONS3, sink);
                }
                "grid" => {
                    mark_option_surface_tokens(&tokens[1..], puzzle_3d::GRID_BARE_OPTIONS3, sink);
                }
                "pixelate" => {
                    mark_option_surface_tokens(&tokens[1..], puzzle_3d::PIXELATE_OPTIONS3, sink);
                }
                _ => {}
            }
            true
        }
        Some(SurfaceOptionBlock::Render2)
            if PUZZLE_RENDER_BLOCK_OPTIONS.contains(&first.text.as_str()) =>
        {
            add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Setting);
            if first.text == "grid" {
                mark_option_surface_tokens(&tokens[1..], PUZZLE_RENDER_GRID_OPTIONS, sink);
            }
            true
        }
        Some(SurfaceOptionBlock::Camera3) => {
            mark_option_surface_tokens(tokens, puzzle_3d::CAMERA_OPTIONS3, sink)
        }
        Some(SurfaceOptionBlock::Grid3) => {
            mark_option_surface_tokens(tokens, puzzle_3d::GRID_BARE_OPTIONS3, sink)
        }
        Some(SurfaceOptionBlock::Pixelate3) => {
            mark_option_surface_tokens(tokens, puzzle_3d::PIXELATE_OPTIONS3, sink)
        }
        Some(SurfaceOptionBlock::Grid2) => {
            mark_option_surface_tokens(tokens, PUZZLE_RENDER_GRID_OPTIONS, sink)
        }
        Some(SurfaceOptionBlock::Animation)
            if ANIMATION_BLOCK_OPTIONS.contains(&first.text.as_str()) =>
        {
            add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Setting);
            if first.text == "tween" {
                mark_option_surface_tokens(&tokens[1..], ANIMATION_TWEEN_OPTIONS, sink);
            }
            true
        }
        Some(SurfaceOptionBlock::Tween) => {
            mark_option_surface_tokens(tokens, ANIMATION_TWEEN_OPTIONS, sink)
        }
        Some(SurfaceOptionBlock::LevelMenu) => {
            mark_option_surface_tokens(tokens, LEVEL_MENU_OPTIONS, sink)
        }
        Some(SurfaceOptionBlock::Theme) => mark_theme_setting_surface_tokens(tokens, sink),
        _ => false,
    }
}

fn mark_option_surface_tokens(
    tokens: &[SourceToken],
    option_names: &'static [&'static str],
    sink: &mut SurfaceSink,
) -> bool {
    let mut marked_any = false;
    for token in tokens {
        let name = token
            .text
            .split_once('=')
            .map_or(token.text.as_str(), |(name, _)| name);
        if !name.is_empty()
            && option_names.contains(&name)
            && mark_token_part(sink, token, name, SurfaceSemanticKind::Setting)
        {
            marked_any = true;
        }
    }
    marked_any
}

fn mark_theme_setting_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) -> bool {
    let mut marked_any = false;
    for token in tokens {
        let name = token
            .text
            .trim_start_matches("--")
            .split_once('=')
            .map_or(token.text.as_str(), |(name, _)| name);
        let normalized = name.replace('_', "-").to_ascii_lowercase();
        if THEME_SETTING_SPECS.iter().any(|spec| {
            normalized == spec.canonical.replace('_', "-")
                || spec.aliases.iter().any(|alias| normalized == *alias)
        }) && mark_token_part(sink, token, name, SurfaceSemanticKind::Setting)
        {
            marked_any = true;
        }
    }
    marked_any
}

fn mark_token_part(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    part: &str,
    kind: SurfaceSemanticKind,
) -> bool {
    let Some(relative_start) = token.text.find(part) else {
        return false;
    };
    if part.is_empty() {
        return false;
    }
    sink.mark(
        SourceSpan {
            start: token.start + relative_start,
            end: token.start + relative_start + part.len(),
        },
        kind,
    );
    true
}

fn is_scene_surface_scope(scope: Option<SourceScope>) -> bool {
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

fn record_scene_surface_line(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    let Some(first) = tokens.first() else {
        return;
    };
    if first.text == "scene" {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
        if let Some(name) = tokens.get(1) {
            add_scene_effect_token_range(sink, name, SurfaceSemanticKind::Scene);
        }
        return;
    }
    if scope == Some(SourceScope::SceneState) {
        return;
    }
    if scene_block_keyword(&first.text)
        || puzzle_scene::SceneComponentKind::from_keyword(&first.text).is_some()
    {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
    }
    record_scene_layout_attr_surface_tokens(tokens, sink);
    if let Some(arrow) = tokens.iter().position(|token| token.text == "->") {
        if tokens
            .iter()
            .any(|token| token.text.contains('[') || token.text.contains(']'))
        {
            return;
        }
        if matches!(first.text.as_str(), "button" | "choice") {
            sink.extend(scene_expr_surface_document(&tokens[1..arrow]));
        }
        record_scene_condition_surface_tokens(&tokens[..arrow], sink);
        sink.extend(scene_effect_surface_document(&tokens[arrow + 1..]));
        return;
    }
    match first.text.as_str() {
        "title" | "subtitle" | "text" if tokens.len() > 1 => {
            sink.extend(scene_expr_surface_document(&tokens[1..]));
            return;
        }
        _ => {}
    }
    if first.text == "button" || first.text == "choice" || scope == Some(SourceScope::LevelMenu) {
        return;
    }
    sink.extend(scene_effect_surface_document(tokens));
}

fn record_scene_layout_attr_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    let Some(first) = tokens.first() else {
        return;
    };
    let attr_start = match first.text.as_str() {
        "layout" | "row" | "column" | "box" => 1,
        "puzzle" | "puzzle3" | "frame" => 2,
        _ => return,
    };
    let attrs = tokens
        .iter()
        .skip(attr_start)
        .take_while(|token| token.text != "{")
        .cloned()
        .collect::<Vec<_>>();
    if attrs.is_empty() {
        return;
    }
    let attr_texts = attrs
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    if puzzle_scene::parse_scene_layout_attrs(&attr_texts).is_err() {
        return;
    }
    for token in &attrs {
        mark_scene_layout_attr_token(token, sink);
    }
}

fn mark_scene_layout_attr_token(token: &SourceToken, sink: &mut SurfaceSink) {
    let (name, value) = token
        .text
        .split_once('=')
        .map_or((token.text.as_str(), None), |(name, value)| {
            (name, Some(value))
        });
    if !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
    {
        let name_start = token.text.find(name).unwrap_or(0);
        sink.mark(
            SourceSpan {
                start: token.start + name_start,
                end: token.start + name_start + name.len(),
            },
            SurfaceSemanticKind::Keyword,
        );
    }
    if let Some(value) = value
        && !value.is_empty()
    {
        let value_start = token.text.find(value).unwrap_or(token.text.len());
        let kind = if value.parse::<u16>().is_ok() {
            SurfaceSemanticKind::Number
        } else {
            SurfaceSemanticKind::Literal
        };
        sink.mark(
            SourceSpan {
                start: token.start + value_start,
                end: token.start + value_start + value.len(),
            },
            kind,
        );
    } else if token.text.parse::<u16>().is_ok() {
        sink.mark(
            SourceSpan {
                start: token.start,
                end: token.end,
            },
            SurfaceSemanticKind::Number,
        );
    } else if matches!(
        token.text.as_str(),
        "left" | "right" | "center" | "top" | "bottom" | "true" | "false"
    ) {
        sink.mark(
            SourceSpan {
                start: token.start,
                end: token.end,
            },
            SurfaceSemanticKind::Literal,
        );
    }
}

fn scene_expr_surface_document(tokens: &[SourceToken]) -> SurfaceDocument {
    let mut sink = SurfaceSink::default();
    let Some((source, base_start)) = source_tokens_text(tokens) else {
        return sink.into_document();
    };
    let trimmed_start = source
        .find(|ch: char| !ch.is_whitespace())
        .unwrap_or(source.len());
    let trimmed_end = source
        .rfind(|ch: char| !ch.is_whitespace())
        .map(|index| {
            index
                + source[index..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(0)
        })
        .unwrap_or(trimmed_start);
    if trimmed_start >= trimmed_end {
        return sink.into_document();
    }
    let expr = &source[trimmed_start..trimmed_end];
    let expr_start = base_start + trimmed_start;
    if let Ok(parsed) = parse_scene_expr(expr, expr) {
        add_scene_expr_surface_tokens(&parsed, expr, expr_start, &mut sink);
    }
    sink.into_document()
}

fn source_tokens_text(tokens: &[SourceToken]) -> Option<(String, usize)> {
    let first = tokens.first()?;
    let mut out = String::new();
    let mut cursor = first.start;
    for token in tokens {
        if token.start > cursor {
            out.push_str(&" ".repeat(token.start - cursor));
        }
        out.push_str(&token.text);
        cursor = token.end;
    }
    Some((out, first.start))
}

fn add_scene_expr_surface_tokens(
    expr: &SceneExpr,
    source: &str,
    absolute_start: usize,
    sink: &mut SurfaceSink,
) {
    match expr {
        SceneExpr::Bool(_) => {
            mark_scene_expr_trimmed(sink, source, absolute_start, SurfaceSemanticKind::Literal);
        }
        SceneExpr::Int(_) => {
            mark_scene_expr_trimmed(sink, source, absolute_start, SurfaceSemanticKind::Number);
        }
        SceneExpr::Text(_) => {
            mark_scene_expr_trimmed(sink, source, absolute_start, SurfaceSemanticKind::String);
        }
        SceneExpr::Path(parts) => {
            add_scene_path_surface_tokens(parts, absolute_start, sink);
        }
        SceneExpr::Call { name, args: _ } => {
            if let Some(name_start) = source.find(name) {
                sink.mark(
                    SourceSpan {
                        start: absolute_start + name_start,
                        end: absolute_start + name_start + name.len(),
                    },
                    SurfaceSemanticKind::Effect,
                );
            }
            for arg in scene_expr_call_arg_spans(source) {
                if let Ok(parsed) = parse_scene_expr(&source[arg.clone()], &source[arg.clone()]) {
                    add_scene_expr_surface_tokens(
                        &parsed,
                        &source[arg.clone()],
                        absolute_start + arg.start,
                        sink,
                    );
                } else {
                    let arg_source = &source[arg.clone()];
                    if let Some(parts) = parse_view_path(arg_source) {
                        add_scene_path_surface_tokens(&parts, absolute_start + arg.start, sink);
                    }
                }
            }
        }
    }
}

fn mark_scene_expr_trimmed(
    sink: &mut SurfaceSink,
    source: &str,
    absolute_start: usize,
    kind: SurfaceSemanticKind,
) {
    let Some(start) = source.find(|ch: char| !ch.is_whitespace()) else {
        return;
    };
    let Some(end_start) = source.rfind(|ch: char| !ch.is_whitespace()) else {
        return;
    };
    let end = end_start
        + source[end_start..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0);
    if start < end {
        sink.mark(
            SourceSpan {
                start: absolute_start + start,
                end: absolute_start + end,
            },
            kind,
        );
    }
}

fn add_scene_path_surface_tokens(parts: &[String], absolute_start: usize, sink: &mut SurfaceSink) {
    let part_refs = parts.iter().map(String::as_str).collect::<Vec<_>>();
    let mut offset = 0usize;
    for (index, part) in part_refs.iter().enumerate() {
        if let Some(syntax) = level_path_part_syntax(&part_refs, index) {
            let kind = match syntax {
                LevelPathPartSyntax::Owner => SurfaceSemanticKind::State,
                LevelPathPartSyntax::TextProperty => SurfaceSemanticKind::String,
                LevelPathPartSyntax::NumberProperty => SurfaceSemanticKind::Number,
                LevelPathPartSyntax::ConditionProperty => SurfaceSemanticKind::Condition,
            };
            sink.mark(
                SourceSpan {
                    start: absolute_start + offset,
                    end: absolute_start + offset + part.len(),
                },
                kind,
            );
        }
        offset += part.len() + 1;
    }
}

fn scene_expr_call_arg_spans(source: &str) -> Vec<std::ops::Range<usize>> {
    let Some(open) = source.find('(') else {
        return Vec::new();
    };
    let Some(close) = source.rfind(')') else {
        return Vec::new();
    };
    if open >= close {
        return Vec::new();
    }
    let mut args = Vec::new();
    let mut start = open + 1;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (relative, ch) in source[open + 1..close].char_indices() {
        let index = open + 1 + relative;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                if let Some(range) = trimmed_range(source, start, index) {
                    args.push(range);
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if let Some(range) = trimmed_range(source, start, close) {
        args.push(range);
    }
    args
}

fn trimmed_range(source: &str, start: usize, end: usize) -> Option<std::ops::Range<usize>> {
    if start >= end {
        return None;
    }
    let slice = &source[start..end];
    let left = slice.find(|ch: char| !ch.is_whitespace())?;
    let right_start = slice.rfind(|ch: char| !ch.is_whitespace())?;
    let right = right_start
        + slice[right_start..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0);
    Some(start + left..start + right)
}

fn record_scene_condition_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    let Some(first) = tokens.first() else {
        return;
    };
    if first.text == "if" {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
    }
}

fn scene_block_keyword(value: &str) -> bool {
    matches!(
        value,
        "keys" | "on_scene_start" | "resources" | "rules" | "state" | "transitions" | "layout"
    )
}

fn record_rewrite_surface_line(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    if let Some(arrow) = tokens.iter().position(|token| token.text == "->") {
        let rhs = &tokens[arrow + 1..];
        let effect_start = rhs
            .iter()
            .rposition(|token| token.text.contains(']'))
            .map_or(0, |index| index + 1);
        if effect_start < rhs.len() {
            sink.extend(rewrite_effect_surface_document(&rhs[effect_start..]));
        }
        return;
    }

    if scope == Some(SourceScope::Other) {
        sink.extend(rewrite_effect_surface_document(tokens));
    }
}
