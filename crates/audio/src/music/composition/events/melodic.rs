use std::cmp::Ordering;

use crate::{MusicRole, MusicScoreEvent, MusicTrack, prng::Mulberry32};

use super::{
    BarRenderContext, degree_note, event_rng, note_event,
    rhythm::{OnsetOptions, onset_weight, pulse_onsets, stochastic_onsets, textural_event_count},
};
use crate::music::composition::{PhraseBar, form::Carrier, section::RealizedSection};

type MelodicNote = (u8, i32, u16);

pub(super) fn add_melodic_identity(
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
        loop_handoff,
        ..
    } = context;
    if section.roles.identity.carrier != Carrier::MelodicLine {
        return Err(format!(
            "melodic identity generator received carrier `{}`",
            section.roles.identity.carrier.as_str()
        ));
    }

    let pattern = generate_melodic_line(seed, local_bar, section, phrase, loop_handoff);
    for (index, &(step, offset, duration)) in pattern.iter().enumerate() {
        let accent = identity_accent(
            seed,
            "melodic-line",
            section,
            phrase,
            local_bar,
            step,
            index,
            pattern.len(),
        );
        events.push(note_event(
            MusicTrack::Lead,
            bar,
            step,
            duration,
            [degree_note(tonic, scale, chord_root + offset, 12)],
            MusicRole::Identity,
            0.13 * section.identity_level * accent,
        ));
    }
    Ok(())
}

pub(super) fn add_harmony_motion(
    events: &mut Vec<MusicScoreEvent>,
    context: &BarRenderContext<'_>,
) -> Result<(), String> {
    let section = context.section;
    add_harmony_role(
        events,
        context,
        "motion",
        MusicRole::Motion,
        0.052 * section.motion_level,
    )
}

pub(super) fn add_harmony_identity(
    events: &mut Vec<MusicScoreEvent>,
    context: &BarRenderContext<'_>,
) -> Result<(), String> {
    let section = context.section;
    add_harmony_role(
        events,
        context,
        "identity",
        MusicRole::Identity,
        0.075 * section.identity_level,
    )
}

pub(super) fn add_bass_identity(
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
    if section.roles.identity.carrier != Carrier::BassRiff {
        return Err(format!(
            "bass identity generator received carrier `{}`",
            section.roles.identity.carrier.as_str()
        ));
    }
    let mut rng = event_rng(seed, "identity", "bass-riff", section, local_bar);
    let extra_draw = rng.uniform();
    let count = js_round(
        1.4 + phrase.pace * 2.2
            + usize::from(extra_draw > 0.68 - phrase.pickup * 0.22 && section.state.density > 0.28)
                as f64,
    )
    .clamp(1, 4) as usize;
    let mut base_rng = motif_rng(seed, "identity", "bass-riff", "rhythm", section);
    let base_count = js_round(2.0 + base_rng.uniform() * 2.0).clamp(2, 4) as usize;
    let base = project_rhythm_cell(
        &generated_rhythm_cell(seed, "identity", "bass-riff", section, base_count as f64),
        0,
        14,
        if phrase.pace < 0.42 { 5 } else { 3 },
    )
    .into_iter()
    .map(i32::from)
    .collect::<Vec<_>>();
    let developed = develop_rhythm_motif(
        &base,
        phrase,
        local_bar,
        &mut rng,
        motif_presence(seed, "bass-riff", local_bar, section, phrase),
        true,
    );
    let steps = fit_rhythm_motif(
        &developed,
        count,
        0,
        14,
        if phrase.pace < 0.42 { 5 } else { 3 },
        &mut rng,
        phrase.syncopation,
        true,
    );
    let contour = identity_contour_offsets(
        steps.len(),
        seed,
        "bass-riff",
        local_bar,
        section,
        phrase,
        &mut rng,
        -1,
        4,
    );
    let pattern_len = steps.len();
    for (index, step) in steps.into_iter().enumerate() {
        let motif_offset = contour.get(index).copied().unwrap_or(0);
        let approach = if step >= 11 { 0.28 } else { 0.12 };
        let offset = weighted_pick(
            &[
                (motif_offset, 0.62),
                ((motif_offset + 1).clamp(-1, 4), 0.14 + approach),
                ((motif_offset - 1).clamp(-1, 4), 0.14),
                (0, 0.18),
            ],
            &mut rng,
        );
        let duration = if step <= 1 || step >= 12 {
            2 + u16::from(step <= 1)
        } else {
            2 + u16::from(step % 4 == 0)
        };
        let accent = identity_accent(
            seed,
            "bass-riff",
            section,
            phrase,
            local_bar,
            step,
            index,
            pattern_len,
        );
        events.push(note_event(
            MusicTrack::Bass,
            bar,
            step,
            duration,
            [degree_note(tonic, scale, chord_root + offset, -24)],
            MusicRole::Identity,
            0.17 * section.identity_level * accent,
        ));
    }
    Ok(())
}

