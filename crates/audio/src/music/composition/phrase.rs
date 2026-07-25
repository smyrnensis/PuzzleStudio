use crate::{MusicRole, MusicTrack, prng::Mulberry32};

use super::{
    BarState, PhraseBar, PhraseShape, TransitionBridge, TransitionContext,
    form::{Carrier, CompositionRoles},
    section::{RealizedSection, SectionVector},
};

const TRANSITION_BRIDGE_IMPACT: f64 = 0.22;
const TRANSITION_ENVELOPE_IMPACT: f64 = 0.46;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct PhraseContext {
    pub section_energy: f64,
    pub previous_section_energy: Option<f64>,
    pub next_section_energy: Option<f64>,
    pub transition_in: Option<TransitionContext>,
    pub transition_out: Option<TransitionContext>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PhraseControl {
    index: usize,
    value: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct PhraseEnergyCurve {
    archetype: String,
    controls: Vec<PhraseControl>,
    peak_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PhraseBarState {
    target_center: f64,
    height_bias: f64,
    closure: f64,
    tension: f64,
    stability: f64,
    pace: f64,
}

pub(super) fn build_bar_state_trajectory(
    section_plan: &[RealizedSection],
    bars: u16,
) -> Result<Vec<BarState>, String> {
    if section_plan.is_empty() || bars == 0 {
        return Ok(Vec::new());
    }
    let mut trajectory = Vec::with_capacity(usize::from(bars));
    for (plan_index, section) in section_plan.iter().enumerate() {
        let has_loop_neighbor = section_plan.len() > 1;
        let previous = if plan_index > 0 {
            section_plan.get(plan_index - 1)
        } else if has_loop_neighbor {
            section_plan.last()
        } else {
            None
        };
        let next = if plan_index + 1 < section_plan.len() {
            section_plan.get(plan_index + 1)
        } else if has_loop_neighbor {
            section_plan.first()
        } else {
            None
        };
        let transition_in = previous
            .map(|previous| transition_context(previous, section))
            .transpose()?
            .flatten();
        let transition_out = next
            .map(|next| transition_context(section, next))
            .transpose()?
            .flatten();
        let phrase_shape = build_phrase_shape(
            section.index,
            section.variant,
            section.degree_offset,
            section.progression_shift,
            section.motif_variant,
            section.state,
            PhraseContext {
                section_energy: section.energy,
                previous_section_energy: previous.map(|value| value.energy),
                next_section_energy: next.map(|value| value.energy),
                transition_in,
                transition_out,
            },
        );
        for phrase_bar in phrase_shape.bars {
            if trajectory.len() >= usize::from(bars) {
                return Ok(trajectory);
            }
            let local_bar = phrase_bar.index;
            trajectory.push(BarState {
                bar: (plan_index * 8 + usize::from(local_bar)) as u16,
                section_index: section.index,
                local_bar,
                phrase_archetype: phrase_shape.archetype.clone(),
                phrase_bar: with_transition_projection(
                    phrase_bar,
                    previous,
                    section,
                    next,
                    transition_in,
                    transition_out,
                )?,
                transition_in,
                transition_out,
            });
        }
    }
    Ok(trajectory)
}

pub(super) fn build_phrase_shape(
    index: usize,
    variant: i32,
    degree_offset: i32,
    progression_shift: i32,
    motif_variant: u16,
    state: SectionVector,
    context: PhraseContext,
) -> PhraseShape {
    let seed = [
        index.to_string(),
        variant.to_string(),
        degree_offset.to_string(),
        progression_shift.to_string(),
        motif_variant.to_string(),
        js_number(state.novelty),
        js_number(state.stability),
        js_number(state.density),
        js_number(state.tension),
        js_number(state.closure_pressure),
        js_number(state.memory_distance),
    ]
    .join(":");
    let mut rng = Mulberry32::from_text(&seed);
    let curve = build_phrase_energy_curve(&mut rng, state, variant);
    let raw_energies = (0..8)
        .map(|bar| phrase_energy_at(bar, &curve, &mut rng))
        .collect::<Vec<_>>();
    let energies = raw_energies
        .into_iter()
        .enumerate()
        .map(|(bar, energy)| contextual_phrase_energy(energy, context, bar))
        .collect::<Vec<_>>();

    let mut bars = Vec::with_capacity(8);
    let mut previous_target = 0.0;
    for bar in 0..8 {
        let energy = energies[bar];
        let next_energy = energies[(bar + 1).min(7)];
        let previous_energy = energies[bar.saturating_sub(1)];
        let slope_in = energy - previous_energy;
        let slope_out = next_energy - energy;
        let bar_state = phrase_bar_state(bar, energy, slope_out, previous_target, state, &mut rng);
        let entry_progress = transition_entry_progress(context, bar);
        let boundary =
            phrase_boundary_for_bar(bar, bar_state, slope_in, slope_out, energy, &mut rng)
                * entry_progress;
        let pickup =
            phrase_pickup_for_bar(bar, energy, slope_out, boundary, &mut rng) * entry_progress;
        let space = phrase_space_for_bar(energy, bar_state, &mut rng);
        bars.push(PhraseBar {
            index: bar as u8,
            target_center: round2(bar_state.target_center),
            height_bias: round2(bar_state.height_bias),
            closure: round2(bar_state.closure),
            tension: round2(bar_state.tension),
            stability: round2(bar_state.stability),
            pace: round2(bar_state.pace),
            energy: round2(energy),
            space: round2(space),
            boundary: round2(boundary),
            pickup: round2(pickup),
            tone_anchor: phrase_tone_anchor(bar, bar_state, energy, boundary, &mut rng),
            color_accent: phrase_color_accent(bar, energy, pickup, space, &mut rng),
            syncopation: round2(clamp(
                0.18 + rng.uniform() * 0.5 + pickup * 0.25 + slope_out.abs() * 0.18,
                0.16,
                0.84,
            )),
            transition_in: None,
            transition_out: None,
            transition_entry_bridge: None,
            transition_bridge: None,
        });
        previous_target = bar_state.target_center;
    }

    PhraseShape {
        archetype: curve.archetype,
        bars,
    }
}

fn transition_entry_progress(context: PhraseContext, bar_index: usize) -> f64 {
    let incoming_impact = context.transition_in.map_or(0.0, |value| value.impact);
    if incoming_impact < TRANSITION_ENVELOPE_IMPACT {
        return 1.0;
    }
    let bars = transition_span(incoming_impact);
    if bar_index >= usize::from(bars) {
        return 1.0;
    }
    smoothstep((bar_index + 1) as f64 / f64::from(bars + 1))
}

fn transition_exit_progress(context: PhraseContext, bar_index: usize) -> f64 {
    let outgoing_impact = context.transition_out.map_or(0.0, |value| value.impact);
    if outgoing_impact < TRANSITION_ENVELOPE_IMPACT {
        return 0.0;
    }
    let bars = transition_span(outgoing_impact);
    let start = 8 - usize::from(bars);
    if bar_index < start {
        return 0.0;
    }
    smoothstep((bar_index - start + 1) as f64 / f64::from(bars + 1))
}

fn contextual_phrase_energy(energy: f64, context: PhraseContext, bar_index: usize) -> f64 {
    let entry = transition_entry_progress(context, bar_index);
    let exit = transition_exit_progress(context, bar_index);
    let entry_scale = interpolate(
        section_energy_ratio(context.previous_section_energy, context.section_energy),
        1.0,
        entry,
    );
    let exit_scale = interpolate(
        1.0,
        section_energy_ratio(context.next_section_energy, context.section_energy),
        exit,
    );
    clamp(energy * entry_scale * exit_scale, 0.48, 1.28)
}

fn section_energy_ratio(reference_energy: Option<f64>, section_energy: f64) -> f64 {
    let Some(reference) = reference_energy else {
        return 1.0;
    };
    if !reference.is_finite() || !section_energy.is_finite() || section_energy <= 0.0 {
        return 1.0;
    }
    clamp((0.42 + reference) / (0.42 + section_energy), 0.78, 1.12)
}

fn smoothstep(value: f64) -> f64 {
    let x = clamp(value, 0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn build_phrase_energy_curve(
    rng: &mut Mulberry32,
    state: SectionVector,
    variant: i32,
) -> PhraseEnergyCurve {
    let first_pivot = random_int(rng, 1, 3);
    let second_pivot = random_int(rng, 4, 6);
    let late_pressure = state.closure_pressure * 0.68 + state.stability * 0.18;
    let spread_pressure = state.memory_distance * 0.56 + state.tension * 0.28;
    let peak_candidates = (1..=7)
        .map(|candidate| {
            (
                candidate,
                0.08 + gaussian_score(candidate as f64, 2.0 + spread_pressure * 3.5, 1.8)
                    * (1.0 - late_pressure)
                    + gaussian_score(candidate as f64, 5.8, 1.4) * late_pressure,
            )
        })
        .collect::<Vec<_>>();
    let peak_index = weighted_pick(&peak_candidates, rng);
    let valley_candidates = (1..=6)
        .map(|candidate| {
            (
                candidate,
                0.08 + gaussian_score(
                    candidate as f64,
                    2.4 + state.stability * 2.8 + (1.0 - state.density) * 1.4,
                    1.9,
                ),
            )
        })
        .collect::<Vec<_>>();
    let valley_index = weighted_pick(&valley_candidates, rng);
    let start = clamp(
        0.7 + state.stability * 0.22 + state.density * 0.24 + rng.uniform() * 0.26,
        0.56,
        1.24,
    );
    let end = clamp(
        0.68 + state.closure_pressure * 0.38 + state.tension * 0.16 + rng.uniform() * 0.26,
        0.56,
        1.36,
    );
    let peak = clamp(
        start.max(end) + 0.1 + state.tension * 0.2 + state.density * 0.1 + rng.uniform() * 0.2,
        0.82,
        1.38,
    );
    let valley = clamp(
        start.min(end)
            - 0.08
            - (1.0 - state.density) * 0.18
            - state.stability * 0.06
            - rng.uniform() * 0.16,
        0.44,
        1.08,
    );
    let mid_a = clamp(
        start + (rng.uniform() - 0.44) * 0.34 + f64::from(variant) * 0.025,
        0.56,
        1.28,
    );
    let mid_b = clamp(
        end + (rng.uniform() - 0.5) * 0.36 - (1.0 - state.density) * 0.08,
        0.52,
        1.3,
    );
    let controls = merge_phrase_controls([
        PhraseControl {
            index: 0,
            value: start,
        },
        PhraseControl {
            index: first_pivot,
            value: mid_a,
        },
        PhraseControl {
            index: valley_index,
            value: valley,
        },
        PhraseControl {
            index: peak_index,
            value: peak,
        },
        PhraseControl {
            index: second_pivot,
            value: mid_b,
        },
        PhraseControl {
            index: 7,
            value: end,
        },
    ]);
    PhraseEnergyCurve {
        archetype: format!(
            "state-{peak_index}-{valley_index}-{}-{}",
            js_round(state.novelty * 10.0),
            js_round(state.closure_pressure * 10.0)
        ),
        controls,
        peak_index,
    }
}

fn merge_phrase_controls(points: impl IntoIterator<Item = PhraseControl>) -> Vec<PhraseControl> {
    let mut merged = Vec::<PhraseControl>::new();
    for point in points {
        if let Some(previous) = merged.iter_mut().find(|value| value.index == point.index) {
            if (point.value - 1.0).abs() > (previous.value - 1.0).abs() {
                *previous = point;
            }
        } else {
            merged.push(point);
        }
    }
    merged.sort_by_key(|point| point.index);
    merged
}

fn phrase_energy_at(index: usize, curve: &PhraseEnergyCurve, rng: &mut Mulberry32) -> f64 {
    let mut left = curve.controls[0];
    let mut right = *curve
        .controls
        .last()
        .expect("phrase controls are non-empty");
    for controls in curve.controls.windows(2) {
        let a = controls[0];
        let b = controls[1];
        if index >= a.index && index <= b.index {
            left = a;
            right = b;
            break;
        }
    }
    let span = (right.index - left.index).max(1);
    let x = (index - left.index) as f64 / span as f64;
    let eased = x * x * (3.0 - 2.0 * x);
    let wave = (((index + 1) as f64 * 0.85) + curve.peak_index as f64).sin() * 0.04;
    let jitter = (rng.uniform() - 0.5) * 0.08;
    clamp(
        interpolate(left.value, right.value, eased) + wave + jitter,
        0.5,
        1.36,
    )
}

fn phrase_bar_state(
    index: usize,
    energy: f64,
    slope_out: f64,
    previous_target: f64,
    state: SectionVector,
    rng: &mut Mulberry32,
) -> PhraseBarState {
    let rising = slope_out > 0.06;
    let falling = slope_out < -0.06;
    let local_progress = index as f64 / 7.0;
    let closure = clamp(
        state.closure_pressure * (0.42 + local_progress * 0.68)
            + if index == 7 { 0.42 } else { 0.0 }
            + if falling { 0.12 } else { 0.0 }
            + (rng.uniform() - 0.5) * 0.14,
        0.0,
        1.0,
    );
    let tension = clamp(
        state.tension * 0.72
            + state.novelty * 0.2
            + if rising { 0.18 } else { 0.0 }
            + (energy - 1.0).max(0.0) * 0.22
            - closure * 0.18
            + (rng.uniform() - 0.5) * 0.16,
        0.0,
        1.0,
    );
    let stability = clamp(
        state.stability * 0.72 + closure * 0.26 - tension * 0.18 + (rng.uniform() - 0.5) * 0.12,
        0.0,
        1.0,
    );
    let drift = clamp(
        previous_target * 0.42
            + (state.memory_distance - 0.5) * 0.8
            + slope_out * 1.8
            + (rng.uniform() - 0.5) * (0.8 + state.novelty),
        -1.0,
        1.0,
    );
    let target_center = clamp(
        drift * (1.0 - closure * 0.42) - stability * 0.24 + tension * 0.28,
        -1.0,
        1.0,
    );
    let height_bias = clamp(
        (energy - 0.94) * 0.72 + tension * 0.36 - stability * 0.18 + (rng.uniform() - 0.5) * 0.28,
        -1.0,
        1.0,
    );
    let pace = clamp(
        0.18 + state.density * 0.46 + tension * 0.26 - stability * 0.16
            + (energy - 1.0).max(0.0) * 0.14
            + (rng.uniform() - 0.5) * 0.16,
        0.08,
        0.92,
    );
    PhraseBarState {
        target_center,
        height_bias,
        closure,
        tension,
        stability,
        pace,
    }
}

fn phrase_boundary_for_bar(
    index: usize,
    bar_state: PhraseBarState,
    slope_in: f64,
    slope_out: f64,
    energy: f64,
    rng: &mut Mulberry32,
) -> f64 {
    if index == 7 {
        return 1.0;
    }
    let turns = js_sign(slope_in) != js_sign(slope_out) && (slope_in - slope_out).abs() > 0.08;
    let probability = 0.06
        + bar_state.closure * 0.44
        + if turns { 0.18 } else { 0.0 }
        + if energy > 1.12 { 0.08 } else { 0.0 };
    if rng.uniform() < probability {
        clamp(
            0.42 + rng.uniform() * 0.28 + bar_state.closure * 0.34,
            0.0,
            0.92,
        )
    } else {
        0.0
    }
}

fn phrase_pickup_for_bar(
    index: usize,
    energy: f64,
    slope_out: f64,
    boundary: f64,
    rng: &mut Mulberry32,
) -> f64 {
    if index >= 7 {
        return 0.0;
    }
    let lift = slope_out.max(0.0);
    let boundary_push = if boundary > 0.58 { 0.12 } else { 0.0 };
    let base = if lift > 0.05 {
        0.2 + lift * 0.82
    } else if rng.uniform() < 0.12 {
        0.18
    } else {
        0.0
    };
    clamp(
        base + boundary_push + if energy > 1.12 { 0.06 } else { 0.0 },
        0.0,
        0.74,
    )
}

fn phrase_space_for_bar(energy: f64, bar_state: PhraseBarState, rng: &mut Mulberry32) -> f64 {
    let low_energy_space = clamp(0.38 - energy * 0.18, 0.0, 0.28);
    let instability_space = clamp(
        (1.0 - bar_state.stability) * 0.2 + (1.0 - bar_state.tension) * 0.08,
        0.0,
        0.28,
    );
    let closure_space = if bar_state.closure > 0.7 { 0.08 } else { 0.0 };
    clamp(
        0.1 + low_energy_space + instability_space + closure_space + rng.uniform() * 0.22,
        0.08,
        0.78,
    )
}

fn phrase_tone_anchor(
    index: usize,
    bar_state: PhraseBarState,
    energy: f64,
    boundary: f64,
    rng: &mut Mulberry32,
) -> bool {
    if index == 0 || index == 7 || bar_state.stability > 0.66 || bar_state.closure > 0.68 {
        return true;
    }
    rng.uniform()
        < 0.2 + if energy < 0.92 { 0.18 } else { 0.0 } + if boundary > 0.58 { 0.18 } else { 0.0 }
}

fn phrase_color_accent(
    index: usize,
    energy: f64,
    pickup: f64,
    space: f64,
    rng: &mut Mulberry32,
) -> bool {
    if index == 7 {
        return rng.uniform() < 0.42;
    }
    rng.uniform()
        < 0.14
            + if energy > 1.04 { 0.22 } else { 0.0 }
            + pickup * 0.18
            + if space > 0.48 { 0.12 } else { 0.0 }
}

fn transition_span(impact: f64) -> u8 {
    if impact > 0.72 { 3 } else { 2 }
}

fn with_transition_projection(
    mut phrase_bar: PhraseBar,
    previous: Option<&RealizedSection>,
    section: &RealizedSection,
    next: Option<&RealizedSection>,
    transition_in: Option<TransitionContext>,
    transition_out: Option<TransitionContext>,
) -> Result<PhraseBar, String> {
    phrase_bar.transition_in = transition_in;
    phrase_bar.transition_out = transition_out;
    phrase_bar.transition_entry_bridge = match (previous, transition_in) {
        (Some(previous), Some(transition)) => {
            Some(transition_bridge(previous, section, transition)?)
        }
        _ => None,
    };
    phrase_bar.transition_bridge = match (next, transition_out) {
        (Some(next), Some(transition)) => Some(transition_bridge(section, next, transition)?),
        _ => None,
    };
    Ok(phrase_bar)
}

fn transition_bridge(
    left: &RealizedSection,
    right: &RealizedSection,
    transition: TransitionContext,
) -> Result<TransitionBridge, String> {
    let continuity = composition_roles()
        .into_iter()
        .find_map(|role| {
            let left_carrier = carrier_for_role(left.roles, role);
            let right_carrier = carrier_for_role(right.roles, role);
            (left_carrier == right_carrier && is_continuity_carrier(role, left_carrier))
                .then_some((role, right_carrier))
        })
        .ok_or_else(|| {
            format!(
                "transition section-{}->section-{} has no continuity carrier",
                left.index, right.index
            )
        })?;
    let tracks = tracks_for_carrier(continuity.1);
    let track = tracks.first().copied().ok_or_else(|| {
        format!(
            "transition section-{}->section-{} continuity carrier {:?} has no playback track",
            left.index, right.index, continuity.1
        )
    })?;
    Ok(TransitionBridge {
        role: continuity.0,
        carrier: continuity.1,
        track,
        target_degree_offset: right.degree_offset,
        target_progression_shift: right.progression_shift,
        impact: transition.impact,
        bars: transition.bars,
    })
}

fn transition_context(
    left: &RealizedSection,
    right: &RealizedSection,
) -> Result<Option<TransitionContext>, String> {
    let impact = section_transition_impact(left, right);
    if impact < TRANSITION_BRIDGE_IMPACT {
        return Ok(None);
    }
    if !impact.is_finite() {
        return Err(format!(
            "transition section-{}->section-{} produced a non-finite impact",
            left.index, right.index
        ));
    }
    Ok(Some(TransitionContext {
        impact: round2(impact),
        bars: transition_span(impact),
    }))
}

fn section_transition_impact(left: &RealizedSection, right: &RealizedSection) -> f64 {
    let carrier_change = carrier_change_ratio(left, right);
    let density_shift = (right.state.density - left.state.density).abs();
    let energy_shift = (right.energy - left.energy).abs();
    let novelty_lift = (right.state.novelty - left.state.novelty).max(0.0);
    let distance_lift = (right.state.memory_distance - left.state.memory_distance).max(0.0);
    let closure_lift = (right.state.closure_pressure - left.state.closure_pressure).max(0.0);
    let foreground_lift = if carrier_change > 0.0 { 0.1 } else { 0.0 };
    clamp(
        carrier_change * 0.34
            + density_shift * 0.22
            + energy_shift * 0.38
            + novelty_lift * 0.18
            + distance_lift * 0.18
            + closure_lift * 0.1
            + foreground_lift,
        0.0,
        1.0,
    )
}

fn carrier_change_ratio(left: &RealizedSection, right: &RealizedSection) -> f64 {
    let changed = composition_roles()
        .into_iter()
        .filter(|&role| carrier_for_role(left.roles, role) != carrier_for_role(right.roles, role))
        .count();
    changed as f64 / composition_roles().len() as f64
}

fn composition_roles() -> [MusicRole; 6] {
    [
        MusicRole::Identity,
        MusicRole::Time,
        MusicRole::Tone,
        MusicRole::Motion,
        MusicRole::Color,
        MusicRole::Boundary,
    ]
}

fn carrier_for_role(roles: CompositionRoles, role: MusicRole) -> Carrier {
    match role {
        MusicRole::Identity => roles.identity.carrier,
        MusicRole::Time => roles.time.carrier,
        MusicRole::Tone => roles.tone.carrier,
        MusicRole::Motion => roles.motion.carrier,
        MusicRole::Color => roles.color.carrier,
        MusicRole::Boundary => roles.boundary.carrier,
    }
}

fn is_continuity_carrier(role: MusicRole, carrier: Carrier) -> bool {
    match role {
        MusicRole::Identity | MusicRole::Time => carrier != Carrier::None,
        MusicRole::Tone => carrier != Carrier::Implied && carrier != Carrier::None,
        MusicRole::Motion => matches!(
            carrier,
            Carrier::AnswerLine | Carrier::HarmonyArp | Carrier::BassWalk
        ),
        MusicRole::Color => matches!(
            carrier,
            Carrier::AirPad | Carrier::NoiseHalo | Carrier::OrganBed
        ),
        MusicRole::Boundary => false,
    }
}

fn tracks_for_carrier(carrier: Carrier) -> &'static [MusicTrack] {
    use Carrier::*;
    match carrier {
        BassRiff | BassPulse | RootBass | BassWalk => &[MusicTrack::Bass],
        ArpPulse | HarmonyArp | ChordPad | Drone | AirPad | OrganBed => &[MusicTrack::Chord],
        RhythmHook | DrumGrid | ThinPulse | DrumFill => &[MusicTrack::Drums],
        AnswerLine | RegisterTurn => &[MusicTrack::Counter],
        None | Implied | RestGap => &[],
        MelodicLine | PercussionFill | NoiseHalo | BrightAccent | ContrastNote => {
            &[MusicTrack::Lead]
        }
    }
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

fn random_int(rng: &mut Mulberry32, min: usize, max: usize) -> usize {
    min + (rng.uniform() * (max - min + 1) as f64).floor() as usize
}

fn interpolate(left: f64, right: f64, progress: f64) -> f64 {
    left + (right - left) * progress
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max)
}

fn js_sign(value: f64) -> i8 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

fn round2(value: f64) -> f64 {
    js_round(value * 100.0) as f64 / 100.0
}

fn js_round(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

fn js_number(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrase_shape_matches_javascript_transition_vector() {
        let state = SectionVector {
            progress: 0.43,
            novelty: 0.61,
            stability: 0.34,
            density: 0.72,
            tension: 0.66,
            closure_pressure: 0.28,
            memory_distance: 0.57,
        };
        let shape = build_phrase_shape(
            3,
            2,
            -1,
            3,
            4172,
            state,
            PhraseContext {
                section_energy: 0.73,
                previous_section_energy: Some(0.42),
                next_section_energy: Some(0.88),
                transition_in: Some(TransitionContext {
                    impact: 0.67,
                    bars: 2,
                }),
                transition_out: Some(TransitionContext {
                    impact: 0.81,
                    bars: 3,
                }),
            },
        );
        assert_eq!(shape.archetype, "state-4-2-6-3");
        let expected = [
            (
                1.0, 0.3, 0.07, 0.83, 0.13, 0.71, 0.86, 0.68, 0.0, 0.09, true, false, 0.28,
            ),
            (
                0.63, 0.16, 0.26, 0.5, 0.23, 0.59, 1.02, 0.53, 0.0, 0.0, true, false, 0.24,
            ),
            (
                0.9, 0.09, 0.23, 0.79, 0.14, 0.7, 0.78, 0.71, 0.0, 0.44, false, true, 0.75,
            ),
            (
                0.28, 0.3, 0.22, 0.78, 0.18, 0.66, 1.08, 0.64, 0.0, 0.37, false, false, 0.69,
            ),
            (
                0.0, 0.43, 0.38, 0.54, 0.27, 0.73, 1.28, 0.6, 0.0, 0.06, false, true, 0.47,
            ),
            (
                0.37, 0.29, 0.24, 0.82, 0.16, 0.71, 1.08, 0.51, 0.71, 0.38, false, false, 0.61,
            ),
            (
                0.06, 0.49, 0.3, 0.77, 0.16, 0.65, 1.15, 0.65, 0.0, 0.33, false, true, 0.54,
            ),
            (
                0.2, 0.47, 0.78, 0.53, 0.3, 0.62, 1.24, 0.72, 1.0, 0.0, true, false, 0.46,
            ),
        ];
        for (bar, expected) in shape.bars.iter().zip(expected) {
            let actual_numbers = [
                bar.target_center,
                bar.height_bias,
                bar.closure,
                bar.tension,
                bar.stability,
                bar.pace,
                bar.energy,
                bar.space,
                bar.boundary,
                bar.pickup,
                bar.syncopation,
            ];
            let expected_numbers = [
                expected.0,
                expected.1,
                expected.2,
                expected.3,
                expected.4,
                expected.5,
                expected.6,
                expected.7,
                expected.8,
                expected.9,
                expected.12,
            ];
            assert_eq!(actual_numbers, expected_numbers);
            assert_eq!(
                (bar.tone_anchor, bar.color_accent),
                (expected.10, expected.11)
            );
        }
    }
}
