use crate::prng::Mulberry32;

use super::form::{CompositionForm, CompositionRoles};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SectionVector {
    pub progress: f64,
    pub novelty: f64,
    pub stability: f64,
    pub density: f64,
    pub tension: f64,
    pub closure_pressure: f64,
    pub memory_distance: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RealizedSection {
    pub index: usize,
    pub variant: i32,
    pub motif_variant: u16,
    pub roles: CompositionRoles,
    pub degree_offset: i32,
    pub progression_shift: i32,
    pub state: SectionVector,
    pub energy: f64,
    pub identity_level: f64,
    pub motion_level: f64,
    pub color_level: f64,
    pub boundary_level: f64,
}

pub(super) fn realize_section(
    state: SectionVector,
    index: usize,
    rng: &mut Mulberry32,
    roles: CompositionRoles,
    motif_variant: u16,
) -> RealizedSection {
    let variant = variant_for_state(state, rng);
    let degree_offset = degree_offset_for_state(state, rng);
    let progression_shift = progression_shift_for_state(state, rng);
    let energy = clamp(
        0.36 + state.density * 0.34 + state.tension * 0.24 + state.closure_pressure * 0.16,
        0.16,
        0.96,
    );
    RealizedSection {
        index,
        variant,
        motif_variant,
        roles,
        degree_offset,
        progression_shift,
        state,
        energy: round2(energy),
        identity_level: round2(clamp(
            0.76 + energy * 0.34 + state.stability * 0.12
                - if state.density < 0.28 { 0.16 } else { 0.0 }
                + state.progress * 0.01,
            0.62,
            1.28,
        )),
        motion_level: round2(clamp(
            0.66 + energy * 0.48 + state.memory_distance * 0.18 + state.tension * 0.12
                - state.closure_pressure * 0.08,
            0.56,
            1.36,
        )),
        color_level: round2(clamp(
            0.52 + state.density * 0.48 + state.novelty * 0.12,
            0.44,
            1.24,
        )),
        boundary_level: round2(clamp(
            0.68 + state.closure_pressure * 0.42 + state.memory_distance * 0.18,
            0.7,
            1.34,
        )),
    }
}

fn variant_for_state(state: SectionVector, rng: &mut Mulberry32) -> i32 {
    if state.progress == 0.0 {
        0
    } else if state.closure_pressure > 0.72 {
        weighted_pick(&[(0, 0.28), (1, 0.24), (3, 0.48)], rng)
    } else if state.density < 0.32 {
        weighted_pick(&[(0, 0.36), (3, 0.42), (1, 0.22)], rng)
    } else if state.memory_distance > 0.5 {
        [2, 3][(rng.uniform() * 2.0).floor() as usize]
    } else {
        [1, 2][(rng.uniform() * 2.0).floor() as usize]
    }
}

fn degree_offset_for_state(state: SectionVector, rng: &mut Mulberry32) -> i32 {
    if state.progress == 0.0 {
        return 0;
    }
    let center = ((state.novelty - state.stability) * 2.2 + state.tension * 1.4
        - state.closure_pressure * 0.8)
        .round() as i32;
    let width = if state.memory_distance > 0.55 { 3 } else { 2 };
    (center + random_int_signed(rng, -width, width)).clamp(-2, 3)
}

fn progression_shift_for_state(state: SectionVector, rng: &mut Mulberry32) -> i32 {
    if state.progress == 0.0 {
        return 0;
    }
    let pull_home = state.closure_pressure > 0.68 || state.stability > 0.72;
    weighted_pick(
        &[
            (0, if pull_home { 0.34 } else { 0.08 }),
            (1, 0.18 + state.novelty * 0.16),
            (2, 0.22 + state.tension * 0.18),
            (3, 0.16 + state.memory_distance * 0.24),
        ],
        rng,
    )
}

pub(super) fn build_state_trajectory(
    rng: &mut Mulberry32,
    form: CompositionForm,
    section_count: usize,
) -> Vec<SectionVector> {
    if section_count <= 1 {
        return vec![section_vector(0.0, 0.08, 0.88, 0.42, 0.22, 0.86, 0.06)];
    }
    let mut trajectory: Vec<SectionVector> = Vec::with_capacity(section_count);
    for index in 0..section_count {
        let progress = index as f64 / (section_count - 1) as f64;
        let previous = trajectory.last().copied();
        let middle_lift = (progress * std::f64::consts::PI).sin();
        let novelty = if index == 0 {
            0.08
        } else {
            clamp(
                0.1 + form.contrast * 0.28
                    + progress * 0.23
                    + middle_lift * form.contrast * 0.12
                    + (rng.uniform() - 0.5) * 0.34,
                0.02,
                0.92,
            )
        };
        let closure = if index == section_count - 1 {
            clamp(0.78 + rng.uniform() * 0.18, 0.0, 1.0)
        } else {
            clamp(
                0.16 + progress * progress * 0.42 + (rng.uniform() - 0.5) * 0.24,
                0.04,
                0.78,
            )
        };
        let memory = previous.map_or(0.06, |previous| {
            clamp(
                previous.memory_distance * (0.48 + rng.uniform() * 0.22)
                    + novelty * 0.62
                    + middle_lift * form.contrast * 0.08
                    - closure * 0.2,
                0.0,
                0.96,
            )
        });
        let tension = clamp(
            0.16 + novelty * 0.5
                + memory * 0.26
                + progress * 0.14
                + middle_lift * form.contrast * 0.06
                - closure * 0.22
                + (rng.uniform() - 0.5) * 0.22,
            0.04,
            0.94,
        );
        let density = clamp(
            form.density * 0.34 + 0.28 + tension * 0.22 + novelty * 0.16 - form.space * 0.14
                + (rng.uniform() - 0.5) * 0.18,
            0.12,
            0.92,
        );
        let stability = clamp(
            0.9 - novelty * 0.42 - memory * 0.22 - tension * 0.16
                + closure * 0.24
                + (rng.uniform() - 0.5) * 0.12,
            0.08,
            0.96,
        );
        trajectory.push(section_vector(
            progress, novelty, stability, density, tension, closure, memory,
        ));
    }
    enforce_state_trajectory(&mut trajectory, rng, section_count);
    trajectory
}

pub(super) fn build_motif_variant_trajectory(
    rng: &mut Mulberry32,
    trajectory: &[SectionVector],
) -> Vec<u16> {
    let section_count = trajectory.len();
    let family_count = clamp(
        (1.52 + (section_count as f64).sqrt() * 0.38 + rng.uniform() * 0.9).round(),
        1.0,
        section_count.min(4).max(1) as f64,
    ) as usize;
    let families = (0..family_count)
        .map(|_| random_int(rng, 0, 9999) as u16)
        .collect::<Vec<_>>();
    let dwell = weighted_pick(
        &[
            (
                1.45,
                0.18 + 5_usize.saturating_sub(section_count) as f64 * 0.04,
            ),
            (1.85, 0.42),
            (2.35, 0.3),
            (2.9, 0.1 + section_count.saturating_sub(5) as f64 * 0.04),
        ],
        rng,
    );
    let phase = rng.uniform() * 0.86;
    let state_drift = 0.2 + rng.uniform() * 0.36;
    let closure_return = rng.uniform() * 0.28;
    trajectory
        .iter()
        .enumerate()
        .map(|(index, state)| {
            let phrase_position = index as f64 / dwell
                + phase
                + state.memory_distance * state_drift
                + state.tension * 0.12
                - state.closure_pressure * state.progress * closure_return;
            let family_index =
                (phrase_position.floor() as isize).rem_euclid(family_count as isize) as usize;
            families[family_index]
        })
        .collect()
}

fn enforce_state_trajectory(
    trajectory: &mut [SectionVector],
    rng: &mut Mulberry32,
    section_count: usize,
) {
    if section_count >= 4
        && !trajectory
            .iter()
            .any(|state| state.memory_distance >= 0.5 || state.tension >= 0.68)
    {
        let index = if section_count == 4 {
            2
        } else {
            random_int(rng, 2, (section_count - 2).max(2))
        };
        trajectory[index] = section_vector(
            index as f64 / (section_count - 1) as f64,
            0.68 + rng.uniform() * 0.18,
            0.24 + rng.uniform() * 0.16,
            0.56 + rng.uniform() * 0.22,
            0.68 + rng.uniform() * 0.18,
            0.24 + rng.uniform() * 0.16,
            0.58 + rng.uniform() * 0.24,
        );
    }
    if section_count >= 8
        && !trajectory[4..].iter().any(|state| {
            state.memory_distance >= 0.5 || state.closure_pressure >= 0.68 || state.density <= 0.32
        })
    {
        let index = random_int(rng, 4, section_count - 2);
        trajectory[index] = section_vector(
            index as f64 / (section_count - 1) as f64,
            0.58 + rng.uniform() * 0.28,
            0.22 + rng.uniform() * 0.18,
            0.24 + rng.uniform() * 0.54,
            0.56 + rng.uniform() * 0.26,
            0.34 + rng.uniform() * 0.34,
            0.54 + rng.uniform() * 0.3,
        );
    }
    let final_index = trajectory.len() - 1;
    let final_state = trajectory[final_index];
    trajectory[final_index] = section_vector(
        1.0,
        final_state.novelty * 0.62,
        0.68 + rng.uniform() * 0.22,
        final_state.density,
        final_state.tension * 0.72,
        0.82 + rng.uniform() * 0.14,
        final_state.memory_distance * 0.48,
    );
}

fn section_vector(
    progress: f64,
    novelty: f64,
    stability: f64,
    density: f64,
    tension: f64,
    closure_pressure: f64,
    memory_distance: f64,
) -> SectionVector {
    SectionVector {
        progress: round2(clamp(progress, 0.0, 1.0)),
        novelty: round2(clamp(novelty, 0.0, 1.0)),
        stability: round2(clamp(stability, 0.0, 1.0)),
        density: round2(clamp(density, 0.0, 1.0)),
        tension: round2(clamp(tension, 0.0, 1.0)),
        closure_pressure: round2(clamp(closure_pressure, 0.0, 1.0)),
        memory_distance: round2(clamp(memory_distance, 0.0, 1.0)),
    }
}

fn random_int(rng: &mut Mulberry32, min: usize, max: usize) -> usize {
    min + (rng.uniform() * (max - min + 1) as f64).floor() as usize
}

fn random_int_signed(rng: &mut Mulberry32, min: i32, max: i32) -> i32 {
    min + (rng.uniform() * f64::from(max - min + 1)).floor() as i32
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

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.min(max).max(min)
}

fn round2(value: f64) -> f64 {
    (value * 100.0 + 0.5).floor() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MusicRole;
    use crate::music::composition::form::CompositionForm;

    #[test]
    fn single_section_motif_variant_matches_javascript() {
        let mut rng = Mulberry32::from_text("composition:same-seed");
        // The four-chord progression consumes one weighted draw per added root.
        for _ in 0..3 {
            rng.uniform();
        }
        let trajectory = build_state_trajectory(
            &mut rng,
            CompositionForm {
                focus: MusicRole::Time,
                density: 0.45,
                space: 0.74,
                contrast: 0.19,
                pulse: 0.3,
            },
            1,
        );
        assert_eq!(
            build_motif_variant_trajectory(&mut rng, &trajectory),
            [9774]
        );
    }
}
