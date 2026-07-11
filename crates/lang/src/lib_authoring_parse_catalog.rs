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

fn record_authoring_declaration_surface_tokens(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    match scope {
        Some(SourceScope::Tags) => record_tag_declaration_surface_tokens(tokens, sink),
        Some(SourceScope::Layers) => record_layer_declaration_surface_tokens(tokens, sink),
        Some(SourceScope::Group) => record_group_declaration_surface_tokens(tokens, sink),
        Some(SourceScope::Mark) => record_mark_declaration_surface_tokens(tokens, sink),
        _ => {}
    }
}

fn record_tag_declaration_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    let [name, separator, values @ ..] = tokens else {
        return;
    };
    if separator.text != "=" || values.is_empty() {
        return;
    }
    add_surface_symbol(sink, name, SurfaceSemanticKind::Group);
    for value in values {
        add_surface_symbol(sink, value, SurfaceSemanticKind::Variant);
    }
}

fn record_layer_declaration_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    if tokens
        .first()
        .is_some_and(|token| matches!(token.text.as_str(), "for" | "}"))
    {
        return;
    }
    let selector_start = usize::from(tokens.first().is_some_and(|token| token.text == "each"));
    let selector_tokens = if let Some(separator) = tokens.iter().position(|token| token.text == "=")
    {
        if separator > 0 {
            add_surface_symbol(sink, &tokens[0], SurfaceSemanticKind::Group);
        }
        &tokens[separator + 1..]
    } else {
        &tokens[selector_start..]
    };
    for token in selector_tokens {
        add_selector_head_surface_symbol(sink, token, SurfaceSemanticKind::Object);
    }
}

fn record_group_declaration_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    let Some(separator) = tokens.iter().position(|token| token.text == "=") else {
        return;
    };
    if separator > 0 {
        add_surface_symbol(sink, &tokens[0], SurfaceSemanticKind::Group);
    }
}

fn record_mark_declaration_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    let Some(name) = tokens.first() else {
        return;
    };
    if matches!(name.text.as_str(), "}" | "for") {
        return;
    }
    add_selector_head_surface_symbol(sink, name, SurfaceSemanticKind::Mark);
}

fn add_selector_head_surface_symbol(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    kind: SurfaceSemanticKind,
) {
    let Some((start, end)) = surface_identifier_bounds(&token.text) else {
        return;
    };
    let span = SourceSpan {
        start: token.start + start,
        end: token.start + end,
    };
    sink.mark(span, kind);
}

fn add_surface_symbol(sink: &mut SurfaceSink, token: &SourceToken, kind: SurfaceSemanticKind) {
    let span = SourceSpan {
        start: token.start,
        end: token.end,
    };
    sink.mark(span, kind);
}

