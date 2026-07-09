fn parse_level_block(
    lines: &[String],
    start: usize,
    existing_count: usize,
) -> Result<(LevelBlock, usize), DiagnosticReport> {
    parse_level_block_with_default_puzzle(lines, start, existing_count, None)
}

fn parse_level_block_with_default_puzzle(
    lines: &[String],
    start: usize,
    existing_count: usize,
    default_puzzle: Option<&str>,
) -> Result<(LevelBlock, usize), DiagnosticReport> {
    let level_name = parse_level_header_name_or_auto(
        &lines[start],
        puzzle_authoring::unnamed_level_name(existing_count),
    )?;
    parse_named_level_body(
        lines,
        start,
        level_name,
        &LevelsHeader {
            pack: None,
            puzzle: default_puzzle.map(str::to_string),
        },
    )
}

#[derive(Clone, Debug)]
struct PendingLevelBlock {
    start: usize,
    default_puzzle: Option<String>,
    kind: PendingLevelBlockKind,
}

impl PendingLevelBlock {
    fn levels(start: usize, default_puzzle: Option<String>) -> Self {
        Self {
            start,
            default_puzzle,
            kind: PendingLevelBlockKind::Levels,
        }
    }

    fn level(start: usize, default_puzzle: Option<String>) -> Self {
        Self {
            start,
            default_puzzle,
            kind: PendingLevelBlockKind::Level,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingLevelBlockKind {
    Levels,
    Level,
}

fn parse_pending_level_block(
    lines: &[String],
    pending: &PendingLevelBlock,
    level_blocks: &mut Vec<LevelBlock>,
    catalog: &mut Catalog,
    render_overlays: &mut OverlayDefs,
    empty_char: &mut Option<char>,
) -> Result<usize, DiagnosticReport> {
    match pending.kind {
        PendingLevelBlockKind::Levels => parse_levels_block(
            lines,
            pending.start,
            level_blocks,
            catalog,
            render_overlays,
            empty_char,
            pending.default_puzzle.as_deref(),
        ),
        PendingLevelBlockKind::Level => {
            let (level, next_i) = parse_level_block_with_default_puzzle(
                lines,
                pending.start,
                level_blocks.len(),
                pending.default_puzzle.as_deref(),
            )?;
            level_blocks.push(level);
            Ok(next_i)
        }
    }
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
                let auto_name = puzzle_authoring::namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    level_blocks.len(),
                    namespace_count,
                );
                let level_name = parse_level_header_name_or_auto(&lines[i], auto_name)?;
                let (level, next_i) = if puzzle_authoring::is_braced_level_header(&lines[i]) {
                    parse_named_level_body(lines, i, level_name, &header)?
                } else {
                    parse_unbraced_level_body(lines, i + 1, level_name, &header)?
                };
                level_blocks.push(level);
                i = next_i;
            }
            ["{"] => {
                namespace_count += 1;
                let name = puzzle_authoring::namespaced_unnamed_level_name(
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
                let name = puzzle_authoring::namespaced_unnamed_level_name(
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
    puzzle_authoring::parse_level_header_name_or_auto(line, auto_name)
        .map_err(|error| parse_error(line, error.message()))
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
            for_expansion_values(sources, &value_sets, &catalog.numeric_variable_defaults, line)?;
        validate_identifier(binding, line, "expansion binding")?;
        let (body_lines, next_i) = collect_statement_block_lines(lines, start + 1, line)?;
        for value in values {
            let expanded_lines = expand_for_binding_lines(
                &body_lines,
                binding,
                &value,
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
    animation: &mut AnimationDef,
) -> Result<usize, DiagnosticReport> {
    let (node, next_i) = authoring_grammar::parse_placed_authoring_node(
        lines,
        start,
        authoring_grammar::AuthoringKind::Root,
        "render block missing closing brace",
    )?;
    if node.kind != authoring_grammar::AuthoringKind::PuzzleRenderConfig {
        return Err(parse_error(&lines[start], "render header must be: render"));
    }
    let mut parsed = render.clone();
    let mut parsed_animation = animation.clone();
    for definition in &node.definition_rows {
        match definition.key.as_str() {
            "cell_size" => {
                if definition.op != Some(authoring_grammar::AuthoringDefinitionOp::Equals) {
                    return Err(parse_error(
                        &definition.source_line,
                        "cell_size directive must be: cell_size = <pixels>",
                    ));
                }
                let Some(value) = definition.single_value() else {
                    return Err(parse_error(
                        &definition.source_line,
                        "cell_size directive must be: cell_size = <pixels>",
                    ));
                };
                parsed.cell_size = Some(parse_puzzle_render_cell_size(value, &definition.source_line)?);
            }
            "tween_duration" => {
                if definition.op != Some(authoring_grammar::AuthoringDefinitionOp::Equals) {
                    return Err(parse_error(
                        &definition.source_line,
                        "tween_duration must be: tween_duration = <duration>",
                    ));
                }
                parsed_animation.tween.enabled = true;
                apply_tween_duration_definition(definition, &mut parsed_animation.tween)?;
            }
            other => {
                return Err(parse_error(
                    &definition.source_line,
                    &format!("unknown render setting {other}"),
                ));
            }
        }
    }
    for child in &node.children {
        match child.kind {
            authoring_grammar::AuthoringKind::PuzzleRenderGridConfig => {
                apply_puzzle_render_grid_node(child, &mut parsed.grid)?;
            }
            authoring_grammar::AuthoringKind::TweenConfig => {
                parsed_animation.tween.enabled = true;
                apply_animation_tween_node(child, &mut parsed_animation.tween)?;
            }
            _ => {
                return Err(parse_error(
                    &child.source_line,
                    &format!("unknown render directive {}", child.surface),
                ));
            }
        }
    }
    *render = parsed;
    *animation = parsed_animation;
    Ok(next_i)
}

fn apply_puzzle_render_grid_node(
    node: &authoring_grammar::AuthoringNode,
    grid: &mut PuzzleGridRenderDef,
) -> Result<(), DiagnosticReport> {
    for definition in &node.definition_rows {
        match definition.key.as_str() {
            "type" => {
                if definition.op != Some(authoring_grammar::AuthoringDefinitionOp::Equals) {
                    return Err(parse_error(
                        &definition.source_line,
                        "grid type directive must be: type = \"occupied_cells\" or type = \"all_cells\"",
                    ));
                }
                let Some(value) = definition.single_value() else {
                    return Err(parse_error(
                        &definition.source_line,
                        "grid type directive must be: type = \"occupied_cells\" or type = \"all_cells\"",
                    ));
                };
                apply_puzzle_render_grid_type(value, &definition.source_line, grid)?;
            }
            other => {
                return Err(parse_error(
                    &definition.source_line,
                    &format!("unknown grid setting {other}"),
                ));
            }
        }
    }
    Ok(())
}

fn apply_puzzle_render_grid_type(
    value: &str,
    line: &str,
    grid: &mut PuzzleGridRenderDef,
) -> Result<(), DiagnosticReport> {
    let spec = authoring_grammar::authoring_definition_spec(
        authoring_grammar::AuthoringKind::PuzzleRenderGridConfig,
        "type",
    )
    .expect("grid type definition exists");
    match authoring_grammar::definition_value_literal(spec, value, line)? {
        "occupied_cells" => {
            grid.occupied_cells = true;
            grid.all_cells = false;
        }
        "all_cells" => {
            grid.occupied_cells = false;
            grid.all_cells = true;
        }
        other => {
            return Err(parse_error(
                line,
                &format!("unknown grid render type {other}"),
            ));
        }
    }
    Ok(())
}

fn parse_puzzle_render_cell_size(value: &str, line: &str) -> Result<u16, DiagnosticReport> {
    let size = value
        .parse::<u16>()
        .map_err(|_| parse_error(line, "cell_size must be an integer from 1 to 256"))?;
    if !(1..=256).contains(&size) {
        return Err(parse_error(
            line,
            "cell_size must be an integer from 1 to 256",
        ));
    }
    Ok(size)
}

fn parse_render_animation_block(
    lines: &[String],
    start: usize,
    animation: &mut AnimationDef,
) -> Result<usize, DiagnosticReport> {
    let (node, next_i) = authoring_grammar::parse_placed_authoring_node(
        lines,
        start,
        authoring_grammar::AuthoringKind::Root,
        "render block missing closing brace",
    )?;
    if node.kind != authoring_grammar::AuthoringKind::PuzzleRenderConfig {
        return Err(parse_error(&lines[start], "render header must be: render"));
    }
    let mut parsed = animation.clone();
    for definition in &node.definition_rows {
        match definition.key.as_str() {
            "tween_duration" => {
                if definition.op != Some(authoring_grammar::AuthoringDefinitionOp::Equals) {
                    return Err(parse_error(
                        &definition.source_line,
                        "tween_duration must be: tween_duration = <duration>",
                    ));
                }
                parsed.tween.enabled = true;
                apply_tween_duration_definition(definition, &mut parsed.tween)?;
            }
            other => {
                return Err(parse_error(
                    &definition.source_line,
                    &format!("unknown render setting {other}"),
                ));
            }
        }
    }
    for child in &node.children {
        match child.kind {
            authoring_grammar::AuthoringKind::TweenConfig => {
                parsed.tween.enabled = true;
                apply_animation_tween_node(child, &mut parsed.tween)?;
            }
            _ => {
                return Err(parse_error(
                    &child.source_line,
                    &format!("unknown render directive {}", child.surface),
                ));
            }
        }
    }
    *animation = parsed;
    Ok(next_i)
}

fn apply_animation_tween_node(
    node: &authoring_grammar::AuthoringNode,
    tween: &mut TweenAnimationDef,
) -> Result<(), DiagnosticReport> {
    for definition in &node.definition_rows {
        match definition.key.as_str() {
            "duration" => {
                apply_tween_duration_definition(definition, tween)?;
            }
            other => {
                return Err(parse_error(
                    &definition.source_line,
                    &format!("unknown tween setting {other}"),
                ));
            }
        }
    }
    Ok(())
}

fn apply_tween_duration_definition(
    definition: &authoring_grammar::AuthoringDefinitionRow,
    tween: &mut TweenAnimationDef,
) -> Result<(), DiagnosticReport> {
    let Some(value) = definition.single_value() else {
        return Err(parse_error(
            &definition.source_line,
            "tween duration must be one value",
        ));
    };
    if value.is_empty() {
        return Err(parse_error(
            &definition.source_line,
            "tween duration must not be empty",
        ));
    }
    tween.interval_ms = parse_animation_duration_ms(value, &definition.source_line)?;
    Ok(())
}

fn parse_animation_duration_ms(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    let milliseconds = parse_wait_duration_ms(value, line)?;
    if milliseconds == 0 {
        return Err(parse_error(line, "tween duration must be greater than 0"));
    }
    Ok(milliseconds)
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
    if let Ok(condition) = parse_condition_expr(
        line,
        line,
        &catalog.input_names,
        &catalog.variable_names,
        &catalog.condition_names,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.maps,
        &catalog.object_groups,
    ) {
        return Ok(condition);
    }

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

    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        ["all", target, "on", cover] => {
            let expr = format!("none([ {target} no {cover} ])");
            parse_condition_expr(
                &expr,
                line,
                &catalog.input_names,
                &catalog.variable_names,
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
                &catalog.variable_names,
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
                &catalog.variable_names,
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
                &catalog.variable_names,
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
    let mut name_override = None::<String>;
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
        if nested_blocks == 0 && let Some(parsed) = canonical_level_name_override(&lines[i])? {
            if name_override.is_some() {
                return Err(parse_error(&lines[i], "duplicate level name"));
            }
            name_override = Some(parsed);
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
            name: name_override.unwrap_or(name),
            pack: header.pack.clone(),
            puzzle: header.puzzle.clone(),
            lines: level_lines,
        },
        i + 1,
    ))
}

fn canonical_level_name_override(line: &str) -> Result<Option<String>, DiagnosticReport> {
    let Some(row) = crate::authoring_grammar::parse_authoring_definition_row(
        crate::authoring_grammar::AuthoringKind::LevelConfig,
        line,
    )?
    else {
        return Ok(None);
    };
    if row.key != "name" {
        return Ok(None);
    }
    let Some(value) = row.single_value() else {
        return Err(parse_error(line, "level name must have one value"));
    };
    let parsed = puzzle_authoring::parse_quoted_text(value)
        .ok_or_else(|| parse_error(line, "level name must be a quoted string"))?;
    Ok(Some(parsed))
}

fn braced_level_name_override_or(
    lines: &[String],
    start: usize,
    fallback: String,
) -> Result<String, DiagnosticReport> {
    let mut name = None::<String>;
    let mut nested_blocks = 0usize;
    let mut i = start + 1;
    while i < lines.len() {
        if is_block_close_line(&lines[i]) {
            if nested_blocks == 0 {
                return Ok(name.unwrap_or(fallback));
            }
            nested_blocks -= 1;
            i += 1;
            continue;
        }
        if nested_blocks == 0 && let Some(parsed) = canonical_level_name_override(&lines[i])? {
            if name.is_some() {
                return Err(parse_error(&lines[i], "duplicate level name"));
            }
            name = Some(parsed);
            i += 1;
            continue;
        }
        if is_level_body_block(&split_header_tokens(&lines[i])) {
            nested_blocks += 1;
        }
        i += 1;
    }
    Err(parse_error(&lines[start], "level missing closing brace"))
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
                &catalog.variable_names,
                &catalog.numeric_variable_defaults,
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

        if let Some(statement) = parse_level_event_sugar(line)? {
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

fn parse_level_event_sugar(line: &str) -> Result<Option<StatementAst>, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    if !is_level_event_sugar(line, &tokens) {
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
    Ok(Some(StatementAst::Effect {
        source_line: line.to_string(),
        source_line_number: None,
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
    let Some(_) = crate::syntax::legend_block_row_syntax(&tokens, true) else {
        return Err(parse_error(
            line,
            "level legend row must be: <char> = <selector...>",
        ));
    };

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
    let Some(syntax) = crate::syntax::level_legend_directive_syntax(tokens, true) else {
        return Err(parse_error(
            line,
            "level legend must be: legend <char> = <selector...>",
        ));
    };

    let ch = parse_char(tokens.get(1), line, "missing legend char")?;
    if ch == empty_char || tokens[syntax.rhs_start..] == ["empty"] {
        return Err(parse_error(line, "level-local legend cannot define empty"));
    }
    let selector_sets = selector_sets(
        &tokens[syntax.rhs_start..],
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
