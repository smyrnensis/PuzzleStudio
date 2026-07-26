#[cfg(not(target_arch = "wasm32"))]
pub mod investigation;

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use puzzle_core::{
    GridGuard, GridRuleCondition, GridRuleStep, GridSize, GridState, InputId, ObjectId, Size2,
    Size3,
};
use puzzle_lang::{LoadedDocumentModel, LoadedGame, LoadedGridGame, WorkspaceSourceDocument};
#[cfg(not(target_arch = "wasm32"))]
use puzzle_play::GameSession;
use puzzle_play::{GridGameSession, GridHeadlessSession, loaded_document_scene_host_loaded_game};
use puzzle_runtime_contract::{
    RuntimeModelKind, SolverAdvanceResponse, SolverMove, SolverObservation, SolverPreparedArtifact,
    SolverResult, SolverSearchProgress, SolverSearchRequest, SolverSearchStats, SolverSearchStatus,
    SolverStateSnapshot, SolverStep,
};
use puzzle_solver::{
    GridPuzzleDomain, GridSearchState, GridStateKey, ResumableAdvanceOutcome, ResumableBestFirst,
    ResumableSearchAllowance, ResumableSearchLimits, SearchDomain, SearchStats,
    solver_strategy_has_deadend, solver_strategy_object_roots, solver_strategy_score,
};

pub const MAX_PREPARED_ARTIFACTS: usize = 8;
pub const MAX_PREPARED_SOURCE_BYTES: usize = 64 * 1024 * 1024;
pub const PREPARED_ARTIFACT_IDLE_TTL_MS: u64 = 15 * 60 * 1000;

enum PreparedModel {
    TwoD(Arc<LoadedGame>),
    ThreeD(Arc<LoadedGridGame<3, Size3>>),
}

struct PreparedArtifact {
    model: PreparedModel,
    estimated_bytes: usize,
    last_used_ms: u64,
    active_searches: usize,
    pinned: bool,
}

struct ActiveGridSearch<const D: usize, Size: GridSize<D>> {
    loaded: Arc<LoadedGridGame<D, Size>>,
    logical_loaded: Arc<LoadedGridGame<D, Size>>,
    state_slicer: puzzle_solver::SolverStateSlicer,
    level_index: usize,
    initial_session: GridGameSession<D, Size>,
    domain: GridPuzzleDomain<D, Size>,
    machine: ResumableBestFirst<GridSearchState<D, Size>, InputId, GridStateKey<D>>,
}

enum ActiveSearch {
    TwoD(ActiveGridSearch<2, Size2>),
    ThreeD(ActiveGridSearch<3, Size3>),
}

struct SearchEntry {
    artifact_id: String,
    search: ActiveSearch,
}

#[cfg(not(target_arch = "wasm32"))]
struct SolverObservationSampler {
    observations: Vec<SolverObservation>,
    max_samples: usize,
    next_expanded: usize,
    stride: usize,
}

#[cfg(not(target_arch = "wasm32"))]
impl SolverObservationSampler {
    fn new(max_samples: usize) -> Self {
        Self {
            observations: Vec::new(),
            max_samples: max_samples.max(1),
            next_expanded: 0,
            stride: 1,
        }
    }

    fn observe(&mut self, observation: Option<SolverObservation>) {
        let Some(observation) = observation else {
            return;
        };
        if observation.progress.expanded < self.next_expanded {
            return;
        }
        self.next_expanded = observation.progress.expanded.saturating_add(self.stride);
        self.observations.push(observation);
        if self.observations.len() > self.max_samples {
            self.observations = self.observations.iter().step_by(2).cloned().collect();
            self.stride = self.stride.saturating_mul(2).max(1);
            self.next_expanded = self.observations.last().map_or(0, |sample| {
                sample.progress.expanded.saturating_add(self.stride)
            });
        }
    }

    fn into_observations(self) -> Vec<SolverObservation> {
        self.observations
    }
}

pub struct SolverService {
    artifacts: HashMap<String, PreparedArtifact>,
    searches: HashMap<u32, SearchEntry>,
    next_loaded_artifact_id: u64,
    next_search_id: u32,
}

impl Default for SolverService {
    fn default() -> Self {
        Self::new()
    }
}

impl SolverService {
    pub fn new() -> Self {
        Self {
            artifacts: HashMap::new(),
            searches: HashMap::new(),
            next_loaded_artifact_id: 1,
            next_search_id: 1,
        }
    }

    pub fn prepare_loaded_game(
        &mut self,
        loaded: Arc<LoadedGame>,
        now_ms: u64,
    ) -> SolverPreparedArtifact {
        let artifact_id = format!("loaded-{}", self.next_loaded_artifact_id);
        self.next_loaded_artifact_id = self.next_loaded_artifact_id.wrapping_add(1).max(1);
        let model = PreparedModel::TwoD(loaded);
        let info = prepared_info(&artifact_id, &model);
        self.artifacts.insert(
            artifact_id,
            PreparedArtifact {
                model,
                estimated_bytes: 0,
                last_used_ms: now_ms,
                active_searches: 0,
                pinned: true,
            },
        );
        info
    }

