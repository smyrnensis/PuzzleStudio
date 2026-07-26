fn parse_map_definition(
    lines: &[source::LogicalLine],
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
    for name in puzzle_authoring::ABSOLUTE_DIRECTION_SET_NAMES {
        let Some(values) = puzzle_authoring::movement_mark_set_values(name, 2) else {
            continue;
        };
        symbols.value_set_names.insert(name.to_string());
        symbols.direction_sets.insert(name.to_string());
        let values = values
            .iter()
            .copied()
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

fn parser_catalog_completion_symbols(
    catalog: &Catalog,
) -> crate::surface::SurfaceCompletionSymbols {
    let mut symbols = crate::surface::SurfaceCompletionSymbols::default();
    symbols
        .objects
        .extend(catalog.object_schemas.keys().cloned());
    symbols.objects.extend(
        catalog
            .object_names
            .keys()
            .filter(|name| !name.contains(':') && !catalog.object_groups.contains_key(*name))
            .cloned(),
    );
    symbols.groups.extend(
        catalog
            .object_groups
            .keys()
            .filter(|name| !catalog.named_layers.contains_key(*name))
            .cloned(),
    );
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
    for (name, values) in &catalog.object_axes {
        symbols.value_set_names.insert(name.clone());
        symbols
            .value_sets
            .entry(name.clone())
            .or_insert(values.clone());
        symbols.object_name_atoms.extend(values.iter().cloned());
    }
    for (object, schema) in &catalog.object_schemas {
        symbols
            .object_axes
            .entry(object.clone())
            .or_insert(schema.axes.clone());
        symbols
            .object_name_atoms
            .extend(schema.axes.iter().cloned());
    }
    for name in catalog.mark_names.keys() {
        if !name.starts_with("__") {
            symbols.markes.insert(name.clone());
        }
    }
    symbols
}

fn surface_completion_direction_value(value: &str) -> bool {
    puzzle_authoring::MOVEMENT_DIRECTIONS_3D.contains(&value)
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
) -> crate::surface::ParseProduct<Result<Catalog, DiagnosticReport>> {
    let mut recognition = crate::surface::ParserRecognition::default();
    for entry in &model.catalog_entries {
        recognition.merge(entry.semantics.fixed.clone());
    }
    let value = (|| {
        let mut catalog = Catalog::for_dimension(model.dimension);
        let mut named_layers = HashMap::<String, u16>::new();
        let mut layer_count = None::<u16>;
        let mut direction_priority = None::<Vec<String>>;
        let mut visual_priorities = Vec::<VisualOrderPriorityDef>::new();

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
                    .ok_or_else(|| {
                        parse_error(&line.text, "tag row must be: <name> = <value...>")
                    })?;
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

        let pending_groups =
            collect_puzzle_group_declarations_from_entries(&model.catalog_entries)?;
        let mut resolved_groups = HashSet::<String>::new();
        for entry in &model.catalog_entries {
            let tokens = split_header_tokens(&entry.header.text);
            match tokens.as_slice() {
                ["layers"] => {
                    let mut lines = entry.body.clone();
                    lines.push(source::LogicalLine::new(BLOCK_CLOSE, entry.header.line));
                    let next = parse_layers_block(
                        &lines,
                        0,
                        &mut named_layers,
                        &mut layer_count,
                        &mut catalog,
                        &pending_groups,
                        &mut resolved_groups,
                        &mut direction_priority,
                        &mut visual_priorities,
                        true,
                    )?;
                    if next != lines.len() {
                        return Err(parse_error(
                            &entry.header.text,
                            "layers block was not fully consumed",
                        ));
                    }
                    refresh_layer_tags_and_value_sets(&named_layers, &mut catalog);
                }
                ["layers", count] => {
                    layer_count = Some(parse_u16(
                        Some(count),
                        &entry.header.text,
                        "missing layer count",
                    )?);
                }
                ["layers", ..] => {
                    return Err(parse_error(
                        &entry.header.text,
                        "layers header is malformed",
                    ));
                }
                _ => {}
            }
        }
        resolve_pending_group_definitions(
            &pending_groups,
            None,
            &mut resolved_groups,
            &mut catalog,
        )?;

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
        let direction_priority = direction_priority.unwrap_or_else(|| {
            crate::lib_authoring_parse_order::default_direction_priority(&catalog)
        });
        crate::lib_authoring_parse_order::validate_direction_priority(
            &direction_priority,
            &catalog,
            "layers",
        )?;
        visual_priorities
            .retain(|priority| !priority.objects.is_empty() || !priority.animations.is_empty());
        crate::lib_authoring_parse_order::validate_layer_priorities(
            &visual_priorities,
            &catalog,
            "layers",
        )?;
        catalog.visual_order = VisualOrderDef {
            direction_priority,
            priorities: visual_priorities,
        };
        for entry in &model.catalog_entries {
            for selector in &entry.semantics.selectors {
                project_selector_occurrence(selector, &catalog, &mut recognition);
            }
        }
        Ok(catalog)
    })();
    crate::surface::ParseProduct::new(value, recognition)
}

fn project_selector_occurrence(
    selector: &model_syntax::SelectorOccurrenceSyntax,
    catalog: &Catalog,
    recognition: &mut crate::surface::ParserRecognition,
) {
    let mut offset = 0;
    for (index, part) in selector.text.split(':').enumerate() {
        if !part.is_empty() {
            mark_selector_component(
                recognition,
                selector.span.start + offset,
                part,
                index == 0,
                catalog,
            );
        }
        offset += part.len() + 1;
    }
}

fn mark_line_token(
    recognition: &mut crate::surface::ParserRecognition,
    line: &source::LogicalLine,
    text: Option<&str>,
    kind: crate::surface::SurfaceSemanticKind,
) {
    let Some(text) = text else {
        return;
    };
    for token in &line.tokens {
        if token.text == text {
            recognition.mark(
                crate::surface::SourceSpan {
                    start: token.start,
                    end: token.end,
                },
                kind,
            );
        }
    }
}

fn mark_selector_token(
    recognition: &mut crate::surface::ParserRecognition,
    line: &source::LogicalLine,
    selector: &str,
    catalog: &Catalog,
) {
    for token in &line.tokens {
        if token.text != selector {
            continue;
        }
        let mut offset = 0usize;
        for (index, part) in selector.split(':').enumerate() {
            if part.is_empty() {
                offset += 1;
                continue;
            }
            mark_selector_component(recognition, token.start + offset, part, index == 0, catalog);
            offset += part.len() + 1;
        }
    }
}

fn catalog_entry_lines(entry: &model_syntax::PuzzleEntrySyntax) -> Vec<source::LogicalLine> {
    let mut lines = Vec::with_capacity(entry.body.len() + 2);
    lines.push(entry.header.clone());
    lines.extend(entry.body.iter().cloned());
    lines.push(source::LogicalLine::new(BLOCK_CLOSE, entry.header.line));
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
            push_pending_group_declaration(line, &mut groups, &mut names)?;
        }
    }
    Ok(groups)
}

