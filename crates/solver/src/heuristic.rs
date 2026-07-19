use puzzle_core::{
    ComparisonOp, GridCompiledGame, GridCoord, GridSize, GridState, ObjectId, eval_condition_kind,
};
use puzzle_lang::{GridQueryExpr, LoadedGridGame, QueryExprOf, SolverStrategyDirection};
use std::collections::BTreeSet;

pub fn solver_strategy_object_roots<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
) -> BTreeSet<ObjectId> {
    let mut roots = BTreeSet::new();
    for query in loaded
        .solver_strategy
        .terms
        .iter()
        .map(|term| &term.value)
        .chain(
            loaded
                .solver_strategy
                .deadends
                .iter()
                .flat_map(|deadend| deadend.values()),
        )
    {
        collect_query_object_roots(query, &mut roots);
    }
    roots
}

fn collect_query_object_roots<const D: usize>(
    value: &GridQueryExpr<D>,
    roots: &mut BTreeSet<ObjectId>,
) {
    match value {
        QueryExprOf::Variable(_) => {}
        QueryExprOf::Value(kind) => crate::object_refs::collect_condition_value_roots(kind, roots),
        QueryExprOf::Distance { from, to } => {
            roots.extend(from.iter().copied());
            roots.extend(to.iter().copied());
        }
        QueryExprOf::AllOnDistance { subjects, covers } => {
            roots.extend(subjects.iter().copied());
            roots.extend(covers.iter().copied());
        }
        QueryExprOf::Compare { left, .. } => collect_query_object_roots(left, roots),
    }
}

pub fn solver_strategy_score<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    state: &GridState<D, Size>,
) -> i64 {
    loaded
        .solver_strategy
        .terms
        .iter()
        .map(|term| {
            let value = query_value(&loaded.game, state, &term.value);
            match term.direction {
                SolverStrategyDirection::Maximize => value.saturating_mul(-term.weight),
                SolverStrategyDirection::Minimize => value.saturating_mul(term.weight),
                SolverStrategyDirection::Prefer => {
                    if value != 0 {
                        0
                    } else {
                        term.weight
                    }
                }
                SolverStrategyDirection::Avoid => {
                    if value == 0 {
                        0
                    } else {
                        term.weight
                    }
                }
            }
        })
        .sum()
}

pub fn solver_strategy_has_deadend<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    state: &GridState<D, Size>,
) -> bool {
    loaded
        .solver_strategy
        .has_deadend_with(|query| query_value(&loaded.game, state, query) != 0)
}

fn query_value<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    value: &GridQueryExpr<D>,
) -> i64 {
    query_value_with(
        value,
        &mut |variable| {
            state
                .variable_value(variable)
                .expect("query variable was resolved during lowering")
        },
        &mut |kind| eval_condition_kind(game, state, kind, None, None),
        &mut |from, to| distance(game, state, from, to),
        &mut |subjects, covers| all_on_distance(game, state, subjects, covers),
    )
}

fn query_value_with<Object, Value, Variable, EvalVariable, EvalValue, EvalDistance, EvalAllOn>(
    value: &QueryExprOf<Object, Value, Variable>,
    eval_variable: &mut EvalVariable,
    eval_value: &mut EvalValue,
    eval_distance: &mut EvalDistance,
    eval_all_on: &mut EvalAllOn,
) -> i64
where
    Variable: Copy,
    EvalVariable: FnMut(Variable) -> i64,
    EvalValue: FnMut(&Value) -> i64,
    EvalDistance: FnMut(&[Object], &[Object]) -> i64,
    EvalAllOn: FnMut(&[Object], &[Object]) -> i64,
{
    match value {
        QueryExprOf::Variable(variable) => eval_variable(*variable),
        QueryExprOf::Value(kind) => eval_value(kind),
        QueryExprOf::Distance { from, to } => eval_distance(from, to),
        QueryExprOf::AllOnDistance { subjects, covers } => eval_all_on(subjects, covers),
        QueryExprOf::Compare { left, op, right } => {
            let left =
                query_value_with(left, eval_variable, eval_value, eval_distance, eval_all_on);
            if compare(left, *op, *right) { 1 } else { 0 }
        }
    }
}

fn compare(left: i64, op: ComparisonOp, right: i64) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::NotEq => left != right,
        ComparisonOp::Greater => left > right,
        ComparisonOp::GreaterEq => left >= right,
        ComparisonOp::Less => left < right,
        ComparisonOp::LessEq => left <= right,
    }
}

fn distance<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    from: &[ObjectId],
    to: &[ObjectId],
) -> i64 {
    let from_positions = object_positions(game, state, from);
    let to_positions = object_positions(game, state, to);
    let fallback = state.size.axes().into_iter().map(i64::from).sum();
    from_positions
        .iter()
        .flat_map(|a| to_positions.iter().map(move |b| manhattan(*a, *b)))
        .min()
        .unwrap_or(fallback)
}

fn all_on_distance<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    subjects: &[ObjectId],
    covers: &[ObjectId],
) -> i64 {
    let cover_positions = object_positions(game, state, covers);
    let fallback = state.size.axes().into_iter().map(i64::from).sum();
    object_positions(game, state, subjects)
        .into_iter()
        .filter(|position| {
            !covers
                .iter()
                .any(|cover| state.has_object_at(game, *position, *cover))
        })
        .map(|position| {
            cover_positions
                .iter()
                .map(|cover| manhattan(position, *cover))
                .min()
                .unwrap_or(fallback)
                .max(1)
        })
        .sum()
}

fn object_positions<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    objects: &[ObjectId],
) -> Vec<GridCoord<D>> {
    let mut positions = Vec::new();
    for object in objects {
        for slot in state.object_positions(*object) {
            let Some(position) = state.slot_coord(*slot) else {
                continue;
            };
            if !positions.contains(&position) && state.has_object_at(game, position, *object) {
                positions.push(position);
            }
        }
    }
    positions
}

fn manhattan<const D: usize>(left: GridCoord<D>, right: GridCoord<D>) -> i64 {
    left.axes()
        .into_iter()
        .zip(right.axes())
        .map(|(a, b)| i64::from(a.abs_diff(b)))
        .sum()
}
