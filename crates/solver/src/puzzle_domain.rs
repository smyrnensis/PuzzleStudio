use puzzle_core::Size2;

type SolverTestDomain = crate::grid_domain::GridPuzzleDomain<2, Size2>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SearchBudget, SearchDomain, SearchOutcome, SolverStateSlicer, exact_bfs};
    use puzzle_core::{InputId, ObjectId};
    use puzzle_lang::parse_game2d as parse_game;
    use std::sync::Arc;
    use std::time::Duration;

    fn object_named(loaded: &puzzle_lang::LoadedGame, name: &str) -> ObjectId {
        loaded
            .object_labels
            .iter()
            .find_map(|(id, label)| (label == name).then_some(*id))
            .unwrap_or_else(|| panic!("missing object {name}"))
    }

    #[test]
    fn solves_first_sokoban_level_and_replays_to_goal() {
        let source = r#"
const title = solver_push_goal

puzzle default {
layers {
floor = Goal
actor = Player Box Wall
}

keys {
d ArrowRight -> right
}

rules {
input right [ Player | Box | no actor ] -> [ | Player | Box ]
input right [ Player | no actor ] -> [ | Player ]
}

win_conditions {
all Goal on Box
}
}

levels tiny of default {
legend {
. = empty
P = Player
B = Box
G = Goal
}

level "start" {
PBG
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let inputs = vec![right];
        let game = Arc::new(loaded.clone());
        let goal_game = loaded.clone();
        let mut domain = SolverTestDomain::new(game.clone(), 0, inputs, move |state| {
            goal_game.is_goal_complete(state)
        });
        let initial = loaded.levels[0].initial_state.clone();
        let solver_initial = domain.initial_state(initial.clone()).unwrap();

        let outcome = exact_bfs(
            &mut domain,
            solver_initial,
            SearchBudget::bounded(80, 1_000_000, Duration::from_secs(5)),
        );
        let SearchOutcome::Solved(witness) = outcome else {
            panic!("expected solved, got {outcome:?}");
        };
        assert_eq!(
            witness.actions.len(),
            usize::try_from(witness.depth).unwrap()
        );

        let mut headless =
            puzzle_play::HeadlessSession::from_level_state(&loaded, 0, initial).unwrap();
        for action in witness.actions {
            headless.apply_input(&loaded, action).unwrap();
        }
        assert!(headless.completed() || loaded.is_goal_complete(headless.state()));
    }

    #[test]
    fn reports_depth_budget() {
        let source = r#"
const title = solver_depth_budget

puzzle default {
layers {
floor = Goal
actor = Player Box
}

keys {
d ArrowRight -> right
}

rules {
input right [ Player | Box | no actor ] -> [ | Player | Box ]
input right [ Player | no actor ] -> [ | Player ]
}

win_conditions {
all Goal on Box
}
}

levels tiny of default {
legend {
. = empty
P = Player
B = Box
G = Goal
}

level "start" {
PBG
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let inputs = vec![right];
        let game = Arc::new(loaded.clone());
        let goal_game = loaded.clone();
        let mut domain = SolverTestDomain::new(game, 0, inputs, move |state| {
            goal_game.is_goal_complete(state)
        });

        let initial = domain
            .initial_state(loaded.levels[0].initial_state.clone())
            .unwrap();
        let outcome = exact_bfs(
            &mut domain,
            initial,
            SearchBudget {
                max_depth: Some(0),
                max_stored_nodes: Some(100_000),
                max_frontier: None,
                max_duration: Some(Duration::from_secs(5)),
            },
        );

        assert!(matches!(outcome, SearchOutcome::BudgetExceeded(_)));
    }

    #[test]
    fn solver_handles_transition_local_mark_rules() {
        let source = r#"
const title = mark_solver

puzzle default {
layers {
floor = Goal
actor = Player Box Wall
}

marks {
push
dest
}

win_conditions {
all Goal on Box
}

rules {
if input == right {
once right [ Player | Box | no actor ] -> [ Player | Box{push} | {dest} ]
once right [ Box{push} | {dest} ] -> [ | Box ]
once right [ Player | no actor ] -> [ | Player ]
}
}
}

levels tiny of default {
legend {
. = empty
P = Player
B = Box
G = Goal
* = Goal Box
}

level "start" {
PBG
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let inputs = vec![right];
        let game = Arc::new(loaded.clone());
        let goal_game = loaded.clone();
        let mut domain = SolverTestDomain::new(game.clone(), 0, inputs, move |state| {
            goal_game.is_goal_complete(state)
        });

        let initial = domain
            .initial_state(loaded.levels[0].initial_state.clone())
            .unwrap();
        let outcome = exact_bfs(
            &mut domain,
            initial.clone(),
            SearchBudget::bounded(4, 100, Duration::from_secs(5)),
        );
        let SearchOutcome::Solved(witness) = outcome else {
            panic!("expected solved, got {outcome:?}");
        };
        assert_eq!(witness.actions, vec![right]);

        let solved = domain.step(&initial, &right).unwrap();
        assert!(loaded.is_goal_complete(solved.state()));
    }

    #[test]
    fn solver_state_slicer_projects_initial_key_and_transition_input() {
        let source = r#"
const title = solver_state_slicer

puzzle default {
layers {
actor = Player Floor
}

keys {
d ArrowRight -> right
}

rules {
input right [ Player | no actor ] -> [ | Player ]
}

levels tiny of default {
legend {
. = empty
P = Player
F = Floor
}

level "start" {
PF
}
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let player = object_named(&loaded, "Player");
        let floor = object_named(&loaded, "Floor");
        let game = Arc::new(loaded.clone());
        let state_slicer = SolverStateSlicer::from_ignored_objects(vec![floor]);
        let mut domain =
            SolverTestDomain::with_state_slicer(game.clone(), 0, vec![right], state_slicer, |_| {
                false
            });
        let initial = loaded.levels[0].initial_state.clone();
        let solver_initial = domain.initial_state(initial.clone()).unwrap();
        let projected_initial = domain
            .initial_state(initial.without_objects(&[floor]))
            .unwrap();

        assert_eq!(
            domain.key(&domain.initial_state(initial.clone()).unwrap()),
            domain.key(&projected_initial)
        );
        assert_eq!(domain.key(&solver_initial), domain.key(&projected_initial));

        let stepped = domain.step(&solver_initial, &right).unwrap().into_state();

        assert!(!stepped.has_object(&game.game, 1, 0, floor));
        assert!(stepped.has_object(&game.game, 1, 0, player));
    }

    #[test]
    fn solver_treats_win_command_as_goal() {
        let source = r#"
const title = win_solver

puzzle board {
layers {
actor = Player Exit
}

keys {
d ArrowRight -> right
}

rules {
input right [ Player | Exit ] -> win
input right [ Player | no actor ] -> [ | Player ]
}
}

levels tiny of board {
legend {
. = empty
P = Player
E = Exit
}

level "start" {
PE
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let game = Arc::new(loaded.clone());
        let mut domain = SolverTestDomain::for_level_completion(game, 0, vec![right]);
        let initial = domain
            .initial_state(loaded.levels[0].initial_state.clone())
            .unwrap();

        let solved = domain.step(&initial, &right).unwrap();
        assert!(solved.completed());
        assert!(domain.is_goal(&solved));

        let outcome = exact_bfs(
            &mut domain,
            initial,
            SearchBudget::bounded(1, 10, Duration::from_secs(5)),
        );
        let SearchOutcome::Solved(witness) = outcome else {
            panic!("expected solved, got {outcome:?}");
        };
        assert_eq!(witness.actions, vec![right]);
    }

    #[test]
    fn semantic_goal_matches_completion_observation_before_next_level() {
        let source = r#"
const title = completion_observation
puzzle default {
layers {
floor = Goal
actor = Player Box Wall
}
groups { solid = Player Box Wall }
win_conditions {
some Goal
all Goal on Box
}
on_level_clear { next_level }
rules {
once input directions [ Player | Box | no solid ] -> [ | Player | Box ]
once input directions [ Player | no solid ] -> [ | Player ]
}
levels {
legend {
. = empty
G = Goal
P = Player
B = Box
# = Wall
}
level "first" {
#####
#PBG#
#####
}
level "second" {
#####
#P.G#
#####
}
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let goal_game = loaded.clone();
        let mut domain =
            SolverTestDomain::new(Arc::new(loaded.clone()), 0, vec![right], move |state| {
                goal_game.is_goal_complete(state)
            });
        let initial = domain
            .initial_state(loaded.levels[0].initial_state.clone())
            .unwrap();

        let completed = domain.step(&initial, &right).unwrap();

        assert!(completed.completed());
        assert!(loaded.is_goal_complete(completed.observation_state()));
        assert_eq!(completed.observation_state(), completed.state());
        assert!(domain.is_goal(&completed));

        let outcome = exact_bfs(
            &mut domain,
            initial,
            SearchBudget::bounded(1, 10, Duration::from_secs(1)),
        );
        let SearchOutcome::Solved(witness) = outcome else {
            panic!("expected solved, got {outcome:?}");
        };
        assert_eq!(witness.actions, vec![right]);
    }

    #[test]
    fn solver_handles_deterministic_random_rules() {
        let source = r#"
const title = random_solver

puzzle default {
layers {
actor = A B
}
empty .

win_conditions {
count(B) == 1
}

rules {
random [ A ] -> [ B ]
}

levels tiny of default {
legend {
. = empty
A = A
B = B
}

level "start" {
AAA
}
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let action = InputId(0);
        let game = Arc::new(loaded.clone());
        let goal_game = loaded.clone();
        let mut domain = SolverTestDomain::new(game, 0, vec![action], move |state| {
            goal_game.is_goal_complete(state)
        });
        let initial = domain
            .initial_state(loaded.levels[0].initial_state.clone())
            .unwrap();

        let first = domain.step(&initial, &action).unwrap();
        let second = domain.step(&initial, &action).unwrap();
        assert_eq!(domain.key(&first), domain.key(&second));
        assert!(loaded.is_goal_complete(first.state()));

        let outcome = exact_bfs(
            &mut domain,
            initial,
            SearchBudget::bounded(1, 10, Duration::from_secs(5)),
        );
        let SearchOutcome::Solved(witness) = outcome else {
            panic!("expected solved, got {outcome:?}");
        };
        assert_eq!(witness.actions, vec![action]);
    }

    #[test]
    fn solver_edge_completes_again_within_one_logical_input() {
        let loaded = parse_game(include_str!(
            "../../play/tests/fixtures/again_atomic.puzzle"
        ))
        .unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let goal_game = loaded.clone();
        let mut domain =
            SolverTestDomain::new(Arc::new(loaded.clone()), 0, vec![right], move |state| {
                goal_game.is_goal_complete(state)
            });
        let initial = domain
            .initial_state(loaded.levels[0].initial_state.clone())
            .unwrap();

        let stepped = domain.step(&initial, &right).unwrap();
        assert!(stepped.completed());
        assert!(loaded.is_goal_complete(stepped.state()));

        let outcome = exact_bfs(
            &mut domain,
            initial,
            SearchBudget::bounded(1, 10, Duration::from_secs(1)),
        );
        let SearchOutcome::Solved(witness) = outcome else {
            panic!("expected solved, got {outcome:?}");
        };
        assert_eq!(witness.depth, 1);
        assert_eq!(witness.actions, vec![right]);
    }
}
