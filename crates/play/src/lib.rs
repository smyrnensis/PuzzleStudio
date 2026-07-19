use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};

use puzzle_core::{
    GridExecutableProgram, GridRuleFiring, GridRuleFiringSummary, GridSize, GridState,
    GridTransitionError, InputId, LayerId, ObjectId, PatchOp, ProgramContinuation, RuleId, Size2,
    State as PuzzleState, TransitionCommand, transition_program_continuation_segment_trace,
    transition_program_segment_trace, transition_program_sequence_outcome,
    transition_program_sequence_summary_outcome, transition_program_sequence_without_input_outcome,
    transition_program_sequence_without_input_summary_outcome,
};
use puzzle_lang::{
    AsciiLegend, LevelMenuDef, LoadedDocument, LoadedDocumentModel, LoadedGame, LoadedGridGame,
    LoadedGridLevel, ModelOperationSound, ResourceSelection, RuleAnimation, RuleAnimationTrigger,
    RuleEffect, SceneBinaryOp, SceneComponent, SceneEffect, SceneEffectParam, SceneExpr,
    SceneLevelKey, ScenePuzzleInitializer, SceneTransitionTrigger, SceneValue, SceneVarKind,
    parse_scene_effect_params, parse_scene_expression,
};
use puzzle_runtime_contract::{
    RuntimeAnimationEvent, RuntimeCoord, RuntimePresentationEvent, RuntimePresentationEventKind,
};

mod runtime_sounds;
mod session_history;

pub use runtime_sounds::{
    RuntimeMusicSoundDef, RuntimeSfxSoundDef, RuntimeSoundsDef, runtime_sounds_def,
};

use session_history::SessionHistory;

#[cfg(test)]
use puzzle_core::TransitionError;

pub fn loaded_document_scene_host_loaded_game(
    document: &LoadedDocument,
) -> Result<LoadedGame, String> {
    let [LoadedDocumentModel::Puzzle2d { game, .. }] = document.models.as_slice() else {
        return Err(
            "a document with multiple puzzle worlds requires the document session runtime"
                .to_string(),
        );
    };
    Ok(game.clone())
}

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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnimationEvent {
    Move {
        name: String,
        object: ObjectId,
        from_object: Option<ObjectId>,
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
        z: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationEventKind {
    Sound(SoundEvent),
    Message(MessageEvent),
    Wait(WaitEvent),
    Animation(AnimationEvent),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationEvent {
    pub context: PresentationContext,
    pub kind: PresentationEventKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationContext {
    pub scene: String,
    pub puzzle: String,
    pub level_index: Option<usize>,
}

pub fn animation_events_contract<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    events: &[AnimationEvent],
) -> Vec<RuntimeAnimationEvent> {
    let runtime_z = |z| (D > 2).then_some(z);
    events
        .iter()
        .map(|event| match event {
            AnimationEvent::Move {
                name,
                object,
                from_object,
                from_x,
                from_y,
                from_z,
                to_x,
                to_y,
                to_z,
            } => RuntimeAnimationEvent::Move {
                name: name.clone(),
                object_id: object.0,
                from_object: from_object.map(|object| loaded.object_name(object).to_string()),
                from: RuntimeCoord {
                    x: *from_x,
                    y: *from_y,
                    z: runtime_z(*from_z),
                },
                to: RuntimeCoord {
                    x: *to_x,
                    y: *to_y,
                    z: runtime_z(*to_z),
                },
            },
            AnimationEvent::CantMove {
                name,
                object,
                x,
                y,
                z,
            } => RuntimeAnimationEvent::CantMove {
                name: name.clone(),
                object_id: object.0,
                position: RuntimeCoord {
                    x: *x,
                    y: *y,
                    z: runtime_z(*z),
                },
            },
        })
        .collect()
}

pub fn presentation_events_contract<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    events: &[PresentationEvent],
) -> Vec<RuntimePresentationEvent> {
    events
        .iter()
        .map(|event| {
            let event_kind = match &event.kind {
                PresentationEventKind::Sound(SoundEvent::PlaySfx { name }) => {
                    RuntimePresentationEventKind::PlaySfx { name: name.clone() }
                }
                PresentationEventKind::Sound(SoundEvent::PlayMusic { name }) => {
                    RuntimePresentationEventKind::PlayMusic { name: name.clone() }
                }
                PresentationEventKind::Sound(SoundEvent::PauseMusic { name }) => {
                    RuntimePresentationEventKind::PauseMusic { name: name.clone() }
                }
                PresentationEventKind::Sound(SoundEvent::ResumeMusic { name }) => {
                    RuntimePresentationEventKind::ResumeMusic { name: name.clone() }
                }
                PresentationEventKind::Sound(SoundEvent::StopMusic { name }) => {
                    RuntimePresentationEventKind::StopMusic { name: name.clone() }
                }
                PresentationEventKind::Message(MessageEvent::Message { text }) => {
                    RuntimePresentationEventKind::Message { text: text.clone() }
                }
                PresentationEventKind::Wait(WaitEvent::Wait { milliseconds }) => {
                    RuntimePresentationEventKind::Wait {
                        milliseconds: *milliseconds,
                    }
                }
                PresentationEventKind::Animation(animation) => {
                    RuntimePresentationEventKind::Animation {
                        animation: animation_events_contract(
                            loaded,
                            std::slice::from_ref(animation),
                        )
                        .into_iter()
                        .next()
                        .expect("one animation event must serialize as one runtime event"),
                    }
                }
            };
            RuntimePresentationEvent {
                scene: event.context.scene.clone(),
                puzzle: event.context.puzzle.clone(),
                level_index: event.context.level_index,
                event: event_kind,
            }
        })
        .collect()
}

pub fn animation_events_contract_2d(
    loaded: &LoadedGame,
    events: &[AnimationEvent],
) -> Vec<RuntimeAnimationEvent> {
    animation_events_contract(loaded, events)
}

#[derive(Clone, Debug)]
pub struct GridSceneRuntimeState<const D: usize, Size: GridSize<D>> {
    pub values: HashMap<String, SceneValue>,
    pub puzzles: HashMap<String, GridWorldInstanceState<D, Size>>,
}

impl<const D: usize, Size: GridSize<D>> Default for GridSceneRuntimeState<D, Size> {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
            puzzles: HashMap::new(),
        }
    }
}

pub type SceneRuntimeState = GridSceneRuntimeState<2, Size2>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorldInstanceId {
    pub scene: String,
    pub puzzle: String,
}

#[derive(Clone, Debug)]
pub struct GridWorldInstanceState<const D: usize, Size: GridSize<D>> {
    pub model: String,
    pub state: GridState<D, Size>,
    pub initial_state: GridState<D, Size>,
    pub checkpoint_state: Option<GridState<D, Size>>,
    pub active_level_index: Option<usize>,
    level_started: bool,
    history: SessionHistory<GridState<D, Size>>,
    cleared_levels: Vec<bool>,
    selected_level_index: usize,
    level_initial_state_override: Option<GridLevelInitialStateOverride<D, Size>>,
    persistent_vars: Vec<i64>,
    last_transition_trace: Option<GridTransitionTrace<D>>,
}

pub type WorldInstanceState = GridWorldInstanceState<2, Size2>;

#[derive(Clone, Debug)]
struct GridLevelInitialStateOverride<const D: usize, Size: GridSize<D>> {
    state: GridState<D, Size>,
}

impl<const D: usize, Size: GridSize<D>> Deref for GridWorldInstanceState<D, Size> {
    type Target = GridState<D, Size>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

#[derive(Clone, Debug, Default)]
struct GridModelInputResult<const D: usize> {
    world: Option<WorldInstanceId>,
    undo_base_len: usize,
    cancelled: bool,
    items: Vec<QueuedTurnItem>,
    transition_trace: Option<GridTransitionTrace<D>>,
}

#[derive(Clone, Debug)]
pub struct GridTransitionTrace<const D: usize> {
    pub input: InputId,
    pub target: Option<String>,
    pub progressed: bool,
    pub observable: bool,
    pub cancelled: bool,
    pub commands: Vec<TransitionCommand>,
    pub firings: Vec<GridRuleFiring<D>>,
}

pub type TransitionTrace = GridTransitionTrace<2>;

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
enum QueuedTurnItem {
    Effect(QueuedRuleEffect),
    Animation(AnimationEvent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputExecutionMode {
    Player,
    PlayerTrace,
    Headless,
}

impl InputExecutionMode {
    fn materializes_presentation(self) -> bool {
        matches!(self, Self::Player | Self::PlayerTrace)
    }

    fn collects_trace(self) -> bool {
        matches!(self, Self::PlayerTrace)
    }
}

#[derive(Clone, Debug)]
struct GridProgramOutcome<const D: usize, Size: GridSize<D>> {
    next_state: GridState<D, Size>,
    progressed: bool,
    observable: bool,
    cancelled: bool,
    commands: Vec<TransitionCommand>,
    firings: GridCapturedFirings<D>,
    items: Vec<QueuedTurnItem>,
}

#[derive(Clone, Debug)]
struct GridPendingInput<const D: usize, Size: GridSize<D>> {
    world: WorldInstanceId,
    programs: Vec<GridExecutableProgram<D>>,
    program_index: usize,
    continuation: Option<ProgramContinuation>,
    started: bool,
    program_input: Option<InputId>,
    target: Option<String>,
    undo_base_len: usize,
    turn_start_state: GridState<D, Size>,
    program_start_state: GridState<D, Size>,
    semantic_items: Vec<QueuedTurnItem>,
    lifecycle_item_base: usize,
    phase: GridPendingInputPhase,
    condition_effect: Option<SceneEffect>,
    previous_input: Option<String>,
    owns_turn_sfx: bool,
    separate_turn_sfx: bool,
    previous_turn_sfx: Option<HashSet<String>>,
    again_count: usize,
    records_trace: bool,
    mode: InputExecutionMode,
    trace: GridTransitionTrace<D>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GridPendingInputPhase {
    Rules,
    LevelClear,
}

#[derive(Clone, Debug)]
struct GridLifecycleOutcome<const D: usize, Size: GridSize<D>> {
    next_state: GridState<D, Size>,
    cancelled: bool,
    commands: Vec<TransitionCommand>,
    firings: GridCapturedFirings<D>,
}

#[derive(Clone, Debug)]
enum GridCapturedFirings<const D: usize> {
    Summary(Vec<GridRuleFiringSummary>),
    Detailed(Vec<GridRuleFiring<D>>),
}

impl<const D: usize> GridCapturedFirings<D> {
    fn is_empty(&self) -> bool {
        match self {
            Self::Summary(firings) => firings.is_empty(),
            Self::Detailed(firings) => firings.is_empty(),
        }
    }

    fn into_detailed(self) -> Vec<GridRuleFiring<D>> {
        match self {
            Self::Detailed(firings) => firings,
            Self::Summary(_) => {
                unreachable!("detailed firings were required for the traced input mode")
            }
        }
    }

    fn extend(&mut self, next: Self) {
        match (self, next) {
            (Self::Summary(current), Self::Summary(next)) => current.extend(next),
            (Self::Detailed(current), Self::Detailed(next)) => current.extend(next),
            _ => unreachable!("lifecycle firing capture mode changed during one transition"),
        }
    }
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
pub struct GridGameSession<const D: usize, Size: GridSize<D>> {
    focused_scene: String,
    visible_scenes: Vec<String>,
    focus_history: Vec<String>,
    scene_states: HashMap<String, GridSceneRuntimeState<D, Size>>,
    routed_world: WorldInstanceId,
    execution_world: Option<WorldInstanceId>,
    session_values: HashMap<String, SceneValue>,
    current_input: Option<String>,
    current_turn_sfx: Option<HashSet<String>>,
    presentation_events: Vec<PresentationEvent>,
    input_execution_mode: InputExecutionMode,
    last_level_completion: Option<GridLevelCompletion<D, Size>>,
    pending_input: Option<GridPendingInput<D, Size>>,
}

pub type GameSession = GridGameSession<2, Size2>;

/// The committed level state that satisfied turn completion before any
/// `on_level_clear` effects or navigation changed the active session state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridLevelCompletion<const D: usize, Size: GridSize<D>> {
    level_index: usize,
    state: GridState<D, Size>,
}

pub type LevelCompletion = GridLevelCompletion<2, Size2>;

impl<const D: usize, Size: GridSize<D>> GridLevelCompletion<D, Size> {
    pub fn level_index(&self) -> usize {
        self.level_index
    }

    pub fn state(&self) -> &GridState<D, Size> {
        &self.state
    }
}

/// A headless play session whose one public operation is one complete semantic
/// model input. It intentionally owns no clock or presentation scheduling:
/// `GridGameSession::apply_input` finishes `again` turns and lifecycle work
/// synchronously before returning.
#[derive(Clone, Debug)]
pub struct GridHeadlessSession<const D: usize, Size: GridSize<D>> {
    session: GridGameSession<D, Size>,
    tracked_level_index: usize,
    completion: Option<GridLevelCompletion<D, Size>>,
}

pub type HeadlessSession = GridHeadlessSession<2, Size2>;

impl<const D: usize, Size: GridSize<D>> GridHeadlessSession<D, Size> {
    pub fn from_level_state(
        game: &LoadedGridGame<D, Size>,
        level_index: usize,
        state: GridState<D, Size>,
    ) -> Result<Self, GridTransitionError<D>> {
        let mut session = GridGameSession::new(game);
        session.start_level_from_state(game, level_index, state.clone(), false)?;
        if session.active_level_index() != Some(level_index) || session.state() != &state {
            return Err(GridTransitionError::<D>::InvalidCommand(
                "headless level state was changed before its first semantic input".to_string(),
            ));
        }
        session.clear_headless_transients();
        Ok(Self {
            session,
            tracked_level_index: level_index,
            completion: None,
        })
    }

    pub fn from_game_session(
        session: GridGameSession<D, Size>,
        tracked_level_index: usize,
    ) -> Result<Self, GridTransitionError<D>> {
        Self::from_game_session_with_state_projection(
            session,
            tracked_level_index,
            GridState::clone,
        )
    }

    pub fn from_game_session_with_state_projection(
        mut session: GridGameSession<D, Size>,
        tracked_level_index: usize,
        mut project_state: impl FnMut(&GridState<D, Size>) -> GridState<D, Size>,
    ) -> Result<Self, GridTransitionError<D>> {
        if tracked_level_index >= session.cleared_levels().len() {
            return Err(GridTransitionError::<D>::InvalidCommand(format!(
                "headless tracked level index out of range: {tracked_level_index}"
            )));
        }
        session.clear_headless_transients();
        session.project_persistent_states(&mut project_state);
        Ok(Self {
            session,
            tracked_level_index,
            completion: None,
        })
    }

    pub fn apply_input(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        input: InputId,
    ) -> Result<(), GridTransitionError<D>> {
        self.session.apply_headless_input(game, input)?;
        self.completion = self
            .session
            .last_level_completion()
            .filter(|completion| completion.level_index() == self.tracked_level_index)
            .cloned();
        self.session.clear_headless_transients();
        Ok(())
    }

    pub fn state(&self) -> &GridState<D, Size> {
        self.session.state()
    }

    pub fn completed(&self) -> bool {
        self.completion.is_some()
    }

    pub fn completion(&self) -> Option<&GridLevelCompletion<D, Size>> {
        self.completion.as_ref()
    }

    pub fn observation_state(&self) -> &GridState<D, Size> {
        self.completion
            .as_ref()
            .map_or_else(|| self.state(), GridLevelCompletion::state)
    }

    pub fn session(&self) -> &GridGameSession<D, Size> {
        &self.session
    }

    pub fn into_session(self) -> GridGameSession<D, Size> {
        self.session
    }

    /// Exact future-relevant session state used for search deduplication.
    /// Presentation events, debug traces, and undo history are excluded because
    /// semantic model inputs cannot observe or invoke those player controls.
    pub fn search_key(&self) -> Vec<u8> {
        self.session
            .headless_search_key(self.tracked_level_index, self.completed())
    }
}

impl<const D: usize, Size: GridSize<D>> Deref for GridGameSession<D, Size> {
    type Target = GridWorldInstanceState<D, Size>;

    fn deref(&self) -> &Self::Target {
        let world = self.execution_world.as_ref().unwrap_or(&self.routed_world);
        self.scene_states
            .get(&world.scene)
            .and_then(|scene| scene.puzzles.get(&world.puzzle))
            .expect("routed world instance must exist")
    }
}

impl<const D: usize, Size: GridSize<D>> DerefMut for GridGameSession<D, Size> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let world = self
            .execution_world
            .as_ref()
            .unwrap_or(&self.routed_world)
            .clone();
        self.scene_states
            .get_mut(&world.scene)
            .and_then(|scene| scene.puzzles.get_mut(&world.puzzle))
            .expect("routed world instance must exist")
    }
}

impl<const D: usize, Size: GridSize<D>> GridGameSession<D, Size> {
    pub fn new(game: &LoadedGridGame<D, Size>) -> Self {
        let focused_scene = initial_scene_name(game).to_string();
        let routed_world = initial_world_instance(game, &focused_scene)
            .expect("loaded game must define at least one puzzle world instance");
        let mut session = Self {
            focused_scene: focused_scene.clone(),
            visible_scenes: vec![focused_scene.clone()],
            focus_history: Vec::new(),
            scene_states: HashMap::new(),
            routed_world: routed_world.clone(),
            execution_world: None,
            session_values: game
                .variables
                .iter()
                .map(|variable| (variable.name.clone(), variable.default.clone()))
                .collect(),
            current_input: None,
            current_turn_sfx: None,
            presentation_events: Vec::new(),
            input_execution_mode: InputExecutionMode::Player,
            last_level_completion: None,
            pending_input: None,
        };
        session.create_scene(game, &routed_world.scene);
        if routed_world.scene != focused_scene {
            session.create_scene(game, &focused_scene);
        }
        if routed_world.scene == focused_scene {
            let _ = session.start_current_world(game, true);
        }
        let _ = session.apply_scene_start_transition(game);
        let _ = session.apply_level_start_transition(game);
        session
    }

    fn clear_headless_transients(&mut self) {
        self.presentation_events.clear();
        self.last_level_completion = None;
        for scene in self.scene_states.values_mut() {
            for puzzle in scene.puzzles.values_mut() {
                puzzle.history.clear();
                puzzle.last_transition_trace = None;
            }
        }
    }

    fn project_persistent_states(
        &mut self,
        project_state: &mut impl FnMut(&GridState<D, Size>) -> GridState<D, Size>,
    ) {
        for scene in self.scene_states.values_mut() {
            for puzzle in scene.puzzles.values_mut() {
                puzzle.state = project_state(&puzzle.state);
                puzzle.initial_state = project_state(&puzzle.initial_state);
                puzzle.checkpoint_state = puzzle.checkpoint_state.as_ref().map(&mut *project_state);
                if let Some(initial) = &mut puzzle.level_initial_state_override {
                    initial.state = project_state(&initial.state);
                }
            }
        }
    }

    fn headless_search_key(&self, tracked_level_index: usize, completed: bool) -> Vec<u8> {
        fn push_u64(out: &mut Vec<u8>, value: u64) {
            out.extend_from_slice(&value.to_le_bytes());
        }

        fn push_text(out: &mut Vec<u8>, value: &str) {
            push_u64(out, value.len() as u64);
            out.extend_from_slice(value.as_bytes());
        }

        fn push_value(out: &mut Vec<u8>, value: &SceneValue) {
            match value {
                SceneValue::Bool(value) => {
                    out.push(0);
                    out.push(u8::from(*value));
                }
                SceneValue::Int(value) => {
                    out.push(1);
                    push_u64(out, *value as u64);
                }
                SceneValue::Text(value) => {
                    out.push(2);
                    push_text(out, value);
                }
                SceneValue::Symbol(value) => {
                    out.push(3);
                    push_text(out, value);
                }
                SceneValue::LevelRef(value) => {
                    out.push(4);
                    push_u64(out, *value as u64);
                }
            }
        }

        fn push_state<const D: usize, Size: GridSize<D>>(
            out: &mut Vec<u8>,
            state: &GridState<D, Size>,
        ) {
            for axis in state.size.axes() {
                push_u64(out, u64::from(axis));
            }
            push_u64(out, u64::from(state.layer_count));
            push_u64(out, state.slots().len() as u64);
            for object in state.slots() {
                push_u64(out, u64::from(object.0));
            }
            push_state_marks(out, state);
            push_u64(out, state.visible_variables().len() as u64);
            for value in state.visible_variables() {
                push_u64(out, *value as u64);
            }
            push_u64(out, state.level_fired_rules().len() as u64);
            for rule in state.level_fired_rules() {
                push_u64(out, u64::from(rule.0));
            }
        }

        fn push_state_marks<const D: usize, Size: GridSize<D>>(
            out: &mut Vec<u8>,
            state: &GridState<D, Size>,
        ) {
            for marks in [state.cell_mark(), state.slot_mark()] {
                push_u64(out, marks.len() as u64);
                for cell in marks {
                    push_u64(out, cell.len() as u64);
                    for mark in cell {
                        push_u64(out, u64::from(mark.mark.0));
                        out.push(u8::from(mark.value.is_some()));
                        if let Some(value) = mark.value {
                            push_u64(out, value as u64);
                        }
                    }
                }
            }
        }

        fn push_optional_state<const D: usize, Size: GridSize<D>>(
            out: &mut Vec<u8>,
            state: Option<&GridState<D, Size>>,
        ) {
            out.push(u8::from(state.is_some()));
            if let Some(state) = state {
                push_state(out, state);
            }
        }

        let mut out = Vec::new();
        let current_world = self.current_world_id();
        push_u64(&mut out, tracked_level_index as u64);
        out.push(u8::from(completed));
        push_text(&mut out, &self.focused_scene);
        push_u64(&mut out, self.visible_scenes.len() as u64);
        for scene in &self.visible_scenes {
            push_text(&mut out, scene);
        }
        push_u64(&mut out, self.focus_history.len() as u64);
        for scene in &self.focus_history {
            push_text(&mut out, scene);
        }
        let mut scene_names = self.scene_states.keys().cloned().collect::<Vec<_>>();
        scene_names.sort();
        push_u64(&mut out, scene_names.len() as u64);
        for scene_name in scene_names {
            push_text(&mut out, &scene_name);
            let scene = &self.scene_states[&scene_name];
            let mut values = scene.values.iter().collect::<Vec<_>>();
            values.sort_by_key(|(name, _)| *name);
            push_u64(&mut out, values.len() as u64);
            for (name, value) in values {
                push_text(&mut out, name);
                push_value(&mut out, value);
            }
            let mut puzzle_names = scene.puzzles.keys().cloned().collect::<Vec<_>>();
            puzzle_names.sort();
            push_u64(&mut out, puzzle_names.len() as u64);
            for puzzle_name in puzzle_names {
                push_text(&mut out, &puzzle_name);
                let puzzle = &scene.puzzles[&puzzle_name];
                push_text(&mut out, &puzzle.model);
                if current_world.scene == scene_name && current_world.puzzle == puzzle_name {
                    // The solver key already owns this state's dimensions,
                    // slots, variables, and fired rules. Marks are the only
                    // remaining core state not represented there.
                    push_state_marks(&mut out, &puzzle.state);
                } else {
                    push_state(&mut out, &puzzle.state);
                }
                push_state(&mut out, &puzzle.initial_state);
                push_optional_state(&mut out, puzzle.checkpoint_state.as_ref());
                push_u64(
                    &mut out,
                    puzzle
                        .active_level_index
                        .map_or(u64::MAX, |index| index as u64),
                );
                out.push(u8::from(puzzle.level_started));
                push_u64(&mut out, puzzle.cleared_levels.len() as u64);
                for cleared in &puzzle.cleared_levels {
                    out.push(u8::from(*cleared));
                }
                push_u64(&mut out, puzzle.selected_level_index as u64);
                push_optional_state(
                    &mut out,
                    puzzle
                        .level_initial_state_override
                        .as_ref()
                        .map(|initial| &initial.state),
                );
                push_u64(&mut out, puzzle.persistent_vars.len() as u64);
                for value in &puzzle.persistent_vars {
                    push_u64(&mut out, *value as u64);
                }
            }
        }
        push_text(&mut out, &self.routed_world.scene);
        push_text(&mut out, &self.routed_world.puzzle);
        out.push(u8::from(self.execution_world.is_some()));
        if let Some(world) = &self.execution_world {
            push_text(&mut out, &world.scene);
            push_text(&mut out, &world.puzzle);
        }
        let mut session_values = self.session_values.iter().collect::<Vec<_>>();
        session_values.sort_by_key(|(name, _)| *name);
        push_u64(&mut out, session_values.len() as u64);
        for (name, value) in session_values {
            push_text(&mut out, name);
            push_value(&mut out, value);
        }
        out.push(u8::from(self.current_input.is_some()));
        if let Some(input) = &self.current_input {
            push_text(&mut out, input);
        }
        out
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

    pub fn routed_world(&self) -> &WorldInstanceId {
        &self.routed_world
    }

    pub fn world_state(&self, world: &WorldInstanceId) -> Option<&GridWorldInstanceState<D, Size>> {
        self.scene_states
            .get(&world.scene)
            .and_then(|scene| scene.puzzles.get(&world.puzzle))
    }

    fn with_execution_world<T>(
        &mut self,
        world: WorldInstanceId,
        run: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = self.execution_world.replace(world);
        let result = run(self);
        self.execution_world = previous;
        result
    }

    fn current_world_id(&self) -> &WorldInstanceId {
        self.execution_world.as_ref().unwrap_or(&self.routed_world)
    }

    pub fn accepts_model_input(&self, game: &LoadedGridGame<D, Size>) -> bool {
        self.current_scene_accepts_model_input(game)
    }

    pub fn level_index(&self) -> usize {
        self.active_level_index.unwrap_or(0)
    }

    pub fn active_level_index(&self) -> Option<usize> {
        self.active_level_index
    }

    pub fn selected_level_index(&self) -> usize {
        self.selected_level_index
    }

    pub fn current_level<'a>(
        &self,
        game: &'a LoadedGridGame<D, Size>,
    ) -> &'a LoadedGridLevel<D, Size> {
        &game.levels[self.level_index()]
    }

    pub fn state(&self) -> &GridState<D, Size> {
        &self.state
    }

    pub fn scene_state(&self) -> Option<&GridSceneRuntimeState<D, Size>> {
        self.scene_states.get(&self.focused_scene)
    }

    pub fn scene_state_for(&self, name: &str) -> Option<&GridSceneRuntimeState<D, Size>> {
        self.scene_states.get(name)
    }

