mod melodic;
mod rhythm;

use super::section::RealizedSection;
use super::{PhraseBar, form::Carrier};
use crate::{
    MusicNote, MusicRole, MusicScoreEvent, MusicTimbreRef, MusicTrack, NoiseVoice, prng::Mulberry32,
};
use rhythm::{OnsetOptions, stochastic_onsets, weighted_step};

#[derive(Clone, Copy)]
pub(super) struct BarRenderContext<'a> {
    pub seed: &'a str,
    pub section: &'a RealizedSection,
    pub phrase: &'a PhraseBar,
    pub tonic: i16,
    pub scale: &'a [i16],
    pub chord_root: i32,
    pub chord: [i16; 3],
    pub bar: u16,
    pub local_bar: u8,
    pub loop_handoff: bool,
}

pub(super) fn assemble_bar(
    events: &mut Vec<MusicScoreEvent>,
    context: BarRenderContext<'_>,
) -> Result<(), String> {
    add_identity_layer(events, &context)?;
    add_time_layer(events, &context)?;
    add_tone_layer(events, &context)?;
    add_motion_layer(events, &context)?;
    add_color_layer(events, &context)?;
    add_boundary_layer(events, &context)?;
    add_transition_bridges(events, &context);
    Ok(())
}

fn add_transition_bridges(events: &mut Vec<MusicScoreEvent>, context: &BarRenderContext<'_>) {
    let &BarRenderContext {
        seed,
        section,
        phrase,
        bar,
        local_bar,
        ..
    } = context;
    if let Some(bridge) = phrase.transition_bridge {
        if local_bar >= 8 - bridge.bars {
            let carrier = format!(
                "section-bridge:{}:{}",
                role_name(bridge.role),
                bridge.carrier.as_str()
            );
            let mut rng = event_rng(seed, "boundary", &carrier, section, local_bar);
            let progress = if bridge.bars <= 1 {
                1.0
            } else {
                f64::from(local_bar - (8 - bridge.bars)) / f64::from(bridge.bars - 1)
            };
            let velocity = (0.028 + bridge.impact * 0.044) * (0.72 + progress * 0.42);
            let step = if local_bar == 7 {
                weighted_pick(
                    &[(12_u8, 0.24), (13, 0.28), (14, 0.34), (15, 0.14)],
                    &mut rng,
                )
            } else {
                random_int(&mut rng, 9, 13) as u8
            };
            add_bridge_event(
                events,
                &mut rng,
                context,
                bridge.track,
                bridge.target_degree_offset,
                step,
                velocity,
            );
        }
    }
    if let Some(bridge) = phrase.transition_entry_bridge {
        if local_bar < bridge.bars {
            let bar_start = u32::from(bar) * 16;
            let has_entry = events.iter().any(|event| {
                event.track == bridge.track && event.step >= bar_start && event.step < bar_start + 4
            });
            if !has_entry {
                let carrier = format!(
                    "section-entry:{}:{}",
                    role_name(bridge.role),
                    bridge.carrier.as_str()
                );
                let mut rng = event_rng(seed, "boundary", &carrier, section, local_bar);
                let progress = if bridge.bars <= 1 {
                    1.0
                } else {
                    1.0 - f64::from(local_bar) / f64::from(bridge.bars - 1)
                };
                let velocity = (0.022 + bridge.impact * 0.034) * (0.68 + progress * 0.28);
                let step =
                    weighted_pick(&[(0_u8, 0.42), (1, 0.32), (2, 0.18), (3, 0.08)], &mut rng);
                add_entry_bridge_event(
                    events,
                    &mut rng,
                    context,
                    bridge.track,
                    bridge.target_degree_offset,
                    step,
                    velocity,
                );
            }
        }
    }
}

fn add_bridge_event(
    events: &mut Vec<MusicScoreEvent>,
    rng: &mut Mulberry32,
    context: &BarRenderContext<'_>,
    track: MusicTrack,
    target_degree: i32,
    step: u8,
    velocity: f64,
) {
    let &BarRenderContext {
        tonic, scale, bar, ..
    } = context;
    match track {
        MusicTrack::Drums => {
            let voice = weighted_pick(
                &[
                    (NoiseVoice::Hat, 0.48),
                    (NoiseVoice::Snare, 0.34),
                    (NoiseVoice::Kick, 0.18),
                ],
                rng,
            );
            events.push(noise_event(bar, step, voice, MusicRole::Boundary, velocity));
        }
        MusicTrack::Bass => {
            let degree =
                target_degree + weighted_pick(&[(0, 0.48), (1, 0.24), (-1, 0.16), (2, 0.12)], rng);
            events.push(note_event(
                track,
                bar,
                step,
                2,
                [degree_note(tonic, scale, degree, -24)],
                MusicRole::Boundary,
                velocity * 1.2,
            ));
        }
        MusicTrack::Chord => {
            let degree =
                target_degree + weighted_pick(&[(0, 0.48), (1, 0.24), (-1, 0.16), (2, 0.12)], rng);
            let chord_offset = [0, 2, 4][(rng.uniform() * 3.0).floor() as usize];
            events.push(note_event(
                track,
                bar,
                step,
                2,
                [degree_note(tonic, scale, degree + chord_offset, 12)],
                MusicRole::Boundary,
                velocity * 0.9,
            ));
        }
        MusicTrack::Counter | MusicTrack::Lead => {
            let degree =
                target_degree + weighted_pick(&[(0, 0.48), (1, 0.24), (-1, 0.16), (2, 0.12)], rng);
            events.push(note_event(
                track,
                bar,
                step,
                2,
                [degree_note(
                    tonic,
                    scale,
                    degree,
                    if track == MusicTrack::Counter { 0 } else { 12 },
                )],
                MusicRole::Boundary,
                velocity,
            ));
        }
    }
}

