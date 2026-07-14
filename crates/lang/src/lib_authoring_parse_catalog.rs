fn parse_map_definition(
    lines: &[String],
    start: usize,
    value_sets: &HashMap<String, Vec<String>>,
) -> Result<(ValueMap, usize), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let ["map", name, axis] = header.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "map header must be: map <name> <tag_set>",
        ));
    };
    if !is_identifier(name) {
        return Err(parse_error(&lines[start], "map name must be an identifier"));
    }
    let value_set_values = value_sets
        .get(*axis)
        .ok_or_else(|| parse_error(&lines[start], "map tag set must name an existing tag set"))?;

    let mut values = HashMap::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            [from, "->", to] => {
                if !value_set_values.iter().any(|value| value == from) {
                    return Err(parse_error(&lines[i], "map input is not in tag set"));
                }
                if !value_set_values.iter().any(|value| value == to) {
                    return Err(parse_error(&lines[i], "map output is not in tag set"));
                }
                if values
                    .insert((*from).to_string(), (*to).to_string())
                    .is_some()
                {
                    return Err(parse_error(&lines[i], "duplicate map input"));
                }
            }
            _ => {
                return Err(parse_error(
                    &lines[i],
                    "map row must be: <value> -> <value>",
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "map missing closing brace"));
    }

    for value in value_set_values {
        if !values.contains_key(value) {
            return Err(parse_error(&lines[start], "map must cover every tag value"));
        }
    }

    Ok((
        ValueMap {
            name: (*name).to_string(),
            axis: (*axis).to_string(),
            values,
        },
        i + 1,
    ))
}

fn project_builtin_completion_symbols(sink: &mut SurfaceSink) {
    let symbols = sink.completion_symbols_mut();
    for (name, values) in [
        ("directions", vec!["up", "down", "left", "right"]),
        ("horizontal", vec!["left", "right"]),
        ("vertical", vec!["up", "down"]),
    ] {
        symbols.value_set_names.insert(name.to_string());
        symbols.direction_sets.insert(name.to_string());
        let values = values
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<String>>();
        symbols.object_name_atoms.extend(values.iter().cloned());
        symbols.value_sets.insert(name.to_string(), values);
    }
    symbols.directions.extend(
        ["up", "down", "left", "right"]
            .into_iter()
            .map(str::to_string),
    );

    add_surface_completion_effect_commands(
        surface_builtin_effect_commands_for_catalog(),
        &mut symbols.effects,
        &mut symbols.emissions,
    );
    add_surface_completion_effect_commands(
        surface_model_effect_commands_for_catalog(),
        &mut symbols.model_effects,
        &mut symbols.emissions,
    );
    add_surface_completion_effect_commands(
        surface_scene_effect_commands_for_catalog(),
        &mut symbols.scene_effects,
        &mut symbols.emissions,
    );
}

fn record_parser_catalog_completion_symbols(catalog: &Catalog, sink: &mut SurfaceSink) {
    let symbols = sink.completion_symbols_mut();
    symbols.objects.extend(
        catalog
            .object_schemas
            .keys()
            .filter(|name| !catalog.object_groups.contains_key(*name))
            .cloned(),
    );
    symbols.groups.extend(catalog.object_groups.keys().cloned());
    symbols
        .inputs
        .extend(catalog.input_labels.values().cloned());
    symbols
        .states
        .extend(catalog.variable_labels.values().cloned());
    symbols
        .condition_defs
        .extend(catalog.condition_labels.values().cloned());
    for (name, values) in &catalog.value_sets {
        symbols.value_set_names.insert(name.clone());
        symbols
            .value_sets
            .entry(name.clone())
            .or_insert(values.clone());
        symbols.object_name_atoms.extend(values.iter().cloned());
        if values
            .iter()
            .all(|value| surface_completion_direction_value(value))
        {
            symbols.direction_sets.insert(name.clone());
        }
    }
    for (object, axes) in &catalog.object_axes {
        symbols
            .object_axes
            .entry(object.clone())
            .or_insert(axes.clone());
        symbols.object_name_atoms.extend(axes.iter().cloned());
    }
    for name in catalog.mark_names.keys() {
        if !name.starts_with("__") {
            symbols.markes.insert(name.clone());
        }
    }
}

fn surface_completion_direction_value(value: &str) -> bool {
    matches!(value, "up" | "down" | "left" | "right")
}

fn normalize_surface_completion_symbols(sink: &mut SurfaceSink) {
    let symbols = sink.completion_symbols_mut();
    let value_set_names = symbols.value_set_names.iter().cloned().collect::<Vec<_>>();
    let group_names = symbols.groups.iter().cloned().collect::<Vec<_>>();
    for name in value_set_names {
        sink.completion_symbols_mut()
            .object_name_atoms
            .remove(&name);
    }
    for name in group_names {
        sink.completion_symbols_mut().objects.remove(&name);
        sink.completion_symbols_mut()
            .object_name_atoms
            .remove(&name);
    }
}

