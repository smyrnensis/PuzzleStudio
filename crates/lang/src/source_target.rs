use crate::source::{SourceScope, split_header_tokens, strip_line_comment};
use crate::surface::{SurfaceDocument, SurfaceLine, SurfaceOptionBlock, SurfaceVisualSpriteRefs};
use crate::{PuzzleSourceProfile, SpriteColor3, SpriteVoxels3};
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;

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

#[derive(Clone, Debug, PartialEq)]
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
    pub source_sprite: Option<SourceSpriteDocument>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceSpriteDocument {
    pub dimension: SourceSpriteDimension,
    pub status: SourceSpriteStatus,
    pub prelude_rows: Vec<String>,
    pub palette_tokens: Vec<String>,
    pub resolved_palette: Vec<SourceSpritePaletteEntry>,
    pub palette: Vec<String>,
    pub pixel_rows: Vec<String>,
    pub rows: Vec<String>,
    pub duration_ms: Option<u64>,
    pub frame_duration_ms: Option<u64>,
    pub animation_frames: Vec<Vec<String>>,
    pub shape_ref: Option<String>,
    pub resolved_shape_rows: Vec<String>,
    pub color_assets: Vec<SourceSpriteColorAsset>,
    pub shape_assets: Vec<SourceSpriteShapeAsset>,
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub depth: Option<usize>,
    pub size: Option<usize>,
    pub cells: Vec<Option<usize>>,
    pub frames: Vec<Vec<Vec<Option<usize>>>>,
    pub spatial_ops: Vec<crate::VisualSpriteTransform>,
    pub spatial_ops3: Vec<crate::SpriteSpatialOp3>,
}

pub type SourceSpriteTarget = SourceSpriteDocument;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceSpriteDimension {
    #[default]
    Two,
    Three,
}

impl SourceSpriteDimension {
    fn as_str(self) -> &'static str {
        match self {
            Self::Two => "2d",
            Self::Three => "3d",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SourceSpriteStatus {
    Complete,
    #[default]
    Incomplete,
    Invalid,
}

impl SourceSpriteStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Incomplete => "incomplete",
            Self::Invalid => "invalid",
        }
    }
}

pub type SourceSprite3dStatus = SourceSpriteStatus;

pub fn resolve_source_target(source: &str, cursor_offset: usize) -> Option<SourceTarget> {
    resolve_source_target_for_profile(source, cursor_offset, PuzzleSourceProfile::Puzzle2d)
}

pub fn resolve_source_target_for_profile(
    source: &str,
    cursor_offset: usize,
    profile: PuzzleSourceProfile,
) -> Option<SourceTarget> {
    let cursor = cursor_offset.min(source.len());
    let mut document = crate::parse_surface_source_target_document(source);
    document.source_profile = Some(profile);
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
    let entries = resolve_source_entries_from_document(source, document);
    resolve_source_target_from_entries(source, document, &entries, cursor)
}

pub(crate) fn resolve_source_target_from_entries(
    source: &str,
    document: &SurfaceDocument,
    entries: &[SourceTarget],
    cursor: usize,
) -> Option<SourceTarget> {
    let mut target = entries
        .iter()
        .find(|entry| cursor >= entry.start && cursor <= entry.end)?
        .clone();
    match target.kind {
        SourceTargetKind::Sprite => {
            target.source_sprite = source_sprite_for_target(source, document, &target);
        }
        SourceTargetKind::Sprite3d => {
            target.source_sprite = source_sprite3d_for_target(source, document, &target);
        }
        _ => {}
    }
    Some(target)
}

pub(crate) fn resolve_source_entries_from_document(
    source: &str,
    document: &SurfaceDocument,
) -> Vec<SourceTarget> {
    let mut entries = Vec::new();
    entries.extend(resolve_sound_entries(source, document));
    entries.extend(resolve_level3d_entries(source, document));
    entries.extend(resolve_level_entries(source, document));
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

pub(crate) fn source_entries_json_from_entries(entries: &[SourceTarget]) -> String {
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
    out.push('}');
}

fn push_source_sprite_json(out: &mut String, sprite: &SourceSpriteTarget) {
    out.push('{');
    push_json_string(out, "dimension", sprite.dimension.as_str());
    out.push(',');
    push_json_string(out, "status", sprite.status.as_str());
    out.push(',');
    push_json_string_array(out, "preludeRows", &sprite.prelude_rows);
    out.push(',');
    push_json_string_array(out, "paletteTokens", &sprite.palette_tokens);
    out.push_str(",\"resolvedPalette\":");
    push_source_sprite_palette_json(out, &sprite.resolved_palette);
    out.push(',');
    push_json_string_array(out, "pixelRows", &sprite.pixel_rows);
    if let Some(duration_ms) = sprite.duration_ms {
        out.push(',');
        push_json_number(out, "durationMs", duration_ms as usize);
    }
    if let Some(frame_duration_ms) = sprite.frame_duration_ms {
        out.push(',');
        push_json_number(out, "frameDurationMs", frame_duration_ms as usize);
    }
    if !sprite.animation_frames.is_empty() {
        out.push_str(",\"animationFrames\":");
        push_json_string_matrix_value(out, &sprite.animation_frames);
    }
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
    out.push_str(",\"extent\":{");
    push_json_number(out, "width", sprite.width.unwrap_or(0));
    out.push(',');
    push_json_number(out, "height", sprite.height.unwrap_or(0));
    out.push(',');
    push_json_number(out, "depth", sprite.depth.unwrap_or(1));
    out.push('}');
    out.push_str(",\"frames\":");
    push_source_sprite_edit_frames_json(out, &sprite.frames);
    out.push_str(",\"spatialOps\":");
    if sprite.dimension == SourceSpriteDimension::Two {
        push_source_sprite2d_spatial_ops_json(out, &sprite.spatial_ops);
    } else {
        push_source_sprite3d_spatial_ops_json(out, &sprite.spatial_ops3);
    }
    out.push('}');
}

fn push_source_sprite2d_spatial_ops_json(out: &mut String, ops: &[crate::VisualSpriteTransform]) {
    out.push('[');
    for (index, op) in ops.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        match op {
            crate::VisualSpriteTransform::Translate { x, y, space } => write!(
                out,
                "{{\"kind\":\"translate2\",\"space\":\"{}\",\"value\":[{x},{y}]}}",
                sprite_space_name2(*space)
            )
            .unwrap(),
            crate::VisualSpriteTransform::Rotate { degrees, space } => write!(
                out,
                "{{\"kind\":\"rotate2\",\"space\":\"{}\",\"degrees\":{degrees}}}",
                sprite_space_name2(*space)
            )
            .unwrap(),
            crate::VisualSpriteTransform::Flip { enabled } => {
                write!(out, "{{\"kind\":\"flip2\",\"enabled\":{enabled}}}").unwrap()
            }
        }
    }
    out.push(']');
}

