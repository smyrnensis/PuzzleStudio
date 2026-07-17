fn parse_level_block_with_default_puzzle(
    lines: &[source::LogicalLine],
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

pub(crate) fn parse_level_resource_entry(
    entry: &crate::model_syntax::PuzzleEntrySyntax,
    existing_count: usize,
    default_puzzle: Option<&str>,
) -> Result<crate::level::LevelResourceSyntax, DiagnosticReport> {
    match entry.directive {
        puzzle_authoring::PuzzleDirectiveSurface::Legend => {
            let mut resource = crate::level::LevelResourceSyntax::default();
            for line in &entry.body {
                if line.text.ends_with('{') {
                    return Err(parse_error(line, "legend accepts rows, not nested blocks"));
                }
                if !line.text.is_empty() {
                    resource
                        .legends
                        .push(parse_level_legend_syntax(line, false)?);
                }
            }
            Ok(resource)
        }
        puzzle_authoring::PuzzleDirectiveSurface::Level => {
            let mut lines = Vec::with_capacity(entry.body.len() + 2);
            lines.push(entry.header.clone());
            lines.extend(entry.body.iter().cloned());
            lines.push(
                entry
                    .closing
                    .clone()
                    .unwrap_or_else(|| source::LogicalLine::new(BLOCK_CLOSE, entry.header.line)),
            );
            let (level, next) =
                parse_level_block_with_default_puzzle(&lines, 0, existing_count, default_puzzle)?;
            if next != lines.len() {
                return Err(parse_error(
                    &entry.header,
                    "level block was not fully consumed",
                ));
            }
            Ok(crate::level::LevelResourceSyntax {
                legends: Vec::new(),
                levels: vec![level],
            })
        }
        puzzle_authoring::PuzzleDirectiveSurface::Levels => {
            parse_levels_resource_entry(entry, existing_count, default_puzzle)
        }
        _ => Err(parse_error(
            &entry.header,
            "canonical level resource must be legend, level, or levels",
        )),
    }
}

fn parse_levels_resource_entry(
    entry: &crate::model_syntax::PuzzleEntrySyntax,
    existing_count: usize,
    default_puzzle: Option<&str>,
) -> Result<crate::level::LevelResourceSyntax, DiagnosticReport> {
    let mut lines = Vec::with_capacity(entry.body.len() + 2);
    lines.push(entry.header.clone());
    lines.extend(entry.body.iter().cloned());
    lines.push(
        entry
            .closing
            .clone()
            .unwrap_or_else(|| source::LogicalLine::new(BLOCK_CLOSE, entry.header.line)),
    );
    let header = parse_levels_header(&lines[0], default_puzzle)?;
    let mut resource = crate::level::LevelResourceSyntax::default();
    let mut namespace_count = 0usize;
    let mut index = 1;
    while index < lines.len() && !is_block_close_line(&lines[index]) {
        let tokens = split_header_tokens(&lines[index]);
        match tokens.as_slice() {
            ["legend"] => {
                let block =
                    puzzle_authoring::collect_row_block_surface(&lines, index + 1, "legend")
                        .map_err(|error| parse_error(&lines[index], error.message()))?;
                for line in &lines[block.body_start..block.body_end] {
                    if !line.text.is_empty() {
                        resource
                            .legends
                            .push(parse_level_legend_syntax(line, false)?);
                    }
                }
                index = block.next_index;
            }
            ["legend", ..] => {
                resource
                    .legends
                    .push(parse_level_legend_syntax(&lines[index], true)?);
                index += 1;
            }
            ["level", ..] => {
                namespace_count += 1;
                let auto_name = puzzle_authoring::namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    existing_count + resource.levels.len(),
                    namespace_count,
                );
                let name = parse_level_header_name_or_auto(&lines[index], auto_name)?;
                let (level, next) = if puzzle_authoring::is_braced_level_header(&lines[index]) {
                    parse_named_level_body(&lines, index, name, &header)?
                } else {
                    parse_unbraced_level_body(
                        &lines,
                        index + 1,
                        name,
                        &header,
                        Some(&lines[index]),
                    )?
                };
                resource.levels.push(level);
                index = next;
            }
            ["{"] => {
                namespace_count += 1;
                let name = puzzle_authoring::namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    existing_count + resource.levels.len(),
                    namespace_count,
                );
                let (level, next) = parse_named_level_body(&lines, index, name, &header)?;
                resource.levels.push(level);
                index = next;
            }
            [] => index += 1,
            _ if lines[index].trim_end().ends_with('{') => {
                return Err(parse_error(
                    &lines[index],
                    "braced level header must be `level <name> {` or `{` for an unnamed level",
                ));
            }
            _ => {
                namespace_count += 1;
                let name = puzzle_authoring::namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    existing_count + resource.levels.len(),
                    namespace_count,
                );
                let (level, next) =
                    parse_unbraced_level_body(&lines, index, name, &header, None)?;
                resource.levels.push(level);
                index = next;
            }
        }
    }
    Ok(resource)
}

