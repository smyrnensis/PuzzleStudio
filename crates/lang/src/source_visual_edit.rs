use serde::Deserialize;

use crate::source_target::resolve_source_entries_from_document;
use crate::{SourceTargetKind, parse_surface_source_target_document_for_owner_dimension};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualEditMutationResult {
    pub source: String,
    pub start: usize,
    pub end: usize,
    pub name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VisualEditRequest {
    operation: String,
    dimension: String,
    name: Option<String>,
    original_name: Option<String>,
    cursor: Option<usize>,
    palette: Option<Vec<String>>,
    frames: Option<Vec<Vec<Vec<Vec<Option<usize>>>>>>,
    duration_ms: Option<u64>,
    frame_duration_ms: Option<u64>,
    shape_ref: Option<String>,
    prelude_rows: Option<Vec<String>>,
    spatial_ops: Option<Vec<VisualEditSpatialOp>>,
    color_bindings: Option<Vec<VisualEditColorBinding>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VisualEditColorBinding {
    name: String,
    color: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum VisualEditSpatialOp {
    Translate2 {
        space: String,
        value: [f64; 2],
    },
    Rotate2 {
        space: String,
        degrees: f64,
    },
    Translate3 {
        space: String,
        value: [f64; 3],
    },
    Rotate3 {
        space: String,
        axis: [f64; 3],
        degrees: f64,
    },
}

pub fn mutate_visual_source(
    source: &str,
    request_json: &str,
) -> Result<VisualEditMutationResult, String> {
    let request: VisualEditRequest = serde_json::from_str(request_json)
        .map_err(|error| format!("invalid visual edit request: {error}"))?;
    let dimension = match request.dimension.as_str() {
        "2d" => crate::ModelDimension::Two,
        "3d" => crate::ModelDimension::Three,
        other => return Err(format!("unknown visual edit dimension `{other}`")),
    };
    let document = parse_surface_source_target_document_for_owner_dimension(source, dimension);
    let entries = resolve_source_entries_from_document(&document);
    match request.operation.as_str() {
        "insert" => insert_visual(source, &document, &entries, dimension, &request, false),
        "insertEmpty" => insert_visual(source, &document, &entries, dimension, &request, true),
        "update" => replace_visual(source, &document, &entries, dimension, &request, false),
        "duplicate" => replace_visual(source, &document, &entries, dimension, &request, true),
        other => Err(format!("unknown visual edit operation `{other}`")),
    }
}

fn replace_visual(
    source: &str,
    document: &crate::surface::SurfaceDocument,
    entries: &[crate::SourceTarget],
    dimension: crate::ModelDimension,
    request: &VisualEditRequest,
    duplicate: bool,
) -> Result<VisualEditMutationResult, String> {
    let original = request
        .original_name
        .as_deref()
        .or(request.name.as_deref())
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "visual edit requires an original name".to_string())?;
    let matching = matching_visual_targets(entries, dimension, original);
    let [target] = matching.as_slice() else {
        return Err(format!(
            "visual `{original}` must resolve to exactly one source entry"
        ));
    };
    let owned_source = if !duplicate {
        let mutated =
            mutate_linked_definitions(source, document, target, dimension, original, request)?;
        (mutated != source).then_some(mutated)
    } else {
        None
    };
    let source = owned_source.as_deref().unwrap_or(source);
    let entries = if owned_source.is_some() {
        let document = parse_surface_source_target_document_for_owner_dimension(source, dimension);
        resolve_source_entries_from_document(&document)
    } else {
        entries.to_vec()
    };
    let matching = matching_visual_targets(&entries, dimension, original);
    let [target] = matching.as_slice() else {
        return Err(format!(
            "visual `{original}` must resolve to exactly one source entry"
        ));
    };
    let name = if duplicate {
        duplicate_name(&entries, dimension, original, request.name.as_deref())?
    } else {
        request.name.clone().unwrap_or_else(|| original.to_string())
    };
    let text = serialize_visual(request, &name)?;
    if duplicate {
        let position = line_end_after(source, target.end);
        return insert_text(source, position, &text, name);
    }
    let mut next = String::with_capacity(source.len() - (target.end - target.start) + text.len());
    next.push_str(&source[..target.start]);
    next.push_str(&text);
    next.push_str(&source[target.end..]);
    Ok(VisualEditMutationResult {
        source: next,
        start: target.start,
        end: target.start + text.len(),
        name,
    })
}

fn matching_visual_targets<'a>(
    entries: &'a [crate::SourceTarget],
    dimension: crate::ModelDimension,
    name: &str,
) -> Vec<&'a crate::SourceTarget> {
    entries
        .iter()
        .filter(|entry| {
            entry.kind == SourceTargetKind::Visual
                && entry.dimension == Some(dimension)
                && entry.name == name
        })
        .collect()
}