    pub fn session_values(&self) -> &HashMap<String, SceneValue> {
        &self.session_values
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn cleared_levels(&self) -> &[bool] {
        &self.cleared_levels
    }

    pub fn progress_save_data(&self, game: &LoadedGridGame<D, Size>) -> ProgressSaveData {
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
                    let name = game.variable_labels.get(var)?;
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
        game: &LoadedGridGame<D, Size>,
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
                .position(|var| game.variable_labels.get(var) == Some(&saved_var.name))
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
        self.apply_persistent_vars_to_current_world(game);
        Ok(())
    }

    fn preferred_level_index(&self, game: &LoadedGridGame<D, Size>) -> Option<usize> {
        self.active_level_index.or_else(|| {
            (self.selected_level_index < game.levels.len()).then_some(self.selected_level_index)
        })
    }

    pub fn take_presentation_events(&mut self) -> Vec<PresentationEvent> {
        std::mem::take(&mut self.presentation_events)
    }

    fn push_presentation_event(&mut self, kind: PresentationEventKind) {
        if !self.input_execution_mode.materializes_presentation() {
            return;
        }
        let world = self.current_world_id().clone();
        self.presentation_events.push(PresentationEvent {
            context: PresentationContext {
                scene: world.scene,
                puzzle: world.puzzle,
                level_index: self.active_level_index,
            },
            kind,
        });
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
        self.history.clear();
    }

    fn clear_game_progress(&mut self, game: &LoadedGridGame<D, Size>) {
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
        game: &LoadedGridGame<D, Size>,
        level: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) {
        if let Some(index) = self.eval_effect_level_index(game, level, bindings) {
            self.selected_level_index = index;
        }
    }

    fn set_level_cleared_progress(
        &mut self,
        game: &LoadedGridGame<D, Size>,
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

    fn reset_persistent_vars(&mut self, game: &LoadedGridGame<D, Size>) {
        self.persistent_vars = persistent_var_default_values(game);
        self.apply_persistent_vars_to_current_world(game);
    }

    fn reset_persistent_var(&mut self, game: &LoadedGridGame<D, Size>, name: &str) -> bool {
        let Some(index) = persistent_var_index_by_name(game, name) else {
            return false;
        };
        let defaults = persistent_var_default_values(game);
        let Some(value) = self.persistent_vars.get_mut(index) else {
            return true;
        };
        *value = defaults.get(index).copied().unwrap_or(0);
        self.apply_persistent_vars_to_current_world(game);
        true
    }

    fn apply_persistent_vars(
        &self,
        game: &LoadedGridGame<D, Size>,
        state: &mut GridState<D, Size>,
    ) {
        for (index, var) in game.persistent_vars.iter().enumerate() {
            if let Some(value) = self.persistent_vars.get(index) {
                let _ = state.set_visible_variable(*var, *value);
            }
        }
    }

    fn capture_persistent_vars(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        state: &GridState<D, Size>,
    ) {
        self.persistent_vars = persistent_var_values(game, state);
    }

    fn apply_persistent_vars_to_current_world(&mut self, game: &LoadedGridGame<D, Size>) {
        let vars = self.persistent_vars.clone();
        apply_persistent_var_values(game, &vars, &mut self.state);
        apply_persistent_var_values(game, &vars, &mut self.initial_state);
        if let Some(state) = &mut self.checkpoint_state {
            apply_persistent_var_values(game, &vars, state);
        }
    }

    pub fn has_next_level(&self, game: &LoadedGridGame<D, Size>) -> bool {
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
        game: &LoadedGridGame<D, Size>,
        input: InputId,
    ) -> Result<(), GridTransitionError<D>> {
        if self.is_waiting() {
            return Err(GridTransitionError::<D>::InvalidCommand(
                "cannot apply input while a turn is waiting".to_string(),
            ));
        }
        self.last_level_completion = None;
        self.apply_input_with_mode(game, input, InputExecutionMode::Player)
    }

    pub fn apply_traced_input(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        input: InputId,
    ) -> Result<(), GridTransitionError<D>> {
        if self.is_waiting() {
            return Err(GridTransitionError::<D>::InvalidCommand(
                "cannot apply input while a turn is waiting".to_string(),
            ));
        }
        self.last_level_completion = None;
        self.apply_input_with_mode(game, input, InputExecutionMode::PlayerTrace)
    }

    pub fn is_waiting(&self) -> bool {
        self.pending_input.is_some()
    }

    pub fn resume_wait(
        &mut self,
        game: &LoadedGridGame<D, Size>,
    ) -> Result<(), GridTransitionError<D>> {
        if self.pending_input.is_none() {
            return Err(GridTransitionError::<D>::InvalidCommand(
                "cannot resume because no turn is waiting".to_string(),
            ));
        }
        self.run_pending_input(game)
    }

    pub fn last_level_completion(&self) -> Option<&GridLevelCompletion<D, Size>> {
        self.last_level_completion.as_ref()
    }

    /// Applies one complete semantic model input without materializing any
    /// presentation timeline or debug trace. Turn completion, `again`, scene
    /// effects, and lifecycle handling remain owned by the same player path.
    pub fn apply_headless_input(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        input: InputId,
    ) -> Result<(), GridTransitionError<D>> {
        self.last_level_completion = None;
        self.apply_input_with_mode(game, input, InputExecutionMode::Headless)?;
        Ok(())
    }

    fn apply_input_with_mode(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        input: InputId,
        mode: InputExecutionMode,
    ) -> Result<(), GridTransitionError<D>> {
        let previous_mode = self.input_execution_mode;
        self.input_execution_mode = mode;
        let result = self.apply_input_in_current_mode(game, input);
        self.input_execution_mode = previous_mode;
        result
    }

    fn apply_input_in_current_mode(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        input: InputId,
    ) -> Result<(), GridTransitionError<D>> {
        self.last_transition_trace = None;
        let previous_input = self.current_input.clone();
        self.current_input = game.input_labels.get(&input).cloned();
        let undo_base_len = self.history.undo_len();
        let owns_turn_sfx = self.begin_turn_sfx_dedup();
        if !self.current_scene_accepts_model_input(game) {
            let result = self.apply_focused_scene_input_transition(game, undo_base_len);
            self.current_input = previous_input;
            self.end_turn_sfx_dedup(owns_turn_sfx);
            return result;
        }
        if self.input_execution_mode.materializes_presentation() {
            let result = self.start_segmented_model_input(
                game,
                input,
                previous_input.clone(),
                owns_turn_sfx,
            );
            if result.is_err() {
                self.current_input = previous_input;
                self.end_turn_sfx_dedup(owns_turn_sfx);
            }
            return result;
        }
        let result = match self.apply_model_input(game, input, undo_base_len) {
            Ok(result) => result,
            Err(error) => {
                self.current_input = previous_input;
                self.end_turn_sfx_dedup(owns_turn_sfx);
                return Err(error);
            }
        };
        let world = result
            .world
            .clone()
            .unwrap_or_else(|| self.current_world_id().clone());
        if self.input_execution_mode.collects_trace() {
            let transition_trace = result.transition_trace.clone();
            self.with_execution_world(world.clone(), |session| {
                session.last_transition_trace = transition_trace;
            });
        }
        if !result.cancelled {
            let condition_result = self.with_execution_world(world, |session| {
                session.apply_turn_completion(game, result.items, result.undo_base_len)
            });
            self.current_input = previous_input;
            self.end_turn_sfx_dedup(owns_turn_sfx);
            condition_result?;
            return Ok(());
        }
        self.current_input = previous_input;
        self.end_turn_sfx_dedup(owns_turn_sfx);
        Ok(())
    }

    fn start_segmented_model_input(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        input: InputId,
        previous_input: Option<String>,
        owns_turn_sfx: bool,
    ) -> Result<(), GridTransitionError<D>> {
        let target = game
            .scenes
            .iter()
            .find(|scene| scene.name == self.focused_scene)
            .and_then(|scene| scene.puzzle_rule.as_ref())
            .map(|rule| rule.target.clone());
        let world = if let Some(target) = target.as_deref() {
            let Some((scene_name, puzzle_name)) = self.resolve_puzzle_target(game, target) else {
                return Err(invalid_puzzle_target_error(target));
            };
            if scene_puzzle_initializer(game, &scene_name, &puzzle_name).is_none() {
                return Err(invalid_puzzle_target_error(target));
            }
            self.create_scene(game, &scene_name);
            WorldInstanceId {
                scene: scene_name,
                puzzle: puzzle_name,
            }
        } else {
            self.current_world_id().clone()
        };
        let input_label = self.current_input.clone();
        let pending = self.segmented_pending_for_input(
            game,
            world,
            input,
            target,
            None,
            previous_input.clone(),
            owns_turn_sfx,
            false,
            None,
            0,
            true,
        )?;
        let Some(pending) = pending else {
            self.current_input = previous_input;
            self.end_turn_sfx_dedup(owns_turn_sfx);
            return Ok(());
        };
        self.current_input = input_label;
        self.pending_input = Some(pending);
        self.run_pending_input(game)
    }

    #[allow(clippy::too_many_arguments)]
    fn segmented_pending_for_input(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        world: WorldInstanceId,
        input: InputId,
        target: Option<String>,
        undo_base_len: Option<usize>,
        previous_input: Option<String>,
        owns_turn_sfx: bool,
        separate_turn_sfx: bool,
        previous_turn_sfx: Option<HashSet<String>>,
        again_count: usize,
        records_trace: bool,
    ) -> Result<Option<GridPendingInput<D, Size>>, GridTransitionError<D>> {
        let mode = self.input_execution_mode;
        self.with_execution_world(world.clone(), |session| {
            let Some(level_index) = session.active_level_index else {
                return Ok(None);
            };
            let programs = game
                .programs_for_level(level_index)
                .ok_or_else(|| {
                    GridTransitionError::<D>::InvalidCommand(format!(
                        "active level index out of range: {level_index}"
                    ))
                })?
                .into_iter()
                .cloned()
                .collect();
            let mut start_state = session.state.clone();
            session.apply_persistent_vars(game, &mut start_state);
            Ok(Some(GridPendingInput {
                world,
                programs,
                program_index: 0,
                continuation: None,
                started: false,
                program_input: Some(input),
                target: target.clone(),
                undo_base_len: undo_base_len.unwrap_or_else(|| session.history.undo_len()),
                turn_start_state: start_state.clone(),
                program_start_state: start_state,
                semantic_items: Vec::new(),
                lifecycle_item_base: 0,
                phase: GridPendingInputPhase::Rules,
                condition_effect: None,
                previous_input,
                owns_turn_sfx,
                separate_turn_sfx,
                previous_turn_sfx,
                again_count,
                records_trace,
                mode,
                trace: GridTransitionTrace {
                    input,
                    target,
                    progressed: false,
                    observable: false,
                    cancelled: false,
                    commands: Vec::new(),
                    firings: Vec::new(),
                },
            }))
        })
    }

    fn run_pending_input(
        &mut self,
        game: &LoadedGridGame<D, Size>,
    ) -> Result<(), GridTransitionError<D>> {
        let pending = self.pending_input.take().ok_or_else(|| {
            GridTransitionError::<D>::InvalidCommand(
                "cannot resume because no turn is waiting".to_string(),
            )
        })?;
        let recovery = pending.clone();
        let world = pending.world.clone();
        let previous_mode = self.input_execution_mode;
        self.input_execution_mode = pending.mode;
        let result = self.with_execution_world(world, |session| {
            session.run_pending_input_in_world(game, pending)
        });
        self.input_execution_mode = previous_mode;
        if result.is_err() {
            self.with_execution_world(recovery.world, |session| {
                session.state = recovery.turn_start_state;
                session.capture_persistent_vars(game, &session.state.clone());
                session.history.truncate_undo(recovery.undo_base_len);
            });
            if recovery.separate_turn_sfx {
                self.end_separate_turn_sfx_dedup(recovery.previous_turn_sfx);
            }
            self.current_input = recovery.previous_input;
            self.end_turn_sfx_dedup(recovery.owns_turn_sfx);
        }
        result
    }

    fn run_pending_input_in_world(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        mut pending: GridPendingInput<D, Size>,
    ) -> Result<(), GridTransitionError<D>> {
        loop {
            let program = pending
                .programs
                .get(pending.program_index)
                .expect("pending program index is valid");
            let segment = if pending.started {
                transition_program_continuation_segment_trace(
                    &game.game,
                    program,
                    pending
                        .continuation
                        .as_ref()
                        .expect("started pending input has a continuation"),
                    &self.state,
                    pending.program_input,
                    None,
                    |boundary| rule_boundary_pauses(game, boundary.firings),
                )?
            } else {
                transition_program_segment_trace(
                    &game.game,
                    program,
                    &pending.program_start_state,
                    pending.program_input,
                    None,
                    |boundary| rule_boundary_pauses(game, boundary.firings),
                )?
            };
            let trace = segment.trace;
            let next_state = trace.next_state.clone();
            let firings = GridCapturedFirings::Detailed(trace.firings.clone());
            let items = queued_turn_items_for_outcome(
                game,
                (pending.phase == GridPendingInputPhase::Rules)
                    .then_some(pending.target.as_deref())
                    .flatten(),
                &trace.commands,
                &firings,
                &next_state,
                pending.mode,
            );
            match pending.phase {
                GridPendingInputPhase::Rules => {
                    self.replace_state_if_changed(game, next_state);
                    pending.trace.progressed |= trace.progressed;
                    pending.trace.observable |= trace.observable;
                    pending.trace.cancelled |= trace.cancelled;
                    pending.trace.commands.extend(trace.commands);
                    pending.trace.firings.extend(trace.firings);
                }
                GridPendingInputPhase::LevelClear => {
                    self.replace_state_if_changed_without_undo(game, next_state);
                }
            }
            let (semantic_items, emitted_wait) = self.materialize_segment_presentation(game, items);
            pending.semantic_items.extend(semantic_items);

            if trace.cancelled {
                match pending.phase {
                    GridPendingInputPhase::Rules => {
                        self.state = pending.turn_start_state;
                        self.capture_persistent_vars(game, &self.state.clone());
                        self.history.truncate_undo(pending.undo_base_len);
                        if pending.mode.collects_trace() && pending.records_trace {
                            self.last_transition_trace = Some(pending.trace);
                        }
                        if pending.separate_turn_sfx {
                            self.end_separate_turn_sfx_dedup(pending.previous_turn_sfx);
                        }
                        self.current_input = pending.previous_input;
                        self.end_turn_sfx_dedup(pending.owns_turn_sfx);
                        return Ok(());
                    }
                    GridPendingInputPhase::LevelClear => {
                        pending.semantic_items.truncate(pending.lifecycle_item_base);
                        return self.finish_segmented_input(game, pending);
                    }
                }
            }

            pending.continuation = segment.remaining_program;
            pending.started = true;
            if emitted_wait {
                self.pending_input = Some(pending);
                return Ok(());
            }
            if pending.continuation.is_some() {
                continue;
            }
            pending.program_index += 1;
            if pending.program_index < pending.programs.len() {
                pending.started = false;
                pending.program_start_state = self.state.clone();
                continue;
            }
            match pending.phase {
                GridPendingInputPhase::Rules => {
                    if self.prepare_segmented_level_clear(game, &mut pending) {
                        continue;
                    }
                    return self.finish_segmented_input(game, pending);
                }
                GridPendingInputPhase::LevelClear => {
                    return self.finish_segmented_input(game, pending);
                }
            }
        }
    }

    fn prepare_segmented_level_clear(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        pending: &mut GridPendingInput<D, Size>,
    ) -> bool {
        let win_targets = pending
            .semantic_items
            .iter()
            .filter_map(|item| match item {
                QueuedTurnItem::Effect(effect) if matches!(effect.effect, RuleEffect::Win) => {
                    Some(effect.target.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let force_clear = !win_targets.is_empty() && self.can_force_clear_state(game, &self.state);
        let forced_win_targets: &[Option<String>] = if force_clear { &win_targets } else { &[] };
        pending.condition_effect = self.condition_transition_effect(game, forced_win_targets);
        let clears = force_clear || self.can_clear_state(game, &self.state);
        if clears
            && self.last_level_completion.is_none()
            && let Some(level_index) = self.active_level_index
        {
            self.last_level_completion = Some(GridLevelCompletion {
                level_index,
                state: self.state.clone(),
            });
        }
        if !clears || game.is_lose_complete(&self.state) {
            return false;
        }
        self.mark_current_level_cleared();
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
        let level_clear_program = game.level_clear_program_for_level(self.level_index());
        let programs = model_clear_program
            .into_iter()
            .chain(level_clear_program)
            .cloned()
            .collect::<Vec<_>>();
        if programs.is_empty() {
            return false;
        }
        pending.lifecycle_item_base = pending.semantic_items.len();
        pending.phase = GridPendingInputPhase::LevelClear;
        pending.programs = programs;
        pending.program_index = 0;
        pending.continuation = None;
        pending.started = false;
        pending.program_input = None;
        pending.program_start_state = self.state.clone();
        true
    }

    fn finish_segmented_input(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        mut pending: GridPendingInput<D, Size>,
    ) -> Result<(), GridTransitionError<D>> {
        if pending.mode.collects_trace() && pending.records_trace {
            self.last_transition_trace = Some(pending.trace.clone());
        }
        let commands = self.apply_turn_effect_items(game, pending.semantic_items)?;
        let pending_again = self.resolve_turn_commands(game, commands, pending.condition_effect)?;
        self.compress_undo_stack_to_turn_boundary(pending.undo_base_len);

        if pending.separate_turn_sfx {
            self.end_separate_turn_sfx_dedup(pending.previous_turn_sfx.take());
        }
        if let Some(target) = pending_again
            && pending.again_count < MAX_AGAIN_TURNS_PER_INPUT
        {
            let world = if let Some(target) = target.as_deref() {
                let Some((scene_name, puzzle_name)) = self.resolve_puzzle_target(game, target)
                else {
                    return Err(invalid_puzzle_target_error(target));
                };
                if scene_puzzle_initializer(game, &scene_name, &puzzle_name).is_none() {
                    return Err(invalid_puzzle_target_error(target));
                }
                self.create_scene(game, &scene_name);
                WorldInstanceId {
                    scene: scene_name,
                    puzzle: puzzle_name,
                }
            } else {
                pending.world.clone()
            };
            let same_world = world == pending.world;
            let previous_turn_sfx = self.begin_separate_turn_sfx_dedup();
            let restore_turn_sfx = previous_turn_sfx.clone();
            let next = match self.segmented_pending_for_input(
                game,
                world,
                InputId(0),
                target,
                same_world.then_some(pending.undo_base_len),
                pending.previous_input.clone(),
                pending.owns_turn_sfx,
                true,
                previous_turn_sfx,
                pending.again_count + 1,
                false,
            ) {
                Ok(next) => next,
                Err(error) => {
                    self.end_separate_turn_sfx_dedup(restore_turn_sfx);
                    return Err(error);
                }
            };
            let Some(next) = next else {
                self.end_separate_turn_sfx_dedup(restore_turn_sfx);
                self.current_input = pending.previous_input;
                self.end_turn_sfx_dedup(pending.owns_turn_sfx);
                return Ok(());
            };
            self.pending_input = Some(next);
            return self.run_pending_input(game);
        }
        self.current_input = pending.previous_input;
        self.end_turn_sfx_dedup(pending.owns_turn_sfx);
        Ok(())
    }

    fn materialize_segment_presentation(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        items: Vec<QueuedTurnItem>,
    ) -> (Vec<QueuedTurnItem>, bool) {
        let mut semantic_items = Vec::new();
        let mut emitted_wait = false;
        for item in items {
            let QueuedTurnItem::Effect(effect) = item else {
                let QueuedTurnItem::Animation(animation) = item else {
                    unreachable!()
                };
                self.push_presentation_event(PresentationEventKind::Animation(animation));
                continue;
            };
            let QueuedRuleEffect { target, effect } = effect;
            match effect {
                RuleEffect::PlaySfx { name } => {
                    if self.should_emit_turn_sfx(&name) {
                        self.push_presentation_event(PresentationEventKind::Sound(
                            SoundEvent::PlaySfx { name },
                        ));
                    }
                }
                RuleEffect::PlayMusic { name } => self.push_presentation_event(
                    PresentationEventKind::Sound(SoundEvent::PlayMusic { name }),
                ),
                RuleEffect::PauseMusic { name } => self.push_presentation_event(
                    PresentationEventKind::Sound(SoundEvent::PauseMusic { name }),
                ),
                RuleEffect::ResumeMusic { name } => self.push_presentation_event(
                    PresentationEventKind::Sound(SoundEvent::ResumeMusic { name }),
                ),
                RuleEffect::StopMusic { name } => self.push_presentation_event(
                    PresentationEventKind::Sound(SoundEvent::StopMusic { name }),
                ),
                RuleEffect::Wait { milliseconds } => {
                    if milliseconds > 0 {
                        emitted_wait = true;
                        self.push_presentation_event(PresentationEventKind::Wait(
                            WaitEvent::Wait { milliseconds },
                        ));
                    }
                }
                RuleEffect::WaitAnimation | RuleEffect::EmitAnimation { .. } => {}
                RuleEffect::Message { text, literal } => {
                    let text = self.resolve_message_text(&text, literal);
                    self.push_presentation_event(PresentationEventKind::Message(
                        MessageEvent::Message { text },
                    ));
                    if game.default_wait_ms > 0 {
                        emitted_wait = true;
                        self.push_presentation_event(PresentationEventKind::Wait(
                            WaitEvent::Wait {
                                milliseconds: game.default_wait_ms,
                            },
                        ));
                    }
                }
                effect => {
                    semantic_items.push(QueuedTurnItem::Effect(QueuedRuleEffect { target, effect }))
                }
            }
        }
        (semantic_items, emitted_wait)
    }

    pub fn last_transition_trace(&self) -> Option<&GridTransitionTrace<D>> {
        self.last_transition_trace.as_ref()
    }

    fn apply_model_input(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        input: InputId,
        undo_base_len: usize,
    ) -> Result<GridModelInputResult<D>, GridTransitionError<D>> {
        let target = game
            .scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
            .and_then(|screen| screen.puzzle_rule.as_ref())
            .map(|rule| rule.target.clone());
        if let Some(target) = target {
            return self.apply_model_input_to_target(game, &target, input);
        }
        self.apply_model_input_to_current_level(game, input, undo_base_len, None)
    }

    fn apply_model_input_to_target(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        target: &str,
        input: InputId,
    ) -> Result<GridModelInputResult<D>, GridTransitionError<D>> {
        let Some((scene_name, puzzle_name)) = self.resolve_puzzle_target(game, target) else {
            return Err(invalid_puzzle_target_error(target));
        };
        if scene_puzzle_initializer(game, &scene_name, &puzzle_name).is_none() {
            return Err(invalid_puzzle_target_error(target));
        }
        self.create_scene(game, &scene_name);
        let world = WorldInstanceId {
            scene: scene_name,
            puzzle: puzzle_name,
        };
        let Some(undo_base_len) = self
            .world_state(&world)
            .map(|world| world.history.undo_len())
        else {
            return Err(invalid_puzzle_target_error(target));
        };
        self.with_execution_world(world, |session| {
            session.apply_model_input_to_current_level(game, input, undo_base_len, Some(target))
        })
    }

    fn apply_model_input_to_current_level(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        input: InputId,
        undo_base_len: usize,
        target: Option<&str>,
    ) -> Result<GridModelInputResult<D>, GridTransitionError<D>> {
        if self.active_level_index.is_none() {
            return Ok(GridModelInputResult::<D>::default());
        }
        let mut state = self.state.clone();
        self.apply_persistent_vars(game, &mut state);
        let level_index = self
            .active_level_index
            .expect("active level was checked before applying model input");
        let programs = game.programs_for_level(level_index).ok_or_else(|| {
            GridTransitionError::<D>::InvalidCommand(format!(
                "active level index out of range: {level_index}"
            ))
        })?;
        let outcome = transition_program_outcome_with_effects(
            game,
            &programs,
            &state,
            input,
            target,
            self.input_execution_mode,
        )?;
        let cancelled = outcome.cancelled;
        self.replace_state_if_changed(game, outcome.next_state);
        let transition_trace = if self.input_execution_mode.collects_trace() {
            Some(GridTransitionTrace::<D> {
                input,
                target: target.map(str::to_string),
                progressed: outcome.progressed,
                observable: outcome.observable,
                cancelled,
                commands: outcome.commands,
                firings: outcome.firings.into_detailed(),
            })
        } else {
            None
        };
        Ok(GridModelInputResult::<D> {
            world: Some(self.current_world_id().clone()),
            undo_base_len,
            cancelled,
            items: outcome.items,
            transition_trace,
        })
    }

    fn apply_turn_completion(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        items: Vec<QueuedTurnItem>,
        undo_base_len: usize,
    ) -> Result<(), GridTransitionError<D>> {
        let win_targets = items
            .iter()
            .filter_map(|item| match item {
                QueuedTurnItem::Effect(effect) if matches!(effect.effect, RuleEffect::Win) => {
                    Some(effect.target.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let force_clear = !win_targets.is_empty() && self.can_force_clear_state(game, &self.state);
        let forced_win_targets: &[Option<String>] = if force_clear { &win_targets } else { &[] };
        let condition_effect = self.condition_transition_effect(game, forced_win_targets);
        if (force_clear || self.can_clear_state(game, &self.state))
            && self.last_level_completion.is_none()
            && let Some(level_index) = self.active_level_index
        {
            self.last_level_completion = Some(GridLevelCompletion {
                level_index,
                state: self.state.clone(),
            });
        }
        let mut items = items;
        items.extend(self.apply_model_level_clear(game, force_clear)?);
        self.resolve_turn_effects(game, items, condition_effect, undo_base_len)
    }

    fn resolve_turn_effects(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        items: Vec<QueuedTurnItem>,
        condition_effect: Option<SceneEffect>,
        undo_base_len: usize,
    ) -> Result<(), GridTransitionError<D>> {
        let commands = self.apply_turn_effect_items(game, items)?;
        if let Some(target) = self.resolve_turn_commands(game, commands, condition_effect)? {
            self.apply_again_turns(game, target, undo_base_len)?;
        }
        Ok(())
    }

    fn apply_turn_effect_items(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        items: Vec<QueuedTurnItem>,
    ) -> Result<Vec<QueuedTransitionCommand>, GridTransitionError<D>> {
        let mut commands = Vec::new();
        let mut index = 0;
        while index < items.len() {
            let item = items[index].clone();
            let QueuedTurnItem::Effect(effect) = item else {
                let QueuedTurnItem::Animation(animation) = item else {
                    unreachable!()
                };
                self.push_presentation_event(PresentationEventKind::Animation(animation));
                index += 1;
                continue;
            };
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
                        self.push_presentation_event(PresentationEventKind::Sound(
                            SoundEvent::PlaySfx { name },
                        ));
                    }
                }
                RuleEffect::PlayMusic { name } => {
                    self.push_presentation_event(PresentationEventKind::Sound(
                        SoundEvent::PlayMusic { name },
                    ));
                }
                RuleEffect::PauseMusic { name } => {
                    self.push_presentation_event(PresentationEventKind::Sound(
                        SoundEvent::PauseMusic { name },
                    ));
                }
                RuleEffect::ResumeMusic { name } => {
                    self.push_presentation_event(PresentationEventKind::Sound(
                        SoundEvent::ResumeMusic { name },
                    ));
                }
                RuleEffect::StopMusic { name } => {
                    self.push_presentation_event(PresentationEventKind::Sound(
                        SoundEvent::StopMusic { name },
                    ));
                }
                RuleEffect::Wait { milliseconds } => {
                    self.push_presentation_event(PresentationEventKind::Wait(WaitEvent::Wait {
                        milliseconds,
                    }));
                }
                RuleEffect::WaitAnimation => {}
                RuleEffect::EmitAnimation { .. } => {}
                RuleEffect::Message { text, literal } => {
                    let text = self.resolve_message_text(&text, literal);
                    self.push_presentation_event(PresentationEventKind::Message(
                        MessageEvent::Message { text },
                    ));
                    self.push_presentation_event(PresentationEventKind::Wait(WaitEvent::Wait {
                        milliseconds: game.default_wait_ms,
                    }));
                }
                RuleEffect::Scene { effect } => {
                    self.apply_screen_effect(game, &effect, &HashMap::new())?;
                }
            }
            index += 1;
        }
        Ok(commands)
    }

    fn resolve_turn_commands(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        commands: Vec<QueuedTransitionCommand>,
        condition_effect: Option<SceneEffect>,
    ) -> Result<Option<Option<String>>, GridTransitionError<D>> {
        let mut pending_next_level = None::<Option<String>>;
        let mut pending_again = None::<Option<String>>;
        let mut pending_restart = false;
        for command in commands {
            match command.command {
                TransitionCommand::Win => {}
                TransitionCommand::Restart => {
                    pending_restart = true;
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
        if pending_restart {
            let preceding_presentation = std::mem::take(&mut self.presentation_events);
            self.restart_level(game)?;
            self.presentation_events
                .splice(0..0, preceding_presentation);
            return Ok(None);
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
            return Ok(None);
        }
        Ok(pending_again)
    }

    fn apply_again_turns(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        target: Option<String>,
        undo_base_len: usize,
    ) -> Result<(), GridTransitionError<D>> {
        for _ in 0..MAX_AGAIN_TURNS_PER_INPUT {
            let previous_turn_sfx = self.begin_separate_turn_sfx_dedup();
            let result = match if let Some(target) = target.as_deref() {
                self.apply_model_input_to_target(game, target, InputId(0))
            } else {
                self.apply_model_input_to_current_level(game, InputId(0), undo_base_len, None)
            } {
                Ok(result) => result,
                Err(error) => {
                    self.end_separate_turn_sfx_dedup(previous_turn_sfx);
                    return Err(error);
                }
            };
            self.compress_undo_stack_to_turn_boundary(undo_base_len);
            if result.cancelled {
                self.end_separate_turn_sfx_dedup(previous_turn_sfx);
                return Ok(());
            }
            let has_again = result
                .items
                .iter()
                .any(|item| matches!(item, QueuedTurnItem::Effect(effect) if matches!(effect.effect, RuleEffect::Again)));
            let items: Vec<_> = result
                .items
                .into_iter()
                .filter(|item| !matches!(item, QueuedTurnItem::Effect(effect) if matches!(effect.effect, RuleEffect::Again)))
                .collect();
            let completion = self.apply_turn_completion(game, items, undo_base_len);
            self.compress_undo_stack_to_turn_boundary(undo_base_len);
            self.end_separate_turn_sfx_dedup(previous_turn_sfx);
            completion?;
            if !has_again {
                return Ok(());
            }
        }
        Ok(())
    }

    fn compress_undo_stack_to_turn_boundary(&mut self, undo_base_len: usize) {
        let keep_len = undo_base_len.saturating_add(1);
        if self.history.undo_len() > keep_len {
            self.history.truncate_undo(keep_len);
        }
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

    fn emit_model_operation_sfx(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        operation: ModelOperationSound,
    ) {
        for sound in &game.model_operation_sounds {
            if sound.operation != operation {
                continue;
            }
            let name = &sound.sfx_name;
            if self.should_emit_turn_sfx(name) {
                self.push_presentation_event(PresentationEventKind::Sound(SoundEvent::PlaySfx {
                    name: name.clone(),
                }));
            }
        }
    }

    pub fn apply_command(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        command: &str,
    ) -> Result<(), GridTransitionError<D>> {
        if self.is_waiting() {
            return Err(GridTransitionError::<D>::InvalidCommand(
                "cannot apply a command while a turn is waiting".to_string(),
            ));
        }
        match command {
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

        if let Some(effect) = parse_runtime_command(command, game.default_wait_ms) {
            self.apply_screen_effect(game, &effect, &HashMap::new())?;
            return Ok(());
        }
        if command.trim_start().starts_with("goto ") || command.trim_start().starts_with("start ") {
            return Err(GridTransitionError::<D>::InvalidCommand(
                command.to_string(),
            ));
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
        game: &LoadedGridGame<D, Size>,
        input: &str,
    ) -> Result<bool, GridTransitionError<D>> {
        if !self.current_scene_has_input_transition(game, input) {
            return Ok(false);
        }
        let previous_input = self.current_input.clone();
        self.current_input = Some(input.to_string());
        let undo_base_len = self.history.undo_len();
        let result = self.apply_focused_scene_input_transition(game, undo_base_len);
        self.current_input = previous_input;
        result?;
        Ok(true)
    }

    fn current_scene_has_input_transition(
        &self,
        game: &LoadedGridGame<D, Size>,
        input: &str,
    ) -> bool {
        game.scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
            .is_some_and(|screen| {
                screen.transitions.iter().any(|transition| {
                    transition_condition_mentions_input(&transition.trigger, input)
                })
            })
    }

    fn apply_input_name(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        input: &str,
    ) -> Result<(), GridTransitionError<D>> {
        let Some(input_id) = input_id_by_label(game, input) else {
            return Ok(());
        };

        if self.current_scene_accepts_model_input(game) {
            self.apply_input_in_current_mode(game, input_id)?;
        } else {
            let previous_input = self.current_input.clone();
            self.current_input = Some(input.to_string());
            let undo_base_len = self.history.undo_len();
            let result = self.apply_focused_scene_input_transition(game, undo_base_len);
            self.current_input = previous_input;
            result?;
        }

        Ok(())
    }

    fn apply_component_effect(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        effect: &str,
    ) -> Result<(), GridTransitionError<D>> {
        if self.current_scene_has_level_menu(game) && self.apply_level_menu_command(game, effect) {
            return Ok(());
        }

        Ok(())
    }

    fn condition_transition_effect(
        &self,
        game: &LoadedGridGame<D, Size>,
        forced_win_targets: &[Option<String>],
    ) -> Option<SceneEffect> {
        let Some(screen) = game
            .scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
        else {
            return None;
        };
        screen.transitions.iter().find_map(|transition| {
            let condition = match &transition.trigger {
                SceneTransitionTrigger::Condition(condition)
                | SceneTransitionTrigger::Signal(condition) => condition,
                SceneTransitionTrigger::SceneStart | SceneTransitionTrigger::LevelStart => {
                    return None;
                }
            };
            self.is_screen_condition_true_with_forced_win(game, condition, forced_win_targets)
                .then(|| transition.effect.clone())
        })
    }

    fn apply_focused_scene_input_transition(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        undo_base_len: usize,
    ) -> Result<(), GridTransitionError<D>> {
        let condition_effect = self.condition_transition_effect(game, &[]);
        self.resolve_turn_effects(game, Vec::new(), condition_effect, undo_base_len)
    }

    fn apply_model_level_clear(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        force_clear: bool,
    ) -> Result<Vec<QueuedTurnItem>, GridTransitionError<D>> {
        if self.active_level_index.is_none() {
            return Ok(Vec::new());
        }
        if force_clear || self.can_clear_state(game, &self.state) {
            self.mark_current_level_cleared();
            return self.apply_level_clear_hook(game, force_clear);
        }
        Ok(Vec::new())
    }

    fn apply_model_level_start(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        emit_events: bool,
    ) -> Result<(), GridTransitionError<D>> {
        let Some(level_index) = self.active_level_index else {
            return Ok(());
        };
        let mut state = self.state.clone();
        self.apply_persistent_vars(game, &mut state);
        let outcome = self.model_level_start_outcome(game, &state, level_index)?;
        if outcome.firings.is_empty() && outcome.commands.is_empty() && outcome.next_state == state
        {
            self.state = state;
            self.apply_persistent_vars_to_current_world(game);
            return Ok(());
        }
        let mut next_state = outcome.next_state.clone();
        self.capture_persistent_vars(game, &next_state);
        self.apply_persistent_vars(game, &mut next_state);
        self.state = next_state;
        self.apply_persistent_vars_to_current_world(game);
        if emit_events && !outcome.cancelled {
            let items = queued_turn_items_for_outcome(
                game,
                None,
                &outcome.commands,
                &outcome.firings,
                &outcome.next_state,
                self.input_execution_mode,
            );
            let undo_base_len = self.history.undo_len();
            self.resolve_turn_effects(game, items, None, undo_base_len)?;
        }
        Ok(())
    }

    fn start_current_world(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        emit_events: bool,
    ) -> Result<(), GridTransitionError<D>> {
        if self.level_started {
            return Ok(());
        }
        self.apply_model_level_start(game, emit_events)?;
        self.level_started = true;
        self.initial_state = self.state.clone();
        Ok(())
    }

    fn activate_level(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        level_index: usize,
        emit_events: bool,
    ) -> Result<(), GridTransitionError<D>> {
        let Some(level) = game.levels.get(level_index) else {
            return Ok(());
        };
        self.active_level_index = Some(level_index);
        self.selected_level_index = level_index;
        self.checkpoint_state = None;
        self.level_initial_state_override = None;
        self.state = level.initial_state.clone();
        let persistent_vars = self.persistent_vars.clone();
        apply_persistent_var_values(game, &persistent_vars, &mut self.state);
        self.level_started = false;
        self.start_current_world(game, emit_events)
    }

    fn model_level_start_outcome(
        &self,
        game: &LoadedGridGame<D, Size>,
        state: &GridState<D, Size>,
        level_index: usize,
    ) -> Result<GridLifecycleOutcome<D, Size>, GridTransitionError<D>> {
        let local_program = game.level_start_program_for_level(level_index);
        if let Some(program) = &game.level_start_program {
            let mut programs = vec![program];
            programs.extend(local_program);
            return self.lifecycle_programs_outcome(game, state, &programs);
        }
        if game.run_rules_on_level_start {
            let mut outcome = self.lifecycle_input_program_outcome(
                game,
                state,
                &game
                    .programs_for_level(level_index)
                    .expect("active level has validated program references"),
            )?;
            if !outcome.cancelled
                && let Some(program) = local_program
            {
                self.extend_lifecycle_outcome(game, program, &mut outcome)?;
            }
            return Ok(outcome);
        }
        self.lifecycle_programs_outcome(game, state, &local_program.into_iter().collect::<Vec<_>>())
    }

    fn lifecycle_programs_outcome(
        &self,
        game: &LoadedGridGame<D, Size>,
        state: &GridState<D, Size>,
        programs: &[&GridExecutableProgram<D>],
    ) -> Result<GridLifecycleOutcome<D, Size>, GridTransitionError<D>> {
        if self.requires_detailed_firings(game) {
            let outcome =
                transition_program_sequence_without_input_outcome(&game.game, state, programs)?;
            return Ok(GridLifecycleOutcome::<D, Size> {
                next_state: outcome.next_state,
                cancelled: outcome.cancelled,
                commands: outcome.commands,
                firings: GridCapturedFirings::Detailed(outcome.firings),
            });
        }
        let outcome =
            transition_program_sequence_without_input_summary_outcome(&game.game, state, programs)?;
        Ok(GridLifecycleOutcome::<D, Size> {
            next_state: outcome.next_state,
            cancelled: outcome.cancelled,
            commands: outcome.commands,
            firings: GridCapturedFirings::Summary(outcome.firings),
        })
    }

    fn lifecycle_input_program_outcome(
        &self,
        game: &LoadedGridGame<D, Size>,
        state: &GridState<D, Size>,
        programs: &[&GridExecutableProgram<D>],
    ) -> Result<GridLifecycleOutcome<D, Size>, GridTransitionError<D>> {
        if self.requires_detailed_firings(game) {
            let outcome =
                transition_program_sequence_outcome(&game.game, state, programs, InputId(0))?;
            return Ok(GridLifecycleOutcome::<D, Size> {
                next_state: outcome.next_state,
                cancelled: outcome.cancelled,
                commands: outcome.commands,
                firings: GridCapturedFirings::Detailed(outcome.firings),
            });
        }
        let outcome =
            transition_program_sequence_summary_outcome(&game.game, state, programs, InputId(0))?;
        Ok(GridLifecycleOutcome::<D, Size> {
            next_state: outcome.next_state,
            cancelled: outcome.cancelled,
            commands: outcome.commands,
            firings: GridCapturedFirings::Summary(outcome.firings),
        })
    }

    fn requires_detailed_firings(&self, game: &LoadedGridGame<D, Size>) -> bool {
        self.input_execution_mode.collects_trace()
            || (self.input_execution_mode.materializes_presentation()
                && !game.rule_animations.is_empty())
    }

    fn extend_lifecycle_outcome(
        &self,
        game: &LoadedGridGame<D, Size>,
        program: &GridExecutableProgram<D>,
        outcome: &mut GridLifecycleOutcome<D, Size>,
    ) -> Result<(), GridTransitionError<D>> {
        let next = self.lifecycle_input_program_outcome(game, &outcome.next_state, &[program])?;
        outcome.next_state = next.next_state;
        outcome.cancelled |= next.cancelled;
        outcome.commands.extend(next.commands);
        outcome.firings.extend(next.firings);
        Ok(())
    }

    fn apply_screen_effect_during_turn(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        effect: &SceneEffect,
        bindings: &HashMap<String, String>,
        pending_next_level: &mut Option<Option<String>>,
    ) -> Result<(), GridTransitionError<D>> {
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
            SceneEffect::Sequence { effects } => {
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
        game: &LoadedGridGame<D, Size>,
        force_clear: bool,
    ) -> Result<Vec<QueuedTurnItem>, GridTransitionError<D>> {
        if !force_clear && !game.is_goal_complete(&self.state) {
            return Ok(Vec::new());
        }
        if game.is_lose_complete(&self.state) {
            return Ok(Vec::new());
        }
        let mut items = Vec::new();
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
        let level_clear_program = game.level_clear_program_for_level(self.level_index());
        let programs = model_clear_program
            .into_iter()
            .chain(level_clear_program)
            .collect::<Vec<_>>();
        if !programs.is_empty() {
            let mut state = self.state.clone();
            self.apply_persistent_vars(game, &mut state);
            let outcome = self.lifecycle_programs_outcome(game, &state, &programs)?;
            self.state = outcome.next_state;
            self.capture_persistent_vars(game, &self.state.clone());
            if !outcome.cancelled {
                items.extend(queued_turn_items_for_outcome(
                    game,
                    None,
                    &outcome.commands,
                    &outcome.firings,
                    &self.state,
                    self.input_execution_mode,
                ));
            }
        }
        self.apply_persistent_vars_to_current_world(game);
        Ok(items)
    }

    fn is_screen_condition_true(
        &self,
        game: &LoadedGridGame<D, Size>,
        condition: &SceneExpr,
    ) -> bool {
        self.is_screen_condition_true_with_forced_win(game, condition, &[])
    }

    fn is_screen_condition_true_with_forced_win(
        &self,
        game: &LoadedGridGame<D, Size>,
        condition: &SceneExpr,
        forced_win_targets: &[Option<String>],
    ) -> bool {
        self.eval_screen_condition_bool(game, condition, forced_win_targets)
            .unwrap_or(false)
    }

    fn eval_screen_condition_bool(
        &self,
        game: &LoadedGridGame<D, Size>,
        condition: &SceneExpr,
        forced_win_targets: &[Option<String>],
    ) -> Option<bool> {
        match condition {
            SceneExpr::Bool(value) => Some(*value),
            SceneExpr::Binary { op, left, right } => match op {
                SceneBinaryOp::And => {
                    if !self.eval_screen_condition_bool(game, left, forced_win_targets)? {
                        return Some(false);
                    }
                    self.eval_screen_condition_bool(game, right, forced_win_targets)
                }
                SceneBinaryOp::Eq => {
                    let left = self.screen_condition_value(game, left)?;
                    let right = self.screen_condition_value(game, right)?;
                    Some(left == right)
                }
                SceneBinaryOp::In => {
                    let left = self.screen_condition_value(game, left)?;
                    self.screen_condition_set_contains(game, right, &left)
                }
                SceneBinaryOp::NotEq => {
                    let left = self.screen_condition_value(game, left)?;
                    let right = self.screen_condition_value(game, right)?;
                    Some(left != right)
                }
            },
            SceneExpr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if self.eval_screen_condition_bool(game, condition, forced_win_targets)? {
                    self.eval_screen_condition_bool(game, then_branch, forced_win_targets)
                } else {
                    self.eval_screen_condition_bool(game, else_branch, forced_win_targets)
                }
            }
            SceneExpr::Path(path) => {
                let condition = path.join(".");
                if let Some(value) = self.level_path_value(game, &condition) {
                    return value.parse::<bool>().ok();
                }
                if self.forced_win_condition_atom_true(game, &condition, forced_win_targets) {
                    return Some(true);
                }
                let scoped = self.condition_state_and_name(game, &condition);
                if let Some((state, condition_name)) = scoped.or_else(|| {
                    self.active_level_index
                        .is_some()
                        .then_some((&self.state, condition_name(&condition)))
                }) {
                    return Some(self.is_model_condition_true(game, state, condition_name));
                }
                self.screen_condition_value(game, &SceneExpr::Path(path.clone()))
                    .and_then(|value| value.parse::<bool>().ok())
            }
            _ => self
                .eval_effect_value(game, condition, &HashMap::new())
                .and_then(|value| scene_value_bool(&value)),
        }
    }

    fn forced_win_condition_atom_true(
        &self,
        game: &LoadedGridGame<D, Size>,
        condition: &str,
        forced_win_targets: &[Option<String>],
    ) -> bool {
        if forced_win_targets.is_empty() || condition_name(condition) != "win_conditions" {
            return false;
        }
        let parts = condition.split('.').collect::<Vec<_>>();
        let candidate = match parts.as_slice() {
            [_] => return forced_win_targets.iter().any(Option::is_none),
            [puzzle, _] => self.resolve_puzzle_target(game, puzzle),
            [scene, puzzle, _] => Some(((*scene).to_string(), (*puzzle).to_string())),
            _ => None,
        };
        let Some(candidate) = candidate else {
            return false;
        };
        forced_win_targets.iter().any(|target| {
            let Some(target) = target else {
                return self.can_force_clear_state(game, &self.state);
            };
            if self
                .resolve_puzzle_target(game, target)
                .is_none_or(|resolved| resolved != candidate)
            {
                return false;
            }
            self.puzzle_target_state(game, target)
                .is_some_and(|state| self.can_force_clear_state(game, state))
        })
    }

    fn is_model_condition_true(
        &self,
        game: &LoadedGridGame<D, Size>,
        state: &GridState<D, Size>,
        condition_name: &str,
    ) -> bool {
        if condition_name == "win_conditions" {
            return self.can_clear_state(game, state);
        }
        game.is_condition_true(condition_name, state)
            || game.is_variable_truthy(condition_name, state)
    }

    fn can_clear_state(&self, game: &LoadedGridGame<D, Size>, state: &GridState<D, Size>) -> bool {
        game.is_goal_complete(state) && !game.is_lose_complete(state)
    }

    fn can_force_clear_state(
        &self,
        game: &LoadedGridGame<D, Size>,
        state: &GridState<D, Size>,
    ) -> bool {
        !game.is_lose_complete(state)
    }

    fn puzzle_target_state<'a>(
        &'a self,
        game: &LoadedGridGame<D, Size>,
        target: &str,
    ) -> Option<&'a GridState<D, Size>> {
        let (scene_name, puzzle_name) = self.resolve_puzzle_target(game, target)?;
        self.scene_states
            .get(&scene_name)
            .and_then(|scene_state| scene_state.puzzles.get(&puzzle_name))
            .map(|puzzle| &puzzle.state)
    }

    fn screen_condition_value(
        &self,
        game: &LoadedGridGame<D, Size>,
        expr: &SceneExpr,
    ) -> Option<String> {
        match expr {
            SceneExpr::Path(path) if path.len() == 1 && path[0] == "input" => {
                self.current_input.clone()
            }
            SceneExpr::Path(path) => {
                let value = path.join(".");
                self.scene_path_value(game, &value)
                    .map(|value| scene_value_to_string(&value))
                    .or_else(|| {
                        level_index_from_value(game, &value)
                            .map(SceneValue::LevelRef)
                            .map(|value| scene_value_to_string(&value))
                    })
                    .or_else(|| self.scene_value_string(&value))
                    .or_else(|| self.level_path_value(game, &value))
                    .or_else(|| is_simple_identifier(&value).then(|| value.to_string()))
            }
            _ => self
                .eval_effect_value(game, expr, &HashMap::new())
                .map(|value| scene_value_to_string(&value)),
        }
    }

    fn screen_condition_set_contains(
        &self,
        game: &LoadedGridGame<D, Size>,
        set: &SceneExpr,
        value: &str,
    ) -> Option<bool> {
        match set {
            SceneExpr::Path(path) if path.len() == 1 => {
                Some(scene_builtin_value_set_contains(game, &path[0], value))
            }
            SceneExpr::Text(name) => Some(scene_builtin_value_set_contains(game, name, value)),
            _ => None,
        }
    }

    fn scene_path_value(&self, game: &LoadedGridGame<D, Size>, value: &str) -> Option<SceneValue> {
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

    fn level_path_value(&self, game: &LoadedGridGame<D, Size>, value: &str) -> Option<String> {
        let parts = value.split('.').collect::<Vec<_>>();
        let [target, "level", property] = parts.as_slice() else {
            return None;
        };
        let scene = self.level_scene_from_target(game, target);
        let level_index = self
            .scene_state()
            .and_then(|state| state.puzzles.get(*target))
            .and_then(|puzzle| puzzle.active_level_index)?;
        self.level_property_value(game, &scene, level_index, property)
    }

    fn level_property_value(
        &self,
        game: &LoadedGridGame<D, Size>,
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
        _game: &'a LoadedGridGame<D, Size>,
        condition: &'a str,
    ) -> Option<(&'a GridState<D, Size>, &'a str)> {
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
        game: &LoadedGridGame<D, Size>,
        effect: &SceneEffect,
        bindings: &HashMap<String, String>,
    ) -> Result<(), GridTransitionError<D>> {
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
                    self.push_presentation_event(PresentationEventKind::Message(
                        MessageEvent::Message { text },
                    ));
                    self.push_presentation_event(PresentationEventKind::Wait(WaitEvent::Wait {
                        milliseconds: game.default_wait_ms,
                    }));
                }
                Ok(())
            }
            SceneEffect::Wait { milliseconds } => {
                self.push_presentation_event(PresentationEventKind::Wait(WaitEvent::Wait {
                    milliseconds: milliseconds.unwrap_or(game.default_wait_ms),
                }));
                Ok(())
            }
            SceneEffect::Conditional { condition, effect } => {
                if self.is_screen_condition_true(game, condition) {
                    self.apply_screen_effect(game, effect, bindings)?;
                }
                Ok(())
            }
            SceneEffect::PlaySfx { name } => {
                if self.should_emit_turn_sfx(name) {
                    self.push_presentation_event(PresentationEventKind::Sound(
                        SoundEvent::PlaySfx { name: name.clone() },
                    ));
                }
                Ok(())
            }
            SceneEffect::PlayMusic { name } => {
                self.push_presentation_event(PresentationEventKind::Sound(SoundEvent::PlayMusic {
                    name: name.clone(),
                }));
                Ok(())
            }
            SceneEffect::PauseMusic { name } => {
                self.push_presentation_event(PresentationEventKind::Sound(
                    SoundEvent::PauseMusic { name: name.clone() },
                ));
                Ok(())
            }
            SceneEffect::ResumeMusic { name } => {
                self.push_presentation_event(PresentationEventKind::Sound(
                    SoundEvent::ResumeMusic { name: name.clone() },
                ));
                Ok(())
            }
            SceneEffect::StopMusic { name } => {
                self.push_presentation_event(PresentationEventKind::Sound(SoundEvent::StopMusic {
                    name: name.clone(),
                }));
                Ok(())
            }
            SceneEffect::Goto { scene, params } => {
                self.apply_screen_params(game, scene, params, bindings)?;
                self.goto_scene(game, scene);
                Ok(())
            }
            SceneEffect::Enter { scene, params } => {
                self.apply_screen_params(game, scene, params, bindings)?;
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
                self.goto_level_target(game, target, level, bindings)
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
                        self.apply_input_in_current_mode(game, input)?;
                    }
                }
                Ok(())
            }
            SceneEffect::Copy { source, target } => {
                self.copy_puzzle_state(game, source, target);
                Ok(())
            }
            SceneEffect::SetVariable { name, value } => {
                if self.scene_variable_kind(game, &self.focused_scene, name)
                    == Some(SceneVarKind::Signal)
                {
                    return self.apply_signal_assignment(game, name, value, bindings);
                }
                self.set_scene_variable(game, name, value, bindings)
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
            SceneEffect::Sequence { effects } => {
                for effect in effects {
                    self.apply_screen_effect(game, effect, bindings)?;
                }
                Ok(())
            }
        }
    }

    fn apply_screen_params(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        scene_name: &str,
        params: &[SceneEffectParam],
        bindings: &HashMap<String, String>,
    ) -> Result<(), GridTransitionError<D>> {
        if !params.is_empty() {
            self.create_scene(game, scene_name);
        }
        for param in params {
            match param {
                SceneEffectParam::Level(level) => {
                    let Some(index) = self.eval_effect_level_index(game, level, bindings) else {
                        return Err(GridTransitionError::<D>::InvalidCommand(format!(
                            "goto {scene_name}(<level>)"
                        )));
                    };
                    if scene_accepts_level(game, scene_name, index) {
                        let Some(world) = scene_input_world_instance(game, scene_name) else {
                            return Err(GridTransitionError::<D>::InvalidCommand(format!(
                                "goto {scene_name}(<level>)"
                            )));
                        };
                        self.with_execution_world(world, |session| {
                            let _ = session.activate_level(game, index, true);
                            session.history.clear();
                        });
                    } else {
                        return Err(GridTransitionError::<D>::InvalidCommand(format!(
                            "goto {scene_name}(<level>)"
                        )));
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
        Ok(())
    }

    pub fn restart_level(
        &mut self,
        game: &LoadedGridGame<D, Size>,
    ) -> Result<(), GridTransitionError<D>> {
        if self.is_waiting() {
            return Err(GridTransitionError::<D>::InvalidCommand(
                "cannot restart while a turn is waiting".to_string(),
            ));
        }
        if self.active_level_index.is_none() {
            return Ok(());
        }
        self.begin_new_level_attempt();
        self.emit_model_operation_sfx(game, ModelOperationSound::Restart);
        if let Some(checkpoint) = &self.checkpoint_state {
            let mut next = checkpoint.clone();
            self.apply_persistent_vars(game, &mut next);
            self.replace_state_if_changed_without_undo(game, next);
            self.apply_model_level_start(game, true)?;
            self.apply_level_start_transition(game)?;
            return Ok(());
        }
        if let Some(initial) = self.level_initial_state_override.clone() {
            let mut next = initial.state;
            self.apply_persistent_vars(game, &mut next);
            self.replace_state_if_changed_without_undo(game, next);
            self.apply_model_level_start(game, true)?;
            self.apply_level_start_transition(game)?;
            return Ok(());
        }
        let mut next = self.current_level(game).initial_state.clone();
        self.apply_persistent_vars(game, &mut next);
        self.replace_state_if_changed_without_undo(game, next);
        self.apply_model_level_start(game, true)?;
        self.apply_level_start_transition(game)?;
        Ok(())
    }

    fn begin_new_level_attempt(&mut self) {
        self.history.clear();
        self.presentation_events.clear();
    }

    pub fn advance_level(&mut self, game: &LoadedGridGame<D, Size>) {
        let scene = if scene_is_level_scene(game, &self.focused_scene) {
            self.focused_scene.clone()
        } else {
            initial_level_scene_name(game).to_string()
        };
        self.advance_level_in_scene(game, &scene);
    }

    pub fn previous_level(&mut self, game: &LoadedGridGame<D, Size>) {
        let scene = if scene_is_level_scene(game, &self.focused_scene) {
            self.focused_scene.clone()
        } else {
            initial_level_scene_name(game).to_string()
        };
        self.previous_level_in_scene(game, &scene);
    }

    fn advance_level_from_target(&mut self, game: &LoadedGridGame<D, Size>, target: &str) {
        let Some(world) = self.resolve_world_target(game, Some(target)) else {
            return;
        };
        let scene = world.scene.clone();
        self.with_execution_world(world, |session| {
            session.advance_level_in_scene(game, &scene);
        });
    }

    fn previous_level_from_target(&mut self, game: &LoadedGridGame<D, Size>, target: &str) {
        let Some(world) = self.resolve_world_target(game, Some(target)) else {
            return;
        };
        let scene = world.scene.clone();
        self.with_execution_world(world, |session| {
            session.previous_level_in_scene(game, &scene);
        });
    }

    fn level_scene_from_target(&self, game: &LoadedGridGame<D, Size>, target: &str) -> String {
        self.resolve_puzzle_target(game, target)
            .map(|(scene, _)| scene)
            .filter(|scene| scene_is_level_scene(game, scene))
            .unwrap_or_else(|| self.focused_scene.clone())
    }

    fn advance_level_in_scene(&mut self, game: &LoadedGridGame<D, Size>, scene: &str) {
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
        self.history.clear();
        self.selected_level_index = next_level;
    }

    fn previous_level_in_scene(&mut self, game: &LoadedGridGame<D, Size>, scene: &str) {
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
        self.history.clear();
        self.selected_level_index = previous_level;
    }

    pub fn start_level(&mut self, game: &LoadedGridGame<D, Size>, level_index: usize) {
        if level_index >= game.levels.len() {
            return;
        }

        let scene = initial_level_scene_name(game).to_string();
        self.select_focused_scene(game, &scene);
        let _ = self.activate_level(game, level_index, true);
        self.history.clear();
        let _ = self.apply_scene_start_transition(game);
        let _ = self.apply_level_start_transition(game);
    }

    pub fn start_level_from_state(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        level_index: usize,
        mut state: GridState<D, Size>,
        materialize_level_start: bool,
    ) -> Result<(), GridTransitionError<D>> {
        if level_index >= game.levels.len() {
            return Err(GridTransitionError::<D>::InvalidCommand(format!(
                "level index out of range: {level_index}"
            )));
        }

        let scene = initial_level_scene_name(game).to_string();
        self.select_focused_scene(game, &scene);
        self.active_level_index = Some(level_index);
        self.selected_level_index = level_index;
        self.checkpoint_state = None;
        self.level_initial_state_override = Some(GridLevelInitialStateOverride::<D, Size> {
            state: state.clone(),
        });
        self.history.clear();
        self.apply_persistent_vars(game, &mut state);
        self.state = state;
        self.level_started = false;
        if materialize_level_start {
            self.start_current_world(game, true)?;
        }
        let _ = self.apply_scene_start_transition(game);
        let _ = self.apply_level_start_transition(game);
        Ok(())
    }

    fn goto_level_target(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        target: &str,
        level: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) -> Result<(), GridTransitionError<D>> {
        let Some(index) = self.eval_effect_level_index(game, level, bindings) else {
            return Err(GridTransitionError::<D>::InvalidCommand(format!(
                "{target}.goto <level>"
            )));
        };
        if game.scenes.iter().any(|scene| scene.name == target) {
            if !scene_accepts_level(game, target, index) {
                return Err(GridTransitionError::<D>::InvalidCommand(format!(
                    "{target}.goto <level>"
                )));
            }
            let Some(world) = self.resolve_world_target(game, Some(target)) else {
                return Err(GridTransitionError::<D>::InvalidCommand(format!(
                    "{target}.goto <level>"
                )));
            };
            self.with_execution_world(world, |session| {
                let _ = session.activate_level(game, index, true);
                session.history.clear();
            });
            self.goto_scene(game, target);
            return Ok(());
        }
        let value = game
            .levels
            .get(index)
            .map(|level| level.name.clone())
            .ok_or_else(|| {
                GridTransitionError::<D>::InvalidCommand(format!("{target}.goto <level>"))
            })?;
        self.load_puzzle_state(game, target, &value, bindings);
        Ok(())
    }

    pub fn undo(&mut self, game: &LoadedGridGame<D, Size>) {
        if self.active_level_index.is_none() {
            return;
        }
        if let Some(previous) = self.history.pop_undo() {
            self.emit_model_operation_sfx(game, ModelOperationSound::Undo);
            let state = self.state.clone();
            self.history.push_redo(state);
            self.state = previous;
            self.apply_persistent_vars_to_current_world(game);
        }
    }

    pub fn redo(&mut self, game: &LoadedGridGame<D, Size>) {
        if self.active_level_index.is_none() {
            return;
        }
        if let Some(next) = self.history.pop_redo() {
            let state = self.state.clone();
            self.history.push_undo(state);
            self.state = next;
            self.apply_persistent_vars_to_current_world(game);
        }
    }

    fn replace_state_if_changed(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        next: GridState<D, Size>,
    ) -> bool {
        let undo_state = self
            .input_execution_mode
            .materializes_presentation()
            .then(|| self.state.clone());
        self.replace_state_if_changed_with_undo(game, next, undo_state)
    }

    fn replace_state_if_changed_without_undo(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        next: GridState<D, Size>,
    ) -> bool {
        self.replace_state_if_changed_with_undo(game, next, None)
    }

    fn replace_state_if_changed_with_undo(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        mut next: GridState<D, Size>,
        undo_state: Option<GridState<D, Size>>,
    ) -> bool {
        if self.active_level_index.is_none() {
            self.state = next;
            return false;
        }
        self.capture_persistent_vars(game, &next);
        self.apply_persistent_vars(game, &mut next);
        if states_equal_ignoring_persistent_vars(game, &next, &self.state) {
            self.state = next;
            self.apply_persistent_vars_to_current_world(game);
            return false;
        }

        if let Some(undo_state) = undo_state {
            self.history.push_undo(undo_state);
        }
        self.state = next;
        self.history.clear_redo();
        self.apply_persistent_vars_to_current_world(game);
        true
    }

    fn goto_scene(&mut self, game: &LoadedGridGame<D, Size>, name: &str) {
        self.create_scene(game, name);
        self.visible_scenes.clear();
        self.show_scene(game, name);
        self.focus_history.clear();
        self.focus_scene(game, name);
    }

    fn enter_scene(&mut self, game: &LoadedGridGame<D, Size>, name: &str) {
        self.create_scene(game, name);
        if self.focused_scene != name {
            self.focus_history.push(self.focused_scene.clone());
        }
        self.show_scene(game, name);
        self.focus_scene(game, name);
    }

    fn back_or_initial(&mut self, game: &LoadedGridGame<D, Size>) {
        let current = self.focused_scene.clone();
        let previous = self
            .focus_history
            .pop()
            .unwrap_or_else(|| initial_scene_name(game).to_string());
        self.hide_scene_only(&current);
        self.focus_scene(game, &previous);
    }

    fn create_scene(&mut self, game: &LoadedGridGame<D, Size>, name: &str) {
        if !self.scene_states.contains_key(name) {
            self.reset_scene_state(game, name);
        }
    }

    fn delete_scene(&mut self, game: &LoadedGridGame<D, Size>, name: &str) {
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
            self.route_inputs_to_focused_scene(game);
        }
    }

    fn show_scene(&mut self, game: &LoadedGridGame<D, Size>, name: &str) {
        self.create_scene(game, name);
        if !self.visible_scenes.iter().any(|screen| screen == name) {
            self.visible_scenes.push(name.to_string());
        }
    }

    fn hide_scene(&mut self, game: &LoadedGridGame<D, Size>, name: &str) {
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
            self.route_inputs_to_focused_scene(game);
        }
    }

    fn hide_scene_only(&mut self, name: &str) {
        self.visible_scenes.retain(|screen| screen != name);
    }

    fn toggle_scene(&mut self, game: &LoadedGridGame<D, Size>, name: &str) {
        if self.visible_scenes.iter().any(|screen| screen == name) {
            self.hide_scene(game, name);
        } else {
            self.show_scene(game, name);
        }
    }

    fn focus_scene(&mut self, game: &LoadedGridGame<D, Size>, name: &str) {
        self.select_focused_scene(game, name);
        if self.routed_world.scene == name {
            let _ = self.start_current_world(game, true);
        }
        let _ = self.apply_scene_start_transition(game);
        let _ = self.apply_level_start_transition(game);
    }

    fn select_focused_scene(&mut self, game: &LoadedGridGame<D, Size>, name: &str) {
        self.create_scene(game, name);
        self.show_scene(game, name);
        self.focused_scene = name.to_string();
        self.route_inputs_to_focused_scene(game);
    }

    fn route_inputs_to_focused_scene(&mut self, game: &LoadedGridGame<D, Size>) {
        if let Some(world) = scene_input_world_instance(game, &self.focused_scene) {
            self.routed_world = world;
        }
    }

    fn apply_scene_start_transition(
        &mut self,
        game: &LoadedGridGame<D, Size>,
    ) -> Result<(), GridTransitionError<D>> {
        self.apply_lifecycle_transition(game, SceneTransitionTrigger::SceneStart)
    }

    fn apply_level_start_transition(
        &mut self,
        game: &LoadedGridGame<D, Size>,
    ) -> Result<(), GridTransitionError<D>> {
        self.apply_lifecycle_transition(game, SceneTransitionTrigger::LevelStart)
    }

    fn apply_lifecycle_transition(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        trigger: SceneTransitionTrigger,
    ) -> Result<(), GridTransitionError<D>> {
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
        game: &LoadedGridGame<D, Size>,
        expr: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) -> Option<String> {
        self.eval_effect_value(game, expr, bindings)
            .map(|value| scene_value_to_string(&value))
    }

    fn eval_effect_level_index(
        &self,
        game: &LoadedGridGame<D, Size>,
        expr: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) -> Option<usize> {
        match expr {
            SceneExpr::Int(index) => {
                return level_index_by_omitted_collection_ordinal(
                    game,
                    usize::try_from(*index).ok()?,
                );
            }
            SceneExpr::Text(id) => {
                return level_index_by_omitted_collection_id(game, id);
            }
            SceneExpr::LevelSelector {
                collection, key, ..
            } => return level_index_by_selector(game, collection, key),
            SceneExpr::Path(path) if path.len() == 1 => {
                if let Some(value) = bindings.get(&path[0]) {
                    if let Ok(index) = value.parse::<usize>() {
                        return level_index_by_omitted_collection_ordinal(game, index);
                    }
                    return level_index_by_omitted_collection_id(game, value);
                }
                if path[0] == "level" {
                    return self.active_level_index;
                }
                return self
                    .scene_value(&path[0])
                    .and_then(|value| level_index_from_scene_value(game, value))
                    .or_else(|| level_index_by_omitted_collection_id(game, &path[0]));
            }
            _ => {}
        }
        self.eval_effect_value(game, expr, bindings)
            .and_then(|value| {
                matches!(value, SceneValue::LevelRef(_))
                    .then(|| level_index_from_scene_value(game, &value))
                    .flatten()
            })
    }

    fn eval_effect_value(
        &self,
        game: &LoadedGridGame<D, Size>,
        expr: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) -> Option<SceneValue> {
        match expr {
            SceneExpr::Bool(value) => Some(SceneValue::Bool(*value)),
            SceneExpr::Int(value) => Some(SceneValue::Int(*value)),
            SceneExpr::Text(value) => Some(SceneValue::Text(value.clone())),
            SceneExpr::LevelSelector {
                collection,
                key,
                property,
            } => {
                let index = level_index_by_selector(game, collection, key)?;
                if let Some(field) = property {
                    return level_ref_field(game, self, index, field);
                }
                Some(SceneValue::LevelRef(index))
            }
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
            SceneExpr::Call { name, args } if name == "join" => {
                let mut out = String::new();
                for arg in args {
                    let value = self.eval_effect_value(game, arg, bindings)?;
                    out.push_str(&scene_value_to_string(&value));
                }
                Some(SceneValue::Text(out))
            }
            SceneExpr::Call { name, args } if name == "next" && args.len() == 1 => {
                let index = self.eval_effect_level_index(game, &args[0], bindings)?;
                Some(SceneValue::LevelRef(
                    index.saturating_add(1).min(game.levels.len() - 1),
                ))
            }
            SceneExpr::Binary { op, left, right } => {
                let left = self.eval_effect_value(game, left, bindings)?;
                match op {
                    SceneBinaryOp::And => {
                        if !scene_value_bool(&left)? {
                            return Some(SceneValue::Bool(false));
                        }
                        let right = self.eval_effect_value(game, right, bindings)?;
                        Some(SceneValue::Bool(scene_value_bool(&right)?))
                    }
                    SceneBinaryOp::Eq => {
                        let right = self.eval_effect_value(game, right, bindings)?;
                        Some(SceneValue::Bool(left == right))
                    }
                    SceneBinaryOp::In => {
                        Some(SceneValue::Bool(self.screen_condition_set_contains(
                            game,
                            right,
                            &scene_value_to_string(&left),
                        )?))
                    }
                    SceneBinaryOp::NotEq => {
                        let right = self.eval_effect_value(game, right, bindings)?;
                        Some(SceneValue::Bool(left != right))
                    }
                }
            }
            SceneExpr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.eval_effect_value(game, condition, bindings)?;
                if scene_value_bool(&condition)? {
                    self.eval_effect_value(game, then_branch, bindings)
                } else {
                    self.eval_effect_value(game, else_branch, bindings)
                }
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

    fn scene_variable_kind(
        &self,
        game: &LoadedGridGame<D, Size>,
        scene_name: &str,
        name: &str,
    ) -> Option<SceneVarKind> {
        game.scenes
            .iter()
            .find(|scene| scene.name == scene_name)
            .and_then(|scene| {
                scene
                    .state
                    .variables
                    .iter()
                    .find(|variable| variable.name == name)
            })
            .map(|variable| variable.kind)
    }

    fn scene_signal_default(
        &self,
        game: &LoadedGridGame<D, Size>,
        scene_name: &str,
        name: &str,
    ) -> Option<SceneValue> {
        game.scenes
            .iter()
            .find(|scene| scene.name == scene_name)
            .and_then(|scene| {
                scene
                    .state
                    .variables
                    .iter()
                    .find(|variable| variable.name == name)
            })
            .filter(|variable| variable.kind == SceneVarKind::Signal)
            .map(|variable| variable.default.clone())
    }

    fn apply_signal_assignment(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        name: &str,
        expr: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) -> Result<(), GridTransitionError<D>> {
        let Some(value) = self.eval_effect_value(game, expr, bindings) else {
            return Err(GridTransitionError::<D>::InvalidCommand(name.to_string()));
        };
        let scene_name = self.focused_scene.clone();
        let previous_input = self.current_input.clone();
        let previous_value = self
            .scene_states
            .get(&scene_name)
            .and_then(|state| state.values.get(name))
            .cloned();
        if let Some(state) = self.scene_states.get_mut(&scene_name) {
            state.values.insert(name.to_string(), value.clone());
        }
        if name == "input" {
            self.current_input = Some(scene_value_to_string(&value));
        }
        let undo_base_len = self.history.undo_len();
        let result = self.apply_focused_scene_input_transition(game, undo_base_len);
        let default_value = self.scene_signal_default(game, &scene_name, name);
        if let Some(state) = self.scene_states.get_mut(&scene_name) {
            if let Some(default) = default_value {
                state.values.insert(name.to_string(), default);
            } else if let Some(previous) = previous_value {
                state.values.insert(name.to_string(), previous);
            } else {
                state.values.remove(name);
            }
        }
        self.current_input = previous_input;
        result
    }

    fn set_scene_variable(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        name: &str,
        expr: &SceneExpr,
        bindings: &HashMap<String, String>,
    ) -> Result<(), GridTransitionError<D>> {
        let Some(value) = self.eval_effect_value(game, expr, bindings) else {
            return Err(GridTransitionError::<D>::InvalidCommand(name.to_string()));
        };
        let scene_name = self.focused_scene.clone();
        if let Some(mutable) = scene_variable_mutability(game, &scene_name, name) {
            if !mutable {
                return Err(GridTransitionError::<D>::InvalidCommand(name.to_string()));
            }
            if let Some(state) = self.scene_states.get_mut(&scene_name)
                && state.values.contains_key(name)
            {
                state.values.insert(name.to_string(), value);
                return Ok(());
            }
        }
        if let Some(mutable) = session_variable_mutability(game, name) {
            if !mutable {
                return Err(GridTransitionError::<D>::InvalidCommand(name.to_string()));
            }
            self.session_values.insert(name.to_string(), value);
            return Ok(());
        }
        Err(GridTransitionError::<D>::InvalidCommand(name.to_string()))
    }

    fn reset_scene_state(&mut self, game: &LoadedGridGame<D, Size>, name: &str) {
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

    fn default_scene_state(
        &self,
        game: &LoadedGridGame<D, Size>,
        name: &str,
    ) -> GridSceneRuntimeState<D, Size> {
        let Some(screen) = game.scenes.iter().find(|screen| screen.name == name) else {
            return GridSceneRuntimeState::<D, Size>::default();
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
                let (state, active_level_index) = match &puzzle.initializer {
                    ScenePuzzleInitializer::CurrentLevel => {
                        first_level_index_for_scene(game, name, None)
                            .or_else(|| (!game.levels.is_empty()).then_some(0))
                            .and_then(|level_index| {
                                game.levels
                                    .get(level_index)
                                    .map(|level| (level.initial_state.clone(), Some(level_index)))
                            })
                            .unwrap_or_else(|| (neutral_state(game), None))
                    }
                    ScenePuzzleInitializer::Level(level_name) => game
                        .levels
                        .iter()
                        .enumerate()
                        .find(|(_, level)| level.name == *level_name)
                        .map(|(index, level)| (level.initial_state.clone(), Some(index)))
                        .unwrap_or_else(|| (neutral_state(game), None)),
                };
                let persistent_vars = persistent_var_values(game, &state);
                (
                    puzzle.name.clone(),
                    GridWorldInstanceState::<D, Size> {
                        model: puzzle.model.clone(),
                        initial_state: state.clone(),
                        checkpoint_state: None,
                        state,
                        active_level_index,
                        level_started: false,
                        history: SessionHistory::new(),
                        cleared_levels: vec![false; game.levels.len()],
                        selected_level_index: active_level_index.unwrap_or(0),
                        level_initial_state_override: None,
                        persistent_vars,
                        last_transition_trace: None,
                    },
                )
            })
            .collect();

        GridSceneRuntimeState::<D, Size> { values, puzzles }
    }

    fn resolve_puzzle_target(
        &self,
        game: &LoadedGridGame<D, Size>,
        target: &str,
    ) -> Option<(String, String)> {
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

    fn resolve_world_target(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        target: Option<&str>,
    ) -> Option<WorldInstanceId> {
        let Some(target) = target else {
            return Some(self.current_world_id().clone());
        };
        let (scene, puzzle) = self.resolve_puzzle_target(game, target)?;
        scene_puzzle_initializer(game, &scene, &puzzle)?;
        self.create_scene(game, &scene);
        let world = WorldInstanceId { scene, puzzle };
        self.world_state(&world).is_some().then_some(world)
    }

    fn copy_puzzle_state(&mut self, game: &LoadedGridGame<D, Size>, source: &str, target: &str) {
        let Some((source_scene, source_puzzle)) = self.resolve_puzzle_target(game, source) else {
            return;
        };
        let Some((target_scene, target_puzzle)) = self.resolve_puzzle_target(game, target) else {
            return;
        };
        self.create_scene(game, &source_scene);
        self.create_scene(game, &target_scene);
        let Some(state) = self
            .scene_states
            .get(&source_scene)
            .and_then(|screen| screen.puzzles.get(&source_puzzle))
            .map(|puzzle| puzzle.state.clone())
        else {
            return;
        };
        let target = WorldInstanceId {
            scene: target_scene,
            puzzle: target_puzzle,
        };
        self.with_execution_world(target, |session| {
            let mut state = state;
            session.apply_persistent_vars(game, &mut state);
            session.replace_state_if_changed(game, state);
        });
    }

    fn reset_puzzle_state(&mut self, game: &LoadedGridGame<D, Size>, target: &str) {
        let Some(world) = self.resolve_world_target(game, Some(target)) else {
            return;
        };
        self.with_execution_world(world, |session| {
            session.emit_model_operation_sfx(game, ModelOperationSound::Restart);
            let mut state = session
                .checkpoint_state
                .clone()
                .unwrap_or_else(|| session.initial_state.clone());
            session.apply_persistent_vars(game, &mut state);
            session.replace_state_if_changed(game, state);
        });
    }

    fn save_checkpoint(&mut self, game: &LoadedGridGame<D, Size>, target: Option<&str>) {
        let Some(world) = self.resolve_world_target(game, target) else {
            return;
        };
        self.with_execution_world(world, |session| {
            if session.active_level_index.is_some() {
                session.checkpoint_state = Some(session.state.clone());
            }
        });
    }

    fn clear_checkpoint(&mut self, game: &LoadedGridGame<D, Size>, target: Option<&str>) {
        let Some(world) = self.resolve_world_target(game, target) else {
            return;
        };
        self.with_execution_world(world, |session| session.checkpoint_state = None);
    }

    fn load_puzzle_state(
        &mut self,
        game: &LoadedGridGame<D, Size>,
        target: &str,
        source: &str,
        bindings: &HashMap<String, String>,
    ) {
        let Some(level_index) = self.eval_puzzle_level_ref(game, target, source, bindings) else {
            return;
        };
        let Some(world) = self.resolve_world_target(game, Some(target)) else {
            return;
        };
        self.with_execution_world(world, |session| {
            session.begin_new_level_attempt();
            let _ = session.activate_level(game, level_index, false);
            session.initial_state = session.state.clone();
        });
    }

    fn eval_puzzle_level_ref(
        &self,
        game: &LoadedGridGame<D, Size>,
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
            return current_puzzle.and_then(|puzzle| puzzle.active_level_index);
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

    fn current_scene_has_level_menu(&self, game: &LoadedGridGame<D, Size>) -> bool {
        game.scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
            .is_some_and(|screen| screen.components.iter().any(component_has_level_menu))
    }

    fn current_scene_accepts_model_input(&self, game: &LoadedGridGame<D, Size>) -> bool {
        if game.scenes.is_empty() {
            return self.active_level_index.is_some();
        }
        scene_is_level_scene(game, &self.focused_scene)
    }

    fn apply_level_menu_command(&mut self, game: &LoadedGridGame<D, Size>, command: &str) -> bool {
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
        game: &LoadedGridGame<D, Size>,
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

    fn level_menu_cursor_position(
        &self,
        game: &LoadedGridGame<D, Size>,
        level_indices: &[usize],
    ) -> usize {
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
        game: &LoadedGridGame<D, Size>,
        level_indices: &[usize],
        position: usize,
    ) {
        self.selected_level_index = level_indices
            .get(position)
            .copied()
            .unwrap_or_else(|| game.levels.len() + position.saturating_sub(level_indices.len()));
    }

    fn current_level_menu<'a>(
        &self,
        game: &'a LoadedGridGame<D, Size>,
    ) -> Option<&'a LevelMenuDef> {
        game.scenes
            .iter()
            .find(|screen| screen.name == self.focused_scene)
            .and_then(|screen| find_level_menu(&screen.components))
    }
}

fn initial_scene_name<const D: usize, Size: GridSize<D>>(game: &LoadedGridGame<D, Size>) -> &str {
    game.scenes
        .first()
        .map(|screen| screen.name.as_str())
        .unwrap_or("playing")
}

fn initial_world_instance<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    initial_scene: &str,
) -> Option<WorldInstanceId> {
    scene_input_world_instance(game, initial_scene).or_else(|| {
        game.scenes.iter().find_map(|scene| {
            scene_primary_puzzle_name(game, &scene.name).map(|puzzle| WorldInstanceId {
                scene: scene.name.clone(),
                puzzle,
            })
        })
    })
}

fn scene_input_world_instance<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    scene_name: &str,
) -> Option<WorldInstanceId> {
    let scene = game.scenes.iter().find(|scene| scene.name == scene_name)?;
    if let Some(target) = scene.puzzle_rule.as_ref().map(|rule| rule.target.as_str()) {
        let parts = target.split('.').collect::<Vec<_>>();
        match parts.as_slice() {
            [puzzle]
                if scene
                    .state
                    .puzzles
                    .iter()
                    .any(|candidate| candidate.name == *puzzle) =>
            {
                return Some(WorldInstanceId {
                    scene: scene_name.to_string(),
                    puzzle: (*puzzle).to_string(),
                });
            }
            [target_scene, puzzle] => {
                return Some(WorldInstanceId {
                    scene: (*target_scene).to_string(),
                    puzzle: (*puzzle).to_string(),
                });
            }
            [target_scene] => {
                if let Some(puzzle) = scene_primary_puzzle_name(game, target_scene) {
                    return Some(WorldInstanceId {
                        scene: (*target_scene).to_string(),
                        puzzle,
                    });
                }
            }
            _ => {}
        }
    }
    scene_primary_puzzle_name(game, scene_name).map(|puzzle| WorldInstanceId {
        scene: scene_name.to_string(),
        puzzle,
    })
}

fn neutral_state<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
) -> GridState<D, Size> {
    game.levels
        .first()
        .expect("loaded grid game must define at least one level")
        .initial_state
        .clone()
}

fn initial_level_scene_name<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
) -> &str {
    game.scenes
        .iter()
        .find(|screen| {
            scene_is_level_scene(game, &screen.name)
                && !game.levels.iter().any(|level| level.puzzle == screen.name)
        })
        .or_else(|| {
            game.scenes
                .iter()
                .find(|screen| scene_is_level_scene(game, &screen.name))
        })
        .map(|screen| screen.name.as_str())
        .unwrap_or_else(|| initial_scene_name(game))
}

fn scene_is_level_scene<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    name: &str,
) -> bool {
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

fn game_has_scene_level_owner<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
) -> bool {
    game.scenes.iter().any(|scene| {
        scene.puzzle_rule.is_some()
            || scene
                .state
                .puzzles
                .iter()
                .any(|puzzle| puzzle.initializer == ScenePuzzleInitializer::CurrentLevel)
    })
}

fn persistent_var_values<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    state: &GridState<D, Size>,
) -> Vec<i64> {
    game.persistent_vars
        .iter()
        .map(|var| state.variable_value(*var).unwrap_or(0))
        .collect()
}

fn persistent_var_default_values<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
) -> Vec<i64> {
    game.persistent_vars
        .iter()
        .map(|var| {
            game.levels
                .first()
                .and_then(|level| level.initial_state.variable_value(*var))
                .unwrap_or(0)
        })
        .collect()
}

fn persistent_var_index_by_name<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    name: &str,
) -> Option<usize> {
    game.persistent_vars.iter().position(|var| {
        game.variable_labels
            .get(var)
            .is_some_and(|label| label == name)
    })
}

fn apply_persistent_var_values<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    values: &[i64],
    state: &mut GridState<D, Size>,
) {
    for (index, var) in game.persistent_vars.iter().enumerate() {
        if let Some(value) = values.get(index) {
            let _ = state.set_visible_variable(*var, *value);
        }
    }
}

fn states_equal_ignoring_persistent_vars<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    left: &GridState<D, Size>,
    right: &GridState<D, Size>,
) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    for var in &game.persistent_vars {
        let _ = left.set_visible_variable(*var, 0);
        let _ = right.set_visible_variable(*var, 0);
    }
    left == right
}

fn scene_puzzle_initializer<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
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

fn scene_primary_puzzle_name<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    scene_name: &str,
) -> Option<String> {
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

fn scene_value_bool(value: &SceneValue) -> Option<bool> {
    match value {
        SceneValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn scene_value_from_effect_atom<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    value: &str,
) -> SceneValue {
    level_index_from_value(game, value)
        .map(SceneValue::LevelRef)
        .unwrap_or_else(|| SceneValue::Symbol(value.to_string()))
}

fn scene_builtin_value_set_contains<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    set: &str,
    value: &str,
) -> bool {
    match set {
        "directions" => ["up", "down", "left", "right"]
            .into_iter()
            .any(|direction| direction == value && input_id_by_label(game, direction).is_some()),
        "horizontal" => ["left", "right"]
            .into_iter()
            .any(|direction| direction == value && input_id_by_label(game, direction).is_some()),
        "vertical" => ["up", "down"]
            .into_iter()
            .any(|direction| direction == value && input_id_by_label(game, direction).is_some()),
        _ => false,
    }
}

fn level_index_from_scene_value<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    value: &SceneValue,
) -> Option<usize> {
    match value {
        SceneValue::LevelRef(index) => (*index < game.levels.len()).then_some(*index),
        SceneValue::Int(index) => usize::try_from(*index)
            .ok()
            .filter(|index| *index < game.levels.len()),
        SceneValue::Text(value) | SceneValue::Symbol(value) => level_index_from_value(game, value),
        SceneValue::Bool(_) => None,
    }
}

fn scene_value_field<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
    value: &SceneValue,
    field: &str,
) -> Option<SceneValue> {
    match value {
        SceneValue::LevelRef(index) => level_ref_field(game, session, *index, field),
        _ => None,
    }
}

fn level_ref_field<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
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

fn scene_variable_mutability<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    scene_name: &str,
    name: &str,
) -> Option<bool> {
    game.scenes
        .iter()
        .find(|scene| scene.name == scene_name)?
        .state
        .variables
        .iter()
        .find(|variable| variable.name == name)
        .map(|variable| variable.mutable)
}

