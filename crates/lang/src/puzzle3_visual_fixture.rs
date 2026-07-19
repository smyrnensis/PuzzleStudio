use std::collections::BTreeMap;
use std::fmt::Write;

use crate::{
    LoadedGridGame, SpatialPresentation, ViewportFollow3, ViewportHeight3, ViewportMode3,
    VoxelColor, VoxelSpriteSet,
};
use puzzle_core::{GridState, ObjectId, Size3};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VisualFixtureExportError {
    MissingLevels,
    MissingObjectName { object: ObjectId },
}

pub fn export_visual_fixture_json(
    game: &LoadedGridGame<3, Size3>,
    presentation: &SpatialPresentation,
) -> Result<String, VisualFixtureExportError> {
    export_visual_fixture_json_with_title(game, presentation, None)
}

pub fn export_visual_fixture_json_with_title(
    game: &LoadedGridGame<3, Size3>,
    presentation: &SpatialPresentation,
    title: Option<&str>,
) -> Result<String, VisualFixtureExportError> {
    export_visual_fixture_json_with_title_and_scenes(game, presentation, title, None, &[])
}

pub fn export_visual_fixture_json_with_title_and_scenes(
    game: &LoadedGridGame<3, Size3>,
    presentation: &SpatialPresentation,
    title: Option<&str>,
    scene_fields_json: Option<&str>,
    level_bundle_names: &[String],
) -> Result<String, VisualFixtureExportError> {
    let object_names = game
        .object_labels
        .iter()
        .map(|(object, label)| (*object, label.clone()))
        .collect::<BTreeMap<_, _>>();
    let title = title
        .map(str::to_string)
        .unwrap_or_else(|| fixture_title(presentation));

    let mut out = String::new();
    out.push_str("{\n");
    write_json_string_field(&mut out, 1, "title", &title, true);
    let _ = writeln!(out, "  \"layerCount\": {},", game.game.layer_count);
    let size = game
        .levels
        .first()
        .ok_or(VisualFixtureExportError::MissingLevels)?
        .initial_state
        .size;
    write_size_field(&mut out, 1, "size", size, true);
    write_render(&mut out, game, presentation);
    write_directions(&mut out);
    write_direction_sets(&mut out);
    write_inputs(&mut out, game);
    write_objects(&mut out, game, presentation, &object_names)?;
    write_visual_order(&mut out, presentation);
    write_scenes(&mut out, scene_fields_json);
    write_levels(&mut out, game, presentation, &object_names)?;
    write_level_bundles(&mut out, game, level_bundle_names);
    write_sprites(&mut out, presentation.sprite_set.as_ref());
    out.push_str("}\n");
    Ok(out)
}

fn write_visual_order(out: &mut String, presentation: &SpatialPresentation) {
    out.push_str("  \"order\": ");
    out.push_str(
        &serde_json::to_string(&presentation.visual_order)
            .expect("compiled 3D sprite order serialization must succeed"),
    );
    out.push_str(",\n");
}

