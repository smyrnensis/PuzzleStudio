mod events;
mod form;
mod phrase;
mod section;

use crate::prng::Mulberry32;
use crate::{
    GeneratedMusicTrack, MusicMix, MusicRecipe, MusicRole, MusicScore, MusicTimbres, MusicTrack,
    MusicTransport, NoiseVoice, PercussionTimbre, PitchedTimbre, PlaybackTone,
    generate_spectral_timbre, generate_transient_timbre,
};
const KEYS: [(&str, i16); 7] = [
    ("C", 60),
    ("D", 62),
    ("E", 64),
    ("F", 65),
    ("G", 67),
    ("A", 69),
    ("Bb", 70),
];
const IONIAN: &[i16] = &[0, 2, 4, 5, 7, 9, 11];
const NATURAL_MINOR: &[i16] = &[0, 2, 3, 5, 7, 8, 10];
const MIXOLYDIAN: &[i16] = &[0, 2, 4, 5, 7, 9, 10];
const DORIAN: &[i16] = &[0, 2, 3, 5, 7, 9, 10];
const LYDIAN: &[i16] = &[0, 2, 4, 6, 7, 9, 11];
const PHRYGIAN: &[i16] = &[0, 1, 3, 5, 7, 8, 10];
const MAJOR_PENTATONIC: &[i16] = &[0, 2, 4, 7, 9];
const MINOR_PENTATONIC: &[i16] = &[0, 3, 5, 7, 10];
const SUSPENDED_PENTATONIC: &[i16] = &[0, 2, 5, 7, 10];

