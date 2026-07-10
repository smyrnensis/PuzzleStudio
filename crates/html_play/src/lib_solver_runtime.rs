#[derive(Clone, Copy, Debug)]
struct SolverConfig {
    #[cfg(feature = "solver")]
    max_depth: u32,
    #[cfg(feature = "solver")]
    max_nodes: usize,
    #[cfg(feature = "solver")]
    max_duration: Duration,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            #[cfg(feature = "solver")]
            max_depth: 128,
            #[cfg(feature = "solver")]
            max_nodes: 1_000_000,
            #[cfg(feature = "solver")]
            max_duration: Duration::from_secs(5),
        }
    }
}

#[cfg(feature = "solver")]
impl SolverConfig {
    fn budget(self) -> SearchBudget {
        SearchBudget::bounded(self.max_depth, self.max_nodes, self.max_duration)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_arg<T>(args: &mut impl Iterator<Item = String>, name: &str) -> Result<T, AppError>
where
    T: std::str::FromStr,
{
    let Some(value) = args.next() else {
        return Err(AppError::Config(format!("{name} requires a value")));
    };
    value
        .parse()
        .map_err(|_| AppError::Config(format!("{name} has an invalid value")))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "solver"))]
fn parse_solver_depth_arg(
    solver: &mut SolverConfig,
    args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    solver.max_depth = parse_arg(args, "--solver-depth")?;
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "solver")))]
fn parse_solver_depth_arg(
    _solver: &mut SolverConfig,
    _args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    Err(AppError::Config(
        "--solver-depth requires the html-play solver feature".to_string(),
    ))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "solver"))]
fn parse_solver_nodes_arg(
    solver: &mut SolverConfig,
    args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    solver.max_nodes = parse_arg(args, "--solver-nodes")?;
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "solver")))]
fn parse_solver_nodes_arg(
    _solver: &mut SolverConfig,
    _args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    Err(AppError::Config(
        "--solver-nodes requires the html-play solver feature".to_string(),
    ))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "solver"))]
fn parse_solver_ms_arg(
    solver: &mut SolverConfig,
    args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    let milliseconds: u64 = parse_arg(args, "--solver-ms")?;
    solver.max_duration = Duration::from_millis(milliseconds);
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "solver")))]
fn parse_solver_ms_arg(
    _solver: &mut SolverConfig,
    _args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    Err(AppError::Config(
        "--solver-ms requires the html-play solver feature".to_string(),
    ))
}

struct ServerState {
    loaded: LoadedGame,
    session: GameSession,
    source: String,
    puzzle_path: String,
    game_css: String,
    game_visuals_js: String,
    solver: SolverConfig,
    has_progress_save: bool,
}

impl ServerState {
    fn new(
        loaded: LoadedGame,
        source: String,
        puzzle_path: String,
        game_css: String,
        game_visuals_js: String,
        solver: SolverConfig,
    ) -> Self {
        let session = GameSession::new(&loaded);
        Self {
            loaded,
            session,
            source,
            puzzle_path,
            game_css,
            game_visuals_js,
            solver,
            has_progress_save: false,
        }
    }

    fn snapshot_json(&mut self) -> String {
        let sound_events = self.session.take_sound_events();
        let message_events = self.session.take_message_events();
        let wait_events = self.session.take_wait_events();
        let animation_events = self.session.take_animation_events();
        let mut out = String::new();
        out.push('{');
        push_top_scope_context(&mut out, &self.loaded, self.has_progress_save);
        out.push(',');
        push_export_sounds(&mut out, &self.loaded.sounds);
        out.push(',');
        push_export_theme(&mut out, &self.loaded.theme);
        out.push(',');
        push_json_number(&mut out, "defaultWaitMs", self.loaded.default_wait_ms);
        out.push(',');
        push_json_number(&mut out, "defaultAgainMs", self.loaded.default_again_ms);
        out.push(',');
        push_export_input_buffer(&mut out, &self.loaded);
        out.push(',');
        push_export_animation(&mut out, &self.loaded);
        out.push(',');
        push_sound_events(&mut out, &sound_events);
        out.push(',');
        push_message_events(&mut out, &message_events);
        out.push(',');
        push_wait_events(&mut out, &wait_events);
        out.push(',');
        push_animation_events(&mut out, &self.loaded, &animation_events);
        out.push(',');
        push_level_context(
            &mut out,
            &self.loaded,
            self.session.cleared_levels(),
            self.session.active_level_index(),
        );
        out.push(',');
        out.push_str("\"levelIndex\":");
        if let Some(level_index) = self.session.active_level_index() {
            out.push_str(&(level_index as u64).to_string());
        } else {
            out.push_str("null");
        }
        out.push(',');
        push_json_number(&mut out, "levelCount", self.loaded.levels.len() as u64);
        out.push(',');
        push_json_pair(&mut out, "scene", self.session.scene());
        out.push(',');
        push_json_pair(&mut out, "currentScene", self.session.scene());
        out.push(',');
        push_json_pair(&mut out, "focusedScreen", self.session.focused_scene());
        out.push(',');
        push_json_pair(&mut out, "focusedScene", self.session.focused_scene());
        out.push(',');
        push_json_bool(
            &mut out,
            "acceptsModelInput",
            self.session.accepts_model_input(&self.loaded),
        );
        out.push(',');
        push_visible_scenes(&mut out, self.session.visible_scenes());
        out.push(',');
        push_session_state(&mut out, &self.loaded, &self.session);
        out.push(',');
        push_scene_state(&mut out, &self.loaded, &self.session);
        out.push(',');
        push_scene_puzzles(&mut out, self.session.scene_state());
        out.push(',');
        push_scene_puzzle_state(&mut out, &self.loaded, &self.session);
        out.push(',');
        push_json_number(
            &mut out,
            "selectedLevelIndex",
            self.session.selected_level_index() as u64,
        );
        out.push(',');
        push_json_bool(&mut out, "canUndo", self.session.can_undo());
        out.push(',');
        push_json_bool(&mut out, "canRedo", self.session.can_redo());
        out.push(',');
        let scene_state = focused_scene_state(&self.loaded, &self.session);
        let focused_scene = self
            .loaded
            .scenes
            .iter()
            .find(|scene| scene.name == self.session.focused_scene());
        if let Some(scene_state) = scene_state {
            push_scene(
                &mut out,
                &self.loaded,
                scene_state,
                Some(self.session.current_level(&self.loaded)),
                focused_scene.map(|scene| &scene.resources),
            );
        } else if self.loaded.scenes.is_empty() {
            push_scene(
                &mut out,
                &self.loaded,
                self.session.state(),
                Some(self.session.current_level(&self.loaded)),
                focused_scene.map(|scene| &scene.resources),
            );
        } else {
            out.push_str("\"scene\":null");
        }
        out.push(',');
        push_scene_layers(&mut out, &self.loaded, &self.session);
        out.push(',');
        push_inputs(&mut out, &self.loaded);
        out.push(',');
        push_levels(&mut out, &self.loaded, self.session.cleared_levels());
        out.push(',');
        push_scenes(&mut out, "scenes", &self.loaded);
        out.push(',');
        push_scenes(&mut out, "screens", &self.loaded);
        out.push('}');
        out
    }