fn mutate_linked_definitions(
    source: &str,
    document: &crate::surface::SurfaceDocument,
    target: &crate::SourceTarget,
    dimension: crate::ModelDimension,
    original: &str,
    request: &VisualEditRequest,
) -> Result<String, String> {
    let mut next = source.to_string();
    let mut current_document = document.clone();
    let mut current_target = target.clone();
    for binding in request.color_bindings.as_deref().unwrap_or_default() {
        next = upsert_named_color_definition(&next, &current_document, &current_target, binding)?;
        (current_document, current_target) = reparsed_visual_target(&next, dimension, original)?;
    }
    if let Some(shape_name) = request
        .shape_ref
        .as_deref()
        .filter(|name| !name.trim().is_empty())
    {
        let frames = request
            .frames
            .as_ref()
            .filter(|frames| !frames.is_empty())
            .ok_or_else(|| "linked shape update requires frames".to_string())?;
        next = upsert_named_shape_definition(
            &next,
            &current_document,
            &current_target,
            shape_name,
            frames,
            request.palette.as_ref().map_or(0, Vec::len),
        )?;
    }
    Ok(next)
}

fn reparsed_visual_target(
    source: &str,
    dimension: crate::ModelDimension,
    name: &str,
) -> Result<(crate::surface::SurfaceDocument, crate::SourceTarget), String> {
    let document = parse_surface_source_target_document_for_owner_dimension(source, dimension);
    let entries = resolve_source_entries_from_document(&document);
    let matching = matching_visual_targets(&entries, dimension, name);
    let [target] = matching.as_slice() else {
        return Err(format!(
            "visual `{name}` must resolve to exactly one source entry after asset mutation"
        ));
    };
    Ok((document, (*target).clone()))
}

fn visual_resource_for_target<'a>(
    document: &'a crate::surface::SurfaceDocument,
    target: &crate::SourceTarget,
) -> Result<&'a crate::surface::SurfaceVisualResourceProduct, String> {
    let matching = document
        .visual_resources
        .iter()
        .filter(|resource| resource.open_brace < target.start && target.end <= resource.close_brace)
        .collect::<Vec<_>>();
    let [resource] = matching.as_slice() else {
        return Err(format!(
            "visual `{}` must belong to exactly one visuals resource",
            target.name
        ));
    };
    Ok(resource)
}

