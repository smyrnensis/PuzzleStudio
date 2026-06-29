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
        if depth == 1 && (is_braced_level_header(line) || matches!(tokens.as_slice(), ["{"])) {
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
        | ["scratch"]
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

fn parse_sounds_block(
    lines: &[String],
    start: usize,
    sounds: &mut SoundsDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["sounds"]) {
        return Err(parse_error(&lines[start], "sounds header must be: sounds"));
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            ["sfx", name, settings @ ..] => {
                validate_qualified_identifier(name, line, "sfx sounds name")?;
                if sounds.sfx.iter().any(|entry| entry.name == *name) {
                    return Err(parse_error(line, "duplicate sfx sounds name"));
                }
                let seed = required_sound_setting(settings, "seed", line)?;
                let type_target = optional_sound_setting(settings, "type").unwrap_or("random");
                let volume = parse_sound_f64(
                    optional_sound_setting(settings, "volume").unwrap_or("1"),
                    line,
                    "volume",
                )?;
                validate_sound_atom(seed, line, "sfx seed")?;
                validate_sound_atom(type_target, line, "sfx type")?;
                if !(0.0..=1.0).contains(&volume) {
                    return Err(parse_error(line, "sfx volume must be between 0 and 1"));
                }
                sounds.sfx.push(SfxSoundDef {
                    name: (*name).to_string(),
                    seed: seed.to_string(),
                    type_target: type_target.to_string(),
                    volume,
                });
            }
            ["music", name, settings @ ..] => {
                validate_qualified_identifier(name, line, "music sounds name")?;
                if sounds.music.iter().any(|entry| entry.name == *name) {
                    return Err(parse_error(line, "duplicate music sounds name"));
                }
                let seed = required_sound_setting(settings, "seed", line)?;
                validate_sound_atom(seed, line, "music seed")?;
                let height = parse_sound_f64(
                    optional_sound_setting(settings, "height")
                        .or_else(|| optional_sound_setting(settings, "tone"))
                        .unwrap_or("0.5"),
                    line,
                    "height",
                )?;
                let bars = parse_sound_u16(
                    optional_sound_setting(settings, "bars").unwrap_or("8"),
                    line,
                    "bars",
                )?;
                let bpm = parse_sound_u16(
                    optional_sound_setting(settings, "bpm").unwrap_or("110"),
                    line,
                    "bpm",
                )?;
                let volume = parse_sound_f64(
                    optional_sound_setting(settings, "volume").unwrap_or("0.5"),
                    line,
                    "volume",
                )?;
                if !(0.0..=1.0).contains(&height) {
                    return Err(parse_error(line, "music height must be between 0 and 1"));
                }
                if !matches!(bars, 8 | 16 | 32 | 64) {
                    return Err(parse_error(
                        line,
                        "music bars must be one of 8, 16, 32, or 64",
                    ));
                }
                if !(40..=180).contains(&bpm) {
                    return Err(parse_error(line, "music bpm must be between 40 and 180"));
                }
                if !(0.0..=1.0).contains(&volume) {
                    return Err(parse_error(line, "music volume must be between 0 and 1"));
                }
                sounds.music.push(MusicSoundDef {
                    name: (*name).to_string(),
                    seed: seed.to_string(),
                    height,
                    bars,
                    bpm,
                    volume,
                });
            }
            _ => {
                return Err(parse_error(
                    line,
                    "sounds entry must be: sfx <name> seed=<seed> type=<type> volume=<0..1> | music <name> seed=<seed> bars=<8|16|32|64> height=<0..1> bpm=<40..180> volume=<0..1>",
                ));
            }
        }
        i += 1;
    }

    Err(parse_error(&lines[start], "sounds missing closing brace"))
}

#[derive(Clone, Debug)]
struct ModelSoundTrigger {
    kind: ModelSoundTriggerKind,
    objects: Vec<ObjectId>,
    sfx_name: String,
}

#[derive(Clone, Debug)]
struct ModelSoundTriggerSpec {
    kind: ModelSoundTriggerKind,
    selector: String,
    sfx_name: String,
    line: String,
}

#[derive(Clone, Debug)]
struct ModelOperationSoundSpec {
    operation: ModelOperationSound,
    sfx_name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModelSoundTriggerKind {
    Move,
    CantMove,
}

fn model_sounds_block_starts(lines: &[String], start: usize) -> bool {
    lines.get(start + 1).is_some_and(|first| {
        matches!(
            split_header_tokens(first).as_slice(),
            ["move" | "cantmove", ..]
        )
    })
}

fn parse_model_sounds_block(
    lines: &[String],
    start: usize,
    triggers: &mut Vec<ModelSoundTriggerSpec>,
    operation_sounds: &mut Vec<ModelOperationSoundSpec>,
    allow_operation_sounds: bool,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["sounds"]) {
        return Err(parse_error(
            &lines[start],
            "model sounds header must be: sounds",
        ));
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        let trigger_kind = match tokens.as_slice() {
            ["move", ..] => Some(ModelSoundTriggerKind::Move),
            ["cantmove", ..] => Some(ModelSoundTriggerKind::CantMove),
            _ => None,
        };
        let operation = match tokens.as_slice() {
            ["undo", ..] => Some(ModelOperationSound::Undo),
            ["restart", ..] => Some(ModelOperationSound::Restart),
            _ => None,
        };
        match (trigger_kind, operation, tokens.as_slice()) {
            (Some(kind), _, [_, selector, "->", "sfx", name]) => {
                validate_qualified_identifier(name, line, "sfx name")?;
                triggers.push(ModelSoundTriggerSpec {
                    kind,
                    selector: (*selector).to_string(),
                    sfx_name: (*name).to_string(),
                    line: line.clone(),
                });
            }
            (_, Some(operation), [_, "->", "sfx", name]) if allow_operation_sounds => {
                validate_qualified_identifier(name, line, "sfx name")?;
                operation_sounds.push(ModelOperationSoundSpec {
                    operation,
                    sfx_name: (*name).to_string(),
                });
            }
            (_, Some(_), [_, "->", "sfx", _]) => {
                return Err(parse_error(
                    line,
                    "undo/restart sounds must be inside a puzzle sounds block",
                ));
            }
            _ => {
                return Err(parse_error(
                    line,
                    "model sounds entry must be: move <object-selector> -> sfx <name> | cantmove <object-selector> -> sfx <name> | undo -> sfx <name> | restart -> sfx <name>",
                ));
            }
        }
        i += 1;
    }

    Err(parse_error(
        &lines[start],
        "model sounds missing closing brace",
    ))
}

fn resolve_model_operation_sounds(
    specs: &[ModelOperationSoundSpec],
) -> Vec<ModelOperationSoundDef> {
    specs
        .iter()
        .map(|spec| ModelOperationSoundDef {
            operation: spec.operation,
            sfx_name: spec.sfx_name.clone(),
        })
        .collect()
}

fn resolve_model_sound_triggers(
    specs: &[ModelSoundTriggerSpec],
    catalog: &Catalog,
) -> Result<Vec<ModelSoundTrigger>, DiagnosticReport> {
    let value_sets = catalog_value_sets(catalog);
    specs
        .iter()
        .map(|spec| {
            let selector = resolve_object_selector(
                &spec.selector,
                &spec.line,
                &catalog.object_names,
                &catalog.object_schemas,
                &value_sets,
                &catalog.maps,
                &catalog.object_groups,
                &HashMap::new(),
            )
            .map_err(|error| model_sound_selector_error(error, spec))?;
            if selector
                .alternatives
                .iter()
                .any(|object| catalog.visual_objects.contains(object))
            {
                return Err(parse_error(
                    &spec.line,
                    "model sound triggers cannot target display objects",
                ));
            }
            Ok(ModelSoundTrigger {
                kind: spec.kind,
                objects: selector.alternatives,
                sfx_name: spec.sfx_name.clone(),
            })
        })
        .collect()
}

fn model_sound_selector_error(
    error: DiagnosticReport,
    spec: &ModelSoundTriggerSpec,
) -> DiagnosticReport {
    if error
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.message.starts_with("unknown object selector"))
    {
        parse_error(
            &spec.line,
            &format!(
                "unknown model sound trigger object selector `{}`",
                spec.selector
            ),
        )
    } else {
        error
    }
}

fn parse_theme_block(
    lines: &[String],
    start: usize,
    theme: &mut ThemeDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    match header.as_slice() {
        ["theme", name] => {
            parse_theme_name_directive(&lines[start], name, theme)?;
        }
        _ => {
            return Err(parse_error(
                &lines[start],
                "theme header must be: theme <theme> or theme <theme> {",
            ));
        }
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        match parse_theme_setting_tokens(tokens.as_slice(), line) {
            Ok(Some((name, value))) => upsert_theme_variable(theme, name, value),
            Ok(None) => {}
            Err(error) => return Err(error),
        }
        i += 1;
    }

    Err(parse_error(&lines[start], "theme missing closing brace"))
}

fn parse_theme_setting_tokens(
    tokens: &[&str],
    line: &str,
) -> Result<Option<(String, String)>, DiagnosticReport> {
    match tokens {
        [name, value] => {
            let name = normalize_theme_setting_name(name, line)?;
            validate_theme_value(value, line)?;
            Ok(Some((name, (*value).to_string())))
        }
        [name, "=", value] => {
            let name = normalize_theme_setting_name(name, line)?;
            validate_theme_value(value, line)?;
            Ok(Some((name, (*value).to_string())))
        }
        _ => Err(parse_error(
            line,
            "theme entry must be: <setting> <value> or <setting> = <value>",
        )),
    }
}

fn parse_theme_statement(
    lines: &[String],
    start: usize,
    theme: &mut ThemeDef,
) -> Result<usize, DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    let ["theme", name] = tokens.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "theme header must be: theme <theme> or theme <theme> {",
        ));
    };
    if lines[start].trim_end().ends_with('{')
        || lines
            .get(start + 1)
            .is_some_and(|line| is_block_close_line(line) || is_theme_setting_line(line))
    {
        return parse_theme_block(lines, start, theme);
    }
    parse_theme_name_directive(&lines[start], name, theme)?;
    Ok(start + 1)
}

fn parse_theme_name_directive(
    line: &str,
    name: &str,
    theme: &mut ThemeDef,
) -> Result<(), DiagnosticReport> {
    validate_qualified_identifier(name, line, "theme name")?;
    theme.name = Some(name.to_string());
    Ok(())
}

fn is_theme_setting_line(line: &str) -> bool {
    let tokens = split_header_tokens(line);
    parse_theme_setting_tokens(tokens.as_slice(), line).is_ok()
}

fn parse_assets_block(
    lines: &[String],
    start: usize,
    assets: &mut AssetsDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["assets"]) {
        return Err(parse_error(&lines[start], "assets header must be: assets"));
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            ["css", path] => assets.entries.push(AssetDef {
                kind: AssetKind::Css,
                path: parse_asset_path(path, line)?,
            }),
            ["script", path] => assets.entries.push(AssetDef {
                kind: AssetKind::Script,
                path: parse_asset_path(path, line)?,
            }),
            _ => {
                return Err(parse_error(
                    line,
                    "assets entry must be: css \"path\" | script \"path\"",
                ));
            }
        }
        i += 1;
    }
    Err(parse_error(&lines[start], "assets missing closing brace"))
}

fn parse_asset_path(token: &str, line: &str) -> Result<String, DiagnosticReport> {
    let path = token
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| parse_error(line, "asset path must be quoted"))?;
    if path.is_empty() {
        return Err(parse_error(line, "asset path must not be empty"));
    }
    if path.starts_with('/') || path.contains('\\') || path.split('/').any(|part| part == "..") {
        return Err(parse_error(
            line,
            "asset path must be a game-folder relative path",
        ));
    }
    Ok(path.to_string())
}

fn parse_metadata_text(line: &str, keyword: &str) -> Result<String, DiagnosticReport> {
    let Some(rest) = line.strip_prefix(keyword) else {
        return Err(parse_error(
            line,
            "metadata directive has the wrong keyword",
        ));
    };
    let value = rest.trim();
    if value.is_empty() {
        return Err(parse_error(line, "metadata value must not be empty"));
    }
    Ok(parse_quoted_text(value).unwrap_or_else(|| value.to_string()))
}

fn normalize_theme_setting_name(name: &str, line: &str) -> Result<String, DiagnosticReport> {
    let normalized = name
        .trim_start_matches("--")
        .replace('_', "-")
        .to_ascii_lowercase();
    for spec in THEME_SETTING_SPECS {
        if normalized == spec.canonical.replace('_', "-")
            || spec.aliases.iter().any(|alias| normalized == *alias)
        {
            return Ok(spec.css_variable.to_string());
        }
    }
    Err(parse_error(
        line,
        &format!(
            "theme setting must be one of: {}",
            THEME_SETTING_SPECS
                .iter()
                .map(|spec| spec.canonical)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ))
}

fn validate_theme_value(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    let is_safe_css_token = !value.is_empty()
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '#' | '.' | ',' | '%' | '(' | ')' | '-' | '_' | '/' | ':' | '+'
                )
        });
    if is_safe_css_token {
        Ok(())
    } else {
        Err(parse_error(
            line,
            "theme setting value must be a compact CSS token without spaces",
        ))
    }
}

fn upsert_theme_variable(theme: &mut ThemeDef, name: String, value: String) {
    if let Some(existing) = theme
        .variables
        .iter_mut()
        .find(|variable| variable.name == name)
    {
        existing.value = value;
    } else {
        theme.variables.push(ThemeVariableDef { name, value });
    }
}

fn required_sound_setting<'a>(
    settings: &'a [&'a str],
    key: &str,
    line: &str,
) -> Result<&'a str, DiagnosticReport> {
    optional_sound_setting(settings, key)
        .ok_or_else(|| parse_error(line, &format!("sounds setting `{key}` is required")))
}

fn optional_sound_setting<'a>(settings: &'a [&'a str], key: &str) -> Option<&'a str> {
    settings.iter().find_map(|setting| {
        let (found_key, value) = setting.split_once('=')?;
        (found_key == key).then_some(value)
    })
}

fn validate_sound_atom(value: &str, line: &str, label: &str) -> Result<(), DiagnosticReport> {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch == '_' || ch == '-' || ch == '.' || ch.is_ascii_alphanumeric())
    {
        Ok(())
    } else {
        Err(parse_error(
            line,
            &format!("{label} must be a compact atom"),
        ))
    }
}

fn parse_sound_f64(value: &str, line: &str, label: &str) -> Result<f64, DiagnosticReport> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| parse_error(line, &format!("{label} must be a number")))?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(parse_error(line, &format!("{label} must be finite")))
    }
}

fn parse_sound_u16(value: &str, line: &str, label: &str) -> Result<u16, DiagnosticReport> {
    value
        .parse::<u16>()
        .map_err(|_| parse_error(line, &format!("{label} must be u16")))
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
    level_blocks: &mut Vec<LevelBlock>,
    render_overlays: &mut OverlayDefs,
    model_sound_triggers: &mut Vec<ModelSoundTriggerSpec>,
    model_operation_sounds: &mut Vec<ModelOperationSoundSpec>,
    named_conditions: &mut HashMap<String, (String, ConditionAst)>,
    run_rules_on_level_start: &mut bool,
    visuals: &mut VisualsDef,
    render: &mut PuzzleRenderDef,
    animation: &mut AnimationDef,
    puzzle_screen: &mut PuzzleScreenDef,
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
                i = parse_tags_block(lines, i, catalog)?;
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
            "scratch" => {
                i = parse_scratch_block(lines, i, catalog)?;
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
                if tokens.len() == 1 {
                    diagnostics.extend(
                        parse_error(line, "`group { ... }` was removed; use `groups { ... }`")
                            .into_diagnostics(),
                    );
                    i = skip_logical_block(lines, i);
                } else {
                    parse_group_directive(
                        &tokens,
                        line,
                        &catalog.object_names,
                        &catalog.object_schemas,
                        &catalog_value_sets(&catalog),
                        &catalog.maps,
                        &catalog.visual_objects,
                        &mut catalog.object_groups,
                    )?;
                    i += 1;
                }
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
                    "display blocks are not supported; use `display <rule>` inside transitions, on_level_start, or on_level_clear",
                ).into_diagnostics());
                i = recover_after_directive_error(lines, i);
            }
            "levels" => {
                i = parse_levels_block(
                    lines,
                    i,
                    level_blocks,
                    catalog,
                    render_overlays,
                    empty_char,
                    Some(name),
                )?;
            }
            "level" => {
                let (level, next_i) = parse_level_block(lines, i, level_blocks.len())?;
                level_blocks.push(level);
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

fn parse_level_block(
    lines: &[String],
    start: usize,
    existing_count: usize,
) -> Result<(LevelBlock, usize), DiagnosticReport> {
    let level_name =
        parse_level_header_name_or_auto(&lines[start], unnamed_level_name(existing_count))?;
    parse_named_level_body(lines, start, level_name, &LevelsHeader::default())
}

fn parse_levels_block(
    lines: &[String],
    start: usize,
    level_blocks: &mut Vec<LevelBlock>,
    catalog: &mut Catalog,
    render_overlays: &mut OverlayDefs,
    empty_char: &mut Option<char>,
    default_puzzle: Option<&str>,
) -> Result<usize, DiagnosticReport> {
    let header = parse_levels_header(&lines[start], default_puzzle)?;
    let mut namespace_count = 0usize;
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["legend"] => {
                i = parse_legend_block(lines, i, catalog, render_overlays, empty_char)?;
            }
            ["legend", ..] => {
                parse_legend_directive(
                    &tokens,
                    &lines[i],
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                    &mut catalog.render_chars,
                    &mut catalog.char_objects,
                    render_overlays,
                )?;
                i += 1;
            }
            ["level", ..] => {
                namespace_count += 1;
                let auto_name = namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    level_blocks.len(),
                    namespace_count,
                );
                let level_name = parse_level_header_name_or_auto(&lines[i], auto_name)
                    .map(|name| namespaced_level_name_if_needed(header.pack.as_deref(), name))?;
                let (level, next_i) = if is_braced_level_header(&lines[i]) {
                    parse_named_level_body(lines, i, level_name, &header)?
                } else {
                    parse_unbraced_level_body(lines, i + 1, level_name, &header)?
                };
                level_blocks.push(level);
                i = next_i;
            }
            ["{"] => {
                namespace_count += 1;
                let name = namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    level_blocks.len(),
                    namespace_count,
                );
                let (level, next_i) = parse_named_level_body(lines, i, name, &header)?;
                level_blocks.push(level);
                i = next_i;
            }
            [] => i += 1,
            _ if lines[i].trim_end().ends_with('{') => {
                return Err(parse_error(
                    &lines[i],
                    "braced level header must be `level <name> {` or `{` for an unnamed level",
                ));
            }
            _ => {
                namespace_count += 1;
                let name = namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    level_blocks.len(),
                    namespace_count,
                );
                let (level, next_i) = parse_unbraced_level_body(lines, i, name, &header)?;
                level_blocks.push(level);
                i = next_i;
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "levels missing closing brace"));
    }

    Ok(i + 1)
}

#[derive(Clone, Debug, Default)]
struct LevelsHeader {
    pack: Option<String>,
    puzzle: Option<String>,
}

fn parse_levels_header(
    line: &str,
    default_puzzle: Option<&str>,
) -> Result<LevelsHeader, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        ["levels"] => Ok(LevelsHeader {
            pack: None,
            puzzle: default_puzzle.map(str::to_string),
        }),
        ["levels", "of", puzzle] => {
            validate_qualified_identifier(puzzle, line, "levels puzzle")?;
            Ok(LevelsHeader {
                pack: None,
                puzzle: Some((*puzzle).to_string()),
            })
        }
        ["levels", pack, "of", puzzle] => {
            validate_qualified_identifier(pack, line, "levels pack")?;
            validate_qualified_identifier(puzzle, line, "levels puzzle")?;
            Ok(LevelsHeader {
                pack: Some((*pack).to_string()),
                puzzle: Some((*puzzle).to_string()),
            })
        }
        _ => Err(parse_error(
            line,
            "levels header must be: levels, levels of <puzzle>, or levels <pack> of <puzzle>",
        )),
    }
}

fn resolve_level_block_puzzles(
    levels: &mut [LevelBlock],
    puzzle_models: &[String],
) -> Result<(), DiagnosticReport> {
    let unique_models = puzzle_models.iter().collect::<HashSet<_>>();
    for level in levels {
        if let Some(puzzle) = &level.puzzle {
            if !unique_models.contains(puzzle) {
                return Err(DiagnosticReport::error(format!(
                    "levels target unknown puzzle model: {puzzle}"
                )));
            }
            continue;
        }
        match unique_models.len() {
            0 => {
                return Err(DiagnosticReport::error(
                    "bare levels requires one puzzle definition".to_string(),
                ));
            }
            1 => {
                level.puzzle = unique_models.iter().next().map(|name| (*name).clone());
            }
            _ => {
                return Err(DiagnosticReport::error(
                    "bare levels is ambiguous with multiple puzzle models; use `levels of <puzzle>` or `levels <pack> of <puzzle>`".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn parse_level_header_name_or_auto(
    line: &str,
    auto_name: String,
) -> Result<String, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    if tokens.len() == 1 {
        return Ok(auto_name);
    }
    if tokens.len() < 2 {
        return Err(parse_error(line, "level header must be: level <name>"));
    }
    Ok(tokens[1..].join(" "))
}

fn is_braced_level_header(line: &str) -> bool {
    line.trim_end().ends_with('{') && matches!(split_header_tokens(line).as_slice(), ["level", ..])
}

fn unnamed_level_name(existing_count: usize) -> String {
    format!("unnamed level {}", existing_count + 1)
}

fn namespaced_level_name_if_needed(namespace: Option<&str>, name: String) -> String {
    match namespace {
        Some(namespace) if !name.starts_with(&format!("{namespace}.")) => {
            format!("{namespace}.{name}")
        }
        _ => name,
    }
}

fn namespaced_unnamed_level_name(
    namespace: Option<&str>,
    existing_count: usize,
    namespace_count: usize,
) -> String {
    match namespace {
        Some(namespace) => format!("{namespace}.{namespace_count}"),
        None => unnamed_level_name(existing_count),
    }
}

fn parse_conditions_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    named_conditions: &mut HashMap<String, (String, ConditionAst)>,
) -> Result<usize, DiagnosticReport> {
    let header_tokens = split_header_tokens(&lines[start]);
    let condition_name = header_tokens.first().copied().unwrap_or("win_conditions");
    let combinator = match header_tokens.as_slice() {
        [_] => ConditionBlockCombinator::All,
        [_, "all"] => ConditionBlockCombinator::All,
        [_, "any"] => ConditionBlockCombinator::Any,
        _ => {
            return Err(parse_error(
                &lines[start],
                &format!("{condition_name} block must be: {condition_name} [all | any]"),
            ));
        }
    };
    if named_conditions.contains_key(condition_name) {
        return Err(parse_error(
            &lines[start],
            &format!("duplicate {condition_name} definition"),
        ));
    }

    let mut conditions = Vec::new();
    let mut descriptions = Vec::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        i = parse_condition_block_entry(
            lines,
            i,
            condition_name,
            catalog,
            &mut conditions,
            &mut descriptions,
        )?;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            &format!("{condition_name} missing closing brace"),
        ));
    }
    if conditions.is_empty() {
        return Err(parse_error(
            &lines[start],
            &format!("{condition_name} requires at least one condition"),
        ));
    }

    named_conditions.insert(
        condition_name.to_string(),
        (
            descriptions.join(combinator.description_joiner()),
            if conditions.len() == 1 {
                conditions.remove(0)
            } else {
                combinator.combine(conditions)
            },
        ),
    );
    Ok(i + 1)
}

fn parse_condition_block_entry(
    lines: &[String],
    start: usize,
    condition_name: &str,
    catalog: &Catalog,
    conditions: &mut Vec<ConditionAst>,
    descriptions: &mut Vec<String>,
) -> Result<usize, DiagnosticReport> {
    let line = &lines[start];
    let tokens = split_header_tokens(line);
    if matches!(tokens.as_slice(), ["for", _, "in", ..]) {
        let ["for", binding, "in", sources @ ..] = tokens.as_slice() else {
            unreachable!("checked by matches");
        };
        let value_sets = catalog_value_sets(catalog);
        let values =
            for_expansion_values(sources, &value_sets, &catalog.numeric_global_defaults, line)?;
        validate_identifier(binding, line, "expansion binding")?;
        let (body_lines, next_i) = collect_statement_block_lines(lines, start + 1, line)?;
        for value in values {
            let expanded_lines = expand_for_binding_lines(
                &body_lines,
                binding,
                value.axis.as_deref(),
                &value.value,
                &catalog.maps,
            )?;
            parse_condition_rows(
                &expanded_lines,
                condition_name,
                catalog,
                conditions,
                descriptions,
            )?;
        }
        return Ok(next_i);
    }

    let condition = parse_condition_block_row(line, condition_name, catalog)?;
    descriptions.push(line.clone());
    conditions.push(condition);
    Ok(start + 1)
}

fn parse_condition_rows(
    lines: &[String],
    condition_name: &str,
    catalog: &Catalog,
    conditions: &mut Vec<ConditionAst>,
    descriptions: &mut Vec<String>,
) -> Result<(), DiagnosticReport> {
    let mut i = 0;
    while i < lines.len() {
        i = parse_condition_block_entry(
            lines,
            i,
            condition_name,
            catalog,
            conditions,
            descriptions,
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConditionBlockCombinator {
    All,
    Any,
}

impl ConditionBlockCombinator {
    fn description_joiner(self) -> &'static str {
        match self {
            Self::All => " and ",
            Self::Any => " or ",
        }
    }

    fn combine(self, conditions: Vec<ConditionAst>) -> ConditionAst {
        match self {
            Self::All => ConditionAst::All(conditions),
            Self::Any => ConditionAst::Any(conditions),
        }
    }
}

fn parse_puzzle_screen_block(
    lines: &[String],
    start: usize,
    puzzle_screen: &mut PuzzleScreenDef,
) -> Result<usize, DiagnosticReport> {
    let mut parsed = puzzle_screen.clone();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            _ => {
                parse_puzzle_screen_directive(line, &mut parsed)?;
                i += 1;
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "puzzle screen missing closing brace",
        ));
    }
    validate_puzzle_screen(&parsed, &lines[start])?;
    *puzzle_screen = parsed;
    Ok(i + 1)
}

fn parse_puzzle_render_block(
    lines: &[String],
    start: usize,
    render: &mut PuzzleRenderDef,
) -> Result<usize, DiagnosticReport> {
    let mut parsed = render.clone();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name] if *name == PUZZLE_RENDER_BLOCK_OPTIONS[0] => {
                i = parse_puzzle_render_grid_block(lines, i, &mut parsed.grid)?;
            }
            [name, options @ ..] if *name == PUZZLE_RENDER_BLOCK_OPTIONS[0] => {
                parse_puzzle_render_grid_options(options, line, &mut parsed.grid)?;
                i += 1;
            }
            [other, ..] => {
                return Err(parse_error(
                    line,
                    &format!("unknown render directive {other}"),
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "render block missing closing brace",
        ));
    }
    *render = parsed;
    Ok(i + 1)
}

pub(crate) const PUZZLE_RENDER_BLOCK_OPTIONS: &[&str] = &["grid"];
pub(crate) const PUZZLE_RENDER_GRID_OPTIONS: &[&str] = &["occupied_cells", "all_cells"];
pub(crate) const ANIMATION_BLOCK_OPTIONS: &[&str] = &["tween"];
pub(crate) const ANIMATION_TWEEN_OPTIONS: &[&str] = &["duration"];

fn parse_animation_block(
    lines: &[String],
    start: usize,
    animation: &mut AnimationDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["animation"]) {
        return Err(parse_error(
            &lines[start],
            "animation header must be: animation",
        ));
    }

    let mut parsed = animation.clone();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name] if *name == ANIMATION_BLOCK_OPTIONS[0] => {
                parsed.tween.enabled = true;
                if lines
                    .get(i + 1)
                    .is_some_and(|next| is_block_close_line(next))
                {
                    i += 1;
                } else {
                    i = parse_animation_tween_block(lines, i, &mut parsed.tween)?;
                }
            }
            [name, options @ ..] if *name == ANIMATION_BLOCK_OPTIONS[0] => {
                parsed.tween.enabled = true;
                parse_animation_tween_options(options, line, &mut parsed.tween)?;
                i += 1;
            }
            [other, ..] => {
                return Err(parse_error(
                    line,
                    &format!("unknown animation directive {other}"),
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "animation block missing closing brace",
        ));
    }
    *animation = parsed;
    Ok(i + 1)
}

fn parse_animation_tween_block(
    lines: &[String],
    start: usize,
    tween: &mut TweenAnimationDef,
) -> Result<usize, DiagnosticReport> {
    let mut parsed = tween.clone();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name, "=", value] | [name, value] if *name == ANIMATION_TWEEN_OPTIONS[0] => {
                parsed.interval_ms = parse_animation_duration_ms(value, line)?;
                i += 1;
            }
            [other, ..] => {
                return Err(parse_error(line, &format!("unknown tween setting {other}")));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "tween block missing closing brace",
        ));
    }
    *tween = parsed;
    Ok(i + 1)
}

fn parse_animation_tween_options(
    options: &[&str],
    line: &str,
    tween: &mut TweenAnimationDef,
) -> Result<(), DiagnosticReport> {
    if options.is_empty() {
        return Err(parse_error(
            line,
            "tween directive requires at least one option",
        ));
    }
    for option in options {
        let Some((name, value)) = option.split_once('=') else {
            return Err(parse_error(
                line,
                "tween option must be name=value in inline form",
            ));
        };
        match name {
            name if name == ANIMATION_TWEEN_OPTIONS[0] && !value.is_empty() => {
                tween.interval_ms = parse_animation_duration_ms(value, line)?;
            }
            name if name == ANIMATION_TWEEN_OPTIONS[0] => {
                return Err(parse_error(line, "tween duration must not be empty"));
            }
            other => return Err(parse_error(line, &format!("unknown tween setting {other}"))),
        }
    }
    Ok(())
}

fn parse_animation_duration_ms(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    let milliseconds = parse_wait_duration_ms(value, line)?;
    if milliseconds == 0 {
        return Err(parse_error(line, "tween duration must be greater than 0"));
    }
    Ok(milliseconds)
}

fn parse_puzzle_render_grid_block(
    lines: &[String],
    start: usize,
    grid: &mut PuzzleGridRenderDef,
) -> Result<usize, DiagnosticReport> {
    let mut parsed = grid.clone();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name] if *name == PUZZLE_RENDER_GRID_OPTIONS[0] => {
                parsed.occupied_cells = true;
                i += 1;
            }
            [name] if *name == PUZZLE_RENDER_GRID_OPTIONS[1] => {
                parsed.all_cells = true;
                i += 1;
            }
            [other, ..] => {
                return Err(parse_error(line, &format!("unknown grid setting {other}")));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "grid block missing closing brace",
        ));
    }
    *grid = parsed;
    Ok(i + 1)
}

fn parse_puzzle_render_grid_options(
    options: &[&str],
    line: &str,
    grid: &mut PuzzleGridRenderDef,
) -> Result<(), DiagnosticReport> {
    if options.is_empty() {
        return Err(parse_error(
            line,
            "grid directive requires at least one option",
        ));
    }
    for option in options {
        match *option {
            option if option == PUZZLE_RENDER_GRID_OPTIONS[0] => grid.occupied_cells = true,
            option if option == PUZZLE_RENDER_GRID_OPTIONS[1] => grid.all_cells = true,
            other => return Err(parse_error(line, &format!("unknown grid setting {other}"))),
        }
    }
    Ok(())
}

fn parse_puzzle_screen_directive(
    line: &str,
    puzzle_screen: &mut PuzzleScreenDef,
) -> Result<(), DiagnosticReport> {
    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        ["flickscreen", "full"] => {
            puzzle_screen.viewport_size = ViewportSizeDef::Full;
            puzzle_screen.viewport_mode = ViewportModeDef::Paged;
        }
        ["flickscreen", ..] => {
            let (width, height) = parse_screen_size_directive(line, "flickscreen")?;
            puzzle_screen.viewport_size = ViewportSizeDef::Size { width, height };
            puzzle_screen.viewport_mode = ViewportModeDef::Paged;
        }
        ["zoomscreen", ..] => {
            let (width, height) = parse_screen_size_directive(line, "zoomscreen")?;
            puzzle_screen.viewport_size = ViewportSizeDef::Size { width, height };
            puzzle_screen.viewport_mode = ViewportModeDef::Centered;
        }
        ["screen_focus", selector] => {
            validate_identifier(selector, line, "viewport focus selector")?;
            puzzle_screen.viewport_focus = (*selector).to_string();
        }
        ["frame_focus", ..] | ["frame_size", ..] | ["switch_frame"] | ["follow_frame"] => {
            return Err(parse_error(
                line,
                "`frame_*` screen directives were removed; use `flickscreen`, `zoomscreen`, or `screen_focus`",
            ));
        }
        [other, ..] => {
            return Err(parse_error(
                line,
                &format!("unknown puzzle screen directive {other}"),
            ));
        }
        [] => {}
    }
    Ok(())
}

fn validate_puzzle_screen(
    puzzle_screen: &PuzzleScreenDef,
    line: &str,
) -> Result<(), DiagnosticReport> {
    if !matches!(puzzle_screen.viewport_size, ViewportSizeDef::Size { .. })
        && puzzle_screen.viewport_mode == ViewportModeDef::Centered
    {
        return Err(parse_error(
            line,
            "centered viewport requires `zoomscreen <w> <h>`",
        ));
    }
    Ok(())
}