fn record_surface_builtin_completion_symbols(sink: &mut SurfaceSink) {
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

fn record_surface_completion_value_sets(scan: &SurfaceScan, sink: &mut SurfaceSink) {
    for line in &scan.lines {
        if line.scope != Some(SourceScope::Tags) {
            continue;
        }
        let tokens = line
            .structural_token_spans
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let [name, separator, values @ ..] = tokens.as_slice() else {
            continue;
        };
        if *separator != "=" || !surface_completion_tag_set_tokens(name, values) {
            continue;
        }
        record_surface_completion_value_set(name, values, sink);
    }
}

fn record_surface_completion_value_set(name: &str, values: &[&str], sink: &mut SurfaceSink) {
    let symbols = sink.completion_symbols_mut();
    symbols.value_set_names.insert(name.to_string());
    let values = values
        .iter()
        .filter(|value| surface_catalog_identifier(value))
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    if values
        .iter()
        .all(|value| surface_completion_direction_value(value))
    {
        symbols.direction_sets.insert(name.to_string());
    }
    symbols.object_name_atoms.extend(values.iter().cloned());
    symbols.value_sets.insert(name.to_string(), values);
}

fn record_surface_completion_line(
    option_block: Option<SurfaceOptionBlock>,
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    if record_authoring_content_completion_line(option_block, tokens, sink) {
        return;
    }
    if record_authoring_child_completion_line(option_block, tokens, sink) {
        return;
    }
    let token_texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    match token_texts.as_slice() {
        ["puzzle", name, ..] if scope.is_none() => {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().puzzles, name);
        }
        ["scene", name, ..] => {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().scenes, name);
        }
        ["level", name, ..] => {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().levels, name);
        }
        ["routine", name, ..] => {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().routines, name);
        }
        ["input", name, ..] | ["direction", name, ..] => {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().inputs, name);
        }
        ["query", name, ..] => {
            insert_surface_completion_identifier(
                &mut sink.completion_symbols_mut().condition_defs,
                name,
            );
        }
        ["shape", table, ..] if scope == Some(SourceScope::Visuals) => {
            record_surface_completion_visual_table_ref(table, true, sink);
        }
        ["colors", table, ..] if scope == Some(SourceScope::Visuals) => {
            record_surface_completion_visual_color_ref(table, sink);
        }
        [name, "=", ..] if scope == Some(SourceScope::VisualColorTable) => {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().colors, name);
        }
        [table_ref] if scope == Some(SourceScope::VisualColorTable) => {
            record_surface_completion_visual_color_ref(table_ref, sink);
        }
        [name] if scope == Some(SourceScope::VisualShapeTable) => {
            record_surface_completion_visual_table_ref(name, true, sink);
        }
        ["var" | "const", name, ..]
        | ["persistent", "var" | "const", name, ..]
        | ["persistent", name, ..] => {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().states, name);
        }
        ["puzzle" | "puzzle3", name, "=", ..]
            if matches!(
                scope,
                Some(SourceScope::SceneLayout | SourceScope::SceneState)
            ) =>
        {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().states, name);
        }
        [name, "=", ..]
            if matches!(
                scope,
                Some(SourceScope::SceneLayout | SourceScope::SceneState)
            ) =>
        {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().states, name);
        }
        [name, "=", selectors @ ..] if scope == Some(SourceScope::Group) => {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().groups, name);
            record_surface_completion_selector_specs(selectors, sink);
        }
        [..] if scope == Some(SourceScope::Layers) => {
            record_surface_completion_layer_row(&token_texts, sink);
        }
        [name, "=", ty] if scope == Some(SourceScope::Mark) => {
            record_surface_completion_mark_spec(name, Some(*ty), sink);
        }
        [spec] if scope == Some(SourceScope::Mark) => {
            let cleaned = surface_completion_clean_spec(spec);
            let (name, ty) = cleaned
                .split_once('=')
                .map_or((cleaned, None), |(name, ty)| (name, Some(ty)));
            record_surface_completion_mark_spec(name, ty, sink);
        }
        [..] if scope == Some(SourceScope::Keys) => {
            record_surface_completion_keys(&token_texts, sink);
        }
        _ => {}
    }
}

fn record_authoring_content_completion_line(
    option_block: Option<SurfaceOptionBlock>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    let Some(SurfaceOptionBlock::Authoring(kind)) = option_block else {
        return false;
    };
    let crate::authoring_grammar::AuthoringBody::Content(content) =
        crate::authoring_grammar::authoring_kind_spec(kind).body
    else {
        return false;
    };
    let line = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let Ok(Some(row)) = crate::authoring_grammar::parse_authoring_content_row(content, &line)
    else {
        return false;
    };
    if let Some(values) = crate::authoring_grammar::authoring_capture_values(&row.captures, "path")
    {
        for value in values {
            insert_surface_completion_path_like(&mut sink.completion_symbols_mut().assets, value);
        }
    }
    true
}

