pub mod compiled_game;
pub mod ids;
pub mod patch;
pub mod state;
pub mod transition;

pub use compiled_game::{
    ComparisonOp, CompiledGame, ConditionDef, ConditionValueKind, Effect, GapTerm, Guard,
    LocalFrame, LocalFrameExtent, MarkDef, MarkKind, MarkPattern, MarkValueMatch, MatchCell,
    ObjectDef, ObjectSetMarkPattern, ObjectSetMatcher, Offset, Pattern, Pattern2, PatternComponent,
    Rule, RuleApplication, RuleCondition, RuleStep, VariableUpdateOp, WriteOp,
};
pub use ids::{ConditionId, InputId, LayerId, MarkId, ObjectId, RuleId, VariableId};
pub use patch::{Patch, PatchError, PatchOp};
pub use state::{CellView, State, StateError};
pub use transition::{
    ProgramBoundarySnapshot, ProgramContinuation, ProgramSegmentTrace, StepTrace,
    TransitionCommand, TransitionError, TransitionOutcome, TransitionResult, count_pattern_matches,
    has_pattern_match, transition_outcome, transition_program,
    transition_program_continuation_segment_trace, transition_program_outcome,
    transition_program_segment_trace, transition_program_trace, transition_solver_outcome,
    transition_solver_state, transition_state, transition_trace,
};
