use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness<Action> {
    pub actions: Vec<Action>,
    pub depth: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchStats {
    pub visited: usize,
    pub expanded: usize,
    pub frontier: usize,
    pub max_depth_reached: u32,
    pub elapsed: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchFailure<Action, Error> {
    pub action: Action,
    pub depth: u32,
    pub error: Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchOutcome<Action, Error> {
    Solved(Witness<Action>),
    Exhausted(SearchStats),
    BudgetExceeded(SearchStats),
    Failed(SearchFailure<Action, Error>),
}
