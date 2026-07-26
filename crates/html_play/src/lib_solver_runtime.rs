#[derive(Clone, Copy, Debug)]
struct SolverConfig {
    #[cfg(feature = "solver")]
    max_depth: u32,
    #[cfg(feature = "solver")]
    max_stored_nodes: usize,
    #[cfg(feature = "solver")]
    max_duration: Duration,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            #[cfg(feature = "solver")]
            max_depth: 128,
            #[cfg(feature = "solver")]
            max_stored_nodes: 1_000_000,
            #[cfg(feature = "solver")]
            max_duration: Duration::from_secs(5),
        }
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
fn parse_solver_stored_nodes_arg(
    solver: &mut SolverConfig,
    args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    solver.max_stored_nodes = parse_arg(args, "--solver-stored-nodes")?;
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "solver")))]
fn parse_solver_stored_nodes_arg(
    _solver: &mut SolverConfig,
    _args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    Err(AppError::Config(
        "--solver-stored-nodes requires the html-play solver feature".to_string(),
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
    loaded: Arc<LoadedGame>,
    runtime: RuntimeSession,
    source: String,
    puzzle_path: String,
    game_css: String,
    game_visuals_js: String,
    solver: SolverConfig,
    #[cfg(feature = "solver")]
    solver_service: puzzle_solver_runtime::SolverService,
    #[cfg(feature = "solver")]
    solver_artifact_id: String,
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
        let mut runtime = RuntimeSession::from_document(document.clone())
            .expect("server document was validated before runtime construction");
        runtime.set_progress_persistence_enabled(false);
        let loaded = Arc::new(loaded);
        #[cfg(feature = "solver")]
        let (solver_service, solver_artifact_id) = {
            let mut service = puzzle_solver_runtime::SolverService::new();
            let prepared = service.prepare_loaded_game(Arc::clone(&loaded), 0);
            (service, prepared.artifact_id)
        };
        Self {
            document,
            loaded,
            runtime,
            source,
            puzzle_path,
            game_css,
            game_visuals_js,
            solver,
            #[cfg(feature = "solver")]
            solver_service,
            #[cfg(feature = "solver")]
            solver_artifact_id,
        }
    }

    fn scene_json(&self) -> String {
        let snapshot: serde_json::Value = serde_json::from_str(&self.runtime.snapshot_json())
            .expect("runtime snapshot JSON should parse");
        serde_json::json!({ "scene": snapshot.get("scene").cloned().unwrap_or_default() })
            .to_string()
    }

    #[cfg(all(feature = "solver", not(target_arch = "wasm32")))]
    fn solve_json(&mut self) -> Result<String, AppError> {
        let (_, session) = self
            .runtime
            .solver_session_2d()
            .ok_or_else(|| AppError::Config("solver requires a 2d runtime session".to_string()))?;
        let level_index = session
            .active_level_index()
            .ok_or_else(|| AppError::Config("solver requires an active level".to_string()))?;
        let response = self
            .solver_service
            .solve_game_session_to_completion(
                &self.solver_artifact_id,
                level_index,
                session,
                self.solver.max_depth,
                self.solver.max_stored_nodes,
                self.solver.max_duration,
                0,
            )
            .map_err(AppError::Config)?;
        serde_json::to_string(&response)
            .map_err(|error| AppError::Config(format!("solver response JSON failed: {error}")))
    }
}