fn add_entry_bridge_event(
    events: &mut Vec<MusicScoreEvent>,
    rng: &mut Mulberry32,
    context: &BarRenderContext<'_>,
    track: MusicTrack,
    target_degree: i32,
    step: u8,
    velocity: f64,
) {
    let &BarRenderContext {
        tonic,
        scale,
        chord,
        bar,
        ..
    } = context;
    match track {
        MusicTrack::Drums => {
            let voice = weighted_pick(
                &[
                    (NoiseVoice::Kick, 0.4),
                    (NoiseVoice::Hat, 0.38),
                    (NoiseVoice::Snare, 0.22),
                ],
                rng,
            );
            events.push(noise_event(bar, step, voice, MusicRole::Boundary, velocity));
        }
        MusicTrack::Bass => events.push(note_event(
            track,
            bar,
            step,
            2,
            [chord[0] - 24],
            MusicRole::Boundary,
            velocity * 1.18,
        )),
        MusicTrack::Chord => {
            let note = chord[(rng.uniform() * 3.0).floor() as usize];
            events.push(note_event(
                track,
                bar,
                step,
                2,
                [note],
                MusicRole::Boundary,
                velocity * 0.9,
            ));
        }
        MusicTrack::Counter | MusicTrack::Lead => events.push(note_event(
            track,
            bar,
            step,
            2,
            [degree_note(
                tonic,
                scale,
                target_degree,
                if track == MusicTrack::Counter { 0 } else { 12 },
            )],
            MusicRole::Boundary,
            velocity,
        )),
    }
}

fn role_name(role: MusicRole) -> &'static str {
    match role {
        MusicRole::Identity => "identity",
        MusicRole::Time => "time",
        MusicRole::Tone => "tone",
        MusicRole::Motion => "motion",
        MusicRole::Color => "color",
        MusicRole::Boundary => "boundary",
    }
}

fn add_identity_layer(
    events: &mut Vec<MusicScoreEvent>,
    context: &BarRenderContext<'_>,
) -> Result<(), String> {
    let &BarRenderContext {
        seed,
        section,
        phrase,
        bar,
        local_bar,
        ..
    } = context;
    match section.roles.identity.carrier {
        Carrier::MelodicLine => melodic::add_melodic_identity(events, context),
        Carrier::BassRiff => melodic::add_bass_identity(events, context),
        Carrier::HarmonyArp => melodic::add_harmony_identity(events, context),
        Carrier::RhythmHook => {
            let mut rng = event_rng(seed, "identity", "rhythm-hook", section, local_bar);
            let density = 0.36
                + section.identity_level * 0.16
                + phrase.energy * 0.12
                + phrase.boundary * 0.12
                + phrase.pickup * 0.1;
            let kick_count = 2 + usize::from(rng.uniform() < density);
            let kick_syncopation = 0.18 + phrase.syncopation * 0.62 + rng.uniform() * 0.18;
            let kicks = stochastic_onsets(
                &mut rng,
                kick_count as f64,
                OnsetOptions {
                    min: 0,
                    max: 14,
                    min_gap: 3,
                    anchor_start: true,
                    anchor_end: None,
                    strong_beat_bias: 0.78,
                    syncopation: kick_syncopation,
                },
            );
            let snare_count = 1 + usize::from(rng.uniform() < density);
            let snares = stochastic_onsets(
                &mut rng,
                snare_count as f64,
                OnsetOptions {
                    min: 3,
                    max: 15,
                    min_gap: 4,
                    anchor_start: false,
                    anchor_end: None,
                    strong_beat_bias: 0.34,
                    syncopation: 0.44 + phrase.syncopation * 0.34,
                },
            );
            let hat_count =
                2 + usize::from(rng.uniform() < density) + usize::from(rng.uniform() < 0.35);
            let hats = stochastic_onsets(
                &mut rng,
                hat_count as f64,
                OnsetOptions {
                    min: 1,
                    max: 15,
                    min_gap: 2,
                    anchor_start: false,
                    anchor_end: None,
                    strong_beat_bias: 0.18,
                    syncopation: 0.5 + phrase.syncopation * 0.38,
                },
            );
            let mut hits = Vec::new();
            for step in kicks {
                let weight = if step == 0 {
                    1.0
                } else {
                    0.72 + rng.uniform() * 0.22
                };
                hits.push((step, NoiseVoice::Kick, weight));
            }
            for step in snares {
                hits.push((step, NoiseVoice::Snare, 0.78 + rng.uniform() * 0.24));
            }
            for step in hats {
                hits.push((step, NoiseVoice::Hat, 0.72 + rng.uniform() * 0.28));
            }
            hits.sort_by_key(|(step, voice, _)| (*step, noise_sort_key(*voice)));
            let count = hits.len();
            let bar_lift = if local_bar == 7 { 1.12 } else { 1.0 };
            for (index, (step, voice, weight)) in hits.into_iter().enumerate() {
                let accent = melodic::identity_accent(
                    seed,
                    "rhythm-hook",
                    section,
                    phrase,
                    local_bar,
                    step,
                    index,
                    count,
                );
                let base = match voice {
                    NoiseVoice::Kick => 0.22,
                    NoiseVoice::Snare => 0.17,
                    NoiseVoice::Hat => 0.085,
                };
                events.push(noise_event(
                    bar,
                    step,
                    voice,
                    MusicRole::Identity,
                    base * weight * section.identity_level * bar_lift * accent,
                ));
            }
            Ok(())
        }
        carrier => Err(format!(
            "identity carrier `{}` is invalid",
            carrier.as_str()
        )),
    }
}

