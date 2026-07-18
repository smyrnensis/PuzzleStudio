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
            max_nodes: 1000,
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
    document: puzzle_lang::LoadedDocument,
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
        document: puzzle_lang::LoadedDocument,
        loaded: LoadedGame,
        source: String,
        puzzle_path: String,
        game_css: String,
        game_visuals_js: String,
        solver: SolverConfig,
    ) -> Self {
        let session = GameSession::new(&loaded);
        Self {
            document,
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
        let presentation_events = self.session.take_presentation_events();
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
        push_export_input_buffer(&mut out, &self.loaded);
        out.push(',');
        push_export_animation(&mut out, &self.loaded);
        out.push(',');
        push_presentation_events(&mut out, &self.loaded, &presentation_events);
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
        let input = input_id_by_name(&self.loaded, input_name)
            .ok_or_else(|| AppError::Config(format!("unknown input: {input_name}")))?;
        self.session.apply_traced_input(&self.loaded, input)?;
        let debug = self.session.last_transition_trace().cloned();
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

    #[cfg(feature = "solver")]
    fn solve_json(&self) -> Result<String, AppError> {
        let level_index = self
            .session
            .active_level_index()
            .ok_or_else(|| AppError::Config("solver requires an active level".to_string()))?;
        let response =
            solve_current_session(&self.loaded, level_index, self.session.clone(), self.solver)?;
        let mut out = String::new();
        push_solution_response(&mut out, &self.loaded, &response);
        Ok(out)
    }
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
struct SolutionStep<State, Input> {
    index: usize,
    input: Option<Input>,
    state: State,
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
type GridSolutionResponse<const D: usize, Size> = SolutionResponse<GridState<D, Size>, InputId>;
#[cfg(feature = "solver")]
type GridSolutionStep<const D: usize, Size> = SolutionStep<GridState<D, Size>, InputId>;
#[cfg(feature = "solver")]
type GridSearchObservation<const D: usize, Size> = SearchObservation<GridState<D, Size>>;

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

#[cfg(all(feature = "solver", test))]
fn solve_current_state(
    loaded: &LoadedGame,
    level_index: usize,
    initial: State,
    solver: SolverConfig,
) -> Result<PuzzleSolutionResponse, AppError> {
    solve_current_state_with_budget(loaded, level_index, initial, solver.budget())
}

#[cfg(feature = "solver")]
fn solve_current_session(
    loaded: &LoadedGame,
    level_index: usize,
    session: GameSession,
    solver: SolverConfig,
) -> Result<PuzzleSolutionResponse, AppError> {
    solve_current_session_with_budget(loaded, level_index, session, solver.budget())
}

#[cfg(feature = "solver")]
fn solve_current_session_with_budget(
    loaded: &LoadedGame,
    level_index: usize,
    session: GameSession,
    budget: SearchBudget,
) -> Result<PuzzleSolutionResponse, AppError> {
    let initial = session.state().clone();
    solve_current_state_with_goal_with_budget_inner(
        loaded,
        level_index,
        SolverGoal2::BuiltIn,
        None,
        initial,
        Some(session),
        budget,
        None::<fn(&State, SearchProgress)>,
    )
}

#[cfg(all(feature = "solver", test))]
fn solve_current_state_with_budget(
    loaded: &LoadedGame,
    level_index: usize,
    initial: State,
    budget: SearchBudget,
) -> Result<PuzzleSolutionResponse, AppError> {
    solve_current_state_with_budget_inner(
        loaded,
        level_index,
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
fn solver_model_and_state_slicer<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    initial: &GridState<D, Size>,
    exact_goal: Option<&GridState<D, Size>>,
    explicit_goal: Option<&GridGoalExpr<D>>,
    explicit_lose: Option<&GridGoalExpr<D>>,
) -> (LoadedGridGame<D, Size>, puzzle_solver::SolverStateSlicer) {
    let mut roots = BTreeSet::new();
    if let Some(goal) = exact_goal {
        puzzle_solver::object_refs::collect_state_roots(initial, &mut roots);
        puzzle_solver::object_refs::collect_state_roots(goal, &mut roots);
    } else if let Some(goal) = explicit_goal {
        puzzle_solver::object_refs::collect_goal_expr_roots(&loaded.game, goal, &mut roots);
    } else if let Some(goal) = &loaded.goal {
        puzzle_solver::object_refs::collect_goal_expr_roots(&loaded.game, &goal.expr, &mut roots);
    }
    if let Some(lose) = explicit_lose {
        puzzle_solver::object_refs::collect_goal_expr_roots(&loaded.game, lose, &mut roots);
    } else if let Some(lose) = &loaded.lose {
        puzzle_solver::object_refs::collect_goal_expr_roots(&loaded.game, &lose.expr, &mut roots);
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
    solver_model_and_state_slicer_from_roots(loaded, initial, roots)
}

#[cfg(feature = "solver")]
fn solver_model_and_state_slicer_from_roots<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    initial: &GridState<D, Size>,
    roots: BTreeSet<ObjectId>,
) -> (LoadedGridGame<D, Size>, puzzle_solver::SolverStateSlicer) {
    let slice = puzzle_solver::SolverSlice::from_loaded_game_roots(loaded, [initial], roots);
    let state_slicer = puzzle_solver::SolverStateSlicer::<ObjectId>::from_kept_objects(
        &loaded.game,
        slice.kept_objects(),
    );
    let solver_model = slice.project_loaded_game(loaded, &state_slicer);
    (solver_model, state_slicer)
}

#[cfg(feature = "solver")]
fn solver_game_and_state_slicer_for_collect(
    loaded: &LoadedGame,
    initial: &State,
    selector: &SolverCollectSelector2,
    explicit_lose: Option<&GoalExpr>,
) -> (LoadedGame, puzzle_solver::SolverStateSlicer) {
    let mut roots = BTreeSet::new();
    match selector {
        SolverCollectSelector2::Predicate(predicate) => {
            puzzle_solver::object_refs::collect_goal_expr_roots(
                &loaded.game,
                predicate,
                &mut roots,
            );
        }
        SolverCollectSelector2::Maximize(value) => {
            puzzle_solver::object_refs::collect_goal_value_roots(&loaded.game, value, &mut roots);
        }
    }
    if let Some(lose) = explicit_lose {
        puzzle_solver::object_refs::collect_goal_expr_roots(&loaded.game, lose, &mut roots);
    } else if let Some(lose) = &loaded.lose {
        puzzle_solver::object_refs::collect_goal_expr_roots(&loaded.game, &lose.expr, &mut roots);
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
    solver_model_and_state_slicer_from_roots(loaded, initial, roots)
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
        QueryExprOf::AllOnDistance { subjects, covers } => {
            roots.extend(subjects.iter().copied());
            roots.extend(covers.iter().copied());
        }
        QueryExprOf::Compare { left, .. } => collect_query_expr_roots(left, roots, collect_value),
    }
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
        state,
        program,
        InputId(0),
    )?)
}

#[cfg(all(feature = "solver", test))]
fn solve_current_state_with_budget_inner<O>(
    loaded: &LoadedGame,
    level_index: usize,
    initial: State,
    budget: SearchBudget,
    on_progress: Option<O>,
) -> Result<PuzzleSolutionResponse, AppError>
where
    O: FnMut(&State, SearchProgress),
{
    solve_current_state_with_goal_with_budget_inner(
        loaded,
        level_index,
        SolverGoal2::BuiltIn,
        None,
        initial,
        None,
        budget,
        on_progress,
    )
}

#[cfg(feature = "solver")]
fn solve_current_state_with_goal_with_budget_inner<O>(
    loaded: &LoadedGame,
    level_index: usize,
    goal: SolverGoal2,
    lose: Option<GoalExpr>,
    initial: State,
    initial_session: Option<GameSession>,
    budget: SearchBudget,
    on_progress: Option<O>,
) -> Result<PuzzleSolutionResponse, AppError>
where
    O: FnMut(&State, SearchProgress),
{
    let selected_game = loaded.compiled_game_for_level(level_index).ok_or_else(|| {
        AppError::Config(format!("solver level index out of range: {level_index}"))
    })?;
    let inputs = solver_inputs_for_program(loaded, selected_game.program());
    if inputs.is_empty() {
        return Err(AppError::Config("no model inputs available".to_string()));
    }

    let explicit_goal = match &goal {
        SolverGoal2::BuiltIn => None,
        SolverGoal2::Expr(goal) => Some(goal),
        SolverGoal2::ExactState(_) => None,
    };
    let exact_goal = match &goal {
        SolverGoal2::ExactState(goal) => Some(goal),
        SolverGoal2::BuiltIn | SolverGoal2::Expr(_) => None,
    };
    let (solver_model, state_slicer) = if initial_session.is_some() {
        (loaded.clone(), puzzle_solver::SolverStateSlicer::new())
    } else {
        solver_model_and_state_slicer(loaded, &initial, exact_goal, explicit_goal, lose.as_ref())
    };
    let game = Arc::new(solver_model.game.clone());
    let solver_loaded = Arc::new(solver_model);
    let projected_initial = state_slicer.project_state(&initial);
    let projected_exact_goal = exact_goal.map(|goal| state_slicer.project_state(goal));
    let exact_heuristic = projected_exact_goal
        .as_ref()
        .map(|goal| ExactStateHeuristic::new(&game, &projected_initial, goal));
    let domain_goal = match &goal {
        SolverGoal2::BuiltIn => GridSearchGoal::LevelCompletion,
        SolverGoal2::Expr(_) | SolverGoal2::ExactState(_) => {
            let goal_for_domain = goal.clone();
            let goal_expr_game = game.clone();
            GridSearchGoal::StatePredicate(Box::new(move |state: &State| match &goal_for_domain {
                SolverGoal2::Expr(goal) => eval_goal_expr(&goal_expr_game, state, goal),
                SolverGoal2::ExactState(_) => projected_exact_goal
                    .as_ref()
                    .is_some_and(|goal| state == goal),
                SolverGoal2::BuiltIn => unreachable!("built-in goal has its own matcher"),
            }))
        }
    };
    let mut domain = GridPuzzleDomain::<2, Size2>::with_goal(
        solver_loaded,
        level_index,
        inputs,
        state_slicer,
        domain_goal,
    );
    let replay_session = initial_session.clone();
    let solver_initial = match initial_session {
        Some(session) => domain.initial_session(session),
        None => domain.initial_state(initial.clone()),
    }
    .map_err(|error| AppError::Config(format!("{error:?}")))?;
    let score_game = loaded.clone();
    let deadend_game = loaded.clone();
    let score_goal = goal.clone();
    let lose_game = loaded.clone();
    let lose_expr_game = game.clone();
    let replay_game = loaded.clone();
    solve_domain_with_observations(
        &mut domain,
        solver_initial,
        budget,
        move |state| {
            let target_score = match &score_goal {
                SolverGoal2::BuiltIn | SolverGoal2::Expr(_) => 0,
                SolverGoal2::ExactState(_) => exact_heuristic
                    .as_ref()
                    .map_or(0, |heuristic| heuristic.score(state.observation_state())),
            };
            target_score + solver_strategy_score(&score_game, state.observation_state())
        },
        move |state| {
            let deadend = solver_has_deadend(&deadend_game, state.observation_state());
            let lose = if let Some(lose) = &lose {
                eval_goal_expr(&lose_expr_game, state.observation_state(), lose)
            } else {
                lose_game.is_lose_complete(state.observation_state())
            };
            deadend || lose
        },
        |state| state.observation_state().clone(),
        on_progress,
        move |solution_inputs| {
            solution_steps(
                &replay_game,
                level_index,
                initial,
                replay_session,
                solution_inputs,
            )
        },
    )
}

#[cfg(feature = "solver")]
fn solve_current_state_collect_with_budget_inner<O>(
    loaded: &LoadedGame,
    level_index: usize,
    selector: SolverCollectSelector2,
    lose: Option<GoalExpr>,
    initial: State,
    initial_session: Option<GameSession>,
    budget: SearchBudget,
    max_results: usize,
    on_progress: Option<O>,
) -> Result<PuzzleCollectResponse, AppError>
where
    O: FnMut(&State, SearchProgress),
{
    let selected_game = loaded.compiled_game_for_level(level_index).ok_or_else(|| {
        AppError::Config(format!("solver level index out of range: {level_index}"))
    })?;
    let inputs = solver_inputs_for_program(loaded, selected_game.program());
    if inputs.is_empty() {
        return Err(AppError::Config("no model inputs available".to_string()));
    }
    if max_results == 0 {
        return Err(AppError::Config(
            "solver collect maxResults must be greater than zero".to_string(),
        ));
    }

    let (solver_model, state_slicer) = if initial_session.is_some() {
        (loaded.clone(), puzzle_solver::SolverStateSlicer::new())
    } else {
        solver_game_and_state_slicer_for_collect(loaded, &initial, &selector, lose.as_ref())
    };
    let game = Arc::new(solver_model.game.clone());
    let solver_loaded = Arc::new(solver_model);
    let mut domain = GridPuzzleDomain::<2, Size2>::with_state_slicer(
        solver_loaded,
        level_index,
        inputs,
        state_slicer,
        |_state: &State| false,
    );
    let solver_initial = match initial_session {
        Some(session) => domain.initial_session(session),
        None => domain.initial_state(initial),
    }
    .map_err(|error| AppError::Config(format!("{error:?}")))?;
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
        move |state| {
            collect_priority_score(&score_game, state.observation_state(), &score_selector)
        },
        move |state| {
            let deadend = solver_has_deadend(loaded, state.observation_state());
            let lose = if let Some(lose) = &lose {
                eval_goal_expr(&lose_expr_game, state.observation_state(), lose)
            } else {
                lose_game.is_lose_complete(state.observation_state())
            };
            deadend || lose
        },
        |search_match| {
            let state = search_match.state.observation_state().clone();
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
            let observation = state.observation_state().clone();
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
    game: &LoadedGame,
    level_index: usize,
    state: State,
    session: Option<GameSession>,
    inputs: &[InputId],
) -> Result<Vec<PuzzleSolutionStep>, AppError> {
    let mut session = match session {
        Some(session) => puzzle_play::HeadlessSession::from_game_session(session, level_index)?,
        None => puzzle_play::HeadlessSession::from_level_state(game, level_index, state)?,
    };
    let mut steps = Vec::with_capacity(inputs.len() + 1);
    steps.push(SolutionStep {
        index: 0,
        input: None,
        state: session.state().clone(),
    });

    for (index, input) in inputs.iter().enumerate() {
        session.apply_input(game, *input)?;
        steps.push(SolutionStep {
            index: index + 1,
            input: Some(*input),
            state: session.observation_state().clone(),
        });
    }

    Ok(steps)
}

#[cfg(feature = "solver")]
fn solve_current_grid_session_with_budget<const D: usize, Size: GridSize<D>>(
    model: &LoadedGridGame<D, Size>,
    level_index: usize,
    session: puzzle_play::GridGameSession<D, Size>,
    budget: SearchBudget,
) -> Result<GridSolutionResponse<D, Size>, AppError> {
    let initial = session.state().clone();
    solve_current_grid_state_with_budget_inner(
        model,
        level_index,
        initial,
        Some(session),
        budget,
        None::<fn(&GridState<D, Size>, SearchProgress)>,
    )
}

#[cfg(feature = "solver")]
fn grid_play_session_from_state<const D: usize, Size: GridSize<D>>(
    model: &LoadedGridGame<D, Size>,
    level_index: usize,
    state: GridState<D, Size>,
    materialize_level_start: bool,
) -> Result<puzzle_play::GridGameSession<D, Size>, AppError> {
    let mut session = puzzle_play::GridGameSession::new(model);
    session
        .start_level_from_state(model, level_index, state, materialize_level_start)
        .map_err(|error| AppError::Config(format!("{error:?}")))?;
    Ok(session)
}

#[cfg(feature = "solver")]
fn solve_current_grid_state_with_budget_inner<const D: usize, Size: GridSize<D>, O>(
    model: &LoadedGridGame<D, Size>,
    level_index: usize,
    initial: GridState<D, Size>,
    initial_session: Option<puzzle_play::GridGameSession<D, Size>>,
    budget: SearchBudget,
    on_progress: Option<O>,
) -> Result<GridSolutionResponse<D, Size>, AppError>
where
    O: FnMut(&GridState<D, Size>, SearchProgress),
{
    let inputs = solver_inputs_for_grid_model(&model.inputs);
    if inputs.is_empty() {
        return Err(AppError::Config("no model inputs available".to_string()));
    }
    let _goal = model
        .goal
        .clone()
        .ok_or_else(|| AppError::Config("solver requires win_conditions".to_string()))?;

    if level_index >= model.levels.len() {
        return Err(AppError::Config(format!(
            "solver level index out of range: {level_index}"
        )));
    }
    let (solver_model, state_slicer) = if initial_session.is_some() {
        (model.clone(), puzzle_solver::SolverStateSlicer::new())
    } else {
        solver_model_and_state_slicer(model, &initial, None, None, None)
    };
    let game = Arc::new(solver_model.game.clone());
    let mut domain = GridPuzzleDomain::<D, Size>::with_state_slicer_for_level_completion(
        Arc::new(solver_model),
        level_index,
        inputs,
        state_slicer,
    );
    let replay_session = initial_session.clone();
    let solver_initial = match initial_session {
        Some(session) => domain.initial_session(session),
        None => domain.initial_state(initial.clone()),
    }
    .map_err(|error| AppError::Config(format!("{error:?}")))?;
    let replay_model = model.clone();
    let score_game = Arc::clone(&game);
    let score_strategy = model.solver_strategy.clone();
    let deadend_game = Arc::clone(&game);
    let deadend_strategy = model.solver_strategy.clone();
    solve_domain_with_observations(
        &mut domain,
        solver_initial,
        budget,
        move |state| {
            solver_strategy_score_for_grid(&score_game, &score_strategy, state.observation_state())
        },
        move |state| {
            solver_has_deadend_for_grid(&deadend_game, &deadend_strategy, state.observation_state())
        },
        |state| state.observation_state().clone(),
        on_progress,
        move |solution_inputs| {
            grid_solution_steps(
                &replay_model,
                level_index,
                initial,
                replay_session,
                solution_inputs,
            )
        },
    )
}

#[cfg(feature = "solver")]
fn grid_solution_steps<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    level_index: usize,
    state: GridState<D, Size>,
    session: Option<puzzle_play::GridGameSession<D, Size>>,
    inputs: &[InputId],
) -> Result<Vec<GridSolutionStep<D, Size>>, AppError> {
    let mut session = match session {
        Some(session) => puzzle_play::GridHeadlessSession::from_game_session(session, level_index),
        None => puzzle_play::GridHeadlessSession::from_level_state(game, level_index, state),
    }
    .map_err(|error| AppError::Config(format!("{error:?}")))?;
    let mut steps = Vec::with_capacity(inputs.len() + 1);
    steps.push(SolutionStep {
        index: 0,
        input: None,
        state: session.state().clone(),
    });

    for (index, input) in inputs.iter().enumerate() {
        session
            .apply_input(game, *input)
            .map_err(|error| AppError::Config(format!("{error:?}")))?;
        steps.push(SolutionStep {
            index: index + 1,
            input: Some(*input),
            state: session.observation_state().clone(),
        });
    }

    Ok(steps)
}

#[cfg(feature = "solver")]
fn solver_strategy_score(loaded: &LoadedGame, state: &State) -> i64 {
    solver_strategy_score_for_grid(&loaded.game, &loaded.solver_strategy, state)
}

#[cfg(feature = "solver")]
fn solver_has_deadend(loaded: &LoadedGame, state: &State) -> bool {
    solver_has_deadend_for_grid(&loaded.game, &loaded.solver_strategy, state)
}

#[cfg(feature = "solver")]
fn solver_strategy_score_for_grid<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    strategy: &GridSolverStrategy<D>,
    state: &GridState<D, Size>,
) -> i64 {
    solver_strategy_score_with(strategy, |value| {
        solver_query_expr_value_for_grid(game, state, value)
    })
}

#[cfg(feature = "solver")]
fn solver_has_deadend_for_grid<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    strategy: &GridSolverStrategy<D>,
    state: &GridState<D, Size>,
) -> bool {
    strategy.has_deadend_with(|query| solver_query_expr_value_for_grid(game, state, query) != 0)
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
fn solver_query_expr_value_for_grid<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    value: &puzzle_lang::GridQueryExpr<D>,
) -> i64 {
    solver_query_expr_value_with(
        value,
        &mut |variable| {
            state
                .variable_value(variable)
                .expect("query variable was resolved during lowering")
        },
        &mut |kind| eval_condition_kind(game, state, kind, None, None),
        &mut |from, to| solver_strategy_distance_for_grid(game, state, from, to),
        &mut |subjects, covers| all_objects_on_score_for_grid(game, state, subjects, covers),
    )
}