fn session_variable_mutability<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    name: &str,
) -> Option<bool> {
    game.variables
        .iter()
        .find(|variable| variable.name == name)
        .map(|variable| variable.mutable)
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
    if let Some(rest) = command_text.strip_prefix("current_level = ") {
        return Some(SceneEffect::SetCurrentLevel {
            level: parse_runtime_level_expr(rest.trim())?,
        });
    }
    if let Some(rest) = command_text.strip_prefix("level.cleared = ") {
        return Some(SceneEffect::SetLevelCleared {
            level: None,
            cleared: parse_runtime_bool(rest.trim())?,
        });
    }
    if let Some((selector, cleared)) = command_text.split_once(".cleared = ") {
        let level = parse_runtime_level_selector_expr(selector.trim())?;
        return Some(SceneEffect::SetLevelCleared {
            level: Some(level),
            cleared: parse_runtime_bool(cleared.trim())?,
        });
    }
    if let Some((name, value)) = parse_runtime_variable_assignment(command_text) {
        return Some(SceneEffect::SetVariable {
            name: name.to_string(),
            value: parse_runtime_expr(value)?,
        });
    }
    if let Some(text) = command_text.strip_prefix("message ") {
        return Some(SceneEffect::Message {
            text: parse_runtime_expr(text.trim())?,
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
        "start" => Some(SceneEffect::Sequence {
            effects: vec![
                SceneEffect::Reset {
                    scene: screen.to_string(),
                },
                SceneEffect::Goto {
                    scene: screen.to_string(),
                    params,
                },
            ],
        }),
        _ => None,
    }
}

fn parse_runtime_variable_assignment(value: &str) -> Option<(&str, &str)> {
    let (name, rhs) = value.split_once('=')?;
    let name = name.trim();
    let rhs = rhs.trim();
    if rhs.is_empty() || !is_simple_identifier(name) || matches!(name, "current_level" | "level") {
        return None;
    }
    Some((name, rhs))
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
        let params = parse_runtime_params(args)?;
        return Some((screen.trim(), params));
    }
    Some((value, Vec::new()))
}

fn parse_runtime_params(value: &str) -> Option<Vec<SceneEffectParam>> {
    parse_scene_effect_params(value).ok()
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

fn invalid_puzzle_target_error<const D: usize>(target: &str) -> GridTransitionError<D> {
    GridTransitionError::<D>::InvalidCommand(format!("unknown puzzle target: {target}"))
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

pub fn runtime_effects_for_outcome<const D: usize>(
    rule_effects: &HashMap<RuleId, Vec<RuleEffect>>,
    commands: &[TransitionCommand],
    firings: &[GridRuleFiring<D>],
) -> Vec<RuleEffect> {
    runtime_effects_for_firings(rule_effects, commands, firings)
}

trait GridFiringView<const D: usize> {
    fn rule(&self) -> RuleId;
    fn detailed(&self) -> Option<&GridRuleFiring<D>>;
}

impl<const D: usize> GridFiringView<D> for GridRuleFiring<D> {
    fn rule(&self) -> RuleId {
        self.rule
    }

    fn detailed(&self) -> Option<&GridRuleFiring<D>> {
        Some(self)
    }
}

impl<const D: usize> GridFiringView<D> for GridRuleFiringSummary {
    fn rule(&self) -> RuleId {
        self.rule
    }

    fn detailed(&self) -> Option<&GridRuleFiring<D>> {
        None
    }
}

fn runtime_effects_for_firings<const D: usize, Firing: GridFiringView<D>>(
    rule_effects: &HashMap<RuleId, Vec<RuleEffect>>,
    commands: &[TransitionCommand],
    firings: &[Firing],
) -> Vec<RuleEffect> {
    let effects = firings
        .iter()
        .filter_map(|firing| rule_effects.get(&firing.rule()))
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    if !effects.is_empty() {
        return effects;
    }
    commands
        .iter()
        .map(|command| match command {
            TransitionCommand::Win => RuleEffect::Win,
            TransitionCommand::Restart => RuleEffect::Restart,
            TransitionCommand::NextLevel => RuleEffect::NextLevel,
            TransitionCommand::Again => RuleEffect::Again,
            TransitionCommand::Checkpoint => RuleEffect::Checkpoint,
            TransitionCommand::ClearCheckpoint => RuleEffect::ClearCheckpoint,
        })
        .collect()
}

fn queued_effects_for_outcome<const D: usize, Size: GridSize<D>, Firing: GridFiringView<D>>(
    game: &LoadedGridGame<D, Size>,
    target: Option<&str>,
    commands: &[TransitionCommand],
    firings: &[Firing],
) -> Vec<QueuedRuleEffect> {
    runtime_effects_for_firings(&game.rule_effects, commands, firings)
        .into_iter()
        .map(|effect| QueuedRuleEffect {
            target: target.map(str::to_string),
            effect,
        })
        .collect()
}

fn rule_effect_is_presentation_only(effect: &RuleEffect) -> bool {
    matches!(
        effect,
        RuleEffect::PlaySfx { .. }
            | RuleEffect::PlayMusic { .. }
            | RuleEffect::PauseMusic { .. }
            | RuleEffect::ResumeMusic { .. }
            | RuleEffect::StopMusic { .. }
            | RuleEffect::Wait { .. }
            | RuleEffect::WaitAnimation
            | RuleEffect::EmitAnimation { .. }
            | RuleEffect::Message { .. }
    )
}

fn rule_boundary_pauses<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    firings: &[GridRuleFiring<D>],
) -> bool {
    firings
        .last()
        .and_then(|firing| game.rule_effects.get(&firing.rule))
        .is_some_and(|effects| {
            effects.iter().any(|effect| {
                matches!(
                    effect,
                    RuleEffect::Wait { .. }
                        | RuleEffect::WaitAnimation
                        | RuleEffect::Message { .. }
                )
            })
        })
}

fn queued_turn_items_for_outcome<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    target: Option<&str>,
    commands: &[TransitionCommand],
    firings: &GridCapturedFirings<D>,
    next_state: &GridState<D, Size>,
    mode: InputExecutionMode,
) -> Vec<QueuedTurnItem> {
    match firings {
        GridCapturedFirings::Summary(firings) => {
            queued_turn_items_for_firings(game, target, commands, firings, next_state, mode)
        }
        GridCapturedFirings::Detailed(firings) => {
            queued_turn_items_for_firings(game, target, commands, firings, next_state, mode)
        }
    }
}

fn queued_turn_items_for_firings<const D: usize, Size: GridSize<D>, Firing: GridFiringView<D>>(
    game: &LoadedGridGame<D, Size>,
    target: Option<&str>,
    commands: &[TransitionCommand],
    firings: &[Firing],
    next_state: &GridState<D, Size>,
    mode: InputExecutionMode,
) -> Vec<QueuedTurnItem> {
    if !mode.materializes_presentation() {
        let mut items = Vec::new();
        let mut has_rule_effects = false;
        for firing in firings {
            let Some(effects) = game.rule_effects.get(&firing.rule()) else {
                continue;
            };
            has_rule_effects |= !effects.is_empty();
            items.extend(
                effects
                    .iter()
                    .filter(|effect| !rule_effect_is_presentation_only(effect))
                    .cloned()
                    .map(|effect| {
                        QueuedTurnItem::Effect(QueuedRuleEffect {
                            target: target.map(str::to_string),
                            effect,
                        })
                    }),
            );
        }
        if !has_rule_effects {
            items.extend(
                queued_effects_for_outcome(game, target, commands, firings)
                    .into_iter()
                    .map(QueuedTurnItem::Effect),
            );
        }
        return items;
    }

    let mut items = Vec::new();
    let mut animation_window = Vec::new();
    let mut has_rule_effects = false;
    for firing in firings {
        let animations = if game.rule_animations.contains_key(&firing.rule()) {
            let firing = firing
                .detailed()
                .expect("animated rule firing must retain its patch");
            animation_events_for_trace(game, std::slice::from_ref(firing), next_state)
        } else {
            Vec::new()
        };
        animation_window.extend(animations.iter().cloned());
        items.extend(animations.into_iter().map(QueuedTurnItem::Animation));
        let Some(effects) = game.rule_effects.get(&firing.rule()) else {
            continue;
        };
        has_rule_effects |= !effects.is_empty();
        for effect in effects {
            let effect = if matches!(effect, RuleEffect::WaitAnimation) {
                let Some(milliseconds) = animation_wait_milliseconds(game, &animation_window)
                else {
                    continue;
                };
                animation_window.clear();
                RuleEffect::Wait { milliseconds }
            } else {
                effect.clone()
            };
            items.push(QueuedTurnItem::Effect(QueuedRuleEffect {
                target: target.map(str::to_string),
                effect,
            }));
        }
    }
    if !has_rule_effects {
        items.extend(
            queued_effects_for_outcome(game, target, commands, firings)
                .into_iter()
                .map(QueuedTurnItem::Effect),
        );
    }
    items
}

fn transition_program_outcome_with_effects<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    programs: &[&GridExecutableProgram<D>],
    state: &GridState<D, Size>,
    input: InputId,
    target: Option<&str>,
    mode: InputExecutionMode,
) -> Result<GridProgramOutcome<D, Size>, GridTransitionError<D>> {
    let detailed_firings = mode.collects_trace()
        || (mode.materializes_presentation() && !game.rule_animations.is_empty());
    if programs.iter().all(|program| program.is_empty()) {
        return Ok(GridProgramOutcome::<D, Size> {
            next_state: state.clone(),
            progressed: false,
            observable: false,
            cancelled: false,
            commands: Vec::new(),
            firings: if detailed_firings {
                GridCapturedFirings::Detailed(Vec::new())
            } else {
                GridCapturedFirings::Summary(Vec::new())
            },
            items: Vec::new(),
        });
    }

    let (next_state, progressed, observable, cancelled, commands, firings) = if detailed_firings {
        let outcome = transition_program_sequence_outcome(&game.game, state, programs, input)?;
        (
            outcome.next_state,
            outcome.progressed,
            outcome.observable,
            outcome.cancelled,
            outcome.commands,
            GridCapturedFirings::Detailed(outcome.firings),
        )
    } else {
        let outcome =
            transition_program_sequence_summary_outcome(&game.game, state, programs, input)?;
        (
            outcome.next_state,
            outcome.progressed,
            outcome.observable,
            outcome.cancelled,
            outcome.commands,
            GridCapturedFirings::Summary(outcome.firings),
        )
    };
    let items = queued_turn_items_for_outcome(game, target, &commands, &firings, &next_state, mode);
    Ok(GridProgramOutcome::<D, Size> {
        next_state,
        progressed,
        observable,
        cancelled,
        commands,
        firings,
        items,
    })
}

