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
        | puzzle_scene::SceneComponentKind::Title
        | puzzle_scene::SceneComponentKind::Subtitle
        | puzzle_scene::SceneComponentKind::Row
        | puzzle_scene::SceneComponentKind::Column
        | puzzle_scene::SceneComponentKind::Box
        | puzzle_scene::SceneComponentKind::Conditional
        | puzzle_scene::SceneComponentKind::For => true,
        puzzle_scene::SceneComponentKind::LevelMenu => true,
        puzzle_scene::SceneComponentKind::Frame => tokens.len() >= 2,
    }
}

fn collect_authoring_entry(
    lines: &[String],
    start: usize,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let first = &lines[start];
    let tokens = split_header_tokens(first);
    if matches!(tokens.as_slice(), ["levels", ..]) {
        return collect_levels_authoring_entry(lines, start);
    }
    if !starts_authoring_block(&tokens, first) {
        return Ok((vec![first.clone()], start + 1));
    }

    let mut entry = Vec::new();
    let mut block_stack = vec![authoring_block_kind(&tokens)];
    let mut i = start;
    while i < lines.len() {
        let line = &lines[i];
        if i != start {
            let tokens = split_header_tokens(line);
            if tokens.first().copied() == Some(BLOCK_CLOSE) {
                let closed = block_stack
                    .pop()
                    .ok_or_else(|| parse_error(line, "closing brace without block"))?;
                entry.push(line.clone());
                i += 1;
                if block_stack.is_empty() {
                    return Ok((entry, i));
                }
                if closed == AuthoringBlockKind::If && next_line_is_else(lines, i) {
                    entry.push(lines[i].clone());
                    i += 1;
                    block_stack.push(AuthoringBlockKind::Other);
                }
                continue;
            }
            if let Some(kind) = authoring_nested_block_kind(&tokens, line) {
                block_stack.push(kind);
            }
        }
        entry.push(line.clone());
        i += 1;
    }
    Err(parse_error(first, "block missing closing brace"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthoringBlockKind {
    If,
    Other,
}

fn authoring_block_kind(tokens: &[&str]) -> AuthoringBlockKind {
    if tokens.first().copied() == Some("if") {
        AuthoringBlockKind::If
    } else {
        AuthoringBlockKind::Other
    }
}

fn authoring_nested_block_kind(tokens: &[&str], line: &str) -> Option<AuthoringBlockKind> {
    let trimmed = line.trim_end();
    if starts_authoring_block(tokens, line) || trimmed.ends_with('{') || trimmed.ends_with("->") {
        Some(authoring_block_kind(tokens))
    } else {
        None
    }
}

fn next_line_is_else(lines: &[String], index: usize) -> bool {
    lines
        .get(index)
        .is_some_and(|line| matches!(split_header_tokens(line).as_slice(), ["else"]))
}

fn collect_levels_authoring_entry(
    lines: &[String],
    start: usize,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let first = &lines[start];
    let mut entry = vec![first.clone()];
    let mut depth = 1usize;

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.first().copied() == Some(BLOCK_CLOSE) {
            depth -= 1;
            entry.push(line.clone());
            if depth == 0 {
                return Ok((entry, i + 1));
            }
            i += 1;
            continue;
        }
        if depth == 1
            && (puzzle_authoring::is_braced_level_header(line)
                || matches!(tokens.as_slice(), ["{"]))
        {
            depth += 1;
        } else if !matches!(tokens.as_slice(), ["level", ..])
            && starts_authoring_block(&tokens, line)
        {
            depth += 1;
        }
        entry.push(line.clone());
        i += 1;
    }
    Err(parse_error(first, "levels block missing closing brace"))
}

fn starts_authoring_block(tokens: &[&str], line: &str) -> bool {
    match tokens {
        ["map", ..]
        | ["on_level_start"]
        | ["on_level_clear"]
        | ["on_last_level_clear"]
        | ["on_display"]
        | ["marks"]
        | ["groups"]
        | ["layers"]
        | ["win_conditions", ..]
        | ["lose_conditions", ..]
        | ["sprites"]
        | ["sounds"]
        | ["screen"]
        | ["layout", ..]
        | ["routine", ..]
        | ["rules"]
        | ["levels", ..]
        | ["resources"]
        | ["level", ..]
        | ["state"]
        | ["keys"]
        | ["on_scene_start"]
        | ["input", ..]
        | ["action", ..]
        | ["if", ..]
        | ["row", ..]
        | ["column", ..]
        | ["box", ..]
        | ["for", ..]
        | ["level_menu"] => true,
        ["legend"] => true,
        ["button", ..] if line.trim_end().ends_with(" with") => true,
        ["choice", ..] if line.trim_end().ends_with(" with") => true,
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_puzzle_definition(
    lines: &[String],
    line_numbers: &[usize],
    start: usize,
    layer_count: &mut Option<u16>,
    empty_char: &mut Option<char>,
    named_layers: &mut HashMap<String, u16>,
    catalog: &mut Catalog,
    condition_definitions: &mut Vec<ConditionDefinitionAst>,
    controls: &mut Controls,
    directions: &mut Vec<Direction>,
    rule_definitions: &mut Vec<RuleDefinitionAst>,
    main_statements: &mut Option<Vec<StatementAst>>,
    main_local_frame: &mut Option<LocalFrame<ObjectId>>,
    level_start_statements: &mut Option<Vec<StatementAst>>,
    level_start_local_frame: &mut Option<LocalFrame<ObjectId>>,
    level_clear_statements: &mut Option<Vec<StatementAst>>,
    level_clear_local_frame: &mut Option<LocalFrame<ObjectId>>,
    last_level_clear_statements: &mut Option<Vec<StatementAst>>,
    last_level_clear_local_frame: &mut Option<LocalFrame<ObjectId>>,
    display_statements: &mut Option<Vec<StatementAst>>,
    render_overlays: &mut OverlayDefs,
    model_sound_triggers: &mut Vec<ModelSoundTriggerSpec>,
    model_operation_sounds: &mut Vec<ModelOperationSoundSpec>,
    named_conditions: &mut HashMap<String, (String, ConditionAst)>,
    run_rules_on_level_start: &mut bool,
    visuals: &mut VisualsDef,
    render: &mut PuzzleRenderDef,
    animation: &mut AnimationDef,
    puzzle_screen: &mut PuzzleScreenDef,
    pending_level_blocks: &mut Vec<PendingLevelBlock>,
) -> Result<(usize, String), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let name = match header.as_slice() {
        ["puzzle", name] => *name,
        _ => {
            return Err(parse_error(
                &lines[start],
                "puzzle header must be: puzzle <name>",
            ));
        }
    };
    validate_qualified_identifier(name, &lines[start], "puzzle name")?;

    collect_puzzle_tag_declarations(lines, start + 1, catalog)?;

    let mut i = start + 1;
    let mut diagnostics = Vec::new();
    let mut pending_visual_blocks = Vec::<usize>::new();
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.is_empty() {
            i += 1;
            continue;
        }

        match tokens[0] {
            assignment_name if tokens.get(1).copied() == Some("=") => {
                parse_assignment_directive(assignment_name, line, catalog, named_conditions)?;
                i += 1;
            }
            "tags" => {
                if tokens.len() != 1 {
                    return Err(parse_error(line, "tags header must be: tags"));
                }
                i = skip_tags_block(lines, i)?;
            }
            "map" => {
                let value_sets = catalog_value_sets(catalog);
                let (map, next_i) = parse_map_definition(lines, i, &value_sets)?;
                if catalog.maps.insert(map.name.clone(), map).is_some() {
                    return Err(parse_error(line, "duplicate map"));
                }
                i = next_i;
            }
            "run_rules_on_level_start" => {
                if tokens.len() != 1 {
                    return Err(parse_error(
                        line,
                        "run_rules_on_level_start takes no values",
                    ));
                }
                *run_rules_on_level_start = true;
                i += 1;
            }
            lifecycle_block if puzzle_lifecycle_event(lifecycle_block).is_some() => {
                let local_frame = parse_program_local_frame_modifier(&tokens[1..], line, catalog)?;
                let lifecycle = puzzle_lifecycle_event(lifecycle_block).unwrap();
                let (event, statements, next_i) =
                    match parse_lifecycle_block(lines, Some(line_numbers), i, lifecycle, catalog) {
                        Ok(parsed) => parsed,
                        Err(report) => {
                            diagnostics.extend(report.into_diagnostics());
                            i = recover_after_directive_error(lines, i);
                            continue;
                        }
                    };
                match event.as_str() {
                    "level_start" => {
                        if level_start_statements.is_some() {
                            diagnostics.extend(
                                parse_error(line, "multiple level_start blocks are not supported")
                                    .into_diagnostics(),
                            );
                            i = recover_after_directive_error(lines, i);
                            continue;
                        }
                        *level_start_statements = Some(statements);
                        *level_start_local_frame = local_frame;
                    }
                    "level_clear" => {
                        if level_clear_statements.is_some() {
                            diagnostics.extend(
                                parse_error(line, "multiple level_clear blocks are not supported")
                                    .into_diagnostics(),
                            );
                            i = recover_after_directive_error(lines, i);
                            continue;
                        }
                        *level_clear_statements = Some(statements);
                        *level_clear_local_frame = local_frame;
                    }
                    "last_level_clear" => {
                        if last_level_clear_statements.is_some() {
                            diagnostics.extend(
                                parse_error(
                                    line,
                                    "multiple last_level_clear blocks are not supported",
                                )
                                .into_diagnostics(),
                            );
                            i = recover_after_directive_error(lines, i);
                            continue;
                        }
                        *last_level_clear_statements = Some(statements);
                        *last_level_clear_local_frame = local_frame;
                    }
                    _ => unreachable!("matched lifecycle event"),
                }
                i = next_i;
            }
            "layers" if tokens.len() == 1 => {
                i = parse_layers_block(lines, i + 1, named_layers, layer_count, catalog)?;
                refresh_layer_tags_and_value_sets(named_layers, catalog);
            }
            "layers" => {
                *layer_count = Some(parse_u16(tokens.get(1), line, "missing layer count")?);
                i += 1;
            }
            "collision_layers" => {
                diagnostics.extend(
                    parse_error(line, "`collision_layers` was removed; use `layers { ... }`")
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "empty" => {
                *empty_char = Some(parse_char(tokens.get(1), line, "missing empty char")?);
                i += 1;
            }
            "marks" => {
                i = parse_mark_block(lines, i, catalog)?;
            }
            "input" => {
                let (direction, next_i) = parse_command_definition(lines, i, catalog)?;
                if let Some(direction) = direction {
                    directions.push(direction);
                }
                i = next_i;
            }
            "inputs" => {
                diagnostics.extend(
                    parse_error(
                        line,
                        "`inputs { ... }` was removed; use `keys { <key...> -> <input> }`",
                    )
                    .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "keys" => {
                i = parse_model_keys_block(lines, i, catalog, controls)?;
            }
            "var" | "const" | "persistent" => {
                parse_global_directive(
                    &tokens,
                    line,
                    &mut catalog.global_names,
                    &mut catalog.global_labels,
                    &mut catalog.global_defaults,
                    &mut catalog.numeric_global_defaults,
                    &mut catalog.persistent_vars,
                    &mut catalog.constant_globals,
                )?;
                i += 1;
            }
            "global" => {
                diagnostics.extend(
                    parse_error(line, "`global` was removed; use `var`").into_diagnostics(),
                );
                i += 1;
            }
            "condition" => {
                let definition = parse_condition_directive(
                    &tokens,
                    line,
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(&catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                    &mut catalog.condition_names,
                    &mut catalog.condition_labels,
                )?;
                condition_definitions.push(definition);
                i += 1;
            }
            "effect" => {
                diagnostics.extend(
                    parse_error(line, "effect definitions are obsolete; use routine")
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "groups" => {
                if tokens.len() == 1 {
                    i = parse_group_block(lines, i, catalog)?;
                } else {
                    return Err(parse_error(line, "groups block must be: groups { ... }"));
                }
            }
            "group" => {
                let message = if tokens.len() == 1 {
                    "`group { ... }` was removed; use `groups { ... }`"
                } else {
                    "`group <name> = ...` was removed; use `groups { <name> = ... }`"
                };
                diagnostics.extend(parse_error(line, message).into_diagnostics());
                i = recover_after_directive_error(lines, i);
            }
            "direction" => {
                if let Some(direction) = parse_direction_directive(&tokens, line, catalog)? {
                    directions.push(direction);
                }
                i += 1;
            }
            "legend" => {
                diagnostics.extend(
                    parse_error(line, "`legend` must be inside `levels { ... }`")
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "render_overlay" => {
                let (overlays, level_objects, ch) = parse_render_overlay(
                    &tokens,
                    line,
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(&catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                )?;
                render_overlays.extend(overlays);
                if let Some(objects) = level_objects {
                    catalog.char_objects.insert(ch, objects);
                }
                i += 1;
            }
            "win_conditions" | "lose_conditions" => {
                i = parse_conditions_block(lines, i, catalog, named_conditions)?;
            }
            "sprites" => {
                pending_visual_blocks.push(i);
                let (_, next_i) = collect_authoring_entry(lines, i)?;
                i = next_i;
            }
            "render" => {
                i = parse_puzzle_render_block(lines, i, render)?;
            }
            "animation" => {
                i = parse_animation_block(lines, i, animation)?;
            }
            "sounds" => {
                i = parse_model_sounds_block(
                    lines,
                    i,
                    model_sound_triggers,
                    model_operation_sounds,
                    true,
                )?;
            }
            "screen" | "layout" => {
                i = parse_puzzle_screen_block(lines, i, puzzle_screen)?;
            }
            "flickscreen" | "zoomscreen" | "screen_focus" => {
                parse_puzzle_screen_directive(line, puzzle_screen)?;
                i += 1;
            }
            "frame_focus" | "frame_size" | "switch_frame" | "follow_frame" => {
                diagnostics.extend(parse_error(
                    line,
                    "`frame_*` screen directives were removed; use `flickscreen`, `zoomscreen`, or `screen_focus`",
                ).into_diagnostics());
                i += 1;
            }
            "routine" => {
                match parse_rule_definition(
                    lines,
                    Some(line_numbers),
                    i,
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                    &catalog.input_names,
                    &catalog.global_names,
                    &catalog.numeric_global_defaults,
                    &catalog.condition_names,
                ) {
                    Ok((definition, next_i)) => {
                        rule_definitions.push(definition);
                        i = next_i;
                    }
                    Err(report) => {
                        diagnostics.extend(report.into_diagnostics());
                        i = recover_after_directive_error(lines, i);
                    }
                }
            }
            "rule" => {
                diagnostics.extend(
                    parse_error(line, "`rule` was removed; use `routine`").into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "rules" => {
                let local_frame = parse_program_local_frame_modifier(&tokens[1..], line, catalog)?;
                if main_statements.is_some() {
                    diagnostics.extend(
                        parse_error(line, "multiple puzzle rules blocks are not supported")
                            .into_diagnostics(),
                    );
                    i = recover_after_directive_error(lines, i);
                    continue;
                }
                match parse_statement_block(
                    lines,
                    Some(line_numbers),
                    i + 1,
                    &[BLOCK_CLOSE],
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                    &catalog.input_names,
                    &catalog.global_names,
                    &catalog.numeric_global_defaults,
                    &catalog.condition_names,
                    named_conditions,
                    &[],
                ) {
                    Ok((statements, next_i)) => {
                        *main_statements = Some(statements);
                        *main_local_frame = local_frame;
                        i = next_i;
                    }
                    Err(report) => {
                        diagnostics.extend(report.into_diagnostics());
                        i = recover_after_directive_error(lines, i);
                    }
                }
            }
            "main" | "transitions" => {
                diagnostics.extend(
                    parse_error(line, "`main`/`transitions` were removed; use `rules`")
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "on_display" => {
                if tokens.len() != 1 {
                    return Err(parse_error(line, "display hook header must be: on_display"));
                }
                if display_statements.is_some() {
                    diagnostics.extend(
                        parse_error(line, "multiple on_display blocks are not supported")
                            .into_diagnostics(),
                    );
                    i = recover_after_directive_error(lines, i);
                    continue;
                }
                match parse_statement_block(
                    lines,
                    Some(line_numbers),
                    i + 1,
                    &[BLOCK_CLOSE],
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                    &catalog.input_names,
                    &catalog.global_names,
                    &catalog.numeric_global_defaults,
                    &catalog.condition_names,
                    named_conditions,
                    &[],
                ) {
                    Ok((statements, next_i)) => {
                        validate_display_hook_statements(&statements)?;
                        *display_statements = Some(statements);
                        i = next_i;
                    }
                    Err(report) => {
                        diagnostics.extend(report.into_diagnostics());
                        i = recover_after_directive_error(lines, i);
                    }
                }
            }
            "display" => {
                diagnostics.extend(parse_error(
                    line,
                    "`display ...` syntax was removed; use @ display objects and @routine calls",
                ).into_diagnostics());
                i = recover_after_directive_error(lines, i);
            }
            "levels" => {
                pending_level_blocks.push(PendingLevelBlock::levels(i, Some(name.to_string())));
                let (_, next_i) = collect_levels_authoring_entry(lines, i)?;
                i = next_i;
            }
            "level" => {
                pending_level_blocks.push(PendingLevelBlock::level(i, Some(name.to_string())));
                let (_, next_i) = parse_level_block(lines, i, 0)?;
                i = next_i;
            }
            other => {
                diagnostics.extend(
                    parse_error(line, &format!("unknown puzzle directive {other}"))
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "puzzle missing closing brace"));
    }
    for visual_start in pending_visual_blocks {
        if let Err(report) = parse_visuals_block(lines, visual_start, catalog, visuals) {
            diagnostics.extend(report.into_diagnostics());
        }
    }
    if !diagnostics.is_empty() {
        return Err(DiagnosticReport::from_diagnostics(diagnostics));
    }
    validate_puzzle_screen(puzzle_screen, &lines[start])?;

    Ok((i + 1, name.to_string()))
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
