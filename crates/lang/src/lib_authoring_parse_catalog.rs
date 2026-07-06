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
    if matches!(values.first().copied(), Some("rotation" | "translation")) {
        return parse_typed_axis_directive(name, values, line, catalog);
    }
    let expanded_values =
        expand_numeric_ranges_in_value_list(values, &catalog.numeric_variable_defaults, line)?;
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

fn parse_typed_axis_directive(
    name: &str,
    values: &[&str],
    line: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    if is_builtin_value_set(name) {
        return Err(parse_error(line, "built-in tag set cannot be redefined"));
    }
    if catalog.value_sets.contains_key(name)
        || catalog.object_axes.contains_key(name)
        || catalog.axis_kinds.contains_key(name)
    {
        return Err(parse_error(line, "duplicate tag set"));
    }

    let kind = match values.first().copied() {
        Some("rotation") => AxisKind::Rotation,
        Some("translation") => AxisKind::Translation,
        _ => {
            return Err(parse_error(
                line,
                "axis declaration must start with rotation or translation",
            ))
        }
    };
    let body = values[1..].join(" ");
    let expanded_values = match kind {
        AxisKind::Rotation => parse_rotation_axis_values(&body, line)?,
        AxisKind::Translation => parse_translation_axis_values(&body, line)?,
    };
    if expanded_values.is_empty() {
        return Err(parse_error(line, "axis must have at least one value"));
    }
    catalog
        .object_axes
        .insert(name.to_string(), expanded_values);
    catalog.axis_kinds.insert(name.to_string(), kind);
    Ok(())
}

fn parse_rotation_axis_values(body: &str, line: &str) -> Result<Vec<String>, DiagnosticReport> {
    let body = body.trim();
    if let Some(step) = body.strip_prefix("step ").map(str::trim) {
        let step = parse_degree_value(step, line)?;
        return expand_rotation_range(Rational::ZERO, Rational::integer(360), false, step, line);
    }
    let (range, step) = split_axis_range_and_step(body, line)?;
    let (start, end, inclusive) = parse_degree_range(range, line)?;
    let step = parse_degree_value(step, line)?;
    expand_rotation_range(start, end, inclusive, step, line)
}

fn parse_translation_axis_values(body: &str, line: &str) -> Result<Vec<String>, DiagnosticReport> {
    let body = body.trim();
    let scalar_values = if let Some(step) = body.strip_prefix("step ").map(str::trim) {
        let step = parse_rational_value(step, line)?;
        expand_rational_range(Rational::ZERO, Rational::integer(1), false, step, line)?
    } else if body.contains(" step ") {
        let (range, step) = split_axis_range_and_step(body, line)?;
        let (start, end, inclusive) = parse_rational_range(range, line)?;
        let step = parse_rational_value(step, line)?;
        expand_rational_range(start, end, inclusive, step, line)?
    } else {
        parse_rational_list(body, line)?
    };

    let mut values = Vec::new();
    for x in &scalar_values {
        for y in &scalar_values {
            values.push(format!("{},{}", x.format(), y.format()));
        }
    }
    Ok(values)
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
        return Err(parse_error(line, "rotation range must use ... or ..<"));
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
        return Err(parse_error(line, "translation range must use ... or ..<"));
    };
    Ok((
        parse_rational_value(start.trim(), line)?,
        parse_rational_value(end.trim(), line)?,
        true,
    ))
}

fn expand_rotation_range(
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

fn parse_rational_list(body: &str, line: &str) -> Result<Vec<Rational>, DiagnosticReport> {
    let mut values = Vec::new();
    for value in body.split(',') {
        let value = value.trim();
        if value.is_empty() {
            return Err(parse_error(
                line,
                "translation value list must not contain empty values",
            ));
        }
        let value = parse_rational_value(value, line)?;
        if !values.contains(&value) {
            values.push(value);
        }
    }
    Ok(values)
}

fn parse_degree_value(value: &str, line: &str) -> Result<Rational, DiagnosticReport> {
    let value = value
        .trim()
        .strip_suffix("deg")
        .ok_or_else(|| parse_error(line, "rotation values must use deg"))?;
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
        return Err(parse_error(line, "assignment name does not match directive"));
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
        let axis_kinds = catalog.axis_kinds.clone();
        define_object_spec(
            term,
            layer,
            None,
            line,
            &value_sets,
            &axis_kinds,
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
            [name, ..]
                if crate::syntax::named_selector_assignment_syntax(&tokens, true).is_some() =>
            {
                let syntax = crate::syntax::named_selector_assignment_syntax(&tokens, true)
                    .expect("guarded named layer assignment syntax");
                let selectors = &tokens[syntax.rhs_start..];
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
        let axis_kinds = catalog.axis_kinds.clone();
        define_object_spec(
            selector,
            UNASSIGNED_LAYER,
            None,
            line,
            &value_sets,
            &axis_kinds,
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
            | StatementAst::DisplayCall { .. }
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
