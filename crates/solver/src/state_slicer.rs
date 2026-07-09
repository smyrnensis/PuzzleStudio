use crate::relevance::SolverRelevance;
use puzzle_core::{CompiledGame, ObjectId as ObjectId2, State};
use puzzle_grid3d::{Game3, ObjectId as ObjectId3, State3};

pub trait SolverStateProjection: Sized {
    type ObjectId: Copy;

    fn without_solver_objects(&self, ignored_objects: &[Self::ObjectId]) -> Self;
}

impl SolverStateProjection for State {
    type ObjectId = ObjectId2;

    fn without_solver_objects(&self, ignored_objects: &[Self::ObjectId]) -> Self {
        self.without_objects(ignored_objects)
    }
}

impl SolverStateProjection for State3 {
    type ObjectId = ObjectId3;

    fn without_solver_objects(&self, ignored_objects: &[Self::ObjectId]) -> Self {
        self.without_objects(ignored_objects)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SolverStateSlicer<ObjectId = ObjectId2> {
    ignored_objects: Vec<ObjectId>,
}

impl<ObjectId> SolverStateSlicer<ObjectId> {
    pub fn new() -> Self {
        Self {
            ignored_objects: Vec::new(),
        }
    }

    pub(crate) fn from_ignored_objects(ignored_objects: Vec<ObjectId>) -> Self {
        Self { ignored_objects }
    }
}

impl<ObjectId: Copy> SolverStateSlicer<ObjectId> {
    pub fn project_state<StateT>(&self, state: &StateT) -> StateT
    where
        StateT: SolverStateProjection<ObjectId = ObjectId>,
    {
        state.without_solver_objects(&self.ignored_objects)
    }
}

impl SolverStateSlicer<ObjectId2> {
    pub fn from_relevance(game: &CompiledGame, relevance: &SolverRelevance) -> Self {
        Self::from_ignored_objects(relevance.ignored_objects_for_game(game))
    }
}

impl SolverStateSlicer<ObjectId3> {
    pub fn from_relevance(game: &Game3, relevance: &SolverRelevance<ObjectId3>) -> Self {
        Self::from_ignored_objects(relevance.ignored_objects_for_game(game))
    }
}