    fn scene_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        let scene_state = focused_scene_state(&self.loaded, &self.session);
        let focused_scene = self
            .loaded
            .scenes
            .iter()
            .find(|scene| scene.name == self.session.focused_scene());
        if let Some(scene_state) = scene_state {
            push_scene(
                &mut out,
                &self.loaded,
                scene_state,
                Some(self.session.current_level(&self.loaded)),
                focused_scene.map(|scene| &scene.resources),
            );
        } else if self.loaded.scenes.is_empty() {
            push_scene(
                &mut out,
                &self.loaded,
                self.session.state(),
                Some(self.session.current_level(&self.loaded)),
                focused_scene.map(|scene| &scene.resources),
            );
        } else {
            out.push_str("\"scene\":null");
        }
        out.push('}');
        out
    }

    fn apply_input_name(&mut self, input_name: &str) -> Result<(), AppError> {
        let input = input_id_by_name(&self.loaded, input_name)
            .ok_or_else(|| AppError::Config(format!("unknown input: {input_name}")))?;
        self.session.apply_input(&self.loaded, input)?;
        Ok(())
    }

    fn apply_debug_input_name_json(&mut self, input_name: &str) -> Result<String, AppError> {
        self.apply_input_name(input_name)?;
        let debug = self.session.last_debug_transition().cloned();
        let snapshot = self.snapshot_json();
        let mut out = String::new();
        out.push('{');
        out.push_str("\"snapshot\":");
        out.push_str(&snapshot);
        out.push_str(",\"debug\":");
        out.push_str(
            &puzzle_game_runtime::debug_transition_value(&self.loaded, debug.as_ref()).to_string(),
        );
        out.push('}');
        Ok(out)
    }

    fn apply_command_name(&mut self, command_name: &str) -> Result<(), AppError> {
        self.session.apply_command(&self.loaded, command_name)?;
        Ok(())
    }

    fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), AppError> {
        if level_index >= self.loaded.levels.len() {
            return Err(AppError::Config(format!(
                "level index out of range: {level_index}"
            )));
        }
        let state = state_from_json(&self.loaded, state_json)?;
        self.session
            .start_level_from_state(&self.loaded, level_index, state, materialize_level_start)
            .map_err(AppError::CoreTransition)
    }

    fn progress_save_json(&self) -> String {
        let save = self.session.progress_save_data(&self.loaded);
        let mut out = String::new();
        push_progress_save_data(&mut out, &save);
        out
    }

    fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), AppError> {
        let save = progress_save_data_from_json(save_json).map_err(AppError::Config)?;
        self.session
            .restore_progress_save_data(&self.loaded, &save)
            .map_err(|error| AppError::Config(format!("{error:?}")))?;
        self.has_progress_save = true;
        Ok(())
    }

    #[cfg(feature = "solver")]
    fn solve_json(&self) -> Result<String, AppError> {
        let response =
            solve_current_state(&self.loaded, self.session.state().clone(), self.solver)?;
        let mut out = String::new();
        push_solution_response(&mut out, &self.loaded, &response);
        Ok(out)
    }
}

pub struct StandaloneSessionBridge {
    state: ServerState,
}

impl StandaloneSessionBridge {
    pub fn from_source(source: &str, puzzle_path: &str) -> Result<Self, String> {
        let document = puzzle_lang::parse_game_for_path(source, puzzle_path)
            .map_err(|error| error.to_string())?;
        let loaded = loaded_document_scene_host_loaded_game(&document)?;
        Ok(Self {
            state: ServerState::new(
                loaded,
                source.to_string(),
                puzzle_path.to_string(),
                String::new(),
                String::new(),
                SolverConfig::default(),
            ),
        })
    }

    pub fn snapshot_json(&mut self) -> String {
        self.state.snapshot_json()
    }

