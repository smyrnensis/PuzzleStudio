use crate::source::{
    SourceContext, SourceContextLine, SourceScope, scan_source_context, split_header_tokens,
    strip_line_comment,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceTargetKind {
    Level,
    Level3d,
    Sprite,
    Sprite3d,
    Sounds,
}

impl SourceTargetKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Level => "level",
            Self::Level3d => "level3d",
            Self::Sprite => "sprite",
            Self::Sprite3d => "sprite3d",
            Self::Sounds => "sounds",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SoundSourceTargetKind {
    Sfx,
    Music,
}

impl SoundSourceTargetKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Sfx => "sfx",
            Self::Music => "music",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceTarget {
    pub kind: SourceTargetKind,
    pub name: String,
    pub start: usize,
    pub end: usize,
    pub body_start: Option<usize>,
    pub body_end: Option<usize>,
    pub level_index: Option<usize>,
    pub sound_kind: Option<SoundSourceTargetKind>,
    pub params: Vec<(String, String)>,
    pub source_sprite: Option<SourceSpriteTarget>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceSpriteTarget {
    pub prelude_rows: Vec<String>,
    pub palette_tokens: Vec<String>,
    pub resolved_palette: Vec<SourceSpritePaletteEntry>,
    pub pixel_rows: Vec<String>,
    pub shape_ref: Option<String>,
    pub resolved_shape_rows: Vec<String>,
    pub color_assets: Vec<SourceSpriteColorAsset>,
    pub shape_assets: Vec<SourceSpriteShapeAsset>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceSpritePaletteEntry {
    pub source: String,
    pub color: String,
    pub linked: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceSpriteColorAsset {
    pub name: String,
    pub color: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceSpriteShapeAsset {
    pub name: String,
    pub rows: Vec<String>,
}

pub fn resolve_source_target(source: &str, cursor_offset: usize) -> Option<SourceTarget> {
    let cursor = cursor_offset.min(source.len());
    let context = scan_source_context(source);
    resolve_source_target_from_context(source, &context, cursor)
}

pub(crate) fn resolve_source_target_from_context(
    source: &str,
    context: &SourceContext,
    cursor: usize,
) -> Option<SourceTarget> {
    let visual_refs = collect_visual_sprite_refs(source, context);
    resolve_sounds_target(context, cursor)
        .or_else(|| resolve_level3d_target(source, context, cursor))
        .or_else(|| resolve_level_target(source, context, cursor))
        .or_else(|| resolve_sprite3d_target(source, context, cursor))
        .or_else(|| resolve_sprite_target(source, context, cursor, &visual_refs))
}

pub fn source_target_json(target: Option<&SourceTarget>) -> String {
    let mut out = String::new();
    out.push_str("{\"target\":");
    match target {
        Some(target) => push_target_json(&mut out, target),
        None => out.push_str("null"),
    }
    out.push('}');
    out
}

fn push_target_json(out: &mut String, target: &SourceTarget) {
    out.push('{');
    push_json_string(out, "kind", target.kind.as_str());
    out.push(',');
    push_json_string(out, "name", &target.name);
    out.push(',');
    push_json_number(out, "start", target.start);
    out.push(',');
    push_json_number(out, "end", target.end);
    if let Some(body_start) = target.body_start {
        out.push(',');
        push_json_number(out, "bodyStart", body_start);
    }
    if let Some(body_end) = target.body_end {
        out.push(',');
        push_json_number(out, "bodyEnd", body_end);
    }
    if let Some(level_index) = target.level_index {
        out.push(',');
        push_json_number(out, "levelIndex", level_index);
    }
    if let Some(sound_kind) = &target.sound_kind {
        out.push(',');
        push_json_string(out, "soundKind", sound_kind.as_str());
    }
    if !target.params.is_empty() {
        out.push_str(",\"params\":{");
        for (index, (key, value)) in target.params.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_json_string_value(out, key);
            out.push(':');
            push_json_string_value(out, value);
        }
        out.push('}');
    }
    if let Some(sprite) = &target.source_sprite {
        out.push_str(",\"sourceSprite\":");
        push_source_sprite_json(out, sprite);
    }
    out.push('}');
}

fn push_source_sprite_json(out: &mut String, sprite: &SourceSpriteTarget) {
    out.push('{');
    push_json_string_array(out, "preludeRows", &sprite.prelude_rows);
    out.push(',');
    push_json_string_array(out, "paletteTokens", &sprite.palette_tokens);
    out.push_str(",\"resolvedPalette\":");
    push_source_sprite_palette_json(out, &sprite.resolved_palette);
    out.push(',');
    push_json_string_array(out, "pixelRows", &sprite.pixel_rows);
    out.push_str(",\"shapeRef\":");
    match &sprite.shape_ref {
        Some(shape_ref) => push_json_string_value(out, shape_ref),
        None => out.push_str("null"),
    }
    out.push_str(",\"resolvedShapeRows\":");
    push_json_string_array_value(out, &sprite.resolved_shape_rows);
    out.push_str(",\"colorAssets\":");
    push_source_sprite_color_assets_json(out, &sprite.color_assets);
    out.push_str(",\"shapeAssets\":");
    push_source_sprite_shape_assets_json(out, &sprite.shape_assets);
    out.push('}');
}

fn push_source_sprite_palette_json(out: &mut String, entries: &[SourceSpritePaletteEntry]) {
    out.push('[');
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_string(out, "source", &entry.source);
        out.push(',');
        push_json_string(out, "color", &entry.color);
        out.push_str(",\"linked\":");
        out.push_str(if entry.linked { "true" } else { "false" });
        out.push('}');
    }
    out.push(']');
}

fn push_source_sprite_color_assets_json(out: &mut String, entries: &[SourceSpriteColorAsset]) {
    out.push('[');
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_string(out, "name", &entry.name);
        out.push(',');
        push_json_string(out, "color", &entry.color);
        out.push('}');
    }
    out.push(']');
}

fn push_source_sprite_shape_assets_json(out: &mut String, entries: &[SourceSpriteShapeAsset]) {
    out.push('[');
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_string(out, "name", &entry.name);
        out.push_str(",\"rows\":");
        push_json_string_array_value(out, &entry.rows);
        out.push('}');
    }
    out.push(']');
}

fn resolve_sounds_target(context: &SourceContext, cursor: usize) -> Option<SourceTarget> {
    for line in &context.lines {
        let line_end = line_end(line);
        if cursor < line.start || cursor > line_end || line.scope != Some(SourceScope::Sounds) {
            continue;
        }
        let (sound_kind, name, params) = parse_sound_definition(line)?;
        return Some(SourceTarget {
            kind: SourceTargetKind::Sounds,
            name,
            start: line.start,
            end: line_end,
            body_start: None,
            body_end: None,
            level_index: None,
            sound_kind: Some(sound_kind),
            params,
            source_sprite: None,
        });
    }
    None
}

fn parse_sound_definition(
    line: &SourceContextLine,
) -> Option<(SoundSourceTargetKind, String, Vec<(String, String)>)> {
    let [kind, name, rest @ ..] = line.tokens.as_slice() else {
        return None;
    };
    let sound_kind = match kind.as_str() {
        "sfx" => SoundSourceTargetKind::Sfx,
        "music" => SoundSourceTargetKind::Music,
        _ => return None,
    };
    Some((sound_kind, name.clone(), parse_assignment_params(rest)))
}

fn parse_assignment_params(tokens: &[String]) -> Vec<(String, String)> {
    tokens
        .iter()
        .filter_map(|token| {
            let (key, value) = token.split_once('=')?;
            Some((key.to_string(), trim_quotes(value).to_string()))
        })
        .collect()
}

fn trim_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|stripped| stripped.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|stripped| stripped.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn resolve_level_target(
    source: &str,
    context: &SourceContext,
    cursor: usize,
) -> Option<SourceTarget> {
    let level3d_blocks = level3d_blocks(source, context);
    let mut level_index = 0usize;
    for (index, line) in context.lines.iter().enumerate() {
        if level3d_blocks
            .iter()
            .any(|block| line.start > block.open_index && line.start < block.close_index)
        {
            continue;
        }
        let Some(name) = level_name(line) else {
            continue;
        };
        let start = line.start;
        let (end, body_start, body_end) = level_range(source, context, index);
        let current_index = level_index;
        level_index += 1;
        if cursor < start || cursor > end {
            continue;
        }
        return Some(SourceTarget {
            kind: SourceTargetKind::Level,
            name,
            start,
            end,
            body_start: Some(body_start),
            body_end: Some(body_end),
            level_index: Some(current_index),
            sound_kind: None,
            params: Vec::new(),
            source_sprite: None,
        });
    }
    None
}

fn resolve_level3d_target(
    source: &str,
    context: &SourceContext,
    cursor: usize,
) -> Option<SourceTarget> {
    let mut level_index = 0usize;
    for block in level3d_blocks(source, context) {
        let bundle = block.bundle.clone();
        let model = block.model.clone();
        if cursor <= block.open_index || cursor >= block.close_index {
            level_index += context
                .lines
                .iter()
                .filter(|line| line.start > block.open_index && line.start < block.close_index)
                .filter(|line| level_name(line).is_some())
                .count();
            continue;
        }
        for (index, line) in context.lines.iter().enumerate() {
            if line.start <= block.open_index || line.start >= block.close_index {
                continue;
            }
            let Some(name) = level_name(line) else {
                continue;
            };
            let start = line.start;
            let (end, body_start, body_end) = level_range(source, context, index);
            let current_index = level_index;
            level_index += 1;
            if cursor < start || cursor > end {
                continue;
            }
            return Some(SourceTarget {
                kind: SourceTargetKind::Level3d,
                name,
                start,
                end,
                body_start: Some(body_start),
                body_end: Some(body_end),
                level_index: Some(current_index),
                sound_kind: None,
                params: vec![("bundle".to_string(), bundle), ("model".to_string(), model)],
                source_sprite: None,
            });
        }
    }
    None
}

fn level_name(line: &SourceContextLine) -> Option<String> {
    match line.tokens.as_slice() {
        [keyword, name, ..] if keyword == "level" => Some(clean_name_token(name)),
        _ => None,
    }
}

fn level_range(source: &str, context: &SourceContext, start_index: usize) -> (usize, usize, usize) {
    let line = &context.lines[start_index];
    let header_end = line_end(line);
    if let Some(open_index) = source[line.start..header_end]
        .find('{')
        .map(|offset| line.start + offset)
    {
        let end = find_matching_brace(source, open_index)
            .map(|index| index + 1)
            .unwrap_or(header_end);
        return (end, open_index + 1, end.saturating_sub(1));
    }

    let body_start = header_end;
    let mut end = header_end;
    let mut body_end = body_start;
    for next in context.lines.iter().skip(start_index + 1) {
        if level_name(next).is_some() {
            break;
        }
        let next_trimmed = code_trim(&next.content);
        let in_level = matches!(
            next.scope,
            Some(SourceScope::Level | SourceScope::UnbracedLevel | SourceScope::Legend)
        );
        if !in_level && !next_trimmed.is_empty() {
            break;
        }
        end = line_end(next);
        if !next_trimmed.is_empty() {
            body_end = line_end(next);
        }
        if next.scope == Some(SourceScope::UnbracedLevel) && next_trimmed.is_empty() {
            break;
        }
    }
    (end, body_start, body_end)
}

fn resolve_sprite_target(
    source: &str,
    context: &SourceContext,
    cursor: usize,
    visual_refs: &VisualSpriteRefs,
) -> Option<SourceTarget> {
    let sprite3d_blocks = sprite3d_blocks(source, context);
    let mut covered_until = 0usize;
    for (index, line) in context.lines.iter().enumerate() {
        if line.start < covered_until {
            continue;
        }
        if !sprite_header_scope(line.scope) {
            continue;
        }
        if sprite3d_blocks
            .iter()
            .any(|block| line.start > block.open_index && line.start < block.close_index)
        {
            continue;
        }
        let line_end = line_end(line);
        if let Some((name, body_start)) = line_style_sprite_header(line, visual_refs) {
            if cursor < line.start || cursor > line_end {
                continue;
            }
            let source_sprite =
                source_sprite_target(source, &name, body_start, line_end, visual_refs);
            return Some(SourceTarget {
                kind: SourceTargetKind::Sprite,
                name,
                start: line.start,
                end: line_end,
                body_start: Some(body_start),
                body_end: Some(line_end),
                level_index: None,
                sound_kind: None,
                params: Vec::new(),
                source_sprite,
            });
        }
        let Some(name) = sprite_name(line) else {
            continue;
        };
        if let Some(open_index) = source[line.start..line_end]
            .find('{')
            .map(|offset| line.start + offset)
        {
            let end = find_matching_brace(source, open_index)? + 1;
            if cursor < line.start {
                continue;
            }
            if cursor > end {
                covered_until = end;
                continue;
            }
            let source_sprite = source_sprite_target(
                source,
                &name,
                open_index + 1,
                end.saturating_sub(1),
                visual_refs,
            );
            return Some(SourceTarget {
                kind: SourceTargetKind::Sprite,
                name,
                start: line.start,
                end,
                body_start: Some(open_index + 1),
                body_end: Some(end.saturating_sub(1)),
                level_index: None,
                sound_kind: None,
                params: Vec::new(),
                source_sprite,
            });
        }
        let Some((end, body_start, body_end)) = unbraced_sprite_range(context, index, visual_refs)
        else {
            continue;
        };
        if cursor < line.start {
            continue;
        }
        if cursor > end {
            covered_until = end;
            continue;
        }
        let source_sprite = source_sprite_target(source, &name, body_start, body_end, visual_refs);
        return Some(SourceTarget {
            kind: SourceTargetKind::Sprite,
            name,
            start: line.start,
            end,
            body_start: Some(body_start),
            body_end: Some(body_end),
            level_index: None,
            sound_kind: None,
            params: Vec::new(),
            source_sprite,
        });
    }
    None
}

fn source_sprite_target(
    source: &str,
    target_name: &str,
    body_start: usize,
    body_end: usize,
    visual_refs: &VisualSpriteRefs,
) -> Option<SourceSpriteTarget> {
    let body = source.get(body_start..body_end)?;
    let mut target = SourceSpriteTarget::default();
    let mut saw_palette = false;
    for raw_line in body.lines() {
        let trimmed = code_trim(raw_line);
        if trimmed.is_empty() {
            continue;
        }
        let tokens = split_header_tokens(trimmed);
        match tokens.as_slice() {
            ["rotate", ..] | ["pixels_per_cell", ..] | ["offset", ..] => {
                target.prelude_rows.push(trimmed.to_string());
            }
            ["colors", colors @ ..] if !saw_palette && !colors.is_empty() => {
                target.palette_tokens = colors.iter().map(|token| (*token).to_string()).collect();
                saw_palette = true;
            }
            ["shape", shape] => {
                target.shape_ref = Some((*shape).to_string());
            }
            [shape]
                if saw_palette
                    && target.shape_ref.is_none()
                    && target.pixel_rows.is_empty()
                    && visual_refs.contains_shape(shape) =>
            {
                target.shape_ref = Some((*shape).to_string());
            }
            _ if !saw_palette && is_sprite_entry_start_color_row(trimmed, visual_refs) => {
                target.palette_tokens = tokens.iter().map(|token| (*token).to_string()).collect();
                saw_palette = true;
            }
            _ if saw_palette && target.shape_ref.is_none() => {
                target.pixel_rows.push(trimmed.to_string());
            }
            _ => {}
        }
    }
    target.color_assets = visual_refs
        .color_assets
        .iter()
        .map(|(name, color)| SourceSpriteColorAsset {
            name: name.clone(),
            color: color.clone(),
        })
        .collect();
    target
        .color_assets
        .sort_by(|left, right| left.name.cmp(&right.name));
    target.shape_assets = visual_refs
        .shape_assets
        .iter()
        .map(|(name, rows)| SourceSpriteShapeAsset {
            name: name.clone(),
            rows: rows.clone(),
        })
        .collect();
    target
        .shape_assets
        .sort_by(|left, right| left.name.cmp(&right.name));
    enrich_source_sprite_target_from_loaded_visual(source, target_name, &mut target);
    if target.resolved_palette.is_empty() {
        target.resolved_palette = direct_source_sprite_palette(&target.palette_tokens);
    }
    Some(target)
}

fn enrich_source_sprite_target_from_loaded_visual(
    source: &str,
    target_name: &str,
    target: &mut SourceSpriteTarget,
) {
    let Some(sprite) = loaded_visual_sprite_for_source_target(source, target_name) else {
        return;
    };
    match sprite.kind {
        crate::VisualSpriteKind::Solid(color) => {
            let source = target
                .palette_tokens
                .first()
                .cloned()
                .unwrap_or_else(|| color.clone());
            target.resolved_palette = vec![SourceSpritePaletteEntry {
                linked: source != color && !is_sprite_color(&source),
                source,
                color,
            }];
        }
        crate::VisualSpriteKind::Ascii { pattern, colors } => {
            target.resolved_palette = colors
                .into_iter()
                .enumerate()
                .map(|(index, color)| {
                    let source = target
                        .palette_tokens
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| color.color.clone());
                    SourceSpritePaletteEntry {
                        linked: source != color.color && !is_sprite_color(&source),
                        source,
                        color: color.color,
                    }
                })
                .collect();
            if target.shape_ref.is_some() {
                target.resolved_shape_rows = pattern;
            }
        }
        crate::VisualSpriteKind::Image { .. } => {}
    }
}

