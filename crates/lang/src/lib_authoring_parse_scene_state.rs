fn parse_button_label(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    parse_scene_expr(value, line)
}

struct ParsedSceneStateBlock {
    variables: Vec<SceneVarDef>,
    puzzles: Vec<ScenePuzzleDef>,
}

enum ParsedSceneStateEntry {
    Variable(SceneVarDef),
    Puzzle(ScenePuzzleDef),
}

struct ParsedScenePuzzleLayoutDeclaration {
    puzzle: ScenePuzzleDef,
    layout: SceneLayoutDef,
}

fn inferred_scene_puzzle_slot(name: &str, lifetime: SceneStateLifetime) -> ScenePuzzleDef {
    ScenePuzzleDef {
        name: name.to_string(),
        model: name.to_string(),
        initializer: ScenePuzzleInitializer::CurrentLevel,
        lifetime,
    }
}

fn parse_scene_puzzle_layout_declaration(
    line: &str,
    lifetime: SceneStateLifetime,
) -> Result<Option<ParsedScenePuzzleLayoutDeclaration>, DiagnosticReport> {
    if let Some((declaration, attrs)) = parse_typed_scene_puzzle_declaration(line, lifetime)? {
        return Ok(Some(ParsedScenePuzzleLayoutDeclaration {
            puzzle: declaration,
            layout: parse_model_window_layout_attrs_for_line(&attrs, line)?,
        }));
    }
    let Some((name, value)) = parse_assignment_row(line) else {
        return Ok(None);
    };
    if !is_identifier(name) {
        return Ok(None);
    }
    let tokens = split_header_tokens(value);
    reject_old_scene_puzzle_initializer(tokens.as_slice(), line)?;
    Ok(None)
}

fn parse_typed_scene_puzzle_declaration<'a>(
    line: &'a str,
    lifetime: SceneStateLifetime,
) -> Result<Option<(ScenePuzzleDef, Vec<&'a str>)>, DiagnosticReport> {
    let Some((lhs, rhs)) = parse_assignment_row(line) else {
        return Ok(None);
    };
    let lhs_tokens = split_header_tokens(lhs);
    let rhs_tokens = split_header_tokens(rhs);
    let name = match lhs_tokens.as_slice() {
        ["puzzle", name] => *name,
        ["puzzle", ..] => {
            return Err(parse_error(
                line,
                "scene puzzle declaration must be: puzzle <slot> = <model> | puzzle <slot> = <model> level <level>",
            ));
        }
        ["puzzle3", ..] => {
            return Err(parse_error(
                line,
                "`puzzle3` was removed; use `puzzle <slot> = <model>` in .puzzle files",
            ));
        }
        _ => return Ok(None),
    };
    validate_identifier(name, line, "puzzle slot name")?;
    let (model, initializer, attrs) =
        parse_scene_puzzle_initializer_rhs(rhs_tokens.as_slice(), line)?;
    Ok(Some((
        ScenePuzzleDef {
            name: name.to_string(),
            model,
            initializer,
            lifetime,
        },
        attrs,
    )))
}

fn parse_scene_puzzle_initializer_rhs<'a>(
    tokens: &[&'a str],
    line: &str,
) -> Result<(String, ScenePuzzleInitializer, Vec<&'a str>), DiagnosticReport> {
    match tokens {
        ["puzzle", ..] => Err(parse_error(
            line,
            "scene puzzle declaration must put the kind on the left: puzzle <slot> = <model>",
        )),
        ["puzzle3", ..] => Err(parse_error(
            line,
            "`puzzle3` was removed; use `puzzle <slot> = <model>` in .puzzle files",
        )),
        ["current_level", ..] => Err(parse_error(
            line,
            "current_level is not scene syntax; use `<model>` for the current level",
        )),
        [model, "level", level_name, attrs @ ..] => {
            validate_qualified_identifier(model, line, "puzzle model name")?;
            validate_qualified_identifier(level_name, line, "level name")?;
            Ok((
                (*model).to_string(),
                ScenePuzzleInitializer::Level((*level_name).to_string()),
                attrs.to_vec(),
            ))
        }
        [model, attrs @ ..] => {
            validate_qualified_identifier(model, line, "puzzle model name")?;
            Ok((
                (*model).to_string(),
                ScenePuzzleInitializer::CurrentLevel,
                attrs.to_vec(),
            ))
        }
        [] => Err(parse_error(line, "scene puzzle declaration must name a model")),
    }
}