    pub fn prepare_workspace(
        &mut self,
        entry_path: &str,
        documents: Vec<WorkspaceSourceDocument>,
        now_ms: u64,
    ) -> Result<SolverPreparedArtifact, String> {
        if entry_path.trim().is_empty() {
            return Err("solver workspace requires an explicit entry path".to_string());
        }
        if documents.is_empty() {
            return Err("solver workspace requires at least one document".to_string());
        }
        if documents
            .iter()
            .any(|document| document.path.trim().is_empty())
        {
            return Err("solver workspace document paths must be explicit".to_string());
        }
        let artifact_id = workspace_fingerprint(entry_path, &documents);
        if let Some(artifact) = self.artifacts.get_mut(&artifact_id) {
            artifact.last_used_ms = now_ms;
            return Ok(prepared_info(&artifact_id, &artifact.model));
        }

        let estimated_bytes = documents
            .iter()
            .map(|document| document.path.len().saturating_add(document.source.len()))
            .sum();
        if estimated_bytes > MAX_PREPARED_SOURCE_BYTES {
            return Err(format!(
                "solver workspace requires {estimated_bytes} source bytes, exceeding the {MAX_PREPARED_SOURCE_BYTES}-byte prepared-artifact limit"
            ));
        }
        let document = puzzle_lang::parse_workspace_game(entry_path, &documents)
            .map_err(|error| error.to_string())?;
        let model = match document.single_model() {
            Some(LoadedDocumentModel::Puzzle3d { game, .. }) => {
                PreparedModel::ThreeD(Arc::new(game.clone()))
            }
            _ => PreparedModel::TwoD(Arc::new(loaded_document_scene_host_loaded_game(&document)?)),
        };
        let info = prepared_info(&artifact_id, &model);
        self.artifacts.insert(
            artifact_id,
            PreparedArtifact {
                model,
                estimated_bytes,
                last_used_ms: now_ms,
                active_searches: 0,
                pinned: false,
            },
        );
        self.evict(now_ms);
        if !self.artifacts.contains_key(&info.artifact_id) {
            return Err(
                "solver prepared-artifact capacity is exhausted by pinned or active artifacts"
                    .to_string(),
            );
        }
        Ok(info)
    }

    pub fn pin_artifact(&mut self, artifact_id: Option<&str>, now_ms: u64) -> Result<(), String> {
        if let Some(id) = artifact_id {
            if !self.artifacts.contains_key(id) {
                return Err(format!("prepared solver artifact {id:?} does not exist"));
            }
        }
        for (id, artifact) in &mut self.artifacts {
            artifact.pinned = artifact_id.is_some_and(|selected| selected == id);
            if artifact.pinned {
                artifact.last_used_ms = now_ms;
            }
        }
        self.evict(now_ms);
        Ok(())
    }

    pub fn start(
        &mut self,
        artifact_id: &str,
        request: SolverSearchRequest,
        now_ms: u64,
    ) -> Result<u32, String> {
        if request.max_depth == 0 || request.max_stored_nodes == 0 {
            return Err("solver maxDepth and maxStoredNodes must be positive".to_string());
        }
        let search = {
            let artifact = self.artifacts.get_mut(artifact_id).ok_or_else(|| {
                format!("prepared solver artifact {artifact_id:?} does not exist")
            })?;
            artifact.last_used_ms = now_ms;
            match &artifact.model {
                PreparedModel::TwoD(loaded) => ActiveSearch::TwoD(start_grid_search(
                    Arc::clone(loaded),
                    request,
                    SolverStateSnapshot::into_state2,
                )?),
                PreparedModel::ThreeD(loaded) => ActiveSearch::ThreeD(start_grid_search(
                    Arc::clone(loaded),
                    request,
                    SolverStateSnapshot::into_state3,
                )?),
            }
        };
        self.store_search(artifact_id, search)
    }

    pub fn advance(
        &mut self,
        search_id: u32,
        max_expanded_nodes: usize,
        max_millis: u64,
        now_ms: u64,
    ) -> Result<SolverAdvanceResponse, String> {
        if max_expanded_nodes == 0 || max_millis == 0 {
            return Err("solver advance allowance must be positive".to_string());
        }
        self.advance_with_duration(
            search_id,
            max_expanded_nodes,
            Some(Duration::from_millis(max_millis)),
            now_ms,
        )
    }

