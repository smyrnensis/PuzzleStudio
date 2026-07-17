fn scene_entry_is_component(tokens: &[&str]) -> bool {
    let Some(kind) = tokens
        .first()
        .and_then(|keyword| puzzle_scene::SceneComponentKind::from_keyword(keyword))
    else {
        return false;
    };
    match kind {
        puzzle_scene::SceneComponentKind::Button
        | puzzle_scene::SceneComponentKind::Choice
        | puzzle_scene::SceneComponentKind::Text
        | puzzle_scene::SceneComponentKind::Row
        | puzzle_scene::SceneComponentKind::Column
        | puzzle_scene::SceneComponentKind::Box
        | puzzle_scene::SceneComponentKind::Conditional
        | puzzle_scene::SceneComponentKind::For => true,
        puzzle_scene::SceneComponentKind::LevelMenu => true,
        puzzle_scene::SceneComponentKind::Viewport | puzzle_scene::SceneComponentKind::Frame => {
            tokens.len() >= 2
        }
    }
}

#[derive(Clone, Copy)]
enum AuthoringEntryOwner {
    SceneLayoutCondition,
    SceneCondition,
    SceneLifecycle,
    SceneRoutine,
}

impl AuthoringEntryOwner {
    fn missing_close_message(self) -> &'static str {
        match self {
            AuthoringEntryOwner::SceneLayoutCondition => {
                "layout condition block missing closing brace"
            }
            AuthoringEntryOwner::SceneCondition => "condition block missing closing brace",
            AuthoringEntryOwner::SceneLifecycle => "scene lifecycle block missing closing brace",
            AuthoringEntryOwner::SceneRoutine => "scene routine block missing closing brace",
        }
    }
}

fn collect_authoring_entry(
    lines: &[source::LogicalLine],
    start: usize,
    owner: AuthoringEntryOwner,
) -> Result<(Vec<source::LogicalLine>, usize), DiagnosticReport> {
    let first = &lines[start];
    let mut depth = first.structural_brace_delta();
    if depth <= 0 {
        return Ok((vec![first.clone()], start + 1));
    }

    let mut entry = vec![first.clone()];
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let next_depth = depth + line.structural_brace_delta();
        if next_depth < 0 {
            return Err(parse_error(line, "closing brace without block"));
        }
        entry.push(line.clone());
        i += 1;
        if next_depth == 0 {
            return Ok((entry, i));
        }
        depth = next_depth;
    }
    Err(parse_error(first, owner.missing_close_message()))
}

fn next_line_is_else(lines: &[source::LogicalLine], index: usize) -> bool {
    lines
        .get(index)
        .is_some_and(|line| matches!(split_header_tokens(line).as_slice(), ["else"]))
}

fn collect_braced_body_until_close(
    lines: &[source::LogicalLine],
    start: usize,
    header_line: &str,
    missing_close_message: &str,
) -> Result<(Vec<source::LogicalLine>, usize), DiagnosticReport> {
    let mut body = Vec::new();
    let mut depth = 1i32;
    let mut i = start;
    while i < lines.len() {
        let line = &lines[i];
        let next_depth = depth + line.structural_brace_delta();
        if next_depth < 0 {
            return Err(parse_error(line, "closing brace without block"));
        }
        if next_depth == 0 {
            return Ok((body, i + 1));
        }
        body.push(line.clone());
        depth = next_depth;
        i += 1;
    }
    Err(parse_error(header_line, missing_close_message))
}

fn lower_model_statement_block(
    syntax: &model_syntax::RuleStatementsSyntax,
    catalog: &Catalog,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
) -> Result<(Vec<StatementAst>, Option<LocalFrame<ObjectId>>), DiagnosticReport> {
    let modifier = syntax.modifier.as_deref().unwrap_or("");
    let modifier_tokens = split_header_tokens(modifier);
    let local_frame =
        parse_program_local_frame_modifier(&modifier_tokens, &syntax.header, catalog)?;
    let statements = lower_statement_syntax(
        &syntax.statements,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.maps,
        &catalog.object_groups,
        &catalog.input_names,
        &catalog.variable_names,
        &catalog.numeric_variable_defaults,
        &catalog.condition_names,
        named_conditions,
        &[],
    )?;
    Ok((statements, local_frame))
}

