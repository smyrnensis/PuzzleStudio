use std::collections::BTreeMap;
use std::fmt::Write;

use crate::{
    ParsedPuzzle3, SpriteColor3, SpriteSet3, ViewportFollow3, ViewportHeight3, ViewportMode3,
};
use puzzle_grid3d::{Direction3, LevelBundle3, LevelCell3, ObjectId, Size3};
use puzzle_runtime_contract::{
    Puzzle3RuntimeModel, RUNTIME_CONTRACT_VERSION, RuntimeContract, RuntimeModelContract,
    runtime_contract_json,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VisualFixtureExportError3 {
    MissingLevelBundle,
    MissingObjectName { object: ObjectId },
    RuntimeContract(String),
}

pub fn export_visual_fixture_json(
    parsed: &ParsedPuzzle3,
) -> Result<String, VisualFixtureExportError3> {
    export_visual_fixture_json_with_title(parsed, None)
}

pub fn export_visual_fixture_json_with_title(
    parsed: &ParsedPuzzle3,
    title: Option<&str>,
) -> Result<String, VisualFixtureExportError3> {
    export_visual_fixture_json_with_title_and_scenes(parsed, title, None, &[])
}

pub fn export_visual_fixture_json_with_title_and_scenes(
    parsed: &ParsedPuzzle3,
    title: Option<&str>,
    scene_fields_json: Option<&str>,
    level_bundle_names: &[String],
) -> Result<String, VisualFixtureExportError3> {
    let bundle = parsed
        .level_bundle
        .as_ref()
        .ok_or(VisualFixtureExportError3::MissingLevelBundle)?;
    let object_names = parsed
        .object_labels
        .iter()
        .map(|(object, label)| (*object, label.clone()))
        .collect::<BTreeMap<_, _>>();
    let title = title
        .map(str::to_string)
        .unwrap_or_else(|| fixture_title(parsed));

    let mut out = String::new();
    out.push_str("{\n");
    write_json_string_field(&mut out, 1, "title", &title, true);
    let _ = writeln!(
        out,
        "  \"runtimeContractVersion\": {},",
        RUNTIME_CONTRACT_VERSION
    );
    write_runtime_contract(&mut out, parsed, bundle)?;
    let _ = writeln!(out, "  \"layerCount\": {},", parsed.game.layer_count);
    write_size_field(&mut out, 1, "size", bundle.levels[0].level.size, true);
    write_render(&mut out, parsed);
    write_directions(&mut out);
    write_direction_sets(&mut out);
    write_controls(&mut out, parsed);
    write_inputs(&mut out, parsed);
    write_objects(&mut out, parsed, &object_names)?;
    write_visual_order(&mut out, parsed);
    write_scenes(&mut out, scene_fields_json);
    write_levels(&mut out, parsed, &object_names)?;
    write_level_bundles(&mut out, parsed, level_bundle_names);
    write_sprites(&mut out, parsed.sprite_set.as_ref());
    out.push_str("}\n");
    Ok(out)
}

fn write_visual_order(out: &mut String, parsed: &ParsedPuzzle3) {
    out.push_str("  \"order\": ");
    out.push_str(
        &serde_json::to_string(&parsed.visual_order)
            .expect("compiled 3D sprite order serialization must succeed"),
    );
    out.push_str(",\n");
}

fn write_runtime_contract(
    out: &mut String,
    parsed: &ParsedPuzzle3,
    bundle: &LevelBundle3,
) -> Result<(), VisualFixtureExportError3> {
    let model = Puzzle3RuntimeModel::checked_new(
        parsed.game.clone(),
        parsed.local_frame.clone(),
        parsed.rule_camera_effects.clone(),
        bundle.clone(),
        parsed.win_condition.clone(),
        parsed.lifecycle.clone(),
        parsed.on_level_start_camera_effects.clone(),
    )
    .map_err(|error| VisualFixtureExportError3::RuntimeContract(error.to_string()))?;
    let contract = RuntimeContract::checked_new(RuntimeModelContract::Puzzle3(model))
        .map_err(|error| VisualFixtureExportError3::RuntimeContract(error.to_string()))?;
    let contract_json = runtime_contract_json(&contract)
        .map_err(|error| VisualFixtureExportError3::RuntimeContract(error.to_string()))?;
    out.push_str("  \"runtimeContract\": ");
    out.push_str(&contract_json);
    out.push_str(",\n");
    Ok(())
}

fn write_render(out: &mut String, parsed: &ParsedPuzzle3) {
    let camera = &parsed.render.camera;
    let pixelate = &parsed.render.pixelate;
    let _ = write!(
        out,
        "  \"render\": {{ \"camera\": {{ \"yawDegrees\": {}, \"pitchDegrees\": {}, \"rollDegrees\": {}, \"zoom\": {}, \"interactiveLook\": {}, \"interactiveZoom\": {} }}, ",
        camera.yaw_degrees,
        camera.pitch_degrees,
        camera.roll_degrees,
        format_zoom(camera.zoom_milli),
        camera.interactive_look,
        camera.interactive_zoom,
    );
    let _ = write!(
        out,
        "\"grid\": {{ \"visibility\": {}, \"occupiedCells\": {} }}, \"sprite\": {{ \"shade\": {} }}, \"shadow\": {}, \"pixelate\": {{ \"enabled\": {}, \"scale\": {}, \"smoothing\": {} }}, \"animation\": {{ \"tween\": {{ \"enabled\": {}, \"intervalMs\": {} }} }}, \"viewport\": ",
        if parsed.render.grid.occupied_cells {
            1
        } else {
            0
        },
        parsed.render.grid.occupied_cells,
        parsed.render.sprite.shade,
        parsed.render.shadow,
        pixelate.enabled,
        pixelate.scale,
        pixelate.smoothing,
        parsed.animation.tween.enabled,
        parsed.animation.tween.interval_ms,
    );
    let viewport = &parsed.render.viewport;
    let Some(framing) = viewport.framing else {
        out.push_str("null },\n");
        return;
    };
    let mode = match viewport.mode {
        ViewportMode3::Full => "full",
        ViewportMode3::Centered => "centered",
        ViewportMode3::Paged => "paged",
    };
    let follow = match viewport.follow {
        ViewportFollow3::Snap => "snap",
        ViewportFollow3::Smooth => "smooth",
    };
    out.push_str("{ ");
    let _ = write!(
        out,
        "\"mode\": {}, \"follow\": {}, \"focus\": {}, \"focusObjects\": [",
        json_string(mode),
        json_string(follow),
        json_string(&viewport.focus),
    );
    for (index, object) in viewport_focus_objects(parsed).iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "{}", object.0);
    }
    out.push_str("], \"framingBox\": { ");
    let _ = write!(
        out,
        "\"width\": {}, \"depth\": {}, \"height\": ",
        framing.width, framing.depth,
    );
    match framing.height {
        ViewportHeight3::Full => out.push_str("\"full\""),
        ViewportHeight3::Size(height) => {
            let _ = write!(out, "{height}");
        }
    }
    out.push_str(" } } },\n");
}