fn add_surface_completion_effect_commands(
    commands: Vec<(&'static str, SemanticKind)>,
    effects: &mut BTreeSet<String>,
    emissions: &mut BTreeSet<String>,
) {
    for (command, kind) in commands {
        match kind {
            SemanticKind::Emission => {
                emissions.insert(command.to_string());
            }
            SemanticKind::Effect => {
                effects.insert(command.to_string());
            }
            _ => {}
        }
    }
}

fn surface_builtin_effect_commands_for_catalog() -> Vec<(&'static str, SemanticKind)> {
    [
        "apply",
        "again",
        "back",
        "cancel",
        "checkpoint",
        "clear_checkpoint",
        "close",
        "clear_history",
        "clear_undo_history",
        "clear_game_progress",
        "continue",
        "copy",
        "create",
        "delete",
        "enter",
        "focus",
        "goto",
        "hide",
        "input",
        "component_effect",
        "load",
        "message",
        "next_level",
        "open",
        "pause_music",
        "play_music",
        "reset",
        "restart",
        "resume",
        "resume_music",
        "sfx",
        "show",
        "start",
        "stop_music",
        "toggle",
        "wait",
        "win",
    ]
    .into_iter()
    .filter_map(|command| {
        let kind = if command == "sfx" {
            SemanticKind::Effect
        } else {
            match rewrite_effect_command_syntax(command) {
                Some(RewriteEffectCommandSyntax::Emission) => SemanticKind::Emission,
                Some(RewriteEffectCommandSyntax::Effect) => SemanticKind::Effect,
                None if scene_effect_command_syntax(command).is_some() || command == "restart" => {
                    SemanticKind::Effect
                }
                None => return None,
            }
        };
        Some((command, kind))
    })
    .collect()
}

fn surface_model_effect_commands_for_catalog() -> Vec<(&'static str, SemanticKind)> {
    [
        "again",
        "cancel",
        "checkpoint",
        "clear_checkpoint",
        "message",
        "next_level",
        "restart",
        "sfx",
        "wait",
        "win",
    ]
    .into_iter()
    .filter_map(|command| {
        let kind = if command == "sfx" {
            SemanticKind::Effect
        } else {
            match rewrite_effect_command_syntax(command)? {
                RewriteEffectCommandSyntax::Emission => SemanticKind::Emission,
                RewriteEffectCommandSyntax::Effect => SemanticKind::Effect,
            }
        };
        Some((command, kind))
    })
    .collect()
}

fn surface_scene_effect_commands_for_catalog() -> Vec<(&'static str, SemanticKind)> {
    [
        "apply",
        "clear",
        "clear_game_progress",
        "clear_history",
        "clear_undo_history",
        "component_effect",
        "copy",
        "goto",
        "input",
        "load",
        "message",
        "pause_music",
        "play_music",
        "resume_music",
        "sfx",
        "start",
        "stop_music",
        "wait",
    ]
    .into_iter()
    .filter_map(|command| {
        scene_effect_command_syntax(command)?;
        Some((command, SemanticKind::Effect))
    })
    .collect()
}

fn build_puzzle_catalog(
    model: &model_syntax::PuzzleModelSyntax,
) -> Result<Catalog, DiagnosticReport> {
    let mut catalog = Catalog::for_dimension(model.dimension);
    let mut named_layers = HashMap::<String, u16>::new();
    let mut layer_count = None::<u16>;

    for entry in &model.catalog_entries {
        let tokens = split_header_tokens(&entry.header.text);
        if tokens.as_slice() != ["tags"] {
            continue;
        }
        for line in &entry.body {
            if line.text.trim().is_empty() {
                continue;
            }
            let assignment = puzzle_authoring::selector_assignment_surface(&line.text)
                .ok_or_else(|| parse_error(&line.text, "tag row must be: <name> = <value...>"))?;
            parse_tag_set_directive(
                assignment.name,
                &assignment.selectors,
                &line.text,
                &mut catalog,
            )?;
        }
    }

    for entry in &model.catalog_entries {
        let tokens = split_header_tokens(&entry.header.text);
        if tokens.first().copied() != Some("map") {
            continue;
        }
        let lines = catalog_entry_lines(entry);
        let value_sets = catalog_value_sets(&catalog);
        let (map, next) = parse_map_definition(&lines, 0, &value_sets)?;
        if next != lines.len() {
            return Err(parse_error(
                &entry.header.text,
                "map block was not fully consumed",
            ));
        }
        if catalog.maps.insert(map.name.clone(), map).is_some() {
            return Err(parse_error(&entry.header.text, "duplicate map"));
        }
    }

    let pending_groups = collect_puzzle_group_declarations_from_entries(&model.catalog_entries)?;
    let mut resolved_groups = HashSet::<String>::new();
    for entry in &model.catalog_entries {
        let tokens = split_header_tokens(&entry.header.text);
        match tokens.as_slice() {
            ["slots"] => {
                let mut lines = entry
                    .body
                    .iter()
                    .map(|line| line.text.clone())
                    .collect::<Vec<_>>();
                lines.push(BLOCK_CLOSE.to_string());
                let next = parse_layers_block(
                    &lines,
                    0,
                    &mut named_layers,
                    &mut layer_count,
                    &mut catalog,
                    &pending_groups,
                    &mut resolved_groups,
                )?;
                if next != lines.len() {
                    return Err(parse_error(
                        &entry.header.text,
                        "slots block was not fully consumed",
                    ));
                }
                refresh_layer_tags_and_value_sets(&named_layers, &mut catalog);
            }
            ["slots", count] => {
                layer_count = Some(parse_u16(
                    Some(count),
                    &entry.header.text,
                    "missing layer count",
                )?);
            }
            ["slots", ..] => {
                return Err(parse_error(&entry.header.text, "slots header is malformed"));
            }
            _ => {}
        }
    }
    resolve_pending_group_definitions(&pending_groups, None, &mut resolved_groups, &mut catalog)?;

    for entry in &model.catalog_entries {
        if split_header_tokens(&entry.header.text).as_slice() != ["marks"] {
            continue;
        }
        let lines = catalog_entry_lines(entry);
        let next = parse_mark_block(&lines, 0, &mut catalog)?;
        if next != lines.len() {
            return Err(parse_error(
                &entry.header.text,
                "marks block was not fully consumed",
            ));
        }
    }

    catalog.layer_count = layer_count;
    catalog.named_layers = named_layers;
    Ok(catalog)
}

fn catalog_entry_lines(entry: &model_syntax::PuzzleEntrySyntax) -> Vec<String> {
    let mut lines = Vec::with_capacity(entry.body.len() + 2);
    lines.push(entry.header.text.clone());
    lines.extend(entry.body.iter().map(|line| line.text.clone()));
    lines.push(BLOCK_CLOSE.to_string());
    lines
}

fn collect_puzzle_group_declarations_from_entries(
    entries: &[model_syntax::PuzzleEntrySyntax],
) -> Result<Vec<PendingGroupDefinition>, DiagnosticReport> {
    let mut groups = Vec::new();
    let mut names = HashSet::<String>::new();
    for entry in entries {
        let tokens = split_header_tokens(&entry.header.text);
        match tokens.as_slice() {
            ["groups"] => {}
            ["groups", ..] => {
                return Err(parse_error(
                    &entry.header.text,
                    "groups block must be: groups { ... }",
                ));
            }
            _ => continue,
        }
        for line in &entry.body {
            if line.text.trim().is_empty() {
                continue;
            }
            let Some(assignment) = puzzle_authoring::selector_assignment_surface(&line.text) else {
                return Err(parse_error(
                    &line.text,
                    "group row must be: <name> = <selector...>",
                ));
            };
            validate_selector_alias_name(assignment.name, &line.text, "group name")?;
            if !names.insert(assignment.name.to_string()) {
                return Err(parse_error(&line.text, "duplicate group"));
            }
            groups.push(PendingGroupDefinition {
                name: assignment.name.to_string(),
                selectors: assignment
                    .selectors
                    .iter()
                    .map(|selector| (*selector).to_string())
                    .collect(),
                source_line: line.text.clone(),
            });
        }
    }
    Ok(groups)
}

fn parser_surface_catalog_from_source_scan(
    source_scan: &source::SurfaceSourceScan,
) -> Result<Catalog, DiagnosticReport> {
    let logical_lines = source_scan.strict_logical_lines()?;
    let parts = parse_document_source_parts_from_logical_lines(logical_lines)?;
    let model = parts.models.first().ok_or_else(|| {
        DiagnosticReport::error("parser catalog requires a puzzle model".to_string())
    })?;
    build_puzzle_catalog(model)
}

/// Editor-specific integration of parser-owned authoring facts. Unlike full
/// compilation it never resolves rules, routines, or lifecycle programs.
pub(crate) struct LevelEditorIntegration {
    pub(crate) catalog: Catalog,
    pub(crate) empty_char: Option<char>,
    pub(crate) visuals: VisualsDef,
    pub(crate) levels: Vec<LevelEditorIntegratedLevel>,
    pub(crate) diagnostics: Vec<String>,
}

pub(crate) struct LevelEditorIntegratedLevel {
    pub(crate) source_level_index: usize,
    pub(crate) name: String,
    pub(crate) state: State,
    pub(crate) layers: Vec<State>,
    pub(crate) regions: Vec<LevelRegionDef>,
    pub(crate) char_objects: HashMap<char, Vec<ObjectId>>,
}

pub(crate) fn integrate_level_editor_authoring(
    source: &str,
) -> Result<LevelEditorIntegration, DiagnosticReport> {
    let parts = parse_document_source_parts(source)?;
    integrate_level_editor_document_parts(parts)
}

fn integrate_level_editor_document_parts(
    parts: DocumentSourceParts,
) -> Result<LevelEditorIntegration, DiagnosticReport> {
    let dimension = parts
        .models
        .first()
        .map(|model| model.dimension)
        .unwrap_or_default();
    let lines = parts
        .model_lines
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>();
    let mut catalog = Catalog::for_dimension(dimension);
    let mut level_blocks = Vec::<LevelBlock>::new();
    let mut pending_level_blocks = Vec::<PendingLevelBlock>::new();
    let mut pending_visual_blocks = Vec::<usize>::new();
    let mut render_overlays = Vec::<(Vec<ObjectId>, char)>::new();
    let mut empty_char = Some('.');
    let mut diagnostics = Vec::<String>::new();
    let mut i = 0usize;
    while i < lines.len() {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["puzzle", _] => match parse_editor_puzzle_catalog(&lines, i, &mut catalog, false) {
                Ok((next_i, mut levels, mut visuals, _)) => {
                    pending_level_blocks.append(&mut levels);
                    pending_visual_blocks.append(&mut visuals);
                    i = next_i;
                }
                Err(report) => {
                    diagnostics.push(report.to_string());
                    i = recover_after_directive_error(&lines, i);
                }
            },
            ["levels", ..] => {
                pending_level_blocks.push(PendingLevelBlock::levels(i, None));
                match collect_levels_authoring_entry(&lines, i) {
                    Ok((_, next_i)) => i = next_i,
                    Err(report) => {
                        diagnostics.push(report.to_string());
                        pending_level_blocks.pop();
                        i = recover_after_directive_error(&lines, i);
                    }
                }
            }
            ["level", ..] => {
                pending_level_blocks.push(PendingLevelBlock::level(i, None));
                match parse_level_block(&lines, i, 0) {
                    Ok((_, next_i)) => i = next_i,
                    Err(report) => {
                        diagnostics.push(report.to_string());
                        pending_level_blocks.pop();
                        i = recover_after_directive_error(&lines, i);
                    }
                }
            }
            ["sprites", ..] => {
                pending_visual_blocks.push(i);
                match collect_authoring_entry(&lines, i, AuthoringEntryOwner::DocumentVisuals) {
                    Ok((_, next_i)) => i = next_i,
                    Err(report) => {
                        diagnostics.push(report.to_string());
                        pending_visual_blocks.pop();
                        i = recover_after_directive_error(&lines, i);
                    }
                }
            }
            _ => i += 1,
        }
    }
    for pending in &pending_level_blocks {
        if let Err(report) = parse_pending_level_block(
            &lines,
            pending,
            &mut level_blocks,
            &mut catalog,
            &mut render_overlays,
            &mut empty_char,
        ) {
            diagnostics.push(report.to_string());
        }
    }
    let mut visuals = VisualsDef::default();
    for visual_start in pending_visual_blocks {
        if let Err(report) = parse_visuals_block(&lines, visual_start, &catalog, &mut visuals) {
            diagnostics.push(report.to_string());
        }
    }
    let layer_count = catalog
        .object_defs
        .iter()
        .filter_map(|object| (object.layer_id.0 != UNASSIGNED_LAYER).then_some(object.layer_id.0))
        .max()
        .map_or(0, |layer| layer.saturating_add(1));
    let game = CompiledGame::new(layer_count, catalog.object_defs.clone(), Vec::new());
    let mut levels = Vec::new();
    let empty_char = empty_char.expect("level editor always reserves dot for empty");
    for (source_level_index, level) in level_blocks.into_iter().enumerate() {
        let level_name = level.name.clone();
        let parsed = (|| {
            let body = parse_level_body_for_editor(&level, &catalog, empty_char)?;
            let mut char_objects = catalog.char_objects.clone();
            char_objects.extend(body.local_char_objects);
            let parsed = parse_level(&game, &body.lines, Some(empty_char), &char_objects, &[])?;
            Ok::<_, DiagnosticReport>(LevelEditorIntegratedLevel {
                source_level_index,
                name: level.name,
                state: parsed.state,
                layers: parsed.layer_states,
                regions: parsed.regions,
                char_objects,
            })
        })();
        match parsed {
            Ok(level) => levels.push(level),
            Err(report) => diagnostics.push(format!("level `{level_name}`: {report}")),
        }
    }
    Ok(LevelEditorIntegration {
        catalog,
        empty_char: Some(empty_char),
        visuals,
        levels,
        diagnostics,
    })
}