fn write_render(
    out: &mut String,
    game: &LoadedGridGame<3, Size3>,
    presentation: &SpatialPresentation,
) {
    let camera = &game.render.camera;
    let pixelate = &game.render.pixelate;
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
        if game.render.grid.occupied_cells {
            1
        } else {
            0
        },
        game.render.grid.occupied_cells,
        game.render.sprite.shade,
        game.render.shadow,
        pixelate.enabled,
        pixelate.scale,
        pixelate.smoothing,
        game.animation.tween.enabled,
        game.animation.tween.interval_ms,
    );
    let viewport = &game.render.viewport;
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
    for (index, object) in presentation.viewport_focus_objects.iter().enumerate() {
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

fn fixture_title(presentation: &SpatialPresentation) -> String {
    presentation
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

fn write_inputs(out: &mut String, game: &LoadedGridGame<3, Size3>) {
    out.push_str("  \"inputs\": [");
    for (index, input) in game.inputs.iter().enumerate() {
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
            if let Some(name) = input_direction_name(direction.axes()) {
                write!(out, ", \"direction\": {}", json_string(name)).unwrap();
            }
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

fn input_direction_name(axes: [i16; 3]) -> Option<&'static str> {
    match axes {
        [0, 0, 1] => Some("up"),
        [0, 0, -1] => Some("down"),
        [-1, 0, 0] => Some("left"),
        [1, 0, 0] => Some("right"),
        [0, 1, 0] => Some("front"),
        [0, -1, 0] => Some("back"),
        _ => None,
    }
}

fn write_objects(
    out: &mut String,
    game: &LoadedGridGame<3, Size3>,
    presentation: &SpatialPresentation,
    names: &BTreeMap<ObjectId, String>,
) -> Result<(), VisualFixtureExportError> {
    out.push_str("  \"objects\": {\n");
    for (index, (object, name)) in names.iter().enumerate() {
        let comma = if index + 1 == names.len() { "" } else { "," };
        let layer = game
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
            fixture_sprite_value(presentation, name),
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
    game: &LoadedGridGame<3, Size3>,
    presentation: &SpatialPresentation,
    names: &BTreeMap<ObjectId, String>,
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
        write_state_cells_field(
            out,
            3,
            "cells",
            &level.initial_state,
            names,
            presentation.sprite_set.as_ref(),
        )?;
        out.push('\n');
        writeln!(out, "    }}{}", comma).unwrap();
    }
    out.push_str("  ],\n");
    write_state_cells_field(
        out,
        1,
        "cells",
        &first.initial_state,
        names,
        presentation.sprite_set.as_ref(),
    )?;
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
    names: &BTreeMap<ObjectId, String>,
    sprite_set: Option<&VoxelSpriteSet>,
) -> Result<(), VisualFixtureExportError> {
    let cells = state
        .slots()
        .chunks(usize::from(state.layer_count))
        .enumerate()
        .filter_map(|(index, slots)| {
            let objects = slots
                .iter()
                .copied()
                .filter(|object| !object.is_empty())
                .collect::<Vec<_>>();
            (!objects.is_empty()).then(|| (state.cell_coord(index), objects))
        })
        .filter_map(|(position, objects)| position.map(|position| (position, objects)))
        .collect::<Vec<_>>();
    write_indent(out, indent);
    writeln!(out, "{}: [", json_string(name)).unwrap();
    for (index, (position, objects)) in cells.iter().enumerate() {
        let comma = if index + 1 == cells.len() { "" } else { "," };
        write_indent(out, indent + 1);
        out.push_str("{ ");
        let [x, y, z] = position.axes();
        write!(
            out,
            "\"position\": {{ \"x\": {}, \"y\": {}, \"z\": {} }}, ",
            x, y, z
        )
        .unwrap();
        out.push_str("\"objects\": [");
        for (object_index, object) in objects.iter().enumerate() {
            if object_index > 0 {
                out.push_str(", ");
            }
            let object_name = names
                .get(object)
                .ok_or(VisualFixtureExportError::MissingObjectName { object: *object })?;
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

fn fixture_sprite_value(presentation: &SpatialPresentation, object_name: &str) -> String {
    fixture_sprite_value_from_set(presentation.sprite_set.as_ref(), object_name)
}

fn fixture_sprite_value_from_set(sprite_set: Option<&VoxelSpriteSet>, object_name: &str) -> String {
    sprite_set
        .and_then(|sprites| sprites.sprite(object_name))
        .map(|sprite| json_string(&sprite.name))
        .unwrap_or_else(|| "null".to_string())
}

fn write_sprites(out: &mut String, sprite_set: Option<&VoxelSpriteSet>) {
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
                VoxelColor::Transparent => (*key, "transparent"),
                VoxelColor::Hex(value) => (*key, value.as_str()),
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
        out.push_str(&serde_json::to_string(&sprite.transforms.iter().map(|op| match op {
            crate::VisualSpriteTransform::Translate { space, value } => serde_json::json!({"kind":"translate3","space":match space { crate::VisualSpriteSpace::World => "world", crate::VisualSpriteSpace::Local => "local" },"value":value}),
            crate::VisualSpriteTransform::Rotate { space, axis, degrees } => serde_json::json!({"kind":"rotate3","space":match space { crate::VisualSpriteSpace::World => "world", crate::VisualSpriteSpace::Local => "local" },"axis":axis,"degrees":degrees}),
            crate::VisualSpriteTransform::Flip { enabled } => serde_json::json!({"kind":"flip3","enabled":enabled}),
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
