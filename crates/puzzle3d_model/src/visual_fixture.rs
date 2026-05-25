use std::collections::BTreeMap;
use std::fmt::Write;

use crate::{
    Direction3, Guard3, Lifecycle3, LifecycleCommand3, MatchCell3, ObjectId, ParsedPuzzle3,
    Pattern3, Rule3, RuleApplication3, SceneAction3, SceneAlignX3, SceneAlignY3, SceneComponent3,
    SceneLayout3, SelectorCatalog3, Size3, SpriteColor3, SpriteSet3, WinCondition3, WriteOp3,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VisualFixtureExportError3 {
    MissingLevelBundle,
    MissingObjectName { object: ObjectId },
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
    let bundle = parsed
        .level_bundle
        .as_ref()
        .ok_or(VisualFixtureExportError3::MissingLevelBundle)?;
    let object_names = object_names(&parsed.catalog);
    let title = title
        .map(str::to_string)
        .unwrap_or_else(|| fixture_title(parsed));

    let mut out = String::new();
    out.push_str("{\n");
    write_json_string_field(&mut out, 1, "title", &title, true);
    write_size_field(&mut out, 1, "size", bundle.levels[0].level.size, true);
    write_camera(&mut out, parsed);
    write_settings(&mut out, parsed);
    write_directions(&mut out);
    write_direction_sets(&mut out);
    write_controls(&mut out);
    write_inputs(&mut out, parsed);
    write_rules(&mut out, "rules", &parsed.rules);
    write_lifecycle(&mut out, &parsed.lifecycle);
    if let Some(condition) = parsed.win_condition.as_ref() {
        write_win_condition(&mut out, condition);
    }
    write_objects(&mut out, parsed, &object_names)?;
    write_scenes(&mut out, parsed);
    write_levels(&mut out, parsed, &object_names)?;
    write_level_bundles(&mut out, parsed);
    write_sprites(&mut out, parsed.sprite_set.as_ref());
    out.push_str("}\n");
    Ok(out)
}

fn write_camera(out: &mut String, parsed: &ParsedPuzzle3) {
    let camera = &parsed.settings.camera;
    let _ = writeln!(
        out,
        "  \"camera\": {{ \"yawDegrees\": {}, \"pitchDegrees\": {}, \"zoom\": {} }},",
        camera.yaw_degrees,
        camera.pitch_degrees,
        format_zoom(camera.zoom_milli),
    );
}

fn write_settings(out: &mut String, parsed: &ParsedPuzzle3) {
    let camera = &parsed.settings.camera;
    let _ = writeln!(
        out,
        "  \"settings\": {{ \"interactiveLook\": {}, \"interactiveZoom\": {}, \"grid\": {{ \"visible\": {}, \"occupied_cells\": {} }}, \"shade\": {} }},",
        camera.interactive_look,
        camera.interactive_zoom,
        parsed.settings.grid.occupied_cells,
        parsed.settings.grid.occupied_cells,
        parsed.settings.sprite.shade,
    );
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

fn object_names(catalog: &SelectorCatalog3) -> BTreeMap<ObjectId, String> {
    let mut names = BTreeMap::new();
    for object in &catalog.objects {
        names.insert(object.id, object.name.clone());
    }
    for family in &catalog.families {
        for variant in &family.variants {
            let suffix = variant.values.join(":");
            let name = if suffix.is_empty() {
                family.name.clone()
            } else {
                format!("{}:{suffix}", family.name)
            };
            names.insert(variant.id, name);
        }
    }
    names
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
    out.push_str("    \"horizontal\": [\"left\", \"right\", \"forward\", \"backward\"],\n");
    out.push_str("    \"vertical\": [\"up\", \"down\"]\n");
    out.push_str("  },\n");
}

fn write_controls(out: &mut String) {
    out.push_str("  \"controls\": {\n");
    out.push_str("    \"keys\": {\n");
    let keys = [
        ("ArrowLeft", "left"),
        ("ArrowRight", "right"),
        ("ArrowUp", "forward"),
        ("ArrowDown", "backward"),
        ("KeyA", "left"),
        ("KeyD", "right"),
        ("KeyW", "forward"),
        ("KeyS", "backward"),
        ("a", "left"),
        ("d", "right"),
        ("w", "forward"),
        ("s", "backward"),
        ("Left", "left"),
        ("Right", "right"),
        ("Up", "forward"),
        ("Down", "backward"),
    ];
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
    for (index, input) in parsed.game.inputs.iter().enumerate() {
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
        out.push_str(" }");
    }
    out.push_str("],\n");
}

fn write_lifecycle(out: &mut String, lifecycle: &Lifecycle3) {
    out.push_str("  \"lifecycle\": {\n");
    out.push_str("    \"onLevelStart\": ");
    write_rule_array(out, &lifecycle.on_level_start);
    out.push_str(",\n");
    out.push_str("    \"onLevelClear\": [");
    for (index, command) in lifecycle.on_level_clear.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        match command {
            LifecycleCommand3::NextLevel => out.push_str("\"next_level\""),
        }
    }
    out.push_str("]\n");
    out.push_str("  },\n");
}

fn write_rules(out: &mut String, name: &str, rules: &[Rule3]) {
    write_indent(out, 1);
    write!(out, "{}: ", json_string(name)).unwrap();
    write_rule_array(out, rules);
    out.push_str(",\n");
}

fn write_rule_array(out: &mut String, rules: &[Rule3]) {
    out.push('[');
    for (index, rule) in rules.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write_rule_json(out, rule, index);
    }
    out.push(']');
}