#[allow(clippy::too_many_arguments)]
fn lower_puzzle_model(
    model: &model_syntax::PuzzleModelSyntax,
    catalog_product: &Catalog,
    layer_count: &mut Option<u16>,
    empty_char: &mut Option<char>,
    named_layers: &mut HashMap<String, u16>,
    catalog: &mut Catalog,
    query_definitions: &mut Vec<QueryDefinitionAst>,
    query_names: &mut HashSet<String>,
    condition_definitions: &mut Vec<ConditionDefinitionAst>,
    controls: &mut Controls,
    directions: &mut Vec<DirectionalInput>,
    rule_definitions: &mut Vec<RuleDefinitionAst>,
    main_statements: &mut Option<Vec<StatementAst>>,
    main_local_frame: &mut Option<LocalFrame<ObjectId>>,
    level_start_statements: &mut Option<Vec<StatementAst>>,
    level_start_local_frame: &mut Option<LocalFrame<ObjectId>>,
    level_clear_statements: &mut Option<Vec<StatementAst>>,
    level_clear_local_frame: &mut Option<LocalFrame<ObjectId>>,
    last_level_clear_statements: &mut Option<Vec<StatementAst>>,
    last_level_clear_local_frame: &mut Option<LocalFrame<ObjectId>>,
    render_overlays: &mut OverlayDefs,
    model_sound_triggers: &mut Vec<ModelSoundTriggerSpec>,
    model_operation_sounds: &mut Vec<ModelOperationSoundSpec>,
    solver_strategy: &mut Option<SolverStrategyAst>,
    named_conditions: &mut HashMap<String, (String, ConditionAst)>,
    run_rules_on_level_start: &mut bool,
    visuals: &mut VisualsDef,
    render: &mut PuzzleRenderDef,
    animation: &mut AnimationDef,
    puzzle_screen: &mut PuzzleScreenDef,
    level_blocks: &mut Vec<LevelBlock>,
    recognition: &mut crate::surface::ParserRecognition,
) -> Result<String, DiagnosticReport> {
    let model_source = source::LogicalLine::new(&model.source_line, model.source_line_number);
    validate_qualified_identifier(&model.name, &model_source, "puzzle name")?;
    *catalog = catalog_product.clone();
    *layer_count = catalog.layer_count;
    *named_layers = catalog.named_layers.clone();
    for legend in &model.body.levels.legends {
        apply_level_resource_legend(legend, catalog, render_overlays, empty_char, recognition)?;
    }
    level_blocks.extend(model.body.levels.levels.iter().cloned());
    lower_model_key_bindings(&model.body.keys, catalog, controls)?;

    for declaration in &model.body.variables {
        if catalog.variable_names.contains_key(&declaration.name) {
            return Err(parse_error(&declaration.source, "duplicate var or const"));
        }
        let id = VariableId(catalog.variable_defaults.len() as u16);
        catalog.variable_names.insert(declaration.name.clone(), id);
        catalog.variable_labels.insert(id, declaration.name.clone());
        catalog.variable_defaults.push(declaration.default);
        if declaration.numeric {
            catalog
                .numeric_variable_defaults
                .insert(declaration.name.clone(), declaration.default);
        }
        if declaration.persistent {
            catalog.persistent_vars.push(id);
        }
        if declaration.constant {
            catalog.constant_variables.push(id);
        }
    }
    for syntax in &model.body.named_conditions {
        if named_conditions.contains_key(&syntax.name) {
            return Err(parse_error(&syntax.source, "duplicate condition"));
        }
        let condition = parse_condition_expr(
            &syntax.expression,
            &syntax.source,
            &catalog.input_names,
            &catalog.variable_names,
            &catalog.condition_names,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
        )?;
        named_conditions.insert(
            syntax.name.clone(),
            (syntax.expression.clone(), condition),
        );
    }
    for syntax in &model.body.inputs {
        let input = catalog
            .input_names
            .get(&syntax.name)
            .copied()
            .map(Ok)
            .unwrap_or_else(|| add_input_name(&syntax.name, &syntax.source, catalog))?;
        if let Some(direction) = &syntax.direction {
            validate_direction_name(direction, catalog, &syntax.source)?;
            directions.push(DirectionalInput {
                input,
                direction: direction.clone(),
            });
        }
    }
    for syntax in &model.body.direction_aliases {
        add_direction_alias(
            &syntax.alias,
            &syntax.canonical,
            &syntax.source,
            catalog,
        )?;
    }
    if let Some(character) = model.body.empty_char {
        *empty_char = Some(character);
    }
    *run_rules_on_level_start |= model.body.run_rules_on_level_start;
    *puzzle_screen = model.body.screen.clone();
    model_sound_triggers.extend(model.body.sounds.triggers.iter().cloned());
    model_operation_sounds.extend(model.body.sounds.operations.iter().cloned());

    let mut diagnostics = Vec::new();
    for query in &model.body.queries {
        let (query, core_definition) = lower_query_definition_syntax(
            &query.definition,
            &query.source,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
            &catalog.variable_names,
            query_names,
            &mut catalog.condition_names,
            &mut catalog.condition_labels,
        )?;
        query_definitions.push(query);
        if let Some(definition) = core_definition {
            condition_definitions.push(definition);
        }
    }
    if let Some(win_conditions) = &model.body.win_conditions {
        lower_condition_block_syntax_rows(
            &win_conditions.header,
            &win_conditions.rows,
            catalog,
            named_conditions,
        )?;
    }
    if let Some(lose_conditions) = &model.body.lose_conditions {
        lower_condition_block_syntax_rows(
            &lose_conditions.header,
            &lose_conditions.rows,
            catalog,
            named_conditions,
        )?;
    }
    for syntax in &model.body.render_overlays {
        let (overlays, level_objects) = lower_render_overlay_syntax(
            syntax,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
        )?;
        render_overlays.extend(overlays);
        if let Some(objects) = level_objects {
            catalog.char_objects.insert(syntax.character, objects);
        }
    }
    if let Some(product) = &model.body.render {
        *render = product.render.clone();
        *animation = product.animation.clone();
    }
    for syntax in &model.body.routines {
        match lower_rule_definition_syntax(
            &syntax.statement,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
            &catalog.input_names,
            &catalog.variable_names,
            &catalog.numeric_variable_defaults,
            &catalog.condition_names,
            named_conditions,
        ) {
            Ok(definition) => rule_definitions.push(definition),
            Err(report) => diagnostics.extend(report.into_diagnostics()),
        }
    }
    if let Some(syntax) = &model.body.rules {
        let (statements, local_frame) =
            lower_model_statement_block(syntax, catalog, named_conditions)?;
        *main_statements = Some(statements);
        *main_local_frame = local_frame;
    }
    if let Some(syntax) = &model.body.on_level_start {
        let (statements, local_frame) =
            lower_model_statement_block(syntax, catalog, named_conditions)?;
        *level_start_statements = Some(statements);
        *level_start_local_frame = local_frame;
    }
    if let Some(syntax) = &model.body.on_level_clear {
        let (statements, local_frame) =
            lower_model_statement_block(syntax, catalog, named_conditions)?;
        *level_clear_statements = Some(statements);
        *level_clear_local_frame = local_frame;
    }
    if let Some(syntax) = &model.body.on_last_level_clear {
        let (statements, local_frame) =
            lower_model_statement_block(syntax, catalog, named_conditions)?;
        *last_level_clear_statements = Some(statements);
        *last_level_clear_local_frame = local_frame;
    }

    *solver_strategy = model.body.solver.clone();
    for entry in &model.body.sprite_resources {
        let parsed = parse_visuals_entry(entry, catalog, visuals);
        recognition.merge(parsed.recognition);
        if let Err(report) = parsed.value {
            diagnostics.extend(report.into_diagnostics());
        }
    }
    if !diagnostics.is_empty() {
        return Err(DiagnosticReport::from_diagnostics(diagnostics));
    }
    Ok(model.name.clone())
}