fn sprite_space_name2(space: crate::VisualSpriteSpace) -> &'static str {
    match space {
        crate::VisualSpriteSpace::World => "world",
        crate::VisualSpriteSpace::Local => "local",
    }
}

fn push_source_sprite_edit_frames_json(out: &mut String, frames: &[Vec<Vec<Option<usize>>>]) {
    out.push('[');
    for (frame_index, layers) in frames.iter().enumerate() {
        if frame_index > 0 {
            out.push(',');
        }
        out.push_str("{\"layers\":[");
        for (layer_index, cells) in layers.iter().enumerate() {
            if layer_index > 0 {
                out.push(',');
            }
            out.push_str("{\"cells\":[");
            for (cell_index, cell) in cells.iter().enumerate() {
                if cell_index > 0 {
                    out.push(',');
                }
                match cell {
                    Some(index) => out.push_str(&index.to_string()),
                    None => out.push_str("null"),
                }
            }
            out.push_str("]}");
        }
        out.push_str("]}");
    }
    out.push(']');
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

fn push_source_sprite3d_spatial_ops_json(out: &mut String, ops: &[crate::SpriteSpatialOp3]) {
    out.push('[');
    for (index, op) in ops.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        match op {
            crate::SpriteSpatialOp3::Translate { space, value } => write!(out, "{{\"kind\":\"translate3\",\"space\":\"{}\",\"value\":[{},{},{}]}}", sprite_space_name3(*space), value[0], value[1], value[2]).unwrap(),
            crate::SpriteSpatialOp3::Rotate { space, axis, degrees } => write!(out, "{{\"kind\":\"rotate3\",\"space\":\"{}\",\"axis\":[{},{},{}],\"degrees\":{degrees}}}", sprite_space_name3(*space), axis[0], axis[1], axis[2]).unwrap(),
        }
    }
    out.push(']');
}

fn sprite_space_name3(space: crate::SpriteSpace3) -> &'static str {
    match space {
        crate::SpriteSpace3::World => "world",
        crate::SpriteSpace3::Local => "local",
    }
}

fn resolve_sound_entries(source: &str, context: &SurfaceDocument) -> Vec<SourceTarget> {
    context
        .lines
        .iter()
        .filter(|line| {
            line.option_block
                == Some(SurfaceOptionBlock::Authoring(
                    crate::authoring_grammar::AuthoringKind::SoundsConfig,
                ))
        })
        .filter_map(|line| {
            let sound = parse_sound_block(source, line)?;
            Some(SourceTarget {
                kind: SourceTargetKind::Sounds,
                name: sound.name,
                start: sound.start,
                end: sound.end,
                body_start: None,
                body_end: None,
                level_index: None,
                sound_kind: Some(sound.kind),
                params: sound.params,
                source_sprite: None,
            })
        })
        .collect()
}

struct ParsedSoundBlock {
    kind: SoundSourceTargetKind,
    name: String,
    start: usize,
    end: usize,
    params: Vec<(String, String)>,
}

fn parse_sound_block(source: &str, line: &SurfaceLine) -> Option<ParsedSoundBlock> {
    let open_relative = line.content.find('{')?;
    let open = line.start + open_relative;
    let close = matching_source_brace(source, open)?;
    let start = line.start + line.content.len() - line.content.trim_start().len();
    let end = close + 1;
    let text = source.get(start..end)?;
    let header = &line.content[..open_relative];
    let authoring_kind = sound_authoring_kind(header)?;
    let node = crate::authoring_grammar::parse_authoring_node_source(text, authoring_kind).ok()?;
    let kind = match node.kind {
        crate::authoring_grammar::AuthoringKind::SfxSoundConfig => SoundSourceTargetKind::Sfx,
        crate::authoring_grammar::AuthoringKind::MusicSoundConfig => SoundSourceTargetKind::Music,
        _ => return None,
    };
    let [name] = node.header_args.as_slice() else {
        return None;
    };
    Some(ParsedSoundBlock {
        kind,
        name: name.clone(),
        start,
        end,
        params: sound_definition_params(&node),
    })
}

