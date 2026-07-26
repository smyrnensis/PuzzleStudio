use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    error::Error,
    fmt,
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
    sync::Arc,
};

use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, AudioContextState, AudioScheduledSourceNode,
    AudioWorkletNode, AudioWorkletNodeOptions, Event, GainNode, MessageEvent,
};

use puzzle_audio::{
    AudioAssetCatalog, AudioCapabilityState, AudioDeviceCommand, AudioDeviceStateError,
    AudioDeviceVoiceRegistry, AudioVoiceId, MusicAssetId, SfxAssetId, encode_music_worklet_asset,
};

const MUSIC_WORKLET_SOURCE: &str = include_str!("../generated/puzzle_audio_worklet.js");
const MUSIC_WORKLET_PROCESSOR: &str = "puzzle-music-processor-v1";
const MUSIC_WORKLET_READY_DEADLINE_MILLISECONDS: i32 = 2_000;

/// A concrete failure at the browser-device boundary.
///
/// These failures are intended to become audio diagnostics. They do not imply
/// that the game session itself is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserAudioError {
    ContextUnavailable {
        error: String,
    },
    ContextClosed,
    UnlockAttemptExhausted,
    UnlockCompletionOutOfOrder {
        expected: Option<u64>,
        received: u64,
    },
    UnlockCompletionOwnerMismatch {
        received: u64,
    },
    WorkletCompletionNotInFlight,
    WorkletCompletionOwnerMismatch,
    CatalogReplacementWhileVoicesAreActive,
    CapabilityNotReady {
        state: AudioCapabilityState,
    },
    DeviceState(AudioDeviceStateError),
    MissingSfxAsset {
        voice: AudioVoiceId,
        asset: SfxAssetId,
    },
    MissingSfxGain {
        voice: AudioVoiceId,
        asset: SfxAssetId,
    },
    MissingMusicAsset {
        voice: AudioVoiceId,
        asset: MusicAssetId,
    },
    MissingMusicGain {
        voice: AudioVoiceId,
        asset: MusicAssetId,
    },
    MissingVoice {
        voice: AudioVoiceId,
    },
    WebAudioOperation {
        operation: &'static str,
        error: String,
    },
    VoiceOperation {
        voice: AudioVoiceId,
        operation: &'static str,
        error: String,
    },
    MusicCallback {
        voice: AudioVoiceId,
        asset: MusicAssetId,
        operation: &'static str,
        error: String,
    },
}

impl BrowserAudioError {
    pub fn voice_id(&self) -> Option<AudioVoiceId> {
        match self {
            Self::DeviceState(AudioDeviceStateError::DuplicateVoice(voice))
            | Self::DeviceState(AudioDeviceStateError::MissingVoice(voice))
            | Self::DeviceState(AudioDeviceStateError::NonFiniteGain { voice, .. })
            | Self::DeviceState(AudioDeviceStateError::NegativeGain { voice, .. })
            | Self::MissingSfxAsset { voice, .. }
            | Self::MissingSfxGain { voice, .. }
            | Self::MissingMusicAsset { voice, .. }
            | Self::MissingMusicGain { voice, .. }
            | Self::MissingVoice { voice }
            | Self::VoiceOperation { voice, .. }
            | Self::MusicCallback { voice, .. } => Some(*voice),
            Self::ContextUnavailable { .. }
            | Self::ContextClosed
            | Self::UnlockAttemptExhausted
            | Self::UnlockCompletionOutOfOrder { .. }
            | Self::UnlockCompletionOwnerMismatch { .. }
            | Self::WorkletCompletionNotInFlight
            | Self::WorkletCompletionOwnerMismatch
            | Self::CatalogReplacementWhileVoicesAreActive
            | Self::CapabilityNotReady { .. }
            | Self::WebAudioOperation { .. } => None,
        }
    }
}

impl fmt::Display for BrowserAudioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContextUnavailable { error } => {
                write!(formatter, "Web Audio context is unavailable: {error}")
            }
            Self::ContextClosed => write!(formatter, "Web Audio context is closed"),
            Self::UnlockAttemptExhausted => {
                write!(formatter, "Web Audio unlock attempt identity is exhausted")
            }
            Self::UnlockCompletionOutOfOrder { expected, received } => write!(
                formatter,
                "Web Audio unlock completion {received} does not match the in-flight attempt {expected:?}"
            ),
            Self::UnlockCompletionOwnerMismatch { received } => write!(
                formatter,
                "Web Audio unlock completion {received} belongs to a different backend"
            ),
            Self::WorkletCompletionNotInFlight => {
                write!(
                    formatter,
                    "Web Audio worklet completion has no in-flight load"
                )
            }
            Self::WorkletCompletionOwnerMismatch => {
                write!(
                    formatter,
                    "Web Audio worklet completion belongs to a different backend"
                )
            }
            Self::CatalogReplacementWhileVoicesAreActive => write!(
                formatter,
                "Web Audio catalog replacement requires every voice to be stopped explicitly"
            ),
            Self::CapabilityNotReady { state } => {
                write!(formatter, "Web Audio output is not ready ({state:?})")
            }
            Self::DeviceState(error) => {
                write!(
                    formatter,
                    "Web Audio device command state is invalid: {error:?}"
                )
            }
            Self::MissingSfxAsset { asset, .. } => {
                write!(formatter, "audio catalog has no SFX asset {asset:?}")
            }
            Self::MissingSfxGain { asset, .. } => {
                write!(
                    formatter,
                    "audio catalog has no gain for SFX asset {asset:?}"
                )
            }
            Self::MissingMusicAsset { asset, .. } => {
                write!(formatter, "audio catalog has no music asset {asset:?}")
            }
            Self::MissingMusicGain { asset, .. } => {
                write!(
                    formatter,
                    "audio catalog has no gain for music asset {asset:?}"
                )
            }
            Self::MissingVoice { voice } => {
                write!(formatter, "Web Audio has no voice {voice:?}")
            }
            Self::WebAudioOperation { operation, error } => {
                write!(formatter, "Web Audio `{operation}` failed: {error}")
            }
            Self::VoiceOperation {
                voice,
                operation,
                error,
            } => write!(
                formatter,
                "Web Audio `{operation}` failed for voice {voice:?}: {error}"
            ),
            Self::MusicCallback {
                voice,
                asset,
                operation,
                error,
            } => write!(
                formatter,
                "Web Audio music callback `{operation}` failed for voice {voice:?}, asset {asset:?}: {error}"
            ),
        }
    }
}

impl Error for BrowserAudioError {}

struct ActiveSfxVoice {
    source: AudioBufferSourceNode,
    _gain: GainNode,
    _on_ended: Closure<dyn FnMut()>,
}

struct ActiveMusicVoice {
    node: AudioWorkletNode,
    gain: GainNode,
    state: MusicVoiceState,
    ready_deadline_window: web_sys::Window,
    ready_deadline_id: i32,
    _on_ready_deadline: Closure<dyn FnMut()>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_processor_error: Closure<dyn FnMut(Event)>,
}

