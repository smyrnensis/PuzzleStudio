fn parse_button_label(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    parse_scene_expr(value, line)
}

struct ParsedScreenStateBlock {
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
        kind: INFERRED_SCENE_PUZZLE_KIND.to_string(),
        model: name.to_string(),
        initializer: ScenePuzzleInitializer::CurrentLevel,
        lifetime,
    }
}

fn parse_scene_puzzle_layout_declaration(
    line: &str,
    lifetime: SceneStateLifetime,
) -> Result<Option<ParsedScenePuzzleLayoutDeclaration>, DiagnosticReport> {
    let Some((name, value)) = parse_assignment_row(line) else {
        return Ok(None);
    };
    if !is_identifier(name) {
        return Ok(None);
    }
    let tokens = split_header_tokens(value);
    let Some((kind, model, initializer, attrs)) =
        parse_scene_puzzle_initializer_tokens(tokens.as_slice(), line)?
    else {
        return Ok(None);
    };
    Ok(Some(ParsedScenePuzzleLayoutDeclaration {
        puzzle: ScenePuzzleDef {
            name: name.to_string(),
            kind,
            model,
            initializer,
            lifetime,
        },
        layout: parse_scene_layout_attrs_for_line(attrs, line)?,
    }))
}

fn parse_scene_puzzle_initializer_tokens<'a>(
    tokens: &'a [&'a str],
    line: &str,
) -> Result<Option<(String, String, ScenePuzzleInitializer, &'a [&'a str])>, DiagnosticReport> {
    match tokens {
        ["puzzle", "current_level", ..] => Err(parse_error(
            line,
            "current_level is not scene syntax; use `puzzle <name>` for the current level",
        )),
        ["puzzle", puzzle_name, "level", level_name, attrs @ ..] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            validate_qualified_identifier(level_name, line, "level name")?;
            Ok(Some((
                "puzzle".to_string(),
                (*puzzle_name).to_string(),
                ScenePuzzleInitializer::Level((*level_name).to_string()),
                attrs,
            )))
        }
        ["puzzle", puzzle_name, attrs @ ..] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            Ok(Some((
                "puzzle".to_string(),
                (*puzzle_name).to_string(),
                ScenePuzzleInitializer::CurrentLevel,
                attrs,
            )))
        }
        ["puzzle3", puzzle_name, "level", level_name, attrs @ ..] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle3 model name")?;
            validate_qualified_identifier(level_name, line, "level name")?;
            Ok(Some((
                "puzzle3".to_string(),
                (*puzzle_name).to_string(),
                ScenePuzzleInitializer::Level((*level_name).to_string()),
                attrs,
            )))
        }
        ["puzzle3", puzzle_name, attrs @ ..] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle3 model name")?;
            Ok(Some((
                "puzzle3".to_string(),
                (*puzzle_name).to_string(),
                ScenePuzzleInitializer::CurrentLevel,
                attrs,
            )))
        }
        ["puzzle", ..] => Err(parse_error(
            line,
            "scene puzzle initializer must be: puzzle <name> | puzzle <name> level <level>",
        )),
        ["puzzle3", ..] => Err(parse_error(
            line,
            "scene puzzle3 initializer must be: puzzle3 <name> | puzzle3 <name> level <level>",
        )),
        _ => Ok(None),
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
    let (name, value) = require_assignment_row(line, "scene state must be: <name> = <value>")?;
    if !is_identifier(name) {
        return Err(parse_error(line, "scene state name must be an identifier"));
    }
    if let Some(initializer) = parse_screen_puzzle_initializer(value, line)? {
        if prefixed_variable {
            return Err(parse_error(
                line,
                "var or const cannot define a puzzle slot",
            ));
        }
        return Ok(ParsedSceneStateEntry::Puzzle(ScenePuzzleDef {
            name: name.to_string(),
            kind: initializer.kind,
            model: initializer.model,
            initializer: initializer.initializer,
            lifetime,
        }));
    }
    Ok(ParsedSceneStateEntry::Variable(SceneVarDef {
        name: name.to_string(),
        default: parse_scene_value(value, line)?,
        lifetime,
        mutable,
    }))
}