fn parse_editor_puzzle_catalog(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
    strict: bool,
) -> Result<(usize, Vec<PendingLevelBlock>, Vec<usize>, u16), DiagnosticReport> {
    if let Err(error) = collect_puzzle_tag_declarations(lines, start + 1, catalog)
        && strict
    {
        return Err(error);
    }
    let pending_groups = match collect_puzzle_group_declarations(lines, start + 1) {
        Ok(groups) => groups,
        Err(error) if strict => return Err(error),
        Err(_) => Vec::new(),
    };
    let mut resolved_groups = HashSet::<String>::new();
    let mut named_layers = HashMap::<String, u16>::new();
    let mut layer_count = None::<u16>;
    let puzzle_name = split_header_tokens(&lines[start])
        .get(1)
        .map(|name| (*name).to_string());
    let mut pending_level_blocks = Vec::<PendingLevelBlock>::new();
    let mut pending_visual_blocks = Vec::<usize>::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        let directive = puzzle_authoring::puzzle_directive_surface(&lines[i]);
        match directive {
            puzzle_authoring::PuzzleDirectiveSurface::Empty => i += 1,
            puzzle_authoring::PuzzleDirectiveSurface::Tags => {
                if tokens.as_slice() != ["tags"] {
                    if strict {
                        return Err(parse_error(&lines[i], "tags header must be: tags"));
                    }
                    i = recover_after_directive_error(lines, i);
                } else {
                    i = skip_tags_block(lines, i).unwrap_or(i + 1);
                }
            }
            puzzle_authoring::PuzzleDirectiveSurface::Slots => match tokens.as_slice() {
                ["slots"] => {
                    let parsed = parse_layers_block(
                        lines,
                        i + 1,
                        &mut named_layers,
                        &mut layer_count,
                        catalog,
                        &pending_groups,
                        &mut resolved_groups,
                    );
                    i = match parsed {
                        Ok(next_i) => next_i,
                        Err(error) if strict => return Err(error),
                        Err(_) => recover_after_directive_error(lines, i),
                    };
                    refresh_layer_tags_and_value_sets(&named_layers, catalog);
                }
                ["slots", count] => {
                    match parse_u16(Some(count), &lines[i], "missing layer count") {
                        Ok(count) => layer_count = Some(count),
                        Err(error) if strict => return Err(error),
                        Err(_) => {}
                    }
                    i += 1;
                }
                _ if strict => {
                    return Err(parse_error(&lines[i], "slots header is malformed"));
                }
                _ => i = recover_after_directive_error(lines, i),
            },
            puzzle_authoring::PuzzleDirectiveSurface::Groups => {
                i = match collect_authoring_entry(lines, i, AuthoringEntryOwner::PuzzleGroups) {
                    Ok((_, next_i)) => next_i,
                    Err(error) if strict => return Err(error),
                    Err(_) => recover_after_directive_error(lines, i),
                };
                let resolved = resolve_pending_group_definitions(
                    &pending_groups,
                    None,
                    &mut resolved_groups,
                    catalog,
                );
                if let Err(error) = resolved
                    && strict
                {
                    return Err(error);
                }
            }
            puzzle_authoring::PuzzleDirectiveSurface::Levels => {
                pending_level_blocks.push(PendingLevelBlock::levels(i, puzzle_name.clone()));
                i = collect_levels_authoring_entry(lines, i)?.1;
            }
            puzzle_authoring::PuzzleDirectiveSurface::Level => {
                pending_level_blocks.push(PendingLevelBlock::level(i, puzzle_name.clone()));
                i = parse_level_block(lines, i, 0)?.1;
            }
            puzzle_authoring::PuzzleDirectiveSurface::Sprites => {
                pending_visual_blocks.push(i);
                i = collect_authoring_entry(lines, i, AuthoringEntryOwner::DocumentVisuals)?.1;
            }
            _ => match tokens.as_slice() {
                ["map", ..] => {
                    let value_sets = catalog_value_sets(catalog);
                    match parse_map_definition(lines, i, &value_sets) {
                        Ok((map, next_i)) => {
                            catalog.maps.insert(map.name.clone(), map);
                            i = next_i;
                        }
                        Err(error) if strict => return Err(error),
                        Err(_) => i = recover_after_directive_error(lines, i),
                    }
                }
                ["marks"] => {
                    i = match parse_mark_block(lines, i, catalog) {
                        Ok(next_i) => next_i,
                        Err(error) if strict => return Err(error),
                        Err(_) => recover_after_directive_error(lines, i),
                    };
                }
                _ => i = recover_after_directive_error(lines, i),
            },
        }
    }
    if let Err(error) =
        resolve_pending_group_definitions(&pending_groups, None, &mut resolved_groups, catalog)
        && strict
    {
        return Err(error);
    }
    Ok((
        i.saturating_add(1),
        pending_level_blocks,
        pending_visual_blocks,
        layer_count.unwrap_or(0),
    ))
}

fn parse_tags_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        let assignment = puzzle_authoring::selector_assignment_surface(line)
            .ok_or_else(|| parse_error(line, "tag row must be: <name> = <value...>"))?;
        parse_tag_set_directive(assignment.name, &assignment.selectors, line, catalog)?;
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "tags missing closing brace"));
    }
    Ok(i + 1)
}

fn collect_puzzle_tag_declarations(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    let mut i = start;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            [] => i += 1,
            ["tags"] => {
                i = parse_tags_block(lines, i, catalog)?;
            }
            ["tags", ..] => {
                return Err(parse_error(&lines[i], "tags header must be: tags"));
            }
            _ => {
                i = recover_after_directive_error(lines, i);
            }
        }
    }
    Ok(())
}

fn skip_tags_block(lines: &[String], start: usize) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "tags missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_tag_set_directive(
    name: &str,
    values: &[&str],
    line: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    validate_identifier(name, line, "tag set name")?;
    let (expanded_values, value_type) =
        parse_tag_domain_values(values, &catalog.numeric_variable_defaults, line)?;
    if is_builtin_value_set(name) {
        return Err(parse_error(line, "built-in tag set cannot be redefined"));
    }
    if catalog.value_sets.contains_key(name) || catalog.object_axes.contains_key(name) {
        return Err(parse_error(line, "duplicate tag set"));
    }
    catalog
        .object_axes
        .insert(name.to_string(), expanded_values);
    catalog.axis_types.insert(name.to_string(), value_type);
    Ok(())
}

pub(crate) fn parse_tag_domain_values(
    values: &[&str],
    numeric_variables: &HashMap<String, i64>,
    line: &str,
) -> Result<(Vec<String>, ValueType), DiagnosticReport> {
    if values.is_empty() {
        return Err(parse_error(line, "tag set must have at least one value"));
    }
    if matches!(
        values.first().copied(),
        Some("rotation" | "translation" | "angle" | "vec2")
    ) {
        return Err(parse_error(
            line,
            "tag value types are inferred from literals; use 0deg or (<x>, <y>) values",
        ));
    }
    let frame3_domain = crate::frame3_literal::parse_frame3_domain(&values.join(" "))
        .map_err(|error| parse_error(line, &error))?;
    let (expanded_values, value_type) = if let Some(values) = frame3_domain {
        (values, ValueType::Frame3)
    } else if values.iter().all(|value| {
        let value = value.trim();
        value.starts_with('(') && value.ends_with(')')
    }) {
        (parse_vec2_domain_values(values, line)?, ValueType::Vec2)
    } else if values.iter().any(|value| value.contains("deg")) {
        (parse_angle_domain_values(values, line)?, ValueType::Angle)
    } else if values.contains(&"step")
        && values
            .iter()
            .any(|value| value.contains("...") || value.contains("..<"))
    {
        parse_numeric_domain_range(values, line)?
    } else {
        let expanded = expand_numeric_ranges_in_value_list(values, numeric_variables, line)?;
        let value_type = infer_tag_value_type(&expanded, line)?;
        (
            normalize_tag_values(expanded, value_type, line)?,
            value_type,
        )
    };
    if expanded_values.is_empty() {
        return Err(parse_error(line, "tag set must have at least one value"));
    }
    Ok((expanded_values, value_type))
}