    pub fn advance_nodes(
        &mut self,
        search_id: u32,
        max_expanded_nodes: usize,
        now_ms: u64,
    ) -> Result<SolverAdvanceResponse, String> {
        if max_expanded_nodes == 0 {
            return Err("solver advance node allowance must be positive".to_string());
        }
        self.advance_with_duration(search_id, max_expanded_nodes, None, now_ms)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn solve_game_session_to_completion(
        &mut self,
        artifact_id: &str,
        level_index: usize,
        session: GameSession,
        max_depth: u32,
        max_stored_nodes: usize,
        max_duration: Duration,
        now_ms: u64,
    ) -> Result<SolverResult, String> {
        if max_depth == 0 || max_stored_nodes == 0 {
            return Err("solver maxDepth and maxStoredNodes must be positive".to_string());
        }
        let search = {
            let artifact = self.artifacts.get_mut(artifact_id).ok_or_else(|| {
                format!("prepared solver artifact {artifact_id:?} does not exist")
            })?;
            artifact.last_used_ms = now_ms;
            let PreparedModel::TwoD(loaded) = &artifact.model else {
                return Err("2d game session requires a 2d solver artifact".to_string());
            };
            ActiveSearch::TwoD(start_grid_search_from_session(
                Arc::clone(loaded),
                level_index,
                session,
                max_depth,
                max_stored_nodes,
            )?)
        };
        let search_id = self.store_search(artifact_id, search)?;
        self.solve_to_completion(search_id, max_duration, now_ms)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn solve_to_completion(
        &mut self,
        search_id: u32,
        max_duration: Duration,
        now_ms: u64,
    ) -> Result<SolverResult, String> {
        const ADVANCE_NODES: usize = 64;
        const MAX_OBSERVATIONS: usize = 96;

        let started = Instant::now();
        let mut observations = SolverObservationSampler::new(MAX_OBSERVATIONS);
        let mut last_stats = SolverSearchStats {
            visited: 0,
            expanded: 0,
            frontier: 0,
            max_depth_reached: 0,
            elapsed_ms: 0,
        };
        loop {
            let elapsed = started.elapsed();
            if elapsed >= max_duration {
                self.cancel(
                    search_id,
                    now_ms.saturating_add(elapsed.as_millis().min(u128::from(u64::MAX)) as u64),
                )?;
                last_stats.elapsed_ms = last_stats
                    .elapsed_ms
                    .max(elapsed.as_millis().min(u128::from(u64::MAX)) as u64);
                return Ok(SolverResult {
                    model: RuntimeModelKind::TwoD,
                    result: SolverSearchStatus::BudgetExceeded,
                    depth: None,
                    moves: Vec::new(),
                    steps: Vec::new(),
                    observations: observations.into_observations(),
                    stats: last_stats,
                    error: None,
                });
            }
            let response = self.advance_with_duration(
                search_id,
                ADVANCE_NODES,
                Some(max_duration.saturating_sub(elapsed)),
                now_ms.saturating_add(elapsed.as_millis().min(u128::from(u64::MAX)) as u64),
            )?;
            last_stats = response.stats.clone();
            observations.observe(response.observation);
            if response.status == SolverSearchStatus::Paused {
                continue;
            }
            let mut result = response.result.ok_or_else(|| {
                format!(
                    "terminal solver status {:?} did not include a result",
                    response.status
                )
            })?;
            result.observations = observations.into_observations();
            return Ok(result);
        }
    }

    fn advance_with_duration(
        &mut self,
        search_id: u32,
        max_expanded_nodes: usize,
        max_duration: Option<Duration>,
        now_ms: u64,
    ) -> Result<SolverAdvanceResponse, String> {
        let mut entry = self
            .searches
            .remove(&search_id)
            .ok_or_else(|| format!("solver search handle {search_id} does not exist"))?;
        let response = match &mut entry.search {
            ActiveSearch::TwoD(search) => advance_grid_search(
                search,
                max_expanded_nodes,
                max_duration,
                RuntimeModelKind::TwoD,
                SolverStateSnapshot::from_state2,
            ),
            ActiveSearch::ThreeD(search) => advance_grid_search(
                search,
                max_expanded_nodes,
                max_duration,
                RuntimeModelKind::ThreeD,
                SolverStateSnapshot::from_state3,
            ),
        };
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                self.release_artifact_search(&entry.artifact_id, now_ms);
                return Err(error);
            }
        };
        if response.status == SolverSearchStatus::Paused {
            self.searches.insert(search_id, entry);
        } else {
            self.release_artifact_search(&entry.artifact_id, now_ms);
        }
        Ok(response)
    }

    pub fn cancel(&mut self, search_id: u32, now_ms: u64) -> Result<(), String> {
        let entry = self
            .searches
            .remove(&search_id)
            .ok_or_else(|| format!("solver search handle {search_id} does not exist"))?;
        self.release_artifact_search(&entry.artifact_id, now_ms);
        Ok(())
    }

