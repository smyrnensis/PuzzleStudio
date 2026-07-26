use std::collections::{BTreeMap, HashMap};

use puzzle_core::{
    GridCompiledGame, GridSize, GridState, InputId, ObjectId, Size2, Size3, State as PuzzleState,
};
#[cfg(feature = "editor-debug")]
use puzzle_core::{
    GridCoord, GridPatch, MarkId, MarkValueMatch as CoreMarkValueMatch, PatchOp, RuleId,
    TransitionCommand, VariableId, VariableUpdateOp,
};
use puzzle_lang::{
    ArrowKey, KeyTrigger, Level, LevelId, LoadedDocument, LoadedDocumentModel, LoadedGame,
    LoadedGridGame, ResourceSelection, STANDARD_MESSAGE_COMPONENT, STANDARD_MESSAGE_DISMISS_EVENT,
    STANDARD_MESSAGE_TEXT_PROPERTY, SceneComponent, SceneDef, SceneEffect, SceneExpr,
    SceneLayoutDef, SceneTextContent, SceneTextRoleDef, SceneValue, ThemeDef, ViewportModeDef,
    ViewportProjectionDef, ViewportSizeDef, VisualFitMode as LoadedVisualFitMode, VisualKind,
    VisualSampling as LoadedVisualSampling, VisualSpace as LoadedVisualSpace,
    VisualTransform as LoadedVisualTransform,
};
#[cfg(feature = "editor-debug")]
use puzzle_play::GridTransitionTrace;
use puzzle_play::{
    GameSession, GridGameSession, ProgressSaveData, presentation_events_contract,
    runtime_sounds_def, scene_value_to_string,
};
use puzzle_presentation::{
    VisualOrderRef, VisualPriorityRef, cell_render_order_2d, resolve_object_priority,
    resolve_pixel_frame, resolve_visual_affine, resolve_voxel_frame,
};
use puzzle_runtime_contract::{
    RuntimeChoiceDirection, RuntimePresentationEvent, RuntimeProgressSaveRequest,
    RuntimePuzzle3Resources, RuntimePuzzle3Snapshot, RuntimeResolvedCompositionGroup,
    RuntimeResolvedFitMode, RuntimeResolvedPlayback, RuntimeResolvedRenderCell,
    RuntimeResolvedRenderInstance, RuntimeResolvedRenderScene, RuntimeResolvedSampling,
    RuntimeResolvedVisualClip, RuntimeResolvedVisualFrame, RuntimeResolvedVisualLayout,
    RuntimeStateSnapshot2d, RuntimeStateSnapshot3d, RuntimeVisualComposition, RuntimeVisualSpace,
    RuntimeVisualTransform, STANDALONE_RUNTIME_EXPORT_VERSION, SessionAction, SolverStateSnapshot,
    StandaloneRuntimeExport,
};
use puzzle_session_contract::{
    RuntimeAnimationSettings, RuntimeAuthoredComponentProjection, RuntimeComponentPresentation,
    RuntimeGridRender2d, RuntimeInputBinding, RuntimeInputBufferSettings, RuntimeKeyBinding,
    RuntimeLevelRecord, RuntimeMusicSound, RuntimePuzzle2Cell, RuntimePuzzle2Layer,
    RuntimePuzzle2Resources, RuntimePuzzle2Screen, RuntimePuzzle2Settings, RuntimePuzzle2Snapshot,
    RuntimeRegion2d, RuntimeRender2d, RuntimeRendererState, RuntimeResolvedEventBinding,
    RuntimeResolvedScene, RuntimeResolvedSceneComponent, RuntimeResourceSelection,
    RuntimeResourceSelectionMode, RuntimeSessionSnapshot, RuntimeSfxSound, RuntimeSounds,
    RuntimeSurface, RuntimeSurfaceComponent, RuntimeTheme, RuntimeTweenSettings,
    RuntimeViewportDimension, RuntimeViewportMode2d, RuntimeViewportSize2d, ordered_scene_values,
};
use serde_json::{Value, json};

pub struct RuntimeSession {
    model: Box<dyn StandaloneSessionModel>,
    revision: u64,
    queued_input: Option<String>,
    progress_persistence_enabled: bool,
    has_persisted_progress: bool,
    pending_progress_save: Option<RuntimeProgressSaveRequest>,
    next_progress_request_id: u32,
    last_progress_save: ProgressSaveData,
}

trait StandaloneSessionModel {
    fn snapshot(
        &self,
        revision: u64,
        has_progress_save: bool,
        presentation_events: &[RuntimePresentationEvent],
    ) -> RuntimeSessionSnapshot;
    fn take_presentation_events(&mut self) -> Vec<RuntimePresentationEvent>;
    fn is_waiting(&self) -> bool;
    fn accepts_model_input(&self) -> bool;
    fn queues_input_while_waiting(&self) -> bool;
    fn has_input_name(&self, input_name: &str) -> bool;
    fn resume_wait(&mut self) -> Result<(), String>;
    fn apply_component_event(&mut self, instance: &str, event: &str) -> Result<(), String>;
    fn apply_input_name(&mut self, input_name: &str) -> Result<(), String>;
    fn apply_choice_move(&mut self, direction: RuntimeChoiceDirection) -> Result<(), String>;
    fn apply_choice_activate(&mut self, index: Option<usize>) -> Result<(), String>;
    #[cfg(feature = "editor-debug")]
    fn apply_debug_input_name_json(&mut self, input_name: &str) -> Result<String, String>;
    fn apply_command_name(&mut self, command_name: &str) -> Result<(), String>;
    fn apply_scene_effect(&mut self, effect: &SceneEffect) -> Result<(), String>;
    fn undo(&mut self);
    fn redo(&mut self);
    fn restart(&mut self) -> Result<(), String>;
    fn next_level(&mut self) -> Result<(), String>;
    fn previous_level(&mut self) -> Result<(), String>;
    fn goto_level(&mut self, level: usize) -> Result<(), String>;
    fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String>;
    fn progress_save_data(&self) -> ProgressSaveData;
    fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String>;
    fn resolve_scene_presentation(
        &self,
        scene_name: &str,
        state_override: &HashMap<String, SceneValue>,
        has_progress_save: bool,
    ) -> Result<RuntimeResolvedScene, String>;
    fn solver_session_2d(&self) -> Option<(LoadedGame, GameSession)>;
    fn renderer_state(&self) -> Option<RuntimeRendererState>;
}

struct GridSessionRuntime<const D: usize, Size: GridSize<D>, Projection> {
    loaded: LoadedGridGame<D, Size>,
    session: GridGameSession<D, Size>,
    projection: Projection,
}

trait GridSessionProjection<const D: usize, Size: GridSize<D>> {
    fn decode_state_json(
        &self,
        game: &GridCompiledGame<D>,
        state_json: &str,
    ) -> Result<GridState<D, Size>, String>;

    fn snapshot_grid(
        &self,
        loaded: &LoadedGridGame<D, Size>,
        session: &GridGameSession<D, Size>,
    ) -> ProjectedGridSnapshot;

    fn solver_state(&self, state: &GridState<D, Size>) -> SolverStateSnapshot;

    fn solver_session_2d(
        &self,
        _loaded: &LoadedGridGame<D, Size>,
        _session: &GridGameSession<D, Size>,
    ) -> Option<(LoadedGame, GameSession)> {
        None
    }

    fn renderer_state(
        &self,
        _loaded: &LoadedGridGame<D, Size>,
        _session: &GridGameSession<D, Size>,
    ) -> Option<RuntimeRendererState> {
        None
    }

    fn puzzle3_authoring_resources(
        &self,
        _loaded: &LoadedGridGame<D, Size>,
    ) -> Option<RuntimePuzzle3Resources> {
        None
    }
}

struct ProjectedGridSnapshot {
    scene: Option<RuntimeRendererState>,
    scene_puzzle_state: BTreeMap<String, RuntimeRendererState>,
    scene_layers: Vec<ProjectedSceneLayer>,
}

#[derive(Clone)]
struct ProjectedSceneLayer {
    id: String,
    projection: RuntimeAuthoredComponentProjection,
}

#[derive(Default)]
struct CanvasProjection;

struct SpatialProjection {
    resources: RuntimePuzzle3Resources,
}

impl RuntimeSession {
    pub fn from_source(source: &str, puzzle_path: &str) -> Result<Self, String> {
        let document = puzzle_lang::parse_game_for_path(source, puzzle_path)
            .map_err(|error| error.to_string())?;
        Self::from_document(document)
    }

    pub fn from_export_json(export_json: &str) -> Result<Self, String> {
        let export: StandaloneRuntimeExport<LoadedDocument> = serde_json::from_str(export_json)
            .map_err(|error| format!("invalid standalone runtime export: {error}"))?;
        let runtime_bundle = export.runtime_loaded_document;
        if runtime_bundle.version != STANDALONE_RUNTIME_EXPORT_VERSION {
            return Err(format!(
                "unsupported runtimeLoadedDocument version: {}",
                runtime_bundle.version
            ));
        }
        Self::from_document(runtime_bundle.document)
    }

    pub fn from_document(document: LoadedDocument) -> Result<Self, String> {
        let model = standalone_session_model(document)?;
        let last_progress_save = model.progress_save_data();
        Ok(Self {
            model,
            revision: 0,
            queued_input: None,
            progress_persistence_enabled: true,
            has_persisted_progress: false,
            pending_progress_save: None,
            next_progress_request_id: 1,
            last_progress_save,
        })
    }

    pub fn snapshot_json(&self) -> String {
        self.snapshot_json_with_events(&[])
    }

    pub fn snapshot(&self) -> RuntimeSessionSnapshot {
        self.snapshot_with_events(&[])
    }

    pub fn resolve_scene_presentation_json(
        &self,
        scene_name: &str,
        state_json: &str,
    ) -> Result<String, String> {
        let state_override = if state_json.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(state_json)
                .map_err(|error| format!("invalid scene preview state: {error}"))?
        };
        let state_override = scene_values_from_json(&state_override)?;
        let presentation = self.model.resolve_scene_presentation(
            scene_name,
            &state_override,
            self.has_persisted_progress,
        )?;
        puzzle_presentation_json::resolved_scene_to_value(&presentation)
            .and_then(|presentation| serde_json::to_string(&presentation))
            .map_err(|error| format!("scene presentation could not be serialized: {error}"))
    }

    fn response_snapshot(&mut self) -> RuntimeSessionSnapshot {
        let events = self.model.take_presentation_events();
        self.snapshot_with_events(&events)
    }

    fn snapshot_json_with_events(&self, events: &[RuntimePresentationEvent]) -> String {
        puzzle_presentation_json::to_string(&self.snapshot_with_events(events))
            .expect("snapshot JSON should serialize")
    }

    fn snapshot_with_events(&self, events: &[RuntimePresentationEvent]) -> RuntimeSessionSnapshot {
        self.model
            .snapshot(self.revision, self.has_persisted_progress, events)
    }

    pub fn dispatch(&mut self, action: SessionAction) -> Result<String, String> {
        #[cfg(feature = "editor-debug")]
        if let SessionAction::DebugInput { name } = &action {
            let response = self.model.apply_debug_input_name_json(name)?;
            self.revision = self.next_revision()?;
            return Ok(response);
        }
        let snapshot = self.dispatch_typed(action)?;
        puzzle_presentation_json::to_string(&snapshot)
            .map_err(|error| format!("snapshot JSON could not be serialized: {error}"))
    }

    pub fn dispatch_typed(
        &mut self,
        action: SessionAction,
    ) -> Result<RuntimeSessionSnapshot, String> {
        if self.model.is_waiting() {
            if let SessionAction::Input { name } = &action {
                if !self.model.has_input_name(name) {
                    return Err(format!("unknown input: {name}"));
                }
                if self.model.queues_input_while_waiting() {
                    self.queued_input = Some(name.clone());
                    self.revision = self.next_revision()?;
                }
                return Ok(self.snapshot());
            }
            if !matches!(
                &action,
                SessionAction::Initialize
                    | SessionAction::Snapshot
                    | SessionAction::Resume
                    | SessionAction::ComponentEvent { .. }
            ) {
                return Err("session action is unavailable while a turn is waiting".to_string());
            }
        }

        if matches!(action, SessionAction::Initialize) {
            return Ok(self.response_snapshot());
        }
        if matches!(action, SessionAction::Snapshot) {
            return Ok(self.snapshot());
        }

        let should_persist = !matches!(
            action,
            SessionAction::DebugInput { .. } | SessionAction::ChoiceMove { .. }
        );
        match action {
            SessionAction::Initialize | SessionAction::Snapshot => {
                unreachable!("snapshot request returned above")
            }
            SessionAction::Resume => {
                self.model.resume_wait()?;
                if !self.model.is_waiting()
                    && let Some(input) = self.queued_input.take()
                {
                    self.model.apply_input_name(&input)?;
                }
            }
            SessionAction::ComponentEvent { instance, event } => {
                self.model.apply_component_event(&instance, &event)?;
            }
            SessionAction::Undo => {
                self.model.undo();
            }
            SessionAction::Redo => {
                self.model.redo();
            }
            SessionAction::Restart => {
                self.model.restart()?;
            }
            SessionAction::NextLevel => {
                self.model.next_level()?;
            }
            SessionAction::PreviousLevel => {
                self.model.previous_level()?;
            }
            SessionAction::GotoLevel { level } => {
                self.model.goto_level(level)?;
            }
            SessionAction::Input { name } => {
                self.model.apply_input_name(&name)?;
            }
            SessionAction::ChoiceMove { direction } => {
                self.model.apply_choice_move(direction)?;
            }
            SessionAction::ChoiceActivate { index } => {
                self.model.apply_choice_activate(index)?;
            }
            #[cfg(feature = "editor-debug")]
            SessionAction::DebugInput { .. } => {
                return Err("debug input has an editor-specific JSON result".to_string());
            }
            #[cfg(not(feature = "editor-debug"))]
            SessionAction::DebugInput { .. } => {
                return Err("debug input is unavailable in the player runtime".to_string());
            }
            SessionAction::Command { name } => {
                self.model.apply_command_name(&name)?;
            }
            SessionAction::SceneEffect { effect } => {
                self.model.apply_scene_effect(&effect)?;
            }
        }

        self.revision = self.next_revision()?;
        if should_persist {
            self.refresh_progress_save_request()?;
        }
        Ok(self.response_snapshot())
    }

    pub fn dispatch_json(&mut self, action_json: &str) -> Result<String, String> {
        let action: SessionAction = serde_json::from_str(action_json)
            .map_err(|error| format!("invalid session action: {error}"))?;
        self.dispatch(action)
    }

    #[cfg(feature = "editor-debug")]
    pub fn apply_debug_input_name_json(&mut self, input_name: &str) -> Result<String, String> {
        self.model.apply_debug_input_name_json(input_name)
    }

    pub fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String> {
        self.model
            .set_current_state_json(state_json, level_index, materialize_level_start)
    }

    pub fn renderer_state_json(&self) -> Result<String, String> {
        let state = self
            .model
            .renderer_state()
            .ok_or_else(|| "runtime model does not expose a 2D renderer state".to_string())?;
        puzzle_presentation_json::renderer_to_string(&state)
            .map_err(|error| format!("renderer state could not be serialized: {error}"))
    }

    pub fn progress_save_json(&self) -> String {
        serde_json::to_string(&self.model.progress_save_data())
            .expect("progress save JSON should serialize")
    }

    pub fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String> {
        self.model.restore_progress_save_json(save_json)?;
        self.last_progress_save = self.model.progress_save_data();
        self.pending_progress_save = None;
        self.has_persisted_progress = true;
        Ok(())
    }

    pub fn set_progress_persistence_enabled(&mut self, enabled: bool) {
        self.progress_persistence_enabled = enabled;
        if !enabled {
            self.pending_progress_save = None;
        }
    }

    pub fn progress_save_request(&self) -> Option<RuntimeProgressSaveRequest> {
        self.pending_progress_save.clone()
    }

    pub fn confirm_progress_save_written(&mut self, request_id: u32) -> Result<(), String> {
        let Some(request) = &self.pending_progress_save else {
            return Err("progress save acknowledgement has no pending request".to_string());
        };
        if request.request_id != request_id {
            return Err(format!(
                "progress save acknowledgement {request_id} is stale; pending request is {}",
                request.request_id
            ));
        }
        self.pending_progress_save = None;
        self.has_persisted_progress = true;
        Ok(())
    }

    pub fn confirm_progress_save_cleared(&mut self) {
        self.pending_progress_save = None;
        self.has_persisted_progress = false;
    }

    pub fn solver_session_2d(&self) -> Option<(LoadedGame, GameSession)> {
        self.model.solver_session_2d()
    }

    pub fn accepts_model_input(&self) -> bool {
        self.model.accepts_model_input()
    }

    fn next_revision(&self) -> Result<u64, String> {
        self.revision
            .checked_add(1)
            .ok_or_else(|| "runtime session revision counter exhausted".to_string())
    }

    fn refresh_progress_save_request(&mut self) -> Result<(), String> {
        if !self.progress_persistence_enabled {
            return Ok(());
        }
        let save = self.model.progress_save_data();
        if self.has_persisted_progress && save == self.last_progress_save {
            return Ok(());
        }
        self.last_progress_save = save.clone();
        let request_id = self.next_progress_request_id.max(1);
        self.next_progress_request_id = request_id
            .checked_add(1)
            .ok_or_else(|| "progress save request counter exhausted".to_string())?;
        self.pending_progress_save = Some(RuntimeProgressSaveRequest {
            request_id,
            save_json: serde_json::to_string(&save)
                .expect("progress save request JSON should serialize"),
        });
        Ok(())
    }
}