fn apply_level_resource_legend(
    syntax: &crate::level::LevelLegendSyntax,
    catalog: &mut Catalog,
    render_overlays: &mut OverlayDefs,
    empty_char: &mut Option<char>,
    recognition: &mut crate::surface::ParserRecognition,
) -> Result<(), DiagnosticReport> {
    recognize_level_resource_legend(syntax, catalog, recognition);
    if syntax.selectors == ["empty"] {
        if syntax.ch != crate::syntax::DEFAULT_LEVEL_EMPTY_CHAR {
            return Err(parse_error(
                &syntax.source,
                "levels use `.` for empty; remove the non-dot empty legend row",
            ));
        }
        *empty_char = Some(crate::syntax::DEFAULT_LEVEL_EMPTY_CHAR);
        return Ok(());
    }
    if syntax.selectors.iter().any(|selector| selector == "empty") {
        return Err(parse_error(
            &syntax.source,
            "empty cannot be mixed with object selectors",
        ));
    }
    let mut tokens = vec!["legend".to_string(), syntax.ch.to_string(), "=".to_string()];
    tokens.extend(syntax.selectors.iter().cloned());
    let tokens = tokens.iter().map(String::as_str).collect::<Vec<_>>();
    parse_legend_directive(
        &tokens,
        &syntax.source,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.maps,
        &catalog.object_groups,
        &mut catalog.render_chars,
        &mut catalog.char_objects,
        render_overlays,
    )?;
    Ok(())
}