fn parser_surface_catalog_from_source_scan(
    source_scan: &source::SurfaceSourceScan,
    owner_dimension: Option<crate::ModelDimension>,
) -> crate::surface::ParseProduct<Result<ParserSurfaceSnapshot, DiagnosticReport>> {
    let mut recognition = crate::surface::ParserRecognition::default();
    let logical_lines = source_scan.editor_logical_lines();
    project_surface_sound_products(&logical_lines, &mut recognition);
    let (model_lines, pending_scenes) =
        match split_document_scene_sources(logical_lines, &mut recognition) {
            Ok(parts) => parts,
            Err(report) => return crate::surface::ParseProduct::new(Err(report), recognition),
        };
    let document_entries = match model_syntax::parse_document_entries(&model_lines) {
        Ok(entries) => entries,
        Err(report) => return crate::surface::ParseProduct::new(Err(report), recognition),
    };
    for entry in document_entries
        .iter()
        .filter(|entry| entry.directive == puzzle_authoring::PuzzleDirectiveSurface::Import)
    {
        recognition.merge(entry.semantics.fixed.clone());
    }
    let shell = match parse_document_shell_entries(&document_entries) {
        Ok(shell) => shell,
        Err(report) => return crate::surface::ParseProduct::new(Err(report), recognition),
    };
    let mut compile_diagnostics = Vec::new();
    if let Err(report) = model_syntax::validate_closed_entries(&document_entries, "document") {
        compile_diagnostics.extend(report.into_diagnostics());
    }
    let (models, mut diagnostics) =
        match model_syntax::parse_puzzle_models_from_document_entries(&document_entries) {
            Ok(models) => {
                if let Err(report) = model_syntax::validate_puzzle_model_diagnostics(&models) {
                    compile_diagnostics.extend(report.into_diagnostics());
                }
                let diagnostics = models
                    .iter()
                    .flat_map(|model| model.diagnostics.iter().map(ToString::to_string))
                    .collect();
                (models, diagnostics)
            }
            Err(report) => {
                let diagnostics = vec![report.to_string()];
                compile_diagnostics.extend(report.into_diagnostics());
                (Vec::new(), diagnostics)
            }
        };
    let scenes = match parse_pending_scene_sources(&pending_scenes, &models, &mut recognition) {
        Ok(scenes) => scenes,
        Err(report) => return crate::surface::ParseProduct::new(Err(report), recognition),
    };
    let mut model_catalogs = Vec::with_capacity(models.len());
    for model in &models {
        let parsed_catalog = build_puzzle_catalog(model);
        recognition.merge(parsed_catalog.recognition);
        match parsed_catalog.value {
            Ok(catalog) => model_catalogs.push(catalog),
            Err(report) => {
                diagnostics.push(report.to_string());
                compile_diagnostics.extend(report.into_diagnostics());
            }
        }
    }
    let loose_entries = models.is_empty().then(|| document_entries.clone());
    let parts = DocumentSourceParts {
        shell,
        models,
        model_catalogs,
        scenes,
        recognition,
    };
    let compile_parts = if compile_diagnostics.is_empty() {
        Ok(parts.clone())
    } else {
        Err(DiagnosticReport::from_diagnostics(compile_diagnostics))
    };
    let mut integrated = integrate_level_editor_document_parts(parts);
    if let (Some(entries), Some(dimension)) = (loose_entries, owner_dimension) {
        let catalog = Catalog::for_dimension(dimension);
        let mut level_count = 0;
        for entry in &entries {
            match entry.directive {
                puzzle_authoring::PuzzleDirectiveSurface::Visuals => {
                    let parsed =
                        parse_visuals_entry(entry, &catalog, &mut integrated.value.visuals);
                    integrated.recognition.merge(parsed.recognition);
                    if let Err(report) = parsed.value {
                        integrated.value.diagnostics.push(report.to_string());
                    }
                }
                puzzle_authoring::PuzzleDirectiveSurface::Level
                | puzzle_authoring::PuzzleDirectiveSurface::Levels => {
                    match parse_level_resource_entry(entry, level_count, None) {
                        Ok(resource) => {
                            project_level_products(
                                &resource.levels,
                                dimension,
                                &mut integrated.recognition,
                            );
                            level_count += resource.levels.len();
                        }
                        Err(report) => integrated.value.diagnostics.push(report.to_string()),
                    }
                }
                _ => {}
            }
        }
    }
    integrated.value.diagnostics.append(&mut diagnostics);
    let mut compile_parts = compile_parts;
    if let Ok(parts) = &mut compile_parts {
        parts.recognition = integrated.recognition.clone();
    }
    crate::surface::ParseProduct::new(
        Ok(ParserSurfaceSnapshot {
            level_editor: integrated.value,
            compile_parts,
        }),
        integrated.recognition,
    )
}

