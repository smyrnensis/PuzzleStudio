use puzzle_core::ComparisonOp;
use std::collections::HashMap;

use crate::{
    DiagnosticReport, QueryExprOf, SolverDeadendOf, SolverStrategyDirection, SolverStrategyOf,
    SolverStrategyTermOf, is_block_header_line, split_header_tokens,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SolverSurfaceQueryDefinition {
    pub(crate) name: String,
    pub(crate) source_line: String,
    pub(crate) expr: SolverSurfaceQueryExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SolverSurfaceQueryExpr {
    Named(String),
    Call {
        name: String,
        args: Vec<SolverSurfaceQueryArg>,
    },
    Compare {
        left: Box<SolverSurfaceQueryExpr>,
        op: ComparisonOp,
        right: i64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SolverSurfaceQueryArg {
    Selector(String),
    Pattern(SolverSurfacePatternArg),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SolverSurfacePatternArg {
    pub(crate) source: String,
    pub(crate) orientation: SolverSurfacePatternOrientation,
    pub(crate) pattern: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SolverSurfacePatternOrientation {
    Neutral,
    Input { axis: Option<String> },
    Orientation(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OrientedPatternArgSurface {
    pub(crate) orientation: OrientedPatternArgOrientationSurface,
    pub(crate) pattern: std::ops::Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OrientedPatternArgOrientationSurface {
    Neutral,
    Input {
        input: std::ops::Range<usize>,
        axis: Option<std::ops::Range<usize>>,
    },
    Orientation {
        orientation: std::ops::Range<usize>,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SolverSurfaceStrategy {
    pub(crate) terms: Vec<SolverSurfaceStrategyTerm>,
    pub(crate) deadends: Vec<SolverSurfaceDeadend>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SolverSurfaceStrategyTerm {
    pub(crate) source_line: String,
    pub(crate) direction: SolverStrategyDirection,
    pub(crate) value: SolverSurfaceQueryExpr,
    pub(crate) weight: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SolverSurfaceDeadend {
    pub(crate) source_line: String,
    pub(crate) combinator: SolverSurfaceDeadendCombinator,
    pub(crate) values: Vec<SolverSurfaceQueryExpr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SolverSurfaceDeadendCombinator {
    All,
    Any,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NamedQueryState {
    Visiting,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SolverQueryCallKind {
    Count,
    Exists,
    None,
}

pub(crate) struct NamedQueryLowerer<'a, Definition, Value> {
    definitions_by_name: HashMap<String, &'a Definition>,
    cache: HashMap<String, Value>,
    states: HashMap<String, NamedQueryState>,
    stack: Vec<String>,
}

impl<'a, Definition, Value> NamedQueryLowerer<'a, Definition, Value>
where
    Value: Clone,
{
    pub(crate) fn new(definitions: &'a [Definition], name: fn(&Definition) -> &str) -> Self {
        Self {
            definitions_by_name: definitions
                .iter()
                .map(|definition| (name(definition).to_string(), definition))
                .collect(),
            cache: HashMap::new(),
            states: HashMap::new(),
            stack: Vec::new(),
        }
    }

    pub(crate) fn into_cache(self) -> HashMap<String, Value> {
        self.cache
    }

    pub(crate) fn lower_named<Context, Error>(
        &mut self,
        name: &str,
        source_line: &str,
        context: &Context,
        lower_definition: fn(&mut Self, &Context, &Definition) -> Result<Value, Error>,
        cycle_error: fn(Vec<String>, &str) -> Error,
        unknown_error: fn(&str, &str) -> Error,
    ) -> Result<Value, Error> {
        match self.states.get(name).copied() {
            Some(NamedQueryState::Done) => {
                return Ok(self
                    .cache
                    .get(name)
                    .expect("done query has a lowered expression")
                    .clone());
            }
            Some(NamedQueryState::Visiting) => {
                let mut cycle = self.stack.clone();
                cycle.push(name.to_string());
                return Err(cycle_error(cycle, source_line));
            }
            None => {}
        }
        let Some(definition) = self.definitions_by_name.get(name).copied() else {
            return Err(unknown_error(name, source_line));
        };
        self.states
            .insert(name.to_string(), NamedQueryState::Visiting);
        self.stack.push(name.to_string());
        let lowered = lower_definition(self, context, definition)?;
        self.stack.pop();
        self.states.insert(name.to_string(), NamedQueryState::Done);
        self.cache.insert(name.to_string(), lowered.clone());
        Ok(lowered)
    }
}

pub(crate) trait SolverQueryLoweringAdapter<Context> {
    type Object: Clone;
    type Value: Clone;
    type Variable: Clone;
    type Error;

    fn lower_variable(
        _name: &str,
        _source_line: &str,
        _context: &Context,
    ) -> Result<Option<Self::Variable>, Self::Error> {
        Ok(None)
    }

    fn lower_distance_selector(
        selector: &SolverSurfaceQueryArg,
        source_line: &str,
        context: &Context,
    ) -> Result<Vec<Self::Object>, Self::Error>;

    fn lower_selector_query_value(
        kind: SolverQueryCallKind,
        selector: &str,
        source_line: &str,
        context: &Context,
    ) -> Result<Self::Value, Self::Error>;

    fn lower_pattern_query_value(
        kind: SolverQueryCallKind,
        pattern: &SolverSurfacePatternArg,
        source_line: &str,
        context: &Context,
    ) -> Result<Self::Value, Self::Error>;

    fn query_call_error(message: &'static str, source_line: &str) -> Self::Error;

    fn lower_call(
        name: &str,
        args: &[SolverSurfaceQueryArg],
        source_line: &str,
        context: &Context,
    ) -> Result<QueryExprOf<Self::Object, Self::Value, Self::Variable>, Self::Error> {
        if name == "distance" {
            let [from, to] = args else {
                return Err(Self::query_call_error(
                    "distance query must be: distance(<selector>, <selector>)",
                    source_line,
                ));
            };
            return Ok(QueryExprOf::Distance {
                from: Self::lower_distance_selector(from, source_line, context)?,
                to: Self::lower_distance_selector(to, source_line, context)?,
            });
        }
        let kind = match solver_query_call_kind(name) {
            Some(kind) => kind,
            None => {
                return Err(Self::query_call_error(
                    "unknown query function",
                    source_line,
                ));
            }
        };
        let [arg] = args else {
            return Err(Self::query_call_error(
                "query expression must have exactly one argument",
                source_line,
            ));
        };
        let value = match arg {
            SolverSurfaceQueryArg::Selector(selector) => {
                Self::lower_selector_query_value(kind, selector, source_line, context)?
            }
            SolverSurfaceQueryArg::Pattern(pattern) => {
                Self::lower_pattern_query_value(kind, pattern, source_line, context)?
            }
        };
        Ok(QueryExprOf::Value(value))
    }

    fn cycle_error(cycle: Vec<String>, source_line: &str) -> Self::Error;

    fn unknown_query_error(name: &str, source_line: &str) -> Self::Error;
}

fn solver_query_call_kind(name: &str) -> Option<SolverQueryCallKind> {
    match name {
        "count" => Some(SolverQueryCallKind::Count),
        "exists" | "some" => Some(SolverQueryCallKind::Exists),
        "none" | "no" => Some(SolverQueryCallKind::None),
        _ => None,
    }
}

pub(crate) fn lower_query_definitions_with<Adapter, Context>(
    definitions: &[SolverSurfaceQueryDefinition],
    context: &Context,
) -> Result<
    HashMap<String, QueryExprOf<Adapter::Object, Adapter::Value, Adapter::Variable>>,
    Adapter::Error,
>
where
    Adapter: SolverQueryLoweringAdapter<Context>,
{
    let mut lowerer = NamedQueryLowerer::new(definitions, surface_query_definition_name);
    for definition in definitions {
        lowerer.lower_named(
            &definition.name,
            &definition.source_line,
            context,
            lower_query_definition_with::<Adapter, Context>,
            Adapter::cycle_error,
            Adapter::unknown_query_error,
        )?;
    }
    Ok(lowerer.into_cache())
}

pub(crate) fn lower_solver_strategy_with<Adapter, Context>(
    strategy: Option<SolverSurfaceStrategy>,
    definitions: &[SolverSurfaceQueryDefinition],
    context: &Context,
) -> Result<
    SolverStrategyOf<QueryExprOf<Adapter::Object, Adapter::Value, Adapter::Variable>>,
    Adapter::Error,
>
where
    Adapter: SolverQueryLoweringAdapter<Context>,
{
    let Some(strategy) = strategy else {
        return Ok(SolverStrategyOf::default());
    };
    let mut lowerer = NamedQueryLowerer::new(definitions, surface_query_definition_name);
    let terms = strategy
        .terms
        .into_iter()
        .map(|term| {
            let value = lower_query_expr_with::<Adapter, Context>(
                &term.value,
                &term.source_line,
                context,
                &mut lowerer,
            )?;
            Ok(SolverStrategyTermOf {
                direction: term.direction,
                value,
                weight: term.weight,
            })
        })
        .collect::<Result<Vec<_>, Adapter::Error>>()?;
    let deadends = strategy
        .deadends
        .into_iter()
        .map(|deadend| {
            let values = deadend
                .values
                .into_iter()
                .map(|value| {
                    lower_query_expr_with::<Adapter, Context>(
                        &value,
                        &deadend.source_line,
                        context,
                        &mut lowerer,
                    )
                })
                .collect::<Result<Vec<_>, Adapter::Error>>()?;
            Ok(match deadend.combinator {
                SolverSurfaceDeadendCombinator::All => SolverDeadendOf::All(values),
                SolverSurfaceDeadendCombinator::Any => SolverDeadendOf::Any(values),
            })
        })
        .collect::<Result<Vec<_>, Adapter::Error>>()?;
    Ok(SolverStrategyOf { terms, deadends })
}

fn surface_query_definition_name(definition: &SolverSurfaceQueryDefinition) -> &str {
    &definition.name
}

fn lower_query_definition_with<Adapter, Context>(
    lowerer: &mut NamedQueryLowerer<
        '_,
        SolverSurfaceQueryDefinition,
        QueryExprOf<Adapter::Object, Adapter::Value, Adapter::Variable>,
    >,
    context: &Context,
    definition: &SolverSurfaceQueryDefinition,
) -> Result<QueryExprOf<Adapter::Object, Adapter::Value, Adapter::Variable>, Adapter::Error>
where
    Adapter: SolverQueryLoweringAdapter<Context>,
{
    lower_query_expr_with::<Adapter, Context>(
        &definition.expr,
        &definition.source_line,
        context,
        lowerer,
    )
}

fn lower_query_expr_with<Adapter, Context>(
    expr: &SolverSurfaceQueryExpr,
    source_line: &str,
    context: &Context,
    lowerer: &mut NamedQueryLowerer<
        '_,
        SolverSurfaceQueryDefinition,
        QueryExprOf<Adapter::Object, Adapter::Value, Adapter::Variable>,
    >,
) -> Result<QueryExprOf<Adapter::Object, Adapter::Value, Adapter::Variable>, Adapter::Error>
where
    Adapter: SolverQueryLoweringAdapter<Context>,
{
    match expr {
        SolverSurfaceQueryExpr::Named(name) => {
            if let Some(variable) = Adapter::lower_variable(name, source_line, context)? {
                return Ok(QueryExprOf::Variable(variable));
            }
            lowerer.lower_named(
                name,
                source_line,
                context,
                lower_query_definition_with::<Adapter, Context>,
                Adapter::cycle_error,
                Adapter::unknown_query_error,
            )
        }
        SolverSurfaceQueryExpr::Call { name, args } => {
            Adapter::lower_call(name, args, source_line, context)
        }
        SolverSurfaceQueryExpr::Compare { left, op, right } => Ok(QueryExprOf::Compare {
            left: Box::new(lower_query_expr_with::<Adapter, Context>(
                left,
                source_line,
                context,
                lowerer,
            )?),
            op: *op,
            right: *right,
        }),
    }
}

pub(crate) fn parse_query_definition(
    line: &str,
) -> Result<SolverSurfaceQueryDefinition, DiagnosticReport> {
    let Some(rest) = line.strip_prefix("query ") else {
        return Err(parse_error(
            line,
            "query must be: query <name> = <query_expr>",
        ));
    };
    let (name, value) = puzzle_authoring::parse_assignment_row(rest)
        .ok_or_else(|| parse_error(line, "query must be: query <name> = <query_expr>"))?;
    validate_query_name(name, line)?;
    Ok(SolverSurfaceQueryDefinition {
        name: name.to_string(),
        source_line: line.to_string(),
        expr: parse_query_expr(value, line)?,
    })
}

pub(crate) fn parse_solver_block(
    lines: &[String],
    start: usize,
) -> Result<(usize, SolverSurfaceStrategy), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if header.as_slice() != ["solver"] || !is_block_header_line(&lines[start]) {
        return Err(parse_error(
            &lines[start],
            "solver block must be: solver { ... }",
        ));
    }

    parse_solver_body(lines, start + 1, true, &lines[start])
}

pub(crate) fn parse_solver_entry_body(
    lines: &[String],
) -> Result<SolverSurfaceStrategy, DiagnosticReport> {
    parse_solver_body(lines, 0, false, "solver {").map(|(_, solver)| solver)
}

fn parse_solver_body(
    lines: &[String],
    mut i: usize,
    closing_brace_required: bool,
    owner_line: &str,
) -> Result<(usize, SolverSurfaceStrategy), DiagnosticReport> {
    let mut strategy = None::<SolverSurfaceStrategy>;
    let mut deadends = Vec::new();
    while i < lines.len() {
        let line = &lines[i];
        if line == "}" {
            let mut solver = strategy.unwrap_or_default();
            solver.deadends = deadends;
            return Ok((i + 1, solver));
        }
        let tokens = split_header_tokens(line);
        if tokens.is_empty() {
            i += 1;
            continue;
        }
        match tokens.as_slice() {
            ["strategy"] if is_block_header_line(line) => {
                if strategy.is_some() {
                    return Err(parse_error(line, "duplicate solver strategy block"));
                }
                let (next_i, parsed_strategy) = parse_solver_strategy_block(lines, i)?;
                strategy = Some(parsed_strategy);
                i = next_i;
            }
            ["strategy", ..] => {
                return Err(parse_error(
                    line,
                    "solver strategy block must be: strategy { ... }",
                ));
            }
            ["deadend"] if is_block_header_line(line) => {
                let (next_i, deadend) =
                    parse_solver_deadend_block(lines, i, SolverSurfaceDeadendCombinator::All)?;
                deadends.push(deadend);
                i = next_i;
            }
            ["deadend", "all"] if is_block_header_line(line) => {
                let (next_i, deadend) =
                    parse_solver_deadend_block(lines, i, SolverSurfaceDeadendCombinator::All)?;
                deadends.push(deadend);
                i = next_i;
            }
            ["deadend", "any"] if is_block_header_line(line) => {
                let (next_i, deadend) =
                    parse_solver_deadend_block(lines, i, SolverSurfaceDeadendCombinator::Any)?;
                deadends.push(deadend);
                i = next_i;
            }
            ["deadend"] => return Err(parse_error(line, "deadend must have a query or block")),
            ["deadend", "all"] | ["deadend", "any"] => {
                return Err(parse_error(
                    line,
                    "deadend block must be: deadend [all | any] { ... }",
                ));
            }
            ["deadend", ..] if is_block_header_line(line) => {
                return Err(parse_error(
                    line,
                    "deadend block must be: deadend [all | any] { ... }",
                ));
            }
            ["deadend", ..] => {
                let value = line
                    .trim_start()
                    .strip_prefix("deadend")
                    .expect("deadend came from the line prefix")
                    .trim();
                if value.is_empty() {
                    return Err(parse_error(line, "deadend must have a query"));
                }
                deadends.push(SolverSurfaceDeadend {
                    source_line: line.to_string(),
                    combinator: SolverSurfaceDeadendCombinator::All,
                    values: vec![parse_query_expr(value, line)?],
                });
                i += 1;
            }
            _ => return Err(parse_error(line, "unknown solver block row")),
        }
    }
    if closing_brace_required {
        Err(parse_error(
            owner_line,
            "solver block missing closing brace",
        ))
    } else {
        let mut solver = strategy.unwrap_or_default();
        solver.deadends = deadends;
        Ok((i, solver))
    }
}

fn parse_solver_deadend_block(
    lines: &[String],
    start: usize,
    combinator: SolverSurfaceDeadendCombinator,
) -> Result<(usize, SolverSurfaceDeadend), DiagnosticReport> {
    let source_line = lines[start].clone();
    let mut values = Vec::new();
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if line == "}" {
            if values.is_empty() {
                return Err(parse_error(
                    &source_line,
                    "deadend block requires at least one query",
                ));
            }
            return Ok((
                i + 1,
                SolverSurfaceDeadend {
                    source_line,
                    combinator,
                    values,
                },
            ));
        }
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        values.push(parse_query_expr(line, line)?);
        i += 1;
    }
    Err(parse_error(
        &source_line,
        "deadend block missing closing brace",
    ))
}

fn parse_solver_strategy_block(
    lines: &[String],
    start: usize,
) -> Result<(usize, SolverSurfaceStrategy), DiagnosticReport> {
    let mut terms = Vec::new();
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if line == "}" {
            return Ok((
                i + 1,
                SolverSurfaceStrategy {
                    terms,
                    deadends: Vec::new(),
                },
            ));
        }
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        terms.push(parse_solver_strategy_row(line)?);
        i += 1;
    }
    Err(parse_error(
        &lines[start],
        "solver strategy block missing closing brace",
    ))
}

fn parse_solver_strategy_row(line: &str) -> Result<SolverSurfaceStrategyTerm, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    let Some(verb) = tokens.first().copied() else {
        return Err(parse_error(line, "solver strategy row is empty"));
    };
    let direction = match verb {
        "maximize" => SolverStrategyDirection::Maximize,
        "minimize" => SolverStrategyDirection::Minimize,
        "prefer" => SolverStrategyDirection::Prefer,
        "avoid" => SolverStrategyDirection::Avoid,
        _ => {
            return Err(parse_error(
                line,
                "solver strategy row must start with maximize, minimize, prefer, or avoid",
            ));
        }
    };
    let rest = line
        .trim_start()
        .strip_prefix(verb)
        .expect("verb came from the line prefix")
        .trim();
    if rest.is_empty() {
        return Err(parse_error(line, "solver strategy row missing query"));
    }
    let (value, weight) = parse_solver_strategy_value_and_weight(rest, line)?;
    Ok(SolverSurfaceStrategyTerm {
        source_line: line.to_string(),
        direction,
        value: parse_query_expr(value, line)?,
        weight,
    })
}

fn parse_solver_strategy_value_and_weight<'a>(
    value: &'a str,
    line: &str,
) -> Result<(&'a str, i64), DiagnosticReport> {
    let Some((value, weight)) = value.rsplit_once(" weight ") else {
        return Ok((value.trim(), 1));
    };
    let weight = weight.trim().parse::<i64>().map_err(|_| {
        parse_error(
            line,
            "solver strategy weight must be an integer: weight <int>",
        )
    })?;
    if weight <= 0 {
        return Err(parse_error(line, "solver strategy weight must be positive"));
    }
    let value = value.trim();
    if value.is_empty() {
        return Err(parse_error(line, "solver strategy row missing query"));
    }
    Ok((value, weight))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solver_block_parses_direct_deadend_queries() {
        let lines = [
            "solver {",
            "deadend blocked",
            "deadend any {",
            "blocked",
            "other_blocked",
            "}",
            "strategy {",
            "avoid blocked",
            "}",
            "}",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();

        let (next, solver) = parse_solver_block(&lines, 0).unwrap();
        let entry_solver = parse_solver_entry_body(&lines[1..lines.len() - 1]).unwrap();

        assert_eq!(next, lines.len());
        assert_eq!(entry_solver, solver);
        assert_eq!(solver.terms.len(), 1);
        assert_eq!(solver.deadends.len(), 2);
        assert!(matches!(
            solver.deadends[0].values[0],
            SolverSurfaceQueryExpr::Named(ref name) if name == "blocked"
        ));
        assert_eq!(
            solver.deadends[1].combinator,
            SolverSurfaceDeadendCombinator::Any
        );
        assert_eq!(solver.deadends[1].values.len(), 2);
    }
}

fn parse_query_expr(expr: &str, line: &str) -> Result<SolverSurfaceQueryExpr, DiagnosticReport> {
    if let Some((left, op, right)) = split_comparison(expr) {
        let right = parse_query_literal(right.trim(), line)?;
        return Ok(SolverSurfaceQueryExpr::Compare {
            left: Box::new(parse_query_expr(left.trim(), line)?),
            op,
            right,
        });
    }

    match puzzle_authoring::parse_optional_call_surface_with_suffix(expr)
        .map_err(|()| parse_error(line, "query expression missing closing )"))?
    {
        Some((call, suffix)) => {
            if !suffix.trim().is_empty() {
                return Err(parse_error(
                    line,
                    "query expression must not have trailing text",
                ));
            }
            if !puzzle_authoring::is_identifier(call.name) {
                return Err(parse_error(
                    line,
                    "query function name must be an identifier",
                ));
            }
            return Ok(SolverSurfaceQueryExpr::Call {
                name: call.name.to_string(),
                args: call
                    .args
                    .into_iter()
                    .map(|arg| parse_query_arg_surface(arg, line))
                    .collect::<Result<Vec<_>, _>>()?,
            });
        }
        None if expr.contains('(') => {
            return Err(parse_error(
                line,
                "query expression must be a name, query function, or comparison",
            ));
        }
        None => {}
    }

    validate_query_name(expr, line)?;
    Ok(SolverSurfaceQueryExpr::Named(expr.to_string()))
}

fn parse_query_arg_surface(
    arg: &str,
    line: &str,
) -> Result<SolverSurfaceQueryArg, DiagnosticReport> {
    let Some(surface) = oriented_pattern_arg_surface(arg, line)? else {
        return Ok(SolverSurfaceQueryArg::Selector(arg.trim().to_string()));
    };
    Ok(SolverSurfaceQueryArg::Pattern(SolverSurfacePatternArg {
        source: arg.to_string(),
        orientation: owned_pattern_orientation(arg, &surface.orientation),
        pattern: arg[surface.pattern].to_string(),
    }))
}

fn owned_pattern_orientation(
    arg: &str,
    orientation: &OrientedPatternArgOrientationSurface,
) -> SolverSurfacePatternOrientation {
    match orientation {
        OrientedPatternArgOrientationSurface::Neutral => SolverSurfacePatternOrientation::Neutral,
        OrientedPatternArgOrientationSurface::Input { axis, .. } => {
            SolverSurfacePatternOrientation::Input {
                axis: axis.as_ref().map(|axis| arg[axis.clone()].to_string()),
            }
        }
        OrientedPatternArgOrientationSurface::Orientation { orientation } => {
            SolverSurfacePatternOrientation::Orientation(arg[orientation.clone()].to_string())
        }
    }
}

pub(crate) fn oriented_pattern_arg_surface(
    arg: &str,
    line: &str,
) -> Result<Option<OrientedPatternArgSurface>, DiagnosticReport> {
    let trimmed = strip_wrapping_pattern_parens_range(arg, trim_arg_range(arg, 0..arg.len()));
    if arg[trimmed.clone()].starts_with('[') {
        return Ok(Some(OrientedPatternArgSurface {
            orientation: OrientedPatternArgOrientationSurface::Neutral,
            pattern: trimmed,
        }));
    }

    let Some(open_offset) = arg[trimmed.clone()].find('[') else {
        return Ok(None);
    };
    let open_index = trimmed.start + open_offset;
    let orientation = trim_arg_range(arg, trimmed.start..open_index);
    let pattern = trim_arg_range(arg, open_index..trimmed.end);
    if orientation.is_empty() {
        return Ok(Some(OrientedPatternArgSurface {
            orientation: OrientedPatternArgOrientationSurface::Neutral,
            pattern,
        }));
    }
    let orientation_tokens = pattern_arg_token_spans(arg, orientation.clone());
    let orientation = match orientation_tokens.as_slice() {
        [input] if input.text == "input" => OrientedPatternArgOrientationSurface::Input {
            input: input.range.clone(),
            axis: None,
        },
        [input, axis] if input.text == "input" => {
            if !puzzle_authoring::is_identifier(axis.text) {
                return Err(parse_error(
                    line,
                    "input orientation set must be a single identifier",
                ));
            }
            OrientedPatternArgOrientationSurface::Input {
                input: input.range.clone(),
                axis: Some(axis.range.clone()),
            }
        }
        _ => {
            let orientation_text = &arg[orientation.clone()];
            if !puzzle_authoring::is_identifier(orientation_text) {
                return Err(parse_error(
                    line,
                    "pattern orientation must be a single identifier or input <set>",
                ));
            }
            OrientedPatternArgOrientationSurface::Orientation { orientation }
        }
    };
    if pattern_has_embedded_direction_marker(&arg[pattern.clone()]) {
        return Err(parse_error(
            line,
            "pattern cannot combine orientation prefix and embedded direction marker",
        ));
    }
    Ok(Some(OrientedPatternArgSurface {
        orientation,
        pattern,
    }))
}

fn strip_wrapping_pattern_parens_range(
    value: &str,
    range: std::ops::Range<usize>,
) -> std::ops::Range<usize> {
    if !value[range.clone()].starts_with('(') {
        return range;
    }
    let after_open_start = range.start + 1;
    let after_open = &value[after_open_start..range.end];
    let Some(close_index) = matching_close_paren(after_open) else {
        return range;
    };
    if close_index + 1 != after_open.len() {
        return range;
    }
    trim_arg_range(value, after_open_start..after_open_start + close_index)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PatternArgTokenSpan<'a> {
    text: &'a str,
    range: std::ops::Range<usize>,
}

fn pattern_arg_token_spans(
    arg: &str,
    range: std::ops::Range<usize>,
) -> Vec<PatternArgTokenSpan<'_>> {
    let mut tokens = Vec::new();
    let mut index = range.start;
    while index < range.end {
        let Some(start_offset) = arg[index..range.end].find(|ch: char| !ch.is_whitespace()) else {
            break;
        };
        let start = index + start_offset;
        let end = arg[start..range.end]
            .find(char::is_whitespace)
            .map_or(range.end, |offset| start + offset);
        tokens.push(PatternArgTokenSpan {
            text: &arg[start..end],
            range: start..end,
        });
        index = end;
    }
    tokens
}

fn trim_arg_range(arg: &str, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let start = arg[range.clone()]
        .find(|ch: char| !ch.is_whitespace())
        .map_or(range.end, |offset| range.start + offset);
    let end = arg[start..range.end]
        .rfind(|ch: char| !ch.is_whitespace())
        .map(|offset| {
            let index = start + offset;
            index + arg[index..].chars().next().map(char::len_utf8).unwrap_or(0)
        })
        .unwrap_or(start);
    start..end
}

fn matching_close_paren(value: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(index),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn pattern_has_embedded_direction_marker(pattern: &str) -> bool {
    let trimmed = pattern.trim();
    let Some(after_open) = trimmed.strip_prefix('[') else {
        return false;
    };
    let rest = after_open.trim_start();
    let Some(marker) = rest.chars().next() else {
        return false;
    };
    if !matches!(marker, '>' | '<' | '^' | 'v') {
        return false;
    }
    let marker_len = marker.len_utf8();
    rest[marker_len..]
        .chars()
        .next()
        .is_some_and(char::is_whitespace)
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

fn parse_query_literal(token: &str, line: &str) -> Result<i64, DiagnosticReport> {
    match token {
        "true" => Ok(1),
        "false" => Ok(0),
        _ => token
            .parse()
            .map_err(|_| parse_error(line, "expected true, false, or integer")),
    }
}

fn validate_query_name(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    if puzzle_authoring::is_qualified_identifier(value) {
        Ok(())
    } else {
        Err(parse_error(
            line,
            "query name must be a qualified identifier",
        ))
    }
}

fn parse_error(line: &str, message: &str) -> DiagnosticReport {
    DiagnosticReport::error_at_line(message, line)
}