fn upsert_named_color_definition(
    source: &str,
    document: &crate::surface::SurfaceDocument,
    target: &crate::SourceTarget,
    binding: &VisualEditColorBinding,
) -> Result<String, String> {
    let resource = visual_resource_for_target(document, target)?;
    let matches = document
        .visual_color_definitions
        .iter()
        .filter(|definition| {
            definition.name == binding.name
                && resource.span.start <= definition.value_span.start
                && definition.value_span.end <= resource.span.end
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [definition] => replace_source_span(source, definition.value_span, &binding.color),
        [] if binding.name.contains(':') => Err(format!(
            "named color `{}` has no authored table row to update",
            binding.name
        )),
        [] => insert_named_color_definition(source, document, resource, binding),
        _ => Err(format!(
            "named color `{}` must resolve to at most one definition in its visuals resource",
            binding.name
        )),
    }
}

fn insert_named_color_definition(
    source: &str,
    document: &crate::surface::SurfaceDocument,
    resource: &crate::surface::SurfaceVisualResourceProduct,
    binding: &VisualEditColorBinding,
) -> Result<String, String> {
    let blocks = visual_asset_blocks_in_resource(
        document,
        resource,
        crate::surface::SurfaceVisualAssetBlockKind::Palette,
    );
    match blocks.as_slice() {
        [block] => insert_before_close(
            source,
            block.close_brace,
            &format!("{} = {}", binding.name, binding.color),
        ),
        [] => insert_after_open(
            source,
            resource.open_brace,
            &format!("palette {{\n{} = {}\n}}", binding.name, binding.color),
        ),
        _ => Err("visuals resource contains multiple palette blocks".to_string()),
    }
}

fn upsert_named_shape_definition(
    source: &str,
    document: &crate::surface::SurfaceDocument,
    target: &crate::SourceTarget,
    name: &str,
    frames: &[Vec<Vec<Vec<Option<usize>>>>],
    palette_len: usize,
) -> Result<String, String> {
    let resource = visual_resource_for_target(document, target)?;
    let matches = document
        .visual_shape_definitions
        .iter()
        .filter(|definition| {
            definition.name == name
                && resource.span.start <= definition.span.start
                && definition.span.end <= resource.span.end
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [definition] => {
            let indent = source[definition.span.start..definition.span.end]
                .chars()
                .take_while(|ch| matches!(ch, ' ' | '\t'))
                .collect::<String>();
            let mut lines = vec![format!("{indent}{}", definition.header)];
            if definition.braced {
                lines[0].push_str(" {");
            }
            serialize_shape_frames(&mut lines, frames, palette_len, &indent)?;
            if definition.braced {
                lines.push(format!("{indent}}}"));
            }
            replace_source_span(source, definition.span, &lines.join("\n"))
        }
        [] if name.contains(':') => Err(format!(
            "named shape `{name}` has no authored table value to update"
        )),
        [] => insert_named_shape_definition(source, document, resource, name, frames, palette_len),
        _ => Err(format!(
            "named shape `{name}` must resolve to at most one definition in its visuals resource"
        )),
    }
}

fn insert_named_shape_definition(
    source: &str,
    document: &crate::surface::SurfaceDocument,
    resource: &crate::surface::SurfaceVisualResourceProduct,
    name: &str,
    frames: &[Vec<Vec<Vec<Option<usize>>>>],
    palette_len: usize,
) -> Result<String, String> {
    let mut lines = vec![name.to_string()];
    serialize_shape_frames(&mut lines, frames, palette_len, "")?;
    let definition = lines.join("\n");
    let blocks = visual_asset_blocks_in_resource(
        document,
        resource,
        crate::surface::SurfaceVisualAssetBlockKind::Shapes,
    );
    match blocks.as_slice() {
        [block] => insert_before_close(source, block.close_brace, &definition),
        [] => insert_after_open(
            source,
            resource.open_brace,
            &format!("shapes {{\n{definition}\n}}"),
        ),
        _ => Err("visuals resource contains multiple shapes blocks".to_string()),
    }
}

fn visual_asset_blocks_in_resource<'a>(
    document: &'a crate::surface::SurfaceDocument,
    resource: &crate::surface::SurfaceVisualResourceProduct,
    kind: crate::surface::SurfaceVisualAssetBlockKind,
) -> Vec<&'a crate::surface::SurfaceVisualAssetBlockProduct> {
    document
        .visual_asset_blocks
        .iter()
        .filter(|block| {
            block.kind == kind
                && resource.span.start <= block.span.start
                && block.span.end <= resource.span.end
        })
        .collect()
}

fn replace_source_span(
    source: &str,
    span: crate::surface::SourceSpan,
    replacement: &str,
) -> Result<String, String> {
    if span.start > span.end
        || span.end > source.len()
        || !source.is_char_boundary(span.start)
        || !source.is_char_boundary(span.end)
    {
        return Err("visual asset mutation received an invalid source span".to_string());
    }
    Ok(format!(
        "{}{}{}",
        &source[..span.start],
        replacement,
        &source[span.end..]
    ))
}

fn insert_before_close(source: &str, close: usize, text: &str) -> Result<String, String> {
    if close > source.len() || !source.is_char_boundary(close) {
        return Err("visual asset block has an invalid closing offset".to_string());
    }
    Ok(format!(
        "{}\n{}\n{}",
        &source[..close],
        text,
        &source[close..]
    ))
}

fn insert_after_open(source: &str, open: usize, text: &str) -> Result<String, String> {
    let position = open.saturating_add(1);
    if position > source.len() || !source.is_char_boundary(position) {
        return Err("visuals resource has an invalid opening offset".to_string());
    }
    Ok(format!(
        "{}\n{}\n{}",
        &source[..position],
        text,
        &source[position..]
    ))
}