fn parse_top_level_var_directive(
    _tokens: &[&str],
    line: &str,
) -> Result<SceneVarDef, DiagnosticReport> {
    let (rest, lifetime, mutable) = if let Some(rest) = line.trim().strip_prefix("persistent var ")
    {
        (rest.trim_start(), SceneStateLifetime::Persistent, true)
    } else if let Some(rest) = line.trim().strip_prefix("persistent const ") {
        (rest.trim_start(), SceneStateLifetime::Persistent, false)
    } else if let Some(rest) = line.trim().strip_prefix("var ") {
        (rest.trim_start(), SceneStateLifetime::Instance, true)
    } else if let Some(rest) = line.trim().strip_prefix("const ") {
        (rest.trim_start(), SceneStateLifetime::Instance, false)
    } else {
        return Err(parse_error(
            line,
            "top-level variable must be: var <name> = <literal> or const <name> = <literal>",
        ));
    };
    let (name, value) = require_assignment_row(
        rest,
        "top-level variable must be: var <name> = <literal> or const <name> = <literal>",
    )?;
    validate_identifier(name, line, "variable name")?;
    Ok(SceneVarDef {
        name: name.to_string(),
        default: parse_scene_value(value, line)?,
        lifetime,
        mutable,
    })
}

fn parse_default_wait_time_directive(tokens: &[&str], line: &str) -> Result<u64, DiagnosticReport> {
    let ["default_wait_time", "=", duration] = tokens else {
        return Err(parse_error(
            line,
            "default_wait_time must be: default_wait_time = <duration>",
        ));
    };
    parse_wait_duration_ms(duration, line)
}

fn parse_input_buffer_block(
    lines: &[String],
    start: usize,
    input_buffer: &mut InputBufferDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["input_buffer"]) {
        return Err(parse_error(
            &lines[start],
            "input_buffer header must be: input_buffer",
        ));
    }

    let mut parsed = input_buffer.clone();
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            *input_buffer = parsed;
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            ["queue_during_wait", "=", value] => {
                parsed.queue_during_wait = parse_boolean_option(value, line)?;
            }
            ["fast_forward_wait", "=", value] => {
                parsed.fast_forward_wait = parse_boolean_option(value, line)?;
            }
            ["min_wait", "=", value] => {
                parsed.min_wait_ms = parse_wait_duration_ms(value, line)?;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "input_buffer entry must be: queue_during_wait = <true|false> | fast_forward_wait = <true|false> | min_wait = <duration>",
                ));
            }
        }
        i += 1;
    }

    Err(parse_error(
        &lines[start],
        "input_buffer missing closing brace",
    ))
}

fn parse_again_interval_directive(tokens: &[&str], line: &str) -> Result<u64, DiagnosticReport> {
    match tokens {
        ["again_interval", "=", duration] => parse_wait_duration_ms(duration, line),
        ["again_interval", seconds] => parse_seconds_duration_ms(seconds, line),
        _ => Err(parse_error(
            line,
            "again_interval must be: again_interval = <duration> or again_interval <seconds>",
        )),
    }
}

#[derive(Clone, Debug)]
struct ParsedScenePuzzleInitializer {
    kind: String,
    model: String,
    initializer: ScenePuzzleInitializer,
}