    pub fn materialize_state(
        &mut self,
        artifact_id: &str,
        level_index: usize,
        state: SolverStateSnapshot,
        materialize_level_start: bool,
        now_ms: u64,
    ) -> Result<SolverStateSnapshot, String> {
        let artifact = self
            .artifacts
            .get_mut(artifact_id)
            .ok_or_else(|| format!("prepared solver artifact {artifact_id:?} does not exist"))?;
        artifact.last_used_ms = now_ms;
        match &artifact.model {
            PreparedModel::TwoD(loaded) => {
                let state = state.into_state2(&loaded.game)?;
                let mut session = GridGameSession::new_headless_before_level_start(loaded);
                session
                    .start_level_from_state(loaded, level_index, state, materialize_level_start)
                    .map_err(|error| format!("{error:?}"))?;
                Ok(SolverStateSnapshot::from_state2(session.state()))
            }
            PreparedModel::ThreeD(loaded) => {
                let state = state.into_state3(&loaded.game)?;
                let mut session = GridGameSession::new_headless_before_level_start(loaded);
                session
                    .start_level_from_state(loaded, level_index, state, materialize_level_start)
                    .map_err(|error| format!("{error:?}"))?;
                Ok(SolverStateSnapshot::from_state3(session.state()))
            }
        }
    }

    fn store_search(&mut self, artifact_id: &str, search: ActiveSearch) -> Result<u32, String> {
        let search_id = self.allocate_search_id()?;
        self.artifacts
            .get_mut(artifact_id)
            .expect("artifact existed while its search was created")
            .active_searches += 1;
        self.searches.insert(
            search_id,
            SearchEntry {
                artifact_id: artifact_id.to_string(),
                search,
            },
        );
        Ok(search_id)
    }

    fn allocate_search_id(&mut self) -> Result<u32, String> {
        let start = self.next_search_id.max(1);
        loop {
            let id = self.next_search_id.max(1);
            self.next_search_id = id.wrapping_add(1).max(1);
            if !self.searches.contains_key(&id) {
                return Ok(id);
            }
            if self.next_search_id == start {
                return Err("solver search handle space is exhausted".to_string());
            }
        }
    }

    fn release_artifact_search(&mut self, artifact_id: &str, now_ms: u64) {
        if let Some(artifact) = self.artifacts.get_mut(artifact_id) {
            artifact.active_searches = artifact.active_searches.saturating_sub(1);
            artifact.last_used_ms = now_ms;
        }
        self.evict(now_ms);
    }

    fn evict(&mut self, now_ms: u64) {
        self.artifacts.retain(|_, artifact| {
            artifact.pinned
                || artifact.active_searches > 0
                || now_ms.saturating_sub(artifact.last_used_ms) <= PREPARED_ARTIFACT_IDLE_TTL_MS
        });
        while self.artifacts.len() > MAX_PREPARED_ARTIFACTS
            || self
                .artifacts
                .values()
                .map(|artifact| artifact.estimated_bytes)
                .sum::<usize>()
                > MAX_PREPARED_SOURCE_BYTES
        {
            let candidate = self
                .artifacts
                .iter()
                .filter(|(_, artifact)| !artifact.pinned && artifact.active_searches == 0)
                .min_by_key(|(_, artifact)| artifact.last_used_ms)
                .map(|(id, _)| id.clone());
            let Some(candidate) = candidate else {
                break;
            };
            self.artifacts.remove(&candidate);
        }
    }
}

fn prepared_info(id: &str, model: &PreparedModel) -> SolverPreparedArtifact {
    match model {
        PreparedModel::TwoD(loaded) => SolverPreparedArtifact {
            artifact_id: id.to_string(),
            model_kind: RuntimeModelKind::TwoD,
            level_count: loaded.levels.len(),
        },
        PreparedModel::ThreeD(loaded) => SolverPreparedArtifact {
            artifact_id: id.to_string(),
            model_kind: RuntimeModelKind::ThreeD,
            level_count: loaded.levels.len(),
        },
    }
}

fn start_grid_search<const D: usize, Size: GridSize<D>>(
    loaded: Arc<LoadedGridGame<D, Size>>,
    request: SolverSearchRequest,
    decode_state: fn(
        SolverStateSnapshot,
        &puzzle_core::GridCompiledGame<D>,
    ) -> Result<GridState<D, Size>, String>,
) -> Result<ActiveGridSearch<D, Size>, String> {
    let SolverSearchRequest {
        level_index,
        state,
        materialize_level_start,
        max_depth,
        max_stored_nodes,
    } = request;
    if level_index >= loaded.levels.len() {
        return Err(format!("solver level index out of range: {}", level_index));
    }
    let state = decode_state(state, &loaded.game)?;
    let mut initial_session = GridGameSession::new_headless_before_level_start(&loaded);
    initial_session
        .start_level_from_state(&loaded, level_index, state, materialize_level_start)
        .map_err(|error| format!("{error:?}"))?;
    start_grid_search_from_session(
        loaded,
        level_index,
        initial_session,
        max_depth,
        max_stored_nodes,
    )
}