fn reject_old_scene_puzzle_initializer(
    tokens: &[&str],
    line: &str,
) -> Result<(), DiagnosticReport> {
    match tokens {
        ["puzzle", ..] => Err(parse_error(
            line,
            "scene puzzle declaration must be: puzzle <slot> = <model>",
        )),
        ["puzzle3", ..] => Err(parse_error(
            line,
            "`puzzle3` was removed; use `puzzle <slot> = <model>` in .puzzle files",
        )),
        _ => Ok(()),
    }
}

fn parse_scene_state_entry(
    line: &str,
    lifetime: SceneStateLifetime,
) -> Result<ParsedSceneStateEntry, DiagnosticReport> {
    let line = line.trim();
    let mut prefixed_variable = false;
    let (line, lifetime, mutable) = if let Some(rest) = line.strip_prefix("persistent var ") {
        prefixed_variable = true;
        (rest.trim_start(), SceneStateLifetime::Persistent, true)
    } else if let Some(rest) = line.strip_prefix("persistent const ") {
        prefixed_variable = true;
        (rest.trim_start(), SceneStateLifetime::Persistent, false)
    } else if let Some(rest) = line.strip_prefix("var ") {
        prefixed_variable = true;
        (rest.trim_start(), lifetime, true)
    } else if let Some(rest) = line.strip_prefix("const ") {
        prefixed_variable = true;
        (rest.trim_start(), lifetime, false)
    } else {
        (line, lifetime, true)
    };
    if let Some((puzzle, attrs)) = parse_typed_scene_puzzle_declaration(line, lifetime)? {
        if !attrs.is_empty() {
            return Err(parse_error(
                line,
                "scene state puzzle declarations do not accept layout attributes",
            ));
        }
        if prefixed_variable {
            return Err(parse_error(
                line,
                "var or const cannot define a puzzle slot",
            ));
        }
        return Ok(ParsedSceneStateEntry::Puzzle(puzzle));
    }
    let (name, value) = require_assignment_row(line, "scene state must be: <name> = <value>")?;
    if !is_identifier(name) {
        return Err(parse_error(line, "scene state name must be an identifier"));
    }
    reject_old_scene_puzzle_initializer(split_header_tokens(value).as_slice(), line)?;
    let (kind, default) = parse_scene_var_default(value, line)?;
    Ok(ParsedSceneStateEntry::Variable(SceneVarDef {
        name: name.to_string(),
        kind,
        default,
        lifetime,
        mutable,
    }))
}

fn parse_top_level_var_directive(
    _tokens: &[&str],
    line: &str,
) -> Result<SceneVarDef, DiagnosticReport> {
    let Some(row) =
        authoring_grammar::parse_authoring_row(authoring_grammar::AuthoringKind::Root, line)?
    else {
        return Err(parse_error(
            line,
            "top-level variable must be: var <name> = <literal> or const <name> = <literal>",
        ));
    };
    let (lifetime, mutable) = match row.kind {
        authoring_grammar::AuthoringRowKind::VarDeclaration => (SceneStateLifetime::Instance, true),
        authoring_grammar::AuthoringRowKind::ConstDeclaration => {
            (SceneStateLifetime::Instance, false)
        }
        authoring_grammar::AuthoringRowKind::PersistentVarDeclaration => {
            (SceneStateLifetime::Persistent, true)
        }
        authoring_grammar::AuthoringRowKind::PersistentConstDeclaration => {
            (SceneStateLifetime::Persistent, false)
        }
        _ => {
            return Err(parse_error(
                &row.source_line,
                "top-level variable row has the wrong authoring kind",
            ));
        }
    };
    let Some(name) = row.single_capture("name") else {
        return Err(parse_error(&row.source_line, "variable name is missing"));
    };
    let Some(value) = row.joined_capture("value") else {
        return Err(parse_error(&row.source_line, "variable value is missing"));
    };
    validate_identifier(name, line, "variable name")?;
    let (kind, default) = parse_scene_var_default(&value, line)?;
    Ok(SceneVarDef {
        name: name.to_string(),
        kind,
        default,
        lifetime,
        mutable,
    })
}

fn parse_root_scalar_definition_value(
    line: &str,
    key: &str,
    usage_message: &str,
) -> Result<String, DiagnosticReport> {
    let Some(definition) = authoring_grammar::parse_authoring_definition_row(
        authoring_grammar::AuthoringKind::Root,
        line,
    )?
    else {
        return Err(parse_error(line, usage_message));
    };
    if definition.key != key
        || definition.op != Some(authoring_grammar::AuthoringDefinitionOp::Equals)
    {
        return Err(parse_error(line, usage_message));
    }
    definition
        .single_value()
        .map(str::to_string)
        .ok_or_else(|| parse_error(line, usage_message))
}

