use crate::{LevelEditorIntegration, SourceTarget, SourceTargetKind, VisualDef, VisualKind};
use serde_json::{Value, json};

const CONTRACT_VERSION: usize = 2;

/// The editor-facing projection of the level authoring integration.
///
/// This module intentionally owns only the projection. Parsing, object identity,
/// legend resolution, and level integration remain in the language layer. The
/// projection is deliberately split into a small manifest, per-level slot buffers,
/// and per-object renderer payloads so a browser never receives a whole game-sized
/// JSON snapshot merely to open one level.
pub(crate) fn level_editor_manifest_json(
    parsed: &LevelEditorIntegration,
    entries: &[SourceTarget],
) -> Result<String, String> {
    let level_targets = entries
        .iter()
        .filter(|entry| {
            entry.kind == SourceTargetKind::Level
                && entry.dimension == Some(crate::ModelDimension::Two)
        })
        .collect::<Vec<_>>();
    let objects = parsed
        .catalog
        .object_defs
        .iter()
        .filter(|object| object.layer_id.0 != crate::UNASSIGNED_LAYER)
        .map(|object| {
            json!({
                "id": object.id.0,
                "name": object_label(parsed, object.id.0),
                "layer": object.layer_id.0,
            })
        })
        .collect::<Vec<_>>();
    let levels = parsed
        .levels
        .iter()
        .enumerate()
        .map(|(index, level)| {
            let target = level_targets.get(level.source_level_index).copied();
            json!({
                "name": level.name,
                "start": target.map_or(0, |target| target.start),
                "end": target.map_or(0, |target| target.end),
                "levelIndex": index,
                "sourceLevelIndex": target.and_then(|target| target.level_index).unwrap_or(level.source_level_index),
                "width": level.state.width,
                "height": level.state.height,
                "layerCount": level.state.layer_count,
                "authoredLayerCount": level.layers.len(),
                "regions": level.regions,
                "legend": legend_value(parsed, &level.char_objects),
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&json!({
        "version": CONTRACT_VERSION,
        "kind": "puzzle2d-level-editor",
        "objects": objects,
        "legend": legend_value(parsed, &parsed.catalog.char_objects),
        "levels": levels,
        "diagnostics": parsed.diagnostics,
    }))
    .map_err(|error| format!("failed to serialize level editor manifest: {error}"))
}

/// Returns the canonical object IDs for one integrated level state. `None` selects
/// the composite state; `Some(n)` selects one authored ASCII layer.
pub(crate) fn level_editor_level_slots(
    parsed: &LevelEditorIntegration,
    level_index: usize,
    authored_layer: Option<usize>,
) -> Result<Vec<u32>, String> {
    let level = parsed
        .levels
        .get(level_index)
        .ok_or_else(|| format!("level editor level index out of range: {level_index}"))?;
    let state = match authored_layer {
        Some(layer_index) => level.layers.get(layer_index).ok_or_else(|| {
            format!(
                "level editor authored layer index out of range: level {level_index}, layer {layer_index}"
            )
        })?,
        None => &level.state,
    };
    Ok(state
        .slots()
        .iter()
        .map(|object| u32::from(object.0))
        .collect())
}

/// Returns the renderer-ready payload for one object. `null` means that the object
/// has no visual binding; it is not substituted with a guessed visual.
pub(crate) fn level_editor_visual_payload_json(
    parsed: &LevelEditorIntegration,
    object_id: u16,
) -> Result<String, String> {
    let object_name = object_label(parsed, object_id);
    let Some(alias) = parsed
        .visuals
        .aliases
        .iter()
        .find(|alias| alias.object == object_name)
    else {
        return Ok("null".to_string());
    };
    let visual = parsed
        .visuals
        .entries
        .iter()
        .find(|visual| visual.name == alias.visual)
        .ok_or_else(|| {
            format!(
                "level editor visual binding for `{object_name}` references missing visual `{}`",
                alias.visual
            )
        })?;
    serde_json::to_string(&renderer_visual_value(visual)).map_err(|error| {
        format!("failed to serialize level editor visual `{object_name}`: {error}")
    })
}

fn object_label(parsed: &LevelEditorIntegration, object_id: u16) -> String {
    parsed
        .catalog
        .object_labels
        .get(&puzzle_core::ObjectId(object_id))
        .cloned()
        .unwrap_or_else(|| format!("object_{object_id}"))
}

fn legend_value(
    parsed: &LevelEditorIntegration,
    entries: &std::collections::HashMap<char, Vec<puzzle_core::ObjectId>>,
) -> Value {
    let empty_has_objects = parsed
        .empty_char
        .is_some_and(|empty_char| entries.contains_key(&empty_char));
    let mut entries = entries
        .iter()
        .map(|(symbol, objects)| {
            json!({
                "symbol": symbol.to_string(),
                "objectIds": objects.iter().map(|object| object.0).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    if let Some(empty_char) = parsed.empty_char.filter(|_| !empty_has_objects) {
        entries.push(json!({ "symbol": empty_char.to_string(), "objectIds": [] }));
    }
    entries.sort_by(|left, right| left["symbol"].as_str().cmp(&right["symbol"].as_str()));
    entries.dedup_by(|left, right| left["symbol"] == right["symbol"]);
    Value::Array(entries)
}

fn renderer_visual_value(visual: &VisualDef) -> Value {
    let mut value = match &visual.kind {
        VisualKind::Solid(color) => json!({ "colors": { "0": color }, "pattern": ["0"] }),
        VisualKind::Image { source } => json!({ "source": source }),
        VisualKind::Ascii { colors } => json!({
            "colors": colors.iter().map(|color| (color.token.to_string(), Value::String(color.color.clone()))).collect::<serde_json::Map<_, _>>(),
            "pattern": visual.frames.first().and_then(|frame| frame.planes.first()).cloned().unwrap_or_default(),
        }),
    };
    let object = value
        .as_object_mut()
        .expect("renderer visual payload must be an object");
    if !visual.transforms.is_empty() {
        object.insert("transforms".to_string(), json!(visual.transforms));
    }
    if visual.fit != Default::default() {
        object.insert("fit".to_string(), json!(visual.fit));
    }
    if let Some(sampling) = visual.sampling {
        object.insert("sampling".to_string(), json!(sampling));
    }
    if let Some(duration_ms) = visual.animation_duration_ms {
        object.insert("durationMs".to_string(), json!(duration_ms));
        object.insert(
            "frames".to_string(),
            json!(
                visual
                    .frames
                    .iter()
                    .filter_map(|frame| frame.planes.first())
                    .collect::<Vec<_>>()
            ),
        );
    }
    if let Some(pixels_per_cell) = visual.pixels_per_cell {
        object.insert(
            "pixelsPerCell".to_string(),
            json!({ "width": pixels_per_cell.width, "height": pixels_per_cell.height }),
        );
    }
    value
}

#[cfg(test)]
mod tests {
    #[test]
    fn contract_projection_does_not_parse_source_syntax() {
        let source = include_str!("level_editor_source.rs");
        for parts in [
            ["split_header", "_tokens"],
            ["strip_line", "_comment"],
            ["Source", "Scope"],
            ["legend_block_row", "_syntax"],
            ["parse_layers", "_block"],
            ["parse_legend", "_block"],
        ] {
            let forbidden = parts.concat();
            assert!(
                !source.contains(&forbidden),
                "level editor contract projection must not parse through {forbidden}"
            );
        }
    }
}
