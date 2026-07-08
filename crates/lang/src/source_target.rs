use crate::puzzle3_parse::{
    is_canonical_sprite_palette_line, parse_canonical_sprite_palette_line, parse_sprite_voxels,
};
use crate::source::{SourceScope, split_header_tokens, strip_line_comment};
use crate::surface::{SurfaceDocument, SurfaceLine, SurfaceVisualSpriteRefs};
use crate::{SpriteColor3, SpriteVoxels3};
use std::collections::HashSet;

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
    pub source_sprite3d: Option<SourceSprite3dTarget>,
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceSprite3dTarget {
    pub status: SourceSprite3dStatus,
    pub palette_tokens: Vec<String>,
    pub palette: Vec<String>,
    pub rows: Vec<String>,
    pub size: Option<usize>,
    pub cells: Vec<Option<usize>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SourceSprite3dStatus {
    Complete,
    #[default]
    Incomplete,
    Invalid,
}

impl SourceSprite3dStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Incomplete => "incomplete",
            Self::Invalid => "invalid",
        }
    }
}

pub fn resolve_source_target(source: &str, cursor_offset: usize) -> Option<SourceTarget> {
    let cursor = cursor_offset.min(source.len());
    let document = crate::parse_surface_source_target_document(source);
    resolve_source_target_from_document(source, &document, cursor)
}

pub fn source_entries_json(source: &str) -> String {
    let document = crate::parse_surface_source_target_document(source);
    let entries = resolve_source_entries_from_document(source, &document);
    source_entries_json_from_entries(&entries)
}

pub(crate) fn resolve_source_target_from_document(
    source: &str,
    document: &SurfaceDocument,
    cursor: usize,
) -> Option<SourceTarget> {
    let mut target = resolve_source_entries_from_document(source, document)
        .into_iter()
        .find(|entry| cursor >= entry.start && cursor <= entry.end)?;
    if target.kind == SourceTargetKind::Sprite {
        target.source_sprite = source_sprite_for_target(source, document, &target);
    }
    Some(target)
}

