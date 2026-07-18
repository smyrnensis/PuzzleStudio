use crate::domain::SearchDomain;
use crate::stable_hash::{fnv_mix, fnv_seed};
use crate::state_slicer::SolverStateSlicer;
use puzzle_core::{
    GridCompiledGame, GridRuleStep, GridSize, GridState, GridTransitionError, InputId, ObjectId,
};
use puzzle_lang::LoadedGridGame;
use puzzle_play::{GridGameSession, GridHeadlessSession};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridStateKey<const D: usize> {
    hash: u64,
    completed: bool,
    size: [u16; D],
    layer_count: u16,
    slots: Vec<u16>,
    visible_variables: Vec<i64>,
    level_fired_rules: Vec<u16>,
    session: Vec<u8>,
}

impl<const D: usize> GridStateKey<D> {
    pub fn from_state<Size: GridSize<D>>(state: &GridState<D, Size>) -> Self {
        Self::from_parts(state, false)
    }

    fn from_search_state<Size: GridSize<D>>(state: &GridSearchState<D, Size>) -> Self {
        let mut key = Self::from_parts(state.observation_state(), state.completed());
        key.session = state.headless.search_key();
        for byte in &key.session {
            key.hash = fnv_mix(key.hash, u64::from(*byte));
        }
        key
    }

