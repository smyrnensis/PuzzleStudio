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
        unique_name(entries, kind, original)
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
        let source = "puzzle3 world {\n  layers { solid = Box }\n}\n\nsprites art of world {\nBox {\ncolors = red\nshape = {\n0\n}\n}\n}\n";
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
}