fn serialize_shape_frames(
    lines: &mut Vec<String>,
    frames: &[Vec<Vec<Vec<Option<usize>>>>],
    palette_len: usize,
    indent: &str,
) -> Result<(), String> {
    for (frame_index, layers) in frames.iter().enumerate() {
        if frame_index > 0 {
            lines.push(format!("{indent}>"));
        }
        for (layer_index, rows) in layers.iter().enumerate() {
            if layer_index > 0 {
                lines.push(format!("{indent}-"));
            }
            for row in rows {
                lines.push(format!(
                    "{indent}{}",
                    row.iter()
                        .map(|cell| palette_char(*cell, palette_len))
                        .collect::<Result<String, _>>()?
                ));
            }
        }
    }
    Ok(())
}

fn insert_visual(
    source: &str,
    document: &crate::surface::SurfaceDocument,
    entries: &[crate::SourceTarget],
    dimension: crate::ModelDimension,
    request: &VisualEditRequest,
    empty: bool,
) -> Result<VisualEditMutationResult, String> {
    let cursor = request.cursor.unwrap_or(source.len()).min(source.len());
    let block = document
        .visual_resources
        .iter()
        .filter(|block| block.dimension == dimension)
        .find(|block| cursor > block.open_brace && cursor < block.close_brace)
        .or_else(|| {
            document
                .visual_resources
                .iter()
                .find(|block| block.dimension == dimension)
        });
    if empty {
        if let Some(block) = block {
            return insert_text(source, block.close_brace, "", String::new());
        }
        let text = "visuals {\n\n}\n";
        return insert_text(source, line_end_after(source, cursor), text, String::new());
    }
    let name = request
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "visual insert requires a name".to_string())?;
    if visual_name_exists(entries, dimension, &name) {
        return Err(format!("visual `{name}` already exists"));
    }
    let text = serialize_visual(request, &name)?;
    if let Some(block) = block {
        return insert_text(source, block.close_brace, &text, name);
    }
    let header = format!("visuals {{\n{text}\n}}\n");
    insert_text(source, source.len(), &header, name).map(|mut result| {
        result.start += "visuals {\n".len();
        result.end = result.start + text.len();
        result
    })
}

fn serialize_visual(request: &VisualEditRequest, name: &str) -> Result<String, String> {
    let palette = request
        .palette
        .as_ref()
        .filter(|palette| !palette.is_empty())
        .ok_or_else(|| "visual edit requires a non-empty palette".to_string())?;
    let mut lines = vec![format!("visual {name} {{")];
    for row in request.prelude_rows.as_deref().unwrap_or_default() {
        let row = row.trim();
        if !row.is_empty()
            && !row.starts_with("selector")
            && !row.starts_with("duration")
            && !row.starts_with("frame_duration")
        {
            lines.push(row.to_string());
        }
    }
    if let Some(duration) = request.duration_ms {
        lines.push(format!("duration = {duration}ms"));
    }
    if let Some(duration) = request.frame_duration_ms {
        lines.push(format!("frame_duration = {duration}ms"));
    }
    for op in request.spatial_ops.as_deref().unwrap_or_default() {
        serialize_spatial_op(&mut lines, op, &request.dimension)?;
    }
    lines.push(format!("colors = {}", palette.join(" ")));
    if let Some(shape) = request
        .shape_ref
        .as_deref()
        .filter(|shape| !shape.trim().is_empty())
    {
        lines.push(format!("shape = {}", shape.trim()));
    } else {
        let frames = request
            .frames
            .as_ref()
            .filter(|frames| !frames.is_empty())
            .ok_or_else(|| "visual edit requires frames or a shape reference".to_string())?;
        lines.push("shape = {".to_string());
        for (frame_index, layers) in frames.iter().enumerate() {
            if frame_index > 0 {
                lines.push(">".to_string());
            }
            if request.dimension == "2d" && layers.len() != 1 {
                return Err("2D visual edit requires exactly one Z layer per frame".to_string());
            }
            for (layer_index, rows) in layers.iter().enumerate() {
                if layer_index > 0 {
                    lines.push("-".to_string());
                }
                for row in rows {
                    lines.push(
                        row.iter()
                            .map(|cell| palette_char(*cell, palette.len()))
                            .collect::<Result<String, _>>()?,
                    );
                }
            }
        }
        lines.push("}".to_string());
    }
    lines.push("}".to_string());
    Ok(lines.join("\n"))
}