fn recognize_level_resource_legend(
    syntax: &crate::level::LevelLegendSyntax,
    catalog: &Catalog,
    recognition: &mut crate::surface::ParserRecognition,
) {
    let legend = syntax.ch.to_string();
    mark_line_token(
        recognition,
        &syntax.source,
        Some(&legend),
        crate::surface::SurfaceSemanticKind::Literal,
    );
    for selector in &syntax.selectors {
        if selector == "empty" {
            mark_line_token(
                recognition,
                &syntax.source,
                Some(selector),
                crate::surface::SurfaceSemanticKind::Literal,
            );
        } else {
            mark_selector_token(recognition, &syntax.source, selector, catalog);
        }
    }
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
    let surface = puzzle_authoring::resource_header_surface(line, "levels")
        .map_err(|error| parse_error(line, error.message()))?;
    Ok(LevelsHeader {
        pack: surface.name.map(str::to_string),
        puzzle: surface.owner.or(default_puzzle).map(str::to_string),
    })
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

fn lower_condition_block_syntax_rows(
    header: &source::LogicalLine,
    lines: &[source::LogicalLine],
    catalog: &Catalog,
    named_conditions: &mut HashMap<String, (String, ConditionAst)>,
) -> Result<(), DiagnosticReport> {
    let header_tokens = split_header_tokens(header);
    let condition_name = header_tokens.first().copied().unwrap_or("win_conditions");
    let combinator = match header_tokens.as_slice() {
        [_] => ConditionBlockCombinator::All,
        [_, "all"] => ConditionBlockCombinator::All,
        [_, "any"] => ConditionBlockCombinator::Any,
        _ => {
            return Err(parse_error(
                header,
                &format!("{condition_name} block must be: {condition_name} [all | any]"),
            ));
        }
    };
    if named_conditions.contains_key(condition_name) {
        return Err(parse_error(
            header,
            &format!("duplicate {condition_name} definition"),
        ));
    }

    let mut conditions = Vec::new();
    let mut descriptions = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        i = parse_condition_block_entry(
            lines,
            i,
            condition_name,
            catalog,
            &mut conditions,
            &mut descriptions,
        )?;
    }
    if conditions.is_empty() {
        return Err(parse_error(
            header,
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
    Ok(())
}

fn parse_condition_block_entry(
    lines: &[source::LogicalLine],
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
        let values = for_expansion_values(
            sources,
            &value_sets,
            &catalog.numeric_variable_defaults,
            line,
        )?;
        validate_identifier(binding, line, "expansion binding")?;
        let (body_lines, next_i) = collect_statement_block_lines(lines, start + 1, line)?;
        for value in values {
            let expanded_lines =
                expand_for_binding_lines(&body_lines, binding, &value, &catalog.maps)?;
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
    descriptions.push(line.to_string());
    conditions.push(condition);
    Ok(start + 1)
}

fn parse_condition_rows(
    lines: &[source::LogicalLine],
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

pub(crate) fn lower_puzzle_render_node(
    node: &authoring_grammar::AuthoringNode,
) -> Result<(PuzzleRenderDef, AnimationDef), DiagnosticReport> {
    let mut render = PuzzleRenderDef::default();
    let mut animation = AnimationDef::default();
    apply_puzzle_render_node(node, &mut render, &mut animation)?;
    Ok((render, animation))
}

fn apply_puzzle_render_node(
    node: &authoring_grammar::AuthoringNode,
    render: &mut PuzzleRenderDef,
    animation: &mut AnimationDef,
) -> Result<(), DiagnosticReport> {
    if node.kind != authoring_grammar::AuthoringKind::PuzzleRenderConfig {
        return Err(parse_error(
            &node.source_line,
            "render header must be: render",
        ));
    }
    let mut parsed = render.clone();
    let mut parsed_animation = animation.clone();
    let mut tween_duration_source_line = None::<String>;
    for definition in &node.definition_rows {
        match definition.key.as_str() {
            "tween_duration" => {
                if definition.op != Some(authoring_grammar::AuthoringDefinitionOp::Equals) {
                    return Err(parse_error(
                        &definition.source_line,
                        "tween_duration must be: tween_duration = <duration>",
                    ));
                }
                tween_duration_source_line = Some(definition.source_line.clone());
                apply_tween_duration_definition(definition, &mut parsed_animation.tween)?;
            }
            "tween" => {
                if definition.op != Some(authoring_grammar::AuthoringDefinitionOp::Equals) {
                    return Err(parse_error(
                        &definition.source_line,
                        "tween must be: tween = true or tween = false",
                    ));
                }
                parsed_animation.tween.enabled = parse_puzzle_render_tween_enabled(definition)?;
            }
            "shade" => {
                parsed.sprite.shade = render_boolean_value(definition)?;
            }
            "shadow" => {
                parsed.shadow = render_boolean_value(definition)?;
            }
            other => {
                return Err(parse_error(
                    &definition.source_line,
                    &format!("unknown render setting {other}"),
                ));
            }
        }
    }
    if let Some(source_line) = tween_duration_source_line
        && !parsed_animation.tween.enabled
    {
        return Err(parse_error(
            &source_line,
            "tween_duration requires tween = true",
        ));
    }
    for child in &node.children {
        match child.kind {
            authoring_grammar::AuthoringKind::PuzzleRenderGridConfig => {
                apply_puzzle_render_grid_node(child, &mut parsed.grid)?;
            }
            authoring_grammar::AuthoringKind::PuzzleRenderCameraConfig => {
                apply_puzzle_render_camera_node(child, &mut parsed.camera)?;
            }
            authoring_grammar::AuthoringKind::PuzzleRenderPixelateConfig => {
                apply_puzzle_render_pixelate_node(child, &mut parsed.pixelate)?;
            }
            authoring_grammar::AuthoringKind::PuzzleRenderViewportConfig => {
                apply_puzzle_render_viewport_node(child, &mut parsed.viewport)?;
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
    Ok(())
}

fn render_definition_value(
    definition: &authoring_grammar::AuthoringDefinitionRow,
) -> Result<&str, DiagnosticReport> {
    if definition.op != Some(authoring_grammar::AuthoringDefinitionOp::Equals) {
        return Err(parse_error(
            &definition.source_line,
            &format!("render setting {} requires `=`", definition.key),
        ));
    }
    definition.single_value().ok_or_else(|| {
        parse_error(
            &definition.source_line,
            &format!("render setting {} requires one value", definition.key),
        )
    })
}

fn render_boolean_value(
    definition: &authoring_grammar::AuthoringDefinitionRow,
) -> Result<bool, DiagnosticReport> {
    let value = render_definition_value(definition)?;
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(parse_error(
            &definition.source_line,
            &format!("{} must be true or false", definition.key),
        )),
    }
}

fn apply_puzzle_render_camera_node(
    node: &authoring_grammar::AuthoringNode,
    camera: &mut CameraSettings3,
) -> Result<(), DiagnosticReport> {
    for definition in &node.definition_rows {
        let value = render_definition_value(definition)?;
        match definition.key.as_str() {
            "yaw" => camera.yaw_degrees = render_degrees(value, definition)?,
            "pitch" => camera.pitch_degrees = render_degrees(value, definition)?,
            "roll" => camera.roll_degrees = render_degrees(value, definition)?,
            "zoom" => camera.zoom_milli = render_zoom_milli(value, definition)?,
            "interactive_look" => camera.interactive_look = render_boolean_value(definition)?,
            "interactive_zoom" => camera.interactive_zoom = render_boolean_value(definition)?,
            other => {
                return Err(parse_error(
                    &definition.source_line,
                    &format!("unknown camera setting: {other}"),
                ));
            }
        }
    }
    Ok(())
}

fn apply_puzzle_render_pixelate_node(
    node: &authoring_grammar::AuthoringNode,
    pixelate: &mut PixelateRenderSettings3,
) -> Result<(), DiagnosticReport> {
    for definition in &node.definition_rows {
        let value = render_definition_value(definition)?;
        match definition.key.as_str() {
            "enabled" => pixelate.enabled = render_boolean_value(definition)?,
            "scale" => pixelate.scale = render_positive_u16(value, "pixelate scale")?,
            "smoothing" => pixelate.smoothing = render_boolean_value(definition)?,
            other => {
                return Err(parse_error(
                    &definition.source_line,
                    &format!("unknown pixelate setting: {other}"),
                ));
            }
        }
    }
    Ok(())
}

fn apply_puzzle_render_viewport_node(
    node: &authoring_grammar::AuthoringNode,
    viewport: &mut ViewportSettings3,
) -> Result<(), DiagnosticReport> {
    for row in &node.rows {
        if row.kind == authoring_grammar::AuthoringRowKind::ViewportFocus {
            viewport.focus = row
                .single_capture("selector")
                .ok_or_else(|| parse_error(&row.source_line, "focus requires one selector"))?
                .to_string();
            continue;
        }
        let values = row
            .captures
            .iter()
            .find(|capture| capture.name == "size")
            .map(|capture| capture.values.as_slice())
            .unwrap_or_default();
        let (width, depth, height) = match values {
            [width, depth] => (width.as_str(), depth.as_str(), None),
            [width, depth, height] => (width.as_str(), depth.as_str(), Some(height.as_str())),
            _ => {
                return Err(parse_error(
                    &row.source_line,
                    "viewport size requires width, depth, and optional height",
                ));
            }
        };
        let (mode, follow, directive) = match row.kind {
            authoring_grammar::AuthoringRowKind::ViewportFlickscreen => {
                (ViewportMode3::Paged, ViewportFollow3::Snap, "flickscreen")
            }
            authoring_grammar::AuthoringRowKind::ViewportZoomscreen => {
                (ViewportMode3::Centered, ViewportFollow3::Snap, "zoomscreen")
            }
            authoring_grammar::AuthoringRowKind::ViewportSmoothscreen => (
                ViewportMode3::Centered,
                ViewportFollow3::Smooth,
                "smoothscreen",
            ),
            _ => return Err(parse_error(&row.source_line, "unknown viewport directive")),
        };
        viewport.mode = mode;
        viewport.follow = follow;
        viewport.framing = Some(ViewportFraming3 {
            width: render_positive_u16(width, &format!("{directive} width"))?,
            depth: render_positive_u16(depth, &format!("{directive} depth"))?,
            height: match height {
                Some("full") | None => ViewportHeight3::Full,
                Some(value) => {
                    ViewportHeight3::Size(render_positive_u16(value, "viewport height")?)
                }
            },
        });
    }
    Ok(())
}

fn render_positive_u16(value: &str, name: &str) -> Result<u16, DiagnosticReport> {
    let value = value
        .parse::<u16>()
        .map_err(|_| DiagnosticReport::error(format!("{name} must be a positive integer")))?;
    if value == 0 {
        return Err(DiagnosticReport::error(format!(
            "{name} must be greater than zero"
        )));
    }
    Ok(value)
}

fn render_degrees(
    value: &str,
    definition: &authoring_grammar::AuthoringDefinitionRow,
) -> Result<i16, DiagnosticReport> {
    parse_render_degrees(value, &definition.key)
        .map_err(|error| parse_error(&definition.source_line, &error))
}

fn render_zoom_milli(
    value: &str,
    definition: &authoring_grammar::AuthoringDefinitionRow,
) -> Result<u16, DiagnosticReport> {
    parse_render_zoom_milli(value, &definition.key)
        .map_err(|error| parse_error(&definition.source_line, &error))
}

pub(crate) fn parse_render_degrees(value: &str, name: &str) -> Result<i16, String> {
    value
        .parse::<i16>()
        .map_err(|_| format!("{name} must be an integer degree value"))
}

pub(crate) fn parse_render_zoom_milli(value: &str, name: &str) -> Result<u16, String> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.chars().all(|ch| ch.is_ascii_digit())
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
        || fraction.len() > 3
    {
        return Err(format!(
            "{name} must be a positive number with at most three decimal places"
        ));
    }
    let whole = whole
        .parse::<u32>()
        .map_err(|_| format!("{name} must be positive"))?;
    let fraction = format!("{fraction:0<3}")
        .parse::<u32>()
        .map_err(|_| format!("{name} must be positive"))?;
    let milli = whole
        .checked_mul(1000)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| format!("{name} is too large"))?;
    if milli == 0 || milli > u32::from(u16::MAX) {
        return Err(format!("{name} must be greater than 0 and not too large"));
    }
    Ok(milli as u16)
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

fn parse_puzzle_render_tween_enabled(
    definition: &authoring_grammar::AuthoringDefinitionRow,
) -> Result<bool, DiagnosticReport> {
    let Some(value) = definition.single_value() else {
        return Err(parse_error(
            &definition.source_line,
            "tween must be one boolean value",
        ));
    };
    let spec = authoring_grammar::authoring_definition_spec(
        authoring_grammar::AuthoringKind::PuzzleRenderConfig,
        "tween",
    )
    .expect("render tween definition exists");
    match authoring_grammar::definition_value_literal(spec, value, &definition.source_line)? {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(parse_error(
            &definition.source_line,
            &format!("unknown tween value {other}"),
        )),
    }
}

fn parse_animation_duration_ms(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    let milliseconds = parse_wait_duration_ms(value, line)?;
    if milliseconds == 0 {
        return Err(parse_error(line, "tween duration must be greater than 0"));
    }
    Ok(milliseconds)
}

pub(crate) fn parse_puzzle_screen_directive(
    line: &source::LogicalLine,
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

pub(crate) fn validate_puzzle_screen(
    puzzle_screen: &PuzzleScreenDef,
    line: &source::LogicalLine,
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
    line: &source::LogicalLine,
    directive: &str,
) -> Result<(u16, u16), DiagnosticReport> {
    let value = line
        .text
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
    let surface = puzzle_authoring::win_condition_row_surface(line)
        .map_err(|error| parse_error(line, error))?;
    match surface {
        puzzle_authoring::WinConditionRowSurface::AllOn { subject, cover } => {
            let subjects = resolve_object_selector(
                subject,
                line,
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
                &HashMap::new(),
            )?
            .alternatives;
            let covers = resolve_object_selector(
                cover,
                line,
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
                &HashMap::new(),
            )?
            .alternatives;
            Ok(ConditionAst::AllObjectsOn { subjects, covers })
        }
        puzzle_authoring::WinConditionRowSurface::SomeOn { subject, cover } => {
            let expr = format!("exists([ {subject} {cover} ])");
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
        puzzle_authoring::WinConditionRowSurface::Query {
            quantifier,
            argument,
        } => {
            if let Some(pattern) = parse_condition_pattern_arg(
                argument,
                line,
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
            )? {
                return Ok(ConditionAst::InlineConditionNonZero(match quantifier {
                    puzzle_authoring::WinConditionQuantifier::Exists => {
                        ConditionValueAst::ExistsMatches(pattern)
                    }
                    puzzle_authoring::WinConditionQuantifier::None => {
                        ConditionValueAst::NoneMatches(pattern)
                    }
                }));
            }
            let function = match quantifier {
                puzzle_authoring::WinConditionQuantifier::Exists => "exists",
                puzzle_authoring::WinConditionQuantifier::None => "none",
            };
            let expr = format!("{function}({argument})");
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
        puzzle_authoring::WinConditionRowSurface::Expression(expression) => {
            parse_condition_expr(
                expression,
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
            .map_err(|_| {
                parse_error(
                    line,
                    &format!(
                        "{condition_name} row must be a condition expression, all <object> on <object>, some/no [pattern], some <object> on <object>, or some/no <object>"
                    ),
                )
            })
        }
    }
}

fn parse_named_level_body(
    lines: &[source::LogicalLine],
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
        if nested_blocks == 0
            && let Some(parsed) = canonical_level_name_override(&lines[i])?
        {
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

    let name = name_override.unwrap_or(name);
    let source_name = (!matches!(split_header_tokens(&lines[start]).as_slice(), ["{"]))
        .then(|| name.clone())
        .unwrap_or_default();
    let (source_span, body_span) = braced_level_source_spans(lines, start, i);
    Ok((
        canonicalize_level_block(LevelBlock {
            name,
            source_name,
            source_span,
            body_span,
            pack: header.pack.clone(),
            puzzle: header.puzzle.clone(),
            lines: level_lines,
            legends: Vec::new(),
            on_level_start: Vec::new(),
            on_level_clear: Vec::new(),
            rules_before: None,
            rules_after: None,
            level_start_effect_rows: Vec::new(),
            level_clear_effect_rows: Vec::new(),
        })?,
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

fn parse_unbraced_level_body(
    lines: &[source::LogicalLine],
    start: usize,
    name: String,
    header: &LevelsHeader,
    source_header: Option<&source::LogicalLine>,
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

    let (source_span, body_span) = unbraced_level_source_spans(source_header, &level_lines);
    let source_name = source_header.map_or_else(String::new, |_| name.clone());
    Ok((
        canonicalize_level_block(LevelBlock {
            name,
            source_name,
            source_span,
            body_span,
            pack: header.pack.clone(),
            puzzle: header.puzzle.clone(),
            lines: level_lines,
            legends: Vec::new(),
            on_level_start: Vec::new(),
            on_level_clear: Vec::new(),
            rules_before: None,
            rules_after: None,
            level_start_effect_rows: Vec::new(),
            level_clear_effect_rows: Vec::new(),
        })?,
        i,
    ))
}

fn braced_level_source_spans(
    lines: &[source::LogicalLine],
    header_index: usize,
    closing_index: usize,
) -> (crate::surface::SourceSpan, crate::surface::SourceSpan) {
    let header = &lines[header_index];
    let closing = &lines[closing_index];
    let start = header.tokens.first().map_or(0, |token| token.start);
    let header_end = header.tokens.last().map_or(start, |token| token.end);
    let body_start = header
        .tokens
        .iter()
        .find(|token| token.text == "{")
        .map_or(header_end, |token| token.end);
    let body_end = closing.tokens.first().map_or_else(
        || {
            lines[header_index + 1..closing_index]
                .iter()
                .rev()
                .find_map(|line| line.tokens.last().map(|token| token.end))
                .unwrap_or(body_start)
        },
        |token| token.start,
    );
    let end = closing
        .tokens
        .last()
        .map_or(body_end, |token| token.end);
    (
        crate::surface::SourceSpan { start, end },
        crate::surface::SourceSpan {
            start: body_start,
            end: body_end,
        },
    )
}

fn unbraced_level_source_spans(
    header: Option<&source::LogicalLine>,
    body: &[source::LogicalLine],
) -> (crate::surface::SourceSpan, crate::surface::SourceSpan) {
    let body_start = body
        .first()
        .and_then(|line| line.tokens.first())
        .map_or(0, |token| token.start);
    let body_end = body
        .iter()
        .rev()
        .find_map(|line| line.tokens.last().map(|token| token.end))
        .unwrap_or(body_start);
    let start = header
        .and_then(|line| line.tokens.first())
        .map_or(body_start, |token| token.start);
    (
        crate::surface::SourceSpan {
            start,
            end: body_end,
        },
        crate::surface::SourceSpan {
            start: body_start,
            end: body_end,
        },
    )
}

fn is_level_body_block(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["legend"]
            | ["on_level_start"]
            | ["on_level_clear"]
            | ["rules"]
            | ["rules", "before"]
            | ["rules", "after"]
    )
}

fn canonicalize_level_block(mut level: LevelBlock) -> Result<LevelBlock, DiagnosticReport> {
    let source_lines = std::mem::take(&mut level.lines);
    let mut saw_map_row = false;
    let mut index = 0;
    while index < source_lines.len() {
        let line = &source_lines[index];
        let tokens = split_header_tokens(line);
        if tokens.is_empty() {
            if saw_map_row {
                level.lines.push(line.clone());
            }
            index += 1;
            continue;
        }

        if matches!(
            tokens.as_slice(),
            ["rules"] | ["rules", "before"] | ["rules", "after"]
        ) {
            let before = tokens.as_slice() == ["rules", "before"];
            let target = if before {
                &mut level.rules_before
            } else {
                &mut level.rules_after
            };
            if target.is_some() {
                return Err(parse_error(
                    line,
                    if before {
                        "duplicate level rules before block"
                    } else {
                        "duplicate level rules after block (rules is the after shorthand)"
                    },
                ));
            }
            let (statements, next) = puzzle_authoring::collect_rule_statement_block(
                &source_lines,
                index + 1,
                "level rules",
            )
            .map_err(|error| {
                let source = error
                    .line_index()
                    .and_then(|line_index| source_lines.get(line_index))
                    .unwrap_or(line);
                parse_error(source, &error.message())
            })?;
            *target = Some(crate::model_syntax::RuleStatementsSyntax::new(
                line.clone(),
                before.then(|| "before".to_string()),
                statements,
            ));
            index = next;
            continue;
        }
        if tokens.first() == Some(&"rules") {
            return Err(parse_error(
                line,
                "level rules block header must be: rules | rules before | rules after",
            ));
        }

        if matches!(tokens.as_slice(), ["on_level_start"] | ["on_level_clear"]) {
            let (statements, next) = puzzle_authoring::collect_rule_statement_block(
                &source_lines,
                index + 1,
                "level lifecycle",
            )
            .map_err(|error| {
                let source = error
                    .line_index()
                    .and_then(|line_index| source_lines.get(line_index))
                    .unwrap_or(line);
                parse_error(source, &error.message())
            })?;
            let syntax =
                crate::model_syntax::RuleStatementsSyntax::new(line.clone(), None, statements);
            if tokens[0] == "on_level_start" {
                level.on_level_start.push(syntax);
            } else {
                level.on_level_clear.push(syntax);
            }
            index = next;
            continue;
        }
        if tokens[0] == "on_level_start" || tokens[0] == "on_level_clear" {
            return Err(parse_error(
                line,
                "level lifecycle block header must be: on_level_start | on_level_clear",
            ));
        }

        if is_level_event_sugar(line, &tokens) {
            if saw_map_row {
                level.level_clear_effect_rows.push(line.clone());
            } else {
                level.level_start_effect_rows.push(line.clone());
            }
            index += 1;
            continue;
        }

        if tokens[0] != "legend" {
            saw_map_row = true;
            level.lines.push(line.clone());
            index += 1;
            continue;
        }

        if tokens.len() == 1 {
            index += 1;
            while index < source_lines.len() && !is_block_close_line(&source_lines[index]) {
                if !source_lines[index].text.is_empty() {
                    level
                        .legends
                        .push(parse_level_legend_syntax(&source_lines[index], false)?);
                }
                index += 1;
            }
            if index >= source_lines.len() {
                return Err(parse_error(line, "level legend missing closing brace"));
            }
            index += 1;
            continue;
        }

        level.legends.push(parse_level_legend_syntax(line, true)?);
        index += 1;
    }
    Ok(level)
}

fn parse_level_legend_syntax(
    line: &source::LogicalLine,
    has_legend_prefix: bool,
) -> Result<crate::level::LevelLegendSyntax, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    let assignment = if has_legend_prefix {
        let Some(syntax) = crate::syntax::level_legend_directive_syntax(&tokens, true) else {
            return Err(parse_error(
                line,
                "level legend must be: legend <char> = <selector...>",
            ));
        };
        (&tokens[1], &tokens[syntax.rhs_start..])
    } else {
        let Some(syntax) = crate::syntax::legend_block_row_syntax(&tokens, true) else {
            return Err(parse_error(
                line,
                "level legend row must be: <char> = <selector...>",
            ));
        };
        (&tokens[0], &tokens[syntax.rhs_start..])
    };
    let ch = parse_char(Some(assignment.0), line, "missing legend char")?;
    Ok(crate::level::LevelLegendSyntax {
        source: line.clone(),
        ch,
        selectors: assignment
            .1
            .iter()
            .map(|selector| (*selector).to_string())
            .collect(),
    })
}

#[derive(Clone, Debug)]
struct PreparedLevelBody {
    name: String,
    pack: Option<String>,
    puzzle: String,
    lines: Vec<source::LogicalLine>,
    char_objects: HashMap<char, Vec<ObjectId>>,
    level_start_statements: Vec<StatementAst>,
    level_clear_statements: Vec<StatementAst>,
    rules_before_statements: Vec<StatementAst>,
    rules_after_statements: Vec<StatementAst>,
}

#[derive(Clone, Debug, Default)]
struct ParsedLevelBody {
    lines: Vec<source::LogicalLine>,
    local_char_objects: HashMap<char, Vec<ObjectId>>,
    level_start_statements: Vec<StatementAst>,
    level_clear_statements: Vec<StatementAst>,
    rules_before_statements: Vec<StatementAst>,
    rules_after_statements: Vec<StatementAst>,
}

#[allow(clippy::too_many_arguments)]
fn parse_level_body(
    level: &LevelBlock,
    catalog: &Catalog,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
) -> Result<ParsedLevelBody, DiagnosticReport> {
    parse_level_body_with_rules(level, catalog, named_conditions, true)
}

fn parse_level_body_for_editor(
    level: &LevelBlock,
    catalog: &Catalog,
) -> Result<ParsedLevelBody, DiagnosticReport> {
    parse_level_body_with_rules(level, catalog, &HashMap::new(), false)
}

fn parse_level_body_with_rules(
    level: &LevelBlock,
    catalog: &Catalog,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    include_rules: bool,
) -> Result<ParsedLevelBody, DiagnosticReport> {
    let mut body = ParsedLevelBody {
        lines: level.lines.clone(),
        ..ParsedLevelBody::default()
    };
    for legend in &level.legends {
        let mut tokens = vec!["legend".to_string(), legend.ch.to_string(), "=".to_string()];
        tokens.extend(legend.selectors.iter().cloned());
        let tokens = tokens.iter().map(String::as_str).collect::<Vec<_>>();
        let (ch, objects) = parse_level_legend_directive(&tokens, &legend.source, catalog)?;
        body.local_char_objects.insert(ch, objects);
    }
    if !include_rules {
        return Ok(body);
    }

    let lower = |syntax: &crate::model_syntax::RuleStatementsSyntax| {
        lower_statement_syntax(
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
        )
    };
    if let Some(syntax) = &level.rules_before {
        body.rules_before_statements = lower(syntax)?;
    }
    if let Some(syntax) = &level.rules_after {
        body.rules_after_statements = lower(syntax)?;
    }
    for syntax in &level.on_level_start {
        body.level_start_statements.extend(lower(syntax)?);
    }
    for syntax in &level.on_level_clear {
        body.level_clear_statements.extend(lower(syntax)?);
    }
    for line in &level.level_start_effect_rows {
        body.level_start_statements.push(
            parse_level_event_sugar(line)?
                .expect("canonical level start effect row was classified as event sugar"),
        );
    }
    for line in &level.level_clear_effect_rows {
        body.level_clear_statements.push(
            parse_level_event_sugar(line)?
                .expect("canonical level clear effect row was classified as event sugar"),
        );
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

fn parse_level_legend_directive(
    tokens: &[&str],
    line: &str,
    catalog: &Catalog,
) -> Result<(char, Vec<ObjectId>), DiagnosticReport> {
    let Some(syntax) = crate::syntax::level_legend_directive_syntax(tokens, true) else {
        return Err(parse_error(
            line,
            "level legend must be: legend <char> = <selector...>",
        ));
    };

    let ch = parse_char(tokens.get(1), line, "missing legend char")?;
    if tokens[syntax.rhs_start..] == ["empty"] {
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