fn standalone_session_model(
    mut document: LoadedDocument,
) -> Result<Box<dyn StandaloneSessionModel>, String> {
    if document.models.len() != 1 {
        return Err(
            "a document with multiple puzzle worlds requires model-addressed session routing"
                .to_string(),
        );
    }
    match document.models.first().expect("single model was checked") {
        LoadedDocumentModel::Puzzle2d { game, .. } => game.validate_program_references()?,
        LoadedDocumentModel::Puzzle3d { game, .. } => game.validate_program_references()?,
    }
    match document.models.pop().expect("single model was checked") {
        LoadedDocumentModel::Puzzle2d { game: loaded, .. } => {
            let session = GridGameSession::try_new(&loaded)
                .map_err(|error| format!("failed to start initial level: {error:?}"))?;
            Ok(Box::new(GridSessionRuntime {
                session,
                loaded,
                projection: CanvasProjection,
            }))
        }
        LoadedDocumentModel::Puzzle3d {
            game: loaded,
            presentation,
            ..
        } => {
            let resources = puzzle_lang::runtime_puzzle3_resources(&loaded, &presentation)
                .map_err(|error| {
                    format!("failed to materialize Puzzle3 runtime view: {error:?}")
                })?;
            let session = GridGameSession::try_new(&loaded)
                .map_err(|error| format!("failed to start initial level: {error:?}"))?;
            Ok(Box::new(GridSessionRuntime {
                session,
                loaded,
                projection: SpatialProjection { resources },
            }))
        }
    }
}

impl<const D: usize, Size, Projection> StandaloneSessionModel
    for GridSessionRuntime<D, Size, Projection>
where
    Size: GridSize<D> + 'static,
    Projection: GridSessionProjection<D, Size> + 'static,
{
    fn snapshot(
        &self,
        revision: u64,
        has_progress_save: bool,
        presentation_events: &[RuntimePresentationEvent],
    ) -> RuntimeSessionSnapshot {
        let busy = self.session.is_waiting();
        let projected = self.projection.snapshot_grid(&self.loaded, &self.session);
        let surface = surface_snapshot(
            &self.loaded,
            &self.session,
            projected.scene_layers.clone(),
            has_progress_save,
        );
        let solver_state = self.projection.solver_state(self.session.state());
        RuntimeSessionSnapshot {
            revision,
            has_progress_save,
            sounds: sounds_snapshot(&self.loaded),
            theme: theme_snapshot(&self.loaded.theme),
            default_wait_ms: self.loaded.default_wait_ms,
            input_buffer: input_buffer_settings(&self.loaded),
            animation: animation_settings(&self.loaded),
            presentation_events: presentation_events.to_vec(),
            level_index: self.session.active_level_index(),
            level_count: self.loaded.levels.len(),
            levels: level_records(&self.loaded, &self.session),
            scene: projected.scene,
            accepts_model_input: self.session.accepts_model_input(&self.loaded),
            game_state: ordered_scene_values(self.session.session_values()),
            scene_state: scene_state(self.session.scene_state()),
            scene_puzzles: scene_puzzles(self.session.scene_state()),
            scene_puzzle_state: projected.scene_puzzle_state,
            puzzle3_authoring_resources: self
                .projection
                .puzzle3_authoring_resources(&self.loaded),
            surface,
            solver_state,
            selected_level_index: self.session.selected_level_index(),
            busy,
            can_undo: self.session.can_undo(),
            can_redo: self.session.can_redo(),
            inputs: inputs(&self.loaded),
            scenes: self.loaded.scenes.clone(),
        }
    }

    fn take_presentation_events(&mut self) -> Vec<RuntimePresentationEvent> {
        puzzle_presentation::resolve_presentation_events(
            presentation_events_contract::<D>(&self.session.take_presentation_events()),
            &runtime_visual_order(&self.loaded.visuals.order),
        )
        .expect("validated presentation animation channels must resolve")
    }

    fn is_waiting(&self) -> bool {
        self.session.is_waiting()
    }

    fn accepts_model_input(&self) -> bool {
        self.session.accepts_model_input(&self.loaded)
    }

    fn queues_input_while_waiting(&self) -> bool {
        self.loaded.input_buffer.queue_during_wait
    }

    fn has_input_name(&self, input_name: &str) -> bool {
        input_id_by_name(&self.loaded, input_name).is_some()
    }

    fn resume_wait(&mut self) -> Result<(), String> {
        self.session
            .resume_wait(&self.loaded)
            .map_err(|error| format!("{error:?}"))
    }

    fn apply_component_event(&mut self, instance: &str, event: &str) -> Result<(), String> {
        self.session
            .apply_component_event(&self.loaded, instance, event)
            .map_err(|error| format!("{error:?}"))
    }

    fn apply_input_name(&mut self, input_name: &str) -> Result<(), String> {
        let input = input_id_by_name(&self.loaded, input_name)
            .ok_or_else(|| format!("unknown input: {input_name}"))?;
        self.session
            .apply_input(&self.loaded, input)
            .map_err(|error| format!("{error:?}"))
    }

    fn apply_choice_move(&mut self, direction: RuntimeChoiceDirection) -> Result<(), String> {
        self.session
            .apply_choice_move(&self.loaded, direction)
            .map_err(|error| format!("{error:?}"))
    }

    fn apply_choice_activate(&mut self, index: Option<usize>) -> Result<(), String> {
        self.session
            .apply_choice_activate(&self.loaded, index)
            .map_err(|error| format!("{error:?}"))
    }

    #[cfg(feature = "editor-debug")]
    fn apply_debug_input_name_json(&mut self, input_name: &str) -> Result<String, String> {
        let input = input_id_by_name(&self.loaded, input_name)
            .ok_or_else(|| format!("unknown input: {input_name}"))?;
        self.session
            .apply_traced_input(&self.loaded, input)
            .map_err(|error| format!("{error:?}"))?;
        let debug = self.session.last_transition_trace().cloned();
        let presentation_events = self.take_presentation_events();
        Ok(json!({
            "snapshot": puzzle_presentation_json::to_value(
                &self.snapshot(0, false, &presentation_events)
            ).expect("debug snapshot JSON should serialize"),
            "debug": debug_transition_value_grid(&self.loaded, debug.as_ref()),
        })
        .to_string())
    }

    fn apply_command_name(&mut self, command_name: &str) -> Result<(), String> {
        self.session
            .apply_command(&self.loaded, command_name)
            .map_err(|error| format!("{error:?}"))
    }

    fn apply_scene_effect(&mut self, effect: &SceneEffect) -> Result<(), String> {
        self.session
            .apply_scene_effect(&self.loaded, effect)
            .map_err(|error| format!("{error:?}"))
    }

    fn undo(&mut self) {
        self.session.undo(&self.loaded);
    }

    fn redo(&mut self) {
        self.session.redo(&self.loaded);
    }

    fn restart(&mut self) -> Result<(), String> {
        self.session
            .restart_level(&self.loaded)
            .map_err(|error| format!("{error:?}"))
    }

    fn next_level(&mut self) -> Result<(), String> {
        self.session
            .advance_level(&self.loaded)
            .map_err(|error| format!("{error:?}"))
    }

    fn previous_level(&mut self) -> Result<(), String> {
        self.session
            .previous_level(&self.loaded)
            .map_err(|error| format!("{error:?}"))
    }

    fn goto_level(&mut self, level: usize) -> Result<(), String> {
        if level >= self.loaded.levels.len() {
            return Err(format!("level index out of range: {level}"));
        }
        self.session
            .start_level(&self.loaded, level)
            .map_err(|error| format!("{error:?}"))
    }

    fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String> {
        if level_index >= self.loaded.levels.len() {
            return Err(format!("level index out of range: {level_index}"));
        }
        let state = self
            .projection
            .decode_state_json(&self.loaded.game, state_json)?;
        self.session
            .start_level_from_state(&self.loaded, level_index, state, materialize_level_start)
            .map_err(|error| format!("{error:?}"))
    }

    fn progress_save_data(&self) -> ProgressSaveData {
        self.session.progress_save_data(&self.loaded)
    }

    fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String> {
        let save: ProgressSaveData = serde_json::from_str(save_json)
            .map_err(|error| format!("invalid progress save: {error}"))?;
        self.session
            .restore_progress_save_data(&self.loaded, &save)
            .map_err(|error| format!("{error:?}"))
    }

    fn resolve_scene_presentation(
        &self,
        scene_name: &str,
        state_override: &HashMap<String, SceneValue>,
        has_progress_save: bool,
    ) -> Result<RuntimeResolvedScene, String> {
        let definition = self
            .loaded
            .scenes
            .iter()
            .find(|definition| definition.name == scene_name)
            .ok_or_else(|| format!("unknown scene presentation: {scene_name}"))?;
        let instance = self
            .session
            .surface_state()
            .components()
            .iter()
            .find(|component| component.definition == scene_name);
        let instance_id = instance.map_or(scene_name, |component| component.id.as_str());
        let properties = instance
            .map(|component| &component.properties)
            .cloned()
            .unwrap_or_default();
        resolved_scene_definition(
            &self.loaded,
            &self.session,
            instance_id,
            definition,
            &properties,
            has_progress_save,
            state_override,
        )
    }

    fn solver_session_2d(&self) -> Option<(LoadedGame, GameSession)> {
        self.projection
            .solver_session_2d(&self.loaded, &self.session)
    }

    fn renderer_state(&self) -> Option<RuntimeRendererState> {
        self.projection.renderer_state(&self.loaded, &self.session)
    }
}

impl GridSessionProjection<2, Size2> for CanvasProjection {
    fn decode_state_json(
        &self,
        game: &GridCompiledGame<2>,
        state_json: &str,
    ) -> Result<GridState<2, Size2>, String> {
        serde_json::from_str::<RuntimeStateSnapshot2d>(state_json)
            .map_err(|error| error.to_string())?
            .into_state(game)
    }

    fn snapshot_grid(
        &self,
        loaded: &LoadedGame,
        session: &GridGameSession<2, Size2>,
    ) -> ProjectedGridSnapshot {
        ProjectedGridSnapshot {
            scene: focused_scene(loaded, session),
            scene_puzzle_state: scene_puzzle_states(loaded, session),
            scene_layers: scene_layers(loaded, session),
        }
    }