fn parse_numeric_domain_range(
    values: &[&str],
    line: &str,
) -> Result<(Vec<String>, ValueType), DiagnosticReport> {
    let body = values.join(" ");
    let (range, step) = split_axis_range_and_step(&body, line)?;
    let (start, end, inclusive) = parse_rational_range(range, line)?;
    let step = parse_rational_value(step, line)?;
    let value_type = if values.iter().all(|value| !value.contains(['/', '.'])) {
        ValueType::Int
    } else {
        ValueType::Rational
    };
    Ok((
        expand_rational_range(start, end, inclusive, step, line)?
            .into_iter()
            .map(Rational::format)
            .collect(),
        value_type,
    ))
}

fn normalize_tag_values(
    values: Vec<String>,
    value_type: ValueType,
    line: &str,
) -> Result<Vec<String>, DiagnosticReport> {
    let mut normalized = Vec::with_capacity(values.len());
    let mut seen = HashSet::new();
    for value in values {
        let value = match value_type {
            ValueType::Int | ValueType::Rational => parse_rational_value(&value, line)?.format(),
            ValueType::Frame3 => crate::frame3_literal::normalize_frame3_literal(&value)
                .map_err(|error| parse_error(line, &error))?,
            _ => value,
        };
        if !seen.insert(value.clone()) {
            return Err(parse_error(line, "tag domain contains a duplicate value"));
        }
        normalized.push(value);
    }
    Ok(normalized)
}

fn infer_tag_value_type(values: &[String], line: &str) -> Result<ValueType, DiagnosticReport> {
    if values
        .iter()
        .all(|value| matches!(value.as_str(), "up" | "down" | "left" | "right"))
    {
        return Ok(ValueType::Direction);
    }
    if values
        .iter()
        .all(|value| matches!(value.as_str(), "true" | "false"))
    {
        return Ok(ValueType::Bool);
    }
    let parsed_numbers = values
        .iter()
        .map(|value| parse_rational_value(value, line))
        .collect::<Result<Vec<_>, _>>();
    if parsed_numbers.is_ok() {
        return Ok(if values.iter().all(|value| !value.contains(['/', '.'])) {
            ValueType::Int
        } else {
            ValueType::Rational
        });
    }
    if values
        .iter()
        .all(|value| value.starts_with('"') && value.ends_with('"'))
    {
        return Ok(ValueType::String);
    }
    Ok(ValueType::Nominal)
}

fn parse_angle_domain_values(values: &[&str], line: &str) -> Result<Vec<String>, DiagnosticReport> {
    let body = values.join(" ");
    if body.contains("...") || body.contains("..<") {
        let (range, step) = split_axis_range_and_step(&body, line)?;
        let (start, end, inclusive) = parse_degree_range(range, line)?;
        let step = parse_degree_value(step, line)?;
        return expand_angle_range(start, end, inclusive, step, line);
    }
    values
        .iter()
        .map(|value| parse_degree_value(value, line).map(|value| format!("{}deg", value.format())))
        .collect()
}

fn parse_vec2_domain_values(values: &[&str], line: &str) -> Result<Vec<String>, DiagnosticReport> {
    let mut expanded = Vec::new();
    let mut seen = HashSet::new();
    for value in values {
        let inner = value
            .trim()
            .strip_prefix('(')
            .and_then(|value| value.strip_suffix(')'))
            .ok_or_else(|| parse_error(line, "vec2 domain item must be parenthesized"))?;
        let (x, y) = split_vec2_components(inner, line)?;
        let x_domain = parse_vec2_component_domain(x, line)?;
        let y_domain = parse_vec2_component_domain(y, line)?;
        for item in expand_vec2_domain(&x_domain, &y_domain) {
            if !seen.insert(item.clone()) {
                return Err(parse_error(line, "vec2 domain contains a duplicate value"));
            }
            expanded.push(item);
        }
    }
    Ok(expanded)
}

fn split_vec2_components<'a>(
    value: &'a str,
    line: &str,
) -> Result<(&'a str, &'a str), DiagnosticReport> {
    let mut depth = 0usize;
    let mut comma = None;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                if comma.replace(index).is_some() {
                    return Err(parse_error(
                        line,
                        "vec2 value must have exactly two components",
                    ));
                }
            }
            _ => {}
        }
    }
    let comma = comma.ok_or_else(|| parse_error(line, "vec2 value must have two components"))?;
    let x = value[..comma].trim();
    let y = value[comma + 1..].trim();
    if x.is_empty() || y.is_empty() {
        return Err(parse_error(line, "vec2 components must not be empty"));
    }
    Ok((x, y))
}

fn expand_vec2_domain(x_domain: &[Rational], y_domain: &[Rational]) -> Vec<String> {
    let mut values = Vec::new();
    for x in x_domain {
        for y in y_domain {
            values.push(format!("({},{})", x.format(), y.format()));
        }
    }
    values
}

fn parse_vec2_component_domain(body: &str, line: &str) -> Result<Vec<Rational>, DiagnosticReport> {
    if !body.contains("...") && !body.contains("..<") {
        return Ok(vec![parse_rational_value(body, line)?]);
    }
    let (range, step) = split_axis_range_and_step(body, line)?;
    let (start, end, inclusive) = parse_rational_range(range, line)?;
    let step = parse_rational_value(step, line)?;
    expand_rational_range(start, end, inclusive, step, line)
}

fn split_axis_range_and_step<'a>(
    body: &'a str,
    line: &str,
) -> Result<(&'a str, &'a str), DiagnosticReport> {
    let Some((range, step)) = body.split_once(" step ") else {
        return Err(parse_error(
            line,
            "axis range declaration must include step",
        ));
    };
    let range = range.trim();
    let step = step.trim();
    if range.is_empty() || step.is_empty() {
        return Err(parse_error(line, "axis range and step must not be empty"));
    }
    Ok((range, step))
}

fn parse_degree_range(
    value: &str,
    line: &str,
) -> Result<(Rational, Rational, bool), DiagnosticReport> {
    if let Some((start, end)) = value.split_once("..<") {
        return Ok((
            parse_degree_value(start.trim(), line)?,
            parse_degree_value(end.trim(), line)?,
            false,
        ));
    }
    let Some((start, end)) = value.split_once("...") else {
        return Err(parse_error(line, "angle range must use ... or ..<"));
    };
    Ok((
        parse_degree_value(start.trim(), line)?,
        parse_degree_value(end.trim(), line)?,
        true,
    ))
}

fn parse_rational_range(
    value: &str,
    line: &str,
) -> Result<(Rational, Rational, bool), DiagnosticReport> {
    if let Some((start, end)) = value.split_once("..<") {
        return Ok((
            parse_rational_value(start.trim(), line)?,
            parse_rational_value(end.trim(), line)?,
            false,
        ));
    }
    let Some((start, end)) = value.split_once("...") else {
        return Err(parse_error(
            line,
            "vec2 component range must use ... or ..<",
        ));
    };
    Ok((
        parse_rational_value(start.trim(), line)?,
        parse_rational_value(end.trim(), line)?,
        true,
    ))
}

fn expand_angle_range(
    start: Rational,
    end: Rational,
    inclusive: bool,
    step: Rational,
    line: &str,
) -> Result<Vec<String>, DiagnosticReport> {
    expand_rational_range(start, end, inclusive, step, line).map(|values| {
        values
            .into_iter()
            .map(|value| format!("{}deg", value.format()))
            .collect()
    })
}

fn expand_rational_range(
    start: Rational,
    end: Rational,
    inclusive: bool,
    step: Rational,
    line: &str,
) -> Result<Vec<Rational>, DiagnosticReport> {
    if step.is_zero() {
        return Err(parse_error(line, "axis step must not be zero"));
    }
    if step.cmp(Rational::ZERO) == std::cmp::Ordering::Less {
        return Err(parse_error(line, "axis step must be positive"));
    }
    if start.cmp(end) == std::cmp::Ordering::Greater {
        return Err(parse_error(
            line,
            "axis range start must be less than or equal to end",
        ));
    }
    let mut values = Vec::new();
    let mut current = start;
    let mut guard = 0usize;
    loop {
        let order = current.cmp(end);
        if order == std::cmp::Ordering::Greater
            || (!inclusive && order == std::cmp::Ordering::Equal)
        {
            break;
        }
        values.push(current);
        current = current.add(step);
        guard += 1;
        if guard > 10_000 {
            return Err(parse_error(line, "axis range produced too many values"));
        }
    }
    if inclusive {
        if values.last().copied() != Some(end) {
            return Err(parse_error(
                line,
                "axis step must land exactly on inclusive range end",
            ));
        }
    } else if current != end {
        return Err(parse_error(
            line,
            "axis step must land exactly on exclusive range end",
        ));
    }
    Ok(values)
}