fn project_surface_sound_products(
    lines: &[source::LogicalLine],
    recognition: &mut crate::surface::ParserRecognition,
) {
    let mut index = 0;
    while index < lines.len() {
        if split_header_tokens(&lines[index]).as_slice() != ["sounds"]
            || !is_block_header_line(&lines[index])
        {
            index += 1;
            continue;
        }
        let Ok((node, next)) = authoring_grammar::parse_authoring_node_with_kind(
            lines,
            index,
            authoring_grammar::AuthoringKind::SoundsConfig,
            "sounds missing closing brace",
        ) else {
            index += 1;
            continue;
        };
        recognition.sound_products.extend(
            node.children
                .iter()
                .filter_map(|child| surface_sound_product(lines, child)),
        );
        index = next;
    }
}

fn surface_sound_product(
    lines: &[source::LogicalLine],
    node: &authoring_grammar::AuthoringNode,
) -> Option<crate::surface::SurfaceSoundProduct> {
    let (kind, name) = authoring_grammar::authoring_symbol_exports(node.kind)
        .iter()
        .find_map(|export| {
            let index = match export.source {
                authoring_grammar::AuthoringSymbolExportSource::HeaderArg(index) => index,
            };
            let kind = match export.target {
                authoring_grammar::AuthoringSymbolExportTarget::Sfx => {
                    crate::surface::SurfaceSoundKind::Sfx
                }
                authoring_grammar::AuthoringSymbolExportTarget::Music => {
                    crate::surface::SurfaceSoundKind::Music
                }
            };
            Some((kind, node.header_args.get(index)?))
        })?;
    let start = lines
        .get(node.source_index)?
        .tokens
        .first()
        .map(|token| token.start)?;
    let end = lines
        .get(node.closing_index)?
        .source_span()
        .map(|(_, end)| end)
        .or_else(|| {
            lines
                .get(node.source_index..=node.closing_index)?
                .iter()
                .rev()
                .find_map(|line| line.tokens.last().map(|token| token.end))
        })?;
    Some(crate::surface::SurfaceSoundProduct {
        span: crate::surface::SourceSpan { start, end },
        kind,
        name: name.clone(),
        params: node
            .definition_rows
            .iter()
            .filter_map(|row| row.single_value().map(|value| (row.key.clone(), value)))
            .map(|(key, value)| (key, trim_authoring_quotes(value).to_string()))
            .collect(),
    })
}

fn trim_authoring_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|stripped| stripped.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|stripped| stripped.strip_suffix('\''))
        })
        .unwrap_or(value)
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

