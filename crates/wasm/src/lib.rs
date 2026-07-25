use std::{cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc};

use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

use puzzle_audio::{
    AudioAssetCatalog, AudioVoiceId, MusicAssetId, MusicRecipe, SFX_TYPES, SfxAssetId, SfxRecipe,
};

type SourceAnalysisRevision = u32;

#[wasm_bindgen(typescript_custom_section)]
const WORKSPACE_SOURCE_TYPES: &str = r#"
export interface WorkspaceSourceDocument {
    readonly path: string;
    readonly source: string;
}

export interface WorkspacePresentationManifest {
    readonly themeName: string | null;
    readonly cssPaths: string[];
    readonly scriptPaths: string[];
    readonly filePaths: string[];
    readonly visualImageAssets: ReadonlyArray<{
        readonly id: string;
        readonly path: string;
        readonly format: "png" | "jpeg";
    }>;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ReadonlyArray<WorkspaceSourceDocument>")]
    pub type WorkspaceSourceDocuments;

    #[wasm_bindgen(typescript_type = "WorkspacePresentationManifest")]
    pub type WorkspacePresentationManifestJs;
}

thread_local! {
    static SOURCE_ANALYSES: RefCell<SourceAnalysisStore> =
        RefCell::new(SourceAnalysisStore::default());
}

#[wasm_bindgen]
pub struct WasmSolverService {
    inner: puzzle_solver_runtime::SolverService,
}

