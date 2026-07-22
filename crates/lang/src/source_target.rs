use crate::PuzzleSourceProfile;
use crate::surface::{SurfaceDocument, SurfaceVisualRefs};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceTargetKind {
    Level,
    Visual,
    Sounds,
}

impl SourceTargetKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Level => "level",
            Self::Visual => "visual",
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
    pub source_visual: Option<SourceVisualDocument>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceVisualDocument {
    pub dimension: crate::ModelDimension,
    pub status: SourceVisualStatus,
    pub prelude_rows: Vec<String>,
    pub palette_tokens: Vec<String>,
    pub resolved_palette: Vec<SourceVisualPaletteEntry>,
    pub palette: Vec<String>,
    pub pixel_rows: Vec<String>,
    pub rows: Vec<String>,
    pub duration_ms: Option<u64>,
    pub frame_duration_ms: Option<u64>,
    pub animation_frames: Vec<Vec<String>>,
    pub shape_ref: Option<String>,
    pub resolved_shape_rows: Vec<String>,
    pub color_assets: Vec<SourceVisualColorAsset>,
    pub shape_assets: Vec<SourceVisualShapeAsset>,
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub depth: Option<usize>,
    pub size: Option<usize>,
    pub cells: Vec<Option<usize>>,
    pub frames: Vec<Vec<Vec<Option<usize>>>>,
    pub transforms: Vec<crate::VisualTransform>,
}

