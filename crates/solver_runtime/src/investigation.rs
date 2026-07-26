use puzzle_core::{GridCoord, InputId, ObjectId, Size2, State, TransitionCommand};
use puzzle_lang::{LoadedDocumentModel, LoadedGame};
use puzzle_play::{GameSession, TransitionTrace, cell_objects};
use puzzle_solver::{
    GridPuzzleDomain, GridSearchState, GridStateKey, ResumableAdvanceOutcome, ResumableBestFirst,
    ResumablePauseReason, ResumableSearchAllowance, ResumableSearchCandidate,
    ResumableSearchLimits, ResumableSearchStatus, SearchBudget, SearchDomain, SearchOutcome,
    SearchStats, best_first, exact_bfs,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

type AgentSearchDomain = GridPuzzleDomain<2, Size2>;
type AgentSearchState = GridSearchState<2, Size2>;
type AgentStateKey = GridStateKey<2>;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvestigationError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl InvestigationError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

#[derive(Default)]
pub struct InvestigationService {
    next_session: u64,
    sessions: HashMap<String, InvestigationSession>,
}

impl InvestigationService {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum InvestigationCommand {
    Compile {
        path: String,
        #[serde(default)]
        model: Option<String>,
    },
    Manifest {
        session_id: String,
    },
    Run {
        session_id: String,
        from_state_id: String,
        inputs: Vec<String>,
        #[serde(default)]
        observation: ObservationRequest,
    },
    InspectState {
        session_id: String,
        state_id: String,
    },
    ExportSemanticState {
        session_id: String,
        state_id: String,
    },
    ImportSemanticState {
        session_id: String,
        artifact: SemanticStateArtifact,
    },
    DeriveState(DeriveStateRequest),
    StartLevelFromState(StartLevelFromStateRequest),
    ImportSemanticGoal {
        session_id: String,
        artifact: SemanticGoalArtifact,
    },
    EvaluateSemanticGoal {
        session_id: String,
        goal_id: String,
        state_id: String,
    },
    SolveSemanticGoal {
        session_id: String,
        goal_id: String,
        from_state_id: String,
        algorithm: SemanticGoalSearchAlgorithm,
        budget: SemanticGoalSearchBudget,
    },
    CreateSearch {
        session_id: String,
        goal_id: String,
        from_state_id: String,
        algorithm: SemanticGoalSearchAlgorithm,
        limits: SearchSessionLimits,
    },
    AdvanceSearch {
        session_id: String,
        search_id: String,
        allowance: SearchSessionAllowance,
    },
    InspectSearch {
        session_id: String,
        search_id: String,
        candidate_limit: usize,
    },
    MaterializeSearchCandidate {
        session_id: String,
        search_id: String,
        candidate_id: String,
    },
    CloseSearch {
        session_id: String,
        search_id: String,
    },
    InspectRun {
        session_id: String,
        run_id: String,
        at: Vec<usize>,
        #[serde(default)]
        include_trace: bool,
    },
    CompareStates {
        session_id: String,
        left_state_id: String,
        right_state_id: String,
    },
    Close {
        session_id: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationRequest {
    #[serde(default)]
    pub mode: ObservationMode,
    #[serde(default)]
    pub indices: Vec<usize>,
}

impl Default for ObservationRequest {
    fn default() -> Self {
        Self {
            mode: ObservationMode::Summary,
            indices: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservationMode {
    #[default]
    Summary,
    Events,
    Indices,
    All,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticStateArtifact {
    pub version: u32,
    pub kind: String,
    pub base_state_id: String,
    pub base_state_hash: String,
    pub level_index: usize,
    pub level_name: String,
    pub width: u16,
    pub height: u16,
    pub empty: String,
    pub legend: BTreeMap<String, SemanticLegendMeaning>,
    pub lines: Vec<String>,
    pub variables: BTreeMap<String, i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticObjectPositions {
    pub object: String,
    pub positions: Vec<[u16; 2]>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticVariableValue {
    pub variable: String,
    pub value: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeriveStateRequest {
    pub session_id: String,
    pub base_state_id: String,
    #[serde(default)]
    pub expected_base_hash: Option<String>,
    #[serde(default)]
    pub set_object_positions: Vec<SemanticObjectPositions>,
    #[serde(default)]
    pub set_variables: Vec<SemanticVariableValue>,
    #[serde(default, rename = "assert")]
    pub assertions: Vec<SemanticStateAssertion>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartLevelFromStateRequest {
    pub session_id: String,
    pub level_index: usize,
    #[serde(default)]
    pub expected_level_name: Option<String>,
    #[serde(default)]
    pub set_object_positions: Vec<SemanticObjectPositions>,
    #[serde(default)]
    pub set_variables: Vec<SemanticVariableValue>,
    #[serde(default, rename = "assert")]
    pub assertions: Vec<SemanticStateAssertion>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemanticStateAssertion {
    Exact {
        position: [u16; 2],
        objects: Vec<String>,
    },
    Contains {
        position: [u16; 2],
        objects: Vec<String>,
    },
    Excludes {
        position: [u16; 2],
        objects: Vec<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticGoalArtifact {
    pub version: u32,
    pub kind: String,
    pub base_state_id: String,
    pub base_state_hash: String,
    pub level_index: usize,
    pub level_name: String,
    pub width: u16,
    pub height: u16,
    pub empty: String,
    pub legend: BTreeMap<String, SemanticLegendMeaning>,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SemanticLegendMeaning {
    Exact { objects: Vec<String> },
    Contains { objects: Vec<String> },
    Excludes { objects: Vec<String> },
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticGoalSearchAlgorithm {
    Bfs,
    BestFirst,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticGoalSearchBudget {
    pub max_depth: u32,
    pub max_stored_nodes: usize,
    pub max_millis: u64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSessionLimits {
    pub max_depth: u32,
    pub max_stored_nodes: usize,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSessionAllowance {
    pub max_expanded_nodes: usize,
    pub max_millis: u64,
}

impl InvestigationService {
    pub fn dispatch(&mut self, command: InvestigationCommand) -> Result<Value, InvestigationError> {
        match command {
            InvestigationCommand::Compile { path, model } => self.compile(&path, model.as_deref()),
            InvestigationCommand::Manifest { session_id } => {
                Ok(self.session(&session_id)?.manifest(&session_id))
            }
            InvestigationCommand::Run {
                session_id,
                from_state_id,
                inputs,
                observation,
            } => self.session_mut(&session_id)?.run(
                &session_id,
                &from_state_id,
                &inputs,
                &observation,
            ),
            InvestigationCommand::InspectState {
                session_id,
                state_id,
            } => self.session(&session_id)?.inspect_state(&state_id),
            InvestigationCommand::ExportSemanticState {
                session_id,
                state_id,
            } => self.session(&session_id)?.export_semantic_state(&state_id),
            InvestigationCommand::ImportSemanticState {
                session_id,
                artifact,
            } => self
                .session_mut(&session_id)?
                .import_semantic_state(&artifact),
            InvestigationCommand::DeriveState(request) => {
                self.session_mut(&request.session_id)?.derive_state(
                    &request.base_state_id,
                    request.expected_base_hash.as_deref(),
                    &request.set_object_positions,
                    &request.set_variables,
                    &request.assertions,
                )
            }
            InvestigationCommand::StartLevelFromState(request) => self
                .session_mut(&request.session_id)?
                .start_level_from_state(
                    request.level_index,
                    request.expected_level_name.as_deref(),
                    &request.set_object_positions,
                    &request.set_variables,
                    &request.assertions,
                ),
            InvestigationCommand::ImportSemanticGoal {
                session_id,
                artifact,
            } => self
                .session_mut(&session_id)?
                .import_semantic_goal(&artifact),
            InvestigationCommand::EvaluateSemanticGoal {
                session_id,
                goal_id,
                state_id,
            } => self
                .session(&session_id)?
                .evaluate_semantic_goal(&goal_id, &state_id),
            InvestigationCommand::SolveSemanticGoal {
                session_id,
                goal_id,
                from_state_id,
                algorithm,
                budget,
            } => self.session_mut(&session_id)?.solve_semantic_goal(
                &session_id,
                &goal_id,
                &from_state_id,
                algorithm,
                budget,
            ),
            InvestigationCommand::CreateSearch {
                session_id,
                goal_id,
                from_state_id,
                algorithm,
                limits,
            } => self.session_mut(&session_id)?.create_search(
                &session_id,
                &goal_id,
                &from_state_id,
                algorithm,
                limits,
            ),
            InvestigationCommand::AdvanceSearch {
                session_id,
                search_id,
                allowance,
            } => self
                .session_mut(&session_id)?
                .advance_search(&session_id, &search_id, allowance),
            InvestigationCommand::InspectSearch {
                session_id,
                search_id,
                candidate_limit,
            } => {
                self.session(&session_id)?
                    .inspect_search(&session_id, &search_id, candidate_limit)
            }
            InvestigationCommand::MaterializeSearchCandidate {
                session_id,
                search_id,
                candidate_id,
            } => self.session_mut(&session_id)?.materialize_search_candidate(
                &session_id,
                &search_id,
                &candidate_id,
            ),
            InvestigationCommand::CloseSearch {
                session_id,
                search_id,
            } => self
                .session_mut(&session_id)?
                .close_search(&session_id, &search_id),
            InvestigationCommand::InspectRun {
                session_id,
                run_id,
                at,
                include_trace,
            } => self
                .session(&session_id)?
                .inspect_run(&run_id, &at, include_trace),
            InvestigationCommand::CompareStates {
                session_id,
                left_state_id,
                right_state_id,
            } => self
                .session(&session_id)?
                .compare_states(&left_state_id, &right_state_id),
            InvestigationCommand::Close { session_id } => {
                if self.sessions.remove(&session_id).is_none() {
                    return Err(InvestigationError::new(
                        "unknown_session",
                        format!("agent session {session_id:?} does not exist"),
                    ));
                }
                Ok(json!({ "sessionId": session_id, "closed": true }))
            }
        }
    }

    fn compile(
        &mut self,
        raw_path: &str,
        model: Option<&str>,
    ) -> Result<Value, InvestigationError> {
        let entry = puzzle_lang::resolve_game_entry(raw_path)
            .map_err(|error| InvestigationError::new("compile_failed", error.to_string()))?;
        let root = entry.parent().unwrap_or_else(|| std::path::Path::new("."));
        let workspace = puzzle_workspace::FileWorkspace::load(&entry, root)
            .map_err(|error| InvestigationError::new("compile_failed", error))?;
        let document = workspace
            .compile()
            .map_err(|error| InvestigationError::new("compile_failed", error.to_string()))?;
        let source_hash =
            workspace
                .documents()
                .iter()
                .fold(String::new(), |mut source, document| {
                    source.push_str(&document.path);
                    source.push('\0');
                    source.push_str(&document.source);
                    source.push('\0');
                    source
                });
        let (model_name, game) = select_2d_model(document.models, model)?;

        self.next_session += 1;
        let session_id = format!("session-{}", self.next_session);
        let session = InvestigationSession::new(
            entry.to_string_lossy().into_owned(),
            stable_hash_hex(source_hash.as_bytes()),
            model_name,
            game,
        )?;
        let initial_states = session.initial_state_summaries();
        let source_hash = session.source_hash.clone();
        let model_name = session.model_name.clone();
        self.sessions.insert(session_id.clone(), session);
        Ok(json!({
            "sessionId": session_id,
            "model": model_name,
            "modelKind": "puzzle2d",
            "sourceHash": source_hash,
            "initialStates": initial_states,
        }))
    }

    fn session(&self, session_id: &str) -> Result<&InvestigationSession, InvestigationError> {
        self.sessions.get(session_id).ok_or_else(|| {
            InvestigationError::new(
                "unknown_session",
                format!("agent session {session_id:?} does not exist"),
            )
        })
    }

    fn session_mut(
        &mut self,
        session_id: &str,
    ) -> Result<&mut InvestigationSession, InvestigationError> {
        self.sessions.get_mut(session_id).ok_or_else(|| {
            InvestigationError::new(
                "unknown_session",
                format!("agent session {session_id:?} does not exist"),
            )
        })
    }
}

fn select_2d_model(
    models: Vec<LoadedDocumentModel>,
    requested: Option<&str>,
) -> Result<(String, LoadedGame), InvestigationError> {
    if let Some(requested) = requested {
        for model in models {
            match model {
                LoadedDocumentModel::Puzzle2d { name, game } if name == requested => {
                    return Ok((name, game));
                }
                LoadedDocumentModel::Puzzle3d { name, .. } if name == requested => {
                    return Err(InvestigationError::new(
                        "unsupported_model_kind",
                        "Agent investigation supports puzzle2d models only",
                    ));
                }
                _ => {}
            }
        }
        return Err(InvestigationError::new(
            "unknown_model",
            format!("document has no model named {requested:?}"),
        ));
    }

    if models.len() != 1 {
        let names = models
            .iter()
            .map(|model| match model {
                LoadedDocumentModel::Puzzle2d { name, .. }
                | LoadedDocumentModel::Puzzle3d { name, .. } => name.clone(),
            })
            .collect::<Vec<_>>();
        return Err(InvestigationError::new(
            "ambiguous_model",
            "agent compile requires model when the document does not contain exactly one model",
        )
        .with_details(json!({ "models": names })));
    }
    match models.into_iter().next().expect("one model was checked") {
        LoadedDocumentModel::Puzzle2d { name, game } => Ok((name, game)),
        LoadedDocumentModel::Puzzle3d { .. } => Err(InvestigationError::new(
            "unsupported_model_kind",
            "Agent investigation supports puzzle2d models only",
        )),
    }
}

struct InvestigationSession {
    entry_path: String,
    source_hash: String,
    model_name: String,
    game: LoadedGame,
    inputs_by_name: BTreeMap<String, InputId>,
    next_state: u64,
    next_run: u64,
    next_goal: u64,
    next_search: u64,
    states: HashMap<String, StateRecord>,
    initial_state_ids: Vec<String>,
    runs: HashMap<String, RunRecord>,
    goals: HashMap<String, SemanticGoal>,
    searches: HashMap<String, SearchSession>,
}

#[derive(Clone)]
struct StateRecord {
    level_index: usize,
    state: State,
    replay: StateReplay,
}

#[derive(Clone)]
enum StateReplay {
    Reachable {
        start_level_index: usize,
        inputs: Vec<InputId>,
    },
    Hypothetical {
        origin: HypotheticalOrigin,
        base_state_id: String,
        root_level_index: usize,
        root_state: State,
        inputs: Vec<InputId>,
    },
    LevelStartHypothetical {
        level_index: usize,
        pre_start_state: State,
        inputs: Vec<InputId>,
    },
}

#[derive(Clone, Copy)]
enum HypotheticalOrigin {
    Imported,
    Derived,
}

#[derive(Clone, Copy)]
struct StatePatchDiagnostics {
    label: &'static str,
    empty_code: &'static str,
    invalid_code: &'static str,
    duplicate_object_code: &'static str,
    duplicate_position_code: &'static str,
    layer_conflict_code: &'static str,
    persistent_variable_code: &'static str,
    duplicate_variable_code: &'static str,
}

const DERIVED_STATE_PATCH_DIAGNOSTICS: StatePatchDiagnostics = StatePatchDiagnostics {
    label: "derived state",
    empty_code: "empty_derived_state_patch",
    invalid_code: "invalid_derived_state",
    duplicate_object_code: "duplicate_derived_state_object",
    duplicate_position_code: "duplicate_derived_state_position",
    layer_conflict_code: "derived_state_layer_conflict",
    persistent_variable_code: "derived_state_persistent_variable_change",
    duplicate_variable_code: "duplicate_derived_state_variable",
};

const LEVEL_START_STATE_PATCH_DIAGNOSTICS: StatePatchDiagnostics = StatePatchDiagnostics {
    label: "level start state",
    empty_code: "empty_level_start_state_patch",
    invalid_code: "invalid_level_start_state",
    duplicate_object_code: "duplicate_level_start_state_object",
    duplicate_position_code: "duplicate_level_start_state_position",
    layer_conflict_code: "level_start_state_layer_conflict",
    persistent_variable_code: "level_start_state_persistent_variable_change",
    duplicate_variable_code: "duplicate_level_start_state_variable",
};

#[derive(Clone, Copy)]
struct StateAssertionDiagnostics {
    label: &'static str,
    invalid_code: &'static str,
    failed_code: &'static str,
}

const DERIVED_STATE_ASSERTION_DIAGNOSTICS: StateAssertionDiagnostics = StateAssertionDiagnostics {
    label: "derived state",
    invalid_code: "invalid_derived_state_assertion",
    failed_code: "derived_state_assertion_failed",
};

const LEVEL_START_STATE_ASSERTION_DIAGNOSTICS: StateAssertionDiagnostics =
    StateAssertionDiagnostics {
        label: "level start state",
        invalid_code: "invalid_level_start_state_assertion",
        failed_code: "level_start_state_assertion_failed",
    };

struct RunRecord {
    from_state_id: String,
    terminal_state_id: String,
    inputs: Vec<String>,
    points: Vec<RunPoint>,
}

#[derive(Clone)]
struct SemanticGoal {
    base_state_id: String,
    level_index: usize,
    width: u16,
    height: u16,
    cells: Vec<SemanticGoalCell>,
}

#[derive(Clone)]
enum SemanticGoalCell {
    Exact(Vec<ObjectId>),
    Contains(Vec<ObjectId>),
    Excludes(Vec<ObjectId>),
    Unknown,
}

struct SearchSession {
    from_state_id: String,
    goal_id: String,
    goal: SemanticGoal,
    algorithm: SemanticGoalSearchAlgorithm,
    limits: SearchSessionLimits,
    state_slicer: puzzle_solver::SolverStateSlicer,
    machine: ResumableBestFirst<AgentSearchState, InputId, AgentStateKey>,
    advanced: bool,
    pause_reason: Option<&'static str>,
    failure: Option<Value>,
}

#[derive(Clone)]
struct RunPoint {
    index: usize,
    state: State,
    trace: Option<TransitionTrace>,
    goal: bool,
    lose: bool,
}

impl InvestigationSession {
    fn new(
        entry_path: String,
        source_hash: String,
        model_name: String,
        game: LoadedGame,
    ) -> Result<Self, InvestigationError> {
        let mut inputs_by_name = BTreeMap::new();
        for (id, label) in &game.input_labels {
            if inputs_by_name.insert(label.clone(), *id).is_some() {
                return Err(InvestigationError::new(
                    "compile_failed",
                    format!("duplicate model input label {label:?}"),
                ));
            }
        }
        let mut session = Self {
            entry_path,
            source_hash,
            model_name,
            game,
            inputs_by_name,
            next_state: 0,
            next_run: 0,
            next_goal: 0,
            next_search: 0,
            states: HashMap::new(),
            initial_state_ids: Vec::new(),
            runs: HashMap::new(),
            goals: HashMap::new(),
            searches: HashMap::new(),
        };
        for level_index in 0..session.game.levels.len() {
            let play = session.play_session_for_level(level_index)?;
            let state_id = session.store_state(StateRecord {
                level_index,
                state: play.state().clone(),
                replay: StateReplay::Reachable {
                    start_level_index: level_index,
                    inputs: Vec::new(),
                },
            });
            session.initial_state_ids.push(state_id);
        }
        Ok(session)
    }

    fn play_session_for_level(
        &self,
        level_index: usize,
    ) -> Result<GameSession, InvestigationError> {
        if level_index >= self.game.levels.len() {
            return Err(InvestigationError::new(
                "unknown_level",
                format!("level index {level_index} is out of range"),
            ));
        }
        let mut play = GameSession::new_headless_before_level_start(&self.game);
        play.start_level(&self.game, level_index).map_err(|error| {
            InvestigationError::new(
                "level_start_failed",
                format!("failed to start level {level_index}: {error:?}"),
            )
        })?;
        Ok(play)
    }

    fn replay_state(&self, record: &StateRecord) -> Result<GameSession, InvestigationError> {
        let (mut play, inputs) = match &record.replay {
            StateReplay::Reachable {
                start_level_index,
                inputs,
            } => (self.play_session_for_level(*start_level_index)?, inputs),
            StateReplay::Hypothetical {
                base_state_id,
                root_level_index: _,
                root_state,
                inputs,
                ..
            } => {
                let base = self.states.get(base_state_id).ok_or_else(|| {
                    InvestigationError::new(
                        "unknown_state",
                        format!("hypothetical base state {base_state_id:?} does not exist"),
                    )
                })?;
                let mut play = self.replay_state(base)?;
                play.replace_active_state_snapshot(&self.game, root_state.clone())
                    .map_err(|error| {
                        InvestigationError::new(
                            "transition_failed",
                            format!("failed to materialize hypothetical state: {error:?}"),
                        )
                    })?;
                if play.state() != root_state {
                    return Err(InvestigationError::new(
                        "hypothetical_state_changed",
                        "play lifecycle changed a hypothetical state before its first input",
                    ));
                }
                (play, inputs)
            }
            StateReplay::LevelStartHypothetical {
                level_index,
                pre_start_state,
                inputs,
            } => {
                let mut play = GameSession::new_headless_before_level_start(&self.game);
                play.start_level_from_state(
                    &self.game,
                    *level_index,
                    pre_start_state.clone(),
                    true,
                )
                .map_err(|error| {
                    InvestigationError::new(
                        "transition_failed",
                        format!("failed to start hypothetical level state: {error:?}"),
                    )
                })?;
                (play, inputs)
            }
        };
        for input in inputs {
            apply_semantic_input_with_trace(&mut play, &self.game, *input).map_err(|error| {
                InvestigationError::new("transition_failed", format!("replay failed: {error:?}"))
            })?;
        }
        if play.state() != &record.state {
            return Err(InvestigationError::new(
                "replay_mismatch",
                "stored state does not match authoritative input replay",
            )
            .with_details(json!({
                "storedHash": state_hash(&record.state),
                "replayedHash": state_hash(play.state()),
            })));
        }
        Ok(play)
    }

    fn validate_hypothetical_root(
        &self,
        base_state_id: &str,
        state: &State,
    ) -> Result<(), InvestigationError> {
        let base = self.states.get(base_state_id).ok_or_else(|| {
            InvestigationError::new(
                "unknown_state",
                format!("hypothetical base state {base_state_id:?} does not exist"),
            )
        })?;
        let mut play = self.replay_state(base)?;
        play.replace_active_state_snapshot(&self.game, state.clone())
            .map_err(|error| {
                InvestigationError::new(
                    "transition_failed",
                    format!("failed to materialize hypothetical state: {error:?}"),
                )
            })?;
        if play.state() != state {
            return Err(InvestigationError::new(
                "hypothetical_state_changed",
                "play lifecycle changed a hypothetical state during materialization",
            )
            .with_details(json!({
                "declaredHash": state_hash(state),
                "materializedHash": state_hash(play.state()),
            })));
        }
        Ok(())
    }

    fn apply_state_patch(
        &self,
        base: &State,
        set_object_positions: &[SemanticObjectPositions],
        set_variables: &[SemanticVariableValue],
        diagnostics: StatePatchDiagnostics,
    ) -> Result<State, InvestigationError> {
        if set_object_positions.is_empty() && set_variables.is_empty() {
            return Err(InvestigationError::new(
                diagnostics.empty_code,
                format!(
                    "{} patch requires at least one object-position or variable replacement",
                    diagnostics.label
                ),
            ));
        }

        let mut replacements = Vec::with_capacity(set_object_positions.len());
        let mut replaced_objects = BTreeSet::new();
        for replacement in set_object_positions {
            let objects = resolve_semantic_objects(
                &self.game,
                std::slice::from_ref(&replacement.object),
                &format!("{} object position", diagnostics.label),
                true,
                diagnostics.invalid_code,
            )?;
            let object = objects[0];
            if !replaced_objects.insert(object) {
                return Err(InvestigationError::new(
                    diagnostics.duplicate_object_code,
                    format!(
                        "{} specifies object {:?} more than once",
                        diagnostics.label, replacement.object
                    ),
                ));
            }
            let mut positions = BTreeSet::new();
            for position in &replacement.positions {
                validate_semantic_position(
                    base,
                    *position,
                    &format!("{} object", diagnostics.label),
                )?;
                if !positions.insert(*position) {
                    return Err(InvestigationError::new(
                        diagnostics.duplicate_position_code,
                        format!(
                            "{} repeats position ({},{}) for object {:?}",
                            diagnostics.label, position[0], position[1], replacement.object
                        ),
                    ));
                }
            }
            replacements.push((object, replacement.object.as_str(), positions));
        }

        let mut state = base.clone();
        for (object, name, _) in &replacements {
            let occupied_slots = state.object_positions(*object).to_vec();
            for slot in occupied_slots {
                let position = state.slot_coord(slot).ok_or_else(|| {
                    InvestigationError::new(
                        diagnostics.invalid_code,
                        format!(
                            "{} could not locate existing object {name:?}",
                            diagnostics.label
                        ),
                    )
                })?;
                state
                    .remove_object_at(&self.game.game, position, *object)
                    .map_err(|error| {
                        InvestigationError::new(
                            diagnostics.invalid_code,
                            format!(
                                "{} could not remove existing object {name:?}: {error:?}",
                                diagnostics.label
                            ),
                        )
                    })?;
            }
        }
        for (object, name, positions) in &replacements {
            for [x, y] in positions {
                state
                    .place_object_at(&self.game.game, GridCoord::new([*x, *y]), *object)
                    .map_err(|error| {
                        InvestigationError::new(
                            diagnostics.layer_conflict_code,
                            format!(
                                "{} could not place object {name:?} at ({x},{y}): {error:?}",
                                diagnostics.label
                            ),
                        )
                        .with_details(json!({
                            "object": name,
                            "position": [x, y],
                            "error": format!("{error:?}"),
                        }))
                    })?;
            }
        }

        let variable_by_name = self
            .game
            .variable_labels
            .iter()
            .map(|(id, name)| (name.as_str(), *id))
            .collect::<HashMap<_, _>>();
        let mut replaced_variables = BTreeSet::new();
        for replacement in set_variables {
            let name = &replacement.variable;
            let value = replacement.value;
            let variable = variable_by_name
                .get(name.as_str())
                .copied()
                .ok_or_else(|| {
                    InvestigationError::new(
                        "unknown_variable",
                        format!("{} names unknown variable {name:?}", diagnostics.label),
                    )
                })?;
            if !replaced_variables.insert(variable) {
                return Err(InvestigationError::new(
                    diagnostics.duplicate_variable_code,
                    format!(
                        "{} specifies variable {:?} more than once",
                        diagnostics.label, replacement.variable
                    ),
                ));
            }
            let index = usize::from(variable.0);
            let current = state
                .visible_variables()
                .get(index)
                .copied()
                .ok_or_else(|| {
                    InvestigationError::new(
                        diagnostics.invalid_code,
                        format!(
                            "{} variable {name:?} has no visible state slot",
                            diagnostics.label
                        ),
                    )
                })?;
            if self.game.persistent_vars.contains(&variable) && current != value {
                return Err(InvestigationError::new(
                    diagnostics.persistent_variable_code,
                    format!(
                        "{} cannot change persistent variable {name:?}; preserve its authored value",
                        diagnostics.label
                    ),
                ));
            }
            state
                .set_visible_variable(variable, value)
                .map_err(|error| {
                    InvestigationError::new(
                        diagnostics.invalid_code,
                        format!(
                            "{} could not set variable {name:?}: {error:?}",
                            diagnostics.label
                        ),
                    )
                })?;
        }
        Ok(state)
    }

    fn store_state(&mut self, record: StateRecord) -> String {
        self.next_state += 1;
        let id = format!("state-{}", self.next_state);
        self.states.insert(id.clone(), record);
        id
    }

    fn manifest(&self, session_id: &str) -> Value {
        let mut inputs = self
            .game
            .input_labels
            .iter()
            .map(|(id, name)| json!({ "id": id.0, "name": name }))
            .collect::<Vec<_>>();
        inputs.sort_by_key(|value| value["id"].as_u64().unwrap_or(0));

        let mut objects = self
            .game
            .object_labels
            .iter()
            .map(|(id, name)| {
                json!({
                    "id": id.0,
                    "name": name,
                    "layer": self.game.game.object_layer(*id).map(|layer| layer.0),
                })
            })
            .collect::<Vec<_>>();
        objects.sort_by_key(|value| value["id"].as_u64().unwrap_or(0));

        let mut variables = self
            .game
            .variable_labels
            .iter()
            .map(|(id, name)| json!({ "id": id.0, "name": name }))
            .collect::<Vec<_>>();
        variables.sort_by_key(|value| value["id"].as_u64().unwrap_or(0));

        let levels = self
            .game
            .levels
            .iter()
            .enumerate()
            .map(|(index, level)| {
                let state_id = &self.initial_state_ids[index];
                let state = &self.states[state_id].state;
                json!({
                    "index": index,
                    "name": level.name,
                    "pack": level.pack,
                    "puzzle": level.puzzle,
                    "width": state.width,
                    "height": state.height,
                    "initialStateId": state_id,
                    "initialStateHash": state_hash(state),
                })
            })
            .collect::<Vec<_>>();

        let mut rules = self
            .game
            .rule_debug_info
            .iter()
            .map(|(id, info)| {
                json!({
                    "id": id.0,
                    "sourceLine": info.source_line,
                    "sourceLineNumber": info.source_line_number,
                    "routineStack": info.routine_stack,
                })
            })
            .collect::<Vec<_>>();
        rules.sort_by_key(|value| value["id"].as_u64().unwrap_or(0));

        let mut query_names = self.game.queries.keys().cloned().collect::<Vec<_>>();
        query_names.sort();
        json!({
            "sessionId": session_id,
            "entryPath": self.entry_path,
            "sourceHash": self.source_hash,
            "model": self.model_name,
            "modelKind": "puzzle2d",
            "inputs": inputs,
            "objects": objects,
            "variables": variables,
            "queries": query_names,
            "goal": self.game.goal,
            "lose": self.game.lose,
            "solverStrategy": self.game.solver_strategy,
            "levels": levels,
            "rules": rules,
        })
    }

    fn initial_state_summaries(&self) -> Vec<Value> {
        self.initial_state_ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                json!({
                    "levelIndex": index,
                    "levelName": self.game.levels[index].name,
                    "stateId": id,
                    "stateHash": state_hash(&self.states[id].state),
                })
            })
            .collect()
    }

    fn run(
        &mut self,
        session_id: &str,
        from_state_id: &str,
        input_names: &[String],
        observation: &ObservationRequest,
    ) -> Result<Value, InvestigationError> {
        self.run_verified(
            session_id,
            from_state_id,
            input_names,
            observation,
            None,
            None,
        )
    }

    fn run_verified(
        &mut self,
        session_id: &str,
        from_state_id: &str,
        input_names: &[String],
        observation: &ObservationRequest,
        semantic_goal: Option<&SemanticGoal>,
        expected_terminal: Option<(&State, &puzzle_solver::SolverStateSlicer)>,
    ) -> Result<Value, InvestigationError> {
        let source = self.states.get(from_state_id).cloned().ok_or_else(|| {
            InvestigationError::new(
                "unknown_state",
                format!("agent state {from_state_id:?} does not exist"),
            )
        })?;
        let inputs = input_names
            .iter()
            .map(|name| {
                self.inputs_by_name.get(name).copied().ok_or_else(|| {
                    InvestigationError::new(
                        "unknown_input",
                        format!("unknown model input {name:?}"),
                    )
                    .with_details(json!({
                        "availableInputs": self.inputs_by_name.keys().collect::<Vec<_>>()
                    }))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut play = self.replay_state(&source)?;
        let initial_state = play.state().clone();
        let mut points = vec![RunPoint {
            index: 0,
            state: initial_state.clone(),
            trace: None,
            goal: self.game.is_goal_complete(&initial_state),
            lose: self.game.is_lose_complete(&initial_state),
        }];
        let mut executed = Vec::new();
        let mut won = points[0].goal;
        let mut lost = points[0].lose;
        let mut semantic_goal_reached =
            semantic_goal.is_some_and(|goal| semantic_goal_matches(goal, &initial_state));
        for (offset, input) in inputs.iter().enumerate() {
            if semantic_goal_reached {
                break;
            }
            apply_semantic_input_with_trace(&mut play, &self.game, *input).map_err(|error| {
                InvestigationError::new(
                    "transition_failed",
                    format!("input {} failed: {error:?}", input_names[offset]),
                )
                .with_details(json!({ "inputIndex": offset, "input": input_names[offset] }))
            })?;
            executed.push(*input);
            let terminal_state = play.state().clone();
            let completion = play.last_level_completion().cloned();
            let state = completion.as_ref().map_or_else(
                || terminal_state.clone(),
                |completion| completion.state().clone(),
            );
            let trace = play.last_transition_trace().cloned();
            let goal = self.game.is_goal_complete(&state);
            let lose = self.game.is_lose_complete(&state);
            won = completion.is_some()
                || goal
                || trace.as_ref().is_some_and(|debug| {
                    debug
                        .commands
                        .iter()
                        .any(|command| matches!(command, TransitionCommand::Win))
                });
            lost = lose;
            semantic_goal_reached =
                semantic_goal.is_some_and(|goal| semantic_goal_matches(goal, &state));
            points.push(RunPoint {
                index: offset + 1,
                state,
                trace,
                goal,
                lose,
            });
            if semantic_goal.is_some() {
                if semantic_goal_reached {
                    break;
                }
            } else if expected_terminal.is_none() && (won || lost) {
                break;
            }
        }

        if semantic_goal.is_some() && !semantic_goal_reached {
            return Err(InvestigationError::new(
                "semantic_goal_search_replay_mismatch",
                "solver witness did not reach the semantic goal when replayed through the authoritative play lifecycle",
            )
            .with_details(json!({
                "requestedInputs": input_names.len(),
                "executedInputs": executed.len(),
            })));
        }

        let terminal = play.state().clone();
        if let Some((expected, state_slicer)) = expected_terminal {
            let reconstructed_logical = state_slicer.project_state(&terminal);
            if expected != &reconstructed_logical {
                return Err(InvestigationError::new(
                    "search_candidate_replay_mismatch",
                    "authoritative replay did not reconstruct the solver's logical candidate",
                )
                .with_details(json!({
                    "expectedLogicalHash": state_hash(expected),
                    "actualLogicalHash": state_hash(&reconstructed_logical),
                    "requestedInputs": input_names.len(),
                    "executedInputs": executed.len(),
                })));
            }
        }
        let replay = match source.replay {
            StateReplay::Reachable {
                start_level_index,
                mut inputs,
            } => {
                inputs.extend(executed.iter().copied());
                StateReplay::Reachable {
                    start_level_index,
                    inputs,
                }
            }
            StateReplay::Hypothetical {
                origin,
                base_state_id,
                root_level_index,
                root_state,
                mut inputs,
            } => {
                inputs.extend(executed.iter().copied());
                StateReplay::Hypothetical {
                    origin,
                    base_state_id,
                    root_level_index,
                    root_state,
                    inputs,
                }
            }
            StateReplay::LevelStartHypothetical {
                level_index,
                pre_start_state,
                mut inputs,
            } => {
                inputs.extend(executed.iter().copied());
                StateReplay::LevelStartHypothetical {
                    level_index,
                    pre_start_state,
                    inputs,
                }
            }
        };
        let terminal_state_id = self.store_state(StateRecord {
            level_index: play.active_level_index().unwrap_or(source.level_index),
            state: terminal.clone(),
            replay,
        });
        self.next_run += 1;
        let run_id = format!("run-{}", self.next_run);
        let executed_names = input_names[..executed.len()].to_vec();
        let response_points = observed_points(&self.game, &points, observation);
        let events = run_events(&self.game, &points);
        let summary = run_summary(&self.game, &initial_state, &terminal, &points);
        self.runs.insert(
            run_id.clone(),
            RunRecord {
                from_state_id: from_state_id.to_string(),
                terminal_state_id: terminal_state_id.clone(),
                inputs: executed_names.clone(),
                points,
            },
        );
        Ok(json!({
            "sessionId": session_id,
            "runId": run_id,
            "fromStateId": from_state_id,
            "terminalStateId": terminal_state_id,
            "requestedInputs": input_names.len(),
            "executedInputs": executed_names.len(),
            "inputs": executed_names,
            "result": if semantic_goal_reached { "semantic_goal_reached" } else if won { "solved" } else if lost { "lost" } else { "incomplete" },
            "initialHash": state_hash(&initial_state),
            "terminalHash": state_hash(&terminal),
            "summary": summary,
            "events": if observation.mode == ObservationMode::Summary { Value::Null } else { Value::Array(events) },
            "observations": response_points,
        }))
    }

    fn inspect_state(&self, state_id: &str) -> Result<Value, InvestigationError> {
        let record = self.states.get(state_id).ok_or_else(|| {
            InvestigationError::new(
                "unknown_state",
                format!("agent state {state_id:?} does not exist"),
            )
        })?;
        Ok(json!({
            "stateId": state_id,
            "levelIndex": record.level_index,
            "provenance": state_provenance(&record.replay),
            "state": describe_state(&self.game, &record.state),
        }))
    }

    fn export_semantic_state(&self, state_id: &str) -> Result<Value, InvestigationError> {
        let record = self.states.get(state_id).ok_or_else(|| {
            InvestigationError::new(
                "unknown_state",
                format!("agent state {state_id:?} does not exist"),
            )
        })?;
        let artifact = semantic_artifact_for_state(&self.game, record, state_id)?;
        serde_json::to_value(artifact).map_err(|error| {
            InvestigationError::new(
                "semantic_state_encode_failed",
                format!("failed to encode semantic state: {error}"),
            )
        })
    }

    fn import_semantic_state(
        &mut self,
        artifact: &SemanticStateArtifact,
    ) -> Result<Value, InvestigationError> {
        validate_semantic_artifact_header(artifact)?;
        let base = validate_semantic_base(
            &self.game,
            &self.states,
            &artifact.base_state_id,
            &artifact.base_state_hash,
            artifact.level_index,
            &artifact.level_name,
            artifact.width,
            artifact.height,
            &artifact.lines,
            "semantic state",
        )?;

        let empty = semantic_empty_char(&artifact.empty)?;
        let mut char_objects = HashMap::<char, Vec<ObjectId>>::new();
        for (raw_char, meaning) in &artifact.legend {
            let ch = single_semantic_char(raw_char, "legend key")?;
            if ch == empty {
                return Err(InvestigationError::new(
                    "invalid_semantic_state",
                    "semantic state empty character must not have a legend entry",
                ));
            }
            if ch == '?' {
                return Err(InvestigationError::new(
                    "semantic_state_unknown_reserved",
                    "`?` is reserved for an explicitly declared non-binding unknown semantic goal cell",
                ));
            }
            let names = match meaning {
                SemanticLegendMeaning::Exact { objects } => objects,
                SemanticLegendMeaning::Contains { .. }
                | SemanticLegendMeaning::Excludes { .. }
                | SemanticLegendMeaning::Unknown => {
                    return Err(InvestigationError::new(
                        "semantic_state_predicate_not_allowed",
                        "complete semantic states accept exact legend meanings only",
                    ));
                }
            };
            let objects = resolve_semantic_objects(
                &self.game,
                names,
                "semantic state",
                true,
                "invalid_semantic_legend",
            )?;
            char_objects.insert(ch, objects);
        }

        let variable_by_name = self
            .game
            .variable_labels
            .iter()
            .map(|(id, name)| (name.as_str(), *id))
            .collect::<HashMap<_, _>>();
        let expected_variables = variable_by_name.keys().copied().collect::<BTreeSet<_>>();
        let actual_variables = artifact
            .variables
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if expected_variables != actual_variables {
            return Err(InvestigationError::new(
                "semantic_state_variable_mismatch",
                "semantic state variables must exactly match the compiled variable names",
            )
            .with_details(json!({
                "expected": expected_variables,
                "actual": actual_variables,
            })));
        }
        let mut variable_values = base.state.visible_variables().to_vec();
        for (name, value) in &artifact.variables {
            let id = variable_by_name[name.as_str()];
            let index = usize::from(id.0);
            let Some(slot) = variable_values.get_mut(index) else {
                return Err(InvestigationError::new(
                    "invalid_semantic_state",
                    format!("variable {name:?} has no visible state slot"),
                ));
            };
            *slot = *value;
        }
        let (mut state, _) = puzzle_lang::parse_level_ascii_state(
            &self.game.game,
            &artifact.lines,
            empty,
            &char_objects,
            &variable_values,
        )
        .map_err(|error| InvestigationError::new("invalid_semantic_state", error.to_string()))?;
        for rule in base.state.level_fired_rules() {
            state.mark_level_rule_fired(*rule);
        }
        for variable in &self.game.persistent_vars {
            let index = usize::from(variable.0);
            if base.state.visible_variables().get(index) != state.visible_variables().get(index) {
                let name = self
                    .game
                    .variable_labels
                    .get(variable)
                    .cloned()
                    .unwrap_or_else(|| format!("variable:{index}"));
                return Err(InvestigationError::new(
                    "semantic_state_persistent_variable_change",
                    format!(
                        "semantic state cannot change persistent variable {name:?}; preserve its base value"
                    ),
                ));
            }
        }

        let diff = semantic_state_diff(&self.game, &base.state, &state);
        self.validate_hypothetical_root(&artifact.base_state_id, &state)?;
        let state_id = self.store_state(StateRecord {
            level_index: base.level_index,
            state: state.clone(),
            replay: StateReplay::Hypothetical {
                origin: HypotheticalOrigin::Imported,
                base_state_id: artifact.base_state_id.clone(),
                root_level_index: base.level_index,
                root_state: state,
                inputs: Vec::new(),
            },
        });
        Ok(json!({
            "stateId": state_id,
            "stateHash": state_hash(&self.states[&state_id].state),
            "provenance": state_provenance(&self.states[&state_id].replay),
            "diff": diff,
        }))
    }

    fn derive_state(
        &mut self,
        base_state_id: &str,
        expected_base_hash: Option<&str>,
        set_object_positions: &[SemanticObjectPositions],
        set_variables: &[SemanticVariableValue],
        assertions: &[SemanticStateAssertion],
    ) -> Result<Value, InvestigationError> {
        let base = self.states.get(base_state_id).cloned().ok_or_else(|| {
            InvestigationError::new(
                "unknown_state",
                format!("agent state {base_state_id:?} does not exist"),
            )
        })?;
        let actual_base_hash = state_hash(&base.state);
        if expected_base_hash.is_some_and(|expected| expected != actual_base_hash) {
            return Err(InvestigationError::new(
                "derived_state_base_mismatch",
                "derived state expectedBaseHash does not match its immutable base state",
            )
            .with_details(json!({
                "baseStateId": base_state_id,
                "expectedBaseHash": expected_base_hash,
                "actualBaseHash": actual_base_hash,
            })));
        }
        let state = self.apply_state_patch(
            &base.state,
            set_object_positions,
            set_variables,
            DERIVED_STATE_PATCH_DIAGNOSTICS,
        )?;

        validate_semantic_state_assertions(
            &self.game,
            &state,
            assertions,
            DERIVED_STATE_ASSERTION_DIAGNOSTICS,
        )?;
        self.validate_hypothetical_root(base_state_id, &state)?;

        let diff = semantic_state_diff(&self.game, &base.state, &state);
        let level_name = self.game.levels[base.level_index].name.clone();
        let state_id = self.store_state(StateRecord {
            level_index: base.level_index,
            state: state.clone(),
            replay: StateReplay::Hypothetical {
                origin: HypotheticalOrigin::Derived,
                base_state_id: base_state_id.to_string(),
                root_level_index: base.level_index,
                root_state: state.clone(),
                inputs: Vec::new(),
            },
        });
        Ok(json!({
            "baseStateId": base_state_id,
            "stateId": state_id,
            "stateHash": state_hash(&state),
            "levelIndex": base.level_index,
            "levelName": level_name,
            "provenance": state_provenance(&self.states[&state_id].replay),
            "applied": {
                "setObjectPositions": set_object_positions,
                "setVariables": set_variables,
            },
            "diff": diff,
            "state": describe_state(&self.game, &state),
        }))
    }

    fn start_level_from_state(
        &mut self,
        level_index: usize,
        expected_level_name: Option<&str>,
        set_object_positions: &[SemanticObjectPositions],
        set_variables: &[SemanticVariableValue],
        assertions: &[SemanticStateAssertion],
    ) -> Result<Value, InvestigationError> {
        let level = self.game.levels.get(level_index).ok_or_else(|| {
            InvestigationError::new(
                "unknown_level",
                format!("level index {level_index} is out of range"),
            )
        })?;
        if expected_level_name.is_some_and(|expected| expected != level.name) {
            return Err(InvestigationError::new(
                "level_start_state_level_mismatch",
                "start_level_from_state expectedLevelName does not match levelIndex",
            )
            .with_details(json!({
                "levelIndex": level_index,
                "expectedLevelName": expected_level_name,
                "actualLevelName": level.name,
            })));
        }
        let level_name = level.name.clone();
        let authored_state = level.initial_state.clone();
        let pre_start_state = self.apply_state_patch(
            &authored_state,
            set_object_positions,
            set_variables,
            LEVEL_START_STATE_PATCH_DIAGNOSTICS,
        )?;
        let pre_start_diff = semantic_state_diff(&self.game, &authored_state, &pre_start_state);

        let mut play = GameSession::new_headless_before_level_start(&self.game);
        play.start_level_from_state(&self.game, level_index, pre_start_state.clone(), true)
            .map_err(|error| {
                InvestigationError::new(
                    "level_start_state_failed",
                    format!("failed to start declared level state: {error:?}"),
                )
            })?;
        let state = play.state().clone();
        validate_semantic_state_assertions(
            &self.game,
            &state,
            assertions,
            LEVEL_START_STATE_ASSERTION_DIAGNOSTICS,
        )?;

        let normal_play = self.play_session_for_level(level_index)?;
        let normal_state = normal_play.state();
        let diff = semantic_state_diff(&self.game, normal_state, &state);
        let active_level_index = play.active_level_index().unwrap_or(level_index);
        let active_level_name = self
            .game
            .levels
            .get(active_level_index)
            .map(|active| active.name.clone())
            .unwrap_or_else(|| level_name.clone());
        let state_id = self.store_state(StateRecord {
            level_index: active_level_index,
            state: state.clone(),
            replay: StateReplay::LevelStartHypothetical {
                level_index,
                pre_start_state,
                inputs: Vec::new(),
            },
        });
        Ok(json!({
            "stateId": state_id,
            "stateHash": state_hash(&state),
            "levelIndex": active_level_index,
            "levelName": active_level_name,
            "startedFrom": {
                "levelIndex": level_index,
                "levelName": level_name,
                "authoredStateHash": state_hash(&authored_state),
            },
            "provenance": state_provenance(&self.states[&state_id].replay),
            "applied": {
                "setObjectPositions": set_object_positions,
                "setVariables": set_variables,
            },
            "preStartDiff": pre_start_diff,
            "diff": diff,
            "state": describe_state(&self.game, &state),
        }))
    }

    fn import_semantic_goal(
        &mut self,
        artifact: &SemanticGoalArtifact,
    ) -> Result<Value, InvestigationError> {
        validate_semantic_goal_header(artifact)?;
        let base = validate_semantic_base(
            &self.game,
            &self.states,
            &artifact.base_state_id,
            &artifact.base_state_hash,
            artifact.level_index,
            &artifact.level_name,
            artifact.width,
            artifact.height,
            &artifact.lines,
            "semantic goal",
        )?;
        let empty = semantic_empty_char(&artifact.empty)?;
        let mut meanings = HashMap::<char, SemanticGoalCell>::new();
        for (raw_char, meaning) in &artifact.legend {
            let ch = single_semantic_char(raw_char, "legend key")?;
            if ch == empty {
                return Err(InvestigationError::new(
                    "invalid_semantic_goal",
                    "semantic goal empty character must not have a legend entry",
                ));
            }
            let cell = match meaning {
                SemanticLegendMeaning::Exact { objects } => {
                    SemanticGoalCell::Exact(resolve_semantic_objects(
                        &self.game,
                        objects,
                        "semantic goal exact",
                        true,
                        "invalid_semantic_legend",
                    )?)
                }
                SemanticLegendMeaning::Contains { objects } => {
                    if objects.is_empty() {
                        return Err(InvestigationError::new(
                            "invalid_semantic_goal",
                            "semantic goal contains predicate requires at least one object",
                        ));
                    }
                    SemanticGoalCell::Contains(resolve_semantic_objects(
                        &self.game,
                        objects,
                        "semantic goal contains",
                        true,
                        "invalid_semantic_legend",
                    )?)
                }
                SemanticLegendMeaning::Excludes { objects } => {
                    if objects.is_empty() {
                        return Err(InvestigationError::new(
                            "invalid_semantic_goal",
                            "semantic goal excludes predicate requires at least one object",
                        ));
                    }
                    SemanticGoalCell::Excludes(resolve_semantic_objects(
                        &self.game,
                        objects,
                        "semantic goal excludes",
                        false,
                        "invalid_semantic_legend",
                    )?)
                }
                SemanticLegendMeaning::Unknown => SemanticGoalCell::Unknown,
            };
            meanings.insert(ch, cell);
        }
        let mut cells =
            Vec::with_capacity(usize::from(artifact.width) * usize::from(artifact.height));
        let mut unknown_cells = 0_usize;
        for (y, line) in artifact.lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                let cell = if ch == empty {
                    SemanticGoalCell::Exact(Vec::new())
                } else {
                    clone_goal_cell(meanings.get(&ch).ok_or_else(|| {
                        InvestigationError::new(
                            "invalid_semantic_goal",
                            format!("semantic goal uses undeclared character {ch:?} at ({x},{y})"),
                        )
                    })?)
                };
                if matches!(cell, SemanticGoalCell::Unknown) {
                    unknown_cells += 1;
                }
                cells.push(cell);
            }
        }
        self.next_goal += 1;
        let goal_id = format!("goal-{}", self.next_goal);
        self.goals.insert(
            goal_id.clone(),
            SemanticGoal {
                base_state_id: artifact.base_state_id.clone(),
                level_index: base.level_index,
                width: artifact.width,
                height: artifact.height,
                cells,
            },
        );
        Ok(json!({
            "goalId": goal_id,
            "kind": "puzzle2d-semantic-goal",
            "baseStateId": artifact.base_state_id,
            "unknownCells": unknown_cells,
            "bindingCount": 0,
        }))
    }

    fn evaluate_semantic_goal(
        &self,
        goal_id: &str,
        state_id: &str,
    ) -> Result<Value, InvestigationError> {
        let goal = self.goals.get(goal_id).ok_or_else(|| {
            InvestigationError::new(
                "unknown_goal",
                format!("semantic goal {goal_id:?} does not exist"),
            )
        })?;
        let record = self.states.get(state_id).ok_or_else(|| {
            InvestigationError::new(
                "unknown_state",
                format!("agent state {state_id:?} does not exist"),
            )
        })?;
        if record.level_index != goal.level_index
            || record.state.width != goal.width
            || record.state.height != goal.height
        {
            return Err(InvestigationError::new(
                "semantic_goal_state_mismatch",
                "semantic goal and state must have the same level and dimensions",
            ));
        }
        let (checked_cells, unknown_cells, mismatches) =
            semantic_goal_mismatches(&self.game, goal, &record.state);
        Ok(json!({
            "goalId": goal_id,
            "stateId": state_id,
            "baseStateId": goal.base_state_id,
            "matches": mismatches.is_empty(),
            "checkedCells": checked_cells,
            "unknownCells": unknown_cells,
            "bindingCount": 0,
            "mismatches": mismatches,
        }))
    }

    fn solve_semantic_goal(
        &mut self,
        session_id: &str,
        goal_id: &str,
        from_state_id: &str,
        algorithm: SemanticGoalSearchAlgorithm,
        budget: SemanticGoalSearchBudget,
    ) -> Result<Value, InvestigationError> {
        if budget.max_depth == 0 || budget.max_stored_nodes == 0 || budget.max_millis == 0 {
            return Err(InvestigationError::new(
                "invalid_search_budget",
                "maxDepth, maxStoredNodes, and maxMillis must all be greater than zero",
            ));
        }
        let goal = self.goals.get(goal_id).cloned().ok_or_else(|| {
            InvestigationError::new(
                "unknown_goal",
                format!("semantic goal {goal_id:?} does not exist"),
            )
        })?;
        let source = self.states.get(from_state_id).cloned().ok_or_else(|| {
            InvestigationError::new(
                "unknown_state",
                format!("agent state {from_state_id:?} does not exist"),
            )
        })?;
        if source.level_index != goal.level_index
            || source.state.width != goal.width
            || source.state.height != goal.height
        {
            return Err(InvestigationError::new(
                "semantic_goal_state_mismatch",
                "semantic goal and search source must have the same level and dimensions",
            ));
        }
        let (mut domain, _) = self.semantic_search_domain(&goal, &source.state)?;
        let initial = domain
            .initial_session(self.replay_state(&source)?)
            .map_err(|error| {
                InvestigationError::new(
                    "semantic_search_session_failed",
                    format!("failed to initialize authoritative search session: {error:?}"),
                )
            })?;
        let search_budget = SearchBudget::bounded(
            budget.max_depth,
            budget.max_stored_nodes,
            Duration::from_millis(budget.max_millis),
        );
        let outcome = match algorithm {
            SemanticGoalSearchAlgorithm::Bfs => exact_bfs(&mut domain, initial, search_budget),
            SemanticGoalSearchAlgorithm::BestFirst => {
                let score_goal = goal.clone();
                best_first(&mut domain, initial, search_budget, move |state| {
                    semantic_goal_mismatch_count(&score_goal, state.observation_state()) as i64
                })
            }
        };
        match outcome {
            SearchOutcome::Solved(witness) => {
                let input_names = witness
                    .actions
                    .iter()
                    .map(|input| {
                        self.game.input_labels.get(input).cloned().ok_or_else(|| {
                            InvestigationError::new(
                                "search_input_contract_mismatch",
                                format!("solver returned unnamed input id {}", input.0),
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let mut run = self.run_verified(
                    session_id,
                    from_state_id,
                    &input_names,
                    &ObservationRequest::default(),
                    Some(&goal),
                    None,
                )?;
                let object = run
                    .as_object_mut()
                    .expect("agent run response must be a JSON object");
                object.insert("searchOutcome".to_string(), json!("solved"));
                object.insert("goalId".to_string(), json!(goal_id));
                object.insert(
                    "algorithm".to_string(),
                    json!(search_algorithm_name(algorithm)),
                );
                object.insert("solutionDepth".to_string(), json!(witness.depth));
                Ok(run)
            }
            SearchOutcome::Exhausted(stats) => Ok(search_unsolved_response(
                goal_id,
                from_state_id,
                algorithm,
                "exhausted",
                stats,
            )),
            SearchOutcome::BudgetExceeded(stats) => Ok(search_unsolved_response(
                goal_id,
                from_state_id,
                algorithm,
                "budget_exceeded",
                stats,
            )),
            SearchOutcome::Failed(failure) => Err(InvestigationError::new(
                "semantic_goal_search_failed",
                format!(
                    "search transition failed at depth {} for input {}: {:?}",
                    failure.depth, failure.action.0, failure.error
                ),
            )),
        }
    }

    fn semantic_search_domain(
        &self,
        goal: &SemanticGoal,
        source_state: &State,
    ) -> Result<(AgentSearchDomain, puzzle_solver::SolverStateSlicer), InvestigationError> {
        let inputs = self
            .game
            .input_labels
            .iter()
            .filter_map(|(input, name)| {
                (!matches!(
                    name.as_str(),
                    "undo" | "restart" | "next_level" | "previous_level"
                ))
                .then_some(*input)
            })
            .collect::<Vec<_>>();
        if inputs.is_empty() {
            return Err(InvestigationError::new(
                "no_search_inputs",
                "compiled model has no inputs available for semantic goal search",
            ));
        }
        if goal.level_index >= self.game.levels.len() {
            return Err(InvestigationError::new(
                "search_level_out_of_range",
                format!("semantic goal level {} is out of range", goal.level_index),
            ));
        }
        let mut roots = semantic_goal_root_objects(&self.game, goal);
        for condition in self
            .game
            .conditions
            .values()
            .chain(self.game.goal.iter())
            .chain(self.game.lose.iter())
        {
            puzzle_solver::object_refs::collect_goal_expr_roots(
                &self.game.game,
                &condition.expr,
                &mut roots,
            );
        }
        let slice = puzzle_solver::SolverSlice::from_loaded_level_roots(
            &self.game,
            goal.level_index,
            [source_state],
            roots,
        )
        .ok_or_else(|| {
            InvestigationError::new(
                "search_level_out_of_range",
                format!("semantic goal level {} is out of range", goal.level_index),
            )
        })?;
        let state_slicer = puzzle_solver::SolverStateSlicer::from_kept_objects(
            &self.game.game,
            slice.kept_objects(),
        );
        let solver_game = Arc::new(slice.project_loaded_game(&self.game, &state_slicer));
        let goal_for_domain = goal.clone();
        let domain = AgentSearchDomain::with_state_slicer(
            solver_game,
            goal.level_index,
            inputs,
            state_slicer.clone(),
            move |state| semantic_goal_matches(&goal_for_domain, state),
        );
        Ok((domain, state_slicer))
    }

    fn create_search(
        &mut self,
        session_id: &str,
        goal_id: &str,
        from_state_id: &str,
        algorithm: SemanticGoalSearchAlgorithm,
        limits: SearchSessionLimits,
    ) -> Result<Value, InvestigationError> {
        if limits.max_depth == 0 || limits.max_stored_nodes == 0 {
            return Err(InvestigationError::new(
                "invalid_search_limits",
                "maxDepth and maxStoredNodes must both be greater than zero",
            ));
        }
        let goal = self.goals.get(goal_id).cloned().ok_or_else(|| {
            InvestigationError::new(
                "unknown_goal",
                format!("semantic goal {goal_id:?} does not exist"),
            )
        })?;
        let source = self.states.get(from_state_id).cloned().ok_or_else(|| {
            InvestigationError::new(
                "unknown_state",
                format!("agent state {from_state_id:?} does not exist"),
            )
        })?;
        validate_goal_source(&goal, &source, "search source")?;
        let (domain, state_slicer) = self.semantic_search_domain(&goal, &source.state)?;
        let initial = domain
            .initial_session(self.replay_state(&source)?)
            .map_err(|error| {
                InvestigationError::new(
                    "semantic_search_session_failed",
                    format!("failed to initialize authoritative search session: {error:?}"),
                )
            })?;
        let initial_key = domain.key(&initial);
        let initial_score = search_session_score(algorithm, &goal, &initial);
        let machine = ResumableBestFirst::new(
            initial,
            initial_key,
            initial_score,
            ResumableSearchLimits {
                max_depth: limits.max_depth,
                max_stored_nodes: limits.max_stored_nodes,
            },
        );
        self.next_search += 1;
        let search_id = format!("search-{}", self.next_search);
        self.searches.insert(
            search_id.clone(),
            SearchSession {
                from_state_id: from_state_id.to_string(),
                goal_id: goal_id.to_string(),
                goal,
                algorithm,
                limits,
                state_slicer,
                machine,
                advanced: false,
                pause_reason: None,
                failure: None,
            },
        );
        Ok(json!({
            "sessionId": session_id,
            "searchId": search_id,
            "status": "ready",
            "fromStateId": from_state_id,
            "goalId": goal_id,
            "algorithm": search_algorithm_name(algorithm),
            "limits": {
                "maxDepth": limits.max_depth,
                "maxStoredNodes": limits.max_stored_nodes,
            }
        }))
    }

    fn advance_search(
        &mut self,
        session_id: &str,
        search_id: &str,
        allowance: SearchSessionAllowance,
    ) -> Result<Value, InvestigationError> {
        if allowance.max_expanded_nodes == 0 || allowance.max_millis == 0 {
            return Err(InvestigationError::new(
                "invalid_search_allowance",
                "maxExpandedNodes and maxMillis must both be greater than zero",
            ));
        }
        let (goal, algorithm, status, from_state_id) = {
            let search = self.searches.get(search_id).ok_or_else(|| {
                InvestigationError::new(
                    "unknown_search",
                    format!("search session {search_id:?} does not exist"),
                )
            })?;
            (
                search.goal.clone(),
                search.algorithm,
                search.machine.status(),
                search.from_state_id.clone(),
            )
        };
        if !matches!(status, ResumableSearchStatus::Active) {
            return Err(InvestigationError::new(
                "search_not_advanceable",
                format!(
                    "search session is terminal with status {}",
                    resumable_status_name(status)
                ),
            ));
        }
        let source_state = &self
            .states
            .get(&from_state_id)
            .ok_or_else(|| {
                InvestigationError::new(
                    "unknown_state",
                    format!("agent state {from_state_id:?} does not exist"),
                )
            })?
            .state;
        let (mut domain, _) = self.semantic_search_domain(&goal, source_state)?;
        let search = self
            .searches
            .get_mut(search_id)
            .expect("search existence was checked");
        let outcome = search.machine.advance(
            &mut domain,
            ResumableSearchAllowance {
                max_expanded_nodes: allowance.max_expanded_nodes,
                max_duration: Some(Duration::from_millis(allowance.max_millis)),
            },
            |state| search_session_score(algorithm, &goal, state),
        );
        search.advanced = true;
        let (status, pause_reason, solution_candidate_id) = match outcome {
            ResumableAdvanceOutcome::Paused { reason, .. } => (
                "paused",
                Some(match reason {
                    ResumablePauseReason::ExpandedNodes => "expanded_nodes",
                    ResumablePauseReason::Duration => "duration",
                }),
                None,
            ),
            ResumableAdvanceOutcome::Solved {
                candidate_index, ..
            } => ("solved", None, Some(candidate_id(candidate_index))),
            ResumableAdvanceOutcome::Exhausted { .. } => ("exhausted", None, None),
            ResumableAdvanceOutcome::ResourceLimit { .. } => ("resource_limit", None, None),
            ResumableAdvanceOutcome::Failed { failure, .. } => {
                let details = json!({
                    "depth": failure.depth,
                    "inputId": failure.action.0,
                    "error": format!("{:?}", failure.error),
                });
                search.failure = Some(details);
                ("failed", None, None)
            }
        };
        search.pause_reason = pause_reason;
        Ok(json!({
            "sessionId": session_id,
            "searchId": search_id,
            "status": status,
            "pauseReason": pause_reason,
            "solutionCandidateId": solution_candidate_id,
            "stats": search_stats_json(&search.machine.stats()),
        }))
    }

    fn inspect_search(
        &self,
        session_id: &str,
        search_id: &str,
        candidate_limit: usize,
    ) -> Result<Value, InvestigationError> {
        if candidate_limit == 0 || candidate_limit > 1000 {
            return Err(InvestigationError::new(
                "invalid_candidate_limit",
                "candidateLimit must be between 1 and 1000",
            ));
        }
        let search = self.searches.get(search_id).ok_or_else(|| {
            InvestigationError::new(
                "unknown_search",
                format!("search session {search_id:?} does not exist"),
            )
        })?;
        let candidates = search
            .machine
            .best_candidates(candidate_limit)
            .into_iter()
            .map(|candidate| search_candidate_json(&self.game, &search.goal, candidate))
            .collect::<Result<Vec<_>, _>>()?;
        let solution_candidate_id = match search.machine.status() {
            ResumableSearchStatus::Solved { candidate_index } => {
                Some(candidate_id(candidate_index))
            }
            _ => None,
        };
        Ok(json!({
            "sessionId": session_id,
            "searchId": search_id,
            "status": search_session_status(search),
            "fromStateId": search.from_state_id,
            "goalId": search.goal_id,
            "algorithm": search_algorithm_name(search.algorithm),
            "limits": {
                "maxDepth": search.limits.max_depth,
                "maxStoredNodes": search.limits.max_stored_nodes,
            },
            "stats": search_stats_json(&search.machine.stats()),
            "pauseReason": search.pause_reason,
            "solutionCandidateId": solution_candidate_id,
            "failure": search.failure,
            "candidates": candidates,
        }))
    }

    fn materialize_search_candidate(
        &mut self,
        session_id: &str,
        search_id: &str,
        raw_candidate_id: &str,
    ) -> Result<Value, InvestigationError> {
        let index = parse_candidate_id(raw_candidate_id)?;
        let (from_state_id, state_slicer, candidate) = {
            let search = self.searches.get(search_id).ok_or_else(|| {
                InvestigationError::new(
                    "unknown_search",
                    format!("search session {search_id:?} does not exist"),
                )
            })?;
            let candidate = search.machine.candidate(index).ok_or_else(|| {
                InvestigationError::new(
                    "unknown_candidate",
                    format!("candidate {raw_candidate_id:?} does not exist in {search_id:?}"),
                )
            })?;
            (
                search.from_state_id.clone(),
                search.state_slicer.clone(),
                candidate,
            )
        };
        let input_names = candidate
            .actions
            .iter()
            .map(|input| {
                self.game.input_labels.get(input).cloned().ok_or_else(|| {
                    InvestigationError::new(
                        "search_input_contract_mismatch",
                        format!("solver returned unnamed input id {}", input.0),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut run = self.run_verified(
            session_id,
            &from_state_id,
            &input_names,
            &ObservationRequest::default(),
            None,
            Some((candidate.state.state(), &state_slicer)),
        )?;
        let object = run
            .as_object_mut()
            .expect("agent run response must be a JSON object");
        object.insert("searchId".to_string(), json!(search_id));
        object.insert("candidateId".to_string(), json!(raw_candidate_id));
        object.insert("candidateScore".to_string(), json!(candidate.score));
        Ok(run)
    }

    fn close_search(
        &mut self,
        session_id: &str,
        search_id: &str,
    ) -> Result<Value, InvestigationError> {
        if self.searches.remove(search_id).is_none() {
            return Err(InvestigationError::new(
                "unknown_search",
                format!("search session {search_id:?} does not exist"),
            ));
        }
        Ok(json!({
            "sessionId": session_id,
            "searchId": search_id,
            "closed": true,
        }))
    }

    fn inspect_run(
        &self,
        run_id: &str,
        indices: &[usize],
        include_trace: bool,
    ) -> Result<Value, InvestigationError> {
        let run = self.runs.get(run_id).ok_or_else(|| {
            InvestigationError::new(
                "unknown_run",
                format!("agent run {run_id:?} does not exist"),
            )
        })?;
        let mut points = Vec::new();
        for index in indices {
            let point = run.points.get(*index).ok_or_else(|| {
                InvestigationError::new(
                    "invalid_run_index",
                    format!("run index {index} is out of range"),
                )
                .with_details(json!({ "maxIndex": run.points.len().saturating_sub(1) }))
            })?;
            points.push(describe_run_point(&self.game, point, include_trace));
        }
        Ok(json!({
            "runId": run_id,
            "fromStateId": run.from_state_id,
            "terminalStateId": run.terminal_state_id,
            "inputs": run.inputs,
            "maxIndex": run.points.len().saturating_sub(1),
            "points": points,
        }))
    }

    fn compare_states(&self, left: &str, right: &str) -> Result<Value, InvestigationError> {
        let left_record = self.states.get(left).ok_or_else(|| {
            InvestigationError::new(
                "unknown_state",
                format!("agent state {left:?} does not exist"),
            )
        })?;
        let right_record = self.states.get(right).ok_or_else(|| {
            InvestigationError::new(
                "unknown_state",
                format!("agent state {right:?} does not exist"),
            )
        })?;
        if left_record.level_index != right_record.level_index {
            return Err(InvestigationError::new(
                "incompatible_states",
                "states from different levels cannot be compared",
            ));
        }
        Ok(json!({
            "leftStateId": left,
            "rightStateId": right,
            "same": left_record.state == right_record.state,
            "leftHash": state_hash(&left_record.state),
            "rightHash": state_hash(&right_record.state),
            "changedObjects": changed_object_names(&self.game, &left_record.state, &right_record.state),
            "changedVariables": changed_variable_names(&self.game, &left_record.state, &right_record.state),
            "diff": semantic_state_diff(&self.game, &left_record.state, &right_record.state),
            "goal": {
                "left": self.game.is_goal_complete(&left_record.state),
                "right": self.game.is_goal_complete(&right_record.state),
            },
            "lose": {
                "left": self.game.is_lose_complete(&left_record.state),
                "right": self.game.is_lose_complete(&right_record.state),
            },
        }))
    }
}

fn apply_semantic_input_with_trace(
    play: &mut GameSession,
    game: &LoadedGame,
    input: InputId,
) -> Result<(), puzzle_core::TransitionError> {
    play.apply_headless_traced_input(game, input)
}

fn state_provenance(replay: &StateReplay) -> Value {
    match replay {
        StateReplay::Reachable { inputs, .. } => json!({
            "kind": "reachable",
            "inputCount": inputs.len(),
        }),
        StateReplay::Hypothetical {
            origin,
            base_state_id,
            inputs,
            ..
        } => match origin {
            HypotheticalOrigin::Imported => json!({
                "kind": "hypothetical",
                "origin": "imported",
                "baseStateId": base_state_id,
                "inputCountAfterImport": inputs.len(),
            }),
            HypotheticalOrigin::Derived => json!({
                "kind": "hypothetical",
                "origin": "derived",
                "baseStateId": base_state_id,
                "inputCountAfterDerivation": inputs.len(),
            }),
        },
        StateReplay::LevelStartHypothetical {
            level_index,
            inputs,
            ..
        } => json!({
            "kind": "hypothetical",
            "origin": "level_start",
            "startLevelIndex": level_index,
            "inputCountAfterStart": inputs.len(),
        }),
    }
}

fn semantic_artifact_for_state(
    game: &LoadedGame,
    record: &StateRecord,
    state_id: &str,
) -> Result<SemanticStateArtifact, InvestigationError> {
    let state = &record.state;
    let empty_candidate = game.legend.char_for_cell(&[]);
    let empty = assignable_semantic_char(empty_candidate)
        .then_some(empty_candidate)
        .unwrap_or('.');
    let mut unique = BTreeMap::<Vec<u16>, Vec<ObjectId>>::new();
    for y in 0..state.height {
        for x in 0..state.width {
            let objects = cell_objects(state, x, y);
            unique
                .entry(objects.iter().map(|object| object.0).collect())
                .or_insert(objects);
        }
    }

    let mut used = BTreeSet::from([empty]);
    let mut assigned = BTreeMap::<Vec<u16>, char>::new();
    for (key, objects) in &unique {
        if objects.is_empty() {
            assigned.insert(key.clone(), empty);
            continue;
        }
        let candidate = game.legend.char_for_cell(objects);
        if assignable_semantic_char(candidate) && !used.contains(&candidate) {
            used.insert(candidate);
            assigned.insert(key.clone(), candidate);
        }
    }
    for key in unique.keys() {
        if assigned.contains_key(key) {
            continue;
        }
        let ch = semantic_char_pool()
            .find(|candidate| !used.contains(candidate))
            .ok_or_else(|| {
                InvestigationError::new(
                    "semantic_legend_exhausted",
                    "state has more distinct cell meanings than the ASCII legend can encode",
                )
            })?;
        used.insert(ch);
        assigned.insert(key.clone(), ch);
    }

    let mut legend = BTreeMap::new();
    for (key, objects) in &unique {
        let ch = assigned[key];
        if objects.is_empty() {
            continue;
        }
        legend.insert(
            ch.to_string(),
            SemanticLegendMeaning::Exact {
                objects: objects
                    .iter()
                    .map(|object| game.object_name(*object).to_string())
                    .collect(),
            },
        );
    }
    let lines = (0..state.height)
        .map(|y| {
            (0..state.width)
                .map(|x| {
                    let key = cell_objects(state, x, y)
                        .iter()
                        .map(|object| object.0)
                        .collect::<Vec<_>>();
                    assigned[&key]
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let variables = named_variable_values(game, state)?;
    Ok(SemanticStateArtifact {
        version: 1,
        kind: "puzzle2d-semantic-state".to_string(),
        base_state_id: state_id.to_string(),
        base_state_hash: state_hash(state),
        level_index: record.level_index,
        level_name: game.levels[record.level_index].name.clone(),
        width: state.width,
        height: state.height,
        empty: empty.to_string(),
        legend,
        lines,
        variables,
    })
}

fn named_variable_values(
    game: &LoadedGame,
    state: &State,
) -> Result<BTreeMap<String, i64>, InvestigationError> {
    let mut variables = BTreeMap::new();
    for (index, value) in state.visible_variables().iter().enumerate() {
        let name = game
            .variable_labels
            .iter()
            .find_map(|(id, name)| (usize::from(id.0) == index).then_some(name.clone()))
            .ok_or_else(|| {
                InvestigationError::new(
                    "semantic_state_encode_failed",
                    format!("visible variable {index} has no compiled name"),
                )
            })?;
        variables.insert(name, *value);
    }
    Ok(variables)
}

fn validate_semantic_artifact_header(
    artifact: &SemanticStateArtifact,
) -> Result<(), InvestigationError> {
    if artifact.version != 1 {
        return Err(InvestigationError::new(
            "semantic_state_version_mismatch",
            format!(
                "unsupported semantic state version {}; expected 1",
                artifact.version
            ),
        ));
    }
    if artifact.kind != "puzzle2d-semantic-state" {
        return Err(InvestigationError::new(
            "invalid_semantic_state",
            format!("unsupported semantic state kind {:?}", artifact.kind),
        ));
    }
    Ok(())
}

fn validate_semantic_goal_header(
    artifact: &SemanticGoalArtifact,
) -> Result<(), InvestigationError> {
    if artifact.version != 1 {
        return Err(InvestigationError::new(
            "semantic_goal_version_mismatch",
            format!(
                "unsupported semantic goal version {}; expected 1",
                artifact.version
            ),
        ));
    }
    if artifact.kind != "puzzle2d-semantic-goal" {
        return Err(InvestigationError::new(
            "invalid_semantic_goal",
            format!("unsupported semantic goal kind {:?}", artifact.kind),
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_semantic_base(
    game: &LoadedGame,
    states: &HashMap<String, StateRecord>,
    base_state_id: &str,
    base_state_hash: &str,
    level_index: usize,
    level_name: &str,
    width: u16,
    height: u16,
    lines: &[String],
    label: &str,
) -> Result<StateRecord, InvestigationError> {
    let base = states.get(base_state_id).cloned().ok_or_else(|| {
        InvestigationError::new(
            "unknown_state",
            format!("{label} base {base_state_id:?} does not exist"),
        )
    })?;
    if base_state_hash != state_hash(&base.state) {
        return Err(InvestigationError::new(
            "semantic_state_base_mismatch",
            format!("{label} base hash does not match the current session state"),
        )
        .with_details(json!({
            "expected": state_hash(&base.state),
            "actual": base_state_hash,
        })));
    }
    let level = game.levels.get(base.level_index).ok_or_else(|| {
        InvestigationError::new(
            "unknown_level",
            format!("{label} base level is unavailable"),
        )
    })?;
    if level_index != base.level_index || level_name != level.name {
        return Err(InvestigationError::new(
            "semantic_state_level_mismatch",
            format!("{label} level identity does not match its base state"),
        ));
    }
    if width != base.state.width
        || height != base.state.height
        || lines.len() != usize::from(height)
        || lines
            .iter()
            .any(|line| line.chars().count() != usize::from(width))
    {
        return Err(InvestigationError::new(
            "semantic_state_shape_mismatch",
            format!("{label} ASCII dimensions do not match its base state"),
        ));
    }
    Ok(base)
}

fn resolve_semantic_objects(
    game: &LoadedGame,
    names: &[String],
    label: &str,
    require_distinct_layers: bool,
    structural_error_code: &str,
) -> Result<Vec<ObjectId>, InvestigationError> {
    let object_by_name = game
        .object_labels
        .iter()
        .map(|(id, name)| (name.as_str(), *id))
        .collect::<HashMap<_, _>>();
    let mut objects = Vec::new();
    let mut layers = BTreeSet::new();
    for name in names {
        let object = object_by_name.get(name.as_str()).copied().ok_or_else(|| {
            InvestigationError::new(
                "unknown_object",
                format!("{label} names unknown object {name:?}"),
            )
        })?;
        if objects.contains(&object) {
            return Err(InvestigationError::new(
                structural_error_code,
                format!("{label} repeats object {name:?}"),
            ));
        }
        let layer = game.game.object_layer(object).ok_or_else(|| {
            InvestigationError::new(
                structural_error_code,
                format!("{label} object {name:?} has no compiled layer"),
            )
        })?;
        if require_distinct_layers && !layers.insert(layer) {
            return Err(InvestigationError::new(
                structural_error_code,
                format!("{label} places multiple objects in layer {}", layer.0),
            ));
        }
        objects.push(object);
    }
    objects.sort_by_key(|object| game.game.object_layer(*object).map(|layer| layer.0));
    Ok(objects)
}

fn validate_semantic_position(
    state: &State,
    position: [u16; 2],
    label: &str,
) -> Result<(), InvestigationError> {
    if position[0] >= state.width || position[1] >= state.height {
        return Err(InvestigationError::new(
            "semantic_position_out_of_bounds",
            format!(
                "{label} position ({},{}) is outside {}x{} state",
                position[0], position[1], state.width, state.height
            ),
        )
        .with_details(json!({
            "position": position,
            "width": state.width,
            "height": state.height,
        })));
    }
    Ok(())
}

fn validate_semantic_state_assertions(
    game: &LoadedGame,
    state: &State,
    assertions: &[SemanticStateAssertion],
    diagnostics: StateAssertionDiagnostics,
) -> Result<(), InvestigationError> {
    for assertion in assertions {
        let (kind, position, names, require_distinct_layers) = match assertion {
            SemanticStateAssertion::Exact { position, objects } => {
                ("exact", *position, objects, true)
            }
            SemanticStateAssertion::Contains { position, objects } => {
                if objects.is_empty() {
                    return Err(InvestigationError::new(
                        diagnostics.invalid_code,
                        format!(
                            "{} contains assertion requires at least one object",
                            diagnostics.label
                        ),
                    ));
                }
                ("contains", *position, objects, true)
            }
            SemanticStateAssertion::Excludes { position, objects } => {
                if objects.is_empty() {
                    return Err(InvestigationError::new(
                        diagnostics.invalid_code,
                        format!(
                            "{} excludes assertion requires at least one object",
                            diagnostics.label
                        ),
                    ));
                }
                ("excludes", *position, objects, false)
            }
        };
        validate_semantic_position(state, position, &format!("{} assertion", diagnostics.label))?;
        let expected = resolve_semantic_objects(
            game,
            names,
            &format!("{} assertion", diagnostics.label),
            require_distinct_layers,
            diagnostics.invalid_code,
        )?;
        let actual = cell_objects(state, position[0], position[1]);
        let matches = match assertion {
            SemanticStateAssertion::Exact { .. } => same_object_set(&expected, &actual),
            SemanticStateAssertion::Contains { .. } => {
                expected.iter().all(|object| actual.contains(object))
            }
            SemanticStateAssertion::Excludes { .. } => {
                expected.iter().all(|object| !actual.contains(object))
            }
        };
        if !matches {
            return Err(InvestigationError::new(
                diagnostics.failed_code,
                format!(
                    "{} {kind} assertion failed at ({},{})",
                    diagnostics.label, position[0], position[1]
                ),
            )
            .with_details(json!({
                "kind": kind,
                "position": position,
                "objects": names,
                "actual": semantic_object_names(game, &actual),
            })));
        }
    }
    Ok(())
}

fn clone_goal_cell(cell: &SemanticGoalCell) -> SemanticGoalCell {
    match cell {
        SemanticGoalCell::Exact(objects) => SemanticGoalCell::Exact(objects.clone()),
        SemanticGoalCell::Contains(objects) => SemanticGoalCell::Contains(objects.clone()),
        SemanticGoalCell::Excludes(objects) => SemanticGoalCell::Excludes(objects.clone()),
        SemanticGoalCell::Unknown => SemanticGoalCell::Unknown,
    }
}

fn semantic_goal_cell_matches(cell: &SemanticGoalCell, actual: &[ObjectId]) -> bool {
    match cell {
        SemanticGoalCell::Exact(expected) => same_object_set(expected, actual),
        SemanticGoalCell::Contains(required) => {
            required.iter().all(|object| actual.contains(object))
        }
        SemanticGoalCell::Excludes(forbidden) => {
            forbidden.iter().all(|object| !actual.contains(object))
        }
        SemanticGoalCell::Unknown => true,
    }
}

fn semantic_goal_cell_kind(cell: &SemanticGoalCell) -> &'static str {
    match cell {
        SemanticGoalCell::Exact(_) => "exact",
        SemanticGoalCell::Contains(_) => "contains",
        SemanticGoalCell::Excludes(_) => "excludes",
        SemanticGoalCell::Unknown => "unknown",
    }
}

fn semantic_goal_cell_objects(cell: &SemanticGoalCell) -> &[ObjectId] {
    match cell {
        SemanticGoalCell::Exact(objects)
        | SemanticGoalCell::Contains(objects)
        | SemanticGoalCell::Excludes(objects) => objects,
        SemanticGoalCell::Unknown => &[],
    }
}

fn semantic_goal_matches(goal: &SemanticGoal, state: &State) -> bool {
    semantic_goal_mismatch_count(goal, state) == 0
}

fn semantic_goal_root_objects(game: &LoadedGame, goal: &SemanticGoal) -> BTreeSet<ObjectId> {
    if goal
        .cells
        .iter()
        .any(|cell| matches!(cell, SemanticGoalCell::Exact(_)))
    {
        return game
            .game
            .objects()
            .iter()
            .filter_map(|object| (!object.id.is_empty()).then_some(object.id))
            .collect();
    }
    goal.cells
        .iter()
        .flat_map(|cell| match cell {
            SemanticGoalCell::Contains(objects) | SemanticGoalCell::Excludes(objects) => {
                objects.as_slice()
            }
            SemanticGoalCell::Exact(_) | SemanticGoalCell::Unknown => &[],
        })
        .copied()
        .filter(|object| !object.is_empty())
        .collect()
}

fn validate_goal_source(
    goal: &SemanticGoal,
    source: &StateRecord,
    label: &str,
) -> Result<(), InvestigationError> {
    if source.level_index != goal.level_index
        || source.state.width != goal.width
        || source.state.height != goal.height
    {
        return Err(InvestigationError::new(
            "semantic_goal_state_mismatch",
            format!("semantic goal and {label} must have the same level and dimensions"),
        ));
    }
    Ok(())
}

fn semantic_goal_mismatches(
    game: &LoadedGame,
    goal: &SemanticGoal,
    state: &State,
) -> (usize, usize, Vec<Value>) {
    let mut mismatches = Vec::new();
    let mut checked_cells = 0_usize;
    let mut unknown_cells = 0_usize;
    for y in 0..goal.height {
        for x in 0..goal.width {
            let index = usize::from(y) * usize::from(goal.width) + usize::from(x);
            match &goal.cells[index] {
                SemanticGoalCell::Unknown => unknown_cells += 1,
                predicate => {
                    checked_cells += 1;
                    let actual = cell_objects(state, x, y);
                    if !semantic_goal_cell_matches(predicate, &actual) {
                        mismatches.push(json!({
                            "x": x,
                            "y": y,
                            "predicate": semantic_goal_cell_kind(predicate),
                            "objects": semantic_object_names(game, semantic_goal_cell_objects(predicate)),
                            "actual": semantic_object_names(game, &actual),
                        }));
                    }
                }
            }
        }
    }
    (checked_cells, unknown_cells, mismatches)
}

fn semantic_goal_mismatch_count(goal: &SemanticGoal, state: &State) -> usize {
    if state.width != goal.width || state.height != goal.height {
        return usize::MAX;
    }
    let mut mismatches = 0_usize;
    for y in 0..goal.height {
        for x in 0..goal.width {
            let index = usize::from(y) * usize::from(goal.width) + usize::from(x);
            let cell = &goal.cells[index];
            if !matches!(cell, SemanticGoalCell::Unknown) {
                let actual = cell_objects(state, x, y);
                if !semantic_goal_cell_matches(cell, &actual) {
                    mismatches += 1;
                }
            }
        }
    }
    mismatches
}

fn search_algorithm_name(algorithm: SemanticGoalSearchAlgorithm) -> &'static str {
    match algorithm {
        SemanticGoalSearchAlgorithm::Bfs => "bfs",
        SemanticGoalSearchAlgorithm::BestFirst => "best_first",
    }
}

fn search_session_score(
    algorithm: SemanticGoalSearchAlgorithm,
    goal: &SemanticGoal,
    state: &AgentSearchState,
) -> i64 {
    match algorithm {
        SemanticGoalSearchAlgorithm::Bfs => 0,
        SemanticGoalSearchAlgorithm::BestFirst => {
            semantic_goal_mismatch_count(goal, state.observation_state()) as i64
        }
    }
}

fn candidate_id(index: usize) -> String {
    format!("candidate-{index}")
}

fn parse_candidate_id(value: &str) -> Result<usize, InvestigationError> {
    value
        .strip_prefix("candidate-")
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or_else(|| {
            InvestigationError::new(
                "invalid_candidate_id",
                format!("candidate id {value:?} is invalid"),
            )
        })
}

fn resumable_status_name(status: ResumableSearchStatus) -> &'static str {
    match status {
        ResumableSearchStatus::Active => "active",
        ResumableSearchStatus::Solved { .. } => "solved",
        ResumableSearchStatus::Exhausted => "exhausted",
        ResumableSearchStatus::ResourceLimit => "resource_limit",
        ResumableSearchStatus::Failed => "failed",
    }
}

fn search_session_status(search: &SearchSession) -> &'static str {
    match search.machine.status() {
        ResumableSearchStatus::Active if search.advanced => "paused",
        ResumableSearchStatus::Active => "ready",
        status => resumable_status_name(status),
    }
}

fn search_stats_json(stats: &SearchStats) -> Value {
    json!({
        "visited": stats.visited,
        "expanded": stats.expanded,
        "frontier": stats.frontier,
        "maxDepthReached": stats.max_depth_reached,
        "elapsedMillis": stats.elapsed.as_millis(),
    })
}

fn search_candidate_json(
    game: &LoadedGame,
    goal: &SemanticGoal,
    candidate: ResumableSearchCandidate<AgentSearchState, InputId>,
) -> Result<Value, InvestigationError> {
    let inputs = candidate
        .actions
        .iter()
        .map(|input| {
            game.input_labels.get(input).cloned().ok_or_else(|| {
                InvestigationError::new(
                    "search_input_contract_mismatch",
                    format!("solver candidate contains unnamed input id {}", input.0),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (_, _, goal_diff) =
        semantic_goal_mismatches(game, goal, candidate.state.observation_state());
    Ok(json!({
        "candidateId": candidate_id(candidate.index),
        "score": candidate.score,
        "depth": candidate.depth,
        "discoveryIndex": candidate.discovery_index,
        "inputs": inputs,
        "stateHash": state_hash(candidate.state.observation_state()),
        "goalDiff": goal_diff,
    }))
}

fn search_unsolved_response(
    goal_id: &str,
    from_state_id: &str,
    algorithm: SemanticGoalSearchAlgorithm,
    outcome: &str,
    stats: SearchStats,
) -> Value {
    json!({
        "goalId": goal_id,
        "fromStateId": from_state_id,
        "algorithm": search_algorithm_name(algorithm),
        "searchOutcome": outcome,
        "stats": {
            "visited": stats.visited,
            "expanded": stats.expanded,
            "frontier": stats.frontier,
            "maxDepthReached": stats.max_depth_reached,
            "elapsedMillis": stats.elapsed.as_millis(),
        }
    })
}

fn same_object_set(left: &[ObjectId], right: &[ObjectId]) -> bool {
    let mut left = left.iter().map(|object| object.0).collect::<Vec<_>>();
    let mut right = right.iter().map(|object| object.0).collect::<Vec<_>>();
    left.sort_unstable();
    right.sort_unstable();
    left == right
}

fn semantic_object_names(game: &LoadedGame, objects: &[ObjectId]) -> Vec<String> {
    objects
        .iter()
        .map(|object| game.object_name(*object).to_string())
        .collect()
}

fn single_semantic_char(value: &str, label: &str) -> Result<char, InvestigationError> {
    let mut chars = value.chars();
    let Some(ch) = chars.next() else {
        return Err(InvestigationError::new(
            "invalid_semantic_state",
            format!("semantic state {label} must be one character"),
        ));
    };
    if chars.next().is_some() || !usable_semantic_char(ch) {
        return Err(InvestigationError::new(
            "invalid_semantic_state",
            format!("semantic state {label} must be one printable non-whitespace character"),
        ));
    }
    Ok(ch)
}

fn semantic_empty_char(value: &str) -> Result<char, InvestigationError> {
    let ch = single_semantic_char(value, "empty")?;
    if ch == '?' {
        return Err(InvestigationError::new(
            "invalid_semantic_legend",
            "`?` cannot be the empty character; declare it explicitly as unknown in a semantic goal",
        ));
    }
    Ok(ch)
}

fn usable_semantic_char(ch: char) -> bool {
    ch.is_ascii_graphic() && ch != '"' && ch != '\\'
}

fn assignable_semantic_char(ch: char) -> bool {
    usable_semantic_char(ch) && ch != '?'
}

fn semantic_char_pool() -> impl Iterator<Item = char> {
    "_#@+$*oxABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!%&'(),-/:;<=>[]^`{|}~"
        .chars()
}

fn semantic_state_diff(game: &LoadedGame, before: &State, after: &State) -> Value {
    let mut object_ids = game.object_labels.keys().copied().collect::<Vec<_>>();
    object_ids.sort_by_key(|object| object.0);
    let mut object_changes = Vec::new();
    for object in object_ids {
        let before_positions = object_positions(before, object);
        let after_positions = object_positions(after, object);
        let removed = before_positions
            .difference(&after_positions)
            .map(|(x, y)| json!([x, y]))
            .collect::<Vec<_>>();
        let added = after_positions
            .difference(&before_positions)
            .map(|(x, y)| json!([x, y]))
            .collect::<Vec<_>>();
        if !removed.is_empty() || !added.is_empty() {
            object_changes.push(json!({
                "object": game.object_name(object),
                "removed": removed,
                "added": added,
            }));
        }
    }
    let variable_changes = changed_variable_details(game, before, after);
    json!({
        "objects": object_changes,
        "variables": variable_changes,
    })
}

fn object_positions(state: &State, object: ObjectId) -> BTreeSet<(u16, u16)> {
    let mut positions = BTreeSet::new();
    for y in 0..state.height {
        for x in 0..state.width {
            if cell_objects(state, x, y).contains(&object) {
                positions.insert((x, y));
            }
        }
    }
    positions
}

fn changed_variable_details(game: &LoadedGame, before: &State, after: &State) -> Vec<Value> {
    before
        .visible_variables()
        .iter()
        .zip(after.visible_variables())
        .enumerate()
        .filter_map(|(index, (before, after))| {
            (before != after).then(|| {
                let name = game
                    .variable_labels
                    .iter()
                    .find_map(|(id, name)| (usize::from(id.0) == index).then_some(name.clone()))
                    .unwrap_or_else(|| format!("variable:{index}"));
                json!({ "variable": name, "before": before, "after": after })
            })
        })
        .collect()
}

fn describe_state(game: &LoadedGame, state: &State) -> Value {
    let mut positions = BTreeMap::<(u16, String), Vec<Value>>::new();
    for y in 0..state.height {
        for x in 0..state.width {
            for layer in 0..state.layer_count {
                let index = ((usize::from(y) * usize::from(state.width) + usize::from(x))
                    * usize::from(state.layer_count))
                    + usize::from(layer);
                let object = state.slots()[index];
                if object.is_empty() {
                    continue;
                }
                let name = game.object_name(object).to_string();
                positions
                    .entry((object.0, name))
                    .or_default()
                    .push(json!([x, y]));
            }
        }
    }
    let objects = positions
        .into_iter()
        .map(|((id, name), positions)| json!({ "id": id, "name": name, "positions": positions }))
        .collect::<Vec<_>>();
    let mut variables = game
        .variable_labels
        .iter()
        .filter_map(|(id, name)| {
            state
                .visible_variables()
                .get(usize::from(id.0))
                .map(|value| (name.clone(), *value))
        })
        .collect::<BTreeMap<_, _>>();
    // Preserve an object rather than null when the game has no variables.
    if variables.is_empty() {
        variables = BTreeMap::new();
    }
    json!({
        "hash": state_hash(state),
        "size": { "width": state.width, "height": state.height, "layers": state.layer_count },
        "objects": objects,
        "variables": variables,
        "goal": game.is_goal_complete(state),
        "lose": game.is_lose_complete(state),
    })
}

fn describe_run_point(game: &LoadedGame, point: &RunPoint, include_trace: bool) -> Value {
    let trace = include_trace
        .then(|| point.trace.as_ref().map(describe_trace))
        .flatten();
    json!({
        "index": point.index,
        "state": describe_state(game, &point.state),
        "goal": point.goal,
        "lose": point.lose,
        "trace": trace,
    })
}

fn describe_trace(trace: &TransitionTrace) -> Value {
    json!({
        "inputId": trace.input.0,
        "target": trace.target,
        "cancelled": trace.cancelled,
        "commands": trace.commands.iter().map(command_name).collect::<Vec<_>>(),
        "firedRules": trace.firings.iter().map(|firing| firing.rule.0).collect::<Vec<_>>(),
        "patchCount": trace.firings.len(),
    })
}

fn command_name(command: &TransitionCommand) -> &'static str {
    match command {
        TransitionCommand::Win => "win",
        TransitionCommand::Restart => "restart",
        TransitionCommand::NextLevel => "next_level",
        TransitionCommand::Again => "again",
        TransitionCommand::Checkpoint => "checkpoint",
        TransitionCommand::ClearCheckpoint => "clear_checkpoint",
    }
}

fn observed_points(
    game: &LoadedGame,
    points: &[RunPoint],
    observation: &ObservationRequest,
) -> Vec<Value> {
    let indices = match observation.mode {
        ObservationMode::Summary | ObservationMode::Events => Vec::new(),
        ObservationMode::Indices => observation.indices.clone(),
        ObservationMode::All => (0..points.len()).collect(),
    };
    indices
        .into_iter()
        .filter_map(|index| points.get(index))
        .map(|point| describe_run_point(game, point, false))
        .collect()
}

fn run_events(game: &LoadedGame, points: &[RunPoint]) -> Vec<Value> {
    let mut events = Vec::new();
    let mut previous_rules = Vec::<u16>::new();
    let mut no_op_start = None::<usize>;
    for pair in points.windows(2) {
        let before = &pair[0];
        let after = &pair[1];
        let no_op = before.state == after.state;
        match (no_op_start, no_op) {
            (None, true) => no_op_start = Some(after.index),
            (Some(start), false) => {
                events
                    .push(json!({ "kind": "no_op_range", "start": start, "end": after.index - 1 }));
                no_op_start = None;
            }
            _ => {}
        }
        if before.goal != after.goal {
            events.push(
                json!({ "kind": "goal_changed", "inputIndex": after.index, "value": after.goal }),
            );
        }
        if before.lose != after.lose {
            events.push(
                json!({ "kind": "lose_changed", "inputIndex": after.index, "value": after.lose }),
            );
        }
        if !changed_variable_names(game, &before.state, &after.state).is_empty() {
            events.push(json!({
                "kind": "variables_changed",
                "inputIndex": after.index,
                "variables": changed_variable_names(game, &before.state, &after.state),
            }));
        }
        if let Some(trace) = &after.trace {
            let rules = trace
                .firings
                .iter()
                .map(|firing| firing.rule.0)
                .collect::<Vec<_>>();
            if rules != previous_rules {
                events.push(json!({
                    "kind": "rule_signature_changed",
                    "inputIndex": after.index,
                    "firedRules": rules,
                }));
                previous_rules = trace.firings.iter().map(|firing| firing.rule.0).collect();
            }
            if !trace.commands.is_empty() || trace.cancelled {
                events.push(json!({
                    "kind": "transition_control",
                    "inputIndex": after.index,
                    "cancelled": trace.cancelled,
                    "commands": trace.commands.iter().map(command_name).collect::<Vec<_>>(),
                }));
            }
        }
    }
    if let Some(start) = no_op_start {
        events.push(json!({
            "kind": "no_op_range",
            "start": start,
            "end": points.len().saturating_sub(1),
        }));
    }
    events
}

fn run_summary(game: &LoadedGame, initial: &State, terminal: &State, points: &[RunPoint]) -> Value {
    let commands = points
        .iter()
        .filter_map(|point| point.trace.as_ref())
        .flat_map(|trace| trace.commands.iter().map(command_name))
        .collect::<BTreeSet<_>>();
    json!({
        "changedObjects": changed_object_names(game, initial, terminal),
        "changedVariables": changed_variable_names(game, initial, terminal),
        "commands": commands,
        "goalBefore": game.is_goal_complete(initial),
        "goalAfter": game.is_goal_complete(terminal),
        "loseBefore": game.is_lose_complete(initial),
        "loseAfter": game.is_lose_complete(terminal),
    })
}

fn changed_object_names(game: &LoadedGame, left: &State, right: &State) -> Vec<String> {
    let mut changed = BTreeSet::new();
    for (before, after) in left.slots().iter().zip(right.slots()) {
        if before == after {
            continue;
        }
        if !before.is_empty() {
            changed.insert(game.object_name(*before).to_string());
        }
        if !after.is_empty() {
            changed.insert(game.object_name(*after).to_string());
        }
    }
    changed.into_iter().collect()
}

fn changed_variable_names(game: &LoadedGame, left: &State, right: &State) -> Vec<String> {
    let mut changed = Vec::new();
    for (index, (before, after)) in left
        .visible_variables()
        .iter()
        .zip(right.visible_variables())
        .enumerate()
    {
        if before == after {
            continue;
        }
        let name = game
            .variable_labels
            .iter()
            .find_map(|(id, name)| (usize::from(id.0) == index).then_some(name.clone()))
            .unwrap_or_else(|| format!("variable:{index}"));
        changed.push(name);
    }
    changed.sort();
    changed
}

fn state_hash(state: &State) -> String {
    format!("{:016x}", state.hash())
}

fn stable_hash_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
