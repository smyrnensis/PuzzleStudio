#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    error::Error,
    fmt,
    sync::Arc,
};

#[cfg(not(target_arch = "wasm32"))]
mod audio;
#[cfg(feature = "editor-debug")]
mod editor_authoring;
#[cfg(target_arch = "wasm32")]
mod web_audio;

#[cfg(feature = "editor-debug")]
pub use editor_authoring::EditorAuthoringFrame;

#[cfg(target_arch = "wasm32")]
use bevy::app::PluginGroup;
use bevy::prelude::*;
use bevy::{
    camera::{ClearColorConfig, visibility::RenderLayers},
    ecs::message::MessageReader,
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
        mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    },
    text::LineHeight,
};
use puzzle_assets::DecodedVisualImageCatalog;
use puzzle_audio::{
    AudioCapabilityState, AudioDeviceCommand, AudioDiagnostic, AudioRuntime, AudioVoiceId,
    CANONICAL_AUDIO_SAMPLE_RATE,
};
use puzzle_bevy_renderer::{
    BevyPublicationGroupId, BevyPublicationGroups, BevyPublicationMember, BevyRenderError,
    BevyResolvedFrameQueue, BevyResolvedFrameQueue2d, PuzzleBevy2dPlugin, PuzzleBevy2dView,
    PuzzleBevy3dPlugin, PuzzleBevy3dRenderSettings, PuzzleBevy3dView, PuzzleBevyCamera,
    PuzzleBevyFramebufferRect, PuzzleBevyLighting, PuzzleBevyPixelate, PuzzleBevyRendererSystems,
    PuzzleBevyViewId, PuzzleCameraProjection, prepare_resolved_frame, prepare_resolved_frame_2d,
};
#[cfg(feature = "editor-debug")]
use puzzle_editor_preview_contract::{
    EditorAuthoringHitTarget, EditorAuthoringPresentation, EditorPointerGesture,
    EditorRendererStrategy,
};
#[cfg(feature = "editor-debug")]
use puzzle_game_runtime::RuntimeEditorModelProjection;
use puzzle_game_runtime::RuntimeSession;
use puzzle_player_bootstrap::{PlayerBootstrapError, decode_standalone_player_export};
#[cfg(test)]
use puzzle_runtime_contract::RuntimeProgressPersistenceOperation;
#[cfg(feature = "editor-debug")]
use puzzle_runtime_contract::RuntimeStateSnapshot;
use puzzle_runtime_contract::{
    RuntimeKeyTrigger, RuntimeLinearRgba, RuntimePreparedAnimationSet,
    RuntimePresentationContinuationToken, RuntimePresentationEvent,
    RuntimePresentationTransitionId, RuntimePresentationWait, RuntimeProgressSaveRequest,
    RuntimePuzzle3CameraProjection, RuntimeResolvedNextSample, RuntimeResolvedRenderFrame,
    RuntimeSceneActionToken, RuntimeTheme, RuntimeUiTextStyle, RuntimeViewportSourceId,
    SessionAction, StandaloneProgressStorage,
};
use puzzle_scene::SceneTextRole;
use puzzle_session_contract::{
    RuntimeBusyInputPolicy, RuntimeComponentPresentation, RuntimeRendererState,
    RuntimeResolvedSceneComponent, RuntimeSessionSnapshot, RuntimeSurface,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

fn audio_frame(now_seconds: f64) -> u64 {
    (now_seconds.max(0.0) * f64::from(CANONICAL_AUDIO_SAMPLE_RATE))
        .floor()
        .min(u64::MAX as f64) as u64
}

fn presentation_clock_micros() -> Option<u64> {
    #[cfg(target_arch = "wasm32")]
    {
        let milliseconds = web_sys::window()?.performance()?.now();
        milliseconds
            .is_finite()
            .then(|| (milliseconds.max(0.0) * 1_000.0).min(u64::MAX as f64) as u64)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        static EPOCH: OnceLock<Instant> = OnceLock::new();
        Some(
            u64::try_from(EPOCH.get_or_init(Instant::now).elapsed().as_micros())
                .unwrap_or(u64::MAX),
        )
    }
}

fn wasm_linear_memory_bytes() -> Option<u64> {
    #[cfg(target_arch = "wasm32")]
    {
        Some(
            u64::try_from(core::arch::wasm32::memory_size(0))
                .unwrap_or(u64::MAX)
                .saturating_mul(65_536),
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

fn initial_audio_capability() -> AudioCapabilityState {
    #[cfg(target_arch = "wasm32")]
    {
        AudioCapabilityState::Locked
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        AudioCapabilityState::Ready
    }
}

#[derive(Debug)]
pub enum BevyPlayerError {
    Bootstrap(PlayerBootstrapError),
    Runtime(String),
    MissingViewportSource(RuntimeViewportSourceId),
    MissingViewportLayout(RuntimeViewportSourceId),
    MissingPresentationTarget(RuntimeViewportSourceId),
    MissingPrimaryWindow,
    DuplicateViewportLeaf(RuntimeViewportSourceId),
    UnsupportedFrameComponent {
        kind: String,
        source: String,
    },
    ViewportOrderOverflow {
        count: usize,
    },
    ViewportDimensionMismatch {
        source: RuntimeViewportSourceId,
        surface: puzzle_session_contract::RuntimeViewportDimension,
        renderer: &'static str,
    },
    TwoDimensionalDisplay {
        source: RuntimeViewportSourceId,
        error: String,
    },
    Presentation {
        source: RuntimeViewportSourceId,
        error: String,
    },
    ComponentPresentation {
        component: String,
        error: String,
    },
    MissingAwaitedEvent {
        component: String,
        event: String,
    },
    InvalidCameraZoom {
        source: RuntimeViewportSourceId,
        zoom: f64,
    },
    EditorAuthoring(String),
    Renderer(BevyRenderError),
}

impl fmt::Display for BevyPlayerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bootstrap(error) => error.fmt(formatter),
            Self::Runtime(error) => write!(formatter, "game runtime failed: {error}"),
            Self::MissingViewportSource(source) => write!(
                formatter,
                "resolved viewport {}.{} has no runtime viewport source",
                source.component, source.source
            ),
            Self::MissingViewportLayout(source) => write!(
                formatter,
                "resolved viewport {}.{} has no non-empty Bevy UI layout rectangle",
                source.component, source.source
            ),
            Self::MissingPresentationTarget(source) => write!(
                formatter,
                "presentation event targets missing viewport source {}.{}",
                source.component, source.source
            ),
            Self::MissingPrimaryWindow => {
                write!(
                    formatter,
                    "Bevy viewport rendering requires a primary window"
                )
            }
            Self::DuplicateViewportLeaf(source) => write!(
                formatter,
                "viewport source {}.{} is mounted by more than one resolved UI leaf",
                source.component, source.source
            ),
            Self::UnsupportedFrameComponent { kind, source } => write!(
                formatter,
                "resolved `{kind}` frame component `{source}` is unsupported by the Bevy player; use a typed viewport component"
            ),
            Self::ViewportOrderOverflow { count } => write!(
                formatter,
                "resolved surface has {count} viewport leaves, exceeding Bevy camera order capacity"
            ),
            Self::ViewportDimensionMismatch {
                source,
                surface,
                renderer,
            } => write!(
                formatter,
                "viewport {}.{} declares {surface:?} on the surface but resolves to {renderer}",
                source.component, source.source
            ),
            Self::TwoDimensionalDisplay { source, error } => write!(
                formatter,
                "2D presentation for {}.{} failed: {error}",
                source.component, source.source
            ),
            Self::Presentation { source, error } => write!(
                formatter,
                "presentation for {}.{} failed: {error}",
                source.component, source.source
            ),
            Self::ComponentPresentation { component, error } => {
                write!(
                    formatter,
                    "component {component} presentation failed: {error}"
                )
            }
            Self::MissingAwaitedEvent { component, event } => write!(
                formatter,
                "component {component} awaits undeclared presentation event {event}"
            ),
            Self::InvalidCameraZoom { source, zoom } => write!(
                formatter,
                "3D viewport {}.{} camera zoom must be finite and greater than zero, got {zoom}",
                source.component, source.source
            ),
            Self::EditorAuthoring(error) => write!(formatter, "editor authoring failed: {error}"),
            Self::Renderer(error) => write!(formatter, "Bevy renderer failed: {error}"),
        }
    }
}

impl Error for BevyPlayerError {}

impl From<BevyRenderError> for BevyPlayerError {
    fn from(error: BevyRenderError) -> Self {
        Self::Renderer(error)
    }
}

pub struct LoadedStandaloneBevyPlayer {
    pub host: PuzzleBevyPlayerHost,
    pub progress_storage: StandaloneProgressStorage,
}

pub fn load_standalone_bevy_player(
    export_json: &str,
) -> Result<LoadedStandaloneBevyPlayer, BevyPlayerError> {
    let (runtime, visual_images, progress_storage) = decode_standalone_player_export(export_json)
        .map_err(BevyPlayerError::Bootstrap)?
        .into_parts();
    let host = PuzzleBevyPlayerHost::from_runtime_with_visual_images(runtime, visual_images)?;
    Ok(LoadedStandaloneBevyPlayer {
        host,
        progress_storage,
    })
}

#[derive(Clone)]
struct QueuedPresentationEvent {
    event: RuntimePresentationEvent,
}

#[derive(Clone)]
struct HostViewport {
    source: RuntimeViewportSourceId,
    order: isize,
    renderer: RuntimeRendererState,
    active_animation: Option<RuntimePreparedAnimationSet>,
    animation_epoch_seconds: f64,
    next_sample: Option<HostNextSample>,
    needs_frame: bool,
}

#[derive(Clone, Copy)]
enum HostNextSample {
    Deadline {
        at_seconds: f64,
    },
    DisplayRefresh {
        completion_at_seconds: f64,
        last_sample_seconds: f64,
    },
}

impl HostNextSample {
    fn consume_if_due(&mut self, now_seconds: f64) -> bool {
        match self {
            Self::Deadline { at_seconds } => now_seconds >= *at_seconds,
            Self::DisplayRefresh {
                completion_at_seconds,
                last_sample_seconds,
            } if now_seconds > *last_sample_seconds
                && *last_sample_seconds < *completion_at_seconds =>
            {
                *last_sample_seconds = now_seconds.min(*completion_at_seconds);
                true
            }
            Self::DisplayRefresh { .. } => false,
        }
    }
}

pub struct ResolvedBevyViewportFrame {
    pub source: RuntimeViewportSourceId,
    pub renderer: RuntimeRendererState,
    pub frame: RuntimeResolvedRenderFrame,
}

pub struct PuzzleBevyPlayerHost {
    runtime: RuntimeSession,
    visual_images: Arc<DecodedVisualImageCatalog>,
    snapshot: RuntimeSessionSnapshot,
    ui_projection: UiProjectionIdentity,
    ui_projection_generation: u64,
    viewports: BTreeMap<RuntimeViewportSourceId, HostViewport>,
    animation_origins: BTreeMap<RuntimeViewportSourceId, RuntimeRendererState>,
    animation_origin_transition: Option<RuntimePresentationTransitionId>,
    pending_presentation: VecDeque<QueuedPresentationEvent>,
    pending_presentation_continuation: Option<RuntimePresentationContinuationToken>,
    audio_runtime: AudioRuntime,
    pending_audio: Vec<AudioDeviceCommand>,
    wait_until_seconds: Option<f64>,
    wait_started_seconds: Option<f64>,
    waiting_for_animation_publication: Option<RuntimePresentationTransitionId>,
    renderer_publication_group: Option<BevyPublicationGroupId>,
    renderer_publication_animation_transition: Option<RuntimePresentationTransitionId>,
    queued_input_presentation: Option<RuntimeBusyInputPolicy>,
    clip_epoch_seconds: f64,
    fatal_error: Option<String>,
    #[cfg(feature = "editor-debug")]
    editor_authoring: Option<editor_authoring::EditorAuthoringConfiguration>,
    #[cfg(feature = "editor-debug")]
    editor_authoring_frame: Option<EditorAuthoringFrame>,
    #[cfg(feature = "editor-debug")]
    next_editor_authoring_frame_revision: u64,
}

enum AudioDeviceFeedback {
    #[cfg(target_arch = "wasm32")]
    Capability(AudioCapabilityState),
    VoiceEnded(AudioVoiceId),
    VoiceFailure {
        voice: AudioVoiceId,
        error: String,
    },
    #[cfg(target_arch = "wasm32")]
    DeviceFailure(String),
}

impl PuzzleBevyPlayerHost {
    pub fn from_image_free_source(
        source: &str,
        puzzle_path: &str,
    ) -> Result<Self, BevyPlayerError> {
        Self::from_source_with_visual_images(
            source,
            puzzle_path,
            Arc::new(DecodedVisualImageCatalog::default()),
        )
    }

    pub fn from_source_with_visual_images(
        source: &str,
        puzzle_path: &str,
        visual_images: Arc<DecodedVisualImageCatalog>,
    ) -> Result<Self, BevyPlayerError> {
        let runtime =
            RuntimeSession::from_source(source, puzzle_path).map_err(BevyPlayerError::Runtime)?;
        Self::from_runtime_with_visual_images(runtime, visual_images)
    }

    pub fn from_image_free_runtime(runtime: RuntimeSession) -> Result<Self, BevyPlayerError> {
        Self::from_runtime_with_visual_images(
            runtime,
            Arc::new(DecodedVisualImageCatalog::default()),
        )
    }

    pub fn from_runtime_with_visual_images(
        mut runtime: RuntimeSession,
        visual_images: Arc<DecodedVisualImageCatalog>,
    ) -> Result<Self, BevyPlayerError> {
        let audio_catalog = runtime.audio_catalog();
        let origin_snapshot = runtime.snapshot();
        let origin_viewports = projected_viewports(&origin_snapshot, 0.0)?;
        let snapshot = runtime
            .dispatch_typed(SessionAction::Initialize)
            .map_err(BevyPlayerError::Runtime)?;
        let ui_projection = UiProjectionIdentity::player(&origin_snapshot);
        let mut host = Self {
            runtime,
            visual_images,
            snapshot: origin_snapshot,
            ui_projection,
            ui_projection_generation: 1,
            viewports: origin_viewports,
            animation_origins: BTreeMap::new(),
            animation_origin_transition: None,
            pending_presentation: VecDeque::new(),
            pending_presentation_continuation: None,
            audio_runtime: AudioRuntime::new(audio_catalog, initial_audio_capability()),
            pending_audio: Vec::new(),
            wait_until_seconds: None,
            wait_started_seconds: None,
            waiting_for_animation_publication: None,
            renderer_publication_group: None,
            renderer_publication_animation_transition: None,
            queued_input_presentation: None,
            clip_epoch_seconds: 0.0,
            fatal_error: None,
            #[cfg(feature = "editor-debug")]
            editor_authoring: None,
            #[cfg(feature = "editor-debug")]
            editor_authoring_frame: None,
            #[cfg(feature = "editor-debug")]
            next_editor_authoring_frame_revision: 1,
        };
        host.apply_snapshot(snapshot, 0.0, false)?;
        Ok(host)
    }

    pub fn snapshot(&self) -> &RuntimeSessionSnapshot {
        &self.snapshot
    }

    pub fn fatal_error(&self) -> Option<&str> {
        self.fatal_error.as_deref()
    }

    pub fn viewport_count(&self) -> usize {
        self.viewports.len()
    }

    pub fn take_audio_commands(&mut self) -> Vec<AudioDeviceCommand> {
        std::mem::take(&mut self.pending_audio)
    }

    pub fn set_audio_capability(&mut self, capability: AudioCapabilityState, now_seconds: f64) {
        self.pending_audio.extend(
            self.audio_runtime
                .set_capability(capability, audio_frame(now_seconds)),
        );
    }

    pub fn take_audio_diagnostics(&mut self) -> Vec<AudioDiagnostic> {
        self.audio_runtime.take_diagnostics()
    }

    pub fn audio_capability(&self) -> AudioCapabilityState {
        self.audio_runtime.capability()
    }

    fn audio_catalog(&self) -> Arc<puzzle_audio::AudioAssetCatalog> {
        std::sync::Arc::clone(self.audio_runtime.catalog())
    }

    fn apply_audio_device_feedback(&mut self, feedback: AudioDeviceFeedback, now_seconds: f64) {
        let now_frame = audio_frame(now_seconds);
        match feedback {
            #[cfg(target_arch = "wasm32")]
            AudioDeviceFeedback::Capability(capability) => {
                self.pending_audio
                    .extend(self.audio_runtime.set_capability(capability, now_frame));
            }
            AudioDeviceFeedback::VoiceEnded(voice) => self.audio_runtime.voice_ended(voice),
            AudioDeviceFeedback::VoiceFailure { voice, error } => {
                self.audio_runtime
                    .report_voice_failure(voice, error, now_frame);
            }
            #[cfg(target_arch = "wasm32")]
            AudioDeviceFeedback::DeviceFailure(error) => {
                self.pending_audio
                    .extend(self.audio_runtime.report_device_failure(error, now_frame));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn audio_voice_ended(&mut self, voice: AudioVoiceId) {
        self.apply_audio_device_feedback(AudioDeviceFeedback::VoiceEnded(voice), 0.0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn audio_voice_failed(&mut self, voice: AudioVoiceId, error: String, now_seconds: f64) {
        self.apply_audio_device_feedback(
            AudioDeviceFeedback::VoiceFailure { voice, error },
            now_seconds,
        );
    }

    pub fn dispatch_action(
        &mut self,
        action: SessionAction,
        now_seconds: f64,
    ) -> Result<(), BevyPlayerError> {
        if let Some(error) = &self.fatal_error {
            return Err(BevyPlayerError::Runtime(error.clone()));
        }
        let busy_input_policy = self.snapshot.input_buffer.busy_input;
        let previously_queued_model_input = self.snapshot.queued_model_input;
        let snapshot = self
            .runtime
            .dispatch_typed(action)
            .map_err(BevyPlayerError::Runtime)?;
        let queued_model_input = !previously_queued_model_input && snapshot.queued_model_input;
        self.apply_snapshot_preserving_matching_presentation(snapshot, now_seconds)?;
        if queued_model_input {
            self.apply_queued_input_presentation_policy(busy_input_policy, now_seconds)?;
        }
        Ok(())
    }

    fn apply_queued_input_presentation_policy(
        &mut self,
        policy: RuntimeBusyInputPolicy,
        now_seconds: f64,
    ) -> Result<(), BevyPlayerError> {
        match policy {
            RuntimeBusyInputPolicy::Reject | RuntimeBusyInputPolicy::Queue => {}
            RuntimeBusyInputPolicy::Skip => {
                self.pending_presentation.clear();
                self.wait_until_seconds = None;
                self.wait_started_seconds = None;
                self.waiting_for_animation_publication = None;
                self.renderer_publication_animation_transition = None;
                self.queued_input_presentation = None;
                if let Some(token) = self.pending_presentation_continuation.take() {
                    let snapshot = self
                        .runtime
                        .dispatch_typed(SessionAction::PresentationComplete { token })
                        .map_err(BevyPlayerError::Runtime)?;
                    self.apply_snapshot(snapshot, now_seconds, false)?;
                }
            }
            RuntimeBusyInputPolicy::Accelerate { min_wait_ms } => {
                self.queued_input_presentation = Some(policy);
                if let (Some(started), Some(deadline)) =
                    (self.wait_started_seconds, self.wait_until_seconds)
                {
                    let accelerated_deadline = started + min_wait_ms as f64 / 1_000.0;
                    self.wait_until_seconds = Some(deadline.min(accelerated_deadline));
                }
            }
        }
        Ok(())
    }

    /// Atomically installs the typed editor draft and its model-scoped
    /// authoring presentation contract. Presentation validity is checked
    /// against the prospective state before the runtime is mutated.
    #[cfg(feature = "editor-debug")]
    pub fn hydrate_editor_draft_state(
        &mut self,
        model: &str,
        state: &RuntimeStateSnapshot,
        level_index: usize,
        presentation: EditorAuthoringPresentation,
        now_seconds: f64,
    ) -> Result<(), BevyPlayerError> {
        self.hydrate_editor_model_state(model, state, level_index, false, presentation, now_seconds)
    }

    /// Atomically installs a typed model state and its model-scoped editor
    /// presentation. The prospective renderer is fully validated before the
    /// runtime session commits the state.
    #[cfg(feature = "editor-debug")]
    pub fn hydrate_editor_model_state(
        &mut self,
        model: &str,
        state: &RuntimeStateSnapshot,
        level_index: usize,
        materialize_level_start: bool,
        presentation: EditorAuthoringPresentation,
        now_seconds: f64,
    ) -> Result<(), BevyPlayerError> {
        let renderer = presentation.renderer.clone();
        let requested_source = RuntimeViewportSourceId {
            model: model.to_string(),
            component: String::new(),
            source: String::new(),
        };
        let configuration =
            editor_authoring::EditorAuthoringConfiguration::new(presentation, requested_source);
        configuration
            .validate_for_state(state)
            .map_err(BevyPlayerError::EditorAuthoring)?;
        let preview = self
            .runtime
            .preview_editor_model_state(model, state, level_index, materialize_level_start)
            .map_err(BevyPlayerError::Runtime)?;
        let (configuration, viewports) = prepare_editor_authoring_projection(
            configuration,
            preview,
            &renderer,
            &self.snapshot.theme,
            now_seconds,
        )?;
        let snapshot = self
            .runtime
            .commit_editor_model_state(model, state, level_index, materialize_level_start)
            .map_err(BevyPlayerError::Runtime)?;
        self.install_editor_authoring_projection(snapshot, configuration, viewports, now_seconds);
        Ok(())
    }

    #[cfg(feature = "editor-debug")]
    pub fn editor_authoring_frame(&self) -> Option<&EditorAuthoringFrame> {
        self.editor_authoring_frame.as_ref()
    }

    #[cfg(feature = "editor-debug")]
    pub fn dispatch_editor_pointer(
        &mut self,
        surface_id: &str,
        committed_frame_revision: u64,
        point_css: Vec2,
        gesture: EditorPointerGesture,
    ) -> Result<(u64, Option<EditorAuthoringHitTarget>), BevyPlayerError> {
        let frame = self.editor_authoring_frame.as_ref().ok_or_else(|| {
            BevyPlayerError::EditorAuthoring(
                "editor pointer requires a committed authoring frame".to_string(),
            )
        })?;
        let frame_revision = frame.revision;
        let hit = frame
            .hit_for_gesture(surface_id, committed_frame_revision, point_css, gesture)
            .map_err(BevyPlayerError::EditorAuthoring)?;
        let configuration = self.editor_authoring.as_mut().ok_or_else(|| {
            BevyPlayerError::EditorAuthoring(
                "editor pointer requires an authoring configuration".to_string(),
            )
        })?;
        let viewport = self
            .viewports
            .get_mut(&configuration.viewport_source)
            .ok_or_else(|| {
                BevyPlayerError::EditorAuthoring(format!(
                    "editor authoring viewport {}.{} is no longer mounted",
                    configuration.viewport_source.component, configuration.viewport_source.source
                ))
            })?;
        configuration
            .set_highlight(&mut viewport.renderer, hit.clone(), &self.snapshot.theme)
            .map_err(BevyPlayerError::EditorAuthoring)?;
        viewport.needs_frame = true;
        Ok((frame_revision, hit))
    }

    /// Resolves an editor-forwarded key in the runtime owner. When tracing is
    /// enabled, only resolved model input collects a trace; scene, menu, and
    /// modal actions retain their ordinary player behavior.
    #[cfg(feature = "editor-debug")]
    pub fn dispatch_editor_key(
        &mut self,
        trigger: RuntimeKeyTrigger,
        trace_model_input: bool,
        now_seconds: f64,
    ) -> Result<Option<serde_json::Value>, BevyPlayerError> {
        let Some(configuration) = self.editor_authoring.clone() else {
            let dispatch = self
                .runtime
                .dispatch_editor_key(trigger, trace_model_input)
                .map_err(BevyPlayerError::Runtime)?;
            self.apply_snapshot(dispatch.snapshot, now_seconds, false)?;
            return Ok(dispatch.debug);
        };
        if !configuration.accepts_model_input() {
            return Ok(None);
        }
        let model = configuration.viewport_source.model.clone();
        let dispatch = self
            .runtime
            .dispatch_editor_model_key(&model, trigger, trace_model_input)
            .map_err(BevyPlayerError::Runtime)?;
        self.apply_snapshot(dispatch.snapshot, now_seconds, false)?;
        Ok(dispatch.debug)
    }

    #[cfg(feature = "editor-debug")]
    fn install_editor_authoring_projection(
        &mut self,
        snapshot: RuntimeSessionSnapshot,
        configuration: editor_authoring::EditorAuthoringConfiguration,
        viewports: BTreeMap<RuntimeViewportSourceId, HostViewport>,
        now_seconds: f64,
    ) {
        self.viewports = viewports;
        self.snapshot = snapshot;
        self.editor_authoring = Some(configuration);
        self.refresh_ui_projection();
        self.editor_authoring_frame = None;
        self.pending_presentation.clear();
        self.pending_presentation_continuation = None;
        self.wait_until_seconds = None;
        self.waiting_for_animation_publication = None;
        self.renderer_publication_animation_transition = None;
        self.animation_origins.clear();
        self.animation_origin_transition = None;
        for viewport in self.viewports.values_mut() {
            viewport.active_animation = None;
            viewport.animation_epoch_seconds = now_seconds;
            viewport.needs_frame = true;
        }
    }

    /// Returns editor inspection data paired with the exact player snapshot
    /// currently owned by this host. Development data is never an input to the
    /// renderer.
    #[cfg(feature = "editor-debug")]
    pub fn editor_development_snapshot(
        &self,
    ) -> puzzle_session_contract::RuntimeDevelopmentSessionSnapshot {
        self.runtime
            .development_snapshot_from_player(self.snapshot.clone())
    }

    pub fn restore_progress_save(
        &mut self,
        save_json: &str,
        now_seconds: f64,
    ) -> Result<(), BevyPlayerError> {
        self.runtime
            .restore_progress_save_json(save_json)
            .map_err(BevyPlayerError::Runtime)?;
        self.refresh_runtime_snapshot(now_seconds)
    }

    pub fn pending_progress_save(&self) -> Option<RuntimeProgressSaveRequest> {
        self.runtime.progress_save_request()
    }

    pub fn confirm_progress_persistence_applied(
        &mut self,
        request_id: u32,
        now_seconds: f64,
    ) -> Result<(), BevyPlayerError> {
        self.confirm_progress_persistence_applied_with(
            request_id,
            now_seconds,
            Self::refresh_runtime_snapshot_preserving_presentation,
        )
    }

    fn confirm_progress_persistence_applied_with(
        &mut self,
        request_id: u32,
        now_seconds: f64,
        refresh: impl FnOnce(&mut Self, f64) -> Result<(), BevyPlayerError>,
    ) -> Result<(), BevyPlayerError> {
        self.runtime
            .confirm_progress_persistence_applied(request_id)
            .map_err(BevyPlayerError::Runtime)?;
        // The runtime acknowledgement above is the commit point: the browser
        // adapter has already written this exact request. A subsequent Bevy
        // projection failure must stop the player, but must not be returned as
        // an acknowledgement failure that asks the adapter to retry a request
        // the runtime has consumed.
        if let Err(error) = refresh(self, now_seconds) {
            self.fail(error);
        }
        Ok(())
    }

    pub fn process_presentation(&mut self, now_seconds: f64) -> Result<(), BevyPlayerError> {
        if self.waiting_for_animation_publication.is_some() {
            return Ok(());
        }
        if self
            .wait_until_seconds
            .is_some_and(|deadline| now_seconds < deadline)
        {
            return Ok(());
        }
        self.wait_until_seconds = None;
        self.wait_started_seconds = None;
        while let Some(queued) = self.pending_presentation.pop_front() {
            match queued.event {
                RuntimePresentationEvent::AnimationBatch {
                    source, animations, ..
                } => {
                    let Some(viewport) = self.viewports.get_mut(&source) else {
                        return Err(BevyPlayerError::MissingPresentationTarget(source));
                    };
                    let transition = self
                        .snapshot
                        .presentation
                        .as_ref()
                        .map(|presentation| presentation.id)
                        .ok_or_else(|| {
                            BevyPlayerError::Runtime(
                                "animation batch requires an active transition".to_string(),
                            )
                        })?;
                    if self.animation_origin_transition != Some(transition) {
                        return Err(BevyPlayerError::Runtime(format!(
                            "animation transition {transition:?} has no matching committed origin"
                        )));
                    }
                    let origin = self.animation_origins.get(&source).ok_or_else(|| {
                        BevyPlayerError::Runtime(format!(
                            "animation source {}.{} has no committed origin",
                            source.component, source.source
                        ))
                    })?;
                    let from_scene = renderer_scene(origin);
                    let to_scene = renderer_scene(&viewport.renderer);
                    viewport.active_animation = Some(
                        puzzle_presentation::prepare_render_animation_channels(
                            from_scene,
                            to_scene,
                            &animations,
                        )
                        .map_err(|error| {
                            BevyPlayerError::Presentation {
                                source: source.clone(),
                                error: format!("{error:?}"),
                            }
                        })?,
                    );
                    viewport.animation_epoch_seconds = now_seconds;
                    viewport.needs_frame = true;
                }
                RuntimePresentationEvent::Wait { completion } => match completion {
                    RuntimePresentationWait::Duration { milliseconds } => {
                        let milliseconds = match self.queued_input_presentation {
                            Some(RuntimeBusyInputPolicy::Accelerate { min_wait_ms }) => {
                                milliseconds.min(min_wait_ms)
                            }
                            _ => milliseconds,
                        };
                        self.wait_started_seconds = Some(now_seconds);
                        self.wait_until_seconds = Some(now_seconds + milliseconds as f64 / 1_000.0);
                        return Ok(());
                    }
                    RuntimePresentationWait::AnimationPublication => {
                        let transition = self
                            .snapshot
                            .presentation
                            .as_ref()
                            .map(|presentation| presentation.id)
                            .ok_or_else(|| {
                                BevyPlayerError::Runtime(
                                    "animation publication wait requires an active transition"
                                        .to_string(),
                                )
                            })?;
                        self.waiting_for_animation_publication = Some(transition);
                        return Ok(());
                    }
                },
                RuntimePresentationEvent::Audio { command } => {
                    self.pending_audio
                        .extend(self.audio_runtime.apply(command, audio_frame(now_seconds)));
                }
            }
        }
        if let Some(token) = self.pending_presentation_continuation.take() {
            self.queued_input_presentation = None;
            let snapshot = self
                .runtime
                .dispatch_typed(SessionAction::PresentationComplete { token })
                .map_err(BevyPlayerError::Runtime)?;
            self.apply_snapshot(snapshot, now_seconds, false)?;
        }
        Ok(())
    }

    pub fn resolve_frames(
        &mut self,
        now_seconds: f64,
    ) -> Result<Vec<ResolvedBevyViewportFrame>, BevyPlayerError> {
        let milliseconds = |seconds: f64| {
            ((now_seconds - seconds).max(0.0) * 1_000.0)
                .floor()
                .min(u64::MAX as f64) as u64
        };
        let mut frames = Vec::new();
        for viewport in self.viewports.values_mut() {
            let scheduled_sample_due = viewport
                .next_sample
                .as_mut()
                .is_some_and(|sample| sample.consume_if_due(now_seconds));
            if !viewport.needs_frame && !scheduled_sample_due {
                continue;
            }
            let render_scene = match &viewport.renderer {
                RuntimeRendererState::TwoD(scene) => &scene.render_scene,
                RuntimeRendererState::ThreeD(scene) => &scene.render_scene,
            };
            let frame = puzzle_presentation::resolve_prepared_render_moment(
                render_scene,
                &self.visual_images,
                milliseconds(self.clip_epoch_seconds),
                milliseconds(viewport.animation_epoch_seconds),
                viewport.active_animation.as_ref(),
            )
            .map_err(|error| BevyPlayerError::Presentation {
                source: viewport.source.clone(),
                error: format!("{error:?}"),
            })?;
            viewport.needs_frame = false;
            viewport.next_sample = frame.next_sample.map(|sample| match sample {
                RuntimeResolvedNextSample::Deadline { after_milliseconds } => {
                    HostNextSample::Deadline {
                        at_seconds: now_seconds + after_milliseconds as f64 / 1_000.0,
                    }
                }
                RuntimeResolvedNextSample::DisplayRefresh {
                    completion_after_milliseconds,
                } => HostNextSample::DisplayRefresh {
                    completion_at_seconds: now_seconds
                        + completion_after_milliseconds as f64 / 1_000.0,
                    last_sample_seconds: now_seconds,
                },
            });
            frames.push(ResolvedBevyViewportFrame {
                source: viewport.source.clone(),
                renderer: viewport.renderer.clone(),
                frame,
            });
        }
        Ok(frames)
    }

    fn completed_animation_transition_at(
        &self,
        now_seconds: f64,
    ) -> Result<Option<RuntimePresentationTransitionId>, BevyPlayerError> {
        let Some(transition) = self.waiting_for_animation_publication else {
            return Ok(None);
        };
        if self.snapshot.presentation.as_ref().map(|active| active.id) != Some(transition) {
            return Err(BevyPlayerError::Runtime(format!(
                "animation publication wait references stale transition {transition:?}"
            )));
        }
        let mut found_animation = false;
        for viewport in self.viewports.values() {
            let Some(animation) = viewport.active_animation.as_ref() else {
                continue;
            };
            found_animation = true;
            let elapsed_milliseconds = ((now_seconds - viewport.animation_epoch_seconds).max(0.0)
                * 1_000.0)
                .floor()
                .min(u64::MAX as f64) as u64;
            if !puzzle_presentation::prepared_animation_is_complete(
                renderer_scene(&viewport.renderer),
                animation,
                elapsed_milliseconds,
            )
            .map_err(|error| BevyPlayerError::Presentation {
                source: viewport.source.clone(),
                error: format!("{error:?}"),
            })? {
                return Ok(None);
            }
        }
        if !found_animation {
            return Err(BevyPlayerError::Runtime(format!(
                "animation publication wait for transition {transition:?} has no prepared animation"
            )));
        }
        Ok(Some(transition))
    }

    fn acknowledge_animation_publication(
        &mut self,
        transition: RuntimePresentationTransitionId,
    ) -> Result<(), BevyPlayerError> {
        if self.waiting_for_animation_publication != Some(transition) {
            return Err(BevyPlayerError::Runtime(format!(
                "renderer acknowledged stale animation transition {transition:?}"
            )));
        }
        for viewport in self.viewports.values_mut() {
            viewport.active_animation = None;
        }
        self.waiting_for_animation_publication = None;
        self.animation_origins.clear();
        self.animation_origin_transition = None;
        Ok(())
    }

    fn refresh_runtime_snapshot(&mut self, now_seconds: f64) -> Result<(), BevyPlayerError> {
        let snapshot = self.runtime.snapshot();
        self.apply_snapshot(snapshot, now_seconds, false)
    }

    fn refresh_runtime_snapshot_preserving_presentation(
        &mut self,
        now_seconds: f64,
    ) -> Result<(), BevyPlayerError> {
        let snapshot = self.runtime.snapshot();
        self.apply_snapshot_preserving_matching_presentation(snapshot, now_seconds)
    }

    fn apply_snapshot_preserving_matching_presentation(
        &mut self,
        snapshot: RuntimeSessionSnapshot,
        now_seconds: f64,
    ) -> Result<(), BevyPlayerError> {
        let preserve_event_queue = self
            .snapshot
            .presentation
            .as_ref()
            .zip(snapshot.presentation.as_ref())
            .is_some_and(|(previous, next)| previous.id == next.id);
        self.apply_snapshot(snapshot, now_seconds, preserve_event_queue)
    }

    fn apply_snapshot(
        &mut self,
        snapshot: RuntimeSessionSnapshot,
        now_seconds: f64,
        preserve_event_queue: bool,
    ) -> Result<(), BevyPlayerError> {
        let mut next = projected_viewports(&snapshot, now_seconds)?;
        for (source, viewport) in &mut next {
            if let Some(previous) = self.viewports.get(source) {
                viewport.active_animation = previous.active_animation.clone();
                viewport.animation_epoch_seconds = previous.animation_epoch_seconds;
                if viewport.renderer == previous.renderer {
                    viewport.next_sample = previous.next_sample;
                    viewport.needs_frame = previous.needs_frame;
                }
            }
        }
        #[cfg(feature = "editor-debug")]
        if let Some(configuration) = self.editor_authoring.clone() {
            let model = configuration.viewport_source.model.clone();
            let renderer = configuration.renderer.clone();
            let projection = self
                .runtime
                .current_editor_model_projection(&model)
                .map_err(BevyPlayerError::Runtime)?;
            let (configuration, authoring_viewports) = prepare_editor_authoring_projection(
                configuration,
                projection,
                &renderer,
                &snapshot.theme,
                now_seconds,
            )?;
            next = authoring_viewports;
            self.editor_authoring = Some(configuration);
            self.editor_authoring_frame = None;
        }
        if !preserve_event_queue {
            self.capture_animation_origins(&snapshot)?;
        }
        self.viewports = next;
        if preserve_event_queue {
            self.pending_presentation_continuation = snapshot
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.continuation.clone());
        }
        self.snapshot = snapshot;
        self.refresh_ui_projection();
        if !preserve_event_queue {
            for viewport in self.viewports.values_mut() {
                viewport.active_animation = None;
                viewport.animation_epoch_seconds = now_seconds;
            }
            self.replace_presentation_events();
        }
        Ok(())
    }

    fn replace_presentation_events(&mut self) {
        self.pending_presentation = self
            .snapshot
            .presentation
            .as_ref()
            .into_iter()
            .flat_map(|presentation| presentation.steps.iter())
            .map(|event| QueuedPresentationEvent {
                event: event.clone(),
            })
            .collect();
        self.pending_presentation_continuation = self
            .snapshot
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.continuation.clone());
        self.wait_until_seconds = None;
        self.wait_started_seconds = None;
        self.waiting_for_animation_publication = None;
        self.renderer_publication_animation_transition = None;
        self.queued_input_presentation = None;
    }

    fn capture_animation_origins(
        &mut self,
        snapshot: &RuntimeSessionSnapshot,
    ) -> Result<(), BevyPlayerError> {
        self.animation_origins.clear();
        self.animation_origin_transition = None;
        let Some(transition) = snapshot.presentation.as_ref() else {
            return Ok(());
        };
        if self.snapshot.state_commit != transition.from_state_commit
            || snapshot.state_commit != transition.to_state_commit
        {
            return Err(BevyPlayerError::Runtime(format!(
                "presentation transition {:?} declares state commits {:?} -> {:?}, but the player owns {:?} -> {:?}",
                transition.id,
                transition.from_state_commit,
                transition.to_state_commit,
                self.snapshot.state_commit,
                snapshot.state_commit,
            )));
        }
        self.animation_origins.extend(
            self.viewports
                .iter()
                .map(|(source, viewport)| (source.clone(), viewport.renderer.clone())),
        );
        self.animation_origin_transition = Some(transition.id);
        Ok(())
    }

    fn refresh_ui_projection(&mut self) {
        let next = UiProjectionIdentity::from_host(self);
        if self.ui_projection == next {
            return;
        }
        self.ui_projection = next;
        self.ui_projection_generation = self
            .ui_projection_generation
            .checked_add(1)
            .expect("Bevy UI projection generation must not overflow");
    }

    fn fail(&mut self, error: BevyPlayerError) {
        let message = error.to_string();
        error!("{message}");
        self.fatal_error = Some(message);
        for viewport in self.viewports.values_mut() {
            viewport.next_sample = None;
            viewport.needs_frame = false;
        }
    }
}

fn renderer_scene(
    renderer: &RuntimeRendererState,
) -> &puzzle_runtime_contract::RuntimeResolvedRenderScene {
    match renderer {
        RuntimeRendererState::TwoD(scene) => &scene.render_scene,
        RuntimeRendererState::ThreeD(scene) => &scene.render_scene,
    }
}

fn projected_viewports(
    snapshot: &RuntimeSessionSnapshot,
    now_seconds: f64,
) -> Result<BTreeMap<RuntimeViewportSourceId, HostViewport>, BevyPlayerError> {
    for component in &snapshot.surface.components {
        match &component.presentation {
            RuntimeComponentPresentation::Ready(scene) => {
                if let Some(event_name) = &component.await_event {
                    if scene
                        .events
                        .as_ref()
                        .and_then(|events| events.get(event_name))
                        .is_none()
                    {
                        return Err(BevyPlayerError::MissingAwaitedEvent {
                            component: component.id.clone(),
                            event: event_name.clone(),
                        });
                    }
                }
            }
            RuntimeComponentPresentation::Error { error } => {
                return Err(BevyPlayerError::ComponentPresentation {
                    component: component.id.clone(),
                    error: error.clone(),
                });
            }
        }
    }
    let mut referenced = Vec::new();
    let mut seen = BTreeSet::new();
    for stack in PuzzleBevyUiSurfaceStack::ORDERED {
        for component in &snapshot.surface.components {
            if component.visibility != puzzle_scene::ComponentVisibility::Visible
                || ui_surface_stack(component.placement, component.modal) != stack
            {
                continue;
            }
            let RuntimeComponentPresentation::Ready(scene) = &component.presentation else {
                continue;
            };
            collect_viewport_sources(&scene.components, &mut referenced, &mut seen)?;
        }
    }
    let viewport_count = referenced.len();
    let viewports = referenced
        .into_iter()
        .enumerate()
        .map(
            |(index, (source, dimension))| -> Result<_, BevyPlayerError> {
                let order =
                    isize::try_from(index).map_err(|_| BevyPlayerError::ViewportOrderOverflow {
                        count: viewport_count,
                    })?;
                let renderer = snapshot
                    .viewport_sources
                    .get(&source)
                    .cloned()
                    .ok_or_else(|| BevyPlayerError::MissingViewportSource(source.clone()))?;
                let matches_dimension = matches!(
                    (dimension, &renderer),
                    (
                        puzzle_session_contract::RuntimeViewportDimension::TwoD,
                        RuntimeRendererState::TwoD(_)
                    ) | (
                        puzzle_session_contract::RuntimeViewportDimension::ThreeD,
                        RuntimeRendererState::ThreeD(_)
                    )
                );
                if !matches_dimension {
                    return Err(BevyPlayerError::ViewportDimensionMismatch {
                        source,
                        surface: dimension,
                        renderer: match renderer {
                            RuntimeRendererState::TwoD(_) => "2D",
                            RuntimeRendererState::ThreeD(_) => "3D",
                        },
                    });
                }
                validate_renderer_source(&source, &renderer)?;
                Ok((
                    source.clone(),
                    HostViewport {
                        source,
                        order,
                        renderer,
                        active_animation: None,
                        animation_epoch_seconds: now_seconds,
                        next_sample: None,
                        needs_frame: true,
                    },
                ))
            },
        )
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(viewports)
}

#[cfg(feature = "editor-debug")]
fn prepare_editor_authoring_projection(
    mut configuration: editor_authoring::EditorAuthoringConfiguration,
    projection: RuntimeEditorModelProjection,
    strategy: &EditorRendererStrategy,
    theme: &RuntimeTheme,
    now_seconds: f64,
) -> Result<
    (
        editor_authoring::EditorAuthoringConfiguration,
        BTreeMap<RuntimeViewportSourceId, HostViewport>,
    ),
    BevyPlayerError,
> {
    if projection.source.model != configuration.viewport_source.model {
        return Err(BevyPlayerError::EditorAuthoring(format!(
            "editor authoring requested model {:?}, but runtime projected {:?}",
            configuration.viewport_source.model, projection.source.model
        )));
    }
    if !matches!(
        (strategy, &projection.renderer),
        (
            EditorRendererStrategy::Grid2d,
            RuntimeRendererState::TwoD(_)
        ) | (
            EditorRendererStrategy::Grid3d { .. },
            RuntimeRendererState::ThreeD(_)
        )
    ) {
        return Err(BevyPlayerError::EditorAuthoring(
            "editor authoring renderer strategy does not match the projected model".to_string(),
        ));
    }
    configuration
        .validate_for_solver_state(&projection.solver_state)
        .map_err(BevyPlayerError::EditorAuthoring)?;
    validate_renderer_source(&projection.source, &projection.renderer)?;
    configuration.viewport_source = projection.source.clone();
    let mut renderer = projection.renderer;
    configuration
        .apply_to_renderer(&mut renderer, theme)
        .map_err(BevyPlayerError::EditorAuthoring)?;
    let viewport = HostViewport {
        source: projection.source.clone(),
        order: 0,
        renderer,
        active_animation: None,
        animation_epoch_seconds: now_seconds,
        next_sample: None,
        needs_frame: true,
    };
    Ok((
        configuration,
        BTreeMap::from([(projection.source, viewport)]),
    ))
}

fn collect_viewport_sources(
    components: &[RuntimeResolvedSceneComponent],
    sources: &mut Vec<(
        RuntimeViewportSourceId,
        puzzle_session_contract::RuntimeViewportDimension,
    )>,
    seen: &mut BTreeSet<RuntimeViewportSourceId>,
) -> Result<(), BevyPlayerError> {
    for component in components {
        match component {
            RuntimeResolvedSceneComponent::Viewport {
                source, dimension, ..
            } => {
                if !seen.insert(source.clone()) {
                    return Err(BevyPlayerError::DuplicateViewportLeaf(source.clone()));
                }
                sources.push((source.clone(), *dimension));
            }
            RuntimeResolvedSceneComponent::Row { children, .. }
            | RuntimeResolvedSceneComponent::Column { children, .. }
            | RuntimeResolvedSceneComponent::Box { children, .. } => {
                collect_viewport_sources(children, sources, seen)?;
            }
            RuntimeResolvedSceneComponent::Frame { kind, source, .. } => {
                return Err(BevyPlayerError::UnsupportedFrameComponent {
                    kind: kind.clone(),
                    source: source.clone(),
                });
            }
            RuntimeResolvedSceneComponent::Text { .. }
            | RuntimeResolvedSceneComponent::Button { .. }
            | RuntimeResolvedSceneComponent::Choice { .. } => {}
        }
    }
    Ok(())
}

fn validate_renderer_source(
    source: &RuntimeViewportSourceId,
    renderer: &RuntimeRendererState,
) -> Result<(), BevyPlayerError> {
    if let RuntimeRendererState::TwoD(scene) = renderer {
        if let Some(error) = &scene.display_error {
            return Err(BevyPlayerError::TwoDimensionalDisplay {
                source: source.clone(),
                error: error.clone(),
            });
        }
    }
    Ok(())
}

#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PuzzleBevyUiIdentity {
    pub component: String,
    pub tree_path: Vec<usize>,
}

#[derive(Component, Clone)]
struct PuzzleBevyUiAction(RuntimeSceneActionToken);

#[derive(Component, Clone)]
struct PuzzleBevyUiViewport(RuntimeViewportSourceId);

#[derive(Component)]
struct PuzzleBevyUiSelectedChoice;

#[derive(Component)]
struct PuzzleBevyUiRoot;

#[derive(Component)]
struct PuzzleBevyBackgroundCamera;

#[derive(Component)]
struct PuzzleBevyUiCamera;

#[derive(Component)]
struct PuzzleBevyUiRootLayer;

#[derive(Component)]
struct PuzzleBevyUiContentLayer;

#[derive(Component)]
struct PuzzleBevyUiOverlayLayer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PuzzleBevyUiSurfaceLayer {
    Root,
    Content,
    Overlay,
}

impl PuzzleBevyUiSurfaceLayer {
    fn z_index(self) -> i32 {
        match self {
            Self::Root => 0,
            Self::Content => 10,
            Self::Overlay => 20,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PuzzleBevyUiSurfaceStack {
    Root,
    Content,
    Overlay,
    Modal,
}

impl PuzzleBevyUiSurfaceStack {
    const ORDERED: [Self; 4] = [Self::Root, Self::Content, Self::Overlay, Self::Modal];

    fn layer(self) -> PuzzleBevyUiSurfaceLayer {
        match self {
            Self::Root => PuzzleBevyUiSurfaceLayer::Root,
            Self::Content => PuzzleBevyUiSurfaceLayer::Content,
            Self::Overlay | Self::Modal => PuzzleBevyUiSurfaceLayer::Overlay,
        }
    }

    fn root_z_index(self) -> i32 {
        match self {
            Self::Modal => 100,
            Self::Root | Self::Content | Self::Overlay => 0,
        }
    }
}

fn ui_surface_stack(
    placement: puzzle_scene::ComponentPlacement,
    modal: bool,
) -> PuzzleBevyUiSurfaceStack {
    if modal {
        return PuzzleBevyUiSurfaceStack::Modal;
    }
    match placement {
        puzzle_scene::ComponentPlacement::Root => PuzzleBevyUiSurfaceStack::Root,
        puzzle_scene::ComponentPlacement::Content => PuzzleBevyUiSurfaceStack::Content,
        puzzle_scene::ComponentPlacement::Overlay => PuzzleBevyUiSurfaceStack::Overlay,
    }
}

#[derive(Resource, Default)]
struct SubmittedViewportIds(BTreeSet<PuzzleBevyViewId>);

#[derive(Resource, Default)]
struct SubmittedViewportRects(BTreeMap<RuntimeViewportSourceId, PuzzleBevyFramebufferRect>);

#[derive(Clone, Debug, PartialEq)]
enum UiProjectionIdentity {
    Player {
        theme: RuntimeTheme,
        surface: RuntimeSurface,
    },
    #[cfg(feature = "editor-debug")]
    EditorAuthoring {
        background: RuntimeLinearRgba,
        surface_id: String,
        viewport_source: RuntimeViewportSourceId,
    },
}

impl UiProjectionIdentity {
    fn player(snapshot: &RuntimeSessionSnapshot) -> Self {
        Self::Player {
            theme: snapshot.theme,
            surface: snapshot.surface.clone(),
        }
    }

    fn from_host(host: &PuzzleBevyPlayerHost) -> Self {
        #[cfg(feature = "editor-debug")]
        if let Some(authoring) = host.editor_authoring.as_ref() {
            return Self::EditorAuthoring {
                background: host.snapshot.theme.background,
                surface_id: authoring.surface.surface_id.clone(),
                viewport_source: authoring.viewport_source.clone(),
            };
        }
        Self::player(&host.snapshot)
    }
}

#[derive(Resource, Default)]
struct RenderedUiProjection(Option<u64>);

#[derive(Clone, Copy, Debug, PartialEq)]
struct InteractiveCameraOffset {
    yaw_degrees: f32,
    pitch_degrees: f32,
    zoom_factor: f32,
}

impl Default for InteractiveCameraOffset {
    fn default() -> Self {
        Self {
            yaw_degrees: 0.0,
            pitch_degrees: 0.0,
            zoom_factor: 1.0,
        }
    }
}

#[derive(Resource, Default)]
struct InteractiveCameraState {
    active_look: Option<RuntimeViewportSourceId>,
    offsets: BTreeMap<RuntimeViewportSourceId, InteractiveCameraOffset>,
}

#[derive(Resource, Default)]
struct TouchScrollTargets(HashMap<u64, Entity>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PuzzleBevyPlayerObservation {
    pub sequence: u64,
    pub submission_sequence: u64,
    pub revision: u64,
    pub surface_focus: String,
    pub viewport_count: usize,
    pub submission_interval_micros: u64,
    pub presentation_cpu_micros: Option<u64>,
    pub wasm_linear_memory_bytes: Option<u64>,
    pub progress_fingerprint: u64,
    pub audio_capability: AudioCapabilityState,
}

impl PuzzleBevyPlayerObservation {
    pub fn audio_capability_label(&self) -> &'static str {
        match self.audio_capability {
            AudioCapabilityState::Locked => "locked",
            AudioCapabilityState::Ready => "ready",
            AudioCapabilityState::Suspended => "suspended",
            AudioCapabilityState::Unavailable => "unavailable",
        }
    }
}

#[derive(Resource, Default)]
pub struct PuzzleBevyPlayerObservationState {
    latest: Option<PuzzleBevyPlayerObservation>,
}

#[derive(Resource)]
struct PendingPuzzleBevyPlayerObservation {
    revision: u64,
    surface_focus: String,
    viewport_count: usize,
    submission_interval_micros: u64,
    presentation_started_micros: Option<u64>,
    progress_fingerprint: u64,
    audio_capability: AudioCapabilityState,
    present: bool,
}

impl Default for PendingPuzzleBevyPlayerObservation {
    fn default() -> Self {
        Self {
            revision: 0,
            surface_focus: String::new(),
            viewport_count: 0,
            submission_interval_micros: 0,
            presentation_started_micros: None,
            progress_fingerprint: 0,
            audio_capability: initial_audio_capability(),
            present: false,
        }
    }
}

impl PuzzleBevyPlayerObservationState {
    pub fn latest(&self) -> Option<&PuzzleBevyPlayerObservation> {
        self.latest.as_ref()
    }

    fn record_submission(
        &mut self,
        revision: u64,
        surface_focus: &str,
        viewport_count: usize,
        submission_interval_micros: u64,
        presentation_cpu_micros: Option<u64>,
        wasm_linear_memory_bytes: Option<u64>,
        progress_fingerprint: u64,
        audio_capability: AudioCapabilityState,
    ) {
        let changed = self.latest.as_ref().is_none_or(|latest| {
            latest.revision != revision
                || latest.surface_focus != surface_focus
                || latest.viewport_count != viewport_count
                || latest.progress_fingerprint != progress_fingerprint
        });
        let sequence = if changed {
            self.latest
                .as_ref()
                .map_or(1, |latest| latest.sequence.saturating_add(1))
        } else {
            self.latest.as_ref().map_or(1, |latest| latest.sequence)
        };
        let submission_sequence = self
            .latest
            .as_ref()
            .map_or(1, |latest| latest.submission_sequence.saturating_add(1));
        self.latest = Some(PuzzleBevyPlayerObservation {
            sequence,
            submission_sequence,
            revision,
            surface_focus: surface_focus.to_string(),
            viewport_count,
            submission_interval_micros,
            presentation_cpu_micros,
            wasm_linear_memory_bytes,
            progress_fingerprint,
            audio_capability,
        });
    }
}

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PuzzleBevyPlayerSystems {
    SubmitResolvedFrames,
    CommitObservation,
    ObservationReady,
}

pub struct PuzzleBevyPlayerPlugin;

impl Plugin for PuzzleBevyPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SubmittedViewportIds>()
            .init_resource::<SubmittedViewportRects>()
            .init_resource::<RenderedUiProjection>()
            .init_resource::<InteractiveCameraState>()
            .init_resource::<TouchScrollTargets>()
            .init_resource::<PuzzleBevyPlayerObservationState>()
            .init_resource::<PendingPuzzleBevyPlayerObservation>()
            .configure_sets(
                PostUpdate,
                (
                    PuzzleBevyPlayerSystems::CommitObservation
                        .after(PuzzleBevyRendererSystems::ApplySubmittedFrames),
                    PuzzleBevyPlayerSystems::ObservationReady
                        .after(PuzzleBevyPlayerSystems::CommitObservation),
                ),
            )
            .add_systems(Startup, spawn_ui_root)
            .add_systems(
                Update,
                (
                    dispatch_pointer_actions,
                    update_interactive_camera,
                    dispatch_keyboard_input,
                    advance_presentation,
                    sync_ui_scale,
                    sync_resolved_ui,
                    update_ui_scroll,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (
                    scroll_selected_choice_into_view
                        .after(bevy::ui::UiSystems::Layout)
                        .before(PuzzleBevyPlayerSystems::SubmitResolvedFrames),
                    submit_resolved_frames
                        .in_set(PuzzleBevyPlayerSystems::SubmitResolvedFrames)
                        .after(bevy::ui::UiSystems::Layout)
                        .before(PuzzleBevyRendererSystems::ApplySubmittedFrames),
                    commit_player_observation.in_set(PuzzleBevyPlayerSystems::CommitObservation),
                ),
            );
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(
            Update,
            audio::process_native_audio.after(advance_presentation),
        );
    }
}

/// Builds the browser player shell shared by standalone exports and editor
/// previews. Host-specific plugins are added by each caller after this returns;
/// editor capabilities therefore remain absent from standalone builds.
#[cfg(target_arch = "wasm32")]
pub fn build_browser_player_app(
    host: PuzzleBevyPlayerHost,
    canvas_selector: &str,
    title: &str,
) -> Result<App, String> {
    if canvas_selector.trim().is_empty() {
        return Err("Bevy player canvas selector must not be empty".to_string());
    }
    let window = web_sys::window()
        .ok_or_else(|| "Bevy player canvas validation requires a browser window".to_string())?;
    let document = window
        .document()
        .ok_or_else(|| "Bevy player canvas validation requires a browser document".to_string())?;
    let element = document
        .query_selector(canvas_selector)
        .map_err(|error| {
            format!("Bevy player canvas selector `{canvas_selector}` is invalid: {error:?}")
        })?
        .ok_or_else(|| {
            format!("Bevy player canvas selector `{canvas_selector}` matched no element")
        })?;
    if !element.is_instance_of::<web_sys::HtmlCanvasElement>() {
        return Err(format!(
            "Bevy player canvas selector `{canvas_selector}` must match a canvas element"
        ));
    }

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(bevy::window::WindowPlugin {
        primary_window: Some(Window {
            title: title.to_string(),
            canvas: Some(canvas_selector.to_string()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }));
    install_puzzle_bevy_player(&mut app, host);
    Ok(app)
}

pub fn install_puzzle_bevy_player(app: &mut App, host: PuzzleBevyPlayerHost) -> &mut App {
    #[cfg(not(target_arch = "wasm32"))]
    audio::install_native_audio_backend(app);
    #[cfg(target_arch = "wasm32")]
    let web_audio_catalog = host.audio_catalog();
    app.add_plugins(PuzzleBevy2dPlugin::default())
        .add_plugins(PuzzleBevy3dPlugin)
        .add_plugins(PuzzleBevyPlayerPlugin)
        .insert_non_send(host);
    #[cfg(target_arch = "wasm32")]
    app.add_plugins(web_audio::PuzzleBevyWebAudioPlugin::new(web_audio_catalog));
    app
}

fn uniform_ui_scale(window_size: Vec2, reference_size: Vec2) -> Option<f32> {
    if !window_size.is_finite()
        || !reference_size.is_finite()
        || window_size.x <= 0.0
        || window_size.y <= 0.0
        || reference_size.x <= 0.0
        || reference_size.y <= 0.0
    {
        return None;
    }
    let scale = (window_size.x / reference_size.x).min(window_size.y / reference_size.y);
    scale.is_finite().then_some(scale)
}

fn sync_ui_scale(
    primary_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    host: NonSend<PuzzleBevyPlayerHost>,
    mut ui_scale: ResMut<UiScale>,
) {
    let Ok(window) = primary_window.single() else {
        return;
    };
    let reference = host.snapshot.theme.ui_reference_size;
    let Some(scale) = uniform_ui_scale(
        Vec2::new(window.width(), window.height()),
        Vec2::new(reference.width_px, reference.height_px),
    ) else {
        return;
    };
    if (ui_scale.0 - scale).abs() > f32::EPSILON {
        ui_scale.0 = scale;
    }
}

fn spawn_ui_root(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        RenderLayers::none(),
        PuzzleBevyBackgroundCamera,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: isize::MAX,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::none(),
        IsDefaultUiCamera,
        PuzzleBevyUiCamera,
    ));
    let root = commands
        .spawn((
            PuzzleBevyUiRoot,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Relative,
                ..default()
            },
        ))
        .id();
    let layer_node = || Node {
        position_type: PositionType::Absolute,
        left: px(0),
        right: px(0),
        top: px(0),
        bottom: px(0),
        flex_direction: FlexDirection::Column,
        ..default()
    };
    let root_layer = commands
        .spawn((
            PuzzleBevyUiRootLayer,
            layer_node(),
            ZIndex(PuzzleBevyUiSurfaceLayer::Root.z_index()),
        ))
        .id();
    let content_layer = commands
        .spawn((
            PuzzleBevyUiContentLayer,
            layer_node(),
            ZIndex(PuzzleBevyUiSurfaceLayer::Content.z_index()),
        ))
        .id();
    let overlay_layer = commands
        .spawn((
            PuzzleBevyUiOverlayLayer,
            layer_node(),
            ZIndex(PuzzleBevyUiSurfaceLayer::Overlay.z_index()),
        ))
        .id();
    commands
        .entity(root)
        .add_children(&[root_layer, content_layer, overlay_layer]);
}

fn sync_resolved_ui(
    mut commands: Commands,
    host: NonSend<PuzzleBevyPlayerHost>,
    mut rendered: ResMut<RenderedUiProjection>,
    roots: Query<Entity, With<PuzzleBevyUiRoot>>,
    root_layers: Query<Entity, With<PuzzleBevyUiRootLayer>>,
    content_layers: Query<Entity, With<PuzzleBevyUiContentLayer>>,
    overlay_layers: Query<Entity, With<PuzzleBevyUiOverlayLayer>>,
    mut background_cameras: Query<&mut Camera, With<PuzzleBevyBackgroundCamera>>,
    existing: Query<(Entity, &PuzzleBevyUiIdentity)>,
) {
    let generation = host.ui_projection_generation;
    if rendered.0 == Some(generation) {
        return;
    }
    if roots.single().is_err() {
        return;
    }
    let (Ok(root_layer), Ok(content_layer), Ok(overlay_layer)) = (
        root_layers.single(),
        content_layers.single(),
        overlay_layers.single(),
    ) else {
        return;
    };
    let Ok(mut background_camera) = background_cameras.single_mut() else {
        return;
    };
    rendered.0 = Some(generation);
    background_camera.clear_color =
        ClearColorConfig::Custom(bevy_color(host.snapshot.theme.background));
    let old = existing
        .iter()
        .map(|(entity, id)| (id.clone(), entity))
        .collect::<HashMap<_, _>>();
    let mut retained = BTreeSet::new();
    #[cfg(feature = "editor-debug")]
    if let Some(authoring) = host.editor_authoring.as_ref() {
        let root_id = PuzzleBevyUiIdentity {
            component: authoring.surface.surface_id.clone(),
            tree_path: Vec::new(),
        };
        let viewport_id = PuzzleBevyUiIdentity {
            component: authoring.surface.surface_id.clone(),
            tree_path: vec![0],
        };
        let authoring_root = old
            .get(&root_id)
            .copied()
            .unwrap_or_else(|| commands.spawn_empty().id());
        let authoring_viewport = old
            .get(&viewport_id)
            .copied()
            .unwrap_or_else(|| commands.spawn_empty().id());
        retained.insert(root_id.clone());
        retained.insert(viewport_id.clone());
        commands.entity(authoring_root).insert((
            root_id,
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            ZIndex(PuzzleBevyUiSurfaceLayer::Content.z_index()),
        ));
        commands
            .entity(authoring_root)
            .remove::<(PuzzleBevyUiAction, Button, BackgroundColor)>()
            .replace_children(&[authoring_viewport]);
        commands.entity(authoring_viewport).insert((
            viewport_id,
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            PuzzleBevyUiViewport(authoring.viewport_source.clone()),
        ));
        commands.entity(root_layer).replace_children(&[]);
        commands
            .entity(content_layer)
            .replace_children(&[authoring_root]);
        commands.entity(overlay_layer).replace_children(&[]);
        for entity in removed_ui_subtree_roots(&old, &retained) {
            commands.entity(entity).despawn();
        }
        return;
    }
    let mut root_components = Vec::new();
    let mut content_components = Vec::new();
    let mut overlay_components = Vec::new();
    for surface_component in &host.snapshot.surface.components {
        if surface_component.visibility != puzzle_scene::ComponentVisibility::Visible {
            continue;
        }
        let RuntimeComponentPresentation::Ready(scene) = &surface_component.presentation else {
            continue;
        };
        let focused = host.snapshot.surface.focus == surface_component.id;
        let scene_root_id = PuzzleBevyUiIdentity {
            component: surface_component.id.clone(),
            tree_path: Vec::new(),
        };
        let scene_root = old
            .get(&scene_root_id)
            .copied()
            .unwrap_or_else(|| commands.spawn_empty().id());
        retained.insert(scene_root_id.clone());
        let mut root_node = node_from_layout(&scene.layout, Some(FlexDirection::Column));
        root_node.width = percent(100);
        root_node.height = percent(100);
        if matches!(
            surface_component.placement,
            puzzle_scene::ComponentPlacement::Overlay
        ) || surface_component.modal
        {
            root_node.position_type = PositionType::Absolute;
            root_node.left = px(0);
            root_node.right = px(0);
            root_node.top = px(0);
            root_node.bottom = px(0);
        }
        let surface_stack = ui_surface_stack(surface_component.placement, surface_component.modal);
        commands.entity(scene_root).insert((
            scene_root_id,
            root_node,
            ZIndex(surface_stack.root_z_index()),
        ));
        commands
            .entity(scene_root)
            .remove::<(PuzzleBevyUiAction, Button, BackgroundColor)>();
        if let Some(action) = awaited_pointer_action(surface_component, scene) {
            commands
                .entity(scene_root)
                .insert((Button, PuzzleBevyUiAction(action.clone())));
        }
        let mut scene_children = Vec::new();
        for (index, component) in scene.components.iter().enumerate() {
            let id = PuzzleBevyUiIdentity {
                component: surface_component.id.clone(),
                tree_path: vec![index],
            };
            scene_children.push(sync_ui_component(
                &mut commands,
                &old,
                &mut retained,
                id,
                component,
                focused,
                &host.snapshot.theme,
            ));
        }
        if surface_component.modal {
            commands
                .entity(scene_root)
                .insert(BackgroundColor(bevy_color(host.snapshot.theme.background)));
            let panel_id = PuzzleBevyUiIdentity {
                component: surface_component.id.clone(),
                tree_path: vec![usize::MAX],
            };
            let panel = old
                .get(&panel_id)
                .copied()
                .unwrap_or_else(|| commands.spawn_empty().id());
            retained.insert(panel_id.clone());
            commands.entity(panel).insert((
                panel_id,
                Node {
                    width: percent(100),
                    max_width: px(680),
                    padding: UiRect::all(px(24)),
                    flex_direction: FlexDirection::Column,
                    align_self: AlignSelf::Center,
                    ..default()
                },
                BackgroundColor(bevy_color(host.snapshot.theme.panel)),
            ));
            commands.entity(panel).replace_children(&scene_children);
            commands.entity(scene_root).replace_children(&[panel]);
        } else {
            commands
                .entity(scene_root)
                .replace_children(&scene_children);
        }
        match surface_stack.layer() {
            PuzzleBevyUiSurfaceLayer::Root => root_components.push(scene_root),
            PuzzleBevyUiSurfaceLayer::Content => content_components.push(scene_root),
            PuzzleBevyUiSurfaceLayer::Overlay => overlay_components.push(scene_root),
        }
    }
    commands
        .entity(root_layer)
        .replace_children(&root_components);
    commands
        .entity(content_layer)
        .replace_children(&content_components);
    commands
        .entity(overlay_layer)
        .replace_children(&overlay_components);
    for entity in removed_ui_subtree_roots(&old, &retained) {
        commands.entity(entity).despawn();
    }
}

fn removed_ui_subtree_roots(
    existing: &HashMap<PuzzleBevyUiIdentity, Entity>,
    retained: &BTreeSet<PuzzleBevyUiIdentity>,
) -> Vec<Entity> {
    existing
        .iter()
        .filter(|(identity, _)| {
            if retained.contains(*identity) {
                return false;
            }
            let mut ancestor = (*identity).clone();
            while ancestor.tree_path.pop().is_some() {
                if existing.contains_key(&ancestor) && !retained.contains(&ancestor) {
                    return false;
                }
            }
            true
        })
        .map(|(_, entity)| *entity)
        .collect()
}

fn awaited_pointer_action<'a>(
    component: &'a puzzle_session_contract::RuntimeSurfaceComponent,
    scene: &'a puzzle_session_contract::RuntimeResolvedScene,
) -> Option<&'a RuntimeSceneActionToken> {
    let event_name = component.await_event.as_ref()?;
    let binding = scene.events.as_ref()?.get(event_name)?;
    binding.pointer.then_some(())?;
    binding.action.as_ref()
}

fn sync_ui_component(
    commands: &mut Commands,
    existing: &HashMap<PuzzleBevyUiIdentity, Entity>,
    retained: &mut BTreeSet<PuzzleBevyUiIdentity>,
    id: PuzzleBevyUiIdentity,
    component: &RuntimeResolvedSceneComponent,
    focused: bool,
    theme: &RuntimeTheme,
) -> Entity {
    let entity = existing
        .get(&id)
        .copied()
        .unwrap_or_else(|| commands.spawn_empty().id());
    retained.insert(id.clone());
    let mut entity_commands = commands.entity(entity);
    entity_commands.insert(id.clone());
    entity_commands.replace_children(&[]);
    entity_commands.remove::<(
        PuzzleBevyUiAction,
        PuzzleBevyUiViewport,
        Button,
        Text,
        TextFont,
        TextColor,
        TextLayout,
        LineHeight,
        BorderColor,
        BackgroundColor,
        PuzzleBevyUiSelectedChoice,
    )>();
    let layout = match component {
        RuntimeResolvedSceneComponent::Viewport { layout, .. }
        | RuntimeResolvedSceneComponent::Frame { layout, .. }
        | RuntimeResolvedSceneComponent::Text { layout, .. }
        | RuntimeResolvedSceneComponent::Button { layout, .. }
        | RuntimeResolvedSceneComponent::Choice { layout, .. }
        | RuntimeResolvedSceneComponent::Row { layout, .. }
        | RuntimeResolvedSceneComponent::Column { layout, .. }
        | RuntimeResolvedSceneComponent::Box { layout, .. } => layout,
    };
    if layout.scroll {
        entity_commands
            .entry::<ScrollPosition>()
            .or_insert(ScrollPosition::default());
    } else {
        entity_commands.remove::<ScrollPosition>();
    }
    match component {
        RuntimeResolvedSceneComponent::Text {
            role,
            value,
            text_align,
            layout,
        } => {
            let style = text_style(theme, *role);
            entity_commands.insert((
                node_from_layout(layout, None),
                Text::new(value.clone()),
                TextFont {
                    font_size: FontSize::Px(style.font_size_px),
                    ..default()
                },
                TextColor(if focused {
                    bevy_color(theme.text)
                } else {
                    bevy_color(theme.muted_text)
                }),
                LineHeight::RelativeToFont(style.line_height),
                TextLayout::justify(text_justify(*text_align)),
            ));
        }
        RuntimeResolvedSceneComponent::Button {
            label,
            action,
            layout,
        }
        | RuntimeResolvedSceneComponent::Choice {
            label,
            action,
            selected: false,
            layout,
        } => {
            entity_commands.insert((
                Button,
                Node {
                    padding: UiRect::axes(
                        px(theme.control_layout.padding_horizontal_px),
                        px(theme.control_layout.padding_vertical_px),
                    ),
                    margin: UiRect::all(px(theme.control_layout.margin_px)),
                    border_radius: BorderRadius::all(px(theme.control_layout.corner_radius_px)),
                    ..node_from_layout(layout, None)
                },
                BackgroundColor(bevy_color(if focused {
                    theme.control_focused
                } else {
                    theme.control
                })),
            ));
            if let Some(action) = action {
                entity_commands.insert(PuzzleBevyUiAction(action.clone()));
            }
            entity_commands.insert((
                Text::new(label.clone()),
                TextFont {
                    font_size: FontSize::Px(theme.typography.body.font_size_px),
                    ..default()
                },
                LineHeight::RelativeToFont(theme.typography.body.line_height),
                TextColor(bevy_color(theme.text)),
            ));
        }
        RuntimeResolvedSceneComponent::Choice {
            label,
            action,
            selected: true,
            layout,
        } => {
            entity_commands.insert((
                Button,
                PuzzleBevyUiSelectedChoice,
                Node {
                    padding: UiRect::axes(
                        px(theme.control_layout.padding_horizontal_px),
                        px(theme.control_layout.padding_vertical_px),
                    ),
                    margin: UiRect::all(px(theme.control_layout.margin_px)),
                    border: UiRect::all(px(theme.control_layout.border_width_px)),
                    border_radius: BorderRadius::all(px(theme.control_layout.corner_radius_px)),
                    ..node_from_layout(layout, None)
                },
                BorderColor::all(bevy_color(theme.control_selected_border)),
                BackgroundColor(bevy_color(theme.control_selected)),
            ));
            if let Some(action) = action {
                entity_commands.insert(PuzzleBevyUiAction(action.clone()));
            }
            entity_commands.insert((
                Text::new(label.clone()),
                TextFont {
                    font_size: FontSize::Px(theme.typography.body.font_size_px),
                    ..default()
                },
                LineHeight::RelativeToFont(theme.typography.body.line_height),
                TextColor(bevy_color(theme.text)),
            ));
        }
        RuntimeResolvedSceneComponent::Row {
            layout, children, ..
        }
        | RuntimeResolvedSceneComponent::Column {
            layout, children, ..
        }
        | RuntimeResolvedSceneComponent::Box {
            layout, children, ..
        } => {
            let direction = if matches!(component, RuntimeResolvedSceneComponent::Row { .. }) {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            };
            entity_commands.insert(node_from_layout(layout, Some(direction)));
            let child_entities = children
                .iter()
                .enumerate()
                .map(|(index, child)| {
                    let mut child_id = id.clone();
                    child_id.tree_path.push(index);
                    sync_ui_component(
                        commands, existing, retained, child_id, child, focused, theme,
                    )
                })
                .collect::<Vec<_>>();
            commands.entity(entity).replace_children(&child_entities);
        }
        RuntimeResolvedSceneComponent::Viewport { source, layout, .. } => {
            entity_commands.insert((
                viewport_node_from_layout(layout),
                PuzzleBevyUiViewport(source.clone()),
            ));
        }
        RuntimeResolvedSceneComponent::Frame { .. } => {
            unreachable!("unsupported frame components reject at the snapshot boundary")
        }
    }
    entity
}

fn text_justify(text_align: Option<puzzle_scene::SceneTextAlign>) -> Justify {
    match text_align.unwrap_or_default() {
        puzzle_scene::SceneTextAlign::Start => Justify::Start,
        puzzle_scene::SceneTextAlign::Center => Justify::Center,
        puzzle_scene::SceneTextAlign::End => Justify::End,
    }
}

fn text_style(theme: &RuntimeTheme, role: SceneTextRole) -> RuntimeUiTextStyle {
    match role {
        SceneTextRole::Heading => theme.typography.heading,
        SceneTextRole::Subheading => theme.typography.subheading,
        SceneTextRole::Body => theme.typography.body,
        SceneTextRole::Caption => theme.typography.caption,
    }
}

fn bevy_color(color: RuntimeLinearRgba) -> Color {
    Color::linear_rgba(
        color.red as f32,
        color.green as f32,
        color.blue as f32,
        color.alpha as f32,
    )
}

fn node_from_layout(layout: &puzzle_scene::SceneLayout, direction: Option<FlexDirection>) -> Node {
    use puzzle_scene::{SceneAlign, SceneDistribution, SceneSpace};

    let mut node = Node {
        flex_direction: direction.unwrap_or(FlexDirection::Column),
        align_self: layout
            .align_self
            .map(|align| match align {
                SceneAlign::Start => AlignSelf::FlexStart,
                SceneAlign::Center => AlignSelf::Center,
                SceneAlign::End => AlignSelf::FlexEnd,
                SceneAlign::Stretch => AlignSelf::Stretch,
            })
            .unwrap_or(AlignSelf::Auto),
        align_items: match layout.align {
            SceneAlign::Start => AlignItems::FlexStart,
            SceneAlign::Center => AlignItems::Center,
            SceneAlign::End => AlignItems::FlexEnd,
            SceneAlign::Stretch => AlignItems::Stretch,
        },
        justify_content: match layout.distribute {
            SceneDistribution::Start => JustifyContent::FlexStart,
            SceneDistribution::Center => JustifyContent::Center,
            SceneDistribution::End => JustifyContent::FlexEnd,
            SceneDistribution::Between => JustifyContent::SpaceBetween,
        },
        aspect_ratio: layout
            .aspect_ratio
            .map(|ratio| f32::from(ratio.width) / f32::from(ratio.height)),
        overflow: if layout.scroll {
            Overflow::scroll()
        } else {
            Overflow::visible()
        },
        ..default()
    };
    if let Some(gap) = layout.gap {
        node.row_gap = px(gap);
        node.column_gap = px(gap);
    }
    if let SceneSpace::Fill { weight } = layout.space {
        node.flex_grow = f32::from(weight);
        node.flex_basis = px(0);
    }
    node
}

fn viewport_node_from_layout(layout: &puzzle_scene::SceneLayout) -> Node {
    let mut node = node_from_layout(layout, None);
    if layout.align_self.is_none() {
        node.align_self = AlignSelf::Stretch;
    }
    node
}

fn dispatch_pointer_actions(
    interactions: Query<(&Interaction, &PuzzleBevyUiAction), Changed<Interaction>>,
    time: Res<Time>,
    mut host: NonSendMut<PuzzleBevyPlayerHost>,
) {
    if host.fatal_error.is_some() {
        return;
    }
    for (interaction, action) in &interactions {
        if *interaction == Interaction::Pressed
            && let Err(error) = host.dispatch_action(
                SessionAction::SceneAction {
                    token: action.0.clone(),
                },
                time.elapsed_secs_f64(),
            )
        {
            host.fail(error);
            return;
        }
    }
}

fn update_interactive_camera(
    primary_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    submitted_rects: Res<SubmittedViewportRects>,
    mut interaction: ResMut<InteractiveCameraState>,
    mut host: NonSendMut<PuzzleBevyPlayerHost>,
) {
    if host.fatal_error.is_some() {
        return;
    }
    let Ok(window) = primary_window.single() else {
        return;
    };
    interaction.offsets.retain(|source, _| {
        matches!(
            host.viewports
                .get(source)
                .map(|viewport| &viewport.renderer),
            Some(RuntimeRendererState::ThreeD(_))
        )
    });

    let cursor = window.physical_cursor_position();
    if mouse_buttons.just_pressed(MouseButton::Left) {
        interaction.active_look = cursor.and_then(|cursor| {
            interactive_camera_source_at(
                cursor,
                window.physical_height(),
                &submitted_rects.0,
                &host.viewports,
                |camera| camera.interactive_look,
            )
        });
    }
    if mouse_buttons.just_released(MouseButton::Left) {
        interaction.active_look = None;
    }

    if mouse_buttons.pressed(MouseButton::Left)
        && mouse_motion.delta != Vec2::ZERO
        && let Some(source) = interaction.active_look.clone()
        && let Some(viewport) = host.viewports.get_mut(&source)
        && let RuntimeRendererState::ThreeD(scene) = &viewport.renderer
    {
        let offset = interaction.offsets.entry(source).or_default();
        offset.yaw_degrees += mouse_motion.delta.x * 0.25;
        let base_pitch = f32::from(scene.render.camera.pitch_degrees);
        let next_pitch =
            (base_pitch + offset.pitch_degrees - mouse_motion.delta.y * 0.25).clamp(-88.0, 88.0);
        offset.pitch_degrees = next_pitch - base_pitch;
        viewport.needs_frame = true;
    }

    if mouse_scroll.delta.y != 0.0
        && let Some(cursor) = cursor
        && let Some(source) = interactive_camera_source_at(
            cursor,
            window.physical_height(),
            &submitted_rects.0,
            &host.viewports,
            |camera| camera.interactive_zoom,
        )
        && let Some(viewport) = host.viewports.get_mut(&source)
    {
        let exponent = match mouse_scroll.unit {
            MouseScrollUnit::Line => mouse_scroll.delta.y * 0.12,
            MouseScrollUnit::Pixel => mouse_scroll.delta.y * 0.002,
        };
        let offset = interaction.offsets.entry(source).or_default();
        offset.zoom_factor = (offset.zoom_factor * exponent.exp()).clamp(0.2, 5.0);
        viewport.needs_frame = true;
    }
}

fn interactive_camera_source_at(
    cursor_from_top: Vec2,
    window_height: u32,
    rects: &BTreeMap<RuntimeViewportSourceId, PuzzleBevyFramebufferRect>,
    viewports: &BTreeMap<RuntimeViewportSourceId, HostViewport>,
    enabled: impl Fn(&puzzle_runtime_contract::RuntimePuzzle3Camera) -> bool,
) -> Option<RuntimeViewportSourceId> {
    let cursor_from_bottom = Vec2::new(cursor_from_top.x, window_height as f32 - cursor_from_top.y);
    viewports
        .values()
        .filter_map(|viewport| {
            let RuntimeRendererState::ThreeD(scene) = &viewport.renderer else {
                return None;
            };
            if !enabled(&scene.render.camera) {
                return None;
            }
            let rect = rects.get(&viewport.source)?;
            let min = rect.physical_position.as_vec2();
            let max = min + rect.physical_size.as_vec2();
            (cursor_from_bottom.cmpge(min).all() && cursor_from_bottom.cmplt(max).all())
                .then_some((viewport.order, viewport.source.clone()))
        })
        .max_by_key(|(order, _)| *order)
        .map(|(_, source)| source)
}

fn update_ui_scroll(
    primary_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    touches: Res<Touches>,
    submitted_rects: Res<SubmittedViewportRects>,
    host: NonSend<PuzzleBevyPlayerHost>,
    mut touch_targets: ResMut<TouchScrollTargets>,
    mut scroll_nodes: Query<
        (
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &mut ScrollPosition,
        ),
        Without<PuzzleBevyUiSelectedChoice>,
    >,
) {
    let Ok(window) = primary_window.single() else {
        return;
    };
    let wheel_is_camera_input = window.physical_cursor_position().is_some_and(|cursor| {
        interactive_camera_source_at(
            cursor,
            window.physical_height(),
            &submitted_rects.0,
            &host.viewports,
            |camera| camera.interactive_zoom,
        )
        .is_some()
    });
    if !wheel_is_camera_input
        && mouse_scroll.delta != Vec2::ZERO
        && let Some(cursor) = window.cursor_position()
        && let Some(target) = scroll_container_at(cursor, &mut scroll_nodes)
        && let Ok((_, _, _, mut position)) = scroll_nodes.get_mut(target)
    {
        let scale = match mouse_scroll.unit {
            MouseScrollUnit::Line => 32.0,
            MouseScrollUnit::Pixel => 1.0,
        };
        position.0 -= mouse_scroll.delta * scale;
        position.0 = position.0.max(Vec2::ZERO);
    }

    for touch in touches.iter_just_pressed() {
        if let Some(target) = scroll_container_at(touch.position(), &mut scroll_nodes) {
            touch_targets.0.insert(touch.id(), target);
        }
    }
    for touch in touches.iter() {
        let Some(target) = touch_targets.0.get(&touch.id()).copied() else {
            continue;
        };
        let delta = touch.position() - touch.previous_position();
        if delta != Vec2::ZERO
            && let Ok((_, _, _, mut position)) = scroll_nodes.get_mut(target)
        {
            position.0 -= delta;
            position.0 = position.0.max(Vec2::ZERO);
        }
    }
    for touch in touches
        .iter_just_released()
        .chain(touches.iter_just_canceled())
    {
        touch_targets.0.remove(&touch.id());
    }
}

fn scroll_container_at(
    cursor: Vec2,
    scroll_nodes: &mut Query<
        (
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &mut ScrollPosition,
        ),
        Without<PuzzleBevyUiSelectedChoice>,
    >,
) -> Option<Entity> {
    scroll_nodes
        .iter_mut()
        .filter_map(|(entity, node, transform, _)| {
            let size = node.size();
            let (_, _, center) = transform.to_scale_angle_translation();
            let min = center - size / 2.0;
            let max = center + size / 2.0;
            (cursor.cmpge(min).all() && cursor.cmplt(max).all())
                .then_some((size.x * size.y, entity))
        })
        .min_by(|(left, _), (right, _)| left.total_cmp(right))
        .map(|(_, entity)| entity)
}

fn dispatch_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut keyboard_events: MessageReader<KeyboardInput>,
    time: Res<Time>,
    mut host: NonSendMut<PuzzleBevyPlayerHost>,
) {
    if host.fatal_error.is_some() {
        return;
    }
    if keyboard.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]) {
        return;
    }
    let trigger = keyboard_events
        .read()
        .find(|event| event.state == ButtonState::Pressed)
        .and_then(|event| runtime_key_trigger(&event.logical_key));
    if let Some(trigger) = trigger
        && let Err(error) =
            host.dispatch_action(SessionAction::Key { trigger }, time.elapsed_secs_f64())
    {
        host.fail(error);
    }
}

