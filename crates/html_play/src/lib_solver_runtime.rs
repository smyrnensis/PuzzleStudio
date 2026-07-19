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
    document: puzzle_lang::LoadedDocument,
    loaded: Arc<LoadedGame>,
    session: GameSession,
    source: String,
    puzzle_path: String,
    game_css: String,
    game_visuals_js: String,
    solver: SolverConfig,
    #[cfg(feature = "solver")]
    solver_service: puzzle_solver_runtime::SolverService,
    #[cfg(feature = "solver")]
    solver_artifact_id: String,
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
        let loaded = Arc::new(loaded);
        let session = GameSession::new(&loaded);
        #[cfg(feature = "solver")]
        let (solver_service, solver_artifact_id) = {
            let mut service = puzzle_solver_runtime::SolverService::new();
            let prepared = service.prepare_loaded_game(Arc::clone(&loaded), 0);
            (service, prepared.artifact_id)
        };
        Self {
            document,
            loaded,
            session,
            source,
            puzzle_path,
            game_css,
            game_visuals_js,
            solver,
            #[cfg(feature = "solver")]
            solver_service,
            #[cfg(feature = "solver")]
            solver_artifact_id,
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
        push_json_bool(&mut out, "busy", self.session.is_waiting());
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

    #[cfg(all(feature = "solver", not(target_arch = "wasm32")))]
    fn solve_json(&mut self) -> Result<String, AppError> {
        let level_index = self
            .session
            .active_level_index()
            .ok_or_else(|| AppError::Config("solver requires an active level".to_string()))?;
        let response = self
            .solver_service
            .solve_game_session_to_completion(
                &self.solver_artifact_id,
                level_index,
                self.session.clone(),
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