fn sound_definition_params(
    node: &crate::authoring_grammar::AuthoringNode,
) -> Vec<(String, String)> {
    node.definition_rows
        .iter()
        .filter_map(|row| row.single_value().map(|value| (row.key.clone(), value)))
        .map(|(key, value)| (key, trim_quotes(value).to_string()))
        .collect()
}

fn sound_authoring_kind(header: &str) -> Option<crate::authoring_grammar::AuthoringKind> {
    let tokens = split_header_tokens(header);
    let [kind, _name] = tokens.as_slice() else {
        return None;
    };
    match *kind {
        "sfx" => Some(crate::authoring_grammar::AuthoringKind::SfxSoundConfig),
        "music" => Some(crate::authoring_grammar::AuthoringKind::MusicSoundConfig),
        _ => None,
    }
}

fn matching_source_brace(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_quote = false;
    let mut escaped = false;
    for (offset, ch) in source.get(open..)?.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_quote && ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if in_quote {
            continue;
        }
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
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
        None | Some("legend" | "level" | "levels" | "}")
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
    let sprite_blocks = sprite_blocks(source, context);
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
        let kind = sprite_blocks
            .iter()
            .find(|block| line.start > block.open_index && line.start < block.close_index)
            .map(|block| block.kind.clone())
            .unwrap_or(SourceTargetKind::Sprite);
        if visual_shape_blocks
            .iter()
            .any(|block| line.start > block.open_index && line.start < block.close_index)
        {
            continue;
        }
        let line_end = line_end(line);
        if let Some((name, body_start)) = line_style_sprite_header(line, visual_refs) {
            entries.push(SourceTarget {
                kind: kind.clone(),
                name,
                start: line.start,
                end: line_end,
                body_start: Some(body_start),
                body_end: Some(line_end),
                level_index: None,
                sound_kind: None,
                params: Vec::new(),
                source_sprite: None,
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
                let name =
                    sprite_node_selector_name(source, &line.content, open_index + 1, end - 1)
                        .unwrap_or(name);
                entries.push(SourceTarget {
                    kind: kind.clone(),
                    name,
                    start: line.start,
                    end,
                    body_start: Some(open_index + 1),
                    body_end: Some(end.saturating_sub(1)),
                    level_index: None,
                    sound_kind: None,
                    params: Vec::new(),
                    source_sprite: None,
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
            kind,
            name,
            start: line.start,
            end,
            body_start: Some(body_start),
            body_end: Some(body_end),
            level_index: None,
            sound_kind: None,
            params: Vec::new(),
            source_sprite: None,
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

fn source_sprite3d_for_target(
    source: &str,
    document: &SurfaceDocument,
    target: &SourceTarget,
) -> Option<SourceSpriteTarget> {
    let body_start = target.body_start?;
    let body_end = target.body_end?;
    let body = source.get(body_start..body_end)?;
    let body_lines = body
        .lines()
        .map(|line| code_trim(line).to_string())
        .collect::<Vec<_>>();
    let syntax = crate::sprite_authoring::parse_sprite_node(None, &body_lines);
    if !syntax.issues.is_empty() {
        return Some(SourceSpriteTarget {
            dimension: SourceSpriteDimension::Three,
            status: SourceSprite3dStatus::Invalid,
            ..SourceSpriteTarget::default()
        });
    }
    let Ok(spatial_ops) = crate::puzzle3_sprite::parse_spatial_ops(&syntax) else {
        return Some(SourceSpriteTarget {
            dimension: SourceSpriteDimension::Three,
            status: SourceSprite3dStatus::Invalid,
            ..SourceSpriteTarget::default()
        });
    };
    let palette_tokens = syntax.colors.clone().unwrap_or_default();
    if palette_tokens.is_empty() {
        return Some(SourceSpriteTarget {
            dimension: SourceSpriteDimension::Three,
            status: SourceSprite3dStatus::Incomplete,
            ..SourceSpriteTarget::default()
        });
    }
    let resolved_palette =
        source_sprite_palette_from_refs(&palette_tokens, &document.visual_sprite_refs.color_assets);
    if resolved_palette.is_empty() {
        return Some(SourceSpriteTarget {
            dimension: SourceSpriteDimension::Three,
            status: SourceSprite3dStatus::Invalid,
            palette_tokens,
            ..SourceSpriteTarget::default()
        });
    }
    let palette_line = resolved_palette
        .iter()
        .map(|entry| entry.color.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let Ok(palette_map) = crate::puzzle3_sprite::parse_palette_line(&palette_line) else {
        return Some(SourceSpriteTarget {
            dimension: SourceSpriteDimension::Three,
            status: SourceSprite3dStatus::Invalid,
            palette_tokens,
            ..SourceSpriteTarget::default()
        });
    };
    let resolved = crate::sprite_authoring::resolve_sprite_shape(&syntax, |name| {
        document.visual_sprite_refs.shape_names.contains(name)
    });
    let shape_ref = match &resolved {
        crate::sprite_authoring::ResolvedSpriteShape::Reference(reference) => {
            Some(reference.clone())
        }
        _ => None,
    };
    let frames = match resolved {
        crate::sprite_authoring::ResolvedSpriteShape::Reference(reference) => {
            let rows = document.visual_sprite_refs.shape_assets.get(&reference)?;
            let mut shape_body = Vec::with_capacity(rows.len() + 2);
            shape_body.push("shape = {".to_string());
            shape_body.extend(rows.iter().cloned());
            shape_body.push("}".to_string());
            let shape_syntax = crate::sprite_authoring::parse_sprite_node(None, &shape_body);
            match shape_syntax.shape {
                Some(crate::sprite_authoring::SpriteShapeSyntax::ExplicitInline(frames)) => frames,
                _ => return None,
            }
        }
        crate::sprite_authoring::ResolvedSpriteShape::Inline(frames) => frames,
        crate::sprite_authoring::ResolvedSpriteShape::None => {
            vec![crate::sprite_authoring::SpriteFrameSyntax {
                layers: vec![crate::sprite_authoring::SpriteLayerSyntax {
                    rows: vec![crate::sprite_authoring::SpriteShapeRow {
                        text: "0".to_string(),
                        body_line: 0,
                    }],
                }],
            }]
        }
        crate::sprite_authoring::ResolvedSpriteShape::UnknownBareReference(_)
        | crate::sprite_authoring::ResolvedSpriteShape::AmbiguousBareRow(_) => {
            return Some(SourceSpriteTarget {
                dimension: SourceSpriteDimension::Three,
                status: SourceSprite3dStatus::Invalid,
                palette_tokens,
                ..SourceSpriteTarget::default()
            });
        }
    };
    let frame_layers = frames
        .iter()
        .map(|frame| {
            frame
                .layers
                .iter()
                .map(|layer| {
                    layer
                        .rows
                        .iter()
                        .map(|row| row.text.clone())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let Some(layers) = frame_layers.first() else {
        return Some(SourceSpriteTarget {
            dimension: SourceSpriteDimension::Three,
            status: SourceSprite3dStatus::Invalid,
            palette_tokens,
            ..SourceSpriteTarget::default()
        });
    };
    let mut edit_frames = Vec::with_capacity(frame_layers.len());
    let mut common_size = None;
    for frame in &frame_layers {
        let Ok(voxels) =
            crate::puzzle3_sprite::parse_voxel_layers(&target.name, frame, &palette_map)
        else {
            return Some(SourceSpriteTarget {
                dimension: SourceSpriteDimension::Three,
                status: SourceSprite3dStatus::Invalid,
                palette_tokens,
                ..SourceSpriteTarget::default()
            });
        };
        let (size, cells) = source_sprite3d_cells_from_voxels(&voxels, palette_tokens.len());
        if common_size.is_some_and(|expected| expected != size) {
            return Some(SourceSpriteTarget {
                dimension: SourceSpriteDimension::Three,
                status: SourceSprite3dStatus::Invalid,
                palette_tokens,
                ..SourceSpriteTarget::default()
            });
        }
        common_size = Some(size);
        let layer_len = size * size;
        edit_frames.push(
            cells
                .chunks(layer_len)
                .map(|layer| layer.to_vec())
                .collect::<Vec<_>>(),
        );
    }
    let palette = palette_map
        .values()
        .map(source_sprite3d_color_string)
        .collect::<Vec<_>>();
    let rows = layers
        .iter()
        .enumerate()
        .flat_map(|(index, layer)| {
            let mut rows = layer.clone();
            if index + 1 < layers.len() {
                rows.push("-".to_string());
            }
            rows
        })
        .collect::<Vec<_>>();
    let size = common_size.unwrap_or(0);
    let cells = edit_frames
        .first()
        .map(|layers| layers.iter().flatten().copied().collect())
        .unwrap_or_default();
    Some(SourceSpriteTarget {
        dimension: SourceSpriteDimension::Three,
        status: SourceSprite3dStatus::Complete,
        palette_tokens,
        resolved_palette,
        palette,
        shape_ref,
        color_assets: document
            .visual_sprite_refs
            .color_assets
            .iter()
            .map(|(name, color)| SourceSpriteColorAsset {
                name: name.clone(),
                color: color.clone(),
            })
            .collect(),
        shape_assets: document
            .visual_sprite_refs
            .shape_assets
            .iter()
            .map(|(name, rows)| SourceSpriteShapeAsset {
                name: name.clone(),
                rows: rows.clone(),
            })
            .collect(),
        rows,
        frames: edit_frames,
        duration_ms: syntax
            .duration
            .as_deref()
            .and_then(|value| puzzle_scene::parse_wait_duration_ms_at(value, value).ok()),
        frame_duration_ms: syntax
            .frame_duration
            .as_deref()
            .and_then(|value| puzzle_scene::parse_wait_duration_ms_at(value, value).ok()),
        size: Some(size),
        cells,
        width: Some(size),
        height: Some(size),
        depth: Some(size),
        spatial_ops3: spatial_ops,
        ..SourceSpriteTarget::default()
    })
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
    let body_lines = body
        .lines()
        .map(|line| code_trim(line).to_string())
        .collect::<Vec<_>>();
    let syntax = crate::sprite_authoring::parse_sprite_node(None, &body_lines);
    let resolved_shape = crate::sprite_authoring::resolve_sprite_shape(&syntax, |name| {
        visual_refs.shape_names.contains(name)
    });
    let mut target = SourceSpriteTarget::default();
    let visual_target_name = syntax
        .selector
        .clone()
        .unwrap_or_else(|| target_name.to_string());
    target.palette_tokens = syntax.colors.unwrap_or_default();
    target.prelude_rows = syntax.prelude_rows;
    if let Some(value) = syntax.duration {
        target.duration_ms = puzzle_scene::parse_wait_duration_ms_at(&value, &value).ok();
    }
    if let Some(value) = syntax.frame_duration {
        target.frame_duration_ms = puzzle_scene::parse_wait_duration_ms_at(&value, &value).ok();
    }
    match resolved_shape {
        crate::sprite_authoring::ResolvedSpriteShape::Reference(reference) => {
            target.shape_ref = Some(reference);
        }
        crate::sprite_authoring::ResolvedSpriteShape::Inline(frames) => {
            let frames = crate::sprite_authoring::into_single_layer_frames(frames)
                .unwrap_or_default()
                .into_iter()
                .map(|frame| frame.into_iter().map(|row| row.text).collect::<Vec<_>>())
                .collect::<Vec<_>>();
            target.pixel_rows = frames.first().cloned().unwrap_or_default();
            if frames.len() >= 2 {
                target.animation_frames = frames;
            }
        }
        crate::sprite_authoring::ResolvedSpriteShape::None
        | crate::sprite_authoring::ResolvedSpriteShape::UnknownBareReference(_)
        | crate::sprite_authoring::ResolvedSpriteShape::AmbiguousBareRow(_) => {}
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
    enrich_source_sprite_target_from_loaded_visual(source, &visual_target_name, &mut target);
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
        target.resolved_palette =
            source_sprite_palette_from_refs(&target.palette_tokens, &visual_refs.color_assets);
    }
    populate_source_sprite_edit_frames(&mut target);
    Some(target)
}

fn populate_source_sprite_edit_frames(target: &mut SourceSpriteTarget) {
    if target.resolved_palette.is_empty() {
        target.status = SourceSprite3dStatus::Incomplete;
        return;
    }
    let rows_by_frame = if !target.animation_frames.is_empty() {
        target.animation_frames.clone()
    } else if !target.resolved_shape_rows.is_empty() {
        vec![target.resolved_shape_rows.clone()]
    } else if !target.pixel_rows.is_empty() {
        vec![target.pixel_rows.clone()]
    } else if target.resolved_palette.len() == 1 {
        vec![vec!["0".to_string()]]
    } else {
        target.status = SourceSprite3dStatus::Incomplete;
        return;
    };
    let height = rows_by_frame.first().map_or(0, Vec::len);
    let width = rows_by_frame
        .first()
        .and_then(|rows| rows.iter().map(String::len).max())
        .unwrap_or(0);
    if width == 0
        || height == 0
        || rows_by_frame
            .iter()
            .any(|rows| rows.len() != height || rows.iter().any(|row| row.chars().count() != width))
    {
        target.status = SourceSprite3dStatus::Invalid;
        return;
    }
    let keys = SOURCE_SPRITE3D_PALETTE_KEYS.chars().collect::<Vec<_>>();
    let mut frames = Vec::new();
    for rows in rows_by_frame {
        let mut cells = Vec::with_capacity(width * height);
        for row in rows {
            for ch in row.chars() {
                if ch == '.' || ch == ' ' {
                    cells.push(None);
                    continue;
                }
                let Some(index) = keys.iter().position(|key| *key == ch) else {
                    target.status = SourceSprite3dStatus::Invalid;
                    return;
                };
                if index >= target.resolved_palette.len() {
                    target.status = SourceSprite3dStatus::Invalid;
                    return;
                }
                cells.push(Some(index));
            }
        }
        frames.push(vec![cells]);
    }
    target.width = Some(width);
    target.height = Some(height);
    target.frames = frames;
    target.status = SourceSprite3dStatus::Complete;
}

fn enrich_source_sprite_target_from_loaded_visual(
    source: &str,
    target_name: &str,
    target: &mut SourceSpriteTarget,
) {
    let Some(sprite) = loaded_visual_sprite_for_source_target(source, target_name) else {
        return;
    };
    target.spatial_ops = sprite.transforms.clone();
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

fn source_sprite_palette_from_refs(
    tokens: &[String],
    color_assets: &BTreeMap<String, String>,
) -> Vec<SourceSpritePaletteEntry> {
    if tokens.is_empty() {
        return Vec::new();
    }
    tokens
        .iter()
        .map(|token| {
            if is_sprite_color(token) {
                Some(SourceSpritePaletteEntry {
                    source: token.clone(),
                    color: token.clone(),
                    linked: false,
                })
            } else {
                color_assets
                    .get(token)
                    .map(|color| SourceSpritePaletteEntry {
                        source: token.clone(),
                        color: color.clone(),
                        linked: true,
                    })
            }
        })
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug)]
struct Sprite3dBlock {
    open_index: usize,
    close_index: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct SpriteBlock {
    pub(crate) open_index: usize,
    pub(crate) close_index: usize,
    pub(crate) kind: SourceTargetKind,
}

#[derive(Clone, Debug)]
struct Level3dBlock {
    open_index: usize,
    close_index: usize,
    bundle: String,
    model: String,
}

fn level3d_blocks(source: &str, context: &SurfaceDocument) -> Vec<Level3dBlock> {
    let is_puzzle3 = context.source_profile == Some(PuzzleSourceProfile::Puzzle3d);
    let model3_names = context
        .lines
        .iter()
        .filter(|line| line.scope.is_none())
        .filter_map(|line| match line.tokens.as_slice() {
            [kind, name, ..] if kind == "puzzle3" || (is_puzzle3 && kind == "puzzle") => {
                Some(clean_name_token(name))
            }
            _ => None,
        })
        .collect::<HashSet<_>>();
    let has_model2 = !is_puzzle3
        && context.lines.iter().any(|line| {
            line.scope.is_none() && matches!(line.tokens.as_slice(), [kind, ..] if kind == "puzzle")
        });
    context
        .lines
        .iter()
        .filter(|line| line.tokens.first().is_some_and(|token| token == "levels"))
        .filter_map(|line| {
            let header_end = line_end(line);
            let open_index = source[line.start..header_end]
                .find('{')
                .map(|offset| line.start + offset)?;
            let close_index = find_matching_brace(source, open_index)?;
            let (bundle, model) = parse_levels_tokens(&line.tokens);
            let targets_3d = if model.is_empty() {
                !model3_names.is_empty() && !has_model2
            } else {
                model3_names.contains(&model)
            };
            if !targets_3d {
                return None;
            }
            Some(Level3dBlock {
                open_index,
                close_index,
                bundle,
                model,
            })
        })
        .collect()
}

fn parse_levels_tokens(tokens: &[String]) -> (String, String) {
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
    for (source_slice, slice) in voxels.slices.iter().enumerate() {
        let world_z = size - 1 - source_slice;
        for (y, row) in slice.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch == '.' || ch == ' ' {
                    continue;
                }
                let Some(color_index) = keys.iter().position(|key| *key == ch) else {
                    continue;
                };
                let cell_index = (world_z * size + y) * size + x;
                cells[cell_index] = Some(color_index);
            }
        }
    }
    (size, cells)
}

const SOURCE_SPRITE3D_PALETTE_KEYS: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

pub(crate) fn sprite_blocks(source: &str, context: &SurfaceDocument) -> Vec<SpriteBlock> {
    let is_puzzle3 = context.source_profile == Some(PuzzleSourceProfile::Puzzle3d);
    let models = context
        .lines
        .iter()
        .filter_map(|line| {
            let [kind, name, ..] = line.tokens.as_slice() else {
                return None;
            };
            if kind != "puzzle" && kind != "puzzle3" {
                return None;
            }
            let header_end = line_end(line);
            let open_index = source[line.start..header_end]
                .find('{')
                .map(|offset| line.start + offset)?;
            let close_index = find_matching_brace(source, open_index)?;
            Some((
                clean_name_token(name),
                open_index,
                close_index,
                if kind == "puzzle3" || is_puzzle3 {
                    SourceTargetKind::Sprite3d
                } else {
                    SourceTargetKind::Sprite
                },
            ))
        })
        .collect::<Vec<_>>();
    let model_kinds = models
        .iter()
        .map(|(name, _, _, kind)| (name.clone(), kind.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let unique_kind = models
        .iter()
        .map(|(_, _, _, kind)| kind)
        .next()
        .filter(|first| models.iter().all(|(_, _, _, kind)| kind == *first))
        .cloned();
    context
        .lines
        .iter()
        .filter(|line| line.tokens.first().is_some_and(|token| token == "sprites"))
        .filter_map(|line| {
            let header_end = line_end(line);
            let open_index = source[line.start..header_end]
                .find('{')
                .map(|offset| line.start + offset)?;
            let close_index = find_matching_brace(source, open_index)?;
            let model = line
                .tokens
                .windows(2)
                .find_map(|tokens| (tokens[0] == "of").then(|| clean_name_token(&tokens[1])));
            let kind = model
                .as_ref()
                .and_then(|name| model_kinds.get(name))
                .cloned()
                .or_else(|| {
                    models
                        .iter()
                        .find(|(_, model_open, model_close, _)| {
                            line.start > *model_open && line.start < *model_close
                        })
                        .map(|(_, _, _, kind)| kind.clone())
                })
                .or_else(|| unique_kind.clone())
                .unwrap_or(SourceTargetKind::Sprite);
            Some(SpriteBlock {
                open_index,
                close_index,
                kind,
            })
        })
        .collect()
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

fn sprite_node_selector_name(
    source: &str,
    header: &str,
    body_start: usize,
    body_end: usize,
) -> Option<String> {
    let body = source.get(body_start..body_end)?;
    let lines = body.lines().map(str::to_string).collect::<Vec<_>>();
    crate::sprite_authoring::parse_sprite_node(Some(header), &lines)
        .selector
        .filter(|selector| sprite_definition_name_token(selector))
        .map(|selector| clean_name_token(&selector))
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
            if (keyword == "palette" || keyword == "shapes")
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
        "shape" | "shapes" | "palette" | "colors" | "ascii" | "sprites"
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
    if tokens.first() == Some(&"=") {
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
        || lower.ends_with(".svg")
}

fn line_end(line: &SurfaceLine) -> usize {
    line.start + line.content.len()
}

fn code_trim(line: &str) -> &str {
    strip_line_comment(line).trim()
}

pub(crate) fn find_matching_brace(source: &str, open_index: usize) -> Option<usize> {
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

fn push_json_string_matrix_value(out: &mut String, values: &[Vec<String>]) {
    out.push('[');
    for (index, row) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string_array_value(out, row);
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
        resolve_source_target_for_profile,
    };
    use crate::PuzzleSourceProfile;

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
slots {
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

puzzle board3 {
sprites {
Cube {
colors = #fff
shape = {
0
}
}
}
levels pack of board3 {
level "three" {
0
}
}
}
"#;
        let mut document = crate::parse_surface_document(source);
        document.source_profile = Some(PuzzleSourceProfile::Puzzle3d);
        let entries = resolve_source_entries_from_document(source, &document);

        assert!(
            entries.iter().any(|entry| {
                entry.kind == SourceTargetKind::Sprite3d && entry.name == "Player"
            })
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
    fn resolves_levels_body_to_3d_level() {
        let source = r#"
puzzle push3d {
slots {
Player
}
rules {
}
}

levels basic of push3d {
level "push3d_01" {
___
_P_
}
}
"#;
        let cursor = source.find("_P_").unwrap();
        let target =
            resolve_source_target_for_profile(source, cursor, PuzzleSourceProfile::Puzzle3d)
                .unwrap();

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
sfx clear { seed = clear01; type = jump }
music music_name { seed = test1; bars = 8; height = 0; bpm = 100 }
}
"#;
        let cursor = source.find("height = 0").unwrap();
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
sfx clear { seed = clear01; type = jump }
}
}
"#;
        let cursor = source.find("type = jump").unwrap();
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
        assert!(
            source[target.body_start.unwrap()..target.body_end.unwrap()]
                .trim()
                .is_empty()
        );
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
    fn animation_sprite_rows_resolve_to_sprite_target() {
        let source = r##"
sprites {
Player
duration 120ms
#000 #fff
.0.
111
.0.
>
111
.0.
111
}
"##;

        for cursor_text in ["duration 120ms", ".0.", ">", "111\n}"] {
            let cursor = source.find(cursor_text).unwrap();
            let target = resolve_source_target(source, cursor).unwrap();

            assert_eq!(target.kind, SourceTargetKind::Sprite);
            assert_eq!(target.name, "Player");
            let sprite = target.source_sprite.unwrap();
            assert_eq!(sprite.duration_ms, Some(120));
            assert_eq!(
                sprite.animation_frames,
                vec![
                    vec![".0.".to_string(), "111".to_string(), ".0.".to_string()],
                    vec!["111".to_string(), ".0.".to_string(), "111".to_string()],
                ]
            );
        }
    }

    #[test]
    fn animation_sprite_frame_duration_resolves_to_sprite_target() {
        let source = r##"
sprites {
Player
frame_duration 60ms
#000 #fff
.0.
111
.0.
>
111
.0.
111
}
"##;
        let cursor = source.find("frame_duration 60ms").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();
        let sprite = target.source_sprite.unwrap();

        assert_eq!(sprite.frame_duration_ms, Some(60));
        assert_eq!(sprite.prelude_rows, vec!["frame_duration 60ms".to_string()]);
    }

    #[test]
    fn explicit_shape_block_resolves_animation_rows_without_shape_ref() {
        let source = r##"
sprites {
sprite {
selector = Background
colors = #90ee90 #008000
duration = 500ms
shape = {
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
}
}
}
"##;
        let cursor = source.find("shape =").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();
        let source_sprite = target
            .source_sprite
            .as_ref()
            .expect("source sprite contract");

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Background");
        assert_eq!(source_sprite.shape_ref, None);
        assert!(
            !source_sprite
                .prelude_rows
                .iter()
                .any(|row| row.starts_with("shape"))
        );
        assert_eq!(source_sprite.duration_ms, Some(500));
        assert_eq!(
            source_sprite.animation_frames,
            vec![
                vec![
                    "11111".to_string(),
                    "01111".to_string(),
                    "11101".to_string(),
                    "11111".to_string(),
                    "10111".to_string(),
                ],
                vec![
                    "10111".to_string(),
                    "11111".to_string(),
                    "01111".to_string(),
                    "11101".to_string(),
                    "11111".to_string(),
                ],
            ]
        );
    }

    #[test]
    fn user_named_color_row_stays_in_current_sprite_target() {
        let source = r##"
sprites {
palette {
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
palette {
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
slots {
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
palette {
accent = #e94f64
}
sprite {
selector = Player
colors = accent
shape = {
0
}
}
}
"##;
        let cursor = source.find("colors = accent").unwrap();
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
    fn sprite_edit_contract_distinguishes_transparent_color_from_empty_cell() {
        let source = r#"
puzzle world { slots { actor = Player } }
sprites art of world {
Player {
colors = transparent red
shape = {
0.1
}
}
}
"#;
        let target = resolve_source_target(source, source.find("0.1").unwrap()).unwrap();
        let sprite = target.source_sprite.unwrap();
        assert_eq!(sprite.status, SourceSprite3dStatus::Complete);
        assert_eq!(sprite.frames, vec![vec![vec![Some(0), None, Some(1)]]]);
        assert_eq!(sprite.resolved_palette[0].color, "transparent");
    }

    #[test]
    fn sprite_source_contract_preserves_selector_and_duration_rows() {
        let source = r##"
puzzle main {
slots {
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
palette {
accent = #e94f64
}
Player {
selector = Player
duration 120ms
colors accent
0
}
}
"##;
        let cursor = source.find("duration 120ms").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();
        let source_sprite = target
            .source_sprite
            .as_ref()
            .expect("source sprite contract");

        assert_eq!(
            source_sprite.prelude_rows,
            vec![
                "selector = Player".to_string(),
                "duration 120ms".to_string()
            ]
        );
        assert_eq!(source_sprite.duration_ms, Some(120));
    }

    #[test]
    fn sprite_source_contract_exposes_animation_frames() {
        let source = r##"
puzzle main {
slots {
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
Player {
duration 120ms
#e94f64
0.
..
>
..
.0
}
}
"##;
        let cursor = source.find(">").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();
        let source_sprite = target
            .source_sprite
            .as_ref()
            .expect("source sprite contract");

        assert_eq!(source_sprite.duration_ms, Some(120));
        assert_eq!(
            source_sprite.pixel_rows,
            vec!["0.".to_string(), "..".to_string()]
        );
        assert_eq!(
            source_sprite.animation_frames,
            vec![
                vec!["0.".to_string(), "..".to_string()],
                vec!["..".to_string(), ".0".to_string()],
            ]
        );
    }

    #[test]
    fn consecutive_tagged_sprite_color_name_rows_do_not_become_sprite_headers() {
        let source = r##"
sprites {
palette {
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
palette {
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
    fn bare_shape_reference_uses_shared_owner_scope_resolution() {
        let source = r##"
sprites {
Box
#111 #eee
box_shape

shapes {
box_shape
010
111
010
}
}
"##;
        let cursor = source.find("box_shape\n\nshapes").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();
        let source_sprite = target
            .source_sprite
            .as_ref()
            .expect("source sprite contract");

        assert_eq!(source_sprite.shape_ref.as_deref(), Some("box_shape"));
        assert!(source_sprite.pixel_rows.is_empty());
        assert_eq!(
            source_sprite.resolved_shape_rows,
            vec!["010".to_string(), "111".to_string(), "010".to_string()]
        );
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
rotate directions from up
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
            vec!["rotate directions from up".to_string()]
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
        assert!(body.contains("rotate directions from up"));
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
    fn resolves_stacked_sprite_entry_as_sprite3d() {
        let source = r##"
puzzle push3d {
}
sprites basic of push3d {
Floor {
colors = #90ee90 #008000
shape = {
.....
..0..
-
11111
.....
}
}

Goal {
colors = #00008b
shape = {
.....
.000.
}
}
}
"##;
        let cursor = source.find("..0..").unwrap();
        let target =
            resolve_source_target_for_profile(source, cursor, PuzzleSourceProfile::Puzzle3d)
                .unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite3d);
        assert_eq!(target.name, "Floor");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#90ee90"));
        assert!(body.contains("11111"));
        assert!(!body.contains("Goal"));
        let sprite3d = target.source_sprite.as_ref().unwrap();
        assert_eq!(sprite3d.status, SourceSprite3dStatus::Complete);
        assert_eq!(sprite3d.size, Some(5));
        assert_eq!(sprite3d.palette, vec!["#90ee90", "#008000"]);
        assert_eq!(sprite3d.cells.len(), 125);
        assert_eq!(sprite3d.cells[(4 * 5 + 1) * 5 + 2], Some(0));
        assert_eq!(sprite3d.cells[(3 * 5) * 5], Some(1));
        assert_eq!(sprite3d.cells[(0 * 5 + 1) * 5 + 2], None);
    }

    #[test]
    fn sprite3d_contract_preserves_named_color_shape_and_all_animation_frames() {
        let source = r#"
puzzle board {
}
sprites art of board {
palette {
accent = #123456
}
shapes {
pulse {
0
>
.
}
}
Orb {
duration = 200ms
frame_duration = 100ms
colors = accent
shape = pulse
}
}
"#;
        let cursor = source.find("duration = 200ms").unwrap();
        let target =
            resolve_source_target_for_profile(source, cursor, PuzzleSourceProfile::Puzzle3d)
                .unwrap();
        let sprite = target.source_sprite.unwrap();

        assert_eq!(sprite.status, SourceSprite3dStatus::Complete);
        assert_eq!(sprite.shape_ref.as_deref(), Some("pulse"));
        assert_eq!(sprite.duration_ms, Some(200));
        assert_eq!(sprite.frame_duration_ms, Some(100));
        assert_eq!(sprite.frames.len(), 2);
        assert_eq!(sprite.frames[0], vec![vec![Some(0)]]);
        assert_eq!(sprite.frames[1], vec![vec![None]]);
        assert_eq!(
            sprite.resolved_palette,
            vec![SourceSpritePaletteEntry {
                source: "accent".to_string(),
                color: "#123456".to_string(),
                linked: true,
            }]
        );
        assert!(
            sprite
                .color_assets
                .iter()
                .any(|asset| asset.name == "accent")
        );
        assert!(
            sprite
                .shape_assets
                .iter()
                .any(|asset| asset.name == "pulse")
        );
    }

    #[test]
    fn resolves_second_stacked_sprite_entry_as_sprite3d() {
        let source = r##"
puzzle board {
sprites basic {
Floor {
colors = #90ee90
shape = {
.....
}
}

Goal {
colors = #00008b
shape = {
.000.
}
}
}
}
"##;
        let cursor = source.find(".000.").unwrap();
        let target =
            resolve_source_target_for_profile(source, cursor, PuzzleSourceProfile::Puzzle3d)
                .unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite3d);
        assert_eq!(target.name, "Goal");
    }

    #[test]
    fn resolves_unfinished_sprite3d_name_as_sprite3d_target() {
        let source = r#"
puzzle board {
sprites basic {
Floor {
}
}
}
"#;
        let cursor = source.find("Floor").unwrap() + "Floor".len();
        let target =
            resolve_source_target_for_profile(source, cursor, PuzzleSourceProfile::Puzzle3d)
                .unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite3d);
        assert_eq!(target.name, "Floor");
        assert!(
            source[target.body_start.unwrap()..target.body_end.unwrap()]
                .trim()
                .is_empty()
        );
        assert_eq!(
            target.source_sprite.as_ref().unwrap().status,
            SourceSprite3dStatus::Incomplete
        );
    }

    #[test]
    fn unfinished_sprite3d_stops_before_next_entry_header() {
        let source = r#"
puzzle board {
sprites basic {
Floor {
}
Goal {
}
}
}
"#;
        let cursor = source.find("Goal").unwrap();
        let target =
            resolve_source_target_for_profile(source, cursor, PuzzleSourceProfile::Puzzle3d)
                .unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite3d);
        assert_eq!(target.name, "Goal");
    }
}