fn parse_screen_puzzle_initializer(
    value: &str,
    line: &str,
) -> Result<Option<ParsedScenePuzzleInitializer>, DiagnosticReport> {
    let tokens = split_header_tokens(value);
    match tokens.as_slice() {
        ["puzzle", "current_level"] => Err(parse_error(
            line,
            "current_level is not scene syntax; use `puzzle <name>` for the current level",
        )),
        ["puzzle", puzzle_name, "level", level_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            validate_qualified_identifier(level_name, line, "level name")?;
            Ok(Some(ParsedScenePuzzleInitializer {
                kind: "puzzle".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::Level((*level_name).to_string()),
            }))
        }
        ["puzzle", puzzle_name] => {
            if *puzzle_name == "current_level" {
                return Err(parse_error(
                    line,
                    "current_level is not scene syntax; use `puzzle <name>` for the current level",
                ));
            }
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            Ok(Some(ParsedScenePuzzleInitializer {
                kind: "puzzle".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::CurrentLevel,
            }))
        }
        ["puzzle3", puzzle_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle3 model name")?;
            Ok(Some(ParsedScenePuzzleInitializer {
                kind: "puzzle3".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::CurrentLevel,
            }))
        }
        ["puzzle3", puzzle_name, "level", level_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle3 model name")?;
            validate_qualified_identifier(level_name, line, "level name")?;
            Ok(Some(ParsedScenePuzzleInitializer {
                kind: "puzzle3".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::Level((*level_name).to_string()),
            }))
        }
        ["puzzle", puzzle_name, "current_level"] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            Err(parse_error(
                line,
                "current_level is not scene syntax; use `puzzle <name>` for the current level",
            ))
        }
        ["puzzle", ..] => Err(parse_error(
            line,
            "scene puzzle initializer must be: puzzle <name> | puzzle <name> level <level>",
        )),
        ["puzzle3", ..] => Err(parse_error(
            line,
            "scene puzzle3 initializer must be: puzzle3 <name> | puzzle3 <name> level <level>",
        )),
        _ => Ok(None),
    }
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

fn parse_quoted_text(value: &str) -> Option<String> {
    puzzle_authoring::parse_quoted_text(value)
}

struct ParsedScreenTransitionsBlock {
    transitions: Vec<SceneTransition>,
    puzzle_rule: Option<ScenePuzzleRule>,
}

fn parse_screen_transitions_block(
    lines: &[String],
    start: usize,
) -> Result<(ParsedScreenTransitionsBlock, usize), DiagnosticReport> {
    let mut transitions = Vec::new();
    let mut puzzle_rule = None;
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
            ["if"] | ["if", "all"] if lines[i].trim_end().ends_with('{') => {
                let (transition, next_i) = parse_screen_condition_arrow_block(lines, i)?;
                transitions.push(transition);
                i = next_i;
                continue;
            }
            _ if lines[i].contains("->") => {
                let (transition, next_i) = parse_transition_row(lines, i)?;
                transitions.push(transition);
                i = next_i;
                continue;
            }
            _ => {
                return Err(parse_error(
                    &lines[i],
                    "transitions row must be: step <puzzle> | <input> -> <effect> | if <condition> -> <effect>",
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "transitions missing closing brace",
        ));
    }

    Ok((
        ParsedScreenTransitionsBlock {
            transitions,
            puzzle_rule,
        },
        i + 1,
    ))
}

fn parse_screen_condition_arrow_block(
    lines: &[String],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let header = block_header_text(&lines[start]);
    match split_header_tokens(header).as_slice() {
        ["if"] | ["if", "all"] => {}
        ["if", "any"] => {
            return Err(parse_error(
                &lines[start],
                "scene condition blocks only support all conditions",
            ));
        }
        _ => {
            return Err(parse_error(
                &lines[start],
                "scene condition block must be: if [all] {",
            ));
        }
    }

    let mut conditions = Vec::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        conditions.push(parse_scene_condition_expr(&lines[i], &lines[i])?);
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "scene condition block missing closing brace",
        ));
    }
    if conditions.is_empty() {
        return Err(parse_error(
            &lines[start],
            "scene condition block requires at least one condition",
        ));
    }

    let arrow_i = i + 1;
    let Some(arrow_line) = lines.get(arrow_i) else {
        return Err(parse_error(
            &lines[start],
            "scene condition block must be followed by ->",
        ));
    };
    let (_, effect_text) =
        require_arrow_row(arrow_line, "scene condition block must be followed by ->")?;
    let (effect, next_i) =
        parse_scene_effect_with_optional_block(effect_text, lines, arrow_i)?;
    Ok((
        SceneTransition {
            trigger: SceneTransitionTrigger::Condition(combine_scene_condition_exprs(conditions)),
            effect,
        },
        next_i,
    ))
}

