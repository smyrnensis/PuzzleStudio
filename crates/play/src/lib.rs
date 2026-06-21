use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
};

use puzzle_core::{
    InputId, LayerId, ObjectId, PatchOp, RuleStep, State as PuzzleState, TransitionCommand,
    TransitionError, transition_outcome, transition_program, transition_program_outcome,
    transition_program_trace,
};
use puzzle_lang::{
    AsciiLegend, Level, LevelMenuDef, LoadedGame, ResourceSelection, RuleAnimation,
    RuleAnimationTrigger, RuleEffect, SceneComponent, SceneEffect, SceneEffectParam, SceneExpr,
    ScenePuzzleInitializer, SceneTransitionTrigger, SceneValue,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SoundEvent {
    PlaySfx { name: String },
    PlayMusic { name: String },
    PauseMusic { name: Option<String> },
    ResumeMusic { name: Option<String> },
    StopMusic { name: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageEvent {
    Message { text: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaitEvent {
    Wait { milliseconds: u64 },
    ContinueEffects { milliseconds: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnimationEvent {
    Move {
        name: String,
        object: ObjectId,
        from_x: u16,
        from_y: u16,
        from_z: u16,
        to_x: u16,
        to_y: u16,
        to_z: u16,
    },
    CantMove {
        name: String,
        object: ObjectId,
        x: u16,
        y: u16,
    },
}

#[derive(Clone, Debug, Default)]
pub struct SceneRuntimeState {
    pub values: HashMap<String, SceneValue>,
    pub puzzles: HashMap<String, ScenePuzzleRuntimeState>,
}

#[derive(Clone, Debug)]
pub struct ScenePuzzleRuntimeState {
    pub state: PuzzleState,
    pub initial_state: PuzzleState,
    pub checkpoint_state: Option<PuzzleState>,
    pub level_index: Option<usize>,
}

impl Deref for ScenePuzzleRuntimeState {
    type Target = PuzzleState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

#[derive(Clone, Debug, Default)]
struct ModelInputResult {
    cancelled: bool,
    effects: Vec<QueuedRuleEffect>,
    checkpoint: Option<PendingProgramContinuation>,
}

#[derive(Clone, Debug)]
struct QueuedTransitionCommand {
    target: Option<String>,
    command: TransitionCommand,
}

#[derive(Clone, Debug)]
struct QueuedRuleEffect {
    target: Option<String>,
    effect: RuleEffect,
}

#[derive(Clone, Debug)]
struct PendingEffectContinuation {
    effects: Vec<QueuedRuleEffect>,
    condition_effect: Option<SceneEffect>,
}

#[derive(Clone, Debug)]
struct PendingProgramContinuation {
    target: Option<String>,
    input: InputId,
    wait_ms: u64,
    remaining_program: Vec<RuleStep>,
    effects_after_wait: Vec<QueuedRuleEffect>,
    mode: PendingProgramMode,
}

#[derive(Clone, Debug)]
enum PendingProgramMode {
    TurnCompletion,
}

#[derive(Clone, Debug)]
struct ProgramSegmentOutcome {
    next_state: PuzzleState,
    cancelled: bool,
    effects: Vec<QueuedRuleEffect>,
    animations: Vec<AnimationEvent>,
    checkpoint: Option<EffectCheckpoint>,
}

#[derive(Clone, Debug)]
struct EffectCheckpoint {
    milliseconds: u64,
    effects_after_wait: Vec<QueuedRuleEffect>,
    remaining_program: Vec<RuleStep>,
}

#[derive(Clone, Debug)]
struct LifecycleOutcome {
    next_state: PuzzleState,
    cancelled: bool,
    commands: Vec<TransitionCommand>,
    fired_rules: Vec<puzzle_core::RuleId>,
}

pub const PROGRESS_SAVE_VERSION: u32 = 1;
const MAX_AGAIN_TURNS_PER_INPUT: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgressSaveData {
    pub version: u32,
    pub levels: Vec<LevelProgressSaveData>,
    pub current_level: Option<String>,
    pub persistent_vars: Vec<PersistentVarSaveData>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelProgressSaveData {
    pub name: String,
    pub cleared: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentVarSaveData {
    pub name: String,
    pub value: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgressSaveError {
    UnsupportedVersion(u32),
}

#[derive(Clone, Debug)]
pub struct GameSession {
    level_index: usize,
    active_level_index: Option<usize>,
    state: PuzzleState,
    undo_stack: Vec<PuzzleState>,
    redo_stack: Vec<PuzzleState>,
    cleared_levels: Vec<bool>,
    focused_scene: String,
    visible_scenes: Vec<String>,
    focus_history: Vec<String>,
    scene_states: HashMap<String, SceneRuntimeState>,
    selected_level_index: usize,
    level_checkpoint_state: Option<PuzzleState>,
    session_values: HashMap<String, SceneValue>,
    persistent_vars: Vec<i64>,
    current_input: Option<String>,
    current_turn_sfx: Option<HashSet<String>>,
    sound_events: Vec<SoundEvent>,
    message_events: Vec<MessageEvent>,
    wait_events: Vec<WaitEvent>,
    animation_events: Vec<AnimationEvent>,
    pending_effect_continuation: Option<PendingEffectContinuation>,
    pending_program_continuation: Option<PendingProgramContinuation>,
}

impl GameSession {
    pub fn new(game: &LoadedGame) -> Self {
        let neutral_state = neutral_state(game);
        let mut session = Self {
            level_index: 0,
            active_level_index: None,
            state: neutral_state,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            cleared_levels: vec![false; game.levels.len()],
            focused_scene: initial_scene_name(game).to_string(),
            visible_scenes: vec![initial_scene_name(game).to_string()],
            focus_history: Vec::new(),
            scene_states: HashMap::new(),
            selected_level_index: 0,
            level_checkpoint_state: None,
            session_values: game
                .variables
                .iter()
                .map(|variable| (variable.name.clone(), variable.default.clone()))
                .collect(),
            persistent_vars: persistent_var_default_values(game),
            current_input: None,
            current_turn_sfx: None,
            sound_events: Vec::new(),
            message_events: Vec::new(),
            wait_events: Vec::new(),
            animation_events: Vec::new(),
            pending_effect_continuation: None,
            pending_program_continuation: None,
        };
        let initial_scene = session.focused_scene.clone();
        if !game_has_scene_level_owner(game) {
            let _ = session.activate_level(game, 0, true);
        }
        session.create_scene(game, &initial_scene);
        let _ = session.apply_scene_start_transition(game);
        let _ = session.apply_level_start_transition(game);
        session
    }

    pub fn screen(&self) -> &str {
        &self.focused_scene
    }

    pub fn scene(&self) -> &str {
        &self.focused_scene
    }

    pub fn visible_scenes(&self) -> &[String] {
        &self.visible_scenes
    }

    pub fn focused_scene(&self) -> &str {
        &self.focused_scene
    }

    pub fn level_index(&self) -> usize {
        self.active_level_index.unwrap_or(self.level_index)
    }

    pub fn active_level_index(&self) -> Option<usize> {
        self.active_level_index
    }

    pub fn selected_level_index(&self) -> usize {
        self.selected_level_index
    }

    pub fn current_level<'a>(&self, game: &'a LoadedGame) -> &'a Level {
        &game.levels[self.level_index()]
    }

    pub fn state(&self) -> &PuzzleState {
        &self.state
    }

    pub fn scene_state(&self) -> Option<&SceneRuntimeState> {
        self.scene_states.get(&self.focused_scene)
    }

    pub fn scene_state_for(&self, name: &str) -> Option<&SceneRuntimeState> {
        self.scene_states.get(name)
    }

    pub fn session_values(&self) -> &HashMap<String, SceneValue> {
        &self.session_values
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn cleared_levels(&self) -> &[bool] {
        &self.cleared_levels
    }

    pub fn progress_save_data(&self, game: &LoadedGame) -> ProgressSaveData {
        ProgressSaveData {
            version: PROGRESS_SAVE_VERSION,
            levels: game
                .levels
                .iter()
                .enumerate()
                .map(|(index, level)| LevelProgressSaveData {
                    name: level.name.clone(),
                    cleared: self.cleared_levels.get(index).copied().unwrap_or(false),
                })
                .collect(),
            current_level: self
                .preferred_level_index(game)
                .and_then(|index| game.levels.get(index))
                .map(|level| level.name.clone()),
            persistent_vars: game
                .persistent_vars
                .iter()
                .enumerate()
                .filter_map(|(index, var)| {
                    let name = game.global_labels.get(var)?;
                    let value = self.persistent_vars.get(index).copied().unwrap_or(0);
                    Some(PersistentVarSaveData {
                        name: name.clone(),
                        value,
                    })
                })
                .collect(),
        }
    }

    pub fn restore_progress_save_data(
        &mut self,
        game: &LoadedGame,
        save: &ProgressSaveData,
    ) -> Result<(), ProgressSaveError> {
        if save.version != PROGRESS_SAVE_VERSION {
            return Err(ProgressSaveError::UnsupportedVersion(save.version));
        }

        self.cleared_levels.fill(false);
        for saved_level in &save.levels {
            if !saved_level.cleared {
                continue;
            }
            if let Some(index) = game
                .levels
                .iter()
                .position(|level| level.name == saved_level.name)
            {
                if let Some(cleared) = self.cleared_levels.get_mut(index) {
                    *cleared = true;
                }
            }
        }

        for saved_var in &save.persistent_vars {
            if let Some(index) = game
                .persistent_vars
                .iter()
                .position(|var| game.global_labels.get(var) == Some(&saved_var.name))
            {
                if let Some(value) = self.persistent_vars.get_mut(index) {
                    *value = saved_var.value;
                }
            }
        }

        if let Some(level_name) = &save.current_level {
            if let Some(index) = game
                .levels
                .iter()
                .position(|level| &level.name == level_name)
            {
                self.selected_level_index = index;
                if self.active_level_index.is_some() || !game_has_scene_level_owner(game) {
                    let _ = self.activate_level(game, index, false);
                }
            }
        }
        self.sync_persistent_vars_to_scene_states(game);
        Ok(())
    }

    fn preferred_level_index(&self, game: &LoadedGame) -> Option<usize> {
        self.active_level_index.or_else(|| {
            (self.selected_level_index < game.levels.len()).then_some(self.selected_level_index)
        })
    }

    pub fn take_sound_events(&mut self) -> Vec<SoundEvent> {
        std::mem::take(&mut self.sound_events)
    }

    pub fn take_message_events(&mut self) -> Vec<MessageEvent> {
        std::mem::take(&mut self.message_events)
    }

    pub fn take_wait_events(&mut self) -> Vec<WaitEvent> {
        std::mem::take(&mut self.wait_events)
    }

    pub fn take_animation_events(&mut self) -> Vec<AnimationEvent> {
        std::mem::take(&mut self.animation_events)
    }

    fn mark_current_level_cleared(&mut self) {
        let Some(level_index) = self.active_level_index else {
            return;
        };
        if let Some(cleared) = self.cleared_levels.get_mut(level_index) {
            *cleared = true;
        }
    }

    fn clear_undo_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    fn clear_game_progress(&mut self, game: &LoadedGame) {
        self.cleared_levels.fill(false);
        self.selected_level_index = 0;
        self.reset_persistent_vars(game);
        self.clear_undo_history();
    }

    fn clear_current_level_progress(&mut self) {
        self.selected_level_index = 0;
    }

    fn set_current_level_progress(
        &mut self,
        game: &LoadedGame,
        level: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) {
        if let Some(index) = self.eval_effect_level_index(game, level, bindings) {
            self.selected_level_index = index;
        }
    }

    fn set_level_cleared_progress(
        &mut self,
        game: &LoadedGame,
        level: Option<&SceneExpr>,
        cleared: bool,
        bindings: &HashMap<String, String>,
    ) {
        let index = match level {
            Some(level) => self.eval_effect_level_index(game, level, bindings),
            None => self.active_level_index.or_else(|| {
                (self.selected_level_index < game.levels.len()).then_some(self.selected_level_index)
            }),
        };
        if let Some(index) = index {
            if let Some(entry) = self.cleared_levels.get_mut(index) {
                *entry = cleared;
            }
        }
    }

    fn reset_persistent_vars(&mut self, game: &LoadedGame) {
        self.persistent_vars = persistent_var_default_values(game);
        self.sync_persistent_vars_to_scene_states(game);
    }

    fn reset_persistent_var(&mut self, game: &LoadedGame, name: &str) -> bool {
        let Some(index) = persistent_var_index_by_name(game, name) else {
            return false;
        };
        let defaults = persistent_var_default_values(game);
        let Some(value) = self.persistent_vars.get_mut(index) else {
            return true;
        };
        *value = defaults.get(index).copied().unwrap_or(0);
        self.sync_persistent_vars_to_scene_states(game);
        true
    }

    fn apply_persistent_vars(&self, game: &LoadedGame, state: &mut PuzzleState) {
        for (index, var) in game.persistent_vars.iter().enumerate() {
            if let Some(value) = self.persistent_vars.get(index) {
                let _ = state.set_visible_global(*var, *value);
            }
        }
    }

    fn capture_persistent_vars(&mut self, game: &LoadedGame, state: &PuzzleState) {
        self.persistent_vars = persistent_var_values(game, state);
    }

    fn sync_persistent_vars_to_scene_states(&mut self, game: &LoadedGame) {
        let vars = self.persistent_vars.clone();
        apply_persistent_var_values(game, &vars, &mut self.state);
        if let Some(state) = &mut self.level_checkpoint_state {
            apply_persistent_var_values(game, &vars, state);
        }
        for screen in self.scene_states.values_mut() {
            for puzzle in screen.puzzles.values_mut() {
                apply_persistent_var_values(game, &vars, &mut puzzle.state);
                apply_persistent_var_values(game, &vars, &mut puzzle.initial_state);
                if let Some(state) = &mut puzzle.checkpoint_state {
                    apply_persistent_var_values(game, &vars, state);
                }
            }
        }
    }

    pub fn has_next_level(&self, game: &LoadedGame) -> bool {
        let Some(level_index) = self.active_level_index else {
            return false;
        };
        let indices = scene_level_indices(game, &self.focused_scene);
        indices
            .iter()
            .position(|index| *index == level_index)
            .is_some_and(|position| position + 1 < indices.len())
    }

    pub fn apply_input(
        &mut self,
        game: &LoadedGame,
        input: InputId,
    ) -> Result<(), TransitionError> {
        let previous_input = self.current_input.clone();
        self.current_input = game.input_labels.get(&input).cloned();
        let owns_turn_sfx = self.begin_turn_sfx_dedup();
        let result = match self.apply_model_input(game, input) {
            Ok(result) => result,
            Err(error) => {
                self.current_input = previous_input;
                self.end_turn_sfx_dedup(owns_turn_sfx);
                return Err(error);
            }
        };
        if !result.cancelled {
            if let Some(checkpoint) = result.checkpoint {
                self.queue_program_continuation(checkpoint);
                let condition_result = self.resolve_turn_effects(game, result.effects, None);
                self.current_input = previous_input;
                self.end_turn_sfx_dedup(owns_turn_sfx);
                condition_result?;
                return Ok(());
            }
            let condition_result = self.apply_turn_completion(game, result.effects);
            self.current_input = previous_input;
            self.end_turn_sfx_dedup(owns_turn_sfx);
            condition_result?;
            return Ok(());
        }
        self.current_input = previous_input;
        self.end_turn_sfx_dedup(owns_turn_sfx);
        Ok(())
    }

    fn apply_model_input(
        &mut self,
        game: &LoadedGame,
        input: InputId,
    ) -> Result<ModelInputResult, TransitionError> {
        let target = game
            .scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
            .and_then(|screen| screen.puzzle_rule.as_ref())
            .map(|rule| rule.target.clone());
        if let Some(target) = target {
            return self.apply_model_input_to_target(game, &target, input);
        }

        if self.active_level_index.is_none() {
            return Ok(ModelInputResult::default());
        }
        let mut state = self.state.clone();
        self.apply_persistent_vars(game, &mut state);
        let outcome =
            transition_program_segment_outcome(game, game.game.program(), &state, input, None)?;
        let cancelled = outcome.cancelled;
        self.replace_state_if_changed(game, outcome.next_state);
        self.sync_current_level_puzzles(game);
        self.animation_events.extend(outcome.animations.clone());
        let checkpoint = outcome
            .checkpoint
            .map(|checkpoint| PendingProgramContinuation {
                target: None,
                input,
                wait_ms: checkpoint.milliseconds,
                remaining_program: checkpoint.remaining_program,
                effects_after_wait: checkpoint.effects_after_wait,
                mode: PendingProgramMode::TurnCompletion,
            });
        Ok(ModelInputResult {
            cancelled,
            effects: outcome.effects,
            checkpoint,
        })
    }

    fn apply_model_input_to_target(
        &mut self,
        game: &LoadedGame,
        target: &str,
        input: InputId,
    ) -> Result<ModelInputResult, TransitionError> {
        let Some((scene_name, puzzle_name)) = self.resolve_puzzle_target(game, target) else {
            return self.apply_model_input_to_current_level(game, input);
        };
        let Some(initializer) = scene_puzzle_initializer(game, &scene_name, &puzzle_name) else {
            return self.apply_model_input_to_current_level(game, input);
        };

        self.create_scene(game, &scene_name);
        let Some(mut state) = self
            .scene_states
            .get(&scene_name)
            .and_then(|scene_state| scene_state.puzzles.get(&puzzle_name))
            .cloned()
        else {
            return Ok(ModelInputResult::default());
        };
        self.apply_persistent_vars(game, &mut state.state);
        let outcome = transition_program_segment_outcome(
            game,
            game.game.program(),
            &state.state,
            input,
            Some(target),
        )?;
        let cancelled = outcome.cancelled;
        self.capture_persistent_vars(game, &outcome.next_state);
        self.animation_events.extend(outcome.animations.clone());
        let mut next_state = outcome.next_state.clone();
        self.apply_persistent_vars(game, &mut next_state);
        if let Some(puzzle) = self
            .scene_states
            .get_mut(&scene_name)
            .and_then(|scene_state| scene_state.puzzles.get_mut(&puzzle_name))
        {
            puzzle.state = next_state.clone();
        }
        if initializer == ScenePuzzleInitializer::CurrentLevel && scene_name == self.focused_scene {
            self.replace_state_if_changed(game, outcome.next_state);
        } else {
            self.sync_persistent_vars_to_scene_states(game);
        }
        let checkpoint = outcome
            .checkpoint
            .map(|checkpoint| PendingProgramContinuation {
                target: Some(target.to_string()),
                input,
                wait_ms: checkpoint.milliseconds,
                remaining_program: checkpoint.remaining_program,
                effects_after_wait: checkpoint.effects_after_wait,
                mode: PendingProgramMode::TurnCompletion,
            });
        Ok(ModelInputResult {
            cancelled,
            effects: outcome.effects,
            checkpoint,
        })
    }

    fn apply_model_input_to_current_level(
        &mut self,
        game: &LoadedGame,
        input: InputId,
    ) -> Result<ModelInputResult, TransitionError> {
        if self.active_level_index.is_none() {
            return Ok(ModelInputResult::default());
        }
        let mut state = self.state.clone();
        self.apply_persistent_vars(game, &mut state);
        let outcome =
            transition_program_segment_outcome(game, game.game.program(), &state, input, None)?;
        let cancelled = outcome.cancelled;
        self.replace_state_if_changed(game, outcome.next_state);
        self.sync_current_level_puzzles(game);
        self.animation_events.extend(outcome.animations.clone());
        let checkpoint = outcome
            .checkpoint
            .map(|checkpoint| PendingProgramContinuation {
                target: None,
                input,
                wait_ms: checkpoint.milliseconds,
                remaining_program: checkpoint.remaining_program,
                effects_after_wait: checkpoint.effects_after_wait,
                mode: PendingProgramMode::TurnCompletion,
            });
        Ok(ModelInputResult {
            cancelled,
            effects: outcome.effects,
            checkpoint,
        })
    }

    fn apply_turn_completion(
        &mut self,
        game: &LoadedGame,
        effects: Vec<QueuedRuleEffect>,
    ) -> Result<(), TransitionError> {
        let condition_effect = self.condition_transition_effect(game);
        let force_clear = effects
            .iter()
            .any(|effect| matches!(effect.effect, RuleEffect::Win));
        let mut effects = effects;
        effects.extend(self.apply_model_level_clear(game, force_clear)?);
        self.resolve_turn_effects(game, effects, condition_effect)
    }

    fn resolve_turn_effects(
        &mut self,
        game: &LoadedGame,
        mut effects: Vec<QueuedRuleEffect>,
        condition_effect: Option<SceneEffect>,
    ) -> Result<(), TransitionError> {
        let mut commands = Vec::new();
        let mut index = 0;
        while index < effects.len() {
            let effect = effects[index].clone();
            match effect.effect {
                RuleEffect::Win => {
                    commands.push(QueuedTransitionCommand {
                        target: effect.target,
                        command: TransitionCommand::Win,
                    });
                }
                RuleEffect::Restart => {
                    commands.push(QueuedTransitionCommand {
                        target: effect.target,
                        command: TransitionCommand::Restart,
                    });
                }
                RuleEffect::NextLevel => {
                    commands.push(QueuedTransitionCommand {
                        target: effect.target,
                        command: TransitionCommand::NextLevel,
                    });
                }
                RuleEffect::Again => {
                    commands.push(QueuedTransitionCommand {
                        target: effect.target,
                        command: TransitionCommand::Again,
                    });
                }
                RuleEffect::Checkpoint => {
                    self.save_checkpoint(game, effect.target.as_deref());
                }
                RuleEffect::ClearCheckpoint => {
                    self.clear_checkpoint(game, effect.target.as_deref());
                }
                RuleEffect::PlaySfx { name } => {
                    if self.should_emit_turn_sfx(&name) {
                        self.sound_events.push(SoundEvent::PlaySfx { name });
                    }
                }
                RuleEffect::PlayMusic { name } => {
                    self.sound_events.push(SoundEvent::PlayMusic { name });
                }
                RuleEffect::PauseMusic { name } => {
                    self.sound_events.push(SoundEvent::PauseMusic { name });
                }
                RuleEffect::ResumeMusic { name } => {
                    self.sound_events.push(SoundEvent::ResumeMusic { name });
                }
                RuleEffect::StopMusic { name } => {
                    self.sound_events.push(SoundEvent::StopMusic { name });
                }
                RuleEffect::Wait { milliseconds } => {
                    let remaining = effects.split_off(index + 1);
                    if !remaining.is_empty() || condition_effect.is_some() {
                        self.pending_effect_continuation = Some(PendingEffectContinuation {
                            effects: remaining,
                            condition_effect,
                        });
                        self.wait_events
                            .push(WaitEvent::ContinueEffects { milliseconds });
                    } else {
                        self.wait_events.push(WaitEvent::Wait { milliseconds });
                    }
                    return self.resolve_turn_commands(game, commands, None);
                }
                RuleEffect::WaitAnimation => {}
                RuleEffect::Message { text, literal } => {
                    let text = self.resolve_message_text(&text, literal);
                    self.message_events.push(MessageEvent::Message { text });
                    if self.pending_program_continuation.is_none() {
                        let remaining = effects.split_off(index + 1);
                        if !remaining.is_empty() || condition_effect.is_some() {
                            self.pending_effect_continuation = Some(PendingEffectContinuation {
                                effects: remaining,
                                condition_effect,
                            });
                            self.wait_events.push(WaitEvent::ContinueEffects {
                                milliseconds: game.default_wait_ms,
                            });
                        } else {
                            self.wait_events.push(WaitEvent::Wait {
                                milliseconds: game.default_wait_ms,
                            });
                        }
                        return self.resolve_turn_commands(game, commands, None);
                    }
                }
            }
            index += 1;
        }
        self.resolve_turn_commands(game, commands, condition_effect)
    }

    fn resume_effect_continuation(&mut self, game: &LoadedGame) -> Result<(), TransitionError> {
        if let Some(continuation) = self.pending_program_continuation.take() {
            return self.resume_program_continuation(game, continuation);
        }
        let Some(continuation) = self.pending_effect_continuation.take() else {
            return Ok(());
        };
        self.resolve_turn_effects(game, continuation.effects, continuation.condition_effect)
    }

    fn queue_program_continuation(&mut self, continuation: PendingProgramContinuation) {
        self.wait_events.push(WaitEvent::ContinueEffects {
            milliseconds: continuation.wait_ms,
        });
        self.pending_program_continuation = Some(continuation);
    }

    fn resume_program_continuation(
        &mut self,
        game: &LoadedGame,
        continuation: PendingProgramContinuation,
    ) -> Result<(), TransitionError> {
        let mut state = if let Some(target) = continuation.target.as_deref() {
            if let Some((scene_name, puzzle_name)) = self.resolve_puzzle_target(game, target) {
                self.scene_states
                    .get(&scene_name)
                    .and_then(|scene_state| scene_state.puzzles.get(&puzzle_name))
                    .map(|puzzle| puzzle.state.clone())
                    .unwrap_or_else(|| self.state.clone())
            } else {
                self.state.clone()
            }
        } else {
            self.state.clone()
        };
        self.apply_persistent_vars(game, &mut state);
        let target = continuation.target.as_deref();
        let outcome = transition_program_segment_outcome(
            game,
            &continuation.remaining_program,
            &state,
            continuation.input,
            target,
        )?;
        if let Some(target) = target {
            if let Some((scene_name, puzzle_name)) = self.resolve_puzzle_target(game, target) {
                let initializer = scene_puzzle_initializer(game, &scene_name, &puzzle_name);
                if let Some(puzzle) = self
                    .scene_states
                    .get_mut(&scene_name)
                    .and_then(|scene_state| scene_state.puzzles.get_mut(&puzzle_name))
                {
                    puzzle.state = outcome.next_state.clone();
                }
                if initializer == Some(ScenePuzzleInitializer::CurrentLevel)
                    && scene_name == self.focused_scene
                {
                    self.replace_state_if_changed(game, outcome.next_state);
                }
            }
        } else {
            self.replace_state_if_changed(game, outcome.next_state);
            self.sync_current_level_puzzles(game);
        }

        let mut effects = continuation.effects_after_wait;
        effects.extend(outcome.effects);
        if let Some(checkpoint) = outcome.checkpoint {
            self.pending_program_continuation = Some(PendingProgramContinuation {
                target: continuation.target,
                input: continuation.input,
                wait_ms: checkpoint.milliseconds,
                remaining_program: checkpoint.remaining_program,
                effects_after_wait: checkpoint.effects_after_wait,
                mode: continuation.mode,
            });
            self.wait_events.push(WaitEvent::ContinueEffects {
                milliseconds: checkpoint.milliseconds,
            });
            return self.resolve_turn_effects(game, effects, None);
        }

        match continuation.mode {
            PendingProgramMode::TurnCompletion if effects.is_empty() && game.goal.is_none() => {
                Ok(())
            }
            PendingProgramMode::TurnCompletion => self.apply_turn_completion(game, effects),
        }
    }

    fn resolve_turn_commands(
        &mut self,
        game: &LoadedGame,
        commands: Vec<QueuedTransitionCommand>,
        condition_effect: Option<SceneEffect>,
    ) -> Result<(), TransitionError> {
        let mut pending_next_level = None::<Option<String>>;
        let mut pending_again = None::<Option<String>>;
        let mut pending_restart = None::<Option<String>>;
        for command in commands {
            match command.command {
                TransitionCommand::Win => {}
                TransitionCommand::Restart => {
                    pending_restart.get_or_insert(command.target);
                }
                TransitionCommand::NextLevel => {
                    pending_next_level.get_or_insert(command.target);
                }
                TransitionCommand::Again => {
                    pending_again.get_or_insert(command.target);
                }
                TransitionCommand::Checkpoint | TransitionCommand::ClearCheckpoint => {}
            }
        }
        if let Some(target) = pending_restart {
            if let Some(target) = target {
                self.reset_puzzle_state(game, &target);
            } else {
                self.restart_level(game);
            }
            return Ok(());
        }
        if let Some(effect) = condition_effect {
            self.apply_screen_effect_during_turn(
                game,
                &effect,
                &HashMap::new(),
                &mut pending_next_level,
            )?;
        }
        if let Some(target) = pending_next_level {
            if let Some(target) = target {
                self.advance_level_from_target(game, &target);
            } else {
                self.advance_level(game);
            }
        } else if let Some(target) = pending_again {
            self.apply_again_turns(game, target)?;
        }
        Ok(())
    }

    fn apply_again_turns(
        &mut self,
        game: &LoadedGame,
        target: Option<String>,
    ) -> Result<(), TransitionError> {
        for _ in 0..MAX_AGAIN_TURNS_PER_INPUT {
            let previous_turn_sfx = self.begin_separate_turn_sfx_dedup();
            let result = match if let Some(target) = target.as_deref() {
                self.apply_model_input_to_target(game, target, InputId(0))
            } else {
                self.apply_model_input_to_current_level(game, InputId(0))
            } {
                Ok(result) => result,
                Err(error) => {
                    self.end_separate_turn_sfx_dedup(previous_turn_sfx);
                    return Err(error);
                }
            };
            if result.cancelled {
                self.end_separate_turn_sfx_dedup(previous_turn_sfx);
                return Ok(());
            }
            let has_again = result
                .effects
                .iter()
                .any(|effect| matches!(effect.effect, RuleEffect::Again));
            let effects: Vec<_> = result
                .effects
                .into_iter()
                .filter(|effect| !matches!(effect.effect, RuleEffect::Again))
                .collect();
            let completion = self.apply_turn_completion(game, effects);
            self.end_separate_turn_sfx_dedup(previous_turn_sfx);
            completion?;
            if !has_again {
                return Ok(());
            }
        }
        Ok(())
    }

    fn begin_turn_sfx_dedup(&mut self) -> bool {
        if self.current_turn_sfx.is_some() {
            return false;
        }
        self.current_turn_sfx = Some(HashSet::new());
        true
    }

    fn end_turn_sfx_dedup(&mut self, owned: bool) {
        if owned {
            self.current_turn_sfx = None;
        }
    }

    fn begin_separate_turn_sfx_dedup(&mut self) -> Option<HashSet<String>> {
        let previous = self.current_turn_sfx.take();
        self.current_turn_sfx = Some(HashSet::new());
        previous
    }

    fn end_separate_turn_sfx_dedup(&mut self, previous: Option<HashSet<String>>) {
        self.current_turn_sfx = previous;
    }

    fn should_emit_turn_sfx(&mut self, name: &str) -> bool {
        let Some(seen) = &mut self.current_turn_sfx else {
            return true;
        };
        seen.insert(name.to_string())
    }

    pub fn apply_command(
        &mut self,
        game: &LoadedGame,
        command: &str,
    ) -> Result<(), TransitionError> {
        match command {
            "__continue_effects" => {
                return self.resume_effect_continuation(game);
            }
            "undo" => {
                self.undo(game);
                return Ok(());
            }
            "redo" => {
                self.redo(game);
                return Ok(());
            }
            _ => {}
        }

        if runtime_command_has_quoted_level_arg(command) {
            return Err(TransitionError::InvalidCommand(command.to_string()));
        }

        if let Some(effect) = parse_runtime_command(command, game.default_wait_ms) {
            self.apply_screen_effect(game, &effect, &HashMap::new())?;
            return Ok(());
        }

        if let Some(effect) = parse_puzzle_runtime_command(command) {
            self.apply_screen_effect(game, &effect, &HashMap::new())?;
            return Ok(());
        }

        if self.current_scene_has_level_menu(game) && self.apply_level_menu_command(game, command) {
            return Ok(());
        }

        if self.apply_scene_input_command(game, command)? {
            return Ok(());
        }

        if self.current_scene_accepts_model_input(game) {
            if let Some(input) = input_id_by_label(game, command) {
                self.apply_input(game, input)?;
                return Ok(());
            }
        }

        Ok(())
    }

    fn apply_scene_input_command(
        &mut self,
        game: &LoadedGame,
        input: &str,
    ) -> Result<bool, TransitionError> {
        if !self.current_scene_has_input_transition(game, input) {
            return Ok(false);
        }
        let previous_input = self.current_input.clone();
        self.current_input = Some(input.to_string());
        let result = self.apply_turn_completion(game, Vec::new());
        self.current_input = previous_input;
        result?;
        Ok(true)
    }

    fn current_scene_has_input_transition(&self, game: &LoadedGame, input: &str) -> bool {
        game.scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
            .is_some_and(|screen| {
                screen.transitions.iter().any(|transition| {
                    transition_condition_mentions_input(&transition.trigger, input)
                })
            })
    }

    fn apply_input_name(&mut self, game: &LoadedGame, input: &str) -> Result<(), TransitionError> {
        let Some(input_id) = input_id_by_label(game, input) else {
            return Ok(());
        };

        if self.current_scene_accepts_model_input(game) {
            self.apply_input(game, input_id)?;
        } else {
            let previous_input = self.current_input.clone();
            self.current_input = Some(input.to_string());
            let result = self.apply_turn_completion(game, Vec::new());
            self.current_input = previous_input;
            result?;
        }

        Ok(())
    }

    fn apply_component_effect(
        &mut self,
        game: &LoadedGame,
        effect: &str,
    ) -> Result<(), TransitionError> {
        if self.current_scene_has_level_menu(game) && self.apply_level_menu_command(game, effect) {
            return Ok(());
        }

        Ok(())
    }

    fn condition_transition_effect(&self, game: &LoadedGame) -> Option<SceneEffect> {
        let Some(screen) = game
            .scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
        else {
            return None;
        };
        screen.transitions.iter().find_map(|transition| {
            let SceneTransitionTrigger::Condition(condition) = &transition.trigger else {
                return None;
            };
            self.is_screen_condition_true(game, condition)
                .then(|| transition.effect.clone())
        })
    }

    fn apply_model_level_clear(
        &mut self,
        game: &LoadedGame,
        force_clear: bool,
    ) -> Result<Vec<QueuedRuleEffect>, TransitionError> {
        if self.active_level_index.is_none() {
            return Ok(Vec::new());
        }
        if force_clear || game.is_goal_complete(&self.state) {
            self.mark_current_level_cleared();
            return self.apply_level_clear_hook(game, force_clear);
        }
        Ok(Vec::new())
    }

    fn apply_model_level_start(
        &mut self,
        game: &LoadedGame,
        emit_events: bool,
    ) -> Result<(), TransitionError> {
        let Some(level_index) = self.active_level_index else {
            return Ok(());
        };
        let mut state = self.state.clone();
        self.apply_persistent_vars(game, &mut state);
        let outcome = self.model_level_start_outcome(game, &state, level_index)?;
        if outcome.fired_rules.is_empty()
            && outcome.commands.is_empty()
            && outcome.next_state == state
        {
            self.state = state;
            self.sync_persistent_vars_to_scene_states(game);
            return Ok(());
        }
        let mut next_state = outcome.next_state.clone();
        self.capture_persistent_vars(game, &next_state);
        self.apply_persistent_vars(game, &mut next_state);
        self.state = next_state;
        self.sync_persistent_vars_to_scene_states(game);
        if emit_events && !outcome.cancelled {
            let effects =
                queued_effects_for_outcome(game, None, &outcome.commands, &outcome.fired_rules);
            self.resolve_turn_effects(game, effects, None)?;
        }
        Ok(())
    }

    fn activate_level(
        &mut self,
        game: &LoadedGame,
        level_index: usize,
        emit_events: bool,
    ) -> Result<(), TransitionError> {
        let Some(level) = game.levels.get(level_index) else {
            return Ok(());
        };
        self.level_index = level_index;
        self.active_level_index = Some(level_index);
        self.selected_level_index = level_index;
        self.level_checkpoint_state = None;
        self.state = level.initial_state.clone();
        apply_persistent_var_values(game, &self.persistent_vars, &mut self.state);
        self.apply_model_level_start(game, emit_events)
    }

    fn model_level_start_outcome(
        &self,
        game: &LoadedGame,
        state: &PuzzleState,
        level_index: usize,
    ) -> Result<LifecycleOutcome, TransitionError> {
        let mut outcome = LifecycleOutcome {
            next_state: state.clone(),
            cancelled: false,
            commands: Vec::new(),
            fired_rules: Vec::new(),
        };
        if let Some(program) = &game.level_start_program {
            self.extend_lifecycle_outcome(game, program, &mut outcome)?;
        } else if game.run_rules_on_level_start {
            let next = transition_outcome(&game.game, &outcome.next_state, InputId(0))?;
            outcome.next_state = next.next_state;
            outcome.cancelled |= next.cancelled;
            outcome.commands.extend(next.commands);
            outcome.fired_rules.extend(next.fired_rules);
        }
        if !outcome.cancelled {
            if let Some(program) = game
                .levels
                .get(level_index)
                .and_then(|level| level.level_start_program.as_ref())
            {
                self.extend_lifecycle_outcome(game, program, &mut outcome)?;
            }
        }
        Ok(outcome)
    }

    fn extend_lifecycle_outcome(
        &self,
        game: &LoadedGame,
        program: &[puzzle_core::RuleStep],
        outcome: &mut LifecycleOutcome,
    ) -> Result<(), TransitionError> {
        let next =
            transition_program_outcome(&game.game, program, &outcome.next_state, InputId(0))?;
        outcome.next_state = next.next_state;
        outcome.cancelled |= next.cancelled;
        outcome.commands.extend(next.commands);
        outcome.fired_rules.extend(next.fired_rules);
        Ok(())
    }

    fn materialized_level_initial_state(
        &self,
        game: &LoadedGame,
        level_index: usize,
    ) -> PuzzleState {
        let Some(level) = game.levels.get(level_index) else {
            return neutral_state(game);
        };
        let mut state = level.initial_state.clone();
        self.apply_persistent_vars(game, &mut state);
        match self.model_level_start_outcome(game, &state, level_index) {
            Ok(outcome) => {
                let mut next = outcome.next_state;
                self.apply_persistent_vars(game, &mut next);
                next
            }
            Err(_) => state,
        }
    }

    fn apply_screen_effect_during_turn(
        &mut self,
        game: &LoadedGame,
        effect: &SceneEffect,
        bindings: &HashMap<String, String>,
        pending_next_level: &mut Option<Option<String>>,
    ) -> Result<(), TransitionError> {
        match effect {
            SceneEffect::PuzzleNextLevel { target } => {
                pending_next_level.get_or_insert_with(|| Some(target.clone()));
                Ok(())
            }
            SceneEffect::Conditional { condition, effect } => {
                if self.is_screen_condition_true(game, condition) {
                    self.apply_screen_effect_during_turn(
                        game,
                        effect,
                        bindings,
                        pending_next_level,
                    )?;
                }
                Ok(())
            }
            SceneEffect::Sequence(effects) => {
                for effect in effects {
                    self.apply_screen_effect_during_turn(
                        game,
                        effect,
                        bindings,
                        pending_next_level,
                    )?;
                }
                Ok(())
            }
            _ => self.apply_screen_effect(game, effect, bindings),
        }
    }

    fn apply_level_clear_hook(
        &mut self,
        game: &LoadedGame,
        force_clear: bool,
    ) -> Result<Vec<QueuedRuleEffect>, TransitionError> {
        if !force_clear && !game.is_goal_complete(&self.state) {
            return Ok(Vec::new());
        }
        let mut effects = Vec::new();
        let model_clear_program = if self
            .active_level_index
            .is_some_and(|index| index + 1 >= game.levels.len())
        {
            game.last_level_clear_program
                .as_ref()
                .or(game.level_clear_program.as_ref())
        } else {
            game.level_clear_program.as_ref()
        };
        if let Some(program) = model_clear_program {
            let mut state = self.state.clone();
            self.apply_persistent_vars(game, &mut state);
            let outcome = transition_program_outcome(&game.game, program, &state, InputId(0))?;
            self.state = outcome.next_state;
            self.capture_persistent_vars(game, &self.state.clone());
            if !outcome.cancelled {
                effects.extend(queued_effects_for_outcome(
                    game,
                    None,
                    &outcome.commands,
                    &outcome.fired_rules,
                ));
            }
        }
        if let Some(program) = self.current_level(game).level_clear_program.as_ref() {
            let mut state = self.state.clone();
            self.apply_persistent_vars(game, &mut state);
            let outcome = transition_program_outcome(&game.game, program, &state, InputId(0))?;
            self.state = outcome.next_state;
            self.capture_persistent_vars(game, &self.state.clone());
            if !outcome.cancelled {
                effects.extend(queued_effects_for_outcome(
                    game,
                    None,
                    &outcome.commands,
                    &outcome.fired_rules,
                ));
            }
        }
        if let Some(display_program) = &game.display_level_clear_program {
            let mut state = self.state.clone();
            self.apply_persistent_vars(game, &mut state);
            self.state = transition_program(&game.game, display_program, &state, InputId(0))?;
            self.capture_persistent_vars(game, &self.state.clone());
        }
        self.sync_persistent_vars_to_scene_states(game);
        self.sync_current_level_puzzles(game);
        Ok(effects)
    }

    fn is_screen_condition_true(&self, game: &LoadedGame, condition: &str) -> bool {
        condition
            .split(" and ")
            .all(|part| self.is_screen_condition_atom_true(game, part.trim()))
    }

    fn is_screen_condition_atom_true(&self, game: &LoadedGame, condition: &str) -> bool {
        if let Some((left, right)) = condition.split_once(" == ") {
            return self
                .screen_condition_value(game, left.trim())
                .zip(self.screen_condition_value(game, right.trim()))
                .is_some_and(|(left, right)| left == right);
        }
        if let Some((left, right)) = condition.split_once(" != ") {
            return self
                .screen_condition_value(game, left.trim())
                .zip(self.screen_condition_value(game, right.trim()))
                .is_some_and(|(left, right)| left != right);
        }
        if let Some(value) = self.level_path_value(game, condition) {
            return value == "true";
        }
        let scoped = self.condition_state_and_name(game, condition);
        let Some((state, condition_name)) = scoped.or_else(|| {
            self.active_level_index
                .is_some()
                .then_some((&self.state, condition_name(condition)))
        }) else {
            return false;
        };
        game.is_condition_true(condition_name, state)
            || game.is_global_truthy(condition_name, state)
    }

    fn screen_condition_value(&self, game: &LoadedGame, value: &str) -> Option<String> {
        match value {
            "input" => self.current_input.clone(),
            "true" | "false" => Some(value.to_string()),
            _ => self
                .scene_path_value(game, value)
                .map(|value| scene_value_to_string(&value))
                .or_else(|| {
                    level_index_from_value(game, value)
                        .map(SceneValue::LevelRef)
                        .map(|value| scene_value_to_string(&value))
                })
                .or_else(|| self.scene_value_string(value))
                .or_else(|| self.level_path_value(game, value))
                .or_else(|| parse_runtime_quoted_text(value))
                .or_else(|| value.parse::<i64>().ok().map(|number| number.to_string()))
                .or_else(|| is_simple_identifier(value).then(|| value.to_string())),
        }
    }

    fn scene_path_value(&self, game: &LoadedGame, value: &str) -> Option<SceneValue> {
        let parts = value.split('.').collect::<Vec<_>>();
        match parts.as_slice() {
            [root, field] => {
                let receiver = if *root == "level" {
                    self.active_level_index.map(SceneValue::LevelRef)
                } else {
                    self.scene_value(root).cloned()
                }?;
                scene_value_field(game, self, &receiver, field)
            }
            _ => None,
        }
    }

    fn level_path_value(&self, game: &LoadedGame, value: &str) -> Option<String> {
        let parts = value.split('.').collect::<Vec<_>>();
        let [target, "level", property] = parts.as_slice() else {
            return None;
        };
        let scene = self.level_scene_from_target(game, target);
        let level_index = self
            .scene_state()
            .and_then(|state| state.puzzles.get(*target))
            .and_then(|puzzle| puzzle.level_index)?;
        self.level_property_value(game, &scene, level_index, property)
    }

    fn level_property_value(
        &self,
        game: &LoadedGame,
        scene: &str,
        level_index: usize,
        property: &str,
    ) -> Option<String> {
        let level = game.levels.get(level_index)?;
        match property {
            "name" | "label" => Some(level.name.clone()),
            "index" => Some(level_index.to_string()),
            "last" => Some((!level_has_next_in_scene(game, scene, level_index)).to_string()),
            "has_next" => Some(level_has_next_in_scene(game, scene, level_index).to_string()),
            _ => None,
        }
    }

    fn condition_state_and_name<'a>(
        &'a self,
        _game: &'a LoadedGame,
        condition: &'a str,
    ) -> Option<(&'a PuzzleState, &'a str)> {
        let parts = condition.split('.').collect::<Vec<_>>();
        match parts.as_slice() {
            [puzzle, name] => self
                .scene_state()
                .and_then(|state| state.puzzles.get(*puzzle))
                .map(|puzzle| (&puzzle.state, *name)),
            [screen, puzzle, name] => self
                .scene_states
                .get(*screen)
                .and_then(|state| state.puzzles.get(*puzzle))
                .map(|puzzle| (&puzzle.state, *name)),
            _ => None,
        }
    }

    fn apply_screen_effect(
        &mut self,
        game: &LoadedGame,
        effect: &SceneEffect,
        bindings: &HashMap<String, String>,
    ) -> Result<(), TransitionError> {
        match effect {
            SceneEffect::Input(input) => self.apply_input_name(game, input),
            SceneEffect::ComponentEffect(effect) => self.apply_component_effect(game, effect),
            SceneEffect::RoutineCall(name) => {
                let effect = game
                    .scenes
                    .iter()
                    .find(|scene| scene.name == self.focused_scene)
                    .and_then(|scene| scene.routines.iter().find(|routine| routine.name == *name))
                    .map(|routine| routine.effect.clone())
                    .expect("scene routine call was validated before runtime");
                self.apply_screen_effect(game, &effect, bindings)
            }
            SceneEffect::Message { text } => {
                if let Some(text) = self.eval_effect_string(game, text, bindings) {
                    self.message_events.push(MessageEvent::Message { text });
                    self.wait_events.push(WaitEvent::Wait {
                        milliseconds: game.default_wait_ms,
                    });
                }
                Ok(())
            }
            SceneEffect::Wait { .. } => Ok(()),
            SceneEffect::Conditional { condition, effect } => {
                if self.is_screen_condition_true(game, condition) {
                    self.apply_screen_effect(game, effect, bindings)?;
                }
                Ok(())
            }
            SceneEffect::PlaySfx { name } => {
                if self.should_emit_turn_sfx(name) {
                    self.sound_events
                        .push(SoundEvent::PlaySfx { name: name.clone() });
                }
                Ok(())
            }
            SceneEffect::PlayMusic { name } => {
                self.sound_events
                    .push(SoundEvent::PlayMusic { name: name.clone() });
                Ok(())
            }
            SceneEffect::PauseMusic { name } => {
                self.sound_events
                    .push(SoundEvent::PauseMusic { name: name.clone() });
                Ok(())
            }
            SceneEffect::ResumeMusic { name } => {
                self.sound_events
                    .push(SoundEvent::ResumeMusic { name: name.clone() });
                Ok(())
            }
            SceneEffect::StopMusic { name } => {
                self.sound_events
                    .push(SoundEvent::StopMusic { name: name.clone() });
                Ok(())
            }
            SceneEffect::Goto { scene, params } => {
                self.apply_screen_params(game, scene, params, bindings);
                self.goto_scene(game, scene);
                Ok(())
            }
            SceneEffect::Enter { scene, params } => {
                self.apply_screen_params(game, scene, params, bindings);
                self.enter_scene(game, scene);
                Ok(())
            }
            SceneEffect::Back => {
                self.back_or_initial(game);
                Ok(())
            }
            SceneEffect::Create { scene } => {
                self.create_scene(game, scene);
                Ok(())
            }
            SceneEffect::Reset { scene } => {
                if game.scenes.iter().any(|candidate| candidate.name == *scene) {
                    self.reset_scene_state(game, scene);
                } else if self.reset_persistent_var(game, scene) {
                } else {
                    self.reset_puzzle_state(game, scene);
                }
                Ok(())
            }
            SceneEffect::Delete { scene } => {
                self.delete_scene(game, scene);
                Ok(())
            }
            SceneEffect::Show { scene } => {
                self.show_scene(game, scene);
                Ok(())
            }
            SceneEffect::Hide { scene } => {
                self.hide_scene(game, scene);
                Ok(())
            }
            SceneEffect::Toggle { scene } => {
                self.toggle_scene(game, scene);
                Ok(())
            }
            SceneEffect::Focus { scene } => {
                self.focus_scene(game, scene);
                Ok(())
            }
            SceneEffect::PuzzleNextLevel { target } => {
                self.advance_level_from_target(game, target);
                Ok(())
            }
            SceneEffect::PuzzlePreviousLevel { target } => {
                self.previous_level_from_target(game, target);
                Ok(())
            }
            SceneEffect::GotoLevel { target, level } => {
                self.goto_level_target(game, target, level, bindings);
                Ok(())
            }
            SceneEffect::ResetPuzzle { target } => {
                self.reset_puzzle_state(game, target);
                Ok(())
            }
            SceneEffect::LoadPuzzle { target, source } => {
                self.load_puzzle_state(game, target, source, bindings);
                Ok(())
            }
            SceneEffect::Apply { rule, args, target } => {
                let input_name = args
                    .first()
                    .and_then(|arg| self.eval_effect_string(game, arg, bindings))
                    .unwrap_or_else(|| rule.clone());
                if let Some(input) = input_id_by_label(game, &input_name) {
                    if let Some(target) = target {
                        self.apply_model_input_to_target(game, target, input)?;
                    } else {
                        self.apply_input(game, input)?;
                    }
                }
                Ok(())
            }
            SceneEffect::Copy { source, target } => {
                self.copy_puzzle_state(game, source, target);
                Ok(())
            }
            SceneEffect::ClearUndoHistory => {
                self.clear_undo_history();
                Ok(())
            }
            SceneEffect::ClearGameProgress => {
                self.clear_game_progress(game);
                Ok(())
            }
            SceneEffect::SetCurrentLevel { level } => {
                self.set_current_level_progress(game, level, bindings);
                Ok(())
            }
            SceneEffect::ClearCurrentLevel => {
                self.clear_current_level_progress();
                Ok(())
            }
            SceneEffect::SetLevelCleared { level, cleared } => {
                self.set_level_cleared_progress(game, level.as_ref(), *cleared, bindings);
                Ok(())
            }
            SceneEffect::ResetPersistentVars => {
                self.reset_persistent_vars(game);
                Ok(())
            }
            SceneEffect::Sequence(effects) => {
                for effect in effects {
                    self.apply_screen_effect(game, effect, bindings)?;
                }
                Ok(())
            }
        }
    }

    fn apply_screen_params(
        &mut self,
        game: &LoadedGame,
        scene_name: &str,
        params: &[SceneEffectParam],
        bindings: &HashMap<String, String>,
    ) {
        let mut level_changed = false;
        if !params.is_empty() {
            self.create_scene(game, scene_name);
        }
        for param in params {
            match param {
                SceneEffectParam::Level(level) => {
                    if let Some(index) = self.eval_effect_level_index(game, level, bindings) {
                        if scene_accepts_level(game, scene_name, index) {
                            let _ = self.activate_level(game, index, true);
                            self.undo_stack.clear();
                            self.redo_stack.clear();
                            level_changed = true;
                        }
                    }
                }
                SceneEffectParam::Named { name, value } => {
                    let Some(value) = self.eval_effect_value(game, value, bindings) else {
                        continue;
                    };
                    if let Some(state) = self.scene_states.get_mut(scene_name) {
                        let accepts_param = game
                            .scenes
                            .iter()
                            .find(|scene| scene.name == scene_name)
                            .and_then(|scene| {
                                scene
                                    .state
                                    .variables
                                    .iter()
                                    .find(|variable| variable.name == *name)
                            })
                            .map(|variable| variable.mutable)
                            .unwrap_or(true);
                        if !accepts_param {
                            continue;
                        }
                        state.values.insert(name.clone(), value);
                    }
                }
            }
        }
        if level_changed {
            self.sync_current_level_puzzles_for_scene(game, scene_name);
        }
    }

    pub fn restart_level(&mut self, game: &LoadedGame) {
        if self.active_level_index.is_none() {
            return;
        }
        if let Some(checkpoint) = &self.level_checkpoint_state {
            let mut next = checkpoint.clone();
            self.apply_persistent_vars(game, &mut next);
            self.replace_state_if_changed(game, next);
            self.sync_current_level_puzzles(game);
            return;
        }
        let mut next = self.current_level(game).initial_state.clone();
        self.apply_persistent_vars(game, &mut next);
        self.replace_state_if_changed(game, next);
        let _ = self.apply_model_level_start(game, true);
        let _ = self.apply_level_start_transition(game);
        self.sync_current_level_puzzles(game);
    }

    pub fn advance_level(&mut self, game: &LoadedGame) {
        let scene = if scene_is_level_scene(game, &self.focused_scene) {
            self.focused_scene.clone()
        } else {
            initial_level_scene_name(game).to_string()
        };
        self.advance_level_in_scene(game, &scene);
    }

    fn advance_level_from_target(&mut self, game: &LoadedGame, target: &str) {
        let scene = self.level_scene_from_target(game, target);
        self.advance_level_in_scene(game, &scene);
    }

    fn previous_level_from_target(&mut self, game: &LoadedGame, target: &str) {
        let scene = self.level_scene_from_target(game, target);
        self.previous_level_in_scene(game, &scene);
    }

    fn level_scene_from_target(&self, game: &LoadedGame, target: &str) -> String {
        if scene_is_level_scene(game, target) {
            return target.to_string();
        }
        self.resolve_puzzle_target(game, target)
            .map(|(scene, _)| scene)
            .filter(|scene| scene_is_level_scene(game, scene))
            .unwrap_or_else(|| self.focused_scene.clone())
    }

    fn advance_level_in_scene(&mut self, game: &LoadedGame, scene: &str) {
        let Some(level_index) = self.active_level_index else {
            return;
        };
        let indices = scene_level_indices(game, scene);
        let Some(position) = indices.iter().position(|index| *index == level_index) else {
            return;
        };
        let Some(next_level) = indices.get(position + 1).copied() else {
            return;
        };

        let _ = self.activate_level(game, next_level, true);
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.start_scene(game, scene);
        self.sync_current_level_puzzles(game);
        self.selected_level_index = next_level;
    }

    fn previous_level_in_scene(&mut self, game: &LoadedGame, scene: &str) {
        let Some(level_index) = self.active_level_index else {
            return;
        };
        let indices = scene_level_indices(game, scene);
        let Some(position) = indices.iter().position(|index| *index == level_index) else {
            return;
        };
        let Some(previous_level) = position
            .checked_sub(1)
            .and_then(|index| indices.get(index))
            .copied()
        else {
            return;
        };

        let _ = self.activate_level(game, previous_level, true);
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.start_scene(game, scene);
        self.sync_current_level_puzzles(game);
        self.selected_level_index = previous_level;
    }

    pub fn start_level(&mut self, game: &LoadedGame, level_index: usize) {
        if level_index >= game.levels.len() {
            return;
        }

        let _ = self.activate_level(game, level_index, true);
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.start_scene(game, initial_level_scene_name(game));
        self.sync_current_level_puzzles(game);
    }

    fn goto_level_target(
        &mut self,
        game: &LoadedGame,
        target: &str,
        level: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) {
        let Some(value) = self.eval_effect_string(game, level, bindings) else {
            return;
        };
        let Some(index) = level_index_from_value(game, &value) else {
            return;
        };
        if game.scenes.iter().any(|scene| scene.name == target) {
            if !scene_accepts_level(game, target, index) {
                return;
            }
            let _ = self.activate_level(game, index, true);
            self.undo_stack.clear();
            self.redo_stack.clear();
            self.goto_scene(game, target);
            self.sync_current_level_puzzles(game);
            return;
        }
        self.load_puzzle_state(game, target, &value, bindings);
    }

    pub fn undo(&mut self, game: &LoadedGame) {
        if self.active_level_index.is_none() {
            return;
        }
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(self.state.clone());
            self.state = previous;
            self.sync_persistent_vars_to_scene_states(game);
            self.sync_current_level_puzzles(game);
        }
    }

    pub fn redo(&mut self, game: &LoadedGame) {
        if self.active_level_index.is_none() {
            return;
        }
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.state.clone());
            self.state = next;
            self.sync_persistent_vars_to_scene_states(game);
            self.sync_current_level_puzzles(game);
        }
    }

    fn replace_state_if_changed(&mut self, game: &LoadedGame, mut next: PuzzleState) {
        if self.active_level_index.is_none() {
            self.state = next;
            return;
        }
        self.capture_persistent_vars(game, &next);
        self.apply_persistent_vars(game, &mut next);
        if states_equal_ignoring_persistent_vars(game, &next, &self.state) {
            self.state = next;
            self.sync_persistent_vars_to_scene_states(game);
            return;
        }

        self.undo_stack.push(self.state.clone());
        self.state = next;
        self.redo_stack.clear();
        self.sync_persistent_vars_to_scene_states(game);
    }

    fn goto_scene(&mut self, game: &LoadedGame, name: &str) {
        self.create_scene(game, name);
        self.visible_scenes.clear();
        self.show_scene(game, name);
        self.focus_history.clear();
        self.focus_scene(game, name);
    }

    fn enter_scene(&mut self, game: &LoadedGame, name: &str) {
        self.create_scene(game, name);
        if self.focused_scene != name {
            self.focus_history.push(self.focused_scene.clone());
        }
        self.show_scene(game, name);
        self.focus_scene(game, name);
    }

    fn start_scene(&mut self, game: &LoadedGame, name: &str) {
        self.reset_scene_state(game, name);
        self.visible_scenes.clear();
        self.show_scene(game, name);
        self.focus_history.clear();
        self.focus_scene(game, name);
    }

    fn back_or_initial(&mut self, game: &LoadedGame) {
        let current = self.focused_scene.clone();
        let previous = self
            .focus_history
            .pop()
            .unwrap_or_else(|| initial_scene_name(game).to_string());
        self.hide_scene_only(&current);
        self.focus_scene(game, &previous);
    }

    fn create_scene(&mut self, game: &LoadedGame, name: &str) {
        if !self.scene_states.contains_key(name) {
            let _ = self.ensure_active_level_for_scene(game, name, true);
            self.reset_scene_state(game, name);
        }
    }

    fn ensure_active_level_for_scene(
        &mut self,
        game: &LoadedGame,
        name: &str,
        emit_events: bool,
    ) -> Result<(), TransitionError> {
        let Some(scene) = game.scenes.iter().find(|scene| scene.name == name) else {
            return Ok(());
        };
        if scene.puzzle_rule.is_none()
            && !scene
                .state
                .puzzles
                .iter()
                .any(|puzzle| puzzle.initializer == ScenePuzzleInitializer::CurrentLevel)
        {
            return Ok(());
        }
        if self
            .active_level_index
            .is_some_and(|level_index| scene_accepts_level(game, name, level_index))
        {
            return Ok(());
        }
        let selected_level_index = self.selected_level_index;
        let level_index = (selected_level_index < game.levels.len()
            && scene_accepts_level(game, name, selected_level_index))
        .then_some(selected_level_index)
        .or_else(|| first_level_index_for_scene(game, name, None));
        if let Some(level_index) = level_index {
            self.activate_level(game, level_index, emit_events)?;
        }
        Ok(())
    }

    fn delete_scene(&mut self, game: &LoadedGame, name: &str) {
        self.scene_states.remove(name);
        self.visible_scenes.retain(|screen| screen != name);
        self.focus_history.retain(|screen| screen != name);
        if self.focused_scene == name {
            let previous = self
                .focus_history
                .pop()
                .unwrap_or_else(|| initial_scene_name(game).to_string());
            self.create_scene(game, &previous);
            self.show_scene(game, &previous);
            self.focused_scene = previous;
        }
    }

    fn show_scene(&mut self, game: &LoadedGame, name: &str) {
        self.create_scene(game, name);
        if !self.visible_scenes.iter().any(|screen| screen == name) {
            self.visible_scenes.push(name.to_string());
        }
    }

    fn hide_scene(&mut self, game: &LoadedGame, name: &str) {
        self.hide_scene_only(name);
        if self.focused_scene == name {
            let previous = self
                .visible_scenes
                .last()
                .cloned()
                .or_else(|| self.focus_history.pop())
                .unwrap_or_else(|| initial_scene_name(game).to_string());
            self.create_scene(game, &previous);
            self.show_scene(game, &previous);
            self.focused_scene = previous;
        }
    }

    fn hide_scene_only(&mut self, name: &str) {
        self.visible_scenes.retain(|screen| screen != name);
    }

    fn toggle_scene(&mut self, game: &LoadedGame, name: &str) {
        if self.visible_scenes.iter().any(|screen| screen == name) {
            self.hide_scene(game, name);
        } else {
            self.show_scene(game, name);
        }
    }

    fn focus_scene(&mut self, game: &LoadedGame, name: &str) {
        self.create_scene(game, name);
        self.show_scene(game, name);
        self.focused_scene = name.to_string();
        let _ = self.apply_scene_start_transition(game);
        let _ = self.apply_level_start_transition(game);
    }

    fn apply_scene_start_transition(&mut self, game: &LoadedGame) -> Result<(), TransitionError> {
        self.apply_lifecycle_transition(game, SceneTransitionTrigger::SceneStart)
    }

    fn apply_level_start_transition(&mut self, game: &LoadedGame) -> Result<(), TransitionError> {
        self.apply_lifecycle_transition(game, SceneTransitionTrigger::LevelStart)
    }

    fn apply_lifecycle_transition(
        &mut self,
        game: &LoadedGame,
        trigger: SceneTransitionTrigger,
    ) -> Result<(), TransitionError> {
        let Some(effect) = game
            .scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
            .and_then(|screen| {
                screen.transitions.iter().find_map(|transition| {
                    (transition.trigger == trigger).then(|| transition.effect.clone())
                })
            })
        else {
            return Ok(());
        };

        self.apply_screen_effect(game, &effect, &HashMap::new())
    }

    fn eval_effect_string(
        &self,
        game: &LoadedGame,
        expr: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) -> Option<String> {
        self.eval_effect_value(game, expr, bindings)
            .map(|value| scene_value_to_string(&value))
    }

    fn eval_effect_level_index(
        &self,
        game: &LoadedGame,
        expr: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) -> Option<usize> {
        self.eval_effect_value(game, expr, bindings)
            .and_then(|value| level_index_from_scene_value(game, &value))
    }

    fn eval_effect_value(
        &self,
        game: &LoadedGame,
        expr: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) -> Option<SceneValue> {
        match expr {
            SceneExpr::Bool(value) => Some(SceneValue::Bool(*value)),
            SceneExpr::Int(value) => Some(SceneValue::Int(*value)),
            SceneExpr::Text(value) => Some(SceneValue::Text(value.clone())),
            SceneExpr::Path(path) if path.len() == 1 => {
                if let Some(value) = bindings.get(&path[0]) {
                    return Some(scene_value_from_effect_atom(game, value));
                }
                if path[0] == "level" {
                    if let Some(index) = self.active_level_index {
                        return Some(SceneValue::LevelRef(index));
                    }
                }
                self.scene_value(&path[0])
                    .cloned()
                    .or_else(|| Some(scene_value_from_effect_atom(game, &path[0])))
            }
            SceneExpr::Path(path) if path.len() == 2 => {
                let receiver = self.eval_effect_value(
                    game,
                    &SceneExpr::Path(vec![path[0].clone()]),
                    bindings,
                )?;
                scene_value_field(game, self, &receiver, &path[1])
            }
            SceneExpr::Path(path) if path.len() == 3 && path[1] == "level" => self
                .level_path_value(game, &path.join("."))
                .map(SceneValue::Symbol),
            SceneExpr::Path(path) => Some(SceneValue::Symbol(path.join("."))),
            SceneExpr::Call { name, args } if name == "next" && args.len() == 1 => {
                let index = self.eval_effect_level_index(game, &args[0], bindings)?;
                Some(SceneValue::LevelRef(
                    index.saturating_add(1).min(game.levels.len() - 1),
                ))
            }
            SceneExpr::Call { .. } => None,
        }
    }

    fn resolve_message_text(&self, text: &str, literal: bool) -> String {
        if literal {
            return text.to_string();
        }
        self.scene_value_string(text)
            .unwrap_or_else(|| text.to_string())
    }

    fn scene_value_string(&self, name: &str) -> Option<String> {
        self.scene_value(name).map(scene_value_to_string)
    }

    fn scene_value(&self, name: &str) -> Option<&SceneValue> {
        self.scene_state()
            .and_then(|state| state.values.get(name))
            .or_else(|| self.session_values.get(name))
    }

    fn reset_scene_state(&mut self, game: &LoadedGame, name: &str) {
        let previous_values = self
            .scene_states
            .get(name)
            .map(|state| state.values.clone())
            .unwrap_or_default();
        let mut next = self.default_scene_state(game, name);
        if let Some(screen) = game.scenes.iter().find(|screen| screen.name == name) {
            for variable in &screen.state.variables {
                if variable.lifetime == puzzle_lang::SceneStateLifetime::Persistent {
                    if let Some(value) = previous_values.get(&variable.name) {
                        next.values.insert(variable.name.clone(), value.clone());
                    }
                }
            }
        }
        self.scene_states.insert(name.to_string(), next);
    }

    fn default_scene_state(&self, game: &LoadedGame, name: &str) -> SceneRuntimeState {
        let Some(screen) = game.scenes.iter().find(|screen| screen.name == name) else {
            return SceneRuntimeState::default();
        };

        let values = screen
            .state
            .variables
            .iter()
            .map(|variable| (variable.name.clone(), variable.default.clone()))
            .collect::<HashMap<_, _>>();
        let puzzles = screen
            .state
            .puzzles
            .iter()
            .map(|puzzle| {
                let (mut state, level_index) = match &puzzle.initializer {
                    ScenePuzzleInitializer::CurrentLevel => self
                        .active_level_index
                        .map(|level_index| (self.state.clone(), Some(level_index)))
                        .unwrap_or_else(|| {
                            first_level_index_for_scene(game, name, None)
                                .map(|level_index| {
                                    (
                                        self.materialized_level_initial_state(game, level_index),
                                        Some(level_index),
                                    )
                                })
                                .unwrap_or_else(|| (neutral_state(game), None))
                        }),
                    ScenePuzzleInitializer::Level(level_name) => game
                        .levels
                        .iter()
                        .enumerate()
                        .find(|(_, level)| level.name == *level_name)
                        .map(|(index, _)| {
                            (
                                self.materialized_level_initial_state(game, index),
                                Some(index),
                            )
                        })
                        .unwrap_or_else(|| (neutral_state(game), None)),
                };
                self.apply_persistent_vars(game, &mut state);
                (
                    puzzle.name.clone(),
                    ScenePuzzleRuntimeState {
                        initial_state: state.clone(),
                        checkpoint_state: None,
                        state,
                        level_index,
                    },
                )
            })
            .collect();

        SceneRuntimeState { values, puzzles }
    }

    fn sync_current_level_puzzles(&mut self, game: &LoadedGame) {
        let focused_scene = self.focused_scene.clone();
        self.sync_current_level_puzzles_for_scene(game, &focused_scene);
    }

    fn sync_current_level_puzzles_for_scene(&mut self, game: &LoadedGame, scene_name: &str) {
        let Some(level_index) = self.active_level_index else {
            return;
        };
        let current_initial_state = self.materialized_level_initial_state(game, level_index);
        for screen in game
            .scenes
            .iter()
            .filter(|screen| screen.name == scene_name)
        {
            let Some(state) = self.scene_states.get_mut(&screen.name) else {
                continue;
            };
            for puzzle in &screen.state.puzzles {
                if puzzle.initializer == ScenePuzzleInitializer::CurrentLevel {
                    state.puzzles.insert(
                        puzzle.name.clone(),
                        ScenePuzzleRuntimeState {
                            state: self.state.clone(),
                            initial_state: current_initial_state.clone(),
                            checkpoint_state: self.level_checkpoint_state.clone(),
                            level_index: Some(level_index),
                        },
                    );
                }
            }
        }
    }

    fn resolve_puzzle_target(&self, game: &LoadedGame, target: &str) -> Option<(String, String)> {
        let parts = target.split('.').collect::<Vec<_>>();
        match parts.as_slice() {
            [puzzle] => {
                if self
                    .scene_states
                    .get(&self.focused_scene)
                    .is_some_and(|scene| scene.puzzles.contains_key(*puzzle))
                {
                    return Some((self.focused_scene.clone(), (*puzzle).to_string()));
                }
                if let Some(primary) = scene_primary_puzzle_name(game, puzzle) {
                    return Some(((*puzzle).to_string(), primary));
                }
                Some((self.focused_scene.clone(), (*puzzle).to_string()))
            }
            [screen, puzzle] => Some(((*screen).to_string(), (*puzzle).to_string())),
            _ => None,
        }
    }

    fn copy_puzzle_state(&mut self, game: &LoadedGame, source: &str, target: &str) {
        let Some((source_scene, source_puzzle)) = self.resolve_puzzle_target(game, source) else {
            return;
        };
        let Some((target_scene, target_puzzle)) = self.resolve_puzzle_target(game, target) else {
            return;
        };
        self.create_scene(game, &source_scene);
        self.create_scene(game, &target_scene);
        let Some(mut state) = self
            .scene_states
            .get(&source_scene)
            .and_then(|screen| screen.puzzles.get(&source_puzzle))
            .map(|puzzle| puzzle.state.clone())
        else {
            return;
        };
        self.apply_persistent_vars(game, &mut state);
        if let Some(screen) = self.scene_states.get_mut(&target_scene) {
            if let Some(puzzle) = screen.puzzles.get_mut(&target_puzzle) {
                puzzle.state = state;
            }
        }
    }

    fn reset_puzzle_state(&mut self, game: &LoadedGame, target: &str) {
        let Some((scene_name, puzzle_name)) = self.resolve_puzzle_target(game, target) else {
            return;
        };
        self.create_scene(game, &scene_name);
        let persistent_vars = self.persistent_vars.clone();
        let Some((next_state, level_index)) =
            self.scene_states
                .get_mut(&scene_name)
                .and_then(|scene_state| {
                    let puzzle = scene_state.puzzles.get_mut(&puzzle_name)?;
                    puzzle.state = puzzle
                        .checkpoint_state
                        .clone()
                        .unwrap_or_else(|| puzzle.initial_state.clone());
                    apply_persistent_var_values(game, &persistent_vars, &mut puzzle.state);
                    Some((puzzle.state.clone(), puzzle.level_index))
                })
        else {
            return;
        };
        if scene_puzzle_initializer(game, &scene_name, &puzzle_name)
            == Some(ScenePuzzleInitializer::CurrentLevel)
        {
            self.replace_state_if_changed(game, next_state);
            if let Some(level_index) = level_index {
                self.level_index = level_index;
                self.active_level_index = Some(level_index);
                self.selected_level_index = level_index;
            }
            self.sync_current_level_puzzles_for_scene(game, &scene_name);
        }
    }

    fn save_checkpoint(&mut self, game: &LoadedGame, target: Option<&str>) {
        let Some(target) = target else {
            if self.active_level_index.is_some() {
                self.level_checkpoint_state = Some(self.state.clone());
                self.sync_current_level_puzzles(game);
            }
            return;
        };
        let Some((scene_name, puzzle_name)) = self.resolve_puzzle_target(game, target) else {
            if self.active_level_index.is_some() {
                self.level_checkpoint_state = Some(self.state.clone());
                self.sync_current_level_puzzles(game);
            }
            return;
        };
        self.create_scene(game, &scene_name);
        let checkpoint_state = self
            .scene_states
            .get(&scene_name)
            .and_then(|scene_state| scene_state.puzzles.get(&puzzle_name))
            .map(|puzzle| puzzle.state.clone());
        let Some(checkpoint_state) = checkpoint_state else {
            return;
        };
        if let Some(puzzle) = self
            .scene_states
            .get_mut(&scene_name)
            .and_then(|scene_state| scene_state.puzzles.get_mut(&puzzle_name))
        {
            puzzle.checkpoint_state = Some(checkpoint_state.clone());
        }
        if scene_puzzle_initializer(game, &scene_name, &puzzle_name)
            == Some(ScenePuzzleInitializer::CurrentLevel)
        {
            self.level_checkpoint_state = Some(checkpoint_state);
            self.sync_current_level_puzzles_for_scene(game, &scene_name);
        }
    }

    fn clear_checkpoint(&mut self, game: &LoadedGame, target: Option<&str>) {
        let Some(target) = target else {
            self.level_checkpoint_state = None;
            self.sync_current_level_puzzles(game);
            return;
        };
        let Some((scene_name, puzzle_name)) = self.resolve_puzzle_target(game, target) else {
            self.level_checkpoint_state = None;
            self.sync_current_level_puzzles(game);
            return;
        };
        self.create_scene(game, &scene_name);
        if let Some(puzzle) = self
            .scene_states
            .get_mut(&scene_name)
            .and_then(|scene_state| scene_state.puzzles.get_mut(&puzzle_name))
        {
            puzzle.checkpoint_state = None;
        }
        if scene_puzzle_initializer(game, &scene_name, &puzzle_name)
            == Some(ScenePuzzleInitializer::CurrentLevel)
        {
            self.level_checkpoint_state = None;
            self.sync_current_level_puzzles_for_scene(game, &scene_name);
        }
    }

    fn load_puzzle_state(
        &mut self,
        game: &LoadedGame,
        target: &str,
        source: &str,
        bindings: &HashMap<String, String>,
    ) {
        let Some((scene_name, puzzle_name)) = self.resolve_puzzle_target(game, target) else {
            return;
        };
        let Some(level_index) = self.eval_puzzle_level_ref(game, target, source, bindings) else {
            return;
        };
        self.create_scene(game, &scene_name);
        let state = self.materialized_level_initial_state(game, level_index);
        if let Some(scene_state) = self.scene_states.get_mut(&scene_name) {
            scene_state.puzzles.insert(
                puzzle_name.clone(),
                ScenePuzzleRuntimeState {
                    state: state.clone(),
                    initial_state: state.clone(),
                    checkpoint_state: None,
                    level_index: Some(level_index),
                },
            );
        }
        if scene_puzzle_initializer(game, &scene_name, &puzzle_name)
            == Some(ScenePuzzleInitializer::CurrentLevel)
        {
            self.level_checkpoint_state = None;
            self.level_index = level_index;
            self.active_level_index = Some(level_index);
            self.selected_level_index = level_index;
            self.state = state;
            self.undo_stack.clear();
            self.redo_stack.clear();
            self.sync_current_level_puzzles_for_scene(game, &scene_name);
        }
    }

    fn eval_puzzle_level_ref(
        &self,
        game: &LoadedGame,
        target: &str,
        source: &str,
        bindings: &HashMap<String, String>,
    ) -> Option<usize> {
        let source = source.trim();
        if let Some(inner) = source
            .strip_prefix("next(")
            .and_then(|inner| inner.strip_suffix(')'))
        {
            return self
                .eval_puzzle_level_ref(game, target, inner, bindings)
                .map(|index| index.saturating_add(1).min(game.levels.len() - 1));
        }
        let (target_scene, target_puzzle) = self.resolve_puzzle_target(game, target)?;
        let current_puzzle = self
            .scene_states
            .get(&target_scene)
            .and_then(|state| state.puzzles.get(&target_puzzle));
        if source == format!("{target_puzzle}.level")
            || source == format!("{target_scene}.{target_puzzle}.level")
        {
            return current_puzzle.and_then(|puzzle| puzzle.level_index);
        }
        let level_prefix = format!("{target_puzzle}.levels[");
        let qualified_level_prefix = format!("{target_scene}.{target_puzzle}.levels[");
        if let Some(index) = source
            .strip_prefix(&level_prefix)
            .or_else(|| source.strip_prefix(&qualified_level_prefix))
            .and_then(|inner| inner.strip_suffix(']'))
            .and_then(|index| {
                index.parse::<usize>().ok().or_else(|| {
                    bindings
                        .get(index)
                        .and_then(|value| value.parse::<usize>().ok())
                })
            })
        {
            return (index < game.levels.len()).then_some(index);
        }
        bindings
            .get(source)
            .and_then(|value| level_index_from_value(game, value))
            .or_else(|| level_index_from_value(game, source))
    }

    fn current_scene_has_level_menu(&self, game: &LoadedGame) -> bool {
        game.scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
            .is_some_and(|screen| screen.components.iter().any(component_has_level_menu))
    }

    fn current_scene_accepts_model_input(&self, game: &LoadedGame) -> bool {
        game.scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
            .is_some_and(|screen| screen.puzzle_rule.is_some())
    }

    fn apply_level_menu_command(&mut self, game: &LoadedGame, command: &str) -> bool {
        let (command, cursor_override) = split_command_cursor_override(command);
        let Some(menu) = self.current_level_menu(game) else {
            return false;
        };
        let level_indices = scene_level_indices(game, &self.focused_scene);
        let item_count = level_indices.len() + menu.buttons.len();
        if item_count == 0 {
            self.selected_level_index = 0;
            return false;
        }
        if let Some(cursor) = cursor_override {
            self.set_level_menu_cursor_position(game, &level_indices, cursor.min(item_count - 1));
        }
        match command {
            "up" => {
                self.move_level_menu_cursor(
                    game,
                    menu,
                    &level_indices,
                    -(level_menu_columns(menu) as isize),
                );
                true
            }
            "down" => {
                self.move_level_menu_cursor(
                    game,
                    menu,
                    &level_indices,
                    level_menu_columns(menu) as isize,
                );
                true
            }
            "left" => {
                self.move_level_menu_cursor(game, menu, &level_indices, -1);
                true
            }
            "right" => {
                self.move_level_menu_cursor(game, menu, &level_indices, 1);
                true
            }
            "select" => {
                let cursor = self.level_menu_cursor_position(game, &level_indices);
                if let Some(level_index) = level_indices.get(cursor).copied() {
                    if let Some(action) = &menu.action {
                        let bindings = level_menu_action_bindings(game, level_index);
                        let _ = self.apply_screen_effect(game, action, &bindings);
                    } else {
                        self.start_level(game, level_index);
                    }
                } else if let Some(command_button) =
                    menu.buttons.get(cursor.saturating_sub(level_indices.len()))
                {
                    let _ = self.apply_screen_effect(game, &command_button.effect, &HashMap::new());
                }
                true
            }
            _ => false,
        }
    }

    fn move_level_menu_cursor(
        &mut self,
        game: &LoadedGame,
        menu: &LevelMenuDef,
        level_indices: &[usize],
        delta: isize,
    ) {
        let item_count = level_indices.len() + menu.buttons.len();
        if item_count == 0 || delta == 0 {
            return;
        }
        let current = self.level_menu_cursor_position(game, level_indices);
        if menu.wrap {
            let count = item_count as isize;
            let next = (current as isize + delta).rem_euclid(count);
            self.set_level_menu_cursor_position(game, level_indices, next as usize);
            return;
        }
        let next = current as isize + delta;
        self.set_level_menu_cursor_position(
            game,
            level_indices,
            next.clamp(0, item_count as isize - 1) as usize,
        );
    }

    fn level_menu_cursor_position(&self, game: &LoadedGame, level_indices: &[usize]) -> usize {
        level_indices
            .iter()
            .position(|index| *index == self.selected_level_index)
            .or_else(|| {
                (self.selected_level_index >= game.levels.len())
                    .then(|| level_indices.len() + self.selected_level_index - game.levels.len())
            })
            .unwrap_or(0)
    }

    fn set_level_menu_cursor_position(
        &mut self,
        game: &LoadedGame,
        level_indices: &[usize],
        position: usize,
    ) {
        self.selected_level_index = level_indices
            .get(position)
            .copied()
            .unwrap_or_else(|| game.levels.len() + position.saturating_sub(level_indices.len()));
    }

    fn current_level_menu<'a>(&self, game: &'a LoadedGame) -> Option<&'a LevelMenuDef> {
        game.scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
            .and_then(|screen| find_level_menu(&screen.components))
    }
}

fn initial_scene_name(game: &LoadedGame) -> &str {
    game.scenes
        .first()
        .map(|screen| screen.name.as_str())
        .unwrap_or("playing")
}

fn neutral_state(game: &LoadedGame) -> PuzzleState {
    PuzzleState::empty(1, 1, game.game.layer_count, game.game.object_count())
        .unwrap_or_else(|_| game.levels[0].initial_state.clone())
}

fn initial_level_scene_name(game: &LoadedGame) -> &str {
    game.scenes
        .iter()
        .find(|screen| scene_is_level_scene(game, &screen.name))
        .map(|screen| screen.name.as_str())
        .unwrap_or_else(|| initial_scene_name(game))
}

fn scene_is_level_scene(game: &LoadedGame, name: &str) -> bool {
    game.scenes
        .iter()
        .find(|screen| screen.name == name)
        .is_some_and(|screen| {
            screen.puzzle_rule.is_some()
                || screen
                    .state
                    .puzzles
                    .iter()
                    .any(|puzzle| puzzle.initializer == ScenePuzzleInitializer::CurrentLevel)
        })
}

fn game_has_scene_level_owner(game: &LoadedGame) -> bool {
    game.scenes.iter().any(|scene| {
        scene.puzzle_rule.is_some()
            || scene
                .state
                .puzzles
                .iter()
                .any(|puzzle| puzzle.initializer == ScenePuzzleInitializer::CurrentLevel)
    })
}

fn persistent_var_values(game: &LoadedGame, state: &PuzzleState) -> Vec<i64> {
    game.persistent_vars
        .iter()
        .map(|var| state.global_value(*var).unwrap_or(0))
        .collect()
}

fn persistent_var_default_values(game: &LoadedGame) -> Vec<i64> {
    game.persistent_vars
        .iter()
        .map(|var| {
            game.levels
                .first()
                .and_then(|level| level.initial_state.global_value(*var))
                .unwrap_or(0)
        })
        .collect()
}

fn persistent_var_index_by_name(game: &LoadedGame, name: &str) -> Option<usize> {
    game.persistent_vars.iter().position(|var| {
        game.global_labels
            .get(var)
            .is_some_and(|label| label == name)
    })
}

fn apply_persistent_var_values(game: &LoadedGame, values: &[i64], state: &mut PuzzleState) {
    for (index, var) in game.persistent_vars.iter().enumerate() {
        if let Some(value) = values.get(index) {
            let _ = state.set_visible_global(*var, *value);
        }
    }
}

fn states_equal_ignoring_persistent_vars(
    game: &LoadedGame,
    left: &PuzzleState,
    right: &PuzzleState,
) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    for var in &game.persistent_vars {
        let _ = left.set_visible_global(*var, 0);
        let _ = right.set_visible_global(*var, 0);
    }
    left == right
}

fn scene_puzzle_initializer(
    game: &LoadedGame,
    scene_name: &str,
    puzzle_name: &str,
) -> Option<ScenePuzzleInitializer> {
    game.scenes
        .iter()
        .find(|screen| screen.name == scene_name)
        .and_then(|screen| {
            screen
                .state
                .puzzles
                .iter()
                .find(|puzzle| puzzle.name == puzzle_name)
        })
        .map(|puzzle| puzzle.initializer.clone())
}

fn scene_primary_puzzle_name(game: &LoadedGame, scene_name: &str) -> Option<String> {
    let scene = game.scenes.iter().find(|scene| scene.name == scene_name)?;
    scene
        .state
        .puzzles
        .iter()
        .find(|puzzle| puzzle.initializer == ScenePuzzleInitializer::CurrentLevel)
        .or_else(|| scene.state.puzzles.first())
        .map(|puzzle| puzzle.name.clone())
}

fn scene_value_to_string(value: &SceneValue) -> String {
    match value {
        SceneValue::Bool(value) => value.to_string(),
        SceneValue::Int(value) => value.to_string(),
        SceneValue::Text(value) | SceneValue::Symbol(value) => value.clone(),
        SceneValue::LevelRef(index) => index.to_string(),
    }
}

fn scene_value_from_effect_atom(game: &LoadedGame, value: &str) -> SceneValue {
    level_index_from_value(game, value)
        .map(SceneValue::LevelRef)
        .unwrap_or_else(|| SceneValue::Symbol(value.to_string()))
}

fn level_index_from_scene_value(game: &LoadedGame, value: &SceneValue) -> Option<usize> {
    match value {
        SceneValue::LevelRef(index) => (*index < game.levels.len()).then_some(*index),
        SceneValue::Int(index) => usize::try_from(*index)
            .ok()
            .filter(|index| *index < game.levels.len()),
        SceneValue::Text(value) | SceneValue::Symbol(value) => level_index_from_value(game, value),
        SceneValue::Bool(_) => None,
    }
}

fn scene_value_field(
    game: &LoadedGame,
    session: &GameSession,
    value: &SceneValue,
    field: &str,
) -> Option<SceneValue> {
    match value {
        SceneValue::LevelRef(index) => level_ref_field(game, session, *index, field),
        _ => None,
    }
}

fn level_ref_field(
    game: &LoadedGame,
    session: &GameSession,
    index: usize,
    field: &str,
) -> Option<SceneValue> {
    let level = game.levels.get(index)?;
    match field {
        "index" => Some(SceneValue::Int(i64::try_from(index).ok()?)),
        "num" | "number" => Some(SceneValue::Int(i64::try_from(index + 1).ok()?)),
        "name" | "label" | "title" => Some(SceneValue::Text(level.name.clone())),
        "cleared" | "solved" => Some(SceneValue::Bool(
            session
                .cleared_levels()
                .get(index)
                .copied()
                .unwrap_or(false),
        )),
        _ => None,
    }
}

fn parse_runtime_command(command_text: &str, default_wait_ms: u64) -> Option<SceneEffect> {
    let command_text = command_text.trim();
    if command_text == "clear_undo_history" || command_text == "clear_history" {
        return Some(SceneEffect::ClearUndoHistory);
    }
    if command_text == "clear_game_progress" {
        return Some(SceneEffect::ClearGameProgress);
    }
    if command_text == "clear current_level" {
        return Some(SceneEffect::ClearCurrentLevel);
    }
    if command_text == "reset persistent_vars" {
        return Some(SceneEffect::ResetPersistentVars);
    }
    if let Some(rest) = command_text.strip_prefix("set current_level = ") {
        return Some(SceneEffect::SetCurrentLevel {
            level: parse_runtime_level_expr(rest.trim())?,
        });
    }
    if let Some(rest) = command_text.strip_prefix("set level.cleared = ") {
        return Some(SceneEffect::SetLevelCleared {
            level: None,
            cleared: parse_runtime_bool(rest.trim())?,
        });
    }
    if let Some(rest) = command_text.strip_prefix("set level(") {
        let (level, cleared) = rest.split_once(").cleared = ")?;
        return Some(SceneEffect::SetLevelCleared {
            level: Some(parse_runtime_level_expr(level.trim())?),
            cleared: parse_runtime_bool(cleared.trim())?,
        });
    }
    if let Some(text) = command_text.strip_prefix("message ") {
        return Some(SceneEffect::Message {
            text: parse_runtime_expr(text.trim())
                .unwrap_or_else(|| SceneExpr::Text(text.trim().to_string())),
        });
    }
    if command_text == "wait" {
        return Some(SceneEffect::Wait {
            milliseconds: Some(default_wait_ms),
        });
    }
    if let Some(duration) = command_text.strip_prefix("wait ") {
        return parse_runtime_wait_duration_ms(duration.trim()).map(|milliseconds| {
            SceneEffect::Wait {
                milliseconds: Some(milliseconds),
            }
        });
    }
    if let Some(name) = command_text.strip_prefix("sfx ") {
        let name = name.trim();
        if is_simple_identifier(name) {
            return Some(SceneEffect::PlaySfx {
                name: name.to_string(),
            });
        }
        return None;
    }
    if let Some(name) = command_text.strip_prefix("play_music ") {
        let name = name.trim();
        if is_simple_identifier(name) {
            return Some(SceneEffect::PlayMusic {
                name: name.to_string(),
            });
        }
        return None;
    }
    if command_text == "pause_music" {
        return Some(SceneEffect::PauseMusic { name: None });
    }
    if let Some(name) = command_text.strip_prefix("pause_music ") {
        let name = name.trim();
        if is_simple_identifier(name) {
            return Some(SceneEffect::PauseMusic {
                name: Some(name.to_string()),
            });
        }
        return None;
    }
    if command_text == "resume_music" {
        return Some(SceneEffect::ResumeMusic { name: None });
    }
    if let Some(name) = command_text.strip_prefix("resume_music ") {
        let name = name.trim();
        if is_simple_identifier(name) {
            return Some(SceneEffect::ResumeMusic {
                name: Some(name.to_string()),
            });
        }
        return None;
    }
    if command_text == "stop_music" {
        return Some(SceneEffect::StopMusic { name: None });
    }
    if let Some(name) = command_text.strip_prefix("stop_music ") {
        let name = name.trim();
        if is_simple_identifier(name) {
            return Some(SceneEffect::StopMusic {
                name: Some(name.to_string()),
            });
        }
        return None;
    }
    if let Some(rest) = command_text.strip_prefix("load ") {
        let (target, source) = rest.split_once(" from ")?;
        if validate_runtime_target_path(target) && !source.trim().is_empty() {
            return Some(SceneEffect::LoadPuzzle {
                target: target.trim().to_string(),
                source: source.trim().to_string(),
            });
        }
        return None;
    }
    if command_text.starts_with("start levels ")
        || command_text.starts_with("continue levels ")
        || (command_text.starts_with("start ") && command_text.contains(" in "))
    {
        return None;
    }
    let (command, rest) = command_text.split_once(' ')?;
    let (screen, params) = parse_runtime_scene_target(rest)?;
    if !is_simple_identifier(screen) {
        return None;
    }
    match command {
        "goto" => Some(SceneEffect::Goto {
            scene: screen.to_string(),
            params,
        }),
        "start" => Some(SceneEffect::Sequence(vec![
            SceneEffect::Reset {
                scene: screen.to_string(),
            },
            SceneEffect::Goto {
                scene: screen.to_string(),
                params,
            },
        ])),
        _ => None,
    }
}

fn parse_runtime_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_runtime_scene_target(value: &str) -> Option<(&str, Vec<SceneEffectParam>)> {
    let value = value.trim();
    if let Some((screen, params)) = value.split_once(" with ") {
        return Some((screen.trim(), parse_runtime_params(params.trim())?));
    }
    if let Some((screen, args)) = value.split_once('(') {
        let args = args.strip_suffix(')')?.trim();
        let params = if args.is_empty() {
            Vec::new()
        } else if !args.contains('=') && !args.contains(',') {
            vec![SceneEffectParam::Level(parse_runtime_level_expr(args)?)]
        } else {
            parse_runtime_params(&args.replace(" = ", "="))?
        };
        return Some((screen.trim(), params));
    }
    Some((value, Vec::new()))
}

fn parse_runtime_params(value: &str) -> Option<Vec<SceneEffectParam>> {
    value
        .split(',')
        .map(str::trim)
        .filter(|param| !param.is_empty())
        .map(|param| {
            let (name, value) = param.split_once('=')?;
            let name = name.trim();
            is_simple_identifier(name).then_some(SceneEffectParam::Named {
                name: name.to_string(),
                value: parse_runtime_expr(value.trim())?,
            })
        })
        .collect()
}

fn parse_runtime_wait_duration_ms(value: &str) -> Option<u64> {
    if let Some(milliseconds) = value.strip_suffix("ms") {
        let milliseconds = milliseconds.trim();
        return milliseconds
            .chars()
            .all(|ch| ch.is_ascii_digit())
            .then(|| milliseconds.parse::<u64>().ok())
            .flatten();
    }
    let seconds = value.strip_suffix('s')?.trim();
    let has_decimal = seconds.contains('.');
    let (whole, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
    if whole.is_empty()
        || !whole.chars().all(|ch| ch.is_ascii_digit())
        || (has_decimal && fraction.is_empty())
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
        || fraction.len() > 3
    {
        return None;
    }
    let whole_ms = whole.parse::<u64>().ok()?.checked_mul(1000)?;
    let fraction_ms = if fraction.is_empty() {
        0
    } else {
        format!("{fraction:0<3}").parse::<u64>().ok()?
    };
    whole_ms.checked_add(fraction_ms)
}

fn validate_runtime_target_path(value: &str) -> bool {
    value
        .trim()
        .split('.')
        .all(|part| is_simple_identifier(part))
}

fn parse_puzzle_runtime_command(command_text: &str) -> Option<SceneEffect> {
    if let Some((target_command, level)) = command_text.trim().split_once(' ') {
        let (target, command) = target_command.split_once('.')?;
        if is_simple_identifier(target) && command == "goto" {
            return Some(SceneEffect::GotoLevel {
                target: target.to_string(),
                level: parse_runtime_level_expr(level.trim())?,
            });
        }
        return None;
    }
    let (target, command) = command_text.trim().split_once('.')?;
    if !is_simple_identifier(target) || !is_simple_identifier(command) {
        return None;
    }
    match command {
        "next_level" => Some(SceneEffect::PuzzleNextLevel {
            target: target.to_string(),
        }),
        "previous_level" => Some(SceneEffect::PuzzlePreviousLevel {
            target: target.to_string(),
        }),
        "restart" => Some(SceneEffect::ResetPuzzle {
            target: target.to_string(),
        }),
        _ => None,
    }
}

fn queued_effects_for_outcome(
    game: &LoadedGame,
    target: Option<&str>,
    commands: &[TransitionCommand],
    fired_rules: &[puzzle_core::RuleId],
) -> Vec<QueuedRuleEffect> {
    let mut effects = Vec::new();
    for rule in fired_rules {
        let Some(rule_effects) = game.rule_effects.get(rule) else {
            continue;
        };
        effects.extend(rule_effects.iter().cloned().map(|effect| QueuedRuleEffect {
            target: target.map(str::to_string),
            effect,
        }));
    }
    if effects.is_empty() {
        effects.extend(commands.iter().map(|command| QueuedRuleEffect {
            target: target.map(str::to_string),
            effect: match command {
                TransitionCommand::Win => RuleEffect::Win,
                TransitionCommand::Restart => RuleEffect::Restart,
                TransitionCommand::NextLevel => RuleEffect::NextLevel,
                TransitionCommand::Again => RuleEffect::Again,
                TransitionCommand::Checkpoint => RuleEffect::Checkpoint,
                TransitionCommand::ClearCheckpoint => RuleEffect::ClearCheckpoint,
            },
        }));
    }
    effects
}

fn transition_program_segment_outcome(
    game: &LoadedGame,
    program: &[RuleStep],
    state: &PuzzleState,
    input: InputId,
    target: Option<&str>,
) -> Result<ProgramSegmentOutcome, TransitionError> {
    if program.is_empty() {
        return Ok(ProgramSegmentOutcome {
            next_state: state.clone(),
            cancelled: false,
            effects: Vec::new(),
            animations: Vec::new(),
            checkpoint: None,
        });
    }

    let mut last = None;
    for index in 0..program.len() {
        let outcome = transition_program_trace(&game.game, &program[..=index], state, input)?;
        let effects =
            queued_effects_for_outcome(game, target, &outcome.commands, &outcome.fired_rules);
        let animations = animation_events_for_trace(
            game,
            &outcome.fired_rules,
            &outcome.patches,
            &outcome.next_state,
        );
        if let Some((before_wait, milliseconds, after_wait)) =
            split_effects_at_program_boundary(game, effects, &animations)
        {
            return Ok(ProgramSegmentOutcome {
                next_state: outcome.next_state,
                cancelled: outcome.cancelled,
                effects: before_wait,
                animations,
                checkpoint: Some(EffectCheckpoint {
                    milliseconds,
                    effects_after_wait: after_wait,
                    remaining_program: program[index + 1..].to_vec(),
                }),
            });
        }
        last = Some(outcome);
    }

    let outcome = last.expect("non-empty program has an outcome");
    Ok(ProgramSegmentOutcome {
        effects: queued_effects_for_outcome(game, target, &outcome.commands, &outcome.fired_rules),
        animations: animation_events_for_trace(
            game,
            &outcome.fired_rules,
            &outcome.patches,
            &outcome.next_state,
        ),
        next_state: outcome.next_state,
        cancelled: outcome.cancelled,
        checkpoint: None,
    })
}

pub fn animation_events_for_trace(
    game: &LoadedGame,
    fired_rules: &[puzzle_core::RuleId],
    patches: &[puzzle_core::Patch],
    next_state: &PuzzleState,
) -> Vec<AnimationEvent> {
    let mut events = Vec::new();
    for (rule, patch) in fired_rules.iter().zip(patches.iter()) {
        let Some(animations) = game.rule_animations.get(rule) else {
            continue;
        };
        for RuleAnimation {
            trigger,
            name,
            objects,
        } in animations
        {
            match trigger {
                RuleAnimationTrigger::Move => {
                    for op in patch.ops() {
                        let PatchOp::Move {
                            from_x,
                            from_y,
                            to_x,
                            to_y,
                            object,
                        } = op
                        else {
                            continue;
                        };
                        if objects.contains(object) {
                            push_unique_animation(
                                &mut events,
                                AnimationEvent::Move {
                                    name: name.clone(),
                                    object: *object,
                                    from_x: *from_x,
                                    from_y: *from_y,
                                    from_z: 0,
                                    to_x: *to_x,
                                    to_y: *to_y,
                                    to_z: 0,
                                },
                            );
                        }
                    }
                }
                RuleAnimationTrigger::CantMove => {
                    for op in patch.ops() {
                        let PatchOp::RemoveScratch {
                            x,
                            y,
                            object,
                            scratch,
                            ..
                        } = op
                        else {
                            continue;
                        };
                        if scratch.0 != 0 {
                            continue;
                        }
                        if object.0 != 0 {
                            if objects.contains(object) {
                                push_unique_animation(
                                    &mut events,
                                    AnimationEvent::CantMove {
                                        name: name.clone(),
                                        object: *object,
                                        x: *x,
                                        y: *y,
                                    },
                                );
                            }
                            continue;
                        }
                        for candidate in objects {
                            if next_state.has_object(&game.game, *x, *y, *candidate) {
                                push_unique_animation(
                                    &mut events,
                                    AnimationEvent::CantMove {
                                        name: name.clone(),
                                        object: *candidate,
                                        x: *x,
                                        y: *y,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    events
}

fn push_unique_animation(events: &mut Vec<AnimationEvent>, event: AnimationEvent) {
    if !events.contains(&event) {
        events.push(event);
    }
}

fn split_effects_at_program_boundary(
    game: &LoadedGame,
    effects: Vec<QueuedRuleEffect>,
    animations: &[AnimationEvent],
) -> Option<(Vec<QueuedRuleEffect>, u64, Vec<QueuedRuleEffect>)> {
    let mut boundary = None;
    for (index, effect) in effects.iter().enumerate() {
        match effect.effect {
            RuleEffect::Wait { milliseconds } => {
                boundary = Some((index, index + 1, milliseconds));
                break;
            }
            RuleEffect::WaitAnimation => {
                if let Some(milliseconds) = animation_wait_milliseconds(game, animations) {
                    boundary = Some((index, index + 1, milliseconds));
                    break;
                }
            }
            RuleEffect::Message { .. } => {
                boundary = Some((index + 1, index + 1, game.default_wait_ms));
                break;
            }
            _ => {}
        }
    }
    let (before_end, after_start, milliseconds) = boundary?;
    let before = effects[..before_end].to_vec();
    let after = effects[after_start..].to_vec();
    Some((before, milliseconds, after))
}

fn animation_wait_milliseconds(game: &LoadedGame, animations: &[AnimationEvent]) -> Option<u64> {
    animations
        .iter()
        .map(|animation| match animation {
            AnimationEvent::Move { name, .. } | AnimationEvent::CantMove { name, .. } => {
                animation_duration_milliseconds(game, name)
            }
        })
        .max()
}

fn animation_duration_milliseconds(game: &LoadedGame, name: &str) -> u64 {
    if name == "tween" {
        return game.animation.tween.interval_ms;
    }
    game.default_wait_ms
}

fn parse_runtime_expr(value: &str) -> Option<SceneExpr> {
    if value == "true" {
        return Some(SceneExpr::Bool(true));
    }
    if value == "false" {
        return Some(SceneExpr::Bool(false));
    }
    if let Ok(number) = value.parse::<i64>() {
        return Some(SceneExpr::Int(number));
    }
    if let Some(text) = parse_runtime_quoted_text(value) {
        return Some(SceneExpr::Text(text));
    }
    if let Some((name, rest)) = value.split_once('(') {
        let args = rest.strip_suffix(')')?;
        if !is_simple_identifier(name) {
            return None;
        }
        let args = if args.trim().is_empty() {
            Vec::new()
        } else {
            args.split(',')
                .map(str::trim)
                .map(parse_runtime_expr)
                .collect::<Option<Vec<_>>>()?
        };
        return Some(SceneExpr::Call {
            name: name.to_string(),
            args,
        });
    }
    let parts = value.split('.').collect::<Vec<_>>();
    parts
        .iter()
        .all(|part| is_simple_identifier(part))
        .then(|| SceneExpr::Path(parts.into_iter().map(ToString::to_string).collect()))
}

fn parse_runtime_level_expr(value: &str) -> Option<SceneExpr> {
    if parse_runtime_quoted_text(value).is_some() {
        return None;
    }
    if is_dotted_level_atom(value) {
        return Some(SceneExpr::Text(value.to_string()));
    }
    parse_runtime_expr(value)
}

fn runtime_command_has_quoted_level_arg(command: &str) -> bool {
    let command = command.trim();
    if let Some(value) = command.strip_prefix("set current_level = ") {
        return parse_runtime_quoted_text(value.trim()).is_some();
    }
    if let Some(rest) = command.strip_prefix("set level(") {
        return rest
            .split_once(").cleared = ")
            .is_some_and(|(level, _)| parse_runtime_quoted_text(level.trim()).is_some());
    }
    if let Some(rest) = command
        .strip_prefix("goto ")
        .or_else(|| command.strip_prefix("start "))
    {
        return runtime_scene_target_has_quoted_level_arg(rest);
    }
    if let Some((target_command, level)) = command.split_once(' ') {
        return target_command
            .split_once('.')
            .is_some_and(|(_, action)| action == "goto")
            && parse_runtime_quoted_text(level.trim()).is_some();
    }
    false
}

fn runtime_scene_target_has_quoted_level_arg(target: &str) -> bool {
    let Some((_, args)) = target.trim().split_once('(') else {
        return false;
    };
    let Some(args) = args.strip_suffix(')') else {
        return false;
    };
    let args = args.trim();
    !args.contains('=') && !args.contains(',') && parse_runtime_quoted_text(args).is_some()
}

fn parse_runtime_quoted_text(value: &str) -> Option<String> {
    let inner = value.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.replace("\\\"", "\""))
}

fn is_simple_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_dotted_level_atom(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() > 1
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        })
}

fn level_index_from_value(game: &LoadedGame, value: &str) -> Option<usize> {
    value
        .parse::<usize>()
        .ok()
        .filter(|index| *index < game.levels.len())
        .or_else(|| game.levels.iter().position(|level| level.name == value))
}

fn scene_level_indices(game: &LoadedGame, scene_name: &str) -> Vec<usize> {
    let Some(scene) = game.scenes.iter().find(|scene| scene.name == scene_name) else {
        return (0..game.levels.len()).collect();
    };
    match &scene.resources.levels {
        ResourceSelection::All => (0..game.levels.len()).collect(),
        ResourceSelection::Named(names) => game
            .levels
            .iter()
            .enumerate()
            .filter_map(|(index, level)| {
                names
                    .iter()
                    .any(|name| level_resource_matches(name, &level.name))
                    .then_some(index)
            })
            .collect(),
    }
}

fn first_level_index_for_scene(
    game: &LoadedGame,
    scene_name: &str,
    scope: Option<&str>,
) -> Option<usize> {
    scene_level_indices(game, scene_name)
        .into_iter()
        .find(|index| {
            scope.is_none_or(|scope| level_resource_matches(scope, &game.levels[*index].name))
        })
}

fn level_has_next_in_scene(game: &LoadedGame, scene_name: &str, level_index: usize) -> bool {
    let indices = scene_level_indices(game, scene_name);
    indices
        .iter()
        .position(|index| *index == level_index)
        .is_some_and(|position| position + 1 < indices.len())
}

fn scene_accepts_level(game: &LoadedGame, scene_name: &str, level_index: usize) -> bool {
    scene_level_indices(game, scene_name)
        .into_iter()
        .any(|index| index == level_index)
}

fn transition_condition_mentions_input(trigger: &SceneTransitionTrigger, input: &str) -> bool {
    let SceneTransitionTrigger::Condition(condition) = trigger else {
        return false;
    };
    condition.split(" and ").any(|part| {
        let part = part.trim();
        part.strip_prefix("input == ")
            .is_some_and(|name| name.trim() == input)
    })
}

fn level_resource_matches(resource: &str, level_name: &str) -> bool {
    level_name == resource
        || level_name
            .strip_prefix(resource)
            .is_some_and(|rest| rest.starts_with('.'))
}

fn component_has_level_menu(component: &SceneComponent) -> bool {
    match component {
        SceneComponent::LevelMenu(_) => true,
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => container.children.iter().any(component_has_level_menu),
        SceneComponent::Conditional(conditional) => conditional
            .children
            .iter()
            .chain(conditional.else_children.iter())
            .any(component_has_level_menu),
        SceneComponent::For(for_view) => for_view.children.iter().any(component_has_level_menu),
        SceneComponent::Frame(_)
        | SceneComponent::Title(_)
        | SceneComponent::Subtitle(_)
        | SceneComponent::Text(_)
        | SceneComponent::Button(_)
        | SceneComponent::Choice(_) => false,
    }
}

fn find_level_menu(components: &[SceneComponent]) -> Option<&LevelMenuDef> {
    for component in components {
        match component {
            SceneComponent::LevelMenu(menu) => return Some(menu),
            SceneComponent::Row(container)
            | SceneComponent::Column(container)
            | SceneComponent::Box(container) => {
                if let Some(menu) = find_level_menu(&container.children) {
                    return Some(menu);
                }
            }
            SceneComponent::Conditional(conditional) => {
                if let Some(menu) = find_level_menu(&conditional.children) {
                    return Some(menu);
                }
                if let Some(menu) = find_level_menu(&conditional.else_children) {
                    return Some(menu);
                }
            }
            SceneComponent::For(for_view) => {
                if let Some(menu) = find_level_menu(&for_view.children) {
                    return Some(menu);
                }
            }
            _ => {}
        }
    }
    None
}

fn level_menu_columns(menu: &LevelMenuDef) -> usize {
    menu.columns.map(usize::from).unwrap_or(1).max(1)
}

fn level_menu_action_bindings(game: &LoadedGame, level_index: usize) -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    bindings.insert("level".to_string(), level_index.to_string());
    if let Some(level) = game.levels.get(level_index) {
        bindings.insert("level.name".to_string(), level.name.clone());
        bindings.insert("level.index".to_string(), level_index.to_string());
        bindings.insert("level.label".to_string(), level.name.clone());
        if let Some(pack) = &level.pack {
            bindings.insert("level.pack".to_string(), pack.clone());
        }
    }
    bindings
}

fn split_command_cursor_override(command: &str) -> (&str, Option<usize>) {
    let Some((name, payload)) = command.split_once(':') else {
        return (command, None);
    };
    (name, payload.parse::<usize>().ok())
}

fn input_id_by_label(game: &LoadedGame, input_name: &str) -> Option<InputId> {
    game.input_labels
        .iter()
        .find_map(|(id, label)| (label == input_name).then_some(*id))
}

fn condition_name(value: &str) -> &str {
    value.rsplit_once('.').map_or(value, |(_, name)| name)
}

pub fn render_ascii_top(state: &PuzzleState, legend: &AsciiLegend) -> String {
    let mut out = String::new();

    for y in 0..state.height {
        for x in 0..state.width {
            out.push(legend.char_for_cell(&cell_objects(state, x, y)));
        }
        if y + 1 < state.height {
            out.push('\n');
        }
    }

    out
}

pub fn cell_objects(state: &PuzzleState, x: u16, y: u16) -> Vec<ObjectId> {
    let mut objects = Vec::new();
    for layer in 0..state.layer_count {
        let object = state
            .get_layer(x, y, LayerId(layer))
            .unwrap_or(ObjectId::EMPTY);
        if !object.is_empty() {
            objects.push(object);
        }
    }
    objects
}

#[cfg(test)]
mod tests {
    use super::*;
    use puzzle_lang::parse_game2d as parse_game;

    fn object_named(loaded: &LoadedGame, name: &str) -> ObjectId {
        loaded
            .object_labels
            .iter()
            .find_map(|(object, label)| (label == name).then_some(*object))
            .unwrap()
    }

    fn input_named(loaded: &LoadedGame, name: &str) -> InputId {
        loaded
            .input_labels
            .iter()
            .find_map(|(input, label)| (label == name).then_some(*input))
            .unwrap()
    }

    #[test]
    fn again_command_runs_no_input_follow_up_turn() {
        let loaded = parse_game(
            r#"
title again_runtime
puzzle default {
layers {
__legacy_layer_0 = Before After
}
empty .
rules {
once [ Before ] -> [ After ]
}
levels {
legend {
B = Before
C = After
}
B
}
}
"#,
        )
        .unwrap();
        let after = object_named(&loaded, "After");
        let mut session = GameSession::new(&loaded);

        session
            .resolve_turn_commands(
                &loaded,
                vec![QueuedTransitionCommand {
                    target: None,
                    command: TransitionCommand::Again,
                }],
                None,
            )
            .unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, after));
    }

    #[test]
    fn rewrite_again_effect_lowers_to_transition_command() {
        let loaded = parse_game(
            r#"
title again_effect
puzzle default {
layers {
__legacy_layer_0 = Before After
}
empty .
rules {
once [ Before ] -> [ After ] again
}
levels {
legend {
A = Before
B = After
}
A
}
}
"#,
        )
        .unwrap();

        let outcome =
            transition_outcome(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

        assert_eq!(outcome.commands, vec![TransitionCommand::Again]);
    }

    #[test]
    fn checkpoint_effect_changes_restart_anchor() {
        let loaded = parse_game(
            r#"
title checkpoint_runtime
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
input save
input clear
rules {
if input == right {
once right [ Player | no Player ] -> [ | Player ]
}
if input == save {
checkpoint
}
if input == clear {
clear_checkpoint
}
}
levels {
legend P = Player

level start {
P..
}
}
}
"#,
        )
        .unwrap();
        let player = object_named(&loaded, "Player");
        let mut session = GameSession::new(&loaded);

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();
        assert!(session.state().has_object(&loaded.game, 1, 0, player));

        session
            .apply_input(&loaded, input_named(&loaded, "save"))
            .unwrap();
        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();
        assert!(session.state().has_object(&loaded.game, 2, 0, player));

        session
            .apply_input(&loaded, input_named(&loaded, "restart"))
            .unwrap();
        assert!(session.state().has_object(&loaded.game, 1, 0, player));

        session
            .apply_input(&loaded, input_named(&loaded, "clear"))
            .unwrap();
        session
            .apply_input(&loaded, input_named(&loaded, "restart"))
            .unwrap();
        assert!(session.state().has_object(&loaded.game, 0, 0, player));
    }

    #[test]
    fn rewrite_win_effect_lowers_to_transition_command() {
        let loaded = parse_game(
            r#"
title win_effect
puzzle default {
layers {
floor = Exit
actor = Player
}
rules {
[ Player Exit ] -> win
}
levels {
legend {
. = empty
X = Player Exit
}
X
}
}
"#,
        )
        .unwrap();

        let outcome =
            transition_outcome(&loaded.game, &loaded.levels[0].initial_state, InputId(0)).unwrap();

        assert_eq!(outcome.commands, vec![TransitionCommand::Win]);
    }

    #[test]
    fn enabled_tween_emits_move_animation_event_from_patch_payload() {
        let loaded = parse_game(
            r#"
title tween_runtime
animation {
tween {
duration = 80ms
}
}

puzzle default {
layers {
actor = Player
}
rules {
input directions [ Player ] -> [ Player{>} ]
move
}
levels {
legend {
. = empty
P = Player
}
P.
}
}
"#,
        )
        .unwrap();
        let player = object_named(&loaded, "Player");
        let mut session = GameSession::new(&loaded);

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        assert_eq!(
            session.take_animation_events(),
            vec![AnimationEvent::Move {
                name: "tween".to_string(),
                object: player,
                from_x: 0,
                from_y: 0,
                from_z: 0,
                to_x: 1,
                to_y: 0,
                to_z: 0,
            }]
        );
    }

    #[test]
    fn win_effect_forces_level_clear_lifecycle() {
        let loaded = parse_game(
            r#"
title win_effect_runtime
puzzle default {
layers {
floor = Exit
marker = Cleared
actor = Player
}
win_conditions {
no Exit
}
on_level_clear {
[ Player Exit no Cleared ] -> [ Player Exit Cleared ]
next_level
}
rules {
[ Player Exit ] -> win
}
levels {
legend {
. = empty
X = Player Exit
P = Player
}
X

P
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, InputId(0)).unwrap();

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    fn transition_fixture() -> LoadedGame {
        parse_game(
            r#"
title transition_fixture
puzzle sokoban {
layers {
floor = Goal
actor = Player Box Wall
}
group solid = Player Box Wall
win_conditions {
some Goal
all Goal on Box
}
rules {
once input directions [ Player | Box | no solid ] -> [ | Player | Box ]
once input directions [ Player | no solid ] -> [ | Player ]
}
levels {
legend {
. = empty
G = Goal
# = Wall
* = Goal Box
P = Player
B = Box
}
level first {
#######
#P.B.G#
#######
}
level second {
#######
#P.B.G#
#######
}
}
}
scene playing {
state {
board = puzzle sokoban
}
layout {
board
}
rules {
step board
if board.win_conditions and board.level.last -> goto level_select
if board.win_conditions -> board.next_level
}
}
scene level_select {
layout {
text "Level Select"
}
}
"#,
        )
        .unwrap()
    }

    fn load_named_scene_level(
        loaded: &LoadedGame,
        session: &mut GameSession,
        target: &str,
        level_name: &str,
    ) {
        let index = loaded
            .levels
            .iter()
            .position(|level| level.name == level_name)
            .unwrap_or_else(|| panic!("missing level {level_name}"));
        session
            .apply_command(
                loaded,
                &format!("load {target} from {target}.levels[{index}]"),
            )
            .unwrap();
    }

    fn scene_local_puzzle_fixture() -> LoadedGame {
        parse_game(
            r#"
title scene_local_puzzle_fixture

puzzle sokoban {
var portal_entered = false

layers {
trigger = Portal
solid = Player Wall
}

rules {
once [ Player ] -> set portal_entered = false
for d in directions {
if input == d {
once d [ Player | Portal no solid ] -> [ | Player ] set portal_entered = true
once d [ Player | no solid ] -> [ | Player ]
}
}
}

levels spec of sokoban {
legend {
. = empty
P = Player
O = Portal
# = Wall
}

level hub {
####
#PO#
#..#
####
}
}
}

scene hub {
resources {
levels spec
}
state {
board = puzzle sokoban
spec_board = puzzle sokoban
}
layout {
board
}
rules {
step spec_board
if spec_board.portal_entered -> goto child_1
}
}

scene child_1 {
layout {
text "Child 1"
}
}

scene checkpoint {
resources {
levels spec
}
state {
spec_board = puzzle sokoban
}
layout {
spec_board
}
rules {
step spec_board
}
}
"#,
        )
        .unwrap()
    }

    fn enter_scene_local_hub(loaded: &LoadedGame, session: &mut GameSession) {
        session.apply_command(loaded, "goto hub").unwrap();
        load_named_scene_level(loaded, session, "hub.spec_board", "spec.hub");
        assert_eq!(session.screen(), "hub");
    }

    #[test]
    fn scene_condition_can_read_board_var_directly() {
        let loaded = parse_game(
            r#"
title scene_var_condition

puzzle default {
layers {
__legacy_layer_1 = Player
}
empty .

var moved = false


input tick

rules {
once [ Player ] -> set moved = true
}

levels {
legend P = Player
level start {
P
}
}
}

scene playing {
state {
board = puzzle default
}
layout {
board
}
rules {
step board
if board.moved -> goto moved
}
}

scene moved {
layout {
text "moved"
}
}
"#,
        )
        .unwrap();
        let tick = input_named(&loaded, "tick");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, tick).unwrap();

        assert_eq!(session.screen(), "moved");
    }

    #[test]
    fn scene_start_transition_runs_when_scene_becomes_focused() {
        let loaded = parse_game(
            r#"
title scene_start_fixture
puzzle default {
persistent var moves = 0

layers {
__legacy_layer_0 = Player
}
empty .

rules {
}
levels {
legend P = Player
level start {
P
}
}
}
scene playing {
layout {
text "Playing"
}
rules {
if input == open -> goto menu
}
}
scene menu {
layout {
text "Menu"
}
on_scene_start {
stop_music music_name
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        assert!(session.take_sound_events().is_empty());

        session.apply_command(&loaded, "goto menu").unwrap();

        assert_eq!(
            session.take_sound_events(),
            vec![SoundEvent::StopMusic {
                name: Some("music_name".to_string())
            }]
        );
    }

    #[test]
    fn puzzle_rule_sfx_effect_queues_sound_event_on_match() {
        let loaded = parse_game(
            r#"
title rule_sfx_fixture
sounds {
sfx push seed=push01 type=jump
}
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
right [ Player | ] -> [ | Player ] sfx push
}
levels {
legend {
. = empty
P = Player
}
level start {
P.
}
}
}
"#,
        )
        .unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert_eq!(
            session.take_sound_events(),
            vec![SoundEvent::PlaySfx {
                name: "push".to_string()
            }]
        );
    }

    #[test]
    fn puzzle_rule_music_effect_queues_sound_event_on_match() {
        let loaded = parse_game(
            r#"
title rule_music_fixture
sounds {
music locked_room seed=room01
}
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
right [ Player | ] -> [ | Player ] stop_music locked_room
}
levels {
legend {
. = empty
P = Player
}
level start {
P.
}
}
}
"#,
        )
        .unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert_eq!(
            session.take_sound_events(),
            vec![SoundEvent::StopMusic {
                name: Some("locked_room".to_string())
            }]
        );
    }

    #[test]
    fn model_move_sound_trigger_queues_sound_event_on_matching_move() {
        let loaded = parse_game(
            r#"
title model_move_sfx_fixture
sounds {
sfx push seed=push01 type=jump
}
puzzle default {
layers {
actor = Player Box
}
sounds {
move Box -> sfx push
}
rules {
right [ Player | Box | ] -> [ | Player | Box ]
}
levels {
legend {
. = empty
P = Player
B = Box
}
level start {
PB.
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert_eq!(
            session.take_sound_events(),
            vec![SoundEvent::PlaySfx {
                name: "push".to_string()
            }]
        );
    }

    #[test]
    fn model_cantmove_sound_trigger_queues_sound_event_on_blocked_move_intent() {
        let loaded = parse_game(
            r#"
title model_cantmove_sfx_fixture
sounds {
sfx bump seed=bump01 type=hit
}
puzzle default {
layers {
actor = Player Box Wall
}
sounds {
cantmove Box -> sfx bump
}
rules {
input directions [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
move
}
levels {
legend {
. = empty
P = Player
B = Box
# = Wall
}
level start {
PB#
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert_eq!(
            session.take_sound_events(),
            vec![SoundEvent::PlaySfx {
                name: "bump".to_string()
            }]
        );
    }

    #[test]
    fn model_cantmove_sound_trigger_ignores_direction_cleanup_without_intent() {
        let loaded = parse_game(
            r#"
title model_cantmove_cleanup_without_intent_fixture
sounds {
sfx bump seed=bump01 type=hit
}
puzzle default {
layers {
actor = A
}
sounds {
cantmove A -> sfx bump
}
rules {
right [ A ] -> [ A{no directions} ]
}
levels {
legend {
. = empty
A = A
}
level start {
A
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert!(session.take_sound_events().is_empty());
    }

    #[test]
    fn model_cantmove_sound_trigger_ignores_other_object_set_intent() {
        let loaded = parse_game(
            r#"
title model_cantmove_other_object_set_fixture
sounds {
sfx bump seed=bump01 type=hit
}
puzzle default {
layers {
actor = A
crate = B Wall
}
sounds {
cantmove A -> sfx bump
}
rules {
right [ B ] -> [ > B ]
move
}
levels {
legend {
. = empty
A = A
B = B
# = Wall
}
level start {
B#
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert!(session.take_sound_events().is_empty());
    }

    #[test]
    fn standard_move_collision_blocks_head_on_claims_and_queues_cantmove_sfx() {
        let loaded = parse_game(
            r#"
title model_collision_sfx_fixture
sounds {
sfx bump seed=bump01 type=hit
}
puzzle default {
layers {
actor = A
}
sounds {
cantmove A -> sfx bump
}
rules {
right [ A | | A ] -> [ > A | | < A ]
move
}
levels {
legend {
. = empty
A = A
}
level start {
A.A
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let a = object_named(&loaded, "A");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, a));
        assert!(!session.state().has_object(&loaded.game, 1, 0, a));
        assert!(session.state().has_object(&loaded.game, 2, 0, a));
        assert_eq!(
            session.take_sound_events(),
            vec![SoundEvent::PlaySfx {
                name: "bump".to_string()
            }]
        );
    }

    #[test]
    fn standard_move_collision_blocks_corner_claims_and_queues_cantmove_sfx() {
        let loaded = parse_game(
            r#"
title model_corner_collision_sfx_fixture
sounds {
sfx bump seed=bump01 type=hit
}
puzzle default {
layers {
actor = A
}
sounds {
cantmove A -> sfx bump
}
rules {
right [ A | ; | A ] -> [ > A | ; | ^ A ]
move
}
levels {
legend {
. = empty
A = A
}
level start {
A.
.A
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let a = object_named(&loaded, "A");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, a));
        assert!(!session.state().has_object(&loaded.game, 1, 0, a));
        assert!(!session.state().has_object(&loaded.game, 0, 1, a));
        assert!(session.state().has_object(&loaded.game, 1, 1, a));
        assert_eq!(
            session.take_sound_events(),
            vec![SoundEvent::PlaySfx {
                name: "bump".to_string()
            }]
        );
    }

    #[test]
    fn standard_move_collision_blocks_three_way_claims_and_queues_cantmove_sfx() {
        let loaded = parse_game(
            r#"
title model_three_way_collision_sfx_fixture
sounds {
sfx bump seed=bump01 type=hit
}
puzzle default {
layers {
actor = A
}
sounds {
cantmove A -> sfx bump
}
rules {
right [ A | | A ; | A | ] -> [ > A | | < A ; | ^ A | ]
move
}
levels {
legend {
. = empty
A = A
}
level start {
A.A
.A.
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let a = object_named(&loaded, "A");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, a));
        assert!(!session.state().has_object(&loaded.game, 1, 0, a));
        assert!(session.state().has_object(&loaded.game, 2, 0, a));
        assert!(!session.state().has_object(&loaded.game, 0, 1, a));
        assert!(session.state().has_object(&loaded.game, 1, 1, a));
        assert!(!session.state().has_object(&loaded.game, 2, 1, a));
        assert_eq!(
            session.take_sound_events(),
            vec![SoundEvent::PlaySfx {
                name: "bump".to_string()
            }]
        );
    }

    #[test]
    fn standard_move_blocked_intent_does_not_queue_move_sfx() {
        let loaded = parse_game(
            r#"
title model_blocked_move_sfx_fixture
sounds {
sfx move seed=move01 type=jump
sfx bump seed=bump01 type=hit
}
puzzle default {
layers {
actor = A Wall
}
sounds {
move A -> sfx move
cantmove A -> sfx bump
}
rules {
right [ A ] -> [ > A ]
move
}
levels {
legend {
. = empty
A = A
# = Wall
}
level start {
A#
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let a = object_named(&loaded, "A");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, a));
        assert_eq!(
            session.take_sound_events(),
            vec![SoundEvent::PlaySfx {
                name: "bump".to_string()
            }]
        );
    }

    #[test]
    fn standard_move_mixed_blocked_and_successful_intents_queue_both_sfx() {
        let loaded = parse_game(
            r#"
title model_mixed_move_sfx_fixture
sounds {
sfx move seed=move01 type=jump
sfx bump seed=bump01 type=hit
}
puzzle default {
layers {
actor = A Wall
}
sounds {
move A -> sfx move
cantmove A -> sfx bump
}
rules {
right [ A ] -> [ > A ]
move
}
levels {
legend {
. = empty
A = A
# = Wall
}
level start {
A#.A.
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let a = object_named(&loaded, "A");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, a));
        assert!(session.state().has_object(&loaded.game, 4, 0, a));
        assert_eq!(
            session.take_sound_events(),
            vec![
                SoundEvent::PlaySfx {
                    name: "move".to_string()
                },
                SoundEvent::PlaySfx {
                    name: "bump".to_string()
                }
            ]
        );
    }

    #[test]
    fn standard_move_still_moves_same_direction_chains() {
        let loaded = parse_game(
            r#"
title model_chain_move_fixture
puzzle default {
layers {
actor = A
}
rules {
right [ A | A | ] -> [ > A | > A | ]
move
}
levels {
legend {
. = empty
A = A
}
level start {
AA.
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let a = object_named(&loaded, "A");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert!(!session.state().has_object(&loaded.game, 0, 0, a));
        assert!(session.state().has_object(&loaded.game, 1, 0, a));
        assert!(session.state().has_object(&loaded.game, 2, 0, a));
    }

    #[test]
    fn rule_sfx_is_deduped_within_one_turn() {
        let loaded = parse_game(
            r#"
title rule_sfx_dedup_fixture
sounds {
sfx push seed=push01 type=jump
}
puzzle default {
layers {
actor = Box
}
rules {
once_all right [ Box | ] -> [ | Box ] sfx push
}
levels {
legend {
. = empty
B = Box
}
level start {
B.B.
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert_eq!(
            session.take_sound_events(),
            vec![SoundEvent::PlaySfx {
                name: "push".to_string()
            }]
        );
    }

    #[test]
    fn again_follow_up_turn_has_separate_sfx_dedup_scope() {
        let loaded = parse_game(
            r#"
title again_sfx_scope_fixture
sounds {
sfx push seed=push01 type=jump
}
puzzle default {
layers {
actor = Box
}
rules {
once right [ Box | ] -> [ | Box ] sfx push again
}
levels {
legend {
. = empty
B = Box
}
level start {
B..
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert_eq!(
            session.take_sound_events(),
            vec![
                SoundEvent::PlaySfx {
                    name: "push".to_string()
                },
                SoundEvent::PlaySfx {
                    name: "push".to_string()
                }
            ]
        );
    }

    #[test]
    fn puzzle_do_sfx_effect_queues_sound_event() {
        let loaded = parse_game(
            r#"
title rule_do_sfx_fixture
sounds {
sfx tick seed=tick01 type=jump
}
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
sfx tick
}
levels {
legend {
. = empty
P = Player
}
level start {
P
}
}
}
"#,
        )
        .unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert_eq!(
            session.take_sound_events(),
            vec![SoundEvent::PlaySfx {
                name: "tick".to_string()
            }]
        );
    }

    #[test]
    fn puzzle_wait_effect_queues_wait_event() {
        let loaded = parse_game(
            r#"
title rule_wait_fixture
default_wait_time = 300ms
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
wait
wait 25ms
}
levels {
legend {
. = empty
P = Player
}
level start {
P
}
}
}
"#,
        )
        .unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert_eq!(
            session.take_wait_events(),
            vec![WaitEvent::ContinueEffects { milliseconds: 300 }]
        );

        session
            .apply_command(&loaded, "__continue_effects")
            .unwrap();

        assert_eq!(
            session.take_wait_events(),
            vec![WaitEvent::ContinueEffects { milliseconds: 25 }]
        );

        session
            .apply_command(&loaded, "__continue_effects")
            .unwrap();
        assert!(session.take_wait_events().is_empty());
    }

    #[test]
    fn wait_animation_pauses_until_segment_animation_completes() {
        let loaded = parse_game(
            r#"
title wait_animation_fixture
animation {
tween {
duration = 80ms
}
}
puzzle default {
layers {
actor = Player
marker = Marker
}
rules {
input directions [ Player ] -> [ > Player ]
move
wait animation
[ Player no Marker ] -> [ Player Marker ]
}
levels {
legend {
. = empty
P = Player
}
level start {
P.
}
}
}
"#,
        )
        .unwrap();
        let player = object_named(&loaded, "Player");
        let marker = object_named(&loaded, "Marker");
        let mut session = GameSession::new(&loaded);

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        assert_eq!(
            session.take_wait_events(),
            vec![WaitEvent::ContinueEffects { milliseconds: 80 }]
        );
        assert_eq!(
            session.take_animation_events(),
            vec![AnimationEvent::Move {
                name: "tween".to_string(),
                object: player,
                from_x: 0,
                from_y: 0,
                from_z: 0,
                to_x: 1,
                to_y: 0,
                to_z: 0,
            }]
        );
        assert!(!session.state().has_object(&loaded.game, 1, 0, marker));

        session
            .apply_command(&loaded, "__continue_effects")
            .unwrap();

        assert!(session.state().has_object(&loaded.game, 1, 0, marker));
        assert!(session.take_wait_events().is_empty());
    }

    #[test]
    fn wait_animation_without_animation_is_noop() {
        let loaded = parse_game(
            r#"
title wait_animation_noop_fixture
puzzle default {
layers {
actor = Player
marker = Marker
}
rules {
wait animation
[ Player no Marker ] -> [ Player Marker ]
}
levels {
legend {
. = empty
P = Player
}
level start {
P
}
}
}
"#,
        )
        .unwrap();
        let marker = object_named(&loaded, "Marker");
        let mut session = GameSession::new(&loaded);

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        assert!(session.take_wait_events().is_empty());
        assert!(session.state().has_object(&loaded.game, 0, 0, marker));
    }

    #[test]
    fn scene_message_effect_queues_popup_message() {
        let loaded = parse_game(
            r#"
title scene_message_fixture
default_wait_time = 350ms
var hint = "Push the box"
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
}

levels {
legend {
P = Player
}

level start {
P
}
}
}
scene playing {
layout {
text "Playing"
}
on_scene_start {
message hint
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        assert_eq!(
            session.take_message_events(),
            vec![MessageEvent::Message {
                text: "Push the box".to_string()
            }]
        );
        assert_eq!(
            session.take_wait_events(),
            vec![WaitEvent::Wait { milliseconds: 350 }]
        );
    }

    #[test]
    fn scene_level_name_condition_scopes_lifecycle_message() {
        let loaded = parse_game(
            r#"
title level_name_condition_message
var hint = "First level only"
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
}
levels {
legend {
P = Player
}
level first
P

level second
P
}
}
scene playing {
state {
board = puzzle default
}
layout {
text "Playing"
}
on_scene_start {
if board.level.name == first {
message hint
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();

        assert!(session.take_message_events().is_empty());

        session.start_level(&loaded, 1);
        assert!(session.take_message_events().is_empty());
    }

    #[test]
    fn puzzle_rule_message_effect_queues_popup_message() {
        let loaded = parse_game(
            r#"
title puzzle_message_fixture
default_wait_time = 400ms
var hint = "Found"
puzzle default {
layers {
actor = Player
floor = Goal
}
rules {
once [ Player Goal ] -> message hint
}

levels {
legend {
. = empty
P = Player
G = Goal
* = Player Goal
}

level start {
*
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        session.take_message_events();

        let right = input_id_by_label(&loaded, "right").unwrap();
        session.apply_input(&loaded, right).unwrap();

        assert_eq!(
            session.take_message_events(),
            vec![MessageEvent::Message {
                text: "Found".to_string()
            }]
        );
        assert_eq!(
            session.take_wait_events(),
            vec![WaitEvent::ContinueEffects { milliseconds: 400 }]
        );
    }

    #[test]
    fn message_effect_waits_before_following_rule_segment() {
        let loaded = parse_game(
            r#"
title message_rule_segment_wait
default_wait_time = 450ms
puzzle default {
layers {
__legacy_layer_0 = A B C
}
empty .
rules {
[ A ] -> [ B ] message "changed"
[ B ] -> [ C ]
}
levels {
legend {
A = A
B = B
C = C
}
level start {
A
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let b = object_named(&loaded, "B");
        let c = object_named(&loaded, "C");

        session.apply_input(&loaded, InputId(0)).unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, b));
        assert!(!session.state().has_object(&loaded.game, 0, 0, c));
        assert_eq!(
            session.take_message_events(),
            vec![MessageEvent::Message {
                text: "changed".to_string()
            }]
        );
        assert_eq!(
            session.take_wait_events(),
            vec![WaitEvent::ContinueEffects { milliseconds: 450 }]
        );

        session
            .apply_command(&loaded, "__continue_effects")
            .unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, c));
    }

    #[test]
    fn render_ascii_top_uses_loaded_legend() {
        let source = include_str!("../../../games/spec_2d.puzzle");
        let loaded = parse_game(source).unwrap();

        assert_eq!(
            render_ascii_top(&loaded.levels[0].initial_state, &loaded.legend)
                .lines()
                .count(),
            7
        );
    }

    #[test]
    fn render_ascii_top_uses_the_top_layer_char_for_overlaps() {
        let loaded = parse_game(
            r#"
title overlap_render
puzzle default {
layers {
floor = Floor
target = Goal
solid = Box
}
rules {
}
levels {
legend {
. = empty
F = Floor
G = Goal
B = Box
* = Floor Goal Box
}
*
}
}
"#,
        )
        .unwrap();

        assert_eq!(
            render_ascii_top(&loaded.levels[0].initial_state, &loaded.legend),
            "B"
        );
    }

    #[test]
    fn session_supports_undo_redo_and_restart() {
        let source = include_str!("../../../games/spec_2d.puzzle");
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);
        session.start_level(&loaded, 0);
        let initial = session.state().clone();
        let right = *loaded.controls.keys.get(&b'd').unwrap();

        session.apply_input(&loaded, right).unwrap();
        let moved = session.state().clone();
        assert_ne!(moved, initial);
        assert!(session.can_undo());
        assert!(!session.can_redo());

        session.undo(&loaded);
        assert_eq!(session.state(), &initial);
        assert!(!session.can_undo());
        assert!(session.can_redo());

        session.redo(&loaded);
        assert_eq!(session.state(), &moved);
        assert!(session.can_undo());
        assert!(!session.can_redo());

        session.restart_level(&loaded);
        assert_eq!(session.state(), &initial);
        assert!(session.can_undo());

        session.undo(&loaded);
        assert_eq!(session.state(), &moved);
    }

    #[test]
    fn progress_save_restores_cleared_levels_by_name() {
        let loaded = parse_game(
            r#"
title progress_fixture
puzzle default {
persistent var moves = 0

layers {
__legacy_layer_0 = Player
}
empty .

rules {
}
levels {
legend P = Player

level first
P

level second
P
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let save = ProgressSaveData {
            version: PROGRESS_SAVE_VERSION,
            levels: vec![
                LevelProgressSaveData {
                    name: "second".to_string(),
                    cleared: true,
                },
                LevelProgressSaveData {
                    name: "first".to_string(),
                    cleared: false,
                },
            ],
            current_level: Some("second".to_string()),
            persistent_vars: vec![PersistentVarSaveData {
                name: "moves".to_string(),
                value: 7,
            }],
        };

        session.restore_progress_save_data(&loaded, &save).unwrap();

        assert_eq!(session.cleared_levels(), &[false, true]);
        assert_eq!(session.selected_level_index(), 1);
        assert_eq!(
            session.progress_save_data(&loaded).current_level,
            Some("second".to_string())
        );
        assert_eq!(
            session.progress_save_data(&loaded).persistent_vars,
            vec![PersistentVarSaveData {
                name: "moves".to_string(),
                value: 7,
            }]
        );
        assert_eq!(
            session.progress_save_data(&loaded).levels,
            loaded
                .levels
                .iter()
                .enumerate()
                .map(|(index, level)| LevelProgressSaveData {
                    name: level.name.clone(),
                    cleared: index == 1,
                })
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn spec_2d_goto_level_params_select_progress_level() {
        let source = include_str!("../../../games/spec_2d.puzzle");
        let loaded = parse_game(source).unwrap();
        let cleared_current_level = loaded.levels[1].name.clone();
        let save = ProgressSaveData {
            version: PROGRESS_SAVE_VERSION,
            levels: loaded
                .levels
                .iter()
                .enumerate()
                .map(|(index, level)| LevelProgressSaveData {
                    name: level.name.clone(),
                    cleared: index <= 1,
                })
                .collect(),
            current_level: Some(cleared_current_level.clone()),
            persistent_vars: Vec::new(),
        };
        let mut session = GameSession::new(&loaded);

        session.restore_progress_save_data(&loaded, &save).unwrap();

        assert!(session.cleared_levels()[0]);
        assert!(session.cleared_levels()[1]);
        assert_eq!(
            session.progress_save_data(&loaded).current_level,
            Some(cleared_current_level)
        );

        session
            .apply_command(&loaded, "goto playing(microban.1)")
            .unwrap();

        assert_eq!(session.screen(), "playing");
        assert_eq!(session.level_index(), 0);

        let mut session = GameSession::new(&loaded);
        session.restore_progress_save_data(&loaded, &save).unwrap();
        session.apply_command(&loaded, "goto playing").unwrap();

        assert_eq!(session.screen(), "playing");
        assert_eq!(session.level_index(), 1);
    }

    #[test]
    fn goto_level_param_accepts_dotted_level_name_without_digits() {
        let loaded = parse_game(
            r#"
title dotted_level

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
}
levels {
legend P = Player
level first {
P
}
level test.chain {
P
}
}
}

scene playing(level) {
state {
board = puzzle default
}
layout {
board
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_command(&loaded, "goto playing(test.chain)")
            .unwrap();

        assert_eq!(session.scene(), "playing");
        assert_eq!(session.level_index(), 1);
    }

    #[test]
    fn goto_level_param_rejects_quoted_level_name() {
        let loaded = parse_game(
            r#"
title quoted_level

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
}
levels {
legend P = Player
level microban.1 {
P
}
}
}

scene playing(level) {
state {
board = puzzle default
}
layout {
board
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        let error = session
            .apply_command(&loaded, "goto playing(\"microban.1\")")
            .unwrap_err();

        assert_eq!(
            error,
            TransitionError::InvalidCommand("goto playing(\"microban.1\")".to_string())
        );
    }

    #[test]
    fn game_progress_effects_update_progress_primitives() {
        let loaded = parse_game(
            r#"
title progress_effects

puzzle default {
persistent var score = 5

layers {
__legacy_layer_0 = Player
}
empty .

rules {
}

levels {
legend P = Player

level first
P

level second
P
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_command(&loaded, "set current_level = second")
            .unwrap();
        session
            .apply_command(&loaded, "set level(second).cleared = true")
            .unwrap();
        assert_eq!(session.selected_level_index(), 1);
        assert_eq!(session.cleared_levels(), &[false, true]);

        session.apply_command(&loaded, "reset score").unwrap();
        assert_eq!(
            session.progress_save_data(&loaded).persistent_vars,
            vec![PersistentVarSaveData {
                name: "score".to_string(),
                value: 5,
            }]
        );

        session
            .apply_command(&loaded, "clear_game_progress")
            .unwrap();
        assert_eq!(session.selected_level_index(), 0);
        assert_eq!(session.cleared_levels(), &[false, false]);
        assert_eq!(
            session.progress_save_data(&loaded).persistent_vars,
            vec![PersistentVarSaveData {
                name: "score".to_string(),
                value: 5,
            }]
        );
    }

    #[test]
    fn default_actions_work_from_playing() {
        let source = include_str!("../../../games/spec_2d.puzzle");
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();
        let initial = session.state().clone();

        session.apply_command(&loaded, "right").unwrap();
        let moved = session.state().clone();
        assert_ne!(moved, initial);

        session.apply_command(&loaded, "undo").unwrap();
        assert_eq!(session.state(), &initial);

        session.apply_command(&loaded, "redo").unwrap();
        assert_eq!(session.state(), &moved);

        session.apply_command(&loaded, "playing.restart").unwrap();
        assert_eq!(session.state(), &initial);
        assert_eq!(session.screen(), "playing");

        session.apply_command(&loaded, "undo").unwrap();
        assert_eq!(session.state(), &moved);
    }

    #[test]
    fn puzzle_next_level_restarts_the_target_scene_not_the_initial_scene() {
        let loaded = parse_game(
            r#"
title next_level_target_scene

puzzle board {
layers {
floor = Goal
actor = Box Player
}


input tick

win_conditions {
some Goal
all Goal on Box
}

rules {
}

levels {
legend {
. = empty
* = Goal Box
P = Player
}
level one {
*
P
}

level two {
P
}
}
}

scene title {
title
button "Play" -> goto playing
rules {
}
}

scene playing {
state {
board = puzzle board
}
layout {
board
}
rules {
step board
if board.win_conditions -> board.next_level
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let tick = input_named(&loaded, "tick");

        session.apply_command(&loaded, "goto playing").unwrap();
        session.apply_input(&loaded, tick).unwrap();

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.screen(), "board");
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn persistent_vars_survive_undo_and_clear_undo_history_cuts_undo() {
        let loaded = parse_game(
            r#"
title persistent_history

puzzle default {
persistent var cleared = false

layers {
__legacy_layer_0 = Player
}
empty .

rules {
if input == right {
once right [ Player | no Player ] -> [ | Player ] set cleared = true
}
}

levels {
legend P = Player

level start {
P.
}
}
}

scene playing {
state {
board = puzzle default
}
layout {
board
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let player = object_named(&loaded, "Player");

        session.apply_command(&loaded, "right").unwrap();
        assert_eq!(session.state().visible_globals(), &[1]);
        assert!(session.state().has_object(&loaded.game, 1, 0, player));
        assert!(session.can_undo());

        session.apply_command(&loaded, "undo").unwrap();
        assert_eq!(session.state().visible_globals(), &[1]);
        assert!(session.state().has_object(&loaded.game, 0, 0, player));

        session.apply_command(&loaded, "right").unwrap();
        assert!(session.can_undo());
        session
            .apply_command(&loaded, "clear_undo_history")
            .unwrap();
        assert!(!session.can_undo());
        assert!(!session.can_redo());
        assert_eq!(session.state().visible_globals(), &[1]);
    }

    #[test]
    fn puzzle_load_and_reset_do_not_depend_on_initial_scene_or_playing_name() {
        let source = r#"
title puzzle_load_reset

puzzle default {
layers {
__legacy_layer_1 = Player
}
empty .

rules {
once right [ Player | ] -> [ | Player ]
}

levels {
legend P = Player

level first {
P.
}

level second {
P..
}
}
}

scene title {
keys {
Enter -> input begin
}
rules {
}
}

scene play {
state {
board = puzzle default
}
layout {
board
}
keys {
d -> input right
r -> board.restart
}
rules {
step board
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);
        let player = object_named(&loaded, "Player");

        assert_eq!(session.screen(), "title");
        assert_eq!(loaded.scenes[0].transitions.len(), 0);

        session
            .apply_command(&loaded, "load play.board from play.board.levels[1]")
            .unwrap();
        session.apply_command(&loaded, "goto play").unwrap();
        assert_eq!(session.screen(), "play");
        assert_eq!(session.level_index(), 1);

        session.apply_command(&loaded, "right").unwrap();
        assert!(
            session
                .scene_state()
                .unwrap()
                .puzzles
                .get("board")
                .unwrap()
                .has_object(&loaded.game, 1, 0, player)
        );

        session.apply_command(&loaded, "board.restart").unwrap();
        assert_eq!(session.screen(), "play");
        assert!(
            session
                .scene_state()
                .unwrap()
                .puzzles
                .get("board")
                .unwrap()
                .has_object(&loaded.game, 0, 0, player)
        );

        session.apply_command(&loaded, "undo").unwrap();
        assert!(
            session
                .scene_state()
                .unwrap()
                .puzzles
                .get("board")
                .unwrap()
                .has_object(&loaded.game, 1, 0, player)
        );
    }

    #[test]
    fn cancelled_input_does_not_apply_screen_condition_transitions() {
        let loaded = parse_game(
            r#"
title cancel_screen_transition

puzzle default {
layers {
__legacy_layer_1 = A
}
empty .


win_conditions {
some A
}

input tick

rules {
once [ A ] -> cancel
}

levels {
legend A = A

level start {
A
}
}
}

scene playing {
state {
board = puzzle default
}
layout {
board
}
rules {
step board
if board.win_conditions -> goto level_clear
}
}

scene level_clear {
layout {
text "clear"
}
}
"#,
        )
        .unwrap();
        let tick = input_named(&loaded, "tick");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, tick).unwrap();

        assert_eq!(session.screen(), "playing");
    }

    #[test]
    fn puzzle_transition_only_runs_on_scenes_that_enable_main() {
        let source = include_str!("../../../games/spec_2d.puzzle");
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();
        let initial = session.state().clone();

        session.apply_command(&loaded, "goto menu").unwrap();
        assert_eq!(session.screen(), "menu");

        session.apply_command(&loaded, "right").unwrap();
        assert_eq!(session.state(), &initial);
    }

    #[test]
    fn screen_local_puzzle_persists_across_level_entry() {
        let loaded = scene_local_puzzle_fixture();
        let mut session = GameSession::new(&loaded);
        let player = object_named(&loaded, "Player");

        enter_scene_local_hub(&loaded, &mut session);

        session
            .apply_screen_effect(
                &loaded,
                &SceneEffect::Apply {
                    rule: "down".to_string(),
                    args: Vec::new(),
                    target: Some("hub.board".to_string()),
                },
                &HashMap::new(),
            )
            .unwrap();
        assert!(
            session
                .scene_state()
                .unwrap()
                .puzzles
                .get("board")
                .unwrap()
                .has_object(&loaded.game, 1, 2, player)
        );

        session.apply_command(&loaded, "goto child_1").unwrap();
        assert_eq!(session.screen(), "child_1");

        session.apply_command(&loaded, "goto hub").unwrap();
        assert!(
            session
                .scene_state()
                .unwrap()
                .puzzles
                .get("board")
                .unwrap()
                .has_object(&loaded.game, 1, 2, player)
        );
    }

    #[test]
    fn scene_hub_portal_enters_child_scene() {
        let loaded = scene_local_puzzle_fixture();
        let mut session = GameSession::new(&loaded);

        enter_scene_local_hub(&loaded, &mut session);

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();
        assert_eq!(session.screen(), "child_1");
    }

    #[test]
    fn scene_hub_reset_restores_checkpoint() {
        let loaded = scene_local_puzzle_fixture();
        let mut session = GameSession::new(&loaded);
        let player = object_named(&loaded, "Player");

        enter_scene_local_hub(&loaded, &mut session);
        load_named_scene_level(&loaded, &mut session, "checkpoint.spec_board", "spec.hub");
        session
            .apply_input(&loaded, input_named(&loaded, "down"))
            .unwrap();
        assert!(
            session
                .scene_state()
                .unwrap()
                .puzzles
                .get("spec_board")
                .unwrap()
                .has_object(&loaded.game, 1, 2, player)
        );

        session
            .apply_screen_effect(
                &loaded,
                &SceneEffect::Copy {
                    source: "checkpoint.spec_board".to_string(),
                    target: "hub.spec_board".to_string(),
                },
                &HashMap::new(),
            )
            .unwrap();
        assert!(
            session
                .scene_state()
                .unwrap()
                .puzzles
                .get("spec_board")
                .unwrap()
                .has_object(&loaded.game, 1, 1, player)
        );
    }

    #[test]
    fn sequence_effect_can_update_saved_puzzle_then_return() {
        let loaded = parse_game(
            r#"
title sequence_saved_puzzle

puzzle default {
var marks = 0

layers {
actor = Player
}


input mark
input done

rules {
if input == mark {
once [ Player ] -> [ Player ] marks += 1
}
}

levels {
legend {
. = empty
P = Player
}

level hub {
P
}

level level {
P
}
}
}

scene hub {
state {
board = puzzle default level hub
}
layout {
board
}
rules {
step board
}
}

scene playing {
state {
board = puzzle default
}
layout {
board
}
rules {
if input == done -> {
apply mark to hub.board
goto hub
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_command(&loaded, "playing.goto level")
            .unwrap();
        session
            .apply_input(&loaded, input_named(&loaded, "done"))
            .unwrap();

        let hub_board = session.scene_state().unwrap().puzzles.get("board").unwrap();
        assert_eq!(session.screen(), "hub");
        assert_eq!(hub_board.visible_globals(), &[1]);
    }

    #[test]
    fn session_advances_level_after_nonfinal_clear() {
        let loaded = transition_fixture();
        let mut session = GameSession::new(&loaded);

        for key in "ddd".bytes() {
            let input = *loaded.controls.keys.get(&key).unwrap();
            let label = loaded.input_labels.get(&input).unwrap();
            session.apply_command(&loaded, label).unwrap();
        }

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
        assert_eq!(session.screen(), "playing");
        assert!(!session.can_undo());
        assert!(!session.can_redo());
    }

    #[test]
    fn direct_input_applies_screen_condition_transitions() {
        let loaded = transition_fixture();
        let mut session = GameSession::new(&loaded);

        for key in "ddd".bytes() {
            let input = *loaded.controls.keys.get(&key).unwrap();
            session.apply_input(&loaded, input).unwrap();
        }

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
        assert_eq!(session.screen(), "playing");
    }

    #[test]
    fn session_goes_to_level_select_after_final_clear() {
        let loaded = transition_fixture();
        let mut session = GameSession::new(&loaded);
        let final_level = loaded.levels.len() - 1;
        session.start_level(&loaded, final_level);

        for key in "ddd".bytes() {
            let input = *loaded.controls.keys.get(&key).unwrap();
            let label = loaded.input_labels.get(&input).unwrap();
            session.apply_command(&loaded, label).unwrap();
        }

        assert_eq!(session.level_index(), final_level);
        assert!(loaded.is_condition_true("win_conditions", session.state()));
        assert_eq!(session.screen(), "level_select");
    }

    #[test]
    fn level_clear_hook_runs_before_condition_transition() {
        let loaded = parse_game(
            r#"
title level_clear_hook
puzzle sokoban {
layers {
floor = Goal
actor = Player Box Wall
marker = ClearMark
@visual = display @ClearVisual
}
group solid = Player Box Wall
win_conditions {
some Goal
all Goal on Box
}
on_level_clear {
[ Goal Box no ClearMark ] -> [ Goal Box ClearMark ]
display [ Goal Box no @ClearVisual ] -> [ Goal Box @ClearVisual ]
}
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
level start {
#####
#PBG#
#####
}
}
}
scene playing {
state {
board = puzzle sokoban
}
layout {
board
}
rules {
step board
if board.win_conditions -> goto level_select
}
}
scene level_select {
layout {
text "done"
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let clear_mark = object_named(&loaded, "ClearMark");
        let clear_visual = object_named(&loaded, "@ClearVisual");

        session.apply_input(&loaded, right).unwrap();

        assert_eq!(session.screen(), "level_select");
        assert!(session.state().has_object(&loaded.game, 3, 1, clear_mark));
        assert!(session.state().has_object(&loaded.game, 3, 1, clear_visual));
    }

    #[test]
    fn wait_before_next_level_exposes_cleared_snapshot_until_continuation() {
        let loaded = parse_game(
            r#"
title wait_clear_snapshot
puzzle sokoban {
layers {
floor = Goal
actor = Player Box Wall
}
group solid = Player Box Wall
win_conditions {
some Goal
all Goal on Box
}
on_level_clear {
wait 1s
next_level
}
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
* = Goal Box
}
level first {
#####
#PBG#
#####
}
level second {
#####
#P.G#
#####
}
}
}
scene playing {
state {
board = puzzle sokoban
}
layout {
board
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let right = input_named(&loaded, "right");

        session.apply_input(&loaded, right).unwrap();

        assert_eq!(session.level_index(), 0);
        assert!(loaded.is_condition_true("win_conditions", session.state()));
        assert_eq!(
            session.take_wait_events(),
            vec![WaitEvent::ContinueEffects { milliseconds: 1000 }]
        );

        session
            .apply_command(&loaded, "__continue_effects")
            .unwrap();

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn wait_statement_splits_rules_into_observable_segments() {
        let loaded = parse_game(
            r#"
title wait_statement_segments
puzzle default {
layers {
__legacy_layer_0 = A B C
}
empty .
rules {
[ A ] -> [ B ]
wait 1s
[ B ] -> [ C ]
}
levels {
legend {
A = A
B = B
C = C
}
level start {
A
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let b = object_named(&loaded, "B");
        let c = object_named(&loaded, "C");

        session.apply_input(&loaded, InputId(0)).unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, b));
        assert!(!session.state().has_object(&loaded.game, 0, 0, c));
        assert_eq!(
            session.take_wait_events(),
            vec![WaitEvent::ContinueEffects { milliseconds: 1000 }]
        );

        session
            .apply_command(&loaded, "__continue_effects")
            .unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, c));
    }

    #[test]
    fn screen_transition_can_goto_level_with_payload() {
        let source = include_str!("../../../games/spec_2d.puzzle");
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();

        session.apply_command(&loaded, "goto level_select").unwrap();
        assert_eq!(session.screen(), "level_select");

        session.apply_command(&loaded, "select:1").unwrap();

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.screen(), "playing");
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn scene_level_commands_can_target_level_scene() {
        let loaded = parse_game(
            r#"
title scene_level_commands
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
}
levels {
legend P = Player
level first {
P
}
level second {
P
}
level third {
P
}
}
}

scene playing {
state {
board = puzzle default
}
layout {
board
}
}

scene level_clear {
state {
board = puzzle default
}
layout {
board
}
rules {
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_command(&loaded, "goto level_clear").unwrap();
        session
            .apply_command(&loaded, "playing.next_level")
            .unwrap();
        assert_eq!(session.level_index(), 1);
        assert_eq!(session.screen(), "playing");

        session.apply_command(&loaded, "goto level_clear").unwrap();
        session
            .apply_command(&loaded, "playing.previous_level")
            .unwrap();
        assert_eq!(session.level_index(), 0);
        assert_eq!(session.screen(), "playing");

        session.apply_command(&loaded, "goto level_clear").unwrap();
        session
            .apply_command(&loaded, "playing.goto third")
            .unwrap();
        assert_eq!(session.level_index(), 2);
        assert_eq!(session.screen(), "playing");
    }

    #[test]
    fn level_menu_component_owns_level_menu_commands() {
        let source = r#"
title level_menu_commands

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level first {
P
}
level second {
P
}
}
}

scene playing {
state {
board = puzzle default
}
layout {
board
}
}

scene select {
layout {
level_menu
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_command(&loaded, "goto select").unwrap();
        session.apply_command(&loaded, "down").unwrap();
        assert_eq!(session.selected_level_index(), 1);

        session.apply_command(&loaded, "select").unwrap();
        assert_eq!(session.level_index(), 1);
        assert_eq!(session.screen(), "playing");
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn scene_level_resources_filter_level_menu_and_advance() {
        let source = r#"
title scene_level_resources

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .


rules {
}
}

levels worldA of default {
legend P = Player
level 1
P
level 2
P
}

levels worldB of default {
legend P = Player
level 1
P
level 2
P
}

scene level_select {
resources {
levels worldB
}
layout {
level_menu
}
}

scene playing {
resources {
levels worldB
}
state {
board = puzzle default
}
layout {
board
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_command(&loaded, "select").unwrap();
        assert_eq!(session.level_index(), 2);
        assert_eq!(loaded.levels[session.level_index()].name, "worldB.1");

        session.advance_level(&loaded);
        assert_eq!(session.level_index(), 3);
        assert_eq!(loaded.levels[session.level_index()].name, "worldB.2");

        session.advance_level(&loaded);
        assert_eq!(session.level_index(), 3);
    }

    #[test]
    fn level_menu_matrix_navigation_uses_columns() {
        let source = r#"
title level_menu_matrix

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level first {
P
}
level second {
P
}
level third {
P
}
level fourth {
P
}
level fifth {
P
}
}
}

scene playing {
state {
board = puzzle default
}
layout {
board
}
}

scene select {
layout {
level_menu {
columns = 3
wrap = true
}
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_command(&loaded, "goto select").unwrap();
        session.apply_command(&loaded, "right").unwrap();
        assert_eq!(session.selected_level_index(), 1);

        session.apply_command(&loaded, "down").unwrap();
        assert_eq!(session.selected_level_index(), 4);

        session.apply_command(&loaded, "down").unwrap();
        assert_eq!(session.selected_level_index(), 2);

        session.apply_command(&loaded, "left").unwrap();
        assert_eq!(session.selected_level_index(), 1);
    }

    #[test]
    fn level_menu_select_starts_selected_level() {
        let source = r#"
title level_menu_default_select

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level first {
P
}
level second {
P
}
}
}

scene playing {
state {
board = puzzle default
}
layout {
board
}
}

scene select {
layout {
level_menu
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_command(&loaded, "goto select").unwrap();
        session.apply_command(&loaded, "down").unwrap();
        session.apply_command(&loaded, "select").unwrap();

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.screen(), "playing");
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn level_menu_buttons_share_level_menu_cursor() {
        let source = r#"
title level_menu_buttons

puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player
level first {
P
}
level second {
P
}
}
}

scene playing {
state {
board = puzzle default
}
layout {
board
}
}

scene select {
layout {
level_menu {
button "Title" -> goto title
}
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_command(&loaded, "goto select").unwrap();
        session.apply_command(&loaded, "down").unwrap();
        session.apply_command(&loaded, "down").unwrap();
        assert_eq!(session.selected_level_index(), 1);

        session.apply_command(&loaded, "select").unwrap();
        assert_eq!(session.screen(), "playing");
    }

    #[test]
    fn goto_preserves_fixed_scene_state_without_history_stack() {
        let source = r#"
title screen_history
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level start {
P
}
}
}

scene a {
layout {
text "A"
}
}

scene b {
state {
mark = empty
}
layout {
text "B"
}
}

scene c {
state {
mark = empty
}
layout {
text "C"
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_command(&loaded, "goto b with mark = first")
            .unwrap();
        assert_eq!(session.screen(), "b");

        session.apply_command(&loaded, "goto a").unwrap();
        session.apply_command(&loaded, "goto c").unwrap();
        session.apply_command(&loaded, "goto a").unwrap();
        session.apply_command(&loaded, "goto b").unwrap();

        assert_eq!(session.screen(), "b");
        assert_eq!(
            session
                .scene_state()
                .and_then(|state| state.values.get("mark")),
            Some(&SceneValue::Symbol("first".to_string()))
        );
    }

    #[test]
    fn persistent_scene_var_survives_scene_reset() {
        let source = r#"
title persistent_scene_var
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level start {
P
}
}
}

scene playing {
layout {
text "Playing"
}
}

scene menu {
var transient = empty
persistent var tab = settings
layout {
text tab
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_command(&loaded, "goto menu with transient = changed")
            .unwrap();
        session.apply_command(&loaded, "start menu").unwrap();

        assert_eq!(
            session
                .scene_state()
                .and_then(|state| state.values.get("tab")),
            Some(&SceneValue::Symbol("settings".to_string()))
        );
        assert_eq!(
            session
                .scene_state()
                .and_then(|state| state.values.get("transient")),
            Some(&SceneValue::Symbol("empty".to_string()))
        );
    }

    #[test]
    fn runtime_applies_scene_params_without_overwriting_consts() {
        let source = r#"
title scene_param_rejection
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level start {
P
}
}
}

scene playing {
layout {
text "Playing"
}
}

scene menu {
const tab = levels
layout {
text tab
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_command(&loaded, "goto menu with tab = settings")
            .unwrap();
        assert_eq!(session.screen(), "menu");
        assert_eq!(
            session
                .scene_state()
                .and_then(|state| state.values.get("tab")),
            Some(&SceneValue::Symbol("levels".to_string()))
        );
    }

    #[test]
    fn runtime_scene_words_keep_goto_and_start_only() {
        let source = r#"
title scene_state_words
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level first {
P
}
}
}

scene title {
layout {
text "Title"
}
}

scene playing {
var selected = empty
layout {
text selected
}
}

scene menu {
layout {
text "Menu"
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_command(&loaded, "goto playing with selected = first")
            .unwrap();
        assert_eq!(session.screen(), "playing");
        assert_eq!(
            session
                .scene_state()
                .and_then(|state| state.values.get("selected")),
            Some(&SceneValue::LevelRef(0))
        );

        for old in ["resume playing", "open menu", "enter menu", "back", "close"] {
            session.apply_command(&loaded, old).unwrap();
            assert_eq!(
                session.screen(),
                "playing",
                "{old} should not navigate scenes"
            );
        }

        session.apply_command(&loaded, "start playing").unwrap();
        assert_eq!(
            session
                .scene_state()
                .and_then(|state| state.values.get("selected")),
            Some(&SceneValue::Symbol("empty".to_string()))
        );
    }

    #[test]
    fn scene_params_keep_level_refs_and_fields_are_type_checked() {
        let source = r#"
title level_ref_params
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level first {
P
}

level second {
P
}
}
}

scene detail(selected) {
layout {
text selected.solved
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_command(&loaded, "set level(first).cleared = true")
            .unwrap();
        session
            .apply_command(&loaded, "goto detail with selected = first")
            .unwrap();

        assert_eq!(
            session
                .scene_state()
                .and_then(|state| state.values.get("selected")),
            Some(&SceneValue::LevelRef(0))
        );
        assert_eq!(
            session
                .scene_path_value(&loaded, "selected.solved")
                .map(|value| scene_value_to_string(&value)),
            Some("true".to_string())
        );
        assert_eq!(
            session
                .scene_path_value(&loaded, "selected.name")
                .map(|value| scene_value_to_string(&value)),
            Some("first".to_string())
        );
    }

    #[test]
    fn scene_visibility_and_focus_are_separate() {
        let source = r#"
title screen_focus
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player
level start {
P
}
}
}

scene playing {
state {
board = puzzle default
}
layout {
board
}
}

scene menu {
layout {
text "Menu"
}
}

scene level_select {
layout {
text "Levels"
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        assert_eq!(session.screen(), "playing");
        assert_eq!(session.visible_scenes(), &["playing".to_string()]);

        session.apply_command(&loaded, "goto menu").unwrap();
        assert_eq!(session.screen(), "menu");
        assert_eq!(session.visible_scenes(), &["menu".to_string()]);

        session.apply_command(&loaded, "goto level_select").unwrap();
        assert_eq!(session.screen(), "level_select");
        assert_eq!(session.visible_scenes(), &["level_select".to_string()]);
    }

    #[test]
    fn session_initializes_screen_local_state() {
        let source = r#"
title scene_state
puzzle default {
layers {
actor = Player
}

rules {
}

levels {
legend {
. = empty
P = Player
}
level start {
P
}
}
}

scene playing {
state {
board = puzzle default
message_visible = true
moves = 0
message = "Read this"
}
layout {
board
}
keys {
q -> goto level_select
}
}

scene level_select {
state {
message = "Browse"
}
layout {
level_menu
}
keys {
Escape -> goto playing
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);

        assert_eq!(
            session
                .scene_state()
                .and_then(|state| state.values.get("message")),
            Some(&SceneValue::Text("Read this".to_string()))
        );
        assert_eq!(
            session
                .scene_state()
                .and_then(|state| state.puzzles.get("board"))
                .map(|puzzle| &puzzle.state),
            Some(&loaded.levels[0].initial_state)
        );

        session.apply_command(&loaded, "goto level_select").unwrap();
        assert_eq!(session.screen(), "level_select");
        assert_eq!(
            session
                .scene_state()
                .and_then(|state| state.values.get("message")),
            Some(&SceneValue::Text("Browse".to_string()))
        );

        session.apply_command(&loaded, "goto playing").unwrap();
        assert_eq!(session.screen(), "playing");
        assert_eq!(
            session
                .scene_state()
                .and_then(|state| state.values.get("message_visible")),
            Some(&SceneValue::Bool(true))
        );
    }

    #[test]
    fn puzzle_rule_effect_can_advance_to_next_level() {
        let loaded = parse_game(
            r#"
title rule_next_level

puzzle board {
layers {
floor = Goal
actor = Box Player
}


input tick

rules {
[ Goal Box ] -> next_level
}

levels {
legend {
. = empty
* = Goal Box
P = Player
}
level one {
*
P
}

level two {
P
}
}
}

scene playing {
state {
board = puzzle board
}
layout {
board
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let tick = input_named(&loaded, "tick");

        session.apply_command(&loaded, "goto playing").unwrap();
        session.apply_input(&loaded, tick).unwrap();

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.screen(), "board");
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn puzzle_condition_effect_can_advance_to_next_level() {
        let loaded = parse_game(
            r#"
title condition_next_level

puzzle board {
layers {
floor = Goal
actor = Box Player
}


input tick

win_conditions {
some Goal
all Goal on Box
}

rules {
if win_conditions -> next_level
}

levels {
legend {
. = empty
* = Goal Box
P = Player
}
level one {
*
P
}

level two {
P
}
}
}

scene playing {
state {
board = puzzle board
}
layout {
board
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let tick = input_named(&loaded, "tick");

        session.apply_command(&loaded, "goto playing").unwrap();
        session.apply_input(&loaded, tick).unwrap();

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.screen(), "board");
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn level_start_runs_at_runtime_without_baking_initial_state() {
        let loaded = parse_game(
            r#"
title runtime_level_start

puzzle board {
layers {
__legacy_layer_0 = Player
__legacy_layer_1 = Started
}
empty .


input tick

on_level_start {
once [ Player no Started ] -> [ Player Started ]
message "started"
}

rules {
}

levels {
legend {
P = Player
S = Started
}

level one {
P
}
}
}

scene playing {
state {
board = puzzle board
}
layout {
board
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let started = object_named(&loaded, "Started");

        assert!(
            !loaded.levels[0]
                .initial_state
                .has_object(&loaded.game, 0, 0, started)
        );

        let mut session = GameSession::new(&loaded);

        assert!(session.state().has_object(&loaded.game, 0, 0, started));
        assert_eq!(
            session.take_message_events(),
            vec![MessageEvent::Message {
                text: "started".to_string()
            }]
        );
    }

    #[test]
    fn level_body_message_sugar_runs_on_runtime_lifecycle() {
        let loaded = parse_game(
            r#"
title level_message_sugar

puzzle board {
layers {
__legacy_layer_0 = Player
}
empty .


input tick

win_conditions {
some Player
}

rules {
}

levels {
legend {
P = Player
}

level one {
message "enter one"
P
message "clear one"
}
}
}

scene playing {
state {
board = puzzle board
}
layout {
board
}
rules {
step board
}
}
"#,
        )
        .unwrap();

        assert!(loaded.levels[0].level_start_program.is_some());
        assert!(loaded.levels[0].level_clear_program.is_some());

        let mut session = GameSession::new(&loaded);
        assert_eq!(
            session.take_message_events(),
            vec![MessageEvent::Message {
                text: "enter one".to_string()
            }]
        );

        let tick = input_named(&loaded, "tick");
        session.apply_input(&loaded, tick).unwrap();

        assert_eq!(
            session.take_message_events(),
            vec![MessageEvent::Message {
                text: "clear one".to_string()
            }]
        );
    }

    #[test]
    fn title_scene_does_not_start_first_level_lifecycle() {
        let loaded = parse_game(
            r#"
title title_level_start_boundary

puzzle board {
layers {
__legacy_layer_0 = Player
}
empty .


input tick

rules {
}

levels {
legend {
P = Player
}

level one {
message "enter one"
P
}
}
}

scene title {
button "Play" -> goto playing
}

scene playing {
state {
board = puzzle board
}
layout {
board
}
rules {
step board
}
}
"#,
        )
        .unwrap();

        let mut session = GameSession::new(&loaded);
        assert_eq!(session.scene(), "title");
        assert_eq!(session.active_level_index(), None);
        assert!(session.take_message_events().is_empty());

        session.apply_command(&loaded, "goto playing").unwrap();
        assert_eq!(session.scene(), "playing");
        assert_eq!(session.active_level_index(), Some(0));
        assert_eq!(
            session.take_message_events(),
            vec![MessageEvent::Message {
                text: "enter one".to_string()
            }]
        );
    }

    #[test]
    fn rule_next_level_still_runs_model_clear_and_scene_conditions() {
        let loaded = parse_game(
            r#"
title rule_next_level_turn_completion

puzzle board {
persistent var clear_seen = false

layers {
floor = Goal
actor = Box Player
}


input tick

win_conditions {
some Goal
all Goal on Box
}

on_level_clear {
once [ Goal Box ] -> set clear_seen = true
}

rules {
if win_conditions -> next_level
}

levels {
legend {
. = empty
* = Goal Box
P = Player
}
level one {
*
P
}

level two {
P
}
}
}

scene playing {
state {
board = puzzle board
}
layout {
board
}
rules {
step board
if board.win_conditions -> message "clear"
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let tick = input_named(&loaded, "tick");

        session.apply_input(&loaded, tick).unwrap();

        assert_eq!(session.level_index(), 1);
        assert!(session.cleared_levels()[0]);
        assert_eq!(
            session.take_message_events(),
            vec![MessageEvent::Message {
                text: "clear".to_string()
            }]
        );
        assert_eq!(session.state().visible_globals(), &[1]);
    }
}
