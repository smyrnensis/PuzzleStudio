fn parse_group_definition(
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &mut HashMap<String, Vec<ObjectId>>,
) -> Result<(), DiagnosticReport> {
    let Some(assignment) = puzzle_authoring::selector_assignment_surface(line) else {
        return Err(parse_error(
            line,
            "group row must be: <name> = <selector...>",
        ));
    };

    let name = assignment.name;
    validate_selector_alias_name(name, line, "group name")?;
    if selector_name_conflicts_with(name, object_names, object_schemas, object_groups) {
        return Err(parse_error(
            line,
            "group name must not shadow another selector",
        ));
    }

    let selector_sets = selector_sets(
        &assignment.selectors,
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )?;
    let mut objects = Vec::new();
    for selector_set in selector_sets {
        for object in selector_set {
            if !objects.contains(&object) {
                objects.push(object);
            }
        }
    }
    if objects.is_empty() {
        return Err(parse_error(line, "group must contain at least one object"));
    }

    object_groups.insert(name.to_string(), objects);
    Ok(())
}

type PendingGroupDefinition = puzzle_authoring::SelectorGroupDeclaration;

fn collect_puzzle_group_declarations(
    lines: &[String],
    start: usize,
) -> Result<Vec<PendingGroupDefinition>, DiagnosticReport> {
    let mut groups = Vec::new();
    let mut names = HashSet::<String>::new();
    let mut i = start;
    let mut depth = 1i32;
    while i < lines.len() && depth > 0 {
        let tokens = split_header_tokens(&lines[i]);
        if depth == 1 {
            match tokens.as_slice() {
                ["groups"] => {
                    i = collect_pending_group_block(lines, i, &mut groups, &mut names)?;
                    continue;
                }
                ["groups", ..] => {
                    return Err(parse_error(
                        &lines[i],
                        "groups block must be: groups { ... }",
                    ));
                }
                _ => {}
            }
        }
        depth += raw_brace_delta(&lines[i]);
        i += 1;
    }
    Ok(groups)
}

fn collect_pending_group_block(
    lines: &[String],
    start: usize,
    groups: &mut Vec<PendingGroupDefinition>,
    names: &mut HashSet<String>,
) -> Result<usize, DiagnosticReport> {
    let block = puzzle_authoring::collect_row_block_surface(lines, start + 1, "groups")
        .map_err(|error| parse_error(&lines[start], error.message()))?;
    for line in block.rows {
        let Some(assignment) = puzzle_authoring::selector_assignment_surface(line) else {
            return Err(parse_error(
                line,
                "group row must be: <name> = <selector...>",
            ));
        };
        let name = assignment.name;
        validate_selector_alias_name(name, line, "group name")?;
        if !names.insert(name.to_string()) {
            return Err(parse_error(line, "duplicate group"));
        }
        groups.push(PendingGroupDefinition {
            name: name.to_string(),
            selectors: assignment
                .selectors
                .iter()
                .map(|selector| (*selector).to_string())
                .collect(),
            source_line: line.to_string(),
        });
    }
    Ok(block.next_index)
}
fn resolve_pending_group_definitions(
    pending_groups: &[PendingGroupDefinition],
    only_names: Option<&[String]>,
    resolved_groups: &mut HashSet<String>,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    let names = only_names.map(|names| names.to_vec()).unwrap_or_else(|| {
        pending_groups
            .iter()
            .map(|group| group.name.clone())
            .collect()
    });
    let mut resolving = Vec::<String>::new();
    for name in names {
        resolve_pending_group_definition(
            &name,
            pending_groups,
            resolved_groups,
            &mut resolving,
            catalog,
        )?;
    }
    Ok(())
}