    pub fn request_json(&mut self, method: &str, url: &str) -> Result<String, String> {
        match puzzle_game_runtime::standalone_session_request(method, url)? {
            puzzle_game_runtime::StandaloneSessionRequest::State => Ok(self.snapshot_json()),
            puzzle_game_runtime::StandaloneSessionRequest::Undo => {
                self.state.session.undo(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            puzzle_game_runtime::StandaloneSessionRequest::Redo => {
                self.state.session.redo(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            puzzle_game_runtime::StandaloneSessionRequest::Restart => {
                self.state.session.restart_level(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            puzzle_game_runtime::StandaloneSessionRequest::Next => {
                self.state.session.advance_level(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            puzzle_game_runtime::StandaloneSessionRequest::Input(input_name) => {
                self.state
                    .apply_input_name(&input_name)
                    .map_err(|error| error.to_string())?;
                Ok(self.snapshot_json())
            }
            puzzle_game_runtime::StandaloneSessionRequest::DebugInput(input_name) => self
                .state
                .apply_debug_input_name_json(&input_name)
                .map_err(|error| error.to_string()),
            puzzle_game_runtime::StandaloneSessionRequest::Command(command_name) => {
                self.state
                    .apply_command_name(&command_name)
                    .map_err(|error| error.to_string())?;
                Ok(self.snapshot_json())
            }
        }
    }

    pub fn apply_input_name(&mut self, input_name: &str) -> Result<(), String> {
        self.state
            .apply_input_name(input_name)
            .map_err(|error| error.to_string())
    }

    pub fn apply_debug_input_name_json(&mut self, input_name: &str) -> Result<String, String> {
        self.state
            .apply_debug_input_name_json(input_name)
            .map_err(|error| error.to_string())
    }

    pub fn apply_command_name(&mut self, command_name: &str) -> Result<(), String> {
        self.state
            .apply_command_name(command_name)
            .map_err(|error| error.to_string())
    }

    pub fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String> {
        self.state
            .set_current_state_json(state_json, level_index, materialize_level_start)
            .map_err(|error| error.to_string())
    }

    pub fn progress_save_json(&self) -> String {
        self.state.progress_save_json()
    }

    pub fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String> {
        self.state
            .restore_progress_save_json(save_json)
            .map_err(|error| error.to_string())
    }

    pub fn mark_progress_save_written(&mut self) {
        self.state.has_progress_save = true;
    }

    pub fn clear_progress_save(&mut self) {
        self.state.has_progress_save = false;
    }
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
struct SolutionStep<State, Input> {
    index: usize,
    input: Option<Input>,
    state: State,
    completed: bool,
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
struct SearchObservation<State> {
    progress: SearchProgress,
    state: State,
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
struct SearchObservationSampler<State> {
    observations: Vec<(SearchProgress, State)>,
    max_samples: usize,
    next_expanded: usize,
    stride: usize,
}

#[cfg(feature = "solver")]
impl<State: Clone> SearchObservationSampler<State> {
    fn new(max_samples: usize) -> Self {
        Self {
            observations: Vec::new(),
            max_samples: max_samples.max(1),
            next_expanded: 0,
            stride: 1,
        }
    }

    fn wants(&self, progress: SearchProgress) -> bool {
        progress.expanded >= self.next_expanded
    }

    fn observe(&mut self, state: &State, progress: SearchProgress) -> bool {
        if !self.wants(progress) {
            return false;
        }
        self.record(state, progress);
        true
    }

    fn record(&mut self, state: &State, progress: SearchProgress) {
        self.observations.push((progress, state.clone()));
        self.next_expanded = progress.expanded.saturating_add(self.stride);
        if self.observations.len() > self.max_samples {
            self.observations = self
                .observations
                .iter()
                .step_by(2)
                .cloned()
                .collect::<Vec<_>>();
            self.stride = self.stride.saturating_mul(2).max(1);
            self.next_expanded = progress.expanded.saturating_add(self.stride);
        }
    }
}

#[cfg(feature = "solver")]
impl<State> SearchObservationSampler<State> {
    fn into_observations(self) -> Vec<SearchObservation<State>> {
        self.observations
            .into_iter()
            .map(|(progress, state)| SearchObservation { progress, state })
            .collect()
    }
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
enum SolutionResponse<State, Input> {
    Solved {
        depth: u32,
        moves: Vec<Input>,
        steps: Vec<SolutionStep<State, Input>>,
        observations: Vec<SearchObservation<State>>,
    },
    Exhausted {
        stats: SearchStats,
        observations: Vec<SearchObservation<State>>,
    },
    BudgetExceeded {
        stats: SearchStats,
        observations: Vec<SearchObservation<State>>,
    },
    Failed {
        depth: u32,
        error: String,
        observations: Vec<SearchObservation<State>>,
    },
}

#[cfg(feature = "solver")]
type PuzzleSolutionResponse = SolutionResponse<State, InputId>;
#[cfg(feature = "solver")]
type PuzzleSolutionStep = SolutionStep<State, InputId>;
#[cfg(feature = "solver")]
type PuzzleSearchObservation = SearchObservation<State>;
#[cfg(feature = "solver")]
type Puzzle3SolutionResponse = SolutionResponse<State3, InputId>;
#[cfg(feature = "solver")]
type Puzzle3SolutionStep = SolutionStep<State3, InputId>;
#[cfg(feature = "solver")]
type Puzzle3SearchObservation = SearchObservation<State3>;

#[cfg(feature = "solver")]
fn solve_domain_with_observations<D, ObservationState, Score, IsDead, Observe, OnProgress, Steps>(
    domain: &mut D,
    initial: D::State,
    budget: SearchBudget,
    score: Score,
    is_dead: IsDead,
    mut observe: Observe,
    mut on_progress: Option<OnProgress>,
    steps: Steps,
) -> Result<SolutionResponse<ObservationState, D::Action>, AppError>
where
    D: puzzle_solver::SearchDomain,
    D::Action: Clone,
    D::Error: std::fmt::Debug,
    ObservationState: Clone,
    Score: FnMut(&D::State) -> i64,
    IsDead: FnMut(&D::State) -> bool,
    Observe: FnMut(&D::State) -> ObservationState,
    OnProgress: FnMut(&ObservationState, SearchProgress),
    Steps: FnOnce(&[D::Action]) -> Result<Vec<SolutionStep<ObservationState, D::Action>>, AppError>,
{
    let mut observations = SearchObservationSampler::new(96);
    let outcome = best_first_with_dead_states_and_progress(
        domain,
        initial,
        budget,
        score,
        is_dead,
        |state, progress| {
            let observation = observe(state);
            observations.observe(&observation, progress);
            if let Some(on_progress) = on_progress.as_mut() {
                on_progress(&observation, progress);
            }
        },
    );
    let observations = observations.into_observations();

    match outcome {
        SearchOutcome::Solved(witness) => {
            let steps = steps(&witness.actions)?;
            Ok(SolutionResponse::Solved {
                depth: witness.depth,
                moves: witness.actions,
                steps,
                observations,
            })
        }
        SearchOutcome::Exhausted(stats) => Ok(SolutionResponse::Exhausted {
            stats,
            observations,
        }),
        SearchOutcome::BudgetExceeded(stats) => Ok(SolutionResponse::BudgetExceeded {
            stats,
            observations,
        }),
        SearchOutcome::Failed(failure) => Ok(SolutionResponse::Failed {
            depth: failure.depth,
            error: format!("{:?}", failure.error),
            observations,
        }),
    }
}

#[cfg(feature = "solver")]
fn solve_current_state(
    loaded: &LoadedGame,
    initial: State,
    solver: SolverConfig,
) -> Result<PuzzleSolutionResponse, AppError> {
    solve_current_state_with_budget(loaded, initial, solver.budget())
}

#[cfg(feature = "solver")]
fn solve_current_state_with_budget(
    loaded: &LoadedGame,
    initial: State,
    budget: SearchBudget,
) -> Result<PuzzleSolutionResponse, AppError> {
    solve_current_state_with_budget_inner(
        loaded,
        initial,
        budget,
        None::<fn(&State, SearchProgress)>,
    )
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
enum SolverGoal2 {
    BuiltIn,
    Expr(GoalExpr),
    ExactState(State),
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
enum SolverCollectSelector2 {
    Predicate(GoalExpr),
    Maximize(GoalValue),
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
struct CollectMatch<State, Input> {
    depth: u32,
    score: Option<i64>,
    moves: Vec<Input>,
    state: State,
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
enum CollectResponse<State, Input> {
    Completed {
        stats: SearchStats,
        matches: Vec<CollectMatch<State, Input>>,
        observations: Vec<SearchObservation<State>>,
    },
    LimitReached {
        stats: SearchStats,
        matches: Vec<CollectMatch<State, Input>>,
        observations: Vec<SearchObservation<State>>,
    },
    BudgetExceeded {
        stats: SearchStats,
        matches: Vec<CollectMatch<State, Input>>,
        observations: Vec<SearchObservation<State>>,
    },
    Failed {
        depth: u32,
        error: String,
        matches: Vec<CollectMatch<State, Input>>,
        observations: Vec<SearchObservation<State>>,
    },
}

#[cfg(feature = "solver")]
type PuzzleCollectResponse = CollectResponse<State, InputId>;

#[cfg(feature = "solver")]
fn solver_game_and_state_slicer_for_compiled(
    game: CompiledGame,
    initial: &State,
    goal: Option<&GoalExpr>,
    lose: Option<&GoalExpr>,
) -> (CompiledGame, puzzle_solver::SolverStateSlicer) {
    let mut roots = BTreeSet::new();
    if let Some(goal) = goal {
        collect_goal_expr_roots(&game, goal, &mut roots);
    }
    if let Some(lose) = lose {
        collect_goal_expr_roots(&game, lose, &mut roots);
    }
    let relevance = puzzle_solver::SolverRelevance::from_root_objects(&game, roots);
    if goal.is_none() {
        let state_slicer =
            puzzle_solver::SolverStateSlicer::<ObjectId>::from_relevance(&game, &relevance);
        return (game, state_slicer);
    }
    let availability = puzzle_solver::SolverStageAvailability::from_initial_state(&game, initial);
    let slice =
        puzzle_solver::SolverSlice::from_relevance_and_availability(&relevance, &availability);
    let state_slicer =
        puzzle_solver::SolverStateSlicer::<ObjectId>::from_kept_objects(&game, slice.kept_objects());
    (slice.project_game(&game), state_slicer)
}

#[cfg(feature = "solver")]
fn solver_game_and_state_slicer_for_loaded(
    loaded: &LoadedGame,
    solver_game: CompiledGame,
    initial: &State,
    exact_goal: Option<&State>,
    explicit_goal: Option<&GoalExpr>,
    explicit_lose: Option<&GoalExpr>,
) -> (CompiledGame, puzzle_solver::SolverStateSlicer) {
    let mut roots = BTreeSet::new();
    if let Some(goal) = exact_goal {
        collect_state_objects(initial, &mut roots);
        collect_state_objects(goal, &mut roots);
    } else if let Some(goal) = explicit_goal {
        collect_goal_expr_roots(&solver_game, goal, &mut roots);
    } else if let Some(goal) = &loaded.goal {
        collect_goal_expr_roots(&solver_game, &goal.expr, &mut roots);
    }
    if let Some(lose) = explicit_lose {
        collect_goal_expr_roots(&solver_game, lose, &mut roots);
    } else if let Some(lose) = &loaded.lose {
        collect_goal_expr_roots(&solver_game, &lose.expr, &mut roots);
    }
    for query in loaded
        .solver_strategy
        .terms
        .iter()
        .map(|term| &term.value)
        .chain(
            loaded
                .solver_strategy
                .deadends
                .iter()
                .flat_map(|deadend| deadend.values()),
        )
    {
        collect_query_expr_roots(query, &mut roots, &mut |kind, roots| {
            puzzle_solver::object_refs::collect_condition_value_roots(kind, roots)
        });
    }
    let relevance = puzzle_solver::SolverRelevance::from_root_objects(&solver_game, roots);
    let availability = puzzle_solver::SolverStageAvailability::from_initial_state(
        &solver_game,
        initial,
    );
    let slice = puzzle_solver::SolverSlice::from_relevance_and_availability(
        &relevance,
        &availability,
    );
    let state_slicer = puzzle_solver::SolverStateSlicer::<ObjectId>::from_kept_objects(
        &solver_game,
        slice.kept_objects(),
    );
    (slice.project_game(&solver_game), state_slicer)
}

#[cfg(feature = "solver")]
fn solver_game_and_state_slicer_for_collect(
    loaded: &LoadedGame,
    solver_game: CompiledGame,
    initial: &State,
    selector: &SolverCollectSelector2,
    explicit_lose: Option<&GoalExpr>,
) -> (CompiledGame, puzzle_solver::SolverStateSlicer) {
    let mut roots = BTreeSet::new();
    match selector {
        SolverCollectSelector2::Predicate(predicate) => {
            collect_goal_expr_roots(&solver_game, predicate, &mut roots);
        }
        SolverCollectSelector2::Maximize(value) => {
            collect_goal_value_roots(&solver_game, value, &mut roots);
        }
    }
    if let Some(lose) = explicit_lose {
        collect_goal_expr_roots(&solver_game, lose, &mut roots);
    } else if let Some(lose) = &loaded.lose {
        collect_goal_expr_roots(&solver_game, &lose.expr, &mut roots);
    }
    for query in loaded
        .solver_strategy
        .terms
        .iter()
        .map(|term| &term.value)
        .chain(
            loaded
                .solver_strategy
                .deadends
                .iter()
                .flat_map(|deadend| deadend.values()),
        )
    {
        collect_query_expr_roots(query, &mut roots, &mut |kind, roots| {
            puzzle_solver::object_refs::collect_condition_value_roots(kind, roots)
        });
    }
    let relevance = puzzle_solver::SolverRelevance::from_root_objects(&solver_game, roots);
    let availability =
        puzzle_solver::SolverStageAvailability::from_initial_state(&solver_game, initial);
    let slice =
        puzzle_solver::SolverSlice::from_relevance_and_availability(&relevance, &availability);
    let state_slicer = puzzle_solver::SolverStateSlicer::<ObjectId>::from_kept_objects(
        &solver_game,
        slice.kept_objects(),
    );
    (slice.project_game(&solver_game), state_slicer)
}

#[cfg(feature = "solver")]
fn collect_state_objects(state: &State, roots: &mut BTreeSet<ObjectId>) {
    for object in state.slots() {
        if !object.is_empty() {
            roots.insert(*object);
        }
    }
}

#[cfg(feature = "solver")]
fn solver_state_slicer_for_puzzle3(
    parsed: &ParsedPuzzle3,
) -> puzzle_solver::SolverStateSlicer<ObjectId3> {
    let mut roots = BTreeSet::new();
    if let Some(win_condition) = &parsed.win_condition {
        collect_win_condition3_roots(win_condition, &mut roots);
    }
    for query in parsed
        .solver_strategy
        .terms
        .iter()
        .map(|term| &term.value)
        .chain(
            parsed
                .solver_strategy
                .deadends
                .iter()
                .flat_map(|deadend| deadend.values()),
        )
    {
        collect_query_expr_roots(query, &mut roots, &mut |kind, roots| {
            puzzle_solver::object_refs::collect_condition_value_roots(kind, roots)
        });
    }
    let relevance = puzzle_solver::SolverRelevance::<ObjectId3>::from_game3_root_objects(
        &parsed.game,
        &parsed.rules,
        roots,
    );
    puzzle_solver::SolverStateSlicer::<ObjectId3>::from_relevance(&parsed.game, &relevance)
}

#[cfg(feature = "solver")]
fn collect_goal_expr_roots(
    game: &CompiledGame,
    expr: &GoalExpr,
    roots: &mut BTreeSet<ObjectId>,
) {
    match expr {
        GoalExpr::All(exprs) | GoalExpr::Any(exprs) => {
            for expr in exprs {
                collect_goal_expr_roots(game, expr, roots);
            }
        }
        GoalExpr::Clause(clause) => collect_goal_value_roots(game, &clause.value, roots),
    }
}

#[cfg(feature = "solver")]
fn collect_goal_value_roots(
    game: &CompiledGame,
    value: &GoalValue,
    roots: &mut BTreeSet<ObjectId>,
) {
    match value {
        GoalValue::Variable(_) => {}
        GoalValue::Condition(condition) => {
            if let Some(condition) = game.condition_def(*condition) {
                puzzle_solver::object_refs::collect_condition_value_roots(&condition.kind, roots);
            }
        }
        GoalValue::InlineConditionValue(kind) => {
            puzzle_solver::object_refs::collect_condition_value_roots(kind, roots);
        }
        GoalValue::AllObjectsOn { subjects, covers } => {
            roots.extend(subjects.iter().copied());
            roots.extend(covers.iter().copied());
        }
    }
}

#[cfg(feature = "solver")]
fn collect_query_expr_roots<Object, Value, Variable>(
    value: &QueryExprOf<Object, Value, Variable>,
    roots: &mut BTreeSet<Object>,
    collect_value: &mut impl FnMut(&Value, &mut BTreeSet<Object>),
) where
    Object: Copy + Ord,
{
    match value {
        QueryExprOf::Variable(_) => {}
        QueryExprOf::Value(kind) => collect_value(kind, roots),
        QueryExprOf::Distance { from, to } => {
            roots.extend(from.iter().copied());
            roots.extend(to.iter().copied());
        }
        QueryExprOf::Compare { left, .. } => collect_query_expr_roots(left, roots, collect_value),
    }
}

#[cfg(feature = "solver")]
fn collect_win_condition3_roots(condition: &WinCondition3, roots: &mut BTreeSet<ObjectId3>) {
    match condition {
        WinCondition3::All(conditions) | WinCondition3::Any(conditions) => {
            for condition in conditions {
                collect_win_condition3_roots(condition, roots);
            }
        }
        WinCondition3::SomeObject(object) | WinCondition3::NoObject(object) => {
            if !object.is_empty() {
                roots.insert(*object);
            }
        }
        WinCondition3::SomePattern(pattern) | WinCondition3::NoPattern(pattern) => {
            puzzle_solver::object_refs::collect_pattern_roots(pattern, roots);
        }
        WinCondition3::AllObjectsCoveredByPattern {
            object,
            cover_pattern,
        } => {
            if !object.is_empty() {
                roots.insert(*object);
            }
            puzzle_solver::object_refs::collect_pattern_roots(cover_pattern, roots);
        }
    }
}

#[cfg(feature = "solver")]
fn solve_compiled_state_with_budget_and_progress<O>(
    engine: &puzzle_core_wasm::CompiledEngine,
    goal: Option<GoalExpr>,
    lose: Option<GoalExpr>,
    accept_win_command: bool,
    initial: State,
    budget: SearchBudget,
    mut on_observation: Option<O>,
) -> Result<PuzzleSolutionResponse, AppError>
where
    O: FnMut(&State, SearchProgress),
{
    let mut inputs = BTreeSet::new();
    let game = engine.game();
    collect_solver_inputs(game.program(), &mut inputs);
    let inputs = inputs.into_iter().collect::<Vec<_>>();
    if inputs.is_empty() {
        return Err(AppError::Config("no model inputs available".to_string()));
    }

    let (solver_game, state_slicer) = solver_game_and_state_slicer_for_compiled(
        game.clone(),
        &initial,
        goal.as_ref(),
        lose.as_ref(),
    );
    let game = Arc::new(solver_game);
    let goal_game = game.clone();
    let goal_for_domain = goal.clone();
    let mut domain = PuzzleDomain::with_state_slicer_and_win_command_goal(
        game.clone(),
        inputs,
        state_slicer,
        accept_win_command,
        move |state: &State| {
            goal_for_domain
                .as_ref()
                .is_some_and(|goal| eval_goal_expr(&goal_game, state, goal))
        },
    );
    let solver_initial = domain.initial_state(initial.clone());
    let display_initial = initial.clone();
    let score_game = game.clone();
    let score_goal = goal.clone();
    let lose_game = game.clone();
    let mut observations = SearchObservationSampler::new(96);
    let mut observation_error = None::<AppError>;
    let outcome = best_first_with_dead_states_and_progress(
        &mut domain,
        solver_initial,
        budget,
        move |state| {
            score_goal
                .as_ref()
                .map(|goal| goal_expr_score(&score_game, state.state(), goal))
                .unwrap_or(0)
        },
        move |state| {
            lose.as_ref()
                .is_some_and(|lose| eval_goal_expr(&lose_game, state.state(), lose))
        },
        |state, progress| {
            if observation_error.is_some() {
                return;
            }
            if observations.wants(progress) {
                match compiled_display_state_after_inputs(
                    engine,
                    &display_initial,
                    state.input_history(),
                ) {
                    Ok(display_state) => {
                        observations.record(&display_state, progress);
                        if let Some(on_observation) = on_observation.as_mut() {
                            on_observation(&display_state, progress);
                        }
                    }
                    Err(error) => {
                        observation_error = Some(error);
                    }
                }
            }
        },
    );
    if let Some(error) = observation_error {
        return Err(error);
    }
    let observations = observations.into_observations();

    let response = match outcome {
        SearchOutcome::Solved(witness) => {
            let depth = witness.depth;
            let solution_inputs = witness.actions;
            SolutionResponse::Solved {
                depth,
                steps: compiled_solution_steps(engine, initial, &solution_inputs)?,
                moves: solution_inputs,
                observations,
            }
        }
        SearchOutcome::Exhausted(stats) => SolutionResponse::Exhausted {
            stats,
            observations,
        },
        SearchOutcome::BudgetExceeded(stats) => SolutionResponse::BudgetExceeded {
            stats,
            observations,
        },
        SearchOutcome::Failed(failure) => SolutionResponse::Failed {
            depth: failure.depth,
            error: format!("{:?}", failure.error),
            observations,
        },
    };
    Ok(response)
}

#[cfg(feature = "solver")]
fn materialize_compiled_display_state(
    engine: &puzzle_core_wasm::CompiledEngine,
    state: &State,
) -> Result<State, AppError> {
    let Some(program) = engine.program("display", -1) else {
        return Ok(state.clone());
    };
    if program.is_empty() {
        return Ok(state.clone());
    }
    Ok(transition_program(
        engine.game(),
        program,
        state,
        InputId(0),
    )?)
}

#[cfg(feature = "solver")]
fn compiled_display_state_after_inputs(
    engine: &puzzle_core_wasm::CompiledEngine,
    initial: &State,
    inputs: &[InputId],
) -> Result<State, AppError> {
    let mut state = initial.clone();
    for input in inputs {
        state = transition_state(engine.game(), &state, *input)?;
    }
    materialize_compiled_display_state(engine, &state)
}

#[cfg(feature = "solver")]
fn solve_current_state_with_budget_inner<O>(
    loaded: &LoadedGame,
    initial: State,
    budget: SearchBudget,
    on_progress: Option<O>,
) -> Result<PuzzleSolutionResponse, AppError>
where
    O: FnMut(&State, SearchProgress),
{
    solve_current_state_with_goal_with_budget_inner(
        loaded,
        SolverGoal2::BuiltIn,
        None,
        true,
        initial,
        budget,
        on_progress,
    )
}

#[cfg(feature = "solver")]
fn solve_current_state_with_goal_with_budget_inner<O>(
    loaded: &LoadedGame,
    goal: SolverGoal2,
    lose: Option<GoalExpr>,
    accept_win_command: bool,
    initial: State,
    budget: SearchBudget,
    on_progress: Option<O>,
) -> Result<PuzzleSolutionResponse, AppError>
where
    O: FnMut(&State, SearchProgress),
{
    let inputs = solver_inputs(loaded);
    if inputs.is_empty() {
        return Err(AppError::Config("no model inputs available".to_string()));
    }

    let goal_game = loaded.clone();
    let explicit_goal = match &goal {
        SolverGoal2::BuiltIn => None,
        SolverGoal2::Expr(goal) => Some(goal),
        SolverGoal2::ExactState(_) => None,
    };
    let exact_goal = match &goal {
        SolverGoal2::ExactState(goal) => Some(goal),
        SolverGoal2::BuiltIn | SolverGoal2::Expr(_) => None,
    };
    let (solver_game, state_slicer) = solver_game_and_state_slicer_for_loaded(
        loaded,
        loaded.solver_game(),
        &initial,
        exact_goal,
        explicit_goal,
        lose.as_ref(),
    );
    let game = Arc::new(solver_game);
    let projected_initial = state_slicer.project_state(&initial);
    let projected_exact_goal = exact_goal.map(|goal| state_slicer.project_state(goal));
    let exact_heuristic = projected_exact_goal
        .as_ref()
        .map(|goal| ExactStateHeuristic::new(&game, &projected_initial, goal));
    let goal_for_domain = goal.clone();
    let goal_expr_game = game.clone();
    let mut domain = PuzzleDomain::with_state_slicer_and_win_command_goal(
        game.clone(),
        inputs,
        state_slicer,
        accept_win_command,
        move |state: &State| match &goal_for_domain {
            SolverGoal2::BuiltIn => goal_game.is_goal_complete(state),
            SolverGoal2::Expr(goal) => eval_goal_expr(&goal_expr_game, state, goal),
            SolverGoal2::ExactState(_) => projected_exact_goal
                .as_ref()
                .is_some_and(|goal| state == goal),
        },
    );
    let solver_initial = domain.initial_state(initial.clone());
    let score_game = loaded.clone();
    let deadend_game = loaded.clone();
    let score_expr_game = game.clone();
    let score_goal = goal.clone();
    let lose_game = loaded.clone();
    let lose_expr_game = game.clone();
    let replay_game = loaded.game.clone();
    solve_domain_with_observations(
        &mut domain,
        solver_initial,
        budget,
        move |state| {
            let goal_score = match &score_goal {
                SolverGoal2::BuiltIn => goal_score(&score_game, state.state()),
                SolverGoal2::Expr(goal) => goal_expr_score(&score_expr_game, state.state(), goal),
                SolverGoal2::ExactState(_) => exact_heuristic
                    .as_ref()
                    .map_or(0, |heuristic| heuristic.score(state.state())),
            };
            goal_score + solver_strategy_score(&score_game, state.state())
        },
        move |state| {
            let deadend = solver_has_deadend(&deadend_game, state.state());
            let lose = if let Some(lose) = &lose {
                eval_goal_expr(&lose_expr_game, state.state(), lose)
            } else {
                lose_game.is_lose_complete(state.state())
            };
            deadend || lose
        },
        |state| state.state().clone(),
        on_progress,
        move |solution_inputs| solution_steps(&replay_game, initial, solution_inputs),
    )
}

#[cfg(feature = "solver")]
fn solve_current_state_collect_with_budget_inner<O>(
    loaded: &LoadedGame,
    selector: SolverCollectSelector2,
    lose: Option<GoalExpr>,
    accept_win_command: bool,
    initial: State,
    budget: SearchBudget,
    max_results: usize,
    on_progress: Option<O>,
) -> Result<PuzzleCollectResponse, AppError>
where
    O: FnMut(&State, SearchProgress),
{
    let inputs = solver_inputs(loaded);
    if inputs.is_empty() {
        return Err(AppError::Config("no model inputs available".to_string()));
    }
    if max_results == 0 {
        return Err(AppError::Config(
            "solver collect maxResults must be greater than zero".to_string(),
        ));
    }

    let (solver_game, state_slicer) = solver_game_and_state_slicer_for_collect(
        loaded,
        loaded.solver_game(),
        &initial,
        &selector,
        lose.as_ref(),
    );
    let game = Arc::new(solver_game);
    let mut domain = PuzzleDomain::with_state_slicer_and_win_command_goal(
        game.clone(),
        inputs,
        state_slicer,
        accept_win_command,
        |_state: &State| false,
    );
    let solver_initial = domain.initial_state(initial);
    let score_game = game.clone();
    let score_selector = selector.clone();
    let lose_game = loaded.clone();
    let lose_expr_game = game.clone();
    let mut observations = SearchObservationSampler::new(96);
    let mut on_progress = on_progress;
    let mut matches = Vec::<CollectMatch<State, InputId>>::new();

    let outcome = best_first_scan_with_dead_states_and_progress(
        &mut domain,
        solver_initial,
        budget,
        move |state| collect_priority_score(&score_game, state.state(), &score_selector),
        move |state| {
            let deadend = solver_has_deadend(loaded, state.state());
            let lose = if let Some(lose) = &lose {
                eval_goal_expr(&lose_expr_game, state.state(), lose)
            } else {
                lose_game.is_lose_complete(state.state())
            };
            deadend || lose
        },
        |search_match| {
            let state = search_match.state.state().clone();
            match collect_match_score(&game, &state, &selector) {
                Some(score) => {
                    push_collect_match(
                        &mut matches,
                        CollectMatch {
                            depth: search_match.depth,
                            score,
                            moves: search_match.actions,
                            state,
                        },
                        max_results,
                    );
                    if matches.len() >= max_results
                        && matches.iter().all(|candidate| candidate.score.is_none())
                    {
                        ScanControl::Stop
                    } else {
                        ScanControl::Continue
                    }
                }
                None => ScanControl::Continue,
            }
        },
        |state, progress| {
            let observation = state.state().clone();
            observations.observe(&observation, progress);
            if let Some(on_progress) = on_progress.as_mut() {
                on_progress(&observation, progress);
            }
        },
    );

    sort_collect_matches(&mut matches);
    let observations = observations.into_observations();
    Ok(match outcome {
        ScanOutcome::Completed { stats } => CollectResponse::Completed {
            stats,
            matches,
            observations,
        },
        ScanOutcome::Stopped { stats } => CollectResponse::LimitReached {
            stats,
            matches,
            observations,
        },
        ScanOutcome::BudgetExceeded { stats } => CollectResponse::BudgetExceeded {
            stats,
            matches,
            observations,
        },
        ScanOutcome::Failed { failure } => CollectResponse::Failed {
            depth: failure.depth,
            error: format!("{:?}", failure.error),
            matches,
            observations,
        },
    })
}

#[cfg(feature = "solver")]
fn collect_priority_score(
    game: &CompiledGame,
    state: &State,
    selector: &SolverCollectSelector2,
) -> i64 {
    match selector {
        SolverCollectSelector2::Predicate(predicate) => goal_expr_score(game, state, predicate),
        SolverCollectSelector2::Maximize(value) => goal_value(game, state, value).saturating_neg(),
    }
}

#[cfg(feature = "solver")]
fn collect_match_score(
    game: &CompiledGame,
    state: &State,
    selector: &SolverCollectSelector2,
) -> Option<Option<i64>> {
    match selector {
        SolverCollectSelector2::Predicate(predicate) => {
            eval_goal_expr(game, state, predicate).then_some(None)
        }
        SolverCollectSelector2::Maximize(value) => Some(Some(goal_value(game, state, value))),
    }
}

#[cfg(feature = "solver")]
fn push_collect_match(
    matches: &mut Vec<CollectMatch<State, InputId>>,
    candidate: CollectMatch<State, InputId>,
    max_results: usize,
) {
    if matches.len() < max_results {
        matches.push(candidate);
        return;
    }
    let Some(score) = candidate.score else {
        return;
    };
    let Some((replace_index, _)) = matches
        .iter()
        .enumerate()
        .filter_map(|(index, current)| current.score.map(|current_score| (index, current_score)))
        .min_by_key(|(_, current_score)| *current_score)
    else {
        return;
    };
    if matches[replace_index]
        .score
        .is_some_and(|current_score| score > current_score)
    {
        matches[replace_index] = candidate;
    }
}

#[cfg(feature = "solver")]
fn sort_collect_matches(matches: &mut [CollectMatch<State, InputId>]) {
    matches.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.depth.cmp(&right.depth))
    });
}

#[cfg(feature = "solver")]
fn solution_steps(
    game: &puzzle_core::CompiledGame,
    mut state: State,
    inputs: &[InputId],
) -> Result<Vec<PuzzleSolutionStep>, AppError> {
    let mut steps = Vec::with_capacity(inputs.len() + 1);
    steps.push(SolutionStep {
        index: 0,
        input: None,
        state: state.clone(),
        completed: false,
    });

    for (index, input) in inputs.iter().enumerate() {
        state = transition_state(game, &state, *input)?;
        steps.push(SolutionStep {
            index: index + 1,
            input: Some(*input),
            state: state.clone(),
            completed: false,
        });
    }

    Ok(steps)
}

#[cfg(feature = "solver")]
fn compiled_solution_steps(
    engine: &puzzle_core_wasm::CompiledEngine,
    mut state: State,
    inputs: &[InputId],
) -> Result<Vec<PuzzleSolutionStep>, AppError> {
    let mut steps = Vec::with_capacity(inputs.len() + 1);
    steps.push(SolutionStep {
        index: 0,
        input: None,
        state: materialize_compiled_display_state(engine, &state)?,
        completed: false,
    });

    for (index, input) in inputs.iter().enumerate() {
        state = transition_state(engine.game(), &state, *input)?;
        steps.push(SolutionStep {
            index: index + 1,
            input: Some(*input),
            state: materialize_compiled_display_state(engine, &state)?,
            completed: false,
        });
    }

    Ok(steps)
}

#[cfg(feature = "solver")]
fn solve_current_state3_with_budget(
    parsed: &ParsedPuzzle3,
    initial: State3,
    budget: SearchBudget,
) -> Result<Puzzle3SolutionResponse, AppError> {
    solve_current_state3_with_budget_inner(
        parsed,
        initial,
        budget,
        None::<fn(&State3, SearchProgress)>,
    )
}

#[cfg(feature = "solver")]
fn solve_current_state3_with_budget_inner<O>(
    parsed: &ParsedPuzzle3,
    initial: State3,
    budget: SearchBudget,
    on_progress: Option<O>,
) -> Result<Puzzle3SolutionResponse, AppError>
where
    O: FnMut(&State3, SearchProgress),
{
    let inputs = solver_inputs3(&parsed.game);
    if inputs.is_empty() {
        return Err(AppError::Config("no 3D model inputs available".to_string()));
    }
    let win_condition = parsed
        .win_condition
        .clone()
        .ok_or_else(|| AppError::Config("3D solver requires win_conditions".to_string()))?;

    let game = Arc::new(parsed.game.clone());
    let rules = parsed.rules.clone();
    let goal_game = Arc::clone(&game);
    let score_win_condition = win_condition.clone();
    let state_slicer = solver_state_slicer_for_puzzle3(parsed);
    let mut domain = Puzzle3Domain::with_state_slicer(
        Arc::clone(&game),
        rules.clone(),
        inputs,
        state_slicer,
        move |state: &State3| win_condition.is_met(&goal_game, state),
    );
    let solver_initial = domain.initial_state(initial);
    let replay_initial = solver_initial.clone();
    let score_game = Arc::clone(&game);
    let score_strategy = parsed.solver_strategy.clone();
    let deadend_game = Arc::clone(&game);
    let deadend_strategy = parsed.solver_strategy.clone();
    solve_domain_with_observations(
        &mut domain,
        solver_initial,
        budget,
        move |state| {
            win_condition3_score(&score_game, state, &score_win_condition)
                + solver_strategy_score3(&score_game, &score_strategy, state)
        },
        move |state| solver_has_deadend3(&deadend_game, &deadend_strategy, state),
        |state| state.clone(),
        on_progress,
        move |solution_inputs| {
            solution_steps3(
                &game,
                &rules,
                parsed.win_condition.as_ref(),
                replay_initial,
                solution_inputs,
            )
        },
    )
}

#[cfg(feature = "solver")]
fn solution_steps3(
    game: &Game3,
    rules: &[Rule3],
    win_condition: Option<&WinCondition3>,
    mut state: State3,
    inputs: &[InputId],
) -> Result<Vec<Puzzle3SolutionStep>, AppError> {
    let mut steps = Vec::with_capacity(inputs.len() + 1);
    steps.push(SolutionStep {
        index: 0,
        input: None,
        completed: win_condition.is_some_and(|condition| condition.is_met(game, &state)),
        state: state.clone(),
    });

    for (index, input) in inputs.iter().enumerate() {
        state = transition_program3(game, &state, rules, *input)
            .map_err(|error| AppError::Config(format!("{error:?}")))?;
        steps.push(SolutionStep {
            index: index + 1,
            input: Some(*input),
            completed: win_condition.is_some_and(|condition| condition.is_met(game, &state)),
            state: state.clone(),
        });
    }

    Ok(steps)
}

#[cfg(feature = "solver")]
fn goal_score(loaded: &LoadedGame, state: &State) -> i64 {
    loaded
        .goal
        .as_ref()
        .map(|goal| goal_expr_score(&loaded.game, state, &goal.expr))
        .unwrap_or(0)
}

#[cfg(feature = "solver")]
fn solver_strategy_score(loaded: &LoadedGame, state: &State) -> i64 {
    solver_strategy_score_with(&loaded.solver_strategy, |value| {
        solver_query_expr_value(loaded, state, value)
    })
}

#[cfg(feature = "solver")]
fn solver_has_deadend(loaded: &LoadedGame, state: &State) -> bool {
    loaded
        .solver_strategy
        .has_deadend_with(|query| solver_query_expr_value(loaded, state, query) != 0)
}

#[cfg(feature = "solver")]
fn solver_query_expr_value(loaded: &LoadedGame, state: &State, value: &QueryExpr) -> i64 {
    solver_query_expr_value_with(
        value,
        &mut |variable| {
            state
                .variable_value(variable)
                .expect("query variable was resolved during lowering")
        },
        &mut |kind| goal_condition_value_kind(&loaded.game, state, kind),
        &mut |from, to| solver_strategy_distance(&loaded.game, state, from, to),
    )
}

#[cfg(feature = "solver")]
fn solver_strategy_distance(
    game: &CompiledGame,
    state: &State,
    from: &[ObjectId],
    to: &[ObjectId],
) -> i64 {
    let from_positions = selector_object_positions(game, state, from);
    let to_positions = selector_object_positions(game, state, to);
    let fallback = i64::from(state.width) + i64::from(state.height);
    from_positions
        .iter()
        .flat_map(|(ax, ay)| {
            to_positions
                .iter()
                .map(move |(bx, by)| manhattan(*ax, *ay, *bx, *by))
        })
        .min()
        .unwrap_or(fallback)
}

#[cfg(feature = "solver")]
fn selector_object_positions(
    game: &CompiledGame,
    state: &State,
    objects: &[ObjectId],
) -> Vec<(u16, u16)> {
    let mut positions = Vec::new();
    for object in objects {
        for slot in state.object_positions(*object) {
            let Some(position) = state.slot_position(*slot) else {
                continue;
            };
            if !positions.contains(&position)
                && state.has_object(game, position.0, position.1, *object)
            {
                positions.push(position);
            }
        }
    }
    positions
}

#[cfg(feature = "solver")]
fn solver_strategy_score3(game: &Game3, strategy: &SolverStrategy3, state: &State3) -> i64 {
    solver_strategy_score_with(strategy, |value| {
        solver_query_expr_value3(game, state, value)
    })
}

#[cfg(feature = "solver")]
fn solver_has_deadend3(game: &Game3, strategy: &SolverStrategy3, state: &State3) -> bool {
    strategy.has_deadend_with(|query| solver_query_expr_value3(game, state, query) != 0)
}

#[cfg(feature = "solver")]
fn solver_strategy_score_with<Query, Eval>(
    strategy: &puzzle_lang::SolverStrategyOf<Query>,
    mut eval: Eval,
) -> i64
where
    Eval: FnMut(&Query) -> i64,
{
    strategy
        .terms
        .iter()
        .map(|term| {
            let value = eval(&term.value);
            solver_strategy_term_score(term.direction, term.weight, value)
        })
        .sum()
}

#[cfg(feature = "solver")]
fn solver_strategy_term_score(direction: SolverStrategyDirection, weight: i64, value: i64) -> i64 {
    match direction {
        SolverStrategyDirection::Maximize => value.saturating_mul(-weight),
        SolverStrategyDirection::Minimize => value.saturating_mul(weight),
        SolverStrategyDirection::Prefer => {
            if value != 0 {
                0
            } else {
                weight
            }
        }
        SolverStrategyDirection::Avoid => {
            if value == 0 {
                0
            } else {
                weight
            }
        }
    }
}

#[cfg(feature = "solver")]
fn solver_query_expr_value3(game: &Game3, state: &State3, value: &QueryExpr3) -> i64 {
    solver_query_expr_value_with(
        value,
        &mut |variable| {
            state
                .variable_value(variable)
                .expect("query variable was resolved during lowering")
        },
        &mut |kind| eval_condition_kind(game, state, kind, None),
        &mut |from, to| solver_strategy_distance3(game, state, from, to),
    )
}

#[cfg(feature = "solver")]
fn solver_query_expr_value_with<Object, Value, Variable, EvalVariable, EvalValue, EvalDistance>(
    value: &QueryExprOf<Object, Value, Variable>,
    eval_variable: &mut EvalVariable,
    eval_value: &mut EvalValue,
    eval_distance: &mut EvalDistance,
) -> i64
where
    Variable: Copy,
    EvalVariable: FnMut(Variable) -> i64,
    EvalValue: FnMut(&Value) -> i64,
    EvalDistance: FnMut(&[Object], &[Object]) -> i64,
{
    match value {
        QueryExprOf::Variable(variable) => eval_variable(*variable),
        QueryExprOf::Value(kind) => eval_value(kind),
        QueryExprOf::Distance { from, to } => eval_distance(from, to),
        QueryExprOf::Compare { left, op, right } => {
            let left = solver_query_expr_value_with(left, eval_variable, eval_value, eval_distance);
            if compare_i64(left, *op, *right) {
                1
            } else {
                0
            }
        }
    }
}

#[cfg(feature = "solver")]
fn solver_strategy_distance3(
    game: &Game3,
    state: &State3,
    from: &[ObjectId3],
    to: &[ObjectId3],
) -> i64 {
    let from_positions = selector_object_positions3(game, state, from);
    let to_positions = selector_object_positions3(game, state, to);
    let fallback =
        i64::from(state.size.width) + i64::from(state.size.depth) + i64::from(state.size.height);
    from_positions
        .iter()
        .flat_map(|a| to_positions.iter().map(move |b| manhattan3(*a, *b)))
        .min()
        .unwrap_or(fallback)
}

#[cfg(feature = "solver")]
fn selector_object_positions3(game: &Game3, state: &State3, objects: &[ObjectId3]) -> Vec<Coord3> {
    let mut positions = Vec::new();
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let position = Coord3::new(x, y, z);
                if objects
                    .iter()
                    .any(|object| state.has_object(game, position, *object))
                    && !positions.contains(&position)
                {
                    positions.push(position);
                }
            }
        }
    }
    positions
}

#[cfg(feature = "solver")]
fn manhattan3(a: Coord3, b: Coord3) -> i64 {
    i64::from(a.x.abs_diff(b.x)) + i64::from(a.y.abs_diff(b.y)) + i64::from(a.z.abs_diff(b.z))
}

#[cfg(feature = "solver")]
fn win_condition3_score(game: &Game3, state: &State3, condition: &WinCondition3) -> i64 {
    match condition {
        WinCondition3::All(conditions) => conditions
            .iter()
            .map(|condition| win_condition3_score(game, state, condition))
            .sum(),
        WinCondition3::Any(conditions) => conditions
            .iter()
            .map(|condition| win_condition3_score(game, state, condition))
            .min()
            .unwrap_or(0),
        WinCondition3::AllObjectsCoveredByPattern {
            object,
            cover_pattern,
        } => same_cell_all_objects_on_score3(game, state, *object, cover_pattern).unwrap_or(0),
        WinCondition3::SomeObject(_)
        | WinCondition3::NoObject(_)
        | WinCondition3::SomePattern(_)
        | WinCondition3::NoPattern(_) => 0,
    }
}

#[cfg(feature = "solver")]
fn same_cell_all_objects_on_score3(
    game: &Game3,
    state: &State3,
    subject: ObjectId3,
    cover_pattern: &puzzle_grid3d::Pattern3,
) -> Option<i64> {
    let [cell] = cover_pattern.cells() else {
        return None;
    };
    if cell.offset != puzzle_grid3d::Offset3::ZERO
        || !cell.require_objects.contains(&subject)
        || !cell.require_object_sets.is_empty()
        || !cell.forbid_objects.is_empty()
        || !cell.require_mark.is_empty()
        || !cell.require_object_set_mark.is_empty()
        || !cell.forbid_mark.is_empty()
        || !cell.forbid_object_set_mark.is_empty()
    {
        return None;
    }
    let covers = cell
        .require_objects
        .iter()
        .copied()
        .filter(|object| *object != subject)
        .collect::<Vec<_>>();
    if covers.is_empty() {
        return None;
    }
    let cover_positions = selector_object_positions3(game, state, &covers);
    let fallback =
        i64::from(state.size.width) + i64::from(state.size.depth) + i64::from(state.size.height);
    Some(
        selector_object_positions3(game, state, &[subject])
            .into_iter()
            .filter(|position| {
                !covers
                    .iter()
                    .any(|cover| state.has_object(game, *position, *cover))
            })
            .map(|position| {
                cover_positions
                    .iter()
                    .map(|cover| manhattan3(position, *cover))
                    .min()
                    .unwrap_or(fallback)
                    .max(1)
            })
            .sum(),
    )
}

#[cfg(feature = "solver")]
fn goal_expr_score(game: &CompiledGame, state: &State, expr: &GoalExpr) -> i64 {
    match expr {
        GoalExpr::All(exprs) => exprs
            .iter()
            .map(|expr| goal_expr_score(game, state, expr))
            .sum(),
        GoalExpr::Any(exprs) => exprs
            .iter()
            .map(|expr| goal_expr_score(game, state, expr))
            .min()
            .unwrap_or(0),
        GoalExpr::Clause(clause) => {
            let value = goal_value(game, state, &clause.value);
            if compare_i64(value, clause.op, clause.expected) {
                0
            } else {
                goal_clause_score(
                    game,
                    state,
                    &clause.value,
                    value,
                    clause.op,
                    clause.expected,
                )
            }
        }
    }
}

#[cfg(feature = "solver")]
fn exact_state_score(state: &State, goal: &State) -> i64 {
    if state.width != goal.width
        || state.height != goal.height
        || state.layer_count != goal.layer_count
    {
        return i64::MIN / 4;
    }
    state
        .slots()
        .iter()
        .zip(goal.slots())
        .filter(|(current, expected)| current != expected)
        .count() as i64
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
struct ExactStateHeuristic {
    goal: State,
    changed_objects: Vec<ObjectId>,
}

#[cfg(feature = "solver")]
impl ExactStateHeuristic {
    fn new(game: &CompiledGame, initial: &State, goal: &State) -> Self {
        let changed_objects = (1..=game.object_count())
            .map(|id| ObjectId(id as u16))
            .filter(|object| object_positions_for_exact(initial, *object) != object_positions_for_exact(goal, *object))
            .collect();
        Self {
            goal: goal.clone(),
            changed_objects,
        }
    }

    fn score(&self, state: &State) -> i64 {
        exact_state_score(state, &self.goal)
            + self
                .changed_objects
                .iter()
                .map(|object| exact_object_distance_score(state, &self.goal, *object))
                .sum::<i64>()
    }
}

#[cfg(feature = "solver")]
fn exact_object_distance_score(state: &State, goal: &State, object: ObjectId) -> i64 {
    let current_positions = object_positions_for_exact(state, object);
    let mut goal_positions = object_positions_for_exact(goal, object);
    let fallback = i64::from(state.width) + i64::from(state.height) + 1;
    let mut score = current_positions
        .len()
        .abs_diff(goal_positions.len()) as i64
        * fallback;
    for current in current_positions {
        let Some((index, distance)) = goal_positions
            .iter()
            .enumerate()
            .map(|(index, goal)| (index, manhattan(current.0, current.1, goal.0, goal.1)))
            .min_by_key(|(_, distance)| *distance)
        else {
            score += fallback;
            continue;
        };
        score += distance;
        goal_positions.remove(index);
    }
    score
}

#[cfg(feature = "solver")]
fn manhattan(ax: u16, ay: u16, bx: u16, by: u16) -> i64 {
    i64::from(ax.abs_diff(bx)) + i64::from(ay.abs_diff(by))
}

#[cfg(feature = "solver")]
fn object_positions_for_exact(state: &State, object: ObjectId) -> Vec<(u16, u16)> {
    let mut positions = state
        .object_positions(object)
        .iter()
        .filter_map(|slot| state.slot_position(*slot))
        .collect::<Vec<_>>();
    positions.sort_unstable();
    positions
}

#[cfg(feature = "solver")]
fn eval_goal_expr(game: &CompiledGame, state: &State, expr: &GoalExpr) -> bool {
    match expr {
        GoalExpr::All(exprs) => exprs.iter().all(|expr| eval_goal_expr(game, state, expr)),
        GoalExpr::Any(exprs) => exprs.iter().any(|expr| eval_goal_expr(game, state, expr)),
        GoalExpr::Clause(clause) => compare_i64(
            goal_value(game, state, &clause.value),
            clause.op,
            clause.expected,
        ),
    }
}

#[cfg(feature = "solver")]
fn goal_clause_score(
    game: &CompiledGame,
    state: &State,
    value: &GoalValue,
    current: i64,
    op: ComparisonOp,
    expected: i64,
) -> i64 {
    match value {
        GoalValue::Variable(_) => comparison_violation_score(current, op, expected),
        GoalValue::Condition(condition) => game
            .condition_def(*condition)
            .map(|condition| {
                condition_value_kind_score(game, state, &condition.kind, current, op, expected)
            })
            .unwrap_or_else(|| comparison_violation_score(current, op, expected)),
        GoalValue::InlineConditionValue(kind) => {
            condition_value_kind_score(game, state, kind, current, op, expected)
        }
        GoalValue::AllObjectsOn { subjects, covers } => {
            all_objects_on_score(game, state, subjects, covers)
        }
    }
}

#[cfg(feature = "solver")]
fn all_objects_on_score(
    game: &CompiledGame,
    state: &State,
    subjects: &[ObjectId],
    covers: &[ObjectId],
) -> i64 {
    let cover_positions = selector_object_positions(game, state, covers);
    let fallback = i64::from(state.width) + i64::from(state.height);
    selector_object_positions(game, state, subjects)
        .into_iter()
        .filter(|(x, y)| {
            !covers
                .iter()
                .any(|cover| state.has_object(game, *x, *y, *cover))
        })
        .map(|(x, y)| {
            cover_positions
                .iter()
                .map(|(cover_x, cover_y)| manhattan(x, y, *cover_x, *cover_y))
                .min()
                .unwrap_or(fallback)
                .max(1)
        })
        .sum()
}

#[cfg(feature = "solver")]
fn condition_value_kind_score(
    game: &CompiledGame,
    state: &State,
    kind: &ConditionValueKind,
    current: i64,
    op: ComparisonOp,
    expected: i64,
) -> i64 {
    match kind {
        // A win condition may establish whether a state is better, but it does
        // not establish a spatial preference.  In particular, inferring a
        // distance from `no [ A no B ]` presumes that moving A closer to B is
        // always desirable, which is not generally true for Sokoban-like games.
        // Authors can state that preference explicitly in `solver.strategy`.
        ConditionValueKind::NoneObjects(objects) if current == 0 => objects
            .iter()
            .map(|object| i64::from(state.object_count(*object)))
            .sum(),
        ConditionValueKind::ExistsObjects(_) if current == 0 => 1,
        ConditionValueKind::NoneMatches(patterns) if current == 0 => patterns
            .iter()
            .map(|pattern| i64::from(puzzle_core::count_pattern_matches(game, state, pattern)))
            .sum(),
        ConditionValueKind::ExistsMatches(_) if current == 0 => 1,
        _ => comparison_violation_score(current, op, expected),
    }
}

#[cfg(feature = "solver")]
fn comparison_violation_score(current: i64, op: ComparisonOp, expected: i64) -> i64 {
    match op {
        ComparisonOp::Eq => current.abs_diff(expected) as i64,
        ComparisonOp::NotEq => 1,
        ComparisonOp::Greater => expected
            .saturating_sub(current)
            .max(0)
            .saturating_add(1),
        ComparisonOp::GreaterEq => expected.saturating_sub(current).max(0),
        ComparisonOp::Less => current
            .saturating_sub(expected)
            .max(0)
            .saturating_add(1),
        ComparisonOp::LessEq => current.saturating_sub(expected).max(0),
    }
}

#[cfg(feature = "solver")]
fn compare_i64(left: i64, op: ComparisonOp, right: i64) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::NotEq => left != right,
        ComparisonOp::Greater => left > right,
        ComparisonOp::GreaterEq => left >= right,
        ComparisonOp::Less => left < right,
        ComparisonOp::LessEq => left <= right,
    }
}

#[cfg(feature = "solver")]
fn goal_value(game: &CompiledGame, state: &State, value: &GoalValue) -> i64 {
    match value {
        GoalValue::Variable(variable) => state.variable_value(*variable).unwrap_or(0),
        GoalValue::Condition(condition) => game
            .condition_def(*condition)
            .map(|condition| goal_condition_value_kind(game, state, &condition.kind))
            .unwrap_or(0),
        GoalValue::InlineConditionValue(kind) => goal_condition_value_kind(game, state, kind),
        GoalValue::AllObjectsOn { subjects, covers } => {
            if subjects.iter().all(|subject| {
                state.object_positions(*subject).iter().all(|slot| {
                    state.slot_position(*slot).is_some_and(|(x, y)| {
                        covers
                            .iter()
                            .any(|cover| state.has_object(game, x, y, *cover))
                    })
                })
            }) {
                1
            } else {
                0
            }
        }
    }
}

#[cfg(feature = "solver")]
fn goal_condition_value_kind(game: &CompiledGame, state: &State, kind: &ConditionValueKind) -> i64 {
    match kind {
        ConditionValueKind::CountObjects(objects) => objects
            .iter()
            .map(|object| i64::from(state.object_count(*object)))
            .sum(),
        ConditionValueKind::ExistsObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                1
            } else {
                0
            }
        }
        ConditionValueKind::NoneObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                0
            } else {
                1
            }
        }
        ConditionValueKind::CountMatches(patterns) => patterns
            .iter()
            .map(|pattern| i64::from(puzzle_core::count_pattern_matches(game, state, pattern)))
            .sum(),
        ConditionValueKind::ExistsMatches(patterns) => {
            if patterns
                .iter()
                .any(|pattern| puzzle_core::has_pattern_match(game, state, pattern))
            {
                1
            } else {
                0
            }
        }
        ConditionValueKind::NoneMatches(patterns) => {
            if patterns
                .iter()
                .any(|pattern| puzzle_core::has_pattern_match(game, state, pattern))
            {
                0
            } else {
                1
            }
        }
        ConditionValueKind::CountInputMatches(_)
        | ConditionValueKind::ExistsInputMatches(_)
        | ConditionValueKind::NoneInputMatches(_) => 0,
    }
}

#[cfg(all(test, feature = "solver"))]
mod solver_goal_score_tests {
    use super::*;

    #[test]
    fn no_pattern_goal_scores_violations_without_spatial_distance() {
        let source = r#"
title = no_pattern_score

puzzle board {
layers {
floor = Goal
actor = Box Player
}

win_conditions {
no [ Box no Goal ]
}

rules {
}
}

levels default of board {
legend {
. = empty
B = Box
G = Goal
}

level "start" {
B.BG
}
}
"#;
        let loaded = parse_game(source).expect("test game should load");

        assert_eq!(goal_score(&loaded, &loaded.levels[0].initial_state), 2);
    }

    #[test]
    fn all_objects_on_goal_scores_from_quantified_subjects_to_covers() {
        let source = r#"
title = all_on_score

puzzle board {
layers {
floor = Goal
actor = Box
}

win_conditions {
all Goal on Box
}

rules {
}
}

levels default of board {
legend {
. = empty
B = Box
G = Goal
}

level "start" {
G..B
}
}
"#;
        let loaded = parse_game(source).expect("test game should load");

        assert_eq!(goal_score(&loaded, &loaded.levels[0].initial_state), 3);
    }

    #[test]
    fn all_objects_on_goal_generates_the_same_subject_first_score_in_3d() {
        let parsed = parse_puzzle3d_for_solver(
            r#"
puzzle3 board {
layers {
floor = Goal
actor = Box
}
win_conditions {
all Goal on Box
}
}

levels3 default of board {
legend {
. = empty
G = Goal
B = Box
}
level "start" {
G.B
}
}
"#,
        )
        .expect("test puzzle3 should load");
        let state = parsed
            .level_bundle
            .as_ref()
            .expect("test puzzle3 should have levels")
            .build_level_state(0)
            .expect("test level should build");

        assert_eq!(
            win_condition3_score(
                &parsed.game,
                &state,
                parsed.win_condition.as_ref().expect("test win condition"),
            ),
            2
        );
    }
}
