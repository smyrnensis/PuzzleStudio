use crate::domain::SearchDomain;
use crate::stable_hash::{fnv_mix, fnv_seed};
use crate::state_slicer::SolverStateSlicer;
use puzzle_core::{
    CompiledGame, InputId, LayerId, ObjectId, State, TransitionCommand, TransitionError,
    transition_solver_outcome,
};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PuzzleStateKey {
    hash: u64,
    won: bool,
    slots: Vec<u16>,
    visible_variables: Vec<i64>,
    level_fired_rules: Vec<u16>,
}

impl PuzzleStateKey {
    pub fn from_state(game: &CompiledGame, state: &State) -> Self {
        Self::from_parts(game, state, false)
    }

    fn from_search_state(
        game: &CompiledGame,
        slicer: &SolverStateSlicer,
        state: &PuzzleSearchState,
    ) -> Self {
        let key_state = slicer.project_state(&state.state);
        Self::from_parts(game, &key_state, state.won)
    }

    fn from_parts(game: &CompiledGame, state: &State, won: bool) -> Self {
        let layers = game.main_layers();
        let mut slots =
            Vec::with_capacity(usize::from(state.width) * usize::from(state.height) * layers.len());
        let mut hash = fnv_seed();
        hash = fnv_mix(hash, if won { 1 } else { 0 });

        for y in 0..state.height {
            for x in 0..state.width {
                for layer in &layers {
                    let index = source_slot_index(state, x, y, *layer);
                    let object = state.slots()[index];
                    let object = if game.is_main_object(object) {
                        object
                    } else {
                        ObjectId::EMPTY
                    };
                    slots.push(object.0);
                    hash = fnv_mix(hash, u64::from(object.0));
                }
            }
        }
        for value in state.visible_variables() {
            hash = fnv_mix(hash, *value as u64);
        }
        hash = fnv_mix(hash, state.level_fired_rules().len() as u64);
        for rule in state.level_fired_rules() {
            hash = fnv_mix(hash, u64::from(rule.0));
        }

        Self {
            hash,
            won,
            slots,
            visible_variables: state.visible_variables().to_vec(),
            level_fired_rules: state
                .level_fired_rules()
                .iter()
                .map(|rule| rule.0)
                .collect(),
        }
    }
}

impl Hash for PuzzleStateKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PuzzleSearchState {
    state: State,
    input_history: Vec<InputId>,
    won: bool,
}

