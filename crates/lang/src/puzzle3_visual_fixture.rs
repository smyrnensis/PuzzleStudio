use std::collections::BTreeMap;
use std::fmt::Write;

use crate::{
    LoadedGridGame, SpatialPresentation, ViewportFollow3, ViewportHeight3, ViewportMode3,
    VoxelColor,
};
use puzzle_core::{GridState, ObjectId, Size3};
use puzzle_presentation::{
    VisualComposition, VisualOrderRef, VisualPriorityRef, cell_render_order_3d,
    resolve_object_priority,
};
use puzzle_runtime_contract::{
    RuntimeCoord, RuntimePuzzle3AnimationRender, RuntimePuzzle3Camera,
    RuntimePuzzle3CameraProjection, RuntimePuzzle3Cell, RuntimePuzzle3GridRender,
    RuntimePuzzle3Input, RuntimePuzzle3Object, RuntimePuzzle3ObjectRef,
    RuntimePuzzle3PixelateRender, RuntimePuzzle3Render, RuntimePuzzle3Resources,
    RuntimePuzzle3Size, RuntimePuzzle3SpatialOp, RuntimePuzzle3TweenRender, RuntimePuzzle3Viewport,
    RuntimePuzzle3ViewportFollow, RuntimePuzzle3ViewportFraming, RuntimePuzzle3ViewportHeight,
    RuntimePuzzle3ViewportMode, RuntimePuzzle3Visual, RuntimePuzzle3VisualFrame,
    RuntimePuzzle3VisualOrder, RuntimePuzzle3VisualOrderPriority, RuntimePuzzle3VisualRender,
    RuntimePuzzle3VisualSpace, RuntimeVisualComposition,
};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VisualFixtureExportError {
    MissingLevels,
    MissingObjectName { object: ObjectId },
    MissingObjectLayer { object: ObjectId },
    InvalidVisualOrder { detail: String },
    UnsupportedInputDirection { input: String },
}