#[wasm_bindgen]
impl WasmSolverService {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: puzzle_solver_runtime::SolverService::new(),
        }
    }

    pub fn prepare_workspace(
        &mut self,
        entry_path: &str,
        documents: WorkspaceSourceDocuments,
        now_ms: f64,
    ) -> Result<JsValue, JsValue> {
        let documents = decode_js_value::<Vec<puzzle_lang::WorkspaceSourceDocument>>(
            documents.into(),
            "solver workspace documents",
        )?;
        let prepared = self
            .inner
            .prepare_workspace(entry_path, documents, solver_now_ms(now_ms)?)
            .map_err(|error| JsValue::from_str(&error))?;
        encode_js_value(&prepared, "prepared solver artifact")
    }

    pub fn prepare_source(
        &mut self,
        source: &str,
        puzzle_path: &str,
        now_ms: f64,
    ) -> Result<JsValue, JsValue> {
        if puzzle_path.trim().is_empty() {
            return Err(JsValue::from_str(
                "solver source preparation requires an explicit puzzle path",
            ));
        }
        let prepared = self
            .inner
            .prepare_workspace(
                puzzle_path,
                vec![puzzle_lang::WorkspaceSourceDocument {
                    path: puzzle_path.to_string(),
                    source: source.to_string(),
                }],
                solver_now_ms(now_ms)?,
            )
            .map_err(|error| JsValue::from_str(&error))?;
        encode_js_value(&prepared, "prepared solver artifact")
    }

    pub fn pin_artifact(
        &mut self,
        artifact_id: Option<String>,
        now_ms: f64,
    ) -> Result<(), JsValue> {
        self.inner
            .pin_artifact(artifact_id.as_deref(), solver_now_ms(now_ms)?)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn start(
        &mut self,
        artifact_id: &str,
        request: JsValue,
        now_ms: f64,
    ) -> Result<u32, JsValue> {
        let request = decode_js_value::<puzzle_runtime_contract::SolverSearchRequest>(
            request,
            "solver search request",
        )?;
        self.inner
            .start(artifact_id, request, solver_now_ms(now_ms)?)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn advance(
        &mut self,
        search_id: u32,
        max_expanded_nodes: u32,
        now_ms: f64,
    ) -> Result<JsValue, JsValue> {
        let response = self
            .inner
            .advance_nodes(
                search_id,
                max_expanded_nodes as usize,
                solver_now_ms(now_ms)?,
            )
            .map_err(|error| JsValue::from_str(&error))?;
        encode_js_value(&response, "solver advance response")
    }

    pub fn cancel(&mut self, search_id: u32, now_ms: f64) -> Result<(), JsValue> {
        self.inner
            .cancel(search_id, solver_now_ms(now_ms)?)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn materialize_state(
        &mut self,
        artifact_id: &str,
        level_index: usize,
        state: JsValue,
        materialize_level_start: bool,
        now_ms: f64,
    ) -> Result<JsValue, JsValue> {
        let state =
            decode_js_value::<puzzle_runtime_contract::SolverStateSnapshot>(state, "solver state")?;
        let state = self
            .inner
            .materialize_state(
                artifact_id,
                level_index,
                state,
                materialize_level_start,
                solver_now_ms(now_ms)?,
            )
            .map_err(|error| JsValue::from_str(&error))?;
        encode_js_value(&state, "materialized solver state")
    }
}

impl Default for WasmSolverService {
    fn default() -> Self {
        Self::new()
    }
}

/// Editor-owned sound audition session.
///
/// Authoring recipes cross this editor-only contract directly into Rust
/// synthesis. Only resolved assets and device commands reach WebAudio.
#[wasm_bindgen]
pub struct WasmEditorAudio {
    runtime: puzzle_audio::AudioRuntime,
    backend: puzzle_web_audio::BrowserAudioBackend,
}

#[wasm_bindgen]
impl WasmEditorAudio {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WasmEditorAudio, JsValue> {
        let catalog = Arc::new(
            AudioAssetCatalog::compile(Vec::new(), Vec::new())
                .map_err(|error| JsValue::from_str(&error.to_string()))?,
        );
        let backend = puzzle_web_audio::BrowserAudioBackend::new(catalog.clone());
        if let Some(error) = backend.initialization_error() {
            return Err(JsValue::from_str(&format!(
                "editor audio output is unavailable: {error}"
            )));
        }
        Ok(Self {
            runtime: puzzle_audio::AudioRuntime::new(catalog, backend.capability()),
            backend,
        })
    }

    pub async fn unlock(&mut self, now_ms: f64) -> Result<(), JsValue> {
        let now_frame = editor_audio_frame(now_ms)?;
        let capability = self.backend.unlock().await.map_err(editor_audio_error)?;
        let commands = self.runtime.set_capability(capability, now_frame);
        self.consume(commands, now_frame)
    }

    /// Connects asynchronous Web Audio events to this Rust-owned audition
    /// session. The callback is a wakeup only; typed feedback remains owned by
    /// `BrowserAudioBackend` until `audio_feedback_event` drains it.
    pub fn set_audio_feedback_wakeup(&self, callback: js_sys::Function) {
        self.backend.set_feedback_wakeup(Rc::new(move || {
            callback
                .call0(&JsValue::UNDEFINED)
                .expect("registered editor audio feedback wakeup failed");
        }));
    }

    /// Drains every queued browser audio event in arrival order.
    ///
    /// Voice failures are contained to their voice by `AudioRuntime`; a
    /// device-scoped failure alone may change output capability. Diagnostics
    /// are returned as JSON so the editor can present every settled failure
    /// without interpreting browser audio state.
    pub fn audio_feedback_event(&mut self, now_ms: f64) -> Result<String, JsValue> {
        let now_frame = editor_audio_frame(now_ms)?;
        let mut commands = Vec::new();
        for voice in self.backend.take_ended_voices() {
            self.runtime.voice_ended(voice);
        }
        for error in self.backend.take_feedback_errors() {
            if let Some(voice) = error.voice_id() {
                self.runtime
                    .report_voice_failure(voice, error.to_string(), now_frame);
            } else {
                commands.extend(
                    self.runtime
                        .report_device_failure(error.to_string(), now_frame),
                );
            }
        }
        commands.extend(
            self.runtime
                .set_capability(self.backend.capability(), now_frame),
        );
        self.consume_device_commands(commands, now_frame);
        Ok(editor_audio_diagnostic_json(
            self.runtime
                .take_diagnostics()
                .into_iter()
                .map(|diagnostic| format!("{diagnostic:?}")),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn configure(
        &mut self,
        sfx_seed: &str,
        sfx_type: &str,
        sfx_volume: f64,
        music_seed: &str,
        music_height: f64,
        music_bars: u16,
        music_bpm: u16,
        music_volume: f64,
        now_ms: f64,
    ) -> Result<(), JsValue> {
        let now_frame = editor_audio_frame(now_ms)?;
        let stop_commands = self.runtime.stop_all();
        self.consume(stop_commands, now_frame)?;
        let catalog = Arc::new(
            AudioAssetCatalog::compile(
                vec![(
                    "editor_sfx".to_string(),
                    SfxRecipe {
                        seed: sfx_seed.to_string(),
                        type_target: sfx_type.to_string(),
                        volume: sfx_volume,
                    },
                )],
                vec![(
                    "editor_music".to_string(),
                    MusicRecipe {
                        seed: music_seed.to_string(),
                        height: music_height,
                        bars: music_bars,
                        bpm: music_bpm,
                        volume: music_volume,
                    },
                )],
            )
            .map_err(|error| {
                JsValue::from_str(&format!("editor audio recipe is invalid: {error}"))
            })?,
        );
        self.backend
            .replace_catalog(catalog.clone())
            .map_err(editor_audio_error)?;
        self.runtime = puzzle_audio::AudioRuntime::new(catalog, self.backend.capability());
        Ok(())
    }

    pub fn play_sfx(&mut self, now_ms: f64) -> Result<(), JsValue> {
        let now_frame = editor_audio_frame(now_ms)?;
        self.require_ready()?;
        let commands = self.runtime.apply(
            puzzle_audio::AudioCommand::PlaySfx {
                asset: SfxAssetId(0),
            },
            now_frame,
        );
        self.consume(commands, now_frame)
    }

    pub fn play_music(&mut self, progress: f64, now_ms: f64) -> Result<(), JsValue> {
        if !progress.is_finite() || !(0.0..1.0).contains(&progress) {
            return Err(JsValue::from_str(
                "editor music progress must be finite and in [0, 1)",
            ));
        }
        self.require_ready()?;
        let track = self
            .runtime
            .catalog()
            .music(MusicAssetId(0))
            .ok_or_else(|| JsValue::from_str("editor music asset is not configured"))?;
        let start_frame = (progress * track.loop_frames() as f64).floor() as u64;
        let now_frame = editor_audio_frame(now_ms)?;
        let mut commands = self.runtime.apply(
            puzzle_audio::AudioCommand::PlayMusic {
                asset: MusicAssetId(0),
            },
            now_frame,
        );
        if start_frame != 0 {
            commands.extend(self.runtime.seek_music(
                puzzle_audio::MusicTarget::All,
                start_frame,
                now_frame,
            ));
        }
        self.consume(commands, now_frame)
    }

    pub fn pause_music(&mut self, now_ms: f64) -> Result<(), JsValue> {
        let now_frame = editor_audio_frame(now_ms)?;
        let commands = self.runtime.apply(
            puzzle_audio::AudioCommand::PauseMusic {
                target: puzzle_audio::MusicTarget::All,
            },
            now_frame,
        );
        self.consume(commands, now_frame)
    }

    pub fn resume_music(&mut self, now_ms: f64) -> Result<(), JsValue> {
        let now_frame = editor_audio_frame(now_ms)?;
        let commands = self.runtime.apply(
            puzzle_audio::AudioCommand::ResumeMusic {
                target: puzzle_audio::MusicTarget::All,
            },
            now_frame,
        );
        self.consume(commands, now_frame)
    }

    pub fn stop_music(&mut self, now_ms: f64) -> Result<(), JsValue> {
        let now_frame = editor_audio_frame(now_ms)?;
        let commands = self.runtime.apply(
            puzzle_audio::AudioCommand::StopMusic {
                target: puzzle_audio::MusicTarget::All,
            },
            now_frame,
        );
        self.consume(commands, now_frame)
    }

    pub fn music_progress(&self, now_ms: f64) -> Result<f64, JsValue> {
        let now_frame = editor_audio_frame(now_ms)?;
        let Some(playback) = self.runtime.music_playback(now_frame) else {
            return Ok(0.0);
        };
        let track = self
            .runtime
            .catalog()
            .music(playback.asset)
            .ok_or_else(|| JsValue::from_str("editor music playback references a missing asset"))?;
        if track.loop_frames() == 0 {
            return Ok(0.0);
        }
        Ok((playback.cursor_frame % track.loop_frames()) as f64 / track.loop_frames() as f64)
    }

    pub fn export_sfx_wav(&self) -> Result<Vec<u8>, JsValue> {
        let id = SfxAssetId(0);
        let clip = self
            .runtime
            .catalog()
            .sfx(id)
            .ok_or_else(|| JsValue::from_str("editor SFX asset is not configured"))?;
        let gain = self
            .runtime
            .catalog()
            .sfx_gain(id)
            .ok_or_else(|| JsValue::from_str("editor SFX gain is not configured"))?;
        encode_wav(clip.sample_rate, 1, clip.samples.len(), |frame, _| {
            clip.samples[frame] * gain
        })
        .map_err(|error| JsValue::from_str(&error))
    }

    pub fn export_music_wav(&self) -> Result<Vec<u8>, JsValue> {
        let id = MusicAssetId(0);
        let track = self
            .runtime
            .catalog()
            .music(id)
            .ok_or_else(|| JsValue::from_str("editor music asset is not configured"))?;
        let gain = self
            .runtime
            .catalog()
            .music_gain(id)
            .ok_or_else(|| JsValue::from_str("editor music gain is not configured"))?;
        let channels = usize::from(track.channels());
        let frames = usize::try_from(track.loop_frames())
            .map_err(|_| JsValue::from_str("editor music loop is too large to export"))?;
        let sample_count = frames
            .checked_mul(channels)
            .ok_or_else(|| JsValue::from_str("editor music WAV sample count overflowed"))?;
        let mut wav = wav_header(track.sample_rate(), track.channels(), sample_count)
            .map_err(|error| JsValue::from_str(&error))?;
        const BLOCK_FRAMES: usize = 1_024;
        let mut samples = vec![0.0_f32; BLOCK_FRAMES * channels];
        let mut scratch = vec![0.0_f64; BLOCK_FRAMES * channels];
        let mut frame = 0_usize;
        while frame < frames {
            let block_frames = (frames - frame).min(BLOCK_FRAMES);
            let block_samples = block_frames * channels;
            track
                .render_interleaved(
                    frame as u64,
                    &mut samples[..block_samples],
                    &mut scratch[..block_samples],
                )
                .map_err(|error| {
                    JsValue::from_str(&format!("editor music export failed: {error}"))
                })?;
            for sample in &samples[..block_samples] {
                push_pcm16(&mut wav, *sample * gain);
            }
            frame += block_frames;
        }
        Ok(wav)
    }

    pub fn stop(&mut self, now_ms: f64) -> Result<(), JsValue> {
        let now_frame = editor_audio_frame(now_ms)?;
        let commands = self.runtime.stop_all();
        self.consume(commands, now_frame)
    }

    pub fn set_visible(&mut self, visible: bool, now_ms: f64) -> Result<(), JsValue> {
        let now_frame = editor_audio_frame(now_ms)?;
        let capability = self.backend.set_visible(visible);
        let commands = self.runtime.set_capability(capability, now_frame);
        self.consume(commands, now_frame)
    }
}

impl WasmEditorAudio {
    fn require_ready(&self) -> Result<(), JsValue> {
        let capability = self.backend.capability();
        if capability == puzzle_audio::AudioCapabilityState::Ready {
            Ok(())
        } else {
            Err(JsValue::from_str(&format!(
                "editor audio output is not ready ({capability:?}); unlock it from a user gesture"
            )))
        }
    }

    fn consume(
        &mut self,
        commands: Vec<puzzle_audio::AudioDeviceCommand>,
        now_frame: u64,
    ) -> Result<(), JsValue> {
        self.consume_device_commands(commands, now_frame);
        let diagnostics = self.runtime.take_diagnostics();
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(JsValue::from_str(&format!(
                "editor audio diagnostics: {diagnostics:?}"
            )))
        }
    }

    fn consume_device_commands(
        &mut self,
        commands: Vec<puzzle_audio::AudioDeviceCommand>,
        now_frame: u64,
    ) {
        let mut commands = VecDeque::from(commands);
        while let Some(command) = commands.pop_front() {
            if let Err(error) = self.backend.consume(command) {
                if let Some(voice) = audio_command_voice(command).or_else(|| error.voice_id()) {
                    self.runtime
                        .report_voice_failure(voice, error.to_string(), now_frame);
                } else {
                    commands.extend(
                        self.runtime
                            .report_device_failure(error.to_string(), now_frame),
                    );
                }
            }
        }
    }
}

fn editor_audio_frame(now_ms: f64) -> Result<u64, JsValue> {
    if !now_ms.is_finite() || now_ms < 0.0 {
        return Err(JsValue::from_str("editor audio timestamp is invalid"));
    }
    Ok((now_ms * 48.0).floor() as u64)
}

fn editor_audio_error(error: puzzle_web_audio::BrowserAudioError) -> JsValue {
    JsValue::from_str(&format!("editor audio output failed: {error}"))
}

fn editor_audio_diagnostic_json(diagnostics: impl IntoIterator<Item = String>) -> String {
    serde_json::to_string(&diagnostics.into_iter().collect::<Vec<_>>())
        .expect("editor audio diagnostic strings should serialize")
}

fn audio_command_voice(command: puzzle_audio::AudioDeviceCommand) -> Option<AudioVoiceId> {
    match command {
        puzzle_audio::AudioDeviceCommand::StartSfx { voice, .. }
        | puzzle_audio::AudioDeviceCommand::StartMusic { voice, .. }
        | puzzle_audio::AudioDeviceCommand::PauseVoice { voice, .. }
        | puzzle_audio::AudioDeviceCommand::ResumeVoice { voice, .. }
        | puzzle_audio::AudioDeviceCommand::StopVoice { voice } => Some(voice),
    }
}

#[wasm_bindgen]
pub fn editor_audio_sfx_types() -> js_sys::Array {
    SFX_TYPES
        .into_iter()
        .chain(["wild", "puzzlescript"])
        .map(JsValue::from_str)
        .collect()
}

#[wasm_bindgen]
pub fn editor_audio_random_sfx_preset(seed: &str, type_target: &str) -> Result<JsValue, JsValue> {
    if type_target != "random"
        && type_target != "wild"
        && type_target != "puzzlescript"
        && !SFX_TYPES.contains(&type_target)
    {
        return Err(JsValue::from_str("editor SFX preset type is unsupported"));
    }
    encode_js_value(
        &serde_json::json!({
            "seed": puzzle_audio::random_audio_preset_seed(seed),
            "type": type_target,
        }),
        "editor SFX preset",
    )
}

#[wasm_bindgen]
pub fn editor_audio_random_music_preset(seed: &str) -> Result<JsValue, JsValue> {
    encode_js_value(
        &serde_json::json!({
            "seed": puzzle_audio::random_audio_preset_seed(seed),
            "height": 0.5,
            "bars": 8,
            "bpm": 110,
        }),
        "editor music preset",
    )
}

fn encode_wav(
    sample_rate: u32,
    channels: u16,
    sample_count: usize,
    mut sample_at: impl FnMut(usize, usize) -> f32,
) -> Result<Vec<u8>, String> {
    if channels == 0 || sample_count % usize::from(channels) != 0 {
        return Err("editor WAV export has invalid channel alignment".to_string());
    }
    let mut wav = wav_header(sample_rate, channels, sample_count)?;
    let channel_count = usize::from(channels);
    for index in 0..sample_count {
        let frame = index / channel_count;
        let channel = index % channel_count;
        push_pcm16(&mut wav, sample_at(frame, channel));
    }
    Ok(wav)
}

fn wav_header(sample_rate: u32, channels: u16, sample_count: usize) -> Result<Vec<u8>, String> {
    if channels == 0 || sample_count % usize::from(channels) != 0 {
        return Err("editor WAV export has invalid channel alignment".to_string());
    }
    let data_len = sample_count
        .checked_mul(2)
        .and_then(|length| u32::try_from(length).ok())
        .ok_or_else(|| "editor WAV export exceeds the RIFF size limit".to_string())?;
    let riff_len = 36_u32
        .checked_add(data_len)
        .ok_or_else(|| "editor WAV export exceeds the RIFF size limit".to_string())?;
    let byte_rate = sample_rate
        .checked_mul(u32::from(channels))
        .and_then(|rate| rate.checked_mul(2))
        .ok_or_else(|| "editor WAV export byte rate overflowed".to_string())?;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_len.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&(channels * 2).to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    Ok(wav)
}

fn push_pcm16(wav: &mut Vec<u8>, sample: f32) {
    let sample = sample.clamp(-1.0, 1.0);
    let pcm = if sample < 0.0 {
        (sample * 32_768.0).round() as i16
    } else {
        (sample * 32_767.0).round() as i16
    };
    wav.extend_from_slice(&pcm.to_le_bytes());
}

fn solver_now_ms(value: f64) -> Result<u64, JsValue> {
    if !value.is_finite() || value < 0.0 || value > u64::MAX as f64 {
        return Err(JsValue::from_str("solver timestamp is invalid"));
    }
    Ok(value as u64)
}

fn decode_js_value<T: DeserializeOwned>(value: JsValue, label: &str) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value)
        .map_err(|error| JsValue::from_str(&format!("{label} is invalid: {error}")))
}

fn encode_js_value<T: Serialize>(value: &T, label: &str) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value)
        .map_err(|error| JsValue::from_str(&format!("{label} could not be encoded: {error}")))
}

