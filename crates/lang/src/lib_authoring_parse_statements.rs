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
        &catalog.variable_names,
        &catalog.numeric_variable_defaults,
        &catalog.condition_names,
        &HashMap::new(),
        &[],
    )?;
    Ok((event.to_string(), statements, next_i))
}

fn parse_rule_application(
    tokens: &[&str],
    declaration: &str,
    line: &str,
) -> Result<RuleApplication, DiagnosticReport> {
    match tokens {
        [kind, _] if *kind == declaration => Ok(RuleApplication::Once),
        [kind, _, application] if *kind == declaration => {
            parse_application_keyword(application, line)
        }
        _ => Err(parse_error(
            line,
            "routine header must be: routine <name> [once | once_all | once_per_level | random | repeat]",
        )),
    }
}

fn parse_application_keyword(token: &str, line: &str) -> Result<RuleApplication, DiagnosticReport> {
    match token {
        "once" => Ok(RuleApplication::Once),
        "once_all" => Ok(RuleApplication::OnceAll),
        "once_per_level" => Ok(RuleApplication::OncePerLevel),
        "random" => Ok(RuleApplication::Random),
        "repeat" => Ok(RuleApplication::UntilStable),
        _ => Err(parse_error(
            line,
            "application must be one of: once, once_all, once_per_level, random, repeat",
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
            "fix block must be: fix <once | random | repeat | orientation...>",
        ));
    }

    let mut defaults = FixDefaults::default();
    for token in &tokens[1..] {
        match *token {
            "once" | "once_all" | "once_per_level" | "random" | "repeat" => {
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
    variable_names: &HashMap<String, VariableId>,
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
            variable_names,
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
    variable_names: &HashMap<String, VariableId>,
    numeric_variables: &HashMap<String, i64>,
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
            variable_names,
            numeric_variables,
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
                source_line_number: line_numbers
                    .and_then(|line_numbers| line_numbers.get(start).copied()),
            }],
            start + 1,
        ));
    }
    let effects = parse_rewrite_effect(effect_text, line)?;
    Ok((
        vec![StatementAst::Effect {
            source_line: line.to_string(),
            source_line_number: line_numbers
                .and_then(|line_numbers| line_numbers.get(start).copied()),
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
    variable_names: &HashMap<String, VariableId>,
    numeric_variables: &HashMap<String, i64>,
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
        variable_names,
        numeric_variables,
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
        validate_map_argument(arg, line)?;
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

fn validate_map_argument(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    if let Some((base, label)) = value.split_once('#') {
        validate_identifier(base, line, "map argument")?;
        validate_tag_capture_label(label, line)?;
        return Ok(());
    }
    validate_identifier(value, line, "map argument")
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
    value: &ForExpansionValue,
    maps: &HashMap<String, ValueMap>,
) -> Result<Vec<String>, DiagnosticReport> {
    lines
        .iter()
        .map(|line| expand_for_binding_line(line, binding, value, maps))
        .collect()
}

fn expand_for_binding_line(
    line: &str,
    binding: &str,
    value: &ForExpansionValue,
    maps: &HashMap<String, ValueMap>,
) -> Result<String, DiagnosticReport> {
    let mut env = ValueEnv::default();
    if let Some(axis) = value.axis.as_deref() {
        env.bind(binding, axis, &value.value);
    } else {
        env.bind_untyped(binding, &value.value);
    }
    replace_for_tokens(line, binding, value, &env, maps)
}

fn replace_for_tokens(
    line: &str,
    binding: &str,
    value: &ForExpansionValue,
    env: &ValueEnv,
    maps: &HashMap<String, ValueMap>,
) -> Result<String, DiagnosticReport> {
    crate::rule_syntax::substitute_rule_binding_line(
        line,
        binding,
        |projection| match projection {
            None => Ok(value.value.clone()),
            Some(attr) => value.attrs.get(attr).cloned().ok_or_else(|| {
                parse_error(line, &format!("unknown for projection `{binding}.{attr}`"))
            }),
        },
        |name, arg| {
            if !maps.contains_key(name) {
                return Ok(None);
            }
            let expr = ValueExpr::MapCall {
                name: name.to_string(),
                arg: arg.to_string(),
            };
            eval_bound_value_expr(&expr, env, maps, line).map(Some)
        },
    )
}

#[derive(Clone, Debug)]
pub(crate) struct ForExpansionValue {
    value: String,
    axis: Option<String>,
    attrs: HashMap<String, String>,
}

impl ForExpansionValue {
    pub(crate) fn value(&self) -> &str {
        &self.value
    }

    fn atom(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            axis: None,
            attrs: HashMap::new(),
        }
    }

    fn axis_value(axis: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            axis: Some(axis.into()),
            attrs: HashMap::new(),
        }
    }
}

pub(crate) fn for_expansion_values(
    sources: &[&str],
    value_sets: &HashMap<String, Vec<String>>,
    numeric_variables: &HashMap<String, i64>,
    line: &str,
) -> Result<Vec<ForExpansionValue>, DiagnosticReport> {
    for_expansion_values_with_sets(
        sources,
        value_sets,
        numeric_variables,
        &HashMap::new(),
        line,
    )
}

fn for_expansion_values_with_sets(
    sources: &[&str],
    value_sets: &HashMap<String, Vec<String>>,
    numeric_variables: &HashMap<String, i64>,
    expansion_sets: &HashMap<String, Vec<ForExpansionValue>>,
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
        if let Some(values) = expansion_sets.get(source) {
            return Ok(values.clone());
        }
        if let Some(values) = value_sets.get(source) {
            return Ok(values
                .iter()
                .map(|value| ForExpansionValue::axis_value(source, value.clone()))
                .collect());
        }
        if let Some(values) = numeric_range_values(source, numeric_variables, line)? {
            return Ok(values.into_iter().map(ForExpansionValue::atom).collect());
        }
        return Err(parse_error(
            line,
            "unknown expansion set, tag set, or numeric range",
        ));
    }

    sources
        .iter()
        .flat_map(|source| {
            if let Some(values) = expansion_sets.get(*source) {
                return values.iter().cloned().map(Ok).collect::<Vec<_>>();
            }
            if let Some(values) = value_sets.get(*source) {
                return values
                    .iter()
                    .map(|value| Ok(ForExpansionValue::axis_value(*source, value.clone())))
                    .collect::<Vec<_>>();
            }
            match numeric_range_values(source, numeric_variables, line) {
                Ok(Some(values)) => values
                    .into_iter()
                    .map(|value| Ok(ForExpansionValue::atom(value)))
                    .collect(),
                Ok(None) => vec![Ok(ForExpansionValue::atom(*source))],
                Err(error) => vec![Err(error)],
            }
        })
        .collect()
}

fn expand_numeric_ranges_in_value_list(
    values: &[&str],
    numeric_variables: &HashMap<String, i64>,
    line: &str,
) -> Result<Vec<String>, DiagnosticReport> {
    let mut expanded = Vec::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(parse_error(line, "tag value must not be empty"));
        }
        if let Some(range_values) = numeric_range_values(value, numeric_variables, line)? {
            expanded.extend(range_values);
        } else {
            expanded.push((*value).to_string());
        }
    }
    Ok(expanded)
}

