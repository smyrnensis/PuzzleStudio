use crate::PuzzleSourceProfile;
use crate::surface::{SurfaceDocument, SurfaceVisualSpriteRefs};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceTargetKind {
    Level,
    Sprite,
    Sounds,
}

impl SourceTargetKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Level => "level",
            Self::Sprite => "sprite",
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
    pub dimension: Option<crate::ModelDimension>,
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
    pub dimension: crate::ModelDimension,
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
    pub transforms: Vec<crate::VisualSpriteTransform>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceSpriteShapeAsset {
    Plain {
        name: String,
        frames: Vec<crate::VisualSpriteFrameDef>,
    },
    Table {
        name: String,
        axis: String,
        variants: BTreeMap<String, crate::VisualSpriteFrameDef>,
    },
}

impl SourceSpriteShapeAsset {
    pub fn name(&self) -> &str {
        match self {
            Self::Plain { name, .. } | Self::Table { name, .. } => name,
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

pub fn resolve_source_target(source: &str, cursor_offset: usize) -> Option<SourceTarget> {
    resolve_source_target_for_profile(source, cursor_offset, PuzzleSourceProfile::Puzzle2d)
}

pub fn resolve_source_target_for_profile(
    source: &str,
    cursor_offset: usize,
    profile: PuzzleSourceProfile,
) -> Option<SourceTarget> {
    let cursor = cursor_offset.min(source.len());
    let document = crate::parse_surface_source_target_document_for_profile(source, profile);
    resolve_source_target_from_document(&document, cursor)
}

pub fn source_entries_json(source: &str) -> String {
    let document = crate::parse_surface_source_target_document(source);
    let entries = resolve_source_entries_from_document(&document);
    source_entries_json_from_entries(&entries)
}

pub(crate) fn resolve_source_target_from_document(
    document: &SurfaceDocument,
    cursor: usize,
) -> Option<SourceTarget> {
    let entries = resolve_source_entries_from_document(document);
    resolve_source_target_from_entries(document, &entries, cursor)
}

pub(crate) fn resolve_source_target_from_entries(
    document: &SurfaceDocument,
    entries: &[SourceTarget],
    cursor: usize,
) -> Option<SourceTarget> {
    let mut target = entries
        .iter()
        .find(|entry| cursor >= entry.start && cursor <= entry.end)?
        .clone();
    if target.kind == SourceTargetKind::Sprite {
        target.source_sprite = match target.dimension {
            Some(crate::ModelDimension::Three) => source_sprite3d_for_target(document, &target),
            _ => source_sprite_for_target(document, &target),
        };
    }
    Some(target)
}

pub(crate) fn resolve_source_entries_from_document(
    document: &SurfaceDocument,
) -> Vec<SourceTarget> {
    let mut entries = Vec::new();
    entries.extend(resolve_sound_entries(document));
    entries.extend(resolve_level_entries(document));
    entries.extend(resolve_sprite_entries(document));
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
    if let Some(dimension) = target.dimension {
        out.push(',');
        push_json_string(
            out,
            "dimension",
            match dimension {
                crate::ModelDimension::Two => "2d",
                crate::ModelDimension::Three => "3d",
            },
        );
    }
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
    if sprite.dimension == crate::ModelDimension::Two {
        push_source_sprite2d_spatial_ops_json(out, &sprite.transforms);
    } else {
        push_source_sprite3d_spatial_ops_json(out, &sprite.transforms);
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
            crate::VisualSpriteTransform::Translate { value, space } => write!(
                out,
                "{{\"kind\":\"translate2\",\"space\":\"{}\",\"value\":[{},{}]}}",
                sprite_space_name(*space),
                value[0],
                value[1]
            )
            .unwrap(),
            crate::VisualSpriteTransform::Rotate { degrees, space, .. } => write!(
                out,
                "{{\"kind\":\"rotate2\",\"space\":\"{}\",\"degrees\":{degrees}}}",
                sprite_space_name(*space)
            )
            .unwrap(),
            crate::VisualSpriteTransform::Flip { enabled } => {
                write!(out, "{{\"kind\":\"flip2\",\"enabled\":{enabled}}}").unwrap()
            }
        }
    }
    out.push(']');
}

fn sprite_space_name(space: crate::VisualSpriteSpace) -> &'static str {
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
        match entry {
            SourceSpriteShapeAsset::Plain { name, frames } => {
                out.push('{');
                push_json_string(out, "kind", "plain");
                out.push(',');
                push_json_string(out, "name", name);
                out.push_str(",\"frames\":");
                push_source_sprite_shape_frames_json(out, frames);
                out.push('}');
            }
            SourceSpriteShapeAsset::Table {
                name,
                axis,
                variants,
            } => {
                out.push('{');
                push_json_string(out, "kind", "table");
                out.push(',');
                push_json_string(out, "name", name);
                out.push(',');
                push_json_string(out, "axis", axis);
                out.push_str(",\"variants\":[");
                for (variant_index, (value, frame)) in variants.iter().enumerate() {
                    if variant_index > 0 {
                        out.push(',');
                    }
                    out.push('{');
                    push_json_string(out, "value", value);
                    out.push_str(",\"frame\":");
                    push_json_string_matrix_value(out, &frame.planes);
                    out.push('}');
                }
                out.push_str("]}");
            }
        }
    }
    out.push(']');
}