    fn solver_state(&self, state: &GridState<2, Size2>) -> SolverStateSnapshot {
        SolverStateSnapshot::from_state2(state)
    }

    fn solver_session_2d(
        &self,
        loaded: &LoadedGame,
        session: &GameSession,
    ) -> Option<(LoadedGame, GameSession)> {
        Some((loaded.clone(), session.clone()))
    }

    fn renderer_state(
        &self,
        loaded: &LoadedGame,
        session: &GameSession,
    ) -> Option<RuntimeRendererState> {
        Some(RuntimeRendererState::TwoD(scene_snapshot_for_state(
            loaded,
            session.state(),
            Some(session.current_level(loaded)),
            scene_resources(loaded, session.surface_state().focused_component()),
        )))
    }
}

impl GridSessionProjection<3, Size3> for SpatialProjection {
    fn decode_state_json(
        &self,
        game: &GridCompiledGame<3>,
        state_json: &str,
    ) -> Result<GridState<3, Size3>, String> {
        serde_json::from_str::<RuntimeStateSnapshot3d>(state_json)
            .map_err(|error| error.to_string())?
            .into_state(game)
    }

    fn snapshot_grid(
        &self,
        loaded: &LoadedGridGame<3, Size3>,
        session: &GridGameSession<3, Size3>,
    ) -> ProjectedGridSnapshot {
        let scene_puzzle_state = spatial_scene_puzzle_states(loaded, session, &self.resources);
        let focused_scene = session.surface_state().focused_component();
        let scene = session
            .scene_state()
            .and_then(|state| state.puzzles.values().next())
            .map(|world| {
                RuntimeRendererState::ThreeD(spatial_world_snapshot(
                    loaded,
                    focused_scene,
                    world,
                    &self.resources,
                ))
            });
        let scene_layers = session
            .surface_state()
            .components()
            .iter()
            .filter(|component| {
                loaded
                    .scenes
                    .iter()
                    .any(|definition| definition.name == component.definition)
            })
            .map(|component| {
                let name = &component.definition;
                let state = session.scene_state_for(&component.id);
                let projected =
                    state
                        .and_then(|state| state.puzzles.values().next())
                        .map(|world| {
                            RuntimeRendererState::ThreeD(spatial_world_snapshot(
                                loaded,
                                name,
                                world,
                                &self.resources,
                            ))
                        });
                ProjectedSceneLayer {
                    id: component.id.clone(),
                    projection: RuntimeAuthoredComponentProjection {
                        name: name.clone(),
                        focused: component.id == focused_scene,
                        choice_cursor: (component.id == focused_scene)
                            .then(|| session.focused_choice_cursor(loaded))
                            .flatten(),
                        scene: projected,
                        scene_state: scene_state(state),
                        scene_puzzles: scene_puzzles(state),
                    },
                }
            })
            .collect();
        ProjectedGridSnapshot {
            scene,
            scene_puzzle_state,
            scene_layers,
        }
    }

    fn solver_state(&self, state: &GridState<3, Size3>) -> SolverStateSnapshot {
        SolverStateSnapshot::from_state3(state)
    }

    fn puzzle3_authoring_resources(
        &self,
        _loaded: &LoadedGridGame<3, Size3>,
    ) -> Option<RuntimePuzzle3Resources> {
        Some(self.resources.clone())
    }
}

fn spatial_scene_puzzle_states(
    loaded: &LoadedGridGame<3, Size3>,
    session: &GridGameSession<3, Size3>,
    resources: &RuntimePuzzle3Resources,
) -> BTreeMap<String, RuntimeRendererState> {
    let Some(state) = session.scene_state() else {
        return BTreeMap::new();
    };
    let entries = state
        .puzzles
        .iter()
        .map(|(name, world)| {
            (
                name.clone(),
                RuntimeRendererState::ThreeD(spatial_world_snapshot(
                    loaded,
                    session.surface_state().focused_component(),
                    world,
                    resources,
                )),
            )
        })
        .collect();
    entries
}

fn spatial_world_snapshot(
    loaded: &LoadedGridGame<3, Size3>,
    scene_name: &str,
    world: &puzzle_play::GridWorldInstanceState<3, Size3>,
    resources: &RuntimePuzzle3Resources,
) -> RuntimePuzzle3Snapshot {
    let level_index = world.active_level_index.unwrap_or(0);
    let level = loaded.levels.get(level_index);
    let cells = puzzle_lang::runtime_puzzle3_cells(&world.state, resources)
        .expect("validated Puzzle3 visual order must resolve runtime cells");
    let render_scene = resolved_render_scene_3d(resources, &cells)
        .expect("validated Puzzle3 visuals must resolve a typed render scene");
    RuntimePuzzle3Snapshot {
        component: scene_name.to_string(),
        level_index,
        level_count: loaded.levels.len(),
        level_name: level.map(|level| level.name.clone()),
        size: puzzle_lang::runtime_puzzle3_size(world.state.size),
        cells,
        completed: loaded.is_goal_complete(&world.state),
        has_next_level: level_index + 1 < loaded.levels.len(),
        has_previous_level: level_index > 0,
        render: resources.render.clone(),
        render_scene,
        animation_events: Vec::new(),
        animation_batch_id: None,
    }
}

#[cfg(test)]
fn compiled_state_value(state: &PuzzleState) -> Value {
    serde_json::to_value(RuntimeStateSnapshot2d::from_state(state))
        .expect("runtime state snapshot serializes")
}

fn focused_scene(loaded: &LoadedGame, session: &GameSession) -> Option<RuntimeRendererState> {
    if let Some((state, level)) = scene_puzzle_state(
        loaded,
        session,
        session.surface_state().focused_component(),
        session.surface_state().focused_component(),
    ) {
        return Some(RuntimeRendererState::TwoD(scene_snapshot_for_state(
            loaded,
            state,
            level,
            scene_resources(loaded, session.surface_state().focused_component()),
        )));
    }
    if !loaded.scenes.is_empty() {
        return None;
    }
    Some(RuntimeRendererState::TwoD(scene_snapshot_for_state(
        loaded,
        session.state(),
        Some(session.current_level(loaded)),
        scene_resources(loaded, session.surface_state().focused_component()),
    )))
}

fn scene_snapshot_for_state(
    loaded: &LoadedGame,
    state: &PuzzleState,
    level: Option<&Level>,
    resources: Option<&puzzle_lang::SceneResources>,
) -> RuntimePuzzle2Snapshot {
    scene_snapshot_for_materialized_state(loaded, state, level, resources, None)
}