fn parse_screen_size_directive(
    line: &str,
    directive: &str,
) -> Result<(u16, u16), DiagnosticReport> {
    let value = line
        .strip_prefix(directive)
        .map(str::trim)
        .unwrap_or_default();
    if value == "full" || value == "region" {
        return Err(parse_error(
            line,
            &format!("{directive} {value} is not supported"),
        ));
    }
    if value.starts_with('(') {
        return parse_u16_tuple(value, line, directive);
    }
    if let Some((width, height)) = value.split_once('x').or_else(|| value.split_once('X')) {
        return Ok((
            width
                .trim()
                .parse::<u16>()
                .map_err(|_| parse_error(line, &format!("{directive} width must be u16")))?,
            height
                .trim()
                .parse::<u16>()
                .map_err(|_| parse_error(line, &format!("{directive} height must be u16")))?,
        ));
    }
    let size_tokens = split_header_tokens(value);
    let [width, height] = size_tokens.as_slice() else {
        return Err(parse_error(
            line,
            &format!("{directive} must be: {directive} (w, h)"),
        ));
    };
    Ok((
        parse_u16(Some(width), line, "missing screen width")?,
        parse_u16(Some(height), line, "missing screen height")?,
    ))
}

fn parse_u16_tuple(value: &str, line: &str, name: &str) -> Result<(u16, u16), DiagnosticReport> {
    let inner = value
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| parse_error(line, &format!("{name} tuple must be: (w,h)")))?;
    let Some((left, right)) = inner.split_once(',') else {
        return Err(parse_error(line, &format!("{name} tuple must be: (w,h)")));
    };
    let width = left
        .trim()
        .parse::<u16>()
        .map_err(|_| parse_error(line, &format!("{name} width must be u16")))?;
    let height = right
        .trim()
        .parse::<u16>()
        .map_err(|_| parse_error(line, &format!("{name} height must be u16")))?;
    Ok((width, height))
}

fn parse_condition_block_row(
    line: &str,
    condition_name: &str,
    catalog: &Catalog,
) -> Result<ConditionAst, DiagnosticReport> {
    if let Some(pattern) = line.trim().strip_prefix("some ") {
        let pattern = pattern.trim();
        if let Some(pattern) = parse_condition_pattern_arg(
            pattern,
            line,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
        )? {
            return Ok(ConditionAst::InlineConditionNonZero(
                ConditionValueAst::ExistsMatches(pattern),
            ));
        }
    }
    if let Some(pattern) = line.trim().strip_prefix("no ") {
        let pattern = pattern.trim();
        if let Some(pattern) = parse_condition_pattern_arg(
            pattern,
            line,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
        )? {
            return Ok(ConditionAst::InlineConditionNonZero(
                ConditionValueAst::NoneMatches(pattern),
            ));
        }
    }

    if let Ok(condition) = parse_condition_expr(
        line,
        line,
        &catalog.input_names,
        &catalog.global_names,
        &catalog.condition_names,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.maps,
        &catalog.object_groups,
    ) {
        return Ok(condition);
    }

    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        ["all", target, "on", cover] => {
            let expr = format!("none([ {target} no {cover} ])");
            parse_condition_expr(
                &expr,
                line,
                &catalog.input_names,
                &catalog.global_names,
                &catalog.condition_names,
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
            )
        }
        ["some", target, "on", cover] => {
            let expr = format!("exists([ {target} {cover} ])");
            parse_condition_expr(
                &expr,
                line,
                &catalog.input_names,
                &catalog.global_names,
                &catalog.condition_names,
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
            )
        }
        ["some", target] => {
            let expr = format!("exists({target})");
            parse_condition_expr(
                &expr,
                line,
                &catalog.input_names,
                &catalog.global_names,
                &catalog.condition_names,
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
            )
        }
        ["no", target] => {
            let expr = format!("none({target})");
            parse_condition_expr(
                &expr,
                line,
                &catalog.input_names,
                &catalog.global_names,
                &catalog.condition_names,
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
            )
        }
        _ => Err(parse_error(
            line,
            &format!(
                "{condition_name} row must be a condition expression, all <object> on <object>, some/no [pattern], some <object> on <object>, or some/no <object>"
            ),
        )),
    }
}

fn parse_named_level_body(
    lines: &[String],
    start: usize,
    name: String,
    header: &LevelsHeader,
) -> Result<(LevelBlock, usize), DiagnosticReport> {
    let mut level_lines = Vec::new();
    let mut i = start + 1;
    let mut nested_blocks = 0usize;
    while i < lines.len() {
        if is_block_close_line(&lines[i]) {
            if nested_blocks == 0 {
                break;
            }
            nested_blocks -= 1;
            level_lines.push(lines[i].clone());
            i += 1;
            continue;
        }
        if is_level_body_block(&split_header_tokens(&lines[i])) {
            nested_blocks += 1;
        }
        level_lines.push(lines[i].clone());
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "level missing closing brace"));
    }

    Ok((
        LevelBlock {
            name,
            pack: header.pack.clone(),
            puzzle: header.puzzle.clone(),
            lines: level_lines,
        },
        i + 1,
    ))
}

fn parse_unbraced_level_body(
    lines: &[String],
    start: usize,
    name: String,
    header: &LevelsHeader,
) -> Result<(LevelBlock, usize), DiagnosticReport> {
    let mut level_lines = Vec::new();
    let mut i = start;
    let mut nested_blocks = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        if nested_blocks == 0 && (line.is_empty() || is_block_close_line(line)) {
            break;
        }
        let tokens = split_header_tokens(line);
        if nested_blocks == 0 && matches!(tokens.as_slice(), ["level", ..]) {
            if !level_lines.is_empty() {
                break;
            }
            return Err(parse_error(
                line,
                "unbraced levels must be separated by a blank line",
            ));
        }
        if is_block_close_line(line) {
            nested_blocks = nested_blocks.saturating_sub(1);
            level_lines.push(line.clone());
            i += 1;
            continue;
        }
        if is_level_body_block(&tokens) {
            nested_blocks += 1;
        }
        level_lines.push(line.clone());
        i += 1;
    }
    if level_lines.is_empty() {
        return Err(parse_error(
            &lines[start.saturating_sub(1)],
            "level requires at least one row",
        ));
    }

    Ok((
        LevelBlock {
            name,
            pack: header.pack.clone(),
            puzzle: header.puzzle.clone(),
            lines: level_lines,
        },
        i,
    ))
}

fn is_level_body_block(tokens: &[&str]) -> bool {
    matches!(tokens, ["legend"] | ["on_level_start"] | ["on_level_clear"])
}

#[derive(Clone, Debug)]
struct PreparedLevelBody {
    name: String,
    pack: Option<String>,
    puzzle: String,
    lines: Vec<String>,
    char_objects: HashMap<char, Vec<ObjectId>>,
    level_start_statements: Vec<StatementAst>,
    level_clear_statements: Vec<StatementAst>,
}

#[derive(Clone, Debug, Default)]
struct ParsedLevelBody {
    lines: Vec<String>,
    local_char_objects: HashMap<char, Vec<ObjectId>>,
    level_start_statements: Vec<StatementAst>,
    level_clear_statements: Vec<StatementAst>,
}

#[allow(clippy::too_many_arguments)]
fn parse_level_body(
    level: &LevelBlock,
    catalog: &Catalog,
    empty_char: char,
    default_wait_ms: u64,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
) -> Result<ParsedLevelBody, DiagnosticReport> {
    let mut body = ParsedLevelBody::default();
    let mut saw_map_row = false;
    let mut i = 0;
    while i < level.lines.len() {
        let line = &level.lines[i];
        let tokens = split_header_tokens(line);
        if tokens.is_empty() {
            if saw_map_row {
                body.lines.push(line.clone());
            }
            i += 1;
            continue;
        }

        if matches!(tokens.as_slice(), ["on_level_start"] | ["on_level_clear"]) {
            let (statements, next_i) = parse_statement_block(
                &level.lines,
                None,
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
            )?;
            if tokens[0] == "on_level_start" {
                body.level_start_statements.extend(statements);
            } else {
                body.level_clear_statements.extend(statements);
            }
            i = next_i;
            continue;
        }
        if tokens[0] == "on_level_start" || tokens[0] == "on_level_clear" {
            return Err(parse_error(
                line,
                "level lifecycle block header must be: on_level_start | on_level_clear",
            ));
        }

        if let Some(statement) = parse_level_event_sugar(line, default_wait_ms)? {
            if saw_map_row {
                body.level_clear_statements.push(statement);
            } else {
                body.level_start_statements.push(statement);
            }
            i += 1;
            continue;
        }

        if tokens[0] != "legend" {
            saw_map_row = true;
            body.lines.push(line.clone());
            i += 1;
            continue;
        }

        if tokens.len() == 1 {
            i += 1;
            while i < level.lines.len() && !is_block_close_line(&level.lines[i]) {
                parse_level_legend_block_row(
                    &level.lines[i],
                    catalog,
                    empty_char,
                    &mut body.local_char_objects,
                )?;
                i += 1;
            }
            if i >= level.lines.len() {
                return Err(parse_error(line, "level legend missing closing brace"));
            }
            i += 1;
            continue;
        }

        let (ch, objects) = parse_level_legend_directive(&tokens, line, catalog, empty_char)?;
        body.local_char_objects.insert(ch, objects);
        i += 1;
    }

    Ok(body)
}

fn parse_level_event_sugar(
    line: &str,
    default_wait_ms: u64,
) -> Result<Option<StatementAst>, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    let is_level_event = matches!(tokens.as_slice(), ["sfx", _] | ["wait"] | ["wait", _])
        || line.strip_prefix("message ").is_some();
    if !is_level_event {
        return Ok(None);
    }
    let effects = parse_rewrite_effect(line, line)?;
    if effects.iter().any(|effect| {
        !matches!(
            effect,
            EffectAst::PlaySfx { .. }
                | EffectAst::Wait { .. }
                | EffectAst::WaitAnimation
                | EffectAst::Message { .. }
        )
    }) {
        return Err(parse_error(
            line,
            "level body sugar only supports message, sfx, and wait; put other behavior in on_level_start/on_level_clear",
        ));
    }
    let effects = effects
        .into_iter()
        .map(|effect| match effect {
            EffectAst::Wait { milliseconds: None } => EffectAst::Wait {
                milliseconds: Some(default_wait_ms),
            },
            other => other,
        })
        .collect();
    Ok(Some(StatementAst::Effect {
        source_line: line.to_string(),
        effects,
    }))
}

fn parse_level_legend_block_row(
    line: &str,
    catalog: &Catalog,
    empty_char: char,
    local_char_objects: &mut HashMap<char, Vec<ObjectId>>,
) -> Result<(), DiagnosticReport> {
    let tokens = split_header_tokens(line);
    if tokens.len() < 3 || tokens.get(1).copied() != Some("=") {
        return Err(parse_error(
            line,
            "level legend row must be: <char> = <selector...>",
        ));
    }

    let mut directive_tokens = vec!["legend"];
    directive_tokens.extend(tokens);
    let (ch, objects) = parse_level_legend_directive(&directive_tokens, line, catalog, empty_char)?;
    local_char_objects.insert(ch, objects);
    Ok(())
}

fn parse_level_legend_directive(
    tokens: &[&str],
    line: &str,
    catalog: &Catalog,
    empty_char: char,
) -> Result<(char, Vec<ObjectId>), DiagnosticReport> {
    if tokens.len() < 4 || tokens.get(2).copied() != Some("=") {
        return Err(parse_error(
            line,
            "level legend must be: legend <char> = <selector...>",
        ));
    }

    let ch = parse_char(tokens.get(1), line, "missing legend char")?;
    if ch == empty_char || tokens[3..] == ["empty"] {
        return Err(parse_error(line, "level-local legend cannot define empty"));
    }
    let selector_sets = selector_sets(
        &tokens[3..],
        line,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.maps,
        &catalog.object_groups,
    )?;
    let combinations = cartesian_object_product(&selector_sets);

    let level_objects = if selector_sets.len() == 1 && selector_sets[0].len() == 1 {
        Some(vec![selector_sets[0][0]])
    } else if selector_sets.len() > 1 && combinations.len() == 1 {
        Some(combinations[0].clone())
    } else {
        None
    };
    let Some(objects) = level_objects else {
        return Err(parse_error(
            line,
            "level-local legend must resolve to one concrete object set",
        ));
    };

    Ok((ch, objects))
}

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
    let expanded_values =
        expand_numeric_ranges_in_value_list(values, &catalog.numeric_global_defaults, line)?;
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
    Ok(())
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
    let Some((_, expr)) = line.split_once('=') else {
        return Err(parse_error(line, "assignment must be: <name> = <value>"));
    };
    let expr = expr.trim();
    if looks_like_condition_expr(expr) {
        if named_conditions.contains_key(name) {
            return Err(parse_error(line, "duplicate condition"));
        }
        let condition = parse_condition_expr(
            expr,
            line,
            &catalog.input_names,
            &catalog.global_names,
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

fn display_object_spec<'a>(
    tokens: &[&'a str],
    index: &mut usize,
    line: &str,
) -> Result<Option<&'a str>, DiagnosticReport> {
    if tokens.get(*index).copied() != Some("display") {
        return Ok(None);
    }
    *index += 1;
    let spec = tokens
        .get(*index)
        .copied()
        .ok_or_else(|| parse_error(line, "`display` must be followed by a display object"))?;
    if !is_display_role_token(spec) {
        return Err(parse_error(line, "display object must use an @ name"));
    }
    *index += 1;
    Ok(Some(spec))
}

fn is_display_role_token(token: &str) -> bool {
    puzzle_authoring::is_display_object_token(token)
}

fn validate_selector_alias_name(
    value: &str,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    if is_display_role_token(value) || is_qualified_identifier(value) {
        Ok(())
    } else {
        Err(parse_error(
            line,
            &format!("{label} must be a qualified identifier or @name"),
        ))
    }
}

fn validate_rule_name(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    if is_display_role_token(value) || is_qualified_identifier(value) {
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
        define_object_spec(
            term,
            layer,
            None,
            line,
            &value_sets,
            &mut catalog.object_schemas,
            &mut catalog.object_names,
            &mut catalog.object_labels,
            &mut catalog.object_layers,
            &mut catalog.object_defs,
            &mut catalog.render_chars,
            &mut catalog.char_objects,
        )?
    };
    mark_visual_objects(&declared, visual || is_display_role_token(term), catalog);
    Ok(declared)
}

fn push_terms(objects: &mut Vec<ObjectId>, terms: &[ObjectId]) {
    for object in terms {
        push_unique_object(objects, *object);
    }
}

fn parse_scratch_block(
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
                parse_scratch_directive(name, Some(*ty), line, catalog)?;
                i += 1;
            }
            [spec] => {
                let (name, ty) = spec
                    .split_once('=')
                    .map_or((*spec, None), |(name, ty)| (name, Some(ty)));
                parse_scratch_directive(name, ty, line, catalog)?;
                i += 1;
            }
            [] => i += 1,
            _ => {
                return Err(parse_error(
                    line,
                    "scratch row must be: <name> or <name> = <type>",
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "scratch missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_scratch_directive(
    name: &str,
    ty: Option<&str>,
    line: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    let (name, kind, values) = if let Some(ty) = ty {
        validate_scratch_name(name, line)?;
        if ty.is_empty() {
            return Err(parse_error(line, "scratch type must not be empty"));
        }
        match ty {
            "int" => (name, ScratchKind::Int, Vec::new()),
            "bool" => (name, ScratchKind::Bool, Vec::new()),
            axis if catalog.value_sets.contains_key(axis)
                || catalog.object_axes.contains_key(axis) =>
            {
                (
                    name,
                    ScratchKind::Enum,
                    catalog
                        .value_sets
                        .get(axis)
                        .or_else(|| catalog.object_axes.get(axis))
                        .cloned()
                        .unwrap_or_default(),
                )
            }
            _ => return Err(parse_error(line, "unknown scratch type")),
        }
    } else {
        validate_scratch_name(name, line)?;
        (name, ScratchKind::Bool, Vec::new())
    };
    if catalog.scratch_names.contains_key(name) {
        return Err(parse_error(line, "duplicate scratch"));
    }
    let id = ScratchId(catalog.scratch_defs.len() as u16);
    let def = ScratchDef { id, kind, values };
    catalog.scratch_defs.push(def.clone());
    catalog.scratch_names.insert(name.to_string(), def);
    Ok(())
}

fn parse_layers_block(
    lines: &[String],
    start: usize,
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
    catalog: &mut Catalog,
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
                    &catalog.numeric_global_defaults,
                    &lines[i],
                )?;
                validate_identifier(binding, &lines[i], "expansion binding")?;
                let (body_lines, next_i) = collect_statement_block_lines(lines, i + 1, &lines[i])?;
                for value in &values {
                    let mut expanded_lines = expand_for_binding_lines(
                        &body_lines,
                        binding,
                        value.axis.as_deref(),
                        &value.value,
                        &catalog.maps,
                    )?;
                    expanded_lines.push(BLOCK_CLOSE.to_string());
                    let parsed_i =
                        parse_layers_block(&expanded_lines, 0, named_layers, layer_count, catalog)?;
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
            [name, "=", selectors @ ..] => {
                let layer = layer_id_for_name(name, &lines[i], named_layers, layer_count, catalog)?;
                let objects =
                    define_or_assign_terms_to_layer(selectors, &lines[i], layer, catalog, false)?;
                validate_named_selector_role(
                    name,
                    &objects,
                    &catalog.visual_objects,
                    &lines[i],
                    "layer",
                )?;
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
        if let Some(term) = display_object_spec(terms, &mut i, line)? {
            let declared = parse_layer_term(term, line, layer, true, catalog)?;
            push_terms(&mut objects, &declared);
            continue;
        }
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
        define_object_spec(
            selector,
            UNASSIGNED_LAYER,
            None,
            line,
            &catalog_value_sets(catalog),
            &mut catalog.object_schemas,
            &mut catalog.object_names,
            &mut catalog.object_labels,
            &mut catalog.object_layers,
            &mut catalog.object_defs,
            &mut catalog.render_chars,
            &mut catalog.char_objects,
        )?
    };
    mark_visual_objects(
        &declared,
        visual || is_display_role_token(selector),
        catalog,
    );
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

fn validate_named_selector_role(
    name: &str,
    objects: &[ObjectId],
    visual_objects: &[ObjectId],
    line: &str,
    kind: &str,
) -> Result<(), DiagnosticReport> {
    let display_name = is_display_role_token(name);
    let has_main = objects
        .iter()
        .any(|object| !object.is_empty() && !visual_objects.contains(object));
    let has_display = objects.iter().any(|object| visual_objects.contains(object));
    if display_name && has_main {
        return Err(parse_error(
            line,
            &format!("@{kind} can only contain display objects"),
        ));
    }
    if !display_name && has_display {
        return Err(parse_error(
            line,
            &format!("{kind} containing display objects must use an @ name"),
        ));
    }
    Ok(())
}

fn validate_layer_role_separation(
    catalog: &Catalog,
    named_layers: &HashMap<String, u16>,
) -> Result<(), DiagnosticReport> {
    let mut layer_roles = HashMap::<LayerId, (bool, bool)>::new();
    for definition in &catalog.object_defs {
        if definition.layer_id.0 == UNASSIGNED_LAYER || definition.id.is_empty() {
            continue;
        }
        let visual = catalog.visual_objects.contains(&definition.id);
        let entry = layer_roles
            .entry(definition.layer_id)
            .or_insert((false, false));
        if visual {
            entry.1 = true;
        } else {
            entry.0 = true;
        }
    }

    for (layer, (has_main, has_visual)) in layer_roles {
        if has_main && has_visual {
            let name = named_layers
                .iter()
                .find_map(|(name, named_layer)| {
                    (*named_layer == layer.0 && !name.starts_with("__anonymous_layer_"))
                        .then_some(name.as_str())
                })
                .unwrap_or("<anonymous>");
            return Err(DiagnosticReport::error(format!(
                "layers cannot mix gameplay objects and display objects in the same storage layer ({name}); put display objects in a separate layer"
            )));
        }
    }
    Ok(())
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
    if tokens.len() < 3 || tokens.get(1).copied() != Some("=") {
        return Err(parse_error(
            line,
            "legend row must be: <char> = <empty | selector...>",
        ));
    }

    let ch = parse_char(tokens.first(), line, "missing legend char")?;
    if tokens[2..] == ["empty"] {
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
            StatementAst::DisplayBlock(statements)
            | StatementAst::Block { statements, .. }
            | StatementAst::Fix { statements, .. } => {
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
            | StatementAst::DisplayCall { .. }
            | StatementAst::DisplayRewrite(_)
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
        | ConditionAst::GlobalEquals { .. }
        | ConditionAst::GlobalCompare { .. }
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
        condition: ConditionAst::InputIs("restart".to_string()),
        then_statements: vec![StatementAst::Effect {
            source_line: "restart".to_string(),
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

fn named_direction_vector(value: &str, line: &str) -> Result<(i16, i16), DiagnosticReport> {
    match value {
        "right" => Ok((1, 0)),
        "left" => Ok((-1, 0)),
        "up" => Ok((0, -1)),
        "down" => Ok((0, 1)),
        _ => Err(parse_error(line, "unknown direction name")),
    }
}

fn parse_scene_definition(
    lines: &[String],
    start: usize,
) -> Result<(SceneDef, usize), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let name = match header.as_slice() {
        ["scene", "level_menu", ..] => {
            return Err(parse_error(
                &lines[start],
                "scene level_menu template is not supported; use scene <name> with layout { level_menu { ... } }",
            ));
        }
        ["scene", name] => *name,
        _ => {
            return Err(parse_error(
                &lines[start],
                "scene header must be: scene <name>[(param...)]",
            ));
        }
    };
    let (name, _params) = parse_scene_name_and_params(name, &lines[start])?;

    let mut screen = SceneDef {
        name: name.clone(),
        layout: SceneLayoutDef::default(),
        resources: SceneResources::default(),
        state: SceneStateDef::default(),
        components: Vec::new(),
        key_bindings: Vec::new(),
        routines: Vec::new(),
        transitions: Vec::new(),
        puzzle_rule: None,
    };
    let mut handler = Scene2dBlockHandler {
        screen: &mut screen,
    };
    let next = puzzle_scene::parse_scene_block_with_handler(
        lines,
        start + 1,
        &name,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut handler,
    )?;

    Ok((screen, next))
}

fn parse_scene_name_and_params(
    value: &str,
    line: &str,
) -> Result<(String, Vec<String>), DiagnosticReport> {
    let Some((name, params)) = value.split_once('(') else {
        validate_qualified_identifier(value, line, "scene name")?;
        return Ok((value.to_string(), Vec::new()));
    };
    validate_qualified_identifier(name, line, "scene name")?;
    let params = params
        .strip_suffix(')')
        .ok_or_else(|| parse_error(line, "scene params must end with )"))?;
    let params = if params.trim().is_empty() {
        Vec::new()
    } else {
        params
            .split(',')
            .map(str::trim)
            .map(|param| {
                validate_identifier(param, line, "scene param")?;
                Ok(param.to_string())
            })
            .collect::<Result<Vec<_>, DiagnosticReport>>()?
    };
    Ok((name.to_string(), params))
}

fn resolve_scene_actions(
    scenes: &mut [SceneDef],
    input_labels: &HashMap<InputId, String>,
) -> Result<(), DiagnosticReport> {
    let input_names = input_labels.values().cloned().collect::<HashSet<_>>();
    for scene in scenes {
        resolve_scene_actions_for_scene(scene, &input_names)?;
        validate_scene_routines(scene)?;
        validate_scene_puzzle_rule(scene)?;
    }
    Ok(())
}

fn validate_scene_puzzle_rule(scene: &SceneDef) -> Result<(), DiagnosticReport> {
    let Some(rule) = &scene.puzzle_rule else {
        return Ok(());
    };
    let target = rule
        .target
        .split('.')
        .next_back()
        .unwrap_or(rule.target.as_str());
    if scene
        .state
        .puzzles
        .iter()
        .any(|puzzle| puzzle.name == target)
    {
        return Ok(());
    }
    let declared = scene
        .state
        .puzzles
        .iter()
        .map(|puzzle| puzzle.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(DiagnosticReport::error(format!(
        "scene `{}` runs `step {}` but declares no matching puzzle slot (declared: [{}])",
        scene.name, rule.target, declared
    )))
}

fn resolve_scene_actions_for_scene(
    scene: &mut SceneDef,
    input_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    let routine_names = scene
        .routines
        .iter()
        .map(|routine| routine.name.clone())
        .collect::<HashSet<_>>();
    for binding in &mut scene.key_bindings {
        resolve_scene_effect_action(&mut binding.effect, input_names, &routine_names)?;
    }
    for transition in &mut scene.transitions {
        resolve_scene_effect_action(&mut transition.effect, input_names, &routine_names)?;
    }
    for component in &mut scene.components {
        resolve_scene_component_actions(component, input_names, &routine_names)?;
    }
    for routine in &mut scene.routines {
        resolve_scene_effect_action(&mut routine.effect, input_names, &routine_names)?;
    }
    Ok(())
}

fn resolve_scene_component_actions(
    component: &mut SceneComponent,
    input_names: &HashSet<String>,
    routine_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    match component {
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            resolve_scene_effect_action(&mut button.effect, input_names, routine_names)
        }
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => {
            for child in &mut container.children {
                resolve_scene_component_actions(child, input_names, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::Conditional(conditional) => {
            for child in &mut conditional.children {
                resolve_scene_component_actions(child, input_names, routine_names)?;
            }
            for child in &mut conditional.else_children {
                resolve_scene_component_actions(child, input_names, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::For(for_view) => {
            for child in &mut for_view.children {
                resolve_scene_component_actions(child, input_names, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::LevelMenu(menu) => {
            if let Some(effect) = &mut menu.action {
                resolve_scene_effect_action(effect, input_names, routine_names)?;
            }
            for button in &mut menu.buttons {
                resolve_scene_effect_action(&mut button.effect, input_names, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::Frame(_)
        | SceneComponent::Title(_)
        | SceneComponent::Subtitle(_)
        | SceneComponent::Text(_) => Ok(()),
    }
}

fn resolve_scene_effect_action(
    effect: &mut SceneEffect,
    input_names: &HashSet<String>,
    routine_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    match effect {
        SceneEffect::RoutineCall(name) => {
            let is_input = input_names.contains(name);
            let is_routine = routine_names.contains(name);
            match (is_input, is_routine) {
                (true, true) => Err(DiagnosticReport::error(format!(
                    "ambiguous scene action `{name}`; write `input {name}` or rename the scene routine"
                ))),
                (true, false) => {
                    *effect = SceneEffect::Input(name.clone());
                    Ok(())
                }
                (false, true) => Ok(()),
                (false, false) => Err(DiagnosticReport::error(format!(
                    "unknown scene action: {name}"
                ))),
            }
        }
        SceneEffect::Conditional { effect, .. } => {
            resolve_scene_effect_action(effect, input_names, routine_names)
        }
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                resolve_scene_effect_action(effect, input_names, routine_names)?;
            }
            Ok(())
        }
        SceneEffect::Input(_)
        | SceneEffect::ComponentEffect(_)
        | SceneEffect::Message { .. }
        | SceneEffect::Wait { .. }
        | SceneEffect::PlaySfx { .. }
        | SceneEffect::PlayMusic { .. }
        | SceneEffect::PauseMusic { .. }
        | SceneEffect::ResumeMusic { .. }
        | SceneEffect::StopMusic { .. }
        | SceneEffect::Goto { .. }
        | SceneEffect::Enter { .. }
        | SceneEffect::Back
        | SceneEffect::Create { .. }
        | SceneEffect::Reset { .. }
        | SceneEffect::Delete { .. }
        | SceneEffect::Show { .. }
        | SceneEffect::Hide { .. }
        | SceneEffect::Toggle { .. }
        | SceneEffect::Focus { .. }
        | SceneEffect::PuzzleNextLevel { .. }
        | SceneEffect::PuzzlePreviousLevel { .. }
        | SceneEffect::GotoLevel { .. }
        | SceneEffect::ResetPuzzle { .. }
        | SceneEffect::LoadPuzzle { .. }
        | SceneEffect::Apply { .. }
        | SceneEffect::Copy { .. }
        | SceneEffect::SetVariable { .. }
        | SceneEffect::ClearUndoHistory
        | SceneEffect::ClearGameProgress
        | SceneEffect::SetCurrentLevel { .. }
        | SceneEffect::ClearCurrentLevel
        | SceneEffect::SetLevelCleared { .. }
        | SceneEffect::ResetPersistentVars => Ok(()),
    }
}

fn add_scene_input_key_controls(
    scenes: &[SceneDef],
    input_labels: &HashMap<InputId, String>,
    controls: &mut Controls,
) {
    let input_ids = input_labels
        .iter()
        .map(|(id, label)| (label.as_str(), *id))
        .collect::<HashMap<_, _>>();
    for scene in scenes {
        for binding in &scene.key_bindings {
            let SceneEffect::Input(input) = &binding.effect else {
                continue;
            };
            let Some(input_id) = input_ids.get(input.as_str()).copied() else {
                continue;
            };
            for key in &binding.keys {
                add_key_trigger_to_controls_unchecked(key, input_id, controls);
            }
        }
    }
}

fn add_key_trigger_to_controls_unchecked(
    key: &KeyTrigger,
    input: InputId,
    controls: &mut Controls,
) {
    match key {
        KeyTrigger::Char(ch) if ch.is_ascii() => {
            controls
                .keys
                .insert((*ch as u8).to_ascii_lowercase(), input);
        }
        KeyTrigger::Char(_) => {}
        KeyTrigger::Named(name) => {
            if let Some(arrow) = named_key_to_arrow(name) {
                controls.arrows.insert(arrow, input);
            } else {
                controls.named.insert(name.clone(), input);
            }
        }
    }
}

fn validate_scene_routines(scene: &SceneDef) -> Result<(), DiagnosticReport> {
    let routine_names = scene
        .routines
        .iter()
        .map(|routine| routine.name.clone())
        .collect::<HashSet<_>>();
    for binding in &scene.key_bindings {
        validate_scene_effect_routine_calls(&binding.effect, &routine_names)?;
    }
    for transition in &scene.transitions {
        validate_scene_effect_routine_calls(&transition.effect, &routine_names)?;
    }
    for component in &scene.components {
        validate_scene_component_routine_calls(component, &routine_names)?;
    }

    let routines = scene
        .routines
        .iter()
        .map(|routine| (routine.name.as_str(), routine))
        .collect::<HashMap<_, _>>();
    let mut checked = HashSet::<String>::new();
    for routine in &scene.routines {
        validate_scene_routine_not_recursive(
            routine.name.as_str(),
            &routines,
            &mut Vec::new(),
            &mut checked,
        )?;
    }
    Ok(())
}

fn validate_scene_component_routine_calls(
    component: &SceneComponent,
    routine_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    match component {
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            validate_scene_effect_routine_calls(&button.effect, routine_names)
        }
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => {
            for child in &container.children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::Conditional(conditional) => {
            for child in &conditional.children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            for child in &conditional.else_children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::For(for_view) => {
            for child in &for_view.children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::LevelMenu(menu) => {
            if let Some(effect) = &menu.action {
                validate_scene_effect_routine_calls(effect, routine_names)?;
            }
            for button in &menu.buttons {
                validate_scene_effect_routine_calls(&button.effect, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::Frame(_)
        | SceneComponent::Title(_)
        | SceneComponent::Subtitle(_)
        | SceneComponent::Text(_) => Ok(()),
    }
}

fn validate_scene_effect_routine_calls(
    effect: &SceneEffect,
    routine_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    match effect {
        SceneEffect::RoutineCall(name) => {
            if !routine_names.contains(name) {
                return Err(DiagnosticReport::error(format!(
                    "unknown scene routine: {name}"
                )));
            }
            Ok(())
        }
        SceneEffect::Conditional { effect, .. } => {
            validate_scene_effect_routine_calls(effect, routine_names)
        }
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                validate_scene_effect_routine_calls(effect, routine_names)?;
            }
            Ok(())
        }
        SceneEffect::Input(_)
        | SceneEffect::ComponentEffect(_)
        | SceneEffect::Message { .. }
        | SceneEffect::Wait { .. }
        | SceneEffect::PlaySfx { .. }
        | SceneEffect::PlayMusic { .. }
        | SceneEffect::PauseMusic { .. }
        | SceneEffect::ResumeMusic { .. }
        | SceneEffect::StopMusic { .. }
        | SceneEffect::Goto { .. }
        | SceneEffect::Enter { .. }
        | SceneEffect::Back
        | SceneEffect::Create { .. }
        | SceneEffect::Reset { .. }
        | SceneEffect::Delete { .. }
        | SceneEffect::Show { .. }
        | SceneEffect::Hide { .. }
        | SceneEffect::Toggle { .. }
        | SceneEffect::Focus { .. }
        | SceneEffect::PuzzleNextLevel { .. }
        | SceneEffect::PuzzlePreviousLevel { .. }
        | SceneEffect::GotoLevel { .. }
        | SceneEffect::ResetPuzzle { .. }
        | SceneEffect::LoadPuzzle { .. }
        | SceneEffect::Apply { .. }
        | SceneEffect::Copy { .. }
        | SceneEffect::SetVariable { .. }
        | SceneEffect::ClearUndoHistory
        | SceneEffect::ClearGameProgress
        | SceneEffect::SetCurrentLevel { .. }
        | SceneEffect::ClearCurrentLevel
        | SceneEffect::SetLevelCleared { .. }
        | SceneEffect::ResetPersistentVars => Ok(()),
    }
}

fn validate_scene_routine_not_recursive(
    name: &str,
    routines: &HashMap<&str, &SceneRoutineDef>,
    stack: &mut Vec<String>,
    checked: &mut HashSet<String>,
) -> Result<(), DiagnosticReport> {
    if checked.contains(name) {
        return Ok(());
    }
    if stack.iter().any(|active| active == name) {
        stack.push(name.to_string());
        return Err(DiagnosticReport::error(format!(
            "recursive scene routine call: {}",
            stack.join(" -> ")
        )));
    }
    let Some(routine) = routines.get(name) else {
        return Err(DiagnosticReport::error(format!(
            "unknown scene routine: {name}"
        )));
    };
    stack.push(name.to_string());
    for call in scene_effect_routine_calls(&routine.effect) {
        validate_scene_routine_not_recursive(call, routines, stack, checked)?;
    }
    stack.pop();
    checked.insert(name.to_string());
    Ok(())
}

fn scene_effect_routine_calls(effect: &SceneEffect) -> Vec<&str> {
    let mut calls = Vec::new();
    collect_scene_effect_routine_calls(effect, &mut calls);
    calls
}

fn collect_scene_effect_routine_calls<'a>(effect: &'a SceneEffect, calls: &mut Vec<&'a str>) {
    match effect {
        SceneEffect::RoutineCall(name) => calls.push(name.as_str()),
        SceneEffect::Conditional { effect, .. } => {
            collect_scene_effect_routine_calls(effect, calls);
        }
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                collect_scene_effect_routine_calls(effect, calls);
            }
        }
        _ => {}
    }
}

struct Scene2dBlockHandler<'a> {
    screen: &'a mut SceneDef,
}

impl puzzle_scene::SceneBlockHandler for Scene2dBlockHandler<'_> {
    type Error = DiagnosticReport;

    fn parse_state_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (state, next_i) = parse_scene_state_block(lines, start, SceneStateLifetime::Instance)?;
        self.screen.state.variables.extend(state.variables);
        self.screen.state.puzzles.extend(state.puzzles);
        Ok(next_i)
    }

    fn parse_layout_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (layout_block, next_i) = parse_screen_layout_block(lines, start)?;
        self.screen.layout = layout_block.layout;
        self.screen
            .state
            .variables
            .extend(layout_block.state.variables);
        self.screen.state.puzzles.extend(layout_block.state.puzzles);
        self.screen.components.extend(layout_block.components);
        Ok(next_i)
    }

    fn parse_inputs_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        Err(parse_error(
            &lines[start],
            "`inputs { ... }` was removed; use `keys { <key...> -> <routine-or-effect> }`",
        ))
    }

    fn parse_keys_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (bindings, next_i) = parse_scene_keys_block(lines, start)?;
        self.screen.key_bindings.extend(bindings);
        Ok(next_i)
    }

    fn parse_rules_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (block, next_i) = parse_screen_transitions_block(lines, start)?;
        self.screen.transitions.extend(block.transitions);
        if let Some(puzzle_rule) = block.puzzle_rule {
            self.screen.puzzle_rule = Some(puzzle_rule);
        }
        Ok(next_i)
    }

    fn parse_scene_start_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (transition, next_i) = parse_scene_lifecycle_block(lines, start)?;
        self.screen.transitions.push(transition);
        Ok(next_i)
    }

    fn parse_inline_directive(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let tokens = split_header_tokens(&lines[start]);
        match tokens.as_slice() {
            ["resources"] => parse_scene_resources_block(lines, start, &mut self.screen.resources),
            ["var", ..]
            | ["const", ..]
            | ["persistent", "var", ..]
            | ["persistent", "const", ..] => {
                match parse_scene_state_entry(&lines[start], SceneStateLifetime::Instance)? {
                    ParsedSceneStateEntry::Variable(variable) => {
                        self.screen.state.variables.push(variable);
                    }
                    ParsedSceneStateEntry::Puzzle(_) => {
                        return Err(parse_error(
                            &lines[start],
                            "var cannot define a puzzle slot",
                        ));
                    }
                }
                Ok(start + 1)
            }
            ["on_level_start" | "on_level_clear" | "on_last_level_clear"] => Err(parse_error(
                &lines[start],
                "level lifecycle blocks belong inside puzzle; scene lifecycle block must be on_scene_start",
            )),
            ["input", ..] => Err(parse_error(
                &lines[start],
                "scene input handlers are removed; use `keys { <key...> -> <routine-or-effect> }` and `routine <name> { ... }`",
            )),
            ["action", ..] => Err(parse_error(
                &lines[start],
                "`action` scene handlers were removed; use `routine <name> { ... }`",
            )),
            ["routine", ..] => {
                let (routine, next_i) = parse_scene_routine_block(lines, start)?;
                if self
                    .screen
                    .routines
                    .iter()
                    .any(|existing| existing.name == routine.name)
                {
                    return Err(parse_error(&lines[start], "duplicate scene routine"));
                }
                self.screen.routines.push(routine);
                Ok(next_i)
            }
            ["if", ..] => {
                let (transition, next_i) = parse_screen_condition_block(lines, start)?;
                self.screen.transitions.push(transition);
                Ok(next_i)
            }
            [] => Ok(start + 1),
            _ if scene_entry_is_component(&tokens) => {
                let (component, next_i) = parse_screen_component(lines, start)?;
                self.screen.components.push(component);
                Ok(next_i)
            }
            [other, ..] => Err(parse_error(
                &lines[start],
                &format!("unknown scene directive {other}"),
            )),
        }
    }
}

fn parse_scene_resources_block(
    lines: &[String],
    start: usize,
    resources: &mut SceneResources,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["levels", names @ ..] => {
                resources.levels = parse_resource_selection(names, &lines[i])?;
            }
            ["sprites", names @ ..] => {
                resources.sprites = parse_resource_selection(names, &lines[i])?;
            }
            [] => {}
            [other, ..] => {
                return Err(parse_error(
                    &lines[i],
                    &format!("unknown resources directive {other}"),
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "resources missing closing brace",
        ));
    }
    Ok(i + 1)
}

fn parse_resource_selection(
    names: &[&str],
    line: &str,
) -> Result<ResourceSelection, DiagnosticReport> {
    match names {
        [] | ["all"] => Ok(ResourceSelection::All),
        ["none"] => Ok(ResourceSelection::Named(Vec::new())),
        names => {
            let mut selected = Vec::new();
            for name in names {
                if name.chars().any(|ch| matches!(ch, '{' | '}' | ',' | ';')) {
                    return Err(parse_error(
                        line,
                        "resource names must be whitespace-separated",
                    ));
                }
                selected.push((*name).to_string());
            }
            Ok(ResourceSelection::Named(selected))
        }
    }
}

struct ParsedScreenLayoutBlock {
    layout: SceneLayoutDef,
    state: ParsedScreenStateBlock,
    components: Vec<SceneComponent>,
}

fn parse_screen_layout_block(
    lines: &[String],
    start: usize,
) -> Result<(ParsedScreenLayoutBlock, usize), DiagnosticReport> {
    parse_screen_view_like_block(lines, start, "layout")
}

fn parse_screen_view_like_block(
    lines: &[String],
    start: usize,
    block_name: &str,
) -> Result<(ParsedScreenLayoutBlock, usize), DiagnosticReport> {
    let layout = parse_scene_layout_from_header(&lines[start], block_name)?;
    let mut variables = Vec::new();
    let mut puzzles = Vec::new();
    let mut components = Vec::new();
    let mut hidden = Vec::<String>::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if let Some((slot, visible)) = parse_layer_visibility(&lines[i])? {
            if visible {
                hidden.retain(|name| name != &slot);
                if puzzles
                    .iter()
                    .any(|puzzle: &ScenePuzzleDef| puzzle.name == slot)
                    && !components.iter().any(|component| {
                        scene_puzzle_component_source(component).is_some_and(|name| name == slot)
                    })
                {
                    components.push(scene_puzzle_component(slot));
                }
            } else {
                hidden.push(slot.clone());
                components.retain(|component| {
                    scene_puzzle_component_source(component) != Some(slot.as_str())
                });
            }
            i += 1;
            continue;
        }

        let tokens = split_header_tokens(&lines[i]);
        if matches!(tokens.as_slice(), ["panel", ..]) {
            return Err(parse_error(&lines[i], "unknown layout directive panel"));
        }
        if matches!(tokens.as_slice(), ["if", ..]) {
            let (component, next_i) = parse_view_if_component(lines, i)?;
            components.push(component);
            i = next_i;
            continue;
        }
        if scene_entry_is_component(&tokens) || matches!(tokens.as_slice(), ["puzzle", ..]) {
            let (component, next_i) = parse_screen_component(lines, i)?;
            components.push(component);
            i = next_i;
            continue;
        }

        if lines[i].contains('=') {
            match parse_scene_state_entry(&lines[i], SceneStateLifetime::Instance)? {
                ParsedSceneStateEntry::Puzzle(puzzle) => {
                    if !hidden.iter().any(|name| name == &puzzle.name) {
                        components.push(scene_puzzle_component(puzzle.name.clone()));
                    }
                    puzzles.push(puzzle);
                }
                ParsedSceneStateEntry::Variable(variable) => variables.push(variable),
            }
            i += 1;
            continue;
        }

        let (component, next_i) = parse_screen_component(lines, i)?;
        components.push(component);
        i = next_i;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            &format!("{block_name} missing closing brace"),
        ));
    }

    Ok((
        ParsedScreenLayoutBlock {
            layout,
            state: ParsedScreenStateBlock { variables, puzzles },
            components,
        },
        i + 1,
    ))
}

fn parse_scene_layout_from_header(
    line: &str,
    keyword: &str,
) -> Result<SceneLayoutDef, DiagnosticReport> {
    puzzle_scene::parse_scene_layout_header(line, keyword, puzzle_scene::SceneBlockSyntax::Braces)
        .map_err(DiagnosticReport::from)
}

fn parse_layer_visibility(line: &str) -> Result<Option<(String, bool)>, DiagnosticReport> {
    let Some((name, value)) = line.split_once('=') else {
        return Ok(None);
    };
    let Some(slot) = name.trim().strip_suffix(".visible") else {
        return Ok(None);
    };
    validate_qualified_identifier(slot.trim(), line, "layer name")?;
    match value.trim() {
        "true" => Ok(Some((slot.trim().to_string(), true))),
        "false" => Ok(Some((slot.trim().to_string(), false))),
        _ => Err(parse_error(line, "layer visibility must be true or false")),
    }
}

fn scene_frame_component(kind: impl Into<String>, source: impl Into<String>) -> SceneComponent {
    scene_frame_component_with_layout(kind, source, SceneLayoutDef::default())
}

fn scene_frame_component_with_layout(
    kind: impl Into<String>,
    source: impl Into<String>,
    layout: SceneLayoutDef,
) -> SceneComponent {
    SceneComponent::Frame(puzzle_scene::FrameComponent {
        kind: kind.into(),
        source: source.into(),
        inputs: Vec::new(),
        layout,
    })
}

fn scene_puzzle_component(source: impl Into<String>) -> SceneComponent {
    scene_frame_component("puzzle", source)
}

fn scene_puzzle_component_source(component: &SceneComponent) -> Option<&str> {
    match component {
        SceneComponent::Frame(frame) => Some(frame.source.as_str()),
        _ => None,
    }
}

fn parse_screen_components_block(
    lines: &[String],
    start: usize,
    block_name: &str,
) -> Result<(Vec<SceneComponent>, usize), DiagnosticReport> {
    let mut parse_leaf =
        |lines: &[String], index: usize| -> Result<(usize, SceneComponent), DiagnosticReport> {
            let (component, next) = parse_screen_leaf_component(lines, index)?;
            Ok((next, component))
        };
    let (next, components) = puzzle_scene::parse_scene_component_block(
        lines,
        start + 1,
        block_name,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut parse_leaf,
        &build_scene_container_component,
    )?;
    Ok((components, next))
}

fn parse_screen_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let mut parse_leaf =
        |lines: &[String], index: usize| -> Result<(usize, SceneComponent), DiagnosticReport> {
            let (component, next) = parse_screen_leaf_component(lines, index)?;
            Ok((next, component))
        };
    let (next, component) = puzzle_scene::parse_scene_component_at(
        lines,
        start,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut parse_leaf,
        &build_scene_container_component,
    )?;
    Ok((component, next))
}

fn build_scene_container_component(
    kind: puzzle_scene::SceneComponentKind,
    children: Vec<SceneComponent>,
    layout: SceneLayoutDef,
) -> SceneComponent {
    match kind {
        puzzle_scene::SceneComponentKind::Row => {
            SceneComponent::Row(SceneContainerDef { children, layout })
        }
        puzzle_scene::SceneComponentKind::Column => {
            SceneComponent::Column(SceneContainerDef { children, layout })
        }
        puzzle_scene::SceneComponentKind::Box => {
            SceneComponent::Box(SceneContainerDef { children, layout })
        }
        _ => unreachable!("shared scene parser only builds generic containers"),
    }
}

fn parse_screen_leaf_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    match tokens.as_slice() {
        ["puzzle", "current_level"] => Err(parse_error(
            &lines[start],
            "current_level is not scene syntax; declare a puzzle slot with `board = puzzle <name>`",
        )),
        ["puzzle", state_name, attrs @ ..] => {
            if *state_name == "current_level" {
                return Err(parse_error(
                    &lines[start],
                    "current_level is not scene syntax; declare a puzzle slot with `board = puzzle <name>`",
                ));
            }
            if !is_identifier(state_name) {
                return Err(parse_error(
                    &lines[start],
                    "puzzle state name must be an identifier",
                ));
            }
            let layout = parse_scene_layout_attrs_for_line(attrs, &lines[start])?;
            Ok((
                scene_frame_component_with_layout("puzzle", (*state_name).to_string(), layout),
                start + 1,
            ))
        }
        ["frame", source, attrs @ ..] => {
            if !is_identifier(source) {
                return Err(parse_error(
                    &lines[start],
                    "frame source must be an identifier",
                ));
            }
            let layout = parse_scene_layout_attrs_for_line(attrs, &lines[start])?;
            Ok((
                scene_frame_component_with_layout("frame", (*source).to_string(), layout),
                start + 1,
            ))
        }
        ["puzzle3", source, attrs @ ..] => {
            if !is_identifier(source) {
                return Err(parse_error(
                    &lines[start],
                    "puzzle3 frame source must be an identifier",
                ));
            }
            let layout = parse_scene_layout_attrs_for_line(attrs, &lines[start])?;
            Ok((
                scene_frame_component_with_layout("puzzle3", (*source).to_string(), layout),
                start + 1,
            ))
        }
        ["text", ..] => Ok((parse_text_component(&lines[start])?, start + 1)),
        ["title", ..] => Ok((parse_title_component(&lines[start], true)?, start + 1)),
        ["subtitle", ..] => Ok((parse_title_component(&lines[start], false)?, start + 1)),
        ["button", ..] => parse_button_component(lines, start),
        ["choice", ..] => parse_choice_component(lines, start),
        ["if", ..] => parse_view_if_component(lines, start),
        ["for", ..] => parse_for_component(lines, start),
        ["level_menu"] => {
            let (menu, next_i) = parse_level_menu_component(lines, start)?;
            Ok((SceneComponent::LevelMenu(menu), next_i))
        }
        ["level_menu", ..] => Err(parse_error(
            &lines[start],
            "level_menu takes no inline source or effect; use scene resources to choose levels",
        )),
        [state_name] if is_identifier(state_name) => Ok((
            scene_frame_component("puzzle", (*state_name).to_string()),
            start + 1,
        )),
        [other, ..] => Err(parse_error(
            &lines[start],
            &format!("unknown layout directive {other}"),
        )),
        [] => Err(parse_error(&lines[start], "empty layout directive")),
    }
}

fn parse_scene_layout_attrs_for_line(
    attrs: &[&str],
    line: &str,
) -> Result<SceneLayoutDef, DiagnosticReport> {
    puzzle_scene::parse_scene_layout_attrs(attrs).map_err(|error| parse_error(line, &error.message))
}

fn parse_title_component(line: &str, is_title: bool) -> Result<SceneComponent, DiagnosticReport> {
    let keyword = if is_title { "title" } else { "subtitle" };
    let Some(rest) = line.strip_prefix(keyword) else {
        return Err(parse_error(line, "title must be: title <text-or-path>"));
    };
    let rest = rest.trim();
    let content = if rest.is_empty() {
        SceneExpr::Path(vec![keyword.to_string()])
    } else {
        parse_scene_expr(rest, line)?
    };
    let title = SceneTitleDef {
        content,
        layout: SceneLayoutDef::default(),
    };
    Ok(if is_title {
        SceneComponent::Title(title)
    } else {
        SceneComponent::Subtitle(title)
    })
}

fn parse_text_component(line: &str) -> Result<SceneComponent, DiagnosticReport> {
    let Some(rest) = line.strip_prefix("text") else {
        return Err(parse_error(
            line,
            "text must be: text \"<text>\" | text <state>",
        ));
    };
    let rest = rest.trim();
    if let Some(text) = parse_quoted_text(rest) {
        return Ok(SceneComponent::Text(SceneTextDef {
            content: SceneTextContent::Literal(text),
            layout: SceneLayoutDef::default(),
        }));
    }
    if let Some(path) = parse_view_path(rest) {
        return Ok(SceneComponent::Text(SceneTextDef {
            content: SceneTextContent::Path(path),
            layout: SceneLayoutDef::default(),
        }));
    }
    Err(parse_error(
        line,
        "text must be: text \"<text>\" | text <state>",
    ))
}

fn parse_button_like_def(
    lines: &[String],
    start: usize,
    keyword: &str,
) -> Result<(SceneButtonDef, usize), DiagnosticReport> {
    let line = &lines[start];
    let Some(rest) = line.strip_prefix(keyword) else {
        return Err(parse_error(
            line,
            &format!("{keyword} must be: {keyword} \"<label>\" -> <effect>"),
        ));
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return Err(parse_error(
            line,
            &format!("{keyword} must be: {keyword} \"<label>\" -> <effect>"),
        ));
    }

    let (label, effect, next_i) = if rest.contains('=') {
        return Err(parse_error(
            line,
            &format!("{keyword} command must use `->`; `=` action assignment was removed"),
        ));
    } else if let Some((label, effect)) = rest.split_once("->") {
        let effect_text = effect.trim();
        let (effect, next_i) = parse_scene_effect_with_optional_block(effect_text, lines, start)?;
        (parse_button_label(label.trim(), line)?, effect, next_i)
    } else {
        return Err(parse_error(
            line,
            &format!("{keyword} must be: {keyword} \"<label>\" -> <effect>"),
        ));
    };

    Ok((
        SceneButtonDef {
            label,
            effect,
            layout: SceneLayoutDef::default(),
        },
        next_i,
    ))
}

fn parse_button_def(
    lines: &[String],
    start: usize,
) -> Result<(SceneButtonDef, usize), DiagnosticReport> {
    parse_button_like_def(lines, start, "button")
}

fn parse_button_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let (button, next_i) = parse_button_def(lines, start)?;
    Ok((SceneComponent::Button(button), next_i))
}

fn parse_choice_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let (choice, next_i) = parse_button_like_def(lines, start, "choice")?;
    Ok((SceneComponent::Choice(choice), next_i))
}

fn parse_view_if_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let line = &lines[start];
    let condition = block_header_text(line)
        .strip_prefix("if ")
        .ok_or_else(|| parse_error(line, "layout condition must be: if <condition>"))?
        .trim();
    validate_screen_condition(condition, line)?;
    let (entry, next_i) = collect_authoring_entry(lines, start)?;
    let body = &entry[1..entry.len().saturating_sub(1)];
    let (else_body, next_i) = collect_view_else_body(lines, next_i, line)?;
    if body.is_empty() {
        return Err(parse_error(
            line,
            "layout condition requires at least one component",
        ));
    }
    let children = parse_screen_component_body(body, "if")?;
    let else_children = if else_body.is_empty() {
        Vec::new()
    } else {
        parse_screen_component_body(&else_body, "else")?
    };
    Ok((
        SceneComponent::Conditional(SceneConditionalDef {
            condition: condition.to_string(),
            children,
            else_children,
        }),
        next_i,
    ))
}

fn collect_view_else_body(
    lines: &[String],
    start: usize,
    header_line: &str,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    if !next_line_is_else(lines, start) {
        return Ok((Vec::new(), start));
    }

    let mut body = Vec::new();
    let mut block_stack = vec![AuthoringBlockKind::Other];
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.first().copied() == Some(BLOCK_CLOSE) {
            let closed = block_stack
                .pop()
                .ok_or_else(|| parse_error(line, "closing brace without layout block"))?;
            i += 1;
            if block_stack.is_empty() {
                return Ok((body, i));
            }
            body.push(line.clone());
            if closed == AuthoringBlockKind::If && next_line_is_else(lines, i) {
                body.push(lines[i].clone());
                i += 1;
                block_stack.push(AuthoringBlockKind::Other);
            }
            continue;
        }
        if let Some(kind) = authoring_nested_block_kind(&tokens, line) {
            block_stack.push(kind);
        }
        body.push(line.clone());
        i += 1;
    }
    Err(parse_error(
        header_line,
        "layout else block missing closing brace",
    ))
}

fn parse_screen_component_body(
    body: &[String],
    block_name: &str,
) -> Result<Vec<SceneComponent>, DiagnosticReport> {
    let mut lines = body.to_vec();
    lines.push(BLOCK_CLOSE.to_string());
    let mut parse_leaf =
        |lines: &[String], index: usize| -> Result<(usize, SceneComponent), DiagnosticReport> {
            let (component, next) = parse_screen_leaf_component(lines, index)?;
            Ok((next, component))
        };
    let (next, components) = puzzle_scene::parse_scene_component_block(
        &lines,
        0,
        block_name,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut parse_leaf,
        &build_scene_container_component,
    )?;
    debug_assert_eq!(next, lines.len());
    Ok(components)
}

fn parse_for_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    let ["for", binding, "in", source] = tokens.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "for layout must be: for <item> in <source>",
        ));
    };
    if !is_identifier(binding) {
        return Err(parse_error(
            &lines[start],
            "for binding must be an identifier",
        ));
    }
    let source = parse_for_source(source, &lines[start])?;
    let (children, next_i) = parse_screen_components_block(lines, start, "for")?;
    Ok((
        SceneComponent::For(SceneForDef {
            binding: (*binding).to_string(),
            source,
            children,
        }),
        next_i,
    ))
}

