use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchBudget {
    pub max_depth: Option<u32>,
    pub max_stored_nodes: Option<usize>,
    pub max_frontier: Option<usize>,
    pub max_duration: Option<Duration>,
}

impl SearchBudget {
    pub fn bounded(max_depth: u32, max_stored_nodes: usize, max_duration: Duration) -> Self {
        Self {
            max_depth: Some(max_depth),
            max_stored_nodes: Some(max_stored_nodes),
            max_frontier: None,
            max_duration: Some(max_duration),
        }
    }
}