fn add_motion_layer(
    events: &mut Vec<MusicScoreEvent>,
    context: &BarRenderContext<'_>,
) -> Result<(), String> {
    let &BarRenderContext {
        seed,
        section,
        phrase,
        tonic,
        scale,
        chord_root,
        bar,
        local_bar,
        ..
    } = context;
    match section.roles.motion.carrier {
        Carrier::None => Ok(()),
        Carrier::HarmonyArp => melodic::add_harmony_motion(events, context),
        Carrier::AnswerLine => melodic::add_answer_motion(events, context),
        Carrier::BassWalk => {
            let mut gate_rng = event_rng(seed, "motion", "bass-walk", section, local_bar);
            if phrase.pickup > 0.24
                || !phrase.tone_anchor
                || gate_rng.uniform() < 0.16 + section.motion_level * 0.08 + phrase.energy * 0.08
            {
                let mut rng = event_rng(seed, "motion", "bass-walk", section, local_bar);
                let count = 2 + usize::from(
                    rng.uniform() > 0.56 - phrase.energy * 0.14 - phrase.pickup * 0.18,
                );
                let steps = stochastic_onsets(
                    &mut rng,
                    count as f64,
                    OnsetOptions {
                        min: 2,
                        max: 14,
                        min_gap: 3,
                        anchor_start: false,
                        anchor_end: None,
                        strong_beat_bias: 0.36,
                        syncopation: 0.34 + phrase.syncopation * 0.46,
                    },
                );
                let mut offset =
                    weighted_pick(&[(0, 0.34), (1, 0.32), (2, 0.24), (-1, 0.1)], &mut rng);
                for step in steps {
                    offset += weighted_pick(&[(1, 0.48), (-1, 0.24), (0, 0.28)], &mut rng);
                    events.push(note_event(
                        MusicTrack::Bass,
                        bar,
                        step,
                        2,
                        [degree_note(
                            tonic,
                            scale,
                            chord_root + offset.clamp(-1, 4),
                            -24,
                        )],
                        MusicRole::Motion,
                        0.12 * section.motion_level,
                    ));
                }
            }
            Ok(())
        }
        Carrier::PercussionFill => {
            if phrase.boundary > 0.72 || phrase.pickup > 0.42 {
                let mut rng = event_rng(seed, "motion", "percussion-fill", section, local_bar);
                let steps = stochastic_onsets(
                    &mut rng,
                    2.0,
                    OnsetOptions {
                        min: 9,
                        max: 15,
                        min_gap: 2,
                        anchor_start: false,
                        anchor_end: None,
                        strong_beat_bias: 0.2,
                        syncopation: 0.75,
                    },
                );
                for (index, step) in steps.into_iter().enumerate() {
                    events.push(noise_event(
                        bar,
                        step,
                        if index == 0 {
                            NoiseVoice::Snare
                        } else {
                            NoiseVoice::Hat
                        },
                        MusicRole::Motion,
                        if index == 0 { 0.11 } else { 0.07 },
                    ));
                }
            }
            Ok(())
        }
        carrier => Err(format!("motion carrier `{}` is invalid", carrier.as_str())),
    }
}