pub type SourceVisualTarget = SourceVisualDocument;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceVisualPaletteEntry {
    pub source: String,
    pub color: String,
    pub linked: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceVisualColorAsset {
    pub name: String,
    pub color: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceVisualShapeAsset {
    Plain {
        name: String,
        frames: Vec<crate::VisualFrameDef>,
    },
    Table {
        name: String,
        axis: String,
        variants: BTreeMap<String, crate::VisualFrameDef>,
    },
}

impl SourceVisualShapeAsset {
    pub fn name(&self) -> &str {
        match self {
            Self::Plain { name, .. } | Self::Table { name, .. } => name,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SourceVisualStatus {
    Complete,
    #[default]
    Incomplete,
    Invalid,
}

impl SourceVisualStatus {
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
    if target.kind == SourceTargetKind::Visual {
        target.source_visual = match target.dimension {
            Some(crate::ModelDimension::Three) => source_visual3d_for_target(document, &target),
            _ => source_visual_for_target(document, &target),
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
    entries.extend(resolve_visual_entries(document));
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
    if let Some(visual) = &target.source_visual {
        out.push_str(",\"sourceVisual\":");
        push_source_visual_json(out, visual);
    }
    out.push('}');
}

fn push_source_visual_json(out: &mut String, visual: &SourceVisualTarget) {
    out.push('{');
    push_json_string(out, "dimension", visual.dimension.as_str());
    out.push(',');
    push_json_string(out, "status", visual.status.as_str());
    out.push(',');
    push_json_string_array(out, "preludeRows", &visual.prelude_rows);
    out.push(',');
    push_json_string_array(out, "paletteTokens", &visual.palette_tokens);
    out.push_str(",\"resolvedPalette\":");
    push_source_visual_palette_json(out, &visual.resolved_palette);
    out.push(',');
    push_json_string_array(out, "pixelRows", &visual.pixel_rows);
    if let Some(duration_ms) = visual.duration_ms {
        out.push(',');
        push_json_number(out, "durationMs", duration_ms as usize);
    }
    if let Some(frame_duration_ms) = visual.frame_duration_ms {
        out.push(',');
        push_json_number(out, "frameDurationMs", frame_duration_ms as usize);
    }
    if !visual.animation_frames.is_empty() {
        out.push_str(",\"animationFrames\":");
        push_json_string_matrix_value(out, &visual.animation_frames);
    }
    out.push_str(",\"shapeRef\":");
    match &visual.shape_ref {
        Some(shape_ref) => push_json_string_value(out, shape_ref),
        None => out.push_str("null"),
    }
    out.push_str(",\"resolvedShapeRows\":");
    push_json_string_array_value(out, &visual.resolved_shape_rows);
    out.push_str(",\"colorAssets\":");
    push_source_visual_color_assets_json(out, &visual.color_assets);
    out.push_str(",\"shapeAssets\":");
    push_source_visual_shape_assets_json(out, &visual.shape_assets);
    out.push_str(",\"extent\":{");
    push_json_number(out, "width", visual.width.unwrap_or(0));
    out.push(',');
    push_json_number(out, "height", visual.height.unwrap_or(0));
    out.push(',');
    push_json_number(out, "depth", visual.depth.unwrap_or(1));
    out.push('}');
    out.push_str(",\"frames\":");
    push_source_visual_edit_frames_json(out, &visual.frames);
    out.push_str(",\"spatialOps\":");
    if visual.dimension == crate::ModelDimension::Two {
        push_source_visual2d_spatial_ops_json(out, &visual.transforms);
    } else {
        push_source_visual3d_spatial_ops_json(out, &visual.transforms);
    }
    out.push('}');
}

fn push_source_visual2d_spatial_ops_json(out: &mut String, ops: &[crate::VisualTransform]) {
    out.push('[');
    for (index, op) in ops.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        match op {
            crate::VisualTransform::Translate { value, space } => write!(
                out,
                "{{\"kind\":\"translate2\",\"space\":\"{}\",\"value\":[{},{}]}}",
                visual_space_name(*space),
                value[0],
                value[1]
            )
            .unwrap(),
            crate::VisualTransform::Rotate { degrees, space, .. } => write!(
                out,
                "{{\"kind\":\"rotate2\",\"space\":\"{}\",\"degrees\":{degrees}}}",
                visual_space_name(*space)
            )
            .unwrap(),
            crate::VisualTransform::Flip { enabled } => {
                write!(out, "{{\"kind\":\"flip2\",\"enabled\":{enabled}}}").unwrap()
            }
        }
    }
    out.push(']');
}

fn visual_space_name(space: crate::VisualSpace) -> &'static str {
    match space {
        crate::VisualSpace::World => "world",
        crate::VisualSpace::Local => "local",
    }
}

fn push_source_visual_edit_frames_json(out: &mut String, frames: &[Vec<Vec<Option<usize>>>]) {
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

fn push_source_visual_palette_json(out: &mut String, entries: &[SourceVisualPaletteEntry]) {
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

fn push_source_visual_color_assets_json(out: &mut String, entries: &[SourceVisualColorAsset]) {
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

fn push_source_visual_shape_assets_json(out: &mut String, entries: &[SourceVisualShapeAsset]) {
    out.push('[');
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        match entry {
            SourceVisualShapeAsset::Plain { name, frames } => {
                out.push('{');
                push_json_string(out, "kind", "plain");
                out.push(',');
                push_json_string(out, "name", name);
                out.push_str(",\"frames\":");
                push_source_visual_shape_frames_json(out, frames);
                out.push('}');
            }
            SourceVisualShapeAsset::Table {
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

fn push_source_visual_shape_frames_json(out: &mut String, frames: &[crate::VisualFrameDef]) {
    out.push('[');
    for (index, frame) in frames.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string_matrix_value(out, &frame.planes);
    }
    out.push(']');
}

fn push_source_visual3d_spatial_ops_json(out: &mut String, ops: &[crate::VisualTransform]) {
    out.push('[');
    for (index, op) in ops.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        match op {
            crate::VisualTransform::Translate { space, value } => write!(out, "{{\"kind\":\"translate3\",\"space\":\"{}\",\"value\":[{},{},{}]}}", visual_space_name(*space), value[0], value[1], value[2]).unwrap(),
            crate::VisualTransform::Rotate { space, axis, degrees } => write!(out, "{{\"kind\":\"rotate3\",\"space\":\"{}\",\"axis\":[{},{},{}],\"degrees\":{degrees}}}", visual_space_name(*space), axis[0], axis[1], axis[2]).unwrap(),
            crate::VisualTransform::Flip { enabled } => write!(out, "{{\"kind\":\"flip3\",\"enabled\":{enabled}}}").unwrap(),
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
            source_visual: None,
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
                source_visual: None,
            }
        })
        .collect()
}

fn resolve_visual_entries(context: &SurfaceDocument) -> Vec<SourceTarget> {
    context
        .visual_products
        .iter()
        .filter(|product| !product.name.is_empty())
        .map(|product| SourceTarget {
            kind: SourceTargetKind::Visual,
            dimension: Some(product.dimension),
            name: product.name.clone(),
            start: product.span.start,
            end: product.span.end,
            body_start: Some(product.body_span.start),
            body_end: Some(product.body_span.end),
            level_index: None,
            sound_kind: None,
            params: Vec::new(),
            source_visual: None,
        })
        .collect()
}

fn source_visual_for_target(
    document: &SurfaceDocument,
    target: &SourceTarget,
) -> Option<SourceVisualTarget> {
    source_visual_target(
        visual_product_for_target(document, target)?,
        &document.visual_refs,
    )
}

fn source_visual3d_for_target(
    document: &SurfaceDocument,
    target: &SourceTarget,
) -> Option<SourceVisualTarget> {
    let product = visual_product_for_target(document, target)?;
    let analyzed = &product.body;
    let syntax = &analyzed.syntax;
    let (prelude_rows, transforms) = source_visual_editor_properties(syntax, &target.name);
    if analyzed.error.is_some() {
        return Some(SourceVisualTarget {
            dimension: crate::ModelDimension::Three,
            status: SourceVisualStatus::Invalid,
            prelude_rows,
            transforms,
            ..SourceVisualTarget::default()
        });
    }
    let palette_tokens = syntax.colors.clone().unwrap_or_default();
    if palette_tokens.is_empty() {
        return Some(SourceVisualTarget {
            dimension: crate::ModelDimension::Three,
            status: SourceVisualStatus::Incomplete,
            prelude_rows,
            transforms,
            ..SourceVisualTarget::default()
        });
    }
    let resolved_palette =
        source_visual_palette_from_refs(&palette_tokens, &document.visual_refs.color_assets);
    if resolved_palette.is_empty() {
        return Some(SourceVisualTarget {
            dimension: crate::ModelDimension::Three,
            status: SourceVisualStatus::Invalid,
            prelude_rows,
            palette_tokens,
            transforms,
            ..SourceVisualTarget::default()
        });
    }
    if palette_tokens.len() > SOURCE_VISUAL3D_PALETTE_KEYS.len() {
        return Some(SourceVisualTarget {
            dimension: crate::ModelDimension::Three,
            status: SourceVisualStatus::Invalid,
            prelude_rows,
            palette_tokens,
            transforms,
            ..SourceVisualTarget::default()
        });
    };
    let shape_ref = match &analyzed.shape {
        crate::visual_authoring::ResolvedVisualShape::Reference(reference) => {
            Some(reference.clone())
        }
        _ => None,
    };
    let frames = match &analyzed.shape {
        crate::visual_authoring::ResolvedVisualShape::Reference(reference) => {
            let asset_name = product.shape_asset_name.as_deref()?;
            match document.visual_refs.shape_assets.get(asset_name)? {
                crate::surface::SurfaceVisualShapeAsset::Plain { frames } => frames.clone(),
                crate::surface::SurfaceVisualShapeAsset::Table { .. } => {
                    return Some(source_visual3d_unresolved_table_target(
                        document,
                        syntax,
                        palette_tokens,
                        resolved_palette,
                        reference.clone(),
                        prelude_rows,
                        transforms,
                    ));
                }
            }
        }
        crate::visual_authoring::ResolvedVisualShape::Inline(frames) => frames.clone(),
        crate::visual_authoring::ResolvedVisualShape::None => {
            vec![crate::visual_authoring::VisualFrameSyntax {
                layers: vec![crate::visual_authoring::VisualLayerSyntax {
                    rows: vec![crate::visual_authoring::VisualShapeRow {
                        text: "0".to_string(),
                        body_line: 0,
                    }],
                }],
            }]
        }
        crate::visual_authoring::ResolvedVisualShape::UnknownBareReference(_)
        | crate::visual_authoring::ResolvedVisualShape::AmbiguousBareRow(_) => {
            return Some(SourceVisualTarget {
                dimension: crate::ModelDimension::Three,
                status: SourceVisualStatus::Invalid,
                prelude_rows,
                palette_tokens,
                transforms,
                ..SourceVisualTarget::default()
            });
        }
    };
    let frame_layers = visual_frame_layers(&frames);
    let Some(layers) = frame_layers.first() else {
        return Some(SourceVisualTarget {
            dimension: crate::ModelDimension::Three,
            status: SourceVisualStatus::Invalid,
            prelude_rows,
            palette_tokens,
            transforms,
            ..SourceVisualTarget::default()
        });
    };
    let mut edit_frames = Vec::with_capacity(frame_layers.len());
    let mut common_size = None;
    for frame in &frame_layers {
        let Some((size, cells)) = source_visual3d_cells(frame, palette_tokens.len()) else {
            return Some(SourceVisualTarget {
                dimension: crate::ModelDimension::Three,
                status: SourceVisualStatus::Invalid,
                prelude_rows,
                palette_tokens,
                transforms,
                ..SourceVisualTarget::default()
            });
        };
        if common_size.is_some_and(|expected| expected != size) {
            return Some(SourceVisualTarget {
                dimension: crate::ModelDimension::Three,
                status: SourceVisualStatus::Invalid,
                prelude_rows,
                palette_tokens,
                transforms,
                ..SourceVisualTarget::default()
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
        .map(|entry| visual_editor_color(&entry.color))
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
    Some(SourceVisualTarget {
        dimension: crate::ModelDimension::Three,
        status: SourceVisualStatus::Complete,
        prelude_rows,
        palette_tokens,
        resolved_palette,
        palette,
        shape_ref,
        color_assets: document
            .visual_refs
            .color_assets
            .iter()
            .map(|(name, color)| SourceVisualColorAsset {
                name: name.clone(),
                color: color.clone(),
            })
            .collect(),
        shape_assets: source_visual_shape_assets(&document.visual_refs),
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
        ..SourceVisualTarget::default()
    })
}

fn visual_product_for_target<'a>(
    document: &'a SurfaceDocument,
    target: &SourceTarget,
) -> Option<&'a crate::surface::SurfaceVisualProduct> {
    document
        .visual_products
        .iter()
        .find(|product| product.span.start == target.start && product.span.end == target.end)
}

fn source_visual_shape_frame(
    frame: &crate::visual_authoring::VisualFrameSyntax,
) -> crate::VisualFrameDef {
    crate::VisualFrameDef {
        planes: frame
            .layers
            .iter()
            .map(|layer| layer.rows.iter().map(|row| row.text.clone()).collect())
            .collect(),
    }
}

fn source_visual_shape_assets(visual_refs: &SurfaceVisualRefs) -> Vec<SourceVisualShapeAsset> {
    visual_refs
        .shape_assets
        .iter()
        .map(|(name, asset)| match asset {
            crate::surface::SurfaceVisualShapeAsset::Plain { frames } => {
                SourceVisualShapeAsset::Plain {
                    name: name.clone(),
                    frames: frames.iter().map(source_visual_shape_frame).collect(),
                }
            }
            crate::surface::SurfaceVisualShapeAsset::Table { axis, variants } => {
                SourceVisualShapeAsset::Table {
                    name: name.clone(),
                    axis: axis.clone(),
                    variants: variants
                        .iter()
                        .map(|(value, frame)| (value.clone(), source_visual_shape_frame(frame)))
                        .collect(),
                }
            }
        })
        .collect()
}

fn source_visual3d_unresolved_table_target(
    document: &SurfaceDocument,
    syntax: &crate::visual_authoring::VisualNodeSyntax,
    palette_tokens: Vec<String>,
    resolved_palette: Vec<SourceVisualPaletteEntry>,
    shape_ref: String,
    prelude_rows: Vec<String>,
    transforms: Vec<crate::VisualTransform>,
) -> SourceVisualTarget {
    SourceVisualTarget {
        dimension: crate::ModelDimension::Three,
        status: SourceVisualStatus::Incomplete,
        prelude_rows,
        palette: resolved_palette
            .iter()
            .map(|entry| visual_editor_color(&entry.color))
            .collect(),
        palette_tokens,
        resolved_palette,
        shape_ref: Some(shape_ref),
        color_assets: document
            .visual_refs
            .color_assets
            .iter()
            .map(|(name, color)| SourceVisualColorAsset {
                name: name.clone(),
                color: color.clone(),
            })
            .collect(),
        shape_assets: source_visual_shape_assets(&document.visual_refs),
        duration_ms: syntax
            .duration
            .as_deref()
            .and_then(|value| puzzle_scene::parse_wait_duration_ms_at(value, value).ok()),
        frame_duration_ms: syntax
            .frame_duration
            .as_deref()
            .and_then(|value| puzzle_scene::parse_wait_duration_ms_at(value, value).ok()),
        transforms,
        ..SourceVisualTarget::default()
    }
}

fn source_shape_rows(asset: &SourceVisualShapeAsset) -> Option<Vec<String>> {
    let SourceVisualShapeAsset::Plain { frames, .. } = asset else {
        return None;
    };
    frames
        .first()
        .and_then(|frame| frame.planes.first())
        .cloned()
}

fn visual_frame_layers(
    frames: &[crate::visual_authoring::VisualFrameSyntax],
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

fn source_visual_target(
    product: &crate::surface::SurfaceVisualProduct,
    visual_refs: &SurfaceVisualRefs,
) -> Option<SourceVisualTarget> {
    let mut target = SourceVisualTarget::default();
    let analyzed = &product.body;
    let product_invalid = analyzed.error.is_some();
    let syntax = &analyzed.syntax;
    target.palette_tokens = syntax.colors.clone().unwrap_or_default();
    (target.prelude_rows, target.transforms) =
        source_visual_editor_properties(syntax, syntax.selector.as_deref().unwrap_or("visual"));
    if let Some(value) = &syntax.duration {
        target.duration_ms = puzzle_scene::parse_wait_duration_ms_at(&value, &value).ok();
    }
    if let Some(value) = &syntax.frame_duration {
        target.frame_duration_ms = puzzle_scene::parse_wait_duration_ms_at(&value, &value).ok();
    }
    match &analyzed.shape {
        crate::visual_authoring::ResolvedVisualShape::Reference(reference) => {
            target.shape_ref = Some(reference.clone());
        }
        crate::visual_authoring::ResolvedVisualShape::Inline(frames) => {
            let frames = crate::visual_authoring::into_single_layer_frames(frames.clone())
                .unwrap_or_default()
                .into_iter()
                .map(|frame| frame.into_iter().map(|row| row.text).collect::<Vec<_>>())
                .collect::<Vec<_>>();
            target.pixel_rows = frames.first().cloned().unwrap_or_default();
            if frames.len() >= 2 {
                target.animation_frames = frames;
            }
        }
        crate::visual_authoring::ResolvedVisualShape::None
        | crate::visual_authoring::ResolvedVisualShape::UnknownBareReference(_)
        | crate::visual_authoring::ResolvedVisualShape::AmbiguousBareRow(_) => {}
    }
    target.color_assets = visual_refs
        .color_assets
        .iter()
        .map(|(name, color)| SourceVisualColorAsset {
            name: name.clone(),
            color: color.clone(),
        })
        .collect();
    target
        .color_assets
        .sort_by(|left, right| left.name.cmp(&right.name));
    target.shape_assets = source_visual_shape_assets(visual_refs);
    target
        .shape_assets
        .sort_by(|left, right| left.name().cmp(right.name()));
    target.resolved_palette =
        source_visual_palette_from_refs(&target.palette_tokens, &visual_refs.color_assets);
    if target.resolved_shape_rows.is_empty() {
        if let Some(shape_ref) = &target.shape_ref {
            if let Some(rows) = target
                .shape_assets
                .iter()
                .find(|asset| asset.name() == shape_ref)
                .and_then(source_shape_rows)
            {
                target.resolved_shape_rows = rows;
            }
        }
    }
    populate_source_visual_edit_frames(&mut target);
    if product_invalid {
        target.status = SourceVisualStatus::Invalid;
    }
    Some(target)
}

fn source_visual_editor_properties(
    syntax: &crate::visual_authoring::VisualNodeSyntax,
    line: &str,
) -> (Vec<String>, Vec<crate::VisualTransform>) {
    match crate::eval_visual_transforms(&syntax.properties, &HashMap::new(), line) {
        Ok(transforms) => {
            let prelude_rows = syntax
                .prelude_rows
                .iter()
                .filter(|row| {
                    !syntax
                        .properties
                        .iter()
                        .any(|(_, property_row)| property_row == *row)
                })
                .cloned()
                .collect();
            (prelude_rows, transforms)
        }
        Err(_) => (syntax.prelude_rows.clone(), Vec::new()),
    }
}

fn populate_source_visual_edit_frames(target: &mut SourceVisualTarget) {
    if target.resolved_palette.is_empty() {
        target.status = SourceVisualStatus::Incomplete;
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
        target.status = SourceVisualStatus::Incomplete;
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
        target.status = SourceVisualStatus::Invalid;
        return;
    }
    let keys = SOURCE_VISUAL3D_PALETTE_KEYS.chars().collect::<Vec<_>>();
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
                    target.status = SourceVisualStatus::Invalid;
                    return;
                };
                if index >= target.resolved_palette.len() {
                    target.status = SourceVisualStatus::Invalid;
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
    target.status = SourceVisualStatus::Complete;
}

fn source_visual_palette_from_refs(
    tokens: &[String],
    color_assets: &BTreeMap<String, String>,
) -> Vec<SourceVisualPaletteEntry> {
    if tokens.is_empty() {
        return Vec::new();
    }
    tokens
        .iter()
        .map(|token| {
            if is_visual_color(token) {
                Some(SourceVisualPaletteEntry {
                    source: token.clone(),
                    color: token.clone(),
                    linked: false,
                })
            } else {
                color_assets
                    .get(token)
                    .map(|color| SourceVisualPaletteEntry {
                        source: token.clone(),
                        color: color.clone(),
                        linked: true,
                    })
            }
        })
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}

fn visual_editor_color(color: &str) -> String {
    if color == "transparent" {
        "#00000000".to_string()
    } else {
        color.to_string()
    }
}

fn source_visual3d_cells(
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
    let keys = SOURCE_VISUAL3D_PALETTE_KEYS
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

const SOURCE_VISUAL3D_PALETTE_KEYS: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn is_visual_color(value: &str) -> bool {
    crate::syntax::is_visual_named_color(value) || is_hex_color(value)
}

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit())
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
        SoundSourceTargetKind, SourceTargetKind, SourceVisualPaletteEntry, SourceVisualShapeAsset,
        SourceVisualStatus, resolve_source_entries_from_document, resolve_source_target,
        resolve_source_target_for_profile,
    };
    use crate::PuzzleSourceProfile;

    #[test]
    fn source_target_consumes_surface_visual_refs_instead_of_collecting_assets() {
        let source = include_str!("source_target.rs");
        let forbidden_fragments = [
            ["struct ", "VisualRefs"],
            ["collect_visual", "_visual_refs"],
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
visuals {
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
visuals {
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
                .any(|entry| { entry.kind == SourceTargetKind::Visual && entry.name == "Player" })
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
            entry.kind == SourceTargetKind::Visual
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
layers {
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
    fn resolves_visual_entry_body() {
        let source = r#"
visuals {
Player {
#000 #fff
.0.
111
}
}
"#;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Player");
        assert!(target.body_start.unwrap() < cursor);
        assert!(target.body_end.unwrap() > cursor);
    }

    #[test]
    fn resolves_unfinished_visual_name_as_visual_target() {
        let source = r#"
visuals {
Player
}
"#;
        let cursor = source.find("Player").unwrap() + "Player".len();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Player");
        assert!(
            source[target.body_start.unwrap()..target.body_end.unwrap()]
                .trim()
                .is_empty()
        );
    }

    #[test]
    fn resolves_unfinished_singular_visual_name_as_visual_target() {
        let source = r#"
visuals {
Player
}
"#;
        let cursor = source.find("Player").unwrap() + "Player".len();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Player");
        assert_eq!(target.body_start, target.body_end);
    }

    #[test]
    fn resolves_unfinished_visual_body_as_visual_target() {
        let source = r##"
visuals {
Player
#f
}
"##;
        let cursor = source.find("#f").unwrap() + 1;
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#f"));
    }

    #[test]
    fn resolves_line_style_visual_named_like_color() {
        let source = r##"
visuals {
red #f00
}
"##;
        let cursor = source.find("#f00").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "red");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert_eq!(body.trim(), "#f00");
    }

    #[test]
    fn resolves_split_visual_named_like_color_after_ascii_body() {
        let source = r##"
visuals {
Player
#000
0
red
#f00
}
"##;
        let cursor = source.find("#f00").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "red");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#f00"));
        assert!(!body.contains("#000"));
    }

    #[test]
    fn color_name_row_stays_in_current_visual_target() {
        let source = r##"
visuals {
Player
red blue
01
}
"##;
        let cursor = source.find("red blue").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("red blue"));
        assert!(body.contains("01"));
    }

    #[test]
    fn animation_visual_rows_resolve_to_visual_target() {
        let source = r##"
visuals {
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

            assert_eq!(target.kind, SourceTargetKind::Visual);
            assert_eq!(target.name, "Player");
            let visual = target.source_visual.unwrap();
            assert_eq!(visual.duration_ms, Some(120));
            assert_eq!(
                visual.animation_frames,
                vec![
                    vec![".0.".to_string(), "111".to_string(), ".0.".to_string()],
                    vec!["111".to_string(), ".0.".to_string(), "111".to_string()],
                ]
            );
        }
    }

    #[test]
    fn animation_visual_frame_duration_resolves_to_visual_target() {
        let source = r##"
visuals {
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
        let visual = target.source_visual.unwrap();

        assert_eq!(visual.frame_duration_ms, Some(60));
        assert_eq!(visual.prelude_rows, vec!["frame_duration 60ms".to_string()]);
    }

    #[test]
    fn explicit_shape_block_resolves_animation_rows_without_shape_ref() {
        let source = r##"
visuals {
visual {
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
        let source_visual = target
            .source_visual
            .as_ref()
            .expect("source visual contract");

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Background");
        assert_eq!(source_visual.shape_ref, None);
        assert!(
            !source_visual
                .prelude_rows
                .iter()
                .any(|row| row.starts_with("shape"))
        );
        assert_eq!(source_visual.duration_ms, Some(500));
        assert_eq!(
            source_visual.animation_frames,
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
    fn user_named_color_row_stays_in_current_visual_target() {
        let source = r##"
visuals {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("accent"));
        assert!(body.contains("0"));
        assert!(!body.contains("Box"));
    }

    #[test]
    fn tagged_visual_color_name_row_stays_in_current_visual_target() {
        let source = r##"
visuals {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "GoalCount:5");
        let source_visual = target
            .source_visual
            .as_ref()
            .expect("source visual contract");
        assert_eq!(source_visual.palette_tokens, vec!["GoalCount".to_string()]);
        assert_eq!(
            source_visual.pixel_rows,
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
    fn visual_source_contract_resolves_palette_from_parser_visuals() {
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

visuals {
palette {
accent = #e94f64
}
visual {
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
        let source_visual = target
            .source_visual
            .as_ref()
            .expect("source visual contract");

        assert_eq!(target.name, "Player");
        assert_eq!(source_visual.palette_tokens, vec!["accent".to_string()]);
        assert_eq!(
            source_visual.resolved_palette,
            vec![SourceVisualPaletteEntry {
                source: "accent".to_string(),
                color: "#e94f64".to_string(),
                linked: true,
            }]
        );
    }

    #[test]
    fn visual_edit_contract_distinguishes_transparent_color_from_empty_cell() {
        let source = r#"
puzzle world { layers { actor = Player } }
visuals art of world {
Player {
colors = transparent red
shape = {
0.1
}
}
}
"#;
        let target = resolve_source_target(source, source.find("0.1").unwrap()).unwrap();
        let visual = target.source_visual.unwrap();
        assert_eq!(visual.status, SourceVisualStatus::Complete);
        assert_eq!(visual.frames, vec![vec![vec![Some(0), None, Some(1)]]]);
        assert_eq!(visual.resolved_palette[0].color, "transparent");
    }

    #[test]
    fn visual_source_contract_preserves_selector_and_duration_rows() {
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

visuals {
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
        let source_visual = target
            .source_visual
            .as_ref()
            .expect("source visual contract");

        assert_eq!(
            source_visual.prelude_rows,
            vec![
                "selector = Player".to_string(),
                "duration 120ms".to_string()
            ]
        );
        assert_eq!(source_visual.duration_ms, Some(120));
    }

    #[test]
    fn visual_source_contract_exposes_animation_frames() {
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

visuals {
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
        let source_visual = target
            .source_visual
            .as_ref()
            .expect("source visual contract");

        assert_eq!(source_visual.duration_ms, Some(120));
        assert_eq!(
            source_visual.pixel_rows,
            vec!["0.".to_string(), "..".to_string()]
        );
        assert_eq!(
            source_visual.animation_frames,
            vec![
                vec!["0.".to_string(), "..".to_string()],
                vec!["..".to_string(), ".0".to_string()],
            ]
        );
    }

    #[test]
    fn consecutive_tagged_visual_color_name_rows_do_not_become_visual_headers() {
        let source = r##"
visuals {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "GoalCount:2");
        let source_visual = target
            .source_visual
            .as_ref()
            .expect("source visual contract");
        assert_eq!(source_visual.palette_tokens, vec!["GoalCount".to_string()]);
        assert_eq!(
            source_visual.pixel_rows,
            vec![
                "....................".to_string(),
                "........00..00......".to_string()
            ]
        );
    }

    #[test]
    fn line_style_visual_accepts_user_named_color() {
        let source = r##"
visuals {
palette {
accent = #e94f64
}
Player accent
}
"##;
        let cursor = source.rfind("accent").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert_eq!(body.trim(), "accent");
    }

    #[test]
    fn unfinished_unbraced_visual_stops_before_next_entry_header() {
        let source = r#"
visuals {
Player
Box
}
"#;
        let cursor = source.find("Box").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Box");
    }

    #[test]
    fn resolves_unbraced_at_visual_before_next_at_visual() {
        let source = r##"
visuals {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "@Floor");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#cfcfcf"));
        assert!(!body.contains("@Shade"));
    }

    #[test]
    fn resolves_unbraced_visual_before_line_style_image_visual() {
        let source = r##"
visuals {
Player
#000 #fff
.0.
111
Box visuals/box.png
}
"##;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("111"));
        assert!(!body.contains("Box visuals/box.png"));
    }

    #[test]
    fn resolves_unbraced_schema_visual_with_color_alias_row() {
        let source = r##"
visuals {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Gate:num");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("Gate_color_1 Gate_color_2"));
        assert!(body.contains(".00000."));
    }

    #[test]
    fn resolves_unbraced_variant_visual_with_color_alias_row() {
        let source = r##"
visuals {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Gate:1");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("Gate_color_1 Gate_color_2"));
        assert!(body.contains(".00000."));
    }

    #[test]
    fn resolves_unbraced_tagged_visual_after_tagged_visual() {
        let source = r##"
visuals {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Box:movable");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#bbb"));
        assert!(!body.contains("#aaa"));
    }

    #[test]
    fn resolves_unbraced_visual_before_split_line_image_visual() {
        let source = r##"
visuals {
Player
#000 #fff
.0.
111
Box
visuals/box.png
}
"##;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("111"));
        assert!(!body.contains("Box"));
        assert!(!body.contains("visuals/box.png"));
    }

    #[test]
    fn resolves_unbraced_visual_with_colors_keyword_palette() {
        let source = r##"
visuals {
Player
colors #000 #fff
.0.
111
}
"##;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("colors #000 #fff"));
        assert!(body.contains("111"));
    }

    #[test]
    fn shape_reference_line_resolves_enclosing_unbraced_visual() {
        let source = r##"
visuals {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Box");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("shape box_shape"));
        assert!(!body.contains("Next"));
    }

    #[test]
    fn bare_shape_reference_uses_shared_owner_scope_resolution() {
        let source = r##"
visuals {
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
        let source_visual = target
            .source_visual
            .as_ref()
            .expect("source visual contract");

        assert_eq!(source_visual.shape_ref.as_deref(), Some("box_shape"));
        assert!(source_visual.pixel_rows.is_empty());
        assert_eq!(
            source_visual.resolved_shape_rows,
            vec!["010".to_string(), "111".to_string(), "010".to_string()]
        );
    }

    #[test]
    fn source_visual_contract_preserves_hyphenated_shape_refs() {
        let source = r##"
visuals {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Box");
        let source_visual = target
            .source_visual
            .as_ref()
            .expect("source visual contract");
        assert_eq!(source_visual.shape_ref.as_deref(), Some("box-shape"));
        assert!(
            source_visual
                .shape_assets
                .iter()
                .any(|asset| asset.name() == "box-shape")
        );
        assert_eq!(
            source_visual.resolved_shape_rows,
            vec!["010".to_string(), "111".to_string(), "010".to_string()]
        );
    }

    #[test]
    fn source_visual_contract_preserves_tagged_shape_table_structure() {
        let source = r##"
puzzle board {
tags {
kind = A B
}
layers {
actor = Box:kind
}
visuals {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Box:kind");
        let source_visual = target
            .source_visual
            .as_ref()
            .expect("source visual contract");
        assert_eq!(source_visual.shape_ref.as_deref(), Some("foo:kind"));
        let table = source_visual
            .shape_assets
            .iter()
            .find(|asset| asset.name() == "foo")
            .expect("shape table asset");
        let SourceVisualShapeAsset::Table { axis, variants, .. } = table else {
            panic!("tagged shape must remain a table");
        };
        assert_eq!(axis, "kind");
        assert_eq!(
            variants.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["A", "B"]
        );
        assert!(source_visual.resolved_shape_rows.is_empty());
    }

    #[test]
    fn rotated_unbraced_visual_prelude_stays_in_current_visual_target() {
        let source = r##"
visuals {
@LockedFrame:directions
rotate directions from up
#000000
....................
.000..000..000..000.
}
"##;
        let cursor = source.find(".000..000").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "@LockedFrame:directions");
        let source_visual = target
            .source_visual
            .as_ref()
            .expect("source visual contract");
        assert_eq!(source_visual.status, SourceVisualStatus::Complete);
        assert_eq!(
            source_visual.prelude_rows,
            vec!["rotate directions from up".to_string()]
        );
        assert!(source_visual.transforms.is_empty());
        assert_eq!(source_visual.palette_tokens, vec!["#000000".to_string()]);
        assert_eq!(
            source_visual.pixel_rows,
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
    fn tag_parameterized_3d_visual_target_preserves_authored_transform() {
        let source = r##"
visuals {
TEN:horizontal {
colors = #ffffff #282828
rotate horizontal from back
shape = {
0
}
}
}
"##;
        let cursor = source.find("TEN:horizontal").unwrap();
        let target =
            resolve_source_target_for_profile(source, cursor, PuzzleSourceProfile::Puzzle3d)
                .expect("3D visual target");
        let source_visual = target.source_visual.expect("source visual contract");

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "TEN:horizontal");
        assert_eq!(source_visual.status, SourceVisualStatus::Complete);
        assert_eq!(source_visual.dimension, crate::ModelDimension::Three);
        assert_eq!(
            source_visual.prelude_rows,
            vec!["rotate horizontal from back".to_string()]
        );
        assert!(source_visual.transforms.is_empty());
        assert_eq!(source_visual.size, Some(1));
        assert_eq!(source_visual.cells, vec![Some(0)]);
    }

    #[test]
    fn shape_table_rows_do_not_resolve_as_visual_targets() {
        let source = r##"
visuals {
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
    fn unbraced_visual_target_end_stops_before_next_colors_keyword_visual() {
        let source = r##"
visuals {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.name, "Player");
        let target_source = &source[target.start..target.end];
        assert!(target_source.contains("111"));
        assert!(!target_source.contains("Box"));
        assert!(!target_source.contains("colors #222"));
    }

    #[test]
    fn resolves_stacked_visual_entry_as_visual3d() {
        let source = r##"
puzzle push3d {
dimension = 3
}
visuals basic of push3d {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.dimension, Some(crate::ModelDimension::Three));
        assert_eq!(target.name, "Floor");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#90ee90"));
        assert!(body.contains("11111"));
        assert!(!body.contains("Goal"));
        let visual3d = target.source_visual.as_ref().unwrap();
        assert_eq!(visual3d.status, SourceVisualStatus::Complete);
        assert_eq!(visual3d.size, Some(5));
        assert_eq!(visual3d.palette, vec!["#90ee90", "#008000"]);
        assert_eq!(visual3d.cells.len(), 125);
        assert_eq!(visual3d.cells[(4 * 5 + 1) * 5 + 2], Some(0));
        assert_eq!(visual3d.cells[(3 * 5) * 5], Some(1));
        assert_eq!(visual3d.cells[(0 * 5 + 1) * 5 + 2], None);
    }

    #[test]
    fn visual3d_contract_preserves_named_color_shape_and_all_animation_frames() {
        let source = r#"
puzzle board {
dimension = 3
}
visuals art of board {
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
        let visual = target.source_visual.unwrap();

        assert_eq!(visual.status, SourceVisualStatus::Complete);
        assert_eq!(visual.shape_ref.as_deref(), Some("pulse"));
        assert_eq!(visual.duration_ms, Some(200));
        assert_eq!(visual.frame_duration_ms, Some(100));
        assert_eq!(visual.frames.len(), 2);
        assert_eq!(visual.frames[0], vec![vec![Some(0)]]);
        assert_eq!(visual.frames[1], vec![vec![None]]);
        assert_eq!(
            visual.resolved_palette,
            vec![SourceVisualPaletteEntry {
                source: "accent".to_string(),
                color: "#123456".to_string(),
                linked: true,
            }]
        );
        assert!(
            visual
                .color_assets
                .iter()
                .any(|asset| asset.name == "accent")
        );
        assert!(
            visual
                .shape_assets
                .iter()
                .any(|asset| asset.name() == "pulse")
        );
    }

    #[test]
    fn resolves_second_stacked_visual_entry_as_visual3d() {
        let source = r##"
puzzle board {
dimension = 3
visuals basic {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.dimension, Some(crate::ModelDimension::Three));
        assert_eq!(target.name, "Goal");
    }

    #[test]
    fn resolves_unfinished_visual3d_name_as_visual3d_target() {
        let source = r#"
puzzle board {
dimension = 3
visuals basic {
Floor {
}
}
}
"#;
        let cursor = source.find("Floor").unwrap() + "Floor".len();
        let target =
            resolve_source_target_for_profile(source, cursor, PuzzleSourceProfile::Puzzle3d)
                .unwrap();

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.dimension, Some(crate::ModelDimension::Three));
        assert_eq!(target.name, "Floor");
        assert!(
            source[target.body_start.unwrap()..target.body_end.unwrap()]
                .trim()
                .is_empty()
        );
        assert_eq!(
            target.source_visual.as_ref().unwrap().status,
            SourceVisualStatus::Incomplete
        );
    }

    #[test]
    fn unfinished_visual3d_stops_before_next_entry_header() {
        let source = r#"
puzzle board {
dimension = 3
visuals basic {
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

        assert_eq!(target.kind, SourceTargetKind::Visual);
        assert_eq!(target.dimension, Some(crate::ModelDimension::Three));
        assert_eq!(target.name, "Goal");
    }
}