impl PuzzleSearchState {
    pub fn new(state: State) -> Self {
        Self {
            state,
            input_history: Vec::new(),
            won: false,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn input_history(&self) -> &[InputId] {
        &self.input_history
    }

    pub fn into_state(self) -> State {
        self.state
    }

    pub fn won(&self) -> bool {
        self.won
    }
}

fn source_slot_index(state: &State, x: u16, y: u16, layer: LayerId) -> usize {
    ((usize::from(y) * usize::from(state.width) + usize::from(x)) * usize::from(state.layer_count))
        + usize::from(layer.0)
}

pub struct PuzzleDomain {
    game: Arc<CompiledGame>,
    inputs: Vec<InputId>,
    state_slicer: SolverStateSlicer,
    is_goal: Box<dyn Fn(&State) -> bool>,
    accept_win_command: bool,
    track_input_history: bool,
}

impl PuzzleDomain {
    pub fn new(
        game: Arc<CompiledGame>,
        inputs: Vec<InputId>,
        is_goal: impl Fn(&State) -> bool + 'static,
    ) -> Self {
        Self::with_state_slicer(game, inputs, SolverStateSlicer::new(), is_goal)
    }

    pub fn with_state_slicer(
        game: Arc<CompiledGame>,
        inputs: Vec<InputId>,
        state_slicer: SolverStateSlicer,
        is_goal: impl Fn(&State) -> bool + 'static,
    ) -> Self {
        Self::with_state_slicer_and_win_command_goal(game, inputs, state_slicer, true, is_goal)
    }

    pub fn with_state_slicer_and_win_command_goal(
        game: Arc<CompiledGame>,
        inputs: Vec<InputId>,
        state_slicer: SolverStateSlicer,
        accept_win_command: bool,
        is_goal: impl Fn(&State) -> bool + 'static,
    ) -> Self {
        Self {
            game,
            inputs,
            state_slicer,
            is_goal: Box::new(is_goal),
            accept_win_command,
            track_input_history: true,
        }
    }

    pub fn without_input_history(mut self) -> Self {
        self.track_input_history = false;
        self
    }

    pub fn game(&self) -> &CompiledGame {
        &self.game
    }

    pub fn initial_state(&self, state: State) -> PuzzleSearchState {
        PuzzleSearchState::new(self.state_slicer.project_state(&state))
    }
}

impl SearchDomain for PuzzleDomain {
    type State = PuzzleSearchState;
    type Action = InputId;
    type Key = PuzzleStateKey;
    type Error = TransitionError;

    fn key(&self, state: &Self::State) -> Self::Key {
        PuzzleStateKey::from_search_state(&self.game, &self.state_slicer, state)
    }

    fn actions(&self, _state: &Self::State) -> &[Self::Action] {
        &self.inputs
    }

    fn step(
        &mut self,
        state: &Self::State,
        action: &Self::Action,
    ) -> Result<Self::State, Self::Error> {
        let solver_state = self.state_slicer.project_state(&state.state);
        let outcome = transition_solver_outcome(&self.game, &solver_state, *action)?;
        let next_state = self.state_slicer.project_state(&outcome.next_state);
        let input_history = if self.track_input_history {
            let mut input_history = state.input_history.clone();
            input_history.push(*action);
            input_history
        } else {
            Vec::new()
        };
        Ok(PuzzleSearchState {
            state: next_state,
            input_history,
            won: self.accept_win_command
                && (state.won
                    || outcome
                        .commands
                        .iter()
                        .any(|command| matches!(command, TransitionCommand::Win))),
        })
    }

    fn is_goal(&self, state: &Self::State) -> bool {
        state.won || (self.is_goal)(&state.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SearchBudget, SearchOutcome, exact_bfs};
    use puzzle_core::transition_state;
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
title = solver_push_goal

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
        let game = Arc::new(loaded.game.clone());
        let goal_game = loaded.clone();
        let mut domain = PuzzleDomain::new(game.clone(), inputs, move |state| {
            goal_game.is_goal_complete(state)
        });
        let initial = loaded.levels[0].initial_state.clone();

        let outcome = exact_bfs(
            &mut domain,
            PuzzleSearchState::new(initial.clone()),
            SearchBudget::bounded(80, 1_000_000, Duration::from_secs(5)),
        );
        let SearchOutcome::Solved(witness) = outcome else {
            panic!("expected solved, got {outcome:?}");
        };
        assert_eq!(
            witness.actions.len(),
            usize::try_from(witness.depth).unwrap()
        );

        let mut state = initial;
        for action in witness.actions {
            state = transition_state(&game, &state, action).unwrap();
        }
        assert!(loaded.is_goal_complete(&state));
    }

    #[test]
    fn reports_depth_budget() {
        let source = r#"
title = solver_depth_budget

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
        let game = Arc::new(loaded.game.clone());
        let goal_game = loaded.clone();
        let mut domain =
            PuzzleDomain::new(game, inputs, move |state| goal_game.is_goal_complete(state));

        let outcome = exact_bfs(
            &mut domain,
            PuzzleSearchState::new(loaded.levels[0].initial_state.clone()),
            SearchBudget {
                max_depth: Some(0),
                max_nodes: Some(100_000),
                max_frontier: None,
                max_duration: Some(Duration::from_secs(5)),
            },
        );

        assert!(matches!(outcome, SearchOutcome::BudgetExceeded(_)));
    }

    #[test]
    fn solver_handles_transition_local_mark_rules() {
        let source = r#"
title = mark_solver

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
        let game = Arc::new(loaded.game.clone());
        let goal_game = loaded.clone();
        let mut domain = PuzzleDomain::new(game.clone(), inputs, move |state| {
            goal_game.is_goal_complete(state)
        });

        let outcome = exact_bfs(
            &mut domain,
            PuzzleSearchState::new(loaded.levels[0].initial_state.clone()),
            SearchBudget::bounded(4, 100, Duration::from_secs(5)),
        );
        let SearchOutcome::Solved(witness) = outcome else {
            panic!("expected solved, got {outcome:?}");
        };
        assert_eq!(witness.actions, vec![right]);

        let solved = domain
            .step(
                &PuzzleSearchState::new(loaded.levels[0].initial_state.clone()),
                &right,
            )
            .unwrap();
        assert!(loaded.is_goal_complete(solved.state()));
        assert!(solved.state().slot_mark().iter().all(Vec::is_empty));
        assert!(solved.state().cell_mark().iter().all(Vec::is_empty));
    }

    #[test]
    fn solver_state_slicer_projects_initial_key_and_transition_input() {
        let source = r#"
title = solver_state_slicer

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
        let game = Arc::new(loaded.game.clone());
        let state_slicer = SolverStateSlicer::from_ignored_objects(vec![floor]);
        let mut domain =
            PuzzleDomain::with_state_slicer(game.clone(), vec![right], state_slicer, |_| false);
        let initial = loaded.levels[0].initial_state.clone();
        let solver_initial = domain.initial_state(initial.clone());
        let projected_initial = PuzzleSearchState::new(initial.without_objects(&[floor]));

        assert_eq!(
            domain.key(&PuzzleSearchState::new(initial.clone())),
            domain.key(&projected_initial)
        );
        assert_eq!(domain.key(&solver_initial), domain.key(&projected_initial));

        let stepped = domain.step(&solver_initial, &right).unwrap().into_state();

        assert!(!stepped.has_object(&game, 1, 0, floor));
        assert!(stepped.has_object(&game, 1, 0, player));
    }

    #[test]
    fn solver_treats_win_command_as_goal() {
        let source = r#"
title = win_solver

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
        let game = Arc::new(loaded.game.clone());
        let mut domain = PuzzleDomain::new(game, vec![right], |_| false);

        let solved = domain
            .step(
                &PuzzleSearchState::new(loaded.levels[0].initial_state.clone()),
                &right,
            )
            .unwrap();
        assert!(solved.won());
        assert!(domain.is_goal(&solved));

        let outcome = exact_bfs(
            &mut domain,
            PuzzleSearchState::new(loaded.levels[0].initial_state.clone()),
            SearchBudget::bounded(1, 10, Duration::from_secs(5)),
        );
        let SearchOutcome::Solved(witness) = outcome else {
            panic!("expected solved, got {outcome:?}");
        };
        assert_eq!(witness.actions, vec![right]);
    }

    #[test]
    fn solver_handles_deterministic_random_rules() {
        let source = r#"
title = random_solver

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
        let game = Arc::new(loaded.game.clone());
        let goal_game = loaded.clone();
        let mut domain = PuzzleDomain::new(game, vec![action], move |state| {
            goal_game.is_goal_complete(state)
        });

        let first = domain
            .step(
                &PuzzleSearchState::new(loaded.levels[0].initial_state.clone()),
                &action,
            )
            .unwrap();
        let second = domain
            .step(
                &PuzzleSearchState::new(loaded.levels[0].initial_state.clone()),
                &action,
            )
            .unwrap();
        assert_eq!(first, second);
        assert!(loaded.is_goal_complete(first.state()));

        let outcome = exact_bfs(
            &mut domain,
            PuzzleSearchState::new(loaded.levels[0].initial_state.clone()),
            SearchBudget::bounded(1, 10, Duration::from_secs(5)),
        );
        let SearchOutcome::Solved(witness) = outcome else {
            panic!("expected solved, got {outcome:?}");
        };
        assert_eq!(witness.actions, vec![action]);
    }
}