fn resolve_pending_group_definition(
    name: &str,
    pending_groups: &[PendingGroupDefinition],
    resolved_groups: &mut HashSet<String>,
    resolving: &mut Vec<String>,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    if resolved_groups.contains(name) {
        return Ok(());
    }
    let Some(group) = pending_group_definition(name, pending_groups) else {
        return Ok(());
    };
    if catalog.object_groups.contains_key(name) {
        return Err(parse_error(
            &group.source_line,
            "group name must not shadow another selector",
        ));
    }
    if resolving.iter().any(|candidate| candidate == name) {
        return Err(parse_error(
            &group.source_line,
            "group definitions cannot be cyclic",
        ));
    }
    resolving.push(name.to_string());
    for selector in &group.selectors {
        if pending_group_definition(selector, pending_groups).is_some() {
            resolve_pending_group_definition(
                selector,
                pending_groups,
                resolved_groups,
                resolving,
                catalog,
            )?;
        }
    }
    resolving.pop();

    parse_group_definition(
        &group.source_line,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.maps,
        &mut catalog.object_groups,
    )?;
    resolved_groups.insert(name.to_string());
    Ok(())
}

fn pending_group_definition<'a>(
    name: &str,
    pending_groups: &'a [PendingGroupDefinition],
) -> Option<&'a PendingGroupDefinition> {
    pending_groups.iter().find(|group| group.name == name)
}

fn parse_legend_directive(
    tokens: &[&str],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    render_chars: &mut HashMap<ObjectId, char>,
    char_objects: &mut HashMap<char, Vec<ObjectId>>,
    render_overlays: &mut OverlayDefs,
) -> Result<(), DiagnosticReport> {
    let Some(syntax) = crate::syntax::legend_directive_syntax(tokens, true) else {
        return Err(parse_error(
            line,
            "legend must be: legend <char> = <selector...>",
        ));
    };

    let ch = parse_char(tokens.get(1), line, "missing legend char")?;
    if ch == '.' {
        return Err(parse_error(
            line,
            "levels reserve `.` for empty; use another legend char for objects",
        ));
    }
    let selector_sets = selector_sets(
        &tokens[syntax.rhs_start..],
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )?;
    let combinations = cartesian_object_product(&selector_sets);

    if selector_sets.len() == 1 {
        for object in &selector_sets[0] {
            render_chars.insert(*object, ch);
        }
        if selector_sets[0].len() == 1 {
            char_objects.insert(ch, vec![selector_sets[0][0]]);
        }
        return Ok(());
    }

    for objects in &combinations {
        render_overlays.push((objects.clone(), ch));
    }
    if combinations.len() == 1 {
        char_objects.insert(ch, combinations[0].clone());
    }

    Ok(())
}