fn advance_presentation(time: Res<Time>, mut host: NonSendMut<PuzzleBevyPlayerHost>) {
    if host.fatal_error.is_none()
        && let Err(error) = host.process_presentation(time.elapsed_secs_f64())
    {
        host.fail(error);
    }
}

fn scroll_selected_choice_into_view(
    selected_choices: Query<
        (Entity, &ComputedNode, &UiGlobalTransform),
        (
            With<PuzzleBevyUiSelectedChoice>,
            Changed<PuzzleBevyUiSelectedChoice>,
        ),
    >,
    parents: Query<&ChildOf>,
    mut scroll_nodes: Query<
        (&ComputedNode, &UiGlobalTransform, &mut ScrollPosition),
        Without<PuzzleBevyUiSelectedChoice>,
    >,
) {
    for (selected, selected_node, selected_transform) in &selected_choices {
        let (_, _, selected_center) = selected_transform.to_scale_angle_translation();
        let mut ancestor = parents.get(selected).ok().map(ChildOf::parent);
        while let Some(entity) = ancestor {
            if let Ok((container_node, container_transform, mut scroll_position)) =
                scroll_nodes.get_mut(entity)
            {
                let (_, _, container_center) = container_transform.to_scale_angle_translation();
                scroll_position.0 += scroll_delta_to_reveal(
                    container_center,
                    container_node.size(),
                    selected_center,
                    selected_node.size(),
                );
                scroll_position.0 = scroll_position.0.max(Vec2::ZERO);
            }
            ancestor = parents.get(entity).ok().map(ChildOf::parent);
        }
    }
}