    fn from_parts<Size: GridSize<D>>(state: &GridState<D, Size>, completed: bool) -> Self {
        let size = state.size.axes();
        let mut slots = Vec::with_capacity(state.slots().len());
        let mut hash = fnv_seed();
        hash = fnv_mix(hash, if completed { 1 } else { 0 });
        for axis in size {
            hash = fnv_mix(hash, u64::from(axis));
        }
        hash = fnv_mix(hash, u64::from(state.layer_count));

        for object in state.slots() {
            slots.push(object.0);
            hash = fnv_mix(hash, u64::from(object.0));
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
            completed,
            size,
            layer_count: state.layer_count,
            slots,
            visible_variables: state.visible_variables().to_vec(),
            level_fired_rules: state
                .level_fired_rules()
                .iter()
                .map(|rule| rule.0)
                .collect(),
            session: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GridSearchState<const D: usize, Size: GridSize<D>> {
    headless: GridHeadlessSession<D, Size>,
    input_history: Vec<InputId>,
}

impl<const D: usize, Size: GridSize<D>> GridSearchState<D, Size> {
    fn new(headless: GridHeadlessSession<D, Size>) -> Self {
        Self {
            headless,
            input_history: Vec::new(),
        }
    }

    pub fn state(&self) -> &GridState<D, Size> {
        self.headless.state()
    }

    /// The state committed by the input before level-clear lifecycle and
    /// navigation. For a non-completing input this is the continuation state.
    pub fn observation_state(&self) -> &GridState<D, Size> {
        self.headless.observation_state()
    }

    pub fn input_history(&self) -> &[InputId] {
        &self.input_history
    }

    pub fn into_state(self) -> GridState<D, Size> {
        self.headless.state().clone()
    }

    pub fn completed(&self) -> bool {
        self.headless.completed()
    }
}

impl<const D: usize> Hash for GridStateKey<D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

pub struct GridPuzzleDomain<const D: usize, Size: GridSize<D>> {
    game: Arc<LoadedGridGame<D, Size>>,
    level_index: usize,
    inputs: Vec<InputId>,
    state_slicer: SolverStateSlicer<ObjectId>,
    goal: GridSearchGoal<D, Size>,
    track_input_history: bool,
}

pub enum GridSearchGoal<const D: usize, Size: GridSize<D>> {
    LevelCompletion,
    StatePredicate(Box<dyn Fn(&GridState<D, Size>) -> bool>),
}

impl<const D: usize, Size: GridSize<D>> GridPuzzleDomain<D, Size> {
    pub fn new(
        game: Arc<LoadedGridGame<D, Size>>,
        level_index: usize,
        inputs: Vec<InputId>,
        is_goal: impl Fn(&GridState<D, Size>) -> bool + 'static,
    ) -> Self {
        Self::with_state_slicer(game, level_index, inputs, SolverStateSlicer::new(), is_goal)
    }

    pub fn for_level_completion(
        game: Arc<LoadedGridGame<D, Size>>,
        level_index: usize,
        inputs: Vec<InputId>,
    ) -> Self {
        Self::with_state_slicer_for_level_completion(
            game,
            level_index,
            inputs,
            SolverStateSlicer::new(),
        )
    }

    pub fn with_state_slicer(
        game: Arc<LoadedGridGame<D, Size>>,
        level_index: usize,
        inputs: Vec<InputId>,
        state_slicer: SolverStateSlicer<ObjectId>,
        is_goal: impl Fn(&GridState<D, Size>) -> bool + 'static,
    ) -> Self {
        Self::with_goal(
            game,
            level_index,
            inputs,
            state_slicer,
            GridSearchGoal::StatePredicate(Box::new(is_goal)),
        )
    }

    pub fn with_state_slicer_for_level_completion(
        game: Arc<LoadedGridGame<D, Size>>,
        level_index: usize,
        inputs: Vec<InputId>,
        state_slicer: SolverStateSlicer<ObjectId>,
    ) -> Self {
        Self::with_goal(
            game,
            level_index,
            inputs,
            state_slicer,
            GridSearchGoal::LevelCompletion,
        )
    }

    pub fn with_goal(
        game: Arc<LoadedGridGame<D, Size>>,
        level_index: usize,
        inputs: Vec<InputId>,
        state_slicer: SolverStateSlicer<ObjectId>,
        goal: GridSearchGoal<D, Size>,
    ) -> Self {
        Self {
            game,
            level_index,
            inputs,
            state_slicer,
            goal,
            track_input_history: true,
        }
    }

    pub fn without_input_history(mut self) -> Self {
        self.track_input_history = false;
        self
    }

    pub fn game(&self) -> &GridCompiledGame<D> {
        &self.game.game
    }

    pub fn rules(&self) -> &[GridRuleStep<D>] {
        self.game
            .program_for_level(self.level_index)
            .unwrap_or_default()
    }

    pub fn initial_state(
        &self,
        state: GridState<D, Size>,
    ) -> Result<GridSearchState<D, Size>, GridTransitionError<D>> {
        let state = self.state_slicer.project_state(&state);
        GridHeadlessSession::from_level_state(&self.game, self.level_index, state)
            .map(GridSearchState::new)
    }

    pub fn initial_session(
        &self,
        session: GridGameSession<D, Size>,
    ) -> Result<GridSearchState<D, Size>, GridTransitionError<D>> {
        GridHeadlessSession::from_game_session_with_state_projection(
            session,
            self.level_index,
            |state| self.state_slicer.project_state(state),
        )
        .map(GridSearchState::new)
    }
}

impl<const D: usize, Size: GridSize<D>> SearchDomain for GridPuzzleDomain<D, Size> {
    type State = GridSearchState<D, Size>;
    type Action = InputId;
    type Key = GridStateKey<D>;
    type Error = GridTransitionError<D>;

    fn key(&self, state: &Self::State) -> Self::Key {
        GridStateKey::from_search_state(state)
    }

    fn actions(&self, state: &Self::State) -> &[Self::Action] {
        if state.completed() { &[] } else { &self.inputs }
    }

    fn step(
        &mut self,
        state: &Self::State,
        action: &Self::Action,
    ) -> Result<Self::State, Self::Error> {
        let mut headless = state.headless.clone();
        headless.apply_input(&self.game, *action)?;
        let input_history = if self.track_input_history {
            let mut input_history = state.input_history.clone();
            input_history.push(*action);
            input_history
        } else {
            Vec::new()
        };
        Ok(GridSearchState {
            headless,
            input_history,
        })
    }

    fn is_goal(&self, state: &Self::State) -> bool {
        match &self.goal {
            GridSearchGoal::LevelCompletion => {
                state.completed() || self.game.is_goal_complete(state.observation_state())
            }
            GridSearchGoal::StatePredicate(predicate) => predicate(state.observation_state()),
        }
    }
}