#[derive(Default)]
struct SourceAnalysisStore {
    next_revision: SourceAnalysisRevision,
    active: Option<ActiveSourceAnalysis>,
}

struct ActiveSourceAnalysis {
    revision: SourceAnalysisRevision,
    analysis: puzzle_lang::SourceAnalysis,
}

impl SourceAnalysisStore {
    fn allocate_revision(&mut self) -> SourceAnalysisRevision {
        let revision = self.next_revision.max(1);
        self.next_revision = revision
            .checked_add(1)
            .expect("source analysis revision counter exhausted");
        revision
    }

    fn activate(
        &mut self,
        source: &str,
        source_profile: puzzle_lang::PuzzleSourceProfile,
    ) -> SourceAnalysisRevision {
        if let Some(active) = &self.active {
            if active.analysis.source() == source
                && active.analysis.source_profile() == Some(source_profile)
            {
                return active.revision;
            }
        }
        let revision = self.allocate_revision();
        self.active = Some(ActiveSourceAnalysis {
            revision,
            analysis: puzzle_lang::SourceAnalysis::new_for_profile(source, Some(source_profile)),
        });
        revision
    }

    fn apply_edit(
        &mut self,
        revision: SourceAnalysisRevision,
        start_utf16: usize,
        end_utf16: usize,
        insert: &str,
    ) -> Result<
        (
            SourceAnalysisRevision,
            puzzle_lang::SourceAnalysisEditResult,
        ),
        String,
    > {
        let Some(active) = &self.active else {
            return Err(
                "source analysis is not active; activate the source before editing it".to_string(),
            );
        };
        if active.revision != revision {
            return Err(format!(
                "source analysis revision `{revision}` is stale; active revision is `{}`",
                active.revision
            ));
        }
        let start = utf8_offset_from_utf16(active.analysis.source(), start_utf16);
        let end = utf8_offset_from_utf16(active.analysis.source(), end_utf16);
        let next_revision = self.allocate_revision();
        let active = self.active.as_mut().expect("active analysis checked above");
        let result = active
            .analysis
            .apply_edit(puzzle_lang::SourceAnalysisEdit { start, end }, insert)?;
        active.revision = next_revision;
        Ok((next_revision, result))
    }

