use std::hash::Hash;

pub trait SearchDomain {
    type State: Clone;
    type Action: Clone;
    type Key: Clone + Eq + Hash;
    type Error;

    fn key(&self, state: &Self::State) -> Self::Key;
    fn actions(&self, state: &Self::State) -> &[Self::Action];
    fn step(
        &mut self,
        state: &Self::State,
        action: &Self::Action,
    ) -> Result<Self::State, Self::Error>;
    fn is_goal(&self, state: &Self::State) -> bool;
}
