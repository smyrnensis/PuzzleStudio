use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use puzzle_audio::{AudioAssetCatalog, AudioCommand, MusicRecipe, MusicTarget, SfxRecipe};
#[cfg(any(feature = "editor-debug", test))]
use puzzle_core::GridState;
#[cfg(feature = "editor-debug")]
use puzzle_core::{
    GridCompiledGame, GridCoord, GridPatch, MarkId, MarkValueMatch as CoreMarkValueMatch, PatchOp,
    RuleId, TransitionCommand, VariableId, VariableUpdateOp,
};
use puzzle_core::{GridSize, InputId, ObjectId, Size2, Size3, State as PuzzleState};
use puzzle_lang::{
    ArrowKey, KeyTrigger, LoadedDocument, LoadedDocumentModel, LoadedGame, LoadedGridGame,
    PuzzleGridMode, STANDARD_MESSAGE_COMPONENT, STANDARD_MESSAGE_DISMISS_EVENT,
    STANDARD_MESSAGE_TEXT_PROPERTY, SceneComponent, SceneDef, SceneEffect, SceneExpr,
    SceneLayoutDef, SceneTextContent, SceneTextRoleDef, SceneValue, ThemeDef, ViewportModeDef,
    ViewportProjectionDef, ViewportSizeDef, VisualFitMode as LoadedVisualFitMode, VisualKind,
    VisualSampling as LoadedVisualSampling, VisualSpace as LoadedVisualSpace,
    VisualTransform as LoadedVisualTransform,
};
#[cfg(any(feature = "editor-debug", test))]
use puzzle_lang::{Level, LevelId, ResourceSelection};
#[cfg(feature = "editor-debug")]
use puzzle_play::GridTransitionTrace;
use puzzle_play::{
    GameSession, GridGameSession, PresentationContinuation, ProgressSaveData,
    SceneConditionContext, SharedSceneRuntimeState, SoundEvent, presentation_events_contract,
    scene_value_to_string,
};
use puzzle_presentation::{
    ViewMode2d, VisualOrderRef, VisualPriorityRef, cell_render_order_2d,
    resolve_grid_decoration_2d, resolve_grid_decoration_3d, resolve_object_priority,
    resolve_pixel_frame, resolve_runtime_theme, resolve_view_2d, resolve_visual_affine,
    resolve_voxel_frame,
};
#[cfg(any(feature = "editor-debug", test))]
use puzzle_runtime_contract::SolverStateSnapshot;
use puzzle_runtime_contract::{
    RuntimeChoiceDirection, RuntimeGridMode, RuntimeKeyTrigger,
    RuntimeModelPresentationContinuationToken, RuntimePresentationContinuationToken,
    RuntimePresentationEvent, RuntimeProgressPersistenceOperation, RuntimeProgressSaveRequest,
    RuntimePuzzle3Resources, RuntimePuzzle3Snapshot, RuntimeResolvedCompositionGroup,
    RuntimeResolvedFitMode, RuntimeResolvedPlayback, RuntimeResolvedRenderCell,
    RuntimeResolvedRenderInstance, RuntimeResolvedRenderScene, RuntimeResolvedSampling,
    RuntimeResolvedVisualClip, RuntimeResolvedVisualFrame, RuntimeResolvedVisualLayout,
    RuntimeSceneActionId, RuntimeSceneActionToken, RuntimeViewportSourceId,
    RuntimeVisualComposition, RuntimeVisualSpace, RuntimeVisualTransform, SessionAction,
};
#[cfg(feature = "editor-debug")]
use puzzle_runtime_contract::{RuntimeStateSnapshot2d, RuntimeStateSnapshot3d};
use puzzle_scene::{ComponentActionCell, component_action_cells, component_choice_cells};
use puzzle_session_contract::{
    RuntimeAnimationSettings, RuntimeComponentPresentation, RuntimeInputBinding,
    RuntimeInputBufferSettings, RuntimeKeyBinding, RuntimePuzzle2Cell, RuntimePuzzle2Layer,
    RuntimePuzzle2Snapshot, RuntimeRendererState, RuntimeResolvedEventBinding,
    RuntimeResolvedScene, RuntimeResolvedSceneComponent, RuntimeSessionSnapshot, RuntimeSurface,
    RuntimeSurfaceComponent, RuntimeTheme, RuntimeTweenSettings, RuntimeViewportDimension,
};
#[cfg(any(feature = "editor-debug", test))]
use puzzle_session_contract::{
    RuntimeDevelopmentProjection, RuntimeDevelopmentRendererState,
    RuntimeDevelopmentSessionSnapshot, RuntimeLevelRecord, RuntimePuzzle2DevelopmentSnapshot,
    RuntimePuzzle2Resources, RuntimePuzzle2Settings, RuntimeRegion2d, RuntimeRender2d,
    RuntimeResourceSelection, RuntimeResourceSelectionMode,
};
#[cfg(any(feature = "editor-debug", test))]
use serde_json::Value;
#[cfg(feature = "editor-debug")]
use serde_json::json;

#[cfg_attr(
    not(feature = "solver-session"),
    doc = r#"
The solver session extraction surface is not part of the player contract.

```compile_fail
use puzzle_game_runtime::RuntimeSession;

fn solver_session_is_not_available(runtime: &RuntimeSession) {
    let _ = runtime.solver_session_2d();
}
```
"#
)]
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

#[cfg(feature = "editor-debug")]
pub struct RuntimeDebugDispatch {
    pub snapshot: RuntimeSessionSnapshot,
    pub debug: Value,
}

trait StandaloneSessionModel {
    fn player_snapshot(
        &self,
        revision: u64,
        condition_context: SceneConditionContext,
        presentation_events: &[RuntimePresentationEvent],
    ) -> RuntimeSessionSnapshot;
    #[cfg(any(feature = "editor-debug", test))]
    fn development_projection(
        &self,
        player: &RuntimeSessionSnapshot,
    ) -> RuntimeDevelopmentProjection;
    fn input_bindings(&self) -> Vec<RuntimeInputBinding>;
    fn level_count(&self) -> usize;
    fn take_presentation_events(&mut self) -> Vec<RuntimePresentationEvent>;
    fn audio_catalog(&self) -> Arc<AudioAssetCatalog>;
    fn is_waiting(&self) -> bool;
    fn accepts_model_input(&self) -> bool;
    fn queues_input_while_waiting(&self) -> bool;
    fn has_input_name(&self, input_name: &str) -> bool;
    fn presentation_waits(&self) -> Vec<RuntimeModelPresentationContinuationToken>;
    fn validate_presentation_waits(
        &self,
        waits: &[RuntimeModelPresentationContinuationToken],
    ) -> Result<(), String>;
    fn complete_presentation_waits(
        &mut self,
        waits: &[RuntimeModelPresentationContinuationToken],
    ) -> Result<(), String>;
    fn apply_input_name(&mut self, input_name: &str) -> Result<(), String>;
    fn apply_choice_move(
        &mut self,
        direction: RuntimeChoiceDirection,
        condition_context: SceneConditionContext,
    ) -> Result<(), String>;
    fn validate_choice_move(
        &self,
        direction: RuntimeChoiceDirection,
        condition_context: SceneConditionContext,
    ) -> Result<(), String>;
    fn preview_choice_move(
        &self,
        direction: RuntimeChoiceDirection,
        condition_context: SceneConditionContext,
    ) -> Result<SharedSceneRuntimeState, String>;
    fn apply_scene_action(
        &mut self,
        token: &RuntimeSceneActionToken,
        condition_context: SceneConditionContext,
    ) -> Result<(), String>;
    fn validate_scene_action(
        &self,
        token: &RuntimeSceneActionToken,
        condition_context: SceneConditionContext,
    ) -> Result<(), String>;
    fn preview_scene_action(
        &self,
        token: &RuntimeSceneActionToken,
        condition_context: SceneConditionContext,
    ) -> Result<SharedSceneRuntimeState, String>;
    fn shared_scene_runtime_state(&self) -> SharedSceneRuntimeState;
    fn validate_shared_scene_state(&self, shared: &SharedSceneRuntimeState) -> Result<(), String>;
    fn synchronize_shared_scene_state(
        &mut self,
        shared: &SharedSceneRuntimeState,
    ) -> Result<(), String>;
    #[cfg(feature = "editor-debug")]
    fn apply_debug_input_name(
        &mut self,
        input_name: &str,
        condition_context: SceneConditionContext,
    ) -> Result<RuntimeDebugDispatch, String>;
    fn undo(&mut self);
    fn redo(&mut self);
    fn restart(&mut self) -> Result<(), String>;
    fn next_level(&mut self) -> Result<(), String>;
    fn previous_level(&mut self) -> Result<(), String>;
    fn goto_level(&mut self, level: usize) -> Result<(), String>;
    #[cfg(feature = "editor-debug")]
    fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String>;
    fn progress_save_data(&self) -> ProgressSaveData;
    fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String>;
    fn take_progress_clear_request(&mut self) -> bool;
    #[cfg(feature = "solver-session")]
    fn solver_session_2d(&self) -> Option<(LoadedGame, GameSession)>;
}

struct GridSessionRuntime<const D: usize, Size: GridSize<D>, Projection> {
    model_name: String,
    viewport_models: BTreeMap<(String, String), String>,
    loaded: LoadedGridGame<D, Size>,
    session: GridGameSession<D, Size>,
    projection: Projection,
    theme: RuntimeTheme,
    audio_catalog: Arc<AudioAssetCatalog>,
}

struct DocumentSessionRuntime {
    models: Vec<Box<dyn StandaloneSessionModel>>,
    scene_owner: usize,
}

trait GridSessionProjection<const D: usize, Size: GridSize<D>> {
    #[cfg(feature = "editor-debug")]
    fn decode_state_json(
        &self,
        game: &GridCompiledGame<D>,
        state_json: &str,
    ) -> Result<GridState<D, Size>, String>;

    fn player_viewport_sources(
        &self,
        loaded: &LoadedGridGame<D, Size>,
        session: &GridGameSession<D, Size>,
        theme: &RuntimeTheme,
    ) -> ProjectedPlayerGridSnapshot;

    #[cfg(any(feature = "editor-debug", test))]
    fn development_viewport_sources(
        &self,
        loaded: &LoadedGridGame<D, Size>,
        session: &GridGameSession<D, Size>,
    ) -> BTreeMap<RuntimeViewportSourceId, RuntimeDevelopmentRendererState>;

    #[cfg(any(feature = "editor-debug", test))]
    fn solver_state(&self, state: &GridState<D, Size>) -> SolverStateSnapshot;

    #[cfg(feature = "solver-session")]
    fn solver_session_2d(
        &self,
        _loaded: &LoadedGridGame<D, Size>,
        _session: &GridGameSession<D, Size>,
    ) -> Option<(LoadedGame, GameSession)> {
        None
    }
}

struct ProjectedPlayerGridSnapshot {
    viewport_sources: BTreeMap<RuntimeViewportSourceId, RuntimeRendererState>,
    viewport_errors: BTreeMap<RuntimeViewportSourceId, String>,
}

#[derive(Default)]
struct CanvasProjection;

struct SpatialProjection {
    resources: RuntimePuzzle3Resources,
}