fn resolve_source_entries_from_document(
    source: &str,
    document: &SurfaceDocument,
) -> Vec<SourceTarget> {
    let mut entries = Vec::new();
    entries.extend(resolve_sound_entries(document));
    entries.extend(resolve_level3d_entries(source, document));
    entries.extend(resolve_level_entries(source, document));
    entries.extend(resolve_sprite3d_entries(source, document));
    entries.extend(resolve_sprite_entries(
        source,
        document,
        &document.visual_sprite_refs,
    ));
    entries.sort_by_key(|entry| entry.start);
    entries
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

fn source_entries_json_from_entries(entries: &[SourceTarget]) -> String {
    let mut out = String::from("{\"entries\":[");
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_target_json(&mut out, entry);
    }
    out.push_str("]}");
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
    if let Some(sprite3d) = &target.source_sprite3d {
        out.push_str(",\"sourceSprite3d\":");
        push_source_sprite3d_json(out, sprite3d);
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

fn push_source_sprite3d_json(out: &mut String, sprite: &SourceSprite3dTarget) {
    out.push('{');
    push_json_string(out, "status", sprite.status.as_str());
    out.push(',');
    push_json_string_array(out, "paletteTokens", &sprite.palette_tokens);
    out.push(',');
    push_json_string_array(out, "palette", &sprite.palette);
    out.push(',');
    push_json_string_array(out, "rows", &sprite.rows);
    out.push_str(",\"size\":");
    match sprite.size {
        Some(size) => out.push_str(&size.to_string()),
        None => out.push_str("null"),
    }
    out.push_str(",\"cells\":");
    push_source_sprite3d_cells_json(out, &sprite.cells);
    out.push('}');
}

fn push_source_sprite3d_cells_json(out: &mut String, cells: &[Option<usize>]) {
    out.push('[');
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        match cell {
            Some(value) => out.push_str(&value.to_string()),
            None => out.push_str("null"),
        }
    }
    out.push(']');
}

fn resolve_sound_entries(context: &SurfaceDocument) -> Vec<SourceTarget> {
    context
        .lines
        .iter()
        .filter(|line| line.scope == Some(SourceScope::Sounds))
        .filter_map(|line| {
            let (sound_kind, name, params) = parse_sound_definition(line)?;
            Some(SourceTarget {
                kind: SourceTargetKind::Sounds,
                name,
                start: line.start,
                end: line_end(line),
                body_start: None,
                body_end: None,
                level_index: None,
                sound_kind: Some(sound_kind),
                params,
                source_sprite: None,
                source_sprite3d: None,
            })
        })
        .collect()
}

fn parse_sound_definition(
    line: &SurfaceLine,
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

fn resolve_level_entries(source: &str, context: &SurfaceDocument) -> Vec<SourceTarget> {
    let level3d_blocks = level3d_blocks(source, context);
    let mut entries = Vec::new();
    let mut level_index = 0usize;
    let mut index = 0usize;
    while index < context.lines.len() {
        let line = &context.lines[index];
        if level3d_blocks
            .iter()
            .any(|block| line.start > block.open_index && line.start < block.close_index)
        {
            index += 1;
            continue;
        }
        let target = if let Some(name) = level_name(line) {
            let (end, body_start, body_end) = level_range(source, context, index);
            Some((name, line.start, end, body_start, body_end))
        } else if is_unnamed_level_start(line) {
            let (end, body_start, body_end) = unnamed_level_range(source, context, index);
            Some((String::new(), line.start, end, body_start, body_end))
        } else {
            None
        };
        let Some((name, start, end, body_start, body_end)) = target else {
            index += 1;
            continue;
        };
        entries.push(SourceTarget {
            kind: SourceTargetKind::Level,
            name,
            start,
            end,
            body_start: Some(body_start),
            body_end: Some(body_end),
            level_index: Some(level_index),
            sound_kind: None,
            params: Vec::new(),
            source_sprite: None,
            source_sprite3d: None,
        });
        level_index += 1;
        index = next_level_scan_index(context, index, end);
    }
    entries
}

fn is_unnamed_level_start(line: &SurfaceLine) -> bool {
    if line.scope != Some(SourceScope::Levels) {
        return false;
    }
    !matches!(
        line.tokens.first().map(String::as_str),
        None | Some("legend" | "level" | "levels" | "levels3" | "}")
    )
}

fn unnamed_level_range(
    source: &str,
    context: &SurfaceDocument,
    start_index: usize,
) -> (usize, usize, usize) {
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

    let body_start = line.start;
    let mut end = header_end;
    let mut body_end = if code_trim(&line.content).is_empty() {
        body_start
    } else {
        header_end
    };
    for next in context.lines.iter().skip(start_index + 1) {
        let next_trimmed = code_trim(&next.content);
        let in_level = matches!(
            next.scope,
            Some(SourceScope::Level | SourceScope::UnbracedLevel | SourceScope::Legend)
        );
        if level_name(next).is_some() || is_unnamed_level_start(next) {
            break;
        }
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

fn next_level_scan_index(context: &SurfaceDocument, start_index: usize, end: usize) -> usize {
    context
        .lines
        .iter()
        .enumerate()
        .skip(start_index + 1)
        .find_map(|(index, line)| (line.start > end).then_some(index))
        .unwrap_or(context.lines.len())
}

fn resolve_level3d_entries(source: &str, context: &SurfaceDocument) -> Vec<SourceTarget> {
    let mut entries = Vec::new();
    let mut level_index = 0usize;
    for block in level3d_blocks(source, context) {
        let bundle = block.bundle.clone();
        let model = block.model.clone();
        for (index, line) in context.lines.iter().enumerate() {
            if line.start <= block.open_index || line.start >= block.close_index {
                continue;
            }
            let Some(name) = level_name(line) else {
                continue;
            };
            let (end, body_start, body_end) = level_range(source, context, index);
            entries.push(SourceTarget {
                kind: SourceTargetKind::Level3d,
                name,
                start: line.start,
                end,
                body_start: Some(body_start),
                body_end: Some(body_end),
                level_index: Some(level_index),
                sound_kind: None,
                params: vec![
                    ("bundle".to_string(), bundle.clone()),
                    ("model".to_string(), model.clone()),
                ],
                source_sprite: None,
                source_sprite3d: None,
            });
            level_index += 1;
        }
    }
    entries
}

fn level_name(line: &SurfaceLine) -> Option<String> {
    match line.tokens.as_slice() {
        [keyword, name, ..] if keyword == "level" => Some(clean_name_token(name)),
        _ => None,
    }
}

fn level_range(
    source: &str,
    context: &SurfaceDocument,
    start_index: usize,
) -> (usize, usize, usize) {
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

fn resolve_sprite_entries(
    source: &str,
    context: &SurfaceDocument,
    visual_refs: &SurfaceVisualSpriteRefs,
) -> Vec<SourceTarget> {
    let sprite3d_blocks = sprite3d_blocks(source, context);
    let visual_shape_blocks = visual_shape_table_blocks(source, context);
    let mut entries = Vec::new();
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
        if visual_shape_blocks
            .iter()
            .any(|block| line.start > block.open_index && line.start < block.close_index)
        {
            continue;
        }
        let line_end = line_end(line);
        if let Some((name, body_start)) = line_style_sprite_header(line, visual_refs) {
            entries.push(SourceTarget {
                kind: SourceTargetKind::Sprite,
                name,
                start: line.start,
                end: line_end,
                body_start: Some(body_start),
                body_end: Some(line_end),
                level_index: None,
                sound_kind: None,
                params: Vec::new(),
                source_sprite: None,
                source_sprite3d: None,
            });
            covered_until = line_end;
            continue;
        }
        let Some(name) = sprite_name(line) else {
            continue;
        };
        if let Some(open_index) = source[line.start..line_end]
            .find('{')
            .map(|offset| line.start + offset)
        {
            if let Some(end) = find_matching_brace(source, open_index).map(|index| index + 1) {
                entries.push(SourceTarget {
                    kind: SourceTargetKind::Sprite,
                    name,
                    start: line.start,
                    end,
                    body_start: Some(open_index + 1),
                    body_end: Some(end.saturating_sub(1)),
                    level_index: None,
                    sound_kind: None,
                    params: Vec::new(),
                    source_sprite: None,
                    source_sprite3d: None,
                });
                covered_until = end;
            }
            continue;
        }
        let Some((end, body_start, body_end)) = unbraced_sprite_range(context, index, visual_refs)
        else {
            continue;
        };
        entries.push(SourceTarget {
            kind: SourceTargetKind::Sprite,
            name,
            start: line.start,
            end,
            body_start: Some(body_start),
            body_end: Some(body_end),
            level_index: None,
            sound_kind: None,
            params: Vec::new(),
            source_sprite: None,
            source_sprite3d: None,
        });
        covered_until = end;
    }
    entries
}

fn source_sprite_for_target(
    source: &str,
    document: &SurfaceDocument,
    target: &SourceTarget,
) -> Option<SourceSpriteTarget> {
    let body_start = target.body_start?;
    let body_end = target.body_end?;
    source_sprite_target(
        source,
        &target.name,
        body_start,
        body_end,
        &document.visual_sprite_refs,
    )
}

fn visual_shape_table_blocks(source: &str, context: &SurfaceDocument) -> Vec<Sprite3dBlock> {
    context
        .lines
        .iter()
        .filter_map(|line| {
            if !matches!(
                line.scope,
                Some(SourceScope::Visuals | SourceScope::VisualShapeTable)
            ) || !line.tokens.first().is_some_and(|token| token == "shapes")
            {
                return None;
            }
            let line_end = line_end(line);
            let open_index = source[line.start..line_end]
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

fn source_sprite_target(
    source: &str,
    target_name: &str,
    body_start: usize,
    body_end: usize,
    visual_refs: &SurfaceVisualSpriteRefs,
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
    if target.resolved_shape_rows.is_empty() {
        if let Some(shape_ref) = &target.shape_ref {
            if let Some(asset) = target
                .shape_assets
                .iter()
                .find(|asset| asset.name == *shape_ref)
            {
                target.resolved_shape_rows = asset.rows.clone();
            }
        }
    }
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

fn level3d_blocks(source: &str, context: &SurfaceDocument) -> Vec<Level3dBlock> {
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

fn resolve_sprite3d_entries(source: &str, context: &SurfaceDocument) -> Vec<SourceTarget> {
    let mut entries = Vec::new();
    for block in sprite3d_blocks(source, context) {
        for (index, line) in context.lines.iter().enumerate() {
            if line.start <= block.open_index || line.start >= block.close_index {
                continue;
            }
            let Some(name) = sprite3d_name(line) else {
                continue;
            };
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
            entries.push(SourceTarget {
                kind: SourceTargetKind::Sprite3d,
                name,
                start: line.start,
                end,
                body_start: Some(body_start),
                body_end: Some(body_end),
                level_index: None,
                sound_kind: None,
                params: Vec::new(),
                source_sprite: None,
                source_sprite3d: Some(source_sprite3d_for_range(source, body_start, body_end)),
            });
        }
    }
    entries
}

fn source_sprite3d_for_range(
    source: &str,
    body_start: usize,
    body_end: usize,
) -> SourceSprite3dTarget {
    let rows = source[body_start..body_end]
        .lines()
        .map(|line| code_trim(line).to_string())
        .collect::<Vec<_>>();
    let Some(palette_index) = rows
        .iter()
        .position(|row| !row.is_empty() && is_canonical_sprite_palette_line(row))
    else {
        return SourceSprite3dTarget {
            status: SourceSprite3dStatus::Incomplete,
            ..SourceSprite3dTarget::default()
        };
    };
    let palette_tokens = rows[palette_index]
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let Ok(palette_map) = parse_canonical_sprite_palette_line(&rows[palette_index]) else {
        return SourceSprite3dTarget {
            status: SourceSprite3dStatus::Invalid,
            palette_tokens,
            ..SourceSprite3dTarget::default()
        };
    };
    let palette = palette_map
        .values()
        .map(source_sprite3d_color_string)
        .collect::<Vec<_>>();
    let voxel_rows = rows
        .iter()
        .skip(palette_index + 1)
        .cloned()
        .collect::<Vec<_>>();
    let parse_rows = if voxel_rows.iter().any(|row| !row.is_empty()) {
        voxel_rows.clone()
    } else {
        vec!["0".to_string()]
    };
    let Ok(voxels) = parse_sprite_voxels("source sprite3d", &parse_rows, &palette_map) else {
        return SourceSprite3dTarget {
            status: SourceSprite3dStatus::Invalid,
            palette_tokens,
            palette,
            rows: voxel_rows,
            ..SourceSprite3dTarget::default()
        };
    };
    let (size, cells) = source_sprite3d_cells_from_voxels(&voxels, palette_tokens.len());
    SourceSprite3dTarget {
        status: SourceSprite3dStatus::Complete,
        palette_tokens,
        palette,
        rows: voxel_rows,
        size: Some(size),
        cells,
    }
}

fn source_sprite3d_color_string(color: &SpriteColor3) -> String {
    match color {
        SpriteColor3::Transparent => "#00000000".to_string(),
        SpriteColor3::Hex(value) => value.clone(),
    }
}

fn source_sprite3d_cells_from_voxels(
    voxels: &SpriteVoxels3,
    palette_len: usize,
) -> (usize, Vec<Option<usize>>) {
    let size = usize::from(voxels.width())
        .max(usize::from(voxels.height()))
        .max(usize::from(voxels.depth()));
    let mut cells = vec![None; size * size * size];
    let keys = SOURCE_SPRITE3D_PALETTE_KEYS
        .chars()
        .take(palette_len)
        .collect::<Vec<_>>();
    for (z, slice) in voxels.slices.iter().enumerate() {
        for (y, row) in slice.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch == '.' || ch == ' ' {
                    continue;
                }
                let Some(color_index) = keys.iter().position(|key| *key == ch) else {
                    continue;
                };
                let cell_index = (z * size + y) * size + x;
                cells[cell_index] = Some(color_index);
            }
        }
    }
    (size, cells)
}

const SOURCE_SPRITE3D_PALETTE_KEYS: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn sprite3d_blocks(source: &str, context: &SurfaceDocument) -> Vec<Sprite3dBlock> {
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

fn sprite3d_name(line: &SurfaceLine) -> Option<String> {
    let [name] = line.tokens.as_slice() else {
        return None;
    };
    if sprite_definition_name_token(name) {
        return Some(clean_name_token(name));
    }
    None
}

fn next_sprite3d_palette_line(
    context: &SurfaceDocument,
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
            (!trimmed.is_empty()).then(|| {
                is_sprite_entry_start_color_row(trimmed, &SurfaceVisualSpriteRefs::default())
            })
        })
        .unwrap_or(false)
}

fn sprite_header_scope(scope: Option<SourceScope>) -> bool {
    matches!(
        scope,
        Some(SourceScope::Visuals | SourceScope::VisualShapeEntry)
    )
}

fn sprite_name(line: &SurfaceLine) -> Option<String> {
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
    line: &SurfaceLine,
    visual_refs: &SurfaceVisualSpriteRefs,
) -> Option<(String, usize)> {
    if !matches!(
        line.scope,
        Some(SourceScope::Visuals | SourceScope::VisualShapeEntry)
    ) {
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
    context: &SurfaceDocument,
    start_index: usize,
    visual_refs: &SurfaceVisualSpriteRefs,
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
        if !matches!(
            next.scope,
            Some(SourceScope::Visuals | SourceScope::VisualShapeEntry)
        ) {
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

fn is_unbraced_sprite_header(line: &SurfaceLine) -> bool {
    matches!(
        line.scope,
        Some(SourceScope::Visuals | SourceScope::VisualShapeEntry)
    ) && matches!(line.tokens.as_slice(), [name] if sprite_definition_name_token(name))
        && !line.content.trim_end().ends_with('{')
}

fn is_visual_sprite_entry_boundary<'a>(
    line: &SurfaceLine,
    following: impl Iterator<Item = &'a SurfaceLine>,
    current_saw_color_row: bool,
    visual_refs: &SurfaceVisualSpriteRefs,
) -> bool {
    if !matches!(
        line.scope,
        Some(SourceScope::Visuals | SourceScope::VisualShapeEntry)
    ) {
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
                matches!(
                    next.scope,
                    Some(SourceScope::Visuals | SourceScope::VisualShapeEntry)
                ) && !code_trim(&next.content).is_empty()
            })
            .is_some_and(|next| {
                let next_trimmed = code_trim(&next.content);
                is_visual_image_source(next_trimmed) || is_sprite_color_row(next_trimmed)
            }),
        _ => false,
    }
}

fn starts_next_sprite_entry(
    context: &SurfaceDocument,
    line_index: usize,
    current_saw_color_row: bool,
    visual_refs: &SurfaceVisualSpriteRefs,
) -> bool {
    let Some(line) = context.lines.get(line_index) else {
        return false;
    };
    if !matches!(
        line.scope,
        Some(SourceScope::Visuals | SourceScope::VisualShapeEntry)
    ) {
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

fn is_sprite_entry_start_color_row(line: &str, visual_refs: &SurfaceVisualSpriteRefs) -> bool {
    let colors = sprite_color_row_tokens(line);
    if colors.is_empty() || !colors.iter().all(|color| is_sprite_color_expr(color)) {
        return false;
    }
    colors.len() > 1
        || colors.first().is_some_and(|color| {
            is_sprite_color(color) || color.contains(':') || visual_refs.contains_color(color)
        })
}

fn sprite_color_row_tokens(line: &str) -> Vec<&str> {
    let mut tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.first() == Some(&"colors") {
        tokens.remove(0);
    }
    tokens
}

fn is_sprite_entry_start_color_token(token: &str, visual_refs: &SurfaceVisualSpriteRefs) -> bool {
    is_sprite_color(token) || token.contains(':') || visual_refs.contains_color(token)
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

fn line_end(line: &SurfaceLine) -> usize {
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
        SoundSourceTargetKind, SourceSprite3dStatus, SourceSpritePaletteEntry, SourceTargetKind,
        resolve_source_entries_from_document, resolve_source_target,
    };

    #[test]
    fn source_target_consumes_surface_visual_refs_instead_of_collecting_assets() {
        let source = include_str!("source_target.rs");
        let forbidden_fragments = [
            ["struct ", "VisualSpriteRefs"],
            ["collect_visual", "_sprite_refs"],
            ["collect_visual", "_flat_asset_names"],
            ["collect_visual", "_shape_names"],
            ["visual_asset", "_depth_at_line"],
            ["visual_plain", "_shape_rows"],
            ["visual_color", "_assignment_value"],
        ];
        for parts in forbidden_fragments {
            let forbidden = parts.concat();
            assert!(
                !source.contains(&forbidden),
                "source_target must consume parser surface visual refs, not collect visual assets via {forbidden}"
            );
        }
    }

    #[test]
    fn source_target_consumes_surface_document_instead_of_source_scanner() {
        let source = include_str!("source_target.rs");
        let forbidden_fragments = [
            ["scan_source", "_context"],
            ["Source", "Context"],
            ["Source", "ContextLine"],
            ["resolve_source_target", "_from_context"],
        ];
        for parts in forbidden_fragments {
            let forbidden = parts.concat();
            assert!(
                !source.contains(&forbidden),
                "source_target must query parser-owned SurfaceDocument ranges, not source scanner products via {forbidden}"
            );
        }
    }

    #[test]
    fn source_entries_are_built_from_surface_document_product() {
        let source = r#"
title = source_entries

puzzle board {
layers {
Player
}
sprites {
Player {
#fff
0
}
}
levels {
level "one" {
.
}
level "two" {
.
}
}
}

puzzle3 board3 {
sprites3 {
Cube
#fff
0
}
levels3 pack of board3 {
level "three" {
0
}
}
}
"#;
        let document = crate::parse_surface_document(source);
        let entries = resolve_source_entries_from_document(source, &document);

        assert!(entries.iter().any(|entry| {
            entry.kind == SourceTargetKind::Level
                && entry.name == "one"
                && entry.level_index == Some(0)
        }));
        assert!(entries.iter().any(|entry| {
            entry.kind == SourceTargetKind::Level
                && entry.name == "two"
                && entry.level_index == Some(1)
        }));
        assert!(
            entries
                .iter()
                .any(|entry| { entry.kind == SourceTargetKind::Sprite && entry.name == "Player" })
        );
        assert!(entries.iter().any(|entry| {
            entry.kind == SourceTargetKind::Level3d
                && entry.name == "three"
                && entry.params
                    == vec![
                        ("bundle".to_string(), "pack".to_string()),
                        ("model".to_string(), "board3".to_string()),
                    ]
        }));
        assert!(
            entries
                .iter()
                .any(|entry| { entry.kind == SourceTargetKind::Sprite3d && entry.name == "Cube" })
        );
    }

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
    fn resolves_message_prefixed_unnamed_level_body() {
        let source = r#"
levels {
legend {
. = Background
P = Player
}

message "Level 1"
P

message "Level 2"
PP
}
"#;
        let cursor = source.find("Level 2").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Level);
        assert_eq!(target.name, "");
        assert_eq!(target.level_index, Some(1));
        assert!(target.start <= cursor);
        assert!(target.end >= source.find("PP").unwrap());
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
    fn source_sprite_contract_preserves_hyphenated_shape_refs() {
        let source = r##"
sprites {
shapes {
box-shape
010
111
010
}
Box
#111 #eee
shape box-shape
}
"##;
        let cursor = source.find("shape box-shape").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Box");
        let source_sprite = target
            .source_sprite
            .as_ref()
            .expect("source sprite contract");
        assert_eq!(source_sprite.shape_ref.as_deref(), Some("box-shape"));
        assert!(
            source_sprite
                .shape_assets
                .iter()
                .any(|asset| asset.name == "box-shape")
        );
        assert_eq!(
            source_sprite.resolved_shape_rows,
            vec!["010".to_string(), "111".to_string(), "010".to_string()]
        );
    }

    #[test]
    fn source_sprite_contract_preserves_tagged_shape_refs() {
        let source = r##"
sprites {
shapes {
foo:bar
010
111
010
}
Box
#111 #eee
shape foo:bar
}
"##;
        let cursor = source.find("shape foo:bar").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Box");
        let source_sprite = target
            .source_sprite
            .as_ref()
            .expect("source sprite contract");
        assert_eq!(source_sprite.shape_ref.as_deref(), Some("foo:bar"));
        assert!(
            source_sprite
                .shape_assets
                .iter()
                .any(|asset| asset.name == "foo:bar")
        );
        assert_eq!(
            source_sprite.resolved_shape_rows,
            vec!["010".to_string(), "111".to_string(), "010".to_string()]
        );
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
.....

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
        let sprite3d = target.source_sprite3d.as_ref().unwrap();
        assert_eq!(sprite3d.status, SourceSprite3dStatus::Complete);
        assert_eq!(sprite3d.size, Some(5));
        assert_eq!(sprite3d.palette, vec!["#90ee90", "#008000"]);
        assert_eq!(sprite3d.cells.len(), 125);
        assert!(sprite3d.cells.iter().any(|cell| *cell == Some(0)));
        assert!(sprite3d.cells.iter().any(|cell| *cell == Some(1)));
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
        assert_eq!(
            target.source_sprite3d.as_ref().unwrap().status,
            SourceSprite3dStatus::Incomplete
        );
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