struct ParserSurfaceSnapshot {
    level_editor: LevelEditorIntegration,
    compile_parts: Result<DocumentSourceParts, DiagnosticReport>,
}

pub(crate) struct LevelEditorIntegratedLevel {
    pub(crate) source_level_index: usize,
    pub(crate) name: String,
    pub(crate) state: State,
    pub(crate) layers: Vec<State>,
    pub(crate) regions: Vec<LevelRegionDef>,
    pub(crate) char_objects: HashMap<char, Vec<ObjectId>>,
}

fn parser_document_completion_symbols(
    parts: &DocumentSourceParts,
) -> crate::surface::SurfaceCompletionSymbols {
    let mut symbols = crate::surface::SurfaceCompletionSymbols::default();
    symbols.assets.extend(
        parts
            .shell
            .assets
            .entries
            .iter()
            .map(|asset| asset.path.clone()),
    );
    symbols.sfx.extend(
        parts
            .shell
            .sounds
            .sfx
            .iter()
            .map(|sound| sound.name.clone()),
    );
    symbols.music.extend(
        parts
            .shell
            .sounds
            .music
            .iter()
            .map(|sound| sound.name.clone()),
    );
    for model in &parts.models {
        symbols.levels.extend(
            model
                .body
                .levels
                .levels
                .iter()
                .map(|level| level.name.clone()),
        );
        symbols.routines.extend(
            model
                .body
                .routines
                .iter()
                .filter_map(|routine| routine.statement.tokens().get(1).cloned()),
        );
    }
    for scene in &parts.scenes {
        symbols.states.extend(
            scene
                .state
                .variables
                .iter()
                .map(|variable| variable.name.clone()),
        );
        symbols
            .states
            .extend(scene.state.puzzles.iter().map(|puzzle| puzzle.name.clone()));
        symbols
            .routines
            .extend(scene.routines.iter().map(|routine| routine.name.clone()));
    }
    symbols
}

fn integrate_level_editor_document_parts(
    parts: DocumentSourceParts,
) -> crate::surface::ParseProduct<LevelEditorIntegration> {
    let document_completion_symbols = parser_document_completion_symbols(&parts);
    let mut recognition = parts.recognition;
    recognition
        .completion_symbols
        .merge(document_completion_symbols);
    let dimension = parts
        .models
        .first()
        .map(|model| model.dimension)
        .unwrap_or_default();
    let mut catalog = parts
        .model_catalogs
        .first()
        .cloned()
        .unwrap_or_else(|| Catalog::for_dimension(dimension));
    let level_blocks = parts
        .models
        .iter()
        .filter(|model| model.dimension == crate::ModelDimension::Two)
        .flat_map(|model| model.body.levels.levels.iter().cloned())
        .collect::<Vec<_>>();
    let mut render_overlays = Vec::<(Vec<ObjectId>, char)>::new();
    let mut empty_char = Some('.');
    let mut diagnostics = Vec::<String>::new();
    for model in parts
        .models
        .iter()
        .filter(|model| model.dimension == crate::ModelDimension::Three)
    {
        let parsed = crate::level::recognize_spatial_levels(model);
        recognition.merge(parsed.recognition);
        if let Err(report) = parsed.value {
            diagnostics.push(report.to_string());
        }
    }
    for (index, model) in parts.models.iter().enumerate() {
        let model_catalog = parts.model_catalogs.get(index).unwrap_or(&catalog);
        for legend in &model.body.levels.legends {
            recognize_level_resource_legend(legend, model_catalog, &mut recognition);
        }
        for legend in model
            .body
            .levels
            .levels
            .iter()
            .flat_map(|level| &level.legends)
        {
            recognize_level_resource_legend(legend, model_catalog, &mut recognition);
        }
    }
    if dimension == crate::ModelDimension::Two {
        for model in &parts.models {
            for legend in &model.body.levels.legends {
                if let Err(report) = apply_level_resource_legend(
                    legend,
                    &mut catalog,
                    &mut render_overlays,
                    &mut empty_char,
                    &mut recognition,
                ) {
                    diagnostics.push(report.to_string());
                }
            }
        }
    }
    for (index, model) in parts.models.iter().enumerate() {
        let model_catalog = parts.model_catalogs.get(index).unwrap_or(&catalog);
        project_model_semantics(model, model_catalog, &mut recognition);
    }
    let mut visuals = VisualsDef::default();
    for (index, model) in parts.models.iter().enumerate() {
        let model_catalog = parts.model_catalogs.get(index).unwrap_or(&catalog);
        for entry in &model.body.visual_resources {
            let parsed = parse_visuals_entry(entry, model_catalog, &mut visuals);
            recognition.merge(parsed.recognition);
            if let Err(report) = parsed.value {
                diagnostics.push(report.to_string());
            }
        }
    }
    let mut completion_symbols = parser_catalog_completion_symbols(&catalog);
    completion_symbols
        .visuals
        .extend(visuals.entries.iter().map(|visual| visual.name.clone()));
    completion_symbols
        .shapes
        .extend(recognition.visual_refs.shape_names.iter().cloned());
    completion_symbols
        .colors
        .extend(recognition.visual_refs.color_names.iter().cloned());
    recognition.completion_symbols.merge(completion_symbols);
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
            let body = parse_level_body_for_editor(&level, &catalog)?;
            let mut char_objects = catalog.char_objects.clone();
            char_objects.extend(body.local_char_objects);
            let parsed =
                crate::level::parse_level(
                    &game,
                    &level.source,
                    &body.lines,
                    Some(empty_char),
                    &char_objects,
                    &[],
                );
            recognition.merge(parsed.recognition);
            let parsed = parsed.value?;
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
    crate::surface::ParseProduct::new(
        LevelEditorIntegration {
            catalog,
            empty_char: Some(empty_char),
            visuals,
            levels,
            diagnostics,
        },
        recognition,
    )
}