pub(super) fn add_time_layer(
    events: &mut Vec<MusicScoreEvent>,
    context: &BarRenderContext<'_>,
) -> Result<(), String> {
    let &BarRenderContext {
        seed,
        section,
        phrase,
        chord,
        bar,
        local_bar,
        ..
    } = context;
    match section.roles.time.carrier {
        Carrier::DrumGrid => {
            let mut rng = event_rng(seed, "time", "drum-grid", section, local_bar);
            let kick_count = 1
                + usize::from(phrase.pace > 0.34)
                + usize::from(phrase.energy > 1.18 && phrase.pace > 0.54 && rng.uniform() > 0.52);
            let mut hits = stochastic_onsets(
                &mut rng,
                kick_count as f64,
                OnsetOptions {
                    min: 0,
                    max: 12,
                    min_gap: 5,
                    anchor_start: true,
                    anchor_end: None,
                    strong_beat_bias: 0.88,
                    syncopation: 0.1 + phrase.syncopation * 0.22,
                },
            )
            .into_iter()
            .enumerate()
            .map(|(index, step)| {
                (
                    step,
                    NoiseVoice::Kick,
                    if index == 0 { 1.0 } else { 0.72 } * phrase.energy,
                )
            })
            .collect::<Vec<_>>();
            hits.extend(
                stochastic_onsets(
                    &mut rng,
                    2.0,
                    OnsetOptions {
                        min: 3,
                        max: 13,
                        min_gap: 5,
                        anchor_start: false,
                        anchor_end: None,
                        strong_beat_bias: 0.74,
                        syncopation: 0.18 + phrase.syncopation * 0.32,
                    },
                )
                .into_iter()
                .map(|step| (step, NoiseVoice::Snare, 0.92 * phrase.energy)),
            );
            let hat_count = (1.4 + phrase.pace * 3.6 + usize::from(phrase.pickup > 0.25) as f64)
                .round()
                .clamp(1.0, 6.0);
            hits.extend(
                stochastic_onsets(
                    &mut rng,
                    hat_count,
                    OnsetOptions {
                        min: 1,
                        max: 15,
                        min_gap: if phrase.pace < 0.36 { 4 } else { 2 },
                        anchor_start: false,
                        anchor_end: None,
                        strong_beat_bias: 0.36,
                        syncopation: 0.36 + phrase.syncopation * 0.38,
                    },
                )
                .into_iter()
                .map(|step| (step, NoiseVoice::Hat, 0.88 + phrase.pickup * 0.2)),
            );
            hits.sort_by_key(|(step, voice, _)| (*step, noise_sort_key(*voice)));
            for (step, voice, weight) in hits {
                let base = match voice {
                    NoiseVoice::Kick => 0.2,
                    NoiseVoice::Snare => 0.14,
                    NoiseVoice::Hat => 0.055,
                };
                events.push(noise_event(
                    bar,
                    step,
                    voice,
                    MusicRole::Time,
                    base * weight,
                ));
            }
            Ok(())
        }
        Carrier::BassPulse => {
            let mut rng = event_rng(seed, "time", "bass-pulse", section, local_bar);
            let extra =
                usize::from(rng.uniform() > 0.76 - phrase.energy * 0.16 - phrase.pickup * 0.16);
            let count = (1.2 + phrase.pace * 2.3 + extra as f64)
                .round()
                .clamp(1.0, 4.0);
            let options = OnsetOptions {
                min: 0,
                max: 13,
                min_gap: if phrase.pace < 0.4 { 5 } else { 4 },
                anchor_start: true,
                anchor_end: None,
                strong_beat_bias: 0.82,
                syncopation: 0.12 + phrase.syncopation * 0.32,
            };
            for step in stochastic_onsets(&mut rng, count, options) {
                events.push(note_event(
                    MusicTrack::Bass,
                    bar,
                    step,
                    if step == 0 { 3 } else { 2 },
                    [chord[0] - 24],
                    MusicRole::Time,
                    (if step == 0 { 0.15 } else { 0.11 }) * phrase.energy,
                ));
            }
            Ok(())
        }
        Carrier::ArpPulse => {
            melodic::add_arp_time(events, context)?;
            ensure_time_anchor(events, seed, section, phrase, chord, bar, local_bar);
            Ok(())
        }
        Carrier::ThinPulse => {
            let mut rng = event_rng(seed, "time", "thin-pulse", section, local_bar);
            let pressure = 0.28
                + usize::from(phrase.tone_anchor) as f64 * 0.22
                + phrase.boundary * 0.2
                + phrase.pickup * 0.18
                + phrase.energy * 0.08;
            if rng.uniform() <= pressure {
                let count = 1 + usize::from(
                    rng.uniform() < 0.18 + phrase.pickup * 0.24 + phrase.boundary * 0.12,
                );
                let steps = stochastic_onsets(
                    &mut rng,
                    count as f64,
                    OnsetOptions {
                        min: if phrase.pickup > 0.38 { 1 } else { 0 },
                        max: if phrase.boundary > 0.7 { 15 } else { 13 },
                        min_gap: 4,
                        anchor_start: false,
                        anchor_end: None,
                        strong_beat_bias: 0.26 + phrase.stability * 0.18,
                        syncopation: (0.34 + phrase.syncopation * 0.5 + phrase.pickup * 0.18)
                            .clamp(0.0, 1.0),
                    },
                );
                for (index, step) in steps.into_iter().enumerate() {
                    let voice = weighted_pick(
                        &[
                            (NoiseVoice::Kick, if step <= 2 { 0.42 } else { 0.2 }),
                            (NoiseVoice::Hat, 0.42 + phrase.space * 0.18),
                            (NoiseVoice::Snare, if step >= 7 { 0.22 } else { 0.08 }),
                        ],
                        &mut rng,
                    );
                    let velocity = (if index == 0 { 0.095 } else { 0.065 })
                        * (0.74 + phrase.energy * 0.28).clamp(0.7, 1.12);
                    events.push(noise_event(bar, step, voice, MusicRole::Time, velocity));
                }
            }
            ensure_time_anchor(events, seed, section, phrase, chord, bar, local_bar);
            Ok(())
        }
        carrier => Err(format!(
            "carrier `{}` is invalid for time",
            carrier.as_str()
        )),
    }
}