fn parse_degree_value(value: &str, line: &str) -> Result<Rational, DiagnosticReport> {
    let value = value
        .trim()
        .strip_suffix("deg")
        .ok_or_else(|| parse_error(line, "angle values must use deg"))?;
    parse_rational_value(value.trim(), line)
}

fn parse_rational_value(value: &str, line: &str) -> Result<Rational, DiagnosticReport> {
    let value = value.trim();
    if value.is_empty() {
        return Err(parse_error(line, "rational value must not be empty"));
    }
    let value = value.strip_prefix('+').unwrap_or(value);
    if let Some((numerator, denominator)) = value.split_once('/') {
        let numerator = parse_decimal_integer(numerator.trim(), line)?;
        let denominator = parse_decimal_integer(denominator.trim(), line)?;
        return Rational::new(numerator, denominator)
            .ok_or_else(|| parse_error(line, "rational denominator must not be zero"));
    }
    if let Some((integer, fractional)) = value.split_once('.') {
        if fractional.is_empty() || !fractional.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(parse_error(line, "decimal rational value is malformed"));
        }
        let negative = integer.starts_with('-');
        let integer_abs = integer.strip_prefix('-').unwrap_or(integer);
        if integer_abs.is_empty() || !integer_abs.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(parse_error(line, "decimal rational value is malformed"));
        }
        let denominator = 10_i64.pow(fractional.len() as u32);
        let numerator = integer_abs
            .parse::<i64>()
            .map_err(|_| parse_error(line, "decimal rational value is malformed"))?
            * denominator
            + fractional
                .parse::<i64>()
                .map_err(|_| parse_error(line, "decimal rational value is malformed"))?;
        return Rational::new(if negative { -numerator } else { numerator }, denominator)
            .ok_or_else(|| parse_error(line, "decimal rational value is malformed"));
    }
    parse_decimal_integer(value, line).map(Rational::integer)
}

fn parse_decimal_integer(value: &str, line: &str) -> Result<i64, DiagnosticReport> {
    if value.is_empty() || value == "+" || value == "-" {
        return Err(parse_error(line, "integer value is malformed"));
    }
    value
        .parse::<i64>()
        .map_err(|_| parse_error(line, "integer value is malformed"))
}

fn parse_assignment_directive(
    name: &str,
    line: &str,
    catalog: &mut Catalog,
    named_conditions: &mut HashMap<String, (String, ConditionAst)>,
) -> Result<(), DiagnosticReport> {
    if !is_identifier(name) {
        return Err(parse_error(line, "assignment name must be an identifier"));
    }
    let (parsed_name, expr) = require_assignment_row(line, "assignment must be: <name> = <value>")?;
    if parsed_name != name {
        return Err(parse_error(
            line,
            "assignment name does not match directive",
        ));
    }
    if looks_like_condition_expr(expr) {
        if named_conditions.contains_key(name) {
            return Err(parse_error(line, "duplicate condition"));
        }
        let condition = parse_condition_expr(
            expr,
            line,
            &catalog.input_names,
            &catalog.variable_names,
            &catalog.condition_names,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
        )?;
        named_conditions.insert(name.to_string(), (expr.to_string(), condition));
        return Ok(());
    }

    Err(parse_error(
        line,
        "tag sets must be declared inside `tags { ... }`",
    ))
}

fn catalog_value_sets(catalog: &Catalog) -> HashMap<String, Vec<String>> {
    let mut values = catalog.value_sets.clone();
    for (name, value_set_values) in &catalog.object_axes {
        values.insert(name.clone(), value_set_values.clone());
    }
    values
}

fn catalog_value_set<'a>(catalog: &'a Catalog, name: &str) -> Option<&'a Vec<String>> {
    catalog
        .value_sets
        .get(name)
        .or_else(|| catalog.object_axes.get(name))
}

fn is_builtin_value_set(name: &str) -> bool {
    matches!(name, "directions" | "horizontal" | "vertical" | "slots")
}

fn looks_like_condition_expr(expr: &str) -> bool {
    expr.contains('(')
        || expr.contains("==")
        || expr.contains("!=")
        || expr.contains("<=")
        || expr.contains(">=")
        || expr.contains('<')
        || expr.contains('>')
        || expr
            .split_whitespace()
            .any(|token| matches!(token, "and" | "or"))
}

fn is_at_identifier_token(token: &str) -> bool {
    puzzle_authoring::is_at_identifier_token(token)
}

fn validate_selector_alias_name(
    value: &str,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    if is_at_identifier_token(value) || is_qualified_identifier(value) {
        Ok(())
    } else {
        Err(parse_error(
            line,
            &format!("{label} must be a qualified identifier or @name"),
        ))
    }
}

fn validate_rule_name(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    if is_at_identifier_token(value) || is_qualified_identifier(value) {
        Ok(())
    } else {
        Err(parse_error(
            line,
            "routine name must be a qualified identifier or @name",
        ))
    }
}

fn parse_layer_term(
    term: &str,
    line: &str,
    layer: u16,
    catalog: &mut Catalog,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    let declared = if is_known_object_selector(
        term,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.object_groups,
    ) {
        let selector = resolve_object_selector(
            term,
            line,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
            &HashMap::new(),
        )?;
        for object in &selector.alternatives {
            assign_object_layer(*object, layer, catalog);
        }
        selector.alternatives
    } else {
        let value_sets = catalog_value_sets(catalog);
        let axis_types = catalog.axis_types.clone();
        define_object_spec(
            term,
            layer,
            None,
            line,
            &value_sets,
            &axis_types,
            &mut catalog.object_schemas,
            &mut catalog.object_names,
            &mut catalog.object_labels,
            &mut catalog.object_layers,
            &mut catalog.object_defs,
            &mut catalog.render_chars,
            &mut catalog.char_objects,
        )?
    };
    Ok(declared)
}

fn push_terms(objects: &mut Vec<ObjectId>, terms: &[ObjectId]) {
    for object in terms {
        push_unique_object(objects, *object);
    }
}

fn parse_mark_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [name, "=", ty] => {
                parse_mark_directive(name, Some(*ty), line, catalog)?;
                i += 1;
            }
            [spec] => {
                let (name, ty) =
                    parse_assignment_row(spec).map_or((*spec, None), |(name, ty)| (name, Some(ty)));
                parse_mark_directive(name, ty, line, catalog)?;
                i += 1;
            }
            [] => i += 1,
            _ => {
                return Err(parse_error(
                    line,
                    "mark row must be: <name> or <name> = <type>",
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "mark missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_mark_directive(
    name: &str,
    ty: Option<&str>,
    line: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    let (name, kind, values) = if let Some(ty) = ty {
        validate_mark_name(name, line)?;
        if ty.is_empty() {
            return Err(parse_error(line, "mark type must not be empty"));
        }
        match ty {
            "int" => (name, MarkKind::Int, Vec::new()),
            "bool" => (name, MarkKind::Bool, Vec::new()),
            "flag" => (name, MarkKind::Flag, Vec::new()),
            axis if catalog.value_sets.contains_key(axis)
                || catalog.object_axes.contains_key(axis) =>
            {
                (
                    name,
                    MarkKind::Enum,
                    catalog
                        .value_sets
                        .get(axis)
                        .or_else(|| catalog.object_axes.get(axis))
                        .cloned()
                        .unwrap_or_default(),
                )
            }
            _ => return Err(parse_error(line, "unknown mark type")),
        }
    } else {
        validate_mark_name(name, line)?;
        (name, MarkKind::Flag, Vec::new())
    };
    if catalog.mark_names.contains_key(name) {
        return Err(parse_error(line, "duplicate mark"));
    }
    let id = MarkId(catalog.mark_defs.len() as u16);
    let def = MarkDef { id, kind, values };
    catalog.mark_defs.push(def.clone());
    catalog.mark_names.insert(name.to_string(), def);
    Ok(())
}

fn parse_layers_block(
    lines: &[String],
    start: usize,
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
    catalog: &mut Catalog,
    pending_groups: &[PendingGroupDefinition],
    resolved_groups: &mut HashSet<String>,
) -> Result<usize, DiagnosticReport> {
    let used_groups = predeclare_layer_block_objects(lines, start, pending_groups, catalog)?;
    resolve_pending_group_definitions(
        pending_groups,
        Some(&used_groups),
        resolved_groups,
        catalog,
    )?;

    let mut i = start;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        if tokens.is_empty() {
            i += 1;
            continue;
        }
        match tokens.as_slice() {
            ["for", binding, "in", sources @ ..] => {
                let value_sets = catalog_value_sets(catalog);
                let values = for_expansion_values(
                    sources,
                    &value_sets,
                    &catalog.numeric_variable_defaults,
                    &lines[i],
                )?;
                validate_identifier(binding, &lines[i], "expansion binding")?;
                let (body_lines, next_i) = collect_statement_block_lines(lines, i + 1, &lines[i])?;
                for value in &values {
                    let mut expanded_lines =
                        expand_for_binding_lines(&body_lines, binding, value, &catalog.maps)?;
                    expanded_lines.push(BLOCK_CLOSE.to_string());
                    let parsed_i = parse_layers_block(
                        &expanded_lines,
                        0,
                        named_layers,
                        layer_count,
                        catalog,
                        pending_groups,
                        resolved_groups,
                    )?;
                    if parsed_i != expanded_lines.len() {
                        return Err(parse_error(&lines[i], "for expansion failed"));
                    }
                }
                i = next_i;
                continue;
            }
            ["for", ..] => {
                return Err(parse_error(
                    &lines[i],
                    "for directive must be: for <binding> in <source...>",
                ));
            }
            _ => match puzzle_authoring::slot_row_surface(&lines[i]) {
                Some(puzzle_authoring::SlotRowSurface::Each { selectors }) => {
                    assign_selectors_to_separate_layers(
                        &selectors,
                        &lines[i],
                        named_layers,
                        layer_count,
                        catalog,
                    )?;
                }
                Some(puzzle_authoring::SlotRowSurface::Named(assignment)) => {
                    let layer = layer_id_for_name(
                        assignment.name,
                        &lines[i],
                        named_layers,
                        layer_count,
                        catalog,
                    )?;
                    define_or_assign_terms_to_layer(
                        &assignment.selectors,
                        &lines[i],
                        layer,
                        catalog,
                    )?;
                    register_layer_tag_from_layer(assignment.name, layer, catalog);
                }
                Some(puzzle_authoring::SlotRowSurface::Anonymous { selectors }) => {
                    assign_selectors_to_anonymous_layer(
                        &selectors,
                        &lines[i],
                        named_layers,
                        layer_count,
                        catalog,
                    )?;
                }
                None => return Err(parse_error(&lines[i], "invalid layer row")),
            },
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start - 1],
            "slots missing closing brace",
        ));
    }
    Ok(i + 1)
}