fn scroll_delta_to_reveal(
    container_center: Vec2,
    container_size: Vec2,
    child_center: Vec2,
    child_size: Vec2,
) -> Vec2 {
    let container_min = container_center - container_size / 2.0;
    let container_max = container_center + container_size / 2.0;
    let child_min = child_center - child_size / 2.0;
    let child_max = child_center + child_size / 2.0;
    let axis = |container_min: f32, container_max: f32, child_min: f32, child_max: f32| {
        if child_min < container_min {
            child_min - container_min
        } else if child_max > container_max {
            child_max - container_max
        } else {
            0.0
        }
    };
    Vec2::new(
        axis(container_min.x, container_max.x, child_min.x, child_max.x),
        axis(container_min.y, container_max.y, child_min.y, child_max.y),
    )
}

fn submit_resolved_frames(
    time: Res<Time>,
    primary_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    viewport_nodes: Query<(&PuzzleBevyUiViewport, &ComputedNode, &UiGlobalTransform)>,
    mut queue_2d: ResMut<BevyResolvedFrameQueue2d>,
    mut queue_3d: ResMut<BevyResolvedFrameQueue>,
    mut publication_groups: ResMut<BevyPublicationGroups>,
    mut submitted_ids: ResMut<SubmittedViewportIds>,
    mut submitted_rects: ResMut<SubmittedViewportRects>,
    interactive_camera: Res<InteractiveCameraState>,
    mut pending_observation: ResMut<PendingPuzzleBevyPlayerObservation>,
    mut host: NonSendMut<PuzzleBevyPlayerHost>,
) {
    pending_observation.presentation_started_micros = presentation_clock_micros();
    let completed_groups = publication_groups.drain_completed().collect::<Vec<_>>();
    for completed in completed_groups {
        if host.renderer_publication_group != Some(completed) {
            host.fail(BevyPlayerError::Runtime(format!(
                "renderer completed unexpected publication group {}",
                completed.0
            )));
            return;
        }
        if let Some(transition) = host.renderer_publication_animation_transition.take()
            && let Err(error) = host.acknowledge_animation_publication(transition)
        {
            host.fail(error);
            return;
        }
        host.renderer_publication_group = None;
    }
    if host.renderer_publication_group.is_some() {
        return;
    }
    let Ok(window) = primary_window.single() else {
        if !host.viewports.is_empty() {
            host.fail(BevyPlayerError::MissingPrimaryWindow);
        }
        return;
    };
    let layout_sources = viewport_nodes
        .iter()
        .map(|(viewport, _, _)| viewport.0.clone())
        .collect::<BTreeSet<_>>();
    let framebuffer_by_source = viewport_nodes
        .iter()
        .filter_map(|(viewport, node, transform)| {
            framebuffer_rect(
                node,
                transform,
                UVec2::new(window.physical_width(), window.physical_height()),
            )
            .map(|framebuffer| (viewport.0.clone(), framebuffer))
        })
        .collect::<BTreeMap<_, _>>();
    let missing_layout_node = host
        .viewports
        .values()
        .find(|viewport| !layout_sources.contains(&viewport.source))
        .map(|viewport| viewport.source.clone());
    if let Some(source) = missing_layout_node {
        host.fail(BevyPlayerError::MissingViewportLayout(source));
        return;
    }
    if host
        .viewports
        .values()
        .any(|viewport| !framebuffer_by_source.contains_key(&viewport.source))
    {
        // A resize can temporarily collapse an otherwise mounted UI leaf to
        // zero area. That geometry is not a frame candidate: retain the last
        // committed renderer submission until layout produces a valid rect.
        return;
    }
    for viewport in host.viewports.values_mut() {
        match framebuffer_by_source.get(&viewport.source) {
            Some(rect) if submitted_rects.0.get(&viewport.source) != Some(rect) => {
                viewport.needs_frame = true;
            }
            _ => {}
        }
    }
    let active = host
        .viewports
        .values()
        .map(|viewport| view_id(&viewport.source, &viewport.renderer))
        .collect::<BTreeSet<_>>();
    let result = host.resolve_frames(time.elapsed_secs_f64());
    let frames = match result {
        Ok(frames) => frames,
        Err(error) => {
            host.fail(error);
            return;
        }
    };
    let order_by_id = host
        .viewports
        .values()
        .map(|viewport| {
            (
                view_id(&viewport.source, &viewport.renderer),
                viewport.order,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let queue_2d_checkpoint = queue_2d.clone();
    let queue_3d_checkpoint = queue_3d.clone();
    macro_rules! abort_publication {
        ($error:expr) => {{
            *queue_2d = queue_2d_checkpoint.clone();
            *queue_3d = queue_3d_checkpoint.clone();
            host.fail($error);
            return;
        }};
    }
    if let Err(error) = reconcile_view_camera_orders(&mut queue_2d, &mut queue_3d, &order_by_id) {
        abort_publication!(error);
    }
    let submitted = submitted_ids.0.clone();
    for removed in submitted.difference(&active) {
        let result = match removed.dimension {
            puzzle_bevy_renderer::PuzzleBevyViewDimension::TwoD => queue_2d.remove(removed),
            puzzle_bevy_renderer::PuzzleBevyViewDimension::ThreeD => queue_3d.remove(removed),
        };
        if let Err(error) = result {
            abort_publication!(error.into());
        }
    }
    if frames.is_empty() {
        submitted_ids.0 = active;
        submitted_rects.0 = framebuffer_by_source;
        update_pending_observation(
            &mut pending_observation,
            &host,
            u64::try_from(time.delta().as_micros()).unwrap_or(u64::MAX),
        );
        return;
    }
    let completes_animation_transition =
        match host.completed_animation_transition_at(time.elapsed_secs_f64()) {
            Ok(transition) => transition,
            Err(error) => {
                abort_publication!(error);
            }
        };
    let publication_group = match publication_groups.reserve_group() {
        Ok(group) => group,
        Err(error) => {
            abort_publication!(BevyPlayerError::Runtime(error.to_string()));
        }
    };
    let mut publication_members = Vec::with_capacity(frames.len());
    let clear_color = bevy_color(host.snapshot.theme.background);
    #[cfg(feature = "editor-debug")]
    let authoring_configuration = host.editor_authoring.clone();
    #[cfg(feature = "editor-debug")]
    let mut next_authoring_frame = None;
    for resolved in frames {
        let Some(framebuffer) = framebuffer_by_source.get(&resolved.source).copied() else {
            abort_publication!(BevyPlayerError::MissingViewportLayout(resolved.source));
        };
        let id = view_id(&resolved.source, &resolved.renderer);
        let order = order_by_id[&id];
        let result = match &resolved.renderer {
            RuntimeRendererState::TwoD(scene) => {
                let view = bevy_2d_view(scene, framebuffer, order, clear_color.clone());
                let prepared =
                    match prepare_resolved_frame_2d(&resolved.frame, &host.visual_images, &view) {
                        Ok(prepared) => prepared,
                        Err(error) => {
                            abort_publication!(error.into());
                        }
                    };
                #[cfg(feature = "editor-debug")]
                if let Some(configuration) = authoring_configuration
                    .as_ref()
                    .filter(|configuration| configuration.viewport_source == resolved.source)
                {
                    let css_size =
                        match editor_authoring_css_size(framebuffer, window.scale_factor()) {
                            Ok(css_size) => css_size,
                            Err(error) => {
                                abort_publication!(BevyPlayerError::EditorAuthoring(error));
                            }
                        };
                    match configuration.frame2d(
                        scene,
                        host.next_editor_authoring_frame_revision,
                        css_size,
                    ) {
                        Ok(frame) => next_authoring_frame = Some(frame),
                        Err(error) => {
                            abort_publication!(BevyPlayerError::EditorAuthoring(error));
                        }
                    }
                }
                let view_id =
                    PuzzleBevyViewId::two_d(&resolved.source.component, &resolved.source.source);
                queue_2d
                    .submit_prepared_in_group(
                        view_id.clone(),
                        view,
                        host.visual_images.clone(),
                        prepared,
                        publication_group,
                    )
                    .map(|generation| (view_id, generation))
            }
            RuntimeRendererState::ThreeD(scene) => {
                let camera = match bevy_camera(
                    &resolved.source,
                    scene,
                    interactive_camera.offsets.get(&resolved.source),
                ) {
                    Ok(camera) => camera,
                    Err(error) => {
                        abort_publication!(error);
                    }
                };
                #[cfg(feature = "editor-debug")]
                let mut camera = camera;
                #[cfg(feature = "editor-debug")]
                if let Some(configuration) = authoring_configuration
                    .as_ref()
                    .filter(|configuration| configuration.viewport_source == resolved.source)
                    && let Err(error) = configuration.apply_camera(&mut camera)
                {
                    abort_publication!(BevyPlayerError::EditorAuthoring(error));
                }
                let prepared = match prepare_resolved_frame(&resolved.frame) {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        abort_publication!(error.into());
                    }
                };
                #[cfg(feature = "editor-debug")]
                if let Some(configuration) = authoring_configuration
                    .as_ref()
                    .filter(|configuration| configuration.viewport_source == resolved.source)
                {
                    let css_size =
                        match editor_authoring_css_size(framebuffer, window.scale_factor()) {
                            Ok(css_size) => css_size,
                            Err(error) => {
                                abort_publication!(BevyPlayerError::EditorAuthoring(error));
                            }
                        };
                    match configuration.frame3d(
                        scene,
                        host.next_editor_authoring_frame_revision,
                        css_size,
                        camera.clone(),
                        prepared.bounds,
                    ) {
                        Ok(frame) => next_authoring_frame = Some(frame),
                        Err(error) => {
                            abort_publication!(BevyPlayerError::EditorAuthoring(error));
                        }
                    }
                }
                let view_id =
                    PuzzleBevyViewId::three_d(&resolved.source.component, &resolved.source.source);
                queue_3d
                    .submit_prepared_in_group(
                        view_id.clone(),
                        PuzzleBevy3dView {
                            active: true,
                            order,
                            framebuffer,
                            clear_color,
                            camera,
                            lighting: bevy_lighting(scene),
                            shadows_enabled: scene.render.shadow,
                            render_settings: bevy_render_settings(scene),
                        },
                        prepared,
                        publication_group,
                    )
                    .map(|generation| (view_id, generation))
            }
        };
        match result {
            Ok((view_id, generation)) => {
                publication_members.push(BevyPublicationMember {
                    view_id,
                    generation,
                });
            }
            Err(error) => {
                abort_publication!(error.into());
            }
        }
    }
    #[cfg(feature = "editor-debug")]
    let next_authoring_revision = if next_authoring_frame.is_some() {
        match host.next_editor_authoring_frame_revision.checked_add(1) {
            Some(revision) => Some(revision),
            None => abort_publication!(BevyPlayerError::EditorAuthoring(
                "editor authoring frame revision overflow".to_string(),
            )),
        }
    } else {
        None
    };
    if let Err(error) = publication_groups.register_group(publication_group, publication_members) {
        abort_publication!(BevyPlayerError::Runtime(error.to_string()));
    }
    host.renderer_publication_group = Some(publication_group);
    host.renderer_publication_animation_transition = completes_animation_transition;
    #[cfg(feature = "editor-debug")]
    if let Some(frame) = next_authoring_frame {
        host.editor_authoring_frame = Some(frame);
        host.next_editor_authoring_frame_revision =
            next_authoring_revision.expect("prepared editor frame has a reserved revision");
    }
    submitted_ids.0 = active;
    submitted_rects.0 = framebuffer_by_source;
    update_pending_observation(
        &mut pending_observation,
        &host,
        u64::try_from(time.delta().as_micros()).unwrap_or(u64::MAX),
    );
}

fn update_pending_observation(
    pending: &mut PendingPuzzleBevyPlayerObservation,
    host: &PuzzleBevyPlayerHost,
    submission_interval_micros: u64,
) {
    pending.revision = host.snapshot.session_revision.0;
    pending.surface_focus = host.snapshot.surface.focus.clone();
    pending.viewport_count = host.viewports.len();
    pending.submission_interval_micros = submission_interval_micros;
    pending.progress_fingerprint = host.runtime.progress_state_fingerprint();
    pending.audio_capability = host.audio_capability();
    pending.present = true;
}

fn reconcile_view_camera_orders(
    queue_2d: &mut BevyResolvedFrameQueue2d,
    queue_3d: &mut BevyResolvedFrameQueue,
    order_by_id: &BTreeMap<PuzzleBevyViewId, isize>,
) -> Result<(), BevyPlayerError> {
    let desired_2d = order_by_id
        .iter()
        .filter(|(id, _)| id.dimension == puzzle_bevy_renderer::PuzzleBevyViewDimension::TwoD)
        .map(|(id, order)| (id.clone(), *order))
        .collect::<BTreeMap<_, _>>();
    let desired_3d = order_by_id
        .iter()
        .filter(|(id, _)| id.dimension == puzzle_bevy_renderer::PuzzleBevyViewDimension::ThreeD)
        .map(|(id, order)| (id.clone(), *order))
        .collect::<BTreeMap<_, _>>();
    queue_2d.reconcile_camera_orders(&desired_2d)?;
    queue_3d.reconcile_camera_orders(&desired_3d)?;
    Ok(())
}

fn commit_player_observation(
    mut pending: ResMut<PendingPuzzleBevyPlayerObservation>,
    mut observations: ResMut<PuzzleBevyPlayerObservationState>,
) {
    if !pending.present {
        return;
    }
    let presentation_cpu_micros = pending
        .presentation_started_micros
        .take()
        .zip(presentation_clock_micros())
        .map(|(started_at, finished_at)| finished_at.saturating_sub(started_at));
    observations.record_submission(
        pending.revision,
        &pending.surface_focus,
        pending.viewport_count,
        pending.submission_interval_micros,
        presentation_cpu_micros,
        wasm_linear_memory_bytes(),
        pending.progress_fingerprint,
        pending.audio_capability,
    );
}

fn bevy_2d_view(
    scene: &puzzle_session_contract::RuntimePuzzle2Snapshot,
    framebuffer: PuzzleBevyFramebufferRect,
    order: isize,
    clear_color: Color,
) -> PuzzleBevy2dView {
    PuzzleBevy2dView {
        active: true,
        order,
        framebuffer,
        clear_color,
        origin: Vec2::new(scene.view.origin[0] as f32, scene.view.origin[1] as f32),
        size: Vec2::new(f32::from(scene.view.size[0]), f32::from(scene.view.size[1])),
    }
}

#[cfg(feature = "editor-debug")]
fn editor_authoring_css_size(
    framebuffer: PuzzleBevyFramebufferRect,
    scale_factor: f32,
) -> Result<Vec2, String> {
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err(format!(
            "editor authoring window scale factor must be finite and greater than zero, got {scale_factor}"
        ));
    }
    let size = framebuffer.physical_size.as_vec2() / scale_factor;
    if !size.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
        return Err(
            "editor authoring committed CSS size must be finite and greater than zero".to_string(),
        );
    }
    Ok(size)
}