pub(super) fn add_tone_layer(
    events: &mut Vec<MusicScoreEvent>,
    context: &BarRenderContext<'_>,
) -> Result<(), String> {
    let &BarRenderContext {
        seed,
        section,
        phrase,
        chord,
        bar,
        local_bar,
        ..
    } = context;
    let carrier = section.roles.tone.carrier;
    let mut rng = event_rng(seed, "tone", carrier.as_str(), section, local_bar);
    match carrier {
        Carrier::RootBass => {
            if phrase.tone_anchor || section.variant == 2 && phrase.pickup > 0.28 {
                let step = harmonic_support_step(&mut rng, phrase, 0.72, 6);
                let duration = (random_int(&mut rng, 5, 10) + (phrase.space * 4.0).round() as i32)
                    .clamp(4, 13) as u16;
                events.push(note_event(
                    MusicTrack::Bass,
                    bar,
                    step,
                    duration,
                    [chord[0] - 24],
                    MusicRole::Tone,
                    0.1 * phrase.energy,
                ));
            }
            Ok(())
        }
        Carrier::ChordPad => {
            if phrase.tone_anchor {
                let step = harmonic_support_step(&mut rng, phrase, 0.64, 5);
                let duration = (random_int(&mut rng, 7, 13) + (phrase.space * 3.0).round() as i32)
                    .clamp(6, 15) as u16;
                events.push(note_event(
                    MusicTrack::Chord,
                    bar,
                    step,
                    duration,
                    chord.map(|note| note + 12),
                    MusicRole::Tone,
                    0.048 * phrase.energy,
                ));
            }
            Ok(())
        }
        Carrier::Drone => {
            if phrase.tone_anchor
                && (phrase.stability > 0.62 || phrase.boundary > 0.58 || phrase.energy < 0.9)
            {
                let step = harmonic_support_step(&mut rng, phrase, 0.82, 3);
                let duration =
                    (16 - i32::from(step) + random_int(&mut rng, -1, 1)).clamp(8, 16) as u16;
                events.push(note_event(
                    MusicTrack::Chord,
                    bar,
                    step,
                    duration,
                    [chord[0]],
                    MusicRole::Tone,
                    0.064 * phrase.energy,
                ));
            }
            Ok(())
        }
        Carrier::Implied => Ok(()),
        carrier => Err(format!(
            "carrier `{}` is invalid for tone",
            carrier.as_str()
        )),
    }
}