fn parse_default_wait_time_directive(line: &str) -> Result<u64, DiagnosticReport> {
    let duration = parse_root_scalar_definition_value(
        line,
        "default_wait_time",
        "default_wait_time must be: default_wait_time = <duration>",
    )?;
    parse_wait_duration_ms(&duration, line)
}

fn parse_input_buffer_block(
    lines: &[source::LogicalLine],
    start: usize,
    input_buffer: &mut InputBufferDef,
) -> Result<usize, DiagnosticReport> {
    let (node, next_i) = authoring_grammar::parse_placed_authoring_node(
        lines,
        start,
        authoring_grammar::AuthoringKind::Root,
        "input_buffer missing closing brace",
    )?;
    if node.kind != authoring_grammar::AuthoringKind::InputBufferConfig {
        return Err(parse_error(
            &lines[start],
            "input_buffer header must be: input_buffer",
        ));
    }

    let mut parsed = input_buffer.clone();
    for definition in &node.definition_rows {
        if definition.op != Some(authoring_grammar::AuthoringDefinitionOp::Equals) {
            return Err(parse_error(
                &definition.source_line,
                "input_buffer setting must use `=`",
            ));
        }
        let Some(value) = definition.single_value() else {
            return Err(parse_error(
                &definition.source_line,
                "input_buffer setting must have one value",
            ));
        };
        match definition.key.as_str() {
            "queue_during_wait" => {
                parsed.queue_during_wait = parse_boolean_option(value, &definition.source_line)?;
            }
            "fast_forward_wait" => {
                parsed.fast_forward_wait = parse_boolean_option(value, &definition.source_line)?;
            }
            "min_wait" => {
                parsed.min_wait_ms = parse_wait_duration_ms(value, &definition.source_line)?;
            }
            other => {
                return Err(parse_error(
                    &definition.source_line,
                    &format!("unknown input_buffer setting {other}"),
                ));
            }
        }
    }

    *input_buffer = parsed;
    Ok(next_i)
}

fn parse_scene_value(value: &str, line: &str) -> Result<SceneValue, DiagnosticReport> {
    if value == "true" {
        return Ok(SceneValue::Bool(true));
    }
    if value == "false" {
        return Ok(SceneValue::Bool(false));
    }
    if let Ok(number) = value.parse::<i64>() {
        return Ok(SceneValue::Int(number));
    }
    if let Some(text) = parse_quoted_text(value) {
        return Ok(SceneValue::Text(text));
    }
    if is_identifier(value) {
        return Ok(SceneValue::Symbol(value.to_string()));
    }
    Err(parse_error(
        line,
        "scene state value must be true, false, integer, symbol, or quoted text",
    ))
}

fn parse_scene_var_default(
    value: &str,
    line: &str,
) -> Result<(SceneVarKind, SceneValue), DiagnosticReport> {
    if let Some(default) = value.strip_prefix("signal ") {
        let default = default.trim();
        if default.is_empty() {
            return Err(parse_error(
                line,
                "signal variable must name a default value",
            ));
        }
        return Ok((SceneVarKind::Signal, parse_scene_value(default, line)?));
    }
    Ok((SceneVarKind::Value, parse_scene_value(value, line)?))
}

fn parse_quoted_text(value: &str) -> Option<String> {
    puzzle_authoring::parse_quoted_text(value)
}

struct ParsedSceneRulesBlock {
    puzzle_rule: Option<ScenePuzzleRule>,
    transitions: Vec<SceneTransition>,
}

fn parse_scene_rules_block(
    lines: &[source::LogicalLine],
    start: usize,
) -> Result<(ParsedSceneRulesBlock, usize), DiagnosticReport> {
    let mut puzzle_rule = None;
    let mut transitions = Vec::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["step", target] => {
                validate_target_path(target, &lines[i], "step target")?;
                puzzle_rule = Some(ScenePuzzleRule {
                    target: (*target).to_string(),
                    rule: "rules".to_string(),
                });
            }
            [rule] if rule.contains('.') => {
                return Err(parse_error(
                    &lines[i],
                    "scene rules do not call component rules by path; use `step <puzzle>`",
                ));
            }
            ["if", ..] => {
                let (transition, next_i) = parse_scene_transition_row(lines, i)?;
                transitions.push(transition);
                i = next_i;
                continue;
            }
            _ if lines[i].contains("->") => {
                let (transition, next_i) = parse_scene_transition_row(lines, i)?;
                transitions.push(transition);
                i = next_i;
                continue;
            }
            _ => {
                return Err(parse_error(
                    &lines[i],
                    "scene rules row must be: step <puzzle>",
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "scene rules missing closing brace",
        ));
    }

    Ok((
        ParsedSceneRulesBlock {
            puzzle_rule,
            transitions,
        },
        i + 1,
    ))
}