fn framebuffer_rect(
    node: &ComputedNode,
    transform: &UiGlobalTransform,
    window_size: UVec2,
) -> Option<PuzzleBevyFramebufferRect> {
    let size = node.size();
    if !size.is_finite() || size.x < 1.0 || size.y < 1.0 {
        return None;
    }
    let (_, _, center) = transform.to_scale_angle_translation();
    let top_left = center - size / 2.0;
    let left = top_left.x.floor().clamp(0.0, window_size.x as f32) as u32;
    let right = (top_left.x + size.x)
        .ceil()
        .clamp(0.0, window_size.x as f32) as u32;
    let top = top_left.y.floor().clamp(0.0, window_size.y as f32) as u32;
    let bottom_from_top = (top_left.y + size.y)
        .ceil()
        .clamp(0.0, window_size.y as f32) as u32;
    if right <= left || bottom_from_top <= top {
        return None;
    }
    let width = right - left;
    let height = bottom_from_top - top;
    let bottom = window_size.y - bottom_from_top;
    Some(PuzzleBevyFramebufferRect {
        physical_position: UVec2::new(left, bottom),
        physical_size: UVec2::new(width, height),
    })
}

fn view_id(source: &RuntimeViewportSourceId, renderer: &RuntimeRendererState) -> PuzzleBevyViewId {
    match renderer {
        RuntimeRendererState::TwoD(_) => PuzzleBevyViewId::two_d(&source.component, &source.source),
        RuntimeRendererState::ThreeD(_) => {
            PuzzleBevyViewId::three_d(&source.component, &source.source)
        }
    }
}