fn scene_snapshot_for_materialized_state(
    loaded: &LoadedGame,
    state: &PuzzleState,
    level: Option<&Level>,
    resources: Option<&puzzle_lang::SceneResources>,
    display_error: Option<String>,
) -> RuntimePuzzle2Snapshot {
    let mut cells = Vec::new();
    if display_error.is_none() {
        let visual_order = runtime_visual_order(&loaded.visuals.order);
        let priority_count = u64::try_from(visual_order.priorities.len())
            .expect("validated visual priority count must fit u64");
        for y in 0..state.height {
            for x in 0..state.width {
                let cell_render_order =
                    cell_render_order_2d(&visual_order, state.width, state.height, x, y)
                        .expect("validated 2D visual order must resolve every cell");
                let mut layers = Vec::new();
                for layer in 0..state.layer_count {
                    let slot = ((usize::from(y) * usize::from(state.width)) + usize::from(x))
                        * usize::from(state.layer_count)
                        + usize::from(layer);
                    let object = state.slots()[slot];
                    if object.is_empty() {
                        continue;
                    }
                    let name = object_name(loaded, object);
                    let resolved = resolve_object_priority(&visual_order, &name)
                        .expect("validated visual order must cover every object");
                    let render_priority = u16::try_from(resolved.index)
                        .expect("validated visual priority must fit u16");
                    layers.push(RuntimePuzzle2Layer {
                        layer,
                        object_id: object.0,
                        object: name.clone(),
                        visual: name,
                        render_priority,
                        render_order: cell_render_order
                            .saturating_mul(priority_count)
                            .saturating_add(u64::from(render_priority)),
                        composition: match resolved.composition {
                            puzzle_presentation::VisualComposition::Ordered => {
                                puzzle_runtime_contract::RuntimeVisualComposition::Ordered
                            }
                            puzzle_presentation::VisualComposition::Average => {
                                puzzle_runtime_contract::RuntimeVisualComposition::Average
                            }
                        },
                    });
                }
                cells.push(RuntimePuzzle2Cell {
                    x,
                    y,
                    render_order: cell_render_order,
                    layers,
                });
            }
        }
    }
    let regions = level
        .map(|level| {
            level
                .regions
                .iter()
                .map(|region| RuntimeRegion2d {
                    index: region.index,
                    x: region.x,
                    y: region.y,
                    width: region.width,
                    height: region.height,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    RuntimePuzzle2Snapshot {
        width: state.width,
        height: state.height,
        layer_count: state.layer_count,
        settings: puzzle_settings(loaded),
        animation: animation_settings(loaded),
        screen: screen_snapshot(loaded),
        regions,
        resources: scene_resources_snapshot(resources),
        render_scene: resolved_render_scene_2d(loaded, &cells)
            .expect("validated 2D visuals must resolve a typed render scene"),
        cells,
        display_error,
    }
}

fn resolved_render_scene_2d(
    loaded: &LoadedGame,
    cells: &[RuntimePuzzle2Cell],
) -> Result<RuntimeResolvedRenderScene, puzzle_presentation::PresentationError> {
    let mut transforms = HashMap::new();
    let mut clips = Vec::new();
    for visual in &loaded.visuals.entries {
        let mut frames = match &visual.kind {
            VisualKind::Solid(color) => vec![resolve_pixel_frame(
                &["0".to_string()],
                &BTreeMap::from([("0".to_string(), color.clone())]),
            )?],
            VisualKind::Image { source } => vec![RuntimeResolvedVisualFrame::ExternalImage {
                source: source.clone(),
            }],
            VisualKind::Ascii { colors } => {
                let palette = colors
                    .iter()
                    .map(|color| (color.token.to_string(), color.color.clone()))
                    .collect::<BTreeMap<_, _>>();
                visual
                    .frames
                    .iter()
                    .map(|frame| {
                        let rows = frame.planes.first().ok_or(
                            puzzle_presentation::PresentationError::IncompatibleCompositionFrames,
                        )?;
                        resolve_pixel_frame(rows, &palette)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
        };
        if let Some(grid) = visual.pixels_per_cell {
            let width = u16::try_from(grid.width).map_err(|_| {
                puzzle_presentation::PresentationError::IncompatibleCompositionFrames
            })?;
            let height = u16::try_from(grid.height).map_err(|_| {
                puzzle_presentation::PresentationError::IncompatibleCompositionFrames
            })?;
            for frame in &mut frames {
                if let RuntimeResolvedVisualFrame::Pixels {
                    width: frame_width,
                    height: frame_height,
                    ..
                } = frame
                {
                    *frame_width = width;
                    *frame_height = height;
                }
            }
        }
        let runtime_transforms = visual
            .transforms
            .iter()
            .map(runtime_visual_transform)
            .collect::<Vec<_>>();
        transforms.insert(
            visual.name.clone(),
            resolve_visual_affine(&runtime_transforms)?,
        );
        let frame_duration_ms = visual.animation_duration_ms.map(|duration| {
            duration / u64::try_from(frames.len()).expect("validated frame count must fit u64")
        });
        clips.push(RuntimeResolvedVisualClip {
            id: visual.name.clone(),
            frames,
            frame_duration_ms,
            layout: RuntimeResolvedVisualLayout {
                fit: match visual.fit.mode {
                    LoadedVisualFitMode::Contain => RuntimeResolvedFitMode::Contain,
                    LoadedVisualFitMode::Cover => RuntimeResolvedFitMode::Cover,
                    LoadedVisualFitMode::Stretch => RuntimeResolvedFitMode::Stretch,
                },
                width: u16::try_from(visual.fit.width).map_err(|_| {
                    puzzle_presentation::PresentationError::IncompatibleCompositionFrames
                })?,
                height: u16::try_from(visual.fit.height).map_err(|_| {
                    puzzle_presentation::PresentationError::IncompatibleCompositionFrames
                })?,
                sampling: match visual.sampling {
                    Some(LoadedVisualSampling::Smooth) => RuntimeResolvedSampling::Smooth,
                    Some(LoadedVisualSampling::Pixelated) => RuntimeResolvedSampling::Pixelated,
                    None if matches!(&visual.kind, VisualKind::Image { source } if !source.to_ascii_lowercase().ends_with(".png")) => RuntimeResolvedSampling::Smooth,
                    None => RuntimeResolvedSampling::Pixelated,
                },
                raster: matches!(&visual.kind, VisualKind::Image { .. }),
            },
        });
    }

    let aliases = loaded
        .visuals
        .aliases
        .iter()
        .map(|alias| (alias.object.as_str(), alias.visual.as_str()))
        .collect::<HashMap<_, _>>();
    let mut instances = Vec::new();
    let mut grouped = BTreeMap::<(u16, u16, u64), (RuntimeVisualComposition, Vec<u64>)>::new();
    for cell in cells {
        for layer in &cell.layers {
            let visual = aliases
                .get(layer.object.as_str())
                .copied()
                .unwrap_or(layer.visual.as_str());
            let Some(transform) = transforms.get(visual) else {
                continue;
            };
            let id =
                u64::try_from(instances.len() + 1).expect("render instance count must fit u64");
            instances.push(RuntimeResolvedRenderInstance {
                id,
                object_id: Some(layer.object_id),
                visual: visual.to_string(),
                cell: [i32::from(cell.x), i32::from(cell.y), 0],
                transform: *transform,
                opacity: 1.0,
                frame_elapsed_ms: None,
                playback: RuntimeResolvedPlayback::Loop,
                render_order: layer.render_order,
            });
            let group = grouped
                .entry((cell.x, cell.y, layer.render_order))
                .or_insert((layer.composition, Vec::new()));
            if group.0 != layer.composition {
                return Err(puzzle_presentation::PresentationError::IncompatibleCompositionFrames);
            }
            group.1.push(id);
        }
    }
    Ok(RuntimeResolvedRenderScene {
        clips,
        instances,
        composition_groups: grouped
            .into_iter()
            .map(|((_, _, render_order), (composition, instances))| {
                RuntimeResolvedCompositionGroup {
                    render_order,
                    composition,
                    instances,
                }
            })
            .collect(),
        cells: cells
            .iter()
            .map(|cell| RuntimeResolvedRenderCell {
                position: [i32::from(cell.x), i32::from(cell.y), 0],
                render_order: cell.render_order,
                object_ids: cell.layers.iter().map(|layer| layer.object_id).collect(),
            })
            .collect(),
        render_priority_count: u16::try_from(loaded.visuals.order.priorities.len())
            .expect("validated render priority count must fit u16"),
        animation_duration_ms: loaded.animation.tween.interval_ms,
    })
}

fn runtime_visual_transform(transform: &LoadedVisualTransform) -> RuntimeVisualTransform {
    match transform {
        LoadedVisualTransform::Rotate {
            degrees,
            axis,
            space,
        } => RuntimeVisualTransform::Rotate {
            degrees: *degrees,
            axis: *axis,
            space: runtime_visual_space(*space),
        },
        LoadedVisualTransform::Translate { value, space } => RuntimeVisualTransform::Translate {
            value: *value,
            space: runtime_visual_space(*space),
        },
        LoadedVisualTransform::Flip { enabled } => {
            RuntimeVisualTransform::Flip { enabled: *enabled }
        }
    }
}

fn runtime_visual_space(space: LoadedVisualSpace) -> RuntimeVisualSpace {
    match space {
        LoadedVisualSpace::World => RuntimeVisualSpace::World,
        LoadedVisualSpace::Local => RuntimeVisualSpace::Local,
    }
}

fn resolved_render_scene_3d(
    resources: &RuntimePuzzle3Resources,
    cells: &[puzzle_runtime_contract::RuntimePuzzle3Cell],
) -> Result<RuntimeResolvedRenderScene, puzzle_presentation::PresentationError> {
    let clips = resources
        .visuals
        .iter()
        .map(|(name, visual)| {
            let frames = visual
                .frames
                .iter()
                .map(|frame| resolve_voxel_frame(&frame.layers, &visual.palette))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(RuntimeResolvedVisualClip {
                id: name.clone(),
                frames,
                frame_duration_ms: visual.frame_duration_ms.or_else(|| {
                    visual.duration_ms.map(|duration| {
                        duration
                            / u64::try_from(visual.frames.len())
                                .expect("validated frame count must fit u64")
                    })
                }),
                layout: RuntimeResolvedVisualLayout {
                    fit: RuntimeResolvedFitMode::Contain,
                    width: 1,
                    height: 1,
                    sampling: RuntimeResolvedSampling::Pixelated,
                    raster: false,
                },
            })
        })
        .collect::<Result<Vec<_>, puzzle_presentation::PresentationError>>()?;
    let objects = resources
        .objects
        .values()
        .map(|object| (object.id, object))
        .collect::<HashMap<_, _>>();
    let mut instances = Vec::new();
    let mut groups = Vec::new();
    for cell in cells {
        let mut grouped = BTreeMap::<u64, (RuntimeVisualComposition, Vec<u64>)>::new();
        for object_ref in &cell.objects {
            let object = objects.get(&object_ref.id).ok_or_else(|| {
                puzzle_presentation::PresentationError::UnknownVisual(format!(
                    "object:{}",
                    object_ref.id
                ))
            })?;
            let Some(visual_name) = &object.visual else {
                continue;
            };
            let visual = resources.visuals.get(visual_name).ok_or_else(|| {
                puzzle_presentation::PresentationError::UnknownVisual(visual_name.clone())
            })?;
            let id =
                u64::try_from(instances.len() + 1).expect("render instance count must fit u64");
            instances.push(RuntimeResolvedRenderInstance {
                id,
                object_id: Some(object_ref.id),
                visual: visual_name.clone(),
                cell: [
                    i32::from(cell.position.x),
                    i32::from(cell.position.y),
                    i32::from(cell.position.z.unwrap_or(0)),
                ],
                transform: visual.spatial_affine,
                opacity: 1.0,
                frame_elapsed_ms: None,
                playback: RuntimeResolvedPlayback::Loop,
                render_order: object_ref.render_order,
            });
            let group = grouped
                .entry(object_ref.render_order)
                .or_insert((object.composition, Vec::new()));
            if group.0 != object.composition {
                return Err(puzzle_presentation::PresentationError::IncompatibleCompositionFrames);
            }
            group.1.push(id);
        }
        groups.extend(
            grouped
                .into_iter()
                .map(
                    |(render_order, (composition, instances))| RuntimeResolvedCompositionGroup {
                        render_order,
                        composition,
                        instances,
                    },
                ),
        );
    }
    Ok(RuntimeResolvedRenderScene {
        clips,
        instances,
        composition_groups: groups,
        cells: cells
            .iter()
            .map(|cell| RuntimeResolvedRenderCell {
                position: [
                    i32::from(cell.position.x),
                    i32::from(cell.position.y),
                    i32::from(cell.position.z.unwrap_or(0)),
                ],
                render_order: cell.render_order,
                object_ids: cell.objects.iter().map(|object| object.id).collect(),
            })
            .collect(),
        render_priority_count: u16::try_from(resources.order.priorities.len())
            .expect("validated render priority count must fit u16"),
        animation_duration_ms: resources.render.animation.tween.interval_ms,
    })
}

fn runtime_visual_order(order: &puzzle_lang::VisualOrderDef) -> VisualOrderRef<'_> {
    VisualOrderRef {
        direction_priority: &order.direction_priority,
        priorities: order
            .priorities
            .iter()
            .map(|priority| VisualPriorityRef {
                objects: &priority.objects,
                animations: &priority.animations,
                merge: priority.merge,
            })
            .collect(),
    }
}

fn scene_layers(loaded: &LoadedGame, session: &GameSession) -> Vec<ProjectedSceneLayer> {
    session
        .surface_state()
        .components()
        .iter()
        .filter(|component| {
            loaded
                .scenes
                .iter()
                .any(|definition| definition.name == component.definition)
        })
        .map(|component| {
            let name = &component.definition;
            let state = session.scene_state_for(&component.id);
            let scene = scene_puzzle_state(loaded, session, &component.id, name).map(
                |(puzzle_state, level)| {
                    RuntimeRendererState::TwoD(scene_snapshot_for_state(
                        loaded,
                        puzzle_state,
                        level,
                        scene_resources(loaded, name),
                    ))
                },
            );
            ProjectedSceneLayer {
                id: component.id.clone(),
                projection: RuntimeAuthoredComponentProjection {
                    name: name.clone(),
                    focused: component.id == session.surface_state().focused_component(),
                    choice_cursor: (component.id == session.surface_state().focused_component())
                        .then(|| session.focused_choice_cursor(loaded))
                        .flatten(),
                    scene,
                    scene_state: scene_state(state),
                    scene_puzzles: scene_puzzles(state),
                },
            }
        })
        .collect()
}

fn surface_snapshot<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
    authored_components: Vec<ProjectedSceneLayer>,
    has_progress_save: bool,
) -> RuntimeSurface {
    let components = session
        .surface_state()
        .components()
        .iter()
        .map(|component| {
            let authored_projection = authored_components
                .iter()
                .find(|candidate| candidate.id == component.id)
                .map(|candidate| candidate.projection.clone());
            let presentation = match resolved_surface_component_definition(
                loaded,
                session,
                &component.id,
                &component.definition,
                &component.properties,
                has_progress_save,
            ) {
                Ok(scene) => RuntimeComponentPresentation::Ready(scene),
                Err(error) => RuntimeComponentPresentation::Error { error },
            };
            RuntimeSurfaceComponent {
                id: component.id.clone(),
                definition: component.definition.clone(),
                placement: component.placement,
                visibility: component.visibility,
                modal: component.modal,
                properties: ordered_scene_values(&component.properties),
                await_event: component.awaited_event.clone(),
                authored_projection,
                presentation,
            }
        })
        .collect::<Vec<_>>();
    RuntimeSurface {
        root: session.surface_state().root().map(str::to_string),
        focus: session.surface_state().focused_component().to_string(),
        components,
    }
}

fn resolved_surface_component_definition<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
    instance_id: &str,
    definition_name: &str,
    properties: &HashMap<String, SceneValue>,
    has_progress_save: bool,
) -> Result<RuntimeResolvedScene, String> {
    if definition_name == STANDARD_MESSAGE_COMPONENT {
        let text = properties
            .get(STANDARD_MESSAGE_TEXT_PROPERTY)
            .ok_or_else(|| {
                format!(
                    "component `{instance_id}` is missing required `{STANDARD_MESSAGE_TEXT_PROPERTY}` property"
                )
            })?;
        return Ok(RuntimeResolvedScene {
            name: STANDARD_MESSAGE_COMPONENT.to_string(),
            layout: SceneLayoutDef::default(),
            events: Some(BTreeMap::from([(
                STANDARD_MESSAGE_DISMISS_EVENT.to_string(),
                RuntimeResolvedEventBinding {
                    pointer: true,
                    keys: "input".to_string(),
                },
            )])),
            keys: None,
            components: vec![RuntimeResolvedSceneComponent::Text {
                role: SceneTextRoleDef::Body,
                value: scene_value_to_string(text),
                text_align: None,
                layout: SceneLayoutDef::default(),
            }],
        });
    }

    let definition = loaded
        .scenes
        .iter()
        .find(|definition| definition.name == definition_name)
        .ok_or_else(|| format!("unknown presented component definition: {definition_name}"))?;
    resolved_scene_definition(
        loaded,
        session,
        instance_id,
        definition,
        properties,
        has_progress_save,
        &HashMap::new(),
    )
}

fn resolved_scene_definition<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
    instance_id: &str,
    definition: &SceneDef,
    properties: &HashMap<String, SceneValue>,
    has_progress_save: bool,
    state_override: &HashMap<String, SceneValue>,
) -> Result<RuntimeResolvedScene, String> {
    let values = scene_presentation_values(
        session,
        instance_id,
        properties,
        has_progress_save,
        state_override,
    );
    Ok(RuntimeResolvedScene {
        name: definition.name.clone(),
        layout: definition.layout.clone(),
        components: resolved_scene_components(loaded, session, &definition.components, &values)?,
        keys: Some(
            definition
                .key_bindings
                .iter()
                .map(|binding| RuntimeKeyBinding {
                    effect: binding.effect.clone(),
                    keys: binding.keys.iter().map(key_trigger_name).collect(),
                })
                .collect(),
        ),
        events: None,
    })
}

fn scene_presentation_values<const D: usize, Size: GridSize<D>>(
    session: &GridGameSession<D, Size>,
    instance_id: &str,
    properties: &HashMap<String, SceneValue>,
    has_progress_save: bool,
    state_override: &HashMap<String, SceneValue>,
) -> HashMap<String, SceneValue> {
    let mut values = HashMap::new();
    for (name, value) in session.session_values() {
        values.insert(name.clone(), value.clone());
        values.insert(format!("game.{name}"), value.clone());
        values.insert(format!("gameState.{name}"), value.clone());
    }
    let has_progress_save = SceneValue::Bool(has_progress_save);
    values.insert("has_progress_save".to_string(), has_progress_save.clone());
    values.insert("game.has_progress_save".to_string(), has_progress_save);
    values.insert(
        "levelIndex".to_string(),
        SceneValue::Int(session.level_index() as i64),
    );
    values.insert("canUndo".to_string(), SceneValue::Bool(session.can_undo()));
    values.insert("canRedo".to_string(), SceneValue::Bool(session.can_redo()));
    if let Some(state) = session.scene_state_for(instance_id) {
        for (name, value) in &state.values {
            values.insert(name.clone(), value.clone());
            values.insert(format!("scene.{name}"), value.clone());
            values.insert(format!("sceneState.{name}"), value.clone());
        }
    }
    for (name, value) in state_override {
        values.insert(name.clone(), value.clone());
        values.insert(format!("scene.{name}"), value.clone());
        values.insert(format!("sceneState.{name}"), value.clone());
    }
    for (name, value) in properties {
        values.insert(name.clone(), value.clone());
        values.insert(format!("properties.{name}"), value.clone());
    }
    values
}

fn resolved_scene_components<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
    components: &[SceneComponent],
    values: &HashMap<String, SceneValue>,
) -> Result<Vec<RuntimeResolvedSceneComponent>, String> {
    components
        .iter()
        .map(|component| resolved_scene_component(loaded, session, component, values))
        .collect()
}

fn resolved_scene_component<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
    component: &SceneComponent,
    values: &HashMap<String, SceneValue>,
) -> Result<RuntimeResolvedSceneComponent, String> {
    let resolve = |expr: &SceneExpr| session.resolve_scene_expression(loaded, expr, values);
    Ok(match component {
        SceneComponent::Viewport(viewport) => RuntimeResolvedSceneComponent::Viewport {
            dimension: match viewport.projection {
                ViewportProjectionDef::TwoD => RuntimeViewportDimension::TwoD,
                ViewportProjectionDef::ThreeD => RuntimeViewportDimension::ThreeD,
            },
            source: viewport.source.clone(),
            layout: viewport.layout.clone(),
        },
        SceneComponent::Frame(frame) => RuntimeResolvedSceneComponent::Frame {
            kind: frame.kind.clone(),
            source: frame.source.clone(),
            layout: frame.layout.clone(),
        },
        SceneComponent::Text(text) => {
            let value = match &text.content {
                SceneTextContent::Literal(value) => value.clone(),
                SceneTextContent::Path(path) => {
                    scene_value_to_string(&resolve(&SceneExpr::Path(path.clone()))?)
                }
                SceneTextContent::Expr(expr) => scene_value_to_string(&resolve(expr)?),
            };
            RuntimeResolvedSceneComponent::Text {
                role: text.role,
                value,
                text_align: text.text_align,
                layout: text.layout.clone(),
            }
        }
        SceneComponent::Button(button) => RuntimeResolvedSceneComponent::Button {
            label: scene_value_to_string(&resolve(&button.label)?),
            effect: button.effect.clone(),
            layout: button.layout.clone(),
        },
        SceneComponent::Choice(choice) => RuntimeResolvedSceneComponent::Choice {
            label: scene_value_to_string(&resolve(&choice.label)?),
            effect: choice.effect.clone(),
            layout: choice.layout.clone(),
        },
        SceneComponent::Row(container) => RuntimeResolvedSceneComponent::Row {
            layout: container.layout.clone(),
            children: resolved_scene_components(loaded, session, &container.children, values)?,
        },
        SceneComponent::Column(container) => RuntimeResolvedSceneComponent::Column {
            layout: container.layout.clone(),
            children: resolved_scene_components(loaded, session, &container.children, values)?,
        },
        SceneComponent::Box(container) => RuntimeResolvedSceneComponent::Box {
            layout: container.layout.clone(),
            children: resolved_scene_components(loaded, session, &container.children, values)?,
        },
        SceneComponent::Conditional(conditional) => {
            let condition = resolve(&conditional.condition)?;
            let SceneValue::Bool(condition) = condition else {
                return Err(format!(
                    "scene conditional resolved to a non-boolean value: {condition:?}"
                ));
            };
            RuntimeResolvedSceneComponent::Conditional {
                condition,
                children: resolved_scene_components(
                    loaded,
                    session,
                    &conditional.children,
                    values,
                )?,
                else_children: resolved_scene_components(
                    loaded,
                    session,
                    &conditional.else_children,
                    values,
                )?,
            }
        }
    })
}

fn scene_values_from_json(value: &Value) -> Result<HashMap<String, SceneValue>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "scene preview state must be a JSON object".to_string())?;
    object
        .iter()
        .map(|(name, value)| {
            let value = match value {
                Value::Bool(value) => SceneValue::Bool(*value),
                Value::Number(value) => value
                    .as_i64()
                    .map(SceneValue::Int)
                    .ok_or_else(|| format!("scene preview value `{name}` must be an integer"))?,
                Value::String(value) => SceneValue::Text(value.clone()),
                _ => {
                    return Err(format!(
                        "scene preview value `{name}` must be a boolean, integer, or string"
                    ));
                }
            };
            Ok((name.clone(), value))
        })
        .collect()
}