fn predeclare_layer_block_objects(
    lines: &[String],
    start: usize,
    pending_groups: &[PendingGroupDefinition],
    catalog: &mut Catalog,
) -> Result<Vec<String>, DiagnosticReport> {
    let mut terms = Vec::<String>::new();
    let mut used_groups = Vec::<String>::new();
    collect_layer_block_terms(
        lines,
        start,
        pending_groups,
        catalog,
        &mut terms,
        &mut used_groups,
    )?;
    predeclare_layer_terms(&terms, catalog)?;
    Ok(used_groups)
}

fn collect_layer_block_terms(
    lines: &[String],
    start: usize,
    pending_groups: &[PendingGroupDefinition],
    catalog: &Catalog,
    terms: &mut Vec<String>,
    used_groups: &mut Vec<String>,
) -> Result<usize, DiagnosticReport> {
    let mut i = start;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        if tokens.is_empty() {
            i += 1;
            continue;
        }
        match tokens.as_slice() {
            ["for", binding, "in", sources @ ..] => {
                let value_sets = catalog_value_sets(catalog);
                let values = for_expansion_values(
                    sources,
                    &value_sets,
                    &catalog.numeric_variable_defaults,
                    &lines[i],
                )?;
                validate_identifier(binding, &lines[i], "expansion binding")?;
                let (body_lines, next_i) = collect_statement_block_lines(lines, i + 1, &lines[i])?;
                for value in &values {
                    let mut expanded_lines =
                        expand_for_binding_lines(&body_lines, binding, value, &catalog.maps)?;
                    expanded_lines.push(BLOCK_CLOSE.to_string());
                    let parsed_i = collect_layer_block_terms(
                        &expanded_lines,
                        0,
                        pending_groups,
                        catalog,
                        terms,
                        used_groups,
                    )?;
                    if parsed_i != expanded_lines.len() {
                        return Err(parse_error(&lines[i], "for expansion failed"));
                    }
                }
                i = next_i;
                continue;
            }
            ["for", ..] => {
                return Err(parse_error(
                    &lines[i],
                    "for directive must be: for <binding> in <source...>",
                ));
            }
            _ => {
                let selectors = match puzzle_authoring::slot_row_surface(&lines[i]) {
                    Some(puzzle_authoring::SlotRowSurface::Each { selectors })
                    | Some(puzzle_authoring::SlotRowSurface::Anonymous { selectors }) => selectors,
                    Some(puzzle_authoring::SlotRowSurface::Named(assignment)) => {
                        assignment.selectors
                    }
                    None => return Err(parse_error(&lines[i], "invalid layer row")),
                };
                collect_layer_terms(&selectors, &lines[i], pending_groups, terms, used_groups)?;
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start - 1],
            "slots missing closing brace",
        ));
    }
    Ok(i + 1)
}

fn collect_layer_terms(
    selectors: &[&str],
    _line: &str,
    pending_groups: &[PendingGroupDefinition],
    terms: &mut Vec<String>,
    used_groups: &mut Vec<String>,
) -> Result<(), DiagnosticReport> {
    let expanded = puzzle_authoring::expand_layer_selectors(selectors, pending_groups)
        .map_err(|error| parse_error(&error.source_line, error.message))?;
    for term in expanded.terms {
        terms.push(term);
    }
    for group in expanded.used_groups {
        if !used_groups.contains(&group) {
            used_groups.push(group);
        }
    }
    Ok(())
}

fn predeclare_layer_terms(terms: &[String], catalog: &mut Catalog) -> Result<(), DiagnosticReport> {
    for term in terms {
        predeclare_layer_schema_or_plain_object(term, catalog)?;
    }
    for term in terms {
        predeclare_layer_selector_variants(term, catalog)?;
    }
    Ok(())
}

fn predeclare_layer_schema_or_plain_object(
    term: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    if catalog.object_names.contains_key(term) || catalog.object_groups.contains_key(term) {
        return Ok(());
    }
    let parts = term.split(':').collect::<Vec<_>>();
    if parts.len() == 1 {
        define_layer_object_spec(term, term, catalog)?;
        return Ok(());
    }
    let base = parts[0];
    if catalog.object_schemas.contains_key(base) {
        return Ok(());
    }
    let value_sets = catalog_value_sets(catalog);
    if parts[1..].iter().all(|axis| value_sets.contains_key(*axis)) {
        define_layer_object_spec(term, term, catalog)?;
    }
    Ok(())
}

fn predeclare_layer_selector_variants(
    term: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    let parts = term.split(':').collect::<Vec<_>>();
    if parts.len() <= 1 {
        return Ok(());
    }
    let Some(schema) = catalog.object_schemas.get(parts[0]).cloned() else {
        define_layer_object_spec(term, term, catalog)?;
        return Ok(());
    };
    let value_combinations = layer_selector_variant_values(&parts, &schema, catalog, term)?;
    for values in value_combinations {
        if schema
            .variants
            .iter()
            .any(|variant| variant.values == values)
            || catalog.object_schemas.get(parts[0]).is_some_and(|schema| {
                schema
                    .variants
                    .iter()
                    .any(|variant| variant.values == values)
            })
        {
            continue;
        }
        for (axis, value) in schema.axes.iter().zip(&values) {
            let axis_values = catalog.object_axes.entry(axis.clone()).or_default();
            if !axis_values.contains(value) {
                axis_values.push(value.clone());
            }
        }
        let name = format!("{}:{}", parts[0], values.join(":"));
        if catalog.object_names.contains_key(&name) {
            return Err(parse_error(
                term,
                "object variant name conflicts with existing object",
            ));
        }
        let object = add_object_variant(
            &name,
            UNASSIGNED_LAYER,
            &mut catalog.object_names,
            &mut catalog.object_labels,
            &mut catalog.object_layers,
            &mut catalog.object_defs,
        );
        catalog
            .object_schemas
            .get_mut(parts[0])
            .expect("schema exists while adding layer variant")
            .variants
            .push(ObjectVariant { values, object });
    }
    Ok(())
}

fn define_layer_object_spec(
    spec: &str,
    line: &str,
    catalog: &mut Catalog,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    let value_sets = catalog_value_sets(catalog);
    let axis_types = catalog.axis_types.clone();
    define_object_spec(
        spec,
        UNASSIGNED_LAYER,
        None,
        line,
        &value_sets,
        &axis_types,
        &mut catalog.object_schemas,
        &mut catalog.object_names,
        &mut catalog.object_labels,
        &mut catalog.object_layers,
        &mut catalog.object_defs,
        &mut catalog.render_chars,
        &mut catalog.char_objects,
    )
}