pub fn runtime_puzzle3_resources(
    game: &LoadedGridGame<3, Size3>,
    presentation: &SpatialPresentation,
) -> Result<RuntimePuzzle3Resources, VisualFixtureExportError> {
    let visual_order = presentation_order_ref(&presentation.visual_order);
    let object_names = game
        .object_labels
        .iter()
        .map(|(object, label)| (*object, label.clone()))
        .collect::<BTreeMap<_, _>>();
    let inputs = game
        .inputs
        .iter()
        .map(|input| {
            let direction = input
                .direction
                .map(|direction| {
                    input_direction_name(direction.axes())
                        .map(str::to_string)
                        .ok_or_else(|| VisualFixtureExportError::UnsupportedInputDirection {
                            input: input.name.clone(),
                        })
                })
                .transpose()?;
            Ok(RuntimePuzzle3Input {
                id: input.id.0,
                name: input.name.clone(),
                direction,
                keys: input.keys.clone(),
            })
        })
        .collect::<Result<Vec<_>, VisualFixtureExportError>>()?;
    let objects = object_names
        .iter()
        .map(|(object, name)| {
            let layer = game
                .game
                .object_layer(*object)
                .ok_or(VisualFixtureExportError::MissingObjectLayer { object: *object })?;
            let resolved = resolve_object_priority(&visual_order, name).map_err(|error| {
                VisualFixtureExportError::InvalidVisualOrder {
                    detail: format!("{error:?}"),
                }
            })?;
            Ok((
                name.clone(),
                RuntimePuzzle3Object {
                    id: object.0,
                    name: name.clone(),
                    visual: presentation
                        .visual_set
                        .as_ref()
                        .and_then(|visuals| visuals.visual(name))
                        .map(|visual| visual.name.clone()),
                    layer: layer.0,
                    render_priority: u16::try_from(resolved.index).map_err(|_| {
                        VisualFixtureExportError::InvalidVisualOrder {
                            detail: "visual priority index exceeds u16".to_string(),
                        }
                    })?,
                    composition: match resolved.composition {
                        VisualComposition::Ordered => RuntimeVisualComposition::Ordered,
                        VisualComposition::Average => RuntimeVisualComposition::Average,
                    },
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, VisualFixtureExportError>>()?;
    let visuals = presentation
        .visual_set
        .as_ref()
        .map(|visual_set| {
            visual_set
                .visuals
                .iter()
                .map(|visual| {
                    let palette = visual
                        .palette
                        .iter()
                        .map(|(key, color)| {
                            let color = match color {
                                VoxelColor::Transparent => "transparent".to_string(),
                                VoxelColor::Hex(value) => value.clone(),
                            };
                            (key.to_string(), color)
                        })
                        .collect();
                    let frames = visual
                        .frames
                        .iter()
                        .map(|frame| RuntimePuzzle3VisualFrame {
                            layers: frame.slices.clone(),
                        })
                        .collect();
                    let spatial_ops: Vec<RuntimePuzzle3SpatialOp> = visual
                        .transforms
                        .iter()
                        .map(|transform| match transform {
                            crate::VisualTransform::Rotate {
                                space,
                                axis,
                                degrees,
                            } => RuntimePuzzle3SpatialOp::Rotate {
                                space: runtime_visual_space(*space),
                                axis: *axis,
                                degrees: *degrees,
                            },
                            crate::VisualTransform::Translate { space, value } => {
                                RuntimePuzzle3SpatialOp::Translate {
                                    space: runtime_visual_space(*space),
                                    value: *value,
                                }
                            }
                            crate::VisualTransform::Flip { enabled } => {
                                RuntimePuzzle3SpatialOp::Flip { enabled: *enabled }
                            }
                        })
                        .collect();
                    (
                        visual.name.clone(),
                        RuntimePuzzle3Visual {
                            palette,
                            frames,
                            duration_ms: visual.duration_ms,
                            frame_duration_ms: visual.frame_duration_ms,
                            spatial_affine: puzzle_presentation::resolve_spatial_affine(
                                &spatial_ops,
                            )
                            .expect("validated Puzzle3 visual transforms must resolve"),
                        },
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let camera = &game.render.camera;
    let viewport = game
        .render
        .viewport
        .framing
        .map(|framing| RuntimePuzzle3Viewport {
            mode: match game.render.viewport.mode {
                ViewportMode3::Full => RuntimePuzzle3ViewportMode::Full,
                ViewportMode3::Centered => RuntimePuzzle3ViewportMode::Centered,
                ViewportMode3::Paged => RuntimePuzzle3ViewportMode::Paged,
            },
            follow: match game.render.viewport.follow {
                ViewportFollow3::Snap => RuntimePuzzle3ViewportFollow::Snap,
                ViewportFollow3::Smooth => RuntimePuzzle3ViewportFollow::Smooth,
            },
            focus: game.render.viewport.focus.clone(),
            focus_objects: presentation
                .viewport_focus_objects
                .iter()
                .map(|object| object.0)
                .collect(),
            framing_box: RuntimePuzzle3ViewportFraming {
                width: framing.width,
                depth: framing.depth,
                height: match framing.height {
                    ViewportHeight3::Full => RuntimePuzzle3ViewportHeight::Full,
                    ViewportHeight3::Size(height) => RuntimePuzzle3ViewportHeight::Size(height),
                },
            },
        });
    let order = RuntimePuzzle3VisualOrder {
        direction_priority: presentation.visual_order.direction_priority.clone(),
        priorities: presentation
            .visual_order
            .priorities
            .iter()
            .map(|priority| RuntimePuzzle3VisualOrderPriority {
                objects: priority.objects.clone(),
                animations: priority.animations.clone(),
                merge: priority.merge,
            })
            .collect(),
    };
    Ok(RuntimePuzzle3Resources {
        layer_count: game.game.layer_count,
        inputs,
        objects,
        visuals,
        render: RuntimePuzzle3Render {
            camera: RuntimePuzzle3Camera {
                projection: match camera.projection {
                    crate::CameraProjection3::Perspective => {
                        RuntimePuzzle3CameraProjection::Perspective
                    }
                    crate::CameraProjection3::Orthographic => {
                        RuntimePuzzle3CameraProjection::Orthographic
                    }
                },
                yaw_degrees: camera.yaw_degrees,
                pitch_degrees: camera.pitch_degrees,
                roll_degrees: camera.roll_degrees,
                zoom: f64::from(camera.zoom_milli) / 1000.0,
                interactive_look: camera.interactive_look,
                interactive_zoom: camera.interactive_zoom,
            },
            grid: RuntimePuzzle3GridRender {
                visibility: u8::from(game.render.grid.occupied_cells),
                occupied_cells: game.render.grid.occupied_cells,
            },
            visual: RuntimePuzzle3VisualRender {
                shade: game.render.visual.shade,
            },
            shadow: game.render.shadow,
            pixelate: RuntimePuzzle3PixelateRender {
                enabled: game.render.pixelate.enabled,
                scale: game.render.pixelate.scale,
                smoothing: game.render.pixelate.smoothing,
            },
            animation: RuntimePuzzle3AnimationRender {
                tween: RuntimePuzzle3TweenRender {
                    enabled: game.animation.tween.enabled,
                    interval_ms: game.animation.tween.interval_ms,
                },
            },
            viewport,
        },
        order,
    })
}

pub fn runtime_puzzle3_cells(
    state: &GridState<3, Size3>,
    resources: &RuntimePuzzle3Resources,
) -> Result<Vec<RuntimePuzzle3Cell>, VisualFixtureExportError> {
    let order = &resources.order;
    let objects_by_id = resources
        .objects
        .values()
        .map(|object| (object.id, object))
        .collect::<BTreeMap<_, _>>();
    let visual_order = VisualOrderRef {
        direction_priority: &order.direction_priority,
        priorities: order
            .priorities
            .iter()
            .map(|priority| VisualPriorityRef {
                objects: &priority.objects,
                animations: &priority.animations,
                merge: priority.merge,
            })
            .collect(),
    };
    let mut cells = Vec::new();
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let cell = ((usize::from(z) * usize::from(state.size.depth)) + usize::from(y))
                    * usize::from(state.size.width)
                    + usize::from(x);
                let cell_order = cell_render_order_3d(
                    &visual_order,
                    [state.size.width, state.size.depth, state.size.height],
                    [x, y, z],
                )
                .map_err(|error| VisualFixtureExportError::InvalidVisualOrder {
                    detail: format!("{error:?}"),
                })?;
                let objects = (0..state.layer_count)
                    .filter_map(|layer| {
                        let slot = cell * usize::from(state.layer_count) + usize::from(layer);
                        let object = state.slots()[slot];
                        (!object.is_empty()).then_some((object, layer))
                    })
                    .map(|(object, layer)| {
                        let definition = objects_by_id
                            .get(&object.0)
                            .ok_or(VisualFixtureExportError::MissingObjectName { object })?;
                        Ok(RuntimePuzzle3ObjectRef {
                            id: object.0,
                            layer,
                            render_order: cell_order
                                .saturating_mul(order.priorities.len() as u64)
                                .saturating_add(u64::from(definition.render_priority)),
                        })
                    })
                    .collect::<Result<Vec<_>, VisualFixtureExportError>>()?;
                if !objects.is_empty() {
                    cells.push(RuntimePuzzle3Cell {
                        position: RuntimeCoord { x, y, z: Some(z) },
                        objects,
                        render_order: cell_order,
                    });
                }
            }
        }
    }
    Ok(cells)
}

fn presentation_order_ref(order: &crate::VisualOrderDef) -> VisualOrderRef<'_> {
    VisualOrderRef {
        direction_priority: &order.direction_priority,
        priorities: order
            .priorities
            .iter()
            .map(|priority| VisualPriorityRef {
                objects: &priority.objects,
                animations: &priority.animations,
                merge: priority.merge,
            })
            .collect(),
    }
}

pub fn runtime_puzzle3_size(size: Size3) -> RuntimePuzzle3Size {
    RuntimePuzzle3Size {
        width: size.width,
        depth: size.depth,
        height: size.height,
    }
}

fn runtime_visual_space(space: crate::VisualSpace) -> RuntimePuzzle3VisualSpace {
    match space {
        crate::VisualSpace::World => RuntimePuzzle3VisualSpace::World,
        crate::VisualSpace::Local => RuntimePuzzle3VisualSpace::Local,
    }
}

pub fn export_visual_fixture_json(
    game: &LoadedGridGame<3, Size3>,
    presentation: &SpatialPresentation,
) -> Result<String, VisualFixtureExportError> {
    export_visual_fixture_json_with_scenes(game, presentation, None, &[])
}

pub fn export_visual_fixture_json_with_scenes(
    game: &LoadedGridGame<3, Size3>,
    presentation: &SpatialPresentation,
    scene_fields_json: Option<&str>,
    level_bundle_names: &[String],
) -> Result<String, VisualFixtureExportError> {
    let resources = runtime_puzzle3_resources(game, presentation)?;
    let mut out = String::new();
    out.push_str("{\n");
    let _ = writeln!(out, "  \"layerCount\": {},", game.game.layer_count);
    let size = game
        .levels
        .first()
        .ok_or(VisualFixtureExportError::MissingLevels)?
        .initial_state
        .size;
    write_size_field(&mut out, 1, "size", size, true);
    write_serialized_field(&mut out, "render", &resources.render, true);
    write_directions(&mut out);
    write_direction_sets(&mut out);
    write_serialized_field(&mut out, "inputs", &resources.inputs, true);
    write_serialized_field(&mut out, "objects", &resources.objects, true);
    write_serialized_field(&mut out, "order", &resources.order, true);
    write_scenes(&mut out, scene_fields_json);
    write_levels(&mut out, game, &resources)?;
    write_level_bundles(&mut out, game, level_bundle_names);
    write_serialized_field(&mut out, "visuals", &resources.visuals, false);
    out.push_str("}\n");
    Ok(out)
}

fn write_serialized_field<T: Serialize>(out: &mut String, name: &str, value: &T, comma: bool) {
    writeln!(
        out,
        "  {}: {}{}",
        json_string(name),
        serde_json::to_string_pretty(value).expect("typed Puzzle3 fixture field should serialize"),
        if comma { "," } else { "" },
    )
    .unwrap();
}

fn title_from_identifier(identifier: &str) -> String {
    identifier
        .split('_')
        .filter(|part| !part.is_empty())
        .enumerate()
        .map(|(index, part)| {
            if part.eq_ignore_ascii_case("3d") {
                return "3D".to_string();
            }
            if index > 0 && matches!(part, "a" | "an" | "and" | "in" | "of" | "the") {
                return part.to_string();
            }
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = first.to_uppercase().collect::<String>();
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn write_directions(out: &mut String) {
    let domain = crate::spatial_orientation::SpatialDomain::new(crate::ModelDimension::Three);
    let directions = domain.direction_names();
    out.push_str("  \"directions\": {\n");
    for (index, name) in directions.iter().enumerate() {
        let offset = domain
            .direction_vector(name)
            .expect("canonical direction has an offset")
            .axes();
        let comma = if index + 1 == directions.len() {
            ""
        } else {
            ","
        };
        writeln!(
            out,
            "    \"{}\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}{}",
            name, offset[0], offset[1], offset[2], comma
        )
        .unwrap();
    }
    out.push_str("  },\n");
}

fn write_direction_sets(out: &mut String) {
    out.push_str("  \"directionSets\": {\n");
    out.push_str("    \"horizontal\": [\"left\", \"right\", \"front\", \"back\"],\n");
    out.push_str("    \"vertical\": [\"up\", \"down\"]\n");
    out.push_str("  },\n");
}

fn input_direction_name(axes: [i16; 3]) -> Option<&'static str> {
    match axes {
        [0, 0, -1] => Some("up"),
        [0, 0, 1] => Some("down"),
        [-1, 0, 0] => Some("left"),
        [1, 0, 0] => Some("right"),
        [0, -1, 0] => Some("front"),
        [0, 1, 0] => Some("back"),
        _ => None,
    }
}

fn write_scenes(out: &mut String, scene_fields_json: Option<&str>) {
    if let Some(scene_fields_json) = scene_fields_json {
        out.push_str(scene_fields_json);
        if !scene_fields_json.ends_with('\n') {
            out.push('\n');
        }
        return;
    }
    out.push_str("  \"surface\": { \"root\": \"playing\", \"focus\": \"playing\", \"components\": [{ \"id\": \"playing\", \"definition\": \"playing\", \"placement\": \"root\", \"visibility\": \"visible\", \"modal\": false }] },\n");
    out.push_str("  \"scenes\": [\n");
    out.push_str("    {\n");
    out.push_str("      \"name\": \"playing\",\n");
    out.push_str("      \"puzzles\": [{ \"slot\": \"board\", \"model\": \"default\" }],\n");
    out.push_str("      \"components\": [{ \"kind\": \"puzzle3\", \"source\": \"board\" }]\n");
    out.push_str("    }\n");
    out.push_str("  ],\n");
}

fn write_levels(
    out: &mut String,
    game: &LoadedGridGame<3, Size3>,
    resources: &RuntimePuzzle3Resources,
) -> Result<(), VisualFixtureExportError> {
    let first = game
        .levels
        .first()
        .ok_or(VisualFixtureExportError::MissingLevels)?;
    out.push_str("  \"levelIndex\": 0,\n");
    out.push_str("  \"levels\": [\n");
    for (index, level) in game.levels.iter().enumerate() {
        let comma = if index + 1 == game.levels.len() {
            ""
        } else {
            ","
        };
        out.push_str("    {\n");
        write_json_string_field(out, 3, "name", &level.name, true);
        write_json_string_field(out, 3, "label", &level_label(&level.name), true);
        write_size_field(out, 3, "size", level.initial_state.size, true);
        write_state_cells_field(out, 3, "cells", &level.initial_state, resources)?;
        out.push('\n');
        writeln!(out, "    }}{}", comma).unwrap();
    }
    out.push_str("  ],\n");
    write_state_cells_field(out, 1, "cells", &first.initial_state, resources)?;
    out.push_str(",\n");
    Ok(())
}

fn write_level_bundles(out: &mut String, game: &LoadedGridGame<3, Size3>, extra_names: &[String]) {
    if game.levels.is_empty() {
        return;
    }
    let mut names = vec!["default".to_string(), "levels".to_string()];
    for name in extra_names {
        push_unique_string(&mut names, name);
    }

    out.push_str("  \"levelBundles\": {\n");
    for (name_index, name) in names.iter().enumerate() {
        let comma = if name_index + 1 == names.len() {
            ""
        } else {
            ","
        };
        write_indent(out, 2);
        write!(out, "{}: [", json_string(name)).unwrap();
        for index in 0..game.levels.len() {
            if index > 0 {
                out.push_str(", ");
            }
            write!(out, "{index}").unwrap();
        }
        writeln!(out, "]{}", comma).unwrap();
    }
    out.push_str("  },\n");
}

fn push_unique_string(names: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !names.iter().any(|name| name == value) {
        names.push(value.to_string());
    }
}

fn level_label(name: &str) -> String {
    name.strip_prefix("microban_")
        .and_then(|number| number.parse::<u32>().ok())
        .map(|number| format!("Microban {number:02}"))
        .unwrap_or_else(|| title_from_identifier(name))
}

fn write_state_cells_field(
    out: &mut String,
    indent: usize,
    name: &str,
    state: &GridState<3, Size3>,
    resources: &RuntimePuzzle3Resources,
) -> Result<(), VisualFixtureExportError> {
    let cells = runtime_puzzle3_cells(state, resources)?;
    write_indent(out, indent);
    write!(
        out,
        "{}: {}",
        json_string(name),
        serde_json::to_string(&cells).expect("typed Puzzle3 cells should serialize")
    )
    .unwrap();
    Ok(())
}

fn write_size_field(out: &mut String, indent: usize, name: &str, size: Size3, comma: bool) {
    write_indent(out, indent);
    writeln!(
        out,
        "{}: {{ \"width\": {}, \"depth\": {}, \"height\": {} }}{}",
        json_string(name),
        size.width,
        size.depth,
        size.height,
        if comma { "," } else { "" }
    )
    .unwrap();
}

fn write_json_string_field(out: &mut String, indent: usize, name: &str, value: &str, comma: bool) {
    write_indent(out, indent);
    writeln!(
        out,
        "{}: {}{}",
        json_string(name),
        json_string(value),
        if comma { "," } else { "" }
    )
    .unwrap();
}

fn write_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => write!(out, "\\u{:04x}", ch as u32).unwrap(),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}