fn push_source_sprite_shape_frames_json(out: &mut String, frames: &[crate::VisualSpriteFrameDef]) {
    out.push('[');
    for (index, frame) in frames.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string_matrix_value(out, &frame.planes);
    }
    out.push(']');
}

fn push_source_sprite3d_spatial_ops_json(out: &mut String, ops: &[crate::VisualSpriteTransform]) {
    out.push('[');
    for (index, op) in ops.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        match op {
            crate::VisualSpriteTransform::Translate { space, value } => write!(out, "{{\"kind\":\"translate3\",\"space\":\"{}\",\"value\":[{},{},{}]}}", sprite_space_name(*space), value[0], value[1], value[2]).unwrap(),
            crate::VisualSpriteTransform::Rotate { space, axis, degrees } => write!(out, "{{\"kind\":\"rotate3\",\"space\":\"{}\",\"axis\":[{},{},{}],\"degrees\":{degrees}}}", sprite_space_name(*space), axis[0], axis[1], axis[2]).unwrap(),
            crate::VisualSpriteTransform::Flip { enabled } => write!(out, "{{\"kind\":\"flip3\",\"enabled\":{enabled}}}").unwrap(),
        }
    }
    out.push(']');
}

fn resolve_sound_entries(context: &SurfaceDocument) -> Vec<SourceTarget> {
    context
        .sound_products
        .iter()
        .map(|sound| SourceTarget {
            kind: SourceTargetKind::Sounds,
            dimension: None,
            name: sound.name.clone(),
            start: sound.span.start,
            end: sound.span.end,
            body_start: None,
            body_end: None,
            level_index: None,
            sound_kind: Some(match sound.kind {
                crate::surface::SurfaceSoundKind::Sfx => SoundSourceTargetKind::Sfx,
                crate::surface::SurfaceSoundKind::Music => SoundSourceTargetKind::Music,
            }),
            params: sound.params.clone(),
            source_sprite: None,
        })
        .collect()
}

fn resolve_level_entries(context: &SurfaceDocument) -> Vec<SourceTarget> {
    context
        .level_products
        .iter()
        .map(|product| {
            let mut params = Vec::new();
            if let Some(pack) = &product.pack {
                params.push(("bundle".to_string(), pack.clone()));
            }
            if let Some(puzzle) = &product.puzzle {
                params.push(("model".to_string(), puzzle.clone()));
            }
            SourceTarget {
                kind: SourceTargetKind::Level,
                dimension: Some(product.dimension),
                name: product.name.clone(),
                start: product.span.start,
                end: product.span.end,
                body_start: Some(product.body_span.start),
                body_end: Some(product.body_span.end),
                level_index: Some(product.level_index),
                sound_kind: None,
                params,
                source_sprite: None,
            }
        })
        .collect()
}

fn resolve_sprite_entries(context: &SurfaceDocument) -> Vec<SourceTarget> {
    context
        .sprite_products
        .iter()
        .filter(|product| !product.name.is_empty())
        .map(|product| SourceTarget {
            kind: SourceTargetKind::Sprite,
            dimension: Some(product.dimension),
            name: product.name.clone(),
            start: product.span.start,
            end: product.span.end,
            body_start: Some(product.body_span.start),
            body_end: Some(product.body_span.end),
            level_index: None,
            sound_kind: None,
            params: Vec::new(),
            source_sprite: None,
        })
        .collect()
}

