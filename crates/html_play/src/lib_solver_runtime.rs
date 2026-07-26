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
    runtime: puzzle_game_runtime::RuntimeSession,
    #[cfg(not(target_arch = "wasm32"))]
    visual_images: DecodedVisualImageCatalog,
    game_css: String,
    game_visuals_js: String,
    solver: SolverConfig,
    #[cfg(feature = "solver")]
    solver_service: puzzle_solver_runtime::SolverService,
    #[cfg(feature = "solver")]
    solver_artifact_id: String,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RenderMomentRequest {
    render_scene: puzzle_runtime_contract::RuntimeResolvedRenderScene,
    moment: puzzle_runtime_contract::RuntimeResolvedRenderMoment,
}

struct EditorPreviewState {
    standalone_export: StandaloneRuntimeExport<puzzle_lang::LoadedDocument>,
    runtime: puzzle_game_runtime::RuntimeSession,
    source: String,
    puzzle_path: String,
    game_css: String,
    game_visuals_js: String,
}

impl EditorPreviewState {
    fn new(
        document: puzzle_lang::LoadedDocument,
        source: String,
        puzzle_path: String,
        visual_images: EncodedVisualImageBundle,
        game_css: String,
        game_visuals_js: String,
    ) -> Result<Self, String> {
        let mut runtime = puzzle_game_runtime::RuntimeSession::from_document(document.clone())?;
        runtime.set_progress_persistence_enabled(false);
        let progress_storage = standalone_progress_storage(&document);
        Ok(Self {
            standalone_export: StandaloneRuntimeExport::new(
                document,
                visual_images,
                progress_storage,
            ),
            runtime,
            source,
            puzzle_path,
            game_css,
            game_visuals_js,
        })
    }
}

impl ServerState {
    fn new(
        document: puzzle_lang::LoadedDocument,
        _loaded: LoadedGame,
        #[cfg(not(target_arch = "wasm32"))]
        visual_images: DecodedVisualImageCatalog,
        game_css: String,
        game_visuals_js: String,
        solver: SolverConfig,
    ) -> Self {
        let mut runtime = puzzle_game_runtime::RuntimeSession::from_document(document.clone())
            .expect("parsed HTML host document must construct its runtime session");
        runtime.set_progress_persistence_enabled(false);
        #[cfg(feature = "solver")]
        let loaded = Arc::new(_loaded);
        #[cfg(feature = "solver")]
        let (solver_service, solver_artifact_id) = {
            let mut service = puzzle_solver_runtime::SolverService::new();
            let prepared = service.prepare_loaded_game(Arc::clone(&loaded), 0);
            (service, prepared.artifact_id)
        };
        Self {
            runtime,
            #[cfg(not(target_arch = "wasm32"))]
            visual_images,
            game_css,
            game_visuals_js,
            solver,
            #[cfg(feature = "solver")]
            solver_service,
            #[cfg(feature = "solver")]
            solver_artifact_id,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn snapshot_json(&self) -> Result<(String, usize), String> {
        live_server_snapshot_json(self.runtime.development_snapshot())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn resolve_render_moment_json(&self, request_json: &[u8]) -> Result<String, String> {
        resolve_render_moment_request_json(&self.visual_images, request_json)
    }

    #[cfg(all(feature = "solver", not(target_arch = "wasm32")))]
    fn solve_json(&mut self) -> Result<String, AppError> {
        let (_, session) = self.runtime.solver_session_2d().ok_or_else(|| {
            AppError::Config("solver requires a two-dimensional runtime session".to_string())
        })?;
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
                self.solver.max_nodes,
                self.solver.max_duration,
                0,
            )
            .map_err(AppError::Config)?;
        serde_json::to_string(&response)
            .map_err(|error| AppError::Config(format!("solver response JSON failed: {error}")))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_render_moment_request_json(
    visual_images: &DecodedVisualImageCatalog,
    request_json: &[u8],
) -> Result<String, String> {
    let request = serde_json::from_slice::<RenderMomentRequest>(request_json)
        .map_err(|error| format!("invalid render moment request: {error}"))?;
    let frame = puzzle_presentation::resolve_render_moment(
        &request.render_scene,
        visual_images,
        &request.moment,
    )
    .map_err(|error| format!("render moment resolution failed: {error:?}"))?;
    serde_json::to_string(&frame)
        .map_err(|error| format!("render frame serialization failed: {error}"))
}