fn record_authoring_child_completion_line(
    option_block: Option<SurfaceOptionBlock>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    let Some(SurfaceOptionBlock::Authoring(parent)) = option_block else {
        return false;
    };
    let [surface, ..] = tokens else {
        return false;
    };
    let Some(child) = crate::authoring_grammar::placed_authoring_kind(parent, &surface.text) else {
        return false;
    };
    let mut recorded = false;
    for export in crate::authoring_grammar::authoring_symbol_exports(child) {
        let Some(value) = authoring_symbol_export_value(export.source, tokens) else {
            continue;
        };
        record_authoring_symbol_export(export.target, value, sink);
        recorded = true;
    }
    recorded
}

fn authoring_symbol_export_value(
    source: crate::authoring_grammar::AuthoringSymbolExportSource,
    tokens: &[SourceToken],
) -> Option<&str> {
    match source {
        crate::authoring_grammar::AuthoringSymbolExportSource::HeaderArg(index) => {
            tokens.get(index + 1).map(|token| token.text.as_str())
        }
    }
}

fn record_authoring_symbol_export(
    target: crate::authoring_grammar::AuthoringSymbolExportTarget,
    value: &str,
    sink: &mut SurfaceSink,
) {
    match target {
        crate::authoring_grammar::AuthoringSymbolExportTarget::Sfx => {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().sfx, value);
        }
        crate::authoring_grammar::AuthoringSymbolExportTarget::Music => {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().music, value);
        }
    }
}

fn record_surface_completion_layer_row(tokens: &[&str], sink: &mut SurfaceSink) {
    match tokens {
        [] => {}
        [name, "=", selectors @ ..] => {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().groups, name);
            record_surface_completion_selector_specs(selectors, sink);
        }
        ["each", selectors @ ..] => {
            record_surface_completion_selector_specs(selectors, sink);
        }
        ["for", ..] => {}
        [selectors @ ..] => {
            record_surface_completion_selector_specs(selectors, sink);
        }
    }
}

fn record_surface_completion_selector_specs(specs: &[&str], sink: &mut SurfaceSink) {
    for spec in specs {
        if matches!(*spec, "=" | "each" | "for") {
            continue;
        }
        record_surface_completion_object_spec(spec, sink);
    }
}

fn record_surface_completion_object_spec(spec: &str, sink: &mut SurfaceSink) {
    let cleaned = surface_completion_clean_spec(spec);
    let parts = cleaned.split(':').collect::<Vec<_>>();
    let Some(base) = parts.first().copied() else {
        return;
    };
    insert_surface_completion_identifier(&mut sink.completion_symbols_mut().objects, base);
    let value_set_names = sink.completion_symbols_mut().value_set_names.clone();
    if parts.len() > 1
        && parts[1..]
            .iter()
            .all(|part| value_set_names.contains(*part))
    {
        sink.completion_symbols_mut()
            .object_axes
            .entry(base.to_string())
            .or_insert_with(|| parts[1..].iter().map(|part| (*part).to_string()).collect());
    }
    sink.completion_symbols_mut()
        .object_name_atoms
        .extend(parts[1..].iter().map(|part| (*part).to_string()));
}

fn record_surface_completion_mark_spec(name: &str, ty: Option<&str>, sink: &mut SurfaceSink) {
    insert_surface_completion_identifier(&mut sink.completion_symbols_mut().markes, name);
    if let Some(ty) = ty.filter(|ty| !matches!(*ty, "bool" | "int")) {
        insert_surface_completion_identifier(
            &mut sink.completion_symbols_mut().object_name_atoms,
            ty,
        );
    }
}

fn record_surface_completion_keys(tokens: &[&str], sink: &mut SurfaceSink) {
    let Some(separator) = tokens.iter().position(|token| matches!(*token, "=" | "->")) else {
        return;
    };
    for token in &tokens[..separator] {
        insert_surface_completion_identifier(&mut sink.completion_symbols_mut().inputs, token);
    }
    if let Some(command) = tokens.get(separator + 1) {
        insert_surface_completion_identifier(&mut sink.completion_symbols_mut().commands, command);
    }
}