fn add_harmony_role(
    events: &mut Vec<MusicScoreEvent>,
    context: &BarRenderContext<'_>,
    role_name: &str,
    event_role: MusicRole,
    base_velocity: f64,
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
        loop_handoff,
        ..
    } = context;
    let carrier = match event_role {
        MusicRole::Identity => section.roles.identity.carrier,
        MusicRole::Motion => section.roles.motion.carrier,
        _ => Carrier::None,
    };
    if carrier != Carrier::HarmonyArp {
        return Err(format!(
            "harmony generator received carrier `{}`",
            carrier.as_str()
        ));
    }
    let mut rng = event_rng(seed, role_name, "harmony-arp", section, local_bar);
    let dense = section.state.closure_pressure > 0.66 && section.state.density > 0.52
        || section.motion_level > 1.15;
    let count = textural_event_count(
        &mut rng,
        0.9 + phrase.pace * 1.65 + if dense { 0.34 } else { 0.0 },
        if phrase.pace < 0.34 { 1 } else { 2 },
        0.04 + phrase.tension * 0.08 + phrase.pickup * 0.1,
    );
    let mut base_rng = motif_rng(seed, role_name, "harmony-arp", "rhythm", section);
    let base_count = js_round(2.0 + base_rng.uniform() * 3.0).clamp(2, 5) as usize;
    let base_steps = project_rhythm_cell(
        &generated_rhythm_cell(seed, role_name, "harmony-arp", section, base_count as f64),
        0,
        15,
        if phrase.pace < 0.5 { 4 } else { 3 },
    )
    .into_iter()
    .map(i32::from)
    .collect::<Vec<_>>();
    let memory = motif_presence(seed, "harmony-arp", local_bar, section, phrase);
    let developed = develop_rhythm_motif(&base_steps, phrase, local_bar, &mut rng, memory, true);
    let steps = fit_rhythm_motif(
        &developed,
        count,
        0,
        15,
        if phrase.pace < 0.5 { 4 } else { 3 },
        &mut rng,
        phrase.syncopation,
        true,
    );
    let targets = harmonic_targets_for_bar(phrase);
    let motif_offsets = contour_for_steps(
        &steps,
        seed,
        "harmony-arp",
        local_bar,
        section,
        phrase,
        &mut rng,
        -1,
        9,
    );
    let initial = weighted_pick(
        &targets
            .iter()
            .map(|&target| (target, if target <= 4 { 0.42 } else { 0.24 }))
            .collect::<Vec<_>>(),
        &mut rng,
    );
    let target = weighted_pick(
        &targets
            .iter()
            .map(|&target| {
                (
                    target,
                    if phrase.boundary > 0.6 && target <= 2 {
                        0.48
                    } else {
                        0.24
                    },
                )
            })
            .collect::<Vec<_>>(),
        &mut rng,
    );
    let candidates = (-1..=9).collect::<Vec<_>>();
    let offsets = sample_pitch_path(
        &candidates,
        steps.len(),
        &mut rng,
        initial,
        target,
        phrase,
        section,
        &motif_offsets,
        false,
        loop_handoff,
    );
    for (index, step) in steps.into_iter().enumerate() {
        let offset = offsets[index];
        let chord_bias = if [0, 2, 4, 7].contains(&offset.rem_euclid(7)) {
            0
        } else if rng.uniform() < 0.32 {
            weighted_pick(&[(-1, 0.3), (1, 0.3), (0, 0.4)], &mut rng)
        } else {
            0
        };
        let duration =
            js_round(2.0 + (1.0 - phrase.pace) * 2.2 + phrase.closure * 0.8 + rng.uniform() * 0.8)
                .clamp(2, 5) as u16;
        let velocity = if event_role == MusicRole::Identity {
            base_velocity
                * identity_accent(
                    seed,
                    "harmony-arp",
                    section,
                    phrase,
                    local_bar,
                    step,
                    index,
                    offsets.len(),
                )
        } else {
            base_velocity
        };
        events.push(note_event(
            MusicTrack::Chord,
            bar,
            step,
            duration,
            [degree_note(
                tonic,
                scale,
                chord_root + (offset + chord_bias).clamp(-1, 9),
                12,
            )],
            event_role,
            velocity,
        ));
    }
    Ok(())
}

pub(super) fn add_arp_time(
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
        loop_handoff,
        ..
    } = context;
    if section.roles.time.carrier != Carrier::ArpPulse {
        return Err(format!(
            "arp time generator received carrier `{}`",
            section.roles.time.carrier.as_str()
        ));
    }
    let mut rng = event_rng(seed, "time", "arp-pulse", section, local_bar);
    let count = textural_event_count(
        &mut rng,
        0.78 + phrase.pace * 1.72 + phrase.pickup * 0.42,
        if phrase.pace < 0.36 { 1 } else { 2 },
        0.03 + phrase.tension * 0.07 + phrase.pickup * 0.08,
    );
    let steps = pulse_onsets(
        &mut rng,
        count as f64,
        if section.variant == 2 { 2 } else { 1 },
        if phrase.pace < 0.5 { 5 } else { 4 },
        0.28,
        0.62,
    );
    let candidates = harmonic_pulse_candidates(chord)
        .into_iter()
        .map(i32::from)
        .collect::<Vec<_>>();
    let initial = weighted_pick(
        &candidates
            .iter()
            .map(|&note| {
                (
                    note,
                    if chord.contains(&(note as i16)) {
                        0.42
                    } else {
                        0.22
                    },
                )
            })
            .collect::<Vec<_>>(),
        &mut rng,
    );
    let target = i32::from(harmonic_pulse_target(chord, phrase));
    let notes = sample_pitch_path(
        &candidates,
        steps.len(),
        &mut rng,
        initial,
        target,
        phrase,
        section,
        &[],
        false,
        loop_handoff,
    );
    for (index, step) in steps.into_iter().enumerate() {
        events.push(note_event(
            MusicTrack::Chord,
            bar,
            step,
            if phrase.pace < 0.44 { 2 } else { 1 },
            [notes[index] as i16],
            MusicRole::Time,
            0.052 * phrase.energy,
        ));
    }
    Ok(())
}

pub(super) fn add_answer_motion(
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
        loop_handoff,
        ..
    } = context;
    if section.roles.motion.carrier != Carrier::AnswerLine {
        return Err(format!(
            "answer motion generator received carrier `{}`",
            section.roles.motion.carrier.as_str()
        ));
    }
    let mut rng = event_rng(seed, "motion", "answer-line", section, local_bar);
    let pressure = if phrase.tension > 0.58 && phrase.closure < 0.62 {
        0.58 + phrase.energy * 0.14 + section.state.memory_distance * 0.16
    } else if section.variant == 2 && phrase.pickup > 0.24 {
        0.48
    } else {
        0.12 + phrase.pickup * 0.34
    };
    if rng.uniform() < pressure {
        let count = 2 + usize::from(rng.uniform() > 0.48 - phrase.energy * 0.12);
        let steps = stochastic_onsets(
            &mut rng,
            count as f64,
            OnsetOptions {
                min: 1,
                max: 13,
                min_gap: 3,
                anchor_start: false,
                anchor_end: None,
                strong_beat_bias: 0.34,
                syncopation: 0.32 + phrase.syncopation * 0.42,
            },
        );
        let frame = melodic_frame_for_bar(section, phrase, loop_handoff, &mut rng);
        let steps_len = steps.len();
        for (index, step) in steps.into_iter().enumerate() {
            let offset = frame.outward
                + if index == 0 {
                    1
                } else if index + 1 == steps_len {
                    -1
                } else {
                    random_int(&mut rng, -1, 2)
                };
            events.push(note_event(
                MusicTrack::Counter,
                bar,
                step,
                2,
                [degree_note(tonic, scale, chord_root + offset, 0)],
                MusicRole::Motion,
                0.055 * section.motion_level,
            ));
        }
    }
    Ok(())
}