fn write_rule_json(out: &mut String, rule: &Rule3, index: usize) {
    write!(
        out,
        "{{ \"id\": {}, \"application\": {}, \"guards\": [",
        index + 1,
        json_string(rule_application_name(rule.application))
    )
    .unwrap();
    for (guard_index, guard) in rule.guards.iter().enumerate() {
        if guard_index > 0 {
            out.push_str(", ");
        }
        match guard {
            Guard3::InputIs(input) => {
                write!(out, "{{ \"kind\": \"input_is\", \"input\": {} }}", input.0).unwrap();
            }
        }
    }
    out.push_str("], \"pattern\": ");
    write_pattern_json(out, &rule.pattern);
    out.push_str(", \"writes\": [");
    for (write_index, write_op) in rule.writes.iter().enumerate() {
        if write_index > 0 {
            out.push_str(", ");
        }
        write_write_op_json(out, write_op);
    }
    out.push_str("] }");
}

fn rule_application_name(application: RuleApplication3) -> &'static str {
    match application {
        RuleApplication3::Once => "once",
        RuleApplication3::OnceAll => "once_all",
        RuleApplication3::OncePerLevel => "once_per_level",
        RuleApplication3::UntilStable => "until_stable",
    }
}

fn write_pattern_json(out: &mut String, pattern: &Pattern3) {
    out.push_str("{ \"cells\": [");
    for (index, cell) in pattern.cells.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write_match_cell_json(out, cell);
    }
    out.push_str("] }");
}

fn write_match_cell_json(out: &mut String, cell: &MatchCell3) {
    write!(
        out,
        "{{ \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"require\": [",
        cell.offset.dx, cell.offset.dy, cell.offset.dz
    )
    .unwrap();
    write_object_id_array(out, &cell.require_objects);
    out.push_str("], \"forbid\": [");
    write_object_id_array(out, &cell.forbid_objects);
    out.push_str("] }");
}

fn write_object_id_array(out: &mut String, objects: &[ObjectId]) {
    for (index, object) in objects.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write!(out, "{}", object.0).unwrap();
    }
}

fn write_write_op_json(out: &mut String, write_op: &WriteOp3) {
    match write_op {
        WriteOp3::Add { offset, object } => {
            write!(
                out,
                "{{ \"kind\": \"add\", \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"object\": {} }}",
                offset.dx, offset.dy, offset.dz, object.0
            )
            .unwrap();
        }
        WriteOp3::Remove { offset, object } => {
            write!(
                out,
                "{{ \"kind\": \"remove\", \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"object\": {} }}",
                offset.dx, offset.dy, offset.dz, object.0
            )
            .unwrap();
        }
        WriteOp3::Replace {
            offset,
            remove,
            add,
        } => {
            write!(
                out,
                "{{ \"kind\": \"replace\", \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"remove\": {}, \"add\": {} }}",
                offset.dx, offset.dy, offset.dz, remove.0, add.0
            )
            .unwrap();
        }
        WriteOp3::Move {
            from_offset,
            to_offset,
            object,
        } => {
            write!(
                out,
                "{{ \"kind\": \"move\", \"fromOffset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"toOffset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"object\": {} }}",
                from_offset.dx,
                from_offset.dy,
                from_offset.dz,
                to_offset.dx,
                to_offset.dy,
                to_offset.dz,
                object.0
            )
            .unwrap();
        }
    }
}