#[cfg(test)]
thread_local! {
    static DEVELOPMENT_PROJECTION_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_development_projection() {
    DEVELOPMENT_PROJECTION_COUNT.with(|count| count.set(count.get() + 1));
}

impl RuntimeSession {
    pub fn from_source(source: &str, puzzle_path: &str) -> Result<Self, String> {
        let document = puzzle_lang::parse_game_for_path(source, puzzle_path)
            .map_err(|error| error.to_string())?;
        Self::from_document(document)
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

    #[cfg(any(feature = "editor-debug", test))]
    pub fn snapshot_json(&self) -> String {
        self.snapshot_json_with_events(&[])
    }

    pub fn snapshot(&self) -> RuntimeSessionSnapshot {
        self.snapshot_with_events(&[])
    }

    #[cfg(feature = "editor-debug")]
    pub fn development_snapshot(&self) -> RuntimeDevelopmentSessionSnapshot {
        self.development_snapshot_with_events(&[])
    }

    #[cfg(feature = "editor-debug")]
    pub fn development_snapshot_from_player(
        &self,
        player: RuntimeSessionSnapshot,
    ) -> RuntimeDevelopmentSessionSnapshot {
        self.compose_development_snapshot(player)
    }

    pub fn audio_catalog(&self) -> Arc<AudioAssetCatalog> {
        self.model.audio_catalog()
    }

    fn response_snapshot(&mut self) -> RuntimeSessionSnapshot {
        let events = self.model.take_presentation_events();
        self.snapshot_with_events(&events)
    }

    #[cfg(any(feature = "editor-debug", test))]
    fn snapshot_json_with_events(&self, events: &[RuntimePresentationEvent]) -> String {
        puzzle_presentation_json::to_string(&self.development_snapshot_with_events(events))
            .expect("snapshot JSON should serialize")
    }

    fn snapshot_with_events(&self, events: &[RuntimePresentationEvent]) -> RuntimeSessionSnapshot {
        self.model
            .player_snapshot(self.revision, self.scene_condition_context(), events)
    }

    #[cfg(any(feature = "editor-debug", test))]
    fn development_snapshot_with_events(
        &self,
        events: &[RuntimePresentationEvent],
    ) -> RuntimeDevelopmentSessionSnapshot {
        let player = self.snapshot_with_events(events);
        self.compose_development_snapshot(player)
    }

    #[cfg(any(feature = "editor-debug", test))]
    fn compose_development_snapshot(
        &self,
        player: RuntimeSessionSnapshot,
    ) -> RuntimeDevelopmentSessionSnapshot {
        let development = self.model.development_projection(&player);
        RuntimeDevelopmentSessionSnapshot::from_parts(player, development)
    }

    fn scene_condition_context(&self) -> SceneConditionContext {
        SceneConditionContext {
            has_progress_save: self.has_persisted_progress,
        }
    }

    #[cfg(any(feature = "editor-debug", test))]
    pub fn dispatch(&mut self, action: SessionAction) -> Result<String, String> {
        let snapshot = self.dispatch_typed(action)?;
        let development = self.compose_development_snapshot(snapshot);
        puzzle_presentation_json::to_string(&development)
            .map_err(|error| format!("snapshot JSON could not be serialized: {error}"))
    }

    pub fn dispatch_typed(
        &mut self,
        action: SessionAction,
    ) -> Result<RuntimeSessionSnapshot, String> {
        if let SessionAction::Key { trigger } = action {
            if trigger == RuntimeKeyTrigger::AnyInput {
                return Err(
                    "`any_input` is a binding wildcard and cannot be dispatched as a key"
                        .to_string(),
                );
            }
            let snapshot = self.snapshot_with_events(&[]);
            let inputs = self.model.input_bindings();
            let Some(action) = session_action_for_key(&snapshot, &inputs, trigger) else {
                return Ok(snapshot);
            };
            return self.dispatch_typed(action);
        }

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
                    | SessionAction::PresentationComplete { .. }
                    | SessionAction::SceneAction {
                        token: RuntimeSceneActionToken {
                            action: RuntimeSceneActionId::Event { .. },
                            ..
                        },
                    }
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

        let should_persist = !matches!(action, SessionAction::ChoiceMove { .. });
        match action {
            SessionAction::Initialize | SessionAction::Snapshot | SessionAction::Key { .. } => {
                unreachable!("snapshot request returned above")
            }
            SessionAction::PresentationComplete { token } => {
                if token.revision != self.revision {
                    return Err(format!(
                        "presentation continuation revision {} is stale; current revision is {}",
                        token.revision, self.revision
                    ));
                }
                if token.waits.is_empty() {
                    return Err(
                        "presentation continuation must identify at least one timeline wait"
                            .to_string(),
                    );
                }
                self.model.validate_presentation_waits(&token.waits)?;
                self.model.complete_presentation_waits(&token.waits)?;
                if !self.model.is_waiting()
                    && let Some(input) = self.queued_input.take()
                {
                    self.model.apply_input_name(&input)?;
                }
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
                let condition_context = self.scene_condition_context();
                self.model
                    .validate_choice_move(direction, condition_context)?;
                self.model.apply_choice_move(direction, condition_context)?;
            }
            SessionAction::SceneAction { token } => {
                let condition_context = self.scene_condition_context();
                self.model
                    .validate_scene_action(&token, condition_context)?;
                self.model.apply_scene_action(&token, condition_context)?;
            }
        }

        self.revision = self.next_revision()?;
        if self.model.take_progress_clear_request() {
            self.request_progress_persistence(RuntimeProgressPersistenceOperation::Delete)?;
        } else if should_persist {
            self.refresh_progress_save_request()?;
        }
        Ok(self.response_snapshot())
    }

    #[cfg(feature = "editor-debug")]
    pub fn dispatch_development_typed(
        &mut self,
        action: SessionAction,
    ) -> Result<RuntimeDevelopmentSessionSnapshot, String> {
        let player = self.dispatch_typed(action)?;
        Ok(self.compose_development_snapshot(player))
    }

    #[cfg(any(feature = "editor-debug", test))]
    pub fn dispatch_json(&mut self, action_json: &str) -> Result<String, String> {
        let action: SessionAction = serde_json::from_str(action_json)
            .map_err(|error| format!("invalid session action: {error}"))?;
        self.dispatch(action)
    }

    #[cfg(feature = "editor-debug")]
    pub fn apply_debug_input_name_json(&mut self, input_name: &str) -> Result<String, String> {
        let dispatch = self.apply_debug_input_name(input_name)?;
        let development = self.compose_development_snapshot(dispatch.snapshot);
        Ok(json!({
            "snapshot": puzzle_presentation_json::to_value(&development)
                .expect("debug snapshot JSON should serialize"),
            "debug": dispatch.debug,
        })
        .to_string())
    }

    #[cfg(feature = "editor-debug")]
    pub fn apply_debug_input_name(
        &mut self,
        input_name: &str,
    ) -> Result<RuntimeDebugDispatch, String> {
        let condition_context = self.scene_condition_context();
        self.model
            .apply_debug_input_name(input_name, condition_context)
    }

    #[cfg(feature = "editor-debug")]
    pub fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String> {
        self.model
            .set_current_state_json(state_json, level_index, materialize_level_start)
    }

    pub fn progress_save_json(&self) -> String {
        serde_json::to_string(&self.model.progress_save_data())
            .expect("progress save JSON should serialize")
    }

    pub fn progress_state_fingerprint(&self) -> u64 {
        progress_save_fingerprint(&self.model.progress_save_data())
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

    pub fn confirm_progress_persistence_applied(&mut self, request_id: u32) -> Result<(), String> {
        let Some(request) = &self.pending_progress_save else {
            return Err("progress save acknowledgement has no pending request".to_string());
        };
        if request.request_id != request_id {
            return Err(format!(
                "progress save acknowledgement {request_id} is stale; pending request is {}",
                request.request_id
            ));
        }
        self.has_persisted_progress = matches!(
            request.operation,
            RuntimeProgressPersistenceOperation::Write { .. }
        );
        self.pending_progress_save = None;
        Ok(())
    }

    #[cfg(feature = "solver-session")]
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
        self.request_progress_persistence(RuntimeProgressPersistenceOperation::Write {
            save_json: serde_json::to_string(&save)
                .expect("progress save request JSON should serialize"),
        })
    }

    fn request_progress_persistence(
        &mut self,
        operation: RuntimeProgressPersistenceOperation,
    ) -> Result<(), String> {
        let request_id = self.next_progress_request_id.max(1);
        self.next_progress_request_id = request_id
            .checked_add(1)
            .ok_or_else(|| "progress save request counter exhausted".to_string())?;
        self.pending_progress_save = Some(RuntimeProgressSaveRequest {
            request_id,
            operation,
        });
        Ok(())
    }
}

fn progress_save_fingerprint(save: &ProgressSaveData) -> u64 {
    fn push_bytes(hash: &mut u64, bytes: &[u8]) {
        for byte in bytes {
            *hash = (*hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
        }
    }
    fn push_string(hash: &mut u64, value: &str) {
        push_bytes(hash, &(value.len() as u64).to_le_bytes());
        push_bytes(hash, value.as_bytes());
    }

    let mut hash = 0xcbf29ce484222325;
    push_bytes(&mut hash, &save.version.to_le_bytes());

    let mut levels = save.levels.iter().collect::<Vec<_>>();
    levels.sort_by(|left, right| left.id.cmp(&right.id));
    push_bytes(&mut hash, &(levels.len() as u64).to_le_bytes());
    for level in levels {
        push_string(&mut hash, &level.id);
        push_bytes(&mut hash, &[u8::from(level.cleared)]);
    }

    match &save.current_level {
        Some(level) => {
            push_bytes(&mut hash, &[1]);
            push_string(&mut hash, level);
        }
        None => push_bytes(&mut hash, &[0]),
    }

    let mut persistent_vars = save.persistent_vars.iter().collect::<Vec<_>>();
    persistent_vars.sort_by(|left, right| left.name.cmp(&right.name));
    push_bytes(&mut hash, &(persistent_vars.len() as u64).to_le_bytes());
    for variable in persistent_vars {
        push_string(&mut hash, &variable.name);
        push_bytes(&mut hash, &variable.value.to_le_bytes());
    }
    hash
}

fn standalone_session_model(
    mut document: LoadedDocument,
) -> Result<Box<dyn StandaloneSessionModel>, String> {
    if document.models.is_empty() {
        return Err("a document session requires at least one puzzle model".to_string());
    }
    let document_root = (document.models.len() > 1)
        .then(|| {
            let model_names = document
                .models
                .iter()
                .map(|model| match model {
                    LoadedDocumentModel::Puzzle2d { name, .. }
                    | LoadedDocumentModel::Puzzle3d { name, .. } => name.as_str(),
                })
                .collect::<BTreeSet<_>>();
            document
                .scenes
                .iter()
                .find(|scene| !model_names.contains(scene.name.as_str()))
                .or_else(|| document.scenes.first())
                .map(|scene| scene.name.clone())
        })
        .flatten();
    let viewport_models = document_viewport_model_owners(&document);
    let mut models = Vec::<Box<dyn StandaloneSessionModel>>::with_capacity(document.models.len());
    for (index, model) in document.models.drain(..).enumerate() {
        models.push(grid_session_model(
            model,
            document_root.as_deref(),
            &viewport_models,
            index == 0,
        )?);
    }
    if models.len() == 1 {
        return Ok(models.pop().expect("single document model was checked"));
    }
    Ok(Box::new(DocumentSessionRuntime::try_new(models)?))
}

fn grid_session_model(
    model: LoadedDocumentModel,
    document_root: Option<&str>,
    viewport_models: &BTreeMap<(String, String), String>,
    owns_scene: bool,
) -> Result<Box<dyn StandaloneSessionModel>, String> {
    match model {
        LoadedDocumentModel::Puzzle2d {
            name,
            game: mut loaded,
        } => {
            loaded.validate_program_references()?;
            retain_model_worlds(&mut loaded, &name);
            prioritize_document_root(&mut loaded, document_root);
            let theme = runtime_theme(&loaded.theme)?;
            let audio_catalog = Arc::new(compile_audio_catalog(&loaded.sounds)?);
            let session = if owns_scene {
                GridGameSession::try_new(&loaded)
            } else {
                GridGameSession::try_new_scene_replica(&loaded)
            }
            .map_err(|error| format!("failed to start initial level: {error:?}"))?;
            Ok(Box::new(GridSessionRuntime {
                model_name: name,
                viewport_models: viewport_models.clone(),
                session,
                loaded,
                projection: CanvasProjection,
                theme,
                audio_catalog,
            }))
        }
        LoadedDocumentModel::Puzzle3d {
            name,
            game: mut loaded,
            presentation,
        } => {
            loaded.validate_program_references()?;
            retain_model_worlds(&mut loaded, &name);
            prioritize_document_root(&mut loaded, document_root);
            let theme = runtime_theme(&loaded.theme)?;
            let audio_catalog = Arc::new(compile_audio_catalog(&loaded.sounds)?);
            let resources = puzzle_lang::runtime_puzzle3_resources(&loaded, &presentation)
                .map_err(|error| {
                    format!("failed to materialize Puzzle3 runtime view: {error:?}")
                })?;
            let session = if owns_scene {
                GridGameSession::try_new(&loaded)
            } else {
                GridGameSession::try_new_scene_replica(&loaded)
            }
            .map_err(|error| format!("failed to start initial level: {error:?}"))?;
            Ok(Box::new(GridSessionRuntime {
                model_name: name,
                viewport_models: viewport_models.clone(),
                session,
                loaded,
                projection: SpatialProjection { resources },
                theme,
                audio_catalog,
            }))
        }
    }
}

fn document_viewport_model_owners(document: &LoadedDocument) -> BTreeMap<(String, String), String> {
    fn viewport_sources(components: &[SceneComponent], sources: &mut BTreeSet<String>) {
        for component in components {
            match component {
                SceneComponent::Viewport(viewport) => {
                    sources.insert(viewport.source.clone());
                }
                SceneComponent::Row(container)
                | SceneComponent::Column(container)
                | SceneComponent::Box(container) => viewport_sources(&container.children, sources),
                SceneComponent::Conditional(conditional) => {
                    viewport_sources(&conditional.children, sources);
                    viewport_sources(&conditional.else_children, sources);
                }
                SceneComponent::Frame(_)
                | SceneComponent::Text(_)
                | SceneComponent::Button(_)
                | SceneComponent::Choice(_) => {}
            }
        }
    }

    let model_names = document
        .models
        .iter()
        .map(|model| match model {
            LoadedDocumentModel::Puzzle2d { name, .. }
            | LoadedDocumentModel::Puzzle3d { name, .. } => name.clone(),
        })
        .collect::<BTreeSet<_>>();
    document
        .scenes
        .iter()
        .flat_map(|scene| {
            let mut owners = scene
                .state
                .puzzles
                .iter()
                .map(|puzzle| (puzzle.name.clone(), puzzle.model.clone()))
                .collect::<BTreeMap<_, _>>();
            let mut sources = BTreeSet::new();
            viewport_sources(&scene.components, &mut sources);
            for source in sources {
                if model_names.contains(&source) {
                    owners.entry(source.clone()).or_insert(source);
                }
            }
            owners
                .into_iter()
                .map(|(source, model)| ((scene.name.clone(), source), model))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn prioritize_document_root<const D: usize, Size: GridSize<D>>(
    loaded: &mut LoadedGridGame<D, Size>,
    document_root: Option<&str>,
) {
    let Some(index) =
        document_root.and_then(|root| loaded.scenes.iter().position(|scene| scene.name == root))
    else {
        return;
    };
    loaded.scenes[..=index].rotate_right(1);
}

fn retain_model_worlds<const D: usize, Size: GridSize<D>>(
    loaded: &mut LoadedGridGame<D, Size>,
    model_name: &str,
) {
    for scene in &mut loaded.scenes {
        scene
            .state
            .puzzles
            .retain(|puzzle| puzzle.model == model_name);
    }
}

impl DocumentSessionRuntime {
    fn try_new(models: Vec<Box<dyn StandaloneSessionModel>>) -> Result<Self, String> {
        assert!(
            !models.is_empty(),
            "document session requires at least one model"
        );
        let mut runtime = Self {
            models,
            scene_owner: 0,
        };
        let shared = runtime.scene_owner().shared_scene_runtime_state();
        for (index, model) in runtime.models.iter_mut().enumerate() {
            if index != runtime.scene_owner {
                model.synchronize_shared_scene_state(&shared)?;
            }
        }
        Ok(runtime)
    }

    fn scene_owner(&self) -> &dyn StandaloneSessionModel {
        self.models[self.scene_owner].as_ref()
    }

    fn scene_owner_mut(&mut self) -> &mut dyn StandaloneSessionModel {
        self.models[self.scene_owner].as_mut()
    }

    fn input_model_index(&self, input_name: &str) -> Result<usize, String> {
        let candidates = self
            .models
            .iter()
            .enumerate()
            .filter(|(_, model)| model.accepts_model_input() && model.has_input_name(input_name))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        match candidates.as_slice() {
            [index] => Ok(*index),
            [] if self
                .models
                .iter()
                .any(|model| model.has_input_name(input_name)) =>
            {
                Err(format!(
                    "input `{input_name}` has no focused puzzle model in the current scene"
                ))
            }
            [] => Err(format!("unknown input: {input_name}")),
            _ => Err(format!(
                "input `{input_name}` is ambiguous across {} focused puzzle models; the scene must route model input to one puzzle",
                candidates.len()
            )),
        }
    }

    fn primary_snapshot_index(&self) -> usize {
        self.scene_owner
    }
}

impl StandaloneSessionModel for DocumentSessionRuntime {
    fn player_snapshot(
        &self,
        revision: u64,
        condition_context: SceneConditionContext,
        presentation_events: &[RuntimePresentationEvent],
    ) -> RuntimeSessionSnapshot {
        let primary = self.primary_snapshot_index();
        let mut snapshots = self
            .models
            .iter()
            .map(|model| model.player_snapshot(revision, condition_context, presentation_events))
            .collect::<Vec<_>>();
        let mut snapshot = snapshots.remove(primary);
        let waits = self.presentation_waits();
        snapshot.presentation_continuation =
            (!waits.is_empty()).then_some(RuntimePresentationContinuationToken { revision, waits });
        for model_snapshot in snapshots {
            snapshot
                .viewport_sources
                .extend(model_snapshot.viewport_sources);
            snapshot.busy |= model_snapshot.busy;
            snapshot.can_undo |= model_snapshot.can_undo;
            snapshot.can_redo |= model_snapshot.can_redo;
        }
        snapshot.level_count = self.level_count();
        let focused_model_count = self
            .models
            .iter()
            .filter(|model| model.accepts_model_input())
            .count();
        snapshot.accepts_model_input = focused_model_count == 1;
        if focused_model_count != 1 {
            snapshot.level_index = None;
        }
        snapshot
    }

    #[cfg(any(feature = "editor-debug", test))]
    fn development_projection(
        &self,
        player: &RuntimeSessionSnapshot,
    ) -> RuntimeDevelopmentProjection {
        #[cfg(test)]
        record_development_projection();
        let primary = self.primary_snapshot_index();
        let mut projections = self
            .models
            .iter()
            .map(|model| model.development_projection(player))
            .collect::<Vec<_>>();
        let mut projection = projections.remove(primary);
        for model_projection in projections {
            projection.levels.extend(model_projection.levels);
            projection
                .viewport_sources
                .extend(model_projection.viewport_sources);
        }
        projection.inputs = self.input_bindings();
        projection
    }

    fn input_bindings(&self) -> Vec<RuntimeInputBinding> {
        let mut inputs = Vec::<RuntimeInputBinding>::new();
        for input in self.models.iter().flat_map(|model| model.input_bindings()) {
            if let Some(existing) = inputs
                .iter_mut()
                .find(|existing| existing.name == input.name)
            {
                for trigger in input.triggers {
                    if !existing.triggers.contains(&trigger) {
                        existing.triggers.push(trigger);
                    }
                }
            } else {
                inputs.push(input);
            }
        }
        inputs
    }

    fn level_count(&self) -> usize {
        self.models.iter().map(|model| model.level_count()).sum()
    }

    fn take_presentation_events(&mut self) -> Vec<RuntimePresentationEvent> {
        self.models
            .iter_mut()
            .flat_map(|model| model.take_presentation_events())
            .collect()
    }

    fn audio_catalog(&self) -> Arc<AudioAssetCatalog> {
        self.models
            .first()
            .expect("document session has at least one model")
            .audio_catalog()
    }

    fn is_waiting(&self) -> bool {
        self.models.iter().any(|model| model.is_waiting())
    }

    fn accepts_model_input(&self) -> bool {
        self.models
            .iter()
            .filter(|model| model.accepts_model_input())
            .count()
            == 1
    }

    fn queues_input_while_waiting(&self) -> bool {
        self.models
            .iter()
            .filter(|model| model.is_waiting())
            .all(|model| model.queues_input_while_waiting())
    }

    fn has_input_name(&self, input_name: &str) -> bool {
        self.models
            .iter()
            .any(|model| model.has_input_name(input_name))
    }

    fn presentation_waits(&self) -> Vec<RuntimeModelPresentationContinuationToken> {
        self.models
            .iter()
            .flat_map(|model| model.presentation_waits())
            .collect()
    }

    fn validate_presentation_waits(
        &self,
        waits: &[RuntimeModelPresentationContinuationToken],
    ) -> Result<(), String> {
        let expected = self.presentation_waits();
        if waits != expected {
            return Err(format!(
                "presentation continuation does not match the pending document timeline waits; expected {expected:?}, received {waits:?}"
            ));
        }
        for model in &self.models {
            let model_waits = waits
                .iter()
                .filter(|wait| {
                    model
                        .presentation_waits()
                        .iter()
                        .any(|expected| expected.model == wait.model)
                })
                .cloned()
                .collect::<Vec<_>>();
            model.validate_presentation_waits(&model_waits)?;
        }
        Ok(())
    }

    fn complete_presentation_waits(
        &mut self,
        waits: &[RuntimeModelPresentationContinuationToken],
    ) -> Result<(), String> {
        self.validate_presentation_waits(waits)?;
        for model in &mut self.models {
            let expected = model.presentation_waits();
            if !expected.is_empty() {
                model.complete_presentation_waits(&expected)?;
            }
        }
        Ok(())
    }

    fn apply_input_name(&mut self, input_name: &str) -> Result<(), String> {
        let index = self.input_model_index(input_name)?;
        self.models[index].apply_input_name(input_name)
    }

    fn apply_choice_move(
        &mut self,
        direction: RuntimeChoiceDirection,
        condition_context: SceneConditionContext,
    ) -> Result<(), String> {
        let shared = self
            .scene_owner()
            .preview_choice_move(direction, condition_context)?;
        for (index, model) in self.models.iter().enumerate() {
            if index != self.scene_owner {
                model.validate_shared_scene_state(&shared)?;
            }
        }
        self.scene_owner_mut()
            .apply_choice_move(direction, condition_context)?;
        for (index, model) in self.models.iter_mut().enumerate() {
            if index != self.scene_owner {
                model.synchronize_shared_scene_state(&shared)?;
            }
        }
        Ok(())
    }

    fn validate_choice_move(
        &self,
        direction: RuntimeChoiceDirection,
        condition_context: SceneConditionContext,
    ) -> Result<(), String> {
        self.scene_owner()
            .validate_choice_move(direction, condition_context)
    }

    fn preview_choice_move(
        &self,
        direction: RuntimeChoiceDirection,
        condition_context: SceneConditionContext,
    ) -> Result<SharedSceneRuntimeState, String> {
        self.scene_owner()
            .preview_choice_move(direction, condition_context)
    }

    fn apply_scene_action(
        &mut self,
        token: &RuntimeSceneActionToken,
        condition_context: SceneConditionContext,
    ) -> Result<(), String> {
        let shared = self
            .scene_owner()
            .preview_scene_action(token, condition_context)?;
        for (index, model) in self.models.iter().enumerate() {
            if index != self.scene_owner {
                model.validate_shared_scene_state(&shared)?;
            }
        }
        self.scene_owner_mut()
            .apply_scene_action(token, condition_context)?;
        for (index, model) in self.models.iter_mut().enumerate() {
            if index != self.scene_owner {
                model.synchronize_shared_scene_state(&shared)?;
            }
        }
        Ok(())
    }

    fn validate_scene_action(
        &self,
        token: &RuntimeSceneActionToken,
        condition_context: SceneConditionContext,
    ) -> Result<(), String> {
        self.scene_owner()
            .validate_scene_action(token, condition_context)
    }

    fn preview_scene_action(
        &self,
        token: &RuntimeSceneActionToken,
        condition_context: SceneConditionContext,
    ) -> Result<SharedSceneRuntimeState, String> {
        self.scene_owner()
            .preview_scene_action(token, condition_context)
    }

    fn shared_scene_runtime_state(&self) -> SharedSceneRuntimeState {
        self.scene_owner().shared_scene_runtime_state()
    }

    fn validate_shared_scene_state(&self, shared: &SharedSceneRuntimeState) -> Result<(), String> {
        for model in &self.models {
            model.validate_shared_scene_state(shared)?;
        }
        Ok(())
    }

    fn synchronize_shared_scene_state(
        &mut self,
        shared: &SharedSceneRuntimeState,
    ) -> Result<(), String> {
        self.validate_shared_scene_state(shared)?;
        for model in &mut self.models {
            model.synchronize_shared_scene_state(shared)?;
        }
        Ok(())
    }

    #[cfg(feature = "editor-debug")]
    fn apply_debug_input_name(
        &mut self,
        input_name: &str,
        condition_context: SceneConditionContext,
    ) -> Result<RuntimeDebugDispatch, String> {
        let index = self.input_model_index(input_name)?;
        self.models[index].apply_debug_input_name(input_name, condition_context)
    }

    fn undo(&mut self) {
        for model in &mut self.models {
            model.undo();
        }
    }

    fn redo(&mut self) {
        for model in &mut self.models {
            model.redo();
        }
    }

    fn restart(&mut self) -> Result<(), String> {
        for model in &mut self.models {
            model.restart()?;
        }
        Ok(())
    }

    fn next_level(&mut self) -> Result<(), String> {
        for model in &mut self.models {
            model.next_level()?;
        }
        Ok(())
    }

    fn previous_level(&mut self) -> Result<(), String> {
        for model in &mut self.models {
            model.previous_level()?;
        }
        Ok(())
    }

    fn goto_level(&mut self, level: usize) -> Result<(), String> {
        for model in &mut self.models {
            if level < model.level_count() {
                model.goto_level(level)?;
            }
        }
        Ok(())
    }

    #[cfg(feature = "editor-debug")]
    fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String> {
        let candidates = self
            .models
            .iter()
            .enumerate()
            .filter(|(_, model)| model.accepts_model_input())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let [index] = candidates.as_slice() else {
            return Err("setting raw model state requires one focused puzzle model".to_string());
        };
        self.models[*index].set_current_state_json(state_json, level_index, materialize_level_start)
    }

    fn progress_save_data(&self) -> ProgressSaveData {
        let mut saves = self.models.iter().map(|model| model.progress_save_data());
        let mut combined = saves
            .next()
            .expect("document session has at least one model");
        for save in saves {
            combined.levels.extend(save.levels);
            if combined.current_level.is_none() {
                combined.current_level = save.current_level;
            }
            for variable in save.persistent_vars {
                if !combined
                    .persistent_vars
                    .iter()
                    .any(|existing| existing.name == variable.name)
                {
                    combined.persistent_vars.push(variable);
                }
            }
        }
        combined
    }

    fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String> {
        for model in &mut self.models {
            model.restore_progress_save_json(save_json)?;
        }
        Ok(())
    }

    fn take_progress_clear_request(&mut self) -> bool {
        self.models.iter_mut().fold(false, |requested, model| {
            model.take_progress_clear_request() || requested
        })
    }

    #[cfg(feature = "solver-session")]
    fn solver_session_2d(&self) -> Option<(LoadedGame, GameSession)> {
        let mut sessions = self
            .models
            .iter()
            .filter_map(|model| model.solver_session_2d());
        let session = sessions.next()?;
        sessions.next().is_none().then_some(session)
    }
}

impl<const D: usize, Size, Projection> StandaloneSessionModel
    for GridSessionRuntime<D, Size, Projection>
where
    Size: GridSize<D> + 'static,
    Projection: GridSessionProjection<D, Size> + 'static,
{
    fn player_snapshot(
        &self,
        revision: u64,
        condition_context: SceneConditionContext,
        presentation_events: &[RuntimePresentationEvent],
    ) -> RuntimeSessionSnapshot {
        let busy = self.session.is_waiting();
        let mut projected =
            self.projection
                .player_viewport_sources(&self.loaded, &self.session, &self.theme);
        let mut surface = surface_snapshot(
            &self.loaded,
            &self.session,
            &self.viewport_models,
            condition_context,
        );
        let referenced_sources = referenced_viewport_sources(&surface);
        for (source, error) in &projected.viewport_errors {
            if !referenced_sources.contains(source) {
                continue;
            }
            if let Some(component) = surface
                .components
                .iter_mut()
                .find(|component| component.id == source.component)
            {
                component.presentation = RuntimeComponentPresentation::Error {
                    error: error.clone(),
                };
            }
        }
        projected
            .viewport_sources
            .retain(|source, _| referenced_sources.contains(source));
        RuntimeSessionSnapshot {
            revision,
            has_progress_save: condition_context.has_progress_save,
            theme: self.theme,
            default_wait_ms: self.loaded.default_wait_ms,
            input_buffer: input_buffer_settings(&self.loaded),
            animation: animation_settings(&self.loaded),
            presentation_events: presentation_events.to_vec(),
            presentation_continuation: self.session.presentation_continuation().map(
                |continuation| RuntimePresentationContinuationToken {
                    revision,
                    waits: vec![RuntimeModelPresentationContinuationToken {
                        model: self.model_name.clone(),
                        sequence: continuation.sequence,
                    }],
                },
            ),
            level_index: self.session.active_level_index(),
            level_count: self.loaded.levels.len(),
            accepts_model_input: self.session.accepts_model_input(&self.loaded),
            viewport_sources: projected.viewport_sources,
            surface,
            busy,
            can_undo: self.session.can_undo(),
            can_redo: self.session.can_redo(),
        }
    }

    #[cfg(any(feature = "editor-debug", test))]
    fn development_projection(
        &self,
        player: &RuntimeSessionSnapshot,
    ) -> RuntimeDevelopmentProjection {
        #[cfg(test)]
        record_development_projection();
        let referenced_sources = referenced_viewport_sources(&player.surface);
        let mut viewport_sources = self
            .projection
            .development_viewport_sources(&self.loaded, &self.session);
        viewport_sources.retain(|source, _| referenced_sources.contains(source));
        RuntimeDevelopmentProjection {
            levels: level_records(&self.loaded, &self.session),
            solver_state: self.projection.solver_state(self.session.state()),
            selected_level_index: self.session.selected_level_index(),
            inputs: self.input_bindings(),
            viewport_sources,
        }
    }

    fn input_bindings(&self) -> Vec<RuntimeInputBinding> {
        inputs(&self.loaded)
    }

    fn level_count(&self) -> usize {
        self.loaded.levels.len()
    }

    fn take_presentation_events(&mut self) -> Vec<RuntimePresentationEvent> {
        puzzle_presentation::resolve_presentation_events(
            presentation_events_contract::<D>(&self.session.take_presentation_events(), |event| {
                resolve_audio_command(&self.audio_catalog, event)
            })
            .expect("validated sound references must resolve through the audio catalog"),
            &runtime_visual_order(&self.loaded.visuals.order),
        )
        .expect("validated presentation animation channels must resolve")
    }

    fn audio_catalog(&self) -> Arc<AudioAssetCatalog> {
        Arc::clone(&self.audio_catalog)
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

    fn presentation_waits(&self) -> Vec<RuntimeModelPresentationContinuationToken> {
        self.session
            .presentation_continuation()
            .map(|continuation| {
                vec![RuntimeModelPresentationContinuationToken {
                    model: self.model_name.clone(),
                    sequence: continuation.sequence,
                }]
            })
            .unwrap_or_default()
    }

    fn validate_presentation_waits(
        &self,
        waits: &[RuntimeModelPresentationContinuationToken],
    ) -> Result<(), String> {
        let expected = self.presentation_waits();
        if waits != expected {
            return Err(format!(
                "presentation continuation does not match model `{}` timeline wait; expected {expected:?}, received {waits:?}",
                self.model_name
            ));
        }
        let [wait] = waits else {
            return Ok(());
        };
        self.session
            .validate_presentation_continuation(PresentationContinuation {
                sequence: wait.sequence,
            })
            .map_err(|error| format!("{error:?}"))
    }

    fn complete_presentation_waits(
        &mut self,
        waits: &[RuntimeModelPresentationContinuationToken],
    ) -> Result<(), String> {
        self.validate_presentation_waits(waits)?;
        let [wait] = waits else {
            return Ok(());
        };
        self.session
            .complete_presentation(
                &self.loaded,
                PresentationContinuation {
                    sequence: wait.sequence,
                },
            )
            .map_err(|error| format!("{error:?}"))
    }

    fn apply_input_name(&mut self, input_name: &str) -> Result<(), String> {
        let input = input_id_by_name(&self.loaded, input_name)
            .ok_or_else(|| format!("unknown input: {input_name}"))?;
        self.session
            .apply_input(&self.loaded, input)
            .map_err(|error| format!("{error:?}"))
    }

    fn apply_choice_move(
        &mut self,
        direction: RuntimeChoiceDirection,
        condition_context: SceneConditionContext,
    ) -> Result<(), String> {
        self.session
            .apply_choice_move(&self.loaded, direction, condition_context)
            .map_err(|error| format!("{error:?}"))
    }

    fn validate_choice_move(
        &self,
        direction: RuntimeChoiceDirection,
        condition_context: SceneConditionContext,
    ) -> Result<(), String> {
        let mut session = self.session.clone();
        session
            .apply_choice_move(&self.loaded, direction, condition_context)
            .map_err(|error| format!("{error:?}"))
    }

    fn preview_choice_move(
        &self,
        direction: RuntimeChoiceDirection,
        condition_context: SceneConditionContext,
    ) -> Result<SharedSceneRuntimeState, String> {
        let mut session = self.session.clone();
        session
            .apply_choice_move(&self.loaded, direction, condition_context)
            .map_err(|error| format!("{error:?}"))?;
        Ok(session.shared_scene_runtime_state())
    }

    fn apply_scene_action(
        &mut self,
        token: &RuntimeSceneActionToken,
        condition_context: SceneConditionContext,
    ) -> Result<(), String> {
        self.session
            .apply_scene_action(&self.loaded, token, condition_context)
            .map_err(|error| format!("{error:?}"))
    }

    fn validate_scene_action(
        &self,
        token: &RuntimeSceneActionToken,
        condition_context: SceneConditionContext,
    ) -> Result<(), String> {
        let mut session = self.session.clone();
        session
            .apply_scene_action(&self.loaded, token, condition_context)
            .map_err(|error| format!("{error:?}"))
    }

    fn preview_scene_action(
        &self,
        token: &RuntimeSceneActionToken,
        condition_context: SceneConditionContext,
    ) -> Result<SharedSceneRuntimeState, String> {
        let mut session = self.session.clone();
        session
            .apply_scene_action(&self.loaded, token, condition_context)
            .map_err(|error| format!("{error:?}"))?;
        Ok(session.shared_scene_runtime_state())
    }

    fn shared_scene_runtime_state(&self) -> SharedSceneRuntimeState {
        self.session.shared_scene_runtime_state()
    }

    fn validate_shared_scene_state(&self, shared: &SharedSceneRuntimeState) -> Result<(), String> {
        let mut session = self.session.clone();
        session
            .synchronize_shared_scene_runtime_state(&self.loaded, shared)
            .map_err(|error| format!("{error:?}"))
    }

    fn synchronize_shared_scene_state(
        &mut self,
        shared: &SharedSceneRuntimeState,
    ) -> Result<(), String> {
        self.session
            .synchronize_shared_scene_runtime_state(&self.loaded, shared)
            .map_err(|error| format!("{error:?}"))
    }

    #[cfg(feature = "editor-debug")]
    fn apply_debug_input_name(
        &mut self,
        input_name: &str,
        condition_context: SceneConditionContext,
    ) -> Result<RuntimeDebugDispatch, String> {
        let input = input_id_by_name(&self.loaded, input_name)
            .ok_or_else(|| format!("unknown input: {input_name}"))?;
        self.session
            .apply_traced_input(&self.loaded, input)
            .map_err(|error| format!("{error:?}"))?;
        let debug = self.session.last_transition_trace().cloned();
        let presentation_events = self.take_presentation_events();
        Ok(RuntimeDebugDispatch {
            snapshot: self.player_snapshot(0, condition_context, &presentation_events),
            debug: debug_transition_value_grid(&self.loaded, debug.as_ref()),
        })
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

    #[cfg(feature = "editor-debug")]
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

    fn take_progress_clear_request(&mut self) -> bool {
        self.session.take_progress_clear_request()
    }

    #[cfg(feature = "solver-session")]
    fn solver_session_2d(&self) -> Option<(LoadedGame, GameSession)> {
        self.projection
            .solver_session_2d(&self.loaded, &self.session)
    }
}

impl GridSessionProjection<2, Size2> for CanvasProjection {
    #[cfg(feature = "editor-debug")]
    fn decode_state_json(
        &self,
        game: &GridCompiledGame<2>,
        state_json: &str,
    ) -> Result<GridState<2, Size2>, String> {
        serde_json::from_str::<RuntimeStateSnapshot2d>(state_json)
            .map_err(|error| error.to_string())?
            .into_state(game)
    }

    fn player_viewport_sources(
        &self,
        loaded: &LoadedGame,
        session: &GridGameSession<2, Size2>,
        theme: &RuntimeTheme,
    ) -> ProjectedPlayerGridSnapshot {
        viewport_sources_2d(loaded, session, theme)
    }

    #[cfg(any(feature = "editor-debug", test))]
    fn development_viewport_sources(
        &self,
        loaded: &LoadedGame,
        session: &GridGameSession<2, Size2>,
    ) -> BTreeMap<RuntimeViewportSourceId, RuntimeDevelopmentRendererState> {
        development_viewport_sources_2d(loaded, session)
    }

    #[cfg(any(feature = "editor-debug", test))]
    fn solver_state(&self, state: &GridState<2, Size2>) -> SolverStateSnapshot {
        SolverStateSnapshot::from_state2(state)
    }

    #[cfg(feature = "solver-session")]
    fn solver_session_2d(
        &self,
        loaded: &LoadedGame,
        session: &GameSession,
    ) -> Option<(LoadedGame, GameSession)> {
        Some((loaded.clone(), session.clone()))
    }
}

impl GridSessionProjection<3, Size3> for SpatialProjection {
    #[cfg(feature = "editor-debug")]
    fn decode_state_json(
        &self,
        game: &GridCompiledGame<3>,
        state_json: &str,
    ) -> Result<GridState<3, Size3>, String> {
        serde_json::from_str::<RuntimeStateSnapshot3d>(state_json)
            .map_err(|error| error.to_string())?
            .into_state(game)
    }

    fn player_viewport_sources(
        &self,
        loaded: &LoadedGridGame<3, Size3>,
        session: &GridGameSession<3, Size3>,
        theme: &RuntimeTheme,
    ) -> ProjectedPlayerGridSnapshot {
        viewport_sources_3d(loaded, session, &self.resources, theme)
    }

    #[cfg(any(feature = "editor-debug", test))]
    fn development_viewport_sources(
        &self,
        loaded: &LoadedGridGame<3, Size3>,
        session: &GridGameSession<3, Size3>,
    ) -> BTreeMap<RuntimeViewportSourceId, RuntimeDevelopmentRendererState> {
        development_viewport_sources_3d(loaded, session)
    }

    #[cfg(any(feature = "editor-debug", test))]
    fn solver_state(&self, state: &GridState<3, Size3>) -> SolverStateSnapshot {
        SolverStateSnapshot::from_state3(state)
    }
}

fn viewport_sources_3d(
    loaded: &LoadedGridGame<3, Size3>,
    session: &GridGameSession<3, Size3>,
    resources: &RuntimePuzzle3Resources,
    theme: &RuntimeTheme,
) -> ProjectedPlayerGridSnapshot {
    let mut sources = BTreeMap::new();
    let mut errors = BTreeMap::new();
    for component in session
        .surface_state()
        .components()
        .iter()
        .filter(|component| {
            component.visibility == puzzle_lang::ComponentVisibility::Visible
                && loaded
                    .scenes
                    .iter()
                    .any(|definition| definition.name == component.definition)
        })
    {
        let Some(state) = session.scene_state_for(&component.id) else {
            continue;
        };
        for (source, world) in &state.puzzles {
            let id = RuntimeViewportSourceId {
                model: world.model.clone(),
                component: component.id.clone(),
                source: source.clone(),
            };
            match spatial_world_snapshot(loaded, world, resources, theme) {
                Ok(snapshot) => {
                    sources.insert(id, RuntimeRendererState::ThreeD(snapshot));
                }
                Err(error) => {
                    errors.insert(id, error);
                }
            }
        }
    }
    ProjectedPlayerGridSnapshot {
        viewport_sources: sources,
        viewport_errors: errors,
    }
}

#[cfg(any(feature = "editor-debug", test))]
fn development_viewport_sources_3d(
    loaded: &LoadedGridGame<3, Size3>,
    session: &GridGameSession<3, Size3>,
) -> BTreeMap<RuntimeViewportSourceId, RuntimeDevelopmentRendererState> {
    let mut sources = BTreeMap::new();
    for component in session
        .surface_state()
        .components()
        .iter()
        .filter(|component| {
            component.visibility == puzzle_lang::ComponentVisibility::Visible
                && loaded
                    .scenes
                    .iter()
                    .any(|definition| definition.name == component.definition)
        })
    {
        let Some(state) = session.scene_state_for(&component.id) else {
            continue;
        };
        for (source, world) in &state.puzzles {
            sources.insert(
                RuntimeViewportSourceId {
                    model: world.model.clone(),
                    component: component.id.clone(),
                    source: source.clone(),
                },
                RuntimeDevelopmentRendererState::ThreeD,
            );
        }
    }
    sources
}

fn spatial_world_snapshot(
    loaded: &LoadedGridGame<3, Size3>,
    world: &puzzle_play::GridWorldInstanceState<3, Size3>,
    resources: &RuntimePuzzle3Resources,
    theme: &RuntimeTheme,
) -> Result<RuntimePuzzle3Snapshot, String> {
    let level_index = world.active_level_index.ok_or_else(|| {
        "presented spatial viewport has no active level; level selection must be established by the owning session before presentation".to_string()
    })?;
    let cells = puzzle_lang::runtime_puzzle3_cells(&world.state, resources)
        .expect("validated Puzzle3 visual order must resolve runtime cells");
    let size = puzzle_lang::runtime_puzzle3_size(world.state.size);
    let mut render_scene = resolved_render_scene_3d(resources, &cells)
        .expect("validated Puzzle3 visuals must resolve a typed render scene");
    render_scene.decorations = resolve_grid_decoration_3d(
        runtime_grid_mode(loaded.render.grid),
        [size.width, size.depth, size.height],
        &render_scene.cells,
        theme,
    )
    .into_iter()
    .collect();
    Ok(RuntimePuzzle3Snapshot {
        level_index,
        level_count: loaded.levels.len(),
        size,
        cells,
        completed: loaded.is_goal_complete(&world.state),
        has_next_level: level_index + 1 < loaded.levels.len(),
        has_previous_level: level_index > 0,
        render: resources.render.clone(),
        render_scene,
    })
}

#[cfg(all(test, feature = "editor-debug"))]
fn compiled_state_value(state: &PuzzleState) -> Value {
    serde_json::to_value(RuntimeStateSnapshot2d::from_state(state))
        .expect("runtime state snapshot serializes")
}

fn viewport_sources_2d(
    loaded: &LoadedGame,
    session: &GameSession,
    theme: &RuntimeTheme,
) -> ProjectedPlayerGridSnapshot {
    let mut sources = BTreeMap::new();
    for component in session
        .surface_state()
        .components()
        .iter()
        .filter(|component| {
            component.visibility == puzzle_lang::ComponentVisibility::Visible
                && loaded
                    .scenes
                    .iter()
                    .any(|definition| definition.name == component.definition)
        })
    {
        let Some(state) = session.scene_state_for(&component.id) else {
            continue;
        };
        for (source, puzzle) in &state.puzzles {
            let id = RuntimeViewportSourceId {
                model: puzzle.model.clone(),
                component: component.id.clone(),
                source: source.clone(),
            };
            let player = scene_player_snapshot_for_state(loaded, &puzzle.state, theme);
            sources.insert(id, RuntimeRendererState::TwoD(player));
        }
    }
    ProjectedPlayerGridSnapshot {
        viewport_sources: sources,
        viewport_errors: BTreeMap::new(),
    }
}

#[cfg(any(feature = "editor-debug", test))]
fn development_viewport_sources_2d(
    loaded: &LoadedGame,
    session: &GameSession,
) -> BTreeMap<RuntimeViewportSourceId, RuntimeDevelopmentRendererState> {
    let mut sources = BTreeMap::new();
    for component in session
        .surface_state()
        .components()
        .iter()
        .filter(|component| {
            component.visibility == puzzle_lang::ComponentVisibility::Visible
                && loaded
                    .scenes
                    .iter()
                    .any(|definition| definition.name == component.definition)
        })
    {
        let Some(state) = session.scene_state_for(&component.id) else {
            continue;
        };
        for (source, puzzle) in &state.puzzles {
            let level = puzzle
                .active_level_index
                .and_then(|index| loaded.levels.get(index));
            let id = RuntimeViewportSourceId {
                model: puzzle.model.clone(),
                component: component.id.clone(),
                source: source.clone(),
            };
            let development = scene_development_snapshot_for_state(
                loaded,
                &puzzle.state,
                level,
                scene_resources(loaded, &component.definition),
            );
            sources.insert(id, RuntimeDevelopmentRendererState::TwoD(development));
        }
    }
    sources
}

fn scene_player_snapshot_for_state(
    loaded: &LoadedGame,
    state: &PuzzleState,
    theme: &RuntimeTheme,
) -> RuntimePuzzle2Snapshot {
    scene_player_snapshot_for_materialized_state(loaded, state, None, theme)
}

fn scene_player_snapshot_for_materialized_state(
    loaded: &LoadedGame,
    state: &PuzzleState,
    display_error: Option<String>,
    theme: &RuntimeTheme,
) -> RuntimePuzzle2Snapshot {
    let cells = runtime_puzzle2_cells(loaded, state, display_error.is_none());
    let view = resolved_view_2d(loaded, state.width, state.height, &cells)
        .expect("validated 2D viewport settings must resolve");
    RuntimePuzzle2Snapshot {
        view,
        render_scene: resolved_render_scene_2d(loaded, &cells, view, theme)
            .expect("validated 2D visuals must resolve a typed render scene"),
        display_error,
    }
}

#[cfg(any(feature = "editor-debug", test))]
fn scene_development_snapshot_for_state(
    loaded: &LoadedGame,
    state: &PuzzleState,
    level: Option<&Level>,
    resources: Option<&puzzle_lang::SceneResources>,
) -> RuntimePuzzle2DevelopmentSnapshot {
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
    RuntimePuzzle2DevelopmentSnapshot {
        width: state.width,
        height: state.height,
        layer_count: state.layer_count,
        settings: puzzle_settings(loaded),
        animation: animation_settings(loaded),
        regions,
        resources: scene_resources_snapshot(resources),
        cells: runtime_puzzle2_cells(loaded, state, true),
    }
}

fn runtime_puzzle2_cells(
    loaded: &LoadedGame,
    state: &PuzzleState,
    include_cells: bool,
) -> Vec<RuntimePuzzle2Cell> {
    if !include_cells {
        return Vec::new();
    }
    let visual_order = runtime_visual_order(&loaded.visuals.order);
    let priority_count = u64::try_from(visual_order.priorities.len())
        .expect("validated visual priority count must fit u64");
    let mut cells = Vec::new();
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
                let render_priority =
                    u16::try_from(resolved.index).expect("validated visual priority must fit u16");
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
    cells
}

fn resolved_render_scene_2d(
    loaded: &LoadedGame,
    cells: &[RuntimePuzzle2Cell],
    view: puzzle_runtime_contract::RuntimeResolvedView2d,
    theme: &RuntimeTheme,
) -> Result<RuntimeResolvedRenderScene, puzzle_presentation::PresentationError> {
    let mut transforms = HashMap::new();
    let mut clips = Vec::new();
    for visual in &loaded.visuals.entries {
        let mut frames = match &visual.kind {
            VisualKind::Solid(color) => vec![resolve_pixel_frame(
                &["0".to_string()],
                &BTreeMap::from([("0".to_string(), color.clone())]),
            )?],
            VisualKind::Image { asset } => vec![RuntimeResolvedVisualFrame::RasterImage {
                asset: asset.id.clone(),
                sampling: match visual.sampling {
                    Some(LoadedVisualSampling::Smooth) => RuntimeResolvedSampling::Smooth,
                    Some(LoadedVisualSampling::Pixelated) | None => {
                        RuntimeResolvedSampling::Pixelated
                    }
                },
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
    let resolved_cells = cells
        .iter()
        .map(|cell| RuntimeResolvedRenderCell {
            position: [i32::from(cell.x), i32::from(cell.y), 0],
            render_order: cell.render_order,
            object_ids: cell.layers.iter().map(|layer| layer.object_id).collect(),
        })
        .collect::<Vec<_>>();
    let decorations = resolve_grid_decoration_2d(
        runtime_grid_mode(loaded.render.grid),
        view,
        &resolved_cells,
        theme,
    )
    .into_iter()
    .collect();
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
        cells: resolved_cells,
        decorations,
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
        decorations: Vec::new(),
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

fn surface_snapshot<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
    viewport_models: &BTreeMap<(String, String), String>,
    condition_context: SceneConditionContext,
) -> RuntimeSurface {
    let active_modal = session
        .surface_state()
        .active_modal_component()
        .map(|component| component.id.as_str());
    let components = session
        .surface_state()
        .components()
        .iter()
        .filter(|component| component.visibility == puzzle_lang::ComponentVisibility::Visible)
        .map(|component| {
            let presentation = match resolved_surface_component_definition(
                loaded,
                session,
                viewport_models,
                &component.id,
                &component.definition,
                &component.properties,
                condition_context,
                component.id == session.surface_state().focused_component(),
                active_modal == Some(component.id.as_str()),
            ) {
                Ok(scene) => RuntimeComponentPresentation::Ready(scene),
                Err(error) => RuntimeComponentPresentation::Error { error },
            };
            RuntimeSurfaceComponent {
                id: component.id.clone(),
                placement: component.placement,
                visibility: component.visibility,
                modal: component.modal,
                await_event: component.awaited_event.clone(),
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

fn referenced_viewport_sources(surface: &RuntimeSurface) -> BTreeSet<RuntimeViewportSourceId> {
    fn collect(
        components: &[RuntimeResolvedSceneComponent],
        sources: &mut BTreeSet<RuntimeViewportSourceId>,
    ) {
        for component in components {
            match component {
                RuntimeResolvedSceneComponent::Viewport { source, .. } => {
                    sources.insert(source.clone());
                }
                RuntimeResolvedSceneComponent::Row { children, .. }
                | RuntimeResolvedSceneComponent::Column { children, .. }
                | RuntimeResolvedSceneComponent::Box { children, .. } => {
                    collect(children, sources);
                }
                RuntimeResolvedSceneComponent::Frame { .. }
                | RuntimeResolvedSceneComponent::Text { .. }
                | RuntimeResolvedSceneComponent::Button { .. }
                | RuntimeResolvedSceneComponent::Choice { .. } => {}
            }
        }
    }

    let mut sources = BTreeSet::new();
    for component in &surface.components {
        if let RuntimeComponentPresentation::Ready(scene) = &component.presentation {
            collect(&scene.components, &mut sources);
        }
    }
    sources
}

fn resolved_surface_component_definition<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
    viewport_models: &BTreeMap<(String, String), String>,
    instance_id: &str,
    definition_name: &str,
    properties: &HashMap<String, SceneValue>,
    condition_context: SceneConditionContext,
    node_actions_enabled: bool,
    event_actions_enabled: bool,
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
            layout: SceneLayoutDef::default(),
            events: Some(BTreeMap::from([(
                STANDARD_MESSAGE_DISMISS_EVENT.to_string(),
                RuntimeResolvedEventBinding {
                    pointer: true,
                    keys: vec![RuntimeKeyTrigger::AnyInput],
                    action: event_actions_enabled.then(|| RuntimeSceneActionToken {
                        component: instance_id.to_string(),
                        action: RuntimeSceneActionId::Event {
                            name: STANDARD_MESSAGE_DISMISS_EVENT.to_string(),
                        },
                    }),
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
        viewport_models,
        instance_id,
        definition,
        properties,
        condition_context,
        &HashMap::new(),
        node_actions_enabled,
    )
}

fn resolved_scene_definition<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
    viewport_models: &BTreeMap<(String, String), String>,
    instance_id: &str,
    definition: &SceneDef,
    properties: &HashMap<String, SceneValue>,
    condition_context: SceneConditionContext,
    state_override: &HashMap<String, SceneValue>,
    node_actions_enabled: bool,
) -> Result<RuntimeResolvedScene, String> {
    let values = scene_presentation_values(
        session,
        instance_id,
        properties,
        condition_context,
        state_override,
    );
    let selected_choice = if session.surface_state().focused_component() == instance_id {
        session
            .focused_choice_cursor(loaded, condition_context)
            .map_err(|error| format!("{error:?}"))?
    } else {
        None
    };
    let condition_is_true = |condition: &SceneExpr| {
        session.resolve_scene_condition(loaded, condition, condition_context, &values)
    };
    let action_cells = component_action_cells(&definition.components, condition_is_true)?;
    let choice_cells = component_choice_cells(&definition.components, condition_is_true)?;
    let selected_choice_effect =
        selected_choice.and_then(|index| choice_cells.get(index).map(|cell| cell.effect));
    Ok(RuntimeResolvedScene {
        layout: definition.layout.clone(),
        components: resolved_scene_components(
            loaded,
            session,
            viewport_models,
            &definition.name,
            &definition.components,
            &values,
            instance_id,
            &action_cells,
            selected_choice_effect,
            node_actions_enabled,
        )?,
        keys: node_actions_enabled.then(|| {
            definition
                .key_bindings
                .iter()
                .enumerate()
                .map(|(ordinal, binding)| RuntimeKeyBinding {
                    keys: binding.keys.iter().map(runtime_key_trigger).collect(),
                    action: RuntimeSceneActionToken {
                        component: instance_id.to_string(),
                        action: RuntimeSceneActionId::Key {
                            ordinal: u32::try_from(ordinal)
                                .expect("scene key binding count must fit in u32"),
                        },
                    },
                })
                .collect()
        }),
        events: None,
    })
}

fn scene_presentation_values<const D: usize, Size: GridSize<D>>(
    session: &GridGameSession<D, Size>,
    instance_id: &str,
    properties: &HashMap<String, SceneValue>,
    condition_context: SceneConditionContext,
    state_override: &HashMap<String, SceneValue>,
) -> HashMap<String, SceneValue> {
    let mut values = HashMap::new();
    for (name, value) in session.session_values() {
        values.insert(name.clone(), value.clone());
        values.insert(format!("game.{name}"), value.clone());
        values.insert(format!("gameState.{name}"), value.clone());
    }
    condition_context.insert_values(&mut values);
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
    viewport_models: &BTreeMap<(String, String), String>,
    definition_name: &str,
    components: &[SceneComponent],
    values: &HashMap<String, SceneValue>,
    instance_id: &str,
    action_cells: &[ComponentActionCell<'_, SceneEffect>],
    selected_choice_effect: Option<&SceneEffect>,
    node_actions_enabled: bool,
) -> Result<Vec<RuntimeResolvedSceneComponent>, String> {
    let mut resolved = Vec::new();
    for component in components {
        resolved.extend(resolved_scene_component(
            loaded,
            session,
            viewport_models,
            definition_name,
            component,
            values,
            instance_id,
            action_cells,
            selected_choice_effect,
            node_actions_enabled,
        )?);
    }
    Ok(resolved)
}

fn resolved_scene_component<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
    viewport_models: &BTreeMap<(String, String), String>,
    definition_name: &str,
    component: &SceneComponent,
    values: &HashMap<String, SceneValue>,
    instance_id: &str,
    action_cells: &[ComponentActionCell<'_, SceneEffect>],
    selected_choice_effect: Option<&SceneEffect>,
    node_actions_enabled: bool,
) -> Result<Vec<RuntimeResolvedSceneComponent>, String> {
    let resolve = |expr: &SceneExpr| session.resolve_scene_expression(loaded, expr, values);
    let action_token = |effect: &SceneEffect| {
        action_cells
            .iter()
            .find(|cell| std::ptr::eq(cell.effect, effect))
            .map(|cell| RuntimeSceneActionToken {
                component: instance_id.to_string(),
                action: RuntimeSceneActionId::Node {
                    ordinal: cell.ordinal,
                },
            })
            .ok_or_else(|| "scene action is missing from the scene-owned registry".to_string())
    };
    Ok(match component {
        SceneComponent::Viewport(viewport) => vec![RuntimeResolvedSceneComponent::Viewport {
            dimension: match viewport.projection {
                ViewportProjectionDef::TwoD => RuntimeViewportDimension::TwoD,
                ViewportProjectionDef::ThreeD => RuntimeViewportDimension::ThreeD,
            },
            source: RuntimeViewportSourceId {
                model: viewport_models
                    .get(&(definition_name.to_string(), viewport.source.clone()))
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "viewport `{}` has no declared model owner in component definition `{definition_name}`",
                            viewport.source,
                        )
                    })?,
                component: instance_id.to_string(),
                source: viewport.source.clone(),
            },
            layout: viewport.layout.clone(),
        }],
        SceneComponent::Frame(frame) => vec![RuntimeResolvedSceneComponent::Frame {
            kind: frame.kind.clone(),
            source: frame.source.clone(),
            layout: frame.layout.clone(),
        }],
        SceneComponent::Text(text) => {
            let value = match &text.content {
                SceneTextContent::Literal(value) => value.clone(),
                SceneTextContent::Path(path) => {
                    scene_value_to_string(&resolve(&SceneExpr::Path(path.clone()))?)
                }
                SceneTextContent::Expr(expr) => scene_value_to_string(&resolve(expr)?),
            };
            vec![RuntimeResolvedSceneComponent::Text {
                role: text.role,
                value,
                text_align: text.text_align,
                layout: text.layout.clone(),
            }]
        }
        SceneComponent::Button(button) => {
            vec![RuntimeResolvedSceneComponent::Button {
                label: scene_value_to_string(&resolve(&button.label)?),
                action: node_actions_enabled
                    .then(|| action_token(&button.effect))
                    .transpose()?,
                layout: button.layout.clone(),
            }]
        }
        SceneComponent::Choice(choice) => {
            vec![RuntimeResolvedSceneComponent::Choice {
                label: scene_value_to_string(&resolve(&choice.label)?),
                action: node_actions_enabled
                    .then(|| action_token(&choice.effect))
                    .transpose()?,
                selected: selected_choice_effect
                    .is_some_and(|selected| std::ptr::eq(selected, &choice.effect)),
                layout: choice.layout.clone(),
            }]
        }
        SceneComponent::Row(container) => vec![RuntimeResolvedSceneComponent::Row {
            layout: container.layout.clone(),
            children: resolved_scene_components(
                loaded,
                session,
                viewport_models,
                definition_name,
                &container.children,
                values,
                instance_id,
                action_cells,
                selected_choice_effect,
                node_actions_enabled,
            )?,
        }],
        SceneComponent::Column(container) => vec![RuntimeResolvedSceneComponent::Column {
            layout: container.layout.clone(),
            children: resolved_scene_components(
                loaded,
                session,
                viewport_models,
                definition_name,
                &container.children,
                values,
                instance_id,
                action_cells,
                selected_choice_effect,
                node_actions_enabled,
            )?,
        }],
        SceneComponent::Box(container) => vec![RuntimeResolvedSceneComponent::Box {
            layout: container.layout.clone(),
            children: resolved_scene_components(
                loaded,
                session,
                viewport_models,
                definition_name,
                &container.children,
                values,
                instance_id,
                action_cells,
                selected_choice_effect,
                node_actions_enabled,
            )?,
        }],
        SceneComponent::Conditional(conditional) => {
            let condition = resolve(&conditional.condition)?;
            let SceneValue::Bool(condition) = condition else {
                return Err(format!(
                    "scene conditional resolved to a non-boolean value: {condition:?}"
                ));
            };
            resolved_scene_components(
                loaded,
                session,
                viewport_models,
                definition_name,
                if condition {
                    &conditional.children
                } else {
                    &conditional.else_children
                },
                values,
                instance_id,
                action_cells,
                selected_choice_effect,
                node_actions_enabled,
            )?
        }
    })
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

fn compile_audio_catalog(sounds: &puzzle_lang::SoundsDef) -> Result<AudioAssetCatalog, String> {
    AudioAssetCatalog::compile(
        sounds
            .sfx
            .iter()
            .map(|sound| {
                (
                    sound.name.clone(),
                    SfxRecipe {
                        seed: sound.seed.clone(),
                        type_target: sound.type_target.clone(),
                        volume: sound.volume,
                    },
                )
            })
            .collect(),
        sounds
            .music
            .iter()
            .map(|sound| {
                (
                    sound.name.clone(),
                    MusicRecipe {
                        seed: sound.seed.clone(),
                        height: sound.height,
                        bars: sound.bars,
                        bpm: sound.bpm,
                        volume: sound.volume,
                    },
                )
            })
            .collect(),
    )
    .map_err(|error| error.to_string())
}

fn resolve_audio_command(
    catalog: &AudioAssetCatalog,
    event: &SoundEvent,
) -> Result<AudioCommand, String> {
    let music_target = |name: &Option<String>| {
        name.as_deref()
            .map(|name| {
                catalog
                    .resolve_music(name)
                    .map(MusicTarget::Asset)
                    .ok_or_else(|| format!("unknown music asset `{name}`"))
            })
            .unwrap_or(Ok(MusicTarget::All))
    };
    match event {
        SoundEvent::PlaySfx { name } => catalog
            .resolve_sfx(name)
            .map(|asset| AudioCommand::PlaySfx { asset })
            .ok_or_else(|| format!("unknown SFX asset `{name}`")),
        SoundEvent::PlayMusic { name } => catalog
            .resolve_music(name)
            .map(|asset| AudioCommand::PlayMusic { asset })
            .ok_or_else(|| format!("unknown music asset `{name}`")),
        SoundEvent::PauseMusic { name } => Ok(AudioCommand::PauseMusic {
            target: music_target(name)?,
        }),
        SoundEvent::ResumeMusic { name } => Ok(AudioCommand::ResumeMusic {
            target: music_target(name)?,
        }),
        SoundEvent::StopMusic { name } => Ok(AudioCommand::StopMusic {
            target: music_target(name)?,
        }),
    }
}

fn runtime_theme(theme: &ThemeDef) -> Result<RuntimeTheme, String> {
    resolve_runtime_theme(
        theme.name.as_deref(),
        theme
            .variables
            .iter()
            .map(|variable| (variable.name.as_str(), variable.value.as_str())),
    )
    .map_err(|error| format!("theme could not be resolved: {error:?}"))
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

#[cfg(any(feature = "editor-debug", test))]
fn puzzle_settings(loaded: &LoadedGame) -> RuntimePuzzle2Settings {
    RuntimePuzzle2Settings {
        render: RuntimeRender2d {},
        input_buffer: input_buffer_settings(loaded),
        animation: animation_settings(loaded),
    }
}

fn runtime_grid_mode(mode: PuzzleGridMode) -> RuntimeGridMode {
    match mode {
        PuzzleGridMode::Hidden => RuntimeGridMode::Hidden,
        PuzzleGridMode::OccupiedCells => RuntimeGridMode::OccupiedCells,
        PuzzleGridMode::AllCells => RuntimeGridMode::AllCells,
    }
}

fn resolved_view_2d(
    loaded: &LoadedGame,
    width: u16,
    height: u16,
    cells: &[RuntimePuzzle2Cell],
) -> Result<puzzle_runtime_contract::RuntimeResolvedView2d, puzzle_presentation::PresentationError>
{
    let focus_objects = loaded
        .object_groups
        .get(&loaded.screen.viewport_focus)
        .cloned()
        .or_else(|| object_id_by_name(loaded, &loaded.screen.viewport_focus).map(|id| vec![id]))
        .unwrap_or_default();
    let focus = cells.iter().find_map(|cell| {
        cell.layers
            .iter()
            .any(|layer| {
                focus_objects
                    .iter()
                    .any(|object_id| object_id.0 == layer.object_id)
            })
            .then_some([i32::from(cell.x), i32::from(cell.y)])
    });
    let requested_size = match loaded.screen.viewport_size {
        ViewportSizeDef::Full => None,
        ViewportSizeDef::Size { width, height } => Some([width, height]),
    };
    let mode = match loaded.screen.viewport_mode {
        ViewportModeDef::Paged => ViewMode2d::Paged,
        ViewportModeDef::Centered => ViewMode2d::Centered,
    };
    resolve_view_2d([width, height], requested_size, mode, focus)
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
            triggers: input_triggers(loaded, *id),
        })
        .collect()
}

fn session_action_for_key(
    snapshot: &RuntimeSessionSnapshot,
    inputs: &[RuntimeInputBinding],
    trigger: RuntimeKeyTrigger,
) -> Option<SessionAction> {
    let active_modal = snapshot.surface.components.iter().rev().find(|component| {
        component.visibility == puzzle_scene::ComponentVisibility::Visible && component.modal
    });
    if let Some(modal) = active_modal {
        return match &modal.presentation {
            RuntimeComponentPresentation::Ready(scene) => {
                scene_event_action_for_key(scene, trigger)
            }
            RuntimeComponentPresentation::Error { .. } => None,
        };
    }

    let focused_scene = snapshot.surface.components.iter().find_map(|component| {
        if component.id != snapshot.surface.focus
            || component.visibility != puzzle_scene::ComponentVisibility::Visible
        {
            return None;
        }
        match &component.presentation {
            RuntimeComponentPresentation::Ready(scene) => Some(scene),
            RuntimeComponentPresentation::Error { .. } => None,
        }
    });
    if let Some(action) = focused_scene
        .and_then(|scene| scene_event_action_for_key(scene, trigger))
        .or_else(|| focused_scene.and_then(|scene| scene_key_action_for_key(scene, trigger)))
    {
        return Some(action);
    }

    if let Some(focused_scene) =
        focused_scene.filter(|scene| resolved_components_contain_choice(&scene.components))
    {
        if matches!(
            trigger,
            RuntimeKeyTrigger::Enter
                | RuntimeKeyTrigger::Space
                | RuntimeKeyTrigger::Character { value: 'x' | 'X' }
        ) && let Some(token) = selected_choice_action(&focused_scene.components).cloned()
        {
            return Some(SessionAction::SceneAction { token });
        }

        let semantic_direction = inputs.iter().find_map(|binding| {
            binding
                .triggers
                .iter()
                .any(|candidate| runtime_key_trigger_matches(*candidate, trigger))
                .then(|| runtime_choice_direction_for_input(&binding.name))
                .flatten()
        });
        let direction = semantic_direction.or(match trigger {
            RuntimeKeyTrigger::ArrowUp | RuntimeKeyTrigger::Character { value: 'w' | 'W' } => {
                Some(RuntimeChoiceDirection::Up)
            }
            RuntimeKeyTrigger::ArrowDown | RuntimeKeyTrigger::Character { value: 's' | 'S' } => {
                Some(RuntimeChoiceDirection::Down)
            }
            RuntimeKeyTrigger::ArrowLeft | RuntimeKeyTrigger::Character { value: 'a' | 'A' } => {
                Some(RuntimeChoiceDirection::Left)
            }
            RuntimeKeyTrigger::ArrowRight | RuntimeKeyTrigger::Character { value: 'd' | 'D' } => {
                Some(RuntimeChoiceDirection::Right)
            }
            _ => None,
        });
        if let Some(direction) = direction {
            return Some(SessionAction::ChoiceMove { direction });
        }
    }

    if snapshot.accepts_model_input || (snapshot.busy && snapshot.input_buffer.queue_during_wait) {
        if let Some(input) = inputs.iter().find(|binding| {
            binding
                .triggers
                .iter()
                .any(|candidate| runtime_key_trigger_matches(*candidate, trigger))
        }) {
            return Some(SessionAction::Input {
                name: input.name.clone(),
            });
        }
    }

    match trigger {
        RuntimeKeyTrigger::Character { value: 'z' | 'Z' } if snapshot.can_undo => {
            Some(SessionAction::Undo)
        }
        RuntimeKeyTrigger::Character { value: 'y' | 'Y' } if snapshot.can_redo => {
            Some(SessionAction::Redo)
        }
        _ => None,
    }
}

fn scene_event_action_for_key(
    scene: &RuntimeResolvedScene,
    trigger: RuntimeKeyTrigger,
) -> Option<SessionAction> {
    scene.events.as_ref()?.values().find_map(|binding| {
        binding
            .keys
            .iter()
            .any(|candidate| runtime_key_trigger_matches(*candidate, trigger))
            .then(|| binding.action.clone())
            .flatten()
            .map(|token| SessionAction::SceneAction { token })
    })
}

fn scene_key_action_for_key(
    scene: &RuntimeResolvedScene,
    trigger: RuntimeKeyTrigger,
) -> Option<SessionAction> {
    scene.keys.as_ref()?.iter().find_map(|binding| {
        binding
            .keys
            .iter()
            .any(|candidate| runtime_key_trigger_matches(*candidate, trigger))
            .then(|| SessionAction::SceneAction {
                token: binding.action.clone(),
            })
    })
}

fn runtime_key_trigger_matches(binding: RuntimeKeyTrigger, trigger: RuntimeKeyTrigger) -> bool {
    match (binding, trigger) {
        (RuntimeKeyTrigger::AnyInput, _) => true,
        (
            RuntimeKeyTrigger::Character { value: binding },
            RuntimeKeyTrigger::Character { value: trigger },
        ) => binding.eq_ignore_ascii_case(&trigger),
        (binding, trigger) => binding == trigger,
    }
}

fn runtime_choice_direction_for_input(name: &str) -> Option<RuntimeChoiceDirection> {
    match name {
        "up" => Some(RuntimeChoiceDirection::Up),
        "down" => Some(RuntimeChoiceDirection::Down),
        "left" => Some(RuntimeChoiceDirection::Left),
        "right" => Some(RuntimeChoiceDirection::Right),
        _ => None,
    }
}

fn resolved_components_contain_choice(components: &[RuntimeResolvedSceneComponent]) -> bool {
    components.iter().any(|component| match component {
        RuntimeResolvedSceneComponent::Choice { .. } => true,
        RuntimeResolvedSceneComponent::Row { children, .. }
        | RuntimeResolvedSceneComponent::Column { children, .. }
        | RuntimeResolvedSceneComponent::Box { children, .. } => {
            resolved_components_contain_choice(children)
        }
        _ => false,
    })
}

fn selected_choice_action(
    components: &[RuntimeResolvedSceneComponent],
) -> Option<&RuntimeSceneActionToken> {
    components.iter().find_map(|component| match component {
        RuntimeResolvedSceneComponent::Choice {
            selected: true,
            action: Some(action),
            ..
        } => Some(action),
        RuntimeResolvedSceneComponent::Row { children, .. }
        | RuntimeResolvedSceneComponent::Column { children, .. }
        | RuntimeResolvedSceneComponent::Box { children, .. } => selected_choice_action(children),
        _ => None,
    })
}

fn input_triggers<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    input: InputId,
) -> Vec<RuntimeKeyTrigger> {
    let mut triggers = Vec::new();
    for (key, id) in &loaded.controls.keys {
        if *id == input {
            triggers.push(RuntimeKeyTrigger::Character {
                value: char::from(*key),
            });
        }
    }
    for (arrow, id) in &loaded.controls.arrows {
        if *id == input {
            triggers.push(match arrow {
                ArrowKey::Up => RuntimeKeyTrigger::ArrowUp,
                ArrowKey::Down => RuntimeKeyTrigger::ArrowDown,
                ArrowKey::Left => RuntimeKeyTrigger::ArrowLeft,
                ArrowKey::Right => RuntimeKeyTrigger::ArrowRight,
            });
        }
    }
    for (name, id) in &loaded.controls.named {
        if *id == input {
            triggers.push(runtime_key_trigger(&KeyTrigger::Named(name.clone())));
        }
    }
    triggers.sort();
    triggers.dedup();
    triggers
}

fn runtime_key_trigger(key: &KeyTrigger) -> RuntimeKeyTrigger {
    match key {
        KeyTrigger::Char(value) => RuntimeKeyTrigger::Character { value: *value },
        KeyTrigger::Named(name) => match name.as_str() {
            "ArrowUp" | "arrow_up" => RuntimeKeyTrigger::ArrowUp,
            "ArrowDown" | "arrow_down" => RuntimeKeyTrigger::ArrowDown,
            "ArrowLeft" | "arrow_left" => RuntimeKeyTrigger::ArrowLeft,
            "ArrowRight" | "arrow_right" => RuntimeKeyTrigger::ArrowRight,
            "Enter" => RuntimeKeyTrigger::Enter,
            "Space" => RuntimeKeyTrigger::Space,
            "Escape" => RuntimeKeyTrigger::Escape,
            "Tab" => RuntimeKeyTrigger::Tab,
            "Backspace" => RuntimeKeyTrigger::Backspace,
            unsupported => {
                panic!("validated runtime key trigger must be supported: {unsupported}")
            }
        },
    }
}

#[cfg(any(feature = "editor-debug", test))]
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

#[cfg(any(feature = "editor-debug", test))]
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

#[cfg(any(feature = "editor-debug", test))]
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

#[cfg(any(feature = "editor-debug", test))]
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
    use serde_json::json;

    #[test]
    fn progress_fingerprint_is_order_independent_and_changes_with_owned_state() {
        let progress = ProgressSaveData {
            version: 2,
            levels: vec![
                puzzle_play::LevelProgressSaveData {
                    id: "board.two".to_string(),
                    cleared: true,
                },
                puzzle_play::LevelProgressSaveData {
                    id: "board.one".to_string(),
                    cleared: false,
                },
            ],
            current_level: Some("board.two".to_string()),
            persistent_vars: vec![
                puzzle_play::PersistentVarSaveData {
                    name: "moves".to_string(),
                    value: 1,
                },
                puzzle_play::PersistentVarSaveData {
                    name: "score".to_string(),
                    value: 7,
                },
            ],
        };
        let mut reordered = progress.clone();
        reordered.levels.reverse();
        reordered.persistent_vars.reverse();
        assert_eq!(
            progress_save_fingerprint(&progress),
            progress_save_fingerprint(&reordered)
        );

        let mut changed = progress.clone();
        changed.persistent_vars[0].value = 2;
        assert_ne!(
            progress_save_fingerprint(&progress),
            progress_save_fingerprint(&changed)
        );
    }

    fn presentation_complete_action(snapshot: &Value) -> SessionAction {
        let token = serde_json::from_value(snapshot["presentationContinuation"].clone())
            .expect("a presentation timeline must project its typed continuation");
        SessionAction::PresentationComplete { token }
    }

    fn cell_has_object(cell: &Value, object: &str) -> bool {
        cell["layers"]
            .as_array()
            .is_some_and(|layers| layers.iter().any(|layer| layer["object"] == object))
    }

    fn projected_selected_choice(snapshot: &Value) -> &str {
        fn selected_label(components: &[Value]) -> Option<&str> {
            components.iter().find_map(|component| {
                if component["kind"] == "choice" && component["selected"] == true {
                    return component["label"].as_str();
                }
                selected_label(component["children"].as_array().map_or(&[], Vec::as_slice)).or_else(
                    || {
                        selected_label(
                            component["elseChildren"]
                                .as_array()
                                .map_or(&[], Vec::as_slice),
                        )
                    },
                )
            })
        }

        snapshot["surface"]["components"]
            .as_array()
            .and_then(|components| {
                components.iter().find_map(|component| {
                    selected_label(
                        component["presentation"]["components"]
                            .as_array()
                            .map_or(&[], Vec::as_slice),
                    )
                })
            })
            .unwrap_or_else(|| {
                panic!(
                    "focused choice component should project one selected node: {}",
                    snapshot["surface"]
                )
            })
    }

    fn projected_selected_action(snapshot: &Value) -> RuntimeSceneActionToken {
        fn selected_action(components: &[Value]) -> Option<RuntimeSceneActionToken> {
            components.iter().find_map(|component| {
                if component["kind"] == "choice" && component["selected"] == true {
                    return serde_json::from_value(component["actionToken"].clone()).ok();
                }
                selected_action(component["children"].as_array().map_or(&[], Vec::as_slice))
                    .or_else(|| {
                        selected_action(
                            component["elseChildren"]
                                .as_array()
                                .map_or(&[], Vec::as_slice),
                        )
                    })
            })
        }

        snapshot["surface"]["components"]
            .as_array()
            .and_then(|components| {
                components.iter().find_map(|component| {
                    selected_action(
                        component["presentation"]["components"]
                            .as_array()
                            .map_or(&[], Vec::as_slice),
                    )
                })
            })
            .unwrap_or_else(|| {
                panic!(
                    "focused choice component should project one selected action: {}",
                    snapshot["surface"]
                )
            })
    }

    fn projected_action_for_label(snapshot: &Value, label: &str) -> RuntimeSceneActionToken {
        fn action_for_label(components: &[Value], label: &str) -> Option<RuntimeSceneActionToken> {
            components.iter().find_map(|component| {
                if component["label"] == label {
                    return serde_json::from_value(component["actionToken"].clone()).ok();
                }
                action_for_label(
                    component["children"].as_array().map_or(&[], Vec::as_slice),
                    label,
                )
            })
        }

        snapshot["surface"]["components"]
            .as_array()
            .and_then(|components| {
                components.iter().find_map(|component| {
                    action_for_label(
                        component["presentation"]["components"]
                            .as_array()
                            .map_or(&[], Vec::as_slice),
                        label,
                    )
                })
            })
            .unwrap_or_else(|| panic!("missing executable control `{label}`: {snapshot}"))
    }

    fn activate_selected_choice(runtime: &mut RuntimeSession) -> Value {
        let snapshot: Value = serde_json::from_str(&runtime.snapshot_json()).unwrap();
        let token = projected_selected_action(&snapshot);
        serde_json::from_str(
            &runtime
                .dispatch(SessionAction::SceneAction { token })
                .expect("selected resolved choice must be executable"),
        )
        .unwrap()
    }

    fn first_viewport_state(snapshot: &Value) -> &Value {
        snapshot["viewportSources"]
            .as_array()
            .and_then(|sources| sources.first())
            .map(|source| &source["state"])
            .unwrap_or_else(|| panic!("snapshot must contain a viewport source: {snapshot}"))
    }

    fn viewport_state<'a>(snapshot: &'a Value, component: &str, source: &str) -> &'a Value {
        snapshot["viewportSources"]
            .as_array()
            .and_then(|sources| {
                sources.iter().find(|entry| {
                    entry["id"]["component"] == component && entry["id"]["source"] == source
                })
            })
            .map(|entry| &entry["state"])
            .unwrap_or_else(|| panic!("missing viewport source {component}/{source}: {snapshot}"))
    }

    fn development_value(runtime: &RuntimeSession, player: &RuntimeSessionSnapshot) -> Value {
        let development = runtime.compose_development_snapshot(player.clone());
        puzzle_presentation_json::to_value(&development).unwrap()
    }

    fn runtime_scene_fixture_source() -> &'static str {
        r#"
const title = "Runtime Scene Fixture"

scene title {
layout {
heading title
choice "New Game" -> goto playing("microban.1")
if game.has_progress_save {
choice "Continue" -> goto playing
}
}
}

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
    fn player_snapshot_and_dispatch_do_not_invoke_development_projection() {
        DEVELOPMENT_PROJECTION_COUNT.with(|count| count.set(0));
        let mut runtime = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "player_projection_boundary.puzzle",
        )
        .unwrap();

        let direct = runtime.snapshot();
        let requested = runtime.dispatch_typed(SessionAction::Snapshot).unwrap();
        let initialized = runtime.dispatch_typed(SessionAction::Initialize).unwrap();

        assert_eq!(direct.surface.focus, "title");
        assert_eq!(requested.surface.focus, "title");
        assert_eq!(initialized.surface.focus, "title");
        DEVELOPMENT_PROJECTION_COUNT.with(|count| {
            assert_eq!(
                count.get(),
                0,
                "the official player path must not construct editor/debug projection data"
            );
        });

        let _development = runtime.development_snapshot_with_events(&[]);
        DEVELOPMENT_PROJECTION_COUNT.with(|count| {
            assert!(
                count.get() > 0,
                "the development path must explicitly request its extra projection"
            );
        });
    }

    #[test]
    fn native_backend_dispatch_returns_complete_typed_snapshot_without_json() {
        let mut runtime = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "typed_runtime_scene_fixture.puzzle",
        )
        .unwrap();

        let initialized = runtime.dispatch_typed(SessionAction::Initialize).unwrap();
        assert_eq!(initialized.surface.focus, "title");
        assert!(
            initialized.viewport_sources.is_empty(),
            "a title-only surface must not require a renderer viewport"
        );
        let initialized_json = development_value(&runtime, &initialized);
        let token = projected_selected_action(&initialized_json);
        let snapshot = runtime
            .dispatch_typed(SessionAction::SceneAction { token })
            .unwrap();

        assert_eq!(snapshot.revision, 1);
        assert_eq!(snapshot.level_count, 2);
        assert!(
            snapshot
                .viewport_sources
                .contains_key(&RuntimeViewportSourceId {
                    model: "board".to_string(),
                    component: "playing".to_string(),
                    source: "board".to_string(),
                })
        );
        assert!(!snapshot.surface.components.is_empty());
        assert!(
            snapshot
                .surface
                .components
                .iter()
                .any(|component| component.id == "playing")
        );
        assert!(
            development_value(&runtime, &snapshot)
                .get("scenes")
                .is_none(),
            "runtime snapshots must not publish authored scene definitions"
        );
        let RuntimeRendererState::TwoD(scene) = snapshot
            .viewport_sources
            .values()
            .next()
            .expect("playing viewport source must be projected")
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
        let json = development_value(&runtime, &snapshot);
        assert!(json["viewportSources"][0]["state"]["renderScene"].is_object());
    }

    #[test]
    fn runtime_projects_grid_mode_to_resolved_view_and_unique_line_plan() {
        let source = runtime_scene_fixture_source().replacen(
            "puzzle board {\n",
            "puzzle board {\nrender { grid { type = \"all_cells\" } }\n",
            1,
        );
        let mut runtime =
            RuntimeSession::from_source(&source, "resolved_grid_fixture.puzzle").unwrap();
        let initialized = runtime.dispatch_typed(SessionAction::Initialize).unwrap();
        let token = projected_selected_action(&development_value(&runtime, &initialized));
        let snapshot = runtime
            .dispatch_typed(SessionAction::SceneAction { token })
            .unwrap();
        let RuntimeRendererState::TwoD(scene) = snapshot
            .viewport_sources
            .get(&RuntimeViewportSourceId {
                model: "board".to_string(),
                component: "playing".to_string(),
                source: "board".to_string(),
            })
            .expect("playing board must publish its typed viewport")
        else {
            panic!("fixture must resolve a 2D viewport");
        };
        assert_eq!(
            scene.view,
            puzzle_runtime_contract::RuntimeResolvedView2d {
                origin: [0, 0],
                size: [7, 6],
            }
        );
        let [puzzle_runtime_contract::RuntimeResolvedDecoration::Lines2d { segments, .. }] =
            scene.render_scene.decorations.as_slice()
        else {
            panic!("all_cells must resolve to one 2D line plan");
        };
        assert_eq!(segments.len(), 97);

        let json = development_value(&runtime, &snapshot);
        let state = viewport_state(&json, "playing", "board");
        assert!(state.get("screen").is_none());
        assert!(state["settings"].get("grid").is_none());
        assert_eq!(state["renderScene"]["decorations"][0]["kind"], "lines2d");
    }

    #[test]
    fn snapshot_theme_is_resolved_and_contains_no_authoring_tokens() {
        let source = runtime_scene_fixture_source().replacen(
            "const title = \"Runtime Scene Fixture\"",
            "const title = \"Runtime Scene Fixture\"\ntheme {\npreset = \"clean\"\nbackground_color = #000\naccent_color = #ff000080\n}",
            1,
        );
        let runtime =
            RuntimeSession::from_source(&source, "typed_theme_runtime_fixture.puzzle").unwrap();
        let snapshot = runtime.snapshot();
        snapshot.theme.validate().unwrap();
        assert_eq!(
            snapshot.theme.background,
            puzzle_presentation::resolve_palette_color("#000").unwrap()
        );
        let json = development_value(&runtime, &snapshot);
        assert!(json["theme"]["background"]["red"].is_number());
        assert!(json["theme"]["typography"]["body"]["fontSizePx"].is_number());
        assert!(json["theme"].get("name").is_none());
        assert!(json["theme"].get("variables").is_none());
    }

    #[test]
    fn unresolved_theme_color_is_rejected_before_a_session_is_created() {
        let source = runtime_scene_fixture_source().replacen(
            "const title = \"Runtime Scene Fixture\"",
            "const title = \"Runtime Scene Fixture\"\ntheme {\npreset = \"clean\"\naccent_color = not-a-color\n}",
            1,
        );
        let error =
            match RuntimeSession::from_source(&source, "unresolved_theme_runtime_fixture.puzzle") {
                Ok(_) => panic!("unresolved theme color must not enter the runtime snapshot"),
                Err(error) => error,
            };
        assert!(error.contains("InvalidColor"));
        assert!(error.contains("not-a-color"));
    }

    #[test]
    fn typed_key_resolution_has_one_rust_owned_priority_order() {
        let runtime = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "typed_key_priority_fixture.puzzle",
        )
        .unwrap();
        let development = runtime.development_snapshot_with_events(&[]);
        let mut snapshot = development.player;
        let mut inputs = development.inputs;
        let selected = snapshot
            .surface
            .components
            .iter()
            .find_map(|component| match &component.presentation {
                RuntimeComponentPresentation::Ready(scene) => {
                    selected_choice_action(&scene.components).cloned()
                }
                RuntimeComponentPresentation::Error { .. } => None,
            })
            .expect("title fixture must expose a selected choice action");

        assert_eq!(
            session_action_for_key(&snapshot, &inputs, RuntimeKeyTrigger::Enter),
            Some(SessionAction::SceneAction {
                token: selected.clone(),
            }),
            "choice confirmation is resolved by Rust, not by a presentation adapter"
        );
        assert_eq!(
            session_action_for_key(&snapshot, &inputs, RuntimeKeyTrigger::ArrowDown),
            Some(SessionAction::ChoiceMove {
                direction: RuntimeChoiceDirection::Down,
            }),
            "choice movement is resolved before model input while choices own focus"
        );

        let focused_id = snapshot.surface.focus.clone();
        {
            let focused = snapshot
                .surface
                .components
                .iter_mut()
                .find(|component| component.id == focused_id)
                .expect("focused component must be projected");
            let RuntimeComponentPresentation::Ready(scene) = &mut focused.presentation else {
                panic!("focused title component must have a resolved presentation");
            };
            scene.keys = Some(vec![RuntimeKeyBinding {
                keys: vec![RuntimeKeyTrigger::Enter],
                action: RuntimeSceneActionToken {
                    component: focused_id.clone(),
                    action: RuntimeSceneActionId::Key { ordinal: 0 },
                },
            }]);
        }
        assert_eq!(
            session_action_for_key(&snapshot, &inputs, RuntimeKeyTrigger::Enter),
            Some(SessionAction::SceneAction {
                token: RuntimeSceneActionToken {
                    component: focused_id.clone(),
                    action: RuntimeSceneActionId::Key { ordinal: 0 },
                },
            }),
            "explicit focused-scene keys must shadow generic choice confirmation"
        );

        snapshot
            .surface
            .components
            .iter_mut()
            .find(|component| component.id == focused_id)
            .expect("focused component must remain projected")
            .presentation = RuntimeComponentPresentation::Error {
            error: "test removes focused scene controls".to_string(),
        };
        snapshot.accepts_model_input = true;
        inputs = vec![RuntimeInputBinding {
            id: 7,
            name: "special".to_string(),
            triggers: vec![RuntimeKeyTrigger::Character { value: 'z' }],
        }];
        snapshot.can_undo = true;
        assert_eq!(
            session_action_for_key(
                &snapshot,
                &inputs,
                RuntimeKeyTrigger::Character { value: 'Z' },
            ),
            Some(SessionAction::Input {
                name: "special".to_string(),
            }),
            "an explicit model control must shadow the default undo key"
        );

        inputs.clear();
        assert_eq!(
            session_action_for_key(
                &snapshot,
                &inputs,
                RuntimeKeyTrigger::Character { value: 'z' },
            ),
            Some(SessionAction::Undo)
        );
        snapshot.can_redo = true;
        assert_eq!(
            session_action_for_key(
                &snapshot,
                &inputs,
                RuntimeKeyTrigger::Character { value: 'y' },
            ),
            Some(SessionAction::Redo)
        );
    }

    #[test]
    fn pointer_only_modal_consumes_unmatched_keys_before_focused_scene_actions() {
        let runtime = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "typed_modal_key_priority_fixture.puzzle",
        )
        .unwrap();
        let mut snapshot = runtime.snapshot();
        let underlying_action = session_action_for_key(&snapshot, &[], RuntimeKeyTrigger::Enter)
            .expect("the focused title choice must handle Enter without a modal");
        let modal_action = RuntimeSceneActionToken {
            component: "pointer-modal".to_string(),
            action: RuntimeSceneActionId::Event {
                name: "dismiss".to_string(),
            },
        };
        snapshot.surface.components.push(RuntimeSurfaceComponent {
            id: "pointer-modal".to_string(),
            placement: puzzle_scene::ComponentPlacement::Overlay,
            visibility: puzzle_scene::ComponentVisibility::Visible,
            modal: true,
            await_event: Some("dismiss".to_string()),
            presentation: RuntimeComponentPresentation::Ready(RuntimeResolvedScene {
                layout: SceneLayoutDef::default(),
                components: Vec::new(),
                keys: None,
                events: Some(BTreeMap::from([(
                    "dismiss".to_string(),
                    RuntimeResolvedEventBinding {
                        pointer: true,
                        keys: Vec::new(),
                        action: Some(modal_action.clone()),
                    },
                )])),
            }),
        });

        assert!(
            matches!(underlying_action, SessionAction::SceneAction { .. }),
            "the blocked lower-priority action must be a focused-scene action"
        );
        assert_eq!(
            session_action_for_key(&snapshot, &[], RuntimeKeyTrigger::Enter),
            None,
            "an unmatched key must remain owned by the pointer-only modal"
        );

        let RuntimeComponentPresentation::Ready(modal_scene) = &mut snapshot
            .surface
            .components
            .last_mut()
            .expect("modal must remain mounted")
            .presentation
        else {
            panic!("modal presentation must be ready");
        };
        modal_scene
            .events
            .as_mut()
            .expect("modal must project its event")
            .get_mut("dismiss")
            .expect("modal must project its dismiss event")
            .keys
            .push(RuntimeKeyTrigger::Enter);
        assert_eq!(
            session_action_for_key(&snapshot, &[], RuntimeKeyTrigger::Enter),
            Some(SessionAction::SceneAction {
                token: modal_action,
            }),
            "a matching modal key must still route to the modal event"
        );
    }

    #[test]
    fn typed_key_dispatch_activates_the_selected_choice_without_an_adapter_token_lookup() {
        let mut runtime = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "typed_key_dispatch_fixture.puzzle",
        )
        .unwrap();

        let snapshot = runtime
            .dispatch_typed(SessionAction::Key {
                trigger: RuntimeKeyTrigger::Space,
            })
            .expect("Space must confirm the selected title choice");

        assert_eq!(snapshot.surface.focus, "playing");
        assert_eq!(snapshot.revision, 1);
    }

    #[test]
    fn any_input_is_a_binding_wildcard_not_a_dispatchable_physical_key() {
        let mut runtime = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "typed_key_wildcard_fixture.puzzle",
        )
        .unwrap();

        assert_eq!(
            runtime
                .dispatch_typed(SessionAction::Key {
                    trigger: RuntimeKeyTrigger::AnyInput,
                })
                .unwrap_err(),
            "`any_input` is a binding wildcard and cannot be dispatched as a key"
        );
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
        let RuntimeRendererState::TwoD(scene) = snapshot
            .viewport_sources
            .into_values()
            .next()
            .expect("implicit playing viewport must be projected")
        else {
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
        let mut bridge = RuntimeSession::from_source(source, "export_test.puzzle").unwrap();

        let waiting: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "right".to_string(),
                })
                .expect("input should complete with a presentation wait"),
        )
        .unwrap();
        let waiting_view = first_viewport_state(&waiting);
        assert!(!cell_has_object(&waiting_view["cells"][0], "A"));
        assert!(cell_has_object(&waiting_view["cells"][0], "C"));
        assert!(!cell_has_object(&waiting_view["cells"][0], "B"));
        assert_eq!(waiting["busy"], true);
        assert_eq!(waiting["presentationEvents"][0]["kind"], json!("wait"));
        assert_eq!(waiting["presentationEvents"][0]["milliseconds"], 100);
        assert_eq!(
            waiting["presentationContinuation"],
            json!({
                "revision": 1,
                "waits": [{"model": "default", "sequence": 1}]
            })
        );
        let inspected: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Snapshot).unwrap()).unwrap();
        assert_eq!(inspected["presentationEvents"], json!([]));
        assert_eq!(inspected["busy"], true);
        assert_eq!(
            bridge.dispatch(SessionAction::Undo).unwrap_err(),
            "session action is unavailable while a turn is waiting"
        );

        let completion = presentation_complete_action(&inspected);
        let resumed: Value = serde_json::from_str(
            &bridge
                .dispatch(completion.clone())
                .expect("wait completion should resume the same turn"),
        )
        .unwrap();
        let resumed_view = first_viewport_state(&resumed);
        assert!(!cell_has_object(&resumed_view["cells"][0], "C"));
        assert!(cell_has_object(&resumed_view["cells"][0], "B"));
        assert_eq!(resumed["busy"], false);
        assert!(bridge.dispatch(completion).unwrap_err().contains("stale"));

        let undone: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Undo)
                .expect("undo should return to pre-input state"),
        )
        .unwrap();
        let undone_view = first_viewport_state(&undone);
        assert!(cell_has_object(&undone_view["cells"][0], "A"));
        assert!(!cell_has_object(&undone_view["cells"][0], "B"));
        assert!(!cell_has_object(&undone_view["cells"][0], "C"));
    }

    #[test]
    fn awaited_component_modal_has_no_presentation_continuation() {
        let source = r#"
const title = modal_has_no_timeline_continuation
puzzle board {
input open
layers { actor = A }
rules {
if input == open {
[ A ] -> message "ready"
}
}
levels {
legend { A = A }
level "start" { A }
}
}
"#;
        let mut runtime = RuntimeSession::from_source(source, "modal_wait.puzzle").unwrap();

        let snapshot = runtime
            .dispatch_typed(SessionAction::Input {
                name: "open".to_string(),
            })
            .unwrap();

        assert!(snapshot.busy, "the modal must block further model input");
        assert!(
            snapshot.presentation_continuation.is_none(),
            "awaited component events are completed only through RuntimeSceneActionToken events"
        );
        assert!(snapshot.surface.components.iter().any(|component| {
            component.modal && component.await_event.as_deref() == Some("dismiss")
        }));
    }

    #[test]
    fn document_presentation_completion_preflights_all_waits_and_never_resumes_a_modal() {
        let source = r#"
const title = document_wait_atomicity
puzzle timed {
input tick
layers { actor = A }
rules {
if input == tick {
[ A ] -> wait 100ms
}
}
}
levels timed_levels of timed {
legend { A = A }
level "timed" { A }
}
puzzle dialog {
input open
layers { actor = B }
rules {
if input == open {
[ B ] -> message "ready"
}
}
}
levels dialog_levels of dialog {
legend { B = B }
level "dialog" { B }
}
scene playing {
layout {
row {
puzzle timed_view = timed
puzzle dialog_view = dialog
}
}
}
"#;
        let mut document = puzzle_lang::parse_game_for_path(source, "document_waits.puzzle")
            .expect("the two-model wait fixture must compile");
        let viewport_models = document_viewport_model_owners(&document);
        let mut models = document
            .models
            .drain(..)
            .map(|model| {
                grid_session_model(model, Some("playing"), &viewport_models, true).unwrap()
            })
            .collect::<Vec<_>>();
        models[0].apply_input_name("tick").unwrap();
        models[1].apply_input_name("open").unwrap();
        let mut runtime = DocumentSessionRuntime {
            models,
            scene_owner: 0,
        };
        let waits = runtime.presentation_waits();

        assert_eq!(waits.len(), 1);
        assert_eq!(waits[0].model, "timed");
        assert!(runtime.models[0].is_waiting());
        assert!(runtime.models[1].is_waiting());
        assert!(runtime.models[1].presentation_waits().is_empty());

        let mut wrong = waits.clone();
        wrong[0].sequence += 1;
        assert!(runtime.complete_presentation_waits(&wrong).is_err());
        assert_eq!(runtime.presentation_waits(), waits);
        assert!(runtime.models[0].is_waiting());
        assert!(runtime.models[1].is_waiting());

        runtime.complete_presentation_waits(&waits).unwrap();
        assert!(!runtime.models[0].is_waiting());
        assert!(
            runtime.models[1].is_waiting(),
            "a timeline completion must not dismiss or resume the other model's modal"
        );
        assert!(runtime.presentation_waits().is_empty());
    }

    #[test]
    fn document_snapshot_projects_canonical_waits_when_focus_is_a_later_model() {
        let source = r#"
const title = document_wait_order
puzzle first {
input first_tick
layers { actor = A }
rules {
if input == first_tick {
[ A ] -> wait 100ms
}
}
}
levels first_levels of first {
legend { A = A }
level "first" { A }
}
puzzle second {
input second_tick
layers { actor = B }
rules {
if input == second_tick {
[ B ] -> wait 100ms
}
}
}
levels second_levels of second {
legend { B = B }
level "second" { B }
}
scene playing {
layout {
puzzle second_view = second
}
}
"#;
        let mut document = puzzle_lang::parse_game_for_path(source, "document_wait_order.puzzle")
            .expect("the focused-order fixture must compile");
        let viewport_models = document_viewport_model_owners(&document);
        let mut models = document
            .models
            .drain(..)
            .map(|model| {
                grid_session_model(model, Some("playing"), &viewport_models, true).unwrap()
            })
            .collect::<Vec<_>>();
        models[1].apply_input_name("second_tick").unwrap();
        let runtime = DocumentSessionRuntime {
            models,
            scene_owner: 0,
        };

        assert_eq!(
            runtime.primary_snapshot_index(),
            0,
            "the document scene owner must remain stable when another model owns a pending wait"
        );
        let snapshot = runtime.player_snapshot(7, SceneConditionContext::default(), &[]);
        let continuation = snapshot
            .presentation_continuation
            .expect("the focused model timeline wait must project one document continuation");
        assert_eq!(continuation.revision, 7);
        assert_eq!(
            continuation
                .waits
                .iter()
                .map(|wait| wait.model.as_str())
                .collect::<Vec<_>>(),
            vec!["second"]
        );
    }

    #[test]
    fn document_scene_lifecycle_emits_one_shared_audio_and_wait_timeline() {
        let source = r#"
const title = document_shared_presentation
sounds {
sfx tick {
seed = shared_tick
type = hit
}
}
puzzle first {
layers { actor = A }
rules {}
}
levels first_levels of first {
legend { A = A }
level "first" { A }
}
puzzle second {
layers { actor = B }
rules {}
}
levels second_levels of second {
legend { B = B }
level "second" { B }
}
scene playing {
on_scene_start {
sfx tick
wait 100ms
}
layout {
row {
puzzle first_view = first
puzzle second_view = second
}
}
}
"#;
        let mut runtime =
            RuntimeSession::from_source(source, "document_shared_presentation.puzzle")
                .expect("the multi-model shared presentation fixture must compile");

        let initialized = runtime
            .dispatch_typed(SessionAction::Initialize)
            .expect("the document scene owner must initialize");

        assert_eq!(
            initialized.presentation_events.len(),
            2,
            "shared scene presentation must not be repeated by model replicas"
        );
        assert!(matches!(
            initialized.presentation_events[0],
            RuntimePresentationEvent::Audio { .. }
        ));
        assert!(matches!(
            initialized.presentation_events[1],
            RuntimePresentationEvent::Wait { milliseconds: 100 }
        ));
        assert!(
            initialized.presentation_continuation.is_none(),
            "scene lifecycle waits are ordered presentation effects, not model turn continuations"
        );
    }

    #[test]
    fn document_scene_navigation_materializes_secondary_viewport_from_owner_state() {
        let source = r#"
const title = document_scene_navigation
puzzle first {
layers { actor = A }
rules {}
}
levels first_levels of first {
legend { A = A }
level "first" { A }
}
puzzle second {
layers { actor = B }
rules {}
}
levels second_levels of second {
legend { B = B }
level "second" { B }
}
scene menu {
layout {
button "Open second" -> goto secondary
}
}
scene secondary {
layout {
puzzle second_view = second
}
}
"#;
        let mut runtime = RuntimeSession::from_source(source, "document_scene_navigation.puzzle")
            .expect("the multi-model navigation fixture must compile");
        let before: Value = serde_json::from_str(&runtime.snapshot_json()).unwrap();
        let token = projected_action_for_label(&before, "Open second");

        let after = runtime
            .dispatch_typed(SessionAction::SceneAction { token })
            .expect("the document scene owner must navigate once");

        assert_eq!(after.surface.focus, "secondary");
        assert!(matches!(
            after.viewport_sources.get(&RuntimeViewportSourceId {
                model: "second".to_string(),
                component: "secondary".to_string(),
                source: "second_view".to_string(),
            }),
            Some(RuntimeRendererState::TwoD(_))
        ));
        assert!(
            !after
                .viewport_sources
                .keys()
                .any(|source| source.component == "menu"),
            "secondary model projection must follow the authoritative owner surface"
        );
    }

    #[test]
    fn rejected_document_scene_sequence_leaves_no_partial_presentation_mutation() {
        let source = r#"
const title = document_scene_atomicity
puzzle first {
layers { actor = A }
rules {}
}
levels first_levels of first {
legend { A = A }
level "first" { A }
}
puzzle second {
layers { actor = B }
rules {}
}
levels second_levels of second {
legend { B = B }
level "second" { B }
}
scene playing {
layout {
button "Invalid" -> wait 100ms
puzzle first_view = first
puzzle second_view = second
}
}
"#;
        let mut document =
            puzzle_lang::parse_game_for_path(source, "document_scene_atomicity.puzzle")
                .expect("the atomicity fixture must compile before its runtime fault is injected");
        let game = match &mut document.models[0] {
            LoadedDocumentModel::Puzzle2d { game, .. } => game,
            LoadedDocumentModel::Puzzle3d { .. } => panic!("fixture must lower as Puzzle2"),
        };
        let scene = game
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "playing")
            .unwrap();
        let SceneComponent::Button(button) = &mut scene.components[0] else {
            panic!("fixture must expose its scene action as a button");
        };
        button.effect = SceneEffect::Sequence {
            effects: vec![
                SceneEffect::Wait {
                    milliseconds: Some(100),
                },
                SceneEffect::Goto {
                    scene: "missing_runtime_component".to_string(),
                    params: Vec::new(),
                },
            ],
        };
        let mut runtime =
            RuntimeSession::from_document(document).expect("runtime fault is action-local");
        let before: Value = serde_json::from_str(&runtime.snapshot_json()).unwrap();
        let token = projected_action_for_label(&before, "Invalid");

        let error = runtime
            .dispatch_typed(SessionAction::SceneAction { token })
            .expect_err("the invalid trailing effect must reject the whole scene transaction");
        assert!(error.contains("unknown component target"), "{error}");
        let after = runtime
            .dispatch_typed(SessionAction::Initialize)
            .expect("a rejected scene action must leave the session usable");
        assert_eq!(after.revision, 0);
        assert!(!after.busy);
        assert!(after.presentation_events.is_empty());
        assert!(after.presentation_continuation.is_none());
        assert_eq!(after.surface.focus, "playing");
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

        let resumed: Value = serde_json::from_str(
            &runtime
                .dispatch(presentation_complete_action(&queued))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(resumed["busy"], false);
        assert!(cell_has_object(
            &first_viewport_state(&resumed)["cells"][0],
            "A"
        ));

        let first_undo: Value =
            serde_json::from_str(&runtime.dispatch(SessionAction::Undo).unwrap()).unwrap();
        assert!(cell_has_object(
            &first_viewport_state(&first_undo)["cells"][0],
            "B"
        ));
        let second_undo: Value =
            serde_json::from_str(&runtime.dispatch(SessionAction::Undo).unwrap()).unwrap();
        assert!(cell_has_object(
            &first_viewport_state(&second_undo)["cells"][0],
            "A"
        ));
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
            event_kinds.extend(events.iter().map(|event| {
                if event["kind"] == "audio" {
                    event["command"]["kind"].as_str().unwrap().to_string()
                } else {
                    event["kind"].as_str().unwrap().to_string()
                }
            }));
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
                event_kinds.push("component:modal".to_string());
                let token: RuntimeSceneActionToken = serde_json::from_value(
                    component["presentation"]["events"][STANDARD_MESSAGE_DISMISS_EVENT]
                        ["actionToken"]
                        .clone(),
                )
                .expect("active modal should project its typed dismissal token");
                serde_json::from_str(
                    &bridge
                        .dispatch(SessionAction::SceneAction { token })
                        .unwrap(),
                )
                .unwrap()
            } else {
                serde_json::from_str(
                    &bridge
                        .dispatch(presentation_complete_action(&snapshot))
                        .unwrap(),
                )
                .unwrap()
            };
        }
        assert_eq!(
            event_kinds,
            vec![
                "wait",
                "component:modal",
                "animation_batch",
                "play_sfx",
                "wait"
            ]
        );
        assert!(cell_has_object(
            &first_viewport_state(&snapshot)["cells"][1],
            "Done"
        ));
        assert!(snapshot.get("sounds").is_none());
        let serialized = serde_json::to_string(&snapshot).unwrap();
        for authoring_field in ["tick01", "\"type\"", "\"seed\"", "\"bars\"", "\"bpm\""] {
            assert!(
                !serialized.contains(authoring_field),
                "player snapshot leaked authored sound recipe field `{authoring_field}`"
            );
        }
    }

    #[test]
    fn standalone_snapshot_projects_each_presented_component_instance_state() {
        let source = r#"
const title = runtime_component_instances

scene playing {
layout {
puzzle board = default
button "Create Hidden" -> create panel
}
on_scene_start {
present panel(count = 1) as content
present panel(count = 2) as content
}
}

puzzle default {
layers { actor = Player }
empty .
rules {}
levels {
legend { P = Player }
level "start" { P }
}
}

scene panel {
var count = 1
layout {
text count
button "Increment" -> input increment
puzzle panel_board = default
}
keys {
Enter -> input increment
}
}
"#;
        let bridge =
            RuntimeSession::from_source(source, "runtime_component_instances.puzzle").unwrap();
        let snapshot: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        let panels = snapshot["surface"]["components"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|component| {
                component["id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("component:"))
                    && component["modal"] == false
            })
            .collect::<Vec<_>>();

        assert_eq!(panels.len(), 2);
        assert_ne!(panels[0]["id"], panels[1]["id"]);
        for panel in &panels {
            let id = panel["id"].as_str().unwrap();
            assert!(
                snapshot["viewportSources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|entry| {
                        entry["id"]["component"] == id && entry["id"]["source"] == "panel_board"
                    }),
                "each component instance must own a distinct viewport source"
            );
        }
        assert_eq!(panels[0]["presentation"]["components"][0]["value"], "1");
        assert_eq!(panels[1]["presentation"]["components"][0]["value"], "2");
        assert!(
            panels
                .iter()
                .all(|panel| panel["presentation"]["components"][1]["actionToken"].is_null()),
            "unfocused component instances must not project executable node actions"
        );
        assert!(
            panels
                .iter()
                .all(|panel| panel["presentation"]["keys"].is_null()),
            "unfocused component instances must not project executable key actions"
        );

        let mut hidden_bridge =
            RuntimeSession::from_source(source, "runtime_hidden_component.puzzle").unwrap();
        let initial: Value = serde_json::from_str(&hidden_bridge.snapshot_json()).unwrap();
        let token = projected_action_for_label(&initial, "Create Hidden");
        hidden_bridge
            .dispatch(SessionAction::SceneAction { token })
            .unwrap();
        let snapshot: Value = serde_json::from_str(&hidden_bridge.snapshot_json()).unwrap();
        assert_eq!(
            snapshot["surface"]["components"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|component| component["id"] == "panel")
                .count(),
            0,
            "hidden component instances must be absent from the resolved visible surface"
        );
        assert!(
            snapshot["viewportSources"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["id"]["component"] != "panel"),
            "hidden component instances must not publish viewport registry entries"
        );
    }

    #[test]
    fn viewport_registry_contains_only_sources_referenced_by_resolved_viewport_leaves() {
        let source = r#"
const title = runtime_visible_viewport_sources

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
layout {
if false {
puzzle ghost = default
}
puzzle board = default
}
}
"#;
        let mut bridge =
            RuntimeSession::from_source(source, "runtime_visible_viewport_sources.puzzle").unwrap();
        let snapshot: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::GotoLevel { level: 0 })
                .unwrap(),
        )
        .unwrap();
        let sources = snapshot["viewportSources"].as_array().unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(
            sources[0]["id"],
            json!({ "model": "default", "component": "playing", "source": "board" })
        );
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
        let mut bridge = RuntimeSession::from_source(source, "export_test.puzzle").unwrap();

        let body: Value =
            serde_json::from_str(&bridge.apply_debug_input_name_json("right").unwrap()).unwrap();

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
        assert_eq!(initial["surface"]["focus"], "title");
        assert!(initial.get("title").is_none());
        assert!(initial.get("gameState").is_none());
        assert_eq!(initial["solverState"]["kind"], "2d");
        assert!(initial["solverState"]["slots"].is_array());
        assert!(initial["solverState"].get("slotMarks").is_none());
        assert!(initial["solverState"].get("cellMarks").is_none());
        assert!(initial.get("scenes").is_none());

        let title = initial["surface"]["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|component| component["id"] == "title")
            .unwrap();
        assert!(title.get("definition").is_none());
        assert!(title.get("properties").is_none());
        assert!(title["presentation"].get("name").is_none());
        assert_eq!(title["presentation"]["components"][0]["kind"], "text");
        assert_eq!(title["presentation"]["components"][0]["role"], "heading");

        let playing = activate_selected_choice(&mut bridge);
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
        let bridge = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "runtime_scene_fixture.puzzle",
        )
        .unwrap();

        let snapshot: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        let title = &snapshot["surface"]["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|component| component["id"] == "title")
            .unwrap()["presentation"];
        assert_eq!(title["components"][0]["value"], "Runtime Scene Fixture");
        let components = title["components"].as_array().unwrap();
        assert_eq!(components.len(), 2);
        assert_eq!(components[1]["kind"], "choice");
        assert_eq!(components[1]["label"], "New Game");
        assert!(
            components
                .iter()
                .all(|component| component["kind"] != "conditional")
        );
    }

    #[test]
    fn runtime_session_projects_selected_state_on_the_owned_choice_node() {
        let source = r#"
const title = runtime_choice_cursor

scene menu {
  layout {
    puzzle board
    row {
      choice "A" -> goto a
      choice "B" -> goto b
    }
  }
}

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

        let initial: Value = serde_json::from_str(&runtime.snapshot_json()).unwrap();
        assert_eq!(projected_selected_choice(&initial), "A");
        let initial_action = projected_selected_action(&initial);
        let repeated: Value = serde_json::from_str(
            &runtime
                .dispatch(SessionAction::Snapshot)
                .expect("snapshot should preserve scene action identity"),
        )
        .unwrap();
        assert_eq!(projected_selected_action(&repeated), initial_action);
        assert!(
            initial["surface"]["components"]
                .as_array()
                .unwrap()
                .iter()
                .all(|component| component.get("choiceCursor").is_none())
        );

        let moved: Value = serde_json::from_str(
            &runtime
                .dispatch(SessionAction::ChoiceMove {
                    direction: RuntimeChoiceDirection::Right,
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(projected_selected_choice(&moved), "B");
        let selected_action = projected_selected_action(&moved);
        assert_ne!(selected_action, initial_action);

        let activated: Value = serde_json::from_str(
            &runtime
                .dispatch(SessionAction::SceneAction {
                    token: selected_action,
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(activated["surface"]["focus"], "b");
        let stale_error = runtime
            .dispatch(SessionAction::SceneAction {
                token: initial_action,
            })
            .unwrap_err();
        assert!(stale_error.contains("is not mounted"), "{stale_error}");
    }

    #[test]
    fn standalone_session_bridge_projects_no_viewport_for_text_only_focus() {
        let source = r#"
const title = runtime_focus

scene level_select {
layout {
text "Select"
}
}

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

"#;
        let mut bridge = RuntimeSession::from_source(source, "runtime_focus.puzzle").unwrap();

        let select: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(select["surface"]["focus"], json!("level_select"));
        assert_eq!(select["viewportSources"], json!([]));

        let after_input: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "down".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(after_input["surface"]["focus"], json!("level_select"));
        assert_eq!(after_input["viewportSources"], json!([]));
        assert_eq!(after_input["presentationEvents"], json!([]));
    }

    #[cfg(feature = "editor-debug")]
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
        assert!(cell_has_object(
            &first_viewport_state(&started)["cells"][0],
            "Player"
        ));

        let moved: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "right".to_string(),
                })
                .expect("right should move from editor state"),
        )
        .unwrap();
        assert!(cell_has_object(
            &first_viewport_state(&moved)["cells"][1],
            "Player"
        ));

        let restarted: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Restart)
                .expect("restart should use editor start state"),
        )
        .unwrap();
        assert!(cell_has_object(
            &first_viewport_state(&restarted)["cells"][0],
            "Player"
        ));

        assert!(
            bridge
                .set_current_state_json(&editor_state, 99, false)
                .unwrap_err()
                .contains("level index out of range: 99")
        );
    }

    #[test]
    fn spec_2d_new_game_uses_typed_input_and_component_viewport_source() {
        let source = runtime_scene_fixture_source();
        let mut bridge =
            RuntimeSession::from_source(source, "runtime_scene_fixture.puzzle").unwrap();

        let title: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Snapshot).unwrap()).unwrap();
        assert!(title["inputs"].as_array().unwrap().iter().any(|input| {
            input["name"] == "up"
                && input["triggers"].as_array().is_some_and(|triggers| {
                    triggers.iter().any(|trigger| trigger["kind"] == "arrow_up")
                })
        }));

        let playing = start_spec_2d_new_game(&mut bridge);
        assert_eq!(playing["surface"]["focus"], "playing");
        assert_eq!(playing["levelIndex"], 0);
        let playing_object = playing.as_object().unwrap();
        assert!(playing_object.contains_key("surface"));
        assert!(playing_object.contains_key("viewportSources"));
        assert!(!playing_object.contains_key("visibleScenes"));
        assert!(!playing_object.contains_key("sceneLayers"));
        assert!(!playing_object.contains_key("currentScene"));
        assert!(!playing_object.contains_key("scene"));
        assert!(!playing_object.contains_key("sceneState"));
        assert!(!playing_object.contains_key("scenePuzzles"));
        assert!(!playing_object.contains_key("scenePuzzleState"));
        assert!(!playing_object.contains_key("visibleScreens"));
        assert!(!playing_object.contains_key("screenState"));
        assert!(!playing_object.contains_key("screenPuzzles"));
        let scene = viewport_state(&playing, "playing", "board");
        assert_eq!(scene["cells"].as_array().unwrap().len(), 42);
        assert!(
            scene["cells"]
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
    fn transition_flickscreen_resolves_player_group_to_typed_view() {
        let puzzlescript =
            include_str!("../../../crates/lang/tests/fixtures/puzzlescript/gallery/transition.ps");
        let source = puzzle_lang::translate_puzzlescript_to_canonical(puzzlescript).unwrap();
        let mut bridge =
            RuntimeSession::from_source(&source, "fixtures/transition.puzzle").unwrap();

        let playing: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::GotoLevel { level: 0 })
                .unwrap(),
        )
        .unwrap();
        let scene = first_viewport_state(&playing);

        assert_eq!(scene["view"], json!({"origin": [0, 0], "size": [13, 13]}));
        assert!(scene.get("screen").is_none());
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

            if first_viewport_state(&before) != first_viewport_state(&after)
                || before["canUndo"] != after["canUndo"]
            {
                changed_input = Some(input);
                break;
            }
        }

        assert!(changed_input.is_some());
    }

    fn start_spec_2d_new_game(bridge: &mut RuntimeSession) -> Value {
        activate_selected_choice(bridge)
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
                .dispatch(SessionAction::GotoLevel { level: 0 })
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
                "source": {
                    "model": "mover",
                    "component": "mover",
                    "source": "mover"
                },
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
            viewport_state(&moved, "mover", "mover")["cells"],
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
    fn level_menu_resolves_every_level_progress_path_before_and_after_restore() {
        let source = r#"
const title = level_menu_runtime_progress

scene level_select {
  layout {
    level_menu {
      show_index = true
      show_solved = true
    }
  }
}

puzzle board {
  layers { actor = Player }
  empty .
  rules {}
  levels {
    legend P = Player
    level "first" { P }
    level "second" { P }
  }
}
"#;
        let mut runtime =
            RuntimeSession::from_source(source, "level_menu_runtime_progress.puzzle").unwrap();
        let choice_labels = |snapshot: RuntimeSessionSnapshot| {
            let component = snapshot
                .surface
                .components
                .into_iter()
                .find(|component| component.id == "level_select")
                .expect("level_select must be mounted");
            let RuntimeComponentPresentation::Ready(scene) = component.presentation else {
                panic!("level_select progress expressions must resolve");
            };
            let [RuntimeResolvedSceneComponent::Column { children, .. }] =
                scene.components.as_slice()
            else {
                panic!("level_menu must resolve through its common column contract");
            };
            children
                .iter()
                .map(|child| match child {
                    RuntimeResolvedSceneComponent::Choice { label, .. } => label.clone(),
                    other => panic!("level_menu child must be a choice, got {other:?}"),
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(choice_labels(runtime.snapshot()), ["1. first", "2. second"]);

        let second_level = LevelId::new("board", "second").record_key();
        runtime
            .restore_progress_save_json(
                &json!({
                    "version": 2,
                    "levels": [{"id": second_level, "cleared": true}],
                    "currentLevel": second_level,
                    "persistentVars": [],
                })
                .to_string(),
            )
            .unwrap();

        assert_eq!(
            choice_labels(runtime.snapshot()),
            ["1. first", "✓ 2. second"]
        );
    }

    #[test]
    fn unresolved_scene_condition_fails_presentation_and_typed_action_dispatch() {
        let source = r#"
const title = unresolved_runtime_condition

scene menu {
  layout {
    if true {
      choice "Open" -> goto opened
    }
  }
}

scene opened {
  layout { text "Opened" }
}

puzzle board {
  layers { actor = Player }
  empty .
  rules {}
  levels {
    legend P = Player
    level "first" { P }
  }
}
"#;
        let mut document =
            puzzle_lang::parse_game_for_path(source, "unresolved_runtime_condition.puzzle")
                .unwrap();
        let LoadedDocumentModel::Puzzle2d { game, .. } = &mut document.models[0] else {
            panic!("fixture must lower as Puzzle2");
        };
        let menu = game
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "menu")
            .unwrap();
        let SceneComponent::Conditional(conditional) = &mut menu.components[0] else {
            panic!("fixture must contain a conditional choice");
        };
        conditional.condition = SceneExpr::Path(vec!["unknown_runtime_condition".to_string()]);

        let mut runtime = RuntimeSession::from_document(document).unwrap();
        let snapshot = runtime.snapshot();
        let menu = snapshot
            .surface
            .components
            .iter()
            .find(|component| component.id == "menu")
            .unwrap();
        let RuntimeComponentPresentation::Error { error } = &menu.presentation else {
            panic!("an unresolved condition must fail its owning presentation");
        };
        assert!(
            error.contains("unknown value") && error.contains("unknown_runtime_condition"),
            "{error}"
        );

        let dispatch_error = runtime
            .dispatch_typed(SessionAction::SceneAction {
                token: RuntimeSceneActionToken {
                    component: "menu".to_string(),
                    action: RuntimeSceneActionId::Node { ordinal: 0 },
                },
            })
            .unwrap_err();
        assert!(
            dispatch_error.contains("unknown value")
                && dispatch_error.contains("unknown_runtime_condition"),
            "{dispatch_error}"
        );
    }

    #[test]
    fn restored_continue_token_is_executable_under_the_projected_progress_context() {
        let source = runtime_scene_fixture_source();
        let mut runtime =
            RuntimeSession::from_source(source, "runtime_scene_fixture.puzzle").unwrap();
        let restored_level_key = LevelId::new("board", "microban.2").record_key();
        runtime
            .restore_progress_save_json(
                &json!({
                    "version": 2,
                    "levels": [],
                    "currentLevel": restored_level_key,
                    "persistentVars": [],
                })
                .to_string(),
            )
            .unwrap();

        let projected: Value = serde_json::from_str(&runtime.snapshot_json()).unwrap();
        assert_eq!(projected["has_progress_save"], true);
        let continue_token = projected_action_for_label(&projected, "Continue");
        let continued = runtime
            .dispatch_typed(SessionAction::SceneAction {
                token: continue_token,
            })
            .expect("a Continue token projected for restored progress must remain executable");

        assert_eq!(continued.surface.focus, "playing");
    }

    #[test]
    fn progress_is_persisted_only_after_the_matching_host_acknowledgement() {
        let mut runtime = RuntimeSession::from_source(
            runtime_scene_fixture_source(),
            "runtime_scene_fixture.puzzle",
        )
        .unwrap();
        let after_action = activate_selected_choice(&mut runtime);
        assert_eq!(after_action["has_progress_save"], false);
        let request = runtime
            .progress_save_request()
            .expect("successful session mutation should request persistence");
        assert!(
            runtime
                .confirm_progress_persistence_applied(request.request_id + 1)
                .unwrap_err()
                .contains("stale")
        );
        assert!(matches!(
            request.operation,
            RuntimeProgressPersistenceOperation::Write { .. }
        ));
        assert_eq!(
            serde_json::from_str::<Value>(&runtime.snapshot_json()).unwrap()["has_progress_save"],
            false
        );

        runtime
            .confirm_progress_persistence_applied(request.request_id)
            .unwrap();
        assert!(runtime.progress_save_request().is_none());
        assert_eq!(
            serde_json::from_str::<Value>(&runtime.snapshot_json()).unwrap()["has_progress_save"],
            true
        );
    }

    #[test]
    fn clear_game_progress_requests_typed_persistence_deletion_until_matching_ack() {
        let source = r#"
const title = progress_delete
scene playing {
  layout {
    puzzle board = board
    button "Clear" -> clear_game_progress
  }
}
puzzle board {
  layers { actor = Player }
  rules {}
}
levels default of board {
  legend P = Player
  level "one" { P }
}
"#;
        let mut runtime = RuntimeSession::from_source(source, "progress_delete.puzzle").unwrap();
        runtime
            .restore_progress_save_json(
                &json!({
                    "version": 2,
                    "levels": [],
                    "currentLevel": null,
                    "persistentVars": [],
                })
                .to_string(),
            )
            .unwrap();
        let projected: Value = serde_json::from_str(&runtime.snapshot_json()).unwrap();
        let token = projected_action_for_label(&projected, "Clear");
        let after_clear = runtime
            .dispatch_typed(SessionAction::SceneAction { token })
            .unwrap();
        let request = runtime
            .progress_save_request()
            .expect("clear_game_progress must request persistence deletion");

        assert!(matches!(
            request.operation,
            RuntimeProgressPersistenceOperation::Delete
        ));
        assert!(
            after_clear.has_progress_save,
            "persisted progress remains observable until the host confirms deletion"
        );
        runtime
            .confirm_progress_persistence_applied(request.request_id)
            .unwrap();
        assert!(!runtime.snapshot().has_progress_save);
    }

    #[test]
    fn mixed_dimension_document_session_starts_and_projects_each_owned_world() {
        let source = r#"
const title = mixed_runtime
puzzle flat {
  layers { actor = FlatPlayer }
  input flat_move
  input shared_move
  rules {}
}
levels flat_levels of flat {
  legend P = FlatPlayer
  level "flat" { P }
}
puzzle cube {
  dimension = 3
  layers { actor = CubePlayer }
  input shared_move
  rules {}
}
levels cube_levels of cube {
  legend P = CubePlayer
  level "cube" { P }
}
scene playing {
  layout {
    row {
      puzzle flat_board = flat
      puzzle cube_board = cube
    }
  }
}
"#;
        let mut runtime = RuntimeSession::from_source(source, "mixed_runtime.puzzle")
            .expect("mixed document must construct its document session runtime");
        let snapshot = runtime.snapshot();

        assert!(
            matches!(
                snapshot.viewport_sources.get(&RuntimeViewportSourceId {
                    model: "flat".to_string(),
                    component: "playing".to_string(),
                    source: "flat_board".to_string(),
                }),
                Some(RuntimeRendererState::TwoD(_))
            ),
            "{:?}",
            snapshot.viewport_sources.keys().collect::<Vec<_>>()
        );
        assert!(matches!(
            snapshot.viewport_sources.get(&RuntimeViewportSourceId {
                model: "cube".to_string(),
                component: "playing".to_string(),
                source: "cube_board".to_string(),
            }),
            Some(RuntimeRendererState::ThreeD(_))
        ));
        assert_eq!(snapshot.level_count, 2);
        runtime
            .dispatch_typed(SessionAction::Input {
                name: "flat_move".to_string(),
            })
            .expect("an input owned by one focused model must route to that model");
        let error = runtime
            .dispatch_typed(SessionAction::Input {
                name: "shared_move".to_string(),
            })
            .expect_err("one input must not be broadcast to multiple puzzle models");
        assert!(error.contains("ambiguous across 2 focused puzzle models"));
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
        let spatial_view = viewport_state(&snapshot, "sokoban", "sokoban").clone();
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
        assert!(spatial_view.get("objects").is_none());
        assert!(spatial_view.get("visuals").is_none());
        assert!(spatial_view.get("component").is_none());
        assert!(spatial_view.get("animationEvents").is_none());
        assert!(spatial_view.get("animationBatchId").is_none());
        assert!(spatial_view.get("render").is_some());
        assert!(spatial_view.get("order").is_none());
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