pub fn animation_events_for_trace<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    firings: &[GridRuleFiring<D>],
    next_state: &GridState<D, Size>,
) -> Vec<AnimationEvent> {
    let mut events = Vec::new();
    for firing in firings {
        let Some(animations) = game.rule_animations.get(&firing.rule) else {
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
                    for op in firing.patch.ops() {
                        let PatchOp::Move { from, to, object } = op else {
                            continue;
                        };
                        let from_axes = from.axes();
                        let to_axes = to.axes();
                        let (from_x, from_y, from_z) = spatial_axes(from_axes);
                        let (to_x, to_y, to_z) = spatial_axes(to_axes);
                        if objects.contains(object) {
                            push_unique_animation(
                                &mut events,
                                AnimationEvent::Move {
                                    name: name.clone(),
                                    object: *object,
                                    from_object: None,
                                    from_x,
                                    from_y,
                                    from_z,
                                    to_x,
                                    to_y,
                                    to_z,
                                },
                            );
                        }
                    }
                    for op in firing.patch.ops() {
                        let PatchOp::Replace {
                            position,
                            remove,
                            add,
                        } = op
                        else {
                            continue;
                        };
                        let axes = position.axes();
                        let (x, y, z) = spatial_axes(axes);
                        if objects.contains(add) && sprite_rotation_changes(game, *remove, *add) {
                            push_unique_animation(
                                &mut events,
                                AnimationEvent::Move {
                                    name: name.clone(),
                                    object: *add,
                                    from_object: Some(*remove),
                                    from_x: x,
                                    from_y: y,
                                    from_z: z,
                                    to_x: x,
                                    to_y: y,
                                    to_z: z,
                                },
                            );
                        }
                    }
                }
                RuleAnimationTrigger::CantMove => {
                    for op in firing.patch.ops() {
                        let PatchOp::RemoveMark {
                            position,
                            object,
                            mark,
                            ..
                        } = op
                        else {
                            continue;
                        };
                        let axes = position.axes();
                        let (x, y, z) = spatial_axes(axes);
                        if mark.0 != 0 {
                            continue;
                        }
                        if object.0 != 0 {
                            if objects.contains(object) {
                                push_unique_animation(
                                    &mut events,
                                    AnimationEvent::CantMove {
                                        name: name.clone(),
                                        object: *object,
                                        x,
                                        y,
                                        z,
                                    },
                                );
                            }
                            continue;
                        }
                        for candidate in objects {
                            if next_state.has_object_at(&game.game, *position, *candidate) {
                                push_unique_animation(
                                    &mut events,
                                    AnimationEvent::CantMove {
                                        name: name.clone(),
                                        object: *candidate,
                                        x,
                                        y,
                                        z,
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

fn spatial_axes<const D: usize>(axes: [u16; D]) -> (u16, u16, u16) {
    (
        axes.first().copied().unwrap_or(0),
        axes.get(1).copied().unwrap_or(0),
        axes.get(2).copied().unwrap_or(0),
    )
}

fn sprite_rotation_changes<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    from: ObjectId,
    to: ObjectId,
) -> bool {
    let rotations = |object| {
        let object_name = game.object_name(object);
        let sprite_name = game
            .visuals
            .aliases
            .iter()
            .find_map(|alias| (alias.object == object_name).then_some(alias.sprite.as_str()))?;
        let sprite = game
            .visuals
            .sprites
            .iter()
            .find(|sprite| sprite.name == sprite_name)?;
        Some(
            sprite
                .transforms
                .iter()
                .enumerate()
                .filter_map(|(index, transform)| match transform {
                    puzzle_lang::VisualSpriteTransform::Rotate { degrees, .. } => {
                        Some((index, *degrees))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>(),
        )
    };
    let (Some(from), Some(to)) = (rotations(from), rotations(to)) else {
        return false;
    };
    from.len() == to.len()
        && !from.is_empty()
        && from
            .iter()
            .zip(&to)
            .all(|((from_index, _), (to_index, _))| from_index == to_index)
        && from.iter().zip(&to).any(|((_, from), (_, to))| {
            let delta = (to - from).rem_euclid(360.0);
            delta > f64::EPSILON && (360.0 - delta) > f64::EPSILON
        })
}

fn push_unique_animation(events: &mut Vec<AnimationEvent>, event: AnimationEvent) {
    if !events.contains(&event) {
        events.push(event);
    }
}

fn animation_wait_milliseconds<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    animations: &[AnimationEvent],
) -> Option<u64> {
    animations
        .iter()
        .map(|animation| match animation {
            AnimationEvent::Move { name, .. } | AnimationEvent::CantMove { name, .. } => {
                animation_duration_milliseconds(game, name)
            }
        })
        .max()
}

fn animation_duration_milliseconds<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    name: &str,
) -> u64 {
    if name == "tween" {
        return game.animation.tween.interval_ms;
    }
    game.default_wait_ms
}

fn parse_runtime_expr(value: &str) -> Option<SceneExpr> {
    parse_scene_expression(value).ok()
}

fn parse_runtime_level_expr(value: &str) -> Option<SceneExpr> {
    parse_runtime_expr(value)
}

fn parse_runtime_level_selector_expr(value: &str) -> Option<SceneExpr> {
    let expr = parse_runtime_expr(value)?;
    matches!(expr, SceneExpr::LevelSelector { .. }).then_some(expr)
}

fn is_simple_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn level_index_from_value<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    value: &str,
) -> Option<usize> {
    level_index_by_id(game, value)
}

fn level_index_by_id<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    id: &str,
) -> Option<usize> {
    unique_level_match(
        game.levels
            .iter()
            .enumerate()
            .filter_map(|(index, level)| (level.name == id).then_some(index)),
    )
}

fn level_index_by_selector<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    collection: &str,
    key: &SceneLevelKey,
) -> Option<usize> {
    match key {
        SceneLevelKey::Id(id) => level_index_by_collection_id(game, collection, id),
        SceneLevelKey::Index(index) => {
            level_index_by_collection_ordinal(game, collection, usize::try_from(*index).ok()?)
        }
    }
}

fn level_index_by_collection_id<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    collection: &str,
    id: &str,
) -> Option<usize> {
    let collection = resolve_level_collection_name(game, collection)?;
    unique_level_match(game.levels.iter().enumerate().filter_map(|(index, level)| {
        level_belongs_to_collection(level, collection)
            .then_some(index)
            .filter(|_| level.name == id)
    }))
}

fn level_index_by_collection_ordinal<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    collection: &str,
    ordinal: usize,
) -> Option<usize> {
    let collection = resolve_level_collection_name(game, collection)?;
    game.levels
        .iter()
        .enumerate()
        .filter_map(|(index, level)| {
            level_belongs_to_collection(level, collection).then_some(index)
        })
        .nth(ordinal)
}

fn level_index_by_omitted_collection_id<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    id: &str,
) -> Option<usize> {
    let collection = unique_level_collection_name(game)?;
    level_index_by_collection_id(game, collection, id)
}

fn level_index_by_omitted_collection_ordinal<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    ordinal: usize,
) -> Option<usize> {
    let collection = unique_level_collection_name(game)?;
    level_index_by_collection_ordinal(game, collection, ordinal)
}

fn resolve_level_collection_name<'a, const D: usize, Size: GridSize<D>>(
    game: &'a LoadedGridGame<D, Size>,
    collection: &'a str,
) -> Option<&'a str> {
    if collection == "levels" {
        unique_level_collection_name(game)
    } else if game
        .levels
        .iter()
        .any(|level| level.pack.as_deref() == Some(collection))
    {
        Some(collection)
    } else {
        None
    }
}

