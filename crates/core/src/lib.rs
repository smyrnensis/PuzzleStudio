pub mod compiled_game;
pub mod ids;
pub mod patch;
pub mod state;
pub mod transition;

pub use compiled_game::{
    ComparisonOp, CompiledGame, Effect, GapTerm, GlobalUpdateOp, Guard, LocalFrame,
    LocalFrameExtent, MatchCell, ObjectDef, ObjectSetMatcher, ObjectSetScratchPattern, Offset,
    Pattern, PatternComponent, QueryDef, QueryKind, Rule, RuleApplication, RuleCondition, RuleStep,
    ScratchDef, ScratchKind, ScratchPattern, ScratchValueMatch, WriteOp,
};
pub use ids::{GlobalId, InputId, LayerId, ObjectId, QueryId, RuleId, ScratchId};
pub use patch::{Patch, PatchError, PatchOp};
pub use state::{CellView, State, StateError};
pub use transition::{
    StepTrace, TransitionCommand, TransitionError, TransitionOutcome, TransitionResult,
    count_pattern_matches, has_pattern_match, transition_outcome, transition_program,
    transition_program_outcome, transition_program_trace, transition_solver_state,
    transition_state, transition_trace,
};