fn parse_scene_transition_row(
    lines: &[source::LogicalLine],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let line = &lines[start];
    let Some((condition, effect)) = line
        .trim()
        .strip_prefix("if ")
        .and_then(|row| row.split_once("->"))
    else {
        return Err(parse_error(
            line,
            "scene rules row must be: step <puzzle> or if <condition> -> <effect>",
        ));
    };
    let condition = parse_scene_condition_expr(condition.trim(), line)?;
    let (effect, next_i) = parse_scene_effect_with_optional_block(effect.trim(), lines, start)?;
    Ok((
        SceneTransition {
            trigger: SceneTransitionTrigger::Condition(condition),
            effect,
        },
        next_i,
    ))
}

fn parse_scene_condition_block(
    lines: &[source::LogicalLine],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let line = &lines[start];
    let condition = block_header_text(line)
        .strip_prefix("if ")
        .ok_or_else(|| parse_error(line, "condition block must be: if <condition>"))?
        .trim();
    let condition = parse_scene_condition_expr(condition, line)?;
    let (body, next_i) =
        collect_authoring_entry(lines, start, AuthoringEntryOwner::SceneCondition)?;
    let body = &body[1..body.len().saturating_sub(1)];
    if body.is_empty() {
        return Err(parse_error(
            line,
            "condition block requires at least one effect",
        ));
    }
    Ok((
        SceneTransition {
            trigger: SceneTransitionTrigger::Condition(condition),
            effect: parse_scene_handler_effects(body, line)?,
        },
        next_i,
    ))
}

fn parse_scene_on_block(
    lines: &[source::LogicalLine],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let line = &lines[start];
    let condition = block_header_text(line)
        .strip_prefix("on ")
        .ok_or_else(|| parse_error(line, "scene handler block must be: on <signal condition>"))?
        .trim();
    let condition = parse_scene_condition_expr(condition, line)?;
    let (body, next_i) =
        collect_authoring_entry(lines, start, AuthoringEntryOwner::SceneCondition)?;
    let body = &body[1..body.len().saturating_sub(1)];
    if body.is_empty() {
        return Err(parse_error(
            line,
            "scene handler block requires at least one effect",
        ));
    }
    Ok((
        SceneTransition {
            trigger: SceneTransitionTrigger::Signal(condition),
            effect: parse_scene_handler_effects(body, line)?,
        },
        next_i,
    ))
}

fn parse_scene_lifecycle_block(
    lines: &[source::LogicalLine],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    let [lifecycle @ "on_scene_start"] = tokens.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "scene lifecycle block must be: on_scene_start",
        ));
    };
    let (body, next_i) =
        collect_authoring_entry(lines, start, AuthoringEntryOwner::SceneLifecycle)?;
    let body = &body[1..body.len().saturating_sub(1)];
    if body.is_empty() {
        return Err(parse_error(
            &lines[start],
            "scene lifecycle block requires at least one effect",
        ));
    }
    let trigger = match *lifecycle {
        "on_scene_start" => SceneTransitionTrigger::SceneStart,
        _ => unreachable!(),
    };
    Ok((
        SceneTransition {
            trigger,
            effect: parse_scene_handler_effects(body, &lines[start])?,
        },
        next_i,
    ))
}

fn parse_scene_routine_block(
    lines: &[source::LogicalLine],
    start: usize,
) -> Result<(SceneRoutineDef, usize), DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    let ["routine", name] = tokens.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "scene routine header must be: routine <name>",
        ));
    };
    validate_identifier(name, &lines[start], "scene routine name")?;
    let (body, next_i) = collect_authoring_entry(lines, start, AuthoringEntryOwner::SceneRoutine)?;
    let body = &body[1..body.len().saturating_sub(1)];
    if body.is_empty() {
        return Err(parse_error(
            &lines[start],
            "scene routine requires at least one effect",
        ));
    }
    Ok((
        SceneRoutineDef {
            name: (*name).to_string(),
            effect: parse_scene_handler_effects(body, &lines[start])?,
        },
        next_i,
    ))
}