fn parse_for_source(value: &str, line: &str) -> Result<ForSource, DiagnosticReport> {
    if value == "levels" {
        return Ok(ForSource::Levels);
    }
    if is_identifier(value) {
        return Ok(ForSource::State(value.to_string()));
    }
    Err(parse_error(
        line,
        "for source must be levels or a state identifier",
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SceneEffectCommandSyntax {
    Plain,
    InputTarget,
    ComponentEffectTarget,
    SceneTarget,
    AssetTarget,
    OptionalAssetTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RewriteEffectCommandSyntax {
    Effect,
    Emission,
}

pub(crate) fn scene_effect_command_syntax(token: &str) -> Option<SceneEffectCommandSyntax> {
    match token {
        "input" => Some(SceneEffectCommandSyntax::InputTarget),
        "component_effect" => Some(SceneEffectCommandSyntax::ComponentEffectTarget),
        "goto" | "start" => Some(SceneEffectCommandSyntax::SceneTarget),
        "sfx" | "play_music" => Some(SceneEffectCommandSyntax::AssetTarget),
        "pause_music" | "resume_music" | "stop_music" => {
            Some(SceneEffectCommandSyntax::OptionalAssetTarget)
        }
        "apply"
        | "clear_history"
        | "clear_undo_history"
        | "clear_game_progress"
        | "clear"
        | "copy"
        | "load"
        | "message"
        | "wait" => Some(SceneEffectCommandSyntax::Plain),
        _ => None,
    }
}

pub(crate) fn scene_effect_semantic_tokens(tokens: &[SourceToken]) -> Vec<semantic::SemanticToken> {
    project_surface_semantic_tokens(&scene_effect_surface_document(tokens).semantic_tokens)
}

fn project_surface_semantic_tokens(
    tokens: &[SurfaceSemanticToken],
) -> Vec<semantic::SemanticToken> {
    tokens
        .iter()
        .map(|token| semantic::SemanticToken {
            start: token.span.start,
            end: token.span.end,
            kind: match token.kind {
                SurfaceSemanticKind::Keyword => semantic::SemanticKind::Keyword,
                SurfaceSemanticKind::Literal => semantic::SemanticKind::Literal,
                SurfaceSemanticKind::Binding => semantic::SemanticKind::Binding,
                SurfaceSemanticKind::Effect => semantic::SemanticKind::Effect,
                SurfaceSemanticKind::Emission => semantic::SemanticKind::Emission,
                SurfaceSemanticKind::Input => semantic::SemanticKind::Input,
                SurfaceSemanticKind::State => semantic::SemanticKind::State,
                SurfaceSemanticKind::Condition => semantic::SemanticKind::Condition,
                SurfaceSemanticKind::Scene => semantic::SemanticKind::Scene,
                SurfaceSemanticKind::Asset => semantic::SemanticKind::Asset,
                SurfaceSemanticKind::Setting => semantic::SemanticKind::Setting,
                SurfaceSemanticKind::Number => semantic::SemanticKind::Number,
                SurfaceSemanticKind::String => semantic::SemanticKind::String,
            },
        })
        .collect()
}

fn scene_effect_surface_document(tokens: &[SourceToken]) -> SurfaceDocument {
    if let Some(parts) = split_scene_effect_token_sequence(tokens) {
        let mut sink = SurfaceSink::default();
        for part in parts {
            sink.extend(scene_effect_surface_document(part));
        }
        return sink.into_document();
    }

    let mut sink = SurfaceSink::default();
    let Some(first) = tokens.first() else {
        return sink.into_document();
    };
    let effect_span = source_tokens_span(tokens);

    if first.text.starts_with("cursor.") {
        add_cursor_scene_effect_token(&mut sink, first);
        return surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span);
    }

    if first.text.contains('.') {
        let mut parts = first.text.split('.');
        if let Some(target) = parts.next() {
            add_scene_effect_token_part(&mut sink, first, target, SurfaceSemanticKind::Scene);
        }
        if let Some(effect) = parts.next() {
            add_scene_effect_token_part(&mut sink, first, effect, SurfaceSemanticKind::Effect);
        }
        return surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span);
    }

    if first.text == "start" && add_level_flow_scene_effect_tokens(tokens, &mut sink) {
        return surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span);
    }

    match scene_effect_command_syntax(&first.text) {
        Some(SceneEffectCommandSyntax::InputTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(input) = tokens.get(1) {
                add_scene_command_token(&mut sink, input);
            }
        }
        Some(SceneEffectCommandSyntax::ComponentEffectTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(effect) = tokens.get(1) {
                add_scene_command_token(&mut sink, effect);
            }
        }
        Some(SceneEffectCommandSyntax::SceneTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(scene) = tokens.get(1) {
                add_scene_effect_token_range(&mut sink, scene, SurfaceSemanticKind::Scene);
            }
        }
        Some(SceneEffectCommandSyntax::AssetTarget) => {
            let kind = scene_effect_command_kind(&first.text);
            add_scene_effect_token_range(&mut sink, first, kind);
            if let Some(asset) = tokens.get(1) {
                add_scene_effect_token_range(&mut sink, asset, SurfaceSemanticKind::Asset);
            }
        }
        Some(SceneEffectCommandSyntax::OptionalAssetTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(asset) = tokens.get(1) {
                add_scene_effect_token_range(&mut sink, asset, SurfaceSemanticKind::Asset);
            }
        }
        Some(SceneEffectCommandSyntax::Plain) => {
            let kind = scene_effect_command_kind(&first.text);
            add_scene_effect_token_range(&mut sink, first, kind);
        }
        None => {}
    }

    surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span)
}

fn source_tokens_span(tokens: &[SourceToken]) -> Option<SourceSpan> {
    let start = tokens.first()?.start;
    let end = tokens.last()?.end;
    (start < end).then_some(SourceSpan { start, end })
}

fn surface_document_with_node(
    mut sink: SurfaceSink,
    kind: SurfaceNodeKind,
    span: Option<SourceSpan>,
) -> SurfaceDocument {
    if sink.has_semantic_tokens()
        && let Some(span) = span
    {
        sink.node(kind, span);
    }
    sink.into_document()
}

fn scene_effect_command_kind(token: &str) -> SurfaceSemanticKind {
    if matches!(
        token,
        "sfx" | "play_music" | "pause_music" | "resume_music" | "stop_music"
    ) {
        return SurfaceSemanticKind::Effect;
    }
    if matches!(
        rewrite_effect_command_syntax(token),
        Some(RewriteEffectCommandSyntax::Emission)
    ) {
        SurfaceSemanticKind::Emission
    } else {
        SurfaceSemanticKind::Effect
    }
}

fn add_level_flow_scene_effect_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) -> bool {
    match tokens {
        [command, levels, in_keyword, scene]
            if levels.text == "levels" && in_keyword.text == "in" =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            add_scene_effect_token_range(sink, levels, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, in_keyword, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, scene, SurfaceSemanticKind::Scene);
            true
        }
        [command, levels, scope, in_keyword, scene]
            if levels.text == "levels" && in_keyword.text == "in" =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            add_scene_effect_token_range(sink, levels, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, scope, SurfaceSemanticKind::Scene);
            add_scene_effect_token_range(sink, in_keyword, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, scene, SurfaceSemanticKind::Scene);
            true
        }
        _ => false,
    }
}

fn add_scene_command_token(sink: &mut SurfaceSink, token: &SourceToken) {
    if let Some(cursor_offset) = token.text.find("cursor.") {
        add_scene_effect_token_subrange(
            sink,
            token,
            cursor_offset,
            cursor_offset + "cursor".len(),
            SurfaceSemanticKind::State,
        );
        let value_start = cursor_offset + "cursor.".len();
        if let Some(value_end) = scene_effect_identifier_end(&token.text, value_start) {
            let value = &token.text[value_start..value_end];
            let kind = if matches!(value, "prev" | "next") {
                SurfaceSemanticKind::Effect
            } else {
                SurfaceSemanticKind::Literal
            };
            add_scene_effect_token_subrange(sink, token, value_start, value_end, kind);
        }
    }

    let Some((first_start, first_end)) = scene_effect_first_identifier_bounds(&token.text) else {
        return;
    };
    let after_first = &token.text[first_end..];
    if after_first.starts_with('.') {
        add_scene_effect_token_subrange(
            sink,
            token,
            first_start,
            first_end,
            SurfaceSemanticKind::Scene,
        );
        let command_start = first_end + 1;
        if let Some(command_end) = scene_effect_identifier_end(&token.text, command_start) {
            add_scene_effect_token_subrange(
                sink,
                token,
                command_start,
                command_end,
                SurfaceSemanticKind::Effect,
            );
        }
    } else {
        add_scene_effect_token_subrange(
            sink,
            token,
            first_start,
            first_end,
            SurfaceSemanticKind::Input,
        );
        if after_first.starts_with(':') {
            let binding_start = first_end + 1;
            if let Some(binding_end) = scene_effect_identifier_end(&token.text, binding_start) {
                add_scene_effect_token_subrange(
                    sink,
                    token,
                    binding_start,
                    binding_end,
                    SurfaceSemanticKind::Binding,
                );
            }
        }
    }
}

fn add_cursor_scene_effect_token(sink: &mut SurfaceSink, token: &SourceToken) {
    add_scene_effect_token_part(sink, token, "cursor", SurfaceSemanticKind::State);
    if let Some((_, tail)) = token.text.split_once('.') {
        let kind = if matches!(tail, "prev" | "next") {
            SurfaceSemanticKind::Effect
        } else {
            SurfaceSemanticKind::Literal
        };
        add_scene_effect_token_part(sink, token, tail, kind);
    }
}

fn add_scene_effect_token_range(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    kind: SurfaceSemanticKind,
) {
    let Some((start, end)) = scene_effect_identifier_bounds(token) else {
        return;
    };
    sink.mark(SourceSpan { start, end }, kind);
}

fn add_scene_effect_token_part(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    part: &str,
    kind: SurfaceSemanticKind,
) {
    if part.is_empty() {
        return;
    }
    if let Some(relative) = token.text.find(part) {
        sink.mark(
            SourceSpan {
                start: token.start + relative,
                end: token.start + relative + part.len(),
            },
            kind,
        );
    }
}

fn add_scene_effect_token_subrange(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    relative_start: usize,
    relative_end: usize,
    kind: SurfaceSemanticKind,
) {
    if relative_start >= relative_end || relative_end > token.text.len() {
        return;
    }
    sink.mark(
        SourceSpan {
            start: token.start + relative_start,
            end: token.start + relative_end,
        },
        kind,
    );
}

fn scene_effect_first_identifier_bounds(value: &str) -> Option<(usize, usize)> {
    let start = value
        .char_indices()
        .find_map(|(index, ch)| scene_effect_is_word_start(ch).then_some(index))?;
    scene_effect_identifier_end(value, start).map(|end| (start, end))
}

fn scene_effect_identifier_end(value: &str, start: usize) -> Option<usize> {
    if start >= value.len() {
        return None;
    }
    let mut end = start;
    for (offset, ch) in value[start..].char_indices() {
        if offset == 0 {
            if !scene_effect_is_word_start(ch) {
                return None;
            }
        } else if !scene_effect_is_word_continue(ch) || matches!(ch, ':' | '.') {
            break;
        }
        end = start + offset + ch.len_utf8();
    }
    (end > start).then_some(end)
}

fn scene_effect_identifier_bounds(token: &SourceToken) -> Option<(usize, usize)> {
    let start_offset = token
        .text
        .char_indices()
        .find_map(|(index, ch)| scene_effect_is_word_start(ch).then_some(index))?;
    let end_offset = token.text.char_indices().rev().find_map(|(index, ch)| {
        scene_effect_is_word_continue(ch).then_some(index + ch.len_utf8())
    })?;
    let start = token.start + start_offset;
    let end = token.start + end_offset;
    debug_assert!(end <= token.end);
    (start < end).then_some((start, end))
}

fn scene_effect_is_word_start(ch: char) -> bool {
    ch == '@' || ch == '_' || ch.is_ascii_alphabetic()
}

fn scene_effect_is_word_continue(ch: char) -> bool {
    ch == '@' || ch == '_' || ch == '-' || ch.is_ascii_alphanumeric()
}

