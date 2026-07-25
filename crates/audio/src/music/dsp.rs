use super::{
    GeneratedMusicTrack, MusicNote, MusicRenderError, MusicRole, MusicScoreEvent, MusicTimbreRef,
    MusicTimbres, NoiseRole, PlaybackTone, SpectralTimbreSignal, TransientSignal,
};

const TAU: f64 = std::f64::consts::TAU;
const SILENCE: f64 = 0.0001;
const RENDER_INDEX_BUCKET_FRAMES: u64 = 128;

pub(super) fn sample_at(track: &GeneratedMusicTrack, frame: u64, channel: u16) -> f32 {
    if track.loop_frames == 0 || channel >= track.channels {
        return 0.0;
    }
    let frame = frame % track.loop_frames;
    let score = &track.score;
    let Some(timbres) = score.timbres.as_ref() else {
        return 0.0;
    };
    let mut output = 0.0;
    for &event_index in track.render_index.candidate_indices_at(frame) {
        let event = track.render_index.prepared(event_index);
        let score_event = &score.events[event.event_index as usize];
        let local = match frame.checked_sub(event.start) {
            Some(local) if local < event.duration => local,
            _ => continue,
        };
        event.add_sample(
            score_event,
            timbres,
            &score.mix.playback_tone,
            track.sample_rate,
            local,
            channel,
            &mut output,
        );
    }
    output.clamp(-1.0, 1.0) as f32
}

