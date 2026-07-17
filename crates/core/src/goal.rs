use serde::{Deserialize, Serialize};

use crate::grid_transition::eval_condition_kind;
use crate::{
    ComparisonOp, ConditionId, GridCompiledGame, GridConditionValueKind, GridSize, GridState,
    VariableId,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalConditionOf<Value> {
    pub description: String,
    pub expr: GoalExprOf<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalExprOf<Value> {
    All(Vec<GoalExprOf<Value>>),
    Any(Vec<GoalExprOf<Value>>),
    Clause(GoalClauseOf<Value>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalClauseOf<Value> {
    pub value: GoalValueOf<Value>,
    pub op: ComparisonOp,
    pub expected: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalValueOf<Value> {
    Variable(VariableId),
    Condition(ConditionId),
    InlineConditionValue(Value),
}

pub type GridGoalCondition<const D: usize> = GoalConditionOf<GridConditionValueKind<D>>;
pub type GridGoalExpr<const D: usize> = GoalExprOf<GridConditionValueKind<D>>;
pub type GridGoalClause<const D: usize> = GoalClauseOf<GridConditionValueKind<D>>;
pub type GridGoalValue<const D: usize> = GoalValueOf<GridConditionValueKind<D>>;

impl<Value> GoalConditionOf<Value> {
    pub fn try_map_value<Mapped, Error>(
        &self,
        map: &mut impl FnMut(&Value) -> Result<Mapped, Error>,
    ) -> Result<GoalConditionOf<Mapped>, Error> {
        Ok(GoalConditionOf {
            description: self.description.clone(),
            expr: try_map_goal_expr(&self.expr, map)?,
        })
    }
}

impl<const D: usize> GridGoalCondition<D> {
    pub fn is_met<Size: GridSize<D>>(
        &self,
        game: &GridCompiledGame<D>,
        state: &GridState<D, Size>,
    ) -> bool {
        eval_goal_expr(game, state, &self.expr)
    }
}

fn try_map_goal_expr<Value, Mapped, Error>(
    value: &GoalExprOf<Value>,
    map: &mut impl FnMut(&Value) -> Result<Mapped, Error>,
) -> Result<GoalExprOf<Mapped>, Error> {
    Ok(match value {
        GoalExprOf::All(values) => GoalExprOf::All(
            values
                .iter()
                .map(|value| try_map_goal_expr(value, map))
                .collect::<Result<_, _>>()?,
        ),
        GoalExprOf::Any(values) => GoalExprOf::Any(
            values
                .iter()
                .map(|value| try_map_goal_expr(value, map))
                .collect::<Result<_, _>>()?,
        ),
        GoalExprOf::Clause(clause) => GoalExprOf::Clause(GoalClauseOf {
            value: match &clause.value {
                GoalValueOf::Variable(variable) => GoalValueOf::Variable(*variable),
                GoalValueOf::Condition(condition) => GoalValueOf::Condition(*condition),
                GoalValueOf::InlineConditionValue(value) => {
                    GoalValueOf::InlineConditionValue(map(value)?)
                }
            },
            op: clause.op,
            expected: clause.expected,
        }),
    })
}

fn eval_goal_expr<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    expr: &GridGoalExpr<D>,
) -> bool {
    match expr {
        GoalExprOf::All(exprs) => exprs.iter().all(|expr| eval_goal_expr(game, state, expr)),
        GoalExprOf::Any(exprs) => exprs.iter().any(|expr| eval_goal_expr(game, state, expr)),
        GoalExprOf::Clause(clause) => compare_i64(
            eval_goal_value(game, state, &clause.value),
            clause.op,
            clause.expected,
        ),
    }
}

fn eval_goal_value<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    value: &GridGoalValue<D>,
) -> i64 {
    match value {
        GoalValueOf::Variable(variable) => state.variable_value(*variable).unwrap_or(0),
        GoalValueOf::Condition(condition) => game
            .condition_def(*condition)
            .map(|condition| eval_condition_kind(game, state, &condition.kind, None, None))
            .unwrap_or(0),
        GoalValueOf::InlineConditionValue(kind) => {
            eval_condition_kind(game, state, kind, None, None)
        }
    }
}

fn compare_i64(left: i64, op: ComparisonOp, right: i64) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::NotEq => left != right,
        ComparisonOp::Greater => left > right,
        ComparisonOp::GreaterEq => left >= right,
        ComparisonOp::Less => left < right,
        ComparisonOp::LessEq => left <= right,
    }
}