fn unique_level_collection_name<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
) -> Option<&str> {
    let mut names = game
        .levels
        .iter()
        .map(|level| level.pack.as_deref().unwrap_or("levels"))
        .collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();
    let [name] = names.as_slice() else {
        return None;
    };
    Some(*name)
}

fn level_belongs_to_collection<const D: usize, Size: GridSize<D>>(
    level: &LoadedGridLevel<D, Size>,
    collection: &str,
) -> bool {
    match level.pack.as_deref() {
        Some(pack) => pack == collection,
        None => collection == "levels",
    }
}

fn unique_level_match(mut indices: impl Iterator<Item = usize>) -> Option<usize> {
    let first = indices.next()?;
    indices.next().is_none().then_some(first)
}

fn scene_level_indices<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    scene_name: &str,
) -> Vec<usize> {
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
                    .any(|name| level_resource_matches(name, level))
                    .then_some(index)
            })
            .collect(),
    }
}

fn first_level_index_for_scene<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    scene_name: &str,
    scope: Option<&str>,
) -> Option<usize> {
    scene_level_indices(game, scene_name)
        .into_iter()
        .find(|index| scope.is_none_or(|scope| level_resource_matches(scope, &game.levels[*index])))
}

fn level_has_next_in_scene<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    scene_name: &str,
    level_index: usize,
) -> bool {
    let indices = scene_level_indices(game, scene_name);
    indices
        .iter()
        .position(|index| *index == level_index)
        .is_some_and(|position| position + 1 < indices.len())
}

fn scene_accepts_level<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    scene_name: &str,
    level_index: usize,
) -> bool {
    scene_level_indices(game, scene_name)
        .into_iter()
        .any(|index| index == level_index)
}

fn transition_condition_mentions_input(trigger: &SceneTransitionTrigger, input: &str) -> bool {
    let condition = match trigger {
        SceneTransitionTrigger::Condition(condition)
        | SceneTransitionTrigger::Signal(condition) => condition,
        SceneTransitionTrigger::SceneStart | SceneTransitionTrigger::LevelStart => return false,
    };
    scene_expr_mentions_input_value(condition, input)
}

fn scene_expr_mentions_input_value(expr: &SceneExpr, input: &str) -> bool {
    match expr {
        SceneExpr::Binary {
            op: SceneBinaryOp::Eq,
            left,
            right,
        } => {
            (scene_expr_is_input_path(left)
                && scene_expr_atom_name(right).as_deref() == Some(input))
                || (scene_expr_is_input_path(right)
                    && scene_expr_atom_name(left).as_deref() == Some(input))
        }
        SceneExpr::Binary { left, right, .. } => {
            scene_expr_mentions_input_value(left, input)
                || scene_expr_mentions_input_value(right, input)
        }
        SceneExpr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            scene_expr_mentions_input_value(condition, input)
                || scene_expr_mentions_input_value(then_branch, input)
                || scene_expr_mentions_input_value(else_branch, input)
        }
        SceneExpr::Call { args, .. } => args
            .iter()
            .any(|arg| scene_expr_mentions_input_value(arg, input)),
        _ => false,
    }
}