fn scene_puzzles<const D: usize, Size: GridSize<D>>(
    state: Option<&puzzle_play::GridSceneRuntimeState<D, Size>>,
) -> Vec<String> {
    let Some(state) = state else {
        return Vec::new();
    };
    let mut names = state.puzzles.keys().collect::<Vec<_>>();
    names.sort();
    names.into_iter().map(|name| name.clone()).collect()
}

fn scene_puzzle_states(
    loaded: &LoadedGame,
    session: &GameSession,
) -> BTreeMap<String, RuntimeRendererState> {
    let Some(state) = session.scene_state() else {
        return BTreeMap::new();
    };
    let mut entries = BTreeMap::new();
    let mut names = state.puzzles.keys().collect::<Vec<_>>();
    names.sort();
    for name in names {
        let Some(puzzle) = state.puzzles.get(name) else {
            continue;
        };
        let level = puzzle
            .active_level_index
            .and_then(|index| loaded.levels.get(index));
        let entry = scene_snapshot_for_state(
            loaded,
            &puzzle.state,
            level,
            scene_resources(loaded, session.surface_state().focused_component()),
        );
        entries.insert(name.clone(), RuntimeRendererState::TwoD(entry));
    }
    entries
}

fn key_trigger_name(key: &KeyTrigger) -> String {
    match key {
        KeyTrigger::Char(ch) => ch.to_string(),
        KeyTrigger::Named(name) => name.clone(),
    }
}

fn scene_puzzle_state<'a>(
    loaded: &'a LoadedGame,
    session: &'a GameSession,
    instance_id: &str,
    definition_name: &str,
) -> Option<(&'a PuzzleState, Option<&'a Level>)> {
    let scene = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == definition_name)?;
    let state = session.scene_state_for(instance_id)?;
    let puzzle = if let Some(rule) = &scene.puzzle_rule {
        rule.target
            .split('.')
            .next_back()
            .and_then(|puzzle_name| state.puzzles.get(puzzle_name))
    } else {
        first_puzzle_component(&scene.components)
            .and_then(|puzzle_name| state.puzzles.get(puzzle_name))
    }?;
    let level = puzzle
        .active_level_index
        .and_then(|index| loaded.levels.get(index));
    Some((&puzzle.state, level))
}

fn first_puzzle_component(components: &[SceneComponent]) -> Option<&str> {
    for component in components {
        match component {
            SceneComponent::Viewport(viewport) => {
                return Some(viewport.source.as_str());
            }
            SceneComponent::Row(container)
            | SceneComponent::Column(container)
            | SceneComponent::Box(container) => {
                if let Some(name) = first_puzzle_component(&container.children) {
                    return Some(name);
                }
            }
            SceneComponent::Conditional(conditional) => {
                if let Some(name) = first_puzzle_component(&conditional.children) {
                    return Some(name);
                }
                if let Some(name) = first_puzzle_component(&conditional.else_children) {
                    return Some(name);
                }
            }
            _ => {}
        }
    }
    None
}

fn input_id_by_name<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    input_name: &str,
) -> Option<InputId> {
    loaded
        .input_labels
        .iter()
        .find_map(|(id, label)| (label == input_name).then_some(*id))
}

#[cfg(feature = "editor-debug")]
pub fn debug_transition_value(
    loaded: &LoadedGame,
    debug: Option<&puzzle_play::TransitionTrace>,
) -> Value {
    debug_transition_value_grid(loaded, debug)
}

#[cfg(feature = "editor-debug")]
pub fn debug_transition_value_grid<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    debug: Option<&GridTransitionTrace<D>>,
) -> Value {
    let Some(debug) = debug else {
        return Value::Null;
    };
    json!({
        "kind": "model_input",
        "inputId": debug.input.0,
        "input": loaded.input_labels.get(&debug.input).map(String::as_str).unwrap_or(""),
        "progressed": debug.progressed,
        "observable": debug.observable,
        "cancelled": debug.cancelled,
        "target": debug.target,
        "commands": debug.commands.iter().map(debug_command_value).collect::<Vec<_>>(),
        "executions": debug.firings.iter().enumerate().map(|(index, firing)| {
            json!({
                "index": index,
                "ruleId": firing.rule.0,
                "rule": debug_rule_value(loaded, firing.rule),
                "progressed": firing.progressed,
                "observable": firing.observable,
                "patch": debug_patch_value(loaded, &firing.patch),
            })
        }).collect::<Vec<_>>(),
    })
}

#[cfg(feature = "editor-debug")]
fn debug_rule_value<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    rule: RuleId,
) -> Value {
    let Some(info) = loaded.rule_debug_info.get(&rule) else {
        return json!({ "id": rule.0 });
    };
    json!({
        "id": rule.0,
        "sourceLine": info.source_line,
        "sourceLineNumber": info.source_line_number,
        "routineStack": info.routine_stack,
    })
}

#[cfg(feature = "editor-debug")]
fn debug_command_value(command: &TransitionCommand) -> Value {
    json!({
        "kind": match command {
            TransitionCommand::Win => "win",
            TransitionCommand::Restart => "restart",
            TransitionCommand::NextLevel => "next_level",
            TransitionCommand::Again => "again",
            TransitionCommand::Checkpoint => "checkpoint",
            TransitionCommand::ClearCheckpoint => "clear_checkpoint",
        }
    })
}

#[cfg(feature = "editor-debug")]
fn debug_patch_value<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    patch: &GridPatch<D>,
) -> Vec<Value> {
    patch
        .ops()
        .iter()
        .map(|op| debug_patch_op_value(loaded, op))
        .collect()
}

#[cfg(feature = "editor-debug")]
fn debug_patch_op_value<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    op: &PatchOp<D>,
) -> Value {
    match op {
        PatchOp::Add { position, object } => json!({
            "kind": "add",
            "position": position_value_from_grid(*position),
            "objectId": object.0,
            "object": object_name(loaded, *object),
        }),
        PatchOp::Remove { position, object } => json!({
            "kind": "remove",
            "position": position_value_from_grid(*position),
            "objectId": object.0,
            "object": object_name(loaded, *object),
        }),
        PatchOp::Move { from, to, object } => json!({
            "kind": "move",
            "from": position_value_from_grid(*from),
            "to": position_value_from_grid(*to),
            "objectId": object.0,
            "object": object_name(loaded, *object),
        }),
        PatchOp::Replace {
            position,
            remove,
            add,
        } => json!({
            "kind": "replace",
            "position": position_value_from_grid(*position),
            "remove": remove.0,
            "add": add.0,
            "removeObject": object_name(loaded, *remove),
            "addObject": object_name(loaded, *add),
        }),
        PatchOp::UpdateVariable {
            variable,
            op,
            value,
        } => json!({
            "kind": "update_variable",
            "variableId": variable.0,
            "variable": variable_name(loaded, *variable),
            "op": match op {
                VariableUpdateOp::Set => "set",
                VariableUpdateOp::Add => "add",
                VariableUpdateOp::Subtract => "subtract",
                VariableUpdateOp::Multiply => "multiply",
                VariableUpdateOp::Divide => "divide",
                VariableUpdateOp::Remainder => "remainder",
            },
            "value": value,
        }),
        PatchOp::SetMark {
            position,
            object,
            mark,
            value,
        } => json!({
            "kind": "set_mark",
            "position": position_value_from_grid(*position),
            "objectId": object.0,
            "object": object_name(loaded, *object),
            "mark": mark.0,
            "markName": mark_name(loaded, *mark),
            "value": value,
        }),
        PatchOp::RemoveMark {
            position,
            object,
            mark,
            value,
            match_value,
        } => json!({
            "kind": "remove_mark",
            "position": position_value_from_grid(*position),
            "objectId": object.0,
            "object": object_name(loaded, *object),
            "mark": mark.0,
            "markName": mark_name(loaded, *mark),
            "value": value,
            "match": match match_value {
                CoreMarkValueMatch::Any => "any",
                CoreMarkValueMatch::Exact => "exact",
            },
        }),
    }
}

#[cfg(feature = "editor-debug")]
fn position_value_from_grid<const D: usize>(position: GridCoord<D>) -> Value {
    let axes = position.axes();
    let mut value = serde_json::Map::new();
    value.insert("x".to_string(), json!(axes.first().copied().unwrap_or(0)));
    value.insert("y".to_string(), json!(axes.get(1).copied().unwrap_or(0)));
    if let Some(z) = axes.get(2) {
        value.insert("z".to_string(), json!(z));
    }
    Value::Object(value)
}

fn object_name<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    object: ObjectId,
) -> String {
    loaded.object_name(object).to_string()
}

#[cfg(feature = "editor-debug")]
fn mark_name<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    mark: MarkId,
) -> String {
    loaded
        .mark_labels
        .get(&mark)
        .cloned()
        .unwrap_or_else(|| format!("mark#{}", mark.0))
}

#[cfg(feature = "editor-debug")]
fn variable_name<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    variable: VariableId,
) -> String {
    loaded
        .variable_labels
        .get(&variable)
        .cloned()
        .unwrap_or_else(|| format!("var#{}", variable.0))
}

fn object_id_by_name(loaded: &LoadedGame, object_name: &str) -> Option<ObjectId> {
    loaded
        .object_labels
        .iter()
        .find_map(|(id, label)| (label == object_name).then_some(*id))
}

fn sounds_snapshot<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
) -> RuntimeSounds {
    let sounds = runtime_sounds_def(&loaded.sounds);
    RuntimeSounds {
        sfx: sounds
            .sfx
            .into_iter()
            .map(|sound| RuntimeSfxSound {
                name: sound.name,
                seed: sound.seed,
                type_target: sound.type_target,
                volume: sound.volume,
            })
            .collect(),
        music: sounds
            .music
            .into_iter()
            .map(|sound| RuntimeMusicSound {
                name: sound.name,
                seed: sound.seed,
                height: sound.height,
                bars: sound.bars,
                bpm: sound.bpm,
                volume: sound.volume,
            })
            .collect(),
    }
}

fn theme_snapshot(theme: &ThemeDef) -> RuntimeTheme {
    let variables = theme
        .variables
        .iter()
        .map(|variable| (variable.name.clone(), variable.value.clone()))
        .collect();
    RuntimeTheme {
        name: theme.name.clone(),
        variables,
    }
}

fn input_buffer_settings<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
) -> RuntimeInputBufferSettings {
    RuntimeInputBufferSettings {
        queue_during_wait: loaded.input_buffer.queue_during_wait,
        fast_forward_wait: loaded.input_buffer.fast_forward_wait,
        min_wait_ms: loaded.input_buffer.min_wait_ms,
    }
}

fn animation_settings<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
) -> RuntimeAnimationSettings {
    RuntimeAnimationSettings {
        tween: RuntimeTweenSettings {
            enabled: loaded.animation.tween.enabled,
            interval_ms: loaded.animation.tween.interval_ms,
        },
    }
}

fn puzzle_settings(loaded: &LoadedGame) -> RuntimePuzzle2Settings {
    RuntimePuzzle2Settings {
        render: RuntimeRender2d {},
        grid: RuntimeGridRender2d {
            visibility: loaded.render.grid.occupied_cells || loaded.render.grid.all_cells,
            occupied_cells: loaded.render.grid.occupied_cells,
            all_cells: loaded.render.grid.all_cells,
        },
        input_buffer: input_buffer_settings(loaded),
        animation: animation_settings(loaded),
    }
}

