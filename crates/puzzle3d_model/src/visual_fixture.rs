use std::collections::BTreeMap;
use std::fmt::Write;

use crate::{
    Direction3, Guard3, Lifecycle3, LifecycleCommand3, MatchCell3, ObjectId, ObjectSelector3,
    ParsedPuzzle3, Pattern3, Rule3, RuleApplication3, RuleEffect3, SelectorCatalog3, SelectorTag3,
    Size3, SpriteColor3, SpriteSet3, ViewportFollow3, ViewportHeight3, ViewportMode3,
    WinCondition3, WriteOp3,
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
    let object_names = object_names(&parsed.catalog);
    let title = title
        .map(str::to_string)
        .unwrap_or_else(|| fixture_title(parsed));

    let mut out = String::new();
    out.push_str("{\n");
    write_json_string_field(&mut out, 1, "title", &title, true);
    let _ = writeln!(out, "  \"layerCount\": {},", parsed.game.layer_count);
    write_size_field(&mut out, 1, "size", bundle.levels[0].level.size, true);
    write_camera(&mut out, parsed);
    write_settings(&mut out, parsed);
    write_viewport(&mut out, parsed);
    write_directions(&mut out);
    write_direction_sets(&mut out);
    write_controls(&mut out, parsed);
    write_inputs(&mut out, parsed);
    write_rules(&mut out, "rules", &parsed.rules);
    write_lifecycle(&mut out, &parsed.lifecycle);
    if let Some(condition) = parsed.win_condition.as_ref() {
        write_win_condition(&mut out, condition);
    }
    write_objects(&mut out, parsed, &object_names)?;
    write_scenes(&mut out, scene_fields_json);
    write_levels(&mut out, parsed, &object_names)?;
    write_level_bundles(&mut out, parsed, level_bundle_names);
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
    let pixelate = &parsed.settings.pixelate;
    let _ = writeln!(
        out,
        "  \"settings\": {{ \"interactiveLook\": {}, \"interactiveZoom\": {}, \"grid\": {{ \"visibility\": {}, \"occupied_cells\": {} }}, \"shade\": {}, \"pixelate\": {{ \"enabled\": {}, \"scale\": {}, \"smoothing\": {} }} }},",
        camera.interactive_look,
        camera.interactive_zoom,
        if parsed.settings.grid.occupied_cells {
            1
        } else {
            0
        },
        parsed.settings.grid.occupied_cells,
        parsed.settings.sprite.shade,
        pixelate.enabled,
        pixelate.scale,
        pixelate.smoothing,
    );
}