fn write_win_condition(out: &mut String, condition: &WinCondition3) {
    out.push_str("  \"winCondition\": ");
    write_win_condition_json(out, condition);
    out.push_str(",\n");
}

fn write_win_condition_json(out: &mut String, condition: &WinCondition3) {
    match condition {
        WinCondition3::All(conditions) => {
            out.push_str("{ \"kind\": \"all\", \"conditions\": [");
            write_win_condition_list(out, conditions);
            out.push_str("] }");
        }
        WinCondition3::Any(conditions) => {
            out.push_str("{ \"kind\": \"any\", \"conditions\": [");
            write_win_condition_list(out, conditions);
            out.push_str("] }");
        }
        WinCondition3::SomeObject(object) => {
            write!(
                out,
                "{{ \"kind\": \"some_object\", \"object\": {} }}",
                object.0
            )
            .unwrap();
        }
        WinCondition3::NoObject(object) => {
            write!(
                out,
                "{{ \"kind\": \"no_object\", \"object\": {} }}",
                object.0
            )
            .unwrap();
        }
        WinCondition3::SomePattern(pattern) => {
            out.push_str("{ \"kind\": \"some_pattern\", \"pattern\": ");
            write_pattern_json(out, pattern);
            out.push_str(" }");
        }
        WinCondition3::NoPattern(pattern) => {
            out.push_str("{ \"kind\": \"no_pattern\", \"pattern\": ");
            write_pattern_json(out, pattern);
            out.push_str(" }");
        }
        WinCondition3::AllObjectsCoveredByPattern {
            object,
            cover_pattern,
        } => {
            write!(
                out,
                "{{ \"kind\": \"all_objects_covered_by_pattern\", \"object\": {}, \"coverPattern\": ",
                object.0
            )
            .unwrap();
            write_pattern_json(out, cover_pattern);
            out.push_str(" }");
        }
    }
}

fn write_win_condition_list(out: &mut String, conditions: &[WinCondition3]) {
    for (index, condition) in conditions.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write_win_condition_json(out, condition);
    }
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
            json_string(name),
            layer,
            comma
        )
        .unwrap();
    }
    out.push_str("  },\n");
    Ok(())
}

fn write_scenes(out: &mut String, parsed: &ParsedPuzzle3) {
    let scenes = if parsed.scenes.is_empty() {
        Vec::new()
    } else {
        parsed.scenes.iter().collect::<Vec<_>>()
    };
    let current_scene = scenes
        .iter()
        .find(|scene| scene.name == "title")
        .copied()
        .or_else(|| scenes.first().copied())
        .map(|scene| scene.name.as_str())
        .unwrap_or("playing");
    write_json_string_field(out, 1, "currentScene", current_scene, true);
    out.push_str("  \"scenes\": [\n");
    if scenes.is_empty() {
        out.push_str("    {\n");
        out.push_str("      \"name\": \"playing\",\n");
        out.push_str("      \"puzzles\": [{ \"slot\": \"board\", \"model\": \"default\" }],\n");
        out.push_str("      \"components\": [{ \"kind\": \"puzzle3\", \"source\": \"board\" }]\n");
        out.push_str("    }\n");
    } else {
        for (index, scene) in scenes.iter().enumerate() {
            let comma = if index + 1 == scenes.len() { "" } else { "," };
            out.push_str("    {\n");
            write_json_string_field(out, 3, "name", &scene.name, true);
            write_scene_layout_json_field(out, 3, &scene.layout, true);
            out.push_str("      \"puzzles\": [");
            for (puzzle_index, puzzle) in scene.puzzles.iter().enumerate() {
                if puzzle_index > 0 {
                    out.push_str(", ");
                }
                write!(
                    out,
                    "{{ \"slot\": {}, \"model\": {} }}",
                    json_string(&puzzle.slot),
                    json_string(&puzzle.model)
                )
                .unwrap();
            }
            out.push_str("],\n");
            out.push_str("      \"keys\": {");
            for (key_index, key) in scene.keys.iter().enumerate() {
                if key_index > 0 {
                    out.push_str(", ");
                }
                write!(
                    out,
                    "{}: {}",
                    json_string(&key.key),
                    scene_action_json(&key.action)
                )
                .unwrap();
            }
            out.push_str("},\n");
            out.push_str("      \"components\": [");
            for (component_index, component) in scene.components.iter().enumerate() {
                if component_index > 0 {
                    out.push_str(", ");
                }
                write_scene_component_json(out, component);
            }
            out.push_str("]\n");
            writeln!(out, "    }}{}", comma).unwrap();
        }
    }
    out.push_str("  ],\n");
}