    fn with_analysis<T>(
        &self,
        revision: SourceAnalysisRevision,
        f: impl FnOnce(&puzzle_lang::SourceAnalysis) -> T,
    ) -> Result<T, String> {
        let Some(active) = &self.active else {
            return Err(
                "source analysis is not active; activate the source before querying it".to_string(),
            );
        };
        if active.revision != revision {
            return Err(format!(
                "source analysis revision `{revision}` is stale; active revision is `{}`",
                active.revision
            ));
        }
        Ok(f(&active.analysis))
    }
}

fn source_analysis_error_js_value(message: String) -> JsValue {
    JsValue::from_str(&message)
}

fn with_source_analysis<T>(
    revision: SourceAnalysisRevision,
    f: impl FnOnce(&puzzle_lang::SourceAnalysis) -> T,
) -> Result<T, String> {
    SOURCE_ANALYSES.with(|store| store.borrow().with_analysis(revision, f))
}

fn utf8_offset_from_utf16(source: &str, utf16_offset: usize) -> usize {
    let mut consumed = 0;
    for (byte_offset, ch) in source.char_indices() {
        if consumed >= utf16_offset {
            return byte_offset;
        }
        let next = consumed + ch.len_utf16();
        if next > utf16_offset {
            return byte_offset;
        }
        consumed = next;
    }
    source.len()
}

fn utf16_offset_from_utf8(source: &str, byte_offset: usize) -> usize {
    source
        .char_indices()
        .take_while(|(index, _)| *index < byte_offset.min(source.len()))
        .map(|(_, ch)| ch.len_utf16())
        .sum()
}

fn source_target_with_utf16_offsets(
    source: &str,
    mut target: puzzle_lang::SourceTarget,
) -> puzzle_lang::SourceTarget {
    target.start = utf16_offset_from_utf8(source, target.start);
    target.end = utf16_offset_from_utf8(source, target.end);
    target.body_start = target
        .body_start
        .map(|offset| utf16_offset_from_utf8(source, offset));
    target.body_end = target
        .body_end
        .map(|offset| utf16_offset_from_utf8(source, offset));
    target
}

#[wasm_bindgen]
pub fn activate_source_analysis_with_profile(
    source: &str,
    source_profile: &str,
) -> Result<SourceAnalysisRevision, JsValue> {
    let profile = match source_profile {
        "puzzle2d" => puzzle_lang::PuzzleSourceProfile::Puzzle2d,
        "puzzle3d" => puzzle_lang::PuzzleSourceProfile::Puzzle3d,
        _ => {
            return Err(JsValue::from_str(
                "source analysis profile must be `puzzle2d` or `puzzle3d`",
            ));
        }
    };
    Ok(SOURCE_ANALYSES.with(|store| store.borrow_mut().activate(source, profile)))
}

#[wasm_bindgen]
pub fn apply_source_analysis_edit(
    revision: SourceAnalysisRevision,
    start_utf16: usize,
    end_utf16: usize,
    insert: &str,
) -> Result<String, JsValue> {
    let (revision, result) = SOURCE_ANALYSES
        .with(|store| {
            store
                .borrow_mut()
                .apply_edit(revision, start_utf16, end_utf16, insert)
        })
        .map_err(source_analysis_error_js_value)?;
    Ok(format!(
        "{{\"revision\":{revision},\"rescannedLines\":{},\"totalLines\":{},\"parserCatalogReused\":{}}}",
        result.rescanned_lines,
        result.total_lines,
        if result.parser_catalog_reused {
            "true"
        } else {
            "false"
        }
    ))
}