#[cfg(feature = "solver")]
fn solver_query_expr_value_with<
    Object,
    Value,
    Variable,
    EvalVariable,
    EvalValue,
    EvalDistance,
    EvalAllOnDistance,
>(
    value: &QueryExprOf<Object, Value, Variable>,
    eval_variable: &mut EvalVariable,
    eval_value: &mut EvalValue,
    eval_distance: &mut EvalDistance,
    eval_all_on_distance: &mut EvalAllOnDistance,
) -> i64
where
    Variable: Copy,
    EvalVariable: FnMut(Variable) -> i64,
    EvalValue: FnMut(&Value) -> i64,
    EvalDistance: FnMut(&[Object], &[Object]) -> i64,
    EvalAllOnDistance: FnMut(&[Object], &[Object]) -> i64,
{
    match value {
        QueryExprOf::Variable(variable) => eval_variable(*variable),
        QueryExprOf::Value(kind) => eval_value(kind),
        QueryExprOf::Distance { from, to } => eval_distance(from, to),
        QueryExprOf::AllOnDistance { subjects, covers } => eval_all_on_distance(subjects, covers),
        QueryExprOf::Compare { left, op, right } => {
            let left = solver_query_expr_value_with(
                left,
                eval_variable,
                eval_value,
                eval_distance,
                eval_all_on_distance,
            );
            if compare_i64(left, *op, *right) { 1 } else { 0 }
        }
    }
}