fn parse_program_local_frame_modifier(
    tokens: &[&str],
    line: &str,
    catalog: &Catalog,
) -> Result<Option<LocalFrame<ObjectId>>, DiagnosticReport> {
    if tokens.is_empty() {
        return Ok(None);
    }
    let focus_objects = default_local_frame_focus_objects(catalog, line)?;
    match tokens {
        ["local_radius", radius] => {
            let radius = parse_u16(Some(radius), line, "missing local radius")?;
            Ok(Some(LocalFrame::new(
                LocalFrameExtent::Radius(radius),
                LocalFrameExtent::Radius(radius),
                LocalFrameExtent::Full,
                focus_objects,
            )))
        }
        ["local_frame", x, y] => Ok(Some(LocalFrame::new(
            parse_local_frame_extent(x, line)?,
            parse_local_frame_extent(y, line)?,
            LocalFrameExtent::Full,
            focus_objects,
        ))),
        _ => Err(parse_error(
            line,
            "transition block header must be: rules [local_radius <n> | local_frame <x> <y>] | on_level_start [local_radius <n> | local_frame <x> <y>] | on_level_clear [local_radius <n> | local_frame <x> <y>]",
        )),
    }
}

fn parse_local_frame_extent(token: &str, line: &str) -> Result<LocalFrameExtent, DiagnosticReport> {
    if token == "full" {
        return Ok(LocalFrameExtent::Full);
    }
    parse_u16(Some(&token), line, "missing local frame extent").map(LocalFrameExtent::Radius)
}

fn default_local_frame_focus_objects(
    catalog: &Catalog,
    line: &str,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    for name in ["Player", "player"] {
        if let Some(object) = catalog.object_names.get(name) {
            return Ok(vec![*object]);
        }
    }
    Err(parse_error(
        line,
        "local_frame/local_radius requires an object named Player",
    ))
}