fn record_surface_completion_visual_table_ref(
    table: &str,
    shape_table: bool,
    sink: &mut SurfaceSink,
) {
    if let Some((name, axis)) = table.split_once(':') {
        if shape_table {
            insert_surface_completion_identifier(&mut sink.completion_symbols_mut().shapes, name);
        }
        insert_surface_completion_identifier(
            &mut sink.completion_symbols_mut().object_name_atoms,
            axis,
        );
    } else if shape_table {
        insert_surface_completion_identifier(&mut sink.completion_symbols_mut().shapes, table);
    }
}

fn record_surface_completion_visual_color_ref(table: &str, sink: &mut SurfaceSink) {
    if let Some((name, axis)) = table.split_once(':') {
        insert_surface_completion_identifier(&mut sink.completion_symbols_mut().colors, name);
        insert_surface_completion_identifier(
            &mut sink.completion_symbols_mut().object_name_atoms,
            axis,
        );
    }
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

fn surface_completion_tag_set_tokens(name: &str, values: &[&str]) -> bool {
    surface_catalog_identifier(name)
        && !values.is_empty()
        && values.iter().all(|value| surface_catalog_identifier(value))
}

fn surface_completion_direction_value(value: &str) -> bool {
    matches!(value, "up" | "down" | "left" | "right")
}

fn surface_completion_clean_spec(spec: &str) -> &str {
    let spec = spec.trim_matches(|ch: char| matches!(ch, '[' | ']' | '(' | ')' | '|'));
    spec.split_once('{').map_or(spec, |(head, _)| head)
}

fn insert_surface_completion_identifier(target: &mut BTreeSet<String>, value: &str) {
    if surface_catalog_identifier(value) {
        target.insert(value.to_string());
    }
}

fn insert_surface_completion_path_like(target: &mut BTreeSet<String>, value: &str) {
    let cleaned = value.trim_matches('"');
    if !cleaned.is_empty() {
        target.insert(cleaned.to_string());
    }
}

fn surface_catalog_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '@' || first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| {
            ch == '_' || ch == ':' || ch == '.' || ch == '-' || ch.is_ascii_alphanumeric()
        })
}

fn record_parser_resolved_surface_tokens(
    scan: &SurfaceScan,
    catalog: &Catalog,
    sink: &mut SurfaceSink,
) {
    let value_sets = catalog_value_sets(catalog);
    let mut current_map_axis = None::<String>;
    for line in &scan.lines {
        if let Some(axis) = parser_surface_map_axis(line) {
            current_map_axis = Some(axis);
        }
        record_parser_resolved_layer_surface_tokens(line, catalog, sink);
        record_parser_resolved_group_surface_tokens(line, catalog, sink);
        record_parser_resolved_legend_surface_tokens(line, catalog, sink);
        record_parser_resolved_map_surface_tokens(
            line,
            current_map_axis.as_deref(),
            &value_sets,
            sink,
        );
        record_parser_resolved_for_expansion_surface_tokens(line, catalog, sink);
        record_parser_resolved_rule_surface_tokens(line, catalog, sink);
        record_parser_resolved_condition_surface_tokens(line, catalog, sink);
        if line.scope == Some(SourceScope::Map)
            && line
                .structural_token_spans
                .first()
                .is_some_and(|token| token.text == "}")
        {
            current_map_axis = None;
        }
    }
}

fn record_parser_resolved_condition_surface_tokens(
    line: &SurfaceScanLine,
    catalog: &Catalog,
    sink: &mut SurfaceSink,
) {
    if line.scope != Some(SourceScope::Condition) {
        return;
    }
    let [all, subject, on, cover] = line.structural_token_spans.as_slice() else {
        return;
    };
    if all.text != "all" || on.text != "on" {
        return;
    }
    record_resolved_object_selector_surface_token(subject, &line.content, catalog, sink);
    record_resolved_object_selector_surface_token(cover, &line.content, catalog, sink);
}

