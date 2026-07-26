use std::collections::VecDeque;
#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

use puzzle_runtime_contract::{RuntimePresentationEvent, SessionAction};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmStandaloneSession {
    inner: puzzle_game_runtime::RuntimeSession,
    visual_images: std::sync::Arc<puzzle_assets::DecodedVisualImageCatalog>,
    progress_storage: Option<puzzle_runtime_contract::StandaloneProgressStorage>,
    presentation_timeline: HostPresentationTimeline,
    #[cfg(target_arch = "wasm32")]
    audio_runtime: puzzle_audio::AudioRuntime,
    #[cfg(target_arch = "wasm32")]
    audio_backend: Rc<RefCell<puzzle_web_audio::BrowserAudioBackend>>,
    #[cfg(target_arch = "wasm32")]
    pending_audio_diagnostics: Rc<RefCell<Vec<String>>>,
    #[cfg(target_arch = "wasm32")]
    audio_feedback_wakeup: Rc<RefCell<Option<js_sys::Function>>>,
}

#[wasm_bindgen]
impl WasmStandaloneSession {
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str, puzzle_path: &str) -> Result<WasmStandaloneSession, JsValue> {
        Self::from_runtime(
            puzzle_game_runtime::RuntimeSession::from_source(source, puzzle_path)
                .map_err(|error| JsValue::from_str(&error))?,
        )
    }

    #[wasm_bindgen(js_name = fromExport)]
    pub fn from_export(export_json: &str) -> Result<WasmStandaloneSession, JsValue> {
        let (runtime, visual_images, progress_storage) =
            puzzle_player_bootstrap::decode_standalone_player_export(export_json)
                .map_err(|error| JsValue::from_str(&error.to_string()))?
                .into_parts();
        Self::from_runtime_parts(runtime, visual_images, Some(progress_storage))
    }

    pub fn development_snapshot(&self) -> Result<String, JsValue> {
        puzzle_presentation_json::to_string(&self.inner.development_snapshot()).map_err(|error| {
            JsValue::from_str(&format!(
                "development snapshot JSON could not be serialized: {error}"
            ))
        })
    }

    pub fn dispatch(&mut self, action_json: &str) -> Result<String, JsValue> {
        let action = serde_json::from_str::<SessionAction>(action_json)
            .map_err(|error| JsValue::from_str(&format!("invalid session action: {error}")))?;
        let mut snapshot = self
            .inner
            .dispatch_development_typed(action)
            .map_err(|error| JsValue::from_str(&error))?;
        self.project_snapshot_presentation(&mut snapshot.player);
        puzzle_presentation_json::to_string(&snapshot).map_err(|error| {
            JsValue::from_str(&format!(
                "development snapshot JSON could not be serialized: {error}"
            ))
        })
    }

    pub fn apply_debug_input_name(&mut self, input_name: &str) -> Result<String, JsValue> {
        let mut dispatch = self
            .inner
            .apply_debug_input_name(input_name)
            .map_err(|error| JsValue::from_str(&error))?;
        self.project_snapshot_presentation(&mut dispatch.snapshot);
        let snapshot = puzzle_presentation_json::to_value(
            &self.development_snapshot_with_player(dispatch.snapshot),
        )
        .map_err(|error| {
            JsValue::from_str(&format!(
                "debug development snapshot JSON could not be serialized: {error}"
            ))
        })?;
        Ok(serde_json::json!({
            "snapshot": snapshot,
            "debug": dispatch.debug,
        })
        .to_string())
    }

    pub fn set_current_state(
        &mut self,
        state_json: &str,
        level_index: u32,
        materialize_level_start: bool,
    ) -> Result<(), JsValue> {
        let level_index = usize::try_from(level_index)
            .map_err(|_| JsValue::from_str("level index is out of range"))?;
        self.inner
            .set_current_state_json(state_json, level_index, materialize_level_start)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn resolve_render_moment(
        &self,
        render_scene_json: &str,
        render_moment_json: &str,
    ) -> Result<String, JsValue> {
        let scene = serde_json::from_str::<puzzle_runtime_contract::RuntimeResolvedRenderScene>(
            render_scene_json,
        )
        .map_err(|error| JsValue::from_str(&format!("invalid resolved render scene: {error}")))?;
        let moment = serde_json::from_str::<puzzle_runtime_contract::RuntimeResolvedRenderMoment>(
            render_moment_json,
        )
        .map_err(|error| JsValue::from_str(&format!("invalid resolved render moment: {error}")))?;
        let frame =
            puzzle_presentation::resolve_render_moment(&scene, &self.visual_images, &moment)
                .map_err(|error| {
                    JsValue::from_str(&format!("render moment resolution failed: {error:?}"))
                })?;
        serde_json::to_string(&frame).map_err(|error| {
            JsValue::from_str(&format!("render frame serialization failed: {error}"))
        })
    }

    pub fn progress_save(&self) -> String {
        self.inner.progress_save_json()
    }

    pub fn restore_progress_save(&mut self, save_json: &str) -> Result<(), JsValue> {
        self.inner
            .restore_progress_save_json(save_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn set_progress_persistence_enabled(&mut self, enabled: bool) {
        self.inner.set_progress_persistence_enabled(enabled);
    }

    pub fn progress_storage_key(&self) -> Option<String> {
        self.progress_storage
            .as_ref()
            .map(|storage| storage.key.clone())
    }

    pub fn progress_storage_save_version(&self) -> Option<u32> {
        self.progress_storage
            .as_ref()
            .map(|storage| storage.save_version)
    }

    pub fn visual_image_count(&self) -> u32 {
        u32::try_from(self.visual_images.iter().count())
            .expect("standalone visual image count must fit the browser contract")
    }

    pub fn presentation_event_consumed(&mut self, now_seconds: f64) -> Result<String, JsValue> {
        let audio_commands = self
            .presentation_timeline
            .consume_presented_event()
            .map_err(|error| JsValue::from_str(&error))?;
        self.consume_audio_commands(audio_commands, now_seconds);
        Ok(self.take_pending_audio_diagnostics())
    }

    pub fn presentation_frame(&mut self, now_seconds: f64) -> String {
        let audio_commands = self.presentation_timeline.begin_presented_frame();
        self.consume_audio_commands(audio_commands, now_seconds);
        self.take_pending_audio_diagnostics()
    }

    pub fn progress_save_request(&self) -> String {
        serde_json::to_string(&self.inner.progress_save_request())
            .expect("progress save request JSON should serialize")
    }

    pub fn confirm_progress_persistence_applied(&mut self, request_id: u32) -> Result<(), JsValue> {
        self.inner
            .confirm_progress_persistence_applied(request_id)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn unlock_audio(&mut self, now_seconds: f64) -> Result<String, JsValue> {
        let unlock_start = { self.audio_backend.borrow_mut().begin_unlock() };
        let unlock = match unlock_start {
            Ok(unlock) => unlock,
            Err(error) => {
                let commands = self
                    .audio_runtime
                    .report_device_failure(error.to_string(), audio_frame(now_seconds));
                self.consume_audio_device_commands(commands, audio_frame(now_seconds));
                return Ok(self.take_pending_audio_diagnostics());
            }
        };
        let capability = match unlock {
            puzzle_web_audio::BrowserAudioUnlockStart::Started(task) => {
                let completion = task.await;
                let unlock_result = self.audio_backend.borrow_mut().finish_unlock(completion);
                match unlock_result {
                    Ok(capability) => capability,
                    Err(error) => {
                        let commands = self
                            .audio_runtime
                            .report_device_failure(error.to_string(), audio_frame(now_seconds));
                        self.consume_audio_device_commands(commands, audio_frame(now_seconds));
                        return Ok(self.take_pending_audio_diagnostics());
                    }
                }
            }
            puzzle_web_audio::BrowserAudioUnlockStart::InFlight => {
                self.audio_backend.borrow().capability()
            }
            puzzle_web_audio::BrowserAudioUnlockStart::Ready(capability) => capability,
        };
        let commands = self
            .audio_runtime
            .set_capability(capability, audio_frame(now_seconds));
        self.consume_audio_device_commands(commands, audio_frame(now_seconds));
        Ok(self.take_pending_audio_diagnostics())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_audio_feedback_wakeup(&self, callback: js_sys::Function) {
        *self.audio_feedback_wakeup.borrow_mut() = Some(callback);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn audio_feedback_event(&mut self, now_seconds: f64) -> String {
        let commands = self.reconcile_audio_feedback(now_seconds);
        self.consume_audio_device_commands(commands, audio_frame(now_seconds));
        self.take_pending_audio_diagnostics()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn audio_tick(&mut self, now_seconds: f64) -> String {
        let commands = self.reconcile_audio_feedback(now_seconds);
        self.consume_audio_device_commands(commands, audio_frame(now_seconds));
        self.take_pending_audio_diagnostics()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_audio_visible(&mut self, visible: bool, now_seconds: f64) -> String {
        let capability = self.audio_backend.borrow_mut().set_visible(visible);
        let commands = self
            .audio_runtime
            .set_capability(capability, audio_frame(now_seconds));
        self.consume_audio_device_commands(commands, audio_frame(now_seconds));
        self.take_pending_audio_diagnostics()
    }
}

impl WasmStandaloneSession {
    fn development_snapshot_with_player(
        &self,
        player: puzzle_session_contract::RuntimeSessionSnapshot,
    ) -> puzzle_session_contract::RuntimeDevelopmentSessionSnapshot {
        self.inner.development_snapshot_from_player(player)
    }

    fn project_snapshot_presentation(
        &mut self,
        snapshot: &mut puzzle_session_contract::RuntimeSessionSnapshot,
    ) {
        snapshot.presentation_events = self
            .presentation_timeline
            .push(std::mem::take(&mut snapshot.presentation_events));
    }

    fn from_runtime(inner: puzzle_game_runtime::RuntimeSession) -> Result<Self, JsValue> {
        Self::from_runtime_parts(
            inner,
            std::sync::Arc::new(puzzle_assets::DecodedVisualImageCatalog::default()),
            None,
        )
    }

    fn from_runtime_parts(
        inner: puzzle_game_runtime::RuntimeSession,
        visual_images: std::sync::Arc<puzzle_assets::DecodedVisualImageCatalog>,
        progress_storage: Option<puzzle_runtime_contract::StandaloneProgressStorage>,
    ) -> Result<Self, JsValue> {
        #[cfg(target_arch = "wasm32")]
        {
            let catalog = inner.audio_catalog();
            let audio_backend = Rc::new(RefCell::new(puzzle_web_audio::BrowserAudioBackend::new(
                catalog.clone(),
            )));
            let pending_audio_diagnostics = Rc::new(RefCell::new(Vec::new()));
            let audio_feedback_wakeup = Rc::new(RefCell::new(None));
            install_audio_feedback_wakeup(&audio_backend, &audio_feedback_wakeup);
            start_music_worklet_load(
                Rc::clone(&audio_backend),
                Rc::clone(&pending_audio_diagnostics),
                Rc::clone(&audio_feedback_wakeup),
            );
            let audio_runtime =
                puzzle_audio::AudioRuntime::new(catalog, audio_backend.borrow().capability());
            Ok(Self {
                inner,
                visual_images,
                progress_storage,
                presentation_timeline: HostPresentationTimeline::default(),
                audio_runtime,
                audio_backend,
                pending_audio_diagnostics,
                audio_feedback_wakeup,
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(Self {
                inner,
                visual_images,
                progress_storage,
                presentation_timeline: HostPresentationTimeline::default(),
            })
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn consume_audio_commands(
        &mut self,
        audio_commands: Vec<puzzle_audio::AudioCommand>,
        now_seconds: f64,
    ) {
        let now_frame = audio_frame(now_seconds);
        let mut device_commands = self.reconcile_audio_feedback(now_seconds);
        for command in audio_commands {
            device_commands.extend(self.audio_runtime.apply(command, now_frame));
        }
        self.consume_audio_device_commands(device_commands, now_frame);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn consume_audio_commands(
        &mut self,
        _audio_commands: Vec<puzzle_audio::AudioCommand>,
        _now_seconds: f64,
    ) {
    }

    #[cfg(target_arch = "wasm32")]
    fn take_pending_audio_diagnostics(&mut self) -> String {
        self.pending_audio_diagnostics.borrow_mut().extend(
            self.audio_runtime
                .take_diagnostics()
                .into_iter()
                .map(|diagnostic| format!("{diagnostic:?}")),
        );
        serde_json::to_string(&std::mem::take(
            &mut *self.pending_audio_diagnostics.borrow_mut(),
        ))
        .expect("audio diagnostic strings should serialize")
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn take_pending_audio_diagnostics(&mut self) -> String {
        "[]".to_string()
    }

    #[cfg(target_arch = "wasm32")]
    fn reconcile_audio_feedback(
        &mut self,
        now_seconds: f64,
    ) -> Vec<puzzle_audio::AudioDeviceCommand> {
        let now_frame = audio_frame(now_seconds);
        let mut commands = Vec::new();
        for voice in self.audio_backend.borrow_mut().take_ended_voices() {
            self.audio_runtime.voice_ended(voice);
        }
        for error in self.audio_backend.borrow_mut().take_feedback_errors() {
            if let Some(voice) = error.voice_id() {
                self.audio_runtime
                    .report_voice_failure(voice, error.to_string(), now_frame);
            } else {
                commands.extend(
                    self.audio_runtime
                        .report_device_failure(error.to_string(), now_frame),
                );
            }
        }
        commands.extend(
            self.audio_runtime
                .set_capability(self.audio_backend.borrow().capability(), now_frame),
        );
        commands
    }

    #[cfg(target_arch = "wasm32")]
    fn consume_audio_device_commands(
        &mut self,
        commands: Vec<puzzle_audio::AudioDeviceCommand>,
        now_frame: u64,
    ) {
        for command in commands {
            if let Err(error) = self.audio_backend.borrow_mut().consume(command) {
                let message = error.to_string();
                if let Some(voice) = audio_command_voice(command).or_else(|| error.voice_id()) {
                    self.audio_runtime
                        .report_voice_failure(voice, message.clone(), now_frame);
                } else {
                    self.audio_runtime
                        .report_device_failure(message.clone(), now_frame);
                }
                self.pending_audio_diagnostics.borrow_mut().push(message);
            }
        }
    }
}

#[derive(Default)]
struct HostPresentationTimeline {
    pending: VecDeque<RuntimePresentationEvent>,
}

impl HostPresentationTimeline {
    fn push(&mut self, events: Vec<RuntimePresentationEvent>) -> Vec<RuntimePresentationEvent> {
        let host_events = events
            .iter()
            .filter(|event| !matches!(event, RuntimePresentationEvent::Audio { .. }))
            .cloned()
            .collect();
        self.pending.extend(events);
        host_events
    }

    fn begin_presented_frame(&mut self) -> Vec<puzzle_audio::AudioCommand> {
        self.take_ready_audio()
    }

    fn consume_presented_event(&mut self) -> Result<Vec<puzzle_audio::AudioCommand>, String> {
        match self.pending.front() {
            Some(RuntimePresentationEvent::Wait { .. })
            | Some(RuntimePresentationEvent::AnimationBatch { .. }) => {
                self.pending.pop_front();
                Ok(self.take_ready_audio())
            }
            Some(RuntimePresentationEvent::Audio { .. }) => {
                Err("audio timeline command was not consumed by its Rust owner".to_string())
            }
            None => {
                Err("presentation timeline has no event awaiting browser completion".to_string())
            }
        }
    }

    fn take_ready_audio(&mut self) -> Vec<puzzle_audio::AudioCommand> {
        let mut commands = Vec::new();
        while matches!(
            self.pending.front(),
            Some(RuntimePresentationEvent::Audio { .. })
        ) {
            let Some(RuntimePresentationEvent::Audio { command }) = self.pending.pop_front() else {
                unreachable!("timeline head was checked as audio")
            };
            commands.push(command);
        }
        commands
    }
}

#[cfg(target_arch = "wasm32")]
fn install_audio_feedback_wakeup(
    backend: &Rc<RefCell<puzzle_web_audio::BrowserAudioBackend>>,
    callback: &Rc<RefCell<Option<js_sys::Function>>>,
) {
    let callback = Rc::clone(callback);
    backend.borrow().set_feedback_wakeup(Rc::new(move || {
        wake_audio_feedback_owner(&callback);
    }));
}

#[cfg(target_arch = "wasm32")]
fn start_music_worklet_load(
    backend: Rc<RefCell<puzzle_web_audio::BrowserAudioBackend>>,
    diagnostics: Rc<RefCell<Vec<String>>>,
    wakeup: Rc<RefCell<Option<js_sys::Function>>>,
) {
    let start = backend.borrow_mut().begin_worklet_load();
    match start {
        Ok(puzzle_web_audio::BrowserAudioWorkletStart::Started(task)) => {
            wasm_bindgen_futures::spawn_local(async move {
                let completion = task.await;
                if let Err(error) = backend.borrow_mut().finish_worklet_load(completion) {
                    diagnostics.borrow_mut().push(error.to_string());
                }
                wake_audio_feedback_owner(&wakeup);
            });
        }
        Ok(
            puzzle_web_audio::BrowserAudioWorkletStart::InFlight
            | puzzle_web_audio::BrowserAudioWorkletStart::Ready,
        ) => {}
        Err(error) => {
            diagnostics.borrow_mut().push(error.to_string());
            wake_audio_feedback_owner(&wakeup);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn wake_audio_feedback_owner(callback: &Rc<RefCell<Option<js_sys::Function>>>) {
    if let Some(callback) = callback.borrow().as_ref() {
        callback
            .call0(&JsValue::UNDEFINED)
            .expect("registered browser audio feedback wakeup failed");
    }
}

#[cfg(target_arch = "wasm32")]
fn audio_command_voice(
    command: puzzle_audio::AudioDeviceCommand,
) -> Option<puzzle_audio::AudioVoiceId> {
    match command {
        puzzle_audio::AudioDeviceCommand::StartSfx { voice, .. }
        | puzzle_audio::AudioDeviceCommand::StartMusic { voice, .. }
        | puzzle_audio::AudioDeviceCommand::PauseVoice { voice, .. }
        | puzzle_audio::AudioDeviceCommand::ResumeVoice { voice, .. }
        | puzzle_audio::AudioDeviceCommand::StopVoice { voice } => Some(voice),
    }
}

#[cfg(target_arch = "wasm32")]
fn audio_frame(now_seconds: f64) -> u64 {
    (now_seconds.max(0.0) * f64::from(puzzle_audio::CANONICAL_AUDIO_SAMPLE_RATE))
        .floor()
        .min(u64::MAX as f64) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use puzzle_audio::{AudioCommand, SfxAssetId};
    use serde_json::json;

    #[test]
    fn editor_runtime_resolves_typed_render_moments_with_owned_assets() {
        let source = r#"
const title = render_moment

puzzle board {
layers {
actor = Player
}
empty .
rules {
[ Player ] -> [ Player ]
}
levels {
legend {
. = empty
P = Player
}
level "first" {
P
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
        let session = WasmStandaloneSession::new(source, "render_moment.puzzle")
            .expect("editor runtime fixture");
        let snapshot: serde_json::Value =
            serde_json::from_str(&session.development_snapshot().unwrap()).unwrap();
        let render_scene = snapshot["viewportSources"][0]["state"]["renderScene"].clone();
        assert!(render_scene.is_object());

        let frame: serde_json::Value = serde_json::from_str(
            &session
                .resolve_render_moment(
                    &render_scene.to_string(),
                    &json!({
                        "clipElapsedMs": 0,
                        "animationElapsedMs": 0,
                        "animations": []
                    })
                    .to_string(),
                )
                .expect("resolve typed render moment"),
        )
        .unwrap();
        assert!(frame["batches"].is_array());
        assert!(frame["decorations"].is_array());
        assert_eq!(frame["continueAnimation"], false);
    }

    #[test]
    fn editor_timeline_keeps_audio_out_of_snapshot_json() {
        let mut timeline = HostPresentationTimeline::default();
        let public = timeline.push(vec![
            RuntimePresentationEvent::Audio {
                command: AudioCommand::PlaySfx {
                    asset: SfxAssetId(1),
                },
            },
            RuntimePresentationEvent::Wait { milliseconds: 10 },
        ]);

        assert_eq!(
            timeline.begin_presented_frame(),
            vec![AudioCommand::PlaySfx {
                asset: SfxAssetId(1)
            }]
        );
        assert!(
            !serde_json::to_string(&public)
                .unwrap()
                .contains("\"audio\"")
        );
    }
}