fn project_model_semantics(
    model: &model_syntax::PuzzleModelSyntax,
    catalog: &Catalog,
    recognition: &mut crate::surface::ParserRecognition,
) {
    project_level_products(&model.body.levels.levels, model.dimension, recognition);
    project_syntax_semantics(&model.body.semantics, catalog, recognition);
    for program in [
        model.body.rules.as_ref(),
        model.body.on_level_start.as_ref(),
        model.body.on_level_clear.as_ref(),
        model.body.on_last_level_clear.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        project_syntax_semantics(&program.semantics, catalog, recognition);
    }
    for routine in &model.body.routines {
        project_syntax_semantics(&routine.semantics, catalog, recognition);
    }
    for level in &model.body.levels.levels {
        for program in level
            .on_level_start
            .iter()
            .chain(&level.on_level_clear)
            .chain(level.rules_before.iter())
            .chain(level.rules_after.iter())
        {
            project_syntax_semantics(&program.semantics, catalog, recognition);
        }
    }
    if let Some(win_conditions) = &model.body.win_conditions {
        project_syntax_semantics(&win_conditions.semantics, catalog, recognition);
    }
    if let Some(lose_conditions) = &model.body.lose_conditions {
        project_syntax_semantics(&lose_conditions.semantics, catalog, recognition);
    }
}

fn project_level_products(
    levels: &[crate::level::LevelBlock],
    dimension: crate::ModelDimension,
    recognition: &mut crate::surface::ParserRecognition,
) {
    recognition
        .level_products
        .extend(levels.iter().enumerate().map(|(level_index, level)| {
            crate::surface::SurfaceLevelProduct {
                span: level.source_span,
                body_span: level.body_span,
                name: level.source_name.clone(),
                dimension,
                pack: level.pack.clone(),
                puzzle: level.puzzle.clone(),
                level_index,
            }
        }));
}

fn project_syntax_semantics(
    semantics: &model_syntax::SyntaxSemantics,
    catalog: &Catalog,
    recognition: &mut crate::surface::ParserRecognition,
) {
    recognition.merge(semantics.fixed.clone());
    for selector in &semantics.selectors {
        project_selector_occurrence(selector, catalog, recognition);
    }
    for identifier in &semantics.identifiers {
        let kind = if catalog.condition_names.contains_key(&identifier.text) {
            crate::surface::SurfaceSemanticKind::Condition
        } else if catalog.variable_names.contains_key(&identifier.text) {
            crate::surface::SurfaceSemanticKind::State
        } else if catalog.input_names.contains_key(&identifier.text) {
            crate::surface::SurfaceSemanticKind::Input
        } else {
            crate::surface::SurfaceSemanticKind::Binding
        };
        recognition.mark(identifier.span, kind);
    }
}

