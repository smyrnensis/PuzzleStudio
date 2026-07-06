use crate::{
    Direction3, InputId3, LevelBundle3, LevelBundleError3, Rule3, State3, TransitionError3,
    WinCondition3, transition_program, transition_program_without_input_with_local_frame,
};
use puzzle_runtime_contract::{Lifecycle3, LifecycleCommand3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionLifecycleResult3 {
    pub changed: bool,
    pub cleared: bool,
    pub level_changed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameSession3 {
    current_level_index: usize,
    initial_state: State3,
    current_state: State3,
    undo_stack: Vec<SessionHistoryEntry3>,
    move_count: u32,
    completed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionHistoryEntry3 {
    state: State3,
    move_count: u32,
    completed: bool,
}

impl GameSession3 {
    pub fn new(bundle: &LevelBundle3) -> Result<Self, GameSessionError3> {
        Self::with_level(bundle, 0)
    }

    pub fn with_level(
        bundle: &LevelBundle3,
        level_index: usize,
    ) -> Result<Self, GameSessionError3> {
        Self::with_level_and_lifecycle(bundle, level_index, &Lifecycle3::default())
    }

    pub fn new_with_lifecycle(
        bundle: &LevelBundle3,
        lifecycle: &Lifecycle3,
    ) -> Result<Self, GameSessionError3> {
        Self::with_level_and_lifecycle(bundle, 0, lifecycle)
    }

    pub fn with_level_and_lifecycle(
        bundle: &LevelBundle3,
        level_index: usize,
        lifecycle: &Lifecycle3,
    ) -> Result<Self, GameSessionError3> {
        bundle.validate()?;
        let initial_state = build_level_state_with_lifecycle(bundle, level_index, lifecycle)?;
        Ok(Self {
            current_level_index: level_index,
            current_state: initial_state.clone(),
            initial_state,
            undo_stack: Vec::new(),
            move_count: 0,
            completed: false,
        })
    }

    pub fn current_level_index(&self) -> usize {
        self.current_level_index
    }

    pub fn initial_state(&self) -> &State3 {
        &self.initial_state
    }

    pub fn state(&self) -> &State3 {
        &self.current_state
    }

    pub fn move_count(&self) -> u32 {
        self.move_count
    }

    pub fn completed(&self) -> bool {
        self.completed
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn has_next_level(&self, bundle: &LevelBundle3) -> bool {
        self.current_level_index + 1 < bundle.level_count()
    }

    pub fn has_previous_level(&self) -> bool {
        self.current_level_index > 0
    }

    pub fn set_completed(&mut self, completed: bool) {
        self.completed = completed;
    }

    pub fn mark_completed(&mut self) {
        self.completed = true;
    }

    pub fn apply_input(
        &mut self,
        bundle: &LevelBundle3,
        rules: &[Rule3],
        input: InputId3,
    ) -> Result<bool, GameSessionError3> {
        let next = transition_program(&bundle.game, &self.current_state, rules, input)?;
        if next == self.current_state {
            return Ok(false);
        }

        self.undo_stack.push(SessionHistoryEntry3 {
            state: self.current_state.clone(),
            move_count: self.move_count,
            completed: self.completed,
        });
        self.current_state = next;
        self.move_count = self.move_count.saturating_add(1);
        self.completed = false;
        Ok(true)
    }

    pub fn apply_input_with_win_condition(
        &mut self,
        bundle: &LevelBundle3,
        rules: &[Rule3],
        input: InputId3,
        win_condition: &WinCondition3,
    ) -> Result<bool, GameSessionError3> {
        let changed = self.apply_input(bundle, rules, input)?;
        if changed {
            self.completed = win_condition.is_met(&bundle.game, &self.current_state);
        }
        Ok(changed)
    }

    pub fn apply_input_with_lifecycle(
        &mut self,
        bundle: &LevelBundle3,
        rules: &[Rule3],
        input: InputId3,
        win_condition: &WinCondition3,
        lifecycle: &Lifecycle3,
    ) -> Result<SessionLifecycleResult3, GameSessionError3> {
        let was_completed = self.completed;
        let changed = self.apply_input_with_win_condition(bundle, rules, input, win_condition)?;
        let cleared = changed && !was_completed && self.completed;
        let mut level_changed = false;
        if cleared {
            level_changed = self.run_level_clear_lifecycle(bundle, lifecycle)?;
        }
        Ok(SessionLifecycleResult3 {
            changed,
            cleared,
            level_changed,
        })
    }

    pub fn move_direction(
        &mut self,
        bundle: &LevelBundle3,
        rules: &[Rule3],
        direction: Direction3,
    ) -> Result<bool, GameSessionError3> {
        let input = bundle
            .game
            .inputs
            .iter()
            .find(|input| input.direction == Some(direction))
            .map(|input| input.id)
            .ok_or(GameSessionError3::MissingInputForDirection {
                direction: direction.name,
            })?;
        self.apply_input(bundle, rules, input)
    }

    pub fn move_direction_with_win_condition(
        &mut self,
        bundle: &LevelBundle3,
        rules: &[Rule3],
        direction: Direction3,
        win_condition: &WinCondition3,
    ) -> Result<bool, GameSessionError3> {
        let input = bundle
            .game
            .inputs
            .iter()
            .find(|input| input.direction == Some(direction))
            .map(|input| input.id)
            .ok_or(GameSessionError3::MissingInputForDirection {
                direction: direction.name,
            })?;
        self.apply_input_with_win_condition(bundle, rules, input, win_condition)
    }

    pub fn refresh_completed(&mut self, bundle: &LevelBundle3, win_condition: &WinCondition3) {
        self.completed = win_condition.is_met(&bundle.game, &self.current_state);
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        self.current_state = previous.state;
        self.move_count = previous.move_count;
        self.completed = previous.completed;
        true
    }

    pub fn restart(&mut self) -> bool {
        let changed = self.current_state != self.initial_state
            || self.move_count != 0
            || self.completed
            || !self.undo_stack.is_empty();
        self.current_state = self.initial_state.clone();
        self.undo_stack.clear();
        self.move_count = 0;
        self.completed = false;
        changed
    }

    pub fn restart_with_lifecycle(
        &mut self,
        bundle: &LevelBundle3,
        lifecycle: &Lifecycle3,
    ) -> Result<bool, GameSessionError3> {
        let initial_state =
            build_level_state_with_lifecycle(bundle, self.current_level_index, lifecycle)?;
        let changed = self.current_state != initial_state
            || self.move_count != 0
            || self.completed
            || !self.undo_stack.is_empty();
        self.current_state = initial_state.clone();
        self.initial_state = initial_state;
        self.undo_stack.clear();
        self.move_count = 0;
        self.completed = false;
        Ok(changed)
    }

    pub fn goto_level(
        &mut self,
        bundle: &LevelBundle3,
        level_index: usize,
    ) -> Result<bool, GameSessionError3> {
        self.goto_level_with_lifecycle(bundle, level_index, &Lifecycle3::default())
    }

    pub fn goto_level_with_lifecycle(
        &mut self,
        bundle: &LevelBundle3,
        level_index: usize,
        lifecycle: &Lifecycle3,
    ) -> Result<bool, GameSessionError3> {
        bundle.validate()?;
        let initial_state = build_level_state_with_lifecycle(bundle, level_index, lifecycle)?;
        let changed = self.current_level_index != level_index
            || self.current_state != initial_state
            || self.move_count != 0
            || self.completed
            || !self.undo_stack.is_empty();
        self.current_level_index = level_index;
        self.current_state = initial_state.clone();
        self.initial_state = initial_state;
        self.undo_stack.clear();
        self.move_count = 0;
        self.completed = false;
        Ok(changed)
    }

    pub fn next_level(&mut self, bundle: &LevelBundle3) -> Result<bool, GameSessionError3> {
        self.next_level_with_lifecycle(bundle, &Lifecycle3::default())
    }

    pub fn next_level_with_lifecycle(
        &mut self,
        bundle: &LevelBundle3,
        lifecycle: &Lifecycle3,
    ) -> Result<bool, GameSessionError3> {
        if !self.has_next_level(bundle) {
            return Ok(false);
        }
        self.goto_level_with_lifecycle(bundle, self.current_level_index + 1, lifecycle)
    }

    pub fn previous_level(&mut self, bundle: &LevelBundle3) -> Result<bool, GameSessionError3> {
        self.previous_level_with_lifecycle(bundle, &Lifecycle3::default())
    }

    pub fn previous_level_with_lifecycle(
        &mut self,
        bundle: &LevelBundle3,
        lifecycle: &Lifecycle3,
    ) -> Result<bool, GameSessionError3> {
        if !self.has_previous_level() {
            return Ok(false);
        }
        self.goto_level_with_lifecycle(bundle, self.current_level_index - 1, lifecycle)
    }

    fn run_level_clear_lifecycle(
        &mut self,
        bundle: &LevelBundle3,
        lifecycle: &Lifecycle3,
    ) -> Result<bool, GameSessionError3> {
        let mut level_changed = false;
        let commands = if self.current_level_index + 1 >= bundle.level_count() {
            lifecycle
                .on_last_level_clear
                .as_deref()
                .unwrap_or(&lifecycle.on_level_clear)
        } else {
            &lifecycle.on_level_clear
        };
        for command in commands {
            match command {
                LifecycleCommand3::NextLevel => {
                    if self.next_level_with_lifecycle(bundle, lifecycle)? {
                        level_changed = true;
                    }
                }
            }
        }
        Ok(level_changed)
    }
}

fn build_level_state_with_lifecycle(
    bundle: &LevelBundle3,
    level_index: usize,
    lifecycle: &Lifecycle3,
) -> Result<State3, GameSessionError3> {
    let state = bundle.build_level_state(level_index)?;
    Ok(transition_program_without_input_with_local_frame(
        &bundle.game,
        &state,
        &lifecycle.on_level_start,
        lifecycle.on_level_start_local_frame.as_ref(),
    )?)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameSessionError3 {
    LevelBundle(LevelBundleError3),
    Transition(TransitionError3),
    MissingInputForDirection { direction: &'static str },
}

impl From<LevelBundleError3> for GameSessionError3 {
    fn from(value: LevelBundleError3) -> Self {
        Self::LevelBundle(value)
    }
}

impl From<TransitionError3> for GameSessionError3 {
    fn from(value: TransitionError3) -> Self {
        Self::Transition(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Coord3, Direction3, Game3, InputDef3, LayerId, Level3, LevelCell3, LevelEntry3, MatchCell3,
        ObjectDef3, ObjectId, Offset3, Pattern3, Size3, WriteOp3,
    };

    const PLAYER: ObjectId = ObjectId(1);
    const BOX: ObjectId = ObjectId(2);
    const WALL: ObjectId = ObjectId(3);
    const GOAL: ObjectId = ObjectId(4);
    const ACTOR: LayerId = LayerId(0);
    const FLOOR: LayerId = LayerId(1);
    const INPUT_LEFT: InputId3 = InputId3(0);
    const INPUT_RIGHT: InputId3 = InputId3(1);

    fn session_game() -> Game3 {
        Game3::new_with_inputs(
            1,
            vec![
                ObjectDef3 {
                    id: PLAYER,
                    layer_id: ACTOR,
                },
                ObjectDef3 {
                    id: BOX,
                    layer_id: ACTOR,
                },
                ObjectDef3 {
                    id: WALL,
                    layer_id: ACTOR,
                },
            ],
            vec![
                InputDef3::directional(INPUT_LEFT, "left", Direction3::LEFT),
                InputDef3::directional(INPUT_RIGHT, "right", Direction3::RIGHT),
            ],
        )
    }

    fn session_bundle() -> LevelBundle3 {
        LevelBundle3::checked_new(
            session_game(),
            vec![
                LevelEntry3::new(
                    "one",
                    Level3::new(
                        Size3::new(4, 1, 1),
                        vec![LevelCell3::new(Coord3::new(0, 0, 0), vec![PLAYER])],
                    ),
                ),
                LevelEntry3::new(
                    "two",
                    Level3::new(
                        Size3::new(4, 1, 1),
                        vec![LevelCell3::new(Coord3::new(2, 0, 0), vec![PLAYER])],
                    ),
                ),
            ],
        )
        .unwrap()
    }

    fn move_right_rule() -> Rule3 {
        Rule3::once(
            Pattern3::new(vec![
                MatchCell3::new(Offset3::ZERO).require(PLAYER),
                MatchCell3::new(Direction3::RIGHT.offset)
                    .forbid(PLAYER)
                    .forbid(BOX)
                    .forbid(WALL),
            ]),
            vec![WriteOp3::Move {
                component: 0,
                from_offset: Offset3::ZERO,
                to_offset: Direction3::RIGHT.offset,
                object: PLAYER,
            }],
        )
        .when_input(INPUT_RIGHT)
    }

    fn push_right_rule() -> Rule3 {
        Rule3::once(
            Pattern3::new(vec![
                MatchCell3::new(Offset3::ZERO).require(PLAYER),
                MatchCell3::new(Direction3::RIGHT.offset).require(BOX),
                MatchCell3::new(Direction3::RIGHT.offset.scale(2))
                    .forbid(PLAYER)
                    .forbid(BOX)
                    .forbid(WALL),
            ]),
            vec![
                WriteOp3::Move {
                    component: 0,
                    from_offset: Direction3::RIGHT.offset,
                    to_offset: Direction3::RIGHT.offset.scale(2),
                    object: BOX,
                },
                WriteOp3::Move {
                    component: 0,
                    from_offset: Offset3::ZERO,
                    to_offset: Direction3::RIGHT.offset,
                    object: PLAYER,
                },
            ],
        )
        .when_input(INPUT_RIGHT)
    }

    fn support_win_condition() -> WinCondition3 {
        WinCondition3::All(vec![
            WinCondition3::SomeObject(GOAL),
            WinCondition3::AllObjectsCoveredByPattern {
                object: GOAL,
                cover_pattern: Pattern3::new(vec![
                    MatchCell3::new(Offset3::ZERO).require(BOX),
                    MatchCell3::new(Direction3::DOWN.offset).require(GOAL),
                ]),
            },
        ])
    }

    fn support_bundle() -> LevelBundle3 {
        LevelBundle3::checked_new(
            Game3::new_with_inputs(
                2,
                vec![
                    ObjectDef3 {
                        id: PLAYER,
                        layer_id: ACTOR,
                    },
                    ObjectDef3 {
                        id: BOX,
                        layer_id: ACTOR,
                    },
                    ObjectDef3 {
                        id: WALL,
                        layer_id: ACTOR,
                    },
                    ObjectDef3 {
                        id: GOAL,
                        layer_id: FLOOR,
                    },
                ],
                vec![InputDef3::directional(
                    INPUT_RIGHT,
                    "right",
                    Direction3::RIGHT,
                )],
            ),
            vec![LevelEntry3::new(
                "support",
                Level3::new(
                    Size3::new(3, 1, 2),
                    vec![
                        LevelCell3::new(Coord3::new(0, 0, 1), vec![PLAYER]),
                        LevelCell3::new(Coord3::new(1, 0, 1), vec![BOX]),
                        LevelCell3::new(Coord3::new(2, 0, 0), vec![GOAL]),
                    ],
                ),
            )],
        )
        .unwrap()
    }

    #[test]
    fn session_starts_from_first_bundle_level() {
        let bundle = session_bundle();
        let session = GameSession3::new(&bundle).unwrap();

        assert_eq!(session.current_level_index(), 0);
        assert_eq!(session.move_count(), 0);
        assert!(!session.completed());
        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(0, 0, 0), PLAYER)
        );
    }

    #[test]
    fn session_applies_input_and_records_undo_history() {
        let bundle = session_bundle();
        let rules = vec![move_right_rule()];
        let mut session = GameSession3::new(&bundle).unwrap();

        assert!(session.apply_input(&bundle, &rules, INPUT_RIGHT).unwrap());
        assert_eq!(session.move_count(), 1);
        assert!(session.can_undo());
        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(1, 0, 0), PLAYER)
        );

        assert!(session.undo());
        assert_eq!(session.move_count(), 0);
        assert!(!session.can_undo());
        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(0, 0, 0), PLAYER)
        );
    }

    #[test]
    fn session_does_not_count_noop_input_as_move() {
        let bundle = session_bundle();
        let rules = vec![move_right_rule()];
        let mut session = GameSession3::new(&bundle).unwrap();

        assert!(!session.apply_input(&bundle, &rules, INPUT_LEFT).unwrap());

        assert_eq!(session.move_count(), 0);
        assert!(!session.can_undo());
    }

    #[test]
    fn session_restart_resets_state_count_completion_and_history() {
        let bundle = session_bundle();
        let rules = vec![move_right_rule()];
        let mut session = GameSession3::new(&bundle).unwrap();
        session.apply_input(&bundle, &rules, INPUT_RIGHT).unwrap();
        session.mark_completed();

        assert!(session.restart());

        assert_eq!(session.move_count(), 0);
        assert!(!session.completed());
        assert!(!session.can_undo());
        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(0, 0, 0), PLAYER)
        );
    }

    #[test]
    fn session_can_step_between_bundle_levels() {
        let bundle = session_bundle();
        let mut session = GameSession3::new(&bundle).unwrap();

        assert!(session.next_level(&bundle).unwrap());
        assert_eq!(session.current_level_index(), 1);
        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(2, 0, 0), PLAYER)
        );
        assert!(!session.next_level(&bundle).unwrap());

        assert!(session.previous_level(&bundle).unwrap());
        assert_eq!(session.current_level_index(), 0);
    }

    #[test]
    fn session_move_direction_resolves_directional_input() {
        let bundle = session_bundle();
        let rules = vec![move_right_rule()];
        let mut session = GameSession3::new(&bundle).unwrap();

        assert!(
            session
                .move_direction(&bundle, &rules, Direction3::RIGHT)
                .unwrap()
        );

        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(1, 0, 0), PLAYER)
        );
    }

    #[test]
    fn session_can_update_completion_from_win_condition_after_input() {
        let bundle = support_bundle();
        let rules = vec![push_right_rule()];
        let win = support_win_condition();
        let mut session = GameSession3::new(&bundle).unwrap();

        assert!(!session.completed());

        assert!(
            session
                .move_direction_with_win_condition(&bundle, &rules, Direction3::RIGHT, &win)
                .unwrap()
        );

        assert!(session.completed());
        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(2, 0, 1), BOX)
        );
        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(2, 0, 0), GOAL)
        );
    }
}