fn bevy_camera(
    viewport_source: &RuntimeViewportSourceId,
    scene: &puzzle_runtime_contract::RuntimePuzzle3Snapshot,
    interaction: Option<&InteractiveCameraOffset>,
) -> Result<PuzzleBevyCamera, BevyPlayerError> {
    let source = &scene.render.camera;
    if !source.zoom.is_finite() || source.zoom <= 0.0 {
        return Err(BevyPlayerError::InvalidCameraZoom {
            source: viewport_source.clone(),
            zoom: source.zoom,
        });
    }
    let interaction = interaction.copied().unwrap_or_default();
    Ok(PuzzleBevyCamera {
        projection: match source.projection {
            RuntimePuzzle3CameraProjection::Perspective => PuzzleCameraProjection::Perspective,
            RuntimePuzzle3CameraProjection::Orthographic => PuzzleCameraProjection::Orthographic,
        },
        yaw_degrees: f32::from(source.yaw_degrees) + interaction.yaw_degrees,
        pitch_degrees: f32::from(source.pitch_degrees) + interaction.pitch_degrees,
        roll_degrees: f32::from(source.roll_degrees),
        distance_scale: 2.8 / (source.zoom as f32 * interaction.zoom_factor),
        target: None,
    })
}

fn bevy_lighting(scene: &puzzle_runtime_contract::RuntimePuzzle3Snapshot) -> PuzzleBevyLighting {
    let source = &scene.render.lighting;
    PuzzleBevyLighting {
        intensity: source.intensity as f32,
        ambient: source.ambient as f32,
        yaw_degrees: f32::from(source.yaw_degrees),
        pitch_degrees: f32::from(source.pitch_degrees),
        color: Color::linear_rgba(
            source.color.red as f32,
            source.color.green as f32,
            source.color.blue as f32,
            source.color.alpha as f32,
        ),
    }
}

fn bevy_render_settings(
    scene: &puzzle_runtime_contract::RuntimePuzzle3Snapshot,
) -> PuzzleBevy3dRenderSettings {
    PuzzleBevy3dRenderSettings {
        shade: scene.render.visual.shade,
        pixelate: PuzzleBevyPixelate {
            enabled: scene.render.pixelate.enabled,
            scale: scene.render.pixelate.scale,
            smoothing: scene.render.pixelate.smoothing,
        },
    }
}

fn runtime_key_trigger(key: &Key) -> Option<RuntimeKeyTrigger> {
    match key {
        Key::Character(value) => runtime_key_trigger_for_logical_key(value),
        Key::ArrowUp => runtime_key_trigger_for_logical_key("ArrowUp"),
        Key::ArrowDown => runtime_key_trigger_for_logical_key("ArrowDown"),
        Key::ArrowLeft => runtime_key_trigger_for_logical_key("ArrowLeft"),
        Key::ArrowRight => runtime_key_trigger_for_logical_key("ArrowRight"),
        Key::Enter => runtime_key_trigger_for_logical_key("Enter"),
        Key::Space => runtime_key_trigger_for_logical_key(" "),
        Key::Escape => runtime_key_trigger_for_logical_key("Escape"),
        Key::Tab => runtime_key_trigger_for_logical_key("Tab"),
        Key::Backspace => runtime_key_trigger_for_logical_key("Backspace"),
        _ => None,
    }
}