impl ActiveMusicVoice {
    fn cancel_ready_deadline(&mut self) {
        if self.ready_deadline_id >= 0 {
            self.ready_deadline_window
                .clear_timeout_with_handle(self.ready_deadline_id);
            self.ready_deadline_id = -1;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MusicVoiceMaterialization {
    Pending(AudioDeviceCommand),
    Committed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MusicVoiceDesiredState {
    Playing,
    Paused,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MusicVoiceState {
    materialization: MusicVoiceMaterialization,
    desired: MusicVoiceDesiredState,
    ready: bool,
    resume_acknowledged: bool,
    connected: bool,
}

impl MusicVoiceState {
    fn pending(start: AudioDeviceCommand) -> Self {
        Self {
            materialization: MusicVoiceMaterialization::Pending(start),
            desired: MusicVoiceDesiredState::Playing,
            ready: false,
            resume_acknowledged: true,
            connected: false,
        }
    }

    fn is_pending(self) -> bool {
        matches!(self.materialization, MusicVoiceMaterialization::Pending(_))
    }

    fn request_pause(&mut self) {
        self.desired = MusicVoiceDesiredState::Paused;
        self.resume_acknowledged = false;
    }

    fn request_resume(&mut self) {
        self.desired = MusicVoiceDesiredState::Playing;
        self.resume_acknowledged = false;
    }

    fn acknowledge_ready(&mut self) {
        self.ready = true;
    }

    fn acknowledge_resume(&mut self) {
        self.resume_acknowledged = true;
    }

    fn should_connect(self) -> bool {
        self.ready
            && self.desired == MusicVoiceDesiredState::Playing
            && self.resume_acknowledged
            && !self.connected
    }

    fn pending_start(self) -> Option<AudioDeviceCommand> {
        match self.materialization {
            MusicVoiceMaterialization::Pending(start) => Some(start),
            MusicVoiceMaterialization::Committed => None,
        }
    }

    fn commit(&mut self) {
        self.materialization = MusicVoiceMaterialization::Committed;
    }
}

enum MusicWorkletFeedback {
    Ready { voice: AudioVoiceId },
    ResumeAcknowledged { voice: AudioVoiceId },
    Failed(BrowserAudioError),
}

#[derive(Default)]
struct FeedbackWakeup {
    callback: RefCell<Option<Rc<dyn Fn()>>>,
}

impl FeedbackWakeup {
    fn set(&self, callback: Rc<dyn Fn()>) {
        *self.callback.borrow_mut() = Some(callback);
    }

    fn notify(&self) {
        let callback = self.callback.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}

fn push_worklet_feedback(
    feedback: &Rc<RefCell<Vec<MusicWorkletFeedback>>>,
    wakeup: &Rc<FeedbackWakeup>,
    event: MusicWorkletFeedback,
) {
    feedback.borrow_mut().push(event);
    wakeup.notify();
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum WorkletLoadState {
    NotAttempted,
    Loading,
    Loaded,
    Failed(String),
}

impl WorkletLoadState {
    fn begin_attempt(&mut self) -> bool {
        if *self != Self::NotAttempted {
            return false;
        }
        *self = Self::Loading;
        true
    }

    fn finish(&mut self, result: Result<(), BrowserAudioError>) {
        *self = match result {
            Ok(()) => Self::Loaded,
            Err(error) => Self::Failed(error.to_string()),
        };
    }

    fn require_loaded(&self, voice: AudioVoiceId) -> Result<(), BrowserAudioError> {
        match self {
            Self::Loaded => Ok(()),
            Self::Failed(error) => Err(BrowserAudioError::VoiceOperation {
                voice,
                operation: "AudioWorklet.addModule",
                error: error.clone(),
            }),
            Self::NotAttempted | Self::Loading => Err(BrowserAudioError::VoiceOperation {
                voice,
                operation: "AudioWorklet.addModule",
                error: "music worklet has not finished its one-time load attempt".to_string(),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnlockReservation {
    Start(u64),
    InFlight,
    Ready,
}

#[derive(Debug)]
struct UnlockLifecycle {
    unlocked: bool,
    in_flight: Option<u64>,
    next_attempt: u64,
}

impl Default for UnlockLifecycle {
    fn default() -> Self {
        Self {
            unlocked: false,
            in_flight: None,
            next_attempt: 1,
        }
    }
}

impl UnlockLifecycle {
    fn reserve(&mut self, ready: bool) -> Result<UnlockReservation, BrowserAudioError> {
        if self.in_flight.is_some() {
            return Ok(UnlockReservation::InFlight);
        }
        if self.unlocked && ready {
            return Ok(UnlockReservation::Ready);
        }
        let attempt = self.next_attempt;
        self.next_attempt = self
            .next_attempt
            .checked_add(1)
            .ok_or(BrowserAudioError::UnlockAttemptExhausted)?;
        self.in_flight = Some(attempt);
        Ok(UnlockReservation::Start(attempt))
    }

    fn finish(&mut self, attempt: u64, succeeded: bool) -> Result<(), BrowserAudioError> {
        let expected = self.in_flight;
        if expected != Some(attempt) {
            return Err(BrowserAudioError::UnlockCompletionOutOfOrder {
                expected,
                received: attempt,
            });
        }
        self.in_flight = None;
        self.unlocked |= succeeded;
        Ok(())
    }

    fn is_unlocked(&self) -> bool {
        self.unlocked
    }
}

/// Result of starting Web Audio unlock from a browser gesture callback.
///
/// `Started` contains a non-borrowing `'static` future suitable for the host's
/// local async executor. `InFlight` makes repeated gesture delivery
/// idempotent. `Ready` means no asynchronous work is required.
pub enum BrowserAudioUnlockStart {
    Started(BrowserAudioUnlockTask),
    InFlight,
    Ready(AudioCapabilityState),
}

/// A non-`Send`, browser-local unlock future which owns every Web API handle it
/// needs and does not borrow [`BrowserAudioBackend`].
pub type BrowserAudioUnlockTask =
    Pin<Box<dyn Future<Output = BrowserAudioUnlockCompletion> + 'static>>;

/// Typed completion accepted only by the backend that owns its attempt.
pub struct BrowserAudioUnlockCompletion {
    owner: Rc<()>,
    attempt: u64,
    result: UnlockTaskResult,
}

enum UnlockTaskResult {
    ResumeFailed(BrowserAudioError),
    Resumed,
}

/// Result of starting the one-time music worklet module load.
///
/// This lifecycle is intentionally separate from [`BrowserAudioUnlockStart`]:
/// SFX capability is owned only by `AudioContext.resume()`, while music owns
/// the additional worklet readiness requirement.
pub enum BrowserAudioWorkletStart {
    Started(BrowserAudioWorkletTask),
    InFlight,
    Ready,
}

pub type BrowserAudioWorkletTask =
    Pin<Box<dyn Future<Output = BrowserAudioWorkletCompletion> + 'static>>;

pub struct BrowserAudioWorkletCompletion {
    owner: Rc<()>,
    result: Result<(), BrowserAudioError>,
}

#[derive(Default)]
struct VoiceRegistry {
    active: HashMap<AudioVoiceId, ActiveSfxVoice>,
    ended: BTreeSet<AudioVoiceId>,
}

impl VoiceRegistry {
    fn mark_ended(&mut self, voice: AudioVoiceId) {
        if self.active.contains_key(&voice) {
            self.ended.insert(voice);
        }
    }

    fn take(&mut self, voice: AudioVoiceId) -> Option<(ActiveSfxVoice, bool)> {
        let ended = self.ended.remove(&voice);
        let active = self.active.remove(&voice)?;
        Some((active, ended))
    }

    fn take_ended(&mut self) -> Vec<AudioVoiceId> {
        let ended = std::mem::take(&mut self.ended);
        ended
            .into_iter()
            .filter_map(|voice| {
                let active = self.active.remove(&voice)?;
                active
                    .source
                    .unchecked_ref::<AudioScheduledSourceNode>()
                    .set_onended(None);
                Some(voice)
            })
            .collect()
    }
}

/// Rust-owned Web Audio device adapter.
///
/// `AudioRuntime` remains the owner of lifecycle policy and command ordering.
/// This type only materializes resolved device commands. It deliberately does
/// not inspect authoring recipes, source text, or asset names.
///
/// Browser music is rendered by a dedicated Rust/WASM AudioWorklet. Its small
/// JavaScript module only registers the processor, transports typed messages,
/// and copies Rust-rendered channel buffers.
pub struct BrowserAudioBackend {
    catalog: Arc<AudioAssetCatalog>,
    context: Option<AudioContext>,
    initialization_error: Option<String>,
    unlock: UnlockLifecycle,
    unlock_owner: Rc<()>,
    worklet_load: WorkletLoadState,
    worklet_owner: Rc<()>,
    visible: bool,
    sfx_buffers: HashMap<SfxAssetId, AudioBuffer>,
    voices: Rc<RefCell<VoiceRegistry>>,
    music_voices: HashMap<AudioVoiceId, ActiveMusicVoice>,
    worklet_feedback: Rc<RefCell<Vec<MusicWorkletFeedback>>>,
    feedback_wakeup: Rc<FeedbackWakeup>,
    context_state_wakeup: Rc<FeedbackWakeup>,
    _on_context_state_change: Option<Closure<dyn FnMut(Event)>>,
    device_voices: AudioDeviceVoiceRegistry,
}

impl BrowserAudioBackend {
    pub fn new(catalog: Arc<AudioAssetCatalog>) -> Self {
        let context_state_wakeup = Rc::new(FeedbackWakeup::default());
        match AudioContext::new() {
            Ok(context) => {
                let state_wakeup = Rc::clone(&context_state_wakeup);
                let on_context_state_change =
                    Closure::wrap(Box::new(move |_event: Event| state_wakeup.notify())
                        as Box<dyn FnMut(Event)>);
                context.set_onstatechange(Some(on_context_state_change.as_ref().unchecked_ref()));
                Self {
                    catalog,
                    context: Some(context),
                    initialization_error: None,
                    unlock: UnlockLifecycle::default(),
                    unlock_owner: Rc::new(()),
                    worklet_load: WorkletLoadState::NotAttempted,
                    worklet_owner: Rc::new(()),
                    visible: true,
                    sfx_buffers: HashMap::new(),
                    voices: Rc::new(RefCell::new(VoiceRegistry::default())),
                    music_voices: HashMap::new(),
                    worklet_feedback: Rc::new(RefCell::new(Vec::new())),
                    feedback_wakeup: Rc::new(FeedbackWakeup::default()),
                    context_state_wakeup,
                    _on_context_state_change: Some(on_context_state_change),
                    device_voices: AudioDeviceVoiceRegistry::default(),
                }
            }
            Err(error) => Self {
                catalog,
                context: None,
                initialization_error: Some(js_error(error)),
                unlock: UnlockLifecycle::default(),
                unlock_owner: Rc::new(()),
                worklet_load: WorkletLoadState::NotAttempted,
                worklet_owner: Rc::new(()),
                visible: true,
                sfx_buffers: HashMap::new(),
                voices: Rc::new(RefCell::new(VoiceRegistry::default())),
                music_voices: HashMap::new(),
                worklet_feedback: Rc::new(RefCell::new(Vec::new())),
                feedback_wakeup: Rc::new(FeedbackWakeup::default()),
                context_state_wakeup,
                _on_context_state_change: None,
                device_voices: AudioDeviceVoiceRegistry::default(),
            },
        }
    }

    pub fn capability(&self) -> AudioCapabilityState {
        let Some(context) = &self.context else {
            return AudioCapabilityState::Unavailable;
        };
        if self.initialization_error.is_some() {
            return AudioCapabilityState::Unavailable;
        }
        browser_context_capability(context.state(), self.visible, self.unlock.is_unlocked())
    }

    /// Installs the session-owned wakeup used by asynchronous Web Audio events.
    ///
    /// The callback does not carry untyped payloads. It only asks the owning
    /// Rust session to drain this backend's typed feedback queue.
    pub fn set_feedback_wakeup(&self, callback: Rc<dyn Fn()>) {
        self.feedback_wakeup.set(callback);
    }

    /// Installs the host wakeup for native `AudioContext.statechange` events.
    ///
    /// The callback carries no browser object or inferred policy. The host
    /// reads [`Self::capability`] and reconciles that typed state through its
    /// audio runtime.
    pub fn set_context_state_wakeup(&self, callback: Rc<dyn Fn()>) {
        self.context_state_wakeup.set(callback);
    }

    /// Records host visibility explicitly.
    ///
    /// The host uses the resulting capability transition to ask `AudioRuntime`
    /// for the ordered pause/reconcile commands. Visibility is not inferred by
    /// this backend and no DOM listener is installed here.
    pub fn set_visible(&mut self, visible: bool) -> AudioCapabilityState {
        self.visible = visible;
        self.capability()
    }

    /// Starts unlock synchronously inside the browser gesture callback.
    ///
    /// `AudioContext.resume()` is invoked before this method returns. The
    /// returned task owns its cloned context and promises, so the host can
    /// spawn it without borrowing this backend across an ECS frame.
    pub fn begin_unlock(&mut self) -> Result<BrowserAudioUnlockStart, BrowserAudioError> {
        let context = self.context()?.clone();
        let ready = self.capability() == AudioCapabilityState::Ready;
        let attempt = match self.unlock.reserve(ready)? {
            UnlockReservation::InFlight => return Ok(BrowserAudioUnlockStart::InFlight),
            UnlockReservation::Ready => {
                return Ok(BrowserAudioUnlockStart::Ready(self.capability()));
            }
            UnlockReservation::Start(attempt) => attempt,
        };
        let promise = match context.resume() {
            Ok(promise) => promise,
            Err(error) => {
                self.unlock.finish(attempt, false)?;
                return Err(web_error("AudioContext.resume", error));
            }
        };
        let owner = Rc::clone(&self.unlock_owner);
        let task = Box::pin(async move {
            let resume_result = JsFuture::from(promise)
                .await
                .map_err(|error| web_error("AudioContext.resume promise", error));
            let result = match resume_result {
                Ok(_) => UnlockTaskResult::Resumed,
                Err(error) => UnlockTaskResult::ResumeFailed(error),
            };
            BrowserAudioUnlockCompletion {
                owner,
                attempt,
                result,
            }
        });
        Ok(BrowserAudioUnlockStart::Started(task))
    }

    /// Commits one completed unlock attempt back into the device adapter.
    ///
    /// Resume rejection restores `Locked`, allowing a later explicit gesture
    /// to retry. A duplicate or stale completion is rejected rather than
    /// changing capability state.
    pub fn finish_unlock(
        &mut self,
        completion: BrowserAudioUnlockCompletion,
    ) -> Result<AudioCapabilityState, BrowserAudioError> {
        if !Rc::ptr_eq(&self.unlock_owner, &completion.owner) {
            return Err(BrowserAudioError::UnlockCompletionOwnerMismatch {
                received: completion.attempt,
            });
        }
        match completion.result {
            UnlockTaskResult::ResumeFailed(error) => {
                self.unlock.finish(completion.attempt, false)?;
                Err(error)
            }
            UnlockTaskResult::Resumed => {
                self.unlock.finish(completion.attempt, true)?;
                let state = self.capability();
                if state == AudioCapabilityState::Unavailable {
                    return Err(BrowserAudioError::ContextClosed);
                }
                Ok(state)
            }
        }
    }

    /// Starts the music-only `AudioWorklet.addModule` lifecycle.
    ///
    /// The returned task never owns or waits on `AudioContext.resume()`.
    /// Therefore a slow or failed module load cannot retain an unlock
    /// completion or prevent the host from exposing ready SFX capability.
    pub fn begin_worklet_load(&mut self) -> Result<BrowserAudioWorkletStart, BrowserAudioError> {
        match &self.worklet_load {
            WorkletLoadState::Loading => return Ok(BrowserAudioWorkletStart::InFlight),
            WorkletLoadState::Loaded => return Ok(BrowserAudioWorkletStart::Ready),
            WorkletLoadState::Failed(error) => {
                return Err(BrowserAudioError::WebAudioOperation {
                    operation: "AudioWorklet.addModule",
                    error: error.clone(),
                });
            }
            WorkletLoadState::NotAttempted => {}
        }
        let context = self.context()?.clone();
        let started = self.worklet_load.begin_attempt();
        debug_assert!(started);
        let owner = Rc::clone(&self.worklet_owner);
        Ok(BrowserAudioWorkletStart::Started(Box::pin(async move {
            BrowserAudioWorkletCompletion {
                owner,
                result: load_music_worklet(&context).await,
            }
        })))
    }

    /// Commits the separately-owned music worklet load.
    pub fn finish_worklet_load(
        &mut self,
        completion: BrowserAudioWorkletCompletion,
    ) -> Result<(), BrowserAudioError> {
        if !Rc::ptr_eq(&self.worklet_owner, &completion.owner) {
            return Err(BrowserAudioError::WorkletCompletionOwnerMismatch);
        }
        if self.worklet_load != WorkletLoadState::Loading {
            return Err(BrowserAudioError::WorkletCompletionNotInFlight);
        }
        match completion.result {
            Ok(()) => {
                self.worklet_load.finish(Ok(()));
                Ok(())
            }
            Err(error) => {
                self.worklet_load
                    .finish(Err(BrowserAudioError::WebAudioOperation {
                        operation: "AudioWorklet.addModule",
                        error: error.to_string(),
                    }));
                Err(error)
            }
        }
    }

    /// Consumes commands in their supplied order and contains each device
    /// failure to that command so later independent audio commands still run.
    pub fn consume_all(
        &mut self,
        commands: impl IntoIterator<Item = AudioDeviceCommand>,
    ) -> Vec<BrowserAudioError> {
        commands
            .into_iter()
            .filter_map(|command| self.consume(command).err())
            .collect()
    }

    pub fn consume(&mut self, command: AudioDeviceCommand) -> Result<(), BrowserAudioError> {
        self.validate_command(command)?;
        let was_pending = self
            .music_voices
            .get(&command_voice(command))
            .is_some_and(|active| active.state.is_pending());
        let result = match command {
            AudioDeviceCommand::StartSfx { voice, asset, gain } => {
                self.start_sfx(voice, asset, gain)
            }
            AudioDeviceCommand::StartMusic {
                voice,
                asset,
                start_frame,
                gain,
            } => self.start_music(voice, asset, start_frame, gain),
            AudioDeviceCommand::PauseVoice { voice, at_frame } => self.pause_music(voice, at_frame),
            AudioDeviceCommand::ResumeVoice { voice, at_frame } => {
                self.resume_music(voice, at_frame)
            }
            AudioDeviceCommand::StopVoice { voice } => self.stop_voice(voice),
        };
        if let Err(error) = result {
            self.discard_failed_voice(command_voice(command));
            return Err(error);
        }
        let commits_immediately =
            !matches!(command, AudioDeviceCommand::StartMusic { .. }) && !was_pending;
        if commits_immediately && let Err(error) = self.device_voices.commit(command) {
            self.discard_failed_voice(command_voice(command));
            return Err(BrowserAudioError::DeviceState(error));
        }
        Ok(())
    }

    pub fn initialization_error(&self) -> Option<&str> {
        self.initialization_error.as_deref()
    }

    /// Replaces generated assets without replacing the unlocked browser
    /// context. Runtime-issued stop commands must have drained every voice.
    pub fn replace_catalog(
        &mut self,
        catalog: Arc<AudioAssetCatalog>,
    ) -> Result<(), BrowserAudioError> {
        if !self.device_voices.is_empty()
            || !self.voices.borrow().active.is_empty()
            || !self.music_voices.is_empty()
        {
            return Err(BrowserAudioError::CatalogReplacementWhileVoicesAreActive);
        }
        self.catalog = catalog;
        self.sfx_buffers.clear();
        Ok(())
    }

    /// Removes naturally completed voices from the device registry.
    ///
    /// The host passes each returned id to `AudioRuntime::voice_ended`. Voices
    /// already consumed by `StopVoice` are removed from the ended set and
    /// therefore cannot be reported twice.
    pub fn take_ended_voices(&mut self) -> Vec<AudioVoiceId> {
        let ended = self.voices.borrow_mut().take_ended();
        for voice in &ended {
            self.device_voices.voice_ended(*voice);
        }
        ended
    }

    /// Applies typed Worklet ready/ack feedback and drains callback failures.
    ///
    /// A pending music voice becomes device-committed only here, after the
    /// processor reports `ready`. Draining feedback never creates a new voice
    /// or retries a failed materialization.
    pub fn take_feedback_errors(&mut self) -> Vec<BrowserAudioError> {
        let feedback = std::mem::take(&mut *self.worklet_feedback.borrow_mut());
        let mut errors = Vec::new();
        for event in feedback {
            match event {
                MusicWorkletFeedback::Ready { voice } => {
                    if let Some(active) = self.music_voices.get_mut(&voice) {
                        active.state.acknowledge_ready();
                        if let Err(error) = self.reconcile_music_connection(voice) {
                            errors.push(error);
                        }
                    }
                }
                MusicWorkletFeedback::ResumeAcknowledged { voice } => {
                    if let Some(active) = self.music_voices.get_mut(&voice) {
                        active.state.acknowledge_resume();
                        if let Err(error) = self.reconcile_music_connection(voice) {
                            errors.push(error);
                        }
                    }
                }
                MusicWorkletFeedback::Failed(error)
                    if error
                        .voice_id()
                        .is_none_or(|voice| self.music_voices.contains_key(&voice)) =>
                {
                    errors.push(error);
                }
                MusicWorkletFeedback::Failed(_) => {}
            }
        }
        let failed_voices = errors
            .iter()
            .filter_map(BrowserAudioError::voice_id)
            .collect::<BTreeSet<_>>();
        for voice in failed_voices {
            self.discard_failed_voice(voice);
        }
        errors
    }

    fn validate_command(&self, command: AudioDeviceCommand) -> Result<(), BrowserAudioError> {
        let voice = command_voice(command);
        match command {
            AudioDeviceCommand::StartSfx { .. } | AudioDeviceCommand::StartMusic { .. } => {
                if self.music_voices.contains_key(&voice) {
                    return Err(BrowserAudioError::DeviceState(
                        AudioDeviceStateError::DuplicateVoice(voice),
                    ));
                }
                self.device_voices
                    .validate(command)
                    .map_err(BrowserAudioError::DeviceState)
            }
            AudioDeviceCommand::PauseVoice { .. }
            | AudioDeviceCommand::ResumeVoice { .. }
            | AudioDeviceCommand::StopVoice { .. }
                if self
                    .music_voices
                    .get(&voice)
                    .is_some_and(|active| active.state.is_pending()) =>
            {
                Ok(())
            }
            _ => self
                .device_voices
                .validate(command)
                .map_err(BrowserAudioError::DeviceState),
        }
    }

    fn context(&self) -> Result<&AudioContext, BrowserAudioError> {
        self.context
            .as_ref()
            .ok_or_else(|| BrowserAudioError::ContextUnavailable {
                error: self
                    .initialization_error
                    .clone()
                    .unwrap_or_else(|| "AudioContext construction failed".to_string()),
            })
    }

    fn require_ready(&self) -> Result<&AudioContext, BrowserAudioError> {
        let state = self.capability();
        if state != AudioCapabilityState::Ready {
            return Err(BrowserAudioError::CapabilityNotReady { state });
        }
        self.context()
    }

    fn start_sfx(
        &mut self,
        voice: AudioVoiceId,
        asset: SfxAssetId,
        gain: f32,
    ) -> Result<(), BrowserAudioError> {
        let context = self.require_ready()?.clone();
        let buffer = self.sfx_buffer(&context, voice, asset)?;

        let source = context
            .create_buffer_source()
            .map_err(|error| voice_web_error(voice, "AudioContext.createBufferSource", error))?;
        source.set_buffer(Some(&buffer));
        let gain_node = context
            .create_gain()
            .map_err(|error| voice_web_error(voice, "AudioContext.createGain", error))?;
        gain_node.gain().set_value(gain);
        source
            .connect_with_audio_node(&gain_node)
            .map_err(|error| voice_web_error(voice, "AudioBufferSourceNode.connect", error))?;
        gain_node
            .connect_with_audio_node(&context.destination())
            .map_err(|error| voice_web_error(voice, "GainNode.connect", error))?;
        let registry: Weak<RefCell<VoiceRegistry>> = Rc::downgrade(&self.voices);
        let feedback_wakeup = Rc::clone(&self.feedback_wakeup);
        let on_ended = Closure::wrap(Box::new(move || {
            if let Some(registry) = registry.upgrade() {
                registry.borrow_mut().mark_ended(voice);
                feedback_wakeup.notify();
            }
        }) as Box<dyn FnMut()>);
        source
            .unchecked_ref::<AudioScheduledSourceNode>()
            .set_onended(Some(on_ended.as_ref().unchecked_ref()));

        self.voices.borrow_mut().active.insert(
            voice,
            ActiveSfxVoice {
                source: source.clone(),
                _gain: gain_node,
                _on_ended: on_ended,
            },
        );
        if let Err(error) = source.start() {
            self.voices.borrow_mut().take(voice);
            return Err(voice_web_error(voice, "AudioBufferSourceNode.start", error));
        }
        Ok(())
    }

    fn sfx_buffer(
        &mut self,
        context: &AudioContext,
        voice: AudioVoiceId,
        asset: SfxAssetId,
    ) -> Result<AudioBuffer, BrowserAudioError> {
        if let Some(buffer) = self.sfx_buffers.get(&asset) {
            return Ok(buffer.clone());
        }
        let clip = self
            .catalog
            .sfx(asset)
            .ok_or(BrowserAudioError::MissingSfxAsset { voice, asset })?;
        if self.catalog.sfx_gain(asset).is_none() {
            return Err(BrowserAudioError::MissingSfxGain { voice, asset });
        }
        let length =
            u32::try_from(clip.samples.len()).map_err(|_| BrowserAudioError::VoiceOperation {
                voice,
                operation: "AudioContext.createBuffer",
                error: format!(
                    "SFX asset {asset:?} has {} samples, exceeding the Web Audio u32 buffer limit",
                    clip.samples.len()
                ),
            })?;
        let buffer = context
            .create_buffer(1, length, clip.sample_rate as f32)
            .map_err(|error| voice_web_error(voice, "AudioContext.createBuffer", error))?;
        buffer
            .copy_to_channel(clip.samples.as_ref(), 0)
            .map_err(|error| voice_web_error(voice, "AudioBuffer.copyToChannel", error))?;
        self.sfx_buffers.insert(asset, buffer.clone());
        Ok(buffer)
    }

    fn start_music(
        &mut self,
        voice: AudioVoiceId,
        asset: MusicAssetId,
        start_frame: u64,
        gain: f32,
    ) -> Result<(), BrowserAudioError> {
        let context = self.require_ready()?.clone();
        self.worklet_load.require_loaded(voice)?;
        let track = self
            .catalog
            .music(asset)
            .ok_or(BrowserAudioError::MissingMusicAsset { voice, asset })?;
        if self.catalog.music_gain(asset).is_none() {
            return Err(BrowserAudioError::MissingMusicGain { voice, asset });
        }
        if track.sample_rate() == 0 || track.channels() != 2 {
            return Err(BrowserAudioError::VoiceOperation {
                voice,
                operation: "AudioWorkletNode",
                error: format!(
                    "music asset {asset:?} has unsupported stream format: {} Hz, {} channels",
                    track.sample_rate(),
                    track.channels()
                ),
            });
        }
        let payload = encode_music_worklet_asset(track).map_err(|error| {
            BrowserAudioError::VoiceOperation {
                voice,
                operation: "encode_music_worklet_asset",
                error,
            }
        })?;
        let options = worklet_node_options(voice, &payload, start_frame)?;
        let node = AudioWorkletNode::new_with_options(
            context.unchecked_ref(),
            MUSIC_WORKLET_PROCESSOR,
            &options,
        )
        .map_err(|error| voice_web_error(voice, "AudioWorkletNode.constructor", error))?;
        let message_port = node
            .port()
            .map_err(|error| voice_web_error(voice, "AudioWorkletNode.port", error))?;
        let gain_node = context
            .create_gain()
            .map_err(|error| voice_web_error(voice, "AudioContext.createGain", error))?;
        gain_node.gain().set_value(gain);
        gain_node
            .connect_with_audio_node(&context.destination())
            .map_err(|error| voice_web_error(voice, "GainNode.connect", error))?;
        let ready_deadline_window =
            web_sys::window().ok_or_else(|| BrowserAudioError::VoiceOperation {
                voice,
                operation: "Window.setTimeout",
                error: "browser window is unavailable".to_string(),
            })?;
        let deadline_feedback = Rc::clone(&self.worklet_feedback);
        let deadline_wakeup = Rc::clone(&self.feedback_wakeup);
        let on_ready_deadline = Closure::wrap(Box::new(move || {
            push_worklet_feedback(
                &deadline_feedback,
                &deadline_wakeup,
                MusicWorkletFeedback::Failed(BrowserAudioError::MusicCallback {
                    voice,
                    asset,
                    operation: "AudioWorklet ready deadline",
                    error: format!(
                        "processor did not report ready within {MUSIC_WORKLET_READY_DEADLINE_MILLISECONDS} ms"
                    ),
                }),
            );
        }) as Box<dyn FnMut()>);
        let ready_deadline_id = ready_deadline_window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                on_ready_deadline.as_ref().unchecked_ref(),
                MUSIC_WORKLET_READY_DEADLINE_MILLISECONDS,
            )
            .map_err(|error| voice_web_error(voice, "Window.setTimeout", error))?;

        let worklet_feedback = Rc::clone(&self.worklet_feedback);
        let message_wakeup = Rc::clone(&self.feedback_wakeup);
        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            let data = event.data();
            let kind = message_string(&data, "kind");
            if message_number(&data, "version") != Some(1.0) {
                push_worklet_feedback(
                    &worklet_feedback,
                    &message_wakeup,
                    MusicWorkletFeedback::Failed(BrowserAudioError::MusicCallback {
                        voice,
                        asset,
                        operation: "AudioWorklet MessagePort",
                        error: "worklet feedback requires typed contract version 1".to_string(),
                    }),
                );
                return;
            }
            if kind.as_deref() == Some("ready") {
                push_worklet_feedback(
                    &worklet_feedback,
                    &message_wakeup,
                    MusicWorkletFeedback::Ready { voice },
                );
            } else if kind.as_deref() == Some("ack")
                && message_string(&data, "command").as_deref() == Some("resume")
            {
                push_worklet_feedback(
                    &worklet_feedback,
                    &message_wakeup,
                    MusicWorkletFeedback::ResumeAcknowledged { voice },
                );
            } else if kind.as_deref() == Some("error") {
                push_worklet_feedback(
                    &worklet_feedback,
                    &message_wakeup,
                    MusicWorkletFeedback::Failed(BrowserAudioError::MusicCallback {
                        voice,
                        asset,
                        operation: "AudioWorkletProcessor",
                        error: message_string(&data, "error")
                            .unwrap_or_else(|| "worklet reported an unspecified error".to_string()),
                    }),
                );
                return;
            } else if kind.as_deref() != Some("ack") {
                push_worklet_feedback(
                    &worklet_feedback,
                    &message_wakeup,
                    MusicWorkletFeedback::Failed(BrowserAudioError::MusicCallback {
                        voice,
                        asset,
                        operation: "AudioWorklet MessagePort",
                        error: format!("unexpected worklet message kind {kind:?}"),
                    }),
                );
                return;
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        message_port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        let processor_feedback = Rc::clone(&self.worklet_feedback);
        let processor_wakeup = Rc::clone(&self.feedback_wakeup);
        let on_processor_error = Closure::wrap(Box::new(move |_event: Event| {
            push_worklet_feedback(
                &processor_feedback,
                &processor_wakeup,
                MusicWorkletFeedback::Failed(BrowserAudioError::MusicCallback {
                    voice,
                    asset,
                    operation: "AudioWorkletNode.processorerror",
                    error: "AudioWorklet processor raised an unhandled error".to_string(),
                }),
            );
        }) as Box<dyn FnMut(Event)>);
        node.set_onprocessorerror(Some(on_processor_error.as_ref().unchecked_ref()));

        self.music_voices.insert(
            voice,
            ActiveMusicVoice {
                node,
                gain: gain_node,
                state: MusicVoiceState::pending(AudioDeviceCommand::StartMusic {
                    voice,
                    asset,
                    start_frame,
                    gain,
                }),
                ready_deadline_window,
                ready_deadline_id,
                _on_ready_deadline: on_ready_deadline,
                _on_message: on_message,
                _on_processor_error: on_processor_error,
            },
        );
        Ok(())
    }

    fn pause_music(&mut self, voice: AudioVoiceId, at_frame: u64) -> Result<(), BrowserAudioError> {
        let active = self
            .music_voices
            .get_mut(&voice)
            .ok_or(BrowserAudioError::MissingVoice { voice })?;
        active.state.request_pause();
        if active.state.connected {
            active
                .node
                .disconnect()
                .map_err(|error| voice_web_error(voice, "AudioWorkletNode.disconnect", error))?;
            active.state.connected = false;
        }
        post_worklet_command(&active.node, voice, "pause", Some(at_frame))
    }

    fn resume_music(
        &mut self,
        voice: AudioVoiceId,
        at_frame: u64,
    ) -> Result<(), BrowserAudioError> {
        let active = self
            .music_voices
            .get_mut(&voice)
            .ok_or(BrowserAudioError::MissingVoice { voice })?;
        active.state.request_resume();
        post_worklet_command(&active.node, voice, "resume", Some(at_frame))
    }

    fn stop_voice(&mut self, voice: AudioVoiceId) -> Result<(), BrowserAudioError> {
        if let Some(mut active) = self.music_voices.remove(&voice) {
            active.cancel_ready_deadline();
            active.node.set_onprocessorerror(None);
            if let Ok(port) = active.node.port() {
                port.set_onmessage(None);
            }
            let command_result = post_worklet_command(&active.node, voice, "stop", None);
            let node_result = active
                .node
                .disconnect()
                .map_err(|error| voice_web_error(voice, "AudioWorkletNode.disconnect", error));
            let gain_result = active
                .gain
                .disconnect()
                .map_err(|error| voice_web_error(voice, "GainNode.disconnect", error));
            command_result?;
            node_result?;
            gain_result?;
            return Ok(());
        }
        let (active, ended) = self
            .voices
            .borrow_mut()
            .take(voice)
            .ok_or(BrowserAudioError::MissingVoice { voice })?;
        let scheduled_source = active.source.unchecked_ref::<AudioScheduledSourceNode>();
        scheduled_source.set_onended(None);
        if ended {
            return Ok(());
        }
        scheduled_source
            .stop()
            .map_err(|error| voice_web_error(voice, "AudioBufferSourceNode.stop", error))
    }

    fn reconcile_music_connection(&mut self, voice: AudioVoiceId) -> Result<(), BrowserAudioError> {
        let active = self
            .music_voices
            .get_mut(&voice)
            .ok_or(BrowserAudioError::MissingVoice { voice })?;
        if active.state.should_connect() {
            active
                .node
                .connect_with_audio_node(&active.gain)
                .map_err(|error| voice_web_error(voice, "AudioWorkletNode.connect", error))?;
            active.state.connected = true;
        }
        if active.state.ready && active.state.is_pending() {
            active.cancel_ready_deadline();
            commit_ready_music(&mut active.state, &mut self.device_voices)?;
        }
        Ok(())
    }

    fn discard_failed_voice(&mut self, voice: AudioVoiceId) {
        if let Some(mut active) = self.music_voices.remove(&voice) {
            active.cancel_ready_deadline();
            active.node.set_onprocessorerror(None);
            if let Ok(port) = active.node.port() {
                port.set_onmessage(None);
            }
            let _ = active.node.disconnect();
            let _ = active.gain.disconnect();
        }
        if let Some((active, ended)) = self.voices.borrow_mut().take(voice) {
            let source = active.source.unchecked_ref::<AudioScheduledSourceNode>();
            source.set_onended(None);
            if !ended {
                let _ = source.stop();
            }
        }
        release_failed_device_voice(&mut self.device_voices, voice);
    }
}

impl Drop for BrowserAudioBackend {
    fn drop(&mut self) {
        if let Some(context) = &self.context {
            context.set_onstatechange(None);
            for (_, active) in self.voices.borrow_mut().active.drain() {
                let source = active.source.unchecked_ref::<AudioScheduledSourceNode>();
                source.set_onended(None);
                let _ = source.stop();
            }
            self.voices.borrow_mut().ended.clear();
            for (_, mut active) in self.music_voices.drain() {
                active.cancel_ready_deadline();
                active.node.set_onprocessorerror(None);
                if let Ok(port) = active.node.port() {
                    port.set_onmessage(None);
                    let _ = port.close();
                }
                let _ = active.node.disconnect();
                let _ = active.gain.disconnect();
            }
            let _ = context.close();
        }
    }
}

fn command_voice(command: AudioDeviceCommand) -> AudioVoiceId {
    match command {
        AudioDeviceCommand::StartSfx { voice, .. }
        | AudioDeviceCommand::StartMusic { voice, .. }
        | AudioDeviceCommand::PauseVoice { voice, .. }
        | AudioDeviceCommand::ResumeVoice { voice, .. }
        | AudioDeviceCommand::StopVoice { voice } => voice,
    }
}

fn release_failed_device_voice(registry: &mut AudioDeviceVoiceRegistry, voice: AudioVoiceId) {
    registry.voice_ended(voice);
}

fn commit_ready_music(
    state: &mut MusicVoiceState,
    registry: &mut AudioDeviceVoiceRegistry,
) -> Result<(), BrowserAudioError> {
    let Some(start) = state.pending_start() else {
        return Ok(());
    };
    registry
        .commit(start)
        .map_err(BrowserAudioError::DeviceState)?;
    state.commit();
    Ok(())
}

fn browser_context_capability(
    context_state: AudioContextState,
    visible: bool,
    unlocked: bool,
) -> AudioCapabilityState {
    if context_state == AudioContextState::Closed {
        return AudioCapabilityState::Unavailable;
    }
    if !visible {
        return AudioCapabilityState::Suspended;
    }
    match context_state {
        AudioContextState::Running => AudioCapabilityState::Ready,
        AudioContextState::Suspended if unlocked => AudioCapabilityState::Suspended,
        AudioContextState::Suspended => AudioCapabilityState::Locked,
        AudioContextState::Closed => AudioCapabilityState::Unavailable,
        _ => AudioCapabilityState::Unavailable,
    }
}

async fn load_music_worklet(context: &AudioContext) -> Result<(), BrowserAudioError> {
    let module_url = embedded_worklet_data_url(MUSIC_WORKLET_SOURCE);
    let worklet = context
        .audio_worklet()
        .map_err(|error| web_error("AudioContext.audioWorklet", error))?;
    let promise = worklet
        .add_module(&module_url)
        .map_err(|error| web_error("AudioWorklet.addModule", error))?;
    JsFuture::from(promise)
        .await
        .map_err(|error| web_error("AudioWorklet.addModule promise", error))?;
    Ok(())
}

fn embedded_worklet_data_url(source: &str) -> String {
    let encoded_source = percent_encode_data_url_component(source);
    format!("data:text/javascript;charset=utf-8,{encoded_source}")
}

fn percent_encode_data_url_component(source: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(source.len());
    for byte in source.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

fn worklet_node_options(
    voice: AudioVoiceId,
    payload: &[u8],
    start_frame: u64,
) -> Result<AudioWorkletNodeOptions, BrowserAudioError> {
    let contract = Object::new();
    Reflect::set(
        &contract,
        &JsValue::from_str("version"),
        &JsValue::from_f64(1.0),
    )
    .map_err(|error| voice_web_error(voice, "AudioWorklet processorOptions.version", error))?;
    Reflect::set(
        &contract,
        &JsValue::from_str("asset"),
        &Uint8Array::from(payload),
    )
    .map_err(|error| voice_web_error(voice, "AudioWorklet processorOptions.asset", error))?;
    Reflect::set(
        &contract,
        &JsValue::from_str("startFrame"),
        &JsValue::from_str(&start_frame.to_string()),
    )
    .map_err(|error| voice_web_error(voice, "AudioWorklet processorOptions.startFrame", error))?;

    let options = AudioWorkletNodeOptions::new();
    options.set_number_of_inputs(0);
    options.set_number_of_outputs(1);
    options.set_output_channel_count(&Array::of1(&JsValue::from_f64(2.0)));
    options.set_processor_options(Some(&contract));
    Ok(options)
}

fn post_worklet_command(
    node: &AudioWorkletNode,
    voice: AudioVoiceId,
    kind: &'static str,
    at_frame: Option<u64>,
) -> Result<(), BrowserAudioError> {
    let command = Object::new();
    Reflect::set(
        &command,
        &JsValue::from_str("version"),
        &JsValue::from_f64(1.0),
    )
    .map_err(|error| voice_web_error(voice, "AudioWorklet command.version", error))?;
    Reflect::set(
        &command,
        &JsValue::from_str("kind"),
        &JsValue::from_str(kind),
    )
    .map_err(|error| voice_web_error(voice, "AudioWorklet command.kind", error))?;
    if let Some(frame) = at_frame {
        Reflect::set(
            &command,
            &JsValue::from_str("atFrame"),
            &JsValue::from_str(&frame.to_string()),
        )
        .map_err(|error| voice_web_error(voice, "AudioWorklet command.atFrame", error))?;
    }
    node.port()
        .map_err(|error| voice_web_error(voice, "AudioWorkletNode.port", error))?
        .post_message(&command)
        .map_err(|error| voice_web_error(voice, "MessagePort.postMessage", error))
}

fn message_string(message: &JsValue, field: &str) -> Option<String> {
    Reflect::get(message, &JsValue::from_str(field))
        .ok()
        .and_then(|value| value.as_string())
}

fn message_number(message: &JsValue, field: &str) -> Option<f64> {
    Reflect::get(message, &JsValue::from_str(field))
        .ok()
        .and_then(|value| value.as_f64())
}

fn web_error(operation: &'static str, error: JsValue) -> BrowserAudioError {
    BrowserAudioError::WebAudioOperation {
        operation,
        error: js_error(error),
    }
}

fn voice_web_error(
    voice: AudioVoiceId,
    operation: &'static str,
    error: JsValue,
) -> BrowserAudioError {
    BrowserAudioError::VoiceOperation {
        voice,
        operation,
        error: js_error(error),
    }
}

fn js_error(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_command_reconciliation_unblocks_catalog_replacement() {
        let voice = AudioVoiceId(5);
        let mut registry = AudioDeviceVoiceRegistry::default();
        registry
            .commit(AudioDeviceCommand::StartSfx {
                voice,
                asset: SfxAssetId(0),
                gain: 1.0,
            })
            .expect("test voice materializes");

        release_failed_device_voice(&mut registry, voice);

        assert!(
            registry.is_empty(),
            "a failed materialized command must not keep catalog replacement blocked"
        );
    }

    #[test]
    fn music_worklet_failure_is_contained_to_music_materialization() {
        assert_eq!(
            browser_context_capability(AudioContextState::Running, true, true),
            AudioCapabilityState::Ready,
            "SFX remains available whenever the AudioContext is running"
        );
        let voice = AudioVoiceId(9);
        let failed = WorkletLoadState::Failed("module rejected".to_string());
        let error = failed
            .require_loaded(voice)
            .expect_err("music requires its declared worklet");
        assert_eq!(error.voice_id(), Some(voice));
        assert!(error.to_string().contains("module rejected"));
        assert!(WorkletLoadState::Loaded.require_loaded(voice).is_ok());
    }

    #[test]
    fn worklet_load_attempt_is_never_retried_implicitly() {
        let mut state = WorkletLoadState::NotAttempted;
        assert!(state.begin_attempt());
        assert!(
            !state.begin_attempt(),
            "an in-flight load is already an attempt"
        );
        state.finish(Err(BrowserAudioError::WebAudioOperation {
            operation: "AudioWorklet.addModule",
            error: "module rejected".to_string(),
        }));
        assert!(
            !state.begin_attempt(),
            "a later gesture must not retry failure"
        );
        assert!(matches!(state, WorkletLoadState::Failed(_)));
    }

    #[test]
    fn unlock_lifecycle_is_idempotent_while_in_flight_and_after_success() {
        let mut lifecycle = UnlockLifecycle::default();
        let UnlockReservation::Start(attempt) = lifecycle
            .reserve(false)
            .expect("first gesture starts unlock")
        else {
            panic!("first gesture must own the async task");
        };
        assert_eq!(
            lifecycle
                .reserve(false)
                .expect("duplicate gesture is valid"),
            UnlockReservation::InFlight
        );

        lifecycle
            .finish(attempt, true)
            .expect("matching completion commits unlock");
        assert!(lifecycle.is_unlocked());
        assert_eq!(
            lifecycle.reserve(true).expect("ready state is idempotent"),
            UnlockReservation::Ready
        );
    }

    #[test]
    fn rejected_unlock_is_explicitly_retryable_with_a_new_identity() {
        let mut lifecycle = UnlockLifecycle::default();
        let UnlockReservation::Start(first) = lifecycle
            .reserve(false)
            .expect("first gesture starts unlock")
        else {
            panic!("first gesture must own the async task");
        };
        lifecycle
            .finish(first, false)
            .expect("resume rejection restores locked state");
        let UnlockReservation::Start(second) = lifecycle
            .reserve(false)
            .expect("a later gesture retries explicitly")
        else {
            panic!("retry must own a new async task");
        };

        assert_ne!(first, second);
        assert_eq!(
            lifecycle.finish(first, true),
            Err(BrowserAudioError::UnlockCompletionOutOfOrder {
                expected: Some(second),
                received: first,
            })
        );
        assert!(!lifecycle.is_unlocked());
    }

    #[test]
    fn previously_unlocked_suspended_context_can_be_resumed_again() {
        let mut lifecycle = UnlockLifecycle::default();
        let UnlockReservation::Start(first) = lifecycle
            .reserve(false)
            .expect("first gesture starts unlock")
        else {
            panic!("first gesture must own the async task");
        };
        lifecycle
            .finish(first, true)
            .expect("first resume completes");

        let UnlockReservation::Start(resume) = lifecycle
            .reserve(false)
            .expect("suspended capability requires another resume")
        else {
            panic!("a suspended unlocked context must not report ready");
        };
        lifecycle
            .finish(resume, false)
            .expect("resume rejection is attached to the current attempt");

        assert!(
            lifecycle.is_unlocked(),
            "a later resume rejection does not erase prior unlock history"
        );
        assert!(matches!(
            lifecycle.reserve(false),
            Ok(UnlockReservation::Start(_))
        ));
    }

    #[test]
    fn unlock_completion_is_independent_from_worklet_load_completion() {
        let mut unlock = UnlockLifecycle::default();
        let UnlockReservation::Start(attempt) = unlock
            .reserve(false)
            .expect("gesture reserves the context resume")
        else {
            panic!("first gesture must start a resume");
        };
        let mut worklet = WorkletLoadState::NotAttempted;
        assert!(worklet.begin_attempt());

        unlock
            .finish(attempt, true)
            .expect("context resume commits without waiting for worklet load");

        assert!(unlock.is_unlocked());
        assert_eq!(worklet, WorkletLoadState::Loading);
        assert_eq!(
            browser_context_capability(AudioContextState::Running, true, unlock.is_unlocked()),
            AudioCapabilityState::Ready,
            "SFX capability is ready while music worklet loading remains in flight"
        );
    }

    #[test]
    fn worklet_failure_does_not_change_context_unlock_lifecycle() {
        let mut unlock = UnlockLifecycle::default();
        let UnlockReservation::Start(attempt) =
            unlock.reserve(false).expect("unlock attempt starts")
        else {
            panic!("unlock attempt must start");
        };
        unlock.finish(attempt, true).expect("unlock succeeds");
        let mut worklet = WorkletLoadState::NotAttempted;
        assert!(worklet.begin_attempt());
        worklet.finish(Err(BrowserAudioError::WebAudioOperation {
            operation: "AudioWorklet.addModule",
            error: "module rejected".to_string(),
        }));

        assert!(unlock.is_unlocked());
        assert!(matches!(worklet, WorkletLoadState::Failed(_)));
        assert_eq!(
            browser_context_capability(AudioContextState::Running, true, unlock.is_unlocked()),
            AudioCapabilityState::Ready
        );
    }

    #[test]
    fn pending_music_commits_only_after_typed_ready_and_preserves_desired_state() {
        let voice = AudioVoiceId(12);
        let start = AudioDeviceCommand::StartMusic {
            voice,
            asset: MusicAssetId(3),
            start_frame: 48,
            gain: 0.5,
        };
        let mut state = MusicVoiceState::pending(start);
        let mut registry = AudioDeviceVoiceRegistry::default();

        state.request_pause();
        assert_eq!(registry.voice(voice), None);
        assert_eq!(state.pending_start(), Some(start));

        state.acknowledge_ready();
        commit_ready_music(&mut state, &mut registry)
            .expect("typed ready finalizes exactly one pending voice");
        assert_eq!(
            registry.voice(voice),
            Some(puzzle_audio::AudioDeviceVoiceKind::Music(MusicAssetId(3)))
        );
        assert_eq!(state.materialization, MusicVoiceMaterialization::Committed);
        assert_eq!(state.desired, MusicVoiceDesiredState::Paused);
    }

    #[test]
    fn failed_pending_music_never_reserves_the_device_voice() {
        let voice = AudioVoiceId(14);
        let pending = MusicVoiceState::pending(AudioDeviceCommand::StartMusic {
            voice,
            asset: MusicAssetId(1),
            start_frame: 0,
            gain: 1.0,
        });
        let registry = AudioDeviceVoiceRegistry::default();

        assert!(pending.is_pending());
        assert_eq!(registry.voice(voice), None);
    }

    #[test]
    fn ready_after_pause_commits_without_connecting() {
        let voice = AudioVoiceId(15);
        let mut state = MusicVoiceState::pending(AudioDeviceCommand::StartMusic {
            voice,
            asset: MusicAssetId(2),
            start_frame: 0,
            gain: 1.0,
        });
        let mut registry = AudioDeviceVoiceRegistry::default();

        state.request_pause();
        state.acknowledge_ready();
        commit_ready_music(&mut state, &mut registry).expect("ready commits the pending voice");

        assert_eq!(state.desired, MusicVoiceDesiredState::Paused);
        assert!(!state.should_connect());
        assert_eq!(
            registry.voice(voice),
            Some(puzzle_audio::AudioDeviceVoiceKind::Music(MusicAssetId(2)))
        );
    }

    #[test]
    fn ready_after_resume_waits_for_the_resume_ack_before_connecting() {
        let voice = AudioVoiceId(16);
        let mut state = MusicVoiceState::pending(AudioDeviceCommand::StartMusic {
            voice,
            asset: MusicAssetId(2),
            start_frame: 0,
            gain: 1.0,
        });

        state.request_pause();
        state.request_resume();
        state.acknowledge_ready();
        assert!(!state.should_connect());

        state.acknowledge_resume();
        assert!(state.should_connect());
    }

    #[test]
    fn committed_music_callback_failure_releases_the_device_voice() {
        let voice = AudioVoiceId(17);
        let mut state = MusicVoiceState::pending(AudioDeviceCommand::StartMusic {
            voice,
            asset: MusicAssetId(4),
            start_frame: 0,
            gain: 1.0,
        });
        let mut registry = AudioDeviceVoiceRegistry::default();
        state.acknowledge_ready();
        commit_ready_music(&mut state, &mut registry).expect("ready commits the music voice");

        release_failed_device_voice(&mut registry, voice);

        assert_eq!(registry.voice(voice), None);
        assert!(registry.is_empty());
    }

    #[test]
    fn typed_feedback_wakes_its_owner_once_without_host_polling() {
        use std::cell::Cell;

        let feedback = Rc::new(RefCell::new(Vec::new()));
        let wakeup = Rc::new(FeedbackWakeup::default());
        let wake_count = Rc::new(Cell::new(0));
        let observed_wake_count = Rc::clone(&wake_count);
        wakeup.set(Rc::new(move || {
            observed_wake_count.set(observed_wake_count.get() + 1);
        }));

        push_worklet_feedback(
            &feedback,
            &wakeup,
            MusicWorkletFeedback::Ready {
                voice: AudioVoiceId(18),
            },
        );

        assert_eq!(wake_count.get(), 1);
        assert_eq!(feedback.borrow().len(), 1);
    }

    #[test]
    fn embedded_worklet_uses_direct_wasm_bytes_without_a_decode_path() {
        assert!(!MUSIC_WORKLET_SOURCE.contains("ScriptProcessor"));
        assert!(!MUSIC_WORKLET_SOURCE.contains("atob("));
        assert!(!MUSIC_WORKLET_SOURCE.contains("base64"));
        assert!(!MUSIC_WORKLET_SOURCE.contains("__PUZZLE_AUDIO_WORKLET_WASM_BASE64__"));
        assert!(!MUSIC_WORKLET_SOURCE.contains("__PUZZLE_AUDIO_WORKLET_WASM_BYTES__"));
        assert!(!MUSIC_WORKLET_SOURCE.contains("await __wbg_init"));
        assert!(!MUSIC_WORKLET_SOURCE.contains("let cachedTextDecoder = new TextDecoder"));
        assert!(
            MUSIC_WORKLET_SOURCE.contains("const embeddedBytes = new Uint8Array([0,97,115,109,")
        );
        assert!(MUSIC_WORKLET_SOURCE.contains(
            "AudioWorkletGlobalScope cannot decode a Rust diagnostic because TextDecoder is unavailable."
        ));
        assert!(
            !MUSIC_WORKLET_SOURCE
                .contains("initSync({ module: embeddedBytes });\n\nclass PuzzleMusicProcessor")
        );
        assert!(MUSIC_WORKLET_SOURCE.contains("initSync({ module: embeddedBytes })"));
        assert!(MUSIC_WORKLET_SOURCE.contains("registerProcessor("));
    }

    #[test]
    fn embedded_worklet_has_one_origin_independent_module_address() {
        assert_eq!(
            embedded_worklet_data_url("registerProcessor(\"music\", processor);\n"),
            "data:text/javascript;charset=utf-8,registerProcessor(%22music%22%2C%20processor)%3B%0A"
        );
        assert_eq!(
            embedded_worklet_data_url("// 音\n"),
            "data:text/javascript;charset=utf-8,%2F%2F%20%E9%9F%B3%0A"
        );
    }
}