impl EffectAst {
    fn command_syntax(&self) -> RewriteEffectCommandSyntax {
        match self {
            EffectAst::PlaySfx { .. }
            | EffectAst::PlayMusic { .. }
            | EffectAst::PauseMusic { .. }
            | EffectAst::ResumeMusic { .. }
            | EffectAst::StopMusic { .. }
            | EffectAst::Wait { .. }
            | EffectAst::WaitAnimation
            | EffectAst::Message { .. }
            | EffectAst::Scene(_) => RewriteEffectCommandSyntax::Emission,
            EffectAst::Cancel
            | EffectAst::Win
            | EffectAst::Restart
            | EffectAst::NextLevel
            | EffectAst::Again
            | EffectAst::Checkpoint
            | EffectAst::ClearCheckpoint
            | EffectAst::UpdateGlobal { .. } => RewriteEffectCommandSyntax::Effect,
        }
    }
}

pub(crate) fn rewrite_effect_command_syntax(token: &str) -> Option<RewriteEffectCommandSyntax> {
    let probe = match token {
        "again" | "cancel" | "win" | "restart" | "next_level" | "checkpoint"
        | "clear_checkpoint" | "wait" => token.to_string(),
        "message" => "message \"text\"".to_string(),
        "sfx" => "sfx __highlight_probe".to_string(),
        "play_music" => "play_music __highlight_probe".to_string(),
        "pause_music" => token.to_string(),
        "resume_music" => token.to_string(),
        "stop_music" => token.to_string(),
        _ => return None,
    };
    parse_rewrite_effect_value(&probe, &probe)
        .ok()
        .and_then(|effects| effects.into_iter().next())
        .map(|effect| effect.command_syntax())
}

pub(crate) fn rewrite_effect_semantic_tokens(
    tokens: &[SourceToken],
) -> Vec<semantic::SemanticToken> {
    project_surface_semantic_tokens(&rewrite_effect_surface_document(tokens).semantic_tokens)
}

fn rewrite_effect_surface_document(tokens: &[SourceToken]) -> SurfaceDocument {
    let mut sink = SurfaceSink::default();
    let effect_span = source_tokens_span(tokens);
    add_rewrite_effect_semantic_tokens(tokens, &mut sink);
    surface_document_with_node(sink, SurfaceNodeKind::RewriteEffect, effect_span)
}

fn add_rewrite_effect_semantic_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };

    if first.text == "message" {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Emission);
        if tokens.len() > 1 {
            let text_start = tokens[1].start;
            let text_end = tokens.last().map(|token| token.end).unwrap_or(text_start);
            if text_start < text_end {
                sink.mark(
                    SourceSpan {
                        start: text_start,
                        end: text_end,
                    },
                    SurfaceSemanticKind::String,
                );
            }
        }
        return true;
    }

    if tokens.len() > 2
        && tokens
            .iter()
            .any(|token| is_rewrite_effect_command_token(&token.text))
    {
        return add_simple_rewrite_effect_semantic_tokens(tokens, sink);
    }

    match tokens {
        [command] if command.text == "sfx" => {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            true
        }
        [command]
            if matches_rewrite_effect_command(
                &command.text,
                RewriteEffectCommandSyntax::Effect,
            ) =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            true
        }
        [command]
            if matches_rewrite_effect_command(
                &command.text,
                RewriteEffectCommandSyntax::Emission,
            ) =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Emission);
            true
        }
        [command, duration] if command.text == "wait" => {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Emission);
            add_scene_effect_token_range(sink, duration, SurfaceSemanticKind::Number);
            true
        }
        [command, asset] if command.text == "sfx" => {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            add_scene_effect_token_range(sink, asset, SurfaceSemanticKind::Asset);
            true
        }
        [name, op, value] if is_global_update_operator(&op.text) => {
            add_scene_effect_token_range(sink, name, SurfaceSemanticKind::State);
            add_scene_effect_token_range(sink, value, SurfaceSemanticKind::Number);
            true
        }
        _ => false,
    }
}

fn add_simple_rewrite_effect_semantic_tokens(
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    let mut index = 0usize;
    let mut parsed_any = false;
    while index < tokens.len() {
        match tokens[index].text.to_ascii_lowercase().as_str() {
            "cancel" | "win" | "restart" | "next_level" | "again" | "checkpoint"
            | "clear_checkpoint" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Effect);
                index += 1;
                parsed_any = true;
            }
            "wait" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Emission);
                if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(&tokens[index + 1].text)
                {
                    add_scene_effect_token_range(
                        sink,
                        &tokens[index + 1],
                        SurfaceSemanticKind::Number,
                    );
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            "sfx" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Effect);
                if let Some(asset) = tokens.get(index + 1) {
                    add_scene_effect_token_range(sink, asset, SurfaceSemanticKind::Asset);
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            "play_music" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Emission);
                if let Some(asset) = tokens.get(index + 1) {
                    add_scene_effect_token_range(sink, asset, SurfaceSemanticKind::Asset);
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            "pause_music" | "resume_music" | "stop_music" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Emission);
                if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(&tokens[index + 1].text)
                {
                    add_scene_effect_token_range(
                        sink,
                        &tokens[index + 1],
                        SurfaceSemanticKind::Asset,
                    );
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            _ if index + 2 < tokens.len() && is_global_update_operator(&tokens[index + 1].text) => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::State);
                add_scene_effect_token_range(sink, &tokens[index + 2], SurfaceSemanticKind::Number);
                index += 3;
                parsed_any = true;
            }
            _ => {
                return parsed_any;
            }
        }
    }
    parsed_any
}

fn matches_rewrite_effect_command(token: &str, syntax: RewriteEffectCommandSyntax) -> bool {
    rewrite_effect_command_syntax(&token.to_ascii_lowercase()) == Some(syntax)
}

pub(crate) fn rewrite_direction_prefix_token_index(tokens: &[&str]) -> Option<usize> {
    let mut index = 0usize;
    while tokens
        .get(index)
        .is_some_and(|token| rewrite_application_keyword(token))
    {
        index += 1;
    }
    let direction = tokens.get(index).copied()?;
    if !direction_word(direction) {
        return None;
    }
    tokens
        .get(index + 1)
        .is_some_and(|token| matches!(*token, "[" | "{"))
        .then_some(index)
}

fn rewrite_application_keyword(value: &str) -> bool {
    matches!(
        value,
        "fix" | "once" | "once_all" | "once_per_level" | "repeat"
    )
}

fn direction_word(value: &str) -> bool {
    matches!(value, "up" | "down" | "left" | "right")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SoundSettingValueSyntax {
    String,
    Number,
}

pub(crate) fn sound_setting_value_syntax(key: &str) -> Option<SoundSettingValueSyntax> {
    match key {
        "seed" | "type" => Some(SoundSettingValueSyntax::String),
        "height" | "tone" | "bars" | "bpm" | "volume" => Some(SoundSettingValueSyntax::Number),
        _ => None,
    }
}

pub(crate) const SFX_SOUND_SETTING_OPTIONS: &[&str] = &["seed", "type", "volume"];
pub(crate) const MUSIC_SOUND_SETTING_OPTIONS: &[&str] =
    &["seed", "height", "bars", "bpm", "volume"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MapHeaderTokenSyntax {
    Keyword,
    Name,
    Axis,
}

pub(crate) fn map_header_token_syntax(
    tokens: &[&str],
    index: usize,
) -> Option<MapHeaderTokenSyntax> {
    if !matches!(tokens, ["map", _, _]) {
        return None;
    }
    match index {
        0 => Some(MapHeaderTokenSyntax::Keyword),
        1 => Some(MapHeaderTokenSyntax::Name),
        2 => Some(MapHeaderTokenSyntax::Axis),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SceneStateLhsSyntax {
    PuzzleSlot,
    Variable,
}

pub(crate) fn scene_state_lhs_syntax(tokens: &[&str]) -> Option<(usize, SceneStateLhsSyntax)> {
    match tokens {
        [name, "=", "puzzle", ..] if is_identifier(name) => {
            Some((0, SceneStateLhsSyntax::PuzzleSlot))
        }
        ["var" | "const", name, "=", ..] if is_identifier(name) => {
            Some((1, SceneStateLhsSyntax::Variable))
        }
        ["persistent", "var" | "const", name, "=", ..] if is_identifier(name) => {
            Some((2, SceneStateLhsSyntax::Variable))
        }
        ["persistent", name, "=", ..] if is_identifier(name) => {
            Some((1, SceneStateLhsSyntax::Variable))
        }
        [name, "=", ..] if is_identifier(name) => Some((0, SceneStateLhsSyntax::Variable)),
        _ => None,
    }
}

pub(crate) fn metadata_directive_value_token_index(tokens: &[&str]) -> Option<usize> {
    matches!(
        tokens,
        ["title" | "subtitle" | "author" | "homepage", _, ..]
    )
    .then_some(1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LevelPathPartSyntax {
    Owner,
    TextProperty,
    NumberProperty,
    ConditionProperty,
}

pub(crate) fn level_path_part_syntax(parts: &[&str], index: usize) -> Option<LevelPathPartSyntax> {
    match parts {
        ["level", property] => match index {
            0 => None,
            1 => level_property_syntax(property),
            _ => None,
        },
        [_, "level", property] => match index {
            0 => Some(LevelPathPartSyntax::Owner),
            1 => None,
            2 => level_property_syntax(property),
            _ => None,
        },
        _ => None,
    }
}

fn level_property_syntax(property: &str) -> Option<LevelPathPartSyntax> {
    match property {
        "name" | "label" | "title" => Some(LevelPathPartSyntax::TextProperty),
        "index" | "num" => Some(LevelPathPartSyntax::NumberProperty),
        "cleared" | "solved" | "last" | "has_next" => Some(LevelPathPartSyntax::ConditionProperty),
        _ => None,
    }
}

fn parse_scene_effect_with_optional_block(
    value: &str,
    lines: &[String],
    start: usize,
) -> Result<(SceneEffect, usize), DiagnosticReport> {
    let line = &lines[start];
    if value.is_empty() {
        return Err(parse_error(
            line,
            "effect block must use `{ ... }`; `end` effect blocks were removed",
        ));
    }
    if value == "{" {
        let block_end = matching_effect_block_end(lines, start, lines.len())?;
        let body = lines[start + 1..block_end].to_vec();
        if body.is_empty() {
            return Err(parse_error(
                line,
                "effect block requires at least one effect",
            ));
        }
        return Ok((parse_scene_handler_effects(&body, line)?, block_end + 1));
    }

    Ok((parse_scene_effect(value, line)?, start + 1))
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedSceneEffect {
    pub(crate) surface: SurfaceSceneEffect,
    pub(crate) semantic_tokens: Vec<semantic::SemanticToken>,
}

fn parse_scene_effect(value: &str, line: &str) -> Result<SceneEffect, DiagnosticReport> {
    let parsed = parse_scene_effect_with_semantic_tokens(value, line)?;
    debug_assert!(
        parsed
            .semantic_tokens
            .iter()
            .all(|token| token.start < token.end)
    );
    Ok(parsed.surface.effect)
}

fn parse_scene_effect_with_semantic_tokens(
    value: &str,
    line: &str,
) -> Result<ParsedSceneEffect, DiagnosticReport> {
    let surface = parse_surface_scene_effect(value, line)?;
    let semantic_tokens = project_surface_semantic_tokens(&surface.document.semantic_tokens);
    Ok(ParsedSceneEffect {
        surface,
        semantic_tokens,
    })
}

fn parse_surface_scene_effect(
    value: &str,
    line: &str,
) -> Result<SurfaceSceneEffect, DiagnosticReport> {
    let tokens = source_line_tokens(strip_line_comment(value), 0);
    let document = scene_effect_surface_document(&tokens);
    let effect = parse_scene_effect_value(value, line)?;
    Ok(SurfaceSceneEffect { effect, document })
}

fn parse_scene_effect_value(value: &str, line: &str) -> Result<SceneEffect, DiagnosticReport> {
    if value.contains(" then ") {
        return Err(parse_error(
            line,
            "`then` effect sequences are not supported; use an effect block with one effect per line",
        ));
    }

    if let Some(parts) = split_scene_effect_sequence(value) {
        let mut effects = Vec::new();
        for part in parts {
            effects.push(parse_scene_effect_value(part, line)?);
        }
        return match effects.len() {
            0 => unreachable!("scene effect sequence splitter returned no effects"),
            1 => Ok(effects.remove(0)),
            _ => Ok(SceneEffect::Sequence(effects)),
        };
    }

    if let Some(text) = value.strip_prefix("message ") {
        return Ok(SceneEffect::Message {
            text: parse_scene_expr(text.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("current_level = ") {
        return Ok(SceneEffect::SetCurrentLevel {
            level: parse_scene_level_expr(rest.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("level.cleared = ") {
        return Ok(SceneEffect::SetLevelCleared {
            level: None,
            cleared: parse_scene_effect_bool(rest.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("level(") {
        if let Some((level, cleared)) = rest.split_once(").cleared = ") {
            return Ok(SceneEffect::SetLevelCleared {
                level: Some(parse_scene_level_expr(level.trim(), line)?),
                cleared: parse_scene_effect_bool(cleared.trim(), line)?,
            });
        }
    }
    if let Some((name, rhs)) = parse_scene_variable_assignment(value) {
        return Ok(SceneEffect::SetVariable {
            name: name.to_string(),
            value: parse_scene_expr(rhs, line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("goto ") {
        let (scene, params) = parse_scene_target_params(rest, line)?;
        return Ok(SceneEffect::Goto { scene, params });
    }
    if let Some(rest) = value.strip_prefix("start ") {
        if rest.starts_with("levels ") || rest.contains(" in ") {
            return Err(legacy_start_levels_error(line));
        }
        let (scene, params) = parse_scene_target_params(rest, line)?;
        return Ok(SceneEffect::Sequence(vec![
            SceneEffect::Reset {
                scene: scene.clone(),
            },
            SceneEffect::Goto { scene, params },
        ]));
    }

    let tokens = split_header_tokens(value);
    match tokens.as_slice() {
        ["input", input] => Ok(SceneEffect::Input(
            parse_input_name(input, line)?.to_string(),
        )),
        ["component_effect", effect] => Ok(SceneEffect::ComponentEffect(
            parse_scene_signal_name(effect, line, "component effect")?.to_string(),
        )),
        ["apply", call, "to", target] => {
            validate_target_path(target, line, "apply target")?;
            let (rule, args) = parse_rule_call_expr(call, line)?;
            Ok(SceneEffect::Apply {
                rule,
                args,
                target: Some((*target).to_string()),
            })
        }
        ["apply", call] => {
            let (rule, args) = parse_rule_call_expr(call, line)?;
            Ok(SceneEffect::Apply {
                rule,
                args,
                target: None,
            })
        }
        ["copy", source, "to", target] => {
            validate_target_path(source, line, "copy source")?;
            validate_target_path(target, line, "copy target")?;
            Ok(SceneEffect::Copy {
                source: (*source).to_string(),
                target: (*target).to_string(),
            })
        }
        ["load", target, "from", source] => {
            validate_target_path(target, line, "load target")?;
            Ok(SceneEffect::LoadPuzzle {
                target: (*target).to_string(),
                source: (*source).to_string(),
            })
        }
        ["wait"] => Ok(SceneEffect::Wait { milliseconds: None }),
        ["wait", duration] => Ok(SceneEffect::Wait {
            milliseconds: Some(parse_wait_duration_ms(duration, line)?),
        }),
        ["clear_undo_history"] | ["clear_history"] => Ok(SceneEffect::ClearUndoHistory),
        ["clear_game_progress"] => Ok(SceneEffect::ClearGameProgress),
        ["clear", "current_level"] => Ok(SceneEffect::ClearCurrentLevel),
        ["reset", "persistent_vars"] => Ok(SceneEffect::ResetPersistentVars),
        ["sfx", name] => {
            validate_qualified_identifier(name, line, "sfx sounds name")?;
            Ok(SceneEffect::PlaySfx {
                name: (*name).to_string(),
            })
        }
        ["play_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::PlayMusic {
                name: (*name).to_string(),
            })
        }
        ["pause_music"] => Ok(SceneEffect::PauseMusic { name: None }),
        ["pause_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::PauseMusic {
                name: Some((*name).to_string()),
            })
        }
        ["resume_music"] => Ok(SceneEffect::ResumeMusic { name: None }),
        ["resume_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::ResumeMusic {
                name: Some((*name).to_string()),
            })
        }
        ["stop_music"] => Ok(SceneEffect::StopMusic { name: None }),
        ["stop_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::StopMusic {
                name: Some((*name).to_string()),
            })
        }
        ["reset", target] if target.contains('.') => {
            validate_target_path(target, line, "reset target")?;
            Ok(SceneEffect::ResetPuzzle {
                target: (*target).to_string(),
            })
        }
        ["start", "levels", ..] | ["start", _, "in", _] | ["continue", "levels", ..] => {
            Err(legacy_start_levels_error(line))
        }
        [target_command, level] => {
            if let Some((target, command)) = parse_puzzle_command(target_command, line)? {
                if command == "goto" {
                    return Ok(SceneEffect::GotoLevel {
                        target,
                        level: parse_scene_level_expr(level, line)?,
                    });
                }
            }
            Err(parse_error(
                line,
                "effect must be: input <name> | component_effect <name> | goto <scene> | goto <scene>(<level>) | start <scene> | start <scene>(<level>) | clear_undo_history | clear_game_progress | message <text> | wait <duration> | sfx <name> | play_music <name> | pause_music [name] | resume_music [name] | stop_music [name] | <scene>.goto <level> | copy <puzzle> to <puzzle>",
            ))
        }
        ["input"] => Err(parse_error(line, "input effect must name an input")),
        ["component_effect"] => Err(parse_error(
            line,
            "component_effect must name a component effect",
        )),
        [command_text] => {
            if let Some((target, command)) = parse_puzzle_command(command_text, line)? {
                if command == "next_level" {
                    return Ok(SceneEffect::PuzzleNextLevel { target });
                }
                if command == "previous_level" {
                    return Ok(SceneEffect::PuzzlePreviousLevel { target });
                }
                if command == "restart" {
                    return Ok(SceneEffect::ResetPuzzle { target });
                }
            }
            if is_identifier(command_text) {
                return Ok(SceneEffect::RoutineCall((*command_text).to_string()));
            }
            Err(parse_error(
                line,
                "bare scene effect aliases were removed; use `input <name>`, `component_effect <name>`, a scene routine, or an explicit scene effect",
            ))
        }
        _ => Err(parse_error(
            line,
            "effect must be: input <name> | component_effect <name> | goto <scene> | goto <scene>(<level>) | start <scene> | start <scene>(<level>) | clear_undo_history | clear_game_progress | message <text> | wait <duration> | sfx <name> | play_music <name> | pause_music [name] | resume_music [name] | stop_music [name] | copy <puzzle> to <puzzle>",
        )),
    }
}

fn parse_scene_variable_assignment(value: &str) -> Option<(&str, &str)> {
    let (name, rhs) = value.split_once('=')?;
    let name = name.trim();
    let rhs = rhs.trim();
    if rhs.is_empty() || !is_identifier(name) || reserved_scene_assignment_target(name) {
        return None;
    }
    Some((name, rhs))
}

fn reserved_scene_assignment_target(name: &str) -> bool {
    matches!(name, "current_level" | "level")
}

fn split_scene_effect_sequence(value: &str) -> Option<Vec<&str>> {
    let stripped = strip_line_comment(value);
    let tokens = source_line_tokens(stripped, 0);
    let parts = split_scene_effect_token_sequence(&tokens)?;
    Some(
        parts
            .into_iter()
            .map(|part| stripped[part.first().unwrap().start..part.last().unwrap().end].trim())
            .collect(),
    )
}

fn split_scene_effect_token_sequence(tokens: &[SourceToken]) -> Option<Vec<&[SourceToken]>> {
    let mut parts = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let length = scene_effect_token_length(&tokens[index..])?;
        parts.push(&tokens[index..index + length]);
        index += length;
    }
    (parts.len() > 1).then_some(parts)
}

fn scene_effect_token_length(tokens: &[SourceToken]) -> Option<usize> {
    let first = tokens.first()?.text.as_str();
    match first {
        "input" | "component_effect" | "sfx" | "play_music" => (tokens.len() >= 2).then_some(2),
        "pause_music" | "resume_music" | "stop_music" => {
            if tokens
                .get(1)
                .is_some_and(|token| !is_scene_effect_command_start(&token.text))
            {
                Some(2)
            } else {
                Some(1)
            }
        }
        "wait" => {
            if tokens
                .get(1)
                .is_some_and(|token| !is_scene_effect_command_start(&token.text))
            {
                Some(2)
            } else {
                Some(1)
            }
        }
        "clear_undo_history" | "clear_history" | "clear_game_progress" => Some(1),
        "clear" => (tokens.get(1)?.text == "current_level").then_some(2),
        "reset"
            if tokens
                .get(1)
                .is_some_and(|token| token.text == "persistent_vars") =>
        {
            Some(2)
        }
        "reset" if tokens.get(1).is_some_and(|token| token.text.contains('.')) => Some(2),
        "goto" | "start" => {
            if tokens.get(2).is_some_and(|token| token.text == "with") {
                None
            } else {
                (tokens.len() >= 2).then_some(2)
            }
        }
        _ if first.contains('.') => {
            let command = first.rsplit_once('.').map(|(_, command)| command)?;
            match command {
                "goto" | "goto_level" => (tokens.len() >= 2).then_some(2),
                "next_level" | "previous_level" | "restart" => Some(1),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_scene_effect_command_start(token: &str) -> bool {
    scene_effect_command_syntax(token).is_some()
        || token.rsplit_once('.').is_some_and(|(_, command)| {
            matches!(
                command,
                "goto" | "goto_level" | "next_level" | "previous_level" | "restart"
            )
        })
}

fn parse_scene_effect_bool(value: &str, line: &str) -> Result<bool, DiagnosticReport> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(parse_error(
            line,
            "boolean progress value must be true or false",
        )),
    }
}

fn legacy_start_levels_error(line: &str) -> DiagnosticReport {
    parse_error(
        line,
        "`start levels ... in <scene>` and `continue levels ... in <scene>` are no longer supported; use `goto <puzzle>` for the default playable scene, `goto <puzzle>(<level>)` for a specific level, or `goto <scene>(<level>)` for an explicit level scene",
    )
}

const DEFAULT_WAIT_MS: u64 = 200;
const DEFAULT_AGAIN_MS: u64 = 120;

fn resolve_default_wait_in_scenes(scenes: &mut [SceneDef], default_wait_ms: u64) {
    for scene in scenes {
        for component in &mut scene.components {
            resolve_default_wait_in_component(component, default_wait_ms);
        }
        for binding in &mut scene.key_bindings {
            resolve_default_wait_in_effect(&mut binding.effect, default_wait_ms);
        }
        for routine in &mut scene.routines {
            resolve_default_wait_in_effect(&mut routine.effect, default_wait_ms);
        }
        for transition in &mut scene.transitions {
            resolve_default_wait_in_effect(&mut transition.effect, default_wait_ms);
        }
    }
}

fn resolve_default_wait_in_component(component: &mut SceneComponent, default_wait_ms: u64) {
    match component {
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            resolve_default_wait_in_effect(&mut button.effect, default_wait_ms);
        }
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => {
            for child in &mut container.children {
                resolve_default_wait_in_component(child, default_wait_ms);
            }
        }
        SceneComponent::Conditional(conditional) => {
            for child in &mut conditional.children {
                resolve_default_wait_in_component(child, default_wait_ms);
            }
        }
        SceneComponent::For(for_view) => {
            for child in &mut for_view.children {
                resolve_default_wait_in_component(child, default_wait_ms);
            }
        }
        SceneComponent::LevelMenu(menu) => {
            for button in &mut menu.buttons {
                resolve_default_wait_in_effect(&mut button.effect, default_wait_ms);
            }
        }
        SceneComponent::Frame(_)
        | SceneComponent::Title(_)
        | SceneComponent::Subtitle(_)
        | SceneComponent::Text(_) => {}
    }
}

fn resolve_default_wait_in_effect(effect: &mut SceneEffect, default_wait_ms: u64) {
    match effect {
        SceneEffect::Wait { milliseconds } => {
            if milliseconds.is_none() {
                *milliseconds = Some(default_wait_ms);
            }
        }
        SceneEffect::Conditional { effect, .. } => {
            resolve_default_wait_in_effect(effect, default_wait_ms);
        }
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                resolve_default_wait_in_effect(effect, default_wait_ms);
            }
        }
        SceneEffect::Input(_)
        | SceneEffect::ComponentEffect(_)
        | SceneEffect::RoutineCall(_)
        | SceneEffect::Message { .. }
        | SceneEffect::PlaySfx { .. }
        | SceneEffect::PlayMusic { .. }
        | SceneEffect::PauseMusic { .. }
        | SceneEffect::ResumeMusic { .. }
        | SceneEffect::StopMusic { .. }
        | SceneEffect::Goto { .. }
        | SceneEffect::Enter { .. }
        | SceneEffect::Back
        | SceneEffect::Create { .. }
        | SceneEffect::Reset { .. }
        | SceneEffect::Delete { .. }
        | SceneEffect::Show { .. }
        | SceneEffect::Hide { .. }
        | SceneEffect::Toggle { .. }
        | SceneEffect::Focus { .. }
        | SceneEffect::PuzzleNextLevel { .. }
        | SceneEffect::PuzzlePreviousLevel { .. }
        | SceneEffect::GotoLevel { .. }
        | SceneEffect::ResetPuzzle { .. }
        | SceneEffect::LoadPuzzle { .. }
        | SceneEffect::Apply { .. }
        | SceneEffect::Copy { .. }
        | SceneEffect::SetVariable { .. }
        | SceneEffect::ClearUndoHistory
        | SceneEffect::ClearGameProgress
        | SceneEffect::SetCurrentLevel { .. }
        | SceneEffect::ClearCurrentLevel
        | SceneEffect::SetLevelCleared { .. }
        | SceneEffect::ResetPersistentVars => {}
    }
}

fn parse_wait_duration_ms(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    if let Some(milliseconds) = value.strip_suffix("ms") {
        return parse_whole_milliseconds(milliseconds, line);
    }
    if let Some(seconds) = value.strip_suffix('s') {
        return parse_seconds_duration_ms(seconds, line);
    }
    Err(parse_error(
        line,
        "wait duration must use seconds or milliseconds, for example `wait 0.1s` or `wait 100ms`",
    ))
}

fn parse_whole_milliseconds(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait milliseconds must be a whole number",
        ));
    }
    value
        .parse::<u64>()
        .map_err(|_| parse_error(line, "wait duration is too large"))
}

fn parse_seconds_duration_ms(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    let value = value.trim();
    let has_decimal = value.contains('.');
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty() || !whole.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait seconds must be a non-negative number",
        ));
    }
    if (has_decimal && fraction.is_empty()) || !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait seconds must be a non-negative number",
        ));
    }
    if fraction.len() > 3 {
        return Err(parse_error(
            line,
            "wait seconds can use at most millisecond precision",
        ));
    }
    let whole_ms = whole
        .parse::<u64>()
        .map_err(|_| parse_error(line, "wait duration is too large"))?
        .checked_mul(1000)
        .ok_or_else(|| parse_error(line, "wait duration is too large"))?;
    let fraction_ms = if fraction.is_empty() {
        0
    } else {
        let padded = format!("{fraction:0<3}");
        padded
            .parse::<u64>()
            .map_err(|_| parse_error(line, "wait duration is too large"))?
    };
    whole_ms
        .checked_add(fraction_ms)
        .ok_or_else(|| parse_error(line, "wait duration is too large"))
}

fn parse_rule_call_expr(
    value: &str,
    line: &str,
) -> Result<(String, Vec<SceneExpr>), DiagnosticReport> {
    let Some((name, args)) = value.split_once('(') else {
        validate_qualified_identifier(value, line, "rule name")?;
        return Ok((value.to_string(), Vec::new()));
    };
    validate_qualified_identifier(name, line, "rule name")?;
    let args = args
        .strip_suffix(')')
        .ok_or_else(|| parse_error(line, "rule call args must end with )"))?;
    let args = if args.trim().is_empty() {
        Vec::new()
    } else {
        args.split(',')
            .map(str::trim)
            .map(|arg| parse_scene_expr(arg, line))
            .collect::<Result<Vec<_>, DiagnosticReport>>()?
    };
    Ok((name.to_string(), args))
}

fn parse_scene_call_params(
    value: &str,
    line: &str,
) -> Result<Option<(String, Vec<SceneEffectParam>)>, DiagnosticReport> {
    let Some(open) = value.find('(') else {
        return Ok(None);
    };
    if !value.ends_with(')') {
        return Err(parse_error(line, "scene call must close with `)`"));
    }
    let scene = value[..open].trim();
    validate_qualified_identifier(scene, line, "scene name")?;
    let args = value[open + 1..value.len() - 1].trim();
    if args.is_empty() {
        return Ok(Some((scene.to_string(), Vec::new())));
    }

    let parts = args.split(',').map(str::trim).collect::<Vec<_>>();
    let params = if parts.len() == 1 && !parts[0].contains('=') {
        vec![SceneEffectParam::Level(parse_scene_level_expr(
            parts[0], line,
        )?)]
    } else {
        parse_scene_named_params(&parts, line)?
    };
    Ok(Some((scene.to_string(), params)))
}

fn parse_scene_target_params(
    value: &str,
    line: &str,
) -> Result<(String, Vec<SceneEffectParam>), DiagnosticReport> {
    let value = value.trim();
    if let Some((scene, params)) = value.split_once(" with ") {
        let scene = scene.trim();
        validate_qualified_identifier(scene, line, "scene name")?;
        let parts = params.split(',').map(str::trim).collect::<Vec<_>>();
        return Ok((scene.to_string(), parse_scene_named_params(&parts, line)?));
    }
    if let Some((scene, params)) = parse_scene_call_params(value, line)? {
        return Ok((scene, params));
    }
    validate_qualified_identifier(value, line, "scene name")?;
    Ok((value.to_string(), Vec::new()))
}

fn parse_scene_named_params(
    parts: &[&str],
    line: &str,
) -> Result<Vec<SceneEffectParam>, DiagnosticReport> {
    let mut params = Vec::new();
    for part in parts {
        let (name, value) = part
            .split_once('=')
            .ok_or_else(|| parse_error(line, "scene params must be named `<name> = <expr>`"))?;
        let name = name.trim();
        validate_identifier(name, line, "scene param name")?;
        params.push(SceneEffectParam::Named {
            name: name.to_string(),
            value: parse_scene_expr(value.trim(), line)?,
        });
    }
    Ok(params)
}

fn parse_scene_level_expr(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    if parse_quoted_text(value).is_some() {
        return Err(parse_error(
            line,
            "scene level arguments must not be quoted; use `goto <scene>(<level_name>)`",
        ));
    }
    if is_dotted_level_atom(value) {
        return Ok(SceneExpr::Text(value.to_string()));
    }
    match parse_scene_expr(value, line) {
        Ok(expr) => Ok(expr),
        Err(error) => Err(error),
    }
}

fn parse_scene_expr(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    if value == "true" {
        return Ok(SceneExpr::Bool(true));
    }
    if value == "false" {
        return Ok(SceneExpr::Bool(false));
    }
    if let Ok(number) = value.parse::<i64>() {
        return Ok(SceneExpr::Int(number));
    }
    if let Some(text) = parse_quoted_text(value) {
        return Ok(SceneExpr::Text(text));
    }
    if value.contains('(') {
        let (name, args) = parse_rule_call_expr(value, line)?;
        return Ok(SceneExpr::Call { name, args });
    }
    if value.starts_with("join ") {
        return Err(parse_error(
            line,
            "`join` scene expression is not supported",
        ));
    }
    if let Some(path) = parse_view_path(value) {
        return Ok(SceneExpr::Path(path));
    }
    Err(parse_error(
        line,
        "expression must be true, false, integer, quoted text, or path",
    ))
}

fn is_dotted_level_atom(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() > 1
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        })
}

fn parse_input_name<'a>(value: &'a str, line: &str) -> Result<&'a str, DiagnosticReport> {
    validate_identifier(value, line, "input name")?;
    Ok(value)
}

fn parse_scene_signal_name<'a>(
    value: &'a str,
    line: &str,
    label: &str,
) -> Result<&'a str, DiagnosticReport> {
    validate_qualified_identifier(value, line, label)?;
    Ok(value)
}

fn parse_puzzle_command<'a>(
    value: &'a str,
    line: &str,
) -> Result<Option<(String, &'a str)>, DiagnosticReport> {
    let Some((target, command)) = value.split_once('.') else {
        return Ok(None);
    };
    validate_qualified_identifier(target, line, "puzzle target")?;
    validate_identifier(command, line, "puzzle command")?;
    Ok(Some((target.to_string(), command)))
}

fn validate_target_path(value: &str, line: &str, label: &str) -> Result<(), DiagnosticReport> {
    if parse_view_path(value).is_some() {
        Ok(())
    } else {
        Err(parse_error(
            line,
            &format!("{label} must be an identifier path"),
        ))
    }
}

#[derive(Clone, Copy)]
enum NameClass {
    Identifier,
    Qualified,
}

fn validate_name(
    value: &str,
    class: NameClass,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    let valid = match class {
        NameClass::Identifier => is_identifier(value),
        NameClass::Qualified => is_qualified_identifier(value),
    };
    if valid {
        Ok(())
    } else {
        let expected = match class {
            NameClass::Identifier => "an identifier",
            NameClass::Qualified => "a qualified identifier",
        };
        Err(parse_error(line, &format!("{label} must be {expected}")))
    }
}

fn validate_identifier(value: &str, line: &str, label: &str) -> Result<(), DiagnosticReport> {
    validate_name(value, NameClass::Identifier, line, label)
}

fn validate_qualified_identifier(
    value: &str,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    validate_name(value, NameClass::Qualified, line, label)
}

fn parse_view_path(value: &str) -> Option<Vec<String>> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || !parts.iter().all(|part| is_qualified_identifier(part)) {
        return None;
    }
    Some(parts.into_iter().map(ToString::to_string).collect())
}

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

