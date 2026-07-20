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

fn parse_rule_application(
    tokens: &[String],
    declaration: &str,
    line: &str,
) -> Result<RuleApplication, DiagnosticReport> {
    match tokens {
        [kind, _] if kind == declaration => Ok(RuleApplication::Once),
        [kind, _, application] if kind == declaration => {
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
    tokens: &[String],
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
        match token.as_str() {
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
    lines: &[source::LogicalLine],
    start: usize,
    line: &str,
) -> Result<(Vec<source::LogicalLine>, usize), DiagnosticReport> {
    let mut body = Vec::new();
    let mut depth = 1i32;
    let mut i = start;
    while i < lines.len() {
        let nested_line = &lines[i];
        let next_depth = depth + nested_line.structural_brace_delta();
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
fn lower_statement_condition_syntax(
    syntax: &[puzzle_authoring::RuleStatementSyntax<source::LogicalLine>],
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
) -> Result<ConditionAst, DiagnosticReport> {
    let mut conditions = Vec::new();
    for statement in syntax {
        if statement.statements().is_some() {
            return Err(parse_error(
                statement.source(),
                "if condition block accepts condition rows, not nested blocks",
            ));
        }
        conditions.push(parse_statement_condition(
            statement.text(),
            statement.source(),
            input_names,
            variable_names,
            condition_names,
            named_conditions,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
        )?);
    }
    if conditions.is_empty() {
        return Err(parse_error(
            "if",
            "if condition block requires at least one condition",
        ));
    }
    Ok(if conditions.len() == 1 {
        conditions.remove(0)
    } else {
        combinator.combine(conditions)
    })
}

#[allow(clippy::too_many_arguments)]
fn lower_statement_arrow_syntax(
    statement: &puzzle_authoring::RuleStatementSyntax<source::LogicalLine>,
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
) -> Result<Vec<StatementAst>, DiagnosticReport> {
    let puzzle_authoring::RuleStatementNode::Arrow(target) = statement.node() else {
        return Err(parse_error(
            statement.source(),
            "if condition block must be followed by ->",
        ));
    };
    lower_statement_target_syntax(
        statement,
        target,
        statement.statements(),
        header_line,
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

#[allow(clippy::too_many_arguments)]
fn lower_statement_target_syntax(
    line: &puzzle_authoring::RuleStatementSyntax<source::LogicalLine>,
    target: &puzzle_authoring::RuleStatementTargetSurface,
    nested: Option<&[puzzle_authoring::RuleStatementSyntax<source::LogicalLine>]>,
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
) -> Result<Vec<StatementAst>, DiagnosticReport> {
    match (target, nested) {
        (puzzle_authoring::RuleStatementTargetSurface::Empty, Some(nested)) => {
            lower_statement_syntax(
                nested,
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
        (puzzle_authoring::RuleStatementTargetSurface::Empty, None) => Err(parse_error(
            line.source(),
            "if -> must be followed by an effect or block",
        )),
        (_, Some(_)) => Err(parse_error(
            line.source(),
            "if -> block header must be: -> {",
        )),
        (puzzle_authoring::RuleStatementTargetSurface::Call { name, .. }, None) => {
            Ok(vec![StatementAst::Call {
                name: name.clone(),
                source_line: line.text().to_string(),
                source_line_number: Some(line.source().line),
            }])
        }
        (puzzle_authoring::RuleStatementTargetSurface::Effect { span }, None) => {
            Ok(vec![StatementAst::Effect {
                source_line: line.text().to_string(),
                source_line_number: Some(line.source().line),
                effects: parse_rewrite_effect(&line.text()[span.clone()], header_line)?,
            }])
        }
        (puzzle_authoring::RuleStatementTargetSurface::Invalid { span }, None) => Err(parse_error(
            line.source(),
            &format!("invalid statement target: {}", &line.text()[span.clone()]),
        )),
    }
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

fn expand_for_binding_syntax(
    syntax: &[puzzle_authoring::RuleStatementSyntax<source::LogicalLine>],
    binding: &str,
    value: &AuthoringValue,
    maps: &HashMap<String, ValueMap>,
) -> Result<Vec<puzzle_authoring::RuleStatementSyntax<source::LogicalLine>>, DiagnosticReport> {
    syntax
        .iter()
        .map(|statement| {
            let statements = statement
                .statements()
                .map(|statements| expand_for_binding_syntax(statements, binding, value, maps))
                .transpose()?;
            Ok(statement.instantiate(
                expand_for_binding_line(statement.text(), binding, value, maps)?,
                statements,
            ))
        })
        .collect()
}

fn expand_for_binding_lines(
    lines: &[source::LogicalLine],
    binding: &str,
    value: &AuthoringValue,
    maps: &HashMap<String, ValueMap>,
) -> Result<Vec<source::LogicalLine>, DiagnosticReport> {
    lines
        .iter()
        .map(|line| {
            expand_for_binding_line(line, binding, value, maps).map(|text| line.with_text(text))
        })
        .collect()
}

struct ExpandedForLines {
    bodies: Vec<Vec<source::LogicalLine>>,
    next: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ForIterableCatalog {
    collections: HashMap<String, Vec<AuthoringValue>>,
}

impl ForIterableCatalog {
    pub(crate) fn insert(&mut self, name: impl Into<String>, values: Vec<AuthoringValue>) {
        self.collections.insert(name.into(), values);
    }

    fn get(&self, name: &str) -> Option<&[AuthoringValue]> {
        self.collections.get(name).map(Vec::as_slice)
    }
}

trait ForIterableSource {
    fn values(&self, name: &str) -> Option<Vec<AuthoringValue>>;
}

impl ForIterableSource for ForIterableCatalog {
    fn values(&self, name: &str) -> Option<Vec<AuthoringValue>> {
        self.get(name).map(<[_]>::to_vec)
    }
}

impl ForIterableSource for HashMap<String, Vec<String>> {
    fn values(&self, name: &str) -> Option<Vec<AuthoringValue>> {
        self.get(name).map(|values| {
            values
                .iter()
                .map(|value| AuthoringValue::variant(name, value.clone()))
                .collect()
        })
    }
}

fn expand_for_block_lines<Iterables: ForIterableSource>(
    lines: &[source::LogicalLine],
    start: usize,
    iterables: &Iterables,
    numeric_variables: &HashMap<String, i64>,
    maps: &HashMap<String, ValueMap>,
) -> Result<ExpandedForLines, DiagnosticReport> {
    let line = &lines[start];
    let syntax = puzzle_authoring::for_surface(line)
        .ok_or_else(|| parse_error(line, "for directive must be: for <binding> in <source...>"))?;
    let sources = syntax
        .sources
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let values = resolve_for_expansion_values(&sources, numeric_variables, line, |source| {
        iterables.values(source)
    })?;
    let (body_lines, next) = collect_statement_block_lines(lines, start + 1, line)?;
    let bodies = values
        .iter()
        .map(|value| expand_for_binding_lines(&body_lines, &syntax.binding, value, maps))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ExpandedForLines { bodies, next })
}

fn expand_for_binding_line(
    line: &str,
    binding: &str,
    value: &AuthoringValue,
    maps: &HashMap<String, ValueMap>,
) -> Result<String, DiagnosticReport> {
    let mut env = ValueEnv::default();
    if let Some((axis, variant)) = value.axis_binding() {
        env.bind(binding, axis, variant);
    } else if let Some(source) = value.scalar_source() {
        env.bind_untyped(binding, &source);
    }
    replace_for_tokens(line, binding, value, &env, maps)
}

fn replace_for_tokens(
    line: &str,
    binding: &str,
    value: &AuthoringValue,
    env: &ValueEnv,
    maps: &HashMap<String, ValueMap>,
) -> Result<String, DiagnosticReport> {
    crate::rule_syntax::substitute_rule_binding_line(
        line,
        binding,
        |projection| {
            value.project(projection).map_err(|error| {
                let reference = if projection.is_empty() {
                    binding.to_string()
                } else {
                    format!("{binding}.{}", projection.join("."))
                };
                let message = match error {
                    AuthoringProjectionError::MissingField { owner_type, field } => {
                        format!(
                            "{owner_type} has no field `{field}` while resolving `{reference}`"
                        )
                    }
                    AuthoringProjectionError::FieldRequired { owner_type } => format!(
                        "{owner_type} value requires an explicit field while resolving `{reference}`"
                    ),
                };
                parse_error(line, &message)
            })
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
pub(crate) enum AuthoringValue {
    Symbol(String),
    Text(String),
    Integer(i64),
    Variant {
        axis: String,
        value: String,
    },
    Record {
        type_name: String,
        fields: HashMap<String, AuthoringValue>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AuthoringProjectionError {
    MissingField { owner_type: String, field: String },
    FieldRequired { owner_type: String },
}

impl AuthoringValue {
    pub(crate) fn symbol(value: impl Into<String>) -> Self {
        Self::Symbol(value.into())
    }

    pub(crate) fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub(crate) fn integer(value: i64) -> Self {
        Self::Integer(value)
    }

    fn variant(axis: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Variant {
            axis: axis.into(),
            value: value.into(),
        }
    }

    pub(crate) fn record(
        type_name: impl Into<String>,
        fields: HashMap<String, AuthoringValue>,
    ) -> Self {
        Self::Record {
            type_name: type_name.into(),
            fields,
        }
    }

    fn scalar_source(&self) -> Option<String> {
        match self {
            Self::Symbol(value) | Self::Variant { value, .. } => Some(value.clone()),
            Self::Text(value) => Some(
                serde_json::to_string(value).expect("authoring text should serialize"),
            ),
            Self::Integer(value) => Some(value.to_string()),
            Self::Record { .. } => None,
        }
    }

    fn axis_binding(&self) -> Option<(&str, &str)> {
        match self {
            Self::Variant { axis, value } => Some((axis, value)),
            _ => None,
        }
    }

    fn project(&self, path: &[String]) -> Result<String, AuthoringProjectionError> {
        let Some((field, rest)) = path.split_first() else {
            return self.scalar_source().ok_or_else(|| {
                AuthoringProjectionError::FieldRequired {
                    owner_type: self.type_name().to_string(),
                }
            });
        };
        let Self::Record {
            type_name, fields, ..
        } = self
        else {
            return Err(AuthoringProjectionError::MissingField {
                owner_type: self.type_name().to_string(),
                field: field.clone(),
            });
        };
        let value = fields.get(field).ok_or_else(|| {
            AuthoringProjectionError::MissingField {
                owner_type: type_name.clone(),
                field: field.clone(),
            }
        })?;
        value.project(rest)
    }

    fn type_name(&self) -> &str {
        match self {
            Self::Symbol(_) => "Symbol",
            Self::Text(_) => "Text",
            Self::Integer(_) => "Int",
            Self::Variant { .. } => "Variant",
            Self::Record { type_name, .. } => type_name,
        }
    }
}

pub(crate) fn for_expansion_values(
    sources: &[&str],
    value_sets: &HashMap<String, Vec<String>>,
    numeric_variables: &HashMap<String, i64>,
    line: &str,
) -> Result<Vec<AuthoringValue>, DiagnosticReport> {
    resolve_for_expansion_values(sources, numeric_variables, line, |source| {
        value_sets.get(source).map(|values| {
            values
                .iter()
                .map(|value| AuthoringValue::variant(source, value.clone()))
                .collect()
        })
    })
}

fn resolve_for_expansion_values(
    sources: &[&str],
    numeric_variables: &HashMap<String, i64>,
    line: &str,
    mut resolve_collection: impl FnMut(&str) -> Option<Vec<AuthoringValue>>,
) -> Result<Vec<AuthoringValue>, DiagnosticReport> {
    if sources.is_empty() {
        return Err(parse_error(
            line,
            "for directive must be: for <binding> in <source...>",
        ));
    }
    if sources.len() == 1 {
        let source = sources[0];
        if let Some(values) = resolve_collection(source) {
            return Ok(values);
        }
        if let Some(values) = numeric_range_values(source, numeric_variables, line)? {
            return Ok(values
                .into_iter()
                .map(|value| {
                    AuthoringValue::integer(
                        value
                            .parse::<i64>()
                            .expect("numeric range values must be integers"),
                    )
                })
                .collect());
        }
        return Err(parse_error(
            line,
            "unknown expansion set, tag set, or numeric range",
        ));
    }

    sources
        .iter()
        .flat_map(|source| {
            if let Some(values) = resolve_collection(source) {
                return values.into_iter().map(Ok).collect::<Vec<_>>();
            }
            match numeric_range_values(source, numeric_variables, line) {
                Ok(Some(values)) => values
                    .into_iter()
                    .map(|value| {
                        Ok(AuthoringValue::integer(
                            value
                                .parse::<i64>()
                                .expect("numeric range values must be integers"),
                        ))
                    })
                    .collect(),
                Ok(None) => vec![Ok(AuthoringValue::symbol(*source))],
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

fn lower_statement_syntax(
    syntax: &[puzzle_authoring::RuleStatementSyntax<source::LogicalLine>],
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
) -> Result<Vec<StatementAst>, DiagnosticReport> {
    let mut statements = Vec::new();
    let mut diagnostics = Vec::new();
    let mut local_routine_names = HashSet::<String>::new();
    let mut i = 0;
    macro_rules! recover_current_statement {
        ($result:expr) => {
            match $result {
                Ok(value) => value,
                Err(report) => {
                    let report_line = syntax
                        .get(i)
                        .map(rule_statement_source)
                        .map(AsRef::as_ref)
                        .unwrap_or("");
                    let report_line_number = syntax
                        .get(i)
                        .map(rule_statement_source)
                        .map(|line| line.line);
                    diagnostics.extend(
                        report_with_source_line_number(report, report_line, report_line_number)
                            .into_diagnostics(),
                    );
                    i += 1;
                    continue;
                }
            }
        };
    }
    macro_rules! lower_nested_statement {
        ($nested:expr, $line:expr, $index:expr) => {{
            $nested
                .ok_or_else(|| parse_error($line, "statement block must use `{ ... }`"))
                .and_then(|nested| {
                    lower_statement_syntax(
                        nested,
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
                })
                .map(|nested| (nested, $index + 1))
        }};
    }
    macro_rules! lower_statements {
        ($nested:expr) => {{
            lower_statement_syntax(
                $nested,
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
        }};
    }
    macro_rules! lower_optional_else {
        ($next:expr) => {{
            match syntax.get($next) {
                Some(statement)
                    if matches!(statement.node(), puzzle_authoring::RuleStatementNode::Else) =>
                {
                    match statement.statements() {
                        Some(statements) => {
                            lower_statements!(statements).map(|lowered| (lowered, $next + 1))
                        }
                        None => Err(parse_error(
                            statement.source(),
                            "else block must use `{ ... }`",
                        )),
                    }
                }
                _ => Ok((Vec::new(), $next)),
            }
        }};
    }

    while i < syntax.len() {
        let statement_line = &syntax[i];
        let nested_syntax = statement_line.statements();
        let source_line = statement_line.source();
        let source_line_number = Some(source_line.line);
        let next_statement_i = i + 1;
        let line = statement_line.text();
        let opens_block = nested_syntax.is_some();
        let tokens = statement_line.tokens();
        match statement_line.node() {
            puzzle_authoring::RuleStatementNode::Routine => {
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
                let definition = recover_current_statement!(lower_rule_definition_syntax(
                    statement_line,
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
                    i += 1;
                    continue;
                }
                statements.push(StatementAst::LocalRoutine {
                    definition,
                    source_line: line.to_string(),
                    source_line_number,
                });
                i += 1;
            }
            puzzle_authoring::RuleStatementNode::For(for_syntax) => {
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
                let body_syntax = nested_syntax.expect("checked block syntax");
                for value in &values {
                    let expanded_syntax = match expand_for_binding_syntax(
                        body_syntax,
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
                    let nested = match lower_statement_syntax(
                        &expanded_syntax,
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
                    statements.extend(nested);
                }
                i += 1;
                continue;
            }
            puzzle_authoring::RuleStatementNode::Fix => {
                let defaults =
                    recover_current_statement!(parse_fix_defaults(tokens, line, rule_params));
                let (nested, next_i) =
                    recover_current_statement!(lower_nested_statement!(nested_syntax, line, i));
                statements.push(StatementAst::Fix {
                    defaults,
                    statements: nested,
                });
                i = next_i;
            }
            puzzle_authoring::RuleStatementNode::If(if_surface) => {
                if let Some(combinator) =
                    recover_current_statement!(parse_if_condition_block_header(line))
                {
                    let condition = recover_current_statement!(lower_statement_condition_syntax(
                        nested_syntax.ok_or_else(|| {
                            parse_error(line, "if condition block must use `{ ... }`")
                        })?,
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
                    let arrow_i = i + 1;
                    let arrow = recover_current_statement!(syntax.get(arrow_i).ok_or_else(|| {
                        parse_error(line, "if condition block must be followed by ->")
                    }));
                    let then_statements = recover_current_statement!(lower_statement_arrow_syntax(
                        arrow,
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
                        recover_current_statement!(lower_optional_else!(arrow_i + 1));
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
                        let then_statements = recover_current_statement!(lower_nested_statement!(
                            nested_syntax,
                            line,
                            i
                        ))
                        .0;
                        let (else_statements, after_i) =
                            recover_current_statement!(lower_optional_else!(i + 1));
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
                if let puzzle_authoring::RuleIfSurface::Inline { condition, target } = if_surface {
                    let condition = recover_current_statement!(parse_statement_condition(
                        &line[condition.clone()],
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
                    let then_statements =
                        recover_current_statement!(lower_statement_target_syntax(
                            statement_line,
                            target,
                            nested_syntax,
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
                        recover_current_statement!(lower_optional_else!(i + 1));
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
                let then_statements =
                    recover_current_statement!(lower_nested_statement!(nested_syntax, line, i)).0;
                let (else_statements, next_i) =
                    recover_current_statement!(lower_optional_else!(i + 1));
                statements.push(StatementAst::If {
                    source_line: line.to_string(),
                    source_line_number,
                    condition,
                    then_statements,
                    else_statements,
                });
                i = next_i;
            }
            puzzle_authoring::RuleStatementNode::Else => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(line, "else without if"),
                    line,
                    source_line_number,
                );
                i += 1;
            }
            puzzle_authoring::RuleStatementNode::When => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(line, "use `if` for conditions"),
                    line,
                    source_line_number,
                );
                i += 1;
            }
            puzzle_authoring::RuleStatementNode::Action => {
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
            puzzle_authoring::RuleStatementNode::Emit => {
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
            puzzle_authoring::RuleStatementNode::Do => {
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
            puzzle_authoring::RuleStatementNode::InputEffect(surface) => {
                let input_name = &line[surface.input.clone()];
                recover_current_statement!(validate_identifier(input_name, line, "input name"));
                let condition = ConditionAst::InputIs(input_name.to_string());
                let effect_text = &line[surface.effect.clone()];
                if effect_text.is_empty() || effect_text == "{" {
                    let (then_statements, next_i) =
                        recover_current_statement!(lower_nested_statement!(nested_syntax, line, i));
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
            puzzle_authoring::RuleStatementNode::Effect => {
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
            puzzle_authoring::RuleStatementNode::Rewrite(surface) => {
                let lowered = match (&surface.syntax.after, &surface.target) {
                    (None, puzzle_authoring::RuleStatementTargetSurface::Call { name, .. }) => {
                        lower_conditional_call_statement(
                            line,
                            source_line_number,
                            surface,
                            name,
                            rule_params,
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            variable_names,
                        )
                    }
                    _ => lower_rule_line_rewrite_statement(
                        line,
                        surface,
                        rule_params,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        variable_names,
                    )
                    .map(|rewrite| {
                        StatementAst::Rewrite(rewrite_with_source_line_number(
                            rewrite,
                            source_line_number,
                        ))
                    }),
                };
                match lowered {
                    Ok(statement) => statements.push(statement),
                    Err(report) => extend_report_with_source_line_number(
                        &mut diagnostics,
                        report,
                        line,
                        source_line_number,
                    ),
                }
                i = next_statement_i;
            }
            puzzle_authoring::RuleStatementNode::InvalidRewrite { error, .. } => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(line, error.message()),
                    line,
                    source_line_number,
                );
                i = next_statement_i;
            }
            puzzle_authoring::RuleStatementNode::Once => {
                let (nested, next_i) =
                    recover_current_statement!(lower_nested_statement!(nested_syntax, line, i));
                statements.push(StatementAst::Block {
                    application: RuleApplication::Once,
                    statements: nested,
                });
                i = next_i;
            }
            puzzle_authoring::RuleStatementNode::OnceAll => {
                let (nested, next_i) =
                    recover_current_statement!(lower_nested_statement!(nested_syntax, line, i));
                statements.push(StatementAst::Block {
                    application: RuleApplication::OnceAll,
                    statements: nested,
                });
                i = next_i;
            }
            puzzle_authoring::RuleStatementNode::OncePerLevel => {
                let (nested, next_i) =
                    recover_current_statement!(lower_nested_statement!(nested_syntax, line, i));
                statements.push(StatementAst::Block {
                    application: RuleApplication::OncePerLevel,
                    statements: nested,
                });
                i = next_i;
            }
            puzzle_authoring::RuleStatementNode::Random => {
                let (nested, next_i) =
                    recover_current_statement!(lower_nested_statement!(nested_syntax, line, i));
                statements.push(StatementAst::Block {
                    application: RuleApplication::Random,
                    statements: nested,
                });
                i = next_i;
            }
            puzzle_authoring::RuleStatementNode::Repeat => {
                if tokens.len() == 1 {
                    let (nested, next_i) =
                        recover_current_statement!(lower_nested_statement!(nested_syntax, line, i));
                    statements.push(StatementAst::Block {
                        application: RuleApplication::UntilStable,
                        statements: nested,
                    });
                    i = next_i;
                } else if tokens.get(1).map(String::as_str) == Some("until") {
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
                    let (nested, next_i) =
                        recover_current_statement!(lower_nested_statement!(nested_syntax, line, i));
                    statements.push(StatementAst::RepeatUntil {
                        source_line: line.to_string(),
                        source_line_number,
                        condition,
                        statements: nested,
                    });
                    i = next_i;
                } else {
                    extend_report_with_source_line_number(
                        &mut diagnostics,
                        parse_error(
                            line,
                            "repeat statement must be a block, `repeat until` block, or rewrite",
                        ),
                        line,
                        source_line_number,
                    );
                    i += 1;
                }
            }
            puzzle_authoring::RuleStatementNode::Display => {
                if tokens.len() == 1 {
                    let (_, next_i) =
                        recover_current_statement!(lower_nested_statement!(nested_syntax, line, i));
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
            puzzle_authoring::RuleStatementNode::Call { name } => {
                statements.push(StatementAst::Call {
                    name: name.clone(),
                    source_line: line.to_string(),
                    source_line_number,
                });
                i += 1;
            }
            puzzle_authoring::RuleStatementNode::Other(Some(other))
                if scene_effect_command_syntax(other).is_some() =>
            {
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
            puzzle_authoring::RuleStatementNode::Other(Some(other)) => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(line, &format!("unknown statement directive {other}")),
                    line,
                    source_line_number,
                );
                i += 1;
            }
            puzzle_authoring::RuleStatementNode::Arrow(_) => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(line, "statement arrow must follow an if condition block"),
                    line,
                    source_line_number,
                );
                i += 1;
            }
            puzzle_authoring::RuleStatementNode::ConditionRow => {
                extend_report_with_source_line_number(
                    &mut diagnostics,
                    parse_error(line, "condition row is only valid inside `if [all | any]`"),
                    line,
                    source_line_number,
                );
                i += 1;
            }
            puzzle_authoring::RuleStatementNode::Other(None) => i += 1,
        }
    }

    if !diagnostics.is_empty() {
        Err(DiagnosticReport::from_diagnostics(diagnostics))
    } else {
        Ok(statements)
    }
}

fn rule_statement_source(
    statement: &puzzle_authoring::RuleStatementSyntax<source::LogicalLine>,
) -> &source::LogicalLine {
    statement.source()
}

fn rewrite_with_source_line_number(
    mut rewrite: OrientedRewriteAst,
    source_line_number: Option<usize>,
) -> OrientedRewriteAst {
    rewrite.source_line_number = source_line_number;
    rewrite
}

#[allow(clippy::too_many_arguments)]
fn lower_conditional_call_statement(
    line: &str,
    source_line_number: Option<usize>,
    surface: &puzzle_authoring::RuleRewriteSurface,
    rule_name: &str,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
) -> Result<StatementAst, DiagnosticReport> {
    let (orientation, application, rewrite) =
        rewrite_orientation_and_application(line, &surface.line, rule_params)?;
    if application.is_some() {
        return Err(parse_error(
            line,
            "application-prefixed rewrite cannot target a routine call",
        ));
    }
    let rewrite_source = &line[rewrite];
    let condition = PatternConditionAst {
        predicate: PatternPredicateAst::Some,
        orientation,
        pattern: lower_unresolved_pattern(
            surface.syntax.before.clone(),
            rewrite_source,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            variable_names,
        )?,
    };

    Ok(StatementAst::Conditional {
        source_line: line.to_string(),
        source_line_number,
        condition,
        then_statements: vec![StatementAst::Call {
            name: rule_name.to_string(),
            source_line: line.to_string(),
            source_line_number,
        }],
        else_statements: Vec::new(),
    })
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

#[allow(clippy::too_many_arguments)]
fn lower_rule_line_rewrite_statement(
    line: &str,
    surface: &puzzle_authoring::RuleRewriteSurface,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
) -> Result<OrientedRewriteAst, DiagnosticReport> {
    let (orientation, application, rewrite) =
        rewrite_orientation_and_application(line, &surface.line, rule_params)?;
    let rewrite_source = &line[rewrite];
    let (before, after, effects, after_effects, after_call) = lower_inline_rewrite_syntax(
        &surface.syntax,
        &surface.target,
        rewrite_source,
        line,
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

fn rewrite_orientation_and_application(
    line: &str,
    surface: &puzzle_authoring::RuleLineSurfaceSpans,
    rule_params: &[String],
) -> Result<
    (
        OrientationExpr,
        Option<RuleApplication>,
        std::ops::Range<usize>,
    ),
    DiagnosticReport,
> {
    Ok(match surface {
        puzzle_authoring::RuleLineSurfaceSpans::InputRewrite {
            application,
            surface,
        } => {
            if let Some(axis) = &surface.orientation {
                let axis = &line[axis.clone()];
                validate_identifier(axis, line, "input orientation")?;
            }
            (
                OrientationExpr::InputSet(
                    surface
                        .orientation
                        .as_ref()
                        .map(|axis| line[axis.clone()].to_string())
                        .unwrap_or_else(|| "directions".to_string()),
                ),
                application
                    .as_ref()
                    .map(|application| rule_application_from_surface(application.application)),
                surface.rewrite.clone(),
            )
        }
        puzzle_authoring::RuleLineSurfaceSpans::NeutralRewrite {
            application,
            rewrite,
        } => (
            OrientationExpr::Neutral,
            application
                .as_ref()
                .map(|application| rule_application_from_surface(application.application)),
            rewrite.clone(),
        ),
        puzzle_authoring::RuleLineSurfaceSpans::OrientedRewrite {
            application,
            orientation,
            rewrite,
        } => (
            parse_statement_orientation_expr(&line[orientation.clone()], rule_params),
            application
                .as_ref()
                .map(|application| rule_application_from_surface(application.application)),
            rewrite.clone(),
        ),
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