fn write_scene_component_json(out: &mut String, component: &SceneComponent3) {
    match component {
        SceneComponent3::Title { text, layout } => {
            write!(
                out,
                "{{ \"kind\": \"title\", \"text\": {}",
                json_string(text)
            )
            .unwrap();
            write_layout_json(out, layout);
            out.push_str(" }");
        }
        SceneComponent3::Button {
            label,
            action,
            layout,
        } => {
            write!(
                out,
                "{{ \"kind\": \"button\", \"label\": {}, \"action\": {}",
                json_string(label),
                scene_action_json(action)
            )
            .unwrap();
            write_layout_json(out, layout);
            out.push_str(" }");
        }
        SceneComponent3::LevelMenu {
            levels,
            action,
            layout,
        } => {
            write!(
                out,
                "{{ \"kind\": \"level_menu\", \"levels\": {}, \"action\": {}",
                json_string(levels),
                scene_action_json(action)
            )
            .unwrap();
            write_layout_json(out, layout);
            out.push_str(" }");
        }
        SceneComponent3::Puzzle3 { source, layout } => {
            write!(
                out,
                "{{ \"kind\": \"puzzle3\", \"source\": {}",
                json_string(source)
            )
            .unwrap();
            write_layout_json(out, layout);
            out.push_str(" }");
        }
        SceneComponent3::Row { children, layout } => {
            write_container_json(out, "row", children, layout);
        }
        SceneComponent3::Column { children, layout } => {
            write_container_json(out, "column", children, layout);
        }
        SceneComponent3::Box { children, layout } => {
            write_container_json(out, "box", children, layout);
        }
    }
}

fn write_container_json(
    out: &mut String,
    kind: &str,
    children: &[SceneComponent3],
    layout: &SceneLayout3,
) {
    write!(out, "{{ \"kind\": {kind:?}, \"children\": [").unwrap();
    for (index, child) in children.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write_scene_component_json(out, child);
    }
    out.push(']');
    write_layout_json(out, layout);
    out.push_str(" }");
}

fn write_layout_json(out: &mut String, layout: &SceneLayout3) {
    if layout.size.is_none()
        && layout.gap.is_none()
        && layout.align == SceneLayout3::default().align
    {
        return;
    }
    out.push_str(", \"layout\": {");
    let mut wrote = false;
    if let Some(size) = layout.size {
        write!(
            out,
            "\"size\": {{ \"width\": {}, \"height\": {} }}",
            size.width, size.height
        )
        .unwrap();
        wrote = true;
    }
    if let Some(gap) = layout.gap {
        if wrote {
            out.push_str(", ");
        }
        write!(out, "\"gap\": {gap}").unwrap();
        wrote = true;
    }
    if layout.align != SceneLayout3::default().align {
        if wrote {
            out.push_str(", ");
        }
        write!(
            out,
            "\"align\": {{ \"x\": {}, \"y\": {} }}",
            json_string(align_x_name(layout.align.x)),
            json_string(align_y_name(layout.align.y))
        )
        .unwrap();
    }
    out.push('}');
}

fn write_scene_layout_json_field(
    out: &mut String,
    indent: usize,
    layout: &SceneLayout3,
    comma: bool,
) {
    write_indent(out, indent);
    out.push_str("\"layout\": ");
    write_layout_json_object(out, layout);
    if comma {
        out.push(',');
    }
    out.push('\n');
}