fn source_sprite_for_target(
    document: &SurfaceDocument,
    target: &SourceTarget,
) -> Option<SourceSpriteTarget> {
    source_sprite_target(
        sprite_product_for_target(document, target)?,
        &document.visual_sprite_refs,
    )
}

fn source_sprite3d_for_target(
    document: &SurfaceDocument,
    target: &SourceTarget,
) -> Option<SourceSpriteTarget> {
    let product = sprite_product_for_target(document, target)?;
    let analyzed = &product.body;
    if analyzed.error.is_some() {
        return Some(SourceSpriteTarget {
            dimension: crate::ModelDimension::Three,
            status: SourceSpriteStatus::Invalid,
            ..SourceSpriteTarget::default()
        });
    }
    let syntax = &analyzed.syntax;
    let Ok(transforms) =
        crate::eval_sprite_transforms(&syntax.properties, &HashMap::new(), &target.name)
    else {
        return Some(SourceSpriteTarget {
            dimension: crate::ModelDimension::Three,
            status: SourceSpriteStatus::Invalid,
            ..SourceSpriteTarget::default()
        });
    };
    let palette_tokens = syntax.colors.clone().unwrap_or_default();
    if palette_tokens.is_empty() {
        return Some(SourceSpriteTarget {
            dimension: crate::ModelDimension::Three,
            status: SourceSpriteStatus::Incomplete,
            ..SourceSpriteTarget::default()
        });
    }
    let resolved_palette =
        source_sprite_palette_from_refs(&palette_tokens, &document.visual_sprite_refs.color_assets);
    if resolved_palette.is_empty() {
        return Some(SourceSpriteTarget {
            dimension: crate::ModelDimension::Three,
            status: SourceSpriteStatus::Invalid,
            palette_tokens,
            ..SourceSpriteTarget::default()
        });
    }
    if palette_tokens.len() > SOURCE_SPRITE3D_PALETTE_KEYS.len() {
        return Some(SourceSpriteTarget {
            dimension: crate::ModelDimension::Three,
            status: SourceSpriteStatus::Invalid,
            palette_tokens,
            ..SourceSpriteTarget::default()
        });
    };
    let shape_ref = match &analyzed.shape {
        crate::sprite_authoring::ResolvedSpriteShape::Reference(reference) => {
            Some(reference.clone())
        }
        _ => None,
    };
    let frames = match &analyzed.shape {
        crate::sprite_authoring::ResolvedSpriteShape::Reference(reference) => {
            let asset_name = product.shape_asset_name.as_deref()?;
            match document.visual_sprite_refs.shape_assets.get(asset_name)? {
                crate::surface::SurfaceSpriteShapeAsset::Plain { frames } => frames.clone(),
                crate::surface::SurfaceSpriteShapeAsset::Table { .. } => {
                    return Some(source_sprite3d_unresolved_table_target(
                        document,
                        syntax,
                        palette_tokens,
                        resolved_palette,
                        reference.clone(),
                        transforms,
                    ));
                }
            }
        }
        crate::sprite_authoring::ResolvedSpriteShape::Inline(frames) => frames.clone(),
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
                dimension: crate::ModelDimension::Three,
                status: SourceSpriteStatus::Invalid,
                palette_tokens,
                ..SourceSpriteTarget::default()
            });
        }
    };
    let frame_layers = sprite_frame_layers(&frames);
    let Some(layers) = frame_layers.first() else {
        return Some(SourceSpriteTarget {
            dimension: crate::ModelDimension::Three,
            status: SourceSpriteStatus::Invalid,
            palette_tokens,
            ..SourceSpriteTarget::default()
        });
    };
    let mut edit_frames = Vec::with_capacity(frame_layers.len());
    let mut common_size = None;
    for frame in &frame_layers {
        let Some((size, cells)) = source_sprite3d_cells(frame, palette_tokens.len()) else {
            return Some(SourceSpriteTarget {
                dimension: crate::ModelDimension::Three,
                status: SourceSpriteStatus::Invalid,
                palette_tokens,
                ..SourceSpriteTarget::default()
            });
        };
        if common_size.is_some_and(|expected| expected != size) {
            return Some(SourceSpriteTarget {
                dimension: crate::ModelDimension::Three,
                status: SourceSpriteStatus::Invalid,
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
    let palette = resolved_palette
        .iter()
        .map(|entry| sprite_editor_color(&entry.color))
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
        dimension: crate::ModelDimension::Three,
        status: SourceSpriteStatus::Complete,
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
        shape_assets: source_sprite_shape_assets(&document.visual_sprite_refs),
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
        transforms,
        ..SourceSpriteTarget::default()
    })
}

fn sprite_product_for_target<'a>(
    document: &'a SurfaceDocument,
    target: &SourceTarget,
) -> Option<&'a crate::surface::SurfaceSpriteProduct> {
    document
        .sprite_products
        .iter()
        .find(|product| product.span.start == target.start && product.span.end == target.end)
}

