use std::sync::Arc;

use serde::{Deserialize, Serialize};

mod composition;
mod dsp;
mod timbre;

pub use timbre::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MusicRecipe {
    pub seed: String,
    pub height: f64,
    pub bars: u16,
    pub bpm: u16,
    pub volume: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedMusicTrack {
    sample_rate: u32,
    channels: u16,
    loop_frames: u64,
    score: Arc<MusicScore>,
    render_index: Arc<dsp::MusicRenderIndex>,
}

impl GeneratedMusicTrack {
    pub(crate) fn from_resolved_score(
        sample_rate: u32,
        channels: u16,
        loop_frames: u64,
        score: MusicScore,
    ) -> Result<Self, String> {
        let render_index = dsp::MusicRenderIndex::new(sample_rate, loop_frames, &score)?;
        Ok(Self {
            sample_rate,
            channels,
            loop_frames,
            score: Arc::new(score),
            render_index: Arc::new(render_index),
        })
    }

    /// Returns the immutable resolved score used to build this track.
    ///
    /// Mutation is deliberately not exposed: the render index and score form
    /// one canonical asset and must never be allowed to diverge.
    pub fn score(&self) -> &MusicScore {
        &self.score
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn loop_frames(&self) -> u64 {
        self.loop_frames
    }

    /// Returns one sample from the canonical Rust music stream.
    ///
    /// This stream is the cross-platform audio contract. Historical browser
    /// oscillator and filter behavior is pinned through score, timbre, and
    /// automation goldens rather than retained as a second playback path,
    /// because browser nodes never defined one portable PCM bit stream.
    pub fn sample_at(&self, frame: u64, channel: u16) -> f32 {
        dsp::sample_at(self, frame, channel)
    }

    /// Renders canonical interleaved PCM without allocating.
    ///
    /// `scratch` must contain at least `output.len()` elements. It is exposed
    /// rather than allocated internally so realtime backends can reserve it
    /// before entering their device callback. Accumulation remains `f64`, in
    /// score-event and note order, so every output sample is bit-identical to
    /// the corresponding [`Self::sample_at`] call.
    pub fn render_interleaved(
        &self,
        start_frame: u64,
        output: &mut [f32],
        scratch: &mut [f64],
    ) -> Result<(), MusicRenderError> {
        dsp::render_interleaved(self, start_frame, output, scratch)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicRenderError {
    ZeroChannels,
    OutputNotFrameAligned { samples: usize, channels: u16 },
    ScratchTooSmall { required: usize, available: usize },
}

impl std::fmt::Display for MusicRenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroChannels => write!(formatter, "music track has zero channels"),
            Self::OutputNotFrameAligned { samples, channels } => write!(
                formatter,
                "music output has {samples} samples, which is not aligned to {channels} channels"
            ),
            Self::ScratchTooSmall {
                required,
                available,
            } => write!(
                formatter,
                "music render scratch needs {required} samples, but only {available} are available"
            ),
        }
    }
}

impl std::error::Error for MusicRenderError {}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct MusicScore {
    pub transport: MusicTransport,
    pub mix: MusicMix,
    pub timbres: Option<Box<MusicTimbres>>,
    pub events: Vec<MusicScoreEvent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MusicTransport {
    pub bpm: u16,
    pub bars: u16,
    pub steps_per_bar: u8,
    pub step_duration_beats: f64,
    pub loop_steps: u32,
}

impl Default for MusicTransport {
    fn default() -> Self {
        Self {
            bpm: 100,
            bars: 0,
            steps_per_bar: 16,
            step_duration_beats: 0.25,
            loop_steps: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MusicMix {
    pub volume: f64,
    pub playback_tone: PlaybackTone,
}

impl Default for MusicMix {
    fn default() -> Self {
        Self {
            volume: 1.0,
            playback_tone: PlaybackTone::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaybackTone {
    pub height: f64,
    pub brightness: f64,
    pub presence: f64,
    pub attack: f64,
    pub pitch_shift: f64,
    pub brightness_tilt: f64,
    pub attack_shape: f64,
    pub tone_filter: f64,
    pub bass_filter: f64,
    pub noise_filter: f64,
    pub lead_gain: f64,
    pub harmony_gain: f64,
    pub bass_gain: f64,
    pub high_percussion_gain: f64,
    pub low_percussion_gain: f64,
    pub identity_gain: f64,
    pub time_gain: f64,
    pub color_gain: f64,
    pub boundary_gain: f64,
}

impl Default for PlaybackTone {
    fn default() -> Self {
        Self {
            height: 0.5,
            brightness: 0.5,
            presence: 0.5,
            attack: 0.5,
            pitch_shift: 0.0,
            brightness_tilt: 0.0,
            attack_shape: 0.0,
            tone_filter: 1.0,
            bass_filter: 1.0,
            noise_filter: 1.0,
            lead_gain: 1.0,
            harmony_gain: 0.74,
            bass_gain: 1.0,
            high_percussion_gain: 1.0,
            low_percussion_gain: 1.0,
            identity_gain: 1.0,
            time_gain: 1.0,
            color_gain: 1.0,
            boundary_gain: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PitchedTimbre {
    pub role: MusicRole,
    pub gain: f64,
    pub field: SpectralTimbre,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PercussionTimbre {
    pub voice: NoiseVoice,
    pub gain: f64,
    pub field: TransientTimbre,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MusicTimbres {
    pub identity: PitchedTimbre,
    pub time: PitchedTimbre,
    pub tone: PitchedTimbre,
    pub motion: PitchedTimbre,
    pub color: PitchedTimbre,
    pub boundary: PitchedTimbre,
    pub kick: PercussionTimbre,
    pub snare: PercussionTimbre,
    pub hat: PercussionTimbre,
}

impl MusicTimbres {
    pub fn pitched(&self, role: MusicRole) -> &PitchedTimbre {
        match role {
            MusicRole::Identity => &self.identity,
            MusicRole::Time => &self.time,
            MusicRole::Tone => &self.tone,
            MusicRole::Motion => &self.motion,
            MusicRole::Color => &self.color,
            MusicRole::Boundary => &self.boundary,
        }
    }

    pub fn percussion(&self, voice: NoiseVoice) -> &PercussionTimbre {
        match voice {
            NoiseVoice::Kick => &self.kick,
            NoiseVoice::Snare => &self.snare,
            NoiseVoice::Hat => &self.hat,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicTrack {
    Lead,
    Counter,
    Chord,
    Bass,
    Drums,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicRole {
    Identity,
    Time,
    Tone,
    Motion,
    Color,
    Boundary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoiseVoice {
    Kick,
    Snare,
    Hat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicNote {
    Midi(i16),
    Noise(NoiseVoice),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MusicScoreEvent {
    /// Stable identity in sorted score order. All stochastic source material is
    /// keyed by this identity and local sample position, never by device time.
    pub event_id: u32,
    pub track: MusicTrack,
    pub step: u32,
    pub duration_steps: u16,
    pub notes: Vec<MusicNote>,
    pub timbre: MusicTimbreRef,
    pub role: MusicRole,
    pub velocity: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicTimbreRef {
    Pitched(MusicRole),
    Percussion(NoiseVoice),
}

pub fn generate_music(recipe: &MusicRecipe) -> Result<GeneratedMusicTrack, String> {
    composition::generate(recipe)
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct MusicWorkletAssetRef<'a> {
    version: u8,
    sample_rate: u32,
    channels: u16,
    loop_frames: u64,
    score: &'a MusicScore,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MusicWorkletAsset {
    version: u8,
    sample_rate: u32,
    channels: u16,
    loop_frames: u64,
    score: MusicScore,
}

pub fn encode_music_worklet_asset(track: &GeneratedMusicTrack) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&MusicWorkletAssetRef {
        version: 1,
        sample_rate: track.sample_rate,
        channels: track.channels,
        loop_frames: track.loop_frames,
        score: &track.score,
    })
    .map_err(|error| format!("music worklet asset serialization failed: {error}"))
}

pub fn decode_music_worklet_asset(bytes: &[u8]) -> Result<GeneratedMusicTrack, String> {
    let asset = serde_json::from_slice::<MusicWorkletAsset>(bytes)
        .map_err(|error| format!("music worklet asset decoding failed: {error}"))?;
    if asset.version != 1 {
        return Err(format!(
            "unsupported music worklet asset version {}",
            asset.version
        ));
    }
    if asset.sample_rate == 0 || asset.channels == 0 || asset.loop_frames == 0 {
        return Err(
            "music worklet asset requires non-zero sample rate, channels, and loop frames"
                .to_string(),
        );
    }
    if asset.score.timbres.is_none() {
        return Err("music worklet asset is missing resolved timbres".to_string());
    }
    GeneratedMusicTrack::from_resolved_score(
        asset.sample_rate,
        asset.channels,
        asset.loop_frames,
        asset.score,
    )
}