fn record_parser_resolved_for_expansion_surface_tokens(
    line: &SurfaceScanLine,
    catalog: &Catalog,
    sink: &mut SurfaceSink,
) {
    if line.scope != Some(SourceScope::Other) {
        return;
    }
    let tokens = &line.structural_token_spans;
    let [for_keyword, binding, in_keyword, sources @ ..] = tokens.as_slice() else {
        return;
    };
    if for_keyword.text != "for" || in_keyword.text != "in" || sources.is_empty() {
        return;
    }

    add_surface_symbol(sink, for_keyword, SurfaceSemanticKind::Keyword);
    add_surface_symbol(sink, binding, SurfaceSemanticKind::Binding);
    add_surface_symbol(sink, in_keyword, SurfaceSemanticKind::Keyword);
    for source in sources {
        if catalog_value_set(catalog, &source.text).is_some()
            || catalog.object_groups.contains_key(&source.text)
        {
            add_surface_symbol(sink, source, SurfaceSemanticKind::Group);
        } else {
            record_resolved_object_selector_surface_token(source, &line.content, catalog, sink);
        }
    }
}

fn parser_surface_map_axis(line: &SurfaceScanLine) -> Option<String> {
    let tokens = &line.structural_token_spans;
    let [keyword, _name, axis, ..] = tokens.as_slice() else {
        return None;
    };
    (keyword.text == "map").then(|| axis.text.clone())
}