#[wasm_bindgen]
pub fn active_source_analysis_json(revision: SourceAnalysisRevision) -> Result<String, JsValue> {
    with_source_analysis(revision, puzzle_lang::SourceAnalysis::analysis_json)
        .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_highlight_range_json(
    revision: SourceAnalysisRevision,
    range_start_utf16: usize,
    range_end_utf16: usize,
    include_outline: bool,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        let source = analysis.source();
        let range_start = utf8_offset_from_utf16(source, range_start_utf16);
        let range_end = utf8_offset_from_utf16(source, range_end_utf16);
        analysis.highlight_range_json(range_start, range_end, include_outline)
    })
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_outline_json(
    revision: SourceAnalysisRevision,
) -> Result<String, JsValue> {
    with_source_analysis(revision, puzzle_lang::SourceAnalysis::outline_json)
        .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_suggest_source_completions(
    revision: SourceAnalysisRevision,
    cursor_utf16_offset: usize,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        let source = analysis.source();
        let cursor_offset = utf8_offset_from_utf16(source, cursor_utf16_offset);
        let mut completions = analysis.completion_list(cursor_offset);
        completions.replace_start = utf16_offset_from_utf8(source, completions.replace_start);
        completions.replace_end = utf16_offset_from_utf8(source, completions.replace_end);
        puzzle_lang::completion_list_json(&completions)
    })
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_resolve_source_target(
    revision: SourceAnalysisRevision,
    cursor_utf16_offset: usize,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        let source = analysis.source();
        let cursor_offset = utf8_offset_from_utf16(source, cursor_utf16_offset);
        let target = analysis
            .resolve_target(cursor_offset)
            .map(|target| source_target_with_utf16_offsets(source, target));
        puzzle_lang::source_target_json(target.as_ref())
    })
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_mutate_visual(
    revision: SourceAnalysisRevision,
    request_json: &str,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        puzzle_lang::mutate_visual_source(analysis.source(), request_json).map(|result| {
            let start = utf16_offset_from_utf8(&result.source, result.start);
            let end = utf16_offset_from_utf8(&result.source, result.end);
            serde_json::json!({
                "source": result.source,
                "start": start,
                "end": end,
                "name": result.name,
            })
            .to_string()
        })
    })
    .map_err(source_analysis_error_js_value)?
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_sound_request(
    revision: SourceAnalysisRevision,
    request_json: &str,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        let source = analysis.source();
        let request: puzzle_lang::SoundSourceRequest = serde_json::from_str(request_json)
            .map_err(|error| format!("invalid sound source request: {error}"))?;
        let request = match request {
            puzzle_lang::SoundSourceRequest::Inspect { cursor } => {
                puzzle_lang::SoundSourceRequest::Inspect {
                    cursor: utf8_offset_from_utf16(source, cursor),
                }
            }
            puzzle_lang::SoundSourceRequest::Update {
                target_start,
                original_name,
                definition,
            } => puzzle_lang::SoundSourceRequest::Update {
                target_start: utf8_offset_from_utf16(source, target_start),
                original_name,
                definition,
            },
            request => request,
        };
        analysis.sound_source_request(request).and_then(|response| {
            let value = match response {
                puzzle_lang::SoundSourceResponse::Inspection { definition } => {
                    let definition = definition.map(|mut definition| {
                        definition.start = utf16_offset_from_utf8(source, definition.start);
                        definition.end = utf16_offset_from_utf8(source, definition.end);
                        definition
                    });
                    serde_json::json!({ "kind": "inspection", "definition": definition })
                }
                puzzle_lang::SoundSourceResponse::Formatted { line, definition } => {
                    serde_json::json!({
                        "kind": "formatted",
                        "line": line,
                        "definition": definition,
                    })
                }
                puzzle_lang::SoundSourceResponse::Mutation { mut result } => {
                    result.selection_start =
                        utf16_offset_from_utf8(&result.source, result.selection_start);
                    result.selection_end =
                        utf16_offset_from_utf8(&result.source, result.selection_end);
                    result.definition_start =
                        utf16_offset_from_utf8(&result.source, result.definition_start);
                    result.definition_end =
                        utf16_offset_from_utf8(&result.source, result.definition_end);
                    serde_json::json!({ "kind": "mutation", "result": result })
                }
            };
            serde_json::to_string(&value)
                .map_err(|error| format!("could not encode sound source response: {error}"))
        })
    })
    .map_err(source_analysis_error_js_value)?
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_level_source_request(
    revision: SourceAnalysisRevision,
    request_json: &str,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        let source = analysis.source();
        let request: puzzle_lang::LevelSourceRequest = serde_json::from_str(request_json)
            .map_err(|error| format!("invalid level source request: {error}"))?;
        let request = match request {
            puzzle_lang::LevelSourceRequest::Insert {
                name,
                namespace,
                rows,
                local_legends,
                cursor,
                create_container,
            } => puzzle_lang::LevelSourceRequest::Insert {
                name,
                namespace,
                rows,
                local_legends,
                cursor: cursor.map(|offset| utf8_offset_from_utf16(source, offset)),
                create_container,
            },
            puzzle_lang::LevelSourceRequest::Update {
                target_start,
                name,
                rows,
                local_legends,
            } => puzzle_lang::LevelSourceRequest::Update {
                target_start: utf8_offset_from_utf16(source, target_start),
                name,
                rows,
                local_legends,
            },
            request => request,
        };
        analysis.level_source_request(request).and_then(|response| {
            serde_json::to_string(&serde_json::json!({
                "source": response.source,
                "start": utf16_offset_from_utf8(&response.source, response.start),
                "end": utf16_offset_from_utf8(&response.source, response.end),
                "text": response.text,
            }))
            .map_err(|error| format!("could not encode level source response: {error}"))
        })
    })
    .map_err(source_analysis_error_js_value)?
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_import_at_json(
    revision: SourceAnalysisRevision,
    document_path: &str,
    cursor_utf16_offset: usize,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        let source = analysis.source();
        let cursor = utf8_offset_from_utf16(source, cursor_utf16_offset);
        let reference = analysis
            .import_reference_at(document_path, cursor)
            .map(|reference| {
                serde_json::json!({
                    "range": {
                        "start": utf16_offset_from_utf8(source, reference.range.start),
                        "end": utf16_offset_from_utf8(source, reference.range.end),
                    },
                    "pathRange": {
                        "start": utf16_offset_from_utf8(source, reference.path_range.start),
                        "end": utf16_offset_from_utf8(source, reference.path_range.end),
                    },
                    "rawPath": reference.raw_path,
                    "resolvedPath": reference.resolved_path,
                })
            });
        serde_json::json!({ "version": 1, "reference": reference }).to_string()
    })
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn active_source_analysis_entries_json(
    revision: SourceAnalysisRevision,
) -> Result<String, JsValue> {
    with_source_analysis(revision, puzzle_lang::SourceAnalysis::entries_json)
        .map_err(source_analysis_error_js_value)
}