/// Converts a platform logical-key value into the runtime-owned trigger
/// contract. Native Bevy events and editor-forwarded browser events share this
/// mapping rather than maintaining parallel key semantics.
pub fn runtime_key_trigger_for_logical_key(value: &str) -> Option<RuntimeKeyTrigger> {
    match value {
        "ArrowUp" => Some(RuntimeKeyTrigger::ArrowUp),
        "ArrowDown" => Some(RuntimeKeyTrigger::ArrowDown),
        "ArrowLeft" => Some(RuntimeKeyTrigger::ArrowLeft),
        "ArrowRight" => Some(RuntimeKeyTrigger::ArrowRight),
        "Enter" => Some(RuntimeKeyTrigger::Enter),
        " " | "Spacebar" => Some(RuntimeKeyTrigger::Space),
        "Escape" | "Esc" => Some(RuntimeKeyTrigger::Escape),
        "Tab" => Some(RuntimeKeyTrigger::Tab),
        "Backspace" => Some(RuntimeKeyTrigger::Backspace),
        value => {
            let mut characters = value.chars();
            let value = characters.next()?;
            characters
                .next()
                .is_none()
                .then_some(RuntimeKeyTrigger::Character { value })
        }
    }
}

#[cfg(test)]
mod tests {
    use puzzle_bevy_renderer::{prepare_resolved_frame, prepare_resolved_frame_2d};
    use puzzle_scene::{SceneAlign, SceneAspectRatio, SceneDistribution, SceneLayout, SceneSpace};

    use super::*;
    use puzzle_runtime_contract::StandaloneRuntimeExport;

    const TENETEN: &str = include_str!("../../../games/TENETEN.puzzle");
    const TENETEN3D: &str = include_str!("../../../games/TENETEN3D.puzzle");
    const ANIMATION_TEST_2D: &str = include_str!("../../../games/animation_test.puzzle");
    const EDITOR_TWEEN: &str = r#"
const title = "Editor Tween"

puzzle default {
render {
tween = true
tween_duration = 300ms
}
layers {
actor = Player
}
rules {
input right [ Player | no Player ] -> [ | Player ]
}
}

visuals {
Player {
#fff
0
}
}

levels {
legend {
P = Player
. = empty
}
level "one" {
P.
}
}
"#;

    const TIMED_WAIT: &str = r#"
const title = bevy_timed_wait
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

    #[test]
    fn player_separates_theme_clear_and_ui_composition_from_renderer_views() {
        let mut app = App::new();
        app.add_systems(Startup, spawn_ui_root);
        app.update();

        let mut background_cameras = app.world_mut().query_filtered::<
            (&Camera, &Projection, &RenderLayers),
            With<PuzzleBevyBackgroundCamera>,
        >();
        let background = background_cameras.iter(app.world()).collect::<Vec<_>>();
        assert_eq!(background.len(), 1);
        assert_eq!(background[0].0.order, -1);
        assert!(matches!(
            background[0].0.clear_color,
            ClearColorConfig::Custom(_)
        ));
        assert!(matches!(background[0].1, Projection::Orthographic(_)));
        assert_eq!(background[0].2, &RenderLayers::none());

        let mut ui_cameras = app.world_mut().query_filtered::<
            (&Camera, &Projection, &RenderLayers),
            (With<PuzzleBevyUiCamera>, With<IsDefaultUiCamera>),
        >();
        let ui = ui_cameras.iter(app.world()).collect::<Vec<_>>();
        assert_eq!(ui.len(), 1);
        assert_eq!(ui[0].0.order, isize::MAX);
        assert!(matches!(ui[0].0.clear_color, ClearColorConfig::None));
        assert!(matches!(ui[0].1, Projection::Orthographic(_)));
        assert_eq!(ui[0].2, &RenderLayers::none());

        let mut roots = app
            .world_mut()
            .query_filtered::<Option<&BackgroundColor>, With<PuzzleBevyUiRoot>>();
        let root_background = roots.single(app.world()).unwrap().unwrap();
        assert_eq!(root_background.0.alpha(), 0.0);
    }

    #[test]
    fn player_observation_advances_only_after_semantic_submission_changes() {
        let mut state = PuzzleBevyPlayerObservationState::default();
        state.record_submission(
            4,
            "title",
            0,
            16_000,
            Some(240),
            None,
            11,
            AudioCapabilityState::Locked,
        );
        assert_eq!(
            state.latest(),
            Some(&PuzzleBevyPlayerObservation {
                sequence: 1,
                submission_sequence: 1,
                revision: 4,
                surface_focus: "title".to_string(),
                viewport_count: 0,
                submission_interval_micros: 16_000,
                presentation_cpu_micros: Some(240),
                wasm_linear_memory_bytes: None,
                progress_fingerprint: 11,
                audio_capability: AudioCapabilityState::Locked,
            })
        );

        state.record_submission(
            4,
            "title",
            0,
            17_000,
            Some(260),
            None,
            11,
            AudioCapabilityState::Ready,
        );
        assert_eq!(state.latest().unwrap().sequence, 1);
        assert_eq!(state.latest().unwrap().submission_sequence, 2);
        assert_eq!(
            state.latest().unwrap().audio_capability,
            AudioCapabilityState::Ready
        );
        state.record_submission(
            4,
            "title",
            0,
            17_500,
            Some(270),
            None,
            12,
            AudioCapabilityState::Ready,
        );
        assert_eq!(state.latest().unwrap().sequence, 2);
        assert_eq!(state.latest().unwrap().submission_sequence, 3);
        state.record_submission(
            5,
            "playing",
            1,
            18_000,
            Some(280),
            None,
            12,
            AudioCapabilityState::Ready,
        );
        assert_eq!(
            state.latest(),
            Some(&PuzzleBevyPlayerObservation {
                sequence: 3,
                submission_sequence: 4,
                revision: 5,
                surface_focus: "playing".to_string(),
                viewport_count: 1,
                submission_interval_micros: 18_000,
                presentation_cpu_micros: Some(280),
                wasm_linear_memory_bytes: None,
                progress_fingerprint: 12,
                audio_capability: AudioCapabilityState::Ready,
            })
        );
    }

    #[test]
    fn standalone_export_constructs_the_final_host_with_typed_storage() {
        let document = puzzle_lang::parse_game_for_path(TENETEN, "games/TENETEN.puzzle").unwrap();
        let export = StandaloneRuntimeExport::new(
            document,
            puzzle_assets::EncodedVisualImageBundle::default(),
            StandaloneProgressStorage {
                key: "TENETEN:revision".to_string(),
                save_version: 2,
            },
        );
        let loaded = load_standalone_bevy_player(&serde_json::to_string(&export).unwrap()).unwrap();

        assert_eq!(loaded.progress_storage.key, "TENETEN:revision");
        assert_eq!(loaded.progress_storage.save_version, 2);
        assert!(loaded.host.fatal_error().is_none());
        assert!(loaded.host.snapshot().surface.root.is_some());
        assert!(!loaded.host.snapshot().surface.components.is_empty());
    }

    #[test]
    fn standalone_export_rejects_an_unreferenced_visual_image() {
        let document = puzzle_lang::parse_game_for_path(TENETEN, "games/TENETEN.puzzle").unwrap();
        let export = StandaloneRuntimeExport::new(
            document,
            puzzle_assets::EncodedVisualImageBundle {
                assets: vec![puzzle_assets::EncodedVisualImageAsset {
                    manifest: puzzle_assets::VisualImageAssetManifestEntry::from_path(
                        "visuals/tile.png",
                    )
                    .unwrap(),
                    revision: puzzle_assets::VisualImageAssetRevision(
                        "declared-revision".to_string(),
                    ),
                    bytes: vec![0, 1, 2, 3],
                }],
            },
            StandaloneProgressStorage {
                key: "TENETEN:revision".to_string(),
                save_version: 2,
            },
        );

        let error = match load_standalone_bevy_player(&serde_json::to_string(&export).unwrap()) {
            Ok(_) => panic!("unreferenced visual image must reject the standalone player export"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("visual image set does not match runtime references"),
            "{error}"
        );
    }

    #[test]
    fn host_persistence_operations_refresh_the_typed_snapshot_and_viewports() {
        let mut clear_source =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN, "games/TENETEN.puzzle").unwrap();
        clear_source
            .dispatch_action(
                SessionAction::Key {
                    trigger: RuntimeKeyTrigger::Enter,
                },
                0.0,
            )
            .unwrap();
        let clear = clear_source
            .pending_progress_save()
            .expect("entering the game must request persistence");
        assert_eq!(clear.operation, RuntimeProgressPersistenceOperation::Delete);
        clear_source
            .confirm_progress_persistence_applied(clear.request_id, 0.5)
            .unwrap();
        assert!(!clear_source.snapshot().has_progress_save);

        let mut source =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN, "games/TENETEN.puzzle").unwrap();
        source
            .dispatch_action(SessionAction::Restart, 0.75)
            .unwrap();
        let save = source
            .pending_progress_save()
            .expect("a non-clear mutation must request a progress write");
        let RuntimeProgressPersistenceOperation::Write { save_json } = &save.operation else {
            panic!("gameplay mutation must produce a typed write operation");
        };

