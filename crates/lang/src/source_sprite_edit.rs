use serde::Deserialize;

use crate::source_target::{resolve_source_entries_from_document, sprite_blocks};
use crate::{SourceTargetKind, parse_surface_source_target_document};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteEditMutationResult {
    pub source: String,
    pub start: usize,
    pub end: usize,
    pub name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpriteEditRequest {
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
    spatial_ops: Option<Vec<SpriteEditSpatialOp>>,
    color_bindings: Option<Vec<SpriteEditColorBinding>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpriteEditColorBinding {
    name: String,
    color: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum SpriteEditSpatialOp {
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

pub fn mutate_sprite_source(
    source: &str,
    request_json: &str,
) -> Result<SpriteEditMutationResult, String> {
    let request: SpriteEditRequest = serde_json::from_str(request_json)
        .map_err(|error| format!("invalid sprite edit request: {error}"))?;
    let kind = match request.dimension.as_str() {
        "2d" => SourceTargetKind::Sprite,
        "3d" => SourceTargetKind::Sprite3d,
        other => return Err(format!("unknown sprite edit dimension `{other}`")),
    };
    let document = parse_surface_source_target_document(source);
    let entries = resolve_source_entries_from_document(source, &document);
    match request.operation.as_str() {
        "insert" => insert_sprite(source, &document, &entries, kind, &request, false),
        "insertEmpty" => insert_sprite(source, &document, &entries, kind, &request, true),
        "update" => replace_sprite(source, &entries, kind, &request, false),
        "duplicate" => replace_sprite(source, &entries, kind, &request, true),
        other => Err(format!("unknown sprite edit operation `{other}`")),
    }
}

fn replace_sprite(
    source: &str,
    entries: &[crate::SourceTarget],
    kind: SourceTargetKind,
    request: &SpriteEditRequest,
    duplicate: bool,
) -> Result<SpriteEditMutationResult, String> {
    let original = request
        .original_name
        .as_deref()
        .or(request.name.as_deref())
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "sprite edit requires an original name".to_string())?;
    let owned_source = if !duplicate {
        let mutated = mutate_linked_definitions(source, request)?;
        (mutated != source).then_some(mutated)
    } else {
        None
    };
    let source = owned_source.as_deref().unwrap_or(source);
    let entries = if owned_source.is_some() {
        let document = parse_surface_source_target_document(source);
        resolve_source_entries_from_document(source, &document)
    } else {
        entries.to_vec()
    };
    let matching = entries
        .iter()
        .filter(|entry| entry.kind == kind && entry.name == original)
        .collect::<Vec<_>>();
    let [target] = matching.as_slice() else {
        return Err(format!(
            "sprite `{original}` must resolve to exactly one source entry"
        ));
    };
    let name = if duplicate {
        unique_name(&entries, kind, original)
    } else {
        request.name.clone().unwrap_or_else(|| original.to_string())
    };
    let text = serialize_sprite(request, &name)?;
    if duplicate {
        let position = line_end_after(source, target.end);
        return insert_text(source, position, &text, name);
    }
    let mut next = String::with_capacity(source.len() - (target.end - target.start) + text.len());
    next.push_str(&source[..target.start]);
    next.push_str(&text);
    next.push_str(&source[target.end..]);
    Ok(SpriteEditMutationResult {
        source: next,
        start: target.start,
        end: target.start + text.len(),
        name,
    })
}

fn mutate_linked_definitions(source: &str, request: &SpriteEditRequest) -> Result<String, String> {
    let mut next = source.to_string();
    for binding in request.color_bindings.as_deref().unwrap_or_default() {
        next = replace_named_color_definition(&next, binding)?;
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
        next = replace_named_shape_definition(
            &next,
            shape_name,
            frames,
            request.palette.as_ref().map_or(0, Vec::len),
        )?;
    }
    Ok(next)
}

fn replace_named_color_definition(
    source: &str,
    binding: &SpriteEditColorBinding,
) -> Result<String, String> {
    let document = parse_surface_source_target_document(source);
    let matches = document
        .lines
        .iter()
        .filter(|line| {
            line.scope == Some(crate::source::SourceScope::VisualColorTable)
                && line
                    .tokens
                    .first()
                    .is_some_and(|token| token == &binding.name)
        })
        .collect::<Vec<_>>();
    let [line] = matches.as_slice() else {
        return Err(format!(
            "named color `{}` must resolve to exactly one definition",
            binding.name
        ));
    };
    let line_end = (line.start + line.content.len()).min(source.len());
    let end = if source.as_bytes().get(line_end) == Some(&b'\n') {
        line_end + 1
    } else {
        line_end
    };
    let indent = &source[line.start
        ..line.start + source[line.start..end].len() - source[line.start..end].trim_start().len()];
    let replacement = format!("{indent}{} = {}\n", binding.name, binding.color);
    Ok(format!(
        "{}{}{}",
        &source[..line.start],
        replacement,
        &source[end..]
    ))
}

fn replace_named_shape_definition(
    source: &str,
    name: &str,
    frames: &[Vec<Vec<Vec<Option<usize>>>>],
    palette_len: usize,
) -> Result<String, String> {
    let document = parse_surface_source_target_document(source);
    let matches = document
        .lines
        .iter()
        .filter_map(|line| {
            if line.scope != Some(crate::source::SourceScope::VisualShapeTable)
                || !line.tokens.first().is_some_and(|token| token == name)
            {
                return None;
            }
            let end = (line.start + line.content.len()).min(source.len());
            let open = source[line.start..end]
                .find('{')
                .map(|offset| line.start + offset)?;
            let close = crate::source_target::find_matching_brace(source, open)?;
            Some((line.start, close + 1))
        })
        .collect::<Vec<_>>();
    let [(start, end)] = matches.as_slice() else {
        return Err(format!(
            "named shape `{name}` must resolve to exactly one definition"
        ));
    };
    let indent = &source
        [*start..*start + source[*start..*end].len() - source[*start..*end].trim_start().len()];
    let mut lines = vec![format!("{indent}{name} {{")];
    serialize_shape_frames(&mut lines, frames, palette_len, indent)?;
    lines.push(format!("{indent}}}"));
    Ok(format!(
        "{}{}{}",
        &source[..*start],
        lines.join("\n"),
        &source[*end..]
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

fn insert_sprite(
    source: &str,
    document: &crate::surface::SurfaceDocument,
    entries: &[crate::SourceTarget],
    kind: SourceTargetKind,
    request: &SpriteEditRequest,
    empty: bool,
) -> Result<SpriteEditMutationResult, String> {
    let cursor = request.cursor.unwrap_or(source.len()).min(source.len());
    let blocks = sprite_blocks(source, document);
    let block = blocks
        .iter()
        .find(|block| block.kind == kind && cursor > block.open_index && cursor < block.close_index)
        .or_else(|| blocks.iter().find(|block| block.kind == kind));
    if empty {
        if let Some(block) = block {
            return insert_text(source, block.close_index, "", String::new());
        }
        let text = "sprites {\n\n}\n";
        return insert_text(source, line_end_after(source, cursor), text, String::new());
    }
    let name = request
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "sprite insert requires a name".to_string())?;
    if entries
        .iter()
        .any(|entry| entry.kind == kind && entry.name == name)
    {
        return Err(format!("sprite `{name}` already exists"));
    }
    let text = serialize_sprite(request, &name)?;
    if let Some(block) = block {
        return insert_text(source, block.close_index, &text, name);
    }
    let header = format!("sprites {{\n{text}\n}}\n");
    insert_text(source, source.len(), &header, name).map(|mut result| {
        result.start += "sprites {\n".len();
        result.end = result.start + text.len();
        result
    })
}

fn serialize_sprite(request: &SpriteEditRequest, name: &str) -> Result<String, String> {
    let palette = request
        .palette
        .as_ref()
        .filter(|palette| !palette.is_empty())
        .ok_or_else(|| "sprite edit requires a non-empty palette".to_string())?;
    let mut lines = vec![format!("{name} {{")];
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
            .ok_or_else(|| "sprite edit requires frames or a shape reference".to_string())?;
        lines.push("shape = {".to_string());
        for (frame_index, layers) in frames.iter().enumerate() {
            if frame_index > 0 {
                lines.push(">".to_string());
            }
            if request.dimension == "2d" && layers.len() != 1 {
                return Err("2D sprite edit requires exactly one Z layer per frame".to_string());
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
    op: &SpriteEditSpatialOp,
    dimension: &str,
) -> Result<(), String> {
    match op {
        SpriteEditSpatialOp::Translate2 { space, value } if dimension == "2d" => {
            lines.extend([
                "translate {".to_string(),
                format!("space = {}", checked_space(space)?),
                format!("value = ({}, {})", value[0], value[1]),
                "}".to_string(),
            ]);
        }
        SpriteEditSpatialOp::Rotate2 { space, degrees } if dimension == "2d" => {
            lines.extend([
                "rotate {".to_string(),
                format!("space = {}", checked_space(space)?),
                format!("angle = {degrees}deg"),
                "}".to_string(),
            ]);
        }
        SpriteEditSpatialOp::Translate3 { space, value } if dimension == "3d" => {
            lines.extend([
                "translate {".to_string(),
                format!("space = {}", checked_space(space)?),
                format!("value = ({}, {}, {})", value[0], value[1], value[2]),
                "}".to_string(),
            ]);
        }
        SpriteEditSpatialOp::Rotate3 {
            space,
            axis,
            degrees,
        } if dimension == "3d" => {
            if axis.iter().all(|value| *value == 0.0) {
                return Err("3D sprite rotate axis cannot be zero".to_string());
            }
            lines.extend([
                "rotate {".to_string(),
                format!("space = {}", checked_space(space)?),
                format!("angle = {degrees}deg"),
                format!("axis = ({}, {}, {})", axis[0], axis[1], axis[2]),
                "}".to_string(),
            ]);
        }
        _ => return Err("sprite spatial operation does not match its dimension".to_string()),
    }
    Ok(())
}

fn checked_space(space: &str) -> Result<&str, String> {
    match space {
        "world" | "local" => Ok(space),
        _ => Err(format!("unknown sprite space `{space}`")),
    }
}

fn palette_char(cell: Option<usize>, palette_len: usize) -> Result<char, String> {
    const KEYS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    match cell {
        None => Ok('.'),
        Some(index) if index < palette_len && index < KEYS.len() => Ok(KEYS[index] as char),
        Some(index) => Err(format!(
            "sprite cell palette index `{index}` is out of range"
        )),
    }
}

fn unique_name(entries: &[crate::SourceTarget], kind: SourceTargetKind, original: &str) -> String {
    let base = original.strip_suffix("_copy").unwrap_or(original);
    for index in 1..=10_000 {
        let name = if index == 1 {
            format!("{base}_copy")
        } else {
            format!("{base}_copy_{index}")
        };
        if !entries
            .iter()
            .any(|entry| entry.kind == kind && entry.name == name)
        {
            return name;
        }
    }
    unreachable!("finite source cannot contain every generated duplicate name")
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
) -> Result<SpriteEditMutationResult, String> {
    if !source.is_char_boundary(position) {
        return Err("sprite insertion is not on a UTF-8 boundary".to_string());
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
    Ok(SpriteEditMutationResult {
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
        let source = "puzzle3 world {\n  slots { solid = Box }\n}\n\nsprites art of world {\nBox {\ncolors = red\nshape = {\n0\n}\n}\n}\n";
        let request = serde_json::json!({
            "operation": "update",
            "dimension": "3d",
            "name": "Box",
            "originalName": "Box",
            "palette": ["red", "blue"],
            "frames": [[[[0, 1]], [[1, 0]]]]
        });
        let result = mutate_sprite_source(source, &request.to_string()).unwrap();
        assert!(result.source.contains("shape = {\n01\n-\n10\n}"));
        assert!(!result.source.contains("sprites3"));
    }

    #[test]
    fn mutation_rejects_multiple_z_layers_for_2d() {
        let source = "puzzle world {}\n\nsprites art of world {\nBox {\ncolors = red\nshape = {\n0\n}\n}\n}\n";
        let request = serde_json::json!({
            "operation": "update",
            "dimension": "2d",
            "name": "Box",
            "originalName": "Box",
            "palette": ["red"],
            "frames": [[[[0]], [[0]]]]
        });
        assert!(
            mutate_sprite_source(source, &request.to_string())
                .unwrap_err()
                .contains("exactly one Z layer")
        );
    }

    #[test]
    fn update_mutates_linked_definitions_and_preserves_bindings() {
        let source = "puzzle3 world {\n}\n\nsprites art of world {\npalette {\naccent = red\n}\nshapes {\nbox_shape {\n0\n}\n}\nBox {\ncolors = accent\nshape = box_shape\n}\nOther {\ncolors = accent\nshape = box_shape\n}\n}\n";
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

        let result = mutate_sprite_source(source, &request.to_string()).unwrap();
        assert!(result.source.contains("accent = #123456"));
        assert!(result.source.contains("box_shape {\n0\n-\n0\n}"));
        assert_eq!(result.source.matches("colors = accent").count(), 2);
        assert_eq!(result.source.matches("shape = box_shape").count(), 2);
    }
}