fn parse_screen_condition_block(
    lines: &[String],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let line = &lines[start];
    let condition = block_header_text(line)
        .strip_prefix("if ")
        .ok_or_else(|| parse_error(line, "condition block must be: if <condition>"))?
        .trim();
    let condition = parse_scene_condition_expr(condition, line)?;
    let (body, next_i) = collect_authoring_entry(lines, start)?;
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

fn parse_scene_lifecycle_block(
    lines: &[String],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    let [lifecycle @ "on_scene_start"] = tokens.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "scene lifecycle block must be: on_scene_start",
        ));
    };
    let (body, next_i) = collect_authoring_entry(lines, start)?;
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
    lines: &[String],
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
    let (body, next_i) = collect_authoring_entry(lines, start)?;
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
    lines: &[String],
    header_line: &str,
) -> Result<SceneEffect, DiagnosticReport> {
    parse_scene_handler_effects_range(lines, 0, lines.len(), header_line)
}

fn parse_scene_handler_effects_range(
    lines: &[String],
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
        _ => Ok(SceneEffect::Sequence(effects)),
    }
}

fn matching_effect_block_end(
    lines: &[String],
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

fn parse_transition_row(
    lines: &[String],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let (pattern, effect) = require_arrow_row(
        &lines[start],
        "transition must be: if <condition> -> <effect>",
    )?;
    let (effect, next_i) = parse_scene_effect_with_optional_block(effect, lines, start)?;
    Ok((
        SceneTransition {
            trigger: parse_transition_trigger(pattern.trim(), &lines[start])?,
            effect,
        },
        next_i,
    ))
}

fn parse_transition_trigger(
    value: &str,
    line: &str,
) -> Result<SceneTransitionTrigger, DiagnosticReport> {
    if value == "scene_start" {
        return Err(parse_error(
            line,
            "scene_start is a lifecycle block; write `on_scene_start { ... }` instead",
        ));
    }
    if value == "on_scene_start" {
        return Err(parse_error(
            line,
            "on_scene_start is a lifecycle block; write `on_scene_start { ... }` instead",
        ));
    }
    if value == "level_start" {
        return Err(parse_error(
            line,
            "level_start is a puzzle lifecycle block; put `on_level_start { ... }` inside puzzle",
        ));
    }
    if matches!(
        value,
        "on_level_start" | "on_level_clear" | "on_last_level_clear"
    ) {
        return Err(parse_error(
            line,
            "level lifecycle blocks belong inside puzzle",
        ));
    }
    if let Some(condition) = value.strip_prefix("if ") {
        let condition = condition.trim();
        return Ok(SceneTransitionTrigger::Condition(
            parse_scene_condition_expr(condition, line)?,
        ));
    }
    let tokens = split_header_tokens(value);
    if let [input] = tokens.as_slice() {
        let input = parse_input_name(input, line)?;
        return Ok(SceneTransitionTrigger::Condition(
            scene_input_condition_expr(&input),
        ));
    }
    Err(parse_error(
        line,
        "scene transition triggers must be `<input>` or `if <condition>`",
    ))
}

fn parse_scene_condition_expr(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    parse_scene_expr(value, line)
}

fn combine_scene_condition_exprs(mut conditions: Vec<SceneExpr>) -> SceneExpr {
    let first = conditions.remove(0);
    conditions
        .into_iter()
        .fold(first, |left, right| SceneExpr::Binary {
            op: SceneBinaryOp::And,
            left: Box::new(left),
            right: Box::new(right),
        })
}

fn scene_input_condition_expr(input: &str) -> SceneExpr {
    SceneExpr::Binary {
        op: SceneBinaryOp::Eq,
        left: Box::new(SceneExpr::Path(vec!["input".to_string()])),
        right: Box::new(SceneExpr::Path(vec![input.to_string()])),
    }
}

fn parse_level_menu_component(
    lines: &[String],
    start: usize,
) -> Result<(LevelMenuDef, usize), DiagnosticReport> {
    let next = start + 1;
    if !lines[start].trim_end().ends_with('{') {
        return Ok((LevelMenuDef::default(), next));
    }

    let mut menu = LevelMenuDef::default();
    let mut i = next;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["show_index", "=", value] => menu.show_index = parse_boolean_option(value, &lines[i])?,
            ["show_solved", "=", value] => {
                menu.show_cleared = parse_boolean_option(value, &lines[i])?
            }
            ["show_current" | "show_current_level", "=", _] => {
                return Err(parse_error(
                    &lines[i],
                    "level_menu no longer supports show_current_level",
                ));
            }
            ["layout", "=", "list"] => menu.columns = None,
            ["columns", "=", value] => {
                menu.columns = Some(parse_level_menu_columns(value, &lines[i])?)
            }
            ["wrap", "=", value] => menu.wrap = parse_boolean_option(value, &lines[i])?,
            ["locked", "=", "disabled"] => menu.locked = LevelMenuLocked::Disabled,
            ["locked", "=", "hidden"] => menu.locked = LevelMenuLocked::Hidden,
            ["button", ..] => {
                let (button, next_i) = parse_button_def(lines, i)?;
                menu.buttons.push(button);
                i = next_i;
                continue;
            }
            _ => {
                return Err(parse_error(
                    &lines[i],
                    "level_menu option must be: show_index = <true|false> | show_solved = <true|false> | show_current_level = <true|false> | layout = list | columns = <n> | wrap = <true|false> | locked = <disabled|hidden>",
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "level_menu missing closing brace",
        ));
    }

    Ok((menu, i + 1))
}

pub(crate) const LEVEL_MENU_OPTIONS: &[&str] = &[
    "show_index",
    "show_solved",
    "layout",
    "columns",
    "wrap",
    "locked",
];

fn parse_boolean_option(value: &str, line: &str) -> Result<bool, DiagnosticReport> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(parse_error(line, "boolean option must be true or false")),
    }
}