fn numeric_range_values(
    source: &str,
    numeric_variables: &HashMap<String, i64>,
    line: &str,
) -> Result<Option<Vec<String>>, DiagnosticReport> {
    let (start, end, inclusive) = if let Some((start, end)) = source.split_once("..<") {
        (start, end, false)
    } else if let Some((start, end)) = source.split_once("...") {
        (start, end, true)
    } else {
        return Ok(None);
    };
    if start.is_empty() || end.is_empty() || end.contains("...") || end.contains("..<") {
        return Err(parse_error(
            line,
            "numeric range must be: <integer>...<integer> or <integer>..<integer>",
        ));
    }
    let start = parse_numeric_range_endpoint(start, numeric_variables, line)?;
    let end = parse_numeric_range_endpoint(end, numeric_variables, line)?;
    if start > end || (!inclusive && start == end) {
        return Err(parse_error(
            line,
            "numeric range start must be less than range end",
        ));
    }
    let values = if inclusive {
        (start..=end).map(|value| value.to_string()).collect()
    } else {
        (start..end).map(|value| value.to_string()).collect()
    };
    Ok(Some(values))
}

fn parse_numeric_range_endpoint(
    value: &str,
    numeric_variables: &HashMap<String, i64>,
    line: &str,
) -> Result<i64, DiagnosticReport> {
    if let Ok(parsed) = value.parse::<i64>() {
        return Ok(parsed);
    }
    numeric_variables.get(value).copied().ok_or_else(|| {
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
        "for" | "fix" | "if" | "else" | "when" | "action" | "emit" | "do"
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
    variable_names: &HashMap<String, VariableId>,
    numeric_variables: &HashMap<String, i64>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    rule_params: &[String],
) -> Result<(Vec<StatementAst>, usize), DiagnosticReport> {
    let mut statements = Vec::new();
    let mut diagnostics = Vec::new();
    let mut local_routine_names = HashSet::<String>::new();
    let mut i = start;
    macro_rules! recover_current_statement {
        ($result:expr) => {
            match $result {
                Ok(value) => value,
                Err(report) => {
                    let report_line = lines.get(i).map(String::as_str).unwrap_or("");
                    let report_line_number =
                        line_numbers.and_then(|line_numbers| line_numbers.get(i).copied());
                    diagnostics.extend(
                        report_with_source_line_number(report, report_line, report_line_number)
                            .into_diagnostics(),
                    );
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
                diagnostics.extend(
                    report_with_source_line_number(report, source_line, source_line_number)
                        .into_diagnostics(),
                );
                i += 1;
                continue;
            }
        };
        let opens_block = line.trim_end().ends_with('{');
        let line = block_header_text(line);
        let tokens = split_header_tokens(line);
        match tokens.first().copied() {
            Some("routine") => {
                if !opens_block {
                    extend_report_with_source_line_number(
                        &mut diagnostics,
                        parse_error(line, "local routine block must use `{ ... }`"),
                        line,
                        source_line_number,
                    );
                    i += 1;
                    continue;
                }
                let (definition, next_i) = recover_current_statement!(parse_rule_definition(
                    lines,
                    line_numbers,
                    i,
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
                ));
                if !local_routine_names.insert(definition.name.clone()) {
                    extend_report_with_source_line_number(
                        &mut diagnostics,
                        parse_error(line, "duplicate local routine"),
                        line,
                        source_line_number,
                    );
                    i = next_i;
                    continue;
                }
                statements.push(StatementAst::LocalRoutine {
                    definition,
                    source_line: line.to_string(),
                    source_line_number,
                });
                i = next_i;
            }
            Some("for") => {
                if !opens_block {
                    extend_report_with_source_line_number(
                        &mut diagnostics,
                        parse_error(line, "for block must use `{ ... }`"),
                        line,
                        source_line_number,
                    );
                    i += 1;
                    continue;
                }
                let for_syntax = recover_current_statement!(
                    crate::rule_syntax::parse_rule_for_syntax(line)
                        .map_err(|error| parse_error(line, error))?
                        .ok_or_else(|| parse_error(line, "expected for rule syntax"))
                );
                let source_refs = for_syntax
                    .sources
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                let values = recover_current_statement!(for_expansion_values(
                    &source_refs,
                    value_sets,
                    numeric_variables,
                    line
                ));
                let (body_lines, body_line_numbers, next_i) = recover_current_statement!(
                    collect_statement_block_lines_with_numbers(lines, line_numbers, i + 1, line)
                );
                for value in &values {
                    let mut expanded_lines = match expand_for_binding_lines(
                        &body_lines,
                        &for_syntax.binding,
                        value,
                        maps,
                    ) {
                        Ok(lines) => lines,
                        Err(report) => {
                            extend_report_with_source_line_number(
                                &mut diagnostics,
                                report,
                                line,
                                source_line_number,
                            );
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
                        variable_names,
                        numeric_variables,
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
                        extend_report_with_source_line_number(
                            &mut diagnostics,
                            parse_error(line, "for expansion failed"),
                            line,
                            source_line_number,
                        );
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
                    variable_names,
                    numeric_variables,
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
                            variable_names,
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
                            variable_names,
                            numeric_variables,
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
                            variable_names,
                            numeric_variables,
                            condition_names,
                            named_conditions,
                            rule_params,
                        ));
                    statements.push(StatementAst::If {
                        source_line: line.to_string(),
                        source_line_number,
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
                        variable_names,
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
                            variable_names,
                            numeric_variables,
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
                                        variable_names,
                                        numeric_variables,
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
                            source_line_number,
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
                            source_line_number,
                            condition,
                            then_statements: vec![StatementAst::Call {
                                name: trailing.to_string(),
                                source_line: line.to_string(),
                                source_line_number,
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
                        variable_names,
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
                            variable_names,
                            numeric_variables,
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
                            variable_names,
                            numeric_variables,
                            condition_names,
                            named_conditions,
                            rule_params,
                        ));
                    statements.push(StatementAst::If {
                        source_line: line.to_string(),
                        source_line_number,
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
                    variable_names,
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
                    variable_names,
                    numeric_variables,
                    condition_names,
                    named_conditions,
                    rule_params,
                ));
                if next_i == 0 {
                    extend_report_with_source_line_number(
                        &mut diagnostics,
                        parse_error(line, "if block missing closing brace"),
                        line,
                        source_line_number,
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
                        variable_names,
                        numeric_variables,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ));
                statements.push(StatementAst::If {
                    source_line: line.to_string(),
                    source_line_number,
                    condition,
                    then_statements,
                    else_statements,
                });
                i = next_i;
            }
            Some("else") => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(line, "else without if"),
                    line,
                    source_line_number,
                );
                i += 1;
            }
            Some("when") => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(line, "use `if` for conditions"),
                    line,
                    source_line_number,
                );
                i += 1;
            }
            Some("action") if tokens.len() > 1 => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(
                        line,
                        "`action` statements were removed; use explicit input guards and rewrites",
                    ),
                    line,
                    source_line_number,
                );
                i += 1;
            }
            Some("emit") => {
                match parse_rewrite_effect(line, line) {
                    Ok(effects) => statements.push(StatementAst::Effect {
                        source_line: line.to_string(),
                        source_line_number,
                        effects,
                    }),
                    Err(report) => extend_report_with_source_line_number(
                        &mut diagnostics,
                        report,
                        line,
                        source_line_number,
                    ),
                }
                i += 1;
            }
            Some("do") => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(
                        line,
                        "`do` is obsolete; write the effect statement directly",
                    ),
                    line,
                    source_line_number,
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
                            variable_names,
                            numeric_variables,
                            condition_names,
                            named_conditions,
                            rule_params,
                        ));
                    statements.push(StatementAst::If {
                        source_line: line.to_string(),
                        source_line_number,
                        condition,
                        then_statements,
                        else_statements: Vec::new(),
                    });
                    i = next_i;
                } else {
                    match parse_rewrite_effect(effect_text, line) {
                        Ok(effects) => statements.push(StatementAst::If {
                            source_line: line.to_string(),
                            source_line_number,
                            condition,
                            then_statements: vec![StatementAst::Effect {
                                source_line: line.to_string(),
                                source_line_number,
                                effects,
                            }],
                            else_statements: Vec::new(),
                        }),
                        Err(report) => extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        ),
                    }
                    i += 1;
                }
            }
            _ if is_builtin_rewrite_effect_text(line) => {
                match parse_rewrite_effect(line, line) {
                    Ok(effects) => statements.push(StatementAst::Effect {
                        source_line: line.to_string(),
                        source_line_number,
                        effects,
                    }),
                    Err(report) => extend_report_with_source_line_number(
                        &mut diagnostics,
                        report,
                        line,
                        source_line_number,
                    ),
                }
                i += 1;
            }
            Some("[") => {
                if let Some(statement) = match parse_conditional_call_statement(
                    line,
                    source_line_number,
                    None,
                    rule_params,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    variable_names,
                ) {
                    Ok(statement) => statement,
                    Err(report) => {
                        extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        );
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
                        variable_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        ),
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
                        variable_names,
                        numeric_variables,
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
                        variable_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        ),
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
                        variable_names,
                        numeric_variables,
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
                        variable_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        ),
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
                        variable_names,
                        numeric_variables,
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
                        variable_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        ),
                    }
                    i = next_statement_i;
                }
            }
            Some("random") => {
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
                        variable_names,
                        numeric_variables,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ));
                    statements.push(StatementAst::Block {
                        application: RuleApplication::Random,
                        statements: nested,
                    });
                    i = next_i;
                } else {
                    match parse_application_prefixed_rewrite_statement(
                        line,
                        "random",
                        RuleApplication::Random,
                        rule_params,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        variable_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        ),
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
                        variable_names,
                        numeric_variables,
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
                        extend_report_with_source_line_number(
                            &mut diagnostics,
                            parse_error(
                                line,
                                "repeat until block must be: repeat until <condition>",
                            ),
                            line,
                            source_line_number,
                        );
                        i += 1;
                        continue;
                    }
                    let condition = recover_current_statement!(parse_statement_condition(
                        condition_text,
                        line,
                        input_names,
                        variable_names,
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
                        variable_names,
                        numeric_variables,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ));
                    statements.push(StatementAst::RepeatUntil {
                        source_line: line.to_string(),
                        source_line_number,
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
                        variable_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        ),
                    }
                    i = next_statement_i;
                }
            }
            Some(_) if line.starts_with('[') => {
                if let Some(statement) = match parse_conditional_call_statement(
                    line,
                    source_line_number,
                    None,
                    rule_params,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    variable_names,
                ) {
                    Ok(statement) => statement,
                    Err(report) => {
                        extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        );
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
                        variable_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        ),
                    }
                }
                i = next_statement_i;
            }
            Some("display") => {
                if tokens.len() == 1 {
                    let (_, next_i) = recover_current_statement!(parse_statement_block(
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
                        variable_names,
                        numeric_variables,
                        condition_names,
                        named_conditions,
                        rule_params,
                    ));
                    extend_report_with_source_line_number(
                        &mut diagnostics,
                        parse_error(
                            line,
                            "`display ...` syntax was removed; use @routine calls or bare display rewrites",
                        ),
                        line,
                        source_line_number,
                    );
                    i = next_i;
                } else {
                    extend_report_with_source_line_number(
                        &mut diagnostics,
                        parse_error(
                            line,
                            "`display ...` syntax was removed; use @routine calls or bare display rewrites",
                        ),
                        line,
                        source_line_number,
                    );
                    i += 1;
                }
            }
            Some(first) if is_oriented_rewrite_line(line, first) => {
                if let Some(statement) = match parse_conditional_call_statement(
                    line,
                    source_line_number,
                    Some(first),
                    rule_params,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    variable_names,
                ) {
                    Ok(statement) => statement,
                    Err(report) => {
                        extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        );
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
                        variable_names,
                    ) {
                        Ok(rewrite) => statements.push(StatementAst::Rewrite(
                            rewrite_with_source_line_number(rewrite, source_line_number),
                        )),
                        Err(report) => extend_report_with_source_line_number(
                            &mut diagnostics,
                            report,
                            line,
                            source_line_number,
                        ),
                    }
                }
                i = next_statement_i;
            }
            Some(call) if tokens.len() == 1 && is_shared_rule_call_statement(line, call) => {
                statements.push(StatementAst::Call {
                    name: call.to_string(),
                    source_line: line.to_string(),
                    source_line_number,
                });
                i += 1;
            }
            Some(call) if tokens.len() == 1 && is_at_identifier_token(call) => {
                statements.push(StatementAst::Call {
                    name: call.to_string(),
                    source_line: line.to_string(),
                    source_line_number,
                });
                i += 1;
            }
            Some(other) if scene_effect_command_syntax(other).is_some() => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(
                        line,
                        &format!(
                            "scene effect `{other}` cannot be used in puzzle statement blocks; \
                         put scene effects in a scene lifecycle, scene routine, \
                         or scene component effect"
                        ),
                    ),
                    line,
                    source_line_number,
                );
                i += 1;
            }
            Some(other) => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(line, &format!("unknown statement directive {other}")),
                    line,
                    source_line_number,
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
fn parse_conditional_call_statement(
    line: &str,
    source_line_number: Option<usize>,
    orientation_token: Option<&str>,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
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
        variable_names,
    )?;

    Ok(Some(StatementAst::Conditional {
        source_line: line.to_string(),
        source_line_number,
        condition,
        then_statements: vec![StatementAst::Call {
            name: rule_name.to_string(),
            source_line: line.to_string(),
            source_line_number,
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
    variable_names: &HashMap<String, VariableId>,
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
        variable_names,
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
    _variable_names: &HashMap<String, VariableId>,
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
        pattern: lower_pattern_source(
            &pattern,
            line,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            &HashMap::new(),
        )?,
    })
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
    variable_names: &HashMap<String, VariableId>,
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
        variable_names,
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
    variable_names: &HashMap<String, VariableId>,
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
        variable_names,
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
    variable_names: &HashMap<String, VariableId>,
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
        variable_names,
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
    variable_names: &HashMap<String, VariableId>,
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
    };
    let (before, after, effects, after_effects, after_call) = parse_inline_rewrite(
        rewrite,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        variable_names,
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
        puzzle_authoring::RuleApplicationSurface::Random => RuleApplication::Random,
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
    variable_names: &HashMap<String, VariableId>,
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
        variable_names,
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
    variable_names: &HashMap<String, VariableId>,
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
                        variable_names,
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
                        variable_names,
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
        variable_names,
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
    variable_names: &HashMap<String, VariableId>,
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

        let value = parse_variable_value(right, line)?;
        if variable_names.contains_key(left) {
            return Ok(match op {
                ComparisonOp::Eq => ConditionAst::VariableEquals {
                    name: left.to_string(),
                    value,
                },
                op => ConditionAst::VariableCompare {
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
    if variable_names.contains_key(condition) {
        return Ok(ConditionAst::VariableCompare {
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

fn report_with_source_line_number(
    report: DiagnosticReport,
    source_line: &str,
    source_line_number: Option<usize>,
) -> DiagnosticReport {
    let Some(source_line_number) = source_line_number else {
        return report;
    };
    let diagnostics = report
        .into_diagnostics()
        .into_iter()
        .map(|mut diagnostic| {
            if let Some(span) = &mut diagnostic.primary_span {
                if span.line.is_none() && span.source_line.as_deref() == Some(source_line) {
                    span.line = Some(source_line_number);
                }
            }
            diagnostic
        })
        .collect();
    DiagnosticReport::from_diagnostics(diagnostics)
}

fn extend_report_with_source_line_number(
    diagnostics: &mut Vec<crate::Diagnostic>,
    report: DiagnosticReport,
    source_line: &str,
    source_line_number: Option<usize>,
) {
    diagnostics.extend(
        report_with_source_line_number(report, source_line, source_line_number).into_diagnostics(),
    );
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
