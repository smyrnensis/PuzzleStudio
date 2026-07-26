use crate::budget::SearchBudget;
use crate::domain::SearchDomain;
use crate::report::{SearchFailure, SearchOutcome, SearchProgress, SearchStats, Witness};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::time::Duration;
use std::time::Instant;

#[derive(Clone, Debug)]
struct NodeRecord<State, Action> {
    state: State,
    parent: Option<usize>,
    action: Option<Action>,
    depth: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct QueueEntry {
    score: i64,
    depth: u32,
    sequence: usize,
    node_index: usize,
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .score
            .cmp(&self.score)
            .then_with(|| other.depth.cmp(&self.depth))
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn best_first<D, F>(
    domain: &mut D,
    initial: D::State,
    budget: SearchBudget,
    score: F,
) -> SearchOutcome<D::Action, D::Error>
where
    D: SearchDomain,
    F: FnMut(&D::State) -> i64,
{
    best_first_with_dead_states(domain, initial, budget, score, |_| false)
}

pub fn best_first_with_dead_states<D, F, P>(
    domain: &mut D,
    initial: D::State,
    budget: SearchBudget,
    score: F,
    is_dead: P,
) -> SearchOutcome<D::Action, D::Error>
where
    D: SearchDomain,
    F: FnMut(&D::State) -> i64,
    P: FnMut(&D::State) -> bool,
{
    best_first_with_dead_states_and_progress(domain, initial, budget, score, is_dead, |_, _| {})
}

pub fn best_first_with_dead_states_and_progress<D, F, P, O>(
    domain: &mut D,
    initial: D::State,
    budget: SearchBudget,
    mut score: F,
    mut is_dead: P,
    mut on_progress: O,
) -> SearchOutcome<D::Action, D::Error>
where
    D: SearchDomain,
    F: FnMut(&D::State) -> i64,
    P: FnMut(&D::State) -> bool,
    O: FnMut(&D::State, SearchProgress),
{
    let started_at = budget.max_duration.map(|_| Instant::now());
    let initial_key = domain.key(&initial);
    let mut nodes = vec![NodeRecord {
        state: initial,
        parent: None,
        action: None,
        depth: 0,
    }];
    let mut visited = HashMap::from([(initial_key, 0_usize)]);
    let mut frontier = BinaryHeap::from([QueueEntry {
        score: score(&nodes[0].state),
        depth: 0,
        sequence: 0,
        node_index: 0,
    }]);
    let mut expanded = 0_usize;
    let mut max_depth_reached = 0_u32;
    let mut sequence = 1_usize;
    let mut depth_budget_hit = false;

    if is_dead(&nodes[0].state) {
        return SearchOutcome::Exhausted(stats(
            started_at,
            visited.len(),
            frontier.len(),
            expanded,
            max_depth_reached,
        ));
    }

    if domain.is_goal(&nodes[0].state) {
        return SearchOutcome::Solved(Witness {
            actions: Vec::new(),
            depth: 0,
        });
    }

    loop {
        if let Some(stats) = budget_exceeded(
            budget,
            started_at,
            visited.len(),
            frontier.len(),
            expanded,
            max_depth_reached,
        ) {
            return SearchOutcome::BudgetExceeded(stats);
        }

        let Some(entry) = frontier.pop() else {
            let stats = stats(
                started_at,
                visited.len(),
                frontier.len(),
                expanded,
                max_depth_reached,
            );
            return if depth_budget_hit {
                SearchOutcome::BudgetExceeded(stats)
            } else {
                SearchOutcome::Exhausted(stats)
            };
        };

        let current_index = entry.node_index;
        let current_depth = nodes[current_index].depth;
        max_depth_reached = max_depth_reached.max(current_depth);
        if budget
            .max_depth
            .is_some_and(|max_depth| current_depth >= max_depth)
        {
            depth_budget_hit = true;
            continue;
        }

        expanded += 1;
        on_progress(
            &nodes[current_index].state,
            SearchProgress {
                visited: visited.len(),
                expanded,
                frontier: frontier.len(),
                max_depth_reached,
                depth: current_depth,
            },
        );
        let current_key = domain.key(&nodes[current_index].state);
        let actions = domain.actions(&nodes[current_index].state).to_vec();
        for action in actions {
            let next = match domain.step(&nodes[current_index].state, &action) {
                Ok(next) => next,
                Err(error) => {
                    return SearchOutcome::Failed(SearchFailure {
                        action,
                        depth: current_depth + 1,
                        error,
                    });
                }
            };

            let next_depth = current_depth + 1;
            if is_dead(&next) {
                continue;
            }
            if domain.is_goal(&next) {
                let next_index = nodes.len();
                nodes.push(NodeRecord {
                    state: next,
                    parent: Some(current_index),
                    action: Some(action),
                    depth: next_depth,
                });
                return SearchOutcome::Solved(reconstruct_witness(&nodes, next_index));
            }
            let next_key = domain.key(&next);
            if next_key == current_key || visited.contains_key(&next_key) {
                continue;
            }

            let next_index = nodes.len();
            nodes.push(NodeRecord {
                state: next,
                parent: Some(current_index),
                action: Some(action),
                depth: next_depth,
            });
            visited.insert(next_key, next_index);
            max_depth_reached = max_depth_reached.max(next_depth);

            frontier.push(QueueEntry {
                score: score(&nodes[next_index].state),
                depth: next_depth,
                sequence,
                node_index: next_index,
            });
            sequence += 1;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchMatch<State, Action> {
    pub state: State,
    pub actions: Vec<Action>,
    pub depth: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanControl {
    Continue,
    Stop,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanOutcome<Action, Error> {
    Completed {
        stats: SearchStats,
    },
    Stopped {
        stats: SearchStats,
    },
    BudgetExceeded {
        stats: SearchStats,
    },
    Failed {
        failure: SearchFailure<Action, Error>,
    },
}

pub fn best_first_scan_with_dead_states_and_progress<D, F, P, S, O>(
    domain: &mut D,
    initial: D::State,
    budget: SearchBudget,
    mut score: F,
    mut is_dead: P,
    mut on_discovered: S,
    mut on_progress: O,
) -> ScanOutcome<D::Action, D::Error>
where
    D: SearchDomain,
    D::State: Clone,
    D::Action: Clone,
    F: FnMut(&D::State) -> i64,
    P: FnMut(&D::State) -> bool,
    S: FnMut(SearchMatch<D::State, D::Action>) -> ScanControl,
    O: FnMut(&D::State, SearchProgress),
{
    let started_at = budget.max_duration.map(|_| Instant::now());
    let initial_key = domain.key(&initial);
    let mut nodes = vec![NodeRecord {
        state: initial,
        parent: None,
        action: None,
        depth: 0,
    }];
    let mut visited = HashMap::from([(initial_key, 0_usize)]);
    let mut frontier = BinaryHeap::from([QueueEntry {
        score: score(&nodes[0].state),
        depth: 0,
        sequence: 0,
        node_index: 0,
    }]);
    let mut expanded = 0_usize;
    let mut max_depth_reached = 0_u32;
    let mut sequence = 1_usize;
    let mut depth_budget_hit = false;

    if is_dead(&nodes[0].state) {
        return ScanOutcome::Completed {
            stats: stats(
                started_at,
                visited.len(),
                frontier.len(),
                expanded,
                max_depth_reached,
            ),
        };
    }

    if on_discovered(reconstruct_match(&nodes, 0)) == ScanControl::Stop {
        return ScanOutcome::Stopped {
            stats: stats(
                started_at,
                visited.len(),
                frontier.len(),
                expanded,
                max_depth_reached,
            ),
        };
    }

    loop {
        if let Some(stats) = budget_exceeded(
            budget,
            started_at,
            visited.len(),
            frontier.len(),
            expanded,
            max_depth_reached,
        ) {
            return ScanOutcome::BudgetExceeded { stats };
        }

        let Some(entry) = frontier.pop() else {
            let stats = stats(
                started_at,
                visited.len(),
                frontier.len(),
                expanded,
                max_depth_reached,
            );
            return if depth_budget_hit {
                ScanOutcome::BudgetExceeded { stats }
            } else {
                ScanOutcome::Completed { stats }
            };
        };

        let current_index = entry.node_index;
        let current_depth = nodes[current_index].depth;
        max_depth_reached = max_depth_reached.max(current_depth);
        if budget
            .max_depth
            .is_some_and(|max_depth| current_depth >= max_depth)
        {
            depth_budget_hit = true;
            continue;
        }

        expanded += 1;
        on_progress(
            &nodes[current_index].state,
            SearchProgress {
                visited: visited.len(),
                expanded,
                frontier: frontier.len(),
                max_depth_reached,
                depth: current_depth,
            },
        );
        let current_key = domain.key(&nodes[current_index].state);
        let actions = domain.actions(&nodes[current_index].state).to_vec();
        for action in actions {
            let next = match domain.step(&nodes[current_index].state, &action) {
                Ok(next) => next,
                Err(error) => {
                    return ScanOutcome::Failed {
                        failure: SearchFailure {
                            action,
                            depth: current_depth + 1,
                            error,
                        },
                    };
                }
            };

            if is_dead(&next) {
                continue;
            }

            let next_key = domain.key(&next);
            if next_key == current_key || visited.contains_key(&next_key) {
                continue;
            }

            let next_depth = current_depth + 1;
            let next_index = nodes.len();
            nodes.push(NodeRecord {
                state: next,
                parent: Some(current_index),
                action: Some(action),
                depth: next_depth,
            });
            visited.insert(next_key, next_index);
            max_depth_reached = max_depth_reached.max(next_depth);

            if on_discovered(reconstruct_match(&nodes, next_index)) == ScanControl::Stop {
                return ScanOutcome::Stopped {
                    stats: stats(
                        started_at,
                        visited.len(),
                        frontier.len(),
                        expanded,
                        max_depth_reached,
                    ),
                };
            }

            frontier.push(QueueEntry {
                score: score(&nodes[next_index].state),
                depth: next_depth,
                sequence,
                node_index: next_index,
            });
            sequence += 1;
        }
    }
}

fn reconstruct_witness<State, Action: Clone>(
    nodes: &[NodeRecord<State, Action>],
    mut index: usize,
) -> Witness<Action> {
    let depth = nodes[index].depth;
    let mut actions = Vec::new();
    while let Some(parent) = nodes[index].parent {
        if let Some(action) = &nodes[index].action {
            actions.push(action.clone());
        }
        index = parent;
    }
    actions.reverse();
    Witness { actions, depth }
}

fn reconstruct_match<State: Clone, Action: Clone>(
    nodes: &[NodeRecord<State, Action>],
    index: usize,
) -> SearchMatch<State, Action> {
    let witness = reconstruct_witness(nodes, index);
    SearchMatch {
        state: nodes[index].state.clone(),
        actions: witness.actions,
        depth: witness.depth,
    }
}

fn budget_exceeded(
    budget: SearchBudget,
    started_at: Option<Instant>,
    visited: usize,
    frontier: usize,
    expanded: usize,
    max_depth_reached: u32,
) -> Option<SearchStats> {
    let elapsed = started_at
        .map(|started_at| started_at.elapsed())
        .unwrap_or_default();
    if budget
        .max_stored_nodes
        .is_some_and(|max_stored_nodes| visited >= max_stored_nodes)
        || budget
            .max_frontier
            .is_some_and(|max_frontier| frontier >= max_frontier)
        || budget
            .max_duration
            .is_some_and(|max_duration| elapsed >= max_duration)
    {
        return Some(SearchStats {
            visited,
            expanded,
            frontier,
            max_depth_reached,
            elapsed,
        });
    }
    None
}

fn stats(
    started_at: Option<Instant>,
    visited: usize,
    frontier: usize,
    expanded: usize,
    max_depth_reached: u32,
) -> SearchStats {
    let elapsed = started_at
        .map(|started_at| started_at.elapsed())
        .unwrap_or_default();
    SearchStats {
        visited,
        expanded,
        frontier,
        max_depth_reached,
        elapsed,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResumableSearchLimits {
    pub max_depth: u32,
    pub max_stored_nodes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResumableSearchAllowance {
    pub max_expanded_nodes: usize,
    pub max_duration: Option<Duration>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResumablePauseReason {
    ExpandedNodes,
    Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResumableSearchStatus {
    Active,
    Solved { candidate_index: usize },
    Exhausted,
    ResourceLimit,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumableSearchCandidate<State, Action> {
    pub index: usize,
    pub state: State,
    pub actions: Vec<Action>,
    pub score: i64,
    pub depth: u32,
    pub discovery_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResumableAdvanceOutcome<Action, Error> {
    Paused {
        reason: ResumablePauseReason,
        stats: SearchStats,
    },
    Solved {
        candidate_index: usize,
        stats: SearchStats,
    },
    Exhausted {
        stats: SearchStats,
    },
    ResourceLimit {
        stats: SearchStats,
    },
    Failed {
        failure: SearchFailure<Action, Error>,
        stats: SearchStats,
    },
}

#[derive(Clone, Debug)]
struct ResumableNode<State, Action> {
    state: State,
    parent: Option<usize>,
    action: Option<Action>,
    score: i64,
    depth: u32,
    discovery_index: usize,
}

pub struct ResumableBestFirst<State, Action, Key> {
    limits: ResumableSearchLimits,
    nodes: Vec<ResumableNode<State, Action>>,
    visited: HashMap<Key, usize>,
    frontier: BinaryHeap<QueueEntry>,
    next_sequence: usize,
    expanded: usize,
    max_depth_reached: u32,
    elapsed: Duration,
    status: ResumableSearchStatus,
    initial_goal_checked: bool,
}

impl<State, Action, Key> ResumableBestFirst<State, Action, Key>
where
    State: Clone,
    Action: Clone,
    Key: Eq + std::hash::Hash,
{
    pub fn new(
        initial: State,
        initial_key: Key,
        initial_score: i64,
        limits: ResumableSearchLimits,
    ) -> Self {
        let nodes = vec![ResumableNode {
            state: initial,
            parent: None,
            action: None,
            score: initial_score,
            depth: 0,
            discovery_index: 0,
        }];
        Self {
            limits,
            nodes,
            visited: HashMap::from([(initial_key, 0)]),
            frontier: BinaryHeap::from([QueueEntry {
                score: initial_score,
                depth: 0,
                sequence: 0,
                node_index: 0,
            }]),
            next_sequence: 1,
            expanded: 0,
            max_depth_reached: 0,
            elapsed: Duration::ZERO,
            status: ResumableSearchStatus::Active,
            initial_goal_checked: false,
        }
    }

    pub fn status(&self) -> ResumableSearchStatus {
        self.status
    }

    pub fn stats(&self) -> SearchStats {
        SearchStats {
            visited: self.visited.len(),
            expanded: self.expanded,
            frontier: self.frontier.len(),
            max_depth_reached: self.max_depth_reached,
            elapsed: self.elapsed,
        }
    }

    pub fn candidate(&self, index: usize) -> Option<ResumableSearchCandidate<State, Action>> {
        let node = self.nodes.get(index)?;
        let witness = resumable_witness(&self.nodes, index);
        Some(ResumableSearchCandidate {
            index,
            state: node.state.clone(),
            actions: witness.actions,
            score: node.score,
            depth: node.depth,
            discovery_index: node.discovery_index,
        })
    }

    pub fn best_candidates(&self, limit: usize) -> Vec<ResumableSearchCandidate<State, Action>> {
        let mut indices = (0..self.nodes.len()).collect::<Vec<_>>();
        indices.sort_by_key(|index| {
            let node = &self.nodes[*index];
            (node.score, node.depth, node.discovery_index)
        });
        indices.truncate(limit);
        indices
            .into_iter()
            .filter_map(|index| self.candidate(index))
            .collect()
    }

    pub fn advance<D, F>(
        &mut self,
        domain: &mut D,
        allowance: ResumableSearchAllowance,
        score: F,
    ) -> ResumableAdvanceOutcome<Action, D::Error>
    where
        D: SearchDomain<State = State, Action = Action, Key = Key>,
        F: FnMut(&State) -> i64,
    {
        self.advance_with_dead_states(domain, allowance, score, |_| false)
    }

    pub fn advance_with_dead_states<D, F, IsDead>(
        &mut self,
        domain: &mut D,
        allowance: ResumableSearchAllowance,
        mut score: F,
        mut is_dead: IsDead,
    ) -> ResumableAdvanceOutcome<Action, D::Error>
    where
        D: SearchDomain<State = State, Action = Action, Key = Key>,
        F: FnMut(&State) -> i64,
        IsDead: FnMut(&State) -> bool,
    {
        assert!(
            matches!(self.status, ResumableSearchStatus::Active),
            "terminal resumable search must not be advanced"
        );
        assert!(allowance.max_expanded_nodes > 0);
        assert!(
            allowance
                .max_duration
                .is_none_or(|duration| !duration.is_zero())
        );

        let started_at = allowance.max_duration.map(|_| Instant::now());
        let expanded_before = self.expanded;
        if !self.initial_goal_checked {
            self.initial_goal_checked = true;
            if is_dead(&self.nodes[0].state) {
                self.status = ResumableSearchStatus::Exhausted;
                self.frontier.clear();
                self.elapsed += resumable_elapsed(started_at);
                return ResumableAdvanceOutcome::Exhausted {
                    stats: self.stats(),
                };
            }
            if domain.is_goal(&self.nodes[0].state) {
                self.status = ResumableSearchStatus::Solved { candidate_index: 0 };
                self.elapsed += resumable_elapsed(started_at);
                return ResumableAdvanceOutcome::Solved {
                    candidate_index: 0,
                    stats: self.stats(),
                };
            }
        }

        loop {
            if self.expanded - expanded_before >= allowance.max_expanded_nodes {
                self.elapsed += resumable_elapsed(started_at);
                return ResumableAdvanceOutcome::Paused {
                    reason: ResumablePauseReason::ExpandedNodes,
                    stats: self.stats(),
                };
            }
            if allowance
                .max_duration
                .is_some_and(|duration| resumable_elapsed(started_at) >= duration)
            {
                self.elapsed += resumable_elapsed(started_at);
                return ResumableAdvanceOutcome::Paused {
                    reason: ResumablePauseReason::Duration,
                    stats: self.stats(),
                };
            }

            let Some(entry) = self.frontier.pop() else {
                self.status = ResumableSearchStatus::Exhausted;
                self.elapsed += resumable_elapsed(started_at);
                return ResumableAdvanceOutcome::Exhausted {
                    stats: self.stats(),
                };
            };
            let current_index = entry.node_index;
            let current_depth = self.nodes[current_index].depth;
            self.max_depth_reached = self.max_depth_reached.max(current_depth);
            if current_depth >= self.limits.max_depth {
                continue;
            }

            self.expanded += 1;
            let current_key = domain.key(&self.nodes[current_index].state);
            let actions = domain.actions(&self.nodes[current_index].state).to_vec();
            for action in actions {
                let next = match domain.step(&self.nodes[current_index].state, &action) {
                    Ok(next) => next,
                    Err(error) => {
                        self.status = ResumableSearchStatus::Failed;
                        self.elapsed += resumable_elapsed(started_at);
                        return ResumableAdvanceOutcome::Failed {
                            failure: SearchFailure {
                                action,
                                depth: current_depth + 1,
                                error,
                            },
                            stats: self.stats(),
                        };
                    }
                };
                let next_key = domain.key(&next);
                if next_key == current_key || self.visited.contains_key(&next_key) {
                    continue;
                }
                if is_dead(&next) {
                    continue;
                }
                if self.nodes.len() >= self.limits.max_stored_nodes {
                    self.status = ResumableSearchStatus::ResourceLimit;
                    self.elapsed += resumable_elapsed(started_at);
                    return ResumableAdvanceOutcome::ResourceLimit {
                        stats: self.stats(),
                    };
                }

                let next_depth = current_depth + 1;
                let next_score = score(&next);
                let next_index = self.nodes.len();
                self.nodes.push(ResumableNode {
                    state: next,
                    parent: Some(current_index),
                    action: Some(action),
                    score: next_score,
                    depth: next_depth,
                    discovery_index: self.next_sequence,
                });
                self.visited.insert(next_key, next_index);
                self.max_depth_reached = self.max_depth_reached.max(next_depth);
                self.next_sequence += 1;

                if domain.is_goal(&self.nodes[next_index].state) {
                    self.status = ResumableSearchStatus::Solved {
                        candidate_index: next_index,
                    };
                    self.elapsed += resumable_elapsed(started_at);
                    return ResumableAdvanceOutcome::Solved {
                        candidate_index: next_index,
                        stats: self.stats(),
                    };
                }

                self.frontier.push(QueueEntry {
                    score: next_score,
                    depth: next_depth,
                    sequence: self.nodes[next_index].discovery_index,
                    node_index: next_index,
                });
            }
        }
    }
}

fn resumable_elapsed(started_at: Option<Instant>) -> Duration {
    started_at
        .map(|instant| instant.elapsed())
        .unwrap_or_default()
}

fn resumable_witness<State, Action: Clone>(
    nodes: &[ResumableNode<State, Action>],
    mut index: usize,
) -> Witness<Action> {
    let depth = nodes[index].depth;
    let mut actions = Vec::new();
    while let Some(parent) = nodes[index].parent {
        if let Some(action) = &nodes[index].action {
            actions.push(action.clone());
        }
        index = parent;
    }
    actions.reverse();
    Witness { actions, depth }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LineDomain {
        actions: [u8; 1],
    }

    impl SearchDomain for LineDomain {
        type State = u8;
        type Action = u8;
        type Key = u8;
        type Error = ();

        fn key(&self, state: &Self::State) -> Self::Key {
            *state
        }

        fn actions(&self, _state: &Self::State) -> &[Self::Action] {
            &self.actions
        }

        fn step(
            &mut self,
            state: &Self::State,
            action: &Self::Action,
        ) -> Result<Self::State, Self::Error> {
            Ok(state.saturating_add(*action))
        }

        fn is_goal(&self, state: &Self::State) -> bool {
            *state >= 3
        }
    }

    #[test]
    fn reports_expanded_state_progress() {
        let mut domain = LineDomain { actions: [1] };
        let mut observed = Vec::new();

        let outcome = best_first_with_dead_states_and_progress(
            &mut domain,
            0,
            SearchBudget {
                max_depth: Some(8),
                max_stored_nodes: Some(32),
                max_frontier: None,
                max_duration: None,
            },
            |state| i64::from(3_u8.saturating_sub(*state)),
            |_| false,
            |state, progress| {
                observed.push((*state, progress.expanded, progress.depth));
            },
        );

        assert!(matches!(outcome, SearchOutcome::Solved(_)));
        assert_eq!(observed[0], (0, 1, 0));
        assert!(observed.iter().any(|(_, _, depth)| *depth > 0));
    }

    #[test]
    fn resumable_search_prunes_dead_states_before_goal_acceptance() {
        let mut domain = LineDomain { actions: [1] };
        let mut machine = ResumableBestFirst::new(
            0,
            0,
            0,
            ResumableSearchLimits {
                max_depth: 8,
                max_stored_nodes: 32,
            },
        );
        let outcome = machine.advance_with_dead_states(
            &mut domain,
            ResumableSearchAllowance {
                max_expanded_nodes: 32,
                max_duration: Some(Duration::from_secs(1)),
            },
            |_| 0,
            |state| *state >= 3,
        );
        assert!(matches!(outcome, ResumableAdvanceOutcome::Exhausted { .. }));
    }

    #[test]
    fn resumable_search_prunes_a_dead_initial_goal() {
        let mut domain = LineDomain { actions: [1] };
        let mut machine = ResumableBestFirst::new(
            3,
            3,
            0,
            ResumableSearchLimits {
                max_depth: 8,
                max_stored_nodes: 32,
            },
        );
        let outcome = machine.advance_with_dead_states(
            &mut domain,
            ResumableSearchAllowance {
                max_expanded_nodes: 32,
                max_duration: Some(Duration::from_secs(1)),
            },
            |_| 0,
            |state| *state >= 3,
        );
        assert!(matches!(outcome, ResumableAdvanceOutcome::Exhausted { .. }));
    }

    #[test]
    fn prunes_dead_states_before_accepting_goals() {
        let mut domain = LineDomain { actions: [1] };

        let outcome = best_first_with_dead_states(
            &mut domain,
            0,
            SearchBudget {
                max_depth: Some(8),
                max_stored_nodes: Some(32),
                max_frontier: None,
                max_duration: None,
            },
            |_| 0,
            |state| *state >= 3,
        );

        assert!(matches!(outcome, SearchOutcome::Exhausted(_)));
    }

    #[test]
    fn scans_discovered_states_with_witness_actions() {
        let mut domain = LineDomain { actions: [1] };
        let mut discovered = Vec::new();

        let outcome = best_first_scan_with_dead_states_and_progress(
            &mut domain,
            0,
            SearchBudget {
                max_depth: Some(4),
                max_stored_nodes: Some(32),
                max_frontier: None,
                max_duration: None,
            },
            |state| i64::from(4_u8.saturating_sub(*state)),
            |_| false,
            |search_match| {
                discovered.push((search_match.state, search_match.actions, search_match.depth));
                ScanControl::Continue
            },
            |_, _| {},
        );

        assert!(matches!(outcome, ScanOutcome::BudgetExceeded { .. }));
        assert!(discovered.contains(&(0, Vec::new(), 0)));
        assert!(discovered.contains(&(2, vec![1, 1], 2)));
    }

    #[test]
    fn resumable_search_preserves_frontier_and_witnesses_between_advances() {
        let mut domain = LineDomain { actions: [1] };
        let mut machine = ResumableBestFirst::new(
            0,
            0,
            3,
            ResumableSearchLimits {
                max_depth: 8,
                max_stored_nodes: 32,
            },
        );
        let allowance = ResumableSearchAllowance {
            max_expanded_nodes: 1,
            max_duration: Some(Duration::from_secs(1)),
        };

        let first = machine.advance(&mut domain, allowance, |state| i64::from(3 - *state));
        assert!(matches!(
            first,
            ResumableAdvanceOutcome::Paused {
                reason: ResumablePauseReason::ExpandedNodes,
                ..
            }
        ));
        assert_eq!(machine.stats().expanded, 1);
        assert_eq!(machine.candidate(1).unwrap().actions, vec![1]);

        let second = machine.advance(&mut domain, allowance, |state| i64::from(3 - *state));
        assert!(matches!(second, ResumableAdvanceOutcome::Paused { .. }));
        assert_eq!(machine.stats().expanded, 2);

        let solved = machine.advance(&mut domain, allowance, |state| i64::from(3 - *state));
        let ResumableAdvanceOutcome::Solved {
            candidate_index, ..
        } = solved
        else {
            panic!("third advance must solve the line domain");
        };
        let candidate = machine.candidate(candidate_index).unwrap();
        assert_eq!(candidate.state, 3);
        assert_eq!(candidate.actions, vec![1, 1, 1]);
        assert_eq!(candidate.depth, 3);
    }
}
