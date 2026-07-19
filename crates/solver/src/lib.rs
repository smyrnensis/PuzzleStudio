pub mod best_first;
pub mod bfs;
pub mod budget;
pub mod domain;
pub mod grid_domain;
pub mod heuristic;
pub mod object_refs;
#[cfg(test)]
mod puzzle_domain;
pub mod relevance;
pub mod report;
pub mod slice;
mod stable_hash;
pub mod stage_availability;
pub mod state_slicer;

pub use best_first::{
    ResumableAdvanceOutcome, ResumableBestFirst, ResumablePauseReason, ResumableSearchAllowance,
    ResumableSearchCandidate, ResumableSearchLimits, ResumableSearchStatus, ScanControl,
    ScanOutcome, SearchMatch, best_first, best_first_scan_with_dead_states_and_progress,
    best_first_with_dead_states, best_first_with_dead_states_and_progress,
};
pub use bfs::exact_bfs;
pub use budget::SearchBudget;
pub use domain::SearchDomain;
pub use grid_domain::{GridPuzzleDomain, GridSearchGoal, GridSearchState, GridStateKey};
pub use heuristic::{
    solver_strategy_has_deadend, solver_strategy_object_roots, solver_strategy_score,
};
pub use relevance::SolverRelevance;
pub use report::{SearchFailure, SearchOutcome, SearchProgress, SearchStats, Witness};
pub use slice::SolverSlice;
pub use stage_availability::SolverStageAvailability;
pub use state_slicer::{SolverStateProjection, SolverStateSlicer};