fn layer_selector_variant_values(
    parts: &[&str],
    schema: &ObjectSchema,
    catalog: &Catalog,
    line: &str,
) -> Result<Vec<Vec<String>>, DiagnosticReport> {
    validate_schema_selector_arity(parts, schema, line, "object selector")?;
    let value_sets = catalog_value_sets(catalog);
    let mut combinations = vec![Vec::<String>::new()];
    for (axis_index, axis) in schema.axes.iter().enumerate() {
        let Some(value) = schema_selector_part(parts, schema, axis_index) else {
            return Err(parse_error(
                line,
                "object selector must name every variant slot; use * for unconstrained slots",
            ));
        };
        let values = if value == "*" {
            schema_axis_values(schema, axis_index)?
        } else if let Some(values) = value_sets.get(value) {
            values
                .iter()
                .map(|value| normalize_axis_literal(value, schema, axis_index, line))
                .collect::<Result<Vec<_>, _>>()?
        } else if value == axis {
            schema_axis_values(schema, axis_index)?
        } else if parse_value_expr(value, line)
            .is_ok_and(|expr| matches!(expr, ValueExpr::MapCall { .. }))
        {
            schema_axis_values(schema, axis_index)?
        } else {
            vec![normalize_axis_literal(value, schema, axis_index, line)?]
        };
        let mut next = Vec::new();
        for prefix in &combinations {
            for value in &values {
                let mut expanded = prefix.clone();
                expanded.push(value.clone());
                next.push(expanded);
            }
        }
        combinations = next;
    }
    Ok(combinations)
}

fn define_or_assign_terms_to_layer(
    terms: &[&str],
    line: &str,
    layer: u16,
    catalog: &mut Catalog,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    if terms.is_empty() {
        return Err(parse_error(
            line,
            "layer declaration must name at least one object",
        ));
    }

    let mut objects = Vec::new();
    let mut i = 0;
    while i < terms.len() {
        let term = terms[i];
        let declared = parse_layer_term(term, line, layer, catalog)?;
        push_terms(&mut objects, &declared);
        i += 1;
    }
    Ok(objects)
}

