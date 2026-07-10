use crate::budget::SearchBudget;
use crate::domain::SearchDomain;
use crate::report::{SearchFailure, SearchOutcome, SearchProgress, SearchStats, Witness};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
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
        .max_nodes
        .is_some_and(|max_nodes| visited >= max_nodes)
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
                max_nodes: Some(32),
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
    fn prunes_dead_states_before_accepting_goals() {
        let mut domain = LineDomain { actions: [1] };

        let outcome = best_first_with_dead_states(
            &mut domain,
            0,
            SearchBudget {
                max_depth: Some(8),
                max_nodes: Some(32),
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
                max_nodes: Some(32),
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
}