fn serialize_spatial_op(
    lines: &mut Vec<String>,
    op: &VisualEditSpatialOp,
    dimension: &str,
) -> Result<(), String> {
    match op {
        VisualEditSpatialOp::Translate2 { space, value } if dimension == "2d" => {
            lines.extend([
                "translate {".to_string(),
                format!("space = {}", checked_space(space)?),
                format!("value = ({}, {})", value[0], value[1]),
                "}".to_string(),
            ]);
        }
        VisualEditSpatialOp::Rotate2 { space, degrees } if dimension == "2d" => {
            lines.extend([
                "rotate {".to_string(),
                format!("space = {}", checked_space(space)?),
                format!("angle = {degrees}deg"),
                "}".to_string(),
            ]);
        }
        VisualEditSpatialOp::Translate3 { space, value } if dimension == "3d" => {
            lines.extend([
                "translate {".to_string(),
                format!("space = {}", checked_space(space)?),
                format!("value = ({}, {}, {})", value[0], value[1], value[2]),
                "}".to_string(),
            ]);
        }
        VisualEditSpatialOp::Rotate3 {
            space,
            axis,
            degrees,
        } if dimension == "3d" => {
            if axis.iter().all(|value| *value == 0.0) {
                return Err("3D visual rotate axis cannot be zero".to_string());
            }
            lines.extend([
                "rotate {".to_string(),
                format!("space = {}", checked_space(space)?),
                format!("angle = {degrees}deg"),
                format!("axis = ({}, {}, {})", axis[0], axis[1], axis[2]),
                "}".to_string(),
            ]);
        }
        _ => return Err("visual spatial operation does not match its dimension".to_string()),
    }
    Ok(())
}

fn checked_space(space: &str) -> Result<&str, String> {
    match space {
        "world" | "local" => Ok(space),
        _ => Err(format!("unknown visual space `{space}`")),
    }
}

fn palette_char(cell: Option<usize>, palette_len: usize) -> Result<char, String> {
    const KEYS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    match cell {
        None => Ok('.'),
        Some(index) if index < palette_len && index < KEYS.len() => Ok(KEYS[index] as char),
        Some(index) => Err(format!(
            "visual cell palette index `{index}` is out of range"
        )),
    }
}

fn unique_name(
    entries: &[crate::SourceTarget],
    dimension: crate::ModelDimension,
    original: &str,
) -> String {
    let (object, tags) = original
        .split_once(':')
        .map_or((original, ""), |(object, tags)| (object, tags));
    let base = object.strip_suffix("_copy").unwrap_or(object);
    let tag_suffix = (!tags.is_empty())
        .then(|| format!(":{tags}"))
        .unwrap_or_default();
    for index in 1..=10_000 {
        let name = if index == 1 {
            format!("{base}_copy{tag_suffix}")
        } else {
            format!("{base}_copy_{index}{tag_suffix}")
        };
        if !visual_name_exists(entries, dimension, &name) {
            return name;
        }
    }
    unreachable!("finite source cannot contain every generated duplicate name")
}

fn duplicate_name(
    entries: &[crate::SourceTarget],
    dimension: crate::ModelDimension,
    original: &str,
    requested: Option<&str>,
) -> Result<String, String> {
    let requested = requested.filter(|name| !name.trim().is_empty());
    if let Some(name) = requested.filter(|name| *name != original) {
        if visual_name_exists(entries, dimension, name) {
            return Err(format!("visual `{name}` already exists"));
        }
        return Ok(name.to_string());
    }
    Ok(unique_name(entries, dimension, original))
}

fn visual_name_exists(
    entries: &[crate::SourceTarget],
    dimension: crate::ModelDimension,
    name: &str,
) -> bool {
    entries.iter().any(|entry| {
        entry.kind == SourceTargetKind::Visual
            && entry.dimension == Some(dimension)
            && entry.name == name
    })
}