fn parse_scene_state_block(
    lines: &[String],
    start: usize,
    lifetime: SceneStateLifetime,
) -> Result<(ParsedScreenStateBlock, usize), DiagnosticReport> {
    let mut variables = Vec::new();
    let mut puzzles = Vec::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        match parse_scene_state_entry(&lines[i], lifetime)? {
            ParsedSceneStateEntry::Variable(variable) => variables.push(variable),
            ParsedSceneStateEntry::Puzzle(puzzle) => puzzles.push(puzzle),
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "state missing closing brace"));
    }
    Ok((ParsedScreenStateBlock { variables, puzzles }, i + 1))
}

fn parse_scene_state_entry(
    line: &str,
    lifetime: SceneStateLifetime,
) -> Result<ParsedSceneStateEntry, DiagnosticReport> {
    let line = line.trim();
    if let Some(puzzle) = parse_implicit_scene_puzzle_state_entry(line, lifetime)? {
        return Ok(ParsedSceneStateEntry::Puzzle(puzzle));
    }
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
    let Some((name, value)) = line.split_once('=') else {
        return Err(parse_error(line, "scene state must be: <name> = <value>"));
    };
    let name = name.trim();
    if !is_identifier(name) {
        return Err(parse_error(line, "scene state name must be an identifier"));
    }
    let value = value.trim();
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

fn parse_implicit_scene_puzzle_state_entry(
    line: &str,
    lifetime: SceneStateLifetime,
) -> Result<Option<ScenePuzzleDef>, DiagnosticReport> {
    if let Some((puzzle_name, param)) = parse_scene_puzzle_state_call(line) {
        validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
        validate_identifier(param, line, "scene level param")?;
        if param != "level" {
            return Err(parse_error(
                line,
                "scene puzzle state call must be `<puzzle>(level)`",
            ));
        }
        return Ok(Some(ScenePuzzleDef {
            name: puzzle_name.to_string(),
            kind: INFERRED_SCENE_PUZZLE_KIND.to_string(),
            model: puzzle_name.to_string(),
            initializer: ScenePuzzleInitializer::CurrentLevel,
            lifetime,
        }));
    }
    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        [puzzle_name] if is_qualified_identifier(puzzle_name) => Ok(Some(ScenePuzzleDef {
            name: (*puzzle_name).to_string(),
            kind: INFERRED_SCENE_PUZZLE_KIND.to_string(),
            model: (*puzzle_name).to_string(),
            initializer: ScenePuzzleInitializer::CurrentLevel,
            lifetime,
        })),
        ["puzzle", puzzle_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            Ok(Some(ScenePuzzleDef {
                name: (*puzzle_name).to_string(),
                kind: "puzzle".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::CurrentLevel,
                lifetime,
            }))
        }
        ["puzzle", puzzle_name, "level", level_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            validate_qualified_identifier(level_name, line, "level name")?;
            Ok(Some(ScenePuzzleDef {
                name: (*puzzle_name).to_string(),
                kind: "puzzle".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::Level((*level_name).to_string()),
                lifetime,
            }))
        }
        ["puzzle3", puzzle_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle3 model name")?;
            Ok(Some(ScenePuzzleDef {
                name: (*puzzle_name).to_string(),
                kind: "puzzle3".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::CurrentLevel,
                lifetime,
            }))
        }
        ["puzzle3", puzzle_name, "level", level_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle3 model name")?;
            validate_qualified_identifier(level_name, line, "level name")?;
            Ok(Some(ScenePuzzleDef {
                name: (*puzzle_name).to_string(),
                kind: "puzzle3".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::Level((*level_name).to_string()),
                lifetime,
            }))
        }
        ["puzzle", ..] => Err(parse_error(
            line,
            "scene puzzle state must be: puzzle <name> | puzzle <name> level <level>",
        )),
        ["puzzle3", ..] => Err(parse_error(
            line,
            "scene puzzle3 state must be: puzzle3 <name> | puzzle3 <name> level <level>",
        )),
        _ => Ok(None),
    }
}

fn parse_scene_puzzle_state_call(line: &str) -> Option<(&str, &str)> {
    let (name, rest) = line.split_once('(')?;
    let param = rest.strip_suffix(')')?;
    let name = name.trim();
    let param = param.trim();
    if name.is_empty() || param.is_empty() || param.contains(',') {
        return None;
    }
    Some((name, param))
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
    let Some((name, value)) = rest.split_once('=') else {
        return Err(parse_error(
            line,
            "top-level variable must be: var <name> = <literal> or const <name> = <literal>",
        ));
    };
    let name = name.trim();
    let value = value.trim();
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
    let inner = value.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.replace("\\\"", "\""))
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
        validate_screen_condition(&lines[i], &lines[i])?;
        conditions.push(lines[i].clone());
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
    let Some((_, effect_text)) = arrow_line.split_once("->") else {
        return Err(parse_error(
            arrow_line,
            "scene condition block must be followed by ->",
        ));
    };
    let (effect, next_i) =
        parse_scene_effect_with_optional_block(effect_text.trim(), lines, arrow_i)?;
    Ok((
        SceneTransition {
            trigger: SceneTransitionTrigger::Condition(conditions.join(" and ")),
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
    validate_screen_condition(condition, line)?;
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
            trigger: SceneTransitionTrigger::Condition(condition.to_string()),
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
                validate_screen_condition(&condition, line)?;
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
    let Some((pattern, effect)) = lines[start].split_once("->") else {
        return Err(parse_error(
            &lines[start],
            "transition must be: if <condition> -> <effect>",
        ));
    };
    let (effect, next_i) = parse_scene_effect_with_optional_block(effect.trim(), lines, start)?;
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
        validate_screen_condition(condition, line)?;
        return Ok(SceneTransitionTrigger::Condition(condition.to_string()));
    }
    let tokens = split_header_tokens(value);
    if let [input] = tokens.as_slice() {
        let input = parse_input_name(input, line)?;
        return Ok(SceneTransitionTrigger::Condition(format!(
            "input == {input}"
        )));
    }
    Err(parse_error(
        line,
        "scene transition triggers must be `<input>` or `if <condition>`",
    ))
}

fn validate_screen_condition(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    if value.is_empty() {
        return Err(parse_error(line, "condition must not be empty"));
    }
    for part in value.split(" and ") {
        if validate_screen_condition_atom(part.trim()).is_err() {
            return Err(parse_error(
                line,
                "condition must be identifier paths or path comparisons joined by and",
            ));
        }
    }
    Ok(())
}

fn validate_screen_condition_atom(value: &str) -> Result<(), ()> {
    if parse_view_path(value).is_some() {
        return Ok(());
    }
    for op in [" == ", " != "] {
        if let Some((left, right)) = value.split_once(op) {
            if parse_view_path(left.trim()).is_none() {
                return Err(());
            }
            return validate_screen_condition_value(right.trim());
        }
    }
    Err(())
}

fn validate_screen_condition_value(value: &str) -> Result<(), ()> {
    if value == "true" || value == "false" || value.parse::<i64>().is_ok() {
        return Ok(());
    }
    if parse_quoted_text(value).is_some() {
        return Ok(());
    }
    parse_view_path(value).map(|_| ()).ok_or(())
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
        let Some((keys_text, input_text)) = lines[i].split_once("->") else {
            return Err(parse_error(
                &lines[i],
                "keys row must be: <key...> -> <input>",
            ));
        };
        let keys = keys_text.split_whitespace().collect::<Vec<_>>();
        let input_tokens = split_header_tokens(input_text.trim());
        match input_tokens.as_slice() {
            [input_name] if !keys.is_empty() => {
                let input = catalog
                    .input_names
                    .get(*input_name)
                    .copied()
                    .map(Ok)
                    .unwrap_or_else(|| add_input_name(input_name, &lines[i], catalog))?;
                for key in keys {
                    let trigger = parse_key_trigger(key, &lines[i])?;
                    if !seen_keys.insert(trigger.clone()) {
                        return Err(parse_error(&lines[i], "duplicate model input key"));
                    }
                    add_key_trigger_to_controls(&trigger, input, controls, &lines[i])?;
                }
            }
            _ => {
                return Err(parse_error(
                    &lines[i],
                    "keys row must be: <key...> -> <input>",
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "keys missing closing brace"));
    }
    Ok(i + 1)
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
    if lines[start].contains('=') {
        return Err(parse_error(
            &lines[start],
            "keys row must use `->`: <key...> -> <scene effect-or-input>",
        ));
    }
    let Some((key, effect)) = lines[start].split_once("->") else {
        return Err(parse_error(
            &lines[start],
            "keys row must be: <key...> -> <scene effect-or-input>",
        ));
    };
    let key_tokens = key.split_whitespace().collect::<Vec<_>>();
    if key_tokens.is_empty() {
        return Err(parse_error(
            &lines[start],
            "keys row must name at least one key",
        ));
    }
    let mut triggers = Vec::new();
    for key in key_tokens {
        let trigger = parse_key_trigger(key, &lines[start])?;
        validate_key_trigger_supported(&trigger, &lines[start])?;
        triggers.push(trigger);
    }
    let (effect, next_i) = parse_scene_effect_with_optional_block(effect.trim(), lines, start)?;
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

fn define_object_spec(
    spec: &str,
    layer: u16,
    render_spec: Option<&str>,
    line: &str,
    value_sets: &HashMap<String, Vec<String>>,
    object_schemas: &mut HashMap<String, ObjectSchema>,
    object_names: &mut HashMap<String, ObjectId>,
    object_labels: &mut HashMap<ObjectId, String>,
    object_layers: &mut HashMap<ObjectId, LayerId>,
    object_defs: &mut Vec<ObjectDef>,
    render_chars: &mut HashMap<ObjectId, char>,
    char_objects: &mut HashMap<char, Vec<ObjectId>>,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    let parts = spec.split(':').collect::<Vec<_>>();
    let base = parts[0];
    if parts.len() == 1 {
        if object_names.contains_key(spec) {
            return Err(parse_error(line, "duplicate object"));
        }
        let id = add_object_variant(
            spec,
            layer,
            object_names,
            object_labels,
            object_layers,
            object_defs,
        );
        if let Some(render) = render_spec {
            let ch = parse_render_chars(render, line)?
                .into_iter()
                .next()
                .ok_or_else(|| parse_error(line, "missing object render char"))?;
            render_chars.insert(id, ch);
            char_objects.insert(ch, vec![id]);
        }
        return Ok(vec![id]);
    }

    if object_schemas.contains_key(base) {
        return Err(parse_error(line, "duplicate object family"));
    }

    let axes = parts[1..]
        .iter()
        .map(|axis| {
            if !value_sets.contains_key(*axis) {
                return Err(parse_error(
                    line,
                    "object schema tag slot must name a tag set",
                ));
            }
            Ok((*axis).to_string())
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    let value_combinations = expand_axis_values(&axes, value_sets, line)?;
    let render_chars_for_variants = render_spec
        .map(|render| render_chars_for_variants(render, value_combinations.len(), line))
        .transpose()?;
    let mut variants = Vec::with_capacity(value_combinations.len());
    let mut created = Vec::with_capacity(value_combinations.len());

    for (index, values) in value_combinations.into_iter().enumerate() {
        let name = format!("{base}:{}", values.join(":"));
        let id = add_object_variant(
            &name,
            layer,
            object_names,
            object_labels,
            object_layers,
            object_defs,
        );
        if let Some(chars) = &render_chars_for_variants {
            let ch = chars[index];
            render_chars.insert(id, ch);
            if index == 0 {
                char_objects.insert(ch, vec![id]);
            } else if chars.iter().filter(|candidate| **candidate == ch).count() == 1 {
                char_objects.insert(ch, vec![id]);
            }
        }
        created.push(id);
        variants.push(ObjectVariant { values, object: id });
    }

    object_schemas.insert(base.to_string(), ObjectSchema { axes, variants });
    Ok(created)
}

fn add_object_variant(
    name: &str,
    layer: u16,
    object_names: &mut HashMap<String, ObjectId>,
    object_labels: &mut HashMap<ObjectId, String>,
    object_layers: &mut HashMap<ObjectId, LayerId>,
    object_defs: &mut Vec<ObjectDef>,
) -> ObjectId {
    let id = ObjectId((object_defs.len() + 1) as u16);
    object_names.insert(name.to_string(), id);
    object_labels.insert(id, name.to_string());
    object_layers.insert(id, LayerId(layer));
    object_defs.push(ObjectDef {
        id,
        layer_id: LayerId(layer),
    });
    id
}

fn expand_axis_values(
    axes: &[String],
    value_sets: &HashMap<String, Vec<String>>,
    line: &str,
) -> Result<Vec<Vec<String>>, DiagnosticReport> {
    let mut combinations = vec![Vec::<String>::new()];
    for axis in axes {
        let values = value_sets
            .get(axis)
            .ok_or_else(|| parse_error(line, "unknown object schema tag set"))?;
        let mut next = Vec::new();
        for prefix in &combinations {
            for value in values {
                let mut variant = prefix.clone();
                variant.push(value.clone());
                next.push(variant);
            }
        }
        combinations = next;
    }
    Ok(combinations)
}

fn render_chars_for_variants(
    render: &str,
    variant_count: usize,
    line: &str,
) -> Result<Vec<char>, DiagnosticReport> {
    let chars = parse_render_chars(render, line)?;
    if chars.len() == 1 {
        return Ok(vec![chars[0]; variant_count]);
    }
    if chars.len() == variant_count {
        return Ok(chars);
    }
    Err(parse_error(
        line,
        "object schema render chars must be one char or one char per variant",
    ))
}

fn parse_render_chars(render: &str, line: &str) -> Result<Vec<char>, DiagnosticReport> {
    let chars = render.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Err(parse_error(line, "missing object render char"));
    }
    Ok(chars)
}

type OverlayDefs = Vec<(Vec<ObjectId>, char)>;

#[derive(Clone, Debug)]
struct VisualShapeTable {
    axis: String,
    entries: HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug)]
struct VisualShapeRotation {
    map: Option<String>,
    from: String,
}

impl VisualShapeRotation {
    fn intrinsic(from: &str) -> Self {
        Self {
            map: None,
            from: from.to_string(),
        }
    }

    fn using(map: &str, from: &str) -> Self {
        Self {
            map: Some(map.to_string()),
            from: from.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
struct VisualColorTable {
    axis: String,
    entries: HashMap<String, String>,
}

fn parse_visuals_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<usize, DiagnosticReport> {
    let mut shapes = HashMap::<String, VisualShapeTable>::new();
    let mut plain_shapes = HashMap::<String, Vec<String>>::new();
    let mut color_aliases = HashMap::<String, String>::new();
    let mut colors = HashMap::<String, VisualColorTable>::new();
    let mut i = start + 1;

    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            ["colors"] => {
                i = parse_visual_colors_block(lines, i, catalog, &mut color_aliases, &mut colors)?;
            }
            ["palettes"] => {
                return Err(parse_error(
                    line,
                    "palettes block was removed; write sprite colors rows directly",
                ));
            }
            ["shapes"] => {
                i = parse_visual_shapes_block(lines, i, catalog, &mut plain_shapes, &mut shapes)?;
            }
            ["shape", table_ref] => {
                if !table_ref.contains(':') {
                    if plain_shapes.contains_key(*table_ref) {
                        return Err(parse_error(line, "duplicate visual shape"));
                    }
                    let (pattern, next_i) = parse_visual_plain_shape(lines, i)?;
                    plain_shapes.insert((*table_ref).to_string(), pattern);
                    i = next_i;
                    continue;
                }
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let (table, next_i) = parse_visual_shape_table(lines, i, &axis, None, catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::intrinsic(from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", "using", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["colors", table_ref] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if colors.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual colors"));
                }
                let (table, next_i) = parse_visual_color_table(lines, i, &axis, catalog)?;
                colors.insert(name, table);
                i = next_i;
            }
            [selector, source] if is_visual_image_source(source) => {
                add_image_visuals(selector, line, source, catalog, visuals)?;
                i += 1;
            }
            [selector, color] if is_visual_color_expr_token(color, &color_aliases, &colors) => {
                add_solid_visuals(
                    selector,
                    line,
                    color,
                    &color_aliases,
                    &colors,
                    catalog,
                    visuals,
                )?;
                i += 1;
            }
            [selector, "rotate", "from", from] => {
                let rotation = VisualShapeRotation::intrinsic(from);
                let next_i = parse_sprite_entry(
                    lines,
                    i,
                    selector,
                    Some(rotation),
                    &plain_shapes,
                    &shapes,
                    &color_aliases,
                    &colors,
                    catalog,
                    visuals,
                )?;
                i = next_i;
            }
            [selector, "rotate", "using", map, "from", from] => {
                let rotation = VisualShapeRotation::using(map, from);
                let next_i = parse_sprite_entry(
                    lines,
                    i,
                    selector,
                    Some(rotation),
                    &plain_shapes,
                    &shapes,
                    &color_aliases,
                    &colors,
                    catalog,
                    visuals,
                )?;
                i = next_i;
            }
            [selector, "rotate", map, "from", from] => {
                let rotation = VisualShapeRotation::using(map, from);
                let next_i = parse_sprite_entry(
                    lines,
                    i,
                    selector,
                    Some(rotation),
                    &plain_shapes,
                    &shapes,
                    &color_aliases,
                    &colors,
                    catalog,
                    visuals,
                )?;
                i = next_i;
            }
            [selector] => {
                let next_i = parse_sprite_entry(
                    lines,
                    i,
                    selector,
                    None,
                    &plain_shapes,
                    &shapes,
                    &color_aliases,
                    &colors,
                    catalog,
                    visuals,
                )?;
                i = next_i;
            }
            [other, ..] => {
                return Err(parse_error(
                    line,
                    &format!("unknown sprites directive {other}"),
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "sprites missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_visual_colors_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    color_aliases: &mut HashMap<String, String>,
    colors: &mut HashMap<String, VisualColorTable>,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name, "=", color] => {
                color_aliases.insert((*name).to_string(), (*color).to_string());
                i += 1;
            }
            [table_ref] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if colors.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual colors"));
                }
                let (table, next_i) = parse_visual_color_table(lines, i, &axis, catalog)?;
                colors.insert(name, table);
                i = next_i;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "colors row must be: <name> = <color> | <name>:<tag_set>",
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "colors missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_visual_shapes_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    plain_shapes: &mut HashMap<String, Vec<String>>,
    shapes: &mut HashMap<String, VisualShapeTable>,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name] if !name.contains(':') => {
                let (pattern, next_i) = parse_visual_plain_shape(lines, i)?;
                plain_shapes.insert((*name).to_string(), pattern);
                i = next_i;
            }
            [table_ref] => {
                if let Some((name, axis, value)) =
                    parse_visual_shape_value_ref(table_ref, line, catalog)?
                {
                    let (pattern, next_i) =
                        parse_visual_shape_value_pattern(lines, i, &[], false)?;
                    insert_visual_shape_value(shapes, name, axis, value, pattern, line)?;
                    i = next_i;
                } else {
                    let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                    let (table, next_i) =
                        parse_visual_shape_table(lines, i, &axis, None, catalog)?;
                    shapes.insert(name, table);
                    i = next_i;
                }
            }
            [table_ref, "rotate", "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::intrinsic(from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            [table_ref, "rotate", "using", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            [table_ref, "rotate", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "shape row must be: <name> | <name>:<tag_set>",
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "shapes missing closing brace"));
    }
    Ok(i + 1)
}

#[allow(clippy::too_many_arguments)]
fn parse_sprite_entry(
    lines: &[String],
    start: usize,
    selector: &str,
    initial_rotation: Option<VisualShapeRotation>,
    plain_shapes: &HashMap<String, Vec<String>>,
    shapes: &HashMap<String, VisualShapeTable>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<usize, DiagnosticReport> {
    let is_braced = is_block_header_line(&lines[start]);
    let mut i = start + 1;
    while i < lines.len() && lines[i].is_empty() {
        i += 1;
    }
    if i >= lines.len() || is_block_close_line(&lines[i]) {
        return Err(parse_error(&lines[start], "sprite entry missing colors"));
    }

    let mut color_exprs = None::<Vec<(char, String)>>;
    let mut offset = VisualSpriteOffset::default();
    let mut pixels_per_cell = None::<VisualSpritePixelsPerCell>;
    let mut shape_ref = None::<(String, ValueExpr)>;
    let mut inline_pattern = None::<Vec<String>>;
    let mut rotation = initial_rotation;

    while i < lines.len()
        && !is_block_close_line(&lines[i])
        && (is_braced
            || !is_unbraced_sprite_entry_boundary(lines, i, color_aliases, color_tables, catalog))
    {
        if lines[i].is_empty() {
            i += 1;
            continue;
        }
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            ["colors", colors @ ..] => {
                if colors.is_empty() {
                    return Err(parse_error(line, "sprite colors row must name colors"));
                }
                color_exprs = Some(visual_colors_from_tokens(colors, line)?);
                i += 1;
            }
            ["pixels_per_cell", width, height] => {
                pixels_per_cell = Some(VisualSpritePixelsPerCell {
                    width: parse_positive_u32(width, line, "pixels_per_cell width")?,
                    height: parse_positive_u32(height, line, "pixels_per_cell height")?,
                });
                i += 1;
            }
            ["offset", x, y] => {
                offset = VisualSpriteOffset {
                    x: parse_i32_value(x, line, "sprite offset x")?,
                    y: parse_i32_value(y, line, "sprite offset y")?,
                };
                i += 1;
            }
            [first, ..] if is_removed_translate_transform_token(first) => {
                return Err(removed_translate_transform_error(line));
            }
            ["shape", shape] => {
                shape_ref = Some(parse_sprite_shape_ref(shape, line)?);
                i += 1;
            }
            [shape] if color_exprs.is_some()
                && inline_pattern.is_none()
                && shape_ref.is_none()
                && is_known_visual_shape_ref(shape, plain_shapes, shapes) =>
            {
                shape_ref = Some(parse_sprite_shape_ref(shape, line)?);
                i += 1;
            }
            ["shape"] => {
                let (pattern, next_i) = parse_visual_rows_until_sprite_boundary(
                    lines,
                    i + 1,
                    start,
                    is_braced,
                    color_aliases,
                    color_tables,
                    catalog,
                )?;
                inline_pattern = Some(pattern);
                i = next_i;
            }
            ["rotate", ..] => {
                let Some(parsed_rotation) = parse_visual_shape_rotation_directive(line)? else {
                    return Err(parse_error(
                        line,
                        "sprite rotation must be: rotate from <value>",
                    ));
                };
                if rotation.is_some() {
                    return Err(parse_error(line, "duplicate sprite rotation"));
                }
                rotation = Some(parsed_rotation);
                i += 1;
            }
            [first, ..]
                if color_exprs.is_none()
                    && split_header_tokens(line).iter().all(|color| {
                        is_visual_color_expr_token(color, color_aliases, color_tables)
                    }) =>
            {
                let _ = first;
                color_exprs = Some(visual_colors_from_row(line)?);
                i += 1;
            }
            [_] if color_exprs.is_some() && inline_pattern.is_none() && shape_ref.is_none() => {
                match visual_pattern_row_for_palette(line, color_exprs.as_deref().unwrap()) {
                    Ok(Some(_)) => {
                        let (pattern, next_i) = parse_visual_rows_until_sprite_boundary(
                            lines,
                            i,
                            start,
                            is_braced,
                            color_aliases,
                            color_tables,
                            catalog,
                        )?;
                        inline_pattern = Some(pattern);
                        i = next_i;
                    }
                    Ok(None) => {
                        return Err(parse_error(
                            line,
                            "sprite row must be colors, pixels_per_cell, offset, shape, or rotate",
                        ));
                    }
                    Err(report) => return Err(report),
                }
            }
            _ => {
                return Err(parse_error(
                    line,
                    "sprite row must be colors, pixels_per_cell, offset, shape, or rotate",
                ));
            }
        }
    }
    if is_braced && i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "canonical sprite entry missing closing brace",
        ));
    }

    let color_exprs =
        color_exprs.ok_or_else(|| parse_error(&lines[start], "sprite entry missing colors"))?;
    let next_i = if is_braced { i + 1 } else { i };
    if let Some(rotation) = rotation {
        let Some(pattern) = inline_pattern else {
            return Err(parse_error(
                &lines[start],
                "sprite rotation requires inline ASCII rows",
            ));
        };
        validate_visual_pattern_palette(&pattern, &color_exprs, &lines[start])?;
        let targets = expand_visual_selector(selector, &lines[start], catalog)?;
        let axis = visual_rotation_axis_for_targets(&targets, catalog, &rotation, &lines[start])?;
        let mut entries = HashMap::new();
        entries.insert(rotation.from.clone(), pattern);
        let values = catalog_value_set(catalog, &axis)
            .ok_or_else(|| parse_error(&lines[start], "visual rotation tag set must exist"))?;
        expand_visual_shape_rotations(
            &mut entries,
            values,
            catalog,
            &axis,
            &rotation,
            &lines[start],
        )?;
        let shape = VisualShapeTable { axis, entries };
        add_ascii_visuals(
            selector,
            &lines[start],
            &shape,
            &ValueExpr::Binding(shape.axis.clone()),
            &color_exprs,
            offset,
            pixels_per_cell,
            color_aliases,
            color_tables,
            catalog,
            visuals,
        )?;
    } else if let Some((shape_name, shape_value)) = shape_ref {
        if let Some(shape) = shapes.get(&shape_name) {
            add_ascii_visuals(
                selector,
                &lines[start],
                shape,
                &shape_value,
                &color_exprs,
                offset,
                pixels_per_cell,
                color_aliases,
                color_tables,
                catalog,
                visuals,
            )?;
        } else {
            let pattern = plain_shapes
                .get(&shape_name)
                .ok_or_else(|| parse_error(&lines[start], "unknown sprite shape"))?;
            add_inline_ascii_visuals(
                selector,
                &lines[start],
                pattern,
                &color_exprs,
                offset,
                pixels_per_cell,
                color_aliases,
                color_tables,
                catalog,
                visuals,
            )?;
        }
    } else if let Some(pattern) = inline_pattern {
        add_inline_ascii_visuals(
            selector,
            &lines[start],
            &pattern,
            &color_exprs,
            offset,
            pixels_per_cell,
            color_aliases,
            color_tables,
            catalog,
            visuals,
        )?;
    } else {
        let [(_, color)] = color_exprs.as_slice() else {
            return Err(parse_error(
                &lines[start],
                "solid sprite requires exactly one color",
            ));
        };
        add_solid_visuals(
            selector,
            &lines[start],
            color,
            color_aliases,
            color_tables,
            catalog,
            visuals,
        )?;
    }

    Ok(next_i)
}

fn is_unbraced_sprite_entry_boundary(
    lines: &[String],
    index: usize,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    _catalog: &Catalog,
) -> bool {
    let tokens = split_header_tokens(&lines[index]);
    match tokens.as_slice() {
        ["colors"] | ["shapes"] if is_block_header_line(&lines[index]) => true,
        ["colors", ..]
        | ["shape", ..]
        | ["offset", ..]
        | ["pixels_per_cell", ..]
        | ["rotate", ..] => false,
        [selector]
            if is_block_header_line(&lines[index])
                && !matches!(
                    *selector,
                    "colors" | "offset" | "pixels_per_cell" | "rotate" | "shape"
                ) =>
        {
            true
        }
        [selector, source]
            if is_visual_image_source(source)
                || (is_visual_color_expr_token(source, color_aliases, color_tables)
                    && !is_visual_color_expr_token(selector, color_aliases, color_tables)) =>
        {
            true
        }
        [_selector] => is_unbraced_sprite_entry_header(lines, index, color_aliases, color_tables),
        _ => false,
    }
}

fn visual_colors_from_tokens(
    tokens: &[&str],
    line: &str,
) -> Result<Vec<(char, String)>, DiagnosticReport> {
    tokens
        .iter()
        .enumerate()
        .map(|(index, color)| {
            let token = visual_color_token_for_index(index)
                .ok_or_else(|| parse_error(line, "sprite supports at most 62 colors"))?;
            Ok((token, (*color).to_string()))
        })
        .collect()
}

fn parse_positive_u32(value: &str, line: &str, label: &str) -> Result<u32, DiagnosticReport> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| parse_error(line, &format!("{label} must be a positive integer")))?;
    if parsed == 0 {
        return Err(parse_error(
            line,
            &format!("{label} must be a positive integer"),
        ));
    }
    Ok(parsed)
}

fn parse_i32_value(value: &str, line: &str, label: &str) -> Result<i32, DiagnosticReport> {
    value
        .parse::<i32>()
        .map_err(|_| parse_error(line, &format!("{label} must be an integer")))
}

fn parse_visual_rows_until_sprite_boundary(
    lines: &[String],
    mut i: usize,
    start: usize,
    is_braced: bool,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let mut pattern = Vec::new();
    while i < lines.len()
        && !is_block_close_line(&lines[i])
        && (is_braced
            || !is_unbraced_sprite_entry_boundary(lines, i, color_aliases, color_tables, catalog))
    {
        if lines[i].is_empty() {
            i += 1;
            continue;
        }
        if is_removed_translate_transform_row(&lines[i]) {
            return Err(removed_translate_transform_error(&lines[i]));
        }
        let row_tokens = split_header_tokens(&lines[i]);
        let [row] = row_tokens.as_slice() else {
            return Err(parse_error(
                &lines[i],
                "visual shape row must be a single token row",
            ));
        };
        pattern.push((*row).to_string());
        i += 1;
    }
    if is_braced && i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "visual shape rows missing closing brace",
        ));
    }
    validate_visual_pattern(&pattern, &lines[start])?;
    Ok((pattern, i))
}

fn visual_rotation_axis_for_targets(
    targets: &[VisualSelectorTarget],
    catalog: &Catalog,
    rotation: &VisualShapeRotation,
    line: &str,
) -> Result<String, DiagnosticReport> {
    let first = targets
        .first()
        .ok_or_else(|| parse_error(line, "visual selector matched no objects"))?;
    let mut candidates = first
        .bindings
        .keys()
        .filter(|axis| {
            catalog_value_set(catalog, axis)
                .is_some_and(|values| values.iter().any(|value| value == &rotation.from))
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.retain(|axis| {
        targets
            .iter()
            .all(|target| target.bindings.contains_key(axis))
    });
    let [axis] = candidates.as_slice() else {
        return Err(parse_error(
            line,
            "sprite rotation requires exactly one matching selector tag set",
        ));
    };
    Ok(axis.clone())
}

fn visual_table_key<T>(
    expr: &ValueExpr,
    axis: &str,
    entries: &HashMap<String, T>,
    bindings: &HashMap<String, String>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    if let ValueExpr::Binding(name) = expr {
        if let Some(value) = bindings.get(name) {
            return Ok(value.clone());
        }
        if name == axis
            && let Some(value) = bindings.get(axis)
        {
            return Ok(value.clone());
        }
        if entries.contains_key(name) {
            return Ok(name.clone());
        }
    }
    let env = visual_value_env(bindings);
    if value_expr_result_axis(expr, &env, maps, line)? != axis {
        return Err(parse_error(line, "visual table tag set mismatch"));
    }
    eval_bound_value_expr(expr, &env, maps, line)
}

fn parse_visual_plain_shape(
    lines: &[String],
    start: usize,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let is_braced = is_block_header_line(&lines[start]);
    let mut pattern = Vec::new();
    let mut i = start + 1;
    let mut width = None::<usize>;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if lines[i].is_empty() {
            if is_braced {
                i += 1;
                continue;
            }
            break;
        }
        let row_tokens = split_header_tokens(&lines[i]);
        let [row] = row_tokens.as_slice() else {
            return Err(parse_error(
                &lines[i],
                "visual shape row must be a single token row",
            ));
        };
        let row_width = row.chars().count();
        if !is_braced
            && let Some(expected_width) = width
            && row_width != expected_width
        {
            return Err(parse_error(
                &lines[i],
                "visual shape rows must be equal-width ascii",
            ));
        }
        width = Some(row_width);
        pattern.push((*row).to_string());
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "visual shape missing closing brace",
        ));
    }
    validate_visual_pattern(&pattern, &lines[start])?;
    let next_i = if is_braced { i + 1 } else { i };
    Ok((pattern, next_i))
}

fn parse_sprite_shape_ref(
    shape_ref: &str,
    line: &str,
) -> Result<(String, ValueExpr), DiagnosticReport> {
    let (shape_name, shape_value) = if shape_ref.contains(':') {
        parse_visual_table_expr(shape_ref, line)?
    } else {
        (shape_ref.to_string(), ValueExpr::Binding(String::new()))
    };
    Ok((shape_name, shape_value))
}

fn is_known_visual_shape_ref(
    value: &str,
    plain_shapes: &HashMap<String, Vec<String>>,
    shapes: &HashMap<String, VisualShapeTable>,
) -> bool {
    if plain_shapes.contains_key(value) || shapes.contains_key(value) {
        return true;
    }
    parse_visual_table_expr(value, value)
        .ok()
        .is_some_and(|(name, _)| shapes.contains_key(&name))
}

fn visual_pattern_row_for_palette(
    line: &str,
    color_exprs: &[(char, String)],
) -> Result<Option<String>, DiagnosticReport> {
    let row_tokens = split_header_tokens(line);
    let [row] = row_tokens.as_slice() else {
        return Err(parse_error(
            line,
            "sprite pattern row must be a single token row",
        ));
    };
    if row.contains(['{', '}']) {
        return Err(parse_error(line, "ASCII rows cannot contain braces"));
    }
    let colors = color_exprs
        .iter()
        .map(|(token, _)| *token)
        .collect::<HashSet<_>>();
    if row
        .chars()
        .all(|token| token == '.' || colors.contains(&token))
    {
        Ok(Some((*row).to_string()))
    } else if row
        .chars()
        .all(|token| token == '.' || token.is_ascii_alphanumeric())
    {
        Err(parse_error(
            line,
            "sprite pattern references a color outside the color row",
        ))
    } else {
        Ok(None)
    }
}