fn loaded_visual_sprite_for_source_target(
    source: &str,
    target_name: &str,
) -> Option<crate::VisualSpriteDef> {
    let document = crate::parse_game_document(source).ok()?;
    let clean_target = clean_name_token(target_name);
    let mut candidate_sprite_names = HashSet::<String>::new();
    candidate_sprite_names.insert(clean_target.clone());
    candidate_sprite_names.insert(crate::sprite_name_for_object(&clean_target));
    for model in &document.models {
        let crate::LoadedDocumentModel::Puzzle2d { game, .. } = model else {
            continue;
        };
        for alias in &game.visuals.aliases {
            if alias.object == clean_target {
                candidate_sprite_names.insert(alias.sprite.clone());
            }
        }
        if let Some(sprite) = game
            .visuals
            .sprites
            .iter()
            .find(|sprite| candidate_sprite_names.contains(&sprite.name))
        {
            return Some(sprite.clone());
        }
    }
    None
}

fn direct_source_sprite_palette(tokens: &[String]) -> Vec<SourceSpritePaletteEntry> {
    if tokens.is_empty() || !tokens.iter().all(|token| is_sprite_color(token)) {
        return Vec::new();
    }
    tokens
        .iter()
        .map(|token| SourceSpritePaletteEntry {
            source: token.clone(),
            color: token.clone(),
            linked: false,
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct Sprite3dBlock {
    open_index: usize,
    close_index: usize,
}

#[derive(Clone, Debug)]
struct Level3dBlock {
    open_index: usize,
    close_index: usize,
    bundle: String,
    model: String,
}

fn level3d_blocks(source: &str, context: &SourceContext) -> Vec<Level3dBlock> {
    context
        .lines
        .iter()
        .filter(|line| line.tokens.first().is_some_and(|token| token == "levels3"))
        .filter_map(|line| {
            let header_end = line_end(line);
            let open_index = source[line.start..header_end]
                .find('{')
                .map(|offset| line.start + offset)?;
            let close_index = find_matching_brace(source, open_index)?;
            let (bundle, model) = parse_levels3_tokens(&line.tokens);
            Some(Level3dBlock {
                open_index,
                close_index,
                bundle,
                model,
            })
        })
        .collect()
}

fn parse_levels3_tokens(tokens: &[String]) -> (String, String) {
    let bundle = tokens
        .get(1)
        .filter(|token| token.as_str() != "of" && token.as_str() != "{")
        .cloned()
        .unwrap_or_else(|| "levels".to_string());
    let model = tokens
        .windows(2)
        .find_map(|pair| (pair[0] == "of").then(|| pair[1].clone()))
        .unwrap_or_default();
    (clean_name_token(&bundle), clean_name_token(&model))
}

fn resolve_sprite3d_target(
    source: &str,
    context: &SourceContext,
    cursor: usize,
) -> Option<SourceTarget> {
    for block in sprite3d_blocks(source, context) {
        if cursor <= block.open_index || cursor >= block.close_index {
            continue;
        }
        for (index, line) in context.lines.iter().enumerate() {
            if line.start <= block.open_index || line.start >= block.close_index {
                continue;
            }
            let Some(name) = sprite3d_name(line) else {
                continue;
            };
            let start = line.start;
            let body_start = line_end(line);
            let mut end = body_start;
            let mut body_end = body_start;
            let complete_entry = next_sprite3d_palette_line(context, index, block);
            let mut saw_palette_row = false;
            for (next_index, next) in context.lines.iter().enumerate().skip(index + 1) {
                if next.start >= block.close_index {
                    break;
                }
                let next_trimmed = code_trim(&next.content);
                if !complete_entry && !next_trimmed.is_empty() && sprite3d_name(next).is_some() {
                    break;
                }
                if complete_entry
                    && sprite3d_name(next).is_some()
                    && next_sprite3d_palette_line(context, next_index, block)
                {
                    break;
                }
                end = line_end(next);
                if !next_trimmed.is_empty() {
                    body_end = line_end(next);
                    if is_sprite_color_row(next_trimmed) {
                        saw_palette_row = true;
                    }
                }
                if !complete_entry && saw_palette_row {
                    break;
                }
            }
            if cursor < start || cursor > end {
                continue;
            }
            return Some(SourceTarget {
                kind: SourceTargetKind::Sprite3d,
                name,
                start,
                end,
                body_start: Some(body_start),
                body_end: Some(body_end),
                level_index: None,
                sound_kind: None,
                params: Vec::new(),
                source_sprite: None,
            });
        }
    }
    None
}

fn sprite3d_blocks(source: &str, context: &SourceContext) -> Vec<Sprite3dBlock> {
    context
        .lines
        .iter()
        .filter(|line| line.tokens.first().is_some_and(|token| token == "sprites3"))
        .filter_map(|line| {
            let header_end = line_end(line);
            let open_index = source[line.start..header_end]
                .find('{')
                .map(|offset| line.start + offset)?;
            let close_index = find_matching_brace(source, open_index)?;
            Some(Sprite3dBlock {
                open_index,
                close_index,
            })
        })
        .collect()
}

fn sprite3d_name(line: &SourceContextLine) -> Option<String> {
    let [name] = line.tokens.as_slice() else {
        return None;
    };
    if sprite_definition_name_token(name) {
        return Some(clean_name_token(name));
    }
    None
}

fn next_sprite3d_palette_line(
    context: &SourceContext,
    start_index: usize,
    block: Sprite3dBlock,
) -> bool {
    context
        .lines
        .iter()
        .skip(start_index + 1)
        .take_while(|line| line.start < block.close_index)
        .find_map(|line| {
            let trimmed = code_trim(&line.content);
            (!trimmed.is_empty())
                .then(|| is_sprite_entry_start_color_row(trimmed, &VisualSpriteRefs::default()))
        })
        .unwrap_or(false)
}

fn sprite_header_scope(scope: Option<SourceScope>) -> bool {
    matches!(scope, Some(SourceScope::Visuals))
}

#[derive(Default)]
struct VisualSpriteRefs {
    color_names: HashSet<String>,
    shape_names: HashSet<String>,
    color_assets: HashMap<String, String>,
    shape_assets: HashMap<String, Vec<String>>,
}

impl VisualSpriteRefs {
    fn contains(&self, value: &str) -> bool {
        self.color_names.contains(value)
    }

    fn contains_shape(&self, value: &str) -> bool {
        self.shape_names.contains(value)
    }
}

fn collect_visual_sprite_refs(source: &str, context: &SourceContext) -> VisualSpriteRefs {
    let mut refs = VisualSpriteRefs::default();
    for line in &context.lines {
        let Some(kind) = line.tokens.first().map(String::as_str) else {
            continue;
        };
        if !matches!(
            (kind, line.scope),
            (
                "colors",
                Some(SourceScope::Visuals | SourceScope::VisualColorTable)
            ) | (
                "shapes",
                Some(SourceScope::Visuals | SourceScope::VisualShapeTable)
            )
        ) {
            continue;
        }
        let line_end = line_end(line);
        let Some(open_index) = source[line.start..line_end]
            .find('{')
            .map(|offset| line.start + offset)
        else {
            continue;
        };
        let Some(close_index) = find_matching_brace(source, open_index) else {
            continue;
        };
        match kind {
            "colors" => {
                collect_visual_flat_asset_names(context, open_index, close_index, &mut refs)
            }
            "shapes" => collect_visual_shape_names(context, open_index, close_index, &mut refs),
            _ => {}
        }
    }
    refs
}

fn collect_visual_flat_asset_names(
    context: &SourceContext,
    open_index: usize,
    close_index: usize,
    refs: &mut VisualSpriteRefs,
) {
    for line in &context.lines {
        if line.start <= open_index || line.start >= close_index {
            continue;
        }
        if visual_asset_depth_at_line(context, open_index, line.start) != 0 {
            continue;
        }
        match line.tokens.as_slice() {
            [name, equals, ..] if equals == "=" && is_identifier_token(name) => {
                refs.color_names.insert(name.clone());
                if let Some(color) = visual_color_assignment_value(&line.content) {
                    refs.color_assets.insert(name.clone(), color);
                }
            }
            [name]
                if line.scope == Some(SourceScope::VisualColorTable)
                    && is_identifier_token(name) =>
            {
                refs.color_names.insert(name.clone());
                if let Some(color) = visual_color_assignment_value(&line.content) {
                    refs.color_assets.insert(name.clone(), color);
                }
            }
            _ => {}
        }
    }
}

fn collect_visual_shape_names(
    context: &SourceContext,
    open_index: usize,
    close_index: usize,
    refs: &mut VisualSpriteRefs,
) {
    for line in &context.lines {
        if line.start <= open_index || line.start >= close_index {
            continue;
        }
        if visual_asset_depth_at_line(context, open_index, line.start) != 0 {
            continue;
        }
        let Some(first) = line.tokens.first() else {
            continue;
        };
        if first == "shape" {
            if let Some(name) = line.tokens.get(1) {
                refs.shape_names.insert(name.clone());
                if let Some(rows) = visual_plain_shape_rows(context, line, close_index) {
                    refs.shape_assets.insert(name.clone(), rows);
                }
            }
        } else if sprite_definition_name_token(first) {
            refs.shape_names.insert(first.clone());
            if let Some(rows) = visual_plain_shape_rows(context, line, close_index) {
                refs.shape_assets.insert(first.clone(), rows);
            }
        }
    }
}

fn visual_color_assignment_value(line: &str) -> Option<String> {
    let (_, value) = strip_line_comment(line).split_once('=')?;
    let value = value.trim().split_whitespace().next()?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn visual_plain_shape_rows(
    context: &SourceContext,
    header: &SourceContextLine,
    close_index: usize,
) -> Option<Vec<String>> {
    let mut rows = Vec::new();
    for line in context
        .lines
        .iter()
        .filter(|line| line.start > header.start && line.start < close_index)
    {
        let row = code_trim(&line.content);
        if row.is_empty() {
            if rows.is_empty() {
                continue;
            }
            break;
        }
        if row == "}" {
            break;
        }
        if !rows.is_empty() && sprite_definition_name_token(row) {
            break;
        }
        rows.push(row.to_string());
    }
    (!rows.is_empty()).then_some(rows)
}

fn visual_asset_depth_at_line(
    context: &SourceContext,
    open_index: usize,
    line_start: usize,
) -> usize {
    let mut depth = 0usize;
    for line in &context.lines {
        if line.start <= open_index || line.start >= line_start {
            continue;
        }
        let trimmed = code_trim(&line.content);
        if trimmed == "}" {
            depth = depth.saturating_sub(1);
        }
        if trimmed.ends_with('{') {
            depth += 1;
        }
    }
    depth
}

fn sprite_name(line: &SourceContextLine) -> Option<String> {
    match line.tokens.as_slice() {
        [first, ..]
            if first != "}"
                && first != "{"
                && !first.contains('=')
                && sprite_definition_name_token(first)
                && (line.content.trim_end().ends_with('{') || is_unbraced_sprite_header(line)) =>
        {
            Some(clean_name_token(first))
        }
        _ => None,
    }
}

fn line_style_sprite_header(
    line: &SourceContextLine,
    visual_refs: &VisualSpriteRefs,
) -> Option<(String, usize)> {
    if !matches!(line.scope, Some(SourceScope::Visuals)) {
        return None;
    }
    let [selector, source] = line.tokens.as_slice() else {
        return None;
    };
    if !sprite_definition_name_token(selector)
        || !(is_visual_image_source(source)
            || is_sprite_entry_start_color_token(source, visual_refs))
    {
        return None;
    }
    let body_start = line
        .token_spans
        .get(1)
        .map(|token| token.start)
        .unwrap_or_else(|| line_end(line));
    Some((clean_name_token(selector), body_start))
}

fn clean_name_token(value: &str) -> String {
    value
        .trim_matches(|ch: char| matches!(ch, '{' | '}' | '"' | '\''))
        .to_string()
}

fn unbraced_sprite_range(
    context: &SourceContext,
    start_index: usize,
    visual_refs: &VisualSpriteRefs,
) -> Option<(usize, usize, usize)> {
    let line = &context.lines[start_index];
    if !is_unbraced_sprite_header(line) {
        return None;
    }
    let body_start = line_end(line);
    let mut end = body_start;
    let mut body_end = body_start;
    let mut saw_color_row = false;
    for (next_index, next) in context.lines.iter().enumerate().skip(start_index + 1) {
        if next.scope != Some(SourceScope::Visuals) {
            break;
        }
        let trimmed = code_trim(&next.content);
        if trimmed == "}" {
            break;
        }
        if !trimmed.is_empty()
            && starts_next_sprite_entry(context, next_index, saw_color_row, visual_refs)
        {
            break;
        }
        end = line_end(next);
        if !trimmed.is_empty() {
            body_end = line_end(next);
            if is_sprite_entry_start_color_row(trimmed, visual_refs) {
                saw_color_row = true;
            }
        }
    }
    Some((end, body_start, body_end))
}

fn is_unbraced_sprite_header(line: &SourceContextLine) -> bool {
    matches!(line.scope, Some(SourceScope::Visuals))
        && matches!(line.tokens.as_slice(), [name] if sprite_definition_name_token(name))
        && !line.content.trim_end().ends_with('{')
}

fn is_visual_sprite_entry_boundary<'a>(
    line: &SourceContextLine,
    following: impl Iterator<Item = &'a SourceContextLine>,
    current_saw_color_row: bool,
    visual_refs: &VisualSpriteRefs,
) -> bool {
    if !matches!(line.scope, Some(SourceScope::Visuals)) {
        return false;
    }
    match line.tokens.as_slice() {
        [keyword, ..]
            if (keyword == "colors" || keyword == "shapes")
                && line.content.trim_end().ends_with('{') =>
        {
            true
        }
        [selector, source]
            if sprite_definition_name_token(selector)
                && current_saw_color_row
                && (is_visual_image_source(source)
                    || is_sprite_entry_start_color_token(source, visual_refs)) =>
        {
            true
        }
        [selector] if current_saw_color_row && sprite_definition_name_token(selector) => following
            .skip(1)
            .find(|next| {
                next.scope == Some(SourceScope::Visuals) && !code_trim(&next.content).is_empty()
            })
            .is_some_and(|next| {
                let next_trimmed = code_trim(&next.content);
                is_visual_image_source(next_trimmed) || is_sprite_color_row(next_trimmed)
            }),
        _ => false,
    }
}

fn starts_next_sprite_entry(
    context: &SourceContext,
    line_index: usize,
    current_saw_color_row: bool,
    visual_refs: &VisualSpriteRefs,
) -> bool {
    let Some(line) = context.lines.get(line_index) else {
        return false;
    };
    if !matches!(line.scope, Some(SourceScope::Visuals)) {
        return false;
    }
    if is_visual_sprite_entry_boundary(
        line,
        context.lines.iter().skip(line_index),
        current_saw_color_row,
        visual_refs,
    ) {
        return true;
    }
    if is_sprite_entry_start_color_row(code_trim(&line.content), visual_refs) {
        return false;
    }
    !current_saw_color_row && sprite_name(line).is_some()
}

fn sprite_definition_name_token(value: &str) -> bool {
    if matches!(
        value,
        "shape" | "shapes" | "colors" | "ascii" | "sprites" | "sprites3"
    ) {
        return false;
    }
    let cleaned = value.trim_start_matches('@');
    let Some(first) = cleaned.chars().next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && cleaned
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':'))
}

fn is_sprite_color_row(line: &str) -> bool {
    let colors = sprite_color_row_tokens(line);
    !colors.is_empty() && colors.iter().all(|color| is_sprite_color_expr(color))
}

fn is_sprite_entry_start_color_row(line: &str, visual_refs: &VisualSpriteRefs) -> bool {
    let colors = sprite_color_row_tokens(line);
    if colors.is_empty() || !colors.iter().all(|color| is_sprite_color_expr(color)) {
        return false;
    }
    colors.len() > 1
        || colors.first().is_some_and(|color| {
            is_sprite_color(color) || color.contains(':') || visual_refs.contains(color)
        })
}

fn sprite_color_row_tokens(line: &str) -> Vec<&str> {
    let mut tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.first() == Some(&"colors") {
        tokens.remove(0);
    }
    tokens
}

fn is_sprite_entry_start_color_token(token: &str, visual_refs: &VisualSpriteRefs) -> bool {
    is_sprite_color(token) || token.contains(':') || visual_refs.contains(token)
}

fn is_sprite_color_expr(value: &str) -> bool {
    is_sprite_color(value) || is_sprite_color_ref(value)
}

fn is_sprite_color(value: &str) -> bool {
    crate::syntax::is_visual_named_color(value) || is_hex_color(value)
}

fn is_sprite_color_ref(value: &str) -> bool {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    is_identifier_token(first)
        && parts.all(|part| {
            !part.is_empty()
                && part.chars().all(|ch| {
                    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '+' | '*' | '(' | ')')
                })
        })
}

