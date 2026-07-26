use crate::budget::SearchBudget;
use crate::domain::SearchDomain;
use crate::report::{SearchFailure, SearchOutcome, SearchStats, Witness};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

#[derive(Clone, Debug)]
struct NodeRecord<State, Action> {
    state: State,
    parent: Option<usize>,
    action: Option<Action>,
    depth: u32,
}

pub fn exact_bfs<D>(
    domain: &mut D,
    initial: D::State,
    budget: SearchBudget,
) -> SearchOutcome<D::Action, D::Error>
where
    D: SearchDomain,
{
    let started_at = Instant::now();
    let initial_key = domain.key(&initial);
    let mut nodes = vec![NodeRecord {
        state: initial,
        parent: None,
        action: None,
        depth: 0,
    }];
    let mut visited = HashMap::from([(initial_key, 0_usize)]);
    let mut frontier = VecDeque::from([0_usize]);
    let mut expanded = 0_usize;
    let mut max_depth_reached = 0_u32;

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

        let Some(current_index) = frontier.pop_front() else {
            return SearchOutcome::Exhausted(stats(
                started_at,
                visited.len(),
                frontier.len(),
                expanded,
                max_depth_reached,
            ));
        };

        let current_depth = nodes[current_index].depth;
        max_depth_reached = max_depth_reached.max(current_depth);
        if budget
            .max_depth
            .is_some_and(|max_depth| current_depth >= max_depth)
        {
            return SearchOutcome::BudgetExceeded(stats(
                started_at,
                visited.len(),
                frontier.len(),
                expanded,
                max_depth_reached,
            ));
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

            if domain.is_goal(&nodes[next_index].state) {
                return SearchOutcome::Solved(reconstruct_witness(&nodes, next_index));
            }

            frontier.push_back(next_index);
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
    started_at: Instant,
    visited: usize,
    frontier: usize,
    expanded: usize,
    max_depth_reached: u32,
) -> Option<SearchStats> {
    let elapsed = started_at.elapsed();
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
    started_at: Instant,
    visited: usize,
    frontier: usize,
    expanded: usize,
    max_depth_reached: u32,
) -> SearchStats {
    SearchStats {
        visited,
        expanded,
        frontier,
        max_depth_reached,
        elapsed: started_at.elapsed(),
    }
}