fn source_sprite_shape_frame(
    frame: &crate::sprite_authoring::SpriteFrameSyntax,
) -> crate::VisualSpriteFrameDef {
    crate::VisualSpriteFrameDef {
        planes: frame
            .layers
            .iter()
            .map(|layer| layer.rows.iter().map(|row| row.text.clone()).collect())
            .collect(),
    }
}

fn source_sprite_shape_assets(
    visual_refs: &SurfaceVisualSpriteRefs,
) -> Vec<SourceSpriteShapeAsset> {
    visual_refs
        .shape_assets
        .iter()
        .map(|(name, asset)| match asset {
            crate::surface::SurfaceSpriteShapeAsset::Plain { frames } => {
                SourceSpriteShapeAsset::Plain {
                    name: name.clone(),
                    frames: frames.iter().map(source_sprite_shape_frame).collect(),
                }
            }
            crate::surface::SurfaceSpriteShapeAsset::Table { axis, variants } => {
                SourceSpriteShapeAsset::Table {
                    name: name.clone(),
                    axis: axis.clone(),
                    variants: variants
                        .iter()
                        .map(|(value, frame)| (value.clone(), source_sprite_shape_frame(frame)))
                        .collect(),
                }
            }
        })
        .collect()
}

fn source_sprite3d_unresolved_table_target(
    document: &SurfaceDocument,
    syntax: &crate::sprite_authoring::SpriteNodeSyntax,
    palette_tokens: Vec<String>,
    resolved_palette: Vec<SourceSpritePaletteEntry>,
    shape_ref: String,
    transforms: Vec<crate::VisualSpriteTransform>,
) -> SourceSpriteTarget {
    SourceSpriteTarget {
        dimension: crate::ModelDimension::Three,
        status: SourceSpriteStatus::Incomplete,
        palette: resolved_palette
            .iter()
            .map(|entry| sprite_editor_color(&entry.color))
            .collect(),
        palette_tokens,
        resolved_palette,
        shape_ref: Some(shape_ref),
        color_assets: document
            .visual_sprite_refs
            .color_assets
            .iter()
            .map(|(name, color)| SourceSpriteColorAsset {
                name: name.clone(),
                color: color.clone(),
            })
            .collect(),
        shape_assets: source_sprite_shape_assets(&document.visual_sprite_refs),
        duration_ms: syntax
            .duration
            .as_deref()
            .and_then(|value| puzzle_scene::parse_wait_duration_ms_at(value, value).ok()),
        frame_duration_ms: syntax
            .frame_duration
            .as_deref()
            .and_then(|value| puzzle_scene::parse_wait_duration_ms_at(value, value).ok()),
        transforms,
        ..SourceSpriteTarget::default()
    }
}

fn source_sprite_plain_shape_rows(asset: &SourceSpriteShapeAsset) -> Option<Vec<String>> {
    let SourceSpriteShapeAsset::Plain { frames, .. } = asset else {
        return None;
    };
    frames
        .first()
        .and_then(|frame| frame.planes.first())
        .cloned()
}

fn sprite_frame_layers(
    frames: &[crate::sprite_authoring::SpriteFrameSyntax],
) -> Vec<Vec<Vec<String>>> {
    frames
        .iter()
        .map(|frame| {
            frame
                .layers
                .iter()
                .map(|layer| layer.rows.iter().map(|row| row.text.clone()).collect())
                .collect()
        })
        .collect()
}