fn is_identifier_token(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn is_visual_image_source(value: &str) -> bool {
    let lower = value
        .trim_matches(|ch| matches!(ch, '"' | '\''))
        .to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".svg")
        || lower.ends_with(".avif")
}

fn line_end(line: &SourceContextLine) -> usize {
    line.start + line.content.len()
}

fn code_trim(line: &str) -> &str {
    strip_line_comment(line).trim()
}

fn find_matching_brace(source: &str, open_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None::<char>;
    let mut escaped = false;
    let mut in_comment = false;
    let mut iter = source[open_index..].char_indices().peekable();
    while let Some((relative, ch)) = iter.next() {
        let index = open_index + relative;
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }
        if let Some(quote_ch) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote_ch {
                quote = None;
            }
            continue;
        }
        if ch == '/' && iter.peek().is_some_and(|(_, next)| *next == '/') {
            in_comment = true;
            iter.next();
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn push_json_number(out: &mut String, key: &str, value: usize) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    out.push_str(&value.to_string());
}

fn push_json_string(out: &mut String, key: &str, value: &str) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    push_json_string_value(out, value);
}

fn push_json_string_array(out: &mut String, key: &str, values: &[String]) {
    push_json_string_value(out, key);
    out.push(':');
    push_json_string_array_value(out, values);
}

