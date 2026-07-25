use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};

use bevy::{
    audio::{
        AddAudioSource, AudioSinkPlayback, ChannelCount, Decodable, SampleRate, Source, Volume,
    },
    prelude::*,
    reflect::TypePath,
};
use puzzle_audio::{
    AudioDeviceCommand, AudioDeviceStateError, AudioDeviceVoiceRegistry, AudioVoiceId,
    GeneratedMusicTrack, GeneratedSfxClip, MusicAssetId, SfxAssetId,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NativeAudioOperation {
    SpawnSfx {
        voice: AudioVoiceId,
        asset: SfxAssetId,
        gain: f32,
    },
    SpawnMusic {
        voice: AudioVoiceId,
        asset: MusicAssetId,
        start_frame: u64,
        gain: f32,
    },
    PauseAndSeek {
        voice: AudioVoiceId,
        at_frame: u64,
    },
    ResumeAndSeek {
        voice: AudioVoiceId,
        at_frame: u64,
    },
    StopAndDespawn {
        voice: AudioVoiceId,
    },
}

impl NativeAudioOperation {
    fn voice(self) -> AudioVoiceId {
        match self {
            Self::SpawnSfx { voice, .. }
            | Self::SpawnMusic { voice, .. }
            | Self::PauseAndSeek { voice, .. }
            | Self::ResumeAndSeek { voice, .. }
            | Self::StopAndDespawn { voice } => voice,
        }
    }
}

const MAX_PENDING_MATERIALIZATION_UPDATES: u16 = 120;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PendingMaterializationDeadline {
    pending_updates: u16,
}

impl PendingMaterializationDeadline {
    fn retry(&mut self, voice: AudioVoiceId, target: &str) -> Result<(), String> {
        self.pending_updates = self.pending_updates.saturating_add(1);
        if self.pending_updates >= MAX_PENDING_MATERIALIZATION_UPDATES {
            return Err(format!(
                "native audio {target} for voice {voice:?} was not available after {} update cycles",
                self.pending_updates
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct PendingNativeSpawn {
    entity: Entity,
    start: AudioDeviceCommand,
    control: Option<(AudioDeviceCommand, NativeAudioOperation)>,
    deadline: PendingMaterializationDeadline,
}

#[derive(Debug)]
struct PendingNativeFailure {
    entity: Entity,
    error: String,
}

impl PendingNativeSpawn {
    fn new(entity: Entity, start: AudioDeviceCommand) -> Self {
        Self {
            entity,
            start,
            control: None,
            deadline: PendingMaterializationDeadline::default(),
        }
    }

    fn set_control(&mut self, command: AudioDeviceCommand, operation: NativeAudioOperation) {
        self.control = Some((command, operation));
    }
}

/// Pure validation and translation from platform-neutral device commands to
/// native Bevy operations. It owns only device-voice existence; play/pause
/// policy remains in `puzzle_audio::AudioRuntime`.
#[derive(Clone, Default)]
pub(crate) struct NativeAudioPlanReducer {
    voices: AudioDeviceVoiceRegistry,
    pending_starts: BTreeSet<AudioVoiceId>,
}

impl NativeAudioPlanReducer {
    pub(crate) fn reserve_start(
        &mut self,
        command: AudioDeviceCommand,
    ) -> Result<NativeAudioOperation, AudioDeviceStateError> {
        self.voices.validate(command)?;
        let operation = match command {
            AudioDeviceCommand::StartSfx { voice, asset, gain } => {
                NativeAudioOperation::SpawnSfx { voice, asset, gain }
            }
            AudioDeviceCommand::StartMusic {
                voice,
                asset,
                start_frame,
                gain,
            } => NativeAudioOperation::SpawnMusic {
                voice,
                asset,
                start_frame,
                gain,
            },
            AudioDeviceCommand::PauseVoice { voice, .. }
            | AudioDeviceCommand::ResumeVoice { voice, .. }
            | AudioDeviceCommand::StopVoice { voice } => {
                return Err(AudioDeviceStateError::MissingVoice(voice));
            }
        };
        let voice = operation.voice();
        if !self.pending_starts.insert(voice) {
            return Err(AudioDeviceStateError::DuplicateVoice(voice));
        }
        Ok(operation)
    }

    pub(crate) fn plan_control(
        &self,
        command: AudioDeviceCommand,
    ) -> Result<NativeAudioOperation, AudioDeviceStateError> {
        let voice = command_voice(command);
        if !self.pending_starts.contains(&voice) {
            self.voices.validate(command)?;
        } else if matches!(
            command,
            AudioDeviceCommand::StartSfx { .. } | AudioDeviceCommand::StartMusic { .. }
        ) {
            return Err(AudioDeviceStateError::DuplicateVoice(voice));
        }
        Ok(match command {
            AudioDeviceCommand::PauseVoice { voice, at_frame } => {
                NativeAudioOperation::PauseAndSeek { voice, at_frame }
            }
            AudioDeviceCommand::ResumeVoice { voice, at_frame } => {
                NativeAudioOperation::ResumeAndSeek { voice, at_frame }
            }
            AudioDeviceCommand::StopVoice { voice } => {
                NativeAudioOperation::StopAndDespawn { voice }
            }
            AudioDeviceCommand::StartSfx { voice, .. }
            | AudioDeviceCommand::StartMusic { voice, .. } => {
                return Err(AudioDeviceStateError::DuplicateVoice(voice));
            }
        })
    }

    pub(crate) fn materialize_start(
        &mut self,
        command: AudioDeviceCommand,
    ) -> Result<(), AudioDeviceStateError> {
        let voice = command_voice(command);
        if !self.pending_starts.remove(&voice) {
            return Err(AudioDeviceStateError::MissingVoice(voice));
        }
        self.voices.commit(command)
    }

    pub(crate) fn commit_control(
        &mut self,
        command: AudioDeviceCommand,
    ) -> Result<(), AudioDeviceStateError> {
        self.voices.commit(command)
    }

    fn cancel_pending(&mut self, voice: AudioVoiceId) -> bool {
        self.pending_starts.remove(&voice)
    }

    fn is_pending(&self, voice: AudioVoiceId) -> bool {
        self.pending_starts.contains(&voice)
    }

    fn voice_ended(&mut self, voice: AudioVoiceId) {
        self.pending_starts.remove(&voice);
        self.voices.voice_ended(voice);
    }
}

/// Bevy asset wrapper for a fully resolved, finite PuzzleStudio sound effect.
///
/// Construction validates the device-facing invariants once so `Decodable`,
/// whose API cannot return an error, never has to guess a sample rate or repair
/// invalid samples.
#[derive(Asset, Clone, Debug, TypePath)]
pub(crate) struct NativeSfxAsset {
    sample_rate: SampleRate,
    samples: Arc<[f32]>,
}

impl NativeSfxAsset {
    pub(crate) fn try_from_generated(clip: GeneratedSfxClip) -> Result<Self, String> {
        let sample_rate = SampleRate::new(clip.sample_rate)
            .ok_or_else(|| "generated SFX sample rate must be greater than zero".to_string())?;
        if let Some((index, sample)) = clip
            .samples
            .iter()
            .copied()
            .enumerate()
            .find(|(_, sample)| !sample.is_finite())
        {
            return Err(format!(
                "generated SFX sample {index} must be finite, got {sample}"
            ));
        }
        Ok(Self {
            sample_rate,
            samples: clip.samples,
        })
    }

    #[cfg(test)]
    fn decoder_at_start(&self) -> NativeSfxDecoder {
        self.decoder()
    }
}

impl Decodable for NativeSfxAsset {
    type Decoder = NativeSfxDecoder;

    fn decoder(&self) -> Self::Decoder {
        NativeSfxDecoder {
            sample_rate: self.sample_rate,
            samples: Arc::clone(&self.samples),
            sample_index: 0,
        }
    }
}

/// Seekable mono decoder over immutable generated SFX samples.
pub(crate) struct NativeSfxDecoder {
    sample_rate: SampleRate,
    samples: Arc<[f32]>,
    sample_index: usize,
}

impl Iterator for NativeSfxDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.samples.get(self.sample_index).copied();
        if sample.is_some() {
            self.sample_index += 1;
        }
        sample
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.samples.len().saturating_sub(self.sample_index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for NativeSfxDecoder {}

impl Source for NativeSfxDecoder {
    fn current_span_len(&self) -> Option<usize> {
        if self.sample_index == self.samples.len() {
            Some(0)
        } else {
            // The format is constant for the complete mono clip. Rodio's span
            // contract asks for the total span size, not the remaining count.
            Some(self.samples.len())
        }
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(1).expect("one mono channel is non-zero")
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(duration_for_frames(
            self.samples.len() as u64,
            self.sample_rate,
        ))
    }

    fn try_seek(&mut self, position: Duration) -> Result<(), bevy::audio::SeekError> {
        self.sample_index =
            frame_for_duration(position, self.sample_rate).min(self.samples.len() as u64) as usize;
        Ok(())
    }
}

#[derive(Asset, Clone, Debug, TypePath)]
pub(crate) struct NativeMusicAsset {
    sample_rate: SampleRate,
    track: GeneratedMusicTrack,
}

impl NativeMusicAsset {
    pub(crate) fn try_from_generated(track: GeneratedMusicTrack) -> Result<Self, String> {
        let sample_rate = SampleRate::new(track.sample_rate())
            .ok_or_else(|| "generated music sample rate must be greater than zero".to_string())?;
        if track.channels() != 2 {
            return Err(format!(
                "generated music must have exactly two channels, got {}",
                track.channels()
            ));
        }
        if track.loop_frames() == 0 {
            return Err("generated music loop must contain at least one frame".to_string());
        }
        if track.score().timbres.is_none() {
            return Err("generated music is missing its resolved timbre contract".to_string());
        }
        for channel in 0..track.channels() {
            let sample = track.sample_at(0, channel);
            if !sample.is_finite() {
                return Err(format!(
                    "generated music channel {channel} starts with non-finite sample {sample}"
                ));
            }
        }
        Ok(Self { sample_rate, track })
    }

    fn decoder_at(&self, frame: u64) -> NativeMusicDecoder {
        const BLOCK_FRAMES: usize = 1_024;
        let block_samples = BLOCK_FRAMES * usize::from(self.track.channels());
        NativeMusicDecoder {
            sample_rate: self.sample_rate,
            track: self.track.clone(),
            sample_index: frame.saturating_mul(u64::from(self.track.channels())),
            block: vec![0.0; block_samples].into_boxed_slice(),
            scratch: vec![0.0; block_samples].into_boxed_slice(),
            block_index: block_samples,
        }
    }
}

impl Decodable for NativeMusicAsset {
    type Decoder = NativeMusicDecoder;

    fn decoder(&self) -> Self::Decoder {
        self.decoder_at(0)
    }
}

/// Intrinsically looping stereo source. It is submitted with
/// `PlaybackSettings::ONCE`; using Bevy's LOOP mode would buffer the entire
/// decoded track in Rodio before repeating it.
pub(crate) struct NativeMusicDecoder {
    sample_rate: SampleRate,
    track: GeneratedMusicTrack,
    sample_index: u64,
    block: Box<[f32]>,
    scratch: Box<[f64]>,
    block_index: usize,
}

impl Iterator for NativeMusicDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.block_index == self.block.len() {
            let frame = self.sample_index / u64::from(self.track.channels());
            self.track
                .render_interleaved(frame, &mut self.block, &mut self.scratch)
                .expect("validated music and fixed aligned buffers satisfy the render contract");
            self.block_index = 0;
        }
        let sample = self.block[self.block_index];
        self.block_index += 1;
        self.sample_index = self.sample_index.wrapping_add(1);
        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }
}

impl Source for NativeMusicDecoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(self.track.channels())
            .expect("validated generated music channel count is non-zero")
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }

    fn try_seek(&mut self, position: Duration) -> Result<(), bevy::audio::SeekError> {
        let frame = frame_for_duration(position, self.sample_rate) % self.track.loop_frames();
        self.sample_index = frame.saturating_mul(u64::from(self.track.channels()));
        self.block_index = self.block.len();
        Ok(())
    }
}

pub(crate) fn install_native_audio_backend(app: &mut App) {
    app.add_audio_source::<NativeSfxAsset>()
        .add_audio_source::<NativeMusicAsset>()
        .init_resource::<NativeAudioBackend>();
}

#[derive(Component)]
pub(crate) struct NativeAudioVoice;

#[derive(Resource, Default)]
pub(crate) struct NativeAudioBackend {
    plan: NativeAudioPlanReducer,
    entities: BTreeMap<AudioVoiceId, Entity>,
    sfx_assets: BTreeMap<SfxAssetId, Handle<NativeSfxAsset>>,
    music_assets: BTreeMap<MusicAssetId, Handle<NativeMusicAsset>>,
    pending_spawns: BTreeMap<AudioVoiceId, PendingNativeSpawn>,
}

pub(crate) fn process_native_audio(
    mut commands: Commands,
    time: Res<Time>,
    mut host: NonSendMut<super::PuzzleBevyPlayerHost>,
    mut backend: ResMut<NativeAudioBackend>,
    mut sfx_assets: ResMut<Assets<NativeSfxAsset>>,
    mut music_assets: ResMut<Assets<NativeMusicAsset>>,
    sinks: Query<&AudioSink>,
    live_voices: Query<&NativeAudioVoice>,
) {
    for diagnostic in host.take_audio_diagnostics() {
        warn!("audio output diagnostic: {diagnostic:?}");
    }

    let ended = backend
        .entities
        .iter()
        .filter_map(|(voice, entity)| live_voices.get(*entity).is_err().then_some(*voice))
        .collect::<Vec<_>>();
    for voice in ended {
        backend.entities.remove(&voice);
        backend.pending_spawns.remove(&voice);
        backend.plan.voice_ended(voice);
        host.audio_voice_ended(voice);
    }

    let pending_spawn_voices = backend.pending_spawns.keys().copied().collect::<Vec<_>>();
    for voice in pending_spawn_voices {
        let Some(pending) = backend.pending_spawns.get(&voice).copied() else {
            continue;
        };
        if sinks.get(pending.entity).is_ok() {
            let pending = backend
                .pending_spawns
                .remove(&voice)
                .expect("the pending voice was just observed");
            if let Err(error) = backend.plan.materialize_start(pending.start) {
                commands.entity(pending.entity).despawn();
                host.audio_voice_failed(
                    voice,
                    format!("native audio start commit failed: {error:?}"),
                    time.elapsed_secs_f64(),
                );
                continue;
            }
            backend.entities.insert(voice, pending.entity);
            if let Some((command, operation)) = pending.control {
                match apply_materialized_control(operation, &mut commands, &mut backend, &sinks) {
                    Ok(()) => {
                        if let Err(error) = backend.plan.commit_control(command) {
                            fail_native_voice(voice, &mut commands, &mut backend);
                            host.audio_voice_failed(
                                voice,
                                format!("native audio control commit failed: {error:?}"),
                                time.elapsed_secs_f64(),
                            );
                        }
                    }
                    Err(error) => {
                        fail_native_voice(voice, &mut commands, &mut backend);
                        host.audio_voice_failed(voice, error, time.elapsed_secs_f64());
                    }
                }
            }
            continue;
        }
        if let Err(failure) = advance_pending_materialization(&mut backend, voice) {
            commands.entity(failure.entity).despawn();
            host.audio_voice_failed(voice, failure.error, time.elapsed_secs_f64());
        }
    }

    for command in host.take_audio_commands() {
        if let Err(error) = apply_device_command(
            command,
            &mut commands,
            &mut backend,
            &mut sfx_assets,
            &mut music_assets,
            &sinks,
            &host,
        ) {
            let voice = command_voice(command);
            fail_native_voice(voice, &mut commands, &mut backend);
            host.audio_voice_failed(voice, error, time.elapsed_secs_f64());
        }
    }
}

fn advance_pending_materialization(
    backend: &mut NativeAudioBackend,
    voice: AudioVoiceId,
) -> Result<(), PendingNativeFailure> {
    let result = backend
        .pending_spawns
        .get_mut(&voice)
        .expect("only an explicitly pending voice may advance")
        .deadline
        .retry(voice, "output sink");
    if let Err(error) = result {
        let pending = backend
            .pending_spawns
            .remove(&voice)
            .expect("the expired voice was explicitly pending");
        backend.plan.cancel_pending(voice);
        Err(PendingNativeFailure {
            entity: pending.entity,
            error,
        })
    } else {
        Ok(())
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

fn fail_native_voice(
    voice: AudioVoiceId,
    commands: &mut Commands,
    backend: &mut NativeAudioBackend,
) {
    backend.plan.voice_ended(voice);
    if let Some(pending) = backend.pending_spawns.remove(&voice) {
        commands.entity(pending.entity).despawn();
    }
    if let Some(entity) = backend.entities.remove(&voice) {
        commands.entity(entity).despawn();
    }
}

fn apply_device_command(
    command: AudioDeviceCommand,
    commands: &mut Commands,
    backend: &mut NativeAudioBackend,
    sfx_assets: &mut Assets<NativeSfxAsset>,
    music_assets: &mut Assets<NativeMusicAsset>,
    sinks: &Query<&AudioSink>,
    host: &super::PuzzleBevyPlayerHost,
) -> Result<(), String> {
    match command {
        AudioDeviceCommand::StartSfx { .. } | AudioDeviceCommand::StartMusic { .. } => {
            let operation = backend
                .plan
                .reserve_start(command)
                .map_err(|error| format!("native audio start plan failed: {error:?}"))?;
            if let Err(error) = spawn_pending_voice(
                command,
                operation,
                commands,
                backend,
                sfx_assets,
                music_assets,
                host,
            ) {
                backend.plan.cancel_pending(command_voice(command));
                return Err(error);
            }
            Ok(())
        }
        AudioDeviceCommand::PauseVoice { .. }
        | AudioDeviceCommand::ResumeVoice { .. }
        | AudioDeviceCommand::StopVoice { .. } => {
            let operation = backend
                .plan
                .plan_control(command)
                .map_err(|error| format!("native audio control plan failed: {error:?}"))?;
            let voice = operation.voice();
            if backend.plan.is_pending(voice) {
                if matches!(operation, NativeAudioOperation::StopAndDespawn { .. }) {
                    let pending = backend
                        .pending_spawns
                        .remove(&voice)
                        .ok_or_else(|| format!("native audio has no pending voice {voice:?}"))?;
                    backend.plan.cancel_pending(voice);
                    commands.entity(pending.entity).despawn();
                } else {
                    backend
                        .pending_spawns
                        .get_mut(&voice)
                        .ok_or_else(|| format!("native audio has no pending voice {voice:?}"))?
                        .set_control(command, operation);
                }
                return Ok(());
            }
            apply_materialized_control(operation, commands, backend, sinks)?;
            backend
                .plan
                .commit_control(command)
                .map_err(|error| format!("native audio control commit failed: {error:?}"))
        }
    }
}

fn spawn_pending_voice(
    command: AudioDeviceCommand,
    operation: NativeAudioOperation,
    commands: &mut Commands,
    backend: &mut NativeAudioBackend,
    sfx_assets: &mut Assets<NativeSfxAsset>,
    music_assets: &mut Assets<NativeMusicAsset>,
    host: &super::PuzzleBevyPlayerHost,
) -> Result<(), String> {
    match operation {
        NativeAudioOperation::SpawnSfx { voice, asset, gain } => {
            let handle = if let Some(handle) = backend.sfx_assets.get(&asset) {
                handle.clone()
            } else {
                let clip = host
                    .audio_catalog()
                    .sfx(asset)
                    .cloned()
                    .ok_or_else(|| format!("audio catalog has no SFX asset {asset:?}"))?;
                let handle = sfx_assets.add(NativeSfxAsset::try_from_generated(clip)?);
                backend.sfx_assets.insert(asset, handle.clone());
                handle
            };
            let entity = commands
                .spawn((
                    AudioPlayer(handle),
                    PlaybackSettings {
                        volume: Volume::Linear(gain),
                        ..PlaybackSettings::DESPAWN
                    },
                    NativeAudioVoice,
                ))
                .id();
            backend
                .pending_spawns
                .insert(voice, PendingNativeSpawn::new(entity, command));
            Ok(())
        }
        NativeAudioOperation::SpawnMusic {
            voice,
            asset,
            start_frame,
            gain,
        } => {
            let handle = if let Some(handle) = backend.music_assets.get(&asset) {
                handle.clone()
            } else {
                let track = host
                    .audio_catalog()
                    .music(asset)
                    .cloned()
                    .ok_or_else(|| format!("audio catalog has no music asset {asset:?}"))?;
                let handle = music_assets.add(NativeMusicAsset::try_from_generated(track)?);
                backend.music_assets.insert(asset, handle.clone());
                handle
            };
            let entity = commands
                .spawn((
                    AudioPlayer(handle),
                    PlaybackSettings {
                        volume: Volume::Linear(gain),
                        start_position: Some(duration_for_frames(
                            start_frame,
                            SampleRate::new(puzzle_audio::CANONICAL_AUDIO_SAMPLE_RATE)
                                .expect("canonical sample rate is non-zero"),
                        )),
                        // The decoder loops intrinsically. Bevy LOOP would
                        // buffer the complete decoded track.
                        ..PlaybackSettings::ONCE
                    },
                    NativeAudioVoice,
                ))
                .id();
            backend
                .pending_spawns
                .insert(voice, PendingNativeSpawn::new(entity, command));
            Ok(())
        }
        NativeAudioOperation::PauseAndSeek { .. }
        | NativeAudioOperation::ResumeAndSeek { .. }
        | NativeAudioOperation::StopAndDespawn { .. } => {
            Err("native audio spawn path received a control operation".to_string())
        }
    }
}

fn apply_materialized_control(
    operation: NativeAudioOperation,
    commands: &mut Commands,
    backend: &mut NativeAudioBackend,
    sinks: &Query<&AudioSink>,
) -> Result<(), String> {
    let (voice, at_frame, resume) = match operation {
        NativeAudioOperation::PauseAndSeek { voice, at_frame } => (voice, at_frame, false),
        NativeAudioOperation::ResumeAndSeek { voice, at_frame } => (voice, at_frame, true),
        NativeAudioOperation::StopAndDespawn { voice } => {
            if let Some(entity) = backend.entities.remove(&voice) {
                commands.entity(entity).despawn();
            }
            return Ok(());
        }
        NativeAudioOperation::SpawnSfx { .. } | NativeAudioOperation::SpawnMusic { .. } => {
            return Err("native audio control queue received a spawn operation".to_string());
        }
    };
    let Some(entity) = backend.entities.get(&voice).copied() else {
        return Err(format!("native audio has no entity for voice {voice:?}"));
    };
    let sink = sinks
        .get(entity)
        .map_err(|_| format!("native audio materialized voice {voice:?} has no output sink"))?;
    sink.try_seek(Duration::from_secs_f64(
        at_frame as f64 / f64::from(puzzle_audio::CANONICAL_AUDIO_SAMPLE_RATE),
    ))
    .map_err(|error| format!("native audio seek failed for voice {voice:?}: {error}"))?;
    if resume {
        sink.play();
    } else {
        sink.pause();
    }
    Ok(())
}

fn duration_for_frames(frames: u64, sample_rate: SampleRate) -> Duration {
    let sample_rate = u64::from(sample_rate.get());
    let seconds = frames / sample_rate;
    let remaining_frames = frames % sample_rate;
    let nanos = u128::from(remaining_frames) * 1_000_000_000_u128 / u128::from(sample_rate);
    Duration::new(seconds, nanos as u32)
}

fn frame_for_duration(duration: Duration, sample_rate: SampleRate) -> u64 {
    let frames = duration.as_nanos() * u128::from(sample_rate.get()) / 1_000_000_000_u128;
    frames.min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(sample_rate: u32, samples: &[f32]) -> NativeSfxAsset {
        NativeSfxAsset::try_from_generated(GeneratedSfxClip {
            sample_rate,
            samples: Arc::from(samples),
        })
        .expect("test clip should be valid")
    }

    #[test]
    fn finite_decoder_reports_exact_samples_and_duration() {
        let asset = clip(4, &[0.25, -0.5, 0.75]);
        let mut decoder = asset.decoder_at_start();

        assert_eq!(decoder.channels().get(), 1);
        assert_eq!(decoder.sample_rate().get(), 4);
        assert_eq!(decoder.total_duration(), Some(Duration::from_millis(750)));
        assert_eq!(decoder.current_span_len(), Some(3));
        assert_eq!(decoder.by_ref().collect::<Vec<_>>(), [0.25, -0.5, 0.75]);
        assert_eq!(decoder.current_span_len(), Some(0));
        assert_eq!(decoder.next(), None);
    }

    #[test]
    fn seek_matches_consuming_the_same_number_of_frames_and_saturates() {
        let asset = clip(4, &[0.0, 1.0, 2.0, 3.0]);
        let mut sought = asset.decoder_at_start();
        sought
            .try_seek(Duration::from_millis(500))
            .expect("generated PCM is seekable");
        assert_eq!(sought.collect::<Vec<_>>(), [2.0, 3.0]);

        let mut beyond_end = asset.decoder_at_start();
        beyond_end
            .try_seek(Duration::from_secs(2))
            .expect("seek beyond a known duration saturates");
        assert_eq!(beyond_end.current_span_len(), Some(0));
        assert_eq!(beyond_end.next(), None);
    }

    #[test]
    fn invalid_device_samples_fail_at_asset_construction() {
        let zero_rate = NativeSfxAsset::try_from_generated(GeneratedSfxClip {
            sample_rate: 0,
            samples: Arc::from([0.0]),
        })
        .expect_err("zero sample rate must not reach Decodable");
        assert!(zero_rate.contains("sample rate"));

        let non_finite = NativeSfxAsset::try_from_generated(GeneratedSfxClip {
            sample_rate: 44_100,
            samples: Arc::from([f32::NAN]),
        })
        .expect_err("non-finite PCM must not reach the device");
        assert!(non_finite.contains("sample 0"));
    }

    #[test]
    fn native_music_decoder_refills_blocks_without_changing_canonical_samples() {
        let track = puzzle_audio::generate_music(&puzzle_audio::MusicRecipe {
            seed: "native-block".to_string(),
            height: 0.5,
            bars: 8,
            bpm: 120,
            volume: 0.4,
        })
        .expect("test music should generate");
        let asset =
            NativeMusicAsset::try_from_generated(track.clone()).expect("test music is valid");
        let start_frame = track.loop_frames() - 300;
        let mut decoder = asset.decoder_at(start_frame);

        for sample_offset in 0..5_000_u64 {
            let frame = start_frame + sample_offset / u64::from(track.channels());
            let channel = (sample_offset % u64::from(track.channels())) as u16;
            assert_eq!(
                decoder.next().expect("music decoder is infinite").to_bits(),
                track.sample_at(frame, channel).to_bits()
            );
        }

        decoder
            .try_seek(Duration::from_millis(250))
            .expect("generated music is seekable");
        let sought_frame =
            frame_for_duration(Duration::from_millis(250), asset.sample_rate) % track.loop_frames();
        assert_eq!(
            decoder.next().expect("music decoder is infinite").to_bits(),
            track.sample_at(sought_frame, 0).to_bits()
        );
    }

    #[test]
    fn device_commands_reduce_to_ordered_bevy_operations() {
        let voice = AudioVoiceId(7);
        let mut reducer = NativeAudioPlanReducer::default();

        let start = AudioDeviceCommand::StartMusic {
            voice,
            asset: MusicAssetId(2),
            start_frame: 480,
            gain: 0.75,
        };
        assert_eq!(
            reducer.reserve_start(start),
            Ok(NativeAudioOperation::SpawnMusic {
                voice,
                asset: MusicAssetId(2),
                start_frame: 480,
                gain: 0.75,
            })
        );
        reducer
            .materialize_start(start)
            .expect("observed sink commits the start");

        for (command, expected) in [
            (
                AudioDeviceCommand::PauseVoice {
                    voice,
                    at_frame: 960,
                },
                NativeAudioOperation::PauseAndSeek {
                    voice,
                    at_frame: 960,
                },
            ),
            (
                AudioDeviceCommand::ResumeVoice {
                    voice,
                    at_frame: 1_440,
                },
                NativeAudioOperation::ResumeAndSeek {
                    voice,
                    at_frame: 1_440,
                },
            ),
            (
                AudioDeviceCommand::StopVoice { voice },
                NativeAudioOperation::StopAndDespawn { voice },
            ),
        ] {
            assert_eq!(reducer.plan_control(command), Ok(expected));
            reducer
                .commit_control(command)
                .expect("materialized control commits");
        }
    }

    #[test]
    fn reducer_rejects_duplicate_and_unknown_device_voices() {
        let voice = AudioVoiceId(4);
        let mut reducer = NativeAudioPlanReducer::default();
        let first = AudioDeviceCommand::StartSfx {
            voice,
            asset: SfxAssetId(1),
            gain: 1.0,
        };
        reducer
            .reserve_start(first)
            .expect("first voice allocation should validate");
        reducer
            .materialize_start(first)
            .expect("first allocation materialized");

        assert_eq!(
            reducer.reserve_start(AudioDeviceCommand::StartSfx {
                voice,
                asset: SfxAssetId(2),
                gain: 1.0,
            }),
            Err(AudioDeviceStateError::DuplicateVoice(voice))
        );
        assert_eq!(
            reducer.plan_control(AudioDeviceCommand::PauseVoice {
                voice: AudioVoiceId(99),
                at_frame: 0,
            }),
            Err(AudioDeviceStateError::MissingVoice(AudioVoiceId(99)))
        );
    }

    #[test]
    fn reducer_rejects_invalid_gain_before_allocating_voice() {
        let voice = AudioVoiceId(8);
        let mut reducer = NativeAudioPlanReducer::default();

        assert!(matches!(
            reducer.reserve_start(AudioDeviceCommand::StartSfx {
                voice,
                asset: SfxAssetId(0),
                gain: f32::NAN,
            }),
            Err(AudioDeviceStateError::NonFiniteGain {
                voice: AudioVoiceId(8),
                ..
            })
        ));
        let valid = AudioDeviceCommand::StartSfx {
            voice,
            asset: SfxAssetId(0),
            gain: 1.0,
        };
        reducer
            .reserve_start(valid)
            .expect("invalid command must not reserve its voice");
    }

    #[test]
    fn start_is_reserved_but_not_committed_until_sink_materializes() {
        let voice = AudioVoiceId(12);
        let mut reducer = NativeAudioPlanReducer::default();
        let command = AudioDeviceCommand::StartSfx {
            voice,
            asset: SfxAssetId(1),
            gain: 1.0,
        };

        reducer
            .reserve_start(command)
            .expect("first attempt reserves the pending device operation");
        assert!(reducer.is_pending(voice));
        assert_eq!(reducer.voices.voice(voice), None);
        assert_eq!(
            reducer.reserve_start(command),
            Err(AudioDeviceStateError::DuplicateVoice(voice))
        );

        reducer
            .materialize_start(command)
            .expect("sink observation commits the device voice");
        assert!(!reducer.is_pending(voice));
        assert_eq!(
            reducer.voices.voice(voice),
            Some(puzzle_audio::AudioDeviceVoiceKind::Sfx(SfxAssetId(1)))
        );
    }

    #[test]
    fn pending_controls_coalesce_and_stop_cancels_without_committing_a_voice() {
        let voice = AudioVoiceId(21);
        let start = AudioDeviceCommand::StartMusic {
            voice,
            asset: MusicAssetId(0),
            start_frame: 0,
            gain: 1.0,
        };
        let mut reducer = NativeAudioPlanReducer::default();
        reducer.reserve_start(start).expect("start reserves");
        let mut pending = PendingNativeSpawn::new(Entity::PLACEHOLDER, start);

        let pause = AudioDeviceCommand::PauseVoice {
            voice,
            at_frame: 100,
        };
        let pause_operation = reducer
            .plan_control(pause)
            .expect("pending voice accepts control");
        pending.set_control(pause, pause_operation);
        let resume = AudioDeviceCommand::ResumeVoice {
            voice,
            at_frame: 240,
        };
        let resume_operation = reducer
            .plan_control(resume)
            .expect("pending voice accepts replacement control");
        pending.set_control(resume, resume_operation);
        assert_eq!(pending.control, Some((resume, resume_operation)));

        assert_eq!(
            reducer.plan_control(AudioDeviceCommand::StopVoice { voice }),
            Ok(NativeAudioOperation::StopAndDespawn { voice })
        );
        assert!(reducer.cancel_pending(voice));
        assert_eq!(reducer.voices.voice(voice), None);
    }

    #[test]
    fn materialization_timeout_removes_only_the_expired_pending_voice() {
        let expired_voice = AudioVoiceId(22);
        let independent_voice = AudioVoiceId(23);
        let mut backend = NativeAudioBackend::default();
        for voice in [expired_voice, independent_voice] {
            let start = AudioDeviceCommand::StartSfx {
                voice,
                asset: SfxAssetId(0),
                gain: 1.0,
            };
            backend
                .plan
                .reserve_start(start)
                .expect("independent voice reserves");
            backend
                .pending_spawns
                .insert(voice, PendingNativeSpawn::new(Entity::PLACEHOLDER, start));
        }

        for _ in 1..MAX_PENDING_MATERIALIZATION_UPDATES {
            advance_pending_materialization(&mut backend, expired_voice)
                .expect("deadline has not elapsed");
        }
        let failure = advance_pending_materialization(&mut backend, expired_voice)
            .expect_err("missing native output must fail visibly");
        assert!(failure.error.contains("output sink"));
        assert!(failure.error.contains("AudioVoiceId(22)"));
        assert!(failure.error.contains("120 update cycles"));
        assert!(!backend.pending_spawns.contains_key(&expired_voice));
        assert!(!backend.plan.is_pending(expired_voice));
        assert!(backend.pending_spawns.contains_key(&independent_voice));
        assert!(backend.plan.is_pending(independent_voice));
        assert_eq!(backend.plan.voices.voice(independent_voice), None);
    }
}