pub(super) fn add_color_layer(
    events: &mut Vec<MusicScoreEvent>,
    context: &BarRenderContext<'_>,
) -> Result<(), String> {
    let &BarRenderContext {
        seed,
        section,
        phrase,
        chord,
        bar,
        local_bar,
        ..
    } = context;
    let carrier = section.roles.color.carrier;
    let mut rng = event_rng(seed, "color", carrier.as_str(), section, local_bar);
    match carrier {
        Carrier::None => Ok(()),
        Carrier::AirPad => {
            if phrase.color_accent {
                let step = color_accent_step(&mut rng, phrase, 0, 7);
                events.push(note_event(
                    MusicTrack::Chord,
                    bar,
                    step,
                    random_int(&mut rng, 6, 12) as u16,
                    [chord[1] + 12, chord[2] + 12],
                    MusicRole::Color,
                    0.04 * section.color_level,
                ));
            }
            Ok(())
        }
        Carrier::NoiseHalo => {
            if phrase.color_accent || phrase.space > 0.58 {
                let step = color_accent_step(&mut rng, phrase, 4, 14);
                events.push(note_event(
                    MusicTrack::Lead,
                    bar,
                    step,
                    random_int(&mut rng, 3, 7) as u16,
                    [chord[1] + 12],
                    MusicRole::Color,
                    0.036 * section.color_level * (0.82 + phrase.space),
                ));
            }
            Ok(())
        }
        Carrier::OrganBed => {
            if phrase.tone_anchor {
                let step = color_accent_step(&mut rng, phrase, 1, 8);
                events.push(note_event(
                    MusicTrack::Chord,
                    bar,
                    step,
                    random_int(&mut rng, 6, 10) as u16,
                    chord.map(|note| note + 12),
                    MusicRole::Color,
                    0.042 * section.color_level * phrase.energy,
                ));
            }
            Ok(())
        }
        Carrier::BrightAccent => {
            if phrase.pickup > 0.3 || phrase.boundary > 0.7 {
                let step = color_accent_step(&mut rng, phrase, 7, 15);
                events.push(note_event(
                    MusicTrack::Lead,
                    bar,
                    step,
                    random_int(&mut rng, 1, 3) as u16,
                    [chord[2] + 12],
                    MusicRole::Color,
                    0.055 * section.color_level * phrase.energy,
                ));
            }
            Ok(())
        }
        carrier => Err(format!(
            "carrier `{}` is invalid for color",
            carrier.as_str()
        )),
    }
}

pub(super) fn add_boundary_layer(
    events: &mut Vec<MusicScoreEvent>,
    context: &BarRenderContext<'_>,
) -> Result<(), String> {
    let &BarRenderContext {
        seed,
        section,
        phrase,
        tonic,
        scale,
        bar,
        local_bar,
        loop_handoff,
        ..
    } = context;
    if phrase.boundary < 0.72 {
        return Ok(());
    }
    if loop_handoff && local_bar == 7 {
        let mut rng = event_rng(seed, "boundary", "loop-handoff", section, 7);
        let approach = weighted_pick(&[(-1, 0.26), (1, 0.42), (2, 0.2), (4, 0.12)], &mut rng);
        let lead_step = weighted_pick(
            &[(12_u8, 0.22), (13, 0.24), (14, 0.42), (15, 0.12)],
            &mut rng,
        );
        let bass_step = if lead_step >= 14 { 15 } else { 14 };
        events.push(note_event(
            MusicTrack::Lead,
            bar,
            lead_step,
            1,
            [degree_note(tonic, scale, approach, 12)],
            MusicRole::Boundary,
            0.055 * section.boundary_level,
        ));
        events.push(note_event(
            MusicTrack::Bass,
            bar,
            bass_step,
            1,
            [degree_note(tonic, scale, 0, -12)],
            MusicRole::Boundary,
            0.06 * section.boundary_level,
        ));
        return Ok(());
    }
    match section.roles.boundary.carrier {
        // RestGap owns absence. A phrase boundary is represented by the
        // surrounding layers leaving space; it emits no synthetic event.
        Carrier::RestGap => Ok(()),
        Carrier::DrumFill => {
            let mut rng = event_rng(seed, "boundary", "drum-fill", section, local_bar);
            let count = 2 + usize::from(rng.uniform() > 0.58);
            let steps = stochastic_onsets(
                &mut rng,
                count as f64,
                OnsetOptions {
                    min: 10,
                    max: 15,
                    min_gap: 1,
                    anchor_start: false,
                    anchor_end: None,
                    strong_beat_bias: 0.18,
                    syncopation: 0.84,
                },
            );
            for (index, step) in steps.into_iter().enumerate() {
                events.push(noise_event(
                    bar,
                    step,
                    if index % 2 == 0 {
                        NoiseVoice::Snare
                    } else {
                        NoiseVoice::Hat
                    },
                    MusicRole::Boundary,
                    (if index == 0 { 0.12 } else { 0.08 }) * section.boundary_level,
                ));
            }
            Ok(())
        }
        Carrier::ContrastNote | Carrier::RegisterTurn => {
            melodic::add_pitched_boundary(events, context)
        }
        carrier => Err(format!(
            "carrier `{}` is invalid for boundary",
            carrier.as_str()
        )),
    }
}

fn harmonic_support_step(
    rng: &mut Mulberry32,
    phrase: &PhraseBar,
    early_bias: f64,
    max: i32,
) -> u8 {
    let candidates = (0..=max).collect::<Vec<_>>();
    let (pool, options) = if phrase.pickup > 0.34 && rng.uniform() < 0.62 {
        (
            candidates
                .iter()
                .copied()
                .filter(|step| *step >= 1)
                .collect::<Vec<_>>(),
            OnsetOptions {
                min: 0,
                max,
                min_gap: 1,
                anchor_start: false,
                anchor_end: None,
                strong_beat_bias: 0.22,
                syncopation: (phrase.syncopation + 0.2).clamp(0.0, 1.0),
            },
        )
    } else {
        (
            candidates,
            OnsetOptions {
                min: 0,
                max,
                min_gap: 1,
                anchor_start: false,
                anchor_end: None,
                strong_beat_bias: early_bias,
                syncopation: (phrase.syncopation * 0.42).clamp(0.0, 1.0),
            },
        )
    };
    weighted_step(&pool, rng, options) as u8
}