fn push_json_string_array_value(out: &mut String, values: &[String]) {
    out.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string_value(out, value);
    }
    out.push(']');
}

fn push_json_string_value(out: &mut String, value: &str) {
    out.push('"');
    escape_json_string(out, value);
    out.push('"');
}

fn escape_json_string(out: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SoundSourceTargetKind, SourceSpritePaletteEntry, SourceTargetKind, resolve_source_target,
    };

    #[test]
    fn resolves_level_body_to_named_level() {
        let source = r#"
levels microban of sokoban {
level "microban_01"
#####
#@$.#
#####

level "microban_02"
#####
#.@ #
#####
}
"#;
        let cursor = source.find("#@$.#").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Level);
        assert_eq!(target.name, "microban_01");
        assert_eq!(target.level_index, Some(0));
    }

    #[test]
    fn resolves_levels3_body_to_3d_level() {
        let source = r#"
levels3 basic of push3d {
level "push3d_01" {
___
_P_
}
}
"#;
        let cursor = source.find("_P_").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Level3d);
        assert_eq!(target.name, "push3d_01");
        assert_eq!(target.level_index, Some(0));
        assert!(
            target
                .params
                .iter()
                .any(|param| param == &("bundle".to_string(), "basic".to_string()))
        );
    }

    #[test]
    fn resolves_sound_definition_with_params() {
        let source = r#"
sounds {
sfx clear seed=clear01 type=jump
music music_name seed=test1 bars=8 height=0 bpm=100
}
"#;
        let cursor = source.find("height=0").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sounds);
        assert_eq!(target.name, "music_name");
        assert_eq!(target.sound_kind, Some(SoundSourceTargetKind::Music));
        assert!(
            target
                .params
                .iter()
                .any(|param| param == &("bpm".to_string(), "100".to_string()))
        );
    }

    #[test]
    fn resolves_nested_sound_definition_with_params() {
        let source = r#"
puzzle board {
sounds {
sfx clear seed=clear01 type=jump
}
}
"#;
        let cursor = source.find("type=jump").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sounds);
        assert_eq!(target.name, "clear");
        assert_eq!(target.sound_kind, Some(SoundSourceTargetKind::Sfx));
        assert!(
            target
                .params
                .iter()
                .any(|param| param == &("seed".to_string(), "clear01".to_string()))
        );
    }

    #[test]
    fn resolves_sprite_entry_body() {
        let source = r#"
sprites {
Player {
#000 #fff
.0.
111
}
}
"#;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        assert!(target.body_start.unwrap() < cursor);
        assert!(target.body_end.unwrap() > cursor);
    }

    #[test]
    fn resolves_unfinished_sprite_name_as_sprite_target() {
        let source = r#"
sprites {
Player
}
"#;
        let cursor = source.find("Player").unwrap() + "Player".len();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        assert_eq!(target.body_start, target.body_end);
    }

    #[test]
    fn resolves_unfinished_singular_sprite_name_as_sprite_target() {
        let source = r#"
sprite {
Player
}
"#;
        let cursor = source.find("Player").unwrap() + "Player".len();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        assert_eq!(target.body_start, target.body_end);
    }

    #[test]
    fn resolves_unfinished_sprite_body_as_sprite_target() {
        let source = r##"
sprites {
Player
#f
}
"##;
        let cursor = source.find("#f").unwrap() + 1;
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#f"));
    }

    #[test]
    fn resolves_line_style_sprite_named_like_color() {
        let source = r##"
sprites {
red #f00
}
"##;
        let cursor = source.find("#f00").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "red");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert_eq!(body.trim(), "#f00");
    }

    #[test]
    fn resolves_split_sprite_named_like_color_after_ascii_body() {
        let source = r##"
sprites {
Player
#000
0
red
#f00
}
"##;
        let cursor = source.find("#f00").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "red");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#f00"));
        assert!(!body.contains("#000"));
    }

    #[test]
    fn color_name_row_stays_in_current_sprite_target() {
        let source = r##"
sprites {
Player
red blue
01
}
"##;
        let cursor = source.find("red blue").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("red blue"));
        assert!(body.contains("01"));
    }

    #[test]
    fn user_named_color_row_stays_in_current_sprite_target() {
        let source = r##"
sprites {
colors {
accent = #e94f64
}
Player
accent
0
Box
#222
}
"##;
        let cursor = source.find("accent\n0").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("accent"));
        assert!(body.contains("0"));
        assert!(!body.contains("Box"));
    }

    #[test]
    fn tagged_sprite_color_name_row_stays_in_current_sprite_target() {
        let source = r##"
sprites {
colors {
GoalCount = #acacac
}
GoalCount:5
GoalCount
....................
..........00........
}
"##;
        let cursor = source.find("GoalCount\n....................").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "GoalCount:5");
        let source_sprite = target
            .source_sprite
            .as_ref()
            .expect("source sprite contract");
        assert_eq!(source_sprite.palette_tokens, vec!["GoalCount".to_string()]);
        assert_eq!(
            source_sprite.pixel_rows,
            vec![
                "....................".to_string(),
                "..........00........".to_string()
            ]
        );
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("GoalCount"));
        assert!(body.contains("..........00........"));
    }

    #[test]
    fn sprite_source_contract_resolves_palette_from_parser_visuals() {
        let source = r##"
puzzle main {
layers {
Player
}

rules {
}
}

levels main of main {
legend {
. = empty
P = Player
}
level "one"
P
}

sprites {
colors {
accent = #e94f64
}
Player
accent
0
}
"##;
        let cursor = source.find("accent\n0").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();
        let source_sprite = target
            .source_sprite
            .as_ref()
            .expect("source sprite contract");

        assert_eq!(target.name, "Player");
        assert_eq!(source_sprite.palette_tokens, vec!["accent".to_string()]);
        assert_eq!(
            source_sprite.resolved_palette,
            vec![SourceSpritePaletteEntry {
                source: "accent".to_string(),
                color: "#e94f64".to_string(),
                linked: true,
            }]
        );
    }

    #[test]
    fn consecutive_tagged_sprite_color_name_rows_do_not_become_sprite_headers() {
        let source = r##"
sprites {
colors {
GoalCount = #acacac
}
GoalCount:1
GoalCount
....................
..........00........
GoalCount:2
GoalCount
....................
........00..00......
}
"##;
        let cursor = source
            .find("GoalCount:2\nGoalCount\n....................")
            .unwrap()
            + "GoalCount:2\n".len();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "GoalCount:2");
        let source_sprite = target
            .source_sprite
            .as_ref()
            .expect("source sprite contract");
        assert_eq!(source_sprite.palette_tokens, vec!["GoalCount".to_string()]);
        assert_eq!(
            source_sprite.pixel_rows,
            vec![
                "....................".to_string(),
                "........00..00......".to_string()
            ]
        );
    }

    #[test]
    fn line_style_sprite_accepts_user_named_color() {
        let source = r##"
sprites {
colors {
accent = #e94f64
}
Player accent
}
"##;
        let cursor = source.rfind("accent").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert_eq!(body.trim(), "accent");
    }

    #[test]
    fn unfinished_unbraced_sprite_stops_before_next_entry_header() {
        let source = r#"
sprites {
Player
Box
}
"#;
        let cursor = source.find("Box").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Box");
    }

    #[test]
    fn resolves_unbraced_at_sprite_before_next_at_sprite() {
        let source = r##"
sprites {
@Floor
#cfcfcf #c5c5c5
00000
00100

@Shade
#0002
.....
}
"##;
        let cursor = source.find("00100").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "@Floor");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#cfcfcf"));
        assert!(!body.contains("@Shade"));
    }

    #[test]
    fn resolves_unbraced_sprite_before_line_style_image_sprite() {
        let source = r##"
sprites {
Player
#000 #fff
.0.
111
Box sprites/box.png
}
"##;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("111"));
        assert!(!body.contains("Box sprites/box.png"));
    }

    #[test]
    fn resolves_unbraced_schema_sprite_with_color_alias_row() {
        let source = r##"
sprites {
Gate:num
Gate_color_1 Gate_color_2
.......
.00000.
.00000.
.00000.
.00000.
.00000.
.......
}
"##;
        let cursor = source.find(".00000.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Gate:num");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("Gate_color_1 Gate_color_2"));
        assert!(body.contains(".00000."));
    }

    #[test]
    fn resolves_unbraced_variant_sprite_with_color_alias_row() {
        let source = r##"
sprites {
Gate:1
Gate_color_1 Gate_color_2
.......
.00000.
.00000.
.00000.
.00000.
.00000.
.......
}
"##;
        let cursor = source.find(".00000.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Gate:1");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("Gate_color_1 Gate_color_2"));
        assert!(body.contains(".00000."));
    }

    #[test]
    fn resolves_unbraced_tagged_sprite_after_tagged_sprite() {
        let source = r##"
sprites {
Box:base
#aaa
0
Box:movable
#bbb
0
}
"##;
        let cursor = source.rfind("#bbb").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Box:movable");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#bbb"));
        assert!(!body.contains("#aaa"));
    }

    #[test]
    fn resolves_unbraced_sprite_before_split_line_image_sprite() {
        let source = r##"
sprites {
Player
#000 #fff
.0.
111
Box
sprites/box.png
}
"##;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("111"));
        assert!(!body.contains("Box"));
        assert!(!body.contains("sprites/box.png"));
    }

    #[test]
    fn resolves_unbraced_sprite_with_colors_keyword_palette() {
        let source = r##"
sprites {
Player
colors #000 #fff
.0.
111
}
"##;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("colors #000 #fff"));
        assert!(body.contains("111"));
    }

    #[test]
    fn shape_reference_line_resolves_enclosing_unbraced_sprite() {
        let source = r##"
sprites {
shapes {
box_shape
010
111
010
}
Box
#111 #eee
shape box_shape
Next
#222
0
}
"##;
        let cursor = source.find("shape box_shape").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Box");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("shape box_shape"));
        assert!(!body.contains("Next"));
    }

    #[test]
    fn rotated_unbraced_sprite_prelude_stays_in_current_sprite_target() {
        let source = r##"
sprites {
@LockedFrame:directions
rotate from up
#000000
....................
.000..000..000..000.
}
"##;
        let cursor = source.find(".000..000").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "@LockedFrame:directions");
        let source_sprite = target
            .source_sprite
            .as_ref()
            .expect("source sprite contract");
        assert_eq!(
            source_sprite.prelude_rows,
            vec!["rotate from up".to_string()]
        );
        assert_eq!(source_sprite.palette_tokens, vec!["#000000".to_string()]);
        assert_eq!(
            source_sprite.pixel_rows,
            vec![
                "....................".to_string(),
                ".000..000..000..000.".to_string()
            ]
        );
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("rotate from up"));
        assert!(body.contains(".000..000..000..000."));
    }

    #[test]
    fn shape_table_rows_do_not_resolve_as_sprite_targets() {
        let source = r##"
sprites {
shapes {
mark:kind {
A {
010
111
}
}
}
Box
#111
0
}
"##;
        let cursor = source.find("010").unwrap();

        assert_eq!(resolve_source_target(source, cursor), None);
    }

    #[test]
    fn unbraced_sprite_target_end_stops_before_next_colors_keyword_sprite() {
        let source = r##"
sprites {
Player
#000 #fff
.0.
111
Box
colors #222
0
}
"##;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        let target_source = &source[target.start..target.end];
        assert!(target_source.contains("111"));
        assert!(!target_source.contains("Box"));
        assert!(!target_source.contains("colors #222"));
    }

    #[test]
    fn resolves_sprites3_entry_as_sprite3d() {
        let source = r##"
sprites3 basic of push3d {
Floor
#90ee90 #008000
.....
..0..

11111

Goal
#00008b
.....
.000.
}
"##;
        let cursor = source.find("..0..").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite3d);
        assert_eq!(target.name, "Floor");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#90ee90"));
        assert!(body.contains("11111"));
        assert!(!body.contains("Goal"));
    }

    #[test]
    fn resolves_second_sprites3_entry_as_sprite3d() {
        let source = r##"
sprites3 basic {
Floor
#90ee90
.....

Goal
#00008b
.000.
}
"##;
        let cursor = source.find(".000.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite3d);
        assert_eq!(target.name, "Goal");
    }

    #[test]
    fn resolves_unfinished_sprite3d_name_as_sprite3d_target() {
        let source = r#"
sprites3 basic {
Floor
}
"#;
        let cursor = source.find("Floor").unwrap() + "Floor".len();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite3d);
        assert_eq!(target.name, "Floor");
        assert_eq!(target.body_start, target.body_end);
    }

    #[test]
    fn unfinished_sprite3d_stops_before_next_entry_header() {
        let source = r#"
sprites3 basic {
Floor
Goal
}
"#;
        let cursor = source.find("Goal").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite3d);
        assert_eq!(target.name, "Goal");
    }
}