pub(super) fn add_pitched_boundary(
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
    let carrier = section.roles.boundary.carrier;
    let mut rng = event_rng(seed, "boundary", carrier.as_str(), section, local_bar);
    match carrier {
        Carrier::ContrastNote => {
            let step = random_int(&mut rng, 11, 15) as u8;
            let frame = melodic_frame_for_bar(section, phrase, loop_handoff, &mut rng);
            let settled_variant = frame.settled + random_int(&mut rng, -1, 1);
            let approach = weighted_pick(
                &[
                    (melodic_target_for_bar(frame, phrase), 0.42),
                    (frame.open, 0.28),
                    (settled_variant, 0.2),
                    (frame.upper, 0.1),
                ],
                &mut rng,
            );
            events.push(note_event(
                MusicTrack::Lead,
                bar,
                step,
                2,
                [degree_note(tonic, scale, approach, 12)],
                MusicRole::Boundary,
                0.075 * section.boundary_level,
            ));
            Ok(())
        }
        Carrier::RegisterTurn => {
            let step = random_int(&mut rng, 10, 14) as u8;
            let frame = melodic_frame_for_bar(section, phrase, loop_handoff, &mut rng);
            let turn = frame.outward + weighted_pick(&[(-2, 0.24), (2, 0.38), (4, 0.38)], &mut rng);
            let duration = random_int(&mut rng, 2, 4) as u16;
            events.push(note_event(
                MusicTrack::Counter,
                bar,
                step,
                duration,
                [degree_note(tonic, scale, turn, 12)],
                MusicRole::Boundary,
                0.06 * section.boundary_level,
            ));
            Ok(())
        }
        _ => Err(format!(
            "pitched boundary generator received carrier `{}`",
            carrier.as_str()
        )),
    }
}

fn harmonic_pulse_candidates(chord: [i16; 3]) -> Vec<i16> {
    let source = [
        chord[0],
        chord[1],
        chord[2],
        chord[0] + 12,
        chord[1] + 12,
        chord[2] + 12,
        chord[0] + 2,
        chord[1] - 2,
        chord[2] + 2,
    ];
    // JavaScript's Set removes every duplicate while preserving first-seen
    // order. Vec::dedup would only remove adjacent duplicates and changes the
    // weighted field (and therefore all following RNG choices).
    let mut result = Vec::with_capacity(source.len());
    for note in source {
        if !result.contains(&note) {
            result.push(note);
        }
    }
    result
}

fn harmonic_pulse_target(chord: [i16; 3], phrase: &PhraseBar) -> i16 {
    let notes = [
        chord[0] - 12,
        chord[0],
        chord[1],
        chord[2],
        chord[0] + 12,
        chord[1] + 12,
        chord[2] + 12,
    ];
    let index = js_round(
        2.0 + phrase.target_center * 1.8 + phrase.height_bias * 1.5 + phrase.tension
            - phrase.closure * 1.2,
    )
    .clamp(0, notes.len() as i32 - 1) as usize;
    notes[index]
}

fn harmonic_targets_for_bar(phrase: &PhraseBar) -> [i32; 3] {
    let center = js_round(
        2.0 + phrase.target_center * 3.0 + phrase.height_bias * 2.0 + phrase.tension * 2.0
            - phrase.stability,
    )
    .clamp(-1, 8);
    let pull = phrase.closure * 0.68 + phrase.stability * 0.22;
    [
        js_round(interpolate(f64::from(center), 0.0, pull)).clamp(-1, 8),
        js_round(interpolate(f64::from(center + 2), 2.0, pull * 0.7)).clamp(-1, 8),
        js_round(f64::from(center) + if phrase.height_bias > 0.0 { 3.0 } else { -1.0 })
            .clamp(-1, 8),
    ]
}