fn is_removed_translate_transform_row(line: &str) -> bool {
    split_header_tokens(line)
        .first()
        .is_some_and(|token| is_removed_translate_transform_token(token))
}

fn is_removed_translate_transform_token(token: &str) -> bool {
    token.to_ascii_lowercase().starts_with("translate:")
}

fn removed_translate_transform_error(line: &str) -> DiagnosticReport {
    parse_error(
        line,
        "translate sprite transforms were removed; use `offset <x> <y>`",
    )
}

fn visual_colors_from_row(line: &str) -> Result<Vec<(char, String)>, DiagnosticReport> {
    split_header_tokens(line)
        .iter()
        .enumerate()
        .map(|(index, color)| {
            let token = visual_color_token_for_index(index)
                .ok_or_else(|| parse_error(line, "sprite supports at most 62 colors"))?;
            Ok((token, (*color).to_string()))
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()
}

fn validate_visual_pattern_palette(
    pattern: &[String],
    color_exprs: &[(char, String)],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let colors = color_exprs
        .iter()
        .map(|(token, _)| *token)
        .collect::<HashSet<_>>();
    for row in pattern {
        for token in row.chars() {
            if token == '.' || colors.contains(&token) {
                continue;
            }
            return Err(parse_error(
                line,
                "sprite pattern references a color outside the color row",
            ));
        }
    }
    Ok(())
}

pub(crate) fn is_visual_color_token(value: &str) -> bool {
    value.starts_with('#') || crate::syntax::is_visual_named_color(value)
}

fn is_visual_color_expr_token(
    value: &str,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
) -> bool {
    if is_visual_color_token(value) || color_aliases.contains_key(value) {
        return true;
    }
    parse_visual_table_expr(value, value)
        .ok()
        .is_some_and(|(name, _)| color_tables.contains_key(&name))
}

fn is_visual_image_source(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".svg")
        || lower.ends_with(".avif")
}

fn is_unbraced_sprite_entry_header(
    lines: &[String],
    index: usize,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
) -> bool {
    let tokens = split_header_tokens(&lines[index]);
    let [selector] = tokens.as_slice() else {
        return false;
    };
    if selector.starts_with("translate:") {
        return false;
    }
    next_line_starts_sprite_entry_body(lines, index, color_aliases, color_tables)
}

fn next_line_starts_sprite_entry_body(
    lines: &[String],
    index: usize,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
) -> bool {
    let Some(next) = lines.get(index + 1) else {
        return false;
    };
    if is_block_close_line(next) {
        return false;
    }
    let next_tokens = split_header_tokens(next);
    match next_tokens.as_slice() {
        ["colors", colors @ ..] => {
            !colors.is_empty()
                && colors
                    .iter()
                    .all(|color| is_visual_color_expr_token(color, color_aliases, color_tables))
        }
        ["pixels_per_cell" | "offset" | "rotate" | "shape", ..] => true,
        [color, ..] if is_visual_color_expr_token(color, color_aliases, color_tables) => true,
        _ => false,
    }
}

pub(crate) fn visual_color_token_for_index(index: usize) -> Option<char> {
    const TOKENS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    TOKENS.get(index).map(|token| *token as char)
}

fn parse_visual_table_ref(value: &str, line: &str) -> Result<(String, String), DiagnosticReport> {
    let Some((name, axis)) = value.split_once(':') else {
        return Err(parse_error(line, "visual table must be: <name>:<tag_set>"));
    };
    if !is_identifier(name) {
        return Err(parse_error(line, "visual table name must be an identifier"));
    }
    if !is_identifier(axis) {
        return Err(parse_error(
            line,
            "visual table tag set must be an identifier",
        ));
    }
    Ok((name.to_string(), axis.to_string()))
}

fn parse_visual_table_expr(
    value: &str,
    line: &str,
) -> Result<(String, ValueExpr), DiagnosticReport> {
    let Some((name, value)) = value.split_once(':') else {
        return Err(parse_error(
            line,
            "visual table must be: <name>:<value-expr>",
        ));
    };
    if !is_identifier(name) {
        return Err(parse_error(line, "visual table name must be an identifier"));
    }
    Ok((name.to_string(), parse_value_expr(value, line)?))
}

fn parse_visual_shape_value_ref(
    value: &str,
    line: &str,
    catalog: &Catalog,
) -> Result<Option<(String, String, String)>, DiagnosticReport> {
    let Some((name, value)) = value.split_once(':') else {
        return Ok(None);
    };
    if !is_identifier(name) {
        return Err(parse_error(line, "visual table name must be an identifier"));
    }
    if catalog_value_set(catalog, value).is_some() {
        return Ok(None);
    }
    if !is_identifier(value) {
        return Ok(None);
    }
    let axis = infer_visual_shape_value_axis(value, line, catalog)?;
    Ok(Some((name.to_string(), axis, value.to_string())))
}

fn infer_visual_shape_value_axis(
    value: &str,
    line: &str,
    catalog: &Catalog,
) -> Result<String, DiagnosticReport> {
    let axes = catalog_value_sets(catalog)
        .into_iter()
        .filter_map(|(axis, values)| values.iter().any(|candidate| candidate == value).then_some(axis))
        .collect::<Vec<_>>();
    let [axis] = axes.as_slice() else {
        return Err(parse_error(
            line,
            "visual shape value must belong to exactly one tag set",
        ));
    };
    Ok(axis.clone())
}

fn insert_visual_shape_value(
    shapes: &mut HashMap<String, VisualShapeTable>,
    name: String,
    axis: String,
    value: String,
    pattern: Vec<String>,
    line: &str,
) -> Result<(), DiagnosticReport> {
    let table = shapes.entry(name).or_insert_with(|| VisualShapeTable {
        axis: axis.clone(),
        entries: HashMap::new(),
    });
    if table.axis != axis {
        return Err(parse_error(line, "visual shape tag set mismatch"));
    }
    if table.entries.insert(value, pattern).is_some() {
        return Err(parse_error(line, "duplicate visual shape value"));
    }
    Ok(())
}

fn parse_visual_shape_value_pattern(
    lines: &[String],
    start: usize,
    table_values: &[String],
    stop_on_table_value: bool,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let is_braced = is_block_header_line(&lines[start]);
    let mut pattern = Vec::new();
    let mut i = start + 1;
    while i < lines.len() {
        if is_block_close_line(&lines[i]) {
            if is_braced {
                validate_visual_pattern(&pattern, &lines[start])?;
                return Ok((pattern, i + 1));
            }
            break;
        }
        if !is_braced {
            if lines[i].is_empty() {
                if pattern.is_empty() {
                    i += 1;
                    continue;
                }
                break;
            }
            if !pattern.is_empty()
                && stop_on_table_value
                && is_visual_shape_table_value_header(&lines[i], table_values)
            {
                break;
            }
            if !pattern.is_empty() && is_visual_shape_individual_value_header(&lines[i]) {
                break;
            }
        }
        let row_tokens = split_header_tokens(&lines[i]);
        let [row] = row_tokens.as_slice() else {
            return Err(parse_error(
                &lines[i],
                "visual shape row must be a single token row",
            ));
        };
        pattern.push((*row).to_string());
        i += 1;
    }
    if is_braced {
        return Err(parse_error(
            &lines[start],
            "visual shape value missing closing brace",
        ));
    }
    validate_visual_pattern(&pattern, &lines[start])?;
    Ok((pattern, i))
}

fn is_visual_shape_table_value_header(line: &str, values: &[String]) -> bool {
    let tokens = split_header_tokens(line);
    let [value] = tokens.as_slice() else {
        return false;
    };
    values.iter().any(|candidate| candidate == value)
}

fn is_visual_shape_individual_value_header(line: &str) -> bool {
    let tokens = split_header_tokens(line);
    let [value] = tokens.as_slice() else {
        return false;
    };
    value
        .split_once(':')
        .is_some_and(|(name, value)| is_identifier(name) && is_identifier(value))
}

fn parse_visual_shape_table(
    lines: &[String],
    start: usize,
    axis: &str,
    rotation: Option<VisualShapeRotation>,
    catalog: &Catalog,
) -> Result<(VisualShapeTable, usize), DiagnosticReport> {
    let values = catalog_value_set(catalog, axis).ok_or_else(|| {
        parse_error(
            &lines[start],
            "visual shape tag set must name an existing tag set",
        )
    })?;
    let mut entries = HashMap::new();
    let mut i = start + 1;
    if let Some(rotation) = rotation {
        let mut pattern = Vec::new();
        while i < lines.len() && !is_block_close_line(&lines[i]) {
            let row_tokens = split_header_tokens(&lines[i]);
            let [row] = row_tokens.as_slice() else {
                return Err(parse_error(
                    &lines[i],
                    "visual shape row must be a single token row",
                ));
            };
            pattern.push((*row).to_string());
            i += 1;
        }
        if i >= lines.len() {
            return Err(parse_error(
                &lines[start],
                "visual shape missing closing brace",
            ));
        }
        validate_visual_pattern(&pattern, &lines[i])?;
        entries.insert(rotation.from.clone(), pattern);
        expand_visual_shape_rotations(
            &mut entries,
            values,
            catalog,
            axis,
            &rotation,
            &lines[start],
        )?;
        return Ok((
            VisualShapeTable {
                axis: axis.to_string(),
                entries,
            },
            i + 1,
        ));
    }
    let mut rotation = None::<VisualShapeRotation>;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if lines[i].is_empty() {
            i += 1;
            continue;
        }
        if let Some(parsed_rotation) = parse_visual_shape_rotation_directive(&lines[i])? {
            if rotation.is_some() {
                return Err(parse_error(&lines[i], "duplicate visual shape rotation"));
            }
            if lines[i].trim_end().ends_with('{') {
                let mut pattern = Vec::new();
                i += 1;
                while i < lines.len() && !is_block_close_line(&lines[i]) {
                    let row_tokens = split_header_tokens(&lines[i]);
                    let [row] = row_tokens.as_slice() else {
                        return Err(parse_error(
                            &lines[i],
                            "visual shape row must be a single token row",
                        ));
                    };
                    pattern.push((*row).to_string());
                    i += 1;
                }
                if i >= lines.len() {
                    return Err(parse_error(
                        &lines[start],
                        "visual shape rotation missing closing brace",
                    ));
                }
                validate_visual_pattern(&pattern, &lines[i])?;
                if entries
                    .insert(parsed_rotation.from.clone(), pattern)
                    .is_some()
                {
                    return Err(parse_error(
                        &lines[i],
                        "visual shape rotation source duplicates explicit shape value",
                    ));
                }
                rotation = Some(parsed_rotation);
                i += 1;
                continue;
            }
            if !entries.contains_key(&parsed_rotation.from) {
                let mut pattern = Vec::new();
                i += 1;
                while i < lines.len() && !is_block_close_line(&lines[i]) {
                    let row_tokens = split_header_tokens(&lines[i]);
                    let [row] = row_tokens.as_slice() else {
                        return Err(parse_error(
                            &lines[i],
                            "visual shape row must be a single token row",
                        ));
                    };
                    pattern.push((*row).to_string());
                    i += 1;
                }
                if i >= lines.len() {
                    return Err(parse_error(
                        &lines[start],
                        "visual shape rotation missing closing brace",
                    ));
                }
                validate_visual_pattern(&pattern, &lines[i])?;
                entries.insert(parsed_rotation.from.clone(), pattern);
                rotation = Some(parsed_rotation);
                continue;
            }
            rotation = Some(parsed_rotation);
            i += 1;
            continue;
        }
        let value = block_header_text(&lines[i]);
        if !values.iter().any(|candidate| candidate == value) {
            return Err(parse_error(
                &lines[i],
                "visual shape value is not in tag set",
            ));
        }
        let (pattern, next_i) = parse_visual_shape_value_pattern(lines, i, values, true)?;
        if entries.insert(value.to_string(), pattern).is_some() {
            return Err(parse_error(&lines[i], "duplicate visual shape value"));
        }
        i = next_i;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "visual shape missing closing brace",
        ));
    }
    if let Some(rotation) = rotation {
        expand_visual_shape_rotations(
            &mut entries,
            values,
            catalog,
            axis,
            &rotation,
            &lines[start],
        )?;
    }
    Ok((
        VisualShapeTable {
            axis: axis.to_string(),
            entries,
        },
        i + 1,
    ))
}

fn parse_visual_shape_rotation_directive(
    line: &str,
) -> Result<Option<VisualShapeRotation>, DiagnosticReport> {
    let tokens = split_header_tokens(block_header_text(line));
    match tokens.as_slice() {
        ["rotate", "from", from] => Ok(Some(VisualShapeRotation::intrinsic(from))),
        ["rotate", "using", map, "from", from] => Ok(Some(VisualShapeRotation::using(map, from))),
        ["rotate", map, "from", from] => Ok(Some(VisualShapeRotation::using(map, from))),
        ["rotate", ..] => Err(parse_error(
            line,
            "visual shape rotation must be: rotate from <value> | rotate using <map> from <value>",
        )),
        _ => Ok(None),
    }
}

fn validate_visual_pattern(pattern: &[String], line: &str) -> Result<(), DiagnosticReport> {
    if pattern.is_empty() {
        return Err(parse_error(
            line,
            "visual shape value requires at least one row",
        ));
    }
    let width = pattern[0].chars().count();
    if width == 0
        || pattern
            .iter()
            .any(|row| row.chars().count() != width || !row.is_ascii())
    {
        return Err(parse_error(
            line,
            "visual shape rows must be equal-width ascii",
        ));
    }
    if pattern.iter().any(|row| row.contains(['{', '}'])) {
        return Err(parse_error(line, "ASCII rows cannot contain braces"));
    }
    Ok(())
}

fn expand_visual_shape_rotations(
    entries: &mut HashMap<String, Vec<String>>,
    values: &[String],
    catalog: &Catalog,
    axis: &str,
    rotation: &VisualShapeRotation,
    line: &str,
) -> Result<(), DiagnosticReport> {
    if !values.iter().any(|value| value == &rotation.from) {
        return Err(parse_error(
            line,
            "visual rotation source is not in tag set",
        ));
    }
    let rotation_values = visual_rotation_values(values, catalog, axis, rotation, line)?;
    let mut value = rotation.from.clone();
    let mut pattern = entries
        .get(&value)
        .cloned()
        .ok_or_else(|| parse_error(line, "visual rotation source shape missing"))?;
    let mut visited = Vec::new();

    loop {
        if visited.iter().any(|seen| seen == &value) {
            break;
        }
        visited.push(value.clone());
        let next = rotation_values
            .get(&value)
            .ok_or_else(|| parse_error(line, "visual rotation map value missing"))?
            .clone();
        let next_pattern = rotate_visual_pattern_clockwise(&pattern);
        if next == rotation.from {
            break;
        }
        if let Some(existing) = entries.get(&next) {
            if existing != &next_pattern {
                return Err(parse_error(
                    line,
                    "visual rotation conflicts with explicit shape value",
                ));
            }
        } else {
            entries.insert(next.clone(), next_pattern.clone());
        }
        value = next;
        pattern = next_pattern;
    }

    if visited.len() != values.len() || values.iter().any(|value| !entries.contains_key(value)) {
        return Err(parse_error(
            line,
            "visual rotation map must cycle through every shape tag value",
        ));
    }
    Ok(())
}

fn visual_rotation_values(
    values: &[String],
    catalog: &Catalog,
    axis: &str,
    rotation: &VisualShapeRotation,
    line: &str,
) -> Result<HashMap<String, String>, DiagnosticReport> {
    if let Some(map_name) = &rotation.map {
        let map = catalog
            .maps
            .get(map_name)
            .ok_or_else(|| parse_error(line, "unknown visual rotation map"))?;
        if map.axis != axis {
            return Err(parse_error(line, "visual rotation map tag set mismatch"));
        }
        return Ok(map.values.clone());
    }

    intrinsic_cardinal_visual_rotation_values(values, line)
}

fn intrinsic_cardinal_visual_rotation_values(
    values: &[String],
    line: &str,
) -> Result<HashMap<String, String>, DiagnosticReport> {
    const CARDINAL_ROTATION: [(&str, &str); 4] = [
        ("up", "right"),
        ("right", "down"),
        ("down", "left"),
        ("left", "up"),
    ];
    if values.len() != CARDINAL_ROTATION.len()
        || !CARDINAL_ROTATION
            .iter()
            .all(|(value, _)| values.iter().any(|candidate| candidate == value))
    {
        return Err(parse_error(
            line,
            "visual rotation without a map requires tag values up, right, down, left",
        ));
    }
    Ok(CARDINAL_ROTATION
        .into_iter()
        .map(|(from, to)| (from.to_string(), to.to_string()))
        .collect())
}

fn rotate_visual_pattern_clockwise(pattern: &[String]) -> Vec<String> {
    let rows = pattern
        .iter()
        .map(|row| row.chars().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let height = rows.len();
    let width = rows.first().map_or(0, Vec::len);
    let mut rotated = Vec::with_capacity(width);
    for x in 0..width {
        let mut row = String::with_capacity(height);
        for y in (0..height).rev() {
            row.push(rows[y][x]);
        }
        rotated.push(row);
    }
    rotated
}

fn parse_visual_color_table(
    lines: &[String],
    start: usize,
    axis: &str,
    catalog: &Catalog,
) -> Result<(VisualColorTable, usize), DiagnosticReport> {
    let values = catalog_value_set(catalog, axis).ok_or_else(|| {
        parse_error(
            &lines[start],
            "visual colors tag set must name an existing tag set",
        )
    })?;
    let mut entries = HashMap::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        let [value, "=", color] = tokens.as_slice() else {
            return Err(parse_error(
                &lines[i],
                "visual color row must be: <value> = <color>",
            ));
        };
        if !values.iter().any(|candidate| candidate == value) {
            return Err(parse_error(
                &lines[i],
                "visual color value is not in tag set",
            ));
        }
        if entries
            .insert((*value).to_string(), (*color).to_string())
            .is_some()
        {
            return Err(parse_error(&lines[i], "duplicate visual color value"));
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "visual colors missing closing brace",
        ));
    }
    Ok((
        VisualColorTable {
            axis: axis.to_string(),
            entries,
        },
        i + 1,
    ))
}

fn add_ascii_visuals(
    selector: &str,
    line: &str,
    shape: &VisualShapeTable,
    shape_value_expr: &ValueExpr,
    color_exprs: &[(char, String)],
    offset: VisualSpriteOffset,
    pixels_per_cell: Option<VisualSpritePixelsPerCell>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let env = visual_value_env(&target.bindings);
        if value_expr_result_axis(shape_value_expr, &env, &catalog.maps, line)? != shape.axis {
            return Err(parse_error(line, "visual shape tag set mismatch"));
        }
        let shape_value = eval_bound_value_expr(shape_value_expr, &env, &catalog.maps, line)?;
        if !catalog_value_set(catalog, &shape.axis)
            .is_some_and(|values| values.iter().any(|value| value == &shape_value))
        {
            return Err(parse_error(line, "visual shape value is not in tag set"));
        }
        let pattern = shape
            .entries
            .get(&shape_value)
            .ok_or_else(|| parse_error(line, "visual shape value missing"))?
            .clone();
        validate_visual_pattern_palette(&pattern, color_exprs, line)?;
        let colors = color_exprs
            .iter()
            .map(|(token, expr)| {
                Ok(VisualColorDef {
                    token: *token,
                    color: resolve_visual_color_expr(
                        expr,
                        &target.bindings,
                        color_aliases,
                        color_tables,
                        &catalog.maps,
                        line,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, DiagnosticReport>>()?;
        let sprite = sprite_name_for_object(&target.object_name);
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset,
            pixels_per_cell,
            kind: VisualSpriteKind::Ascii { pattern, colors },
        });
    }
    Ok(())
}

fn add_inline_ascii_visuals(
    selector: &str,
    line: &str,
    pattern: &[String],
    color_exprs: &[(char, String)],
    offset: VisualSpriteOffset,
    pixels_per_cell: Option<VisualSpritePixelsPerCell>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    validate_visual_pattern_palette(pattern, color_exprs, line)?;
    for target in expand_visual_selector(selector, line, catalog)? {
        let colors = color_exprs
            .iter()
            .map(|(token, expr)| {
                Ok(VisualColorDef {
                    token: *token,
                    color: resolve_visual_color_expr(
                        expr,
                        &target.bindings,
                        color_aliases,
                        color_tables,
                        &catalog.maps,
                        line,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, DiagnosticReport>>()?;
        let sprite = sprite_name_for_object(&target.object_name);
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset,
            pixels_per_cell,
            kind: VisualSpriteKind::Ascii {
                pattern: pattern.to_vec(),
                colors,
            },
        });
    }
    Ok(())
}

fn add_solid_visuals(
    selector: &str,
    line: &str,
    color_expr: &str,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let sprite = sprite_name_for_object(&target.object_name);
        let color = resolve_visual_color_expr(
            color_expr,
            &target.bindings,
            color_aliases,
            color_tables,
            &catalog.maps,
            line,
        )?;
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset: VisualSpriteOffset::default(),
            pixels_per_cell: None,
            kind: VisualSpriteKind::Solid(color),
        });
    }
    Ok(())
}

fn add_image_visuals(
    selector: &str,
    line: &str,
    source: &str,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let sprite = sprite_name_for_object(&target.object_name);
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset: VisualSpriteOffset::default(),
            pixels_per_cell: None,
            kind: VisualSpriteKind::Image {
                source: source.to_string(),
            },
        });
    }
    Ok(())
}

fn resolve_visual_color_expr(
    expr: &str,
    bindings: &HashMap<String, String>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    resolve_visual_color_expr_with_aliases(expr, bindings, color_aliases, color_tables, maps, line)
}

fn resolve_visual_color_expr_with_aliases(
    expr: &str,
    bindings: &HashMap<String, String>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    if let Some(color) = color_aliases.get(expr) {
        return Ok(color.clone());
    }
    if let Some((name, value_expr)) = parse_visual_table_expr(expr, line).ok() {
        let table = color_tables
            .get(&name)
            .ok_or_else(|| parse_error(line, "unknown visual colors"))?;
        let value = visual_table_key(
            &value_expr,
            &table.axis,
            &table.entries,
            bindings,
            maps,
            line,
        )?;
        return table
            .entries
            .get(&value)
            .cloned()
            .ok_or_else(|| parse_error(line, "visual color value missing"));
    }
    Ok(expr.to_string())
}

#[derive(Clone, Debug)]
struct VisualSelectorTarget {
    object_name: String,
    bindings: HashMap<String, String>,
}

fn expand_visual_selector(
    selector: &str,
    line: &str,
    catalog: &Catalog,
) -> Result<Vec<VisualSelectorTarget>, DiagnosticReport> {
    if !selector.contains(':')
        && let Some(object) = catalog.object_names.get(selector).copied()
    {
        let name = catalog
            .object_labels
            .get(&object)
            .cloned()
            .unwrap_or_else(|| selector.to_string());
        return Ok(vec![VisualSelectorTarget {
            object_name: name,
            bindings: HashMap::new(),
        }]);
    }
    if let Some(objects) = catalog.object_groups.get(selector) {
        return Ok(objects
            .iter()
            .filter_map(|object| catalog.object_labels.get(object).cloned())
            .map(|object_name| VisualSelectorTarget {
                object_name,
                bindings: HashMap::new(),
            })
            .collect());
    }

    let parts = selector.split(':').collect::<Vec<_>>();
    let Some(schema) = catalog.object_schemas.get(parts[0]) else {
        return Err(parse_error(line, "unknown visual object selector"));
    };
    if parts.len() - 1 > schema.axes.len() {
        return Err(parse_error(
            line,
            "visual object selector has too many tags",
        ));
    }

    let constraints = visual_selector_constraints(&parts, schema, catalog, line)?;
    let assignments = visual_selector_assignments(schema, &constraints, &catalog.maps, line)?;
    let mut targets = Vec::new();
    for (target_values, bindings) in assignments {
        let variant = schema
            .variants
            .iter()
            .find(|variant| variant.values == target_values)
            .ok_or_else(|| parse_error(line, "visual object selector target not found"))?;
        let object_name = catalog
            .object_labels
            .get(&variant.object)
            .cloned()
            .ok_or_else(|| parse_error(line, "visual object label missing"))?;
        if targets
            .iter()
            .any(|target: &VisualSelectorTarget| target.object_name == object_name)
        {
            return Err(parse_error(
                line,
                "visual object selector maps multiple bindings to one object",
            ));
        }
        targets.push(VisualSelectorTarget {
            object_name,
            bindings,
        });
    }
    if targets.is_empty() {
        return Err(parse_error(
            line,
            "visual object selector matched no objects",
        ));
    }
    Ok(targets)
}

fn visual_selector_constraints(
    parts: &[&str],
    schema: &ObjectSchema,
    catalog: &Catalog,
    line: &str,
) -> Result<Vec<VisualSelectorConstraint>, DiagnosticReport> {
    let value_sets = catalog_value_sets(catalog);
    schema
        .axes
        .iter()
        .enumerate()
        .map(|(index, axis)| {
            let Some(part) = parts.get(index + 1).copied() else {
                return Ok(VisualSelectorConstraint::Any);
            };
            let expr = parse_value_expr(part, line)?;
            if expr == ValueExpr::Binding(axis.clone()) {
                return Ok(VisualSelectorConstraint::Any);
            }
            if let ValueExpr::MapCall { arg, .. } = &expr {
                if arg != axis {
                    return Err(parse_error(
                        line,
                        "map argument must match selector tag set",
                    ));
                }
                let ValueExpr::MapCall { name, .. } = &expr else {
                    unreachable!("map call branch only handles map calls");
                };
                let map = catalog
                    .maps
                    .get(name)
                    .ok_or_else(|| parse_error(line, "unknown map"))?;
                if map.axis != *axis {
                    return Err(parse_error(line, "map tag set must match argument tag set"));
                }
                return Ok(VisualSelectorConstraint::Mapped(expr));
            }
            let ValueExpr::Binding(name) = expr else {
                unreachable!("value expr is either binding or map call");
            };
            let axis_values = schema_axis_values(schema, index)?;
            if axis_values.contains(&name) && value_sets.contains_key(&name) {
                Err(ambiguous_selector_tag_error(&name, parts[0], axis, line))
            } else if let Some(values) = value_sets.get(&name) {
                validate_selector_subset(&name, values, &axis_values, parts[0], axis, line)?;
                Ok(VisualSelectorConstraint::ValueSet(values.clone()))
            } else if axis_values.contains(&name) {
                Ok(VisualSelectorConstraint::Fixed(name))
            } else {
                Ok(VisualSelectorConstraint::Fixed(name))
            }
        })
        .collect()
}

fn visual_selector_assignments(
    schema: &ObjectSchema,
    constraints: &[VisualSelectorConstraint],
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<Vec<(Vec<String>, HashMap<String, String>)>, DiagnosticReport> {
    let mut assignments = vec![(Vec::<String>::new(), HashMap::<String, String>::new())];
    for (index, axis) in schema.axes.iter().enumerate() {
        let axis_values = schema_axis_values(schema, index)?;
        let values = match &constraints[index] {
            VisualSelectorConstraint::Any | VisualSelectorConstraint::Mapped(_) => axis_values,
            VisualSelectorConstraint::Fixed(value) => vec![value.clone()],
            VisualSelectorConstraint::ValueSet(values) => values.clone(),
        };
        let mut next = Vec::new();
        for (target_prefix, bindings) in &assignments {
            for value in &values {
                let mut env = visual_value_env(bindings);
                env.bind(axis, axis, value);
                let target_value = match &constraints[index] {
                    VisualSelectorConstraint::Mapped(expr) => {
                        eval_bound_value_expr(expr, &env, maps, line)?
                    }
                    _ => value.clone(),
                };
                if !schema_axis_values(schema, index)?.contains(&target_value) {
                    return Err(parse_error(
                        line,
                        "visual object selector target value is not in tag slot",
                    ));
                }
                let mut target_values = target_prefix.clone();
                target_values.push(target_value);
                let mut next_bindings = bindings.clone();
                next_bindings.insert(axis.clone(), value.clone());
                next.push((target_values, next_bindings));
            }
        }
        assignments = next;
    }
    Ok(assignments)
}

#[derive(Clone, Debug)]
enum VisualSelectorConstraint {
    Any,
    Fixed(String),
    ValueSet(Vec<String>),
    Mapped(ValueExpr),
}

fn sprite_name_for_object(object_name: &str) -> String {
    let mut sprite = String::new();
    for ch in object_name.chars() {
        if ch.is_ascii_alphanumeric() {
            sprite.push(ch);
        } else if !sprite.ends_with('-') {
            sprite.push('-');
        }
    }
    let sprite = sprite.trim_matches('-').to_string();
    if sprite.is_empty() {
        "unknown".to_string()
    } else {
        sprite
    }
}

fn parse_group_directive(
    tokens: &[&str],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    visual_objects: &[ObjectId],
    object_groups: &mut HashMap<String, Vec<ObjectId>>,
) -> Result<(), DiagnosticReport> {
    if tokens.len() < 4 || tokens.get(2).copied() != Some("=") {
        return Err(parse_error(
            line,
            "group must be: group <name> = <selector...>",
        ));
    }

    let name = tokens[1];
    validate_selector_alias_name(name, line, "group name")?;
    if selector_name_conflicts_with(name, object_names, object_schemas, object_groups) {
        return Err(parse_error(
            line,
            "group name must not shadow another selector",
        ));
    }

    let selector_sets = selector_sets(
        &tokens[3..],
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
    validate_named_selector_role(name, &objects, visual_objects, line, "group")?;

    object_groups.insert(name.to_string(), objects);
    Ok(())
}

fn parse_group_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        if tokens.is_empty() {
            i += 1;
            continue;
        }
        if tokens.len() < 3 || tokens.get(1).copied() != Some("=") {
            return Err(parse_error(
                &lines[i],
                "group row must be: <name> = <selector...>",
            ));
        }

        let mut group_tokens = vec!["group"];
        group_tokens.extend(tokens);
        parse_group_directive(
            &group_tokens,
            &lines[i],
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.visual_objects,
            &mut catalog.object_groups,
        )?;
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "groups missing closing brace"));
    }

    Ok(i + 1)
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
    if tokens.len() < 4 || tokens.get(2).copied() != Some("=") {
        return Err(parse_error(
            line,
            "legend must be: legend <char> = <selector...>",
        ));
    }

    let ch = parse_char(tokens.get(1), line, "missing legend char")?;
    let selector_sets = selector_sets(
        &tokens[3..],
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

fn parse_global_directive(
    tokens: &[&str],
    line: &str,
    global_names: &mut HashMap<String, GlobalId>,
    global_labels: &mut HashMap<GlobalId, String>,
    global_defaults: &mut Vec<i64>,
    numeric_global_defaults: &mut HashMap<String, i64>,
    persistent_vars: &mut Vec<GlobalId>,
    constant_globals: &mut Vec<GlobalId>,
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
            if global_names.contains_key(name) {
                return Err(parse_error(line, "duplicate var or const"));
            }
            let id = GlobalId(global_defaults.len() as u16);
            let default = parse_global_value(value, line)?;
            global_names.insert(name.to_string(), id);
            global_labels.insert(id, name.to_string());
            global_defaults.push(default);
            if value.parse::<i64>().is_ok() {
                numeric_global_defaults.insert(name.to_string(), default);
            }
            if persistent {
                persistent_vars.push(id);
            }
            if constant {
                constant_globals.push(id);
            }
            Ok(())
        }
        _ => Err(parse_error(
            line,
            "var or const must be: var <name> = <true | false | number> or const <name> = <true | false | number>",
        )),
    }
}

