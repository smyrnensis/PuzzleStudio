pub mod compiled_game;
pub mod goal;
pub mod grid_transition;
pub mod ids;
pub mod model;
pub mod patch;
pub mod state;
#[cfg(test)]
mod transition;

pub use compiled_game::{
    ComparisonOp, CompiledGame, ConditionDef, ConditionValueKind, Effect, ExecutableProgram,
    GapTerm, GridCompiledGame, GridConditionDef, GridConditionValueKind, GridExecutableProgram,
    GridGapTerm, GridGuard, GridMatchCell, GridOffset, GridPattern, GridPatternComponent, GridRule,
    GridRuleCondition, GridRuleStep, GridWriteOp, Guard, LocalFrame, LocalFrameExtent, MarkDef,
    MarkKind, MarkPattern, MarkValueMatch, MatchCell, ObjectDef, ObjectSetMarkPattern,
    ObjectSetMatcher, Offset, Pattern, Pattern2, PatternComponent, Rule, RuleApplication,
    RuleCondition, RuleStep, VariableUpdateOp, WriteOp, try_project_grid_compiled_game,
    try_project_grid_condition_value, try_project_grid_program,
};
pub use goal::{
    GoalClauseOf, GoalConditionOf, GoalExprOf, GoalValueOf, GridGoalClause, GridGoalCondition,
    GridGoalExpr, GridGoalValue,
};
pub use grid_transition::{
    GridProgramBoundarySnapshot, GridProgramSegmentTrace, GridTransitionError,
    GridTransitionOutcome, ProgramContinuation, TransitionCommand, count_pattern_matches,
    eval_condition_kind, flattened_rules, has_pattern_match, transition_outcome,
    transition_program, transition_program_continuation_segment_trace, transition_program_outcome,
    transition_program_segment_trace, transition_program_sequence_without_input_outcome,
    transition_solver_outcome, transition_solver_state, transition_state, transition_trace,
};
pub use ids::{ConditionId, InputId, LayerId, MarkId, ObjectId, RuleId, VariableId};
pub use model::{GridInput, GridLevel, GridLevelBundle, GridLevelBundleError};
pub use patch::{GridPatch, GridPatchError, Patch, PatchError, PatchOp};
pub use puzzle_kernel::GridCoord;
pub use state::{
    CellView, Coord3, Delta3, GridSize, GridState, GridStateError, Size2, Size3, SlotMark, State,
    StateError,
};
pub type TransitionError = GridTransitionError<2>;
pub type TransitionResult<T = State> = Result<T, TransitionError>;
pub type TransitionOutcome = GridTransitionOutcome<2, Size2>;
pub type StepTrace = TransitionOutcome;
pub type ProgramBoundarySnapshot<'a> = GridProgramBoundarySnapshot<'a, 2, Size2>;
pub type ProgramSegmentTrace = GridProgramSegmentTrace<2, Size2>;
