use crate::domain::SearchDomain;
use crate::stable_hash::{fnv_mix, fnv_seed};
use puzzle_grid3d::{Game3, InputId3, Rule3, State3, TransitionError3, transition_program};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Puzzle3StateKey {
    hash: u64,
    size: (u16, u16, u16),
    layer_count: u16,
    slots: Vec<u16>,
    level_fired_rules: Vec<u16>,
}

impl Puzzle3StateKey {
    pub fn from_state(state: &State3) -> Self {
        let mut slots = Vec::with_capacity(state.slots().len());
        let mut hash = fnv_seed();
        hash = fnv_mix(hash, u64::from(state.size.width));
        hash = fnv_mix(hash, u64::from(state.size.depth));
        hash = fnv_mix(hash, u64::from(state.size.height));
        hash = fnv_mix(hash, u64::from(state.layer_count));

        for object in state.slots() {
            slots.push(object.0);
            hash = fnv_mix(hash, u64::from(object.0));
        }

        hash = fnv_mix(hash, state.level_fired_rules().len() as u64);
        for rule in state.level_fired_rules() {
            hash = fnv_mix(hash, u64::from(rule.0));
        }

        Self {
            hash,
            size: (state.size.width, state.size.depth, state.size.height),
            layer_count: state.layer_count,
            slots,
            level_fired_rules: state
                .level_fired_rules()
                .iter()
                .map(|rule| rule.0)
                .collect(),
        }
    }
}

impl Hash for Puzzle3StateKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

pub struct Puzzle3Domain {
    game: Arc<Game3>,
    rules: Arc<Vec<Rule3>>,
    inputs: Vec<InputId3>,
    is_goal: Box<dyn Fn(&State3) -> bool>,
}

impl Puzzle3Domain {
    pub fn new(
        game: Arc<Game3>,
        rules: Vec<Rule3>,
        inputs: Vec<InputId3>,
        is_goal: impl Fn(&State3) -> bool + 'static,
    ) -> Self {
        Self {
            game,
            rules: Arc::new(rules),
            inputs,
            is_goal: Box::new(is_goal),
        }
    }

    pub fn game(&self) -> &Game3 {
        &self.game
    }

    pub fn rules(&self) -> &[Rule3] {
        &self.rules
    }
}

impl SearchDomain for Puzzle3Domain {
    type State = State3;
    type Action = InputId3;
    type Key = Puzzle3StateKey;
    type Error = TransitionError3;

    fn key(&self, state: &Self::State) -> Self::Key {
        Puzzle3StateKey::from_state(state)
    }

    fn actions(&self, _state: &Self::State) -> &[Self::Action] {
        &self.inputs
    }

    fn step(
        &mut self,
        state: &Self::State,
        action: &Self::Action,
    ) -> Result<Self::State, Self::Error> {
        transition_program(&self.game, state, &self.rules, *action)
    }

    fn is_goal(&self, state: &Self::State) -> bool {
        (self.is_goal)(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SearchBudget, SearchOutcome, exact_bfs};
    use puzzle_3d::parse_puzzle3d;
    use puzzle_grid3d::{ObjectId, Size3, transition_program};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn solves_single_push_support_goal_level_and_replays_to_goal() {
        let source = r#"
puzzle3 push3 {
layers {
floor = Goal
solid = Player Box Wall
}

group solid = Player Box Wall

rules {
input right [ Player | Box | no solid ] -> [ | Player | Box ]
input right [ Player | no solid ] -> [ | Player ]
}

win_conditions {
some Goal
no down [ no Box | Goal ]
}
}

levels3 tiny of push3 {
legend {
. = empty
P = Player
B = Box
G = Goal
}

level one {
PB.

..G
}
}
"#;
        let parsed = parse_puzzle3d(source).unwrap();
        let bundle = parsed.level_bundle.clone().unwrap();
        let initial = bundle.build_level_state(0).unwrap();
        let right = parsed.game.input_by_name("right").unwrap().id;
        let inputs = vec![right];
        let game = Arc::new(parsed.game.clone());
        let rules = parsed.rules.clone();
        let win = parsed.win_condition.clone().unwrap();
        let goal_game = game.clone();
        let mut domain = Puzzle3Domain::new(game.clone(), rules.clone(), inputs, move |state| {
            win.is_met(&goal_game, state)
        });

        let outcome = exact_bfs(
            &mut domain,
            initial.clone(),
            SearchBudget::bounded(2, 100, Duration::from_secs(5)),
        );
        let SearchOutcome::Solved(witness) = outcome else {
            panic!("expected solved, got {outcome:?}");
        };
        assert_eq!(witness.actions, vec![right]);

        let mut state = initial;
        for action in witness.actions {
            state = transition_program(&game, &state, &rules, action).unwrap();
        }
        assert!(parsed.win_condition.unwrap().is_met(&game, &state));
    }

    #[test]
    fn keys_distinguish_same_slots_with_different_3d_shapes() {
        let flat = State3::empty(Size3::new(2, 2, 1), 1).unwrap();
        let tall = State3::empty(Size3::new(2, 1, 2), 1).unwrap();

        assert_ne!(
            Puzzle3StateKey::from_state(&flat),
            Puzzle3StateKey::from_state(&tall)
        );

        let empty = ObjectId::EMPTY;
        assert!(flat.slots().iter().all(|object| *object == empty));
        assert!(tall.slots().iter().all(|object| *object == empty));
    }
}