fn ensure_time_anchor(
    events: &mut Vec<MusicScoreEvent>,
    seed: &str,
    section: &RealizedSection,
    phrase: &PhraseBar,
    chord: [i16; 3],
    bar: u16,
    local_bar: u8,
) {
    let bar_start = u32::from(bar) * 16;
    if events
        .iter()
        .any(|event| event.role == MusicRole::Time && event.step == bar_start)
    {
        return;
    }
    let mut rng = event_rng(
        seed,
        "time-anchor",
        section.roles.time.carrier.as_str(),
        section,
        local_bar,
    );
    let base_velocity = (0.82 + phrase.energy * 0.2 + phrase.stability * 0.08).clamp(0.78, 1.08);
    let target = match section.roles.time.carrier {
        Carrier::ThinPulse => weighted_pick(
            &[
                (MusicTrack::Drums, 0.38),
                (MusicTrack::Bass, 0.24),
                (MusicTrack::Chord, 0.22),
                (MusicTrack::Lead, 0.16),
            ],
            &mut rng,
        ),
        Carrier::BassPulse => MusicTrack::Bass,
        Carrier::ArpPulse => MusicTrack::Chord,
        Carrier::DrumGrid => MusicTrack::Drums,
        _ => unreachable!("time anchor received a non-time carrier"),
    };
    match target {
        MusicTrack::Bass => events.push(note_event(
            MusicTrack::Bass,
            bar,
            0,
            2,
            [chord[0] - 24],
            MusicRole::Time,
            0.075 * base_velocity,
        )),
        MusicTrack::Chord => {
            let note = [chord[0], chord[1], chord[2], chord[0] + 12]
                [(rng.uniform() * 4.0).floor() as usize];
            events.push(note_event(
                MusicTrack::Chord,
                bar,
                0,
                if phrase.pace < 0.44 { 2 } else { 1 },
                [note],
                MusicRole::Time,
                0.045 * base_velocity,
            ));
        }
        MusicTrack::Lead => events.push(note_event(
            MusicTrack::Lead,
            bar,
            0,
            1,
            [chord[0] + 12],
            MusicRole::Time,
            0.04 * base_velocity,
        )),
        _ => events.push(noise_event(
            bar,
            0,
            NoiseVoice::Kick,
            MusicRole::Time,
            0.052 * base_velocity,
        )),
    }
}

fn noise_sort_key(voice: NoiseVoice) -> u8 {
    // JavaScript sorts equal-step sound strings lexically: hat, kick, snare.
    match voice {
        NoiseVoice::Hat => 0,
        NoiseVoice::Kick => 1,
        NoiseVoice::Snare => 2,
    }
}

fn color_accent_step(rng: &mut Mulberry32, phrase: &PhraseBar, min: i32, max: i32) -> u8 {
    let count = 1.0
        + usize::from(phrase.energy > 1.08 && phrase.pickup > 0.28 && rng.uniform() < 0.34) as f64;
    stochastic_onsets(
        rng,
        count,
        OnsetOptions {
            min,
            max,
            min_gap: 2,
            anchor_start: false,
            anchor_end: None,
            strong_beat_bias: if phrase.boundary > 0.6 { 0.22 } else { 0.34 },
            syncopation: (0.24 + phrase.syncopation * 0.58 + phrase.pickup * 0.2).clamp(0.0, 1.0),
        },
    )[0]
}

fn random_int(rng: &mut Mulberry32, min: i32, max: i32) -> i32 {
    min + (rng.uniform() * f64::from(max - min + 1)).floor() as i32
}

fn event_rng(
    seed: &str,
    role: &str,
    carrier: &str,
    section: &RealizedSection,
    local_bar: u8,
) -> Mulberry32 {
    Mulberry32::from_text(&format!(
        "{seed}:{role}:{carrier}:section-{}:{}:{}:{}:{}:{}:{}:{local_bar}",
        section.index,
        section.motif_variant,
        section.variant,
        section.state.novelty,
        section.state.stability,
        section.state.tension,
        section.state.closure_pressure,
    ))
}

pub(super) fn build_progression(rng: &mut Mulberry32, scale_length: i32) -> Vec<i32> {
    let roots = (0..scale_length).collect::<Vec<_>>();
    let mut progression = vec![0];
    for phase in 1..4 {
        let previous = *progression.last().expect("progression starts at home");
        let candidates = roots
            .iter()
            .map(|&root| {
                (
                    root,
                    harmonic_root_weight(root, phase, scale_length, previous),
                )
            })
            .collect::<Vec<_>>();
        progression.push(weighted_pick(&candidates, rng));
    }
    progression
}