/// Returns level-editor metadata for the active source snapshot. Board cells and
/// visual payloads deliberately travel through their own on-demand exports.
#[wasm_bindgen]
pub fn active_source_analysis_level_editor_manifest_json(
    revision: SourceAnalysisRevision,
) -> Result<String, JsValue> {
    with_source_analysis(
        revision,
        puzzle_lang::SourceAnalysis::level_editor_manifest_json,
    )
    .map_err(source_analysis_error_js_value)?
    .map_err(source_analysis_error_js_value)
}

/// Returns a typed object-ID buffer for one integrated level state. Pass `-1` for
/// the composite state, otherwise pass the authored ASCII layer index.
#[wasm_bindgen]
pub fn active_source_analysis_level_editor_level_slots(
    revision: SourceAnalysisRevision,
    level_index: usize,
    authored_layer: i32,
) -> Result<Vec<u32>, JsValue> {
    let authored_layer = match authored_layer {
        -1 => None,
        value if value >= 0 => Some(value as usize),
        value => {
            return Err(source_analysis_error_js_value(format!(
                "level editor authored layer must be -1 or non-negative, got {value}"
            )));
        }
    };
    with_source_analysis(revision, |analysis| {
        analysis.level_editor_level_slots(level_index, authored_layer)
    })
    .map_err(source_analysis_error_js_value)?
    .map_err(source_analysis_error_js_value)
}

/// Returns one renderer-ready visual payload by canonical object ID.
#[wasm_bindgen]
pub fn active_source_analysis_level_editor_visual_json(
    revision: SourceAnalysisRevision,
    object_id: u16,
) -> Result<String, JsValue> {
    with_source_analysis(revision, |analysis| {
        analysis.level_editor_visual_payload_json(object_id)
    })
    .map_err(source_analysis_error_js_value)?
    .map_err(source_analysis_error_js_value)
}