fn parse_scene_handler_effects(
    lines: &[source::LogicalLine],
    header_line: &str,
) -> Result<SceneEffect, DiagnosticReport> {
    parse_scene_handler_effects_range(lines, 0, lines.len(), header_line)
}

fn parse_scene_handler_effects_range(
    lines: &[source::LogicalLine],
    start: usize,
    end: usize,
    header_line: &str,
) -> Result<SceneEffect, DiagnosticReport> {
    let mut effects = Vec::new();
    let mut i = start;
    while i < end {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            ["if", condition @ ..] => {
                let condition = condition.join(" ");
                let condition = parse_scene_condition_expr(&condition, line)?;
                let block_end = matching_effect_block_end(lines, i, end)?;
                let effect = parse_scene_handler_effects_range(lines, i + 1, block_end, line)?;
                effects.push(SceneEffect::Conditional {
                    condition,
                    effect: Box::new(effect),
                });
                i = block_end + 1;
                continue;
            }
            ["update", _target] => {
                return Err(parse_error(
                    line,
                    "`update <target>` was removed; use `apply <rule> to <target>`",
                ));
            }
            _ => effects.push(parse_scene_effect(line, line)?),
        }
        i += 1;
    }
    match effects.len() {
        0 => Err(parse_error(
            header_line,
            "handler requires at least one effect",
        )),
        1 => Ok(effects.remove(0)),
        _ => Ok(SceneEffect::Sequence { effects }),
    }
}

fn matching_effect_block_end(
    lines: &[source::LogicalLine],
    start: usize,
    end: usize,
) -> Result<usize, DiagnosticReport> {
    let mut depth = 0usize;
    for (i, line) in lines.iter().enumerate().take(end).skip(start + 1) {
        let tokens = split_header_tokens(line);
        if matches!(tokens.as_slice(), ["if", ..]) {
            depth += 1;
            continue;
        }
        if is_block_close_line(line) {
            if depth == 0 {
                return Ok(i);
            }
            depth -= 1;
        }
    }
    Err(parse_error(
        &lines[start],
        "if effect block missing closing brace",
    ))
}

fn parse_scene_condition_expr(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    parse_scene_expr(value, line)
}

fn parse_boolean_option(value: &str, line: &str) -> Result<bool, DiagnosticReport> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(parse_error(line, "boolean option must be true or false")),
    }
}

fn parse_key_trigger(token: &str, line: &str) -> Result<KeyTrigger, DiagnosticReport> {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return Err(parse_error(line, "missing key"));
    };
    if chars.next().is_none() {
        return Ok(KeyTrigger::Char(first.to_ascii_lowercase()));
    }
    Ok(KeyTrigger::Named(token.to_string()))
}

fn lower_model_key_bindings(
    bindings: &[model_syntax::ModelKeyBindingSyntax],
    catalog: &mut Catalog,
    controls: &mut Controls,
) -> Result<(), DiagnosticReport> {
    let mut seen_keys = HashSet::<KeyTrigger>::new();
    for binding in bindings {
        let input = catalog
            .input_names
            .get(&binding.target)
            .copied()
            .map(Ok)
            .unwrap_or_else(|| add_input_name(&binding.target, &binding.source, catalog))?;
        for key in &binding.keys {
            let trigger = parse_key_trigger(key, &binding.source)?;
            if !seen_keys.insert(trigger.clone()) {
                return Err(parse_error(&binding.source, "duplicate model input key"));
            }
            add_key_trigger_to_controls(&trigger, input, controls, &binding.source)?;
        }
    }
    Ok(())
}

fn parse_scene_keys_block(
    lines: &[source::LogicalLine],
    start: usize,
    recognition: &mut crate::surface::ParserRecognition,
) -> Result<(Vec<KeyBinding>, usize), DiagnosticReport> {
    let mut bindings = Vec::<KeyBinding>::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let (binding, next_i) = parse_scene_key_binding_at(lines, i)?;
        recognize_scene_key_binding_line(&lines[i], recognition);
        recognize_scene_effect_body(lines, i + 1, next_i, recognition);
        bindings.push(binding);
        i = next_i;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "keys missing closing brace"));
    }
    Ok((bindings, i + 1))
}