fn parse_level_menu_columns(value: &str, line: &str) -> Result<u16, DiagnosticReport> {
    let columns = value
        .parse::<u16>()
        .map_err(|_| parse_error(line, "columns must be an integer"))?;
    if columns == 0 {
        return Err(parse_error(
            line,
            "level_menu columns must be greater than 0",
        ));
    }
    Ok(columns)
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

fn parse_model_keys_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
    controls: &mut Controls,
) -> Result<usize, DiagnosticReport> {
    let mut seen_keys = HashSet::<KeyTrigger>::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let row = parse_keys_surface_row(&lines[i], "input", false)?;
        lower_model_keys_row(row, &lines[i], catalog, controls, &mut seen_keys)?;
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "keys missing closing brace"));
    }
    Ok(i + 1)
}

fn lower_model_keys_row(
    row: KeysSurfaceRow<'_>,
    line: &str,
    catalog: &mut Catalog,
    controls: &mut Controls,
    seen_keys: &mut HashSet<KeyTrigger>,
) -> Result<(), DiagnosticReport> {
    let input_tokens = split_header_tokens(row.target);
    let [input_name] = input_tokens.as_slice() else {
        return Err(parse_error(line, "keys row must be: <key...> -> <input>"));
    };
    let input = catalog
        .input_names
        .get(*input_name)
        .copied()
        .map(Ok)
        .unwrap_or_else(|| add_input_name(input_name, line, catalog))?;
    for key in row.keys {
        let trigger = parse_key_trigger(key, line)?;
        if !seen_keys.insert(trigger.clone()) {
            return Err(parse_error(line, "duplicate model input key"));
        }
        add_key_trigger_to_controls(&trigger, input, controls, line)?;
    }
    Ok(())
}

fn parse_scene_keys_block(
    lines: &[String],
    start: usize,
) -> Result<(Vec<KeyBinding>, usize), DiagnosticReport> {
    let mut bindings = Vec::<KeyBinding>::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let (binding, next_i) = parse_scene_key_binding_at(lines, i)?;
        bindings.push(binding);
        i = next_i;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "keys missing closing brace"));
    }
    Ok((bindings, i + 1))
}

fn parse_scene_key_binding_at(
    lines: &[String],
    start: usize,
) -> Result<(KeyBinding, usize), DiagnosticReport> {
    let row = parse_keys_surface_row(&lines[start], "scene effect-or-input", true)?;
    lower_scene_keys_row(row, lines, start)
}

fn lower_scene_keys_row(
    row: KeysSurfaceRow<'_>,
    lines: &[String],
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

fn add_default_key_controls(input_names: &HashMap<String, InputId>, controls: &mut Controls) {
    for (name, key, arrow) in [
        ("up", b'w', Some(ArrowKey::Up)),
        ("down", b's', Some(ArrowKey::Down)),
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
