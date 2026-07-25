use crate::{MusicRole, prng::Mulberry32};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::music) struct CompositionForm {
    pub focus: MusicRole,
    pub density: f64,
    pub space: f64,
    pub contrast: f64,
    pub pulse: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::music) enum Carrier {
    MelodicLine,
    BassRiff,
    HarmonyArp,
    RhythmHook,
    DrumGrid,
    BassPulse,
    ArpPulse,
    ThinPulse,
    RootBass,
    ChordPad,
    Drone,
    Implied,
    None,
    AnswerLine,
    BassWalk,
    PercussionFill,
    AirPad,
    NoiseHalo,
    OrganBed,
    BrightAccent,
    RestGap,
    DrumFill,
    ContrastNote,
    RegisterTurn,
}

impl Carrier {
    pub(in crate::music) fn as_str(self) -> &'static str {
        match self {
            Self::MelodicLine => "melodic-line",
            Self::BassRiff => "bass-riff",
            Self::HarmonyArp => "harmony-arp",
            Self::RhythmHook => "rhythm-hook",
            Self::DrumGrid => "drum-grid",
            Self::BassPulse => "bass-pulse",
            Self::ArpPulse => "arp-pulse",
            Self::ThinPulse => "thin-pulse",
            Self::RootBass => "root-bass",
            Self::ChordPad => "chord-pad",
            Self::Drone => "drone",
            Self::Implied => "implied",
            Self::None => "none",
            Self::AnswerLine => "answer-line",
            Self::BassWalk => "bass-walk",
            Self::PercussionFill => "percussion-fill",
            Self::AirPad => "air-pad",
            Self::NoiseHalo => "noise-halo",
            Self::OrganBed => "organ-bed",
            Self::BrightAccent => "bright-accent",
            Self::RestGap => "rest-gap",
            Self::DrumFill => "drum-fill",
            Self::ContrastNote => "contrast-note",
            Self::RegisterTurn => "register-turn",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::music) struct CompositionRole {
    pub name: MusicRole,
    pub carrier: Carrier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::music) struct CompositionRoles {
    pub identity: CompositionRole,
    pub time: CompositionRole,
    pub tone: CompositionRole,
    pub motion: CompositionRole,
    pub color: CompositionRole,
    pub boundary: CompositionRole,
}

pub(super) fn build_form_and_roles(rng: &mut Mulberry32) -> (CompositionForm, CompositionRoles) {
    let form = build_composition_form(rng);
    let roles = assign_roles(rng, form);
    (form, roles)
}

fn build_composition_form(rng: &mut Mulberry32) -> CompositionForm {
    let focus = weighted_pick(
        &[
            (MusicRole::Identity, 0.28),
            (MusicRole::Time, 0.18),
            (MusicRole::Tone, 0.18),
            (MusicRole::Motion, 0.2),
            (MusicRole::Color, 0.16),
        ],
        rng,
    );
    CompositionForm {
        focus,
        density: round2(0.28 + rng.uniform() * 0.56),
        space: round2(rng.uniform()),
        contrast: round2(rng.uniform()),
        pulse: round2(rng.uniform()),
    }
}

fn assign_roles(rng: &mut Mulberry32, form: CompositionForm) -> CompositionRoles {
    let identity = weighted_pick(
        &[
            (
                Carrier::MelodicLine,
                if form.focus == MusicRole::Identity {
                    0.48
                } else {
                    0.28
                },
            ),
            (
                Carrier::BassRiff,
                if form.focus == MusicRole::Time {
                    0.26
                } else {
                    0.16
                },
            ),
            (
                Carrier::HarmonyArp,
                if form.focus == MusicRole::Motion {
                    0.3
                } else {
                    0.18
                },
            ),
            (
                Carrier::RhythmHook,
                if form.focus == MusicRole::Time {
                    0.28
                } else {
                    0.14
                },
            ),
        ],
        rng,
    );
    let time = weighted_pick(
        &[
            (
                Carrier::DrumGrid,
                if identity == Carrier::RhythmHook {
                    0.0
                } else if form.focus == MusicRole::Time {
                    0.44
                } else {
                    0.25
                },
            ),
            (Carrier::BassPulse, 0.25),
            (
                Carrier::ArpPulse,
                if form.focus == MusicRole::Motion {
                    0.3
                } else {
                    0.18
                },
            ),
            (
                Carrier::ThinPulse,
                if form.space > 0.68 { 0.34 } else { 0.12 },
            ),
        ],
        rng,
    );
    let tone = weighted_pick(
        &[
            (Carrier::RootBass, 0.34),
            (
                Carrier::ChordPad,
                if form.focus == MusicRole::Color {
                    0.34
                } else {
                    0.24
                },
            ),
            (Carrier::Drone, if form.space > 0.58 { 0.24 } else { 0.12 }),
            (
                Carrier::Implied,
                if form.density > 0.62 { 0.22 } else { 0.1 },
            ),
        ],
        rng,
    );
    let motion = weighted_pick(
        &[
            (Carrier::None, if form.density > 0.68 { 0.24 } else { 0.08 }),
            (
                Carrier::AnswerLine,
                if identity == Carrier::MelodicLine {
                    0.28
                } else {
                    0.14
                },
            ),
            (
                Carrier::HarmonyArp,
                if time != Carrier::ArpPulse {
                    0.25
                } else {
                    0.08
                },
            ),
            (
                Carrier::BassWalk,
                if tone == Carrier::RootBass { 0.2 } else { 0.08 },
            ),
            (
                Carrier::PercussionFill,
                if time == Carrier::DrumGrid {
                    0.18
                } else {
                    0.08
                },
            ),
        ],
        rng,
    );
    let color = weighted_pick(
        &[
            (
                Carrier::None,
                if form.focus == MusicRole::Color {
                    0.06
                } else {
                    0.18
                },
            ),
            (Carrier::AirPad, 0.24),
            (Carrier::NoiseHalo, 0.2),
            (Carrier::OrganBed, 0.16),
            (
                Carrier::BrightAccent,
                if form.contrast > 0.56 { 0.2 } else { 0.08 },
            ),
        ],
        rng,
    );
    let boundary = weighted_pick(
        &[
            (Carrier::RestGap, if form.space > 0.6 { 0.28 } else { 0.1 }),
            (
                Carrier::DrumFill,
                if time == Carrier::DrumGrid {
                    0.26
                } else {
                    0.12
                },
            ),
            (
                Carrier::ContrastNote,
                if form.contrast > 0.42 { 0.28 } else { 0.12 },
            ),
            (
                Carrier::RegisterTurn,
                if identity == Carrier::MelodicLine {
                    0.2
                } else {
                    0.1
                },
            ),
        ],
        rng,
    );
    CompositionRoles {
        identity: role(MusicRole::Identity, identity),
        time: role(MusicRole::Time, time),
        tone: role(MusicRole::Tone, tone),
        motion: role(MusicRole::Motion, motion),
        color: role(MusicRole::Color, color),
        boundary: role(MusicRole::Boundary, boundary),
    }
}

fn role(name: MusicRole, carrier: Carrier) -> CompositionRole {
    CompositionRole { name, carrier }
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

fn round2(value: f64) -> f64 {
    (value * 100.0 + 0.5).floor() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_form_and_roles_match_javascript() {
        let mut rng = Mulberry32::from_text("style:me-seed");
        // generateSong chooses key and scale before constructing the form.
        rng.uniform();
        rng.uniform();
        let (form, roles) = build_form_and_roles(&mut rng);
        assert_eq!(
            form,
            CompositionForm {
                focus: MusicRole::Time,
                density: 0.45,
                space: 0.74,
                contrast: 0.19,
                pulse: 0.3,
            }
        );
        assert_eq!(roles.identity.carrier, Carrier::MelodicLine);
        assert_eq!(roles.time.carrier, Carrier::BassPulse);
        assert_eq!(roles.tone.carrier, Carrier::ChordPad);
        assert_eq!(roles.motion.carrier, Carrier::HarmonyArp);
        assert_eq!(roles.color.carrier, Carrier::NoiseHalo);
        assert_eq!(roles.boundary.carrier, Carrier::RestGap);
    }
}
