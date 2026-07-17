use crate::domain::SearchDomain;
use crate::stable_hash::{fnv_mix, fnv_seed};
use crate::state_slicer::SolverStateSlicer;
use puzzle_core::{
    GridCompiledGame, GridRuleStep, GridSize, GridState, GridTransitionError, InputId, ObjectId,
};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridStateKey<const D: usize> {
    hash: u64,
    size: [u16; D],
    layer_count: u16,
    slots: Vec<u16>,
    visible_variables: Vec<i64>,
    level_fired_rules: Vec<u16>,
}

impl<const D: usize> GridStateKey<D> {
    pub fn from_state<Size: GridSize<D>>(state: &GridState<D, Size>) -> Self {
        Self::from_parts(state)
    }

    fn from_search_state<Size: GridSize<D>>(
        slicer: &SolverStateSlicer<ObjectId>,
        state: &GridState<D, Size>,
    ) -> Self {
        let key_state = slicer.project_state(state);
        Self::from_parts(&key_state)
    }

    fn from_parts<Size: GridSize<D>>(state: &GridState<D, Size>) -> Self {
        let size = state.size.axes();
        let mut slots = Vec::with_capacity(state.slots().len());
        let mut hash = fnv_seed();
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
            size,
            layer_count: state.layer_count,
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

impl<const D: usize> Hash for GridStateKey<D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

pub struct GridPuzzleDomain<const D: usize, Size: GridSize<D>> {
    game: Arc<GridCompiledGame<D>>,
    inputs: Vec<InputId>,
    state_slicer: SolverStateSlicer<ObjectId>,
    is_goal: Box<dyn Fn(&GridState<D, Size>) -> bool>,
}

impl<const D: usize, Size: GridSize<D>> GridPuzzleDomain<D, Size> {
    pub fn new(
        game: Arc<GridCompiledGame<D>>,
        inputs: Vec<InputId>,
        is_goal: impl Fn(&GridState<D, Size>) -> bool + 'static,
    ) -> Self {
        Self::with_state_slicer(game, inputs, SolverStateSlicer::new(), is_goal)
    }

    pub fn with_state_slicer(
        game: Arc<GridCompiledGame<D>>,
        inputs: Vec<InputId>,
        state_slicer: SolverStateSlicer<ObjectId>,
        is_goal: impl Fn(&GridState<D, Size>) -> bool + 'static,
    ) -> Self {
        Self {
            game,
            inputs,
            state_slicer,
            is_goal: Box::new(is_goal),
        }
    }

    pub fn game(&self) -> &GridCompiledGame<D> {
        &self.game
    }

    pub fn rules(&self) -> &[GridRuleStep<D>] {
        self.game.program()
    }

    pub fn initial_state(&self, state: GridState<D, Size>) -> GridState<D, Size> {
        self.state_slicer.project_state(&state)
    }
}

impl<const D: usize, Size: GridSize<D>> SearchDomain for GridPuzzleDomain<D, Size> {
    type State = GridState<D, Size>;
    type Action = InputId;
    type Key = GridStateKey<D>;
    type Error = GridTransitionError<D>;

    fn key(&self, state: &Self::State) -> Self::Key {
        GridStateKey::from_search_state(&self.state_slicer, state)
    }

    fn actions(&self, _state: &Self::State) -> &[Self::Action] {
        &self.inputs
    }

    fn step(
        &mut self,
        state: &Self::State,
        action: &Self::Action,
    ) -> Result<Self::State, Self::Error> {
        let solver_state = self.state_slicer.project_state(state);
        puzzle_core::grid_transition::transition_program(
            &self.game,
            &solver_state,
            self.game.executable_program(),
            *action,
        )
        .map(|state| self.state_slicer.project_state(&state))
    }

    fn is_goal(&self, state: &Self::State) -> bool {
        (self.is_goal)(state)
    }
}