fn harmonic_root_weight(root: i32, phase: i32, scale_length: i32, previous: i32) -> f64 {
    let home_distance = modular_distance(root, 0, scale_length);
    let motion_distance = modular_distance(root, previous, scale_length);
    let phase_target = match phase {
        1 => 2.min(scale_length - 1),
        2 => 3.min(scale_length - 1),
        3 => 1,
        _ => 2,
    };
    let departure = gaussian_score(
        home_distance as f64,
        phase_target as f64,
        if phase == 3 { 0.9 } else { 1.25 },
    );
    let motion = gaussian_score(
        motion_distance as f64,
        if phase == 3 { 1.5 } else { 2.0 },
        1.15,
    );
    let home = if root == 0 {
        if phase == 3 { 0.72 } else { 0.04 }
    } else {
        0.0
    };
    let return_prep = if phase == 3 {
        1.0 / (1.0 + home_distance as f64) + if root == scale_length - 2 { 0.28 } else { 0.0 }
    } else {
        0.0
    };
    let tension = if phase == 2 {
        home_distance as f64 / (scale_length / 2).max(1) as f64
    } else {
        0.0
    };
    let repeat = if root == previous {
        if phase == 3 { 0.48 } else { 0.16 }
    } else {
        1.0
    };
    (departure + motion * 0.58 + home + return_prep + tension * 0.36)
        .mul_add(repeat, 0.0)
        .max(0.001)
}

pub(super) fn build_chord(tonic: i16, scale: &[i16], degree: i32) -> [i16; 3] {
    [
        degree_note(tonic, scale, degree, 0),
        degree_note(tonic, scale, degree + 2, 0),
        degree_note(tonic, scale, degree + 4, 0),
    ]
}

pub(super) fn degree_note(tonic: i16, scale: &[i16], degree: i32, octave: i16) -> i16 {
    let length = scale.len() as i32;
    tonic
        + scale[degree.rem_euclid(length) as usize]
        + degree.div_euclid(length) as i16 * 12
        + octave
}

pub(super) fn note_event(
    track: MusicTrack,
    bar: u16,
    step: u8,
    duration_steps: u16,
    notes: impl IntoIterator<Item = i16>,
    role: MusicRole,
    velocity: f64,
) -> MusicScoreEvent {
    MusicScoreEvent {
        event_id: 0,
        track,
        step: u32::from(bar) * 16 + u32::from(step),
        duration_steps,
        notes: notes
            .into_iter()
            .map(|note| MusicNote::Midi(fit_register(track, note + register_shift(track))))
            .collect(),
        timbre: MusicTimbreRef::Pitched(role),
        role,
        velocity,
    }
}

pub(super) fn noise_event(
    bar: u16,
    step: u8,
    voice: NoiseVoice,
    role: MusicRole,
    velocity: f64,
) -> MusicScoreEvent {
    MusicScoreEvent {
        event_id: 0,
        track: MusicTrack::Drums,
        step: u32::from(bar) * 16 + u32::from(step),
        duration_steps: 1,
        notes: vec![MusicNote::Noise(voice)],
        timbre: MusicTimbreRef::Percussion(voice),
        role,
        velocity,
    }
}

fn register_shift(track: MusicTrack) -> i16 {
    match track {
        MusicTrack::Lead | MusicTrack::Counter | MusicTrack::Chord => -24,
        MusicTrack::Bass | MusicTrack::Drums => 0,
    }
}

fn fit_register(track: MusicTrack, mut note: i16) -> i16 {
    let (min, max) = match track {
        MusicTrack::Lead => (45, 88),
        MusicTrack::Counter => (38, 76),
        MusicTrack::Chord => (36, 86),
        MusicTrack::Bass => (32, 70),
        MusicTrack::Drums => (24, 96),
    };
    while note > max {
        note -= 12;
    }
    while note < min {
        note += 12;
    }
    note
}

fn modular_distance(left: i32, right: i32, size: i32) -> i32 {
    let direct = (left - right).abs() % size;
    direct.min(size - direct)
}

fn gaussian_score(value: f64, target: f64, spread: f64) -> f64 {
    let distance = value - target;
    (-(distance * distance) / (2.0 * spread * spread)).exp()
}

fn weighted_pick<T: Copy>(candidates: &[(T, f64)], rng: &mut Mulberry32) -> T {
    let mut ticket = rng.uniform() * candidates.iter().map(|(_, weight)| weight).sum::<f64>();
    for &(item, weight) in candidates {
        ticket -= weight;
        if ticket <= 0.0 {
            return item;
        }
    }
    candidates
        .last()
        .expect("weighted candidates are non-empty")
        .0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_progression_matches_javascript() {
        let mut rng = Mulberry32::from_text("composition:same-seed");
        assert_eq!(build_progression(&mut rng, 7), [0, 2, 4, 0]);
    }
}