fn screen_snapshot(loaded: &LoadedGame) -> RuntimePuzzle2Screen {
    let viewport_size = match loaded.screen.viewport_size {
        ViewportSizeDef::Full => RuntimeViewportSize2d::Full,
        ViewportSizeDef::Size { width, height } => RuntimeViewportSize2d::Size { width, height },
    };
    let viewport_mode = match loaded.screen.viewport_mode {
        ViewportModeDef::Paged => RuntimeViewportMode2d::Paged,
        ViewportModeDef::Centered => RuntimeViewportMode2d::Centered,
    };
    let viewport_focus_objects = loaded
        .object_groups
        .get(&loaded.screen.viewport_focus)
        .cloned()
        .or_else(|| object_id_by_name(loaded, &loaded.screen.viewport_focus).map(|id| vec![id]))
        .unwrap_or_default()
        .into_iter()
        .map(|id| id.0)
        .collect::<Vec<_>>();
    RuntimePuzzle2Screen {
        viewport_size,
        viewport_focus: loaded.screen.viewport_focus.clone(),
        viewport_focus_objects,
        viewport_mode,
    }
}

fn inputs<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
) -> Vec<RuntimeInputBinding> {
    let mut inputs = loaded.input_labels.iter().collect::<Vec<_>>();
    inputs.sort_by_key(|(id, _)| id.0);
    inputs
        .into_iter()
        .map(|(id, name)| RuntimeInputBinding {
            id: id.0,
            name: name.clone(),
            key: key_for_input(loaded, *id),
            arrow: arrow_for_input(loaded, *id),
            keys: key_triggers_for_input(loaded, *id),
        })
        .collect()
}

fn key_for_input<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    input: InputId,
) -> Option<String> {
    loaded
        .controls
        .keys
        .iter()
        .find_map(|(key, id)| (*id == input).then_some(char::from(*key).to_string()))
}

fn arrow_for_input<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    input: InputId,
) -> Option<String> {
    loaded
        .controls
        .arrows
        .iter()
        .find_map(|(arrow, id)| (*id == input).then_some(arrow_name(*arrow).to_string()))
}

fn key_triggers_for_input<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    input: InputId,
) -> Vec<String> {
    let mut keys = Vec::new();
    for (key, id) in &loaded.controls.keys {
        if *id == input {
            keys.push(char::from(*key).to_string());
        }
    }
    for (arrow, id) in &loaded.controls.arrows {
        if *id == input {
            keys.push(arrow_name(*arrow).to_string());
        }
    }
    for (name, id) in &loaded.controls.named {
        if *id == input {
            keys.push(name.clone());
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

fn arrow_name(arrow: ArrowKey) -> &'static str {
    match arrow {
        ArrowKey::Up => "ArrowUp",
        ArrowKey::Down => "ArrowDown",
        ArrowKey::Left => "ArrowLeft",
        ArrowKey::Right => "ArrowRight",
    }
}

fn scene_state<const D: usize, Size: GridSize<D>>(
    state: Option<&puzzle_play::GridSceneRuntimeState<D, Size>>,
) -> BTreeMap<String, SceneValue> {
    state
        .map(|state| ordered_scene_values(&state.values))
        .unwrap_or_default()
}

fn scene_resources<'a>(
    loaded: &'a LoadedGame,
    scene_name: &str,
) -> Option<&'a puzzle_lang::SceneResources> {
    loaded
        .scenes
        .iter()
        .find(|scene| scene.name == scene_name)
        .map(|scene| &scene.resources)
}

fn scene_resources_snapshot(
    resources: Option<&puzzle_lang::SceneResources>,
) -> RuntimePuzzle2Resources {
    let Some(resources) = resources else {
        return RuntimePuzzle2Resources::default();
    };
    RuntimePuzzle2Resources {
        levels: Some(resource_selection(&resources.levels)),
        visuals: Some(resource_selection(&resources.visuals)),
    }
}

fn resource_selection(selection: &ResourceSelection) -> RuntimeResourceSelection {
    match selection {
        ResourceSelection::All => RuntimeResourceSelection {
            mode: RuntimeResourceSelectionMode::All,
            names: Vec::new(),
        },
        ResourceSelection::Named(names) => RuntimeResourceSelection {
            mode: RuntimeResourceSelectionMode::Named,
            names: names.clone(),
        },
    }
}