fn scene_expr_is_input_path(expr: &SceneExpr) -> bool {
    matches!(expr, SceneExpr::Path(path) if path.len() == 1 && path[0] == "input")
}

fn scene_expr_atom_name(expr: &SceneExpr) -> Option<String> {
    match expr {
        SceneExpr::Path(path) if path.len() == 1 => Some(path[0].clone()),
        SceneExpr::Text(value) => Some(value.clone()),
        _ => None,
    }
}

fn level_resource_matches<const D: usize, Size: GridSize<D>>(
    resource: &str,
    level: &LoadedGridLevel<D, Size>,
) -> bool {
    level.name == resource || level.pack.as_deref() == Some(resource)
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
        SceneComponent::Viewport(_)
        | SceneComponent::Frame(_)
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

fn level_menu_action_bindings<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    level_index: usize,
) -> HashMap<String, String> {
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

fn input_id_by_label<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    input_name: &str,
) -> Option<InputId> {
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
    use puzzle_core::transition_outcome;
    use puzzle_lang::parse_game2d as parse_game;

    fn sound_events(events: &[PresentationEvent]) -> Vec<SoundEvent> {
        events
            .iter()
            .filter_map(|event| match &event.kind {
                PresentationEventKind::Sound(event) => Some(event.clone()),
                _ => None,
            })
            .collect()
    }

    fn message_events(events: &[PresentationEvent]) -> Vec<MessageEvent> {
        events
            .iter()
            .filter_map(|event| match &event.kind {
                PresentationEventKind::Message(event) => Some(event.clone()),
                _ => None,
            })
            .collect()
    }

    fn wait_events(events: &[PresentationEvent]) -> Vec<WaitEvent> {
        events
            .iter()
            .filter_map(|event| match &event.kind {
                PresentationEventKind::Wait(event) => Some(event.clone()),
                _ => None,
            })
            .collect()
    }

    fn animation_events(events: &[PresentationEvent]) -> Vec<AnimationEvent> {
        events
            .iter()
            .filter_map(|event| match &event.kind {
                PresentationEventKind::Animation(event) => Some(event.clone()),
                _ => None,
            })
            .collect()
    }

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
    fn session_runs_the_active_levels_effective_rules() {
        let source = r#"
title = play_level_rules
puzzle board {
  slots {
    actor = Player
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player ] -> [ Player ]
  }
}
levels default of board {
  legend {
    . = empty
    P = Player
  }
  level "local" {
    rules before {
      input right [ Player | no actor ] -> [ | Player ]
    }
    P.
  }
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);
        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        assert!(
            session
                .state()
                .has_object(&loaded.game, 1, 0, object_named(&loaded, "Player"))
        );
    }

    #[test]
    fn loaded_document_scene_host_preserves_2d_render_animation_settings() {
        let source = r#"
title = "Tween Settings"

puzzle board {
render {
tween = true
tween_duration = 75ms
}
slots {
actor = Player
}
rules {
input right [ Player | no Player ] -> [ | Player ]
}
}

levels default of board {
legend {
. = empty
P = Player
}
level "first" {
P.
}
}

scene playing {
rules {
step board
}
layout {
puzzle board = board
}
}
"#;
        let document = puzzle_lang::parse_game_for_path(source, "tween_settings.puzzle").unwrap();
        let loaded = loaded_document_scene_host_loaded_game(&document).unwrap();

        assert!(loaded.animation.tween.enabled);
        assert_eq!(loaded.animation.tween.interval_ms, 75);
    }

    #[test]
    fn runtime_command_uses_scene_expression_parser_for_message() {
        let effect = parse_runtime_command(
            r#"message join("Save: ", if true { "yes" } else { "no" })"#,
            100,
        )
        .unwrap();

        let SceneEffect::Message { text } = effect else {
            panic!("expected message effect");
        };
        assert!(matches!(&text, SceneExpr::Call { name, args }
            if name == "join"
                && matches!(args.as_slice(), [SceneExpr::Text(_), SceneExpr::If { .. }])));
    }

    #[test]
    fn runtime_command_uses_scene_parser_for_goto_params() {
        let effect = parse_runtime_command(
            r#"goto detail with selected = join("a, ", if true { "b" } else { "c" })"#,
            100,
        )
        .unwrap();

        let SceneEffect::Goto { scene, params } = effect else {
            panic!("expected goto effect");
        };
        assert_eq!(scene, "detail");
        assert!(matches!(params.as_slice(),
            [SceneEffectParam::Named { name, value: SceneExpr::Call { name: call, args } }]
            if name == "selected" && call == "join" && args.len() == 2));
    }

    #[test]
    fn runtime_message_rejects_unparsed_text_instead_of_falling_back() {
        assert!(parse_runtime_command("message plain text", 100).is_none());
    }

    #[test]
    fn again_command_runs_no_input_follow_up_turn() {
        let loaded = parse_game(
            r#"
title = again_runtime
puzzle default {
slots {
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
            .resolve_turn_effects(
                &loaded,
                vec![QueuedTurnItem::Effect(QueuedRuleEffect {
                    target: None,
                    effect: RuleEffect::Again,
                })],
                None,
                session.history.undo_len(),
            )
            .unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, after));
    }

    #[test]
    fn wait_in_again_turn_pauses_that_turn_before_its_following_rule() {
        let loaded = parse_game(
            r#"
title = again_wait_segments
puzzle default {
slots {
__legacy_layer_0 = A B C
}
empty .
rules {
once [ B ] -> wait 100ms
once [ B ] -> [ C ]
once right [ A ] -> [ B ] again
}
levels {
legend {
A = A
B = B
C = C
}
level "start" {
A
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let a = object_named(&loaded, "A");
        let b = object_named(&loaded, "B");
        let c = object_named(&loaded, "C");

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        assert!(!session.state().has_object(&loaded.game, 0, 0, a));
        assert!(session.state().has_object(&loaded.game, 0, 0, b));
        assert!(!session.state().has_object(&loaded.game, 0, 0, c));
        assert!(session.is_waiting());
        assert_eq!(
            wait_events(&session.take_presentation_events()),
            vec![WaitEvent::Wait { milliseconds: 100 }]
        );

        session.resume_wait(&loaded).unwrap();

        assert!(!session.state().has_object(&loaded.game, 0, 0, b));
        assert!(session.state().has_object(&loaded.game, 0, 0, c));
        assert!(!session.is_waiting());
        session.undo(&loaded);
        assert!(session.state().has_object(&loaded.game, 0, 0, a));
        assert!(!session.can_undo());
    }

    #[test]
    fn again_follow_up_turn_does_not_add_an_undo_step() {
        let loaded = parse_game(
            r#"
title = again_undo_boundary
puzzle default {
slots {
__legacy_layer_0 = A B C
}
empty .
rules {
once right [ A ] -> [ B ] again
once [ B ] -> [ C ]
}
levels {
legend {
A = A
B = B
C = C
}
level "start" {
A
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let a = object_named(&loaded, "A");
        let b = object_named(&loaded, "B");
        let c = object_named(&loaded, "C");
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, right).unwrap();

        assert!(!session.state().has_object(&loaded.game, 0, 0, a));
        assert!(!session.state().has_object(&loaded.game, 0, 0, b));
        assert!(session.state().has_object(&loaded.game, 0, 0, c));
        assert!(session.can_undo());

        session.undo(&loaded);

        assert!(session.state().has_object(&loaded.game, 0, 0, a));
        assert!(!session.state().has_object(&loaded.game, 0, 0, b));
        assert!(!session.state().has_object(&loaded.game, 0, 0, c));
        assert!(!session.can_undo());
    }

    #[test]
    fn rewrite_again_effect_lowers_to_transition_command() {
        let loaded = parse_game(
            r#"
title = again_effect
puzzle default {
slots {
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
title = checkpoint_runtime
puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .
input save
input delay
input clear
rules {
if input == right {
once right [ Player | no Player ] -> [ | Player ]
}
if input == save {
checkpoint
}
if input == delay {
wait 1s
sfx locked
}
if input == clear {
clear_checkpoint
}
}
levels {
legend P = Player

level "start" {
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
title = win_effect
puzzle default {
slots {
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
title = tween_runtime
puzzle default {
render {
tween = true
tween_duration = 80ms
}
slots {
actor = Player
}
rules {
input right [ Player | no Player ] -> [ | Player ]
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
        let right = input_named(&loaded, "right");
        let programs = loaded.programs_for_level(0).unwrap();
        let outcome = transition_program_outcome_with_effects(
            &loaded,
            &programs,
            session.state(),
            right,
            None,
            InputExecutionMode::Player,
        )
        .unwrap();
        assert!(matches!(outcome.firings, GridCapturedFirings::Detailed(_)));

        session.apply_input(&loaded, right).unwrap();

        assert_eq!(
            animation_events(&session.take_presentation_events()),
            vec![AnimationEvent::Move {
                name: "tween".to_string(),
                object: player,
                from_object: None,
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
    fn tween_emits_same_cell_event_for_wildcard_rotation_variant_rewrite() {
        let loaded = parse_game(
            r#"
title = rotation_tween_variant

puzzle default {
render {
tween = true
tween_duration = 80ms
}
slots {
actor = Player:directions
}
sprites {
Player:directions {
colors = #fff
rotate directions
0
}
}
rules {
input [ Player:* ] -> [ > Player:> ]
}
levels {
legend {
. = empty
P = Player:up
}
P
}
}
"#,
        )
        .unwrap();
        let from = object_named(&loaded, "Player:up");
        let to = object_named(&loaded, "Player:right");
        let mut session = GameSession::new(&loaded);
        session
            .apply_traced_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        let patch_ops = session
            .last_transition_trace()
            .unwrap()
            .firings
            .iter()
            .flat_map(|firing| firing.patch.ops())
            .collect::<Vec<_>>();
        assert!(patch_ops.iter().any(|op| {
            matches!(
                op,
                PatchOp::Replace {
                    position,
                    remove,
                    add,
                } if position.axes() == [0, 0] && *remove == from && *add == to
            )
        }));
        assert!(!patch_ops.iter().any(|op| {
            matches!(op, PatchOp::Remove { position, object } if position.axes() == [0, 0] && *object == from)
                || matches!(op, PatchOp::Add { position, object } if position.axes() == [0, 0] && *object == to)
        }));

        assert_eq!(
            animation_events(&session.take_presentation_events()),
            vec![AnimationEvent::Move {
                name: "tween".to_string(),
                object: to,
                from_object: Some(from),
                from_x: 0,
                from_y: 0,
                from_z: 0,
                to_x: 0,
                to_y: 0,
                to_z: 0,
            }]
        );
    }

    #[test]
    fn tween_does_not_infer_variant_identity_from_same_cell_remove_and_add() {
        let loaded = parse_game(
            r#"
title = unrelated_rotation_rewrite

puzzle default {
render {
tween = true
tween_duration = 80ms
}
slots {
actor = A B
}
sprites {
A {
colors = #fff
rotate 0deg
0
}
B {
colors = #fff
rotate 90deg
0
}
}
rules {
input [ A ] -> [ B ]
}
levels {
legend {
. = empty
A = A
}
A
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        assert!(animation_events(&session.take_presentation_events()).is_empty());
    }

    #[test]
    fn congruent_sprite_rotations_do_not_require_rotation_tween() {
        let loaded = parse_game(
            r#"
title = congruent_rotation_tween

puzzle default {
tags {
facing = 0deg 360deg
}
slots {
actor = Player:facing
}
sprites {
Player:facing {
colors = #fff
rotate facing
0
}
}
rules {
}
levels {
legend {
. = empty
P = Player:0deg
}
P
}
}
"#,
        )
        .unwrap();

        assert!(!sprite_rotation_changes(
            &loaded,
            object_named(&loaded, "Player:0deg"),
            object_named(&loaded, "Player:360deg"),
        ));
    }

    #[test]
    fn runtime_animation_contract_serializes_rotation_tween_source_object() {
        let loaded = parse_game(
            r#"
title = rotation_tween_contract
puzzle default {
slots {
actor = Player:directions
}
rules {
}
levels {
legend {
. = empty
P = Player:up
}
P
}
}
"#,
        )
        .unwrap();
        let from = object_named(&loaded, "Player:up");
        let to = object_named(&loaded, "Player:right");
        let events = animation_events_contract_2d(
            &loaded,
            &[AnimationEvent::Move {
                name: "tween".to_string(),
                object: to,
                from_object: Some(from),
                from_x: 0,
                from_y: 0,
                from_z: 0,
                to_x: 0,
                to_y: 0,
                to_z: 0,
            }],
        );

        assert_eq!(
            serde_json::to_value(events).unwrap(),
            serde_json::json!([{
                "kind": "move",
                "name": "tween",
                "objectId": to.0,
                "fromObject": "Player:up",
                "from": { "x": 0, "y": 0 },
                "to": { "x": 0, "y": 0 }
            }])
        );
    }

    #[test]
    #[should_panic(expected = "compiled object 65535 is missing its required object label")]
    fn runtime_animation_contract_rejects_missing_source_object_label() {
        let loaded = parse_game(
            r#"
title = rotation_tween_missing_label
puzzle default {
slots {
actor = Player
}
rules {
}
levels {
legend {
. = empty
P = Player
}
P
}
}
"#,
        )
        .unwrap();
        let player = object_named(&loaded, "Player");

        let _ = animation_events_contract_2d(
            &loaded,
            &[AnimationEvent::Move {
                name: "tween".to_string(),
                object: player,
                from_object: Some(ObjectId(u16::MAX)),
                from_x: 0,
                from_y: 0,
                from_z: 0,
                to_x: 0,
                to_y: 0,
                to_z: 0,
            }],
        );
    }

    #[test]
    fn win_effect_forces_level_clear_lifecycle() {
        let loaded = parse_game(
            r#"
title = win_effect_runtime
puzzle default {
slots {
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

    #[test]
    fn lose_conditions_block_model_level_clear() {
        let loaded = parse_game(
            r#"
title = lose_blocks_clear
puzzle default {
slots {
actor = Player
}
win_conditions {
some Player
}
lose_conditions {
some Player
}
on_level_clear {
next_level
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "one" {
P
}
level "two" {
.
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, InputId(0)).unwrap();

        assert_eq!(session.level_index(), 0);
        assert_eq!(session.screen(), "default");
        assert!(!session.cleared_levels()[0]);
    }

    #[test]
    fn lose_conditions_block_win_effect_forced_clear() {
        let loaded = parse_game(
            r#"
title = lose_blocks_win_effect
puzzle default {
slots {
actor = Player
}
lose_conditions {
some Player
}
on_level_clear {
next_level
}
rules {
[ Player ] -> win
}
levels {
legend {
. = empty
P = Player
}
level "one" {
P
}
level "two" {
.
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, InputId(0)).unwrap();

        assert_eq!(session.level_index(), 0);
        assert!(!session.cleared_levels()[0]);
    }

    #[test]
    fn lose_conditions_do_not_block_model_next_level_command() {
        let loaded = parse_game(
            r#"
title = lose_allows_next_level
puzzle default {
slots {
actor = Player
}
win_conditions {
some Player
}
lose_conditions {
some Player
}
rules {
if win_conditions -> next_level
}
levels {
legend {
. = empty
P = Player
}
level "one" {
P
}
level "two" {
.
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_input(&loaded, InputId(0)).unwrap();

        assert_eq!(session.level_index(), 1);
        assert!(!session.cleared_levels()[0]);
    }

    #[test]
    fn lose_conditions_make_scene_win_condition_false() {
        let loaded = parse_game(
            r#"
title = lose_blocks_scene_win_condition
puzzle board {
slots {
actor = Player
}
win_conditions {
some Player
}
lose_conditions {
some Player
}
rules {

}
levels {
legend {
. = empty
P = Player
}
P
}
}

scene playing {
layout {
puzzle board = board
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
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();

        session.apply_input(&loaded, InputId(0)).unwrap();

        assert_eq!(session.screen(), "playing");
        assert!(!session.cleared_levels()[0]);
    }

    #[test]
    fn conditional_win_effect_forces_level_clear_lifecycle() {
        let loaded = parse_game(
            r#"
title = conditional_win_effect_runtime
puzzle default {
slots {
floor = Exit
marker = Cleared
actor = Player
}
input clear
query can_clear = exists(Exit)
on_level_clear {
[ Player Exit no Cleared ] -> [ Player Exit Cleared ]
next_level
}
rules {
if can_clear -> win
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

        session
            .apply_input(&loaded, input_named(&loaded, "clear"))
            .unwrap();

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn targeted_conditional_win_effect_forces_level_clear_lifecycle() {
        let loaded = parse_game(
            r#"
title = targeted_conditional_win_effect_runtime
puzzle board {
slots {
floor = Exit
marker = Cleared
actor = Player
}
input clear
win_conditions {
no Exit
}
on_level_clear {
[ Player Exit no Cleared ] -> [ Player Exit Cleared ]
next_level
}
rules {
if input == clear -> win
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

scene playing {
layout {
puzzle board = board
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_command(&loaded, "goto playing").unwrap();
        session
            .apply_input(&loaded, input_named(&loaded, "clear"))
            .unwrap();

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn conditional_win_effect_satisfies_scene_win_condition_transition() {
        let loaded = parse_game(
            r#"
title = conditional_win_effect_scene_transition
puzzle board {
slots {
floor = Exit
actor = Player
}
input clear
query can_clear = exists(Exit)
win_conditions {
no Exit
}
rules {
if can_clear -> win
}
levels {
legend {
. = empty
X = Player Exit
}
X
}
}

scene playing {
layout {
puzzle board = board
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
        let mut session = GameSession::new(&loaded);

        session.apply_command(&loaded, "goto playing").unwrap();
        session
            .apply_input(&loaded, input_named(&loaded, "clear"))
            .unwrap();

        assert_eq!(session.screen(), "level_clear");
        assert!(session.cleared_levels()[0]);
    }

    #[test]
    fn puzzle_rule_goto_effect_changes_scene_after_turn() {
        let loaded = parse_game(
            r#"
title = puzzle_rule_goto_runtime
puzzle default {
slots {
actor = Player
}
input open
rules {
if input == open -> goto menu
}
levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}
}
scene playing {
layout {
puzzle board = default
}
rules {
step board
}
}
scene menu {
layout {
text "Menu"
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();
        assert_eq!(session.screen(), "playing");

        session
            .apply_input(&loaded, input_named(&loaded, "open"))
            .unwrap();

        assert_eq!(session.screen(), "menu");
    }

    fn transition_fixture() -> LoadedGame {
        parse_game(
            r#"
title = transition_fixture
puzzle sokoban {
slots {
floor = Goal
actor = Player Box Wall
}
groups {
solid = Player Box Wall
}
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
level "first" {
#######
#P.B.G#
#######
}
level "second" {
#######
#P.B.G#
#######
}
}
}
scene playing {
layout {
puzzle board = sokoban
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
        level_pack: &str,
        level_name: &str,
    ) {
        let index = loaded
            .levels
            .iter()
            .position(|level| level.pack.as_deref() == Some(level_pack) && level.name == level_name)
            .unwrap_or_else(|| panic!("missing level {level_pack}[{level_name}]"));
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
title = scene_local_puzzle_fixture

puzzle sokoban {
var portal_entered = false

slots {
trigger = Portal
solid = Player Wall
}

rules {
once [ Player ] -> portal_entered = false
for d in directions {
if input == d {
once d [ Player | Portal no solid ] -> [ | Player ] portal_entered = true
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

level "hub" {
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
layout {
spec_board.visible = false
puzzle spec_board = sokoban
puzzle board = sokoban
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
layout {
puzzle spec_board = sokoban
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
        load_named_scene_level(loaded, session, "hub.spec_board", "spec", "hub");
        assert_eq!(session.screen(), "hub");
    }

    #[test]
    fn scene_condition_can_read_board_var_directly() {
        let loaded = parse_game(
            r#"
title = scene_var_condition

puzzle default {
slots {
__legacy_layer_1 = Player
}
empty .

var moved = false


input tick

rules {
once [ Player ] -> moved = true
}

levels {
legend P = Player
level "start" {
P
}
}
}

scene playing {
layout {
puzzle board = default
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
        session.apply_command(&loaded, "goto playing").unwrap();

        session.apply_input(&loaded, tick).unwrap();

        assert_eq!(session.screen(), "moved");
    }

    #[test]
    fn scene_start_transition_runs_when_scene_becomes_focused() {
        let loaded = parse_game(
            r#"
title = scene_start_fixture
puzzle default {
persistent var moves = 0

slots {
__legacy_layer_0 = Player
}
empty .

rules {
}
levels {
legend P = Player
level "start" {
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
        assert!(sound_events(&session.take_presentation_events()).is_empty());

        session.apply_command(&loaded, "goto menu").unwrap();

        assert_eq!(
            sound_events(&session.take_presentation_events()),
            vec![SoundEvent::StopMusic {
                name: Some("music_name".to_string())
            }]
        );
    }

    #[test]
    fn puzzle_rule_sfx_effect_queues_sound_event_on_match() {
        let loaded = parse_game(
            r#"
title = rule_sfx_fixture
sounds {
sfx push { seed = push01; type = jump }
}
puzzle default {
slots {
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
level "start" {
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
            sound_events(&session.take_presentation_events()),
            vec![SoundEvent::PlaySfx {
                name: "push".to_string()
            }]
        );
    }

    #[test]
    fn model_operation_sounds_emit_for_undo_and_restart() {
        let loaded = parse_game(
            r#"
title = operation_sfx_fixture
sounds {
sfx undo_tick { seed = undo01; type = hit }
sfx restart_tick { seed = restart01; type = jump }
}
puzzle default {
sounds {
undo -> sfx undo_tick
restart -> sfx restart_tick
}
slots {
__legacy_layer_0 = Player
}
empty .
rules {
right [ Player | ] -> [ | Player ]
}
levels {
legend {
. = empty
P = Player
}
level "start" {
P.
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let mut session = GameSession::new(&loaded);
        assert!(sound_events(&session.take_presentation_events()).is_empty());

        session.apply_input(&loaded, right).unwrap();
        assert!(sound_events(&session.take_presentation_events()).is_empty());

        session.undo(&loaded);
        assert_eq!(
            sound_events(&session.take_presentation_events()),
            vec![SoundEvent::PlaySfx {
                name: "undo_tick".to_string()
            }]
        );

        session.restart_level(&loaded).unwrap();
        assert_eq!(
            sound_events(&session.take_presentation_events()),
            vec![SoundEvent::PlaySfx {
                name: "restart_tick".to_string()
            }]
        );
    }

    #[test]
    fn restart_input_starts_a_new_level_attempt_and_component_reset_does_not() {
        let loaded = parse_game(
            r#"
title = restart_attempt_lifecycle
sounds {
sfx restart_tick { seed = restart01; type = jump }
sfx locked { seed = locked01; type = lock }
}
puzzle default {
sounds {
restart -> sfx restart_tick
}
slots {
actor = Player
marker = Started
}
empty .
input save
input delay
on_level_start {
sfx locked
once [ Player no Started ] -> [ Player Started ]
}
rules {
if input == save {
checkpoint
}
if input == delay {
wait 1s
sfx locked
}
}
levels {
legend P = Player
level "start" {
P
}
}
}
scene playing {
layout {
puzzle board = default
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();

        assert_eq!(
            sound_events(&session.take_presentation_events()),
            vec![
                SoundEvent::PlaySfx {
                    name: "locked".to_string()
                },
                SoundEvent::PlaySfx {
                    name: "locked".to_string()
                },
            ]
        );

        session
            .apply_input(&loaded, input_named(&loaded, "restart"))
            .unwrap();
        assert!(!session.can_undo());
        assert!(!session.can_redo());
        assert_eq!(
            sound_events(&session.take_presentation_events()),
            vec![
                SoundEvent::PlaySfx {
                    name: "restart_tick".to_string()
                },
                SoundEvent::PlaySfx {
                    name: "locked".to_string()
                },
            ]
        );

        session
            .start_level_from_state(&loaded, 0, loaded.levels[0].initial_state.clone(), false)
            .unwrap();
        assert!(sound_events(&session.take_presentation_events()).is_empty());
        session.restart_level(&loaded).unwrap();
        assert_eq!(
            sound_events(&session.take_presentation_events()),
            vec![
                SoundEvent::PlaySfx {
                    name: "restart_tick".to_string()
                },
                SoundEvent::PlaySfx {
                    name: "locked".to_string()
                },
            ]
        );

        session
            .apply_input(&loaded, input_named(&loaded, "save"))
            .unwrap();
        assert!(sound_events(&session.take_presentation_events()).is_empty());
        session
            .apply_input(&loaded, input_named(&loaded, "restart"))
            .unwrap();
        assert_eq!(
            sound_events(&session.take_presentation_events()),
            vec![
                SoundEvent::PlaySfx {
                    name: "restart_tick".to_string()
                },
                SoundEvent::PlaySfx {
                    name: "locked".to_string()
                },
            ]
        );

        session.apply_command(&loaded, "board.restart").unwrap();
        assert_eq!(
            sound_events(&session.take_presentation_events()),
            vec![SoundEvent::PlaySfx {
                name: "restart_tick".to_string()
            }]
        );

        session
            .apply_input(&loaded, input_named(&loaded, "delay"))
            .unwrap();
        let presentation_events = session.take_presentation_events();
        assert_eq!(
            wait_events(&presentation_events),
            vec![WaitEvent::Wait { milliseconds: 1000 }]
        );
        assert!(sound_events(&presentation_events).is_empty());
        assert!(session.restart_level(&loaded).is_err());
        assert!(session.is_waiting());
        session.resume_wait(&loaded).unwrap();
        assert_eq!(
            sound_events(&session.take_presentation_events()),
            vec![SoundEvent::PlaySfx {
                name: "locked".to_string()
            }]
        );
        session.restart_level(&loaded).unwrap();
        let presentation_events = session.take_presentation_events();
        assert!(wait_events(&presentation_events).is_empty());
        assert_eq!(
            sound_events(&presentation_events),
            vec![
                SoundEvent::PlaySfx {
                    name: "restart_tick".to_string()
                },
                SoundEvent::PlaySfx {
                    name: "locked".to_string()
                },
            ]
        );
    }

    #[test]
    fn puzzle_rule_music_effect_queues_sound_event_on_match() {
        let loaded = parse_game(
            r#"
title = rule_music_fixture
sounds {
music locked_room { seed = room01 }
}
puzzle default {
slots {
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
level "start" {
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
            sound_events(&session.take_presentation_events()),
            vec![SoundEvent::StopMusic {
                name: Some("locked_room".to_string())
            }]
        );
    }

    #[test]
    fn model_move_sound_trigger_queues_sound_event_on_matching_move() {
        let loaded = parse_game(
            r#"
title = model_move_sfx_fixture
sounds {
sfx push { seed = push01; type = jump }
}
puzzle default {
slots {
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
level "start" {
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
            sound_events(&session.take_presentation_events()),
            vec![SoundEvent::PlaySfx {
                name: "push".to_string()
            }]
        );
    }

    #[test]
    fn canonical_cantmove_sound_trigger_is_rejected() {
        let error = parse_game(
            r#"
title = cantmove_sound_rejected
sounds {
sfx bump { seed = bump01; type = hit }
}
puzzle default {
slots {
actor = A
}
sounds {
cantmove A -> sfx bump
}
rules {
right [ A ] -> [ > A ]
}
levels {
legend {
A = A
}
level "start" {
A
}
}
}
"#,
        )
        .unwrap_err()
        .to_string();

        assert!(
            error.contains("model sounds entry must be: move <object-selector> -> sfx <name>"),
            "{error}"
        );
    }

    #[test]
    fn rule_sfx_is_deduped_within_one_turn() {
        let loaded = parse_game(
            r#"
title = rule_sfx_dedup_fixture
sounds {
sfx push { seed = push01; type = jump }
}
puzzle default {
slots {
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
level "start" {
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
            sound_events(&session.take_presentation_events()),
            vec![SoundEvent::PlaySfx {
                name: "push".to_string()
            }]
        );
    }

    #[test]
    fn again_follow_up_turn_has_separate_sfx_dedup_scope() {
        let loaded = parse_game(
            r#"
title = again_sfx_scope_fixture
sounds {
sfx push { seed = push01; type = jump }
}
puzzle default {
slots {
actor = Box
}
rules {
if some(right [ Box | no Box ]) {
once right [ Box | no Box ] -> [ | Box ] sfx push
again
}
}
levels {
legend {
. = empty
B = Box
}
level "start" {
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
            sound_events(&session.take_presentation_events()),
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
title = rule_do_sfx_fixture
sounds {
sfx tick { seed = tick01; type = jump }
}
puzzle default {
slots {
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
level "start" {
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
            sound_events(&session.take_presentation_events()),
            vec![SoundEvent::PlaySfx {
                name: "tick".to_string()
            }]
        );
    }

    #[test]
    fn puzzle_wait_effect_waits_for_animation_or_explicit_duration() {
        let loaded = parse_game(
            r#"
title = rule_wait_fixture
default_wait_time = 300ms
puzzle default {
slots {
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
level "start" {
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
            wait_events(&session.take_presentation_events()),
            vec![WaitEvent::Wait { milliseconds: 25 }]
        );
    }

    #[test]
    fn wait_duration_changes_pause_duration_but_not_segment_states() {
        let source = |duration: &str| {
            format!(
                r#"
title = wait_duration_invariance
puzzle default {{
slots {{
__legacy_layer_0 = A B C
}}
empty .
rules {{
[ A ] -> [ B ]
wait {duration}
[ B ] -> [ C ]
}}
levels {{
legend {{
A = A
}}
level "start" {{
A
}}
}}
}}
"#
            )
        };
        let fast = parse_game(&source("1ms")).unwrap();
        let slow = parse_game(&source("10s")).unwrap();
        let mut fast_session = GameSession::new(&fast);
        let mut slow_session = GameSession::new(&slow);

        fast_session.apply_input(&fast, InputId(0)).unwrap();
        slow_session.apply_input(&slow, InputId(0)).unwrap();

        assert_eq!(fast_session.state(), slow_session.state());
        assert_eq!(
            wait_events(&fast_session.take_presentation_events()),
            vec![WaitEvent::Wait { milliseconds: 1 }]
        );
        assert_eq!(
            wait_events(&slow_session.take_presentation_events()),
            vec![WaitEvent::Wait {
                milliseconds: 10_000
            }]
        );
        fast_session.resume_wait(&fast).unwrap();
        slow_session.resume_wait(&slow).unwrap();
        assert_eq!(fast_session.state(), slow_session.state());
    }

    #[test]
    fn mixed_presentation_segments_keep_authored_order_and_origin_context() {
        let loaded = parse_game(
            r#"
title = mixed_presentation_timeline
default_wait_time = 40ms
sounds {
sfx tick { seed = tick01; type = hit }
}
puzzle default {
render {
tween = true
tween_duration = 80ms
}
slots {
actor = Player
marker = Done
}
empty .
rules {
wait 10ms
message "ready"
input right [ Player | no Player ] -> [ | Player ] sfx tick
wait animation
[ Player no Done ] -> [ Player Done ]
}
levels {
legend {
. = empty
P = Player
}
level "first" {
P.
}
level "second" {
P.
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let done = object_named(&loaded, "Done");

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        let mut events = Vec::new();
        loop {
            events.extend(session.take_presentation_events());
            if !session.is_waiting() {
                break;
            }
            session.resume_wait(&loaded).unwrap();
        }
        assert!(session.state().has_object(&loaded.game, 1, 0, done));
        assert_eq!(
            events
                .iter()
                .map(|event| match &event.kind {
                    PresentationEventKind::Wait(WaitEvent::Wait { milliseconds }) => {
                        format!("wait:{milliseconds}")
                    }
                    PresentationEventKind::Message(_) => "message".to_string(),
                    PresentationEventKind::Animation(_) => "animation".to_string(),
                    PresentationEventKind::Sound(SoundEvent::PlaySfx { .. }) => {
                        "sfx".to_string()
                    }
                    PresentationEventKind::Sound(_) => "music".to_string(),
                })
                .collect::<Vec<_>>(),
            vec![
                "wait:10",
                "message",
                "wait:40",
                "animation",
                "sfx",
                "wait:80"
            ]
        );
        assert!(
            events
                .iter()
                .all(|event| event.context.level_index == Some(0))
        );
    }

    #[test]
    fn wait_animation_pauses_before_following_rule() {
        let loaded = parse_game(
            r#"
title = wait_animation_fixture
puzzle default {
render {
tween = true
tween_duration = 80ms
}
slots {
actor = Player
marker = Marker
}
rules {
input right [ Player | no Player ] -> [ | Player ]
wait animation
[ Player no Marker ] -> [ Player Marker ]
}
levels {
legend {
. = empty
P = Player
}
level "start" {
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

        let presentation_events = session.take_presentation_events();
        assert_eq!(
            wait_events(&presentation_events),
            vec![WaitEvent::Wait { milliseconds: 80 }]
        );
        assert_eq!(
            animation_events(&presentation_events),
            vec![AnimationEvent::Move {
                name: "tween".to_string(),
                object: player,
                from_object: None,
                from_x: 0,
                from_y: 0,
                from_z: 0,
                to_x: 1,
                to_y: 0,
                to_z: 0,
            }]
        );
        assert!(!session.state().has_object(&loaded.game, 1, 0, marker));
        session.resume_wait(&loaded).unwrap();
        assert!(session.state().has_object(&loaded.game, 1, 0, marker));
    }

    #[test]
    fn wait_animation_pauses_before_following_routine() {
        let loaded = parse_game(
            r#"
title = tween_routine_boundary
puzzle default {
render {
tween = true
tween_duration = 80ms
}
slots {
actor = Player
marker = Marker
}
rules {
input right [ Player | no Player ] -> [ | Player ]
wait animation
routine_after_move
}
routine routine_after_move {
[ Player no Marker ] -> [ Player Marker ]
}
levels {
legend {
. = empty
P = Player
}
level "start" {
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

        assert!(session.state().has_object(&loaded.game, 1, 0, player));
        assert!(!session.state().has_object(&loaded.game, 1, 0, marker));
        let presentation_events = session.take_presentation_events();
        assert_eq!(
            wait_events(&presentation_events),
            vec![WaitEvent::Wait { milliseconds: 80 }]
        );
        assert_eq!(
            animation_events(&presentation_events),
            vec![AnimationEvent::Move {
                name: "tween".to_string(),
                object: player,
                from_object: None,
                from_x: 0,
                from_y: 0,
                from_z: 0,
                to_x: 1,
                to_y: 0,
                to_z: 0,
            }]
        );
        session.resume_wait(&loaded).unwrap();
        assert!(session.state().has_object(&loaded.game, 1, 0, marker));
    }

    #[test]
    fn tween_animation_without_wait_does_not_pause_following_routine() {
        let loaded = parse_game(
            r#"
title = tween_without_wait_boundary
puzzle default {
render {
tween = true
tween_duration = 80ms
}
slots {
actor = Player
marker = Marker
}
rules {
input right [ Player | ] -> [ | Player ]
routine_after_move
}
routine routine_after_move {
[ Player no Marker ] -> [ Player Marker ]
}
levels {
legend {
. = empty
P = Player
}
level "start" {
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

        assert!(session.state().has_object(&loaded.game, 1, 0, player));
        assert!(session.state().has_object(&loaded.game, 1, 0, marker));
        let presentation_events = session.take_presentation_events();
        assert!(wait_events(&presentation_events).is_empty());
        assert_eq!(
            animation_events(&presentation_events),
            vec![AnimationEvent::Move {
                name: "tween".to_string(),
                object: player,
                from_object: None,
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
    fn wait_animation_chain_pauses_before_following_routine() {
        let loaded = parse_game(
            r#"
title = standard_move_tween_chain_boundary
puzzle default {
render {
tween = true
tween_duration = 80ms
}
slots {
actor = Player Box
marker = Marker
}
rules {
input right [ Player | Box | no Player no Box ] -> [ | Player | Box ]
wait animation
routine_after_move
}
routine routine_after_move {
[ Player no Marker ] -> [ Player Marker ]
}
levels {
legend {
. = empty
P = Player
B = Box
}
level "start" {
PB.
}
}
}
"#,
        )
        .unwrap();
        let player = object_named(&loaded, "Player");
        let box_object = object_named(&loaded, "Box");
        let marker = object_named(&loaded, "Marker");
        let mut session = GameSession::new(&loaded);

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        assert!(!session.state().has_object(&loaded.game, 0, 0, player));
        assert!(session.state().has_object(&loaded.game, 1, 0, player));
        assert!(session.state().has_object(&loaded.game, 2, 0, box_object));
        assert!(!session.state().has_object(&loaded.game, 1, 0, marker));
        let presentation_events = session.take_presentation_events();
        assert_eq!(
            wait_events(&presentation_events),
            vec![WaitEvent::Wait { milliseconds: 80 }]
        );
        let mut animation_events = animation_events(&presentation_events);
        animation_events.sort_by_key(|event| match event {
            AnimationEvent::Move { object, .. } | AnimationEvent::CantMove { object, .. } => {
                object.0
            }
        });
        assert_eq!(
            animation_events,
            vec![
                AnimationEvent::Move {
                    name: "tween".to_string(),
                    object: player,
                    from_object: None,
                    from_x: 0,
                    from_y: 0,
                    from_z: 0,
                    to_x: 1,
                    to_y: 0,
                    to_z: 0,
                },
                AnimationEvent::Move {
                    name: "tween".to_string(),
                    object: box_object,
                    from_object: None,
                    from_x: 1,
                    from_y: 0,
                    from_z: 0,
                    to_x: 2,
                    to_y: 0,
                    to_z: 0,
                },
            ]
        );
        session.resume_wait(&loaded).unwrap();
        assert!(session.state().has_object(&loaded.game, 1, 0, marker));
    }

    #[test]
    fn wait_animation_chain_pauses_before_following_rule() {
        let loaded = parse_game(
            r#"
title = standard_move_tween_direct_rule_boundary
puzzle default {
render {
tween = true
tween_duration = 80ms
}
slots {
actor = Player Box
marker = Marker
}
rules {
input right [ Player | Box | no Player no Box ] -> [ | Player | Box ]
wait animation
[ Player no Marker ] -> [ Player Marker ]
}
levels {
legend {
. = empty
P = Player
B = Box
}
level "start" {
PB.
}
}
}
"#,
        )
        .unwrap();
        let player = object_named(&loaded, "Player");
        let box_object = object_named(&loaded, "Box");
        let marker = object_named(&loaded, "Marker");
        let mut session = GameSession::new(&loaded);

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        assert!(session.state().has_object(&loaded.game, 1, 0, player));
        assert!(session.state().has_object(&loaded.game, 2, 0, box_object));
        assert!(!session.state().has_object(&loaded.game, 1, 0, marker));
        assert_eq!(
            wait_events(&session.take_presentation_events()),
            vec![WaitEvent::Wait { milliseconds: 80 }]
        );
        session.resume_wait(&loaded).unwrap();
        assert!(session.state().has_object(&loaded.game, 1, 0, marker));
    }

    #[test]
    fn rewrite_suffix_wait_duration_runs_once_after_repeated_rewrite() {
        let loaded = parse_game(
            r#"
title = rewrite_suffix_wait_duration_fixture
puzzle default {
slots {
actor = A
box_layer = B
marker = Marker
}
rules {
[ A ] -> [ B ] wait 25ms
[ B no Marker ] -> [ B Marker ]
}
levels {
legend {
. = empty
A = A
}
level "start" {
AA
}
}
}
"#,
        )
        .unwrap();
        let marker = object_named(&loaded, "Marker");
        let b = object_named(&loaded, "B");
        let mut session = GameSession::new(&loaded);

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        assert_eq!(
            wait_events(&session.take_presentation_events()),
            vec![WaitEvent::Wait { milliseconds: 25 }]
        );
        assert!(session.state().has_object(&loaded.game, 0, 0, b));
        assert!(session.state().has_object(&loaded.game, 1, 0, b));
        assert!(!session.state().has_object(&loaded.game, 0, 0, marker));
        assert!(!session.state().has_object(&loaded.game, 1, 0, marker));
        session.resume_wait(&loaded).unwrap();
        assert!(session.state().has_object(&loaded.game, 0, 0, marker));
        assert!(session.state().has_object(&loaded.game, 1, 0, marker));
    }

    #[test]
    fn wait_animation_without_animation_is_noop() {
        let loaded = parse_game(
            r#"
title = wait_animation_noop_fixture
puzzle default {
slots {
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
level "start" {
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

        assert!(wait_events(&session.take_presentation_events()).is_empty());
        assert!(session.state().has_object(&loaded.game, 0, 0, marker));
    }

    #[test]
    fn scene_message_effect_queues_popup_message() {
        let loaded = parse_game(
            r#"
title = scene_message_fixture
default_wait_time = 350ms
var hint = "Push the box"
puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .
rules {
}

levels {
legend {
P = Player
}

level "start" {
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
        session.apply_command(&loaded, "goto playing").unwrap();

        let presentation_events = session.take_presentation_events();
        assert_eq!(
            message_events(&presentation_events),
            vec![MessageEvent::Message {
                text: "Push the box".to_string()
            }]
        );
        assert_eq!(
            wait_events(&presentation_events),
            vec![WaitEvent::Wait { milliseconds: 350 }]
        );
    }

    #[test]
    fn scene_level_name_condition_scopes_lifecycle_message() {
        let loaded = parse_game(
            r#"
title = level_name_condition_message
var hint = "First level only"
puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .
rules {
}
levels {
legend {
P = Player
}
level "first"
P

level "second"
P
}
}
scene playing {
layout {
board.visible = false
puzzle board = default
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

        assert!(message_events(&session.take_presentation_events()).is_empty());

        session.start_level(&loaded, 1);
        assert!(message_events(&session.take_presentation_events()).is_empty());
    }

    #[test]
    fn puzzle_rule_message_effect_queues_popup_message() {
        let loaded = parse_game(
            r#"
title = puzzle_message_fixture
default_wait_time = 400ms
var hint = "Found"
puzzle default {
slots {
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

level "start" {
*
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        message_events(&session.take_presentation_events());

        let right = input_id_by_label(&loaded, "right").unwrap();
        session.apply_input(&loaded, right).unwrap();

        let presentation_events = session.take_presentation_events();
        assert_eq!(
            message_events(&presentation_events),
            vec![MessageEvent::Message {
                text: "Found".to_string()
            }]
        );
        assert_eq!(
            wait_events(&presentation_events),
            vec![WaitEvent::Wait { milliseconds: 400 }]
        );
    }

    #[test]
    fn message_effect_pauses_before_following_rule() {
        let loaded = parse_game(
            r#"
title = message_rule_segment_wait
default_wait_time = 450ms
puzzle default {
slots {
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
level "start" {
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
        let presentation_events = session.take_presentation_events();
        assert_eq!(
            message_events(&presentation_events),
            vec![MessageEvent::Message {
                text: "changed".to_string()
            }]
        );
        assert_eq!(
            wait_events(&presentation_events),
            vec![WaitEvent::Wait { milliseconds: 450 }]
        );
        session.resume_wait(&loaded).unwrap();
        assert!(!session.state().has_object(&loaded.game, 0, 0, b));
        assert!(session.state().has_object(&loaded.game, 0, 0, c));
    }

    #[test]
    fn routine_wait_pauses_one_undoable_turn() {
        let loaded = parse_game(
            r#"
title = routine_wait_segments
sounds {
sfx fall { seed = fall; type = hit }
}
puzzle default {
slots {
__legacy_layer_0 = A B C
}
empty .
rules {
[ A ] -> [ C ]
fall
}
routine fall {
[ C ] -> wait 100ms
[ C ] -> [ B ] sfx fall
}
levels {
legend {
A = A
B = B
C = C
}
level "start" {
A
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let a = object_named(&loaded, "A");
        let b = object_named(&loaded, "B");
        let c = object_named(&loaded, "C");

        session.apply_input(&loaded, InputId(0)).unwrap();

        assert!(!session.state().has_object(&loaded.game, 0, 0, a));
        assert!(!session.state().has_object(&loaded.game, 0, 0, b));
        assert!(session.state().has_object(&loaded.game, 0, 0, c));
        let presentation_events = session.take_presentation_events();
        assert_eq!(
            wait_events(&presentation_events),
            vec![WaitEvent::Wait { milliseconds: 100 }]
        );
        assert!(sound_events(&presentation_events).is_empty());

        session.resume_wait(&loaded).unwrap();
        assert!(session.state().has_object(&loaded.game, 0, 0, b));
        assert!(!session.state().has_object(&loaded.game, 0, 0, c));
        assert_eq!(
            sound_events(&session.take_presentation_events()),
            vec![SoundEvent::PlaySfx {
                name: "fall".to_string()
            }]
        );

        session.undo(&loaded);

        assert!(session.state().has_object(&loaded.game, 0, 0, a));
        assert!(!session.state().has_object(&loaded.game, 0, 0, b));
        assert!(!session.can_undo());
    }

    #[test]
    fn wait_before_first_state_change_keeps_single_undo_anchor() {
        let loaded = parse_game(
            r#"
title = wait_anchor_segments
puzzle default {
slots {
__legacy_layer_0 = A B
}
empty .
rules {
wait 100ms
[ A ] -> [ B ]
}
levels {
legend {
A = A
B = B
}
level "start" {
A
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let a = object_named(&loaded, "A");
        let b = object_named(&loaded, "B");

        session.apply_input(&loaded, InputId(0)).unwrap();

        assert!(session.state().has_object(&loaded.game, 0, 0, a));
        assert!(!session.state().has_object(&loaded.game, 0, 0, b));
        assert!(!session.can_undo());
        assert_eq!(
            wait_events(&session.take_presentation_events()),
            vec![WaitEvent::Wait { milliseconds: 100 }]
        );

        session.resume_wait(&loaded).unwrap();
        assert!(!session.state().has_object(&loaded.game, 0, 0, a));
        assert!(session.state().has_object(&loaded.game, 0, 0, b));
        assert!(session.can_undo());

        session.undo(&loaded);

        assert!(session.state().has_object(&loaded.game, 0, 0, a));
        assert!(!session.state().has_object(&loaded.game, 0, 0, b));
        assert!(!session.can_undo());
    }

    #[test]
    fn render_ascii_top_uses_loaded_legend() {
        let source =
            include_str!("../../../crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle");
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
title = overlap_render
puzzle default {
slots {
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
        let source =
            include_str!("../../../crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle");
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

        session.restart_level(&loaded).unwrap();
        assert_eq!(session.state(), &initial);
        assert!(!session.can_undo());
        assert!(!session.can_redo());

        session.undo(&loaded);
        assert_eq!(session.state(), &initial);
    }

    #[test]
    fn session_restart_uses_explicit_editor_start_state() {
        let loaded = parse_game(
            r#"
title = editor_state_start

puzzle board {
slots {
actor = Player
}
empty .
input right
rules {
input right [ Player | no Player ] -> [ | Player ]
}
levels {
legend {
. = empty
P = Player
}
level "authored" {
.P
}
level "editor_state" {
P.
}
}
}

scene playing {
layout {
puzzle board = board
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let authored = loaded.levels[0].initial_state.clone();
        let editor_start = loaded.levels[1].initial_state.clone();

        session
            .start_level_from_state(&loaded, 0, editor_start.clone(), false)
            .unwrap();
        assert_eq!(session.level_index(), 0);
        assert_eq!(session.state(), &editor_start);
        assert_ne!(session.state(), &authored);

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();
        let moved = session.state().clone();
        assert_ne!(moved, editor_start);
        assert!(session.can_undo());

        session.restart_level(&loaded).unwrap();
        assert_eq!(session.state(), &editor_start);
        assert_ne!(session.state(), &authored);
        assert!(!session.can_undo());
        assert!(!session.can_redo());

        session.undo(&loaded);
        assert_eq!(session.state(), &editor_start);
    }

    #[test]
    fn progress_save_restores_cleared_levels_by_name() {
        let loaded = parse_game(
            r#"
title = progress_fixture
puzzle default {
persistent var moves = 0

slots {
__legacy_layer_0 = Player
}
empty .

rules {
}
levels {
legend P = Player

level "first"
P

level "second"
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
        let source =
            include_str!("../../../crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle");
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
            .apply_command(&loaded, "goto playing(\"microban_01\")")
            .unwrap();

        assert_eq!(session.screen(), "playing");
        assert_eq!(session.level_index(), 0);

        let mut session = GameSession::new(&loaded);
        session.restore_progress_save_data(&loaded, &save).unwrap();
        let restored_world = session.routed_world().clone();
        session.apply_command(&loaded, "goto playing").unwrap();

        assert_eq!(session.screen(), "playing");
        assert_eq!(session.level_index(), 0);
        assert_eq!(
            session
                .world_state(&restored_world)
                .and_then(|world| world.active_level_index),
            Some(1)
        );
    }

    #[test]
    fn goto_level_param_accepts_quoted_dotted_level_name() {
        let loaded = parse_game(
            r#"
title = dotted_level

puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .
rules {
}
levels {
legend P = Player
level "first" {
P
}
level "test.chain" {
P
}
}
}

scene playing(level) {
layout {
puzzle board = default
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
            .apply_command(&loaded, "goto playing(\"test.chain\")")
            .unwrap();

        assert_eq!(session.scene(), "playing");
        assert_eq!(session.level_index(), 1);
    }

    #[test]
    fn goto_level_param_rejects_legacy_dotted_level_atom() {
        let loaded = parse_game(
            r#"
title = quoted_level

puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .
rules {
}
levels {
legend P = Player
level "microban.1" {
P
}
}
}

scene playing(level) {
layout {
puzzle board = default
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
            .apply_command(&loaded, "goto playing(microban.1)")
            .unwrap_err();

        assert_eq!(
            error,
            TransitionError::InvalidCommand("goto playing(microban.1)".to_string())
        );
    }

    #[test]
    fn goto_level_param_accepts_omitted_single_levels_collection() {
        let loaded = parse_game(
            r#"
title = omitted_level_collection

puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .
rules {
}
levels {
legend P = Player
level "first"
P
level "second"
P
}
}

scene playing(level) {
layout {
puzzle board = default
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_command(&loaded, "goto playing(1)").unwrap();
        assert_eq!(session.level_index(), 1);

        session
            .apply_command(&loaded, "goto playing(\"first\")")
            .unwrap();
        assert_eq!(session.level_index(), 0);
    }

    #[test]
    fn goto_level_param_accepts_named_level_collection_indexing() {
        let loaded = parse_game(
            r#"
title = named_level_collection

puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .
rules {
}
}

levels worldA of default {
legend P = Player
level "a"
P
}

levels worldB of default {
legend P = Player
level "b"
P
}

scene playing(level) {
layout {
puzzle board = default
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_command(&loaded, "goto playing(worldB[\"b\"])")
            .unwrap();
        assert_eq!(session.level_index(), 1);

        let error = session.apply_command(&loaded, "goto playing(levels[0])");
        assert!(matches!(error, Err(TransitionError::InvalidCommand(_))));
    }

    #[test]
    fn game_progress_effects_update_progress_primitives() {
        let loaded = parse_game(
            r#"
title = progress_effects

puzzle default {
persistent var score = 5

slots {
__legacy_layer_0 = Player
}
empty .

rules {
}

levels {
legend P = Player

level "first"
P

level "second"
P
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_command(&loaded, "current_level = second")
            .unwrap();
        session
            .apply_command(&loaded, "levels[\"second\"].cleared = true")
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
    fn scene_variable_assignment_copies_variable_value() {
        let loaded = parse_game(
            r#"
title = scene_var_assignment

var num = 0
var num_run = 7

puzzle default {
slots {
actor = Player
}
empty .

rules {
}

levels {
legend P = Player

level "start" {
P
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session.apply_command(&loaded, "num = num_run").unwrap();

        assert_eq!(
            session.session_values().get("num"),
            Some(&SceneValue::Int(7))
        );
    }

    #[test]
    fn default_actions_work_from_playing() {
        let source =
            include_str!("../../../crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle");
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
    fn explicit_model_input_target_must_resolve() {
        let loaded = parse_game(
            r#"
title = explicit_target_error

puzzle board {
input right

slots {
actor = Player
}
empty .

rules {
input right [ Player | no Player ] -> [ | Player ]
}

levels {
legend {
. = empty
P = Player
}
level "start" {
P.
}
}
}

scene playing {
layout {
puzzle board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();
        let initial = session.state().clone();

        let error = session
            .apply_model_input_to_target(&loaded, "missing_board", input_named(&loaded, "right"))
            .unwrap_err();

        assert_eq!(
            error,
            TransitionError::InvalidCommand("unknown puzzle target: missing_board".to_string())
        );
        assert_eq!(session.state(), &initial);
    }

    #[test]
    fn targeted_model_input_wait_completes_in_the_owning_world() {
        let loaded = parse_game(
            r#"
title = targeted_wait_atomicity

default_wait_time = 100ms

puzzle board {
input right

slots {
actor = Player
}
empty .

rules {
right [ Player ] -> wait 100ms
}

levels {
legend {
P = Player
}
level "start" {
P
}
}
}

scene playing {
layout {
puzzle board = board
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();
        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();
        assert_eq!(
            wait_events(&session.take_presentation_events()),
            vec![WaitEvent::Wait { milliseconds: 100 }]
        );
        let initial = session.state().clone();
        assert_eq!(session.state(), &initial);
    }

    #[test]
    fn signal_input_handler_steps_target_puzzle_for_direction_set() {
        let loaded = parse_game(
            r#"
title = signal_input_handler

puzzle board {
input right

slots {
actor = Player Marker
}
empty .
rules {
right [ Player ] -> [ Marker ]
}
levels {
legend {
P = Player
M = Marker
}
level "start" {
P
}
}
}

scene playing {
var input = signal none
layout {
puzzle board = board
}
on input in directions {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();
        let before = session
            .scene_state_for("playing")
            .unwrap()
            .puzzles
            .get("board")
            .unwrap()
            .state
            .clone();

        session.apply_command(&loaded, "input = right").unwrap();

        let after = &session
            .scene_state_for("playing")
            .unwrap()
            .puzzles
            .get("board")
            .unwrap()
            .state;
        assert_ne!(after, &before);
        assert_eq!(
            session
                .scene_state_for("playing")
                .unwrap()
                .values
                .get("input"),
            Some(&SceneValue::Symbol("none".to_string()))
        );
    }

    #[test]
    fn puzzle_next_level_advances_the_target_world_without_changing_focus() {
        let loaded = parse_game(
            r#"
title = next_level_target_scene

puzzle board {
slots {
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
level "one" {
*
P
}

level "two" {
P
}
}
}

scene title {
layout {
button "Play" -> goto playing
}
rules {
}
}

scene playing {
layout {
puzzle board = board
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
        assert_eq!(session.screen(), "playing");
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn persistent_vars_survive_undo_and_clear_undo_history_cuts_undo() {
        let loaded = parse_game(
            r#"
title = persistent_history

puzzle default {
persistent var cleared = false

slots {
__legacy_layer_0 = Player
}
empty .

rules {
if input == right {
once right [ Player | no Player ] -> [ | Player ] cleared = true
}
}

levels {
legend P = Player

level "start" {
P.
}
}
}

scene playing {
layout {
puzzle board = default
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
        assert_eq!(session.state().visible_variables(), &[1]);
        assert!(session.state().has_object(&loaded.game, 1, 0, player));
        assert!(session.can_undo());

        session.apply_command(&loaded, "undo").unwrap();
        assert_eq!(session.state().visible_variables(), &[1]);
        assert!(session.state().has_object(&loaded.game, 0, 0, player));

        session.apply_command(&loaded, "right").unwrap();
        assert!(session.can_undo());
        session
            .apply_command(&loaded, "clear_undo_history")
            .unwrap();
        assert!(!session.can_undo());
        assert!(!session.can_redo());
        assert_eq!(session.state().visible_variables(), &[1]);
    }

    #[test]
    fn puzzle_load_and_reset_do_not_depend_on_initial_scene_or_playing_name() {
        let source = r#"
title = puzzle_load_reset

puzzle default {
slots {
__legacy_layer_1 = Player
}
empty .

rules {
once right [ Player | ] -> [ | Player ]
}

levels {
legend P = Player

level "first" {
P.
}

level "second" {
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
layout {
puzzle board = default
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

        assert_eq!(session.screen(), "default");
        assert_eq!(
            loaded
                .scenes
                .iter()
                .find(|scene| scene.name == "title")
                .unwrap()
                .transitions
                .len(),
            0
        );

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
title = cancel_screen_transition

puzzle default {
slots {
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

level "start" {
A
}
}
}

scene playing {
layout {
puzzle board = default
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
        session.apply_command(&loaded, "goto playing").unwrap();

        session.apply_input(&loaded, tick).unwrap();

        assert_eq!(session.screen(), "playing");
    }

    #[test]
    fn puzzle_transition_only_runs_on_scenes_that_enable_main() {
        let source =
            include_str!("../../../crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle");
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
    fn direct_model_input_does_not_reach_level_when_level_select_is_focused() {
        let loaded = parse_game(
            r#"
title = focused_input
sounds {
sfx step { seed = step; type = jump }
}
puzzle default {
slots {
actor = Player
}
empty .

rules {
down [ Player | no Player ] -> [ | Player ] sfx step
}

levels {
legend {
. = empty
P = Player
}
level "start" {
P
.
}
}
}

scene playing {
layout {
puzzle board = default
}
rules {
step board
}
}

scene level_select {
layout {
level_menu
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let down = input_named(&loaded, "down");
        let initial = session.state().clone();

        session.apply_command(&loaded, "goto level_select").unwrap();
        assert_eq!(session.screen(), "level_select");
        assert!(!session.accepts_model_input(&loaded));

        session.apply_input(&loaded, down).unwrap();

        assert_eq!(session.screen(), "level_select");
        assert_eq!(session.state(), &initial);
        assert!(!session.can_undo());
        assert!(sound_events(&session.take_presentation_events()).is_empty());
    }

    #[test]
    fn direct_input_can_still_drive_focused_scene_input_transition() {
        let loaded = parse_game(
            r#"
title = scene_input_focus
puzzle default {
slots {
actor = Player
}
empty .
input open
rules {
}
levels {
legend P = Player
level "start" {
P
}
}
}

scene title {
layout {
text "Title"
}
rules {
if input == open -> goto playing
}
}

scene playing {
layout {
puzzle board = default
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto title").unwrap();

        session
            .apply_input(&loaded, input_named(&loaded, "open"))
            .unwrap();

        assert_eq!(session.screen(), "playing");
        assert!(session.accepts_model_input(&loaded));
        assert_eq!(session.active_level_index(), Some(0));
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
    fn world_instances_keep_state_and_history_when_focus_routes_elsewhere() {
        let loaded = parse_game(
            r#"
title = independent_world_instances
puzzle board {
input right
slots {
actor = Player
}
empty .
rules {
[ Player | no Player ] -> [ no Player | Player ]
}
levels {
legend P = Player
P.
}
}
scene left {
layout {
puzzle board = board
}
rules {
step board
}
}
scene right {
layout {
puzzle board = board
}
rules {
step board
}
}
"#,
        )
        .unwrap();
        let left = WorldInstanceId {
            scene: "left".to_string(),
            puzzle: "board".to_string(),
        };
        let right = WorldInstanceId {
            scene: "right".to_string(),
            puzzle: "board".to_string(),
        };
        let input = input_named(&loaded, "right");
        let mut session = GameSession::new(&loaded);

        session.apply_command(&loaded, "goto left").unwrap();
        let initial = session.state().clone();
        session.apply_input(&loaded, input).unwrap();
        let left_moved = session.state().clone();
        assert_ne!(left_moved, initial);

        session.apply_command(&loaded, "goto right").unwrap();
        assert_eq!(session.state(), &initial);
        assert!(session.world_state(&left).unwrap().history.can_undo());
        assert!(!session.world_state(&right).unwrap().history.can_undo());
        session.apply_input(&loaded, input).unwrap();

        session.apply_command(&loaded, "goto left").unwrap();
        assert_eq!(session.state(), &left_moved);
        session.undo(&loaded);
        assert_eq!(session.state(), &initial);
        assert_eq!(session.world_state(&right).unwrap().state, left_moved);
        assert!(session.world_state(&right).unwrap().history.can_undo());
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
        load_named_scene_level(
            &loaded,
            &mut session,
            "checkpoint.spec_board",
            "spec",
            "hub",
        );
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
title = sequence_saved_puzzle

puzzle default {
var marks = 0

slots {
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

level "hub" {
P
}

level "level" {
P
}
}
}

scene hub {
layout {
puzzle board = default level hub
}
rules {
step board
}
}

scene playing {
layout {
puzzle board = default
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

        let hub_scene = session.scene_state().unwrap();
        assert!(
            hub_scene.puzzles.contains_key("board"),
            "focused scene `{}` has puzzle instances {:?}",
            session.screen(),
            hub_scene.puzzles.keys().collect::<Vec<_>>()
        );
        let hub_board = &hub_scene.puzzles["board"];
        assert_eq!(session.screen(), "hub");
        assert_eq!(hub_board.visible_variables(), &[1]);
    }

    #[test]
    fn session_advances_level_after_nonfinal_clear() {
        let loaded = transition_fixture();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();

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
        session.apply_command(&loaded, "goto playing").unwrap();

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
title = level_clear_hook
puzzle sokoban {
slots {
floor = Goal
actor = Player Box Wall
marker = ClearMark
@visual = @ClearVisual
}
groups {
solid = Player Box Wall
}
win_conditions {
some Goal
all Goal on Box
}
on_level_clear {
[ Goal Box no ClearMark ] -> [ Goal Box ClearMark ]
[ Goal Box no @ClearVisual ] -> [ Goal Box @ClearVisual ]
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
level "start" {
#####
#PBG#
#####
}
}
}
scene playing {
layout {
puzzle board = sokoban
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
        session.apply_command(&loaded, "goto playing").unwrap();
        let right = *loaded.controls.keys.get(&b'd').unwrap();
        let clear_mark = object_named(&loaded, "ClearMark");
        let clear_visual = object_named(&loaded, "@ClearVisual");

        session.apply_input(&loaded, right).unwrap();

        let completion = session
            .last_level_completion()
            .expect("clear must retain the state that satisfied completion");
        assert!(
            !completion
                .state()
                .has_object(&loaded.game, 3, 1, clear_mark)
        );
        assert!(
            !completion
                .state()
                .has_object(&loaded.game, 3, 1, clear_visual)
        );
        assert_eq!(session.screen(), "level_select");
        assert!(session.state().has_object(&loaded.game, 3, 1, clear_mark));
        assert!(session.state().has_object(&loaded.game, 3, 1, clear_visual));
    }

    #[test]
    fn wait_before_next_level_pauses_before_navigation() {
        let loaded = parse_game(
            r#"
title = wait_clear_snapshot
puzzle sokoban {
slots {
floor = Goal
actor = Player Box Wall
}
groups {
solid = Player Box Wall
}
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
scene playing {
layout {
puzzle board = sokoban
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

        let completion = session
            .last_level_completion()
            .expect("winning input must retain its pre-navigation observation");
        assert_eq!(completion.level_index(), 0);
        assert!(loaded.is_goal_complete(completion.state()));
        assert_eq!(session.level_index(), 0);
        assert!(session.is_waiting());
        let events = session.take_presentation_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].kind,
            PresentationEventKind::Wait(WaitEvent::Wait { milliseconds: 1000 })
        );
        assert_eq!(events[0].context.level_index, Some(0));

        session.resume_wait(&loaded).unwrap();
        assert_eq!(session.level_index(), 1);
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn lifecycle_animation_wait_pauses_before_navigation() {
        let loaded = parse_game(
            r#"
title = lifecycle_animation_timeline
puzzle default {
render {
tween = true
tween_duration = 80ms
}
slots {
actor = Player
}
empty .
rules {
input right [ Player ] -> [ Player ] win
}
on_level_clear {
[ Player | no Player ] -> [ | Player ]
wait animation
next_level
}
levels {
legend {
. = empty
P = Player
}
level "first" {
P.
}
level "second" {
P.
}
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);

        session
            .apply_input(&loaded, input_named(&loaded, "right"))
            .unwrap();

        assert_eq!(session.level_index(), 0);
        assert!(session.is_waiting());
        let events = session.take_presentation_events();
        assert!(events.len() >= 2);
        assert!(events[..events.len() - 1].iter().all(|event| matches!(
            event.kind,
            PresentationEventKind::Animation(AnimationEvent::Move { .. })
        )));
        assert_eq!(
            events.last().unwrap().kind,
            PresentationEventKind::Wait(WaitEvent::Wait { milliseconds: 80 })
        );
        assert!(
            events
                .iter()
                .all(|event| event.context.level_index == Some(0))
        );

        session.resume_wait(&loaded).unwrap();
        assert_eq!(session.level_index(), 1);
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn wait_statement_pauses_and_resumes_the_same_turn() {
        let loaded = parse_game(
            r#"
title = wait_statement_segments
puzzle default {
slots {
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
level "start" {
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
        assert!(session.is_waiting());
        assert_eq!(
            wait_events(&session.take_presentation_events()),
            vec![WaitEvent::Wait { milliseconds: 1000 }]
        );

        session.resume_wait(&loaded).unwrap();

        assert!(!session.state().has_object(&loaded.game, 0, 0, b));
        assert!(session.state().has_object(&loaded.game, 0, 0, c));
        assert!(!session.is_waiting());
    }

    #[test]
    fn screen_transition_can_goto_level_with_payload() {
        let source = r#"
title = level_select_payload

puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .

rules {
}

levels {
legend P = Player
level "first" {
P
}
level "second" {
P
}
}
}

scene playing {
layout {
puzzle board = default
}
}

scene level_select {
layout {
level_menu
}
}
"#;
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
    fn level_menu_position_select_restarts_current_level_state() {
        let source = r#"
title = level_menu_position_restart

puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .

rules {
right [ Player | no Player ] -> [ | Player ]
}

levels {
legend {
. = empty
P = Player
}

level "first" {
P.
}
level "second" {
P.
}
}
}

scene playing {
layout {
puzzle board = default
}
rules {
step board
}
}

scene level_select {
layout {
level_menu
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);
        session.start_level(&loaded, 1);
        let initial = loaded.levels[1].initial_state.clone();

        session.apply_command(&loaded, "right").unwrap();
        assert_ne!(session.state(), &initial);

        session.apply_command(&loaded, "goto level_select").unwrap();
        assert_eq!(session.level_index(), 1);
        assert_eq!(session.screen(), "level_select");

        session.apply_command(&loaded, "select:1").unwrap();

        assert_eq!(session.level_index(), 1);
        assert_eq!(session.screen(), "playing");
        assert_eq!(session.state(), &initial);
        assert!(!session.can_undo());
    }

    #[test]
    fn scene_level_commands_can_target_level_scene() {
        let loaded = parse_game(
            r#"
title = scene_level_commands
puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .
rules {
}
levels {
legend P = Player
level "first" {
P
}
level "second" {
P
}
level "third" {
P
}
}
}

scene playing {
layout {
puzzle board = default
}
}

scene level_clear {
layout {
puzzle board = default
}
rules {
}
}
"#,
        )
        .unwrap();
        let mut session = GameSession::new(&loaded);
        let playing_world = WorldInstanceId {
            scene: "playing".to_string(),
            puzzle: "board".to_string(),
        };

        session.apply_command(&loaded, "goto level_clear").unwrap();
        session
            .apply_command(&loaded, "playing.next_level")
            .unwrap();
        assert_eq!(session.level_index(), 0);
        assert_eq!(session.screen(), "level_clear");
        assert_eq!(
            session
                .world_state(&playing_world)
                .and_then(|world| world.active_level_index),
            Some(1)
        );

        session.apply_command(&loaded, "goto level_clear").unwrap();
        session
            .apply_command(&loaded, "playing.previous_level")
            .unwrap();
        assert_eq!(session.level_index(), 0);
        assert_eq!(session.screen(), "level_clear");
        assert_eq!(
            session
                .world_state(&playing_world)
                .and_then(|world| world.active_level_index),
            Some(0)
        );

        session.apply_command(&loaded, "goto level_clear").unwrap();
        session
            .apply_command(&loaded, "playing.goto third")
            .unwrap();
        assert_eq!(session.level_index(), 2);
        assert_eq!(session.screen(), "playing");
        assert_eq!(
            session
                .world_state(&playing_world)
                .and_then(|world| world.active_level_index),
            Some(2)
        );
    }

    #[test]
    fn level_menu_component_owns_level_menu_commands() {
        let source = r#"
title = level_menu_commands

puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level "first" {
P
}
level "second" {
P
}
}
}

scene playing {
layout {
puzzle board = default
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
title = scene_level_resources

puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .


rules {
}
}

levels worldA of default {
legend P = Player
level "1"
P
level "2"
P
}

levels worldB of default {
legend P = Player
level "1"
P
level "2"
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
layout {
puzzle board = default
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto level_select").unwrap();

        session.apply_command(&loaded, "select").unwrap();
        assert_eq!(session.level_index(), 2);
        assert_eq!(
            loaded.levels[session.level_index()].pack.as_deref(),
            Some("worldB")
        );
        assert_eq!(loaded.levels[session.level_index()].name, "1");

        session.advance_level(&loaded);
        assert_eq!(session.level_index(), 3);
        assert_eq!(
            loaded.levels[session.level_index()].pack.as_deref(),
            Some("worldB")
        );
        assert_eq!(loaded.levels[session.level_index()].name, "2");

        session.advance_level(&loaded);
        assert_eq!(session.level_index(), 3);
    }

    #[test]
    fn level_menu_matrix_navigation_uses_columns() {
        let source = r#"
title = level_menu_matrix

puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level "first" {
P
}
level "second" {
P
}
level "third" {
P
}
level "fourth" {
P
}
level "fifth" {
P
}
}
}

scene playing {
layout {
puzzle board = default
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
title = level_menu_default_select

puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level "first" {
P
}
level "second" {
P
}
}
}

scene playing {
layout {
puzzle board = default
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
title = level_menu_buttons

puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player
level "first" {
P
}
level "second" {
P
}
}
}

scene playing {
layout {
puzzle board = default
}
}

scene title {
layout {
text "Title"
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
        assert_eq!(session.selected_level_index(), loaded.levels.len());

        session.apply_command(&loaded, "select").unwrap();
        assert_eq!(session.screen(), "title");
    }

    #[test]
    fn goto_preserves_fixed_scene_state_without_history_stack() {
        let source = r#"
title = screen_history
puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level "start" {
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
layout {
mark = empty
text "B"
}
}

scene c {
layout {
mark = empty
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
title = persistent_scene_var
puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level "start" {
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
title = scene_param_rejection
puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level "start" {
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
title = scene_state_words
puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level "first" {
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
title = level_ref_params
puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player

level "first" {
P
}

level "second" {
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
            .apply_command(&loaded, "levels[\"first\"].cleared = true")
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
title = screen_focus
puzzle default {
slots {
__legacy_layer_0 = Player
}
empty .


rules {
}

levels {
legend P = Player
level "start" {
P
}
}
}

scene playing {
layout {
puzzle board = default
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
        session.apply_command(&loaded, "goto playing").unwrap();

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
title = scene_state
puzzle default {
slots {
actor = Player
}

rules {
}

levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}
}

scene playing {
layout {
puzzle board = default
message_visible = true
moves = 0
message = "Read this"
}
keys {
q -> goto level_select
}
}

scene level_select {
layout {
message = "Browse"
level_menu
}
keys {
Escape -> goto playing
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut session = GameSession::new(&loaded);
        session.apply_command(&loaded, "goto playing").unwrap();

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
title = rule_next_level

puzzle board {
slots {
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
level "one" {
*
P
}

level "two" {
P
}
}
}

scene playing {
layout {
puzzle board = board
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
        assert_eq!(session.screen(), "playing");
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn puzzle_condition_effect_can_advance_to_next_level() {
        let loaded = parse_game(
            r#"
title = condition_next_level

puzzle board {
slots {
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
level "one" {
*
P
}

level "two" {
P
}
}
}

scene playing {
layout {
puzzle board = board
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
        assert_eq!(session.screen(), "playing");
        assert_eq!(session.state(), &loaded.levels[1].initial_state);
    }

    #[test]
    fn level_start_runs_at_runtime_without_baking_initial_state() {
        let loaded = parse_game(
            r#"
title = runtime_level_start

puzzle board {
slots {
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

level "one" {
P
}
}
}

scene playing {
layout {
puzzle board = board
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
            message_events(&session.take_presentation_events()),
            vec![MessageEvent::Message {
                text: "started".to_string()
            }]
        );
    }

    #[test]
    fn level_body_message_sugar_runs_on_runtime_lifecycle() {
        let loaded = parse_game(
            r#"
title = level_message_sugar

puzzle board {
slots {
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

level "one" {
message "enter one"
P
message "clear one"
}
}
}

scene playing {
layout {
puzzle board = board
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
            message_events(&session.take_presentation_events()),
            vec![MessageEvent::Message {
                text: "enter one".to_string()
            }]
        );

        let tick = input_named(&loaded, "tick");
        session.apply_input(&loaded, tick).unwrap();

        assert_eq!(
            message_events(&session.take_presentation_events()),
            vec![MessageEvent::Message {
                text: "clear one".to_string()
            }]
        );
    }

    #[test]
    fn title_scene_does_not_start_first_level_lifecycle() {
        let loaded = parse_game(
            r#"
title = title_level_start_boundary

scene title {
layout {
button "Play" -> goto playing
}
}

puzzle board {
slots {
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

level "one" {
message "enter one"
P
}
}
}

scene playing {
layout {
puzzle board = board
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
        assert!(session.scene_state().unwrap().puzzles.is_empty());
        assert!(message_events(&session.take_presentation_events()).is_empty());

        session.apply_command(&loaded, "goto playing").unwrap();
        assert_eq!(session.scene(), "playing");
        assert_eq!(session.active_level_index(), Some(0));
        assert_eq!(
            message_events(&session.take_presentation_events()),
            vec![MessageEvent::Message {
                text: "enter one".to_string()
            }]
        );
    }

    #[test]
    fn rule_next_level_still_runs_model_clear_and_scene_conditions() {
        let loaded = parse_game(
            r#"
title = rule_next_level_turn_completion

puzzle board {
persistent var clear_seen = false

slots {
floor = Goal
actor = Box Player
}


input tick

win_conditions {
some Goal
all Goal on Box
}

on_level_clear {
once [ Goal Box ] -> clear_seen = true
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
level "one" {
*
P
}

level "two" {
P
}
}
}

scene playing {
layout {
puzzle board = board
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

        session.apply_command(&loaded, "goto playing").unwrap();
        session.apply_input(&loaded, tick).unwrap();

        assert_eq!(session.level_index(), 1);
        assert!(session.cleared_levels()[0]);
        assert_eq!(
            message_events(&session.take_presentation_events()),
            vec![MessageEvent::Message {
                text: "clear".to_string()
            }]
        );
        assert_eq!(session.state().visible_variables(), &[1]);
    }

    #[test]
    fn headless_semantic_input_matches_player_again_completion() {
        let loaded = parse_game(include_str!("../tests/fixtures/again_atomic.puzzle")).unwrap();
        let right = input_named(&loaded, "right");
        let mut player = GameSession::new(&loaded);
        player.start_level(&loaded, 0);
        let initial = player.state().clone();

        player.apply_input(&loaded, right).unwrap();

        let mut headless = HeadlessSession::from_level_state(&loaded, 0, initial).unwrap();
        headless.apply_input(&loaded, right).unwrap();
        assert_eq!(headless.state(), player.state());
        assert!(headless.completed());
        assert!(loaded.is_goal_complete(headless.state()));
    }

    #[test]
    fn input_modes_materialize_only_the_requested_outputs() {
        let loaded = parse_game(
            r#"
title = headless_presentation_fixture
sounds {
sfx move { seed = move01; type = jump }
}
puzzle default {
slots {
actor = Player
}
rules {
input right [ Player | no Player ] -> [ | Player ] sfx move
}
levels {
legend {
. = empty
P = Player
}
level "start" {
P.
}
}
}
"#,
        )
        .unwrap();
        let right = input_named(&loaded, "right");
        let mut player = GameSession::new(&loaded);
        let initial = player.state().clone();
        let programs = loaded.programs_for_level(0).unwrap();

        let player_outcome = transition_program_outcome_with_effects(
            &loaded,
            &programs,
            &initial,
            right,
            None,
            InputExecutionMode::Player,
        )
        .unwrap();
        assert!(matches!(
            player_outcome.firings,
            GridCapturedFirings::Summary(_)
        ));
        let traced_outcome = transition_program_outcome_with_effects(
            &loaded,
            &programs,
            &initial,
            right,
            None,
            InputExecutionMode::PlayerTrace,
        )
        .unwrap();
        assert!(matches!(
            traced_outcome.firings,
            GridCapturedFirings::Detailed(_)
        ));
        let headless_outcome = transition_program_outcome_with_effects(
            &loaded,
            &programs,
            &initial,
            right,
            None,
            InputExecutionMode::Headless,
        )
        .unwrap();
        assert!(matches!(
            headless_outcome.firings,
            GridCapturedFirings::Summary(_)
        ));

        player.apply_input(&loaded, right).unwrap();
        assert!(!player.take_presentation_events().is_empty());
        assert!(player.last_transition_trace().is_none());

        let mut traced = GameSession::new(&loaded);
        traced.apply_traced_input(&loaded, right).unwrap();
        assert!(!traced.take_presentation_events().is_empty());
        assert!(traced.last_transition_trace().is_some());

        let mut headless = HeadlessSession::from_level_state(&loaded, 0, initial).unwrap();
        headless.apply_input(&loaded, right).unwrap();
        assert_eq!(headless.state(), player.state());
        let mut session = headless.into_session();
        assert!(session.take_presentation_events().is_empty());
        assert!(session.last_transition_trace().is_none());
    }

    #[test]
    fn headless_search_key_distinguishes_checkpoint_and_scene_context() {
        let loaded = parse_game(
            r#"
title = headless_context_key
puzzle default {
slots { Player }
input save
rules {
if input == save { checkpoint }
}
levels {
legend { P = Player }
level "start" { P }
}
}
scene playing {
layout { puzzle board = default }
}
scene menu {
layout { text "Menu" }
}
"#,
        )
        .unwrap();
        let base = GameSession::new(&loaded);
        let base_key = HeadlessSession::from_game_session(base.clone(), 0)
            .unwrap()
            .search_key();

        let mut checkpoint = base.clone();
        checkpoint
            .apply_input(&loaded, input_named(&loaded, "save"))
            .unwrap();
        let checkpoint_key = HeadlessSession::from_game_session(checkpoint, 0)
            .unwrap()
            .search_key();
        assert_ne!(checkpoint_key, base_key);

        let mut scene = base;
        scene.apply_command(&loaded, "goto menu").unwrap();
        let scene_key = HeadlessSession::from_game_session(scene, 0)
            .unwrap()
            .search_key();
        assert_ne!(scene_key, base_key);
    }
}