fn mark_selector_component(
    recognition: &mut crate::surface::ParserRecognition,
    start: usize,
    component: &str,
    selector_head: bool,
    catalog: &Catalog,
) {
    if let Some(open) = component.find('(')
        && component.ends_with(')')
    {
        let map = &component[..open];
        recognition.mark_resolved(
            crate::surface::SourceSpan {
                start,
                end: start + map.len(),
            },
            crate::surface::SurfaceSemanticKind::Group,
            crate::surface::ParserTokenResolution::ValueMap(map.to_string()),
        );
        let argument = &component[open + 1..component.len() - 1];
        let argument = argument.split('#').next().unwrap_or(argument);
        let (argument_kind, argument_resolution) = if catalog.value_sets.contains_key(argument) {
            (
                crate::surface::SurfaceSemanticKind::Group,
                crate::surface::ParserTokenResolution::ValueSet(argument.to_string()),
            )
        } else if catalog.object_axes.contains_key(argument) {
            (
                crate::surface::SurfaceSemanticKind::Group,
                crate::surface::ParserTokenResolution::ObjectAxis(argument.to_string()),
            )
        } else {
            (
                crate::surface::SurfaceSemanticKind::Variant,
                crate::surface::ParserTokenResolution::Variant(argument.to_string()),
            )
        };
        recognition.mark_resolved(
            crate::surface::SourceSpan {
                start: start + open + 1,
                end: start + open + 1 + argument.len(),
            },
            argument_kind,
            argument_resolution,
        );
        if let Some(hash) = component.find('#') {
            let binding = &component[hash + 1..component.len() - 1];
            recognition.mark_resolved(
                crate::surface::SourceSpan {
                    start: start + hash + 1,
                    end: start + component.len() - 1,
                },
                crate::surface::SurfaceSemanticKind::Binding,
                crate::surface::ParserTokenResolution::Binding(binding.to_string()),
            );
        }
        return;
    }
    let semantic = component.split('#').next().unwrap_or(component);
    let (kind, resolution) = if selector_head {
        if catalog.object_groups.contains_key(semantic) {
            (
                crate::surface::SurfaceSemanticKind::Group,
                crate::surface::ParserTokenResolution::ObjectGroup(semantic.to_string()),
            )
        } else {
            (
                crate::surface::SurfaceSemanticKind::Object,
                crate::surface::ParserTokenResolution::Object(semantic.to_string()),
            )
        }
    } else if catalog.value_sets.contains_key(semantic) {
        (
            crate::surface::SurfaceSemanticKind::Group,
            crate::surface::ParserTokenResolution::ValueSet(semantic.to_string()),
        )
    } else if catalog.object_axes.contains_key(semantic) {
        (
            crate::surface::SurfaceSemanticKind::Group,
            crate::surface::ParserTokenResolution::ObjectAxis(semantic.to_string()),
        )
    } else {
        (
            crate::surface::SurfaceSemanticKind::Variant,
            crate::surface::ParserTokenResolution::Variant(semantic.to_string()),
        )
    };
    recognition.mark_resolved(
        crate::surface::SourceSpan {
            start,
            end: start + semantic.len(),
        },
        kind,
        resolution,
    );
    if let Some(hash) = component.find('#') {
        let binding = &component[hash + 1..];
        recognition.mark_resolved(
            crate::surface::SourceSpan {
                start: start + hash + 1,
                end: start + component.len(),
            },
            crate::surface::SurfaceSemanticKind::Binding,
            crate::surface::ParserTokenResolution::Binding(binding.to_string()),
        );
    }
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
    if let Some(value) = values
        .iter()
        .copied()
        .find(|value| puzzle_authoring::is_selector_tag_syntax_literal(value))
    {
        return Err(parse_error(
            line,
            &format!("tag value {value} is reserved by selector syntax"),
        ));
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
        .all(|value| puzzle_authoring::MOVEMENT_DIRECTIONS_3D.contains(&value.as_str()))
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
    puzzle_authoring::is_absolute_direction_set(name) || name == "layers"
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

fn validate_selector_alias_name(
    value: &str,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    if puzzle_authoring::is_symbol_name(value) {
        Ok(())
    } else {
        Err(parse_error(line, &format!("{label} must be a symbol name")))
    }
}

fn validate_rule_name(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    if puzzle_authoring::is_symbol_name(value) {
        Ok(())
    } else {
        Err(parse_error(line, "routine name must be a symbol name"))
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
    lines: &[source::LogicalLine],
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

fn is_layers_merge_block(line: &str) -> bool {
    is_block_header_line(line) && matches!(split_header_tokens(line).as_slice(), ["merge"])
}

fn parse_layers_block(
    lines: &[source::LogicalLine],
    start: usize,
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
    catalog: &mut Catalog,
    pending_groups: &[PendingGroupDefinition],
    resolved_groups: &mut HashSet<String>,
    direction_priority: &mut Option<Vec<String>>,
    visual_priorities: &mut Vec<VisualOrderPriorityDef>,
    allow_merge: bool,
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
            ["priority", "=", directions @ ..] => {
                if !allow_merge {
                    return Err(parse_error(
                        &lines[i],
                        "directional priority belongs directly to layers, not merge",
                    ));
                }
                if directions.is_empty() {
                    return Err(parse_error(
                        &lines[i],
                        "layers priority requires direction names",
                    ));
                }
                if direction_priority
                    .replace(
                        directions
                            .iter()
                            .map(|value| (*value).to_string())
                            .collect(),
                    )
                    .is_some()
                {
                    return Err(parse_error(
                        &lines[i],
                        "layers may declare directional priority only once",
                    ));
                }
            }
            _ if is_layers_merge_block(&lines[i].text) => {
                if !allow_merge {
                    return Err(parse_error(&lines[i], "layers merge cannot be nested"));
                }
                let mut merged_rows = Vec::new();
                let next = parse_layers_block(
                    lines,
                    i + 1,
                    named_layers,
                    layer_count,
                    catalog,
                    pending_groups,
                    resolved_groups,
                    direction_priority,
                    &mut merged_rows,
                    false,
                )?;
                if merged_rows.is_empty() {
                    return Err(parse_error(&lines[i], "layers merge must not be empty"));
                }
                let mut objects = Vec::new();
                let mut animations = Vec::new();
                for row in merged_rows {
                    objects.extend(row.objects);
                    animations.extend(row.animations);
                }
                objects.sort();
                animations.sort();
                push_visual_priority(
                    visual_priorities,
                    VisualOrderPriorityDef {
                        objects,
                        animations,
                        merge: true,
                    },
                );
                i = next;
                continue;
            }
            ["for", ..] => {
                let value_sets = catalog_value_sets(catalog);
                let expansion = expand_for_block_lines(
                    lines,
                    i,
                    &value_sets,
                    &catalog.numeric_variable_defaults,
                    &catalog.maps,
                )?;
                for mut expanded_lines in expansion.bodies {
                    expanded_lines.push(source::LogicalLine::new(BLOCK_CLOSE, lines[i].line));
                    let parsed_i = parse_layers_block(
                        &expanded_lines,
                        0,
                        named_layers,
                        layer_count,
                        catalog,
                        pending_groups,
                        resolved_groups,
                        direction_priority,
                        visual_priorities,
                        allow_merge,
                    )?;
                    if parsed_i != expanded_lines.len() {
                        return Err(parse_error(&lines[i], "for expansion failed"));
                    }
                }
                i = expansion.next;
                continue;
            }
            _ => {
                let row = parse_layer_row(&lines[i])?;
                if row.each {
                    if !row.animations.is_empty() {
                        return Err(parse_error(
                            &lines[i],
                            "each layer row cannot contain animation references",
                        ));
                    }
                    let first_layer = layer_count.unwrap_or(0);
                    assign_selectors_to_separate_layers(
                        &row.state_selectors,
                        &lines[i],
                        named_layers,
                        layer_count,
                        catalog,
                    )?;
                    let next_layer = layer_count.unwrap_or(first_layer);
                    for layer in first_layer..next_layer {
                        let priority =
                            visual_priority_for_state_layer(layer, Vec::new(), false, catalog)?;
                        push_visual_priority(visual_priorities, priority);
                    }
                } else if let Some(name) = row.name {
                    let objects = if row.state_selectors.is_empty() {
                        Vec::new()
                    } else {
                        let layer =
                            layer_id_for_name(name, &lines[i], named_layers, layer_count, catalog)?;
                        define_or_assign_terms_to_layer(
                            &row.state_selectors,
                            &lines[i],
                            layer,
                            catalog,
                        )?;
                        register_layer_tag_from_layer(name, layer, catalog);
                        visual_priority_for_state_layer(layer, Vec::new(), false, catalog)?.objects
                    };
                    push_visual_priority(
                        visual_priorities,
                        VisualOrderPriorityDef {
                            objects,
                            animations: row.animations,
                            merge: false,
                        },
                    );
                } else {
                    let objects = if row.state_selectors.is_empty() {
                        Vec::new()
                    } else {
                        let layer = anonymous_layer_id(named_layers, layer_count);
                        define_or_assign_terms_to_layer(
                            &row.state_selectors,
                            &lines[i],
                            layer,
                            catalog,
                        )?;
                        visual_priority_for_state_layer(layer, Vec::new(), false, catalog)?.objects
                    };
                    if objects.is_empty() && row.animations.is_empty() {
                        return Err(parse_error(&lines[i], "layer row must not be empty"));
                    }
                    push_visual_priority(
                        visual_priorities,
                        VisualOrderPriorityDef {
                            objects,
                            animations: row.animations,
                            merge: false,
                        },
                    );
                }
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start - 1],
            "layers missing closing brace",
        ));
    }
    Ok(i + 1)
}

fn predeclare_layer_block_objects(
    lines: &[source::LogicalLine],
    start: usize,
    pending_groups: &[PendingGroupDefinition],
    catalog: &mut Catalog,
) -> Result<Vec<String>, DiagnosticReport> {
    let mut terms = Vec::<String>::new();
    let mut used_groups = Vec::<String>::new();
    let collected = collect_layer_block_terms(
        lines,
        start,
        pending_groups,
        catalog,
        &mut terms,
        &mut used_groups,
    );
    predeclare_layer_terms(&terms, catalog)?;
    collected?;
    Ok(used_groups)
}

fn collect_layer_block_terms(
    lines: &[source::LogicalLine],
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
            ["priority", "=", ..] => {
                i += 1;
                continue;
            }
            _ if is_layers_merge_block(&lines[i].text) => {
                let next = collect_layer_block_terms(
                    lines,
                    i + 1,
                    pending_groups,
                    catalog,
                    terms,
                    used_groups,
                )?;
                i = next;
                continue;
            }
            ["for", ..] => {
                let value_sets = catalog_value_sets(catalog);
                let expansion = expand_for_block_lines(
                    lines,
                    i,
                    &value_sets,
                    &catalog.numeric_variable_defaults,
                    &catalog.maps,
                )?;
                for mut expanded_lines in expansion.bodies {
                    expanded_lines.push(source::LogicalLine::new(BLOCK_CLOSE, lines[i].line));
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
                i = expansion.next;
                continue;
            }
            _ => {
                let row = parse_layer_row(&lines[i])?;
                collect_layer_terms(
                    &row.state_selectors,
                    &lines[i],
                    pending_groups,
                    terms,
                    used_groups,
                )?;
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start - 1],
            "layers missing closing brace",
        ));
    }
    Ok(i + 1)
}

