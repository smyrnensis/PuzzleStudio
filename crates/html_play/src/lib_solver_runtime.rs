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
        push_export_animation(&mut out, &self.loaded);
        out.push(',');
        push_sound_events(&mut out, &sound_events);
        out.push(',');
        push_message_events(&mut out, &message_events);
        out.push(',');
        push_wait_events(&mut out, &wait_events);
        out.push(',');
        push_animation_events(&mut out, &animation_events);
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
        let loaded = if document.models.len() > 1 {
            mixed_document_loaded_game(&document)?
        } else {
            match document.single_model() {
                Some(LoadedDocumentModel::Puzzle2d { game, .. }) => game.clone(),
                Some(LoadedDocumentModel::Puzzle3d { .. }) => {
                    puzzle3_document_scene_host_loaded_game(&document)?
                }
                None => return Err("standalone session bridge requires a puzzle model".to_string()),
            }
        };
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
        match (method, url) {
            ("GET", "/api/state") => Ok(self.snapshot_json()),
            ("POST", "/api/command/undo") => {
                self.state.session.undo(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/redo") => {
                self.state.session.redo(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/restart") => {
                self.state.session.restart_level(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/next") => {
                self.state.session.advance_level(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", path) if path.starts_with("/api/input/") => {
                let input_name = percent_decode(&path["/api/input/".len()..]);
                self.state
                    .apply_input_name(&input_name)
                    .map_err(|error| error.to_string())?;
                Ok(self.snapshot_json())
            }
            ("POST", path) if path.starts_with("/api/command/") => {
                let command_name = percent_decode(&path["/api/command/".len()..]);
                self.state
                    .apply_command_name(&command_name)
                    .map_err(|error| error.to_string())?;
                Ok(self.snapshot_json())
            }
            _ => Err(format!("Unsupported exported HTML request: {method} {url}")),
        }
    }

    pub fn apply_input_name(&mut self, input_name: &str) -> Result<(), String> {
        self.state
            .apply_input_name(input_name)
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
struct SolutionStep {
    index: usize,
    input: Option<InputId>,
    state: State,
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
struct SearchObservation {
    progress: SearchProgress,
    state: State,
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
struct SearchObservation3 {
    progress: SearchProgress,
    state: State3,
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

    fn observe(&mut self, state: &State, progress: SearchProgress) -> bool {
        if progress.expanded < self.next_expanded {
            return false;
        }
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
        true
    }
}

#[cfg(feature = "solver")]
impl SearchObservationSampler<State> {
    fn into_2d(self) -> Vec<SearchObservation> {
        self.observations
            .into_iter()
            .map(|(progress, state)| SearchObservation { progress, state })
            .collect()
    }
}

#[cfg(feature = "solver")]
impl SearchObservationSampler<State3> {
    fn into_3d(self) -> Vec<SearchObservation3> {
        self.observations
            .into_iter()
            .map(|(progress, state)| SearchObservation3 { progress, state })
            .collect()
    }
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
enum SolutionResponse {
    Solved {
        depth: u32,
        moves: Vec<InputId>,
        steps: Vec<SolutionStep>,
        observations: Vec<SearchObservation>,
    },
    Exhausted {
        stats: SearchStats,
        observations: Vec<SearchObservation>,
    },
    BudgetExceeded {
        stats: SearchStats,
        observations: Vec<SearchObservation>,
    },
    Failed {
        depth: u32,
        error: String,
        observations: Vec<SearchObservation>,
    },
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
struct SolutionStep3 {
    index: usize,
    input: Option<InputId3>,
    state: State3,
    completed: bool,
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
enum SolutionResponse3 {
    Solved {
        depth: u32,
        moves: Vec<InputId3>,
        steps: Vec<SolutionStep3>,
        observations: Vec<SearchObservation3>,
    },
    Exhausted {
        stats: SearchStats,
        observations: Vec<SearchObservation3>,
    },
    BudgetExceeded {
        stats: SearchStats,
        observations: Vec<SearchObservation3>,
    },
    Failed {
        depth: u32,
        error: String,
        observations: Vec<SearchObservation3>,
    },
}

#[cfg(feature = "solver")]
fn solve_current_state(
    loaded: &LoadedGame,
    initial: State,
    solver: SolverConfig,
) -> Result<SolutionResponse, AppError> {
    solve_current_state_with_budget(loaded, initial, solver.budget())
}

#[cfg(feature = "solver")]
fn solve_current_state_with_budget(
    loaded: &LoadedGame,
    initial: State,
    budget: SearchBudget,
) -> Result<SolutionResponse, AppError> {
    solve_current_state_with_budget_inner(
        loaded,
        initial,
        budget,
        None::<fn(&State, SearchProgress)>,
    )
}

#[cfg(feature = "solver")]
fn solve_compiled_state_with_budget_and_progress<O>(
    engine: &puzzle_core_wasm::CompiledEngine,
    goal: Option<GoalExpr>,
    lose: Option<GoalExpr>,
    initial: State,
    budget: SearchBudget,
    mut on_observation: Option<O>,
) -> Result<SolutionResponse, AppError>
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

    let game = Arc::new(game.clone());
    let goal_game = game.clone();
    let goal_for_domain = goal.clone();
    let mut domain = PuzzleDomain::new(game.clone(), inputs, move |state: &State| {
        goal_for_domain
            .as_ref()
            .is_some_and(|goal| eval_goal_expr(&goal_game, state, goal))
    });
    let solver_initial = PuzzleSearchState::new(initial.without_visual_objects(domain.game()));
    let score_game = game.clone();
    let score_goal = goal.clone();
    let lose_game = game.clone();
    let mut observations = SearchObservationSampler::new(96);
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
            let display_state = materialize_compiled_display_state(engine, state.state());
            if observations.observe(&display_state, progress) {
                if let Some(on_observation) = on_observation.as_mut() {
                    on_observation(&display_state, progress);
                }
            }
        },
    );
    let observations = observations.into_2d();

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
) -> State {
    let Some(program) = engine.program("display", -1) else {
        return state.clone();
    };
    if program.is_empty() {
        return state.clone();
    }
    transition_program(engine.game(), program, state, InputId(0))
        .unwrap_or_else(|_| state.clone())
}

#[cfg(feature = "solver")]
fn solve_current_state_with_budget_inner<O>(
    loaded: &LoadedGame,
    initial: State,
    budget: SearchBudget,
    mut on_progress: Option<O>,
) -> Result<SolutionResponse, AppError>
where
    O: FnMut(&State, SearchProgress),
{
    let inputs = solver_inputs(loaded);
    if inputs.is_empty() {
        return Err(AppError::Config("no model inputs available".to_string()));
    }

    let game = Arc::new(loaded.game.clone());
    let goal_game = loaded.clone();
    let mut domain = PuzzleDomain::new(game.clone(), inputs, move |state: &State| {
        goal_game.is_goal_complete(state)
    });
    let solver_initial = PuzzleSearchState::new(initial.without_visual_objects(domain.game()));
    let score_game = loaded.clone();
    let lose_game = loaded.clone();
    let mut observations = SearchObservationSampler::new(96);
    let outcome = best_first_with_dead_states_and_progress(
        &mut domain,
        solver_initial,
        budget,
        move |state| goal_score(&score_game, state.state()),
        move |state| lose_game.is_lose_complete(state.state()),
        |state, progress| {
            observations.observe(state.state(), progress);
            if let Some(on_progress) = on_progress.as_mut() {
                on_progress(state.state(), progress);
            }
        },
    );
    let observations = observations.into_2d();

    let response = match outcome {
        SearchOutcome::Solved(witness) => {
            let depth = witness.depth;
            let solution_inputs = witness.actions;
            SolutionResponse::Solved {
                depth,
                steps: solution_steps(&loaded.game, initial, &solution_inputs)?,
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
fn solution_steps(
    game: &puzzle_core::CompiledGame,
    mut state: State,
    inputs: &[InputId],
) -> Result<Vec<SolutionStep>, AppError> {
    let mut steps = Vec::with_capacity(inputs.len() + 1);
    steps.push(SolutionStep {
        index: 0,
        input: None,
        state: state.clone(),
    });

    for (index, input) in inputs.iter().enumerate() {
        state = transition_state(game, &state, *input)?;
        steps.push(SolutionStep {
            index: index + 1,
            input: Some(*input),
            state: state.clone(),
        });
    }

    Ok(steps)
}

#[cfg(feature = "solver")]
fn compiled_solution_steps(
    engine: &puzzle_core_wasm::CompiledEngine,
    mut state: State,
    inputs: &[InputId],
) -> Result<Vec<SolutionStep>, AppError> {
    let mut steps = Vec::with_capacity(inputs.len() + 1);
    steps.push(SolutionStep {
        index: 0,
        input: None,
        state: materialize_compiled_display_state(engine, &state),
    });

    for (index, input) in inputs.iter().enumerate() {
        state = transition_state(engine.game(), &state, *input)?;
        steps.push(SolutionStep {
            index: index + 1,
            input: Some(*input),
            state: materialize_compiled_display_state(engine, &state),
        });
    }

    Ok(steps)
}

#[cfg(feature = "solver")]
fn solve_current_state3_with_budget(
    parsed: &ParsedPuzzle3,
    initial: State3,
    budget: SearchBudget,
) -> Result<SolutionResponse3, AppError> {
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
    mut on_progress: Option<O>,
) -> Result<SolutionResponse3, AppError>
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
    let mut domain = Puzzle3Domain::new(
        Arc::clone(&game),
        rules.clone(),
        inputs,
        move |state: &State3| win_condition.is_met(&goal_game, state),
    );
    let mut observations = SearchObservationSampler::new(96);
    let outcome = best_first_with_dead_states_and_progress(
        &mut domain,
        initial.clone(),
        budget,
        |_| 0,
        |_| false,
        |state, progress| {
            observations.observe(state, progress);
            if let Some(on_progress) = on_progress.as_mut() {
                on_progress(state, progress);
            }
        },
    );
    let observations = observations.into_3d();

    let response = match outcome {
        SearchOutcome::Solved(witness) => {
            let depth = witness.depth;
            let solution_inputs = witness.actions;
            SolutionResponse3::Solved {
                depth,
                steps: solution_steps3(
                    &game,
                    &rules,
                    parsed.win_condition.as_ref(),
                    initial,
                    &solution_inputs,
                )?,
                moves: solution_inputs,
                observations,
            }
        }
        SearchOutcome::Exhausted(stats) => SolutionResponse3::Exhausted {
            stats,
            observations,
        },
        SearchOutcome::BudgetExceeded(stats) => SolutionResponse3::BudgetExceeded {
            stats,
            observations,
        },
        SearchOutcome::Failed(failure) => SolutionResponse3::Failed {
            depth: failure.depth,
            error: format!("{:?}", failure.error),
            observations,
        },
    };
    Ok(response)
}

#[cfg(feature = "solver")]
fn solution_steps3(
    game: &Game3,
    rules: &[Rule3],
    win_condition: Option<&WinCondition3>,
    mut state: State3,
    inputs: &[InputId3],
) -> Result<Vec<SolutionStep3>, AppError> {
    let mut steps = Vec::with_capacity(inputs.len() + 1);
    steps.push(SolutionStep3 {
        index: 0,
        input: None,
        completed: win_condition.is_some_and(|condition| condition.is_met(game, &state)),
        state: state.clone(),
    });

    for (index, input) in inputs.iter().enumerate() {
        state = transition_program3(game, &state, rules, *input)
            .map_err(|error| AppError::Config(format!("{error:?}")))?;
        steps.push(SolutionStep3 {
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
                goal_clause_score(game, state, &clause.value, value, clause.expected)
            }
        }
    }
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
    expected: i64,
) -> i64 {
    match value {
        GoalValue::Global(_) => current.abs_diff(expected) as i64,
        GoalValue::Condition(condition) => game
            .condition_def(*condition)
            .map(|condition| {
                condition_value_kind_score(game, state, &condition.kind, current, expected)
            })
            .unwrap_or_else(|| current.abs_diff(expected) as i64),
        GoalValue::InlineConditionValue(kind) => {
            condition_value_kind_score(game, state, kind, current, expected)
        }
    }
}

#[cfg(feature = "solver")]
fn condition_value_kind_score(
    game: &CompiledGame,
    state: &State,
    kind: &ConditionValueKind,
    current: i64,
    expected: i64,
) -> i64 {
    match kind {
        ConditionValueKind::CountMatches(patterns) if expected == 0 => patterns
            .iter()
            .map(|pattern| pattern_distance_score(game, state, pattern))
            .sum(),
        ConditionValueKind::NoneMatches(patterns) if expected != 0 => patterns
            .iter()
            .map(|pattern| pattern_distance_score(game, state, pattern))
            .sum(),
        ConditionValueKind::ExistsMatches(patterns) if expected != 0 => patterns
            .iter()
            .map(|pattern| pattern_distance_score(game, state, pattern))
            .min()
            .unwrap_or(1),
        _ => current.abs_diff(expected) as i64,
    }
}

#[cfg(feature = "solver")]
fn pattern_distance_score(game: &CompiledGame, state: &State, pattern: &Pattern) -> i64 {
    let Some(component) = pattern.components.first() else {
        return 0;
    };
    if pattern.components.len() != 1 || component.cells.len() != 1 {
        return i64::from(puzzle_core::count_pattern_matches(game, state, pattern));
    }
    let cell = &component.cells[0];
    if cell.require_objects.is_empty() || cell.forbid_objects.is_empty() {
        return i64::from(puzzle_core::count_pattern_matches(game, state, pattern));
    }

    let targets = object_positions(game, state, &cell.forbid_objects);
    let fallback = i64::from(state.width) + i64::from(state.height);
    let mut score = 0_i64;
    for y in 0..state.height {
        for x in 0..state.width {
            if !has_all_objects(game, state, x, y, &cell.require_objects) {
                continue;
            }
            if has_all_objects(game, state, x, y, &cell.forbid_objects) {
                continue;
            }
            let distance = targets
                .iter()
                .map(|(tx, ty)| manhattan(x, y, *tx, *ty))
                .min()
                .unwrap_or(fallback);
            score += distance.max(1);
        }
    }
    score
}

#[cfg(feature = "solver")]
fn object_positions(game: &CompiledGame, state: &State, objects: &[ObjectId]) -> Vec<(u16, u16)> {
    if let [object] = objects {
        return state
            .object_positions(*object)
            .iter()
            .filter_map(|slot| state.slot_position(*slot))
            .collect();
    }

    let mut positions = Vec::new();
    for y in 0..state.height {
        for x in 0..state.width {
            if has_all_objects(game, state, x, y, objects) {
                positions.push((x, y));
            }
        }
    }
    positions
}

#[cfg(feature = "solver")]
fn has_all_objects(
    game: &CompiledGame,
    state: &State,
    x: u16,
    y: u16,
    objects: &[ObjectId],
) -> bool {
    objects
        .iter()
        .all(|object| state.has_object(game, x, y, *object))
}

#[cfg(feature = "solver")]
fn manhattan(ax: u16, ay: u16, bx: u16, by: u16) -> i64 {
    i64::from(ax.abs_diff(bx)) + i64::from(ay.abs_diff(by))
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
        GoalValue::Global(global) => state.global_value(*global).unwrap_or(0),
        GoalValue::Condition(condition) => game
            .condition_def(*condition)
            .map(|condition| goal_condition_value_kind(game, state, &condition.kind))
            .unwrap_or(0),
        GoalValue::InlineConditionValue(kind) => goal_condition_value_kind(game, state, kind),
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