fn recognize_scene_key_binding_line(
    line: &source::LogicalLine,
    recognition: &mut crate::surface::ParserRecognition,
) {
    let Some(arrow) = line.tokens.iter().position(|token| token.text == "->") else {
        return;
    };
    for token in &line.tokens[..arrow] {
        recognition.mark(
            crate::surface::SourceSpan {
                start: token.start,
                end: token.end,
            },
            crate::surface::SurfaceSemanticKind::Input,
        );
    }
    recognition.merge(scene_effect_parser_recognition(&line.tokens[arrow + 1..]));
}

fn parse_scene_key_binding_at(
    lines: &[source::LogicalLine],
    start: usize,
) -> Result<(KeyBinding, usize), DiagnosticReport> {
    let row = parse_keys_surface_row(&lines[start], "scene effect-or-input", true)?;
    lower_scene_keys_row(row, lines, start)
}

fn lower_scene_keys_row(
    row: KeysSurfaceRow<'_>,
    lines: &[source::LogicalLine],
    start: usize,
) -> Result<(KeyBinding, usize), DiagnosticReport> {
    let mut triggers = Vec::new();
    for key in row.keys {
        let trigger = parse_key_trigger(key, &lines[start])?;
        validate_key_trigger_supported(&trigger, &lines[start])?;
        triggers.push(trigger);
    }
    let (effect, next_i) = parse_scene_effect_with_optional_block(row.target, lines, start)?;
    Ok((
        KeyBinding {
            keys: triggers,
            effect,
        },
        next_i,
    ))
}

fn add_key_trigger_to_controls(
    key: &KeyTrigger,
    input: InputId,
    controls: &mut Controls,
    line: &str,
) -> Result<(), DiagnosticReport> {
    match key {
        KeyTrigger::Char(ch) if ch.is_ascii() => {
            controls
                .keys
                .insert((*ch as u8).to_ascii_lowercase(), input);
        }
        KeyTrigger::Char(_) => {
            return Err(DiagnosticReport::error(
                "non-ascii model input key bindings are not supported yet".to_string(),
            ));
        }
        KeyTrigger::Named(name) => {
            validate_key_trigger_supported(key, line)?;
            if let Some(arrow) = named_key_to_arrow(name) {
                controls.arrows.insert(arrow, input);
            } else {
                controls.named.insert(name.clone(), input);
            }
        }
    }
    Ok(())
}

fn add_default_key_controls(
    dimension: ModelDimension,
    input_names: &HashMap<String, InputId>,
    controls: &mut Controls,
) {
    let (forward, backward) = match dimension {
        ModelDimension::Two => ("up", "down"),
        ModelDimension::Three => ("front", "back"),
    };
    for (name, key, arrow) in [
        (forward, b'w', Some(ArrowKey::Up)),
        (backward, b's', Some(ArrowKey::Down)),
        ("left", b'a', Some(ArrowKey::Left)),
        ("right", b'd', Some(ArrowKey::Right)),
        ("restart", b'r', None),
    ] {
        let Some(input) = input_names.get(name).copied() else {
            continue;
        };
        controls.keys.entry(key).or_insert(input);
        if let Some(arrow) = arrow {
            controls.arrows.entry(arrow).or_insert(input);
        }
    }
}

fn validate_key_trigger_supported(key: &KeyTrigger, line: &str) -> Result<(), DiagnosticReport> {
    match key {
        KeyTrigger::Char(ch) if ch.is_ascii() => Ok(()),
        KeyTrigger::Char(_) => Err(DiagnosticReport::error(
            "non-ascii input key bindings are not supported yet".to_string(),
        )),
        KeyTrigger::Named(name) if is_supported_named_key(name) => Ok(()),
        KeyTrigger::Named(_) => Err(parse_error(
            line,
            "inputs only support character keys, ArrowUp/ArrowDown/ArrowLeft/ArrowRight, Enter, Space, Escape, Tab, and Backspace",
        )),
    }
}

fn is_supported_named_key(name: &str) -> bool {
    named_key_to_arrow(name).is_some()
        || matches!(name, "Enter" | "Space" | "Escape" | "Tab" | "Backspace")
}

fn named_key_to_arrow(name: &str) -> Option<ArrowKey> {
    match name {
        "ArrowUp" | "arrow_up" => Some(ArrowKey::Up),
        "ArrowDown" | "arrow_down" => Some(ArrowKey::Down),
        "ArrowLeft" | "arrow_left" => Some(ArrowKey::Left),
        "ArrowRight" | "arrow_right" => Some(ArrowKey::Right),
        _ => None,
    }
}