#[cfg(feature = "solver")]
fn solver_strategy_distance_for_grid<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    from: &[ObjectId],
    to: &[ObjectId],
) -> i64 {
    let from_positions = selector_object_positions_for_grid(game, state, from);
    let to_positions = selector_object_positions_for_grid(game, state, to);
    let fallback = state.size.axes().into_iter().map(i64::from).sum();
    from_positions
        .iter()
        .flat_map(|a| to_positions.iter().map(move |b| grid_manhattan(*a, *b)))
        .min()
        .unwrap_or(fallback)
}

#[cfg(feature = "solver")]
fn selector_object_positions_for_grid<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    objects: &[ObjectId],
) -> Vec<GridCoord<D>> {
    let mut positions = Vec::new();
    for object in objects {
        for slot in state.object_positions(*object) {
            let Some(position) = state.slot_coord(*slot) else {
                continue;
            };
            if !positions.contains(&position) && state.has_object_at(game, position, *object) {
                positions.push(position);
            }
        }
    }
    positions
}

#[cfg(feature = "solver")]
fn grid_manhattan<const D: usize>(a: GridCoord<D>, b: GridCoord<D>) -> i64 {
    a.axes()
        .into_iter()
        .zip(b.axes())
        .map(|(left, right)| i64::from(left.abs_diff(right)))
        .sum()
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
            .filter(|object| {
                object_positions_for_exact(initial, *object)
                    != object_positions_for_exact(goal, *object)
            })
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
    let mut score = current_positions.len().abs_diff(goal_positions.len()) as i64 * fallback;
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
    }
}