struct ParsedLayerRow<'a> {
    name: Option<&'a str>,
    each: bool,
    state_selectors: Vec<&'a str>,
    animations: Vec<String>,
}

fn parse_layer_row(line: &str) -> Result<ParsedLayerRow<'_>, DiagnosticReport> {
    let row = puzzle_authoring::slot_row_surface(line)
        .ok_or_else(|| parse_error(line, "invalid layer row"))?;
    let (name, each, selectors) = match row {
        puzzle_authoring::SlotRowSurface::Each { selectors } => (None, true, selectors),
        puzzle_authoring::SlotRowSurface::Named(assignment) => {
            (Some(assignment.name), false, assignment.selectors)
        }
        puzzle_authoring::SlotRowSurface::Anonymous { selectors } => (None, false, selectors),
    };
    let mut state_selectors = Vec::new();
    let mut animations = Vec::new();
    for selector in selectors {
        if let Some(animation) = selector.strip_prefix('!') {
            if animation.is_empty() || animation.starts_with('!') {
                return Err(parse_error(
                    line,
                    "animation layer reference must be !<visual>",
                ));
            }
            animations.push(animation.to_string());
        } else {
            state_selectors.push(selector);
        }
    }
    Ok(ParsedLayerRow {
        name,
        each,
        state_selectors,
        animations,
    })
}