fn start_grid_search_from_session<const D: usize, Size: GridSize<D>>(
    loaded: Arc<LoadedGridGame<D, Size>>,
    level_index: usize,
    initial_session: GridGameSession<D, Size>,
    max_depth: u32,
    max_stored_nodes: usize,
) -> Result<ActiveGridSearch<D, Size>, String> {
    if level_index >= loaded.levels.len() {
        return Err(format!("solver level index out of range: {level_index}"));
    }
    if initial_session.active_level_index() != Some(level_index) {
        return Err(format!(
            "solver session active level {:?} does not match requested level {level_index}",
            initial_session.active_level_index()
        ));
    }
    let real_initial = initial_session.state().clone();
    let (logical_model, state_slicer) =
        logical_model_and_state_slicer(&loaded, level_index, &real_initial)?;
    let logical_loaded = Arc::new(logical_model);
    let inputs = solver_inputs(&logical_loaded, level_index);
    if inputs.is_empty() {
        return Err("no model inputs available".to_string());
    }
    let domain = GridPuzzleDomain::with_state_slicer_for_level_completion(
        Arc::clone(&logical_loaded),
        level_index,
        inputs,
        state_slicer.clone(),
    );
    let initial = domain
        .initial_state(real_initial)
        .map_err(|error| format!("{error:?}"))?;
    let initial_key = domain.key(&initial);
    let initial_score = solver_strategy_score(&logical_loaded, initial.observation_state());
    let machine = ResumableBestFirst::new(
        initial,
        initial_key,
        initial_score,
        ResumableSearchLimits {
            max_depth,
            max_stored_nodes,
        },
    );
    Ok(ActiveGridSearch {
        loaded,
        logical_loaded,
        state_slicer,
        level_index,
        initial_session,
        domain,
        machine,
    })
}

fn advance_grid_search<const D: usize, Size: GridSize<D>>(
    search: &mut ActiveGridSearch<D, Size>,
    max_expanded_nodes: usize,
    max_duration: Option<Duration>,
    model: RuntimeModelKind,
    snapshot: fn(&GridState<D, Size>) -> SolverStateSnapshot,
) -> Result<SolverAdvanceResponse, String> {
    let loaded_for_score = Arc::clone(&search.logical_loaded);
    let loaded_for_deadend = Arc::clone(&search.logical_loaded);
    let outcome = search.machine.advance_with_dead_states(
        &mut search.domain,
        ResumableSearchAllowance {
            max_expanded_nodes,
            max_duration,
        },
        move |state| solver_strategy_score(&loaded_for_score, state.observation_state()),
        move |state| {
            solver_strategy_has_deadend(&loaded_for_deadend, state.observation_state())
                || loaded_for_deadend.is_lose_complete(state.observation_state())
        },
    );
    let (status, stats, solved_index, error) = match outcome {
        ResumableAdvanceOutcome::Paused { stats, .. } => {
            (SolverSearchStatus::Paused, stats, None, None)
        }
        ResumableAdvanceOutcome::Solved {
            candidate_index,
            stats,
        } => (
            SolverSearchStatus::Solved,
            stats,
            Some(candidate_index),
            None,
        ),
        ResumableAdvanceOutcome::Exhausted { stats } => {
            (SolverSearchStatus::Exhausted, stats, None, None)
        }
        ResumableAdvanceOutcome::ResourceLimit { stats } => {
            (SolverSearchStatus::ResourceLimit, stats, None, None)
        }
        ResumableAdvanceOutcome::Failed { failure, stats } => (
            SolverSearchStatus::Failed,
            stats,
            None,
            Some(format!("{:?}", failure.error)),
        ),
    };
    let stats_contract = search_stats(&stats);
    let observation = best_observation(search, &stats, snapshot)?;
    let result = if let Some(index) = solved_index {
        let candidate = search
            .machine
            .candidate(index)
            .ok_or_else(|| format!("solver candidate {index} disappeared"))?;
        Some(solved_result(
            search,
            model,
            candidate.state.state(),
            candidate.actions,
            candidate.depth,
            stats_contract.clone(),
            snapshot,
        )?)
    } else if status != SolverSearchStatus::Paused {
        Some(SolverResult {
            model,
            result: status,
            depth: None,
            moves: Vec::new(),
            steps: Vec::new(),
            observations: observation.clone().into_iter().collect(),
            stats: stats_contract.clone(),
            error,
        })
    } else {
        None
    };
    Ok(SolverAdvanceResponse {
        status,
        stats: stats_contract,
        observation,
        result,
    })
}

fn best_observation<const D: usize, Size: GridSize<D>>(
    search: &ActiveGridSearch<D, Size>,
    stats: &SearchStats,
    snapshot: fn(&GridState<D, Size>) -> SolverStateSnapshot,
) -> Result<Option<SolverObservation>, String> {
    let Some(candidate) = search.machine.best_candidates(1).into_iter().next() else {
        return Ok(None);
    };
    let state = materialize_witness_state(search, &candidate.actions, candidate.state.state())?;
    Ok(Some(SolverObservation {
        progress: SolverSearchProgress {
            visited: stats.visited,
            expanded: stats.expanded,
            frontier: stats.frontier,
            max_depth_reached: stats.max_depth_reached,
            depth: candidate.depth,
        },
        state: snapshot(&state),
    }))
}