        let mut restored =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN, "games/TENETEN.puzzle").unwrap();
        let initial_surface = restored.snapshot().surface.clone();
        restored.restore_progress_save(save_json, 1.0).unwrap();

        assert!(restored.snapshot().has_progress_save);
        assert_ne!(restored.snapshot().surface, initial_surface);
        assert!(restored.pending_progress_save().is_none());
        restored
            .dispatch_action(
                SessionAction::Key {
                    trigger: RuntimeKeyTrigger::Enter,
                },
                2.0,
            )
            .expect("the restored selected Continue token must remain executable");
        assert_eq!(restored.snapshot().surface.root.as_deref(), Some("playing"));

        let pending = source
            .pending_progress_save()
            .expect("the persisted action must keep exposing its typed request");
        let stale_error = source
            .confirm_progress_persistence_applied(pending.request_id + 1, 3.0)
            .unwrap_err();
        assert!(stale_error.to_string().contains("is stale"));
        assert_eq!(source.pending_progress_save(), Some(pending.clone()));

        source
            .confirm_progress_persistence_applied(pending.request_id, 4.0)
            .unwrap();
        assert!(source.pending_progress_save().is_none());
        assert!(source.snapshot().has_progress_save);
    }

    #[test]
    fn progress_ack_during_an_active_timed_wait_preserves_its_deadline() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(TIMED_WAIT, "bevy_timed_wait.puzzle")
                .unwrap();
        host.dispatch_action(
            SessionAction::Input {
                name: "first".to_string(),
            },
            0.0,
        )
        .unwrap();
        let request = host
            .pending_progress_save()
            .expect("the mutation that starts the wait must request persistence");
        let waiting_revision = host.snapshot().session_revision;

        host.process_presentation(0.0).unwrap();
        assert_eq!(host.wait_until_seconds, Some(0.1));
        assert!(host.pending_presentation.is_empty());

        host.confirm_progress_persistence_applied(request.request_id, 0.05)
            .unwrap();

        assert!(host.pending_progress_save().is_none());
        assert_eq!(
            host.wait_until_seconds,
            Some(0.1),
            "persistence acknowledgement must not restart or bypass the authored wait"
        );
        assert!(host.pending_presentation.is_empty());
        assert!(host.pending_presentation_continuation.is_some());

        host.process_presentation(0.099).unwrap();
        assert_eq!(host.snapshot().session_revision, waiting_revision);
        assert!(
            host.snapshot()
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.continuation.as_ref())
                .is_some()
        );

        host.process_presentation(0.1).unwrap();
        assert!(!host.snapshot().busy);
        assert!(
            host.snapshot()
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.continuation.as_ref())
                .is_none()
        );
        assert!(host.pending_presentation_continuation.is_none());
        assert!(host.fatal_error().is_none());
    }

    #[test]
    fn committed_progress_ack_is_not_reported_as_retryable_when_projection_fails() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN, "games/TENETEN.puzzle").unwrap();
        host.dispatch_action(
            SessionAction::Key {
                trigger: RuntimeKeyTrigger::Enter,
            },
            0.0,
        )
        .unwrap();
        let request = host
            .pending_progress_save()
            .expect("the action must request persistence");

        let result = host.confirm_progress_persistence_applied_with(
            request.request_id,
            1.0,
            |_host, _now_seconds| {
                Err(BevyPlayerError::Runtime(
                    "post-ack projection failed".to_string(),
                ))
            },
        );

        assert!(
            result.is_ok(),
            "a committed acknowledgement must not be exposed as retryable"
        );
        assert!(
            host.pending_progress_save().is_none(),
            "the exact runtime request was consumed at the commit point"
        );
        assert_eq!(
            host.fatal_error(),
            Some("game runtime failed: post-ack projection failed"),
            "post-commit projection failure belongs to the player fatal channel"
        );
    }

    #[test]
    fn bevy_ui_consumes_resolved_linear_theme_values() {
        let host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN3D, "games/TENETEN3D.puzzle")
                .unwrap();
        let theme = host.snapshot().theme;
        assert_eq!(
            theme.background,
            puzzle_presentation::resolve_palette_color("#8fcf6f").unwrap()
        );
        assert_eq!(
            bevy_color(theme.background),
            Color::linear_rgba(
                theme.background.red as f32,
                theme.background.green as f32,
                theme.background.blue as f32,
                theme.background.alpha as f32,
            )
        );
        assert_eq!(
            text_style(&theme, SceneTextRole::Heading).font_size_px,
            theme.typography.heading.font_size_px
        );
        assert_eq!(
            LineHeight::RelativeToFont(theme.typography.heading.line_height),
            LineHeight::RelativeToFont(text_style(&theme, SceneTextRole::Heading).line_height)
        );
        assert_eq!(
            text_justify(Some(puzzle_scene::SceneTextAlign::Center)),
            Justify::Center
        );
        assert_eq!(
            text_justify(None),
            Justify::Center,
            "scene text without an authored alignment uses the scene-owned centered default"
        );
        assert_eq!(
            text_justify(Some(puzzle_scene::SceneTextAlign::End)),
            Justify::End
        );
    }

    #[test]
    fn scene_ui_uses_one_uniform_scale_for_wide_and_tall_windows() {
        assert_eq!(
            uniform_ui_scale(Vec2::new(960.0, 720.0), Vec2::new(480.0, 360.0)),
            Some(2.0)
        );
        assert_eq!(
            uniform_ui_scale(Vec2::new(960.0, 540.0), Vec2::new(480.0, 360.0)),
            Some(1.5)
        );
        assert_eq!(
            uniform_ui_scale(Vec2::new(360.0, 720.0), Vec2::new(480.0, 360.0)),
            Some(0.75)
        );
        assert_eq!(uniform_ui_scale(Vec2::ZERO, Vec2::new(480.0, 360.0)), None);
    }

    #[test]
    fn placement_and_modal_state_select_one_shared_surface_stack() {
        assert_eq!(
            PuzzleBevyUiSurfaceStack::ORDERED,
            [
                PuzzleBevyUiSurfaceStack::Root,
                PuzzleBevyUiSurfaceStack::Content,
                PuzzleBevyUiSurfaceStack::Overlay,
                PuzzleBevyUiSurfaceStack::Modal,
            ]
        );
        assert_eq!(
            ui_surface_stack(puzzle_scene::ComponentPlacement::Root, false).layer(),
            PuzzleBevyUiSurfaceLayer::Root
        );
        assert_eq!(
            ui_surface_stack(puzzle_scene::ComponentPlacement::Content, false).layer(),
            PuzzleBevyUiSurfaceLayer::Content
        );
        assert_eq!(
            ui_surface_stack(puzzle_scene::ComponentPlacement::Overlay, false).layer(),
            PuzzleBevyUiSurfaceLayer::Overlay
        );
        assert_eq!(
            ui_surface_stack(puzzle_scene::ComponentPlacement::Content, true).layer(),
            PuzzleBevyUiSurfaceLayer::Overlay,
            "modal content must compose in the overlay layer"
        );
        assert_eq!(
            ui_surface_stack(puzzle_scene::ComponentPlacement::Content, true).root_z_index(),
            100,
            "the same modal tier must own viewport order and UI root z-index"
        );
    }

    #[test]
    fn awaited_pointer_event_attaches_only_its_typed_action() {
        let host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN, "games/TENETEN.puzzle").unwrap();
        let mut component = host.snapshot().surface.components[0].clone();
        let RuntimeComponentPresentation::Ready(mut scene) = component.presentation.clone() else {
            panic!("fixture root presentation must resolve");
        };
        let token = RuntimeSceneActionToken {
            component: component.id.clone(),
            action: puzzle_runtime_contract::RuntimeSceneActionId::Event {
                name: "dismiss".to_string(),
            },
        };
        component.await_event = Some("dismiss".to_string());
        scene.events = Some(BTreeMap::from([(
            "dismiss".to_string(),
            puzzle_session_contract::RuntimeResolvedEventBinding {
                pointer: false,
                keys: Vec::new(),
                action: Some(token.clone()),
            },
        )]));
        assert!(awaited_pointer_action(&component, &scene).is_none());
        scene
            .events
            .as_mut()
            .unwrap()
            .get_mut("dismiss")
            .unwrap()
            .pointer = true;
        assert_eq!(awaited_pointer_action(&component, &scene), Some(&token));
    }

    #[test]
    fn invalid_camera_zoom_fails_at_the_native_adapter_boundary() {
        let host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN3D, "games/TENETEN3D.puzzle")
                .unwrap();
        let (source, scene) = host
            .snapshot()
            .viewport_sources
            .iter()
            .find_map(|(source, renderer)| match renderer {
                RuntimeRendererState::ThreeD(scene) => Some((source.clone(), scene.clone())),
                RuntimeRendererState::TwoD(_) => None,
            })
            .expect("fixture must expose one 3D viewport");
        let mut invalid = scene;
        invalid.render.camera.zoom = 0.0;
        assert!(matches!(
            bevy_camera(&source, &invalid, None),
            Err(BevyPlayerError::InvalidCameraZoom {
                source: failed_source,
                zoom: 0.0
            }) if failed_source == source
        ));
    }

    #[test]
    fn interactive_camera_offset_changes_only_the_host_camera_projection() {
        let host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN3D, "games/TENETEN3D.puzzle")
                .unwrap();
        let (source, scene) = host
            .snapshot()
            .viewport_sources
            .iter()
            .find_map(|(source, renderer)| match renderer {
                RuntimeRendererState::ThreeD(scene) => Some((source, scene)),
                RuntimeRendererState::TwoD(_) => None,
            })
            .expect("fixture must expose a typed 3D viewport");
        let authored = bevy_camera(source, scene, None).unwrap();
        let interactive = bevy_camera(
            source,
            scene,
            Some(&InteractiveCameraOffset {
                yaw_degrees: 12.0,
                pitch_degrees: -7.0,
                zoom_factor: 2.0,
            }),
        )
        .unwrap();

        assert_eq!(interactive.yaw_degrees, authored.yaw_degrees + 12.0);
        assert_eq!(interactive.pitch_degrees, authored.pitch_degrees - 7.0);
        assert_eq!(interactive.distance_scale, authored.distance_scale / 2.0);
        assert_eq!(interactive.roll_degrees, authored.roll_degrees);
        assert_eq!(interactive.projection, authored.projection);
    }

    #[test]
    fn three_dimensional_render_settings_cross_the_player_boundary_without_defaults() {
        let host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN3D, "games/TENETEN3D.puzzle")
                .unwrap();
        let mut scene = host
            .snapshot()
            .viewport_sources
            .values()
            .find_map(|renderer| match renderer {
                RuntimeRendererState::ThreeD(scene) => Some(scene.clone()),
                RuntimeRendererState::TwoD(_) => None,
            })
            .expect("fixture must expose a typed 3D viewport");
        scene.render.visual.shade = false;
        scene.render.pixelate.enabled = true;
        scene.render.pixelate.scale = 5;
        scene.render.pixelate.smoothing = true;

        let settings = bevy_render_settings(&scene);
        assert!(!settings.shade);
        assert!(settings.pixelate.enabled);
        assert_eq!(settings.pixelate.scale, 5);
        assert!(settings.pixelate.smoothing);
    }

    #[test]
    fn unresolved_authored_theme_color_fails_at_session_initialization() {
        let source = TENETEN3D.replacen(
            "background_color = #8fcf6f",
            "background_color = not-a-color",
            1,
        );
        let error = match PuzzleBevyPlayerHost::from_image_free_source(
            &source,
            "games/invalid_theme.puzzle",
        ) {
            Ok(_) => panic!("unresolved authored theme color must fail"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("InvalidColor"));
        assert!(error.to_string().contains("not-a-color"));
    }

    #[test]
    fn teneten_title_boots_without_a_viewport_and_selected_menu_token_activates() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN, "games/TENETEN.puzzle").unwrap();
        assert_eq!(host.viewport_count(), 0);
        host.process_presentation(0.0).unwrap();
        assert_eq!(host.snapshot().surface.focus, "title");
        assert!(host.fatal_error().is_none());
        host.dispatch_action(
            SessionAction::Key {
                trigger: RuntimeKeyTrigger::Enter,
            },
            0.0,
        )
        .unwrap();
        let modal_component = host
            .snapshot()
            .surface
            .components
            .iter()
            .find(|component| component.modal)
            .map(|component| component.id.clone())
            .expect("the first TENETEN level message must remain modal");
        assert!(
            host.snapshot()
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.continuation.as_ref())
                .is_none()
        );
        host.process_presentation(0.0).unwrap();
        assert!(host.snapshot().session_revision.0 > 0);
        assert!(
            host.snapshot()
                .surface
                .components
                .iter()
                .any(|component| component.modal && component.id == modal_component)
        );
        assert!(host.fatal_error().is_none());
    }

    #[test]
    fn teneten3d_uses_the_component_scoped_viewport_source() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN3D, "games/TENETEN3D.puzzle")
                .unwrap();
        host.process_presentation(0.0).unwrap();
        let frames = host.resolve_frames(0.0).unwrap();
        assert_eq!(frames.len(), 1);
        assert!(
            !prepare_resolved_frame(&frames[0].frame)
                .unwrap()
                .voxels
                .is_empty()
        );
    }

    #[test]
    fn two_d_game_uses_the_component_scoped_viewport_source() {
        let mut host = PuzzleBevyPlayerHost::from_image_free_source(
            ANIMATION_TEST_2D,
            "games/animation_test.puzzle",
        )
        .unwrap();
        host.process_presentation(0.0).unwrap();
        let frames = host.resolve_frames(0.0).unwrap();
        assert_eq!(frames.len(), 1);
        let RuntimeRendererState::TwoD(scene) = &frames[0].renderer else {
            panic!("fixture must resolve a 2D viewport")
        };
        let view = bevy_2d_view(
            scene,
            PuzzleBevyFramebufferRect {
                physical_position: UVec2::ZERO,
                physical_size: UVec2::new(320, 240),
            },
            0,
            Color::BLACK,
        );
        assert!(
            !prepare_resolved_frame_2d(&frames[0].frame, host.visual_images.as_ref(), &view)
                .unwrap()
                .meshes
                .is_empty()
        );
    }

    #[test]
    fn bevy_2d_view_consumes_the_runtime_resolved_window() {
        let host = PuzzleBevyPlayerHost::from_image_free_source(
            ANIMATION_TEST_2D,
            "games/animation_test.puzzle",
        )
        .unwrap();
        let RuntimeRendererState::TwoD(mut scene) =
            host.viewports.values().next().unwrap().renderer.clone()
        else {
            panic!("fixture must resolve a 2D viewport")
        };
        scene.view.origin = [3, 5];
        scene.view.size = [7, 4];
        let view = bevy_2d_view(
            &scene,
            PuzzleBevyFramebufferRect {
                physical_position: UVec2::new(11, 13),
                physical_size: UVec2::new(280, 160),
            },
            2,
            Color::BLACK,
        );
        assert_eq!(view.origin, Vec2::new(3.0, 5.0));
        assert_eq!(view.size, Vec2::new(7.0, 4.0));
        assert_eq!(view.framebuffer.physical_position, UVec2::new(11, 13));
        assert_eq!(view.order, 2);
    }

    #[test]
    fn keyboard_mapping_only_converts_logical_keys_to_runtime_triggers() {
        assert_eq!(
            runtime_key_trigger(&Key::ArrowRight),
            Some(RuntimeKeyTrigger::ArrowRight)
        );
        assert_eq!(
            runtime_key_trigger(&Key::Character("。".into())),
            Some(RuntimeKeyTrigger::Character { value: '。' })
        );
        assert_eq!(runtime_key_trigger(&Key::F1), None);
        assert_eq!(
            runtime_key_trigger_for_logical_key("ArrowRight"),
            Some(RuntimeKeyTrigger::ArrowRight)
        );
        assert_eq!(
            runtime_key_trigger_for_logical_key("。"),
            Some(RuntimeKeyTrigger::Character { value: '。' })
        );
        assert_eq!(runtime_key_trigger_for_logical_key("F1"), None);
    }

    #[test]
    fn snapshot_refresh_preserves_active_queue_for_the_same_transition_identity() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(TIMED_WAIT, "bevy_timed_wait.puzzle")
                .unwrap();
        host.dispatch_action(
            SessionAction::Input {
                name: "first".to_string(),
            },
            0.0,
        )
        .unwrap();
        let transition = host
            .snapshot()
            .presentation
            .as_ref()
            .expect("the first input must install a presentation transition")
            .id;

        host.process_presentation(0.0).unwrap();
        assert_eq!(host.wait_until_seconds, Some(0.1));
        host.dispatch_action(SessionAction::Snapshot, 0.05).unwrap();

        assert_eq!(
            host.snapshot()
                .presentation
                .as_ref()
                .expect("snapshot refresh must retain the active transition")
                .id,
            transition
        );
        assert_eq!(
            host.wait_until_seconds,
            Some(0.1),
            "refreshing the same transition must preserve its in-flight deadline"
        );
        assert!(host.pending_presentation.is_empty());
    }

    #[test]
    fn discrete_sample_deadline_is_not_consumed_before_it_is_due() {
        let mut sample = HostNextSample::Deadline { at_seconds: 1.25 };

        assert!(!sample.consume_if_due(1.249));
        assert!(sample.consume_if_due(1.25));
    }

    #[test]
    fn unrelated_frame_rates_remain_independent_nearest_deadlines_in_the_host() {
        let now_seconds = 4.0;
        let mut samples = [7_u64, 11, 13, 19].map(|fps| HostNextSample::Deadline {
            at_seconds: now_seconds + (1_000 / fps) as f64 / 1_000.0,
        });

        let nearest_deadline = samples
            .iter()
            .map(|sample| match sample {
                HostNextSample::Deadline { at_seconds } => *at_seconds,
                HostNextSample::DisplayRefresh { .. } => {
                    panic!("discrete frame rates must remain deadline samples")
                }
            })
            .reduce(f64::min)
            .unwrap();
        assert_eq!(nearest_deadline, now_seconds + 52.0 / 1_000.0);
        assert_eq!(
            samples
                .iter_mut()
                .map(|sample| sample.consume_if_due(nearest_deadline))
                .filter(|due| *due)
                .count(),
            1,
            "the host must wake only the channel with the nearest individual boundary"
        );
    }

    #[test]
    fn display_refresh_samples_once_per_increasing_time_until_finite_completion() {
        let mut sample = HostNextSample::DisplayRefresh {
            completion_at_seconds: 1.03,
            last_sample_seconds: 1.0,
        };

        assert!(!sample.consume_if_due(1.0));
        assert!(sample.consume_if_due(1.01));
        assert!(!sample.consume_if_due(1.01));
        assert!(sample.consume_if_due(1.02));
        assert!(sample.consume_if_due(1.04));
        assert!(!sample.consume_if_due(1.05));
    }

    #[test]
    fn pure_timed_wait_completes_once_after_its_deadline() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(TIMED_WAIT, "bevy_timed_wait.puzzle")
                .unwrap();

        host.dispatch_action(
            SessionAction::Input {
                name: "first".to_string(),
            },
            0.0,
        )
        .unwrap();
        let waiting_revision = host.snapshot().session_revision;
        assert!(
            host.snapshot()
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.continuation.as_ref())
                .is_some()
        );

        host.process_presentation(0.0).unwrap();
        host.dispatch_action(SessionAction::Snapshot, 0.05).unwrap();
        host.process_presentation(0.099).unwrap();
        assert_eq!(host.snapshot().session_revision, waiting_revision);
        assert!(
            host.snapshot()
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.continuation.as_ref())
                .is_some()
        );

        host.process_presentation(0.1).unwrap();
        let completed_revision = host.snapshot().session_revision;
        assert!(!host.snapshot().busy);
        assert!(
            host.snapshot()
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.continuation.as_ref())
                .is_none()
        );
        assert!(host.pending_presentation_continuation.is_none());

        host.process_presentation(1.0).unwrap();
        assert_eq!(host.snapshot().session_revision, completed_revision);
        assert!(host.fatal_error().is_none());
    }

    #[test]
    fn preserved_wait_queue_replaces_its_stale_completion_token() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(TIMED_WAIT, "bevy_timed_wait.puzzle")
                .unwrap();
        host.dispatch_action(
            SessionAction::Input {
                name: "first".to_string(),
            },
            0.0,
        )
        .unwrap();
        let first_token = host
            .snapshot()
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.continuation.clone())
            .expect("the first input must wait");
        host.process_presentation(0.0).unwrap();

        host.dispatch_action(
            SessionAction::Input {
                name: "second".to_string(),
            },
            0.01,
        )
        .unwrap();
        let refreshed_token = host
            .snapshot()
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.continuation.clone())
            .expect("queuing input must retain the timeline wait");
        assert_eq!(refreshed_token.waits, first_token.waits);
        assert!(refreshed_token.session_revision > first_token.session_revision);
        assert_eq!(
            host.pending_presentation_continuation,
            Some(refreshed_token)
        );

        host.process_presentation(0.049).unwrap();
        assert!(host.snapshot().busy);
        host.process_presentation(0.05).unwrap();
        assert!(!host.snapshot().busy);
        assert!(host.pending_presentation_continuation.is_none());
        assert!(host.fatal_error().is_none());
    }

    #[test]
    fn queued_input_policy_preserves_the_original_wait_deadline() {
        let mut document =
            puzzle_lang::parse_game_for_path(TIMED_WAIT, "bevy_timed_wait.puzzle").unwrap();
        let puzzle_lang::LoadedDocumentModel::Puzzle2d { game, .. } = &mut document.models[0]
        else {
            panic!("timed wait fixture must be 2D");
        };
        game.input_buffer.busy_input = puzzle_lang::BusyInputPolicy::Queue;
        let mut host = PuzzleBevyPlayerHost::from_image_free_runtime(
            RuntimeSession::from_document(document).unwrap(),
        )
        .unwrap();
        host.dispatch_action(
            SessionAction::Input {
                name: "first".to_string(),
            },
            0.0,
        )
        .unwrap();
        host.process_presentation(0.0).unwrap();

        host.dispatch_action(
            SessionAction::Input {
                name: "second".to_string(),
            },
            0.05,
        )
        .unwrap();
        host.process_presentation(0.099).unwrap();
        assert!(host.snapshot().busy);
        host.process_presentation(0.1).unwrap();
        assert!(!host.snapshot().busy);
    }

    #[test]
    fn accelerate_policy_shortens_the_wait_to_its_minimum_duration() {
        let mut document =
            puzzle_lang::parse_game_for_path(TIMED_WAIT, "bevy_timed_wait.puzzle").unwrap();
        let puzzle_lang::LoadedDocumentModel::Puzzle2d { game, .. } = &mut document.models[0]
        else {
            panic!("timed wait fixture must be 2D");
        };
        game.input_buffer.busy_input = puzzle_lang::BusyInputPolicy::Accelerate { min_wait_ms: 50 };
        let mut host = PuzzleBevyPlayerHost::from_image_free_runtime(
            RuntimeSession::from_document(document).unwrap(),
        )
        .unwrap();
        host.dispatch_action(
            SessionAction::Input {
                name: "first".to_string(),
            },
            0.0,
        )
        .unwrap();
        host.process_presentation(0.0).unwrap();

        host.dispatch_action(
            SessionAction::Input {
                name: "second".to_string(),
            },
            0.01,
        )
        .unwrap();
        host.process_presentation(0.049).unwrap();
        assert!(host.snapshot().busy);
        host.process_presentation(0.05).unwrap();
        assert!(!host.snapshot().busy);
        assert!(host.pending_presentation_continuation.is_none());
    }

    #[test]
    fn reject_policy_does_not_queue_busy_model_input() {
        let mut document =
            puzzle_lang::parse_game_for_path(TIMED_WAIT, "bevy_timed_wait.puzzle").unwrap();
        let puzzle_lang::LoadedDocumentModel::Puzzle2d { game, .. } = &mut document.models[0]
        else {
            panic!("timed wait fixture must be 2D");
        };
        game.input_buffer.busy_input = puzzle_lang::BusyInputPolicy::Reject;
        let mut host = PuzzleBevyPlayerHost::from_image_free_runtime(
            RuntimeSession::from_document(document).unwrap(),
        )
        .unwrap();
        host.dispatch_action(
            SessionAction::Input {
                name: "first".to_string(),
            },
            0.0,
        )
        .unwrap();
        host.process_presentation(0.0).unwrap();
        let waiting_revision = host.snapshot().session_revision;

        host.dispatch_action(
            SessionAction::Input {
                name: "second".to_string(),
            },
            0.01,
        )
        .unwrap();

        assert_eq!(host.snapshot().session_revision, waiting_revision);
        assert!(!host.snapshot().queued_model_input);
        host.process_presentation(0.099).unwrap();
        assert!(host.snapshot().busy);
    }

    #[test]
    fn skip_policy_completes_the_wait_and_runs_the_queued_input_immediately() {
        let mut document =
            puzzle_lang::parse_game_for_path(TIMED_WAIT, "bevy_timed_wait.puzzle").unwrap();
        let puzzle_lang::LoadedDocumentModel::Puzzle2d { game, .. } = &mut document.models[0]
        else {
            panic!("timed wait fixture must be 2D");
        };
        game.input_buffer.busy_input = puzzle_lang::BusyInputPolicy::Skip;
        let mut host = PuzzleBevyPlayerHost::from_image_free_runtime(
            RuntimeSession::from_document(document).unwrap(),
        )
        .unwrap();
        host.dispatch_action(
            SessionAction::Input {
                name: "first".to_string(),
            },
            0.0,
        )
        .unwrap();
        host.process_presentation(0.0).unwrap();

        host.dispatch_action(
            SessionAction::Input {
                name: "second".to_string(),
            },
            0.01,
        )
        .unwrap();

        assert!(!host.snapshot().busy);
        assert!(host.wait_until_seconds.is_none());
        assert!(host.pending_presentation_continuation.is_none());
        assert!(host.fatal_error().is_none());
    }

    #[test]
    fn modal_busy_state_without_a_timeline_token_is_never_completed_by_the_host() {
        let source = r#"
const title = bevy_modal_wait
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
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(source, "bevy_modal_wait.puzzle").unwrap();
        host.dispatch_action(
            SessionAction::Input {
                name: "open".to_string(),
            },
            0.0,
        )
        .unwrap();
        let modal_component = host
            .snapshot()
            .surface
            .components
            .iter()
            .find(|component| component.modal)
            .map(|component| component.id.clone())
            .expect("message must project a modal component");
        let revision = host.snapshot().session_revision;
        assert!(host.snapshot().busy);
        assert!(
            host.snapshot()
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.continuation.as_ref())
                .is_none()
        );

        host.process_presentation(1.0).unwrap();

        assert_eq!(host.snapshot().session_revision, revision);
        assert!(host.snapshot().busy);
        assert!(
            host.snapshot()
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.continuation.as_ref())
                .is_none()
        );
        assert!(
            host.snapshot()
                .surface
                .components
                .iter()
                .any(|component| component.modal && component.id == modal_component)
        );
        assert!(host.fatal_error().is_none());
    }

    #[test]
    fn stacked_messages_keep_only_the_top_bevy_surface_actionable() {
        let source = r#"
const title = stacked_messages

scene title {
layout {
choice "Start" -> {
start playing
}
}
}

puzzle board {
layers { actor = A }
input noop
rules {
if input == noop {
[ A ] -> [ A ]
}
}
}

levels {
legend { A = A }
level "start"
message "first"
message "second"
A
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
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(source, "stacked_messages.puzzle")
                .unwrap();
        host.dispatch_action(
            SessionAction::Key {
                trigger: RuntimeKeyTrigger::Enter,
            },
            0.0,
        )
        .expect("Bevy must accept an inactive covered modal");

        let modals = host
            .snapshot()
            .surface
            .components
            .iter()
            .filter(|component| component.modal)
            .collect::<Vec<_>>();
        assert_eq!(modals.len(), 2);
        let RuntimeComponentPresentation::Ready(covered_scene) = &modals[0].presentation else {
            panic!("covered message presentation must resolve");
        };
        assert!(
            awaited_pointer_action(modals[0], covered_scene).is_none(),
            "the covered Bevy surface must not receive a pointer action"
        );
        let covered_id = modals[0].id.clone();
        let RuntimeComponentPresentation::Ready(top_scene) = &modals[1].presentation else {
            panic!("top message presentation must resolve");
        };
        let top_action = awaited_pointer_action(modals[1], top_scene)
            .expect("the top Bevy surface must receive the runtime-owned action")
            .clone();

        host.dispatch_action(SessionAction::SceneAction { token: top_action }, 0.0)
            .unwrap();
        let remaining = host
            .snapshot()
            .surface
            .components
            .iter()
            .filter(|component| component.modal)
            .collect::<Vec<_>>();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, covered_id);
        let RuntimeComponentPresentation::Ready(scene) = &remaining[0].presentation else {
            panic!("remaining message presentation must resolve");
        };
        assert!(
            awaited_pointer_action(remaining[0], scene).is_some(),
            "the next Bevy surface must become actionable after the top is dismissed"
        );
    }

    #[test]
    fn authored_lighting_reaches_the_bevy_player_contract() {
        let source = TENETEN3D.replacen(
            "render {\n",
            "render {\nlighting {\nintensity = 0.75\nambient = 1.25\nyaw = -20\npitch = 60\ncolor = #ffd7aa\n}\n",
            1,
        );
        let host =
            PuzzleBevyPlayerHost::from_image_free_source(&source, "games/typed_lighting.puzzle")
                .expect("authored lighting should initialize the typed Bevy host");
        let lighting = host
            .viewports
            .values()
            .find_map(|viewport| match &viewport.renderer {
                RuntimeRendererState::ThreeD(scene) => Some(bevy_lighting(scene)),
                RuntimeRendererState::TwoD(_) => None,
            })
            .expect("the 3D viewport owns authored lighting");
        assert_eq!(lighting.intensity, 0.75);
        assert_eq!(lighting.ambient, 1.25);
        assert_eq!(lighting.yaw_degrees, -20.0);
        assert_eq!(lighting.pitch_degrees, 60.0);
        assert_eq!(
            lighting.color,
            Color::linear_rgba(1.0, 0.6795425, 0.40197778, 1.0)
        );
    }

    #[test]
    fn scene_layout_contract_lowers_to_bevy_node_without_adapter_defaults() {
        let node = node_from_layout(
            &SceneLayout {
                space: SceneSpace::Fill { weight: 3 },
                align_self: Some(SceneAlign::End),
                aspect_ratio: Some(SceneAspectRatio::new(16, 9)),
                gap: Some(7),
                align: SceneAlign::Stretch,
                distribute: SceneDistribution::Between,
                scroll: true,
            },
            Some(FlexDirection::Row),
        );
        assert_eq!(node.flex_direction, FlexDirection::Row);
        assert_eq!(node.flex_grow, 3.0);
        assert_eq!(node.align_self, AlignSelf::FlexEnd);
        assert_eq!(node.align_items, AlignItems::Stretch);
        assert_eq!(node.justify_content, JustifyContent::SpaceBetween);
        assert_eq!(node.aspect_ratio, Some(16.0 / 9.0));
        assert_eq!(node.row_gap, px(7));
        assert_eq!(node.column_gap, px(7));
        assert_eq!(node.overflow, Overflow::scroll());
    }

    #[test]
    fn viewport_sizing_stretches_only_the_unaligned_model_window_cross_axis() {
        let default_viewport = viewport_node_from_layout(&SceneLayout {
            space: SceneSpace::Fill { weight: 1 },
            ..SceneLayout::default()
        });
        assert_eq!(default_viewport.flex_grow, 1.0);
        assert_eq!(default_viewport.align_self, AlignSelf::Stretch);

        let centered_viewport = viewport_node_from_layout(&SceneLayout {
            space: SceneSpace::Fill { weight: 1 },
            align_self: Some(SceneAlign::Center),
            ..SceneLayout::default()
        });
        assert_eq!(centered_viewport.align_self, AlignSelf::Center);

        let generic_child = node_from_layout(
            &SceneLayout {
                space: SceneSpace::Fill { weight: 1 },
                ..SceneLayout::default()
            },
            None,
        );
        assert_eq!(generic_child.align_self, AlignSelf::Auto);
    }

    #[test]
    fn unaligned_model_window_gets_a_non_empty_bevy_ui_layout_inside_a_centered_scene() {
        use bevy::{
            app::TaskPoolPlugin,
            camera::{ComputedCameraValues, RenderTargetInfo, Viewport},
            ecs::schedule::{ScheduleCleanupPolicy, Schedules},
            text::{FontCx, ScaleCx, TextPipeline},
            ui::{UiPlugin, UiSystems},
        };

        let mut app = App::new();
        app.add_plugins((TaskPoolPlugin::default(), UiPlugin));
        app.init_resource::<TextPipeline>();
        app.init_resource::<FontCx>();
        app.init_resource::<ScaleCx>();
        let mut post_update = {
            let mut schedules = app.world_mut().resource_mut::<Schedules>();
            schedules.remove(First);
            schedules.remove(PreUpdate);
            schedules.remove(Update);
            schedules
                .remove(PostUpdate)
                .expect("UiPlugin must install its PostUpdate schedule")
        };
        for set in [UiSystems::Content, UiSystems::PostLayout, UiSystems::Stack] {
            post_update
                .remove_systems_in_set(
                    set,
                    app.world_mut(),
                    ScheduleCleanupPolicy::RemoveSetAndSystemsAllowBreakages,
                )
                .expect("UiPlugin must install the removable UI set");
        }
        app.world_mut()
            .resource_mut::<Schedules>()
            .insert(post_update);

        const WIDTH: u32 = 1000;
        const HEIGHT: u32 = 600;
        app.world_mut().spawn((
            Camera2d,
            Camera {
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::new(WIDTH, HEIGHT),
                        scale_factor: 1.0,
                    }),
                    ..default()
                },
                viewport: Some(Viewport {
                    physical_size: UVec2::new(WIDTH, HEIGHT),
                    ..default()
                }),
                ..default()
            },
        ));
        let scene = app
            .world_mut()
            .spawn(Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .id();
        let viewport = app
            .world_mut()
            .spawn(viewport_node_from_layout(&SceneLayout {
                space: SceneSpace::Fill { weight: 1 },
                ..SceneLayout::default()
            }))
            .id();
        app.world_mut().entity_mut(scene).add_child(viewport);

        app.update();
        app.update();

        let computed = app
            .world()
            .get::<ComputedNode>(viewport)
            .expect("the model window must participate in Bevy UI layout");
        assert!(
            computed.size.x > 0.0 && computed.size.y > 0.0,
            "the model window must receive a non-empty rectangle, got {:?}",
            computed.size
        );
        assert_eq!(computed.size, Vec2::new(WIDTH as f32, HEIGHT as f32));
    }

    #[test]
    fn selected_choice_scroll_delta_reveals_only_clipped_axes() {
        assert_eq!(
            scroll_delta_to_reveal(
                Vec2::new(50.0, 50.0),
                Vec2::new(100.0, 100.0),
                Vec2::new(50.0, 125.0),
                Vec2::new(40.0, 30.0),
            ),
            Vec2::new(0.0, 40.0)
        );
        assert_eq!(
            scroll_delta_to_reveal(
                Vec2::new(50.0, 50.0),
                Vec2::new(100.0, 100.0),
                Vec2::new(20.0, 20.0),
                Vec2::new(20.0, 20.0),
            ),
            Vec2::ZERO
        );
    }

    #[test]
    fn viewport_framebuffer_comes_from_each_ui_leaf_identity() {
        let mut left = ComputedNode::default();
        left.size = Vec2::new(320.0, 240.0);
        let mut right = ComputedNode::default();
        right.size = Vec2::new(640.0, 360.0);
        let left_rect = framebuffer_rect(
            &left,
            &UiGlobalTransform::from_xy(160.0, 120.0),
            UVec2::new(1280, 720),
        )
        .unwrap();
        let right_rect = framebuffer_rect(
            &right,
            &UiGlobalTransform::from_xy(960.0, 180.0),
            UVec2::new(1280, 720),
        )
        .unwrap();
        assert_eq!(left_rect.physical_position, UVec2::new(0, 480));
        assert_eq!(left_rect.physical_size, UVec2::new(320, 240));
        assert_eq!(right_rect.physical_position, UVec2::new(640, 360));
        assert_eq!(right_rect.physical_size, UVec2::new(640, 360));
    }

    #[test]
    fn removed_ui_entities_are_despawned_once_per_removed_subtree() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let child = world.spawn_empty().id();
        let grandchild = world.spawn_empty().id();
        let identity = |tree_path| PuzzleBevyUiIdentity {
            component: "scene".to_string(),
            tree_path,
        };
        let existing = HashMap::from([
            (identity(Vec::new()), root),
            (identity(vec![0]), child),
            (identity(vec![0, 0]), grandchild),
        ]);

        assert_eq!(
            removed_ui_subtree_roots(&existing, &BTreeSet::new()),
            vec![root]
        );
        assert_eq!(
            removed_ui_subtree_roots(&existing, &BTreeSet::from([identity(Vec::new())])),
            vec![child]
        );
    }

    #[test]
    fn animation_batch_routes_to_exact_component_and_source() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(EDITOR_TWEEN, "player_tween.puzzle")
                .unwrap();
        host.dispatch_action(
            SessionAction::Key {
                trigger: RuntimeKeyTrigger::ArrowRight,
            },
            1.0,
        )
        .unwrap();
        let original_source = host.viewports.keys().next().unwrap().clone();
        let animations = host
            .snapshot
            .presentation
            .as_ref()
            .unwrap()
            .steps
            .iter()
            .find_map(|event| match event {
                RuntimePresentationEvent::AnimationBatch { animations, .. } => {
                    Some(animations.clone())
                }
                _ => None,
            })
            .expect("fixture input must emit an animation batch");
        let sibling_source = RuntimeViewportSourceId {
            model: original_source.model.clone(),
            component: original_source.component.clone(),
            source: format!("{}_sibling", original_source.source),
        };
        let mut sibling = host.viewports[&original_source].clone();
        sibling.source = sibling_source.clone();
        host.viewports.insert(sibling_source.clone(), sibling);
        let sibling_origin = host.animation_origins[&original_source].clone();
        host.animation_origins
            .insert(sibling_source.clone(), sibling_origin);
        host.pending_presentation = VecDeque::from([QueuedPresentationEvent {
            event: RuntimePresentationEvent::AnimationBatch {
                source: sibling_source.clone(),
                level_index: Some(0),
                animations,
            },
        }]);
        host.process_presentation(1.0).unwrap();
        assert!(host.viewports[&original_source].active_animation.is_none());
        assert!(host.viewports[&sibling_source].active_animation.is_some());
    }

    #[test]
    fn animation_wait_releases_only_after_the_final_sample_is_published() {
        let source = EDITOR_TWEEN.replace(
            "input right [ Player | no Player ] -> [ | Player ]",
            "input right [ Player | no Player ] -> [ | Player ] wait animation",
        );
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(&source, "player_waiting_tween.puzzle")
                .unwrap();
        host.dispatch_action(
            SessionAction::Key {
                trigger: RuntimeKeyTrigger::ArrowRight,
            },
            1.0,
        )
        .unwrap();
        host.process_presentation(1.0).unwrap();
        let transition = host
            .waiting_for_animation_publication
            .expect("fixture must wait for the animation publication");

        let initial = host.resolve_frames(1.0).unwrap();
        assert!(!initial.is_empty());
        assert_eq!(
            host.completed_animation_transition_at(1.0).unwrap(),
            None,
            "sampling the first frame must not release the continuation"
        );

        let final_sample = host.resolve_frames(1.31).unwrap();
        assert!(!final_sample.is_empty());
        assert_eq!(
            host.completed_animation_transition_at(1.31).unwrap(),
            Some(transition)
        );
        assert_eq!(
            host.waiting_for_animation_publication,
            Some(transition),
            "resolving the final sample is not a renderer publication acknowledgement"
        );

        host.acknowledge_animation_publication(transition).unwrap();
        assert_eq!(host.waiting_for_animation_publication, None);
        assert!(
            host.viewports
                .values()
                .all(|viewport| viewport.active_animation.is_none())
        );
    }

    #[cfg(feature = "editor-debug")]
    #[test]
    fn player_model_input_does_not_invalidate_unchanged_scene_ui() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(EDITOR_TWEEN, "player_tween.puzzle")
                .unwrap();
        let surface_before_input = host.snapshot.surface.clone();
        let ui_generation_before_input = host.ui_projection_generation;
        let revision_before_input = host.snapshot.session_revision;

        host.dispatch_action(
            SessionAction::Key {
                trigger: RuntimeKeyTrigger::ArrowRight,
            },
            1.0,
        )
        .unwrap();

        assert!(host.snapshot.session_revision > revision_before_input);
        assert_eq!(host.snapshot.surface, surface_before_input);
        assert_eq!(
            host.ui_projection_generation, ui_generation_before_input,
            "model input must update the renderer frame without reconfiguring unchanged scene UI"
        );
    }

    #[cfg(feature = "editor-debug")]
    #[test]
    fn editor_authoring_input_keeps_runtime_tween_events_on_the_player_timeline() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(EDITOR_TWEEN, "editor_tween.puzzle")
                .unwrap();
        let puzzle_runtime_contract::SolverStateSnapshot::TwoD {
            width,
            height,
            layer_count,
            slots,
            variables,
            level_fired_rules,
        } = host.editor_development_snapshot().solver_state
        else {
            panic!("editor tween fixture must be two-dimensional")
        };
        let state = RuntimeStateSnapshot::TwoD(puzzle_runtime_contract::RuntimeStateSnapshot2d {
            kind: puzzle_runtime_contract::RuntimeModelKind::TwoD,
            width,
            height,
            layer_count,
            slots,
            variables,
            level_fired_rules,
        });
        host.hydrate_editor_model_state(
            "default",
            &state,
            0,
            false,
            EditorAuthoringPresentation {
                surface: puzzle_editor_preview_contract::EditorAuthoringSurface {
                    surface_id: "preview".to_string(),
                    interaction: puzzle_editor_preview_contract::EditorAuthoringInteraction::Play,
                },
                renderer: EditorRendererStrategy::Grid2d,
            },
            0.0,
        )
        .unwrap();

        let ui_generation_before_input = host.ui_projection_generation;
        let revision_before_input = host.snapshot.session_revision;
        host.dispatch_editor_key(RuntimeKeyTrigger::ArrowRight, false, 1.0)
            .unwrap();
        assert!(
            host.snapshot.session_revision > revision_before_input,
            "the fixture must exercise a new runtime revision"
        );
        assert_eq!(
            host.ui_projection_generation, ui_generation_before_input,
            "model input must update the renderer frame without remounting the editor UI viewport"
        );
        assert!(
            host.snapshot
                .presentation
                .as_ref()
                .is_some_and(|presentation| presentation
                    .steps
                    .iter()
                    .any(|event| matches!(event, RuntimePresentationEvent::AnimationBatch { .. }))),
            "editor input must retain the runtime tween event: {:?}",
            host.snapshot.presentation
        );
        host.process_presentation(1.0).unwrap();

        assert!(
            host.viewports
                .values()
                .any(|viewport| viewport.active_animation.is_some()),
            "editor input must use the same runtime animation queue as standalone input"
        );
    }

    #[test]
    fn surface_and_renderer_viewport_dimensions_must_match() {
        fn flip_first_viewport(components: &mut [RuntimeResolvedSceneComponent]) -> bool {
            for component in components {
                match component {
                    RuntimeResolvedSceneComponent::Viewport { dimension, .. } => {
                        *dimension = match dimension {
                            puzzle_session_contract::RuntimeViewportDimension::TwoD => {
                                puzzle_session_contract::RuntimeViewportDimension::ThreeD
                            }
                            puzzle_session_contract::RuntimeViewportDimension::ThreeD => {
                                puzzle_session_contract::RuntimeViewportDimension::TwoD
                            }
                        };
                        return true;
                    }
                    RuntimeResolvedSceneComponent::Row { children, .. }
                    | RuntimeResolvedSceneComponent::Column { children, .. }
                    | RuntimeResolvedSceneComponent::Box { children, .. } => {
                        if flip_first_viewport(children) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        }

        let host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN3D, "games/TENETEN3D.puzzle")
                .unwrap();
        let mut snapshot = host.snapshot().clone();
        let flipped =
            snapshot.surface.components.iter_mut().any(|component| {
                match &mut component.presentation {
                    RuntimeComponentPresentation::Ready(scene) => {
                        flip_first_viewport(&mut scene.components)
                    }
                    RuntimeComponentPresentation::Error { .. } => false,
                }
            });
        assert!(flipped);
        assert!(matches!(
            projected_viewports(&snapshot, 0.0),
            Err(BevyPlayerError::ViewportDimensionMismatch { .. })
        ));
    }

    #[test]
    fn component_presentation_errors_fail_at_the_snapshot_boundary() {
        let host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN, "games/TENETEN.puzzle").unwrap();
        let mut snapshot = host.snapshot().clone();
        let component = snapshot.surface.components.first_mut().unwrap();
        let component_id = component.id.clone();
        component.presentation = RuntimeComponentPresentation::Error {
            error: "unresolved title".to_string(),
        };
        assert!(matches!(
            projected_viewports(&snapshot, 0.0),
            Err(BevyPlayerError::ComponentPresentation { component, error })
                if component == component_id && error == "unresolved title"
        ));
    }

    #[test]
    fn undeclared_awaited_events_still_fail_at_the_snapshot_boundary() {
        let host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN, "games/TENETEN.puzzle").unwrap();
        let mut snapshot = host.snapshot().clone();
        let component = snapshot.surface.components.first_mut().unwrap();
        let component_id = component.id.clone();
        component.await_event = Some("missing".to_string());

        assert!(matches!(
            projected_viewports(&snapshot, 0.0),
            Err(BevyPlayerError::MissingAwaitedEvent { component, event })
                if component == component_id && event == "missing"
        ));
    }

    #[test]
    fn unsupported_frame_components_fail_at_the_snapshot_boundary() {
        let host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN, "games/TENETEN.puzzle").unwrap();
        let mut snapshot = host.snapshot().clone();
        let component = snapshot.surface.components.first_mut().unwrap();
        let RuntimeComponentPresentation::Ready(scene) = &mut component.presentation else {
            panic!("fixture component must have a resolved presentation");
        };
        scene.components.push(RuntimeResolvedSceneComponent::Frame {
            kind: "frame".to_string(),
            source: "board".to_string(),
            layout: Default::default(),
        });

        assert!(matches!(
            projected_viewports(&snapshot, 0.0),
            Err(BevyPlayerError::UnsupportedFrameComponent { kind, source })
                if kind == "frame" && source == "board"
        ));
    }

    #[test]
    fn one_viewport_source_cannot_back_multiple_ui_leaf_rectangles() {
        fn first_viewport(
            components: &[RuntimeResolvedSceneComponent],
        ) -> Option<RuntimeResolvedSceneComponent> {
            components.iter().find_map(|component| match component {
                RuntimeResolvedSceneComponent::Viewport { .. } => Some(component.clone()),
                RuntimeResolvedSceneComponent::Row { children, .. }
                | RuntimeResolvedSceneComponent::Column { children, .. }
                | RuntimeResolvedSceneComponent::Box { children, .. } => first_viewport(children),
                _ => None,
            })
        }

        let host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN3D, "games/TENETEN3D.puzzle")
                .unwrap();
        let mut snapshot = host.snapshot().clone();
        let duplicated = snapshot.surface.components.iter_mut().any(|component| {
            let RuntimeComponentPresentation::Ready(scene) = &mut component.presentation else {
                return false;
            };
            let Some(viewport) = first_viewport(&scene.components) else {
                return false;
            };
            scene.components.push(viewport);
            true
        });
        assert!(duplicated);
        assert!(matches!(
            projected_viewports(&snapshot, 0.0),
            Err(BevyPlayerError::DuplicateViewportLeaf(_))
        ));
    }

    #[test]
    fn player_reconciles_camera_order_swaps_before_sequential_frame_submission() {
        let mut queue_2d = BevyResolvedFrameQueue2d::default();
        let mut queue_3d = BevyResolvedFrameQueue::default();
        let left = PuzzleBevyViewId::three_d("left", "main");
        let right = PuzzleBevyViewId::three_d("right", "main");
        let frame = RuntimeResolvedRenderFrame {
            batches: Vec::new(),
            decorations: Vec::new(),
            next_sample: None,
        };
        let view = |order, x| PuzzleBevy3dView {
            active: true,
            order,
            framebuffer: PuzzleBevyFramebufferRect {
                physical_position: UVec2::new(x, 0),
                physical_size: UVec2::new(320, 240),
            },
            clear_color: Color::BLACK,
            camera: PuzzleBevyCamera::default(),
            lighting: PuzzleBevyLighting {
                intensity: 1.0,
                ambient: 1.0,
                yaw_degrees: 0.0,
                pitch_degrees: 45.0,
                color: Color::WHITE,
            },
            shadows_enabled: false,
            render_settings: PuzzleBevy3dRenderSettings::default(),
        };
        queue_3d.submit(left.clone(), view(0, 0), &frame).unwrap();
        queue_3d
            .submit(right.clone(), view(1, 320), &frame)
            .unwrap();

        reconcile_view_camera_orders(
            &mut queue_2d,
            &mut queue_3d,
            &BTreeMap::from([(left.clone(), 1), (right.clone(), 0)]),
        )
        .unwrap();
        queue_3d.submit(left, view(1, 0), &frame).unwrap();
        queue_3d.submit(right, view(0, 320), &frame).unwrap();
    }

    #[test]
    fn viewport_camera_order_follows_surface_stack_before_source_ids() {
        fn replace_viewport_source(
            components: &mut [RuntimeResolvedSceneComponent],
            replacement: &RuntimeViewportSourceId,
        ) -> bool {
            for component in components {
                match component {
                    RuntimeResolvedSceneComponent::Viewport { source, .. } => {
                        *source = replacement.clone();
                        return true;
                    }
                    RuntimeResolvedSceneComponent::Row { children, .. }
                    | RuntimeResolvedSceneComponent::Column { children, .. }
                    | RuntimeResolvedSceneComponent::Box { children, .. } => {
                        if replace_viewport_source(children, replacement) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        }

        let host =
            PuzzleBevyPlayerHost::from_image_free_source(TENETEN3D, "games/TENETEN3D.puzzle")
                .unwrap();
        let mut snapshot = host.snapshot().clone();
        let original_source = snapshot.viewport_sources.keys().next().unwrap().clone();
        let renderer = snapshot.viewport_sources[&original_source].clone();
        let original_component = snapshot
            .surface
            .components
            .iter()
            .find(|component| {
                matches!(
                    &component.presentation,
                    RuntimeComponentPresentation::Ready(_)
                )
            })
            .unwrap()
            .clone();
        let root_source = RuntimeViewportSourceId {
            model: original_source.model.clone(),
            component: "root_component".to_string(),
            source: "zzz_root".to_string(),
        };
        let overlay_source = RuntimeViewportSourceId {
            model: original_source.model.clone(),
            component: "overlay_component".to_string(),
            source: "zzz_overlay_first".to_string(),
        };
        let overlay_second_source = RuntimeViewportSourceId {
            model: original_source.model.clone(),
            component: "overlay_second_component".to_string(),
            source: "aaa_overlay_second".to_string(),
        };
        let modal_source = RuntimeViewportSourceId {
            model: original_source.model.clone(),
            component: "modal_component".to_string(),
            source: "000_modal".to_string(),
        };
        let mut root_component = original_component.clone();
        root_component.id = root_source.component.clone();
        root_component.placement = puzzle_scene::ComponentPlacement::Root;
        let RuntimeComponentPresentation::Ready(root_scene) = &mut root_component.presentation
        else {
            unreachable!()
        };
        assert!(replace_viewport_source(
            &mut root_scene.components,
            &root_source
        ));
        let mut overlay_component = original_component.clone();
        overlay_component.id = overlay_source.component.clone();
        overlay_component.placement = puzzle_scene::ComponentPlacement::Overlay;
        let RuntimeComponentPresentation::Ready(overlay_scene) =
            &mut overlay_component.presentation
        else {
            unreachable!()
        };
        assert!(replace_viewport_source(
            &mut overlay_scene.components,
            &overlay_source
        ));
        let mut overlay_second_component = original_component.clone();
        overlay_second_component.id = overlay_second_source.component.clone();
        overlay_second_component.placement = puzzle_scene::ComponentPlacement::Overlay;
        let RuntimeComponentPresentation::Ready(overlay_second_scene) =
            &mut overlay_second_component.presentation
        else {
            unreachable!()
        };
        assert!(replace_viewport_source(
            &mut overlay_second_scene.components,
            &overlay_second_source
        ));
        let mut modal_component = original_component;
        modal_component.id = modal_source.component.clone();
        modal_component.placement = puzzle_scene::ComponentPlacement::Content;
        modal_component.modal = true;
        let RuntimeComponentPresentation::Ready(modal_scene) = &mut modal_component.presentation
        else {
            unreachable!()
        };
        assert!(replace_viewport_source(
            &mut modal_scene.components,
            &modal_source
        ));
        snapshot.surface.components = vec![
            modal_component,
            overlay_component,
            root_component,
            overlay_second_component,
        ];
        snapshot.viewport_sources.clear();
        snapshot
            .viewport_sources
            .insert(root_source.clone(), renderer.clone());
        snapshot
            .viewport_sources
            .insert(overlay_source.clone(), renderer.clone());
        snapshot
            .viewport_sources
            .insert(overlay_second_source.clone(), renderer.clone());
        snapshot
            .viewport_sources
            .insert(modal_source.clone(), renderer);

        let projected = projected_viewports(&snapshot, 0.0).unwrap();
        assert_eq!(projected[&root_source].order, 0);
        assert_eq!(projected[&overlay_source].order, 1);
        assert_eq!(projected[&overlay_second_source].order, 2);
        assert_eq!(projected[&modal_source].order, 3);
    }
}