pub(super) fn render_interleaved(
    track: &GeneratedMusicTrack,
    start_frame: u64,
    output: &mut [f32],
    scratch: &mut [f64],
) -> Result<(), MusicRenderError> {
    render_interleaved_inner(track, start_frame, output, scratch).map(|_| ())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RenderStats {
    event_segment_visits: usize,
    rendered_event_frames: usize,
}

fn render_interleaved_inner(
    track: &GeneratedMusicTrack,
    start_frame: u64,
    output: &mut [f32],
    scratch: &mut [f64],
) -> Result<RenderStats, MusicRenderError> {
    let channels = usize::from(track.channels);
    if channels == 0 {
        return Err(MusicRenderError::ZeroChannels);
    }
    if output.len() % channels != 0 {
        return Err(MusicRenderError::OutputNotFrameAligned {
            samples: output.len(),
            channels: track.channels,
        });
    }
    if scratch.len() < output.len() {
        return Err(MusicRenderError::ScratchTooSmall {
            required: output.len(),
            available: scratch.len(),
        });
    }

    output.fill(0.0);
    let mix = &mut scratch[..output.len()];
    mix.fill(0.0);
    if output.is_empty() || track.loop_frames == 0 {
        return Ok(RenderStats::default());
    }
    let Some(timbres) = track.score.timbres.as_ref() else {
        return Ok(RenderStats::default());
    };

    let score = &track.score;
    let total_frames = output.len() / channels;
    let mut rendered_frames = 0_usize;
    let mut loop_frame = start_frame % track.loop_frames;
    let mut stats = RenderStats::default();

    while rendered_frames < total_frames {
        let remaining_frames = u64::try_from(total_frames - rendered_frames)
            .expect("an addressable output buffer has a u64 frame count");
        let segment_frames =
            usize::try_from((track.loop_frames - loop_frame).min(remaining_frames))
                .expect("segment length is bounded by an existing output slice");
        let segment_end = loop_frame
            + u64::try_from(segment_frames)
                .expect("an addressable output segment has a u64 frame count");
        let segment_start = loop_frame;
        let mut bucket_frame = loop_frame;

        while bucket_frame < segment_end {
            let bucket_end = track.render_index.bucket_end(bucket_frame).min(segment_end);
            for &event_index in track.render_index.candidate_indices_at(bucket_frame) {
                stats.event_segment_visits += 1;
                let event = track.render_index.prepared(event_index);
                let score_event = &score.events[event.event_index as usize];
                let overlap_start = event.start.max(bucket_frame);
                let overlap_end = event.end().min(bucket_end);
                if overlap_start >= overlap_end {
                    continue;
                }

                stats.rendered_event_frames += usize::try_from(overlap_end - overlap_start)
                    .expect("event overlap is bounded by an existing output slice");

                for frame in overlap_start..overlap_end {
                    let local = frame - event.start;
                    let output_frame = rendered_frames
                        + usize::try_from(frame - segment_start)
                            .expect("event overlap is bounded by an existing output slice");
                    for channel in 0..channels {
                        event.add_sample(
                            score_event,
                            timbres,
                            &score.mix.playback_tone,
                            track.sample_rate,
                            local,
                            channel as u16,
                            &mut mix[output_frame * channels + channel],
                        );
                    }
                }
            }
            bucket_frame = bucket_end;
        }

        rendered_frames += segment_frames;
        loop_frame = if segment_end == track.loop_frames {
            0
        } else {
            segment_end
        };
    }

    for (output, mixed) in output.iter_mut().zip(mix.iter().copied()) {
        *output = mixed.clamp(-1.0, 1.0) as f32;
    }
    Ok(stats)
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct MusicRenderIndex {
    events: Vec<PreparedEvent>,
    bucket_offsets: Vec<usize>,
    candidate_event_indices: Vec<u32>,
}

impl MusicRenderIndex {
    pub(super) fn new(
        sample_rate: u32,
        loop_frames: u64,
        score: &super::MusicScore,
    ) -> Result<Self, String> {
        let frames_per_step = f64::from(sample_rate) * 60.0 / f64::from(score.transport.bpm)
            * score.transport.step_duration_beats;
        if !frames_per_step.is_finite() || frames_per_step <= 0.0 {
            return Err("music render index requires a finite positive step duration".to_string());
        }

        let mut events = Vec::new();
        events
            .try_reserve_exact(score.events.len())
            .map_err(|_| "music render index event storage is too large".to_string())?;
        for (event_index, event) in score.events.iter().enumerate() {
            let event_index = u32::try_from(event_index)
                .map_err(|_| "music render index supports at most 4294967296 events".to_string())?;
            events.push(PreparedEvent::new(
                event_index,
                event,
                frames_per_step,
                score,
            ));
        }

        let bucket_count = if loop_frames == 0 {
            0
        } else {
            usize::try_from((loop_frames - 1) / RENDER_INDEX_BUCKET_FRAMES + 1)
                .map_err(|_| "music render index loop is not addressable".to_string())?
        };
        let mut bucket_counts = Vec::new();
        bucket_counts
            .try_reserve_exact(bucket_count)
            .map_err(|_| "music render index bucket storage is too large".to_string())?;
        bucket_counts.resize(bucket_count, 0_usize);

        for event in &events {
            let Some((first, last)) = event.bucket_span(loop_frames) else {
                continue;
            };
            for count in &mut bucket_counts[first..=last] {
                *count = count
                    .checked_add(1)
                    .ok_or_else(|| "music render index candidate count overflowed".to_string())?;
            }
        }

        let mut bucket_offsets = Vec::new();
        bucket_offsets
            .try_reserve_exact(bucket_count.saturating_add(1))
            .map_err(|_| "music render index offset storage is too large".to_string())?;
        bucket_offsets.push(0_usize);
        for count in bucket_counts {
            let next = bucket_offsets
                .last()
                .copied()
                .expect("the initial render bucket offset exists")
                .checked_add(count)
                .ok_or_else(|| "music render index candidate storage overflowed".to_string())?;
            bucket_offsets.push(next);
        }

        let candidate_count = bucket_offsets.last().copied().unwrap_or(0);
        let mut candidate_event_indices = Vec::new();
        candidate_event_indices
            .try_reserve_exact(candidate_count)
            .map_err(|_| "music render index candidate storage is too large".to_string())?;
        candidate_event_indices.resize(candidate_count, 0);
        let mut cursors = Vec::new();
        cursors
            .try_reserve_exact(bucket_count)
            .map_err(|_| "music render index cursor storage is too large".to_string())?;
        cursors.extend_from_slice(&bucket_offsets[..bucket_count]);
        for (prepared_index, event) in events.iter().enumerate() {
            let Some((first, last)) = event.bucket_span(loop_frames) else {
                continue;
            };
            let prepared_index =
                u32::try_from(prepared_index).expect("prepared event count was validated above");
            for bucket in first..=last {
                candidate_event_indices[cursors[bucket]] = prepared_index;
                cursors[bucket] += 1;
            }
        }

        Ok(Self {
            events,
            bucket_offsets,
            candidate_event_indices,
        })
    }

    fn candidate_indices_at(&self, frame: u64) -> &[u32] {
        let bucket = usize::try_from(frame / RENDER_INDEX_BUCKET_FRAMES)
            .expect("a frame in an addressable track has an addressable bucket");
        &self.candidate_event_indices[self.bucket_offsets[bucket]..self.bucket_offsets[bucket + 1]]
    }

    fn prepared(&self, index: u32) -> &PreparedEvent {
        &self.events[index as usize]
    }

    fn bucket_end(&self, frame: u64) -> u64 {
        (frame / RENDER_INDEX_BUCKET_FRAMES + 1).saturating_mul(RENDER_INDEX_BUCKET_FRAMES)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PreparedEvent {
    event_index: u32,
    start: u64,
    duration: u64,
    gain: f64,
    pan: f64,
}

impl PreparedEvent {
    fn new(
        event_index: u32,
        event: &MusicScoreEvent,
        frames_per_step: f64,
        score: &super::MusicScore,
    ) -> Self {
        Self {
            event_index,
            start: (f64::from(event.step) * frames_per_step).round() as u64,
            duration: (f64::from(event.duration_steps) * frames_per_step).round() as u64,
            gain: event.velocity
                * score.mix.volume
                * role_gain(event.role, &score.mix.playback_tone),
            pan: pan(event.event_id),
        }
    }

    fn end(&self) -> u64 {
        self.start.saturating_add(self.duration)
    }

    fn bucket_span(&self, loop_frames: u64) -> Option<(usize, usize)> {
        let overlap_end = self.end().min(loop_frames);
        if self.start >= overlap_end {
            return None;
        }
        Some((
            usize::try_from(self.start / RENDER_INDEX_BUCKET_FRAMES)
                .expect("event start bucket is bounded by an addressable loop"),
            usize::try_from((overlap_end - 1) / RENDER_INDEX_BUCKET_FRAMES)
                .expect("event end bucket is bounded by an addressable loop"),
        ))
    }

    fn add_sample(
        &self,
        event: &MusicScoreEvent,
        timbres: &MusicTimbres,
        tone: &PlaybackTone,
        sample_rate: u32,
        local: u64,
        channel: u16,
        output: &mut f64,
    ) {
        let seconds = local as f64 / f64::from(sample_rate);
        let duration_seconds = self.duration as f64 / f64::from(sample_rate);
        let channel_gain = if channel == 0 {
            ((1.0 - self.pan) * 0.5).sqrt()
        } else {
            ((1.0 + self.pan) * 0.5).sqrt()
        };
        for note in &event.notes {
            *output += match (note, event.timbre) {
                (MusicNote::Midi(midi), MusicTimbreRef::Pitched(role)) => {
                    let timbre = timbres.pitched(role);
                    pitched_sample(
                        &timbre.field.signal,
                        midi_frequency(f64::from(*midi) + tone.pitch_shift),
                        seconds,
                        duration_seconds,
                        event.event_id,
                        local,
                    ) * timbre.gain
                }
                (MusicNote::Noise(voice), MusicTimbreRef::Percussion(_)) => {
                    let timbre = timbres.percussion(*voice);
                    transient_sample(
                        &timbre.field.signal,
                        seconds,
                        duration_seconds,
                        event.event_id,
                        local,
                    ) * timbre.gain
                }
                _ => 0.0,
            } * self.gain
                * channel_gain;
        }
    }
}

fn pitched_sample(
    signal: &SpectralTimbreSignal,
    fundamental: f64,
    time: f64,
    duration: f64,
    event_id: u32,
    frame: u64,
) -> f64 {
    let effective = (duration * signal.envelope.duration_scale).max(0.04);
    let envelope = attack_release(
        time,
        effective,
        signal.envelope.attack.min(effective),
        signal.envelope.sustain,
    );
    let tonal = signal
        .partials
        .iter()
        .map(|partial| {
            let decay = partial.decay.map_or(1.0, |rate| (-rate * time).exp());
            (TAU * fundamental * partial.ratio * time).sin() * partial.gain * decay
        })
        .sum::<f64>();
    let noise = signal.noise.as_ref().map_or(0.0, |noise| {
        let lifetime = match noise.role {
            NoiseRole::Attack => noise.decay.min(effective),
            NoiseRole::Sustain | NoiseRole::Carrier => effective,
        };
        if time >= lifetime {
            0.0
        } else {
            signed_noise(event_id, frame, 0) * noise.gain * (-time / noise.decay).exp()
        }
    });
    (tonal + noise) * envelope * signal.distance_gain
}

fn transient_sample(
    signal: &TransientSignal,
    time: f64,
    duration: f64,
    event_id: u32,
    frame: u64,
) -> f64 {
    let effective = duration
        .max(signal.envelope.decay + signal.envelope.release)
        .max(0.025);
    let envelope = attack_release(time, effective, signal.envelope.attack, 1.0);
    let bands = signal
        .bands
        .iter()
        .enumerate()
        .map(|(index, band)| {
            signed_noise(event_id, frame, index as u32 + 1) * band.gain * (-time / band.decay).exp()
        })
        .sum::<f64>();
    let resonators = signal
        .resonators
        .iter()
        .map(|item| (TAU * item.frequency * time).sin() * item.gain * (-time / item.decay).exp())
        .sum::<f64>();
    (bands + resonators) * envelope * signal.distance_gain
}

fn attack_release(time: f64, duration: f64, attack: f64, sustain: f64) -> f64 {
    if time < attack && attack > 0.0 {
        exp_lerp(SILENCE, 1.0, time / attack)
    } else {
        let progress = ((time - attack) / (duration - attack).max(1e-9)).clamp(0.0, 1.0);
        exp_lerp(sustain.max(SILENCE), SILENCE, progress)
    }
}

fn exp_lerp(left: f64, right: f64, amount: f64) -> f64 {
    left * (right / left).powf(amount.clamp(0.0, 1.0))
}

fn role_gain(role: MusicRole, tone: &super::PlaybackTone) -> f64 {
    match role {
        MusicRole::Identity => tone.identity_gain,
        MusicRole::Time => tone.time_gain,
        MusicRole::Color => tone.color_gain,
        MusicRole::Boundary => tone.boundary_gain,
        MusicRole::Tone | MusicRole::Motion => 1.0,
    }
}

fn midi_frequency(note: f64) -> f64 {
    440.0 * 2.0_f64.powf((note - 69.0) / 12.0)
}

fn pan(event_id: u32) -> f64 {
    (f64::from(mix(event_id ^ 0xa511_e9b3)) / f64::from(u32::MAX) - 0.5) * 0.28
}

fn signed_noise(event_id: u32, frame: u64, lane: u32) -> f64 {
    let folded = frame as u32 ^ (frame >> 32) as u32;
    f64::from(mix(event_id ^ folded.wrapping_mul(0x9e37_79b9) ^ lane)) / f64::from(u32::MAX) * 2.0
        - 1.0
}

fn mix(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MusicMix, MusicRole, MusicScore, MusicScoreEvent, MusicTimbreRef, MusicTimbres, MusicTrack,
        MusicTransport, NoiseVoice, PercussionTimbre, PitchedTimbre, generate_spectral_timbre,
        generate_transient_timbre,
    };

    fn track() -> GeneratedMusicTrack {
        let pitched = |role| PitchedTimbre {
            role,
            gain: 0.5,
            field: generate_spectral_timbre(&format!("seek:{role:?}")),
        };
        let percussion = |voice| PercussionTimbre {
            voice,
            gain: 0.5,
            field: generate_transient_timbre(&format!("seek:{voice:?}")),
        };
        GeneratedMusicTrack::from_resolved_score(
            48_000,
            2,
            96_000,
            MusicScore {
                transport: MusicTransport {
                    bpm: 120,
                    bars: 1,
                    steps_per_bar: 16,
                    step_duration_beats: 0.25,
                    loop_steps: 16,
                },
                mix: MusicMix::default(),
                timbres: Some(Box::new(MusicTimbres {
                    identity: pitched(MusicRole::Identity),
                    time: pitched(MusicRole::Time),
                    tone: pitched(MusicRole::Tone),
                    motion: pitched(MusicRole::Motion),
                    color: pitched(MusicRole::Color),
                    boundary: pitched(MusicRole::Boundary),
                    kick: percussion(NoiseVoice::Kick),
                    snare: percussion(NoiseVoice::Snare),
                    hat: percussion(NoiseVoice::Hat),
                })),
                events: vec![MusicScoreEvent {
                    event_id: 17,
                    track: MusicTrack::Lead,
                    step: 0,
                    duration_steps: 16,
                    notes: vec![MusicNote::Midi(60)],
                    timbre: MusicTimbreRef::Pitched(MusicRole::Identity),
                    role: MusicRole::Identity,
                    velocity: 0.4,
                }],
            },
        )
        .expect("test score produces an addressable render index")
    }

    #[test]
    fn seek_and_resume_reproduce_the_canonical_stream_exactly() {
        let track = track();
        let uninterrupted = (31_337..31_593)
            .map(|frame| track.sample_at(frame, 0).to_bits())
            .collect::<Vec<_>>();
        let resumed = (31_337..31_593)
            .map(|frame| track.sample_at(frame, 0).to_bits())
            .collect::<Vec<_>>();

        assert_eq!(resumed, uninterrupted);
        assert_eq!(
            track.sample_at(track.loop_frames + 31_337, 0).to_bits(),
            track.sample_at(31_337, 0).to_bits()
        );
    }

    #[test]
    fn block_render_is_bit_exact_across_seek_and_loop_boundaries() {
        let mut track = track();
        let mut score = track.score().clone();
        score.events.extend([
            MusicScoreEvent {
                event_id: 18,
                track: MusicTrack::Counter,
                step: 3,
                duration_steps: 5,
                notes: vec![MusicNote::Midi(67), MusicNote::Midi(72)],
                timbre: MusicTimbreRef::Pitched(MusicRole::Motion),
                role: MusicRole::Motion,
                velocity: 0.31,
            },
            MusicScoreEvent {
                event_id: 19,
                track: MusicTrack::Drums,
                step: 7,
                duration_steps: 2,
                notes: vec![MusicNote::Noise(NoiseVoice::Hat)],
                timbre: MusicTimbreRef::Percussion(NoiseVoice::Hat),
                role: MusicRole::Time,
                velocity: 0.23,
            },
        ]);
        track = GeneratedMusicTrack::from_resolved_score(
            track.sample_rate,
            track.channels,
            track.loop_frames,
            score,
        )
        .expect("extended test score produces an addressable render index");

        let frames = 1_027;
        let start = track.loop_frames - 311;
        let mut rendered = vec![f32::NAN; frames * usize::from(track.channels)];
        let mut scratch = vec![f64::NAN; rendered.len()];
        track
            .render_interleaved(start, &mut rendered, &mut scratch)
            .expect("aligned reusable buffers satisfy the render contract");
        let mut expected = Vec::with_capacity(rendered.len());
        for offset in 0..frames as u64 {
            for channel in 0..track.channels {
                expected.push(track.sample_at(start + offset, channel));
            }
        }

        assert_eq!(
            rendered
                .iter()
                .map(|sample| sample.to_bits())
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|sample| sample.to_bits())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn distant_inactive_events_do_not_scale_quantum_candidate_visits() {
        let mut track = track();
        let mut score = track.score().clone();
        score.events = (0..4_096)
            .map(|index| MusicScoreEvent {
                event_id: index,
                track: MusicTrack::Lead,
                step: index,
                duration_steps: 1,
                notes: vec![MusicNote::Midi(60)],
                timbre: MusicTimbreRef::Pitched(MusicRole::Identity),
                role: MusicRole::Identity,
                velocity: 0.2,
            })
            .collect();
        track = GeneratedMusicTrack::from_resolved_score(
            track.sample_rate,
            track.channels,
            4_096 * 6_000,
            score,
        )
        .expect("large test score produces an addressable render index");
        let event_count = track.score().events.len();
        let frames = RENDER_INDEX_BUCKET_FRAMES as usize;
        let mut output = vec![0.0; frames * usize::from(track.channels)];
        let mut scratch = vec![0.0; output.len()];

        let stats = render_interleaved_inner(&track, 0, &mut output, &mut scratch)
            .expect("test buffers are valid");

        assert_eq!(event_count, 4_096);
        assert_eq!(stats.event_segment_visits, 1);
        assert_eq!(stats.rendered_event_frames, frames);
    }

    #[test]
    fn block_render_traverses_events_once_per_indexed_interval() {
        let track = track();
        let frames = 48_000;
        let mut output = vec![0.0; frames * usize::from(track.channels)];
        let mut scratch = vec![0.0; output.len()];

        let stats = render_interleaved_inner(&track, 0, &mut output, &mut scratch)
            .expect("test buffers are valid");

        assert_eq!(
            stats.event_segment_visits,
            frames / RENDER_INDEX_BUCKET_FRAMES as usize
        );
        assert_eq!(stats.rendered_event_frames, frames);
        assert!(
            stats.event_segment_visits < frames,
            "event traversal must not regress to sample_at for every frame"
        );
    }

    #[test]
    fn block_render_rejects_misaligned_or_missing_storage() {
        let track = track();
        assert_eq!(
            track.render_interleaved(0, &mut [0.0; 3], &mut [0.0; 3]),
            Err(MusicRenderError::OutputNotFrameAligned {
                samples: 3,
                channels: 2,
            })
        );
        assert_eq!(
            track.render_interleaved(0, &mut [0.0; 4], &mut [0.0; 3]),
            Err(MusicRenderError::ScratchTooSmall {
                required: 4,
                available: 3,
            })
        );
    }

    #[test]
    fn worklet_asset_wire_round_trips_the_canonical_stream() {
        let track = track();
        let bytes =
            crate::encode_music_worklet_asset(&track).expect("resolved music should serialize");
        let decoded =
            crate::decode_music_worklet_asset(&bytes).expect("versioned music should decode");

        assert_eq!(decoded, track);
        assert_eq!(decoded.sample_rate(), track.sample_rate());
        assert_eq!(decoded.channels(), track.channels());
        assert_eq!(decoded.loop_frames(), track.loop_frames());
        assert_eq!(
            decoded.sample_at(31_337, 1).to_bits(),
            track.sample_at(31_337, 1).to_bits()
        );

        let mut wrong_version = bytes;
        let version = wrong_version
            .windows(11)
            .position(|window| window == br#""version":1"#)
            .expect("wire contains an explicit version")
            + 10;
        wrong_version[version] = b'2';
        assert!(
            crate::decode_music_worklet_asset(&wrong_version)
                .expect_err("unknown worklet wire version must fail")
                .contains("unsupported")
        );
    }
}