fn materialize_witness_state<const D: usize, Size: GridSize<D>>(
    search: &ActiveGridSearch<D, Size>,
    actions: &[InputId],
    expected_logical: &GridState<D, Size>,
) -> Result<GridState<D, Size>, String> {
    let mut replay =
        GridHeadlessSession::from_game_session(search.initial_session.clone(), search.level_index)
            .map_err(|error| format!("{error:?}"))?;
    for input in actions {
        replay
            .apply_input(&search.loaded, *input)
            .map_err(|error| format!("{error:?}"))?;
    }
    let real = replay.observation_state().clone();
    let reconstructed_logical = search.state_slicer.project_state(&real);
    if &reconstructed_logical != expected_logical {
        return Err(
            "materialized solver witness does not reconstruct its logical candidate".to_string(),
        );
    }
    Ok(real)
}

fn solved_result<const D: usize, Size: GridSize<D>>(
    search: &ActiveGridSearch<D, Size>,
    model: RuntimeModelKind,
    expected_logical: &GridState<D, Size>,
    actions: Vec<InputId>,
    depth: u32,
    stats: SolverSearchStats,
    snapshot: fn(&GridState<D, Size>) -> SolverStateSnapshot,
) -> Result<SolverResult, String> {
    materialize_witness_state(search, &actions, expected_logical)?;
    let moves = actions
        .iter()
        .map(|input| solver_move(&search.loaded, *input))
        .collect::<Result<Vec<_>, _>>()?;
    let mut replay =
        GridHeadlessSession::from_game_session(search.initial_session.clone(), search.level_index)
            .map_err(|error| format!("{error:?}"))?;
    let mut steps = vec![SolverStep {
        index: 0,
        input: None,
        state: snapshot(replay.observation_state()),
    }];
    for (index, input) in actions.iter().copied().enumerate() {
        replay
            .apply_input(&search.loaded, input)
            .map_err(|error| format!("{error:?}"))?;
        steps.push(SolverStep {
            index: index + 1,
            input: Some(solver_move(&search.loaded, input)?),
            state: snapshot(replay.observation_state()),
        });
    }
    Ok(SolverResult {
        model,
        result: SolverSearchStatus::Solved,
        depth: Some(depth),
        moves,
        steps,
        observations: Vec::new(),
        stats,
        error: None,
    })
}

fn solver_move<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    input: InputId,
) -> Result<SolverMove, String> {
    let name = loaded
        .input_labels
        .get(&input)
        .cloned()
        .or_else(|| {
            loaded
                .inputs
                .iter()
                .find(|candidate| candidate.id == input)
                .map(|candidate| candidate.name.clone())
        })
        .ok_or_else(|| format!("compiled input {} is missing its label", input.0))?;
    Ok(SolverMove { id: input.0, name })
}

fn search_stats(stats: &SearchStats) -> SolverSearchStats {
    SolverSearchStats {
        visited: stats.visited,
        expanded: stats.expanded,
        frontier: stats.frontier,
        max_depth_reached: stats.max_depth_reached,
        elapsed_ms: stats.elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
    }
}

fn solver_inputs<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    level_index: usize,
) -> Vec<InputId> {
    let mut inputs = BTreeSet::new();
    if let Some(programs) = loaded.programs_for_level(level_index) {
        for program in programs {
            collect_inputs(program.as_steps(), &mut inputs);
        }
    }
    let mut inputs = if inputs.is_empty() {
        loaded
            .inputs
            .iter()
            .map(|input| input.id)
            .collect::<Vec<_>>()
    } else {
        inputs.into_iter().collect()
    };
    inputs.retain(|input| {
        let name = loaded
            .input_labels
            .get(input)
            .map(String::as_str)
            .or_else(|| {
                loaded
                    .inputs
                    .iter()
                    .find(|candidate| candidate.id == *input)
                    .map(|candidate| candidate.name.as_str())
            });
        !matches!(
            name,
            Some("undo" | "restart" | "next_level" | "previous_level")
        )
    });
    inputs.sort();
    inputs
}

fn logical_model_and_state_slicer<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    level_index: usize,
    initial: &GridState<D, Size>,
) -> Result<(LoadedGridGame<D, Size>, puzzle_solver::SolverStateSlicer), String> {
    let mut roots = solver_strategy_object_roots(loaded);
    if let Some(goal) = &loaded.goal {
        puzzle_solver::object_refs::collect_goal_expr_roots(&loaded.game, &goal.expr, &mut roots);
    }
    if let Some(lose) = &loaded.lose {
        puzzle_solver::object_refs::collect_goal_expr_roots(&loaded.game, &lose.expr, &mut roots);
    }
    let slice =
        puzzle_solver::SolverSlice::from_loaded_level_roots(loaded, level_index, [initial], roots)
            .ok_or_else(|| format!("solver level index out of range: {level_index}"))?;
    let state_slicer = puzzle_solver::SolverStateSlicer::<ObjectId>::from_kept_objects(
        &loaded.game,
        slice.kept_objects(),
    );
    let logical = slice.project_loaded_game(loaded, &state_slicer);
    Ok((logical, state_slicer))
}