fn write_viewport(out: &mut String, parsed: &ParsedPuzzle3) {
    let viewport = &parsed.settings.viewport;
    let Some(framing) = viewport.framing else {
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
    out.push_str("  \"viewport\": { ");
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
    out.push_str(" } },\n");
}

fn viewport_focus_objects(parsed: &ParsedPuzzle3) -> Vec<ObjectId> {
    let focus = &parsed.settings.viewport.focus;
    let selector = viewport_focus_selector(focus, &parsed.catalog);
    let mut objects = selector
        .and_then(|selector| parsed.catalog.resolve(&selector).ok())
        .map(|resolved| resolved.alternatives)
        .unwrap_or_default();
    objects.sort_by_key(|object| object.0);
    objects.dedup();
    objects
}

fn viewport_focus_selector(focus: &str, catalog: &SelectorCatalog3) -> Option<ObjectSelector3> {
    let parts = focus.split(':').collect::<Vec<_>>();
    if parts.len() > 1 {
        return Some(ObjectSelector3::variant(
            parts[0],
            parts[1..]
                .iter()
                .map(|part| {
                    if *part == "*" {
                        SelectorTag3::any()
                    } else {
                        SelectorTag3::value(*part)
                    }
                })
                .collect(),
        ));
    }
    if catalog.groups.iter().any(|group| group.name == focus) {
        return Some(ObjectSelector3::group(focus));
    }
    if catalog.objects.iter().any(|object| object.name == focus) {
        return Some(ObjectSelector3::object(focus));
    }
    None
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
    if parsed
        .game
        .inputs
        .iter()
        .any(|input| !input.keys.is_empty())
    {
        for input in &parsed.game.inputs {
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
    out.push_str("], \"effects\": [");
    for (effect_index, effect) in rule.effects.iter().enumerate() {
        if effect_index > 0 {
            out.push_str(", ");
        }
        write_rule_effect_json(out, effect);
    }
    out.push_str("] }");
}

fn write_rule_effect_json(out: &mut String, effect: &RuleEffect3) {
    match effect {
        RuleEffect3::UpdateGlobal { global, op, value } => {
            write!(
                out,
                "{{ \"kind\": \"update_global\", \"global\": {}, \"op\": \"{}\", \"value\": {} }}",
                global.0,
                global_update_op_name(*op),
                value
            )
            .unwrap();
        }
        RuleEffect3::SetCameraYaw(value) => {
            write!(
                out,
                "{{ \"kind\": \"set_camera\", \"variable\": \"yaw\", \"value\": {} }}",
                value
            )
            .unwrap();
        }
        RuleEffect3::SetCameraPitch(value) => {
            write!(
                out,
                "{{ \"kind\": \"set_camera\", \"variable\": \"pitch\", \"value\": {} }}",
                value
            )
            .unwrap();
        }
        RuleEffect3::SetCameraZoom(value) => {
            write!(
                out,
                "{{ \"kind\": \"set_camera\", \"variable\": \"zoom\", \"value\": {} }}",
                format_zoom(*value)
            )
            .unwrap();
        }
        RuleEffect3::ResetCamera => {
            out.push_str("{ \"kind\": \"reset_camera\" }");
        }
    }
}

fn global_update_op_name(op: puzzle_kernel::GlobalUpdateOp) -> &'static str {
    match op {
        puzzle_kernel::GlobalUpdateOp::Set => "set",
        puzzle_kernel::GlobalUpdateOp::Add => "add",
        puzzle_kernel::GlobalUpdateOp::Subtract => "subtract",
        puzzle_kernel::GlobalUpdateOp::Multiply => "multiply",
        puzzle_kernel::GlobalUpdateOp::Divide => "divide",
        puzzle_kernel::GlobalUpdateOp::Remainder => "remainder",
    }
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
    out.push_str("], \"requireObjectSets\": [");
    for (index, object_set) in cell.require_object_sets.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write!(
            out,
            "{{ \"binding\": {}, \"layer\": {}, \"objects\": [",
            object_set.binding, object_set.layer.0
        )
        .unwrap();
        write_object_id_array(out, &object_set.objects);
        out.push_str("] }");
    }
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
        WriteOp3::Add {
            component,
            offset,
            object,
        } => {
            write!(
                out,
                "{{ \"kind\": \"add\", \"component\": {}, \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"object\": {} }}",
                component, offset.dx, offset.dy, offset.dz, object.0
            )
            .unwrap();
        }
        WriteOp3::AddObjectSet {
            component,
            offset,
            binding,
        } => {
            write!(
                out,
                "{{ \"kind\": \"add_object_set\", \"component\": {}, \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"binding\": {} }}",
                component, offset.dx, offset.dy, offset.dz, binding
            )
            .unwrap();
        }
        WriteOp3::Remove {
            component,
            offset,
            object,
        } => {
            write!(
                out,
                "{{ \"kind\": \"remove\", \"component\": {}, \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"object\": {} }}",
                component, offset.dx, offset.dy, offset.dz, object.0
            )
            .unwrap();
        }
        WriteOp3::RemoveObjectSet {
            component,
            offset,
            binding,
        } => {
            write!(
                out,
                "{{ \"kind\": \"remove_object_set\", \"component\": {}, \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"binding\": {} }}",
                component, offset.dx, offset.dy, offset.dz, binding
            )
            .unwrap();
        }
        WriteOp3::Replace {
            component,
            offset,
            remove,
            add,
        } => {
            write!(
                out,
                "{{ \"kind\": \"replace\", \"component\": {}, \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"remove\": {}, \"add\": {} }}",
                component, offset.dx, offset.dy, offset.dz, remove.0, add.0
            )
            .unwrap();
        }
        WriteOp3::Move {
            component,
            from_offset,
            to_offset,
            object,
        } => {
            write!(
                out,
                "{{ \"kind\": \"move\", \"component\": {}, \"fromOffset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"toOffset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"object\": {} }}",
                component,
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
        WriteOp3::MoveObjectSet {
            component,
            from_offset,
            to_offset,
            binding,
        } => {
            write!(
                out,
                "{{ \"kind\": \"move_object_set\", \"component\": {}, \"fromOffset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"toOffset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"binding\": {} }}",
                component,
                from_offset.dx,
                from_offset.dy,
                from_offset.dz,
                to_offset.dx,
                to_offset.dy,
                to_offset.dz,
                binding
            )
            .unwrap();
        }
        WriteOp3::SetScratch {
            component,
            offset,
            object,
            scratch,
            value,
        } => {
            write!(
                out,
                "{{ \"kind\": \"set_scratch\", \"component\": {}, \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"object\": {}, \"scratch\": {}, \"value\": ",
                component, offset.dx, offset.dy, offset.dz, object.0, scratch.0
            )
            .unwrap();
            write_optional_i64(out, *value);
            out.push_str(" }");
        }
        WriteOp3::SetObjectSetScratch {
            component,
            offset,
            binding,
            scratch,
            value,
        } => {
            write!(
                out,
                "{{ \"kind\": \"set_object_set_scratch\", \"component\": {}, \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"binding\": {}, \"scratch\": {}, \"value\": ",
                component, offset.dx, offset.dy, offset.dz, binding, scratch.0
            )
            .unwrap();
            write_optional_i64(out, *value);
            out.push_str(" }");
        }
        WriteOp3::RemoveScratch {
            component,
            offset,
            object,
            scratch,
            value,
            match_value,
        } => {
            write!(
                out,
                "{{ \"kind\": \"remove_scratch\", \"component\": {}, \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"object\": {}, \"scratch\": {}, \"value\": ",
                component, offset.dx, offset.dy, offset.dz, object.0, scratch.0
            )
            .unwrap();
            write_optional_i64(out, *value);
            write!(
                out,
                ", \"match\": \"{}\" }}",
                scratch_value_match_name(*match_value)
            )
            .unwrap();
        }
        WriteOp3::RemoveObjectSetScratch {
            component,
            offset,
            binding,
            scratch,
            value,
            match_value,
        } => {
            write!(
                out,
                "{{ \"kind\": \"remove_object_set_scratch\", \"component\": {}, \"offset\": {{ \"dx\": {}, \"dy\": {}, \"dz\": {} }}, \"binding\": {}, \"scratch\": {}, \"value\": ",
                component, offset.dx, offset.dy, offset.dz, binding, scratch.0
            )
            .unwrap();
            write_optional_i64(out, *value);
            write!(
                out,
                ", \"match\": \"{}\" }}",
                scratch_value_match_name(*match_value)
            )
            .unwrap();
        }
    }
}

fn write_optional_i64(out: &mut String, value: Option<i64>) {
    match value {
        Some(value) => write!(out, "{}", value).unwrap(),
        None => out.push_str("null"),
    }
}

fn scratch_value_match_name(match_value: puzzle_kernel::ScratchValueMatch) -> &'static str {
    match match_value {
        puzzle_kernel::ScratchValueMatch::Any => "any",
        puzzle_kernel::ScratchValueMatch::Exact => "exact",
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
    cells: &[crate::LevelCell3],
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