fn viewport_focus_objects(parsed: &ParsedPuzzle3) -> Vec<ObjectId> {
    parsed.viewport_focus_objects.clone()
}

fn format_zoom(zoom_milli: u16) -> String {
    let whole = zoom_milli / 1000;
    let fraction = zoom_milli % 1000;
    if fraction == 0 {
        return whole.to_string();
    }
    let mut fraction = format!("{fraction:03}");
    while fraction.ends_with('0') {
        fraction.pop();
    }
    format!("{whole}.{fraction}")
}

fn fixture_title(parsed: &ParsedPuzzle3) -> String {
    parsed
        .sprite_set
        .as_ref()
        .and_then(|sprites| sprites.model.as_deref())
        .map(title_from_identifier)
        .unwrap_or_else(|| "Puzzle3".to_string())
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
    out.push_str("  \"directions\": {\n");
    for (index, direction) in Direction3::directions().into_iter().enumerate() {
        let comma = if index + 1 == Direction3::directions().len() {
            ""
        } else {
            ","
        };
        writeln!(
            out,
            "    \"{}\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}{}",
            direction.name, direction.offset.dx, direction.offset.dy, direction.offset.dz, comma
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

fn write_controls(out: &mut String, parsed: &ParsedPuzzle3) {
    out.push_str("  \"controls\": {\n");
    out.push_str("    \"keys\": {\n");
    let default_keys = [
        ("ArrowLeft", "left"),
        ("ArrowRight", "right"),
        ("ArrowUp", "front"),
        ("ArrowDown", "back"),
        ("KeyA", "left"),
        ("KeyD", "right"),
        ("KeyW", "front"),
        ("KeyS", "back"),
        ("a", "left"),
        ("d", "right"),
        ("w", "front"),
        ("s", "back"),
        ("Left", "left"),
        ("Right", "right"),
        ("Up", "front"),
        ("Down", "back"),
    ];
    let mut keys = Vec::<(&str, &str)>::new();
    if parsed.inputs.iter().any(|input| !input.keys.is_empty()) {
        for input in &parsed.inputs {
            for key in &input.keys {
                keys.push((key.as_str(), input.name.as_str()));
            }
        }
    } else {
        keys.extend(default_keys);
    }
    for (index, (key, input)) in keys.iter().enumerate() {
        let comma = if index + 1 == keys.len() { "" } else { "," };
        writeln!(
            out,
            "      {}: {}{}",
            json_string(key),
            json_string(input),
            comma
        )
        .unwrap();
    }
    out.push_str("    }\n");
    out.push_str("  },\n");
}

fn write_inputs(out: &mut String, parsed: &ParsedPuzzle3) {
    out.push_str("  \"inputs\": [");
    for (index, input) in parsed.inputs.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write!(
            out,
            "{{ \"id\": {}, \"name\": {}",
            input.id.0,
            json_string(&input.name)
        )
        .unwrap();
        if let Some(direction) = input.direction {
            write!(out, ", \"direction\": {}", json_string(direction.name)).unwrap();
        }
        if !input.keys.is_empty() {
            out.push_str(", \"keys\": [");
            for (key_index, key) in input.keys.iter().enumerate() {
                if key_index > 0 {
                    out.push_str(", ");
                }
                out.push_str(&json_string(key));
            }
            out.push(']');
        }
        out.push_str(" }");
    }
    out.push_str("],\n");
}

fn write_objects(
    out: &mut String,
    parsed: &ParsedPuzzle3,
    names: &BTreeMap<ObjectId, String>,
) -> Result<(), VisualFixtureExportError3> {
    out.push_str("  \"objects\": {\n");
    for (index, (object, name)) in names.iter().enumerate() {
        let comma = if index + 1 == names.len() { "" } else { "," };
        let layer = parsed
            .game
            .object_layer(*object)
            .map(|layer| layer.0)
            .unwrap_or(0);
        writeln!(
            out,
            "    {}: {{ \"id\": {}, \"name\": {}, \"sprite\": {}, \"layer\": {} }}{}",
            json_string(name),
            object.0,
            json_string(name),
            fixture_sprite_value(parsed, name),
            layer,
            comma
        )
        .unwrap();
    }
    out.push_str("  },\n");
    Ok(())
}

fn write_scenes(out: &mut String, scene_fields_json: Option<&str>) {
    if let Some(scene_fields_json) = scene_fields_json {
        out.push_str(scene_fields_json);
        if !scene_fields_json.ends_with('\n') {
            out.push('\n');
        }
        return;
    }
    write_json_string_field(out, 1, "currentScene", "playing", true);
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
    parsed: &ParsedPuzzle3,
    names: &BTreeMap<ObjectId, String>,
) -> Result<(), VisualFixtureExportError3> {
    let bundle = parsed
        .level_bundle
        .as_ref()
        .ok_or(VisualFixtureExportError3::MissingLevelBundle)?;
    out.push_str("  \"levelIndex\": 0,\n");
    out.push_str("  \"levels\": [\n");
    for (index, entry) in bundle.levels.iter().enumerate() {
        let comma = if index + 1 == bundle.levels.len() {
            ""
        } else {
            ","
        };
        out.push_str("    {\n");
        write_json_string_field(out, 3, "name", &entry.name, true);
        write_json_string_field(out, 3, "label", &level_label(&entry.name), true);
        write_size_field(out, 3, "size", entry.level.size, true);
        write_cells_field(
            out,
            3,
            "cells",
            &entry.level.cells,
            names,
            parsed.sprite_set.as_ref(),
        )?;
        out.push('\n');
        writeln!(out, "    }}{}", comma).unwrap();
    }
    out.push_str("  ],\n");
    write_cells_field(
        out,
        1,
        "cells",
        &bundle.levels[0].level.cells,
        names,
        parsed.sprite_set.as_ref(),
    )?;
    out.push_str(",\n");
    Ok(())
}

fn write_level_bundles(out: &mut String, parsed: &ParsedPuzzle3, extra_names: &[String]) {
    let Some(bundle) = parsed.level_bundle.as_ref() else {
        return;
    };
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
        for index in 0..bundle.levels.len() {
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

fn write_cells_field(
    out: &mut String,
    indent: usize,
    name: &str,
    cells: &[LevelCell3],
    names: &BTreeMap<ObjectId, String>,
    sprite_set: Option<&SpriteSet3>,
) -> Result<(), VisualFixtureExportError3> {
    write_indent(out, indent);
    writeln!(out, "{}: [", json_string(name)).unwrap();
    for (index, cell) in cells.iter().enumerate() {
        let comma = if index + 1 == cells.len() { "" } else { "," };
        write_indent(out, indent + 1);
        out.push_str("{ ");
        write!(
            out,
            "\"position\": {{ \"x\": {}, \"y\": {}, \"z\": {} }}, ",
            cell.position.x, cell.position.y, cell.position.z
        )
        .unwrap();
        out.push_str("\"objects\": [");
        for (object_index, object) in cell.objects.iter().enumerate() {
            if object_index > 0 {
                out.push_str(", ");
            }
            let object_name = names
                .get(object)
                .ok_or(VisualFixtureExportError3::MissingObjectName { object: *object })?;
            write!(
                out,
                "{{ \"id\": {}, \"name\": {}, \"sprite\": {} }}",
                object.0,
                json_string(object_name),
                fixture_sprite_value_from_set(sprite_set, object_name)
            )
            .unwrap();
        }
        writeln!(out, "] }}{}", comma).unwrap();
    }
    write_indent(out, indent);
    out.push(']');
    Ok(())
}

fn fixture_sprite_value(parsed: &ParsedPuzzle3, object_name: &str) -> String {
    fixture_sprite_value_from_set(parsed.sprite_set.as_ref(), object_name)
}

fn fixture_sprite_value_from_set(sprite_set: Option<&SpriteSet3>, object_name: &str) -> String {
    sprite_set
        .and_then(|sprites| sprites.sprite(object_name))
        .map(|sprite| json_string(&sprite.name))
        .unwrap_or_else(|| "null".to_string())
}

fn write_sprites(out: &mut String, sprite_set: Option<&SpriteSet3>) {
    out.push_str("  \"sprites\": {\n");
    let Some(sprite_set) = sprite_set else {
        out.push_str("  }\n");
        return;
    };
    for (index, sprite) in sprite_set.sprites.iter().enumerate() {
        let comma = if index + 1 == sprite_set.sprites.len() {
            ""
        } else {
            ","
        };
        writeln!(out, "    {}: {{", json_string(&sprite.name)).unwrap();
        out.push_str("      \"palette\": {");
        let visible_colors = sprite
            .palette
            .iter()
            .map(|(key, color)| match color {
                SpriteColor3::Transparent => (*key, "transparent"),
                SpriteColor3::Hex(value) => (*key, value.as_str()),
            })
            .collect::<Vec<_>>();
        for (color_index, (key, value)) in visible_colors.iter().enumerate() {
            if color_index > 0 {
                out.push_str(", ");
            }
            write!(
                out,
                "{}: {}",
                json_string(&key.to_string()),
                json_string(value)
            )
            .unwrap();
        }
        out.push_str("},\n");
        out.push_str("      \"frames\": [\n");
        for (frame_index, frame) in sprite.frames.iter().enumerate() {
            out.push_str("        { \"layers\": [\n");
            for (layer_index, layer) in frame.slices.iter().enumerate() {
                out.push_str("          [");
                for (row_index, row) in layer.iter().enumerate() {
                    if row_index > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&json_string(row));
                }
                let layer_comma = if layer_index + 1 == frame.slices.len() {
                    ""
                } else {
                    ","
                };
                writeln!(out, "]{layer_comma}").unwrap();
            }
            let frame_comma = if frame_index + 1 == sprite.frames.len() {
                ""
            } else {
                ","
            };
            writeln!(out, "        ] }}{frame_comma}").unwrap();
        }
        out.push_str("      ],\n");
        match sprite.duration_ms {
            Some(value) => writeln!(out, "      \"durationMs\": {value},").unwrap(),
            None => out.push_str("      \"durationMs\": null,\n"),
        }
        match sprite.frame_duration_ms {
            Some(value) => writeln!(out, "      \"frameDurationMs\": {value},").unwrap(),
            None => out.push_str("      \"frameDurationMs\": null,\n"),
        }
        out.push_str("      \"spatialOps\": ");
        out.push_str(&serde_json::to_string(&sprite.spatial_ops.iter().map(|op| match op {
            crate::SpriteSpatialOp3::Translate { space, value } => serde_json::json!({"kind":"translate3","space":match space { crate::SpriteSpace3::World => "world", crate::SpriteSpace3::Local => "local" },"value":value}),
            crate::SpriteSpatialOp3::Rotate { space, axis, degrees } => serde_json::json!({"kind":"rotate3","space":match space { crate::SpriteSpace3::World => "world", crate::SpriteSpace3::Local => "local" },"axis":axis,"degrees":degrees}),
        }).collect::<Vec<_>>()).expect("sprite spatial ops serialize"));
        out.push('\n');
        writeln!(out, "    }}{}", comma).unwrap();
    }
    out.push_str("  }\n");
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