#[allow(clippy::too_many_arguments)]
fn contour_for_steps(
    steps: &[u8],
    seed: &str,
    carrier: &str,
    local_bar: u8,
    section: &RealizedSection,
    phrase: &PhraseBar,
    rng: &mut Mulberry32,
    min: i32,
    max: i32,
) -> Vec<i32> {
    let source = generated_contour_motif(seed, carrier, steps.len().max(3), min, max, section);
    let memory = motif_presence(seed, carrier, local_bar, section, phrase);
    let source_max = *source.iter().max().unwrap();
    let source_min = *source.iter().min().unwrap();
    let shift = js_round(
        f64::from(section.degree_offset) * 0.28
            + phrase.target_center * 1.4
            + phrase.height_bias * 0.9
            + f64::from(weighted_pick(
                &[
                    (-1, 0.12 + phrase.stability * 0.04),
                    (0, 0.72 + phrase.stability * 0.3),
                    (1, 0.12 + phrase.pickup * 0.08 + phrase.tension * 0.04),
                ],
                rng,
            )),
    )
    .clamp(min - source_max, max - source_min);
    steps
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let base = source[index % source.len()];
            let fresh = random_int(rng, min, max);
            let breath = weighted_pick(
                &[
                    (-1, 0.05 + phrase.syncopation * 0.06 + (1.0 - memory) * 0.08),
                    (0, 0.7 + phrase.stability * 0.18 + memory * 0.2),
                    (1, 0.06 + phrase.pickup * 0.06 + (1.0 - memory) * 0.08),
                ],
                rng,
            );
            js_round(interpolate(
                f64::from(fresh),
                f64::from(base + shift + breath),
                clamp(memory + 0.08, 0.0, 1.0),
            ))
            .clamp(min, max)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn identity_accent(
    seed: &str,
    carrier: &str,
    section: &RealizedSection,
    phrase: &PhraseBar,
    local_bar: u8,
    step: u8,
    index: usize,
    count: usize,
) -> f64 {
    let cell = generated_rhythm_cell(seed, "identity", carrier, section, count as f64);
    let anchors = project_rhythm_cell(&cell, 0, 15, if phrase.pace < 0.44 { 4 } else { 2 });
    let rhythm_score = anchors
        .iter()
        .map(|&anchor| gaussian_score(f64::from(step), f64::from(anchor), 0.9))
        .fold(f64::NEG_INFINITY, f64::max);
    let position = if count <= 1 {
        0.0
    } else {
        index as f64 / (count - 1) as f64
    };
    let contour_score = cell
        .iter()
        .map(|&point| gaussian_score(position, point, 0.18))
        .fold(f64::NEG_INFINITY, f64::max);
    let memory = motif_presence(seed, carrier, local_bar, section, phrase);
    0.88 + rhythm_score * (0.12 + memory * 0.18) + contour_score * memory * 0.08
}

fn generate_melodic_line(
    seed: &str,
    local_bar: u8,
    section: &RealizedSection,
    phrase: &PhraseBar,
    loop_handoff: bool,
) -> Vec<MelodicNote> {
    let mut rng = event_rng(seed, "identity", "melodic-line", section, local_bar);
    let sparse = section.state.density < 0.32 || phrase.space > 0.6;
    let lift =
        section.state.closure_pressure > 0.66 && phrase.energy > 0.96 || phrase.tension > 0.68;
    let count_base =
        1.6 + phrase.pace * 3.1 + if lift { 0.72 } else { 0.0 } - if sparse { 0.58 } else { 0.0 };
    let count = js_round(
        (count_base + rng.uniform() * 1.2 + section.identity_level * 0.18)
            * clamp(0.72 + phrase.energy * 0.28, 0.72, 1.12),
    )
    .clamp(1, 7) as usize;
    let start = if phrase.pickup > 0.45 || phrase.tension > 0.64 {
        random_int(&mut rng, 1, 3)
    } else if sparse {
        random_int(&mut rng, 0, 4)
    } else {
        random_int(&mut rng, 0, 1)
    };
    let phrase_end_step = if phrase.boundary > 0.7 {
        random_int(&mut rng, 13, 15)
    } else if phrase.tension > 0.58 && phrase.closure < 0.58 {
        random_int(&mut rng, 11, 14)
    } else {
        random_int(&mut rng, 9, 15)
    };
    let steps = melodic_phrase_onsets(
        seed,
        local_bar,
        section,
        phrase,
        count,
        start,
        phrase_end_step,
        sparse,
    );
    let pattern = steps
        .into_iter()
        .map(|step| {
            (
                step,
                0,
                melodic_duration(
                    i32::from(step),
                    phrase_end_step,
                    sparse,
                    phrase.pace,
                    &mut rng,
                ),
            )
        })
        .collect::<Vec<_>>();
    shape_melodic_phrase(
        &pattern,
        seed,
        local_bar,
        section,
        phrase,
        loop_handoff,
        &mut rng,
    )
}

#[allow(clippy::too_many_arguments)]
fn melodic_phrase_onsets(
    seed: &str,
    local_bar: u8,
    section: &RealizedSection,
    phrase: &PhraseBar,
    count: usize,
    start: i32,
    phrase_end_step: i32,
    sparse: bool,
) -> Vec<u8> {
    let mut phrase_rng = motif_rng(seed, "identity", "melodic-line", "rhythm", section);
    let mut bar_rng = event_rng(
        seed,
        "identity-phrase-bar",
        "melodic-line",
        section,
        local_bar,
    );
    let memory = motif_presence(seed, "melodic-line", local_bar, section, phrase);
    let base_count = js_round(
        2.0 + phrase_rng.uniform() * 3.0
            + if section.state.density > 0.58 {
                1.0
            } else {
                0.0
            }
            - if sparse { 1.0 } else { 0.0 },
    )
    .clamp(1, 6) as usize;
    let base_end = js_round(12.0 + phrase_rng.uniform() * 3.0).clamp(start + 2, phrase_end_step);
    let base_steps = project_rhythm_cell(
        &generated_rhythm_cell(seed, "identity", "melodic-line", section, base_count as f64),
        start,
        base_end,
        if sparse { 4 } else { 3 },
    );
    let source_steps = rhythm_motif_source_steps(
        &base_steps,
        &mut bar_rng,
        base_count,
        start,
        base_end,
        if sparse { 4 } else { 3 },
        memory,
        if sparse { 0.28 } else { 0.5 },
        phrase.syncopation,
    );
    let motif = develop_rhythm_motif(&source_steps, phrase, local_bar, &mut bar_rng, memory, true);
    fit_rhythm_motif(
        &motif,
        count,
        start,
        phrase_end_step,
        if sparse {
            3
        } else if phrase.pace < 0.46 {
            3
        } else {
            2
        },
        &mut bar_rng,
        phrase.syncopation,
        false,
    )
}

fn motif_rng(
    seed: &str,
    role: &str,
    carrier: &str,
    layer: &str,
    section: &RealizedSection,
) -> Mulberry32 {
    Mulberry32::from_text(&format!(
        "{seed}:phrase-motif:{role}:{carrier}:{layer}:{}",
        section.motif_variant
    ))
}

fn motif_presence(
    seed: &str,
    carrier: &str,
    local_bar: u8,
    section: &RealizedSection,
    phrase: &PhraseBar,
) -> f64 {
    let mut rng = motif_rng(seed, "identity", carrier, "presence", section);
    let phase = rng.uniform() * std::f64::consts::TAU;
    let phrase_wave =
        ((f64::from(local_bar) * std::f64::consts::PI * 0.5 + phase).cos() + 1.0) * 0.5;
    let return_wave =
        ((f64::from(local_bar) * std::f64::consts::PI + phase * 0.47).cos() + 1.0) * 0.5;
    let field = 0.24
        + phrase_wave * (1.08 + section.state.stability * 0.34)
        + return_wave * 0.34
        + phrase.boundary * 0.24
        - section.state.memory_distance * 0.28
        - phrase.tension * 0.18;
    0.42 + 0.52 / (1.0 + (-field).exp())
}

#[allow(clippy::too_many_arguments)]
fn rhythm_motif_source_steps(
    base_steps: &[u8],
    rng: &mut Mulberry32,
    count: usize,
    min: i32,
    max: i32,
    min_gap: i32,
    memory: f64,
    strong_beat_bias: f64,
    syncopation: f64,
) -> Vec<i32> {
    let fresh_steps = stochastic_onsets(
        rng,
        count as f64,
        OnsetOptions {
            min,
            max,
            min_gap,
            anchor_start: false,
            anchor_end: None,
            strong_beat_bias,
            syncopation,
        },
    );
    base_steps
        .iter()
        .enumerate()
        .map(|(index, &step)| {
            let fresh = fresh_steps
                .get(index % fresh_steps.len())
                .copied()
                .unwrap_or(step);
            js_round(interpolate(f64::from(fresh), f64::from(step), memory)).clamp(min, max)
        })
        .collect()
}

fn generated_rhythm_cell(
    seed: &str,
    role: &str,
    carrier: &str,
    section: &RealizedSection,
    count_hint: f64,
) -> Vec<f64> {
    let mut rng = motif_rng(seed, role, carrier, "rhythm-cell", section);
    let count = js_round(count_hint).clamp(2, 5) as usize;
    let phase = rng.uniform() * 0.18;
    let spread = 0.68 + rng.uniform() * 0.22;
    let mut accents = Vec::with_capacity(count);
    for index in 0..count {
        let progress = if count <= 1 {
            0.0
        } else {
            index as f64 / (count - 1) as f64
        };
        let curve = progress.powf(0.82 + rng.uniform() * 0.32);
        let swing =
            (((index + 1) as f64 * 1.7 + phase * 7.0).sin()) * (0.035 + rng.uniform() * 0.035);
        accents.push(round2(clamp(phase + curve * spread + swing, 0.0, 1.0)));
    }
    accents.sort_by(total_cmp);
    accents.dedup();
    accents
}

fn project_rhythm_cell(cell: &[f64], min: i32, max: i32, min_gap: i32) -> Vec<u8> {
    let span = (max - min).max(1);
    let projected = cell
        .iter()
        .map(|&point| js_round(f64::from(min) + point * f64::from(span)).clamp(min, max));
    let mut result = Vec::with_capacity(cell.len());
    for step in projected {
        result.push(nearest_open_step(step, &result, min, max, min_gap));
    }
    result.sort_unstable();
    result.dedup();
    result.into_iter().map(|step| step as u8).collect()
}

fn nearest_open_step(step: i32, selected: &[i32], min: i32, max: i32, min_gap: i32) -> i32 {
    let mut candidates = (min..=max)
        .filter(|candidate| {
            selected
                .iter()
                .all(|other| (candidate - other).abs() >= min_gap)
        })
        .collect::<Vec<_>>();
    let Some(mut best) = candidates.first().copied() else {
        return step;
    };
    for candidate in candidates.drain(1..) {
        if (candidate - step).abs() < (best - step).abs() {
            best = candidate;
        }
    }
    best
}

fn develop_rhythm_motif(
    base_steps: &[i32],
    phrase: &PhraseBar,
    local_bar: u8,
    rng: &mut Mulberry32,
    memory: f64,
    identity_priority: bool,
) -> Vec<i32> {
    let rhythm_identity = if identity_priority { 1.0 } else { 0.0 };
    let phrase_lift = phrase.pickup * 0.42 + phrase.tension * 0.18 - phrase.stability * 0.12;
    let bar_wave = ((f64::from(local_bar) + 1.0) * 1.17 + phrase.target_center * 0.8).sin();
    let phrase_shift = weighted_pick(
        &[
            (
                -1,
                (0.08 + (-bar_wave).max(0.0) * 0.08 + (1.0 - memory) * 0.14)
                    * (1.0 - rhythm_identity * 0.72),
            ),
            (
                0,
                0.58 + phrase.stability * 0.24 + memory * 0.42 + rhythm_identity * 0.78,
            ),
            (
                1,
                (0.1 + phrase_lift * 0.12 + bar_wave.max(0.0) * 0.08 + (1.0 - memory) * 0.12)
                    * (1.0 - rhythm_identity * 0.72),
            ),
            (
                2,
                (0.02 + phrase.pickup * 0.08) * (1.0 - rhythm_identity * 0.84),
            ),
        ],
        rng,
    );
    base_steps
        .iter()
        .enumerate()
        .map(|(index, &step)| {
            let local_nudge = weighted_pick(
                &[
                    (
                        -1,
                        (0.04 + phrase.syncopation * 0.05 + (1.0 - memory) * 0.1)
                            * (1.0 - rhythm_identity * 0.76),
                    ),
                    (
                        0,
                        0.62 - phrase.syncopation * 0.08
                            + index as f64 * 0.01
                            + memory * 0.38
                            + rhythm_identity * 0.7,
                    ),
                    (
                        1,
                        (0.06 + phrase.pickup * 0.05 + (1.0 - memory) * 0.1)
                            * (1.0 - rhythm_identity * 0.76),
                    ),
                ],
                rng,
            );
            step + phrase_shift + local_nudge
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn fit_rhythm_motif(
    motif: &[i32],
    count: usize,
    min: i32,
    max: i32,
    min_gap: i32,
    rng: &mut Mulberry32,
    syncopation: f64,
    identity_priority: bool,
) -> Vec<u8> {
    let rhythm_identity = i32::from(identity_priority);
    let target_count = (count as i32).clamp(1, 7);
    let mut normalized = motif
        .iter()
        .map(|&step| step.clamp(min, max))
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.dedup();
    let mut expanded = normalized.clone();
    let mut expansion_pool = [min, 2, 4, 6, 8, 10, 12, max]
        .into_iter()
        .map(|step| {
            (step
                + weighted_pick(
                    &[
                        (-1, syncopation * 0.18),
                        (0, 0.72),
                        (1, 0.1 + syncopation * 0.16),
                    ],
                    rng,
                ))
            .clamp(min, max)
        })
        .collect::<Vec<_>>();
    expansion_pool.retain(|step| expanded.iter().all(|other| (other - step).abs() >= min_gap));
    expansion_pool.sort_by(|left, right| {
        motif_onset_weight(*right, syncopation)
            .partial_cmp(&motif_onset_weight(*left, syncopation))
            .unwrap_or(Ordering::Equal)
    });
    let additions = (1 - rhythm_identity)
        .min(target_count - expanded.len() as i32)
        .max(0) as usize;
    expanded.extend(expansion_pool.into_iter().take(additions));
    expanded.sort_unstable();
    expanded.dedup();
    let target_len = (target_count
        - rhythm_identity * (target_count - normalized.len() as i32).max(0))
    .max(1) as usize;
    expanded
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, step)| {
            // JS compares with the immediately preceding sorted element,
            // including an element rejected by this same filter.
            if index == 0 || step - expanded[index - 1] >= min_gap {
                Some(step as u8)
            } else {
                None
            }
        })
        .take(target_len)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn shape_melodic_phrase(
    pattern: &[MelodicNote],
    seed: &str,
    local_bar: u8,
    section: &RealizedSection,
    phrase: &PhraseBar,
    loop_handoff: bool,
    rng: &mut Mulberry32,
) -> Vec<MelodicNote> {
    let frame = melodic_frame_for_bar(section, phrase, loop_handoff, rng);
    let target_pitch = melodic_target_for_bar(frame, phrase);
    let count = pattern.len();
    let candidates = (frame.min..=frame.max).collect::<Vec<_>>();
    let motif_offsets = identity_contour_offsets(
        pattern.len(),
        seed,
        "melodic-line",
        local_bar,
        section,
        phrase,
        rng,
        frame.min,
        frame.max,
    );
    let offsets = sample_pitch_path(
        &candidates,
        count,
        rng,
        frame.start,
        target_pitch,
        phrase,
        section,
        &motif_offsets,
        phrase.boundary > 0.72,
        loop_handoff,
    );
    pattern
        .iter()
        .enumerate()
        .map(|(index, &(step, source_offset, duration))| {
            let is_landing = index == count - 1 || step >= 14;
            let source_detail = if is_landing {
                0
            } else {
                js_round(f64::from(source_offset - offsets[index]) * 0.18).clamp(-1, 1)
            };
            let offset = if is_landing && phrase.boundary > 0.72 {
                target_pitch
            } else {
                (offsets[index] + source_detail).clamp(frame.min, frame.max)
            };
            let next_duration = if is_landing {
                duration.max(if phrase.boundary > 0.58 { 3 } else { 2 })
            } else {
                duration.max(if phrase.pace < 0.38 { 3 } else { 2 })
            };
            (step, offset, next_duration)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn identity_contour_offsets(
    count: usize,
    seed: &str,
    carrier: &str,
    local_bar: u8,
    section: &RealizedSection,
    phrase: &PhraseBar,
    rng: &mut Mulberry32,
    min: i32,
    max: i32,
) -> Vec<i32> {
    let source = generated_contour_motif(seed, carrier, count.max(3), min, max, section);
    let memory = motif_presence(seed, carrier, local_bar, section, phrase);
    let source_max = *source.iter().max().expect("contour is non-empty");
    let source_min = *source.iter().min().expect("contour is non-empty");
    let center_shift = js_round(
        f64::from(section.degree_offset) * 0.28
            + phrase.target_center * 1.4
            + phrase.height_bias * 0.9
            + f64::from(weighted_pick(
                &[
                    (-1, 0.12 + phrase.stability * 0.04),
                    (0, 0.72 + phrase.stability * 0.3),
                    (1, 0.12 + phrase.pickup * 0.08 + phrase.tension * 0.04),
                ],
                rng,
            )),
    )
    .clamp(min - source_max, max - source_min);
    (0..count)
        .enumerate()
        .map(|(index, _)| {
            let base = source[index % source.len()];
            let fresh = random_int(rng, min, max);
            let breath = weighted_pick(
                &[
                    (-1, 0.05 + phrase.syncopation * 0.06 + (1.0 - memory) * 0.08),
                    (0, 0.7 + phrase.stability * 0.18 + memory * 0.2),
                    (1, 0.06 + phrase.pickup * 0.06 + (1.0 - memory) * 0.08),
                ],
                rng,
            );
            js_round(interpolate(
                f64::from(fresh),
                f64::from(base + center_shift + breath),
                clamp(memory + 0.08, 0.0, 1.0),
            ))
            .clamp(min, max)
        })
        .collect()
}

fn generated_contour_motif(
    seed: &str,
    carrier: &str,
    length: usize,
    min: i32,
    max: i32,
    section: &RealizedSection,
) -> Vec<i32> {
    let mut rng = motif_rng(seed, "identity", carrier, "contour", section);
    let span = (max - min).max(1);
    let start = js_round(interpolate(
        f64::from(min),
        f64::from(max),
        0.36 + rng.uniform() * 0.26,
    ))
    .clamp(min, max);
    let direction = weighted_pick(&[(-1, 0.36), (1, 0.44), (0, 0.2)], &mut rng);
    let mut contour = vec![start];
    let mut previous = start;
    let mut previous_direction = direction;
    for _ in 1..length {
        let interval = weighted_pick(
            &[
                (-3, if previous_direction < 0 { 0.18 } else { 0.08 }),
                (-2, if previous_direction < 0 { 0.26 } else { 0.12 }),
                (-1, 0.22),
                (0, 0.14),
                (1, 0.24),
                (2, if previous_direction > 0 { 0.28 } else { 0.14 }),
                (3, if previous_direction > 0 { 0.18 } else { 0.08 }),
            ],
            &mut rng,
        );
        let gravity = js_round(f64::from(start - previous) / (2.0_f64).max(f64::from(span) * 0.7));
        let next = (previous + interval + gravity).clamp(min, max);
        let direction = (next - previous).signum();
        if direction != 0 {
            previous_direction = direction;
        }
        previous = next;
        contour.push(next);
    }
    contour
}

#[derive(Clone, Copy)]
struct MelodicFrame {
    start: i32,
    settled: i32,
    open: i32,
    outward: i32,
    upper: i32,
    center: i32,
    min: i32,
    max: i32,
}

fn melodic_frame_for_bar(
    section: &RealizedSection,
    phrase: &PhraseBar,
    loop_handoff: bool,
    rng: &mut Mulberry32,
) -> MelodicFrame {
    let distance = section.state.memory_distance;
    let energy = section.energy;
    let lift =
        section.state.closure_pressure > 0.66 && phrase.energy > 1.02 || phrase.tension > 0.68;
    let sparse = section.state.density < 0.32 || phrase.space > 0.58;
    let center = js_round(
        f64::from(section.degree_offset) * 0.38
            + distance * 5.2
            + (energy - 0.5) * 2.1
            + f64::from(random_int(rng, -1, 1)),
    )
    .clamp(-2, 7);
    let spread = js_round(
        3.0 + distance * 3.4 + phrase.energy * 1.2 + if lift { 1.2 } else { 0.0 }
            - if sparse { 1.1 } else { 0.0 }
            + rng.uniform() * 1.8,
    )
    .clamp(3, 10);
    let min =
        (center - (f64::from(spread) * (0.45 + rng.uniform() * 0.18)).ceil() as i32).clamp(-4, 5);
    let max = (center + (f64::from(spread) * (0.55 + rng.uniform() * 0.22)).ceil() as i32)
        .clamp(min + 2, 11);
    let start = (center + random_int(rng, -2, 1)).clamp(min, max);
    let outward = (start
        + weighted_pick(
            &[
                (-2, if sparse { 0.28 } else { 0.12 }),
                (-1, 0.18),
                (1, 0.26),
                (2, 0.26),
                (3, if lift { 0.18 } else { 0.08 }),
            ],
            rng,
        ))
    .clamp(min, max);
    let upper_base = start.max(outward);
    let upper = (upper_base + random_int(rng, 1, (max - upper_base + 1).max(2))).clamp(min, max);
    let lower_base = start.min(outward);
    let _lower = (lower_base - random_int(rng, 1, (lower_base - min + 1).max(2))).clamp(min, max);
    let settled = (center
        + weighted_pick(
            &[
                (
                    0,
                    if loop_handoff || section.state.closure_pressure > 0.68 {
                        0.48
                    } else {
                        0.2
                    },
                ),
                (-1, 0.2),
                (1, 0.22),
                (2, if distance > 0.52 { 0.2 } else { 0.1 }),
            ],
            rng,
        ))
    .clamp(min, max);
    let open = (settled
        + weighted_pick(
            &[
                (1, 0.34),
                (2, 0.38),
                (3, if distance > 0.42 { 0.18 } else { 0.08 }),
                (-1, 0.1),
            ],
            rng,
        ))
    .clamp(min, max);
    MelodicFrame {
        start,
        settled,
        open,
        outward,
        upper,
        center,
        min,
        max,
    }
}

fn melodic_target_for_bar(frame: MelodicFrame, phrase: &PhraseBar) -> i32 {
    let span = (frame.max - frame.min).max(1);
    let raw_target = f64::from(frame.center)
        + phrase.target_center * f64::from(span) * 0.32
        + phrase.height_bias * f64::from(span) * 0.18
        + phrase.tension * 1.2
        - phrase.stability * 0.8;
    let stable_target = interpolate(raw_target, f64::from(frame.settled), phrase.closure * 0.62);
    let open_target = interpolate(
        stable_target,
        f64::from(frame.open),
        (phrase.tension - phrase.closure).max(0.0) * 0.42,
    );
    js_round(open_target).clamp(frame.min, frame.max)
}

#[allow(clippy::too_many_arguments)]
fn sample_pitch_path(
    candidates: &[i32],
    count: usize,
    rng: &mut Mulberry32,
    initial_pitch: i32,
    target_pitch: i32,
    phrase: &PhraseBar,
    section: &RealizedSection,
    motif_offsets: &[i32],
    exact_final: bool,
    loop_handoff: bool,
) -> Vec<i32> {
    let mut path = Vec::with_capacity(count);
    let mut used = Vec::with_capacity(count);
    let mut previous = nearest_candidate(candidates, initial_pitch);
    let mut previous_direction = 0;
    let distance = section.state.memory_distance;
    let local_randomness = clamp(
        0.12 + distance * 0.22 + phrase.syncopation * 0.18 + rng.uniform() * 0.14,
        0.08,
        0.58,
    );
    for index in 0..count {
        let remaining = count - index - 1;
        if exact_final && remaining == 0 {
            path.push(nearest_candidate(candidates, target_pitch));
            break;
        }
        let progress = if count <= 1 {
            1.0
        } else {
            index as f64 / (count - 1) as f64
        };
        let bridge_center = f64::from(previous)
            + f64::from(target_pitch - previous) / (remaining + 1).max(1) as f64;
        let target_pull = clamp(
            progress
                * progress
                * (0.28 + phrase.boundary * 0.42 + if loop_handoff { 0.18 } else { 0.0 }),
            0.12,
            0.92,
        );
        let weighted = candidates
            .iter()
            .map(|&pitch| {
                let bridge_score = gaussian_score(
                    f64::from(pitch),
                    bridge_center,
                    0.9 + local_randomness * 3.8,
                );
                let target_score = gaussian_score(
                    f64::from(pitch),
                    f64::from(target_pitch),
                    1.1 + (1.0 - target_pull) * 5.0,
                );
                let smooth_score = gaussian_score(
                    f64::from(pitch),
                    f64::from(previous),
                    1.0 + local_randomness * 5.5,
                );
                let motif_score = motif_offsets.get(index).map_or(0.0, |&target| {
                    gaussian_score(
                        f64::from(pitch),
                        f64::from(target),
                        0.9 + local_randomness * 2.6,
                    )
                });
                let actual_direction = (pitch - previous).signum();
                let reversal = previous_direction != 0
                    && actual_direction != 0
                    && actual_direction != previous_direction;
                let reversal_penalty = if reversal {
                    0.54 + local_randomness * 0.54
                } else {
                    1.0
                };
                let repeat_weight = if pitch == previous {
                    0.84 + local_randomness * 0.12
                } else {
                    1.0
                };
                let reuse_weight = if used.contains(&pitch) {
                    0.86 + local_randomness * 0.1
                } else {
                    1.04
                };
                let feasible = gaussian_score(
                    f64::from(pitch),
                    f64::from(target_pitch),
                    1.2_f64.max((remaining + 1) as f64 * (2.2 + local_randomness * 3.0)),
                );
                let weight = (0.04
                    + bridge_score * 0.52
                    + target_score * (0.16 + target_pull * 0.44)
                    + smooth_score * 0.28
                    + motif_score * (0.34 + phrase.stability * 0.42)
                    + feasible * 0.18
                    + rng.uniform() * local_randomness * 0.08)
                    * reversal_penalty
                    * repeat_weight
                    * reuse_weight;
                (pitch, weight.max(0.001))
            })
            .collect::<Vec<_>>();
        let next = weighted_pick(&weighted, rng);
        let direction = (next - previous).signum();
        if direction != 0 {
            previous_direction = direction;
        }
        previous = next;
        path.push(next);
        used.push(next);
    }
    path
}

fn nearest_candidate(candidates: &[i32], target: i32) -> i32 {
    let mut best = candidates[0];
    for &candidate in &candidates[1..] {
        if (candidate - target).abs() < (best - target).abs() {
            best = candidate;
        }
    }
    best
}

fn melodic_duration(
    step: i32,
    phrase_end_step: i32,
    sparse: bool,
    pace: f64,
    rng: &mut Mulberry32,
) -> u16 {
    if step >= phrase_end_step - 1 {
        return if sparse || pace < 0.38 {
            random_int(rng, 4, 7)
        } else {
            random_int(rng, 2, 5)
        } as u16;
    }
    if pace < 0.32 {
        return random_int(rng, 4, 7) as u16;
    }
    if pace < 0.5 || sparse {
        return random_int(rng, 3, 5) as u16;
    }
    random_int(rng, 2, 3) as u16
}

fn motif_onset_weight(step: i32, syncopation: f64) -> f64 {
    onset_weight(
        step,
        OnsetOptions {
            min: 0,
            max: 15,
            min_gap: 1,
            anchor_start: false,
            anchor_end: None,
            strong_beat_bias: 0.5,
            syncopation,
        },
    )
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

fn random_int(rng: &mut Mulberry32, min: i32, max: i32) -> i32 {
    min + (rng.uniform() * f64::from(max - min + 1)).floor() as i32
}

fn interpolate(left: f64, right: f64, amount: f64) -> f64 {
    left + (right - left) * clamp(amount, 0.0, 1.0)
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.min(max).max(min)
}

fn js_round(value: f64) -> i32 {
    (value + 0.5).floor() as i32
}

fn round2(value: f64) -> f64 {
    f64::from(js_round(value * 100.0)) / 100.0
}

fn total_cmp(left: &f64, right: &f64) -> Ordering {
    left.total_cmp(right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MusicNote,
        music::composition::{
            form::{CompositionRole, CompositionRoles},
            section::SectionVector,
        },
    };

    #[test]
    fn same_seed_first_bar_matches_javascript_melodic_events() {
        let section = RealizedSection {
            index: 0,
            variant: 0,
            motif_variant: 9_774,
            roles: CompositionRoles {
                identity: CompositionRole {
                    name: MusicRole::Identity,
                    carrier: Carrier::MelodicLine,
                },
                time: CompositionRole {
                    name: MusicRole::Time,
                    carrier: Carrier::BassPulse,
                },
                tone: CompositionRole {
                    name: MusicRole::Tone,
                    carrier: Carrier::ChordPad,
                },
                motion: CompositionRole {
                    name: MusicRole::Motion,
                    carrier: Carrier::HarmonyArp,
                },
                color: CompositionRole {
                    name: MusicRole::Color,
                    carrier: Carrier::NoiseHalo,
                },
                boundary: CompositionRole {
                    name: MusicRole::Boundary,
                    carrier: Carrier::RestGap,
                },
            },
            degree_offset: 0,
            progression_shift: 0,
            state: SectionVector {
                progress: 0.0,
                novelty: 0.08,
                stability: 0.88,
                density: 0.42,
                tension: 0.22,
                closure_pressure: 0.86,
                memory_distance: 0.06,
            },
            energy: 0.69,
            identity_level: 1.1,
            motion_level: 0.96,
            color_level: 0.73,
            boundary_level: 1.05,
        };
        let phrase = PhraseBar {
            index: 0,
            target_center: -0.29,
            height_bias: -0.1,
            closure: 0.43,
            tension: 0.14,
            stability: 0.73,
            pace: 0.31,
            energy: 0.98,
            space: 0.56,
            boundary: 0.0,
            pickup: 0.0,
            tone_anchor: true,
            color_accent: false,
            syncopation: 0.69,
            transition_in: None,
            transition_out: None,
            transition_entry_bridge: None,
            transition_bridge: None,
        };
        let mut events = Vec::new();
        let scale = [0, 2, 3, 5, 7, 9, 10];
        add_melodic_identity(
            &mut events,
            &BarRenderContext {
                seed: "same-seed",
                section: &section,
                phrase: &phrase,
                tonic: 62,
                scale: &scale,
                chord_root: 0,
                chord: [62, 65, 69],
                bar: 0,
                local_bar: 0,
                loop_handoff: true,
            },
        )
        .unwrap();

        assert_eq!(events.len(), 3);
        assert_eq!(
            events
                .iter()
                .map(|event| (
                    event.step,
                    event.duration_steps,
                    event.notes.clone(),
                    event.track,
                    event.role,
                ))
                .collect::<Vec<_>>(),
            [
                (
                    3,
                    7,
                    vec![MusicNote::Midi(53)],
                    MusicTrack::Lead,
                    MusicRole::Identity,
                ),
                (
                    6,
                    4,
                    vec![MusicNote::Midi(59)],
                    MusicTrack::Lead,
                    MusicRole::Identity,
                ),
                (
                    10,
                    5,
                    vec![MusicNote::Midi(52)],
                    MusicTrack::Lead,
                    MusicRole::Identity,
                ),
            ]
        );
        let expected_velocities = [
            0.152_144_147_930_198_34,
            0.153_951_108_236_436_04,
            0.133_225_805_450_312_65,
        ];
        for (event, expected) in events.iter().zip(expected_velocities) {
            assert!((event.velocity - expected).abs() < 1e-12);
        }
    }
}