fn level_records<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
) -> BTreeMap<String, RuntimeLevelRecord> {
    assert_eq!(
        loaded.levels.len(),
        session.cleared_levels().len(),
        "loaded levels and session progress must have the same length"
    );
    loaded
        .levels
        .iter()
        .zip(session.cleared_levels())
        .enumerate()
        .map(|(index, (level, cleared))| {
            let id = LevelId::new(&level.puzzle, &level.name);
            let key = id.record_key();
            (
                key.clone(),
                RuntimeLevelRecord {
                    id: key,
                    name: level.name.clone(),
                    puzzle: level.puzzle.clone(),
                    pack: level.pack.clone(),
                    ordinal: index + 1,
                    cleared: *cleared,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use puzzle_lang::ComponentPlacement;
    use serde_json::json;

    fn cell_has_object(cell: &Value, object: &str) -> bool {
        cell["layers"]
            .as_array()
            .is_some_and(|layers| layers.iter().any(|layer| layer["object"] == object))
    }

    fn projected_choice_cursor(snapshot: &Value) -> usize {
        snapshot["surface"]["components"]
            .as_array()
            .and_then(|components| {
                components
                    .iter()
                    .find_map(|component| component["choiceCursor"].as_u64())
            })
            .and_then(|cursor| usize::try_from(cursor).ok())
            .unwrap_or_else(|| {
                panic!(
                    "focused choice component should project its cursor: {}",
                    snapshot["surface"]
                )
            })
    }

    fn standalone_export(source: &str) -> Value {
        let document = puzzle_lang::parse_game_for_path(source, "export_test.puzzle").unwrap();
        serde_json::to_value(StandaloneRuntimeExport::new(document)).unwrap()
    }

    fn runtime_scene_fixture_source() -> &'static str {
        r#"
const title = "Runtime Scene Fixture"

puzzle board {
layers {
@Floor
solid = Player Box Wall
}
input up
input down
input left
input right
rules {
input right [ Player | no solid ] -> [ | Player ]
input left [ no solid | Player ] -> [ Player | ]
input down [ Player ; no solid ] -> [ ; Player ]
input up [ no solid ; Player ] -> [ Player ; ]
}
}

levels microban of board {
legend {
. = empty
P = Player @Floor
B = Box @Floor
# = Wall
}
level "microban.1" {
#######
#P...B#
#.....#
#.....#
#.....#
#######
}
level "microban.2" {
#######
#.P..B#
#.....#
#.....#
#.....#
#######
}
}

scene title {
layout {
heading title
choice "New Game" -> goto playing("microban.1")
if game.has_progress_save {
choice "Continue" -> goto playing
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
"#
    }

    #[test]
    fn native_backend_dispatch_returns_complete_typed_snapshot_without_json() {
        let mut runtime = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "typed_runtime_scene_fixture.puzzle",
        )
        .unwrap();

        let snapshot = runtime.dispatch_typed(SessionAction::Initialize).unwrap();

        assert_eq!(snapshot.revision, 0);
        assert_eq!(snapshot.level_count, 2);
        assert!(!snapshot.surface.components.is_empty());
        assert!(snapshot.scenes.iter().any(|scene| scene.name == "title"));
        assert!(snapshot.scenes.iter().any(|scene| scene.name == "playing"));
        let RuntimeRendererState::TwoD(scene) = snapshot
            .scene
            .as_ref()
            .expect("initialized model scene must be projected")
        else {
            panic!("fixture must project a 2D renderer scene");
        };
        assert_eq!(
            scene.render_scene.instances.len(),
            scene
                .render_scene
                .composition_groups
                .iter()
                .map(|group| group.instances.len())
                .sum::<usize>()
        );
        let json = puzzle_presentation_json::to_value(&snapshot).unwrap();
        assert!(json["scene"]["renderScene"].is_object());
    }

    #[test]
    fn snapshot_materializes_authored_visuals_as_typed_linear_rgba_clips() {
        let source = r#"
const title = typed_visual_scene
puzzle default {
layers {
actor = Player
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
visuals {
visual Player {
colors = #f00
0
}
}
"#;
        let runtime = RuntimeSession::from_source(source, "typed_visual_scene.puzzle").unwrap();
        let snapshot = runtime.snapshot();
        let RuntimeRendererState::TwoD(scene) = snapshot.scene.unwrap() else {
            panic!("fixture must project a 2D renderer scene");
        };

        assert_eq!(scene.render_scene.clips.len(), 1);
        assert_eq!(scene.render_scene.instances.len(), 1);
        let RuntimeResolvedVisualFrame::Pixels { pixels, .. } =
            &scene.render_scene.clips[0].frames[0]
        else {
            panic!("authored ASCII visual must become a pixel clip");
        };
        assert_eq!(pixels[0].color.red, 1.0);
        assert_eq!(pixels[0].color.green, 0.0);
        assert_eq!(pixels[0].color.blue, 0.0);

        let projected: serde_json::Value = serde_json::from_str(
            &runtime
                .renderer_state_json()
                .expect("current state must project independently of the active surface"),
        )
        .expect("typed renderer state must serialize as JSON");
        assert_eq!(projected["width"], 1);
        assert_eq!(projected["height"], 1);
        assert_eq!(
            projected["renderScene"]["clips"].as_array().unwrap().len(),
            1
        );
        assert_eq!(
            projected["renderScene"]["instances"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn standalone_session_from_export_requires_runtime_loaded_document() {
        let export = json!({
            "source": "title invalid\nlevels {\nlegend {\nP = Player\n}\nP\n}\n",
            "puzzlePath": "compiled_export.puzzle",
        });

        let error = match RuntimeSession::from_export_json(&export.to_string()) {
            Ok(_) => panic!("export without runtimeLoadedDocument should be rejected"),
            Err(error) => error,
        };
        assert!(error.contains("runtimeLoadedDocument"));
    }

    #[test]
    fn standalone_session_from_export_rejects_compiler_fields() {
        let source = r#"
const title = export_runtime_bundle
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
level "start" {
P
}
}
}
"#;
        let mut export = standalone_export(source);
        export["source"] = json!("this is not puzzle syntax");
        export["puzzlePath"] = json!("bad_source_path.puzzle");

        let error = match RuntimeSession::from_export_json(&export.to_string()) {
            Ok(_) => panic!("compiler fields must be rejected"),
            Err(error) => error,
        };
        assert!(error.contains("unknown field"), "{error}");
    }

    #[test]
    fn standalone_session_from_export_rejects_unsupported_runtime_version() {
        let mut export = standalone_export(runtime_scene_fixture_source());
        export["runtimeLoadedDocument"]["version"] = json!(
            STANDALONE_RUNTIME_EXPORT_VERSION
                .checked_add(1)
                .expect("test version remains in range")
        );

        let error = match RuntimeSession::from_export_json(&export.to_string()) {
            Ok(_) => panic!("unsupported runtime version must be rejected"),
            Err(error) => error,
        };
        assert!(error.contains("unsupported runtimeLoadedDocument version"));
    }

    #[test]
    fn standalone_session_from_export_rejects_dangling_program_reference() {
        let mut export = standalone_export(runtime_scene_fixture_source());
        export["runtimeLoadedDocument"]["document"]["models"][0]["Puzzle2d"]["game"]["levels"][0]
            ["program"] = json!([{"Catalog": 999}, "Main"]);

        let error = match RuntimeSession::from_export_json(&export.to_string()) {
            Ok(_) => panic!("dangling program reference must be rejected"),
            Err(error) => error,
        };
        assert!(error.contains("invalid program reference"), "{error}");
    }

    #[test]
    fn standalone_session_resumes_rules_after_presentation_wait() {
        let source = r#"
const title = export_wait_segments
puzzle default {
layers {
__legacy_layer_0 = A B C
}
empty .
rules {
[ A ] -> [ C ]
fall
}
routine fall {
[ C ] -> wait 100ms
[ C ] -> [ B ]
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
"#;
        let export = standalone_export(source);
        let mut bridge = RuntimeSession::from_export_json(&export.to_string()).unwrap();

        let waiting: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "right".to_string(),
                })
                .expect("input should complete with a presentation wait"),
        )
        .unwrap();
        assert!(!cell_has_object(&waiting["scene"]["cells"][0], "A"));
        assert!(cell_has_object(&waiting["scene"]["cells"][0], "C"));
        assert!(!cell_has_object(&waiting["scene"]["cells"][0], "B"));
        assert_eq!(waiting["busy"], true);
        assert_eq!(waiting["presentationEvents"][0]["kind"], json!("wait"));
        assert_eq!(waiting["presentationEvents"][0]["milliseconds"], 100);
        let inspected: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Snapshot).unwrap()).unwrap();
        assert_eq!(inspected["presentationEvents"], json!([]));
        assert_eq!(inspected["busy"], true);
        assert_eq!(
            bridge.dispatch(SessionAction::Undo).unwrap_err(),
            "session action is unavailable while a turn is waiting"
        );

        let resumed: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Resume)
                .expect("wait completion should resume the same turn"),
        )
        .unwrap();
        assert!(!cell_has_object(&resumed["scene"]["cells"][0], "C"));
        assert!(cell_has_object(&resumed["scene"]["cells"][0], "B"));
        assert_eq!(resumed["busy"], false);
        assert!(
            bridge
                .dispatch(SessionAction::Resume)
                .unwrap_err()
                .contains("no turn is waiting")
        );

        let undone: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Undo)
                .expect("undo should return to pre-input state"),
        )
        .unwrap();
        assert!(cell_has_object(&undone["scene"]["cells"][0], "A"));
        assert!(!cell_has_object(&undone["scene"]["cells"][0], "B"));
        assert!(!cell_has_object(&undone["scene"]["cells"][0], "C"));
    }

    #[test]
    fn runtime_session_owns_the_single_input_queued_during_a_wait() {
        let source = r#"
const title = queued_input_runtime
puzzle default {
input first
input second
layers { actor = A B C }
empty .
rules {
if input == first {
[ A ] -> [ C ]
fall
}
if input == second {
[ B ] -> [ A ]
}
}
routine fall {
[ C ] -> wait 100ms
[ C ] -> [ B ]
}
levels {
legend {
A = A
B = B
C = C
}
level "start" { A }
}
}
"#;
        let mut runtime = RuntimeSession::from_source(source, "queued_input_runtime.puzzle")
            .expect("queued input fixture should compile");
        let waiting: Value = serde_json::from_str(
            &runtime
                .dispatch(SessionAction::Input {
                    name: "first".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        let queued: Value = serde_json::from_str(
            &runtime
                .dispatch(SessionAction::Input {
                    name: "second".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            queued["revision"],
            waiting["revision"].as_u64().unwrap() + 1
        );
        assert_eq!(queued["busy"], true);

        let resumed: Value =
            serde_json::from_str(&runtime.dispatch(SessionAction::Resume).unwrap()).unwrap();
        assert_eq!(resumed["busy"], false);
        assert!(cell_has_object(&resumed["scene"]["cells"][0], "A"));

        let first_undo: Value =
            serde_json::from_str(&runtime.dispatch(SessionAction::Undo).unwrap()).unwrap();
        assert!(cell_has_object(&first_undo["scene"]["cells"][0], "B"));
        let second_undo: Value =
            serde_json::from_str(&runtime.dispatch(SessionAction::Undo).unwrap()).unwrap();
        assert!(cell_has_object(&second_undo["scene"]["cells"][0], "A"));
    }

    #[test]
    fn standalone_snapshot_serializes_mixed_presentation_events_in_authored_order() {
        let source = r#"
const title = runtime_mixed_presentation
default_wait_time = 40ms
sounds {
sfx tick {
seed = tick01
type = hit
}
}
puzzle default {
render {
tween = true
tween_duration = 80ms
}
layers {
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
}
}
"#;
        let mut bridge =
            RuntimeSession::from_source(source, "runtime_mixed_presentation.puzzle").unwrap();

        let mut snapshot: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "right".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        let mut event_kinds = Vec::new();
        loop {
            let events = snapshot["presentationEvents"].as_array().unwrap();
            assert!(events.iter().all(|event| event["levelIndex"] == 0));
            event_kinds.extend(
                events
                    .iter()
                    .map(|event| event["kind"].as_str().unwrap().to_string()),
            );
            if snapshot["busy"] != true {
                break;
            }
            let modal = snapshot["surface"]["components"]
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|component| component["modal"] == true);
            snapshot = if let Some(component) = modal {
                assert_eq!(component["presentation"]["components"][0]["value"], "ready");
                assert_eq!(
                    component["presentation"]["events"][STANDARD_MESSAGE_DISMISS_EVENT]["pointer"],
                    true
                );
                event_kinds.push(format!(
                    "component:{}",
                    component["definition"].as_str().unwrap()
                ));
                serde_json::from_str(
                    &bridge
                        .dispatch(SessionAction::ComponentEvent {
                            instance: component["id"].as_str().unwrap().to_string(),
                            event: component["awaitEvent"].as_str().unwrap().to_string(),
                        })
                        .unwrap(),
                )
                .unwrap()
            } else {
                serde_json::from_str(&bridge.dispatch(SessionAction::Resume).unwrap()).unwrap()
            };
        }
        assert_eq!(
            event_kinds,
            vec![
                "wait",
                "component:standard.message",
                "animation_batch",
                "play_sfx",
                "wait"
            ]
        );
        assert!(cell_has_object(&snapshot["scene"]["cells"][1], "Done"));
    }

    #[test]
    fn standalone_snapshot_projects_each_presented_component_instance_state() {
        let source = r#"
const title = runtime_component_instances

puzzle default {
layers { actor = Player }
empty .
rules {}
levels {
legend { P = Player }
level "start" { P }
}
}

scene playing {
layout { puzzle board = default }
}

scene panel {
var count = 1
layout { text count }
}
"#;
        let mut bridge =
            RuntimeSession::from_source(source, "runtime_component_instances.puzzle").unwrap();
        let present = SceneEffect::PresentComponent {
            definition: "panel".to_string(),
            properties: Vec::new(),
            placement: ComponentPlacement::Content,
            await_event: None,
        };

        let _ = bridge
            .dispatch(SessionAction::SceneEffect {
                effect: present.clone(),
            })
            .unwrap();
        let snapshot: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::SceneEffect { effect: present })
                .unwrap(),
        )
        .unwrap();
        let panels = snapshot["surface"]["components"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|component| component["definition"] == "panel")
            .collect::<Vec<_>>();

        assert_eq!(panels.len(), 2);
        assert_ne!(panels[0]["id"], panels[1]["id"]);
        assert_eq!(panels[0]["sceneState"]["count"], 1);
        assert_eq!(panels[1]["sceneState"]["count"], 1);
        assert_eq!(panels[0]["presentation"]["components"][0]["value"], "1");
        assert_eq!(panels[1]["presentation"]["components"][0]["value"], "1");
    }

    #[cfg(not(feature = "editor-debug"))]
    #[test]
    fn player_session_contract_rejects_editor_debug_input() {
        let mut bridge = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "runtime_scene_fixture.puzzle",
        )
        .unwrap();
        let error = bridge
            .dispatch(SessionAction::DebugInput {
                name: "right".to_string(),
            })
            .unwrap_err();
        assert_eq!(error, "debug input is unavailable in the player runtime");
    }

    #[cfg(feature = "editor-debug")]
    #[test]
    fn standalone_session_debug_input_reports_rule_trace() {
        let source = r#"
const title = "Debug Trace"

puzzle main {
  layers {
    actor = Player
  }

  rules {
    [ Player ] -> [ ]
  }
}

levels main of main {
  legend {
    . = empty
    P = Player
  }

  level "one"
  P
}
"#;
        let export = standalone_export(source);
        let mut bridge = RuntimeSession::from_export_json(&export.to_string()).unwrap();

        let body: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::DebugInput {
                    name: "right".to_string(),
                })
                .unwrap(),
        )
        .unwrap();

        assert_eq!(body["snapshot"]["surface"]["focus"], "main");
        assert_eq!(body["debug"]["input"], "right");
        assert_eq!(body["debug"]["executions"].as_array().unwrap().len(), 1);
        assert_eq!(body["debug"]["executions"][0]["patch"][0]["kind"], "remove");
    }

    #[test]
    fn standalone_session_bridge_uses_rust_session_for_requests() {
        let source = runtime_scene_fixture_source();
        let mut bridge =
            RuntimeSession::from_source(source, "runtime_scene_fixture.puzzle").unwrap();

        let initial: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Snapshot).unwrap()).unwrap();
        assert_eq!(initial["surface"]["focus"], "board");
        assert!(initial.get("title").is_none());
        assert_eq!(initial["gameState"]["title"], "Runtime Scene Fixture");
        assert_eq!(initial["solverState"]["kind"], "2d");
        assert!(initial["solverState"]["slots"].is_array());
        assert!(initial["solverState"].get("slotMarks").is_none());
        assert!(initial["solverState"].get("cellMarks").is_none());
        let title = initial["scenes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scene| scene["name"] == "title")
            .unwrap();
        assert_eq!(title["components"][0]["kind"], "text");
        assert_eq!(title["components"][0]["role"], "heading");

        let playing: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "goto playing".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(playing["surface"]["focus"], "playing");
        assert_eq!(playing["levelIndex"], 0);

        let save: Value = serde_json::from_str(&bridge.progress_save_json()).unwrap();
        assert_eq!(
            save["currentLevel"],
            LevelId::new("board", "microban.1").record_key()
        );
    }

    #[test]
    fn snapshot_resolves_scene_expressions_before_browser_projection() {
        let mut bridge = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "runtime_scene_fixture.puzzle",
        )
        .unwrap();

        let snapshot: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "goto title".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        let title = &snapshot["surface"]["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|component| component["definition"] == "title")
            .unwrap()["presentation"];
        assert_eq!(title["components"][0]["value"], "Runtime Scene Fixture");
        let conditional = title["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|component| component["kind"] == "conditional")
            .unwrap();

        assert_eq!(conditional["condition"], false);
        assert!(conditional["children"][0]["label"].is_string());
        assert!(conditional["condition"].is_boolean());
    }

    #[test]
    fn scene_preview_uses_the_same_rust_expression_resolver() {
        let mut bridge = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "runtime_scene_fixture.puzzle",
        )
        .unwrap();

        let without_save: Value = serde_json::from_str(
            &bridge
                .resolve_scene_presentation_json("title", "{}")
                .unwrap(),
        )
        .unwrap();
        let without_save_conditional = without_save["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|component| component["kind"] == "conditional")
            .unwrap();
        assert_eq!(without_save_conditional["condition"], false);

        bridge.has_persisted_progress = true;
        let with_save: Value = serde_json::from_str(
            &bridge
                .resolve_scene_presentation_json("title", "{}")
                .unwrap(),
        )
        .unwrap();
        let with_save_conditional = with_save["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|component| component["kind"] == "conditional")
            .unwrap();
        assert_eq!(with_save_conditional["condition"], true);
    }

    #[test]
    fn runtime_session_projects_and_acts_on_owned_choice_cursor() {
        let source = r#"
const title = runtime_choice_cursor

puzzle board {
  layers { actor = Player }
  empty .
  rules {
  }
}

levels default of board {
  legend P = Player
  level "one" { P }
}

scene menu {
  layout {
    puzzle board
    row {
      choice "A" -> goto a
      choice "B" -> goto b
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
    text "B"
  }
}
"#;
        let mut runtime = RuntimeSession::from_source(source, "runtime_choice_cursor.puzzle")
            .expect("choice cursor fixture should compile");
        runtime
            .dispatch(SessionAction::Command {
                name: "goto menu".to_string(),
            })
            .expect("choice fixture should enter the menu component");

        let initial: Value = serde_json::from_str(&runtime.snapshot_json()).unwrap();
        assert_eq!(projected_choice_cursor(&initial), 0);

        let moved: Value = serde_json::from_str(
            &runtime
                .dispatch(SessionAction::ChoiceMove {
                    direction: RuntimeChoiceDirection::Right,
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(projected_choice_cursor(&moved), 1);

        let activated: Value = serde_json::from_str(
            &runtime
                .dispatch(SessionAction::ChoiceActivate { index: None })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(activated["surface"]["focus"], "b");
    }

    #[test]
    fn standalone_session_bridge_reports_no_scene_for_non_model_focus() {
        let source = r#"
const title = runtime_focus
puzzle default {
layers {
actor = Player
}
empty .
rules {
down [ Player | no Player ] -> [ | Player ]
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
text "Select"
}
}
"#;
        let mut bridge = RuntimeSession::from_source(source, "runtime_focus.puzzle").unwrap();

        let select: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "goto level_select".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(select["surface"]["focus"], json!("level_select"));
        assert!(select["scene"].is_null());
        assert!(select["surface"]["components"][0]["scene"].is_null());

        let after_input: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "down".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(after_input["surface"]["focus"], json!("level_select"));
        assert!(after_input["scene"].is_null());
        assert_eq!(after_input["presentationEvents"], json!([]));
    }

    #[test]
    fn standalone_session_bridge_starts_from_editor_state() {
        let source = r#"
const title = editor_state_start

puzzle board {
layers {
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
"#;
        let loaded = puzzle_lang::parse_game2d(source).unwrap();
        let editor_state = compiled_state_value(&loaded.levels[1].initial_state).to_string();
        let mut bridge = RuntimeSession::from_source(source, "editor_state_start.puzzle").unwrap();

        bridge
            .set_current_state_json(&editor_state, 0, false)
            .expect("editor state should start level 0");
        let started: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(started["levelIndex"], json!(0));
        assert!(cell_has_object(&started["scene"]["cells"][0], "Player"));

        let moved: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "right".to_string(),
                })
                .expect("right should move from editor state"),
        )
        .unwrap();
        assert!(cell_has_object(&moved["scene"]["cells"][1], "Player"));

        let restarted: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "restart".to_string(),
                })
                .expect("restart should use editor start state"),
        )
        .unwrap();
        assert!(cell_has_object(&restarted["scene"]["cells"][0], "Player"));

        assert!(
            bridge
                .set_current_state_json(&editor_state, 99, false)
                .unwrap_err()
                .contains("level index out of range: 99")
        );
    }

    #[test]
    fn spec_2d_new_game_uses_scene_input_and_scene_puzzle_state() {
        let source = runtime_scene_fixture_source();
        let mut bridge =
            RuntimeSession::from_source(source, "runtime_scene_fixture.puzzle").unwrap();

        let title: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Snapshot).unwrap()).unwrap();
        assert!(
            title["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|input| { input["name"] == "up" && input["arrow"] == "ArrowUp" })
        );

        let playing = start_spec_2d_new_game(&mut bridge);
        assert_eq!(playing["surface"]["focus"], "playing");
        assert_eq!(playing["levelIndex"], 0);
        assert_eq!(playing["scenePuzzles"], json!(["board"]));
        let playing_object = playing.as_object().unwrap();
        assert!(playing_object.contains_key("surface"));
        assert!(!playing_object.contains_key("visibleScenes"));
        assert!(!playing_object.contains_key("sceneLayers"));
        assert!(!playing_object.contains_key("currentScene"));
        assert!(playing_object.contains_key("sceneState"));
        assert!(playing_object.contains_key("scenePuzzles"));
        assert!(!playing_object.contains_key("visibleScreens"));
        assert!(!playing_object.contains_key("screenState"));
        assert!(!playing_object.contains_key("screenPuzzles"));
        assert!(
            playing["scenePuzzleState"]["board"]
                .as_object()
                .is_some_and(|state| !state.contains_key("level"))
        );
        assert_eq!(playing["scene"]["cells"].as_array().unwrap().len(), 42);
        assert!(
            playing["scene"]["cells"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|cell| cell["layers"].as_array().unwrap())
                .any(|layer| layer["object"] == "@Floor")
        );
    }

    #[test]
    fn runtime_construction_reports_initial_level_start_failure() {
        let error = RuntimeSession::from_source(
            r#"
const title = runtime_initial_level_start_failure
puzzle board {
var count = 1
layers { actor = Player }
empty .
on_level_start {
count /= 0
}
rules {}
levels {
legend { P = Player }
level "start" { P }
}
}
"#,
            "runtime_initial_level_start_failure.puzzle",
        )
        .err()
        .expect("failing initial lifecycle must reject runtime construction");

        assert!(error.contains("VariableDivisionByZero"), "{error}");
    }

    #[test]
    fn transition_flickscreen_focuses_player_group() {
        let puzzlescript =
            include_str!("../../../crates/lang/tests/fixtures/puzzlescript/gallery/transition.ps");
        let source = puzzle_lang::translate_puzzlescript_to_canonical(puzzlescript).unwrap();
        let mut bridge =
            RuntimeSession::from_source(&source, "fixtures/transition.puzzle").unwrap();

        let playing: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "goto playing(0)".to_string(),
                })
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            playing["scene"]["screen"]["viewportSize"],
            json!({"kind": "size", "width": 13, "height": 13})
        );
        assert_eq!(playing["scene"]["screen"]["viewportFocus"], "player");
        assert!(
            playing["scene"]["screen"]["viewportFocusObjects"]
                .as_array()
                .unwrap()
                .iter()
                .all(|id| id.as_u64().is_some_and(|id| id > 0))
        );
        assert!(
            playing["scene"]["screen"]["viewportFocusObjects"]
                .as_array()
                .unwrap()
                .len()
                >= 2
        );
    }

    #[test]
    fn spec_2d_direction_input_changes_state_after_new_game() {
        let source = runtime_scene_fixture_source();
        let mut changed_input = None;

        for input in ["up", "down", "left", "right"] {
            let mut bridge =
                RuntimeSession::from_source(source, "runtime_scene_fixture.puzzle").unwrap();
            let before = start_spec_2d_new_game(&mut bridge);
            let after: Value = serde_json::from_str(
                &bridge
                    .dispatch(SessionAction::Input {
                        name: input.to_string(),
                    })
                    .unwrap(),
            )
            .unwrap();

            if before["scene"] != after["scene"] || before["canUndo"] != after["canUndo"] {
                changed_input = Some(input);
                break;
            }
        }

        assert!(changed_input.is_some());
    }

    fn start_spec_2d_new_game(bridge: &mut RuntimeSession) -> Value {
        bridge
            .dispatch(SessionAction::Command {
                name: "clear_game_progress".to_string(),
            })
            .unwrap();
        serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "goto playing(\"microban.1\")".to_string(),
                })
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn standalone_session_bridge_emits_tween_on_first_input() {
        let source = r#"
const title = "Runtime Tween Fixture"

puzzle mover {
  render {
    tween = true
    tween_duration = 300ms
  }
  layers {
    actor = Player
  }
  rules {
    input directions [ Player | no Player ] -> [ | Player ]
  }
}

levels default of mover {
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
    puzzle board = mover
  }
}
"#;
        let mut bridge =
            RuntimeSession::from_source(source, "runtime_tween_fixture.puzzle").unwrap();

        let playing: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "goto playing".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(playing["surface"]["focus"], "playing");
        assert_eq!(playing["levelIndex"], 0);

        let moved: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "right".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(moved["presentationEvents"][0]["kind"], "animation_batch");
        assert_eq!(
            moved["presentationEvents"][0]["animations"][0],
            json!({
                "kind": "move",
                "name": "tween",
                "occurrenceId": 1,
                "objectId": 1,
                "from": { "x": 0, "y": 0 },
                "to": { "x": 1, "y": 0 }
            }),
            "unexpected moved snapshot: {moved}"
        );
    }

    #[test]
    fn spatial_session_bridge_emits_dimensioned_tween_for_addressed_puzzle() {
        let source = r#"
const title = "Spatial Tween Fixture"

puzzle mover {
  dimension = 3
  render {
    tween = true
    tween_duration = 120ms
  }
  layers {
    actor = Player
  }
  rules {
    input right [ Player | no Player ] -> [ | Player ]
  }
}

levels default of mover {
  legend {
    . = empty
    P = Player
  }
  level "first" {
    P.
  }
}
"#;
        let mut bridge = RuntimeSession::from_source(source, "spatial_tween_fixture.puzzle")
            .expect("compile spatial tween fixture");

        let moved: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "right".to_string(),
                })
                .expect("apply spatial model input"),
        )
        .unwrap();

        assert_eq!(moved["animation"]["tween"]["intervalMs"], 120);
        assert_eq!(
            moved["presentationEvents"],
            json!([{
                "scene": "mover",
                "puzzle": "mover",
                "levelIndex": 0,
                "kind": "animation_batch",
                "animations": [{
                    "kind": "move",
                    "name": "tween",
                    "occurrenceId": 1,
                    "objectId": 1,
                    "from": { "x": 0, "y": 0, "z": 0 },
                    "to": { "x": 1, "y": 0, "z": 0 }
                }]
            }])
        );
        assert_eq!(
            moved["scenePuzzleState"]["mover"]["cells"],
            json!([{
                "position": { "x": 1, "y": 0, "z": 0 },
                "renderOrder": 1,
                "objects": [{ "id": 1, "layer": 0, "renderOrder": 1 }]
            }])
        );
    }

    #[test]
    fn standalone_session_bridge_restores_progress_save() {
        let source = runtime_scene_fixture_source();
        let mut bridge =
            RuntimeSession::from_source(source, "runtime_scene_fixture.puzzle").unwrap();
        let restored_level_key = LevelId::new("board", "microban.2").record_key();
        bridge
            .restore_progress_save_json(
                &json!({
                    "version": 2,
                    "levels": [{"id": restored_level_key, "cleared": true}],
                    "currentLevel": restored_level_key,
                    "persistentVars": [],
                })
                .to_string(),
            )
            .unwrap();

        let snapshot: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Snapshot).unwrap()).unwrap();
        assert_eq!(snapshot["selectedLevelIndex"], 1);
        assert_eq!(snapshot["has_progress_save"], true);
        assert_eq!(
            snapshot["levels"][&restored_level_key]["progress"]["cleared"],
            true
        );
        let save: Value = serde_json::from_str(&bridge.progress_save_json()).unwrap();
        assert!(save["levels"].as_array().is_some_and(|levels| {
            levels
                .iter()
                .any(|level| level["id"] == restored_level_key && level["cleared"] == true)
        }));
    }

    #[test]
    fn progress_is_persisted_only_after_the_matching_host_acknowledgement() {
        let mut runtime = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "runtime_scene_fixture.puzzle",
        )
        .unwrap();
        let after_action: Value = serde_json::from_str(
            &runtime
                .dispatch(SessionAction::Command {
                    name: "goto playing".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(after_action["has_progress_save"], false);
        let request = runtime
            .progress_save_request()
            .expect("successful session mutation should request persistence");
        assert!(
            runtime
                .confirm_progress_save_written(request.request_id + 1)
                .unwrap_err()
                .contains("stale")
        );
        assert_eq!(
            serde_json::from_str::<Value>(&runtime.snapshot_json()).unwrap()["has_progress_save"],
            false
        );

        runtime
            .confirm_progress_save_written(request.request_id)
            .unwrap();
        assert!(runtime.progress_save_request().is_none());
        assert_eq!(
            serde_json::from_str::<Value>(&runtime.snapshot_json()).unwrap()["has_progress_save"],
            true
        );
        runtime.confirm_progress_save_cleared();
        assert_eq!(
            serde_json::from_str::<Value>(&runtime.snapshot_json()).unwrap()["has_progress_save"],
            false
        );
    }

    #[test]
    fn standalone_session_bridge_rejects_name_only_progress_entries() {
        let mut bridge = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "runtime_scene_fixture.puzzle",
        )
        .unwrap();
        let error = bridge
            .restore_progress_save_json(
                r#"{"version":2,"levels":[{"name":"microban.2","cleared":true}],"currentLevel":null,"persistentVars":[]}"#,
            )
            .unwrap_err();
        assert!(
            error.contains("unknown field `name`") || error.contains("missing field `id`"),
            "{error}"
        );
    }

    #[test]
    fn standalone_session_runs_spatial_document_through_shared_scene_session() {
        let source = include_str!("../../lang/tests/fixtures/spec_3d_full.puzzle");
        let mut bridge = RuntimeSession::from_source(source, "spec_3d_full.puzzle")
            .expect("spatial document should use the shared grid session");

        let snapshot: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(snapshot["surface"]["focus"], "sokoban");
        assert_eq!(snapshot["solverState"]["kind"], "puzzle3d");
        assert!(snapshot["solverState"].get("slotMarks").is_none());
        assert!(snapshot["solverState"].get("cellMarks").is_none());
        let spatial_view = snapshot["scenePuzzleState"]["sokoban"].clone();
        let typed_view: RuntimePuzzle3Snapshot = serde_json::from_value(spatial_view.clone())
            .expect("Puzzle3 session view should satisfy the complete typed runtime contract");
        assert!(!typed_view.cells.is_empty());
        assert!(
            typed_view
                .render_scene
                .cells
                .iter()
                .any(|cell| !cell.object_ids.is_empty())
        );
        let authoring_resources = snapshot["puzzle3AuthoringResources"]
            .as_object()
            .expect("3D authoring resources must be outside renderer state");
        assert!(authoring_resources["inputs"].as_array().is_some_and(|inputs| !inputs.is_empty()));
        assert!(authoring_resources["objects"].as_object().is_some_and(|objects| !objects.is_empty()));
        assert!(authoring_resources["visuals"].as_object().is_some_and(|visuals| !visuals.is_empty()));
        assert!(spatial_view.get("objects").is_none());
        assert!(spatial_view.get("visuals").is_none());
        assert!(spatial_view.get("render").is_some());
        assert!(spatial_view.get("order").is_none());
        assert!(authoring_resources["order"].is_object());
        let second: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::GotoLevel { level: 1 })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(second["levelIndex"], 1);
        let first: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::PreviousLevel).unwrap()).unwrap();
        assert_eq!(first["levelIndex"], 0);
        bridge
            .dispatch(SessionAction::Input {
                name: "right".to_string(),
            })
            .expect("spatial model input should route through the shared session");
    }

    #[derive(Debug, PartialEq, Eq)]
    struct DimensionIndependentSessionTrace {
        slots: Vec<u16>,
        can_undo: bool,
        can_redo: bool,
        active_level_index: Option<usize>,
    }

    fn session_trace<const D: usize, Size: GridSize<D>>(
        loaded: &LoadedGridGame<D, Size>,
    ) -> Vec<DimensionIndependentSessionTrace> {
        fn capture<const D: usize, Size: GridSize<D>>(
            session: &GridGameSession<D, Size>,
        ) -> DimensionIndependentSessionTrace {
            DimensionIndependentSessionTrace {
                slots: session
                    .state()
                    .slots()
                    .iter()
                    .map(|object| object.0)
                    .collect(),
                can_undo: session.can_undo(),
                can_redo: session.can_redo(),
                active_level_index: session.active_level_index(),
            }
        }

        let mut session = GridGameSession::new(loaded);
        let mut trace = vec![capture(&session)];
        session.apply_input(loaded, InputId(0)).unwrap();
        trace.push(capture(&session));
        session.undo(loaded);
        trace.push(capture(&session));
        session.redo(loaded);
        trace.push(capture(&session));
        session.restart_level(loaded).unwrap();
        trace.push(capture(&session));
        trace
    }

    #[test]
    fn shared_session_actions_have_dimension_independent_semantics() {
        let model = |dimension: &str, path: &str| {
            puzzle_lang::parse_game_for_path(
                &format!(
                    r#"
const title = "Session parity"
puzzle board {{
  {dimension}
  layers {{ actor = A B C D E }}
  rules {{ [ C ] -> [ D ] }}
}}
levels default of board {{
  legend {{ A = A }}
  level "one" {{
    on_level_start {{ [ A ] -> [ B ] }}
    rules before {{ [ B ] -> [ C ] }}
    A
    rules {{ [ D ] -> [ E ] }}
  }}
}}
"#
                ),
                path,
            )
            .unwrap()
        };
        let flat = model("", "session_parity.puzzle");
        let spatial = model("dimension = 3", "session_parity.puzzle");
        let Some(LoadedDocumentModel::Puzzle2d { game: flat, .. }) = flat.single_model() else {
            panic!("expected 2D model");
        };
        let Some(LoadedDocumentModel::Puzzle3d { game: spatial, .. }) = spatial.single_model()
        else {
            panic!("expected 3D model");
        };

        assert_eq!(flat.levels[0].program.references().len(), 3);
        assert!(matches!(
            flat.levels[0].program.references(),
            [
                puzzle_core::GridProgramRef::Catalog(_),
                puzzle_core::GridProgramRef::Main,
                puzzle_core::GridProgramRef::Catalog(_)
            ]
        ));
        assert_eq!(flat.levels[0].program, spatial.levels[0].program);
        assert_eq!(
            flat.levels[0].level_start_program,
            spatial.levels[0].level_start_program
        );
        assert_eq!(
            flat.levels[0].level_clear_program,
            spatial.levels[0].level_clear_program
        );
        assert_eq!(
            flat.program_catalog.programs().len(),
            spatial.program_catalog.programs().len()
        );
        assert_eq!(session_trace(flat), session_trace(spatial));
    }
}