fn visual_priority_for_state_layer(
    layer: u16,
    animations: Vec<String>,
    merge: bool,
    catalog: &Catalog,
) -> Result<VisualOrderPriorityDef, DiagnosticReport> {
    let layer = LayerId(layer);
    let mut objects = catalog
        .object_defs
        .iter()
        .filter_map(|object| (object.layer_id == layer).then_some(object.id))
        .collect::<Vec<_>>();
    objects.sort_by_key(|object| object.0);
    let objects = objects
        .into_iter()
        .map(|object| {
            catalog
                .object_labels
                .get(&object)
                .cloned()
                .ok_or_else(|| DiagnosticReport::error("layer object is missing its name"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(VisualOrderPriorityDef {
        objects,
        animations,
        merge,
    })
}

fn push_visual_priority(
    priorities: &mut Vec<VisualOrderPriorityDef>,
    priority: VisualOrderPriorityDef,
) {
    for previous in priorities.iter_mut() {
        previous
            .objects
            .retain(|object| !priority.objects.contains(object));
    }
    priorities.retain(|previous| !previous.objects.is_empty() || !previous.animations.is_empty());
    priorities.push(priority);
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
                &format!(
                    "object selector must name every variant slot; use {} for unconstrained slots",
                    puzzle_authoring::SELECTOR_WILDCARD
                ),
            ));
        };
        let values = if puzzle_authoring::is_selector_wildcard(value) {
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
        || (puzzle_authoring::is_selector_wildcard(base)
            && selector.contains(':')
            && !object_schemas.is_empty())
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
        .insert("layers".to_string(), values.clone());
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

fn add_spatial_directions(
    line: &str,
    catalog: &mut Catalog,
    directions: &mut Vec<DirectionalInput>,
) -> Result<(), DiagnosticReport> {
    let names = catalog
        .value_sets
        .get("directions")
        .cloned()
        .ok_or_else(|| DiagnosticReport::error("missing canonical directions domain"))?;
    for name in names {
        let input = catalog
            .input_names
            .get(&name)
            .copied()
            .map(Ok)
            .unwrap_or_else(|| add_input_name(&name, line, catalog))?;
        if !directions.iter().any(|direction| direction.input == input) {
            directions.push(DirectionalInput {
                input,
                direction: name,
            });
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

fn directions_include_all_spatial(
    directions: &[DirectionalInput],
    direction_names: &[String],
    input_names: &HashMap<String, InputId>,
) -> bool {
    direction_names.iter().all(|name| {
        input_names
            .get(name)
            .is_some_and(|input| directions.iter().any(|direction| direction.input == *input))
    })
}
