use crate::relevance::SolverRelevance;
use puzzle_core::{GridSize, GridState, ObjectId};
use puzzle_kernel::CompiledGameModel;
use std::collections::BTreeSet;

pub trait SolverStateProjection: Sized {
    type ObjectId: Copy;

    fn without_solver_objects(&self, ignored_objects: &[Self::ObjectId]) -> Self;
}

impl<const D: usize, Size: GridSize<D>> SolverStateProjection for GridState<D, Size> {
    type ObjectId = ObjectId;

    fn without_solver_objects(&self, ignored_objects: &[Self::ObjectId]) -> Self {
        self.without_objects(ignored_objects)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SolverStateSlicer<ObjectId = puzzle_core::ObjectId> {
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

impl SolverStateSlicer<ObjectId> {
    pub fn from_kept_objects<ConditionDef, Rule, Condition, Frame>(
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        kept_objects: &BTreeSet<ObjectId>,
    ) -> Self {
        Self::from_ignored_objects(
            game.objects()
                .iter()
                .filter_map(|object| {
                    (!object.id.is_empty() && !kept_objects.contains(&object.id))
                        .then_some(object.id)
                })
                .collect(),
        )
    }

    pub fn from_relevance<ConditionDef, Rule, Condition, Frame>(
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        relevance: &SolverRelevance,
    ) -> Self {
        Self::from_kept_objects(game, &relevance.relevant_objects().into_iter().collect())
    }
}