fn write_layout_json_object(out: &mut String, layout: &SceneLayout3) {
    out.push('{');
    let mut wrote = false;
    if let Some(size) = layout.size {
        write!(
            out,
            "\"size\": {{ \"width\": {}, \"height\": {} }}",
            size.width, size.height
        )
        .unwrap();
        wrote = true;
    }
    if let Some(gap) = layout.gap {
        if wrote {
            out.push_str(", ");
        }
        write!(out, "\"gap\": {gap}").unwrap();
        wrote = true;
    }
    if layout.align != SceneLayout3::default().align {
        if wrote {
            out.push_str(", ");
        }
        write!(
            out,
            "\"align\": {{ \"x\": {}, \"y\": {} }}",
            json_string(align_x_name(layout.align.x)),
            json_string(align_y_name(layout.align.y))
        )
        .unwrap();
    }
    out.push('}');
}

fn align_x_name(value: SceneAlignX3) -> &'static str {
    match value {
        SceneAlignX3::Left => "left",
        SceneAlignX3::Center => "center",
        SceneAlignX3::Right => "right",
    }
}

fn align_y_name(value: SceneAlignY3) -> &'static str {
    match value {
        SceneAlignY3::Top => "top",
        SceneAlignY3::Center => "center",
        SceneAlignY3::Bottom => "bottom",
    }
}

fn scene_action_json(action: &SceneAction3) -> String {
    match action {
        SceneAction3::Goto { scene } => {
            format!(
                "{{ \"kind\": \"goto\", \"scene\": {} }}",
                json_string(scene)
            )
        }
        SceneAction3::StartLevels { levels, scene } => {
            format!(
                "{{ \"kind\": \"start_levels\", \"levels\": {}, \"scene\": {} }}",
                json_string(levels),
                json_string(scene)
            )
        }
    }
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
        write_cells_field(out, 3, "cells", &entry.level.cells, names)?;
        out.push('\n');
        writeln!(out, "    }}{}", comma).unwrap();
    }
    out.push_str("  ],\n");
    write_cells_field(out, 1, "cells", &bundle.levels[0].level.cells, names)?;
    out.push_str(",\n");
    Ok(())
}

fn write_level_bundles(out: &mut String, parsed: &ParsedPuzzle3) {
    let Some(bundle) = parsed.level_bundle.as_ref() else {
        return;
    };
    let mut names = vec!["default".to_string(), "levels".to_string()];
    for scene in &parsed.scenes {
        collect_component_levels(&mut names, &scene.components);
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

fn collect_component_levels(names: &mut Vec<String>, components: &[SceneComponent3]) {
    for component in components {
        match component {
            SceneComponent3::Button { action, .. } => collect_action_levels(names, action),
            SceneComponent3::LevelMenu { levels, action, .. } => {
                push_unique_string(names, levels);
                collect_action_levels(names, action);
            }
            SceneComponent3::Row { children, .. }
            | SceneComponent3::Column { children, .. }
            | SceneComponent3::Box { children, .. } => collect_component_levels(names, children),
            SceneComponent3::Title { .. } | SceneComponent3::Puzzle3 { .. } => {}
        }
    }
}

fn collect_action_levels(names: &mut Vec<String>, action: &SceneAction3) {
    if let SceneAction3::StartLevels { levels, .. } = action {
        push_unique_string(names, levels);
    }
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
    cells: &[crate::LevelCell3],
    names: &BTreeMap<ObjectId, String>,
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
                json_string(object_name)
            )
            .unwrap();
        }
        writeln!(out, "] }}{}", comma).unwrap();
    }
    write_indent(out, indent);
    out.push(']');
    Ok(())
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
            .filter_map(|(key, color)| match color {
                SpriteColor3::Transparent => None,
                SpriteColor3::Hex(value) => Some((*key, value)),
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
        out.push_str("      \"bitmap\": [\n");
        let mut rows = Vec::new();
        for (slice_index, slice) in sprite.voxels.slices.iter().enumerate() {
            if slice_index > 0 {
                rows.push(String::new());
            }
            rows.extend(slice.iter().cloned());
        }
        for (row_index, row) in rows.iter().enumerate() {
            let row_comma = if row_index + 1 == rows.len() { "" } else { "," };
            writeln!(out, "        {}{}", json_string(row), row_comma).unwrap();
        }
        out.push_str("      ]\n");
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