fn collect_inputs<const D: usize>(program: &[GridRuleStep<D>], inputs: &mut BTreeSet<InputId>) {
    for step in program {
        match step {
            GridRuleStep::Rule(rule) => {
                collect_guard_inputs(&rule.guards, inputs);
            }
            GridRuleStep::ConditionalBlock { condition, steps } => {
                collect_condition_inputs(condition, inputs);
                collect_inputs(steps, inputs);
            }
            GridRuleStep::ConditionalBranch {
                condition,
                then_steps,
                else_steps,
            } => {
                collect_condition_inputs(condition, inputs);
                collect_inputs(then_steps, inputs);
                collect_inputs(else_steps, inputs);
            }
            GridRuleStep::Block {
                stop_condition,
                steps,
                ..
            } => {
                if let Some(condition) = stop_condition {
                    collect_condition_inputs(condition, inputs);
                }
                collect_inputs(steps, inputs);
            }
            GridRuleStep::LocalFrame { steps, .. } => collect_inputs(steps, inputs),
            GridRuleStep::AfterTriggered { steps, then_steps } => {
                collect_inputs(steps, inputs);
                collect_inputs(then_steps, inputs);
            }
        }
    }
}

fn collect_condition_inputs<const D: usize>(
    condition: &GridRuleCondition<D>,
    inputs: &mut BTreeSet<InputId>,
) {
    match condition {
        GridRuleCondition::AnyInputMatches(matches)
        | GridRuleCondition::NoInputMatches(matches) => {
            inputs.extend(matches.iter().map(|(input, _)| *input));
        }
        GridRuleCondition::GuardBranches(branches) => {
            for branch in branches {
                collect_guard_inputs(branch, inputs);
            }
        }
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
        GridRuleCondition::RuleMatches { guards, .. } => collect_guard_inputs(guards, inputs),
=======
        GridRuleCondition::RuleMatches { guards, .. } => {
            for guard in guards {
                if let GridGuard::InputIs(input) = guard {
                    inputs.insert(*input);
                }
            }
        }
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
        GridRuleCondition::AnyMatches(_) | GridRuleCondition::NoMatches(_) => {}
    }
}

fn collect_guard_inputs<const D: usize>(guards: &[GridGuard<D>], inputs: &mut BTreeSet<InputId>) {
    for guard in guards {
        if let GridGuard::InputIs(input) = guard {
            inputs.insert(*input);
        }
    }
}