fn parser_surface_catalog(source: &str) -> Option<Catalog> {
    let parts = parse_document_source_parts(source).ok()?;
    let lines = parts
        .model_lines
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>();
    let mut catalog = Catalog::default();
    let mut i = 0usize;
    while i < lines.len() {
        let tokens = split_header_tokens(&lines[i]);
        if matches!(tokens.as_slice(), ["puzzle", _]) {
            i = record_parser_surface_puzzle_catalog(&lines, i, &mut catalog, false)
                .ok()?
                .0;
        } else {
            i += 1;
        }
    }
    Some(catalog)
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
    let lines = parts
        .model_lines
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>();
    let mut catalog = Catalog::default();
    let mut level_blocks = Vec::<LevelBlock>::new();
    let mut pending_level_blocks = Vec::<PendingLevelBlock>::new();
    let mut pending_visual_blocks = Vec::<usize>::new();
    let mut render_overlays = Vec::<(Vec<ObjectId>, char)>::new();
    let mut empty_char = None::<char>;
    let mut diagnostics = Vec::<String>::new();
    let mut i = 0usize;
    while i < lines.len() {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["puzzle", _] => {
                match record_parser_surface_puzzle_catalog(&lines, i, &mut catalog, false) {
                    Ok((next_i, mut levels, mut visuals)) => {
                        pending_level_blocks.append(&mut levels);
                        pending_visual_blocks.append(&mut visuals);
                        i = next_i;
                    }
                    Err(report) => {
                        diagnostics.push(report.to_string());
                        i = recover_after_directive_error(&lines, i);
                    }
                }
            }
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
    if let Some(empty_char) = empty_char {
        for (source_level_index, level) in level_blocks.into_iter().enumerate() {
            let level_name = level.name.clone();
            let parsed = (|| {
                let body = parse_level_body_for_editor(&level, &catalog, empty_char)?;
                let mut char_objects = catalog.char_objects.clone();
                char_objects.extend(body.local_char_objects);
                let parsed = parse_level(
                    &game,
                    &body.lines,
                    Some(empty_char),
                    &char_objects,
                    &[],
                )?;
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
    } else if !level_blocks.is_empty() {
        diagnostics.push(
            "level editor requires `levels { legend { . = empty } }` before it can integrate level cells"
                .to_string(),
        );
    }
    Ok(LevelEditorIntegration {
        catalog,
        empty_char,
        visuals,
        levels,
        diagnostics,
    })
}

fn record_parser_surface_puzzle_catalog(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
    strict: bool,
) -> Result<(usize, Vec<PendingLevelBlock>, Vec<usize>), DiagnosticReport> {
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
        match tokens.as_slice() {
            [] => i += 1,
            ["tags"] => i = skip_tags_block(lines, i).unwrap_or(i + 1),
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
            ["layers"] => {
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
            ["layers", count] => {
                match parse_u16(Some(count), &lines[i], "missing layer count") {
                    Ok(count) => layer_count = Some(count),
                    Err(error) if strict => return Err(error),
                    Err(_) => {}
                }
                i += 1;
            }
            ["marks"] => {
                i = match parse_mark_block(lines, i, catalog) {
                    Ok(next_i) => next_i,
                    Err(error) if strict => return Err(error),
                    Err(_) => recover_after_directive_error(lines, i),
                };
            }
            ["groups"] => {
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
            ["levels", ..] => {
                pending_level_blocks.push(PendingLevelBlock::levels(i, puzzle_name.clone()));
                i = collect_levels_authoring_entry(lines, i)?.1;
            }
            ["level", ..] => {
                pending_level_blocks.push(PendingLevelBlock::level(i, puzzle_name.clone()));
                i = parse_level_block(lines, i, 0)?.1;
            }
            ["sprites", ..] => {
                pending_visual_blocks.push(i);
                i = collect_authoring_entry(lines, i, AuthoringEntryOwner::DocumentVisuals)?.1;
            }
            _ => i = recover_after_directive_error(lines, i),
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
    ))
}

fn record_parser_resolved_layer_surface_tokens(
    line: &SurfaceScanLine,
    catalog: &Catalog,
    sink: &mut SurfaceSink,
) {
    if line.scope != Some(SourceScope::Layers) {
        return;
    }
    let tokens = &line.structural_token_spans;
    if tokens
        .first()
        .is_some_and(|token| matches!(token.text.as_str(), "for" | "}"))
    {
        return;
    }
    let selector_start = usize::from(tokens.first().is_some_and(|token| token.text == "each"));
    if selector_start == 1
        && let Some(each) = tokens.first()
    {
        sink.mark(
            SourceSpan {
                start: each.start,
                end: each.end,
            },
            SurfaceSemanticKind::Keyword,
        );
    }
    let selector_tokens = if let Some(separator) = tokens.iter().position(|token| token.text == "=")
    {
        &tokens[separator + 1..]
    } else {
        &tokens[selector_start..]
    };
    for token in selector_tokens {
        record_resolved_object_selector_surface_token(token, &line.content, catalog, sink);
    }
}

fn record_parser_resolved_group_surface_tokens(
    line: &SurfaceScanLine,
    catalog: &Catalog,
    sink: &mut SurfaceSink,
) {
    if line.scope != Some(SourceScope::Group) {
        return;
    }
    let tokens = &line.structural_token_spans;
    let Some(separator) = tokens.iter().position(|token| token.text == "=") else {
        return;
    };
    for token in &tokens[separator + 1..] {
        record_resolved_object_selector_surface_token(token, &line.content, catalog, sink);
    }
}

fn record_parser_resolved_legend_surface_tokens(
    line: &SurfaceScanLine,
    catalog: &Catalog,
    sink: &mut SurfaceSink,
) {
    let tokens = &line.structural_token_spans;
    let selector_start = match line.scope {
        Some(SourceScope::Legend) => tokens
            .iter()
            .position(|token| token.text == "=")
            .map(|separator| separator + 1),
        Some(SourceScope::Level | SourceScope::UnbracedLevel)
            if tokens.first().is_some_and(|token| token.text == "legend") =>
        {
            tokens
                .iter()
                .position(|token| token.text == "=")
                .map(|separator| separator + 1)
        }
        _ => None,
    };
    let Some(selector_start) = selector_start else {
        return;
    };
    for token in &tokens[selector_start..] {
        record_resolved_object_selector_surface_token(token, &line.content, catalog, sink);
    }
}

fn record_parser_resolved_map_surface_tokens(
    line: &SurfaceScanLine,
    axis: Option<&str>,
    value_sets: &HashMap<String, Vec<String>>,
    sink: &mut SurfaceSink,
) {
    if line.scope != Some(SourceScope::Map) {
        return;
    }
    let Some(axis) = axis else {
        return;
    };
    let Some(values) = value_sets.get(axis) else {
        return;
    };
    let tokens = &line.structural_token_spans;
    let [from, arrow, to] = tokens.as_slice() else {
        return;
    };
    if arrow.text != "->" {
        return;
    }
    if values.iter().any(|value| value == &from.text) {
        add_surface_symbol(sink, from, SurfaceSemanticKind::Variant);
    }
    if values.iter().any(|value| value == &to.text) {
        add_surface_symbol(sink, to, SurfaceSemanticKind::Variant);
    }
}

fn record_parser_resolved_rule_surface_tokens(
    line: &SurfaceScanLine,
    catalog: &Catalog,
    sink: &mut SurfaceSink,
) {
    if line.scope != Some(SourceScope::Other)
        || puzzle_authoring::rule_line_surface_spans(strip_line_comment(&line.content)).is_err()
    {
        return;
    }
    let mut bracket_depth = 0usize;
    for token in &line.token_spans {
        let opens = token.text.chars().filter(|ch| *ch == '[').count();
        let closes = token.text.chars().filter(|ch| *ch == ']').count();
        bracket_depth = bracket_depth.saturating_add(opens);
        if bracket_depth > 0 {
            record_resolved_object_selector_surface_token(token, &line.content, catalog, sink);
        }
        bracket_depth = bracket_depth.saturating_sub(closes);
    }
}

fn parse_tags_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => {}
            [name, "=", values @ ..] => {
                parse_tag_set_directive(name, values, line, catalog)?;
            }
            _ => {
                return Err(parse_error(line, "tag row must be: <name> = <value...>"));
            }
        }
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
        let expanded =
            expand_numeric_ranges_in_value_list(values, &catalog.numeric_variable_defaults, line)?;
        let value_type = infer_tag_value_type(&expanded, line)?;
        (
            normalize_tag_values(expanded, value_type, line)?,
            value_type,
        )
    };
    if expanded_values.is_empty() {
        return Err(parse_error(line, "tag set must have at least one value"));
    }
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
    matches!(name, "directions" | "horizontal" | "vertical" | "layers")
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
    visual: bool,
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
    mark_visual_objects(&declared, visual, catalog);
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
            ["each", selectors @ ..] => {
                assign_selectors_to_separate_layers(
                    selectors,
                    &lines[i],
                    named_layers,
                    layer_count,
                    catalog,
                    false,
                )?;
            }
            [name, ..]
                if crate::syntax::named_selector_assignment_syntax(&tokens, true).is_some() =>
            {
                let syntax = crate::syntax::named_selector_assignment_syntax(&tokens, true)
                    .expect("guarded named layer assignment syntax");
                let selectors = &tokens[syntax.rhs_start..];
                let layer = layer_id_for_name(name, &lines[i], named_layers, layer_count, catalog)?;
                define_or_assign_terms_to_layer(selectors, &lines[i], layer, catalog, false)?;
                register_layer_tag_from_layer(name, layer, catalog);
            }
            _ => {
                assign_selectors_to_anonymous_layer(
                    &tokens,
                    &lines[i],
                    named_layers,
                    layer_count,
                    catalog,
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
            ["each", selectors @ ..] => {
                collect_layer_terms(selectors, &lines[i], pending_groups, terms, used_groups)?;
            }
            [_, ..] if crate::syntax::named_selector_assignment_syntax(&tokens, true).is_some() => {
                let syntax = crate::syntax::named_selector_assignment_syntax(&tokens, true)
                    .expect("guarded named layer assignment syntax");
                collect_layer_terms(
                    &tokens[syntax.rhs_start..],
                    &lines[i],
                    pending_groups,
                    terms,
                    used_groups,
                )?;
            }
            _ => {
                collect_layer_terms(&tokens, &lines[i], pending_groups, terms, used_groups)?;
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

fn collect_layer_terms(
    selectors: &[&str],
    line: &str,
    pending_groups: &[PendingGroupDefinition],
    terms: &mut Vec<String>,
    used_groups: &mut Vec<String>,
) -> Result<(), DiagnosticReport> {
    let mut resolving = Vec::<String>::new();
    for selector in selectors {
        collect_layer_term(
            selector,
            line,
            pending_groups,
            terms,
            used_groups,
            &mut resolving,
        )?;
    }
    Ok(())
}

fn collect_layer_term(
    selector: &str,
    line: &str,
    pending_groups: &[PendingGroupDefinition],
    terms: &mut Vec<String>,
    used_groups: &mut Vec<String>,
    resolving: &mut Vec<String>,
) -> Result<(), DiagnosticReport> {
    if let Some(group) = pending_group_definition(selector, pending_groups) {
        if resolving.iter().any(|candidate| candidate == selector) {
            return Err(parse_error(line, "group definitions cannot be cyclic"));
        }
        if !used_groups.contains(&group.name) {
            used_groups.push(group.name.clone());
        }
        resolving.push(group.name.clone());
        for group_selector in &group.selectors {
            collect_layer_term(
                group_selector,
                &group.line,
                pending_groups,
                terms,
                used_groups,
                resolving,
            )?;
        }
        resolving.pop();
        return Ok(());
    }
    terms.push(selector.to_string());
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
    visual: bool,
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
        let declared = parse_layer_term(term, line, layer, visual, catalog)?;
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
    visual: bool,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    if selectors.is_empty() {
        return Err(parse_error(
            line,
            "each layer row must name at least one selector",
        ));
    }
    let selector_sets = selectors
        .iter()
        .map(|selector| resolve_or_declare_layer_selector(selector, line, visual, catalog))
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
    visual: bool,
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
    mark_visual_objects(&declared, visual, catalog);
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
    define_or_assign_terms_to_layer(selectors, line, layer, catalog, false)
}

fn push_unique_object(objects: &mut Vec<ObjectId>, object: ObjectId) {
    if !objects.contains(&object) {
        objects.push(object);
    }
}

fn mark_visual_objects(objects: &[ObjectId], visual: bool, catalog: &mut Catalog) {
    if !visual {
        return;
    }
    for object in objects {
        push_unique_object(&mut catalog.visual_objects, *object);
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
    object_names.contains_key(name)
        || object_schemas.contains_key(name)
        || object_groups.contains_key(name)
        || name.split_once(':').is_some_and(|(base, _)| {
            object_names.contains_key(base)
                || object_schemas.contains_key(base)
                || object_groups.contains_key(base)
        })
}

fn parse_legend_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
    render_overlays: &mut OverlayDefs,
    empty_char: &mut Option<char>,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        parse_legend_block_row(&lines[i], catalog, render_overlays, empty_char)?;
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "legend missing closing brace"));
    }

    Ok(i + 1)
}

fn parse_legend_block_row(
    line: &str,
    catalog: &mut Catalog,
    render_overlays: &mut OverlayDefs,
    empty_char: &mut Option<char>,
) -> Result<(), DiagnosticReport> {
    let tokens = split_header_tokens(line);
    let Some(syntax) = crate::syntax::legend_block_row_syntax(&tokens, true) else {
        return Err(parse_error(
            line,
            "legend row must be: <char> = <empty | selector...>",
        ));
    };

    let ch = parse_char(tokens.first(), line, "missing legend char")?;
    if tokens[syntax.rhs_start..] == ["empty"] {
        *empty_char = Some(ch);
        return Ok(());
    }

    let mut directive_tokens = vec!["legend"];
    directive_tokens.extend(tokens);
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
    display_statements: Option<&[StatementAst]>,
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
        display_statements,
    ]
    .into_iter()
    .flatten()
    {
        collect_implicit_inputs_from_statements(statements, &mut names);
    }
    for level in level_bodies {
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
            StatementAst::Call { .. }
            | StatementAst::Effect { .. }
            | StatementAst::Rewrite(_) => {}
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