fn line_end_after(source: &str, position: usize) -> usize {
    source[position.min(source.len())..]
        .find('\n')
        .map(|offset| position + offset + 1)
        .unwrap_or(source.len())
}

fn insert_text(
    source: &str,
    position: usize,
    text: &str,
    name: String,
) -> Result<VisualEditMutationResult, String> {
    if !source.is_char_boundary(position) {
        return Err("visual insertion is not on a UTF-8 boundary".to_string());
    }
    let before = source[..position].trim_end();
    let after = source[position..].trim_start();
    let mut next = String::new();
    next.push_str(before);
    if !next.is_empty() {
        next.push_str("\n\n");
    }
    let start = next.len();
    next.push_str(text.trim_end());
    let end = next.len();
    if !after.is_empty() {
        next.push_str("\n\n");
        next.push_str(after);
    } else {
        next.push('\n');
    }
    Ok(VisualEditMutationResult {
        source: next,
        start,
        end,
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_serializes_common_3d_layers_in_rust() {
        let source = "puzzle world {\n  dimension = 3\n  layers { solid = Box }\n}\n\nvisuals art of world {\nBox {\ncolors = red\nshape = {\n0\n}\n}\n}\n";
        let request = serde_json::json!({
            "operation": "update",
            "dimension": "3d",
            "name": "Box",
            "originalName": "Box",
            "palette": ["red", "blue"],
            "frames": [[[[0, 1]], [[1, 0]]]]
        });
        let result = mutate_visual_source(source, &request.to_string()).unwrap();
        assert!(result.source.contains("shape = {\n01\n-\n10\n}"));
        assert!(!result.source.contains("visuals3"));
    }

    #[test]
    fn mutation_rejects_multiple_z_layers_for_2d() {
        let source = "puzzle world {}\n\nvisuals art of world {\nBox {\ncolors = red\nshape = {\n0\n}\n}\n}\n";
        let request = serde_json::json!({
            "operation": "update",
            "dimension": "2d",
            "name": "Box",
            "originalName": "Box",
            "palette": ["red"],
            "frames": [[[[0]], [[0]]]]
        });
        assert!(
            mutate_visual_source(source, &request.to_string())
                .unwrap_err()
                .contains("exactly one Z layer")
        );
    }

    #[test]
    fn update_mutates_linked_definitions_and_preserves_bindings() {
        let source = "puzzle world {\ndimension = 3\n}\n\nvisuals art of world {\npalette {\naccent = red\n}\nshapes {\nbox_shape {\n0\n}\n}\nBox {\ncolors = accent\nshape = box_shape\n}\nOther {\ncolors = accent\nshape = box_shape\n}\n}\n";
        let request = serde_json::json!({
            "operation": "update",
            "dimension": "3d",
            "name": "Box",
            "originalName": "Box",
            "palette": ["accent"],
            "colorBindings": [{"name": "accent", "color": "#123456"}],
            "shapeRef": "box_shape",
            "frames": [[[[0]], [[0]]]]
        });

        let result = mutate_visual_source(source, &request.to_string()).unwrap();
        assert!(result.source.contains("accent = #123456"));
        assert!(result.source.contains("box_shape {\n0\n-\n0\n}"));
        assert_eq!(result.source.matches("colors = accent").count(), 2);
        assert_eq!(result.source.matches("shape = box_shape").count(), 2);
    }

    #[test]
    fn update_registers_missing_linked_color_and_shape_in_the_visuals_resource() {
        let source =
            "puzzle world {\n}\n\nvisuals {\nvisual Box {\ncolors = red\nshape = {\n0\n}\n}\n}\n";
        let request = serde_json::json!({
            "operation": "update",
            "dimension": "2d",
            "name": "Box",
            "originalName": "Box",
            "palette": ["accent"],
            "colorBindings": [{"name": "accent", "color": "#123456"}],
            "shapeRef": "box_shape",
            "frames": [[[[0]]]]
        });

        let result = mutate_visual_source(source, &request.to_string()).unwrap();
        assert_eq!(result.source.matches("visuals").count(), 1);
        assert!(result.source.contains("palette {\naccent = #123456\n}"));
        assert!(result.source.contains("shapes {\nbox_shape\n0\n}"));
        assert!(result.source.contains("colors = accent"));
        assert!(result.source.contains("shape = box_shape"));
    }

    #[test]
    fn repeated_update_after_normalizing_unbraced_visual_is_idempotent() {
        let source = "puzzle demo {\nvisuals {\nGoal\n#f59e0b #fde68a\n.0.\n111\n.0.\n\nWall\n#334155\n000\n000\n000\n}\n}\n";
        let request = serde_json::json!({
            "operation": "update",
            "dimension": "2d",
            "name": "Goal",
            "originalName": "Goal",
            "palette": ["#f59e0b", "#fde68a"],
            "frames": [[[[null, 0, null], [1, 1, 1], [null, 0, null]]]]
        });

        let first = mutate_visual_source(source, &request.to_string()).unwrap();
        let second = mutate_visual_source(&first.source, &request.to_string()).unwrap();
        assert_eq!(second.source, first.source);
        assert_eq!(first.source.matches('}').count(), 4);
    }

    #[test]
    fn duplicate_of_braced_inline_visual_is_inserted_after_its_closing_brace() {
        let source = "puzzle demo {\nvisuals {\nvisual Goal {\ncolors = red\nshape = {\n0\n}\n}\n\nWall\nblue\n0\n}\n}\n";
        let request = serde_json::json!({
            "operation": "duplicate",
            "dimension": "2d",
            "name": "Goal",
            "originalName": "Goal",
            "palette": ["red"],
            "frames": [[[[0]]]]
        });

        let result = mutate_visual_source(source, &request.to_string()).unwrap();
        assert!(
            result
                .source
                .contains("shape = {\n0\n}\n}\n\nvisual Goal_copy {")
        );
        assert_eq!(result.source.matches("visual Goal").count(), 2);
    }

    #[test]
    fn duplicate_appends_copy_to_the_object_name_before_selector_tags() {
        assert_eq!(
            unique_name(&[], crate::ModelDimension::Two, "Box:red"),
            "Box_copy:red"
        );
        assert_eq!(
            unique_name(&[], crate::ModelDimension::Two, "Box:red:open"),
            "Box_copy:red:open"
        );
    }

    #[test]
    fn duplicate_uses_the_requested_name_when_it_differs_from_the_original() {
        let source =
            "puzzle demo {\n}\n\nvisuals {\nvisual Goal {\ncolors = red\nshape = {\n0\n}\n}\n}\n";
        let request = serde_json::json!({
            "operation": "duplicate",
            "dimension": "2d",
            "name": "Prize",
            "originalName": "Goal",
            "palette": ["red"],
            "frames": [[[[0]]]]
        });

        let result = mutate_visual_source(source, &request.to_string()).unwrap();
        assert_eq!(result.name, "Prize");
        assert!(result.source.contains("visual Prize {"));
        assert!(!result.source.contains("visual Goal_copy {"));
    }

    #[test]
    fn duplicate_rejects_a_requested_name_owned_by_another_visual() {
        let source = "puzzle demo {\n}\n\nvisuals {\nvisual Goal {\ncolors = red\nshape = {\n0\n}\n}\nvisual Prize {\ncolors = blue\nshape = {\n0\n}\n}\n}\n";
        let request = serde_json::json!({
            "operation": "duplicate",
            "dimension": "2d",
            "name": "Prize",
            "originalName": "Goal",
            "palette": ["red"],
            "frames": [[[[0]]]]
        });

        let error = mutate_visual_source(source, &request.to_string()).unwrap_err();
        assert_eq!(error, "visual `Prize` already exists");
    }

    #[test]
    fn insert_reuses_existing_visuals_block() {
        let source = "puzzle demo {\n}\n\nvisuals {\nvisual Existing {\ncolors = blue\nshape = {\n0\n}\n}\n}\n";
        let request = serde_json::json!({
            "operation": "insert",
            "dimension": "2d",
            "name": "Goal",
            "palette": ["red"],
            "frames": [[[[0]]]]
        });

        let result = mutate_visual_source(source, &request.to_string()).unwrap();
        assert_eq!(result.source.matches("visuals").count(), 1);
        assert_eq!(result.source.matches("visual Existing").count(), 1);
        assert!(result.source.contains("visual Existing {"));
        assert!(result.source.contains("\n\nvisual Goal {"));
    }
}