fn is_known_object_selector(
    selector: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> bool {
    let base = selector
        .split_once('{')
        .map_or(selector, |(base, _)| base)
        .split_once(':')
        .map_or(selector, |(base, _)| base);
    object_names.contains_key(selector)
        || object_groups.contains_key(selector)
        || (selector.contains(':') && object_schemas.contains_key(base))
        || (selector.contains(':') && value_sets.contains_key(base))
        || (base == "*" && selector.contains(':') && !object_schemas.is_empty())
}

fn assign_selectors_to_separate_layers(
    selectors: &[&str],
    line: &str,
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
    catalog: &mut Catalog,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    if selectors.is_empty() {
        return Err(parse_error(
            line,
            "each layer row must name at least one selector",
        ));
    }
    let selector_sets = selectors
        .iter()
        .map(|selector| resolve_or_declare_layer_selector(selector, line, catalog))
        .collect::<Result<Vec<_>, _>>()?;
    let mut objects = Vec::new();
    for selector_set in selector_sets {
        for object in selector_set {
            let layer = anonymous_layer_id(named_layers, layer_count);
            assign_object_layer(object, layer, catalog);
            push_unique_object(&mut objects, object);
        }
    }
    Ok(objects)
}

fn resolve_or_declare_layer_selector(
    selector: &str,
    line: &str,
    catalog: &mut Catalog,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    let declared = if is_known_object_selector(
        selector,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.object_groups,
    ) {
        resolve_object_selector(
            selector,
            line,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
            &HashMap::new(),
        )?
        .alternatives
    } else {
        let value_sets = catalog_value_sets(catalog);
        let axis_types = catalog.axis_types.clone();
        define_object_spec(
            selector,
            UNASSIGNED_LAYER,
            None,
            line,
            &value_sets,
            &axis_types,
            &mut catalog.object_schemas,
            &mut catalog.object_names,
            &mut catalog.object_labels,
            &mut catalog.object_layers,
            &mut catalog.object_defs,
            &mut catalog.render_chars,
            &mut catalog.char_objects,
        )?
    };
    Ok(declared)
}

fn assign_selectors_to_anonymous_layer(
    selectors: &[&str],
    line: &str,
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
    catalog: &mut Catalog,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    let layer = anonymous_layer_id(named_layers, layer_count);
    define_or_assign_terms_to_layer(selectors, line, layer, catalog)
}

fn push_unique_object(objects: &mut Vec<ObjectId>, object: ObjectId) {
    if !objects.contains(&object) {
        objects.push(object);
    }
}

fn assign_object_layer(object: ObjectId, layer: u16, catalog: &mut Catalog) {
    let layer = LayerId(layer);
    catalog.object_layers.insert(object, layer);
    if let Some(definition) = catalog
        .object_defs
        .iter_mut()
        .find(|definition| definition.id == object)
    {
        definition.layer_id = layer;
    }
}

fn register_layer_tag_from_layer(name: &str, layer: u16, catalog: &mut Catalog) {
    let layer = LayerId(layer);
    let objects = catalog
        .object_defs
        .iter()
        .filter_map(|definition| (definition.layer_id == layer).then_some(definition.id))
        .collect::<Vec<_>>();
    catalog.object_groups.insert(name.to_string(), objects);
}

fn refresh_layer_tags_and_value_sets(named_layers: &HashMap<String, u16>, catalog: &mut Catalog) {
    let mut layer_ids = catalog
        .object_defs
        .iter()
        .filter(|definition| definition.layer_id.0 != UNASSIGNED_LAYER)
        .map(|definition| definition.layer_id.0)
        .collect::<Vec<_>>();
    layer_ids.sort_unstable();
    layer_ids.dedup();

    let mut layer_names = layer_ids
        .into_iter()
        .map(|layer| {
            let name = named_layers
                .iter()
                .find_map(|(name, named_layer)| (*named_layer == layer).then_some(name.clone()))
                .unwrap_or_else(|| internal_layer_group_name(layer));
            (layer, name)
        })
        .collect::<Vec<_>>();
    layer_names.sort_by(|(left_layer, left_name), (right_layer, right_name)| {
        left_layer
            .cmp(right_layer)
            .then_with(|| left_name.cmp(right_name))
    });

    let values = layer_names
        .iter()
        .map(|(_, name)| name.clone())
        .collect::<Vec<_>>();
    for (layer, name) in layer_names {
        register_layer_tag_from_layer(&name, layer, catalog);
    }
    catalog
        .value_sets
        .insert("slots".to_string(), values.clone());
}

fn internal_layer_group_name(layer: u16) -> String {
    format!("__anonymous_layer_{layer}")
}

fn anonymous_layer_id(
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
) -> u16 {
    let layer = named_layers.len() as u16;
    named_layers.insert(internal_layer_group_name(layer), layer);
    *layer_count = Some(layer.saturating_add(1));
    layer
}

fn layer_id_for_name(
    name: &str,
    line: &str,
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
    catalog: &Catalog,
) -> Result<u16, DiagnosticReport> {
    validate_selector_alias_name(name, line, "layer name")?;
    if let Some(layer) = named_layers.get(name).copied() {
        return Ok(layer);
    }
    if selector_name_conflicts(name, catalog) {
        return Err(parse_error(
            line,
            "layer name must not shadow another selector",
        ));
    }

    let layer = named_layers.len() as u16;
    named_layers.insert(name.to_string(), layer);
    *layer_count = Some(layer.saturating_add(1));
    Ok(layer)
}

fn selector_name_conflicts(name: &str, catalog: &Catalog) -> bool {
    selector_name_conflicts_with(
        name,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog.object_groups,
    )
}

fn selector_name_conflicts_with(
    name: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> bool {
    let conflicts = |candidate: &str| {
        puzzle_authoring::selector_alias_conflicts(
            candidate,
            object_names.keys().map(String::as_str),
            object_schemas.keys().map(String::as_str),
            object_groups.keys().map(String::as_str),
        )
    };
    conflicts(name)
        || name
            .split_once(':')
            .is_some_and(|(base, _)| conflicts(base))
}

fn parse_legend_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
    render_overlays: &mut OverlayDefs,
    empty_char: &mut Option<char>,
) -> Result<usize, DiagnosticReport> {
    let block = puzzle_authoring::collect_row_block_surface(lines, start + 1, "legend")
        .map_err(|error| parse_error(&lines[start], error.message()))?;
    for line in block.rows {
        parse_legend_block_row(line, catalog, render_overlays, empty_char)?;
    }
    Ok(block.next_index)
}

fn parse_legend_block_row(
    line: &str,
    catalog: &mut Catalog,
    render_overlays: &mut OverlayDefs,
    empty_char: &mut Option<char>,
) -> Result<(), DiagnosticReport> {
    let Some(assignment) = puzzle_authoring::selector_assignment_surface(line) else {
        return Err(parse_error(
            line,
            "legend row must be: <char> = <empty | selector...>",
        ));
    };

    let ch = parse_char(Some(&assignment.name), line, "missing legend char")?;
    if assignment.selectors == ["empty"] {
        if ch != '.' {
            return Err(parse_error(
                line,
                "levels use `.` for empty; remove the non-dot empty legend row",
            ));
        }
        *empty_char = Some(ch);
        return Ok(());
    }
    if ch == '.' {
        return Err(parse_error(
            line,
            "levels reserve `.` for empty; use another legend char for objects",
        ));
    }

    let mut directive_tokens = vec!["legend", assignment.name, "="];
    directive_tokens.extend(assignment.selectors);
    parse_legend_directive(
        &directive_tokens,
        line,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.maps,
        &catalog.object_groups,
        &mut catalog.render_chars,
        &mut catalog.char_objects,
        render_overlays,
    )
}

fn add_input_name(
    name: &str,
    line: &str,
    catalog: &mut Catalog,
) -> Result<InputId, DiagnosticReport> {
    if !is_identifier(name) {
        return Err(parse_error(line, "input name must be an identifier"));
    }
    if catalog.input_names.contains_key(name) {
        return Err(parse_error(line, "duplicate input"));
    }

    let id = InputId((catalog.input_names.len() + 1) as u16);
    catalog.input_names.insert(name.to_string(), id);
    catalog.input_labels.insert(id, name.to_string());
    Ok(id)
}

fn add_implicit_input_guards_to_catalog(
    definitions: &[RuleDefinitionAst],
    main_statements: Option<&[StatementAst]>,
    level_start_statements: Option<&[StatementAst]>,
    level_clear_statements: Option<&[StatementAst]>,
    level_bodies: &[PreparedLevelBody],
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    let mut names = BTreeSet::<String>::new();
    for definition in definitions {
        collect_implicit_inputs_from_statements(&definition.statements, &mut names);
    }
    for statements in [
        main_statements,
        level_start_statements,
        level_clear_statements,
    ]
    .into_iter()
    .flatten()
    {
        collect_implicit_inputs_from_statements(statements, &mut names);
    }
    for level in level_bodies {
        collect_implicit_inputs_from_statements(&level.rules_before_statements, &mut names);
        collect_implicit_inputs_from_statements(&level.rules_after_statements, &mut names);
        collect_implicit_inputs_from_statements(&level.level_start_statements, &mut names);
        collect_implicit_inputs_from_statements(&level.level_clear_statements, &mut names);
    }
    for (_, condition) in named_conditions.values() {
        collect_implicit_inputs_from_condition(condition, &mut names);
    }
    for name in names {
        if !catalog.input_names.contains_key(&name) {
            add_input_name(&name, "input guard", catalog)?;
        }
    }
    Ok(())
}

fn collect_implicit_inputs_from_statements(
    statements: &[StatementAst],
    names: &mut BTreeSet<String>,
) {
    for statement in statements {
        match statement {
            StatementAst::LocalRoutine { definition, .. } => {
                collect_implicit_inputs_from_statements(&definition.statements, names);
            }
            StatementAst::Block { statements, .. } | StatementAst::Fix { statements, .. } => {
                collect_implicit_inputs_from_statements(statements, names);
            }
            StatementAst::RepeatUntil {
                condition,
                statements,
                ..
            } => {
                collect_implicit_inputs_from_condition(condition, names);
                collect_implicit_inputs_from_statements(statements, names);
            }
            StatementAst::If {
                condition,
                then_statements,
                else_statements,
                ..
            } => {
                collect_implicit_inputs_from_condition(condition, names);
                collect_implicit_inputs_from_statements(then_statements, names);
                collect_implicit_inputs_from_statements(else_statements, names);
            }
            StatementAst::Conditional {
                then_statements,
                else_statements,
                ..
            } => {
                collect_implicit_inputs_from_statements(then_statements, names);
                collect_implicit_inputs_from_statements(else_statements, names);
            }
            StatementAst::Call { .. } | StatementAst::Effect { .. } | StatementAst::Rewrite(_) => {}
        }
    }
}

fn collect_implicit_inputs_from_condition(condition: &ConditionAst, names: &mut BTreeSet<String>) {
    match condition {
        ConditionAst::All(conditions) | ConditionAst::Any(conditions) => {
            for condition in conditions {
                collect_implicit_inputs_from_condition(condition, names);
            }
        }
        ConditionAst::InputIs(name) => {
            names.insert(name.clone());
        }
        ConditionAst::InputIn(_)
        | ConditionAst::AllObjectsOn { .. }
        | ConditionAst::VariableEquals { .. }
        | ConditionAst::VariableCompare { .. }
        | ConditionAst::ConditionEquals { .. }
        | ConditionAst::ConditionNonZero(_)
        | ConditionAst::ConditionCompare { .. }
        | ConditionAst::InlineConditionValueEquals { .. }
        | ConditionAst::InlineConditionNonZero(_)
        | ConditionAst::InlineConditionCompare { .. } => {}
    }
}

fn add_default_restart_handler(main_statements: Option<&mut Vec<StatementAst>>) {
    let Some(statements) = main_statements else {
        return;
    };
    let mut inputs = BTreeSet::new();
    collect_implicit_inputs_from_statements(statements, &mut inputs);
    if inputs.contains("restart") {
        return;
    }
    statements.push(StatementAst::If {
        source_line: "restart".to_string(),
        source_line_number: None,
        condition: ConditionAst::InputIs("restart".to_string()),
        then_statements: vec![StatementAst::Effect {
            source_line: "restart".to_string(),
            source_line_number: None,
            effects: vec![EffectAst::Restart],
        }],
        else_statements: Vec::new(),
    });
}

fn add_cardinal_directions(
    line: &str,
    catalog: &mut Catalog,
    directions: &mut Vec<Direction>,
) -> Result<(), DiagnosticReport> {
    for (name, dx, dy) in [
        ("up", 0, -1),
        ("down", 0, 1),
        ("left", -1, 0),
        ("right", 1, 0),
    ] {
        let input = catalog
            .input_names
            .get(name)
            .copied()
            .map(Ok)
            .unwrap_or_else(|| add_input_name(name, line, catalog))?;
        if !directions.iter().any(|direction| direction.input == input) {
            directions.push(Direction { input, dx, dy });
        }
    }
    Ok(())
}

fn add_default_non_direction_inputs(
    line: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    for name in ["restart"] {
        if !catalog.input_names.contains_key(name) {
            add_input_name(name, line, catalog)?;
        }
    }
    Ok(())
}

fn has_cardinal_input_names(input_names: &HashMap<String, InputId>) -> bool {
    ["up", "down", "left", "right"]
        .iter()
        .any(|name| input_names.contains_key(*name))
}

fn directions_include_all_cardinals(
    directions: &[Direction],
    input_names: &HashMap<String, InputId>,
) -> bool {
    ["up", "down", "left", "right"].iter().all(|name| {
        input_names
            .get(*name)
            .is_some_and(|input| directions.iter().any(|direction| direction.input == *input))
    })
}

fn parse_command_definition(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
) -> Result<(Option<Direction>, usize), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let keyword = header.first().copied().unwrap_or("input");
    let name = expect(header.get(1), &lines[start], "missing input name")?;
    let input = if let Some(input) = catalog.input_names.get(name).copied() {
        input
    } else {
        add_input_name(name, &lines[start], catalog)?
    };

    match header.as_slice() {
        ["input", _] => {
            let next = start + 1;
            if next >= lines.len() || is_block_close_line(&lines[next]) {
                return Ok((None, next));
            }
            if !is_input_option(&split_header_tokens(&lines[next])) {
                return Ok((None, next));
            }

            let mut direction = None;
            let mut i = next;
            while i < lines.len() && !is_block_close_line(&lines[i]) {
                direction = Some(parse_input_option(&lines[i], input)?);
                i += 1;
            }
            if i >= lines.len() {
                return Err(parse_error(&lines[start], "input missing closing brace"));
            }
            Ok((direction, i + 1))
        }
        ["input", _, "direction", value] => {
            let (dx, dy) = named_direction_vector(value, &lines[start])?;
            Ok((Some(Direction { input, dx, dy }), start + 1))
        }
        _ => Err(parse_error(
            &lines[start],
            &format!("{keyword} must be: input <name> [direction <up|down|left|right>]"),
        )),
    }
}

fn is_input_option(tokens: &[&str]) -> bool {
    matches!(tokens, ["direction", ..])
}

fn parse_input_option(line: &str, input: InputId) -> Result<Direction, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        ["direction", value] => {
            let (dx, dy) = named_direction_vector(value, line)?;
            Ok(Direction { input, dx, dy })
        }
        _ => Err(parse_error(
            line,
            "input option must be: direction <up|down|left|right>",
        )),
    }
}