fn parse_condition_directive(
    _tokens: &[&str],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    condition_names: &mut HashMap<String, ConditionId>,
    condition_labels: &mut HashMap<ConditionId, String>,
) -> Result<ConditionDefinitionAst, DiagnosticReport> {
    let Some(rest) = line.strip_prefix("condition ") else {
        return Err(parse_error(
            line,
            "condition must be: condition <name> = <condition_expr>",
        ));
    };
    let Some((name, expr)) = rest.split_once('=') else {
        return Err(parse_error(
            line,
            "condition must be: condition <name> = <condition_expr>",
        ));
    };
    let name = name.trim();
    validate_qualified_identifier(name, line, "condition name")?;
    if condition_names.contains_key(name) {
        return Err(parse_error(line, "duplicate condition"));
    }
    let id = ConditionId(condition_names.len() as u16);
    let kind = parse_condition_value_expr(
        expr.trim(),
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )?;
    condition_names.insert(name.to_string(), id);
    condition_labels.insert(id, name.to_string());
    Ok(ConditionDefinitionAst { id, kind })
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
        "none" if pattern_arg.is_some() => Ok(ConditionValueAst::NoneMatches(
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
        "none" => Ok(ConditionValueAst::NoneObjects(
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
        _ => Err(parse_error(line, "unknown condition function")),
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
    let trimmed = arg.trim();
    if trimmed.starts_with('[') {
        let (embedded_orientation, pattern) = normalize_embedded_direction_marker(trimmed);
        return Ok(Some((
            embedded_orientation.unwrap_or(OrientationExpr::Neutral),
            pattern,
        )));
    }

    let Some(open_index) = trimmed.find('[') else {
        return Ok(None);
    };
    let orientation = trimmed[..open_index].trim();
    let pattern = trimmed[open_index..].trim();
    if orientation.is_empty() {
        return Ok(Some((OrientationExpr::Neutral, pattern.to_string())));
    }
    let orientation = if let Some(axis) = orientation.strip_prefix("input ").map(str::trim) {
        if !is_identifier(axis) {
            return Err(parse_error(
                line,
                "input orientation set must be a single identifier",
            ));
        }
        OrientationExpr::InputSet(axis.to_string())
    } else if orientation == "input" {
        OrientationExpr::InputSet("directions".to_string())
    } else if !is_identifier(orientation) {
        return Err(parse_error(
            line,
            "pattern orientation must be a single identifier or input <set>",
        ));
    } else {
        parse_statement_orientation_expr(orientation, &[])
    };
    let (embedded_orientation, pattern) = normalize_embedded_direction_marker(pattern);
    if embedded_orientation.is_some() {
        return Err(parse_error(
            line,
            "pattern cannot combine orientation prefix and embedded direction marker",
        ));
    }
    Ok(Some((orientation, pattern)))
}

fn parse_call_expr<'a>(expr: &'a str, line: &str) -> Result<(&'a str, &'a str), DiagnosticReport> {
    let Some((name, rest)) = expr.split_once('(') else {
        return Err(parse_error(
            line,
            "condition expression must be a function call",
        ));
    };
    if !is_identifier(name) {
        return Err(parse_error(
            line,
            "condition function name must be an identifier",
        ));
    }
    let Some(arg) = rest.strip_suffix(')') else {
        return Err(parse_error(line, "condition expression missing closing )"));
    };
    Ok((name, arg.trim()))
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
    global_names: &HashMap<String, GlobalId>,
    numeric_globals: &HashMap<String, i64>,
    condition_names: &HashMap<String, ConditionId>,
) -> Result<(RuleDefinitionAst, usize), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let declaration = header.first().copied().unwrap_or("routine");
    let role = if header.get(1).copied() == Some("display")
        || header
            .get(1)
            .is_some_and(|name| is_display_role_token(name))
    {
        RuleRole::Visual
    } else {
        RuleRole::Main
    };
    let name_index = if header.get(1).copied() == Some("display") {
        2
    } else {
        1
    };
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
        global_names,
        numeric_globals,
        condition_names,
        &HashMap::new(),
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

#[allow(clippy::too_many_arguments)]
fn add_standard_move_rule_if_missing(
    definitions: &mut Vec<RuleDefinitionAst>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    object_layers: &HashMap<ObjectId, LayerId>,
    visual_objects: &[ObjectId],
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
) -> Result<(), DiagnosticReport> {
    if definitions
        .iter()
        .any(|definition| definition.name == "move")
    {
        return Ok(());
    }
    let mut move_layer_groups = object_layers
        .iter()
        .filter_map(|(object, layer)| {
            (!visual_objects.contains(object)).then_some((*layer, *object))
        })
        .collect::<Vec<_>>();
    move_layer_groups.sort_by_key(|(layer, object)| (layer.0, object.0));

    let mut generated_groups = object_groups.clone();
    let mut generated_layer_names = Vec::new();
    let mut i = 0;
    while i < move_layer_groups.len() {
        let layer = move_layer_groups[i].0;
        let group_name = format!("__move_layer_{}", layer.0);
        let mut objects = Vec::new();
        while i < move_layer_groups.len() && move_layer_groups[i].0 == layer {
            objects.push(move_layer_groups[i].1);
            i += 1;
        }
        generated_groups.insert(group_name.clone(), objects);
        generated_layer_names.push(group_name);
    }

    if generated_layer_names.is_empty() {
        return Ok(());
    }
    let mut generated_value_sets = value_sets.clone();
    generated_value_sets.insert("__move_layers".to_string(), generated_layer_names);

    let lines = vec![
        "for l in __move_layers {".to_string(),
        "for d in directions {".to_string(),
        "once_all d [ d l | | < l ] -> [ l | {__move_collision} | l ]".to_string(),
        "once_all d [ d l | ; | ^ l ] -> [ l | {__move_collision} ; | l ]".to_string(),
        "once_all d [ | v l ; d l | ] -> [ | l ; l | {__move_collision} ]".to_string(),
        BLOCK_CLOSE.to_string(),
        "for d in directions {".to_string(),
        "d [ d l | no l no {__move_collision} ] -> [ | l{no directions} ]".to_string(),
        BLOCK_CLOSE.to_string(),
        "for d in directions {".to_string(),
        "once_all d [ d l ] -> [ l ]".to_string(),
        BLOCK_CLOSE.to_string(),
        "once_all [ {__move_collision} ] -> [ ]".to_string(),
        BLOCK_CLOSE.to_string(),
        BLOCK_CLOSE.to_string(),
    ];
    let (statements, next_i) = parse_statement_block(
        &lines,
        None,
        0,
        &[BLOCK_CLOSE],
        object_names,
        object_schemas,
        &generated_value_sets,
        maps,
        &generated_groups,
        input_names,
        global_names,
        &HashMap::new(),
        condition_names,
        &HashMap::new(),
        &[],
    )?;
    if next_i != lines.len() {
        return Err(DiagnosticReport::error(
            "standard move rule expansion failed".to_string(),
        ));
    }

    definitions.push(RuleDefinitionAst {
        name: "move".to_string(),
        role: RuleRole::Main,
        application: RuleApplication::UntilStable,
        statements,
    });
    Ok(())
}

fn parse_rule_name_and_params(
    value: &str,
    line: &str,
) -> Result<(String, Vec<String>), DiagnosticReport> {
    let Some((name, params)) = value.split_once('(') else {
        validate_rule_name(value, line)?;
        return Ok((value.to_string(), Vec::new()));
    };
    validate_rule_name(name, line)?;
    let params = params
        .strip_suffix(')')
        .ok_or_else(|| parse_error(line, "routine params must end with )"))?;
    let params = if params.trim().is_empty() {
        Vec::new()
    } else {
        params
            .split(',')
            .map(str::trim)
            .map(|param| {
                validate_identifier(param, line, "routine param")?;
                Ok(param.to_string())
            })
            .collect::<Result<Vec<_>, DiagnosticReport>>()?
    };
    Ok((name.to_string(), params))
}

fn parse_lifecycle_block(
    lines: &[String],
    line_numbers: Option<&[usize]>,
    start: usize,
    event: &str,
    catalog: &Catalog,
) -> Result<(String, Vec<StatementAst>, usize), DiagnosticReport> {
    let (statements, next_i) = parse_statement_block(
        lines,
        line_numbers,
        start + 1,
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
        &HashMap::new(),
        &[],
    )?;
    Ok((event.to_string(), statements, next_i))
}

fn parse_rule_application(
    tokens: &[&str],
    declaration: &str,
    role: RuleRole,
    line: &str,
) -> Result<RuleApplication, DiagnosticReport> {
    match (role, tokens) {
        (RuleRole::Main, [kind, _]) if *kind == declaration => Ok(RuleApplication::Once),
        (RuleRole::Visual, [kind, "display", _]) if *kind == declaration => {
            Ok(RuleApplication::Once)
        }
        (RuleRole::Visual, [kind, name]) if *kind == declaration && is_display_role_token(name) => {
            Ok(RuleApplication::Once)
        }
        (RuleRole::Main, [kind, _, application]) if *kind == declaration => {
            parse_application_keyword(application, line)
        }
        (RuleRole::Visual, [kind, "display", _, application]) if *kind == declaration => {
            parse_application_keyword(application, line)
        }
        (RuleRole::Visual, [kind, name, application])
            if *kind == declaration && is_display_role_token(name) =>
        {
            parse_application_keyword(application, line)
        }
        _ => Err(parse_error(
            line,
            "routine header must be: routine [display] <name> [once | once_all | once_per_level | repeat]",
        )),
    }
}

fn parse_application_keyword(token: &str, line: &str) -> Result<RuleApplication, DiagnosticReport> {
    match token {
        "once" => Ok(RuleApplication::Once),
        "once_all" => Ok(RuleApplication::OnceAll),
        "once_per_level" => Ok(RuleApplication::OncePerLevel),
        "repeat" => Ok(RuleApplication::UntilStable),
        _ => Err(parse_error(
            line,
            "application must be one of: once, once_all, once_per_level, repeat",
        )),
    }
}

fn parse_fix_defaults(
    tokens: &[&str],
    line: &str,
    rule_params: &[String],
) -> Result<FixDefaults, DiagnosticReport> {
    if tokens.len() < 2 {
        return Err(parse_error(
            line,
            "fix block must be: fix <once | repeat | orientation...>",
        ));
    }

    let mut defaults = FixDefaults::default();
    for token in &tokens[1..] {
        match *token {
            "once" | "once_all" | "once_per_level" | "repeat" => {
                let application = parse_application_keyword(token, line)?;
                if defaults.application.replace(application).is_some() {
                    return Err(parse_error(line, "fix can specify application only once"));
                }
            }
            orientation => {
                if defaults
                    .orientation
                    .replace(parse_statement_orientation_expr(orientation, rule_params))
                    .is_some()
                {
                    return Err(parse_error(line, "fix can specify orientation only once"));
                }
            }
        }
    }

    Ok(defaults)
}

fn collect_statement_block_lines(
    lines: &[String],
    start: usize,
    line: &str,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let mut body = Vec::new();
    let mut depth = 1i32;
    let mut i = start;
    while i < lines.len() {
        let nested_line = &lines[i];
        let delta = statement_block_line_delta(nested_line);
        let next_depth = depth + delta;
        if next_depth == 0 {
            return Ok((body, i + 1));
        }
        if next_depth < 0 {
            return Err(parse_error(
                line,
                "for block has an unmatched closing brace",
            ));
        }
        body.push(nested_line.clone());
        depth = next_depth;
        i += 1;
    }
    Err(parse_error(line, "for block missing closing brace"))
}

fn collect_statement_block_lines_with_numbers(
    lines: &[String],
    line_numbers: Option<&[usize]>,
    start: usize,
    line: &str,
) -> Result<(Vec<String>, Option<Vec<usize>>, usize), DiagnosticReport> {
    let mut body = Vec::new();
    let mut body_numbers = line_numbers.map(|_| Vec::new());
    let mut depth = 1i32;
    let mut i = start;
    while i < lines.len() {
        let nested_line = &lines[i];
        let delta = statement_block_line_delta(nested_line);
        let next_depth = depth + delta;
        if next_depth == 0 {
            return Ok((body, body_numbers, i + 1));
        }
        if next_depth < 0 {
            return Err(parse_error(
                line,
                "for block has an unmatched closing brace",
            ));
        }
        body.push(nested_line.clone());
        if let (Some(line_numbers), Some(body_numbers)) = (line_numbers, &mut body_numbers) {
            if let Some(line_number) = line_numbers.get(i).copied() {
                body_numbers.push(line_number);
            }
        }
        depth = next_depth;
        i += 1;
    }
    Err(parse_error(line, "for block missing closing brace"))
}

fn statement_block_line_delta(line: &str) -> i32 {
    raw_brace_delta(strip_line_comment(line))
}

fn parse_if_condition_block_header(
    line: &str,
) -> Result<Option<ConditionBlockCombinator>, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        ["if"] => Ok(Some(ConditionBlockCombinator::All)),
        ["if", "all"] => Ok(Some(ConditionBlockCombinator::All)),
        ["if", "any"] => Ok(Some(ConditionBlockCombinator::Any)),
        ["if", ..] if line.trim_end().ends_with('{') => Err(parse_error(
            line,
            "if condition block must be: if [all | any] {",
        )),
        _ => Ok(None),
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_statement_condition_block(
    lines: &[String],
    start: usize,
    combinator: ConditionBlockCombinator,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<(ConditionAst, usize), DiagnosticReport> {
    let mut conditions = Vec::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let condition = parse_statement_condition(
            &lines[i],
            &lines[i],
            input_names,
            global_names,
            condition_names,
            named_conditions,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
        )?;
        conditions.push(condition);
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "if condition block missing closing brace",
        ));
    }
    if conditions.is_empty() {
        return Err(parse_error(
            &lines[start],
            "if condition block requires at least one condition",
        ));
    }
    let condition = if conditions.len() == 1 {
        conditions.remove(0)
    } else {
        combinator.combine(conditions)
    };
    Ok((condition, i + 1))
}

#[allow(clippy::too_many_arguments)]
fn parse_statement_arrow_consequence(
    lines: &[String],
    line_numbers: Option<&[usize]>,
    start: usize,
    header_line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    numeric_globals: &HashMap<String, i64>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    rule_params: &[String],
) -> Result<(Vec<StatementAst>, usize), DiagnosticReport> {
    let Some(line) = lines.get(start) else {
        return Err(parse_error(
            header_line,
            "if condition block must be followed by ->",
        ));
    };
    let header = block_header_text(line);
    let Some((_, effect_text)) = header.split_once("->") else {
        return Err(parse_error(
            line,
            "if condition block must be followed by ->",
        ));
    };
    let effect_text = effect_text.trim();

    if line.trim_end().ends_with('{') {
        if !effect_text.is_empty() {
            return Err(parse_error(line, "if -> block header must be: -> {"));
        }
        return parse_statement_block(
            lines,
            line_numbers,
            start + 1,
            &["else", BLOCK_CLOSE],
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            input_names,
            global_names,
            numeric_globals,
            condition_names,
            named_conditions,
            rule_params,
        );
    }

    if effect_text.is_empty() {
        return Err(parse_error(
            line,
            "if -> must be followed by an effect or block",
        ));
    }
    if is_qualified_identifier(effect_text) && !is_builtin_rewrite_effect_text(effect_text) {
        return Ok((
            vec![StatementAst::Call {
                name: effect_text.to_string(),
                source_line: line.to_string(),
            }],
            start + 1,
        ));
    }
    let effects = parse_rewrite_effect(effect_text, line)?;
    Ok((
        vec![StatementAst::Effect {
            source_line: line.to_string(),
            effects,
        }],
        start + 1,
    ))
}

#[allow(clippy::too_many_arguments)]
fn parse_optional_else_statement_block(
    lines: &[String],
    line_numbers: Option<&[usize]>,
    next_i: usize,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    numeric_globals: &HashMap<String, i64>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    rule_params: &[String],
) -> Result<(Vec<StatementAst>, usize), DiagnosticReport> {
    let Some(else_start) = else_block_start(lines, next_i) else {
        return Ok((Vec::new(), next_i));
    };
    parse_statement_block(
        lines,
        line_numbers,
        else_start,
        &[BLOCK_CLOSE],
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        input_names,
        global_names,
        numeric_globals,
        condition_names,
        named_conditions,
        rule_params,
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ValueExpr {
    Binding(String),
    MapCall { name: String, arg: String },
}

#[derive(Clone, Debug, Default)]
struct ValueEnv {
    values: HashMap<String, String>,
    axes: HashMap<String, String>,
}

impl ValueEnv {
    fn bind(&mut self, name: &str, axis: &str, value: &str) {
        self.values.insert(name.to_string(), value.to_string());
        self.axes.insert(name.to_string(), axis.to_string());
    }

    fn bind_untyped(&mut self, name: &str, value: &str) {
        self.values.insert(name.to_string(), value.to_string());
    }
}

fn visual_value_env(bindings: &HashMap<String, String>) -> ValueEnv {
    let mut env = ValueEnv::default();
    for (axis, value) in bindings {
        env.bind(axis, axis, value);
    }
    env
}

fn parse_value_expr(value: &str, line: &str) -> Result<ValueExpr, DiagnosticReport> {
    if let Some((name, arg)) = parse_map_call(value) {
        validate_identifier(name, line, "map name")?;
        validate_identifier(arg, line, "map argument")?;
        return Ok(ValueExpr::MapCall {
            name: name.to_string(),
            arg: arg.to_string(),
        });
    }
    if !is_value_atom(value) {
        return Err(parse_error(
            line,
            "value expression must be an identifier-like atom",
        ));
    }
    Ok(ValueExpr::Binding(value.to_string()))
}

fn eval_bound_value_expr(
    expr: &ValueExpr,
    env: &ValueEnv,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    eval_value_expr(expr, env, maps, line, false)
}

fn eval_value_expr(
    expr: &ValueExpr,
    env: &ValueEnv,
    maps: &HashMap<String, ValueMap>,
    line: &str,
    allow_literal: bool,
) -> Result<String, DiagnosticReport> {
    match expr {
        ValueExpr::Binding(name) => {
            if let Some(value) = env.values.get(name) {
                Ok(value.clone())
            } else if allow_literal {
                Ok(name.clone())
            } else {
                Err(parse_error(
                    line,
                    "value expression binding is not in scope",
                ))
            }
        }
        ValueExpr::MapCall { name, arg } => {
            let map = maps
                .get(name)
                .ok_or_else(|| parse_error(line, "unknown map"))?;
            let value = env
                .values
                .get(arg)
                .ok_or_else(|| parse_error(line, "map argument binding is not in scope"))?;
            let axis = env
                .axes
                .get(arg)
                .ok_or_else(|| parse_error(line, "map argument tag set is not known"))?;
            if map.axis != *axis {
                return Err(parse_error(line, "map tag set must match argument tag set"));
            }
            map.values
                .get(value)
                .cloned()
                .ok_or_else(|| parse_error(line, "map missing input value"))
        }
    }
}

fn value_expr_result_axis(
    expr: &ValueExpr,
    env: &ValueEnv,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    match expr {
        ValueExpr::Binding(name) => env
            .axes
            .get(name)
            .cloned()
            .ok_or_else(|| parse_error(line, "value expression binding tag set is not known")),
        ValueExpr::MapCall { name, arg } => {
            let map = maps
                .get(name)
                .ok_or_else(|| parse_error(line, "unknown map"))?;
            let axis = env
                .axes
                .get(arg)
                .ok_or_else(|| parse_error(line, "map argument tag set is not known"))?;
            if map.axis != *axis {
                return Err(parse_error(line, "map tag set must match argument tag set"));
            }
            Ok(map.axis.clone())
        }
    }
}

fn expand_for_binding_lines(
    lines: &[String],
    binding: &str,
    axis: Option<&str>,
    value: &str,
    maps: &HashMap<String, ValueMap>,
) -> Result<Vec<String>, DiagnosticReport> {
    lines
        .iter()
        .map(|line| expand_for_binding_line(line, binding, axis, value, maps))
        .collect()
}

fn expand_for_binding_line(
    line: &str,
    binding: &str,
    axis: Option<&str>,
    value: &str,
    maps: &HashMap<String, ValueMap>,
) -> Result<String, DiagnosticReport> {
    let mut env = ValueEnv::default();
    if let Some(axis) = axis {
        env.bind(binding, axis, value);
    } else {
        env.bind_untyped(binding, value);
    }
    let expanded = replace_map_call_tokens(line, &env, maps)?;
    Ok(replace_identifier_token(&expanded, binding, value))
}

fn replace_map_call_tokens(
    line: &str,
    env: &ValueEnv,
    maps: &HashMap<String, ValueMap>,
) -> Result<String, DiagnosticReport> {
    let mut out = String::with_capacity(line.len());
    let chars = line.chars().collect::<Vec<_>>();
    let mut i = 0usize;
    while i < chars.len() {
        if is_identifier_start(chars[i]) {
            let name_start = i;
            i += 1;
            while i < chars.len() && is_identifier_continue(chars[i]) {
                i += 1;
            }
            if i < chars.len() && chars[i] == '(' {
                let arg_start = i + 1;
                let mut arg_end = arg_start;
                while arg_end < chars.len() && is_identifier_continue(chars[arg_end]) {
                    arg_end += 1;
                }
                if arg_end > arg_start && arg_end < chars.len() && chars[arg_end] == ')' {
                    let name = chars[name_start..i].iter().collect::<String>();
                    let arg = chars[arg_start..arg_end].iter().collect::<String>();
                    if maps.contains_key(&name) {
                        let expr = ValueExpr::MapCall { name, arg };
                        out.push_str(&eval_bound_value_expr(&expr, env, maps, line)?);
                        i = arg_end + 1;
                        continue;
                    }
                }
            }
            out.extend(chars[name_start..i].iter());
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    Ok(out)
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn replace_identifier_token(line: &str, binding: &str, value: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut token = String::new();
    for ch in line.chars() {
        if ch == '_' || ch.is_ascii_alphanumeric() {
            token.push(ch);
            continue;
        }
        flush_identifier_token(&mut out, &mut token, binding, value);
        out.push(ch);
    }
    flush_identifier_token(&mut out, &mut token, binding, value);
    out
}

fn flush_identifier_token(out: &mut String, token: &mut String, binding: &str, value: &str) {
    if token.is_empty() {
        return;
    }
    if token == binding {
        out.push_str(value);
    } else {
        out.push_str(token);
    }
    token.clear();
}

#[derive(Clone, Debug)]
struct ForExpansionValue {
    value: String,
    axis: Option<String>,
}

fn for_expansion_values(
    sources: &[&str],
    value_sets: &HashMap<String, Vec<String>>,
    numeric_globals: &HashMap<String, i64>,
    line: &str,
) -> Result<Vec<ForExpansionValue>, DiagnosticReport> {
    if sources.is_empty() {
        return Err(parse_error(
            line,
            "for directive must be: for <binding> in <source...>",
        ));
    }
    if sources.len() == 1 {
        let source = sources[0];
        if let Some(values) = value_sets.get(source) {
            return Ok(values
                .iter()
                .map(|value| ForExpansionValue {
                    value: value.clone(),
                    axis: Some(source.to_string()),
                })
                .collect());
        }
        if let Some(values) = numeric_range_values(source, numeric_globals, line)? {
            return Ok(values
                .into_iter()
                .map(|value| ForExpansionValue { value, axis: None })
                .collect());
        }
        return Err(parse_error(
            line,
            "unknown expansion tag set or numeric range",
        ));
    }

    sources
        .iter()
        .flat_map(|source| {
            if let Some(values) = value_sets.get(*source) {
                return values
                    .iter()
                    .map(|value| {
                        Ok(ForExpansionValue {
                            value: value.clone(),
                            axis: Some((*source).to_string()),
                        })
                    })
                    .collect::<Vec<_>>();
            }
            match numeric_range_values(source, numeric_globals, line) {
                Ok(Some(values)) => values
                    .into_iter()
                    .map(|value| Ok(ForExpansionValue { value, axis: None }))
                    .collect(),
                Ok(None) => vec![Ok(ForExpansionValue {
                    value: (*source).to_string(),
                    axis: None,
                })],
                Err(error) => vec![Err(error)],
            }
        })
        .collect()
}

fn expand_numeric_ranges_in_value_list(
    values: &[&str],
    numeric_globals: &HashMap<String, i64>,
    line: &str,
) -> Result<Vec<String>, DiagnosticReport> {
    let mut expanded = Vec::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(parse_error(line, "tag value must not be empty"));
        }
        if let Some(range_values) = numeric_range_values(value, numeric_globals, line)? {
            expanded.extend(range_values);
        } else {
            expanded.push((*value).to_string());
        }
    }
    Ok(expanded)
}

fn numeric_range_values(
    source: &str,
    numeric_globals: &HashMap<String, i64>,
    line: &str,
) -> Result<Option<Vec<String>>, DiagnosticReport> {
    let Some((start, end)) = source.split_once("...") else {
        return Ok(None);
    };
    if start.is_empty() || end.is_empty() || end.contains("...") {
        return Err(parse_error(
            line,
            "numeric range must be: <integer>...<integer>",
        ));
    }
    let start = parse_numeric_range_endpoint(start, numeric_globals, line)?;
    let end = parse_numeric_range_endpoint(end, numeric_globals, line)?;
    if start > end {
        return Err(parse_error(
            line,
            "numeric range start must be less than or equal to end",
        ));
    }
    Ok(Some((start..=end).map(|value| value.to_string()).collect()))
}

fn parse_numeric_range_endpoint(
    value: &str,
    numeric_globals: &HashMap<String, i64>,
    line: &str,
) -> Result<i64, DiagnosticReport> {
    if let Ok(parsed) = value.parse::<i64>() {
        return Ok(parsed);
    }
    numeric_globals.get(value).copied().ok_or_else(|| {
        parse_error(
            line,
            "numeric range endpoints must be integer literals or integer vars",
        )
    })
}

fn collect_multiline_rewrite_statement(
    lines: &[String],
    start: usize,
) -> Result<Option<(String, usize)>, DiagnosticReport> {
    let line = lines[start].trim();
    if let Some(collected) = collect_bracket_multiline_rewrite_statement(lines, start, line)? {
        return Ok(Some(collected));
    }

    let Some(trailing) = rewrite_lhs_trailing(line) else {
        return Ok(None);
    };

    if trailing.is_empty() {
        let Some(next_line) = lines.get(start + 1).map(|line| line.trim()) else {
            return Ok(None);
        };
        let Some(rhs) = next_line.strip_prefix("->").map(str::trim_start) else {
            return Ok(None);
        };
        validate_rewrite_rhs_continuation(rhs, next_line)?;
        return Ok(Some((format!("{line} -> {rhs}"), start + 2)));
    }

    if trailing == "->" {
        let Some(rhs) = lines.get(start + 1).map(|line| line.trim()) else {
            return Ok(None);
        };
        validate_rewrite_rhs_continuation(rhs, line)?;
        return Ok(Some((format!("{line} {rhs}"), start + 2)));
    }

    Ok(None)
}

fn collect_bracket_multiline_rewrite_statement(
    lines: &[String],
    start: usize,
    first_line: &str,
) -> Result<Option<(String, usize)>, DiagnosticReport> {
    let Some(open_index) = first_line.find('[') else {
        return Ok(None);
    };
    let prefix = first_line[..open_index].trim();
    if !can_start_rewrite_lhs(prefix) {
        return Ok(None);
    }

    let mut joined = String::new();
    let mut bracket_depth = 0usize;
    let mut saw_arrow = false;
    let mut i = start;
    while i < lines.len() {
        let line = lines[i].trim();
        if i > start && bracket_depth == 0 && !saw_arrow && !line.starts_with("->") {
            return Ok(None);
        }
        if !joined.is_empty() {
            if bracket_depth > 0 {
                joined.push_str("; ");
            } else {
                joined.push(' ');
            }
        }
        joined.push_str(line);
        bracket_depth = update_square_bracket_depth(bracket_depth, line);
        saw_arrow |= line.contains("->");

        if i == start && bracket_depth == 0 {
            return Ok(None);
        }
        if i > start && bracket_depth == 0 && saw_arrow {
            validate_rewrite_rhs_continuation_after_join(&joined)?;
            return Ok(Some((joined, i + 1)));
        }
        i += 1;
    }

    Ok(None)
}

fn update_square_bracket_depth(mut depth: usize, line: &str) -> usize {
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

fn validate_rewrite_rhs_continuation_after_join(line: &str) -> Result<(), DiagnosticReport> {
    let Some((_, rhs)) = line.split_once("->") else {
        return Ok(());
    };
    validate_rewrite_rhs_continuation(rhs.trim_start(), line)
}

fn validate_rewrite_rhs_continuation(rhs: &str, line: &str) -> Result<(), DiagnosticReport> {
    if rhs.is_empty() || !rhs.starts_with('[') {
        return Err(parse_error(
            line,
            "rewrite continuation after -> must start with a pattern",
        ));
    }
    if rhs.contains("->") {
        return Err(parse_error(
            line,
            "rewrite continuation rhs cannot contain another ->",
        ));
    }
    Ok(())
}

fn rewrite_lhs_trailing(line: &str) -> Option<&str> {
    let open_index = line.find('[')?;
    let prefix = line[..open_index].trim();
    if !can_start_rewrite_lhs(prefix) {
        return None;
    }
    let lhs_end = open_index + pattern_side_syntax_end(&line[open_index..])?;
    Some(line[lhs_end..].trim())
}

fn can_start_rewrite_lhs(prefix: &str) -> bool {
    let tokens = split_header_tokens(prefix);
    match tokens.as_slice() {
        [] => true,
        ["input", axis] => is_identifier(axis),
        [application] if is_rewrite_application_prefix(application) => true,
        [application, "input", axis] if is_rewrite_application_prefix(application) => {
            is_identifier(axis)
        }
        [application, orientation]
            if is_rewrite_application_prefix(application) && is_identifier(orientation) =>
        {
            true
        }
        [orientation] if !is_non_rewrite_statement_prefix(orientation) => {
            is_identifier(orientation)
        }
        _ => false,
    }
}

fn is_rewrite_application_prefix(token: &str) -> bool {
    puzzle_authoring::rule_application_surface(token).is_some()
}

fn is_non_rewrite_statement_prefix(token: &str) -> bool {
    matches!(
        token,
        "for" | "fix" | "if" | "else" | "when" | "action" | "emit" | "do" | "display"
    )
}

fn pattern_side_syntax_end(value: &str) -> Option<usize> {
    let mut index = 0;
    let mut found_block = false;
    while index < value.len() {
        let after_space = value[index..].trim_start();
        index = value.len() - after_space.len();
        if !value[index..].starts_with('[') {
            break;
        }
        let after_open = index + 1;
        let close_offset = value[after_open..].find(']')?;
        index = after_open + close_offset + 1;
        found_block = true;
    }
    found_block.then_some(index)
}

fn else_block_start(lines: &[String], next_i: usize) -> Option<usize> {
    if next_i > 0 && is_else_block_marker(&lines[next_i - 1]) {
        Some(next_i)
    } else if next_i < lines.len() && is_else_block_marker(&lines[next_i]) {
        Some(next_i + 1)
    } else {
        None
    }
}

fn is_else_block_marker(line: &str) -> bool {
    line == "else" || line == "else {"
}

fn statement_block_terminator_matches(line: &str, terminators: &[&str]) -> bool {
    terminators.contains(&line) || (terminators.contains(&"else") && is_else_block_marker(line))
}

fn parse_statement_block(
    lines: &[String],
    line_numbers: Option<&[usize]>,
    start: usize,
    terminators: &[&str],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    numeric_globals: &HashMap<String, i64>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    rule_params: &[String],
) -> Result<(Vec<StatementAst>, usize), DiagnosticReport> {
    let mut statements = Vec::new();
    let mut diagnostics = Vec::new();
    let mut i = start;
    macro_rules! recover_current_statement {
        ($result:expr) => {
            match $result {
                Ok(value) => value,
                Err(report) => {
                    diagnostics.extend(report.into_diagnostics());
                    i = recover_after_directive_error(lines, i);
                    continue;
                }
            }
        };
    }

    while i < lines.len() {
        let source_line = &lines[i];
        let source_line_number = line_numbers.and_then(|line_numbers| line_numbers.get(i).copied());
        if statement_block_terminator_matches(source_line, terminators) {
            return if diagnostics.is_empty() {
                Ok((statements, i + 1))
            } else {
                Err(DiagnosticReport::from_diagnostics(diagnostics))
            };
        }

        let mut next_statement_i = i + 1;
        let joined_line;
        let line = match collect_multiline_rewrite_statement(lines, i) {
            Ok(Some((joined, next_i))) => {
                next_statement_i = next_i;
                joined_line = joined;
                joined_line.as_str()
            }
            Ok(None) => source_line.as_str(),
            Err(report) => {
                diagnostics.extend(report.into_diagnostics());
                i += 1;
                continue;
            }
        };
        let opens_block = line.trim_end().ends_with('{');
        let line = block_header_text(line);
        let tokens = split_header_tokens(line);
        match tokens.first().copied() {
            Some("for") => {
                if !opens_block {
                    diagnostics.extend(
                        parse_error(line, "for block must use `{ ... }`").into_diagnostics(),
                    );
                    i += 1;
                    continue;
                }
                let ["for", binding, "in", sources @ ..] = tokens.as_slice() else {
                    diagnostics.extend(
                        parse_error(line, "for directive must be: for <binding> in <source...>")
                            .into_diagnostics(),
                    );
                    i = recover_after_directive_error(lines, i);
                    continue;
                };
                let values = recover_current_statement!(for_expansion_values(
                    sources,
                    value_sets,
                    numeric_globals,
                    line
                ));
                recover_current_statement!(validate_identifier(binding, line, "expansion binding"));
                let (body_lines, body_line_numbers, next_i) = recover_current_statement!(
                    collect_statement_block_lines_with_numbers(lines, line_numbers, i + 1, line)
                );
                for value in &values {
                    let mut expanded_lines = match expand_for_binding_lines(
                        &body_lines,
                        binding,
                        value.axis.as_deref(),
                        &value.value,
                        maps,
                    ) {
                        Ok(lines) => lines,
                        Err(report) => {
                            diagnostics.extend(report.into_diagnostics());
                            continue;
                        }
                    };
                    let mut expanded_line_numbers = body_line_numbers.clone();
                    expanded_lines.push(BLOCK_CLOSE.to_string());
                    if let Some(line_numbers) = &mut expanded_line_numbers {
                        let Some(source_line_number) = source_line_number else {
                            diagnostics.extend(
                                DiagnosticReport::error(
                                    "internal statement source line number missing",
                                )
                                .into_diagnostics(),
                            );
                            continue;
                        };
                        line_numbers.push(source_line_number);
                    }
                    let (nested, parsed_i) = match parse_statement_block(
                        &expanded_lines,
                        expanded_line_numbers.as_deref(),
                        0,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ) {
                        Ok(parsed) => parsed,
                        Err(report) => {
                            diagnostics.extend(report.into_diagnostics());
                            continue;
                        }
                    };
                    if parsed_i != expanded_lines.len() {
                        diagnostics
                            .extend(parse_error(line, "for expansion failed").into_diagnostics());
                        continue;
                    }
                    statements.extend(nested);
                }
                i = next_i;
                continue;
            }
            Some("fix") => {
                let defaults =
                    recover_current_statement!(parse_fix_defaults(&tokens, line, rule_params));
                let (nested, next_i) = recover_current_statement!(parse_statement_block(
                    lines,
                    line_numbers,
                    i + 1,
                    &[BLOCK_CLOSE],
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    input_names,
                    global_names,
                    numeric_globals,
                    condition_names,
                    named_conditions,
                    rule_params,
                ));
                statements.push(StatementAst::Fix {
                    defaults,
                    statements: nested,
                });
                i = next_i;
            }
            Some("if") => {
                if let Some(combinator) =
                    recover_current_statement!(parse_if_condition_block_header(line))
                {
                    let (condition, arrow_i) =
                        recover_current_statement!(parse_statement_condition_block(
                            lines,
                            i,
                            combinator,
                            input_names,
                            global_names,
                            condition_names,
                            named_conditions,
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                        ));
                    let (then_statements, next_i) =
                        recover_current_statement!(parse_statement_arrow_consequence(
                            lines,
                            line_numbers,
                            arrow_i,
                            line,
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            input_names,
                            global_names,
                            numeric_globals,
                            condition_names,
                            named_conditions,
                            rule_params,
                        ));
                    let (else_statements, next_i) =
                        recover_current_statement!(parse_optional_else_statement_block(
                            lines,
                            line_numbers,
                            next_i,
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            input_names,
                            global_names,
                            numeric_globals,
                            condition_names,
                            named_conditions,
                            rule_params,
                        ));
                    statements.push(StatementAst::If {
                        source_line: line.to_string(),
                        condition,
                        then_statements,
                        else_statements,
                    });
                    i = next_i;
                    continue;
                }
                if let Some((condition, trailing)) =
                    recover_current_statement!(parse_pattern_if_header(
                        line,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    ))
                {
                    if trailing.is_empty() {
                        let (nested, next_i) = recover_current_statement!(parse_statement_block(
                            lines,
                            line_numbers,
                            i + 1,
                            &["else", BLOCK_CLOSE],
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            input_names,
                            global_names,
                            numeric_globals,
                            condition_names,
                            named_conditions,
                            rule_params,
                        ));
                        let (then_statements, else_statements, after_i) =
                            if let Some(else_start) = else_block_start(lines, next_i) {
                                let (else_statements, after_else_i) =
                                    recover_current_statement!(parse_statement_block(
                                        lines,
                                        line_numbers,
                                        else_start,
                                        &[BLOCK_CLOSE],
                                        object_names,
                                        object_schemas,
                                        value_sets,
                                        maps,
                                        object_groups,
                                        input_names,
                                        global_names,
                                        numeric_globals,
                                        condition_names,
                                        named_conditions,
                                        rule_params,
                                    ));
                                (nested, else_statements, after_else_i)
                            } else {
                                (nested, Vec::new(), next_i)
                            };
                        statements.push(StatementAst::Conditional {
                            source_line: line.to_string(),
                            condition,
                            then_statements,
                            else_statements,
                        });
                        i = after_i;
                    } else {
                        recover_current_statement!(validate_qualified_identifier(
                            trailing,
                            line,
                            "routine name"
                        ));
                        statements.push(StatementAst::Conditional {
                            source_line: line.to_string(),
                            condition,
                            then_statements: vec![StatementAst::Call {
                                name: trailing.to_string(),
                                source_line: line.to_string(),
                            }],
                            else_statements: Vec::new(),
                        });
                        i += 1;
                    }
                    continue;
                }
                if let Some((condition_text, _)) = line
                    .strip_prefix("if")
                    .unwrap_or("")
                    .trim_start()
                    .split_once("->")
                {
                    let condition = recover_current_statement!(parse_statement_condition(
                        condition_text.trim(),
                        line,
                        input_names,
                        global_names,
                        condition_names,
                        named_conditions,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    ));
                    let (then_statements, next_i) =
                        recover_current_statement!(parse_statement_arrow_consequence(
                            lines,
                            line_numbers,
                            i,
                            line,
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            input_names,
                            global_names,
                            numeric_globals,
                            condition_names,
                            named_conditions,
                            rule_params,
                        ));
                    let (else_statements, next_i) =
                        recover_current_statement!(parse_optional_else_statement_block(
                            lines,
                            line_numbers,
                            next_i,
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            input_names,
                            global_names,
                            numeric_globals,
                            condition_names,
                            named_conditions,
                            rule_params,
                        ));
                    statements.push(StatementAst::If {
                        source_line: line.to_string(),
                        condition,
                        then_statements,
                        else_statements,
                    });
                    i = next_i;
                    continue;
                }
                let condition = recover_current_statement!(parse_statement_condition(
                    line.strip_prefix("if ").map(str::trim).unwrap_or(""),
                    line,
                    input_names,
                    global_names,
                    condition_names,
                    named_conditions,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                ));
                let (then_statements, next_i) = recover_current_statement!(parse_statement_block(
                    lines,
                    line_numbers,
                    i + 1,
                    &["else", BLOCK_CLOSE],
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    input_names,
                    global_names,
                    numeric_globals,
                    condition_names,
                    named_conditions,
                    rule_params,
                ));
                if next_i == 0 {
                    diagnostics.extend(
                        parse_error(line, "if block missing closing brace").into_diagnostics(),
                    );
                    i = recover_after_directive_error(lines, i);
                    continue;
                }
                let (else_statements, next_i) =
                    recover_current_statement!(parse_optional_else_statement_block(
                        lines,
                        line_numbers,
                        next_i,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ));
                statements.push(StatementAst::If {
                    source_line: line.to_string(),
                    condition,
                    then_statements,
                    else_statements,
                });
                i = next_i;
            }
            Some("else") => {
                diagnostics.extend(parse_error(line, "else without if").into_diagnostics());
                i += 1;
            }
            Some("when") => {
                diagnostics.extend(parse_error(line, "use `if` for conditions").into_diagnostics());
                i += 1;
            }
            Some("action") if tokens.len() > 1 => {
                diagnostics.extend(
                    parse_error(
                        line,
                        "`action` statements were removed; use explicit input guards and rewrites",
                    )
                    .into_diagnostics(),
                );
                i += 1;
            }
            Some("emit") => {
                match parse_rewrite_effect(line, line) {
                    Ok(effects) => statements.push(StatementAst::Effect {
                        source_line: line.to_string(),
                        effects,
                    }),
                    Err(report) => diagnostics.extend(report.into_diagnostics()),
                }
                i += 1;
            }
            Some("do") => {
                diagnostics.extend(
                    parse_error(
                        line,
                        "`do` is obsolete; write the effect statement directly",
                    )
                    .into_diagnostics(),
                );
                i += 1;
            }
            _ if is_input_effect_statement(line) => {
                let (input_name, effect_text) = line
                    .split_once("->")
                    .expect("input effect statement contains arrow");
                let input_name = input_name.trim();
                recover_current_statement!(validate_identifier(input_name, line, "input name"));
                let condition = ConditionAst::InputIs(input_name.to_string());
                let effect_text = effect_text.trim();
                if effect_text.is_empty() || effect_text == "{" {
                    let (then_statements, next_i) =
                        recover_current_statement!(parse_statement_block(
                            lines,
                            line_numbers,
                            i + 1,
                            &[BLOCK_CLOSE],
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            input_names,
                            global_names,
                            numeric_globals,
                            condition_names,
                            named_conditions,
                            rule_params,
                        ));
                    statements.push(StatementAst::If {
                        source_line: line.to_string(),
                        condition,
                        then_statements,
                        else_statements: Vec::new(),
                    });
                    i = next_i;
                } else {
                    match parse_rewrite_effect(effect_text, line) {
                        Ok(effects) => statements.push(StatementAst::If {
                            source_line: line.to_string(),
                            condition,
                            then_statements: vec![StatementAst::Effect {
                                source_line: line.to_string(),
                                effects,
                            }],
                            else_statements: Vec::new(),
                        }),
                        Err(report) => diagnostics.extend(report.into_diagnostics()),
                    }
                    i += 1;
                }
            }
            _ if is_builtin_rewrite_effect_text(line) => {
                match parse_rewrite_effect(line, line) {
                    Ok(effects) => statements.push(StatementAst::Effect {
                        source_line: line.to_string(),
                        effects,
                    }),
                    Err(report) => diagnostics.extend(report.into_diagnostics()),
                }
                i += 1;
            }
            Some("[") => {
                if let Some(statement) = match parse_conditional_call_statement(
                    line,
                    None,
                    rule_params,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    global_names,
                ) {
                    Ok(statement) => statement,
                    Err(report) => {
                        diagnostics.extend(report.into_diagnostics());
                        i = next_statement_i;
                        continue;
                    }
                } {
                    statements.push(statement);
                } else {
                    match parse_neutral_rewrite_statement(
                        line,
                        None,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => diagnostics.extend(report.into_diagnostics()),
                    }
                }
                i = next_statement_i;
            }
            Some("once") => {
                if tokens.len() == 1 {
                    let (nested, next_i) = recover_current_statement!(parse_statement_block(
                        lines,
                        line_numbers,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ));
                    statements.push(StatementAst::Block {
                        application: RuleApplication::Once,
                        statements: nested,
                    });
                    i = next_i;
                } else {
                    match parse_application_prefixed_rewrite_statement(
                        line,
                        "once",
                        RuleApplication::Once,
                        rule_params,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => diagnostics.extend(report.into_diagnostics()),
                    }
                    i = next_statement_i;
                }
            }
            Some("once_all") => {
                if tokens.len() == 1 {
                    let (nested, next_i) = recover_current_statement!(parse_statement_block(
                        lines,
                        line_numbers,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ));
                    statements.push(StatementAst::Block {
                        application: RuleApplication::OnceAll,
                        statements: nested,
                    });
                    i = next_i;
                } else {
                    match parse_application_prefixed_rewrite_statement(
                        line,
                        "once_all",
                        RuleApplication::OnceAll,
                        rule_params,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => diagnostics.extend(report.into_diagnostics()),
                    }
                    i = next_statement_i;
                }
            }
            Some("once_per_level") => {
                if tokens.len() == 1 {
                    let (nested, next_i) = recover_current_statement!(parse_statement_block(
                        lines,
                        line_numbers,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ));
                    statements.push(StatementAst::Block {
                        application: RuleApplication::OncePerLevel,
                        statements: nested,
                    });
                    i = next_i;
                } else {
                    match parse_application_prefixed_rewrite_statement(
                        line,
                        "once_per_level",
                        RuleApplication::OncePerLevel,
                        rule_params,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => diagnostics.extend(report.into_diagnostics()),
                    }
                    i = next_statement_i;
                }
            }
            Some("repeat") => {
                if tokens.len() == 1 {
                    let (nested, next_i) = recover_current_statement!(parse_statement_block(
                        lines,
                        line_numbers,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ));
                    statements.push(StatementAst::Block {
                        application: RuleApplication::UntilStable,
                        statements: nested,
                    });
                    i = next_i;
                } else if tokens.get(1).copied() == Some("until") {
                    let condition_text = line
                        .strip_prefix("repeat")
                        .and_then(|rest| rest.trim_start().strip_prefix("until"))
                        .map(str::trim)
                        .unwrap_or("");
                    if condition_text.is_empty() {
                        diagnostics.extend(
                            parse_error(
                                line,
                                "repeat until block must be: repeat until <condition>",
                            )
                            .into_diagnostics(),
                        );
                        i += 1;
                        continue;
                    }
                    let condition = recover_current_statement!(parse_statement_condition(
                        condition_text,
                        line,
                        input_names,
                        global_names,
                        condition_names,
                        named_conditions,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    ));
                    let (nested, next_i) = recover_current_statement!(parse_statement_block(
                        lines,
                        line_numbers,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ));
                    statements.push(StatementAst::RepeatUntil {
                        source_line: line.to_string(),
                        condition,
                        statements: nested,
                    });
                    i = next_i;
                } else {
                    match parse_application_prefixed_rewrite_statement(
                        line,
                        "repeat",
                        RuleApplication::UntilStable,
                        rule_params,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => diagnostics.extend(report.into_diagnostics()),
                    }
                    i = next_statement_i;
                }
            }
            Some(_) if line.starts_with('[') => {
                if let Some(statement) = match parse_conditional_call_statement(
                    line,
                    None,
                    rule_params,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    global_names,
                ) {
                    Ok(statement) => statement,
                    Err(report) => {
                        diagnostics.extend(report.into_diagnostics());
                        i = next_statement_i;
                        continue;
                    }
                } {
                    statements.push(statement);
                } else {
                    match parse_neutral_rewrite_statement(
                        line,
                        None,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => diagnostics.extend(report.into_diagnostics()),
                    }
                }
                i = next_statement_i;
            }
            Some("display") => {
                if tokens.len() == 1 {
                    let (nested, next_i) = recover_current_statement!(parse_statement_block(
                        lines,
                        line_numbers,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ));
                    statements.push(StatementAst::DisplayBlock(nested));
                    i = next_i;
                } else {
                    match parse_display_statement(
                        line,
                        source_line_number,
                        rule_params,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    ) {
                        Ok(statement) => statements.push(statement),
                        Err(report) => diagnostics.extend(report.into_diagnostics()),
                    }
                    i += 1;
                }
            }
            Some(first) if is_oriented_rewrite_line(line, first) => {
                if let Some(statement) = match parse_conditional_call_statement(
                    line,
                    Some(first),
                    rule_params,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    global_names,
                ) {
                    Ok(statement) => statement,
                    Err(report) => {
                        diagnostics.extend(report.into_diagnostics());
                        i = next_statement_i;
                        continue;
                    }
                } {
                    statements.push(statement);
                } else {
                    match parse_oriented_rewrite_statement(
                        line,
                        first,
                        None,
                        rule_params,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => diagnostics.extend(report.into_diagnostics()),
                    }
                }
                i = next_statement_i;
            }
            Some("move") if is_shared_standard_move_statement(line) => {
                statements.push(StatementAst::Call {
                    name: "move".to_string(),
                    source_line: line.to_string(),
                });
                i += 1;
            }
            Some(call) if tokens.len() == 1 && is_shared_rule_call_statement(line, call) => {
                statements.push(StatementAst::Call {
                    name: call.to_string(),
                    source_line: line.to_string(),
                });
                i += 1;
            }
            Some(call) if tokens.len() == 1 && is_display_role_token(call) => {
                statements.push(StatementAst::DisplayCall {
                    name: call.to_string(),
                    source_line: line.to_string(),
                });
                i += 1;
            }
            Some(other) if scene_effect_command_syntax(other).is_some() => {
                diagnostics.extend(
                    parse_error(
                        line,
                        &format!(
                            "scene effect `{other}` cannot be used in puzzle statement blocks; \
                         put scene effects in a scene lifecycle, scene routine, \
                         or scene component effect"
                        ),
                    )
                    .into_diagnostics(),
                );
                i += 1;
            }
            Some(other) => {
                diagnostics.extend(
                    parse_error(line, &format!("unknown statement directive {other}"))
                        .into_diagnostics(),
                );
                i += 1;
            }
            None => i += 1,
        }
    }

    if !diagnostics.is_empty() {
        Err(DiagnosticReport::from_diagnostics(diagnostics))
    } else {
        Err(parse_error(
            &lines[start],
            "statement block missing closing brace",
        ))
    }
}

fn is_shared_standard_move_statement(line: &str) -> bool {
    matches!(
        puzzle_authoring::rule_statement_surface(line),
        Ok(puzzle_authoring::RuleStatementSurface::RuleLine(
            puzzle_authoring::RuleLineSurface::StandardStep(
                puzzle_authoring::StandardRuleStepSurface::Move
            )
        ))
    )
}

fn is_shared_rule_call_statement(line: &str, expected_name: &str) -> bool {
    matches!(
        puzzle_authoring::rule_statement_surface(line),
        Ok(puzzle_authoring::RuleStatementSurface::Call { name }) if name == expected_name
    )
}

fn rewrite_with_source_line_number(
    mut rewrite: OrientedRewriteAst,
    source_line_number: Option<usize>,
) -> OrientedRewriteAst {
    rewrite.source_line_number = source_line_number;
    rewrite
}

#[allow(clippy::too_many_arguments)]
fn parse_display_statement(
    line: &str,
    source_line_number: Option<usize>,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<StatementAst, DiagnosticReport> {
    let rest = line
        .strip_prefix("display")
        .ok_or_else(|| parse_error(line, "display statement must start with display"))?
        .trim_start();
    if rest.is_empty() {
        return Err(parse_error(
            line,
            "display statement must be: display <rule> or display <rewrite>",
        ));
    }

    let tokens = split_header_tokens(rest);
    if tokens.len() == 1 && (is_qualified_identifier(tokens[0]) || is_display_role_token(tokens[0]))
    {
        return Ok(StatementAst::DisplayCall {
            name: tokens[0].to_string(),
            source_line: line.to_string(),
        });
    }

    let rewrite = match tokens.first().copied() {
        Some("once") => parse_application_prefixed_rewrite_statement(
            rest,
            "once",
            RuleApplication::Once,
            rule_params,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            &HashMap::new(),
        )?,
        Some("once_all") => parse_application_prefixed_rewrite_statement(
            rest,
            "once_all",
            RuleApplication::OnceAll,
            rule_params,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            &HashMap::new(),
        )?,
        Some("once_per_level") => parse_application_prefixed_rewrite_statement(
            rest,
            "once_per_level",
            RuleApplication::OncePerLevel,
            rule_params,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            &HashMap::new(),
        )?,
        Some("repeat") => parse_application_prefixed_rewrite_statement(
            rest,
            "repeat",
            RuleApplication::UntilStable,
            rule_params,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            global_names,
        )?,
        Some(first) if is_oriented_rewrite_line(rest, first) => parse_oriented_rewrite_statement(
            rest,
            first,
            None,
            rule_params,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            global_names,
        )?,
        Some(_) if rest.starts_with('[') => parse_neutral_rewrite_statement(
            rest,
            None,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            global_names,
        )?,
        Some(other) => {
            return Err(parse_error(
                line,
                &format!("unknown display statement directive {other}"),
            ));
        }
        None => unreachable!("empty display statement already rejected"),
    };

    Ok(StatementAst::DisplayRewrite(
        rewrite_with_source_line_number(rewrite, source_line_number),
    ))
}

fn validate_display_hook_statements(statements: &[StatementAst]) -> Result<(), DiagnosticReport> {
    for statement in statements {
        match statement {
            StatementAst::DisplayCall { .. }
            | StatementAst::DisplayRewrite(_)
            | StatementAst::DisplayBlock(_) => {}
            StatementAst::Conditional {
                then_statements,
                else_statements,
                ..
            } => {
                validate_display_hook_statements(then_statements)?;
                validate_display_hook_statements(else_statements)?;
            }
            StatementAst::Block { statements, .. } => {
                validate_display_hook_statements(statements)?;
            }
            StatementAst::RepeatUntil { statements, .. } => {
                validate_display_hook_statements(statements)?;
            }
            StatementAst::Fix { statements, .. } => {
                validate_display_hook_statements(statements)?;
            }
            StatementAst::If {
                then_statements,
                else_statements,
                ..
            } => {
                validate_display_hook_statements(then_statements)?;
                validate_display_hook_statements(else_statements)?;
            }
            StatementAst::Call { .. } | StatementAst::Effect { .. } | StatementAst::Rewrite(_) => {
                return Err(DiagnosticReport::error(
                    "on_display can only contain display statements".to_string(),
                ));
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn parse_conditional_call_statement(
    line: &str,
    orientation_token: Option<&str>,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<Option<StatementAst>, DiagnosticReport> {
    let Some((left, right)) = line.split_once("->") else {
        return Ok(None);
    };
    let rule_name = right.trim();
    if !is_qualified_identifier(rule_name) {
        return Ok(None);
    }
    if is_builtin_rewrite_effect_text(rule_name) {
        return Ok(None);
    }

    let (pattern, orientation) = if let Some(orientation_token) = orientation_token {
        let (orientation, pattern) =
            parse_oriented_rewrite_prefix(left, orientation_token, rule_params)?;
        (pattern, Some(orientation))
    } else {
        (left.trim(), None)
    };
    let condition = parse_pattern_condition(
        PatternPredicateAst::Some,
        pattern,
        line,
        orientation,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;

    Ok(Some(StatementAst::Conditional {
        source_line: line.to_string(),
        condition,
        then_statements: vec![StatementAst::Call {
            name: rule_name.to_string(),
            source_line: line.to_string(),
        }],
        else_statements: Vec::new(),
    }))
}

#[allow(clippy::too_many_arguments)]
fn parse_pattern_if_header<'a>(
    line: &'a str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<Option<(PatternConditionAst, &'a str)>, DiagnosticReport> {
    let Some(rest) = line.strip_prefix("if") else {
        return Ok(None);
    };
    let rest = rest.trim_start();
    let Some((predicate, after_keyword)) = parse_pattern_predicate_keyword(rest) else {
        return Ok(None);
    };
    let after_keyword = after_keyword.trim_start();
    let Some(after_open) = after_keyword.strip_prefix('(') else {
        return Err(parse_error(line, "pattern condition must use parentheses"));
    };
    let close_index = matching_close_paren(after_open)
        .ok_or_else(|| parse_error(line, "pattern condition missing )"))?;
    let pattern = after_open[..close_index].trim();
    let trailing = after_open[close_index + 1..].trim();
    let condition = parse_pattern_condition(
        predicate,
        pattern,
        line,
        None,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;
    Ok(Some((condition, trailing)))
}

fn parse_pattern_predicate_keyword(value: &str) -> Option<(PatternPredicateAst, &str)> {
    if let Some(rest) = value.strip_prefix("some") {
        return Some((PatternPredicateAst::Some, rest));
    }
    if let Some(rest) = value.strip_prefix("none") {
        return Some((PatternPredicateAst::None, rest));
    }
    None
}

fn matching_close_paren(value: &str) -> Option<usize> {
    let mut depth = 1_u16;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn parse_pattern_condition(
    predicate: PatternPredicateAst,
    pattern: &str,
    line: &str,
    orientation: Option<OrientationExpr>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    _global_names: &HashMap<String, GlobalId>,
) -> Result<PatternConditionAst, DiagnosticReport> {
    let Some((pattern_orientation, pattern)) = split_oriented_pattern_arg(pattern, line)? else {
        return Err(parse_error(
            line,
            "pattern condition must contain a pattern",
        ));
    };
    if !matches!(pattern_orientation, OrientationExpr::Neutral) && orientation.is_some() {
        return Err(parse_error(
            line,
            "pattern condition cannot combine multiple orientation prefixes",
        ));
    }
    let orientation = if matches!(pattern_orientation, OrientationExpr::Neutral) {
        orientation.unwrap_or(OrientationExpr::Neutral)
    } else {
        pattern_orientation
    };
    Ok(PatternConditionAst {
        predicate,
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
    })
}

fn normalize_embedded_direction_marker(pattern: &str) -> (Option<OrientationExpr>, String) {
    let trimmed = pattern.trim();
    let Some(after_open) = trimmed.strip_prefix('[') else {
        return (None, trimmed.to_string());
    };
    let rest = after_open.trim_start();
    let Some(marker) = rest.chars().next() else {
        return (None, trimmed.to_string());
    };
    let Some(direction_name) = embedded_direction_name(marker) else {
        return (None, trimmed.to_string());
    };
    let marker_len = marker.len_utf8();
    let after_marker = &rest[marker_len..];
    if !after_marker.chars().next().is_some_and(char::is_whitespace) {
        return (None, trimmed.to_string());
    }
    let normalized = format!("[{}", after_marker.trim_start());
    (
        Some(OrientationExpr::Fixed(DirectionName(
            direction_name.to_string(),
        ))),
        normalized,
    )
}

fn embedded_direction_name(marker: char) -> Option<&'static str> {
    match marker {
        '>' => Some("right"),
        '<' => Some("left"),
        '^' => Some("up"),
        'v' => Some("down"),
        _ => None,
    }
}

fn is_oriented_rewrite_line(line: &str, orientation_token: &str) -> bool {
    if !line.trim_start().starts_with(orientation_token) {
        return false;
    }
    matches!(
        puzzle_authoring::rule_line_surface(line),
        Ok(puzzle_authoring::RuleLineSurface::InputRewrite { .. })
            | Ok(puzzle_authoring::RuleLineSurface::OrientedRewrite { .. })
    )
}

fn parse_oriented_rewrite_statement(
    line: &str,
    orientation_token: &str,
    application: Option<RuleApplication>,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<OrientedRewriteAst, DiagnosticReport> {
    if !line.trim_start().starts_with(orientation_token) {
        return Err(parse_error(line, "missing oriented rewrite"));
    }
    let surface = puzzle_authoring::rule_line_surface(line)
        .map_err(|error| parse_error(line, error.message()))?;
    let parsed = parse_rule_line_rewrite_statement(
        line,
        surface,
        rule_params,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;
    if parsed.application.is_some() {
        return Err(parse_error(line, "unexpected application-prefixed rewrite"));
    }
    Ok(OrientedRewriteAst {
        application,
        ..parsed
    })
}

fn parse_neutral_rewrite_statement(
    line: &str,
    application: Option<RuleApplication>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<OrientedRewriteAst, DiagnosticReport> {
    let surface = puzzle_authoring::rule_line_surface(line)
        .map_err(|error| parse_error(line, error.message()))?;
    let parsed = parse_rule_line_rewrite_statement(
        line,
        surface,
        &[],
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;
    if !matches!(parsed.orientation, OrientationExpr::Neutral) || parsed.application.is_some() {
        return Err(parse_error(line, "expected a neutral rewrite"));
    }
    Ok(OrientedRewriteAst {
        application,
        ..parsed
    })
}

fn parse_application_prefixed_rewrite_statement(
    line: &str,
    prefix: &str,
    application: RuleApplication,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<OrientedRewriteAst, DiagnosticReport> {
    line.strip_prefix(prefix)
        .ok_or_else(|| parse_error(line, "missing application-prefixed rewrite"))?;
    let surface = puzzle_authoring::rule_line_surface(line)
        .map_err(|error| parse_error(line, error.message()))?;
    let parsed = parse_rule_line_rewrite_statement(
        line,
        surface,
        rule_params,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;
    if parsed.application != Some(application) {
        return Err(parse_error(
            line,
            "application prefix must be followed by a rewrite",
        ));
    }
    Ok(parsed)
}

#[allow(clippy::too_many_arguments)]
fn parse_rule_line_rewrite_statement(
    line: &str,
    surface: puzzle_authoring::RuleLineSurface<'_>,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<OrientedRewriteAst, DiagnosticReport> {
    let (orientation, application, rewrite) = match surface {
        puzzle_authoring::RuleLineSurface::InputRewrite {
            application,
            surface,
        } => {
            if let Some(axis) = surface.orientation {
                validate_identifier(axis, line, "input orientation")?;
            }
            (
                OrientationExpr::InputSet(surface.orientation.unwrap_or("directions").to_string()),
                application.map(rule_application_from_surface),
                surface.rewrite,
            )
        }
        puzzle_authoring::RuleLineSurface::NeutralRewrite {
            application,
            rewrite,
        } => (
            OrientationExpr::Neutral,
            application.map(rule_application_from_surface),
            rewrite,
        ),
        puzzle_authoring::RuleLineSurface::OrientedRewrite {
            application,
            orientation,
            rewrite,
        } => (
            parse_statement_orientation_expr(orientation, rule_params),
            application.map(rule_application_from_surface),
            rewrite,
        ),
        puzzle_authoring::RuleLineSurface::StandardStep(_) => {
            return Err(parse_error(line, "expected a rewrite statement"));
        }
    };
    let (before, after, effects, after_effects, after_call) = parse_inline_rewrite(
        rewrite,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;

    Ok(OrientedRewriteAst {
        source_line: line.to_string(),
        source_line_number: None,
        orientation,
        application,
        before,
        after,
        effects,
        after_effects,
        after_call,
    })
}

fn rule_application_from_surface(
    application: puzzle_authoring::RuleApplicationSurface,
) -> RuleApplication {
    match application {
        puzzle_authoring::RuleApplicationSurface::Once => RuleApplication::Once,
        puzzle_authoring::RuleApplicationSurface::OnceAll => RuleApplication::OnceAll,
        puzzle_authoring::RuleApplicationSurface::OncePerLevel => RuleApplication::OncePerLevel,
        puzzle_authoring::RuleApplicationSurface::Repeat => RuleApplication::UntilStable,
    }
}

fn parse_oriented_rewrite_prefix<'a>(
    line: &'a str,
    orientation_token: &str,
    rule_params: &[String],
) -> Result<(OrientationExpr, &'a str), DiagnosticReport> {
    let rest = line
        .strip_prefix(orientation_token)
        .map(str::trim_start)
        .ok_or_else(|| parse_error(line, "missing oriented rewrite"))?;
    if orientation_token == "input" {
        let surface = puzzle_authoring::input_rewrite_surface(line)
            .map_err(|error| parse_error(line, error.message()))?
            .ok_or_else(|| parse_error(line, "missing input-oriented rewrite"))?;
        if let Some(axis) = surface.orientation {
            validate_identifier(axis, line, "input orientation")?;
        }
        return Ok((
            OrientationExpr::InputSet(surface.orientation.unwrap_or("directions").to_string()),
            surface.rewrite,
        ));
    }
    if !rest.starts_with('[') {
        return Err(parse_error(line, "missing oriented rewrite"));
    }
    Ok((
        parse_statement_orientation_expr(orientation_token, rule_params),
        rest,
    ))
}

fn parse_statement_orientation_expr(token: &str, rule_params: &[String]) -> OrientationExpr {
    if token == "input" || rule_params.iter().any(|param| param == token) {
        return OrientationExpr::Input;
    }

    OrientationExpr::Fixed(DirectionName(token.to_string()))
}

#[allow(clippy::too_many_arguments)]
fn parse_statement_condition(
    condition: &str,
    line: &str,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionAst, DiagnosticReport> {
    let condition = condition.trim();
    if let Some((_, named_condition)) = named_conditions.get(condition) {
        return Ok(named_condition.clone());
    }
    parse_condition_expr(
        condition,
        line,
        input_names,
        global_names,
        condition_names,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )
}

fn parse_condition_expr(
    condition: &str,
    line: &str,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionAst, DiagnosticReport> {
    let or_parts = split_condition_keyword(condition, "or");
    if or_parts.len() > 1 {
        return Ok(ConditionAst::Any(
            or_parts
                .into_iter()
                .map(|part| {
                    parse_condition_expr(
                        &part,
                        line,
                        input_names,
                        global_names,
                        condition_names,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    )
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?,
        ));
    }

    let and_parts = split_condition_keyword(condition, "and");
    if and_parts.len() > 1 {
        return Ok(ConditionAst::All(
            and_parts
                .into_iter()
                .map(|part| {
                    parse_condition_expr(
                        &part,
                        line,
                        input_names,
                        global_names,
                        condition_names,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    )
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?,
        ));
    }

    parse_condition_atom(
        condition.trim(),
        line,
        input_names,
        global_names,
        condition_names,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )
}

fn split_condition_keyword(condition: &str, keyword: &str) -> Vec<String> {
    condition
        .split_whitespace()
        .collect::<Vec<_>>()
        .split(|token| *token == keyword)
        .map(|part| part.join(" "))
        .filter(|part| !part.trim().is_empty())
        .collect()
}

fn parse_condition_atom(
    condition: &str,
    line: &str,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionAst, DiagnosticReport> {
    let tokens = condition.split_whitespace().collect::<Vec<_>>();
    if let ["input", "in", axis] = tokens.as_slice() {
        return Ok(ConditionAst::InputIn((*axis).to_string()));
    }

    if let Some(pattern) = condition.strip_prefix("some ") {
        if let Some(pattern) = parse_condition_pattern_arg(
            pattern.trim(),
            line,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
        )? {
            return Ok(ConditionAst::InlineConditionNonZero(
                ConditionValueAst::ExistsMatches(pattern),
            ));
        }
    }
    if let Some(pattern) = condition.strip_prefix("no ") {
        if let Some(pattern) = parse_condition_pattern_arg(
            pattern.trim(),
            line,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
        )? {
            return Ok(ConditionAst::InlineConditionNonZero(
                ConditionValueAst::NoneMatches(pattern),
            ));
        }
    }

    if let Some((left, op, right)) = split_comparison(condition) {
        let left = left.trim();
        let right = right.trim();
        if left == "input" {
            if op != ComparisonOp::Eq {
                return Err(parse_error(line, "input condition only supports =="));
            }
            if input_names.contains_key(right) || is_identifier(right) {
                return Ok(ConditionAst::InputIs(right.to_string()));
            }
            return Err(parse_error(line, "unknown input in condition"));
        }

        let value = parse_global_value(right, line)?;
        if global_names.contains_key(left) {
            return Ok(match op {
                ComparisonOp::Eq => ConditionAst::GlobalEquals {
                    name: left.to_string(),
                    value,
                },
                op => ConditionAst::GlobalCompare {
                    name: left.to_string(),
                    op,
                    value,
                },
            });
        }
        if condition_names.contains_key(left) {
            return Ok(match op {
                ComparisonOp::Eq => ConditionAst::ConditionEquals {
                    name: left.to_string(),
                    value,
                },
                op => ConditionAst::ConditionCompare {
                    name: left.to_string(),
                    op,
                    value,
                },
            });
        }
        if left.contains('(') {
            return Ok(match op {
                ComparisonOp::Eq => ConditionAst::InlineConditionValueEquals {
                    kind: parse_condition_value_expr(
                        left,
                        line,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    )?,
                    value,
                },
                op => ConditionAst::InlineConditionCompare {
                    kind: parse_condition_value_expr(
                        left,
                        line,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    )?,
                    op,
                    value,
                },
            });
        }
        return Err(parse_error(line, "unknown value in condition"));
    }

    if condition_names.contains_key(condition) {
        return Ok(ConditionAst::ConditionNonZero(condition.to_string()));
    }
    if global_names.contains_key(condition) {
        return Ok(ConditionAst::GlobalCompare {
            name: condition.to_string(),
            op: ComparisonOp::NotEq,
            value: 0,
        });
    }
    if condition.contains('(') {
        return Ok(ConditionAst::InlineConditionNonZero(
            parse_condition_value_expr(
                condition,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
            )?,
        ));
    }

    Err(parse_error(line, "unsupported condition"))
}

fn is_input_effect_statement(line: &str) -> bool {
    let Some((left, _)) = line.split_once("->") else {
        return false;
    };
    is_identifier(left.trim())
}

fn split_comparison(condition: &str) -> Option<(&str, ComparisonOp, &str)> {
    for (token, op) in [
        ("==", ComparisonOp::Eq),
        ("!=", ComparisonOp::NotEq),
        (">=", ComparisonOp::GreaterEq),
        ("<=", ComparisonOp::LessEq),
        (">", ComparisonOp::Greater),
        ("<", ComparisonOp::Less),
    ] {
        if let Some((left, right)) = condition.split_once(token) {
            return Some((left, op, right));
        }
    }
    None
}
