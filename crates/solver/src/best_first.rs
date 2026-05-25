use crate::budget::SearchBudget;
use crate::domain::SearchDomain;
use crate::report::{SearchFailure, SearchOutcome, SearchStats, Witness};
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
    mut score: F,
    mut is_dead: P,
) -> SearchOutcome<D::Action, D::Error>
where
    D: SearchDomain,
    F: FnMut(&D::State) -> i64,
    P: FnMut(&D::State) -> bool,
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
            if is_dead(&next) {
                continue;
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