fn source_sprite_target(
    product: &crate::surface::SurfaceSpriteProduct,
    visual_refs: &SurfaceVisualSpriteRefs,
) -> Option<SourceSpriteTarget> {
    let mut target = SourceSpriteTarget::default();
    let analyzed = &product.body;
    let product_invalid = analyzed.error.is_some();
    let syntax = &analyzed.syntax;
    target.palette_tokens = syntax.colors.clone().unwrap_or_default();
    target.prelude_rows = syntax.prelude_rows.clone();
    if let Some(value) = &syntax.duration {
        target.duration_ms = puzzle_scene::parse_wait_duration_ms_at(&value, &value).ok();
    }
    if let Some(value) = &syntax.frame_duration {
        target.frame_duration_ms = puzzle_scene::parse_wait_duration_ms_at(&value, &value).ok();
    }
    match &analyzed.shape {
        crate::sprite_authoring::ResolvedSpriteShape::Reference(reference) => {
            target.shape_ref = Some(reference.clone());
        }
        crate::sprite_authoring::ResolvedSpriteShape::Inline(frames) => {
            let frames = crate::sprite_authoring::into_single_layer_frames(frames.clone())
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
    target.shape_assets = source_sprite_shape_assets(visual_refs);
    target
        .shape_assets
        .sort_by(|left, right| left.name().cmp(right.name()));
    target.resolved_palette =
        source_sprite_palette_from_refs(&target.palette_tokens, &visual_refs.color_assets);
    target.transforms = match crate::eval_sprite_transforms(
        &syntax.properties,
        &HashMap::new(),
        syntax.selector.as_deref().unwrap_or("sprite"),
    ) {
        Ok(transforms) => transforms,
        Err(_) => {
            target.status = SourceSpriteStatus::Invalid;
            return Some(target);
        }
    };
    if target.resolved_shape_rows.is_empty() {
        if let Some(shape_ref) = &target.shape_ref {
            if let Some(rows) = target
                .shape_assets
                .iter()
                .find(|asset| asset.name() == shape_ref)
                .and_then(source_sprite_plain_shape_rows)
            {
                target.resolved_shape_rows = rows;
            }
        }
    }
    populate_source_sprite_edit_frames(&mut target);
    if product_invalid {
        target.status = SourceSpriteStatus::Invalid;
    }
    Some(target)
}

fn populate_source_sprite_edit_frames(target: &mut SourceSpriteTarget) {
    if target.resolved_palette.is_empty() {
        target.status = SourceSpriteStatus::Incomplete;
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
        target.status = SourceSpriteStatus::Incomplete;
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
        target.status = SourceSpriteStatus::Invalid;
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
                    target.status = SourceSpriteStatus::Invalid;
                    return;
                };
                if index >= target.resolved_palette.len() {
                    target.status = SourceSpriteStatus::Invalid;
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
    target.status = SourceSpriteStatus::Complete;
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

fn sprite_editor_color(color: &str) -> String {
    if color == "transparent" {
        "#00000000".to_string()
    } else {
        color.to_string()
    }
}

fn source_sprite3d_cells(
    layers: &[Vec<String>],
    palette_len: usize,
) -> Option<(usize, Vec<Option<usize>>)> {
    let first_layer = layers.first()?;
    let first_row = first_layer.first()?;
    let width = first_row.chars().count();
    let height = first_layer.len();
    if width == 0
        || layers.iter().any(|layer| {
            layer.len() != height || layer.iter().any(|row| row.chars().count() != width)
        })
    {
        return None;
    }
    let size = width.max(height).max(layers.len());
    let mut cells = vec![None; size * size * size];
    let keys = SOURCE_SPRITE3D_PALETTE_KEYS
        .chars()
        .take(palette_len)
        .collect::<Vec<_>>();
    for (source_slice, slice) in layers.iter().enumerate() {
        let world_z = size - 1 - source_slice;
        for (y, row) in slice.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch == '.' || ch == ' ' {
                    continue;
                }
                let color_index = keys.iter().position(|key| *key == ch)?;
                let cell_index = (world_z * size + y) * size + x;
                cells[cell_index] = Some(color_index);
            }
        }
    }
    Some((size, cells))
}

const SOURCE_SPRITE3D_PALETTE_KEYS: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn is_sprite_color(value: &str) -> bool {
    crate::syntax::is_visual_named_color(value) || is_hex_color(value)
}

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit())
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
        SoundSourceTargetKind, SourceSpritePaletteEntry, SourceSpriteShapeAsset,
        SourceSpriteStatus, SourceTargetKind, resolve_source_entries_from_document,
        resolve_source_target, resolve_source_target_for_profile,
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
dimension = 3
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
        let document = crate::parse_surface_document(source);
        let entries = resolve_source_entries_from_document(&document);

        assert!(
            entries
                .iter()
                .any(|entry| { entry.kind == SourceTargetKind::Sprite && entry.name == "Player" })
        );
        assert!(entries.iter().any(|entry| {
            entry.kind == SourceTargetKind::Level
                && entry.dimension == Some(crate::ModelDimension::Three)
                && entry.name == "three"
                && entry.params
                    == vec![
                        ("bundle".to_string(), "pack".to_string()),
                        ("model".to_string(), "board3".to_string()),
                    ]
        }));
        assert!(entries.iter().any(|entry| {
            entry.kind == SourceTargetKind::Sprite
                && entry.dimension == Some(crate::ModelDimension::Three)
                && entry.name == "Cube"
        }));
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
dimension = 3
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

        assert_eq!(target.kind, SourceTargetKind::Level);
        assert_eq!(target.dimension, Some(crate::ModelDimension::Three));
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
        assert_eq!(sprite.status, SourceSpriteStatus::Complete);
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
                .any(|asset| asset.name() == "box-shape")
        );
        assert_eq!(
            source_sprite.resolved_shape_rows,
            vec!["010".to_string(), "111".to_string(), "010".to_string()]
        );
    }

    #[test]
    fn source_sprite_contract_preserves_tagged_shape_table_structure() {
        let source = r##"
puzzle board {
tags {
kind = A B
}
slots {
actor = Box:kind
}
sprites {
shapes {
foo:kind {
A {
010
111
010
}
 B {
111
010
111
}
}
}
Box:kind {
#111 #eee
shape foo:kind
}
}
}
"##;
        let cursor = source.find("shape foo:kind").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Box:kind");
        let source_sprite = target
            .source_sprite
            .as_ref()
            .expect("source sprite contract");
        assert_eq!(source_sprite.shape_ref.as_deref(), Some("foo:kind"));
        let table = source_sprite
            .shape_assets
            .iter()
            .find(|asset| asset.name() == "foo")
            .expect("shape table asset");
        let SourceSpriteShapeAsset::Table { axis, variants, .. } = table else {
            panic!("tagged shape must remain a table");
        };
        assert_eq!(axis, "kind");
        assert_eq!(
            variants.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["A", "B"]
        );
        assert!(source_sprite.resolved_shape_rows.is_empty());
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
dimension = 3
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

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.dimension, Some(crate::ModelDimension::Three));
        assert_eq!(target.name, "Floor");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#90ee90"));
        assert!(body.contains("11111"));
        assert!(!body.contains("Goal"));
        let sprite3d = target.source_sprite.as_ref().unwrap();
        assert_eq!(sprite3d.status, SourceSpriteStatus::Complete);
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
dimension = 3
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

        assert_eq!(sprite.status, SourceSpriteStatus::Complete);
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
                .any(|asset| asset.name() == "pulse")
        );
    }

    #[test]
    fn resolves_second_stacked_sprite_entry_as_sprite3d() {
        let source = r##"
puzzle board {
dimension = 3
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

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.dimension, Some(crate::ModelDimension::Three));
        assert_eq!(target.name, "Goal");
    }

    #[test]
    fn resolves_unfinished_sprite3d_name_as_sprite3d_target() {
        let source = r#"
puzzle board {
dimension = 3
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

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.dimension, Some(crate::ModelDimension::Three));
        assert_eq!(target.name, "Floor");
        assert!(
            source[target.body_start.unwrap()..target.body_end.unwrap()]
                .trim()
                .is_empty()
        );
        assert_eq!(
            target.source_sprite.as_ref().unwrap().status,
            SourceSpriteStatus::Incomplete
        );
    }

    #[test]
    fn unfinished_sprite3d_stops_before_next_entry_header() {
        let source = r#"
puzzle board {
dimension = 3
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

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.dimension, Some(crate::ModelDimension::Three));
        assert_eq!(target.name, "Goal");
    }
}