fn workspace_fingerprint(entry_path: &str, documents: &[WorkspaceSourceDocument]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    hash_bytes(&mut hash, b"solver-artifact-v3\0");
    hash_bytes(&mut hash, entry_path.as_bytes());
    for document in documents {
        hash_bytes(&mut hash, b"\0path\0");
        hash_bytes(&mut hash, document.path.as_bytes());
        hash_bytes(&mut hash, b"\0source\0");
        hash_bytes(&mut hash, document.source.as_bytes());
    }
    format!("{hash:016x}")
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    fn search_request_requires_a_positive_stored_node_limit() {
        let mut service = SolverService::new();
        let error = service
            .start(
                "missing",
                SolverSearchRequest {
                    level_index: 0,
                    state: SolverStateSnapshot::TwoD {
                        width: 1,
                        height: 1,
                        layer_count: 1,
                        slots: vec![0],
                        variables: Vec::new(),
                        level_fired_rules: Vec::new(),
                    },
                    materialize_level_start: false,
                    max_depth: 8,
                    max_stored_nodes: 0,
                },
                0,
            )
            .unwrap_err();

        assert_eq!(error, "solver maxDepth and maxStoredNodes must be positive");
    }

    #[test]
    fn rule_matches_guard_inputs_are_solver_inputs() {
        let condition = GridRuleCondition::<2>::RuleMatches {
            guards: vec![GridGuard::InputIs(InputId(7))],
=======
    fn rule_match_condition_contributes_guard_inputs() {
        let expected = InputId(7);
        let condition = GridRuleCondition::<2>::RuleMatches {
            guards: vec![GridGuard::InputIs(expected)],
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
            pattern: puzzle_core::GridPattern::from_components(Vec::new()),
        };
        let mut inputs = BTreeSet::new();

        collect_condition_inputs(&condition, &mut inputs);

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
        assert_eq!(inputs, BTreeSet::from([InputId(7)]));
=======
        assert_eq!(inputs, BTreeSet::from([expected]));
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    }

    #[test]
    fn prepared_artifact_owns_compiled_rules_and_search_state() {
        let source = r#"
const title = solver_push_goal

puzzle default {
layers {
floor = Goal
actor = Player Box Wall
}

keys {
d ArrowRight -> right
}

rules {
input right [ Player | Box | no actor ] -> [ | Player | Box ]
input right [ Player | no actor ] -> [ | Player ]
}

win_conditions {
all Goal on Box
}
}

levels tiny of default {
legend {
. = empty
P = Player
B = Box
G = Goal
}
level "start" {
PBG
}
}
"#;
        let mut service = SolverService::new();
        let document = puzzle_lang::parse_game_for_path(source, "game.puzzle").unwrap();
        let loaded = loaded_document_scene_host_loaded_game(&document).unwrap();
        let initial_state = SolverStateSnapshot::from_state2(&loaded.levels[0].initial_state);
        let prepared = service
            .prepare_workspace(
                "game.puzzle",
                vec![WorkspaceSourceDocument {
                    path: "game.puzzle".to_string(),
                    source: source.to_string(),
                }],
                0,
            )
            .unwrap();
        assert_eq!(prepared.model_kind, RuntimeModelKind::TwoD);
        let search = service
            .start(
                &prepared.artifact_id,
                SolverSearchRequest {
                    level_index: 0,
                    state: initial_state,
                    materialize_level_start: true,
                    max_depth: 8,
                    max_stored_nodes: 32,
                },
                0,
            )
            .unwrap();
        let response = service.advance_nodes(search, 4, 0).unwrap();
        assert_eq!(response.status, SolverSearchStatus::Solved);
    }

    #[test]
    fn workspace_paths_are_required_contract_fields() {
        let mut service = SolverService::new();
        let document = WorkspaceSourceDocument {
            path: "game.puzzle".to_string(),
            source: "const title = empty".to_string(),
        };
        assert_eq!(
            service.prepare_workspace("", vec![document.clone()], 0),
            Err("solver workspace requires an explicit entry path".to_string())
        );
        assert_eq!(
            service.prepare_workspace(
                "game.puzzle",
                vec![WorkspaceSourceDocument {
                    path: String::new(),
                    source: document.source,
                }],
                0,
            ),
            Err("solver workspace document paths must be explicit".to_string())
        );
    }

    #[test]
    fn search_nodes_are_logical_and_progress_materialization_is_real() {
        let source = r#"
const title = logical_search_real_observation

puzzle default {
layers {
floor = Goal
actor = Player
decor = Dust
}

keys {
d ArrowRight -> right
}

rules {
input right [ Player | no actor ] -> [ | Player ]
}

win_conditions {
all Goal on Player
}
}

levels tiny of default {
legend {
. = empty
P = Player
D = Dust
G = Goal
}
level "start" {
P.....DG
}
}
"#;
        let document = puzzle_lang::parse_game_for_path(source, "game.puzzle").unwrap();
        let loaded = loaded_document_scene_host_loaded_game(&document).unwrap();
        let dust = loaded
            .object_labels
            .iter()
            .find_map(|(id, name)| (name == "Dust").then_some(*id))
            .unwrap();
        let initial_state = SolverStateSnapshot::from_state2(&loaded.levels[0].initial_state);
        let mut service = SolverService::new();
        let prepared = service
            .prepare_workspace(
                "game.puzzle",
                vec![WorkspaceSourceDocument {
                    path: "game.puzzle".to_string(),
                    source: source.to_string(),
                }],
                0,
            )
            .unwrap();
        let search_id = service
            .start(
                &prepared.artifact_id,
                SolverSearchRequest {
                    level_index: 0,
                    state: initial_state,
                    materialize_level_start: true,
                    max_depth: 8,
                    max_stored_nodes: 32,
                },
                0,
            )
            .unwrap();

        let entry = service.searches.get_mut(&search_id).unwrap();
        let ActiveSearch::TwoD(search) = &mut entry.search else {
            panic!("fixture must compile as a 2D search");
        };
        let response = advance_grid_search(
            search,
            1,
            Some(Duration::from_millis(1_000)),
            RuntimeModelKind::TwoD,
            SolverStateSnapshot::from_state2,
        )
        .unwrap();
        assert!(matches!(
            response.status,
            SolverSearchStatus::Paused | SolverSearchStatus::Solved
        ));
        assert!(
            search
                .machine
                .best_candidates(8)
                .iter()
                .all(|candidate| !candidate.state.state().slots().contains(&dust)),
            "logical search nodes must not retain solver-irrelevant objects"
        );
        let observation = response
            .observation
            .expect("paused search has a best candidate");
        let SolverStateSnapshot::TwoD { slots, .. } = observation.state else {
            panic!("fixture observation must be 2D");
        };
        assert!(
            slots.contains(&dust.0),
            "editor observation must materialize the selected witness in the real game"
        );
    }
}