#[wasm_bindgen]
pub fn compile_preview(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, JsValue> {
    let path = if puzzle_path.trim().is_empty() {
        "game.puzzle"
    } else {
        puzzle_path
    };
    html_play::export_editor_preview_html_from_source(source, path, game_css, game_visuals_js)
        .map_err(|error| diagnostic_report_js_value(&error))
}

fn expand_workspace_entry(
    entry_path: &str,
    documents: WorkspaceSourceDocuments,
) -> Result<String, JsValue> {
    let documents = decode_js_value::<Vec<puzzle_lang::WorkspaceSourceDocument>>(
        documents.into(),
        "workspace source documents",
    )?;
    puzzle_lang::expand_game_imports_from_documents(entry_path, &documents)
        .map_err(|error| diagnostic_report_js_value(&error))
}

#[wasm_bindgen]
pub fn workspace_presentation_manifest(
    entry_path: &str,
    documents: WorkspaceSourceDocuments,
) -> Result<WorkspacePresentationManifestJs, JsValue> {
    let documents = decode_js_value::<Vec<puzzle_lang::WorkspaceSourceDocument>>(
        documents.into(),
        "workspace source documents",
    )?;
    let manifest = puzzle_lang::workspace_presentation_manifest(entry_path, &documents)
        .map_err(|error| diagnostic_report_js_value(&error))?;
    encode_js_value(&manifest, "workspace presentation manifest")
        .map(WorkspacePresentationManifestJs::from)
}

#[wasm_bindgen]
pub fn compile_workspace_preview(
    entry_path: &str,
    documents: WorkspaceSourceDocuments,
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, JsValue> {
    let documents = decode_js_value::<Vec<puzzle_lang::WorkspaceSourceDocument>>(
        documents.into(),
        "workspace source documents",
    )?;
    compile_workspace_preview_from_documents(entry_path, &documents, game_css, game_visuals_js)
        .map_err(|error| diagnostic_report_js_value(&error))
}

fn compile_workspace_preview_from_documents(
    entry_path: &str,
    documents: &[puzzle_lang::WorkspaceSourceDocument],
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, puzzle_lang::DiagnosticReport> {
    let expanded =
        puzzle_lang::expand_game_imports_from_documents_with_origins(entry_path, documents)?;
    html_play::export_editor_preview_html_from_source(
        &expanded.source,
        entry_path,
        game_css,
        game_visuals_js,
    )
    .map_err(|error| expanded.remap_diagnostic_report(error))
}

#[wasm_bindgen]
pub fn export_html(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
    player_runtime_module_js: &str,
    player_runtime_wasm_base64: &str,
) -> Result<String, JsValue> {
    let path = if puzzle_path.trim().is_empty() {
        "game.puzzle"
    } else {
        puzzle_path
    };
    html_play::export_html_from_source_with_embedded_wasm(
        source,
        path,
        game_css,
        game_visuals_js,
        player_runtime_module_js,
        player_runtime_wasm_base64,
    )
    .map_err(|error| diagnostic_report_js_value(&error))
}

#[wasm_bindgen]
pub fn export_workspace_html(
    entry_path: &str,
    documents: WorkspaceSourceDocuments,
    game_css: &str,
    game_visuals_js: &str,
    player_runtime_module_js: &str,
    player_runtime_wasm_base64: &str,
) -> Result<String, JsValue> {
    let source = expand_workspace_entry(entry_path, documents)?;
    html_play::export_html_from_source_with_embedded_wasm(
        &source,
        entry_path,
        game_css,
        game_visuals_js,
        player_runtime_module_js,
        player_runtime_wasm_base64,
    )
    .map_err(|error| diagnostic_report_js_value(&error))
}

#[wasm_bindgen]
pub fn generate_visuals_js(source: &str, base_visuals_js: &str) -> Result<String, JsValue> {
    html_play::export_visuals_js_from_source(source, base_visuals_js)
        .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn translate_puzzlescript(source: &str) -> Result<String, JsValue> {
    puzzle_lang::translate_puzzlescript_to_canonical(source)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

fn diagnostic_report_js_value(report: &puzzle_lang::DiagnosticReport) -> JsValue {
    let payload = js_sys::Object::new();
    let diagnostics = js_sys::Array::new();
    for diagnostic in report.diagnostics() {
        diagnostics.push(&diagnostic_js_value(diagnostic));
    }
    let _ = js_sys::Reflect::set(
        &payload,
        &JsValue::from_str("diagnostics"),
        diagnostics.as_ref(),
    );
    payload.into()
}

#[cfg(test)]
fn diagnostic_report_json(report: &puzzle_lang::DiagnosticReport) -> String {
    let mut body = String::new();
    push_diagnostics_json(&mut body, report.diagnostics());
    body
}

#[cfg(test)]
fn push_diagnostics_json(out: &mut String, diagnostics: &[puzzle_lang::Diagnostic]) {
    out.push_str("{\"diagnostics\":[");
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_diagnostic_json(out, diagnostic);
    }
    out.push_str("]}");
}

#[cfg(test)]
fn push_diagnostic_json(out: &mut String, diagnostic: &puzzle_lang::Diagnostic) {
    let span = diagnostic.primary_span.as_ref();
    out.push('{');
    push_json_pair(out, "severity", diagnostic.severity.as_str());
    out.push(',');
    push_json_pair(out, "code", diagnostic.code);
    out.push(',');
    push_json_pair(
        out,
        "file",
        span.and_then(|span| span.file.as_deref()).unwrap_or(""),
    );
    out.push(',');
    push_json_option_number(out, "line", span.and_then(|span| span.line));
    out.push(',');
    push_json_option_number(out, "column", span.and_then(|span| span.column));
    out.push(',');
    push_json_option_string(
        out,
        "sourceLine",
        span.and_then(|span| span.source_line.as_deref()),
    );
    out.push(',');
    push_json_pair(out, "message", &diagnostic.message);
    out.push('}');
}

#[cfg(test)]
fn push_json_pair(out: &mut String, key: &str, value: &str) {
    push_json_string(out, key);
    out.push(':');
    push_json_string(out, value);
}

#[cfg(test)]
fn push_json_option_number(out: &mut String, key: &str, value: Option<usize>) {
    push_json_string(out, key);
    out.push(':');
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

#[cfg(test)]
fn push_json_option_string(out: &mut String, key: &str, value: Option<&str>) {
    push_json_string(out, key);
    out.push(':');
    match value {
        Some(value) => push_json_string(out, value),
        None => out.push_str("null"),
    }
}

#[cfg(test)]
fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn diagnostic_js_value(diagnostic: &puzzle_lang::Diagnostic) -> JsValue {
    let payload = js_sys::Object::new();
    let span = diagnostic.primary_span.as_ref();
    set_js_string(&payload, "severity", diagnostic.severity.as_str());
    set_js_string(&payload, "code", diagnostic.code);
    set_js_string(
        &payload,
        "file",
        span.and_then(|span| span.file.as_deref()).unwrap_or(""),
    );
    set_js_optional_number(&payload, "line", span.and_then(|span| span.line));
    set_js_optional_number(&payload, "column", span.and_then(|span| span.column));
    set_js_optional_string(
        &payload,
        "sourceLine",
        span.and_then(|span| span.source_line.as_deref()),
    );
    set_js_string(&payload, "message", &diagnostic.message);
    payload.into()
}

fn set_js_string(payload: &js_sys::Object, key: &str, value: &str) {
    let _ = js_sys::Reflect::set(payload, &JsValue::from_str(key), &JsValue::from_str(value));
}

fn set_js_optional_number(payload: &js_sys::Object, key: &str, value: Option<usize>) {
    let value = value
        .map(|value| JsValue::from_f64(value as f64))
        .unwrap_or(JsValue::NULL);
    let _ = js_sys::Reflect::set(payload, &JsValue::from_str(key), &value);
}

fn set_js_optional_string(payload: &js_sys::Object, key: &str, value: Option<&str>) {
    let value = value.map(JsValue::from_str).unwrap_or(JsValue::NULL);
    let _ = js_sys::Reflect::set(payload, &JsValue::from_str(key), &value);
}

#[cfg(test)]
mod tests {
    use super::{
        SourceAnalysisRevision, activate_source_analysis_with_profile,
        active_source_analysis_entries_json, active_source_analysis_highlight_range_json,
        active_source_analysis_json, active_source_analysis_outline_json,
        active_source_analysis_suggest_source_completions, apply_source_analysis_edit,
        compile_preview, compile_workspace_preview_from_documents, diagnostic_report_json,
        editor_audio_diagnostic_json, encode_wav, utf8_offset_from_utf16, utf16_offset_from_utf8,
        with_source_analysis,
    };

    fn activate_puzzle2d_source_analysis(source: &str) -> SourceAnalysisRevision {
        activate_source_analysis_with_profile(source, "puzzle2d")
            .expect("activate profile-aware source analysis")
    }

    #[test]
    fn editor_audio_feedback_diagnostics_preserve_arrival_order() {
        assert_eq!(
            editor_audio_diagnostic_json([
                "voice 4 failed".to_string(),
                "voice 7 failed".to_string()
            ]),
            r#"["voice 4 failed","voice 7 failed"]"#
        );
    }

    #[test]
    fn editor_wav_export_has_canonical_header_and_sample_count() {
        let wav =
            encode_wav(48_000, 1, 3, |frame, _| [0.0, 1.0, -1.0][frame]).expect("small editor WAV");
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 6);
        assert_eq!(wav.len(), 50);
    }

    fn invalid_workspace_game(statement: &str) -> String {
        format!(
            r#"title = "Diagnostic origin"

puzzle main {{
layers {{
base = Floor
}}
visuals {{
}}
rules {{
{statement}
}}
levels {{
legend {{
. = empty
}}
level "first"
.
}}
}}
"#
        )
    }

    #[test]
    fn compile_preview_accepts_at_prefixed_object_single_color_visual() {
        let source = r##"
title at_prefixed_object_single_color_preview

puzzle default {
layers {
@floor_slot = @Floor
}
visuals {
@Floor
#eeeeee
}
rules {

}
levels {
legend {
. = empty
}
level "start"
.
}
}
"##;

        let html = compile_preview(source, "game.puzzle", "", "").expect("compile preview");

        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("#eeeeee"));
        assert!(html.contains("PuzzleStudioPreviewState"));
        assert!(html.contains("PuzzleRuntimeWasmLoader"));
        assert!(html.contains("ui-tap"));
        assert!(html.contains("buildSelectLayers"));
    }

    #[test]
    fn diagnostic_report_json_preserves_line_for_editor_links() {
        let report = puzzle_lang::DiagnosticReport::error_at_source_line_number(
            "`action` statements were removed",
            "action jump",
            9,
        );

        let json = diagnostic_report_json(&report);

        assert!(json.contains(r#""diagnostics":["#));
        assert!(json.contains(r#""line":9"#));
        assert!(json.contains(r#""sourceLine":"action jump""#));
        assert!(json.contains(r#""message":"`action` statements were removed""#));
    }

    #[test]
    fn workspace_preview_diagnostic_points_to_imported_document_line() {
        let imported_source = invalid_workspace_game("unknown_imported_statement");
        let expected_line = imported_source
            .lines()
            .position(|line| line == "unknown_imported_statement")
            .expect("invalid imported statement")
            + 1;
        let documents = vec![
            puzzle_lang::WorkspaceSourceDocument {
                path: "games/demo/game.puzzle".to_string(),
                source: "import \"parts/game.puzzle\"\n".to_string(),
            },
            puzzle_lang::WorkspaceSourceDocument {
                path: "games/demo/parts/game.puzzle".to_string(),
                source: imported_source,
            },
        ];

        let report =
            compile_workspace_preview_from_documents("games/demo/game.puzzle", &documents, "", "")
                .expect_err("invalid imported source should fail preview compile");
        let span = report.diagnostics()[0]
            .primary_span
            .as_ref()
            .expect("imported diagnostic span");

        assert_eq!(span.file.as_deref(), Some("games/demo/parts/game.puzzle"));
        assert_eq!(span.line, Some(expected_line));
    }

    #[test]
    fn workspace_preview_diagnostic_remaps_entry_line_after_import_expansion() {
        let game_source = invalid_workspace_game("unknown_entry_statement");
        let source = format!("import \"padding.puzzle\"\n{game_source}");
        let expected_line = source
            .lines()
            .position(|line| line == "unknown_entry_statement")
            .expect("invalid entry statement")
            + 1;
        let documents = vec![
            puzzle_lang::WorkspaceSourceDocument {
                path: "game.puzzle".to_string(),
                source,
            },
            puzzle_lang::WorkspaceSourceDocument {
                path: "padding.puzzle".to_string(),
                source: "// first imported line\n// second imported line\n".to_string(),
            },
        ];

        let report = compile_workspace_preview_from_documents("game.puzzle", &documents, "", "")
            .expect_err("invalid entry source should fail preview compile");
        let span = report.diagnostics()[0]
            .primary_span
            .as_ref()
            .expect("entry diagnostic span");

        assert_eq!(span.file.as_deref(), Some("game.puzzle"));
        assert_eq!(span.line, Some(expected_line));
    }

    #[test]
    fn workspace_preview_import_error_points_to_import_statement() {
        let documents = vec![puzzle_lang::WorkspaceSourceDocument {
            path: "game.puzzle".to_string(),
            source: "// heading\nimport \"missing.puzzle\"\n".to_string(),
        }];

        let report = compile_workspace_preview_from_documents("game.puzzle", &documents, "", "")
            .expect_err("missing import should fail preview compile");
        let span = report.diagnostics()[0]
            .primary_span
            .as_ref()
            .expect("import diagnostic span");

        assert_eq!(span.file.as_deref(), Some("game.puzzle"));
        assert_eq!(span.line, Some(2));
        assert_eq!(
            span.source_line.as_deref(),
            Some("import \"missing.puzzle\"")
        );
    }

    #[test]
    fn active_source_analysis_reuses_exact_source_and_rejects_stale_revisions() {
        let source = "puzzle Demo {\n  sounds {\n    \n  }\n}\n";
        let cursor = source.find("    ").unwrap() + 4;
        let revision = activate_puzzle2d_source_analysis(source);
        assert_eq!(activate_puzzle2d_source_analysis(source), revision);
        let puzzle3_revision = activate_source_analysis_with_profile(source, "puzzle3d")
            .expect("activate 3D source profile");
        assert_ne!(puzzle3_revision, revision);
        let revision = activate_puzzle2d_source_analysis(source);
        assert_ne!(revision, puzzle3_revision);

        let analysis = active_source_analysis_json(revision).expect("analysis json");
        assert!(analysis.contains(r#""version":2"#));
        assert!(analysis.contains(r#""entries":"#));

        let highlight = active_source_analysis_highlight_range_json(
            revision,
            0,
            source.encode_utf16().count(),
            false,
        )
        .expect("highlight spans");
        assert!(highlight.contains(r#""version":3"#));
        assert!(highlight.contains(r#""offsetEncoding":"utf8""#));
        assert!(highlight.contains(&format!(r#""range":{{"start":0,"end":{}}}"#, source.len())));
        assert!(highlight.contains(r#""spans":["#));
        assert!(!highlight.contains(r#""html""#));

        let completions = active_source_analysis_suggest_source_completions(revision, cursor)
            .expect("completions");
        assert!(completions.contains(r#""label":"sfx""#));
        assert!(completions.contains(r#""label":"music""#));

        let entries = active_source_analysis_entries_json(revision).expect("entries");
        assert!(entries.contains(r#""entries":"#));

        let outline = active_source_analysis_outline_json(revision).expect("outline");
        assert!(outline.contains(r#""items":"#));

        let next_revision = activate_puzzle2d_source_analysis("puzzle Other {}\n");
        assert_ne!(next_revision, revision);
        assert!(
            with_source_analysis(revision, puzzle_lang::SourceAnalysis::analysis_json).is_err()
        );
    }

    #[test]
    fn active_source_analysis_boundary_uses_browser_utf16_offsets() {
        let source = "title = \"😀\"\npuzzle Demo {\n  sounds {\n    \n  }\n}\n";
        let cursor_byte = source.find("    ").unwrap() + 4;
        let cursor_utf16 = source[..cursor_byte].encode_utf16().count();
        let revision = activate_puzzle2d_source_analysis(source);

        assert_eq!(utf8_offset_from_utf16(source, cursor_utf16), cursor_byte);
        assert_eq!(utf16_offset_from_utf8(source, cursor_byte), cursor_utf16);

        let completions = active_source_analysis_suggest_source_completions(revision, cursor_utf16)
            .expect("source completions");
        assert!(completions.contains(&format!(r#""replaceStart":{cursor_utf16}"#)));
    }

    #[test]
    fn active_source_analysis_applies_utf16_edits_to_the_existing_session() {
        let source = "puzzle Demo {\n}\n// note\n";
        let revision = activate_puzzle2d_source_analysis(source);
        let cursor_byte = source.find("note").unwrap() + "note".len();
        let cursor_utf16 = source[..cursor_byte].encode_utf16().count();

        let update = apply_source_analysis_edit(revision, cursor_utf16, cursor_utf16, "😀")
            .expect("incremental update");
        assert!(update.contains(r#""rescannedLines":1"#));
        assert!(update.contains(r#""parserCatalogReused":true"#));
        let next_revision = update
            .strip_prefix("{\"revision\":")
            .and_then(|tail| tail.split(',').next())
            .and_then(|value| value.parse::<u32>().ok())
            .expect("updated revision");

        assert!(
            with_source_analysis(revision, puzzle_lang::SourceAnalysis::analysis_json).is_err()
        );
        let highlight = active_source_analysis_highlight_range_json(
            next_revision,
            0,
            source.encode_utf16().count() + 2,
            false,
        )
        .expect("updated highlight");
        assert!(highlight.contains(&format!(r#""sourceLength":{}"#, source.len() + 4)));
    }
}