#[cfg(feature = "solver")]
fn all_objects_on_score_for_grid<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    subjects: &[ObjectId],
    covers: &[ObjectId],
) -> i64 {
    let cover_positions = selector_object_positions_for_grid(game, state, covers);
    let fallback = state.size.axes().into_iter().map(i64::from).sum();
    selector_object_positions_for_grid(game, state, subjects)
        .into_iter()
        .filter(|position| {
            !covers
                .iter()
                .any(|cover| state.has_object_at(game, *position, *cover))
        })
        .map(|position| {
            cover_positions
                .iter()
                .map(|cover| grid_manhattan(position, *cover))
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
        ComparisonOp::Greater => expected.saturating_sub(current).max(0).saturating_add(1),
        ComparisonOp::GreaterEq => expected.saturating_sub(current).max(0),
        ComparisonOp::Less => current.saturating_sub(expected).max(0).saturating_add(1),
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
    }
}

#[cfg(feature = "solver")]
fn goal_condition_value_kind(game: &CompiledGame, state: &State, kind: &ConditionValueKind) -> i64 {
    eval_condition_kind(game, state, kind, None, None)
}

#[cfg(all(test, feature = "solver"))]
mod solver_goal_score_tests {
    use super::*;

    #[test]
    fn generic_no_pattern_goal_scores_direct_pattern_violations_only() {
        let source = r#"
title = no_pattern_score

puzzle board {
slots {
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

        assert_eq!(
            solver_strategy_score(&loaded, &loaded.levels[0].initial_state),
            2
        );
    }

    #[test]
    fn all_objects_on_goal_scores_from_quantified_subjects_to_covers() {
        let source = r#"
title = all_on_score

puzzle board {
slots {
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

        assert!(!loaded.is_goal_complete(&loaded.levels[0].initial_state));
        assert_eq!(
            solver_strategy_score(&loaded, &loaded.levels[0].initial_state),
            3
        );
    }

    #[test]
    fn all_objects_on_goal_generates_the_same_subject_first_score_in_3d() {
        let model = parse_grid_model_for_solver(
            r#"
puzzle board {
dimension = 3
slots {
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
        let state = model.levels[0].initial_state.clone();

        assert_eq!(
            solver_strategy_score_for_grid(&model.game, &model.solver_strategy, &state),
            2
        );
    }
}