fn parse_render_overlay(
    tokens: &[&str],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<(OverlayDefs, Option<Vec<ObjectId>>, char), DiagnosticReport> {
    if tokens.len() < 4 {
        return Err(parse_error(
            line,
            "render_overlay must be: render_overlay <object> <object> [object...] <char>",
        ));
    }

    let ch = parse_char(tokens.last(), line, "missing overlay char")?;
    let selector_sets = selector_sets(
        &tokens[1..tokens.len() - 1],
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )?;
    let combinations = cartesian_object_product(&selector_sets);
    let overlays = combinations
        .iter()
        .map(|objects| (objects.clone(), ch))
        .collect::<Vec<_>>();
    let level_objects = (combinations.len() == 1).then(|| combinations[0].clone());

    Ok((overlays, level_objects, ch))
}

fn selector_sets(
    selectors: &[&str],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<Vec<Vec<ObjectId>>, DiagnosticReport> {
    selectors
        .iter()
        .map(|selector| {
            resolve_object_selector(
                selector,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                &HashMap::new(),
            )
            .map(|selector| selector.alternatives)
        })
        .collect()
}

fn cartesian_object_product(sets: &[Vec<ObjectId>]) -> Vec<Vec<ObjectId>> {
    let mut combinations = vec![Vec::<ObjectId>::new()];
    for set in sets {
        let mut next = Vec::new();
        for prefix in &combinations {
            for object in set {
                let mut combination = prefix.clone();
                combination.push(*object);
                next.push(combination);
            }
        }
        combinations = next;
    }
    combinations
}

fn parse_direction_directive(
    tokens: &[&str],
    line: &str,
    catalog: &mut Catalog,
) -> Result<Option<Direction>, DiagnosticReport> {
    match tokens {
        ["direction", alias, canonical] => {
            add_direction_alias(alias, canonical, line, catalog)?;
            Ok(None)
        }
        _ => Err(parse_error(
            line,
            "direction must be: direction <alias> <up|down|left|right>",
        )),
    }
}

fn add_direction_alias(
    alias: &str,
    canonical: &str,
    line: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    validate_identifier(alias, line, "direction alias")?;
    named_direction_vector(canonical, line)?;
    let canonical_input = catalog
        .input_names
        .get(canonical)
        .copied()
        .map(Ok)
        .unwrap_or_else(|| add_input_name(canonical, line, catalog))?;
    if let Some(existing) = catalog.input_names.get(alias).copied() {
        if existing != canonical_input {
            return Err(parse_error(
                line,
                "direction alias must not redefine an existing input",
            ));
        }
        return Ok(());
    }
    catalog
        .input_names
        .insert(alias.to_string(), canonical_input);
    Ok(())
}

fn parse_variable_directive(
    tokens: &[&str],
    line: &str,
    variable_names: &mut HashMap<String, VariableId>,
    variable_labels: &mut HashMap<VariableId, String>,
    variable_defaults: &mut Vec<i64>,
    numeric_variable_defaults: &mut HashMap<String, i64>,
    persistent_vars: &mut Vec<VariableId>,
    constant_variables: &mut Vec<VariableId>,
) -> Result<(), DiagnosticReport> {
    let parsed = match tokens {
        ["var", name, "=", value] => Some((*name, *value, false, false)),
        ["const", name, "=", value] => Some((*name, *value, false, true)),
        ["persistent", "var", name, "=", value] => Some((*name, *value, true, false)),
        ["persistent", "const", name, "=", value] => Some((*name, *value, true, true)),
        _ => None,
    };
    match parsed {
        Some((name, value, persistent, constant)) => {
            if !is_identifier(name) {
                return Err(parse_error(line, "var or const name must be an identifier"));
            }
            if variable_names.contains_key(name) {
                return Err(parse_error(line, "duplicate var or const"));
            }
            let id = VariableId(variable_defaults.len() as u16);
            let default = parse_variable_value(value, line)?;
            variable_names.insert(name.to_string(), id);
            variable_labels.insert(id, name.to_string());
            variable_defaults.push(default);
            if value.parse::<i64>().is_ok() {
                numeric_variable_defaults.insert(name.to_string(), default);
            }
            if persistent {
                persistent_vars.push(id);
            }
            if constant {
                constant_variables.push(id);
            }
            Ok(())
        }
        _ => Err(parse_error(
            line,
            "var or const must be: var <name> = <true | false | number> or const <name> = <true | false | number>",
        )),
    }
}

fn parse_query_directive(
    _tokens: &[&str],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
    query_names: &mut HashSet<String>,
    condition_names: &mut HashMap<String, ConditionId>,
    condition_labels: &mut HashMap<ConditionId, String>,
) -> Result<(QueryDefinitionAst, Option<ConditionDefinitionAst>), DiagnosticReport> {
    let surface = crate::solver_surface::parse_query_definition(line)?;
    let name = surface.name.as_str();
    if query_names.contains(name) || variable_names.contains_key(name) {
        return Err(parse_error(line, "duplicate query"));
    }
    query_names.insert(name.to_string());
    let core_definition = match &surface.expr {
        crate::solver_surface::SolverSurfaceQueryExpr::Call { name: call, args }
            if call != "distance" =>
        {
            let kind = parse_condition_value_surface_call(
                call,
                args,
                &surface.source_line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
            )?;
            let id = ConditionId(condition_names.len() as u16);
            condition_names.insert(name.to_string(), id);
            condition_labels.insert(id, name.to_string());
            Some(ConditionDefinitionAst { id, kind })
        }
        _ => None,
    };
    Ok((surface, core_definition))
}

fn parse_solver_block(
    lines: &[String],
    start: usize,
) -> Result<(usize, SolverStrategyAst), DiagnosticReport> {
    crate::solver_surface::parse_solver_block(lines, start)
}

fn parse_condition_value_expr(
    expr: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionValueAst, DiagnosticReport> {
    let (name, arg) = parse_call_expr(expr, line)?;
    lower_condition_value_call_2d(
        name,
        arg,
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )
}

fn parse_condition_value_surface_call(
    name: &str,
    args: &[SolverSurfaceQueryArg],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionValueAst, DiagnosticReport> {
    let [arg] = args else {
        return Err(parse_error(
            line,
            "query expression must have exactly one argument",
        ));
    };
    match arg {
        SolverSurfaceQueryArg::Selector(selector) => lower_condition_value_selector_call_2d(
            name,
            selector,
            line,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
        ),
        SolverSurfaceQueryArg::Pattern(pattern) => lower_condition_value_pattern_call_2d(
            name,
            pattern,
            line,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
        ),
    }
}

fn lower_condition_value_selector_call_2d(
    name: &str,
    selector: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionValueAst, DiagnosticReport> {
    let objects = resolve_object_selector(
        selector,
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        &HashMap::new(),
    )?
    .alternatives;
    match name {
        "count" => Ok(ConditionValueAst::CountObjects(objects)),
        "exists" | "some" => Ok(ConditionValueAst::ExistsObjects(objects)),
        "none" | "no" => Ok(ConditionValueAst::NoneObjects(objects)),
        _ => Err(parse_error(line, "unknown query function")),
    }
}

fn lower_condition_value_pattern_call_2d(
    name: &str,
    pattern: &SolverSurfacePatternArg,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionValueAst, DiagnosticReport> {
    let pattern = parse_condition_pattern_surface_arg(
        pattern,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )?;
    match name {
        "count" => Ok(ConditionValueAst::CountMatches(pattern)),
        "exists" | "some" => Ok(ConditionValueAst::ExistsMatches(pattern)),
        "none" | "no" => Ok(ConditionValueAst::NoneMatches(pattern)),
        _ => Err(parse_error(line, "unknown query function")),
    }
}

fn parse_condition_pattern_surface_arg(
    pattern: &SolverSurfacePatternArg,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionPatternAst, DiagnosticReport> {
    let (orientation, pattern_source) = match &pattern.orientation {
        crate::solver_surface::SolverSurfacePatternOrientation::Neutral => {
            (OrientationExpr::Neutral, pattern.pattern.clone())
        }
        crate::solver_surface::SolverSurfacePatternOrientation::Input { axis } => {
            let axis = axis.as_deref().unwrap_or("directions");
            (
                OrientationExpr::InputSet(axis.to_string()),
                pattern.pattern.clone(),
            )
        }
        crate::solver_surface::SolverSurfacePatternOrientation::Orientation(orientation) => (
            parse_statement_orientation_expr(orientation, &[]),
            pattern.pattern.clone(),
        ),
    };
    Ok(ConditionPatternAst {
        orientation,
        pattern: parse_pattern_side(
            &pattern_source,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            &HashMap::new(),
            false,
        )?,
    })
}

fn lower_condition_value_call_2d(
    name: &str,
    arg: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionValueAst, DiagnosticReport> {
    let pattern_arg = parse_condition_pattern_arg(
        arg,
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )?;
    match name {
        "count" if pattern_arg.is_some() => Ok(ConditionValueAst::CountMatches(
            pattern_arg.expect("checked"),
        )),
        "count" => Ok(ConditionValueAst::CountObjects(
            resolve_object_selector(
                arg,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                &HashMap::new(),
            )?
            .alternatives,
        )),
        "exists" | "some" if pattern_arg.is_some() => Ok(ConditionValueAst::ExistsMatches(
            pattern_arg.expect("checked"),
        )),
        "none" | "no" if pattern_arg.is_some() => Ok(ConditionValueAst::NoneMatches(
            pattern_arg.expect("checked"),
        )),
        "exists" => Ok(ConditionValueAst::ExistsObjects(
            resolve_object_selector(
                arg,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                &HashMap::new(),
            )?
            .alternatives,
        )),
        "some" => Ok(ConditionValueAst::ExistsObjects(
            resolve_object_selector(
                arg,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                &HashMap::new(),
            )?
            .alternatives,
        )),
        "none" | "no" => Ok(ConditionValueAst::NoneObjects(
            resolve_object_selector(
                arg,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                &HashMap::new(),
            )?
            .alternatives,
        )),
        _ => Err(parse_error(line, "unknown query function")),
    }
}

fn parse_condition_pattern_arg(
    arg: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<Option<ConditionPatternAst>, DiagnosticReport> {
    let Some((orientation, pattern)) = split_oriented_pattern_arg(arg, line)? else {
        return Ok(None);
    };
    Ok(Some(ConditionPatternAst {
        orientation,
        pattern: parse_pattern_side(
            &pattern,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            &HashMap::new(),
            false,
        )?,
    }))
}

fn split_oriented_pattern_arg(
    arg: &str,
    line: &str,
) -> Result<Option<(OrientationExpr, String)>, DiagnosticReport> {
    let Some(surface) = crate::solver_surface::oriented_pattern_arg_surface(arg, line)? else {
        return Ok(None);
    };
    let pattern = &arg[surface.pattern.clone()];
    if matches!(
        surface.orientation,
        crate::solver_surface::OrientedPatternArgOrientationSurface::Neutral
    ) {
        return Ok(Some((OrientationExpr::Neutral, pattern.to_string())));
    }

    let orientation = match surface.orientation {
        crate::solver_surface::OrientedPatternArgOrientationSurface::Neutral => {
            unreachable!("handled above")
        }
        crate::solver_surface::OrientedPatternArgOrientationSurface::Input { axis, .. } => {
            let axis = axis.map(|axis| &arg[axis]).unwrap_or("directions");
            OrientationExpr::InputSet(axis.to_string())
        }
        crate::solver_surface::OrientedPatternArgOrientationSurface::Orientation {
            orientation,
        } => parse_statement_orientation_expr(&arg[orientation], &[]),
    };
    Ok(Some((orientation, pattern.to_string())))
}

fn parse_call_expr<'a>(expr: &'a str, line: &str) -> Result<(&'a str, &'a str), DiagnosticReport> {
    let (call, suffix) = require_call_surface_with_suffix(
        expr,
        line,
        "query expression must be a function call",
        "query expression missing closing )",
    )?;
    if !suffix.is_empty() {
        return Err(parse_error(
            line,
            "query expression must not have trailing text",
        ));
    }
    if !is_identifier(call.name) {
        return Err(parse_error(
            line,
            "query function name must be an identifier",
        ));
    }
    let [arg] = call.args.as_slice() else {
        return Err(parse_error(
            line,
            "query expression must have exactly one argument",
        ));
    };
    Ok((call.name, arg))
}

fn default_cardinal_directions(input_names: &HashMap<String, InputId>) -> Vec<Direction> {
    let Some(up) = input_names.get("up").copied() else {
        return Vec::new();
    };
    let Some(down) = input_names.get("down").copied() else {
        return Vec::new();
    };
    let Some(left) = input_names.get("left").copied() else {
        return Vec::new();
    };
    let Some(right) = input_names.get("right").copied() else {
        return Vec::new();
    };

    vec![
        Direction {
            input: up,
            dx: 0,
            dy: -1,
        },
        Direction {
            input: down,
            dx: 0,
            dy: 1,
        },
        Direction {
            input: left,
            dx: -1,
            dy: 0,
        },
        Direction {
            input: right,
            dx: 1,
            dy: 0,
        },
    ]
}

fn parse_rule_definition(
    lines: &[String],
    line_numbers: Option<&[usize]>,
    start: usize,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    input_names: &HashMap<String, InputId>,
    variable_names: &HashMap<String, VariableId>,
    numeric_variables: &HashMap<String, i64>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
) -> Result<(RuleDefinitionAst, usize), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let declaration = header.first().copied().unwrap_or("routine");
    let role = RuleRole::Main;
    let name_index = 1;
    let name_spec = expect(
        header.get(name_index),
        &lines[start],
        "missing routine name",
    )?;
    let (name, params) = parse_rule_name_and_params(name_spec, &lines[start])?;
    let application = parse_rule_application(&header, declaration, role, &lines[start])?;

    let (statements, next_i) = parse_statement_block(
        lines,
        line_numbers,
        start + 1,
        &[BLOCK_CLOSE],
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        input_names,
        variable_names,
        numeric_variables,
        condition_names,
        named_conditions,
        &params,
    )?;

    Ok((
        RuleDefinitionAst {
            name,
            role,
            application,
            statements,
        },
        next_i,
    ))
}