fn choose_key_and_scale(rng: &mut Mulberry32) -> (i16, &'static [i16]) {
    let tonic = KEYS[(rng.uniform() * KEYS.len() as f64).floor() as usize].1;
    let scale = weighted_pick(
        &[
            (MAJOR_PENTATONIC, 0.18),
            (MINOR_PENTATONIC, 0.18),
            (SUSPENDED_PENTATONIC, 0.1),
            (IONIAN, 0.14),
            (NATURAL_MINOR, 0.14),
            (DORIAN, 0.12),
            (MIXOLYDIAN, 0.09),
            (LYDIAN, 0.03),
            (PHRYGIAN, 0.02),
        ],
        rng,
    );
    (tonic, scale)
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

pub(super) fn generate(recipe: &MusicRecipe) -> Result<GeneratedMusicTrack, String> {
    validate_recipe(recipe)?;
    let style_seed = composition_style_seed(&recipe.seed)?;
    let mut style_rng = Mulberry32::from_text(&format!("style:{style_seed}"));
    let (tonic, scale) = choose_key_and_scale(&mut style_rng);
    let (form, roles) = form::build_form_and_roles(&mut style_rng);
    let timbres = build_timbres(roles, &style_seed);
    let mut composition_rng = Mulberry32::from_text(&format!("composition:{}", recipe.seed));
    let progression = events::build_progression(&mut composition_rng, scale.len() as i32);
    let trajectory =
        section::build_state_trajectory(&mut composition_rng, form, usize::from(recipe.bars / 8));
    let motifs = section::build_motif_variant_trajectory(&mut composition_rng, &trajectory);
    let sections = trajectory
        .into_iter()
        .zip(motifs)
        .enumerate()
        .map(|(index, (state, motif))| {
            section::realize_section(state, index, &mut composition_rng, roles, motif)
        })
        .collect::<Vec<_>>();
    let bars = phrase::build_bar_state_trajectory(&sections, recipe.bars)?;
    let mut score_events = Vec::new();
    for bar_state in bars {
        let section = sections
            .get(bar_state.section_index)
            .ok_or_else(|| format!("missing section {}", bar_state.section_index))?;
        let progression_index = (i32::from(bar_state.bar) + section.progression_shift)
            .rem_euclid(progression.len() as i32) as usize;
        let chord_root = progression[progression_index] + section.degree_offset;
        let chord = events::build_chord(tonic, scale, chord_root);
        events::assemble_bar(
            &mut score_events,
            events::BarRenderContext {
                seed: &recipe.seed,
                section,
                phrase: &bar_state.phrase_bar,
                tonic,
                scale,
                chord_root,
                chord,
                bar: bar_state.bar,
                local_bar: bar_state.local_bar,
                loop_handoff: bar_state.section_index + 1 == sections.len(),
            },
        )?;
    }
    score_events.sort_by_key(|event| (event.step, track_sort_key(event.track)));
    for (event_id, event) in score_events.iter_mut().enumerate() {
        event.event_id = event_id as u32;
    }
    Ok(completed_track(recipe, timbres, score_events))
}

fn track_sort_key(track: MusicTrack) -> u8 {
    match track {
        MusicTrack::Bass => 0,
        MusicTrack::Chord => 1,
        MusicTrack::Counter => 2,
        MusicTrack::Drums => 3,
        MusicTrack::Lead => 4,
    }
}

fn validate_recipe(recipe: &MusicRecipe) -> Result<(), String> {
    if !recipe.height.is_finite() || !(0.0..=1.0).contains(&recipe.height) {
        return Err("music height must be finite and between zero and one".to_string());
    }
    if !recipe.volume.is_finite() || recipe.volume < 0.0 {
        return Err("music volume must be finite and zero or greater".to_string());
    }
    if !(40..=180).contains(&recipe.bpm) {
        return Err("music BPM must be between 40 and 180".to_string());
    }
    if ![8, 16, 32, 64].contains(&recipe.bars) {
        return Err("music bars must be one of 8, 16, 32, or 64".to_string());
    }
    Ok(())
}

fn composition_style_seed(seed: &str) -> Result<String, String> {
    let units = seed.encode_utf16().collect::<Vec<_>>();
    if units.len() <= 2 {
        return Ok(seed.to_string());
    }
    String::from_utf16(&units[2..])
        .map_err(|_| "music seed splits an unpaired UTF-16 surrogate".to_string())
}

fn completed_track(
    recipe: &MusicRecipe,
    timbres: MusicTimbres,
    events: Vec<crate::MusicScoreEvent>,
) -> GeneratedMusicTrack {
    const SAMPLE_RATE: u32 = 48_000;
    GeneratedMusicTrack::from_resolved_score(
        SAMPLE_RATE,
        2,
        (f64::from(SAMPLE_RATE) * f64::from(recipe.bars) * 240.0 / f64::from(recipe.bpm)).round()
            as u64,
        MusicScore {
            transport: MusicTransport {
                bpm: recipe.bpm,
                bars: recipe.bars,
                steps_per_bar: 16,
                step_duration_beats: 0.25,
                loop_steps: u32::from(recipe.bars) * 16,
            },
            mix: MusicMix {
                volume: recipe.volume,
                playback_tone: PlaybackTone {
                    height: round2(recipe.height),
                    pitch_shift: round2((recipe.height - 0.5) * 24.0),
                    ..PlaybackTone::default()
                },
            },
            timbres: Some(Box::new(timbres)),
            events,
        },
    )
    .expect("validated composition parameters produce an addressable render index")
}

fn round2(value: f64) -> f64 {
    (value * 100.0 + 0.5).floor() / 100.0
}
use form::Carrier;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TransitionContext {
    pub impact: f64,
    pub bars: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TransitionBridge {
    pub role: MusicRole,
    pub carrier: Carrier,
    pub track: MusicTrack,
    pub target_degree_offset: i32,
    pub target_progression_shift: i32,
    pub impact: f64,
    pub bars: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PhraseBar {
    pub index: u8,
    pub target_center: f64,
    pub height_bias: f64,
    pub closure: f64,
    pub tension: f64,
    pub stability: f64,
    pub pace: f64,
    pub energy: f64,
    pub space: f64,
    pub boundary: f64,
    pub pickup: f64,
    pub tone_anchor: bool,
    pub color_accent: bool,
    pub syncopation: f64,
    pub transition_in: Option<TransitionContext>,
    pub transition_out: Option<TransitionContext>,
    pub transition_entry_bridge: Option<TransitionBridge>,
    pub transition_bridge: Option<TransitionBridge>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PhraseShape {
    pub archetype: String,
    pub bars: Vec<PhraseBar>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BarState {
    pub bar: u16,
    pub section_index: usize,
    pub local_bar: u8,
    pub phrase_archetype: String,
    pub phrase_bar: PhraseBar,
    pub transition_in: Option<TransitionContext>,
    pub transition_out: Option<TransitionContext>,
}

fn build_timbres(roles: form::CompositionRoles, style_seed: &str) -> MusicTimbres {
    let pitched = |role: MusicRole, carrier: form::Carrier| {
        let field = generate_spectral_timbre(&format!(
            "{style_seed}:pitched:{}:{}",
            role_name(role),
            carrier.as_str()
        ));
        PitchedTimbre {
            role,
            gain: pitched_role_gain(role) * field.signal.distance_gain,
            field,
        }
    };
    let percussion = |voice: NoiseVoice| {
        let field =
            generate_transient_timbre(&format!("{style_seed}:transient:{}", noise_name(voice)));
        PercussionTimbre {
            voice,
            gain: transient_role_gain(voice) * field.signal.distance_gain,
            field,
        }
    };
    MusicTimbres {
        identity: pitched(MusicRole::Identity, roles.identity.carrier),
        time: pitched(MusicRole::Time, roles.time.carrier),
        tone: pitched(MusicRole::Tone, roles.tone.carrier),
        motion: pitched(MusicRole::Motion, roles.motion.carrier),
        color: pitched(MusicRole::Color, roles.color.carrier),
        boundary: pitched(MusicRole::Boundary, roles.boundary.carrier),
        kick: percussion(NoiseVoice::Kick),
        snare: percussion(NoiseVoice::Snare),
        hat: percussion(NoiseVoice::Hat),
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

fn noise_name(voice: NoiseVoice) -> &'static str {
    match voice {
        NoiseVoice::Kick => "kick",
        NoiseVoice::Snare => "snare",
        NoiseVoice::Hat => "hat",
    }
}

fn pitched_role_gain(role: MusicRole) -> f64 {
    match role {
        MusicRole::Identity => 0.78,
        MusicRole::Time => 0.64,
        MusicRole::Tone => 0.58,
        MusicRole::Motion => 0.56,
        MusicRole::Color => 0.42,
        MusicRole::Boundary => 0.5,
    }
}

fn transient_role_gain(voice: NoiseVoice) -> f64 {
    match voice {
        NoiseVoice::Kick => 0.72,
        NoiseVoice::Snare => 0.58,
        NoiseVoice::Hat => 0.42,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score_fingerprint(events: &[crate::MusicScoreEvent]) -> u64 {
        fn write(hash: &mut u64, bytes: impl IntoIterator<Item = u8>) {
            for byte in bytes {
                *hash ^= u64::from(byte);
                *hash = hash.wrapping_mul(1_099_511_628_211);
            }
        }

        let mut hash = 14_695_981_039_346_656_037_u64;
        for event in events {
            write(&mut hash, event.step.to_le_bytes());
            write(&mut hash, [track_sort_key(event.track)]);
            write(&mut hash, event.duration_steps.to_le_bytes());
            write(&mut hash, [event.notes.len() as u8]);
            for note in &event.notes {
                match note {
                    crate::MusicNote::Midi(note) => {
                        write(&mut hash, [0]);
                        write(&mut hash, note.to_le_bytes());
                    }
                    crate::MusicNote::Noise(voice) => {
                        write(&mut hash, [1]);
                        write(
                            &mut hash,
                            [match voice {
                                NoiseVoice::Kick => 0,
                                NoiseVoice::Snare => 1,
                                NoiseVoice::Hat => 2,
                            }],
                        );
                    }
                }
            }
            write(
                &mut hash,
                [match event.role {
                    MusicRole::Identity => 0,
                    MusicRole::Time => 1,
                    MusicRole::Tone => 2,
                    MusicRole::Motion => 3,
                    MusicRole::Color => 4,
                    MusicRole::Boundary => 5,
                }],
            );
            // Math.exp and Rust libm may differ by one ULP. Canonicalizing at
            // 1e-12 keeps the score golden exact without binding it to a
            // particular transcendental implementation.
            let velocity = (event.velocity * 1_000_000_000_000.0).round() as i64;
            write(&mut hash, velocity.to_le_bytes());
        }
        hash
    }

    #[test]
    fn same_seed_score_matches_javascript_event_count_and_transport() {
        let track = generate(&MusicRecipe {
            seed: "same-seed".to_string(),
            height: 0.5,
            bars: 8,
            bpm: 110,
            volume: 1.0,
        })
        .expect("representative JS-compatible score should compile");
        assert_eq!(track.score().events.len(), 70);
        assert_eq!(track.score().transport.loop_steps, 128);
        assert_eq!(track.loop_frames, 837_818);
    }

    #[test]
    fn same_input_is_identical_and_seed_changes_the_score() {
        let recipe = MusicRecipe {
            seed: "same-seed".to_string(),
            height: 0.5,
            bars: 8,
            bpm: 110,
            volume: 1.0,
        };
        let left = generate(&recipe).unwrap();
        let right = generate(&recipe).unwrap();
        assert_eq!(left, right);
        let changed = generate(&MusicRecipe {
            seed: "other-seed".to_string(),
            ..recipe
        })
        .expect("all generated carrier outcomes must be supported");
        assert_ne!(changed.score().events, left.score().events);
    }

    #[test]
    fn every_generated_carrier_family_compiles_without_an_adapter_fallback() {
        use std::collections::BTreeSet;

        let mut covered: [BTreeSet<&'static str>; 6] = Default::default();
        let required = [4, 4, 4, 5, 5, 4];
        for serial in 0..20_000 {
            let seed = format!("aa{serial}");
            let style = composition_style_seed(&seed).unwrap();
            let mut rng = Mulberry32::from_text(&format!("style:{style}"));
            choose_key_and_scale(&mut rng);
            let (_, roles) = form::build_form_and_roles(&mut rng);
            let carriers = [
                roles.identity.carrier,
                roles.time.carrier,
                roles.tone.carrier,
                roles.motion.carrier,
                roles.color.carrier,
                roles.boundary.carrier,
            ];
            let introduces_family = carriers
                .iter()
                .enumerate()
                .any(|(index, carrier)| !covered[index].contains(carrier.as_str()));
            if introduces_family {
                generate(&MusicRecipe {
                    seed: seed.clone(),
                    height: 0.5,
                    bars: 8,
                    bpm: 100,
                    volume: 1.0,
                })
                .unwrap_or_else(|error| panic!("seed `{seed}` failed: {error}"));
                for (index, carrier) in carriers.into_iter().enumerate() {
                    covered[index].insert(carrier.as_str());
                }
            }
            if covered
                .iter()
                .zip(required)
                .all(|(actual, required)| actual.len() == required)
            {
                break;
            }
        }
        assert_eq!(
            covered.map(|families| families.len()),
            required,
            "selector coverage did not reach every carrier family"
        );
    }

    #[test]
    fn carrier_family_scores_match_javascript_event_count_goldens() {
        let cases = [
            ("aa0", 99),
            ("aa1", 95),
            ("aa2", 69),
            ("aa4", 103),
            ("aa5", 70),
            ("aa6", 94),
            ("aa7", 58),
            ("aa8", 68),
            ("aa10", 107),
            ("aa11", 55),
            ("aa23", 53),
        ];
        for (seed, expected_count) in cases {
            let track = generate(&MusicRecipe {
                seed: seed.to_string(),
                height: 0.5,
                bars: 8,
                bpm: 100,
                volume: 1.0,
            })
            .unwrap_or_else(|error| panic!("seed `{seed}` failed: {error}"));
            assert_eq!(
                track.score().events.len(),
                expected_count,
                "seed `{seed}` diverged from the JavaScript score; role counts: {:?}",
                track.score().events.iter().fold(
                    std::collections::BTreeMap::<String, usize>::new(),
                    |mut counts, event| {
                        *counts
                            .entry(format!("{:?}/{}", event.role, event.step / 16))
                            .or_default() += 1;
                        counts
                    }
                )
            );
        }
    }

    #[test]
    fn representative_carrier_scores_match_exact_javascript_goldens() {
        let cases = [
            ("carrier-0", 8, 57, 0x7f31_b994_1f85_f980),
            ("carrier-3", 8, 97, 0x3a61_7a5b_6adc_8ff9),
            ("carrier-5", 8, 56, 0xa365_0e32_520f_5c55),
            ("carrier-10", 8, 81, 0x98de_8f6e_af38_6c56),
            ("carrier-13", 8, 116, 0x19d4_3333_9544_59df),
            ("carrier-16", 8, 88, 0x1d0a_5c48_a74f_cc09),
            // Two sections exercise both outgoing and incoming transition bridges.
            ("carrier-0", 16, 99, 0x3786_c4ba_cf26_79d5),
        ];
        for (seed, bars, event_count, fingerprint) in cases {
            let track = generate(&MusicRecipe {
                seed: seed.to_string(),
                height: 0.5,
                bars,
                bpm: 110,
                volume: 1.0,
            })
            .unwrap_or_else(|error| panic!("{seed}/{bars}: {error}"));
            assert_eq!(track.score().events.len(), event_count, "{seed}/{bars}");
            assert_eq!(
                score_fingerprint(&track.score().events),
                fingerprint,
                "{seed}/{bars}"
            );
        }
    }
}

// Implemented by `phrase.rs`; declared here as the integration contract:
// build_bar_state_trajectory(
//     section_plan: &[section::RealizedSection],
//     bars: u16,
// ) -> Result<Vec<BarState>, String>
