use std::f64::consts::PI;
use std::sync::Arc;

use crate::prng::{Mulberry32, Rc4};

pub const SFX_TYPES: [&str; 12] = [
    "jump",
    "step",
    "pickup",
    "hit",
    "drag",
    "water",
    "lock",
    "explosion",
    "laser",
    "powerup",
    "select",
    "error",
];

#[derive(Clone, Debug, PartialEq)]
pub struct SfxRecipe {
    pub seed: String,
    pub type_target: String,
    pub volume: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedSfx {
    pub seed: String,
    pub resolved_type: String,
    pub volume: f64,
    pub synthesis: SfxSynthesis,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SfxSynthesis {
    Layers {
        duration: f64,
        profile: SfxProfile,
        layers: Vec<SfxLayer>,
    },
    PuzzleScript(PuzzleScriptParams),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SfxProfile {
    pub engine: String,
    pub variant: String,
    pub pattern: String,
    pub waveform: Waveform,
    pub noise_color: NoiseColor,
    pub filter_bias: f64,
    pub pitch_wobble: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Triangle,
    Square,
    Sawtooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoiseColor {
    White,
    Crackle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SfxFilterKind {
    Lowpass,
    Highpass,
    Bandpass,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SfxLayer {
    Tone {
        name: String,
        start: f64,
        duration: f64,
        waveform: Waveform,
        frequency_start: f64,
        frequency_end: f64,
        gain: f64,
        attack: f64,
        release: f64,
        filter_frequency: f64,
        wobble: f64,
    },
    Noise {
        name: String,
        color: NoiseColor,
        start: f64,
        duration: f64,
        gain: f64,
        attack: f64,
        release: f64,
        filter: SfxFilterKind,
        filter_start: f64,
        filter_end: f64,
    },
    Click {
        start: f64,
        duration: f64,
        gain: f64,
        filter_frequency: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedSfxClip {
    pub sample_rate: u32,
    pub samples: Arc<[f32]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PuzzleScriptParams {
    pub wave_type: u8,
    pub env_attack: f64,
    pub env_sustain: f64,
    pub env_punch: f64,
    pub env_decay: f64,
    pub base_freq: f64,
    pub freq_limit: f64,
    pub freq_ramp: f64,
    pub freq_dramp: f64,
    pub vib_strength: f64,
    pub vib_speed: f64,
    pub arp_mod: f64,
    pub arp_speed: f64,
    pub duty: f64,
    pub duty_ramp: f64,
    pub repeat_speed: f64,
    pub pha_offset: f64,
    pub pha_ramp: f64,
    pub lpf_freq: f64,
    pub lpf_ramp: f64,
    pub lpf_resonance: f64,
    pub hpf_freq: f64,
    pub hpf_ramp: f64,
    pub sound_volume: f64,
    pub sample_rate: u32,
    pub seed: u32,
}

pub fn generate_sfx(recipe: &SfxRecipe) -> Result<GeneratedSfx, String> {
    if !recipe.volume.is_finite() || recipe.volume < 0.0 {
        return Err("SFX volume must be finite and zero or greater".to_string());
    }
    let target = recipe.type_target.as_str();
    if target == "puzzlescript" {
        let numeric_seed = numeric_seed(&recipe.seed);
        return Ok(GeneratedSfx {
            seed: recipe.seed.clone(),
            resolved_type: target.to_string(),
            volume: recipe.volume,
            synthesis: SfxSynthesis::PuzzleScript(generate_puzzlescript(numeric_seed)),
        });
    }
    if target != "random" && target != "wild" && !SFX_TYPES.contains(&target) {
        return Err(format!("unsupported SFX type: {target}"));
    }

    let mut rng = Mulberry32::from_text(&recipe.seed);
    let resolved = if target == "random" {
        SFX_TYPES[(crate::prng::fnv1a_utf16(&recipe.seed) as usize) % SFX_TYPES.len()]
    } else {
        target
    };
    let mood = rng.uniform();
    let intensity = lerp(0.45, 0.92, rng.uniform());
    let length = rng.uniform();
    let (base_duration, low, high) = type_config(resolved);
    let mut duration =
        round3(base_duration * lerp(0.72, 1.45, length) * lerp(0.94, 1.06, rng.uniform()));
    duration = match resolved {
        "drag" => duration.clamp(0.36, 0.7),
        "step" => duration.clamp(0.09, 0.3),
        "water" => duration.clamp(0.18, 0.85),
        _ => duration,
    };
    let base_frequency = f64::from(rng.int_inclusive(low, high));
    let profile = build_profile(&mut rng, resolved, mood, intensity);
    let layers = build_layers(
        &mut rng,
        resolved,
        duration,
        base_frequency,
        mood,
        intensity,
        &profile,
    );
    let (duration, layers) = align_layers_to_attack(layers, duration);
    Ok(GeneratedSfx {
        seed: recipe.seed.clone(),
        resolved_type: resolved.to_string(),
        volume: recipe.volume,
        synthesis: SfxSynthesis::Layers {
            duration,
            profile,
            layers,
        },
    })
}

fn align_layers_to_attack(mut layers: Vec<SfxLayer>, duration: f64) -> (f64, Vec<SfxLayer>) {
    let first = layers.iter().map(layer_start).fold(f64::INFINITY, f64::min);
    if !first.is_finite() || first <= 0.0 {
        return (duration, layers);
    }
    for layer in &mut layers {
        match layer {
            SfxLayer::Tone { start, .. }
            | SfxLayer::Noise { start, .. }
            | SfxLayer::Click { start, .. } => *start = round3(*start - first),
        }
    }
    let audible_end = layers.iter().map(layer_end).fold(0.0, f64::max);
    sort_layers(&mut layers);
    (round3((duration - first).max(audible_end)), layers)
}

pub fn render_sfx(generated: &GeneratedSfx) -> Result<GeneratedSfxClip, String> {
    match &generated.synthesis {
        SfxSynthesis::PuzzleScript(params) => render_puzzlescript(params, generated.volume),
        SfxSynthesis::Layers {
            duration, layers, ..
        } => render_layers(*duration, layers, generated.volume),
    }
}

fn type_config(kind: &str) -> (f64, u32, u32) {
    match kind {
        "wild" => (0.5, 60, 1400),
        "jump" => (0.28, 230, 330),
        "step" => (0.18, 160, 360),
        "pickup" => (0.42, 620, 880),
        "hit" => (0.24, 90, 160),
        "drag" => (0.5, 72, 138),
        "water" => (0.46, 90, 220),
        "lock" => (0.36, 115, 210),
        "explosion" => (0.82, 45, 82),
        "laser" => (0.36, 460, 760),
        "powerup" => (0.72, 260, 430),
        "select" => (0.12, 720, 1120),
        "error" => (0.42, 170, 260),
        _ => unreachable!("validated SFX type"),
    }
}

fn patterns(kind: &str) -> &'static [&'static str] {
    match kind {
        "wild" => &["tone", "noise", "clicks", "sweep", "broken", "stack"],
        "jump" => &["hop", "spring", "rubber", "whoosh"],
        "step" => &["tap", "wood", "stone", "grass", "heavy", "soft"],
        "pickup" => &["coin", "sparkle", "gem", "chord"],
        "hit" => &["punch", "slash", "metal", "crunch"],
        "drag" => &[
            "wood-floor",
            "stone-floor",
            "rough-floor",
            "stuck-start",
            "short-pull",
            "soft-floor",
        ],
        "water" => &["splash", "plop", "ripple", "bubble", "pour", "drip"],
        "lock" => &[
            "latch", "deadbolt", "key-turn", "tumblers", "old-lock", "padlock",
        ],
        "explosion" => &["boom", "puff", "crackle", "burst"],
        "laser" => &["pew", "zap", "down", "charge"],
        "powerup" => &["arpeggio", "swell", "sparkle", "fanfare"],
        "select" => &["cursor", "blip", "press", "soft"],
        "error" => &["buzzer", "fall", "double", "glitch"],
        _ => unreachable!(),
    }
}

fn build_profile(rng: &mut Mulberry32, kind: &str, mood: f64, intensity: f64) -> SfxProfile {
    let variants: &[&str] = match kind {
        "drag" => &["dry", "grainy", "heavy", "stuck", "soft", "rough"],
        "lock" => &["dry", "double", "gritty", "stepped", "heavy", "stuck"],
        "step" => &["dry", "double", "soft", "heavy", "wood", "gravel"],
        "water" => &["small", "deep", "bubbly", "wide", "soft", "choppy"],
        "error" => &["clean", "double", "gritty", "stepped"],
        _ => &["clean", "double", "gritty", "hollow", "wide", "stepped"],
    };
    let waves: &[Waveform] = if mood < 0.4 {
        &[Waveform::Sawtooth, Waveform::Square, Waveform::Triangle]
    } else if mood > 0.6 {
        &[Waveform::Triangle, Waveform::Sine, Waveform::Square]
    } else {
        &[Waveform::Sine, Waveform::Triangle, Waveform::Square]
    };
    let engine = ["arcade", "soft-synth", "bit-crush", "toy-speaker"][rng.index(4)];
    let variant = variants[rng.index(variants.len())];
    let pattern_options = patterns(kind);
    let pattern = pattern_options[rng.index(pattern_options.len())];
    let waveform = match kind {
        "error" => [Waveform::Square, Waveform::Sawtooth][rng.index(2)],
        "drag" | "water" => [Waveform::Sine, Waveform::Triangle][rng.index(2)],
        "lock" => [Waveform::Square, Waveform::Triangle][rng.index(2)],
        _ => waves[rng.index(waves.len())],
    };
    let color = if mood < 0.4 || matches!(kind, "explosion" | "error" | "lock") {
        NoiseColor::Crackle
    } else {
        NoiseColor::White
    };
    let type_bias = match kind {
        "error" => 0.78,
        "drag" => 0.72,
        "water" => 0.86,
        "step" => 0.9,
        _ if mood > 0.6 => 1.18,
        _ if mood < 0.4 => 0.82,
        _ => 1.0,
    };
    let wobble = match kind {
        "lock" => lerp(0.0, 0.018, rng.uniform() * intensity),
        "drag" => lerp(0.004, 0.02, rng.uniform() * intensity),
        "water" => lerp(0.006, 0.035, rng.uniform() * intensity),
        "step" => lerp(0.002, 0.014, rng.uniform() * intensity),
        "error" => lerp(0.04, 0.12, rng.uniform() * intensity),
        _ => lerp(0.01, 0.075, rng.uniform() * intensity),
    };
    SfxProfile {
        engine: engine.to_string(),
        variant: variant.to_string(),
        pattern: pattern.to_string(),
        waveform,
        noise_color: color,
        filter_bias: round2(lerp(0.75, 1.35, intensity) * type_bias),
        pitch_wobble: round2(wobble),
    }
}

fn build_layers(
    rng: &mut Mulberry32,
    kind: &str,
    duration: f64,
    base: f64,
    mood: f64,
    intensity: f64,
    profile: &SfxProfile,
) -> Vec<SfxLayer> {
    if kind == "wild" {
        let count = rng.int_inclusive(1, 5);
        let mut layers = Vec::with_capacity(count as usize + 2);
        for index in 0..count {
            let start = duration * lerp(0.0, 0.72, rng.uniform());
            let layer_duration =
                duration * lerp(0.08, 0.95 - (start / duration).min(0.65), rng.uniform());
            let layer_kind = match profile.pattern.as_str() {
                "noise" => 1,
                "clicks" => 2,
                "stack" => 0,
                _ => [0, 0, 1, 2][rng.index(4)],
            };
            if layer_kind == 0 {
                let sweep = if profile.pattern == "sweep" {
                    lerp(0.18, 4.4, rng.uniform())
                } else {
                    lerp(0.35, 3.6, rng.uniform())
                };
                layers.push(tone(
                    &format!("wild-tone-{}", index + 1),
                    start,
                    layer_duration,
                    [
                        Waveform::Sine,
                        Waveform::Triangle,
                        Waveform::Square,
                        Waveform::Sawtooth,
                    ][rng.index(4)],
                    base * lerp(0.2, 3.2, rng.uniform()),
                    base * sweep,
                    lerp(0.04, 0.28, rng.uniform()),
                    intensity,
                    profile,
                ));
            } else if layer_kind == 1 {
                layers.push(noise(
                    &format!("wild-noise-{}", index + 1),
                    start,
                    layer_duration,
                    lerp(0.03, 0.26, rng.uniform()),
                    [
                        SfxFilterKind::Lowpass,
                        SfxFilterKind::Highpass,
                        SfxFilterKind::Bandpass,
                    ][rng.index(3)],
                    ri(rng, 160, 9000),
                    ri(rng, 80, 9200),
                    profile,
                ));
            } else {
                layers.push(click(
                    start,
                    lerp(0.008, 0.045, rng.uniform()),
                    lerp(0.04, 0.22, rng.uniform()),
                    ri(rng, 420, 9000),
                ));
            }
        }
        if profile.pattern == "broken" {
            layers.push(click(
                duration * lerp(0.16, 0.84, rng.uniform()),
                0.012,
                0.08 + intensity * 0.08,
                ri(rng, 1200, 7600),
            ));
            layers.push(noise(
                "wild-gap-noise",
                duration * lerp(0.34, 0.68, rng.uniform()),
                duration * 0.16,
                0.08 + intensity * 0.08,
                SfxFilterKind::Bandpass,
                ri(rng, 600, 6400),
                ri(rng, 180, 2800),
                profile,
            ));
        }
        vary_layers(rng, &mut layers, duration, base, intensity, profile);
        return layers;
    }
    if kind == "pickup" {
        let mut layers = match profile.pattern.as_str() {
            "gem" => vec![
                tone(
                    "gem-low",
                    0.0,
                    duration * 0.7,
                    Waveform::Sine,
                    base * 0.92,
                    base * 0.94,
                    0.16,
                    intensity,
                    profile,
                ),
                tone(
                    "gem-high",
                    duration * 0.1,
                    duration * 0.72,
                    Waveform::Triangle,
                    base * 2.15,
                    base * 2.18,
                    0.2,
                    intensity,
                    profile,
                ),
                noise(
                    "shine",
                    duration * 0.16,
                    duration * 0.36,
                    0.08,
                    SfxFilterKind::Highpass,
                    4800.0,
                    9000.0,
                    profile,
                ),
            ],
            "coin" => vec![
                tone(
                    "coin",
                    0.0,
                    duration * 0.34,
                    Waveform::Square,
                    base * 1.7,
                    base * 2.05,
                    0.24,
                    intensity,
                    profile,
                ),
                tone(
                    "ring",
                    duration * 0.06,
                    duration * 0.62,
                    Waveform::Sine,
                    base * 2.45,
                    base * 2.42,
                    0.12,
                    intensity,
                    profile,
                ),
                click(0.0, 0.014, 0.08 + intensity * 0.08, 6200.0),
            ],
            "chord" => vec![
                tone(
                    "chord-root",
                    0.0,
                    duration * 0.62,
                    Waveform::Triangle,
                    base,
                    base,
                    0.16,
                    intensity,
                    profile,
                ),
                tone(
                    "chord-third",
                    0.012,
                    duration * 0.58,
                    Waveform::Sine,
                    base * 1.25,
                    base * 1.25,
                    0.15,
                    intensity,
                    profile,
                ),
                tone(
                    "chord-fifth",
                    0.024,
                    duration * 0.54,
                    Waveform::Sine,
                    base * 1.5,
                    base * 1.5,
                    0.16,
                    intensity,
                    profile,
                ),
            ],
            _ => vec![
                tone(
                    "note-1",
                    0.0,
                    duration * 0.45,
                    Waveform::Sine,
                    base,
                    base * 1.01,
                    0.18,
                    intensity,
                    profile,
                ),
                tone(
                    "note-2",
                    duration / 4.8,
                    duration * 0.42,
                    Waveform::Triangle,
                    base * [1.2, 1.25, 1.333][rng.index(3)],
                    base * 1.26,
                    0.18,
                    intensity,
                    profile,
                ),
                tone(
                    "note-3",
                    duration / 4.8 * [1.75, 2.0, 2.35][rng.index(3)],
                    duration * 0.52,
                    Waveform::Sine,
                    base * [1.5, 1.667, 2.0][rng.index(3)],
                    base * 1.5,
                    0.22,
                    intensity,
                    profile,
                ),
                noise(
                    "sparkle",
                    duration / 4.8 * 2.1,
                    duration * 0.28,
                    0.07 + intensity * 0.04,
                    SfxFilterKind::Highpass,
                    5200.0,
                    7600.0,
                    profile,
                ),
            ],
        };
        vary_layers(rng, &mut layers, duration, base, intensity, profile);
        return layers;
    }

    if kind == "jump" {
        let mut layers = match profile.pattern.as_str() {
            "spring" => vec![
                tone(
                    "coil-1",
                    0.0,
                    duration * 0.55,
                    Waveform::Square,
                    base * 0.85,
                    base * 2.6,
                    0.22,
                    intensity,
                    profile,
                ),
                tone(
                    "coil-2",
                    duration * 0.22,
                    duration * 0.45,
                    Waveform::Triangle,
                    base * 1.3,
                    base * 2.2,
                    0.16,
                    intensity,
                    profile,
                ),
                tone(
                    "coil-3",
                    duration * 0.42,
                    duration * 0.35,
                    Waveform::Sine,
                    base * 1.7,
                    base * 2.9,
                    0.12,
                    intensity,
                    profile,
                ),
            ],
            "rubber" => vec![
                tone(
                    "boing",
                    0.0,
                    duration * 1.1,
                    Waveform::Sine,
                    base * 0.72,
                    base * 1.9,
                    0.36,
                    intensity,
                    profile,
                ),
                tone(
                    "bend",
                    duration * 0.12,
                    duration * 0.72,
                    Waveform::Triangle,
                    base * 0.9,
                    base * 1.35,
                    0.14,
                    intensity,
                    profile,
                ),
            ],
            "whoosh" => vec![
                noise(
                    "lift-air",
                    0.0,
                    duration * 0.86,
                    0.16 + intensity * 0.12,
                    SfxFilterKind::Highpass,
                    900.0,
                    5400.0,
                    profile,
                ),
                tone(
                    "body",
                    duration * 0.06,
                    duration * 0.58,
                    Waveform::Triangle,
                    base,
                    base * 2.2,
                    0.18,
                    intensity,
                    profile,
                ),
                click(0.0, 0.02, 0.08 + intensity * 0.06, 1800.0),
            ],
            _ => vec![
                tone(
                    "body",
                    0.0,
                    duration,
                    profile.waveform,
                    base,
                    base * lerp(1.85, 2.35, mood),
                    0.34,
                    intensity,
                    profile,
                ),
                tone(
                    "edge",
                    0.015,
                    duration * 0.62,
                    Waveform::Square,
                    base * 1.5,
                    base * 2.65,
                    0.12,
                    intensity,
                    profile,
                ),
                click(
                    0.0,
                    0.026,
                    0.08 + intensity * 0.08,
                    2800.0 * profile.filter_bias,
                ),
            ],
        };
        vary_layers(rng, &mut layers, duration, base, intensity, profile);
        return layers;
    }

    if kind == "hit" {
        let mut layers = match profile.pattern.as_str() {
            "slash" => vec![
                noise(
                    "slice",
                    0.0,
                    duration * 0.68,
                    0.18 + intensity * 0.2,
                    SfxFilterKind::Highpass,
                    6200.0,
                    1200.0,
                    profile,
                ),
                click(0.0, 0.018, 0.12 + intensity * 0.14, 5200.0),
            ],
            "metal" => vec![
                tone(
                    "clang",
                    0.0,
                    duration * 1.12,
                    Waveform::Square,
                    base * 4.2,
                    base * 2.8,
                    0.22,
                    intensity,
                    profile,
                ),
                tone(
                    "ring",
                    duration * 0.04,
                    duration * 0.95,
                    Waveform::Sine,
                    base * 6.1,
                    base * 5.7,
                    0.12,
                    intensity,
                    profile,
                ),
                click(0.0, 0.016, 0.14 + intensity * 0.12, 7200.0),
            ],
            "crunch" => vec![
                tone(
                    "low-hit",
                    0.0,
                    duration * 0.72,
                    Waveform::Sine,
                    base * 1.5,
                    base * 0.44,
                    0.34,
                    intensity,
                    profile,
                ),
                noise(
                    "crunch",
                    0.0,
                    duration * 0.88,
                    0.26 + intensity * 0.22,
                    SfxFilterKind::Bandpass,
                    2600.0,
                    680.0,
                    profile,
                ),
                noise(
                    "dust",
                    duration * 0.12,
                    duration * 0.62,
                    0.12,
                    SfxFilterKind::Lowpass,
                    900.0,
                    220.0,
                    profile,
                ),
            ],
            _ => vec![
                tone(
                    "thud",
                    0.0,
                    duration * 0.9,
                    Waveform::Sine,
                    base * 1.8,
                    base * 0.55,
                    0.4,
                    intensity,
                    profile,
                ),
                noise(
                    "body",
                    0.0,
                    duration * 0.62,
                    0.22 + intensity * 0.2,
                    SfxFilterKind::Lowpass,
                    1800.0,
                    380.0,
                    profile,
                ),
                click(
                    0.0,
                    0.018,
                    0.18 + intensity * 0.18,
                    3200.0 * profile.filter_bias,
                ),
            ],
        };
        vary_layers(rng, &mut layers, duration, base, intensity, profile);
        return layers;
    }

    if kind == "explosion" {
        let mut layers = match profile.pattern.as_str() {
            "puff" => vec![
                noise(
                    "puff",
                    0.0,
                    duration * 0.72,
                    0.28 + intensity * 0.18,
                    SfxFilterKind::Lowpass,
                    1200.0,
                    140.0,
                    profile,
                ),
                tone(
                    "soft-sub",
                    0.0,
                    duration * 0.42,
                    Waveform::Sine,
                    base,
                    base * 0.54,
                    0.2,
                    intensity,
                    profile,
                ),
            ],
            "crackle" => vec![
                noise(
                    "crackle",
                    0.0,
                    duration * 0.95,
                    0.3 + intensity * 0.32,
                    SfxFilterKind::Bandpass,
                    3200.0,
                    420.0,
                    profile,
                ),
                noise(
                    "smoke",
                    duration * 0.2,
                    duration * 0.8,
                    0.16 + intensity * 0.16,
                    SfxFilterKind::Lowpass,
                    900.0,
                    120.0,
                    profile,
                ),
                click(0.0, 0.03, 0.18 + intensity * 0.18, 1400.0),
                click(duration * 0.16, 0.024, 0.08 + intensity * 0.12, 2200.0),
            ],
            "burst" => vec![
                tone(
                    "blast-tone",
                    0.0,
                    duration * 0.48,
                    Waveform::Sawtooth,
                    base * 3.0,
                    base * 0.5,
                    0.34,
                    intensity,
                    profile,
                ),
                noise(
                    "blast-noise",
                    0.0,
                    duration * 0.7,
                    0.36 + intensity * 0.28,
                    SfxFilterKind::Highpass,
                    5400.0,
                    360.0,
                    profile,
                ),
                click(0.0, 0.022, 0.22 + intensity * 0.14, 4200.0),
            ],
            _ => vec![
                tone(
                    "sub",
                    0.0,
                    duration * 0.78,
                    Waveform::Sine,
                    base * 1.5,
                    base * 0.42,
                    0.46,
                    intensity,
                    profile,
                ),
                noise(
                    "blast",
                    0.01,
                    duration,
                    0.35 + intensity * 0.34,
                    SfxFilterKind::Lowpass,
                    2600.0 * profile.filter_bias,
                    170.0,
                    profile,
                ),
                noise(
                    "debris",
                    duration * 0.18,
                    duration * 0.52,
                    0.12 + intensity * 0.16,
                    SfxFilterKind::Bandpass,
                    1200.0,
                    520.0,
                    profile,
                ),
                click(0.0, 0.035, 0.2 + intensity * 0.16, 900.0),
            ],
        };
        vary_layers(rng, &mut layers, duration, base, intensity, profile);
        return layers;
    }

    if kind == "laser" {
        let mut layers = match profile.pattern.as_str() {
            "zap" => vec![
                tone(
                    "zap",
                    0.0,
                    duration * 0.62,
                    Waveform::Square,
                    base * 1.6,
                    base * 0.42,
                    0.32,
                    intensity,
                    profile,
                ),
                click(0.0, 0.014, 0.12 + intensity * 0.08, 6400.0),
                noise(
                    "electric",
                    duration * 0.05,
                    duration * 0.32,
                    0.06 + intensity * 0.06,
                    SfxFilterKind::Bandpass,
                    5200.0,
                    1800.0,
                    profile,
                ),
            ],
            "down" => vec![
                tone(
                    "falling-beam",
                    0.0,
                    duration,
                    Waveform::Sawtooth,
                    base * 2.8,
                    base * 0.38,
                    0.28,
                    intensity,
                    profile,
                ),
                tone(
                    "thin-edge",
                    duration * 0.08,
                    duration * 0.54,
                    Waveform::Square,
                    base * 3.1,
                    base * 0.62,
                    0.1,
                    intensity,
                    profile,
                ),
            ],
            "charge" => vec![
                tone(
                    "charge",
                    0.0,
                    duration * 0.5,
                    Waveform::Triangle,
                    base * 0.55,
                    base * 2.4,
                    0.16,
                    intensity,
                    profile,
                ),
                tone(
                    "fire",
                    duration * 0.46,
                    duration * 0.48,
                    Waveform::Square,
                    base * 2.4,
                    base * 0.7,
                    0.28,
                    intensity,
                    profile,
                ),
                click(duration * 0.46, 0.02, 0.12 + intensity * 0.1, 4600.0),
            ],
            _ => vec![
                tone(
                    "beam",
                    0.0,
                    duration,
                    profile.waveform,
                    base,
                    base * if mood > 0.5 { 2.6 } else { 0.34 },
                    0.28,
                    intensity,
                    profile,
                ),
                tone(
                    "alias",
                    duration * 0.08,
                    duration * 0.72,
                    Waveform::Square,
                    base * 1.01,
                    base * if mood > 0.5 { 2.6 } else { 0.34 } * 1.02,
                    0.11,
                    intensity,
                    profile,
                ),
                noise(
                    "air",
                    0.0,
                    duration * 0.35,
                    0.05 + intensity * 0.06,
                    SfxFilterKind::Highpass,
                    4200.0,
                    1800.0,
                    profile,
                ),
            ],
        };
        vary_layers(rng, &mut layers, duration, base, intensity, profile);
        return layers;
    }

    if kind == "powerup" {
        let step = duration / 5.6;
        let mut layers = match profile.pattern.as_str() {
            "swell" => vec![
                tone(
                    "swell",
                    0.0,
                    duration * 0.96,
                    Waveform::Sine,
                    base * 0.72,
                    base * 2.2,
                    0.32,
                    intensity,
                    profile,
                ),
                noise(
                    "air-lift",
                    duration * 0.2,
                    duration * 0.58,
                    0.08 + intensity * 0.06,
                    SfxFilterKind::Highpass,
                    1400.0,
                    7200.0,
                    profile,
                ),
            ],
            "sparkle" => vec![
                tone(
                    "spark-1",
                    0.0,
                    duration * 0.24,
                    Waveform::Sine,
                    base * 2.0,
                    base * 2.02,
                    0.16,
                    intensity,
                    profile,
                ),
                tone(
                    "spark-2",
                    step * 1.2,
                    duration * 0.22,
                    Waveform::Triangle,
                    base * 2.5,
                    base * 2.48,
                    0.16,
                    intensity,
                    profile,
                ),
                tone(
                    "spark-3",
                    step * 2.4,
                    duration * 0.32,
                    Waveform::Sine,
                    base * 3.0,
                    base * 3.05,
                    0.18,
                    intensity,
                    profile,
                ),
                noise(
                    "dust",
                    step * 2.0,
                    duration * 0.34,
                    0.05 + intensity * 0.04,
                    SfxFilterKind::Highpass,
                    4800.0,
                    9600.0,
                    profile,
                ),
            ],
            "fanfare" => vec![
                tone(
                    "root",
                    0.0,
                    duration * 0.54,
                    Waveform::Square,
                    base,
                    base * 1.01,
                    0.16,
                    intensity,
                    profile,
                ),
                tone(
                    "fifth",
                    duration * 0.18,
                    duration * 0.54,
                    Waveform::Square,
                    base * 1.5,
                    base * 1.51,
                    0.16,
                    intensity,
                    profile,
                ),
                tone(
                    "octave",
                    duration * 0.36,
                    duration * 0.62,
                    Waveform::Triangle,
                    base * 2.0,
                    base * 2.02,
                    0.2,
                    intensity,
                    profile,
                ),
            ],
            _ => vec![
                tone(
                    "step-1",
                    0.0,
                    duration * 0.28,
                    Waveform::Triangle,
                    base,
                    base * 1.01,
                    0.18,
                    intensity,
                    profile,
                ),
                tone(
                    "step-2",
                    step,
                    duration * 0.3,
                    Waveform::Triangle,
                    base * 1.25,
                    base * 1.26,
                    0.18,
                    intensity,
                    profile,
                ),
                tone(
                    "step-3",
                    step * 2.0,
                    duration * 0.32,
                    Waveform::Sine,
                    base * 1.5,
                    base * 1.52,
                    0.2,
                    intensity,
                    profile,
                ),
                tone(
                    "shine",
                    step * 3.1,
                    duration * 0.42,
                    Waveform::Sine,
                    base * 2.0,
                    base * 2.06,
                    0.22,
                    intensity,
                    profile,
                ),
                noise(
                    "lift",
                    step * 2.4,
                    duration * 0.34,
                    0.06 + intensity * 0.04,
                    SfxFilterKind::Highpass,
                    3600.0,
                    7000.0,
                    profile,
                ),
            ],
        };
        vary_layers(rng, &mut layers, duration, base, intensity, profile);
        return layers;
    }

    if kind == "error" {
        let mut layers = match profile.pattern.as_str() {
            "fall" => vec![
                tone(
                    "fall-1",
                    0.0,
                    duration * 0.42,
                    Waveform::Square,
                    base * 1.8,
                    base * 0.72,
                    0.26,
                    intensity,
                    profile,
                ),
                tone(
                    "fall-2",
                    duration * 0.24,
                    duration * 0.42,
                    Waveform::Sawtooth,
                    base * 1.18,
                    base * 0.46,
                    0.2,
                    intensity,
                    profile,
                ),
                noise(
                    "bad-edge",
                    0.0,
                    duration * 0.44,
                    0.07 + intensity * 0.08,
                    SfxFilterKind::Bandpass,
                    900.0,
                    360.0,
                    profile,
                ),
            ],
            "double" => vec![
                tone(
                    "buzz-1",
                    0.0,
                    duration * 0.3,
                    Waveform::Square,
                    base * 1.08,
                    base * 0.86,
                    0.24,
                    intensity,
                    profile,
                ),
                tone(
                    "rub-1",
                    0.0,
                    duration * 0.3,
                    Waveform::Sawtooth,
                    base * 1.16,
                    base * 0.78,
                    0.12,
                    intensity,
                    profile,
                ),
                tone(
                    "buzz-2",
                    duration * 0.46,
                    duration * 0.32,
                    Waveform::Square,
                    base * 0.9,
                    base * 0.58,
                    0.24,
                    intensity,
                    profile,
                ),
                click(duration * 0.44, 0.016, 0.06 + intensity * 0.08, 1800.0),
            ],
            "glitch" => vec![
                tone(
                    "glitch",
                    0.0,
                    duration * 0.55,
                    Waveform::Sawtooth,
                    base * 2.1,
                    base * 0.4,
                    0.18,
                    intensity,
                    profile,
                ),
                noise(
                    "static",
                    0.0,
                    duration * 0.82,
                    0.12 + intensity * 0.14,
                    SfxFilterKind::Bandpass,
                    2800.0,
                    740.0,
                    profile,
                ),
                click(duration * 0.16, 0.018, 0.08 + intensity * 0.08, 3400.0),
                click(duration * 0.36, 0.014, 0.06 + intensity * 0.07, 5200.0),
            ],
            _ => vec![
                tone(
                    "deny-low",
                    0.0,
                    duration * 0.72,
                    Waveform::Square,
                    base * 1.02,
                    base * 0.58,
                    0.28,
                    intensity,
                    profile,
                ),
                tone(
                    "deny-rub",
                    0.012,
                    duration * 0.68,
                    Waveform::Sawtooth,
                    base * 1.09,
                    base * 0.62,
                    0.16,
                    intensity,
                    profile,
                ),
                noise(
                    "deny-grit",
                    0.0,
                    duration * 0.5,
                    0.08 + intensity * 0.1,
                    SfxFilterKind::Bandpass,
                    760.0,
                    420.0,
                    profile,
                ),
                click(0.0, 0.014, 0.06 + intensity * 0.07, 1400.0),
            ],
        };
        vary_layers(rng, &mut layers, duration, base, intensity, profile);
        return layers;
    }

    if kind == "step" {
        return build_step_layers(rng, duration, base, intensity, profile);
    }
    if kind == "water" {
        return build_water_layers(rng, duration, base, intensity, profile);
    }
    if kind == "select" {
        return build_select_layers(rng, duration, base, intensity, profile);
    }
    if kind == "drag" {
        return build_drag_layers(rng, duration, intensity, profile);
    }
    if kind == "lock" {
        return build_lock_layers(rng, duration, base, intensity, profile);
    }

    unreachable!("validated SFX category was not handled")
}

fn build_step_layers(
    rng: &mut Mulberry32,
    duration: f64,
    base: f64,
    intensity: f64,
    profile: &SfxProfile,
) -> Vec<SfxLayer> {
    let mut layers = match profile.pattern.as_str() {
        "wood" => vec![
            click(0.0, 0.01, 0.09 + intensity * 0.07, ri(rng, 1800, 3600)),
            noise(
                "wood-sole",
                0.006,
                duration * 0.62,
                0.055 + intensity * 0.06,
                SfxFilterKind::Bandpass,
                ri(rng, 720, 1400),
                ri(rng, 260, 620),
                profile,
            ),
            tone(
                "foot-wood",
                0.0,
                duration * 0.58,
                Waveform::Triangle,
                base * 0.78,
                base * 0.46,
                0.08,
                intensity,
                profile,
            ),
        ],
        "stone" => vec![
            click(0.0, 0.012, 0.11 + intensity * 0.08, ri(rng, 2200, 4200)),
            noise(
                "stone-scuff",
                0.004,
                duration * 0.52,
                0.06 + intensity * 0.07,
                SfxFilterKind::Bandpass,
                ri(rng, 980, 1900),
                ri(rng, 320, 760),
                profile,
            ),
            tone(
                "foot-stone",
                0.004,
                duration * 0.42,
                Waveform::Sine,
                base * 0.72,
                base * 0.5,
                0.065,
                intensity,
                profile,
            ),
        ],
        "grass" => vec![
            noise(
                "grass-brush",
                0.0,
                duration * 0.72,
                0.075 + intensity * 0.07,
                SfxFilterKind::Bandpass,
                ri(rng, 1100, 2400),
                ri(rng, 360, 920),
                profile,
            ),
            noise(
                "grass-foot",
                duration * 0.12,
                duration * 0.48,
                0.045 + intensity * 0.04,
                SfxFilterKind::Lowpass,
                ri(rng, 680, 1100),
                ri(rng, 180, 360),
                profile,
            ),
        ],
        "heavy" => vec![
            tone(
                "foot-weight",
                0.0,
                duration * 0.7,
                Waveform::Sine,
                ri(rng, 72, 125),
                ri(rng, 42, 72),
                0.13,
                intensity,
                profile,
            ),
            noise(
                "sole-dust",
                0.012,
                duration * 0.48,
                0.055 + intensity * 0.06,
                SfxFilterKind::Lowpass,
                ri(rng, 520, 980),
                ri(rng, 120, 260),
                profile,
            ),
            click(0.0, 0.012, 0.06 + intensity * 0.055, ri(rng, 1000, 2200)),
        ],
        "soft" => vec![
            tone(
                "soft-foot",
                0.0,
                duration * 0.52,
                Waveform::Sine,
                base * 0.62,
                base * 0.5,
                0.055,
                intensity,
                profile,
            ),
            noise(
                "soft-sole",
                0.01,
                duration * 0.46,
                0.035 + intensity * 0.035,
                SfxFilterKind::Lowpass,
                ri(rng, 420, 820),
                ri(rng, 110, 260),
                profile,
            ),
        ],
        _ => vec![
            click(0.0, 0.009, 0.075 + intensity * 0.06, ri(rng, 1600, 3400)),
            tone(
                "foot-tap",
                0.002,
                duration * 0.48,
                Waveform::Triangle,
                base,
                base * lerp(0.62, 0.82, rng.uniform()),
                0.07,
                intensity,
                profile,
            ),
        ],
    };
    for layer in &mut layers {
        let jitter = if profile.variant == "soft" {
            lerp(0.72, 0.96, rng.uniform())
        } else {
            lerp(0.86, 1.12, rng.uniform())
        };
        match layer {
            SfxLayer::Tone {
                duration: d,
                frequency_start,
                frequency_end,
                gain,
                filter_frequency,
                wobble,
                ..
            } => {
                *d = round3((*d).min(duration * 0.82));
                *frequency_start = (*frequency_start * lerp(0.94, 1.06, rng.uniform())).round();
                *frequency_end = (*frequency_end * lerp(0.9, 1.08, rng.uniform())).round();
                *gain = round3(((*gain) * jitter).min(0.22));
                *filter_frequency = filter_frequency.clamp(360.0, 2600.0).round();
                *wobble = round2((*wobble).min(0.016));
            }
            SfxLayer::Noise {
                duration: d,
                filter_start,
                filter_end,
                gain,
                ..
            } => {
                *d = round3((*d).min(duration * 0.86));
                *filter_start = (*filter_start * lerp(0.9, 1.12, rng.uniform()))
                    .clamp(120.0, 2600.0)
                    .round();
                *filter_end = (*filter_end * lerp(0.88, 1.14, rng.uniform()))
                    .clamp(80.0, 1200.0)
                    .round();
                *gain = round3(((*gain) * jitter).min(0.2));
            }
            SfxLayer::Click {
                duration: d,
                gain,
                filter_frequency,
                ..
            } => {
                *d = round3((*d).min(0.014));
                *gain = round3(((*gain) * jitter).min(0.2));
                *filter_frequency = (*filter_frequency * lerp(0.9, 1.08, rng.uniform()))
                    .clamp(800.0, 4600.0)
                    .round();
            }
        }
    }
    match profile.variant.as_str() {
        "double" => layers.push(noise(
            "step-follow",
            duration * lerp(0.34, 0.48, rng.uniform()),
            duration * 0.28,
            0.026 + intensity * 0.025,
            SfxFilterKind::Lowpass,
            ri(rng, 480, 920),
            ri(rng, 130, 300),
            profile,
        )),
        "gravel" => layers.push(noise(
            "step-grit",
            duration * 0.12,
            duration * 0.42,
            0.035 + intensity * 0.04,
            SfxFilterKind::Bandpass,
            ri(rng, 900, 1800),
            ri(rng, 260, 680),
            profile,
        )),
        "heavy" => layers.push(tone(
            "step-mass",
            0.0,
            duration * 0.62,
            Waveform::Sine,
            ri(rng, 58, 88),
            ri(rng, 38, 58),
            0.07 + intensity * 0.04,
            intensity,
            profile,
        )),
        _ => {}
    }
    for layer in &mut layers {
        if let SfxLayer::Tone {
            start,
            duration: d,
            release,
            ..
        } = layer
        {
            *d = round3((*d).min((duration - *start).max(0.025)));
            *release = round3((*release).min(duration * 0.32));
        }
    }
    sort_layers(&mut layers);
    layers
}

fn build_water_layers(
    rng: &mut Mulberry32,
    duration: f64,
    base: f64,
    intensity: f64,
    profile: &SfxProfile,
) -> Vec<SfxLayer> {
    let mut layers = match profile.pattern.as_str() {
        "plop" => vec![
            tone(
                "water-plop",
                0.0,
                duration * 0.48,
                Waveform::Sine,
                base * 0.92,
                base * 0.48,
                0.15,
                intensity,
                profile,
            ),
            noise(
                "plop-ring",
                duration * 0.05,
                duration * 0.52,
                0.095 + intensity * 0.08,
                SfxFilterKind::Bandpass,
                ri(rng, 720, 1500),
                ri(rng, 180, 420),
                profile,
            ),
            noise(
                "water-tail",
                duration * 0.28,
                duration * 0.52,
                0.045 + intensity * 0.04,
                SfxFilterKind::Lowpass,
                ri(rng, 380, 760),
                ri(rng, 80, 180),
                profile,
            ),
        ],
        "ripple" => vec![
            noise(
                "water-ripple",
                0.0,
                duration * 0.86,
                0.075 + intensity * 0.055,
                SfxFilterKind::Bandpass,
                ri(rng, 520, 980),
                ri(rng, 180, 420),
                profile,
            ),
            tone(
                "ripple-ring",
                duration * 0.08,
                duration * 0.48,
                Waveform::Sine,
                base * 1.25,
                base * 0.92,
                0.055,
                intensity,
                profile,
            ),
        ],
        "bubble" => vec![
            tone(
                "bubble-1",
                0.0,
                duration * 0.24,
                Waveform::Sine,
                base * 1.5,
                base * 1.9,
                0.075,
                intensity,
                profile,
            ),
            tone(
                "bubble-2",
                duration * lerp(0.16, 0.28, rng.uniform()),
                duration * 0.22,
                Waveform::Sine,
                base * 1.2,
                base * 1.7,
                0.065,
                intensity,
                profile,
            ),
            noise(
                "bubble-fizz",
                0.0,
                duration * 0.68,
                0.055 + intensity * 0.055,
                SfxFilterKind::Bandpass,
                ri(rng, 900, 2100),
                ri(rng, 360, 900),
                profile,
            ),
        ],
        "pour" => vec![
            noise(
                "water-pour",
                0.0,
                duration * 0.96,
                0.12 + intensity * 0.1,
                SfxFilterKind::Bandpass,
                ri(rng, 640, 1400),
                ri(rng, 180, 420),
                profile,
            ),
            noise(
                "pour-spray",
                duration * 0.08,
                duration * 0.64,
                0.055 + intensity * 0.05,
                SfxFilterKind::Bandpass,
                ri(rng, 1600, 3200),
                ri(rng, 720, 1300),
                profile,
            ),
            tone(
                "basin-body",
                duration * 0.12,
                duration * 0.52,
                Waveform::Sine,
                base * 0.72,
                base * 0.52,
                0.065,
                intensity,
                profile,
            ),
        ],
        "drip" => vec![
            tone(
                "water-drip",
                0.0,
                duration * 0.26,
                Waveform::Sine,
                base * 1.8,
                base * 1.1,
                0.095,
                intensity,
                profile,
            ),
            noise(
                "drip-ring",
                duration * 0.04,
                duration * 0.5,
                0.045 + intensity * 0.04,
                SfxFilterKind::Bandpass,
                ri(rng, 740, 1500),
                ri(rng, 220, 520),
                profile,
            ),
        ],
        _ => vec![
            noise(
                "water-splash",
                0.0,
                duration * 0.72,
                0.16 + intensity * 0.14,
                SfxFilterKind::Bandpass,
                ri(rng, 900, 2200),
                ri(rng, 220, 560),
                profile,
            ),
            noise(
                "splash-spray",
                0.0,
                duration * 0.38,
                0.08 + intensity * 0.08,
                SfxFilterKind::Bandpass,
                ri(rng, 2200, 4200),
                ri(rng, 900, 1600),
                profile,
            ),
            tone(
                "water-body",
                0.01,
                duration * 0.44,
                Waveform::Sine,
                base,
                base * 0.52,
                0.105,
                intensity,
                profile,
            ),
            noise(
                "water-tail",
                duration * 0.36,
                duration * 0.5,
                0.05 + intensity * 0.045,
                SfxFilterKind::Lowpass,
                ri(rng, 420, 820),
                ri(rng, 90, 220),
                profile,
            ),
        ],
    };
    for layer in &mut layers {
        let jitter = if profile.variant == "soft" {
            lerp(0.72, 0.98, rng.uniform())
        } else {
            lerp(0.88, 1.16, rng.uniform())
        };
        match layer {
            SfxLayer::Tone {
                frequency_start,
                frequency_end,
                gain,
                filter_frequency,
                ..
            } => {
                *frequency_start = (*frequency_start * lerp(0.92, 1.08, rng.uniform())).round();
                *frequency_end = (*frequency_end * lerp(0.88, 1.12, rng.uniform())).round();
                *gain = round3(((*gain) * jitter).min(0.24));
                *filter_frequency = filter_frequency.clamp(320.0, 3000.0).round();
            }
            SfxLayer::Noise {
                filter_start,
                filter_end,
                gain,
                ..
            } => {
                *filter_start = (*filter_start * lerp(0.82, 1.22, rng.uniform()))
                    .clamp(80.0, 5200.0)
                    .round();
                *filter_end = (*filter_end * lerp(0.82, 1.22, rng.uniform()))
                    .clamp(60.0, 2400.0)
                    .round();
                *gain = round3(((*gain) * jitter).min(0.34));
            }
            SfxLayer::Click { gain, .. } => *gain = round3(((*gain) * jitter).min(0.16)),
        }
    }
    match profile.variant.as_str() {
        "deep" => layers.push(tone(
            "water-depth",
            0.0,
            duration * 0.58,
            Waveform::Sine,
            ri(rng, 54, 92),
            ri(rng, 38, 64),
            0.08 + intensity * 0.05,
            intensity,
            profile,
        )),
        "bubbly" => layers.push(tone(
            "bubble-extra",
            duration * lerp(0.32, 0.56, rng.uniform()),
            duration * 0.18,
            Waveform::Sine,
            ri(rng, 240, 520),
            ri(rng, 380, 760),
            0.045 + intensity * 0.035,
            intensity,
            profile,
        )),
        "choppy" => layers.push(noise(
            "water-chop",
            duration * lerp(0.16, 0.34, rng.uniform()),
            duration * 0.34,
            0.055 + intensity * 0.055,
            SfxFilterKind::Bandpass,
            ri(rng, 1200, 2600),
            ri(rng, 420, 940),
            profile,
        )),
        "wide" => layers.push(noise(
            "wide-ripple",
            duration * 0.24,
            duration * 0.58,
            0.04 + intensity * 0.035,
            SfxFilterKind::Bandpass,
            ri(rng, 460, 900),
            ri(rng, 140, 320),
            profile,
        )),
        _ => {}
    }
    sort_layers(&mut layers);
    layers
}

fn build_select_layers(
    rng: &mut Mulberry32,
    duration: f64,
    base: f64,
    intensity: f64,
    profile: &SfxProfile,
) -> Vec<SfxLayer> {
    let mut layers = match profile.pattern.as_str() {
        "cursor" => vec![
            click(0.0, 0.008, 0.085 + intensity * 0.055, ri(rng, 4200, 7200)),
            tone(
                "ui-pip",
                0.004,
                duration * lerp(0.38, 0.56, rng.uniform()),
                Waveform::Triangle,
                base,
                base * lerp(1.02, 1.12, rng.uniform()),
                0.085,
                intensity,
                profile,
            ),
        ],
        "press" => vec![
            click(0.0, 0.01, 0.075 + intensity * 0.05, ri(rng, 3600, 6200)),
            tone(
                "ui-press",
                0.006,
                duration * lerp(0.44, 0.62, rng.uniform()),
                Waveform::Sine,
                base * 0.82,
                base * lerp(0.72, 0.86, rng.uniform()),
                0.075,
                intensity,
                profile,
            ),
        ],
        "soft" => vec![tone(
            "ui-soft",
            0.0,
            duration * lerp(0.48, 0.68, rng.uniform()),
            Waveform::Sine,
            base * 0.76,
            base * lerp(0.74, 0.82, rng.uniform()),
            0.07,
            intensity,
            profile,
        )],
        _ => vec![
            tone(
                "ui-blip",
                0.0,
                duration * lerp(0.42, 0.6, rng.uniform()),
                Waveform::Triangle,
                base,
                base * lerp(1.22, 1.42, rng.uniform()),
                0.095,
                intensity,
                profile,
            ),
            click(0.0, 0.007, 0.055 + intensity * 0.045, ri(rng, 4800, 7600)),
        ],
    };
    for layer in &mut layers {
        let jitter = lerp(0.88, 1.08, rng.uniform());
        match layer {
            SfxLayer::Tone {
                duration: d,
                frequency_start,
                frequency_end,
                gain,
                wobble,
                ..
            } => {
                *d = round3((*d).min(duration * 0.72));
                *frequency_start = (*frequency_start * lerp(0.97, 1.04, rng.uniform())).round();
                *frequency_end = (*frequency_end * lerp(0.97, 1.04, rng.uniform())).round();
                *gain = round3(((*gain) * jitter).min(0.18));
                *wobble = round2((*wobble).min(0.012));
            }
            SfxLayer::Click {
                duration: d,
                gain,
                filter_frequency,
                ..
            } => {
                *d = round3((*d).min(0.012));
                *gain = round3(((*gain) * jitter).min(0.16));
                *filter_frequency = (*filter_frequency * lerp(0.92, 1.08, rng.uniform()))
                    .clamp(3200.0, 8200.0)
                    .round();
            }
            SfxLayer::Noise { .. } => unreachable!("select generator has no base noise layer"),
        }
    }
    if profile.variant == "double" || profile.variant == "stepped" {
        layers.push(click(
            duration * lerp(0.2, 0.34, rng.uniform()),
            0.006,
            0.035 + intensity * 0.035,
            ri(rng, 3600, 6800),
        ));
    } else if profile.variant == "wide" {
        let mut air_profile = profile.clone();
        air_profile.filter_bias = 1.1;
        air_profile.pitch_wobble = 0.0;
        layers.push(tone(
            "ui-air",
            duration * 0.08,
            duration * 0.34,
            Waveform::Sine,
            ri(rng, 1500, 2200),
            ri(rng, 1500, 2400),
            0.035,
            intensity,
            &air_profile,
        ));
    }
    for layer in &mut layers {
        if let SfxLayer::Tone {
            start,
            duration: d,
            release,
            filter_frequency,
            ..
        } = layer
        {
            *d = round3((*d).min((duration - *start).max(0.025)));
            *release = round3((*release).min(duration * 0.28));
            *filter_frequency = filter_frequency.clamp(2600.0, 7800.0).round();
        }
    }
    sort_layers(&mut layers);
    layers
}

#[derive(Clone, Copy)]
struct DragMaterial {
    filter: SfxFilterKind,
    stiction_start: f64,
    stiction_end: f64,
    rub_start: f64,
    rub_end: f64,
    release_start: f64,
    release_end: f64,
    secondary: Option<(&'static str, f64, f64, f64, f64, f64)>,
}

fn drag_material(pattern: &str) -> DragMaterial {
    match pattern {
        "stone-floor" => DragMaterial {
            filter: SfxFilterKind::Lowpass,
            stiction_start: 980.0,
            stiction_end: 340.0,
            rub_start: 760.0,
            rub_end: 170.0,
            release_start: 340.0,
            release_end: 90.0,
            secondary: Some(("stone-grit", 0.18, 0.46, 0.045, 1180.0, 440.0)),
        },
        "rough-floor" => DragMaterial {
            filter: SfxFilterKind::Bandpass,
            stiction_start: 1180.0,
            stiction_end: 420.0,
            rub_start: 980.0,
            rub_end: 260.0,
            release_start: 400.0,
            release_end: 110.0,
            secondary: Some(("rough-grain", 0.16, 0.56, 0.06, 1500.0, 620.0)),
        },
        "stuck-start" => DragMaterial {
            filter: SfxFilterKind::Bandpass,
            stiction_start: 1050.0,
            stiction_end: 360.0,
            rub_start: 860.0,
            rub_end: 220.0,
            release_start: 360.0,
            release_end: 100.0,
            secondary: Some(("stall-rub", 0.08, 0.38, 0.06, 1020.0, 320.0)),
        },
        "short-pull" => DragMaterial {
            filter: SfxFilterKind::Bandpass,
            stiction_start: 920.0,
            stiction_end: 320.0,
            rub_start: 820.0,
            rub_end: 240.0,
            release_start: 340.0,
            release_end: 95.0,
            secondary: None,
        },
        "soft-floor" => DragMaterial {
            filter: SfxFilterKind::Lowpass,
            stiction_start: 640.0,
            stiction_end: 220.0,
            rub_start: 540.0,
            rub_end: 150.0,
            release_start: 260.0,
            release_end: 80.0,
            secondary: Some(("soft-dust", 0.22, 0.42, 0.035, 520.0, 170.0)),
        },
        _ => DragMaterial {
            filter: SfxFilterKind::Bandpass,
            stiction_start: 820.0,
            stiction_end: 280.0,
            rub_start: 700.0,
            rub_end: 210.0,
            release_start: 300.0,
            release_end: 90.0,
            secondary: Some(("wood-grain", 0.2, 0.5, 0.045, 860.0, 260.0)),
        },
    }
}

fn build_drag_layers(
    rng: &mut Mulberry32,
    duration: f64,
    intensity: f64,
    profile: &SfxProfile,
) -> Vec<SfxLayer> {
    let material = drag_material(&profile.pattern);
    let release_at = duration * lerp(0.7, 0.82, rng.uniform());
    let stiction_duration = duration
        * if profile.variant == "stuck" {
            lerp(0.18, 0.26, rng.uniform())
        } else {
            lerp(0.1, 0.17, rng.uniform())
        };
    let rub_start = duration * lerp(0.035, 0.08, rng.uniform());
    let rub_duration =
        (duration * 0.76).max(release_at + duration * lerp(0.12, 0.22, rng.uniform()) - rub_start);
    let body_start = ri(rng, 54, 112);
    let body_end = ri(rng, 36, 68);
    let mut layers = vec![
        noise(
            "stiction-break",
            0.0,
            stiction_duration,
            (if profile.variant == "stuck" {
                0.13
            } else {
                0.075
            }) + intensity * 0.065,
            SfxFilterKind::Bandpass,
            material.stiction_start,
            material.stiction_end,
            profile,
        ),
        noise(
            "floor-rub",
            rub_start,
            rub_duration,
            (if profile.variant == "soft" {
                0.11
            } else {
                0.15
            }) + intensity * 0.13,
            material.filter,
            material.rub_start,
            material.rub_end,
            profile,
        ),
        tone(
            "crate-body",
            0.0,
            release_at + duration * 0.1,
            Waveform::Sine,
            body_start,
            body_end,
            (if profile.variant == "heavy" {
                0.18
            } else {
                0.13
            }) + intensity * 0.075,
            intensity,
            profile,
        ),
        noise(
            "release-dust",
            release_at,
            duration * lerp(0.18, 0.3, rng.uniform()),
            0.035 + intensity * 0.045,
            SfxFilterKind::Lowpass,
            material.release_start,
            material.release_end,
            profile,
        ),
    ];
    if let Some((name, start, length, gain, from, to)) = material.secondary {
        layers.push(noise(
            name,
            duration * start,
            duration * length,
            gain + intensity * 0.05,
            SfxFilterKind::Bandpass,
            from,
            to,
            profile,
        ));
    }
    if profile.pattern == "stuck-start" {
        layers.push(tone(
            "crate-strain",
            duration * 0.04,
            duration * 0.3,
            Waveform::Triangle,
            ri(rng, 120, 180),
            ri(rng, 70, 105),
            0.045 + intensity * 0.035,
            intensity,
            profile,
        ));
    }
    for layer in &mut layers {
        let jitter = if profile.variant == "soft" {
            lerp(0.72, 0.96, rng.uniform())
        } else {
            lerp(0.9, 1.16, rng.uniform())
        };
        match layer {
            SfxLayer::Tone {
                frequency_start,
                frequency_end,
                gain,
                ..
            } => {
                *frequency_start = (*frequency_start * lerp(0.95, 1.04, rng.uniform())).round();
                *frequency_end = (*frequency_end * lerp(0.92, 1.06, rng.uniform())).round();
                *gain = round3(*gain * jitter);
            }
            SfxLayer::Noise {
                filter_start,
                filter_end,
                gain,
                ..
            } => {
                *filter_start = (*filter_start * lerp(0.9, 1.12, rng.uniform())).round();
                *filter_end = (*filter_end * lerp(0.88, 1.14, rng.uniform())).round();
                *gain = round3(*gain * jitter);
            }
            SfxLayer::Click { gain, .. } => *gain = round3(*gain * jitter),
        }
    }
    match profile.variant.as_str() {
        "grainy" | "rough" => layers.push(noise(
            "loose-grit",
            duration * 0.18,
            duration * 0.46,
            0.045 + intensity * 0.055,
            SfxFilterKind::Bandpass,
            ri(rng, 980, 1500),
            ri(rng, 360, 680),
            profile,
        )),
        "heavy" => layers.push(tone(
            "crate-weight",
            0.0,
            release_at + duration * 0.08,
            Waveform::Sine,
            ri(rng, 38, 62),
            ri(rng, 28, 42),
            0.075 + intensity * 0.055,
            intensity,
            profile,
        )),
        "stuck" => layers.push(noise(
            "stiction-hold",
            duration * 0.08,
            duration * 0.24,
            0.045 + intensity * 0.045,
            SfxFilterKind::Bandpass,
            ri(rng, 720, 1100),
            ri(rng, 220, 420),
            profile,
        )),
        _ => {}
    }
    sort_layers(&mut layers);
    layers
}

#[derive(Clone, Copy)]
struct LockStop {
    impact_gain: f64,
    impact_filter: f64,
    clack_mul: f64,
    body_gain: f64,
    thump_gain: f64,
    thump_filter: f64,
}

fn lock_stop_layers(
    stop: f64,
    duration: f64,
    base: f64,
    intensity: f64,
    profile: &SfxProfile,
    options: LockStop,
) -> Vec<SfxLayer> {
    let ring = base * options.clack_mul;
    vec![
        click(stop, 0.013, options.impact_gain, options.impact_filter),
        tone(
            "lock-body",
            stop,
            duration * 0.26,
            Waveform::Sine,
            98.0,
            60.0,
            options.body_gain,
            intensity,
            profile,
        ),
        tone(
            "lock-stop",
            stop,
            duration * 0.15,
            Waveform::Triangle,
            ring,
            ring * 0.84,
            0.13,
            intensity,
            profile,
        ),
        tone(
            "lock-mode-2",
            stop,
            duration * 0.09,
            Waveform::Sine,
            ring * 1.73,
            ring * 1.5,
            0.075,
            intensity,
            profile,
        ),
        tone(
            "lock-mode-3",
            stop,
            duration * 0.055,
            Waveform::Sine,
            ring * 2.62,
            ring * 2.3,
            0.045,
            intensity,
            profile,
        ),
        noise(
            "case-thump",
            stop,
            duration * 0.2,
            options.thump_gain,
            SfxFilterKind::Lowpass,
            options.thump_filter,
            options.thump_filter * 0.32,
            profile,
        ),
    ]
}

fn build_lock_layers(
    rng: &mut Mulberry32,
    duration: f64,
    base: f64,
    intensity: f64,
    profile: &SfxProfile,
) -> Vec<SfxLayer> {
    let mut layers = match profile.pattern.as_str() {
        "deadbolt" => {
            let mut value = vec![
                click(0.0, 0.012, 0.1 + intensity * 0.08, 3600.0),
                noise(
                    "bolt-drag",
                    duration * 0.1,
                    duration * 0.42,
                    0.1 + intensity * 0.1,
                    SfxFilterKind::Bandpass,
                    2400.0,
                    460.0,
                    profile,
                ),
                tone(
                    "bolt-slide",
                    duration * 0.16,
                    duration * 0.32,
                    Waveform::Triangle,
                    base * 0.9,
                    base * 0.6,
                    0.07,
                    intensity,
                    profile,
                ),
            ];
            value.extend(lock_stop_layers(
                duration * 0.62,
                duration,
                base,
                intensity,
                profile,
                LockStop {
                    impact_gain: 0.36,
                    impact_filter: 2400.0,
                    clack_mul: 0.88,
                    body_gain: 0.28,
                    thump_gain: 0.24,
                    thump_filter: 820.0,
                },
            ));
            value
        }
        "key-turn" => {
            let mut value = vec![
                click(0.0, 0.009, 0.08 + intensity * 0.07, 5200.0),
                click(duration * 0.18, 0.01, 0.08 + intensity * 0.07, 4200.0),
                noise(
                    "key-scrape",
                    duration * 0.2,
                    duration * 0.26,
                    0.055 + intensity * 0.06,
                    SfxFilterKind::Bandpass,
                    3600.0,
                    1400.0,
                    profile,
                ),
                tone(
                    "key-turn",
                    duration * 0.26,
                    duration * 0.24,
                    Waveform::Triangle,
                    base * 1.3,
                    base * 0.7,
                    0.06,
                    intensity,
                    profile,
                ),
            ];
            value.extend(lock_stop_layers(
                duration * 0.56,
                duration,
                base,
                intensity,
                profile,
                LockStop {
                    impact_gain: 0.32,
                    impact_filter: 2600.0,
                    clack_mul: 0.84,
                    body_gain: 0.26,
                    thump_gain: 0.22,
                    thump_filter: 780.0,
                },
            ));
            value
        }
        "tumblers" => {
            let mut value = vec![
                click(0.0, 0.008, 0.07 + intensity * 0.06, 5400.0),
                click(duration * 0.15, 0.008, 0.065 + intensity * 0.06, 4800.0),
                click(duration * 0.3, 0.009, 0.07 + intensity * 0.07, 4200.0),
                noise(
                    "pin-scrape",
                    duration * 0.22,
                    duration * 0.24,
                    0.05 + intensity * 0.055,
                    SfxFilterKind::Bandpass,
                    3400.0,
                    1200.0,
                    profile,
                ),
            ];
            value.extend(lock_stop_layers(
                duration * 0.6,
                duration,
                base,
                intensity,
                profile,
                LockStop {
                    impact_gain: 0.32,
                    impact_filter: 2500.0,
                    clack_mul: 0.8,
                    body_gain: 0.26,
                    thump_gain: 0.22,
                    thump_filter: 800.0,
                },
            ));
            value
        }
        "old-lock" => {
            let mut value = vec![
                click(0.0, 0.013, 0.11 + intensity * 0.08, 4200.0),
                noise(
                    "old-bolt-grind",
                    duration * 0.08,
                    duration * 0.5,
                    0.12 + intensity * 0.12,
                    SfxFilterKind::Bandpass,
                    2000.0,
                    360.0,
                    profile,
                ),
                click(duration * 0.28, 0.013, 0.12 + intensity * 0.09, 3200.0),
                tone(
                    "old-case",
                    duration * 0.3,
                    duration * 0.32,
                    Waveform::Triangle,
                    base * 0.85,
                    base * 0.46,
                    0.1,
                    intensity,
                    profile,
                ),
            ];
            value.extend(lock_stop_layers(
                duration * 0.72,
                duration,
                base,
                intensity,
                profile,
                LockStop {
                    impact_gain: 0.4,
                    impact_filter: 2000.0,
                    clack_mul: 0.72,
                    body_gain: 0.32,
                    thump_gain: 0.28,
                    thump_filter: 700.0,
                },
            ));
            value
        }
        "padlock" => {
            let mut value = vec![
                click(0.0, 0.009, 0.09 + intensity * 0.08, 5600.0),
                tone(
                    "shackle-snap",
                    duration * 0.12,
                    duration * 0.2,
                    Waveform::Triangle,
                    base * 2.1,
                    base * 1.1,
                    0.08,
                    intensity,
                    profile,
                ),
                noise(
                    "metal-shell",
                    duration * 0.14,
                    duration * 0.22,
                    0.085 + intensity * 0.1,
                    SfxFilterKind::Bandpass,
                    4200.0,
                    1000.0,
                    profile,
                ),
            ];
            value.extend(lock_stop_layers(
                duration * 0.5,
                duration,
                base,
                intensity,
                profile,
                LockStop {
                    impact_gain: 0.32,
                    impact_filter: 2900.0,
                    clack_mul: 1.0,
                    body_gain: 0.24,
                    thump_gain: 0.2,
                    thump_filter: 900.0,
                },
            ));
            value.push(tone(
                "metal-ring",
                duration * 0.56,
                duration * 0.26,
                Waveform::Triangle,
                base * 3.1,
                base * 2.2,
                0.05,
                intensity,
                profile,
            ));
            value
        }
        _ => {
            let mut value = vec![
                click(0.0, 0.01, 0.09 + intensity * 0.07, 4600.0),
                noise(
                    "latch-scrape",
                    duration * 0.16,
                    duration * 0.24,
                    0.07 + intensity * 0.08,
                    SfxFilterKind::Bandpass,
                    2800.0,
                    720.0,
                    profile,
                ),
                click(duration * 0.42, 0.011, 0.13 + intensity * 0.1, 3600.0),
            ];
            value.extend(lock_stop_layers(
                duration * 0.54,
                duration,
                base,
                intensity,
                profile,
                LockStop {
                    impact_gain: 0.34,
                    impact_filter: 2600.0,
                    clack_mul: 0.86,
                    body_gain: 0.26,
                    thump_gain: 0.22,
                    thump_filter: 820.0,
                },
            ));
            value
        }
    };
    for layer in &mut layers {
        let jitter = lerp(0.96, 1.22, rng.uniform());
        match layer {
            SfxLayer::Tone {
                frequency_start,
                frequency_end,
                gain,
                ..
            } => {
                *frequency_start = (*frequency_start * lerp(0.94, 1.05, rng.uniform())).round();
                *frequency_end = (*frequency_end * lerp(0.9, 1.08, rng.uniform())).round();
                *gain = round3(*gain * jitter);
            }
            SfxLayer::Noise {
                filter_start,
                filter_end,
                gain,
                ..
            } => {
                *filter_start = (*filter_start * lerp(0.88, 1.16, rng.uniform())).round();
                *filter_end = (*filter_end * lerp(0.84, 1.18, rng.uniform())).round();
                *gain = round3(*gain * jitter);
            }
            SfxLayer::Click { gain, .. } => *gain = round3(*gain * jitter),
        }
    }
    let stop_start = layers
        .iter()
        .find_map(|layer| match layer {
            SfxLayer::Tone { name, start, .. } if name == "lock-stop" => Some(*start),
            _ => None,
        })
        .unwrap_or(duration * 0.56);
    match profile.variant.as_str() {
        "double" => layers.push(click(
            stop_start + duration * lerp(0.05, 0.12, rng.uniform()),
            0.01,
            0.08 + intensity * 0.09,
            ri(rng, 2600, 5200),
        )),
        "gritty" => layers.push(noise(
            "lock-grit",
            duration * 0.18,
            duration * 0.28,
            0.06 + intensity * 0.08,
            SfxFilterKind::Bandpass,
            ri(rng, 2600, 5200),
            ri(rng, 520, 1200),
            profile,
        )),
        "stepped" => layers.push(click(
            duration * lerp(0.26, 0.46, rng.uniform()),
            0.009,
            0.08 + intensity * 0.08,
            ri(rng, 4200, 7200),
        )),
        "heavy" => {
            layers.push(tone(
                "lock-mass",
                stop_start,
                duration * 0.24,
                Waveform::Triangle,
                ri(rng, 95, 145),
                ri(rng, 48, 82),
                0.16 + intensity * 0.08,
                intensity,
                profile,
            ));
            layers.push(noise(
                "lock-wood-hit",
                stop_start,
                duration * 0.2,
                0.1 + intensity * 0.12,
                SfxFilterKind::Lowpass,
                ri(rng, 700, 1200),
                ri(rng, 180, 360),
                profile,
            ));
        }
        "stuck" => {
            layers.push(noise(
                "stuck-scrape",
                duration * 0.08,
                duration * 0.42,
                0.07 + intensity * 0.09,
                SfxFilterKind::Bandpass,
                ri(rng, 1800, 3800),
                ri(rng, 360, 900),
                profile,
            ));
            layers.push(click(
                duration * lerp(0.34, 0.5, rng.uniform()),
                0.012,
                0.08 + intensity * 0.08,
                ri(rng, 3200, 6000),
            ));
        }
        _ => {}
    }
    sort_layers(&mut layers);
    layers
}

fn tone(
    name: &str,
    start: f64,
    duration: f64,
    waveform: Waveform,
    from: f64,
    to: f64,
    gain: f64,
    intensity: f64,
    profile: &SfxProfile,
) -> SfxLayer {
    SfxLayer::Tone {
        name: name.to_string(),
        start: round3(start),
        duration: round3(duration.max(0.025)),
        waveform,
        frequency_start: from.round(),
        frequency_end: to.round(),
        gain: round3(gain * lerp(0.62, 1.22, intensity)),
        attack: 0.006,
        release: round3((duration * 0.32).max(0.018)),
        filter_frequency: (lerp(900.0, 4200.0, intensity) * profile.filter_bias).round(),
        wobble: profile.pitch_wobble,
    }
}

fn noise(
    name: &str,
    start: f64,
    duration: f64,
    gain: f64,
    filter: SfxFilterKind,
    from: f64,
    to: f64,
    profile: &SfxProfile,
) -> SfxLayer {
    SfxLayer::Noise {
        name: name.to_string(),
        color: profile.noise_color,
        start: round3(start),
        duration: round3(duration.max(0.02)),
        gain: round3(gain),
        attack: 0.004,
        release: round3((duration * 0.45).max(0.018)),
        filter,
        filter_start: from.round(),
        filter_end: to.round(),
    }
}

fn click(start: f64, duration: f64, gain: f64, filter_frequency: f64) -> SfxLayer {
    SfxLayer::Click {
        start: round3(start),
        duration: round3(duration),
        gain: round3(gain),
        filter_frequency: filter_frequency.round(),
    }
}

fn vary_layers(
    rng: &mut Mulberry32,
    layers: &mut Vec<SfxLayer>,
    duration: f64,
    base: f64,
    intensity: f64,
    profile: &SfxProfile,
) {
    for layer in layers.iter_mut() {
        let gain_jitter = lerp(0.84, 1.18, rng.uniform());
        match layer {
            SfxLayer::Tone {
                frequency_start,
                frequency_end,
                gain,
                ..
            } => {
                *frequency_start = (*frequency_start * lerp(0.92, 1.09, rng.uniform())).round();
                *frequency_end = (*frequency_end * lerp(0.9, 1.12, rng.uniform())).round();
                *gain = round3(*gain * gain_jitter);
            }
            SfxLayer::Noise {
                filter_start,
                filter_end,
                gain,
                ..
            } => {
                *filter_start = (*filter_start * lerp(0.74, 1.32, rng.uniform())).round();
                *filter_end = (*filter_end * lerp(0.74, 1.32, rng.uniform())).round();
                *gain = round3(*gain * gain_jitter);
            }
            SfxLayer::Click { gain, .. } => *gain = round3(*gain * gain_jitter),
        }
    }
    match profile.variant.as_str() {
        "double" => layers.push(tone(
            "ghost",
            duration * lerp(0.18, 0.42, rng.uniform()),
            duration * 0.24,
            profile.waveform,
            base * 1.5,
            base * lerp(1.2, 2.1, rng.uniform()),
            0.09,
            intensity,
            profile,
        )),
        "gritty" => layers.push(noise(
            "grit-tail",
            duration * 0.12,
            duration * 0.38,
            0.08 + intensity * 0.08,
            SfxFilterKind::Bandpass,
            1800.0,
            520.0,
            profile,
        )),
        "hollow" => {
            for layer in layers.iter_mut() {
                if let SfxLayer::Tone { waveform, gain, .. } = layer {
                    *waveform = Waveform::Sine;
                    *gain = round3(*gain * 0.82);
                }
            }
        }
        "wide" => layers.push(tone(
            "upper",
            duration * 0.04,
            duration * 0.5,
            Waveform::Triangle,
            base * 2.0,
            base * lerp(1.6, 2.8, rng.uniform()),
            0.08,
            intensity,
            profile,
        )),
        "stepped" => layers.push(click(
            duration * lerp(0.22, 0.56, rng.uniform()),
            0.018,
            0.06 + intensity * 0.08,
            3600.0 * profile.filter_bias,
        )),
        _ => {}
    }
    layers.sort_by(|left, right| {
        layer_start(left)
            .total_cmp(&layer_start(right))
            .then_with(|| layer_name(left).cmp(layer_name(right)))
    });
}

fn render_layers(
    duration: f64,
    layers: &[SfxLayer],
    volume: f64,
) -> Result<GeneratedSfxClip, String> {
    const RATE: u32 = 48_000;
    let audible_end = layers.iter().map(layer_end).fold(duration, f64::max);
    let mut samples = vec![0_f32; (audible_end * f64::from(RATE)).ceil() as usize + 1];
    for layer in layers {
        let start = (layer_start(layer) * f64::from(RATE)).round() as usize;
        let count = (layer_duration(layer) * f64::from(RATE)).round() as usize;
        let mut noise_rng = Mulberry32::from_text(&layer_seed(layer));
        let mut phase = 0.0;
        let mut filter = CanonicalBiquad::default();
        for index in 0..count {
            let t = index as f64 / f64::from(RATE);
            let progress = index as f64 / count.max(1) as f64;
            let (raw, gain, filter_kind, filter_frequency) = match layer {
                SfxLayer::Tone {
                    waveform,
                    frequency_start,
                    frequency_end,
                    gain,
                    attack,
                    release,
                    filter_frequency,
                    wobble,
                    ..
                } => {
                    let frequency = frequency_start
                        * (frequency_end / frequency_start.max(20.0)).powf(progress);
                    let detune_octaves = if progress <= 0.35 {
                        wobble * progress / 0.35
                    } else {
                        wobble * (1.0 - progress) / 0.65
                    };
                    phase += 2.0 * PI * frequency * 2_f64.powf(detune_octaves) / f64::from(RATE);
                    (
                        wave_sample(*waveform, phase),
                        web_audio_layer_gain(t, layer_duration(layer), *attack, *release, *gain),
                        SfxFilterKind::Lowpass,
                        *filter_frequency,
                    )
                }
                SfxLayer::Noise {
                    color,
                    gain,
                    attack,
                    release,
                    filter: filter_kind,
                    filter_start,
                    filter_end,
                    ..
                } => {
                    let white = noise_rng.uniform() * 2.0 - 1.0;
                    let value = if *color == NoiseColor::Crackle && noise_rng.uniform() > 0.72 {
                        white
                    } else {
                        white * 0.55
                    };
                    (
                        value,
                        web_audio_layer_gain(t, layer_duration(layer), *attack, *release, *gain),
                        *filter_kind,
                        exponential_lerp(*filter_start, *filter_end, progress),
                    )
                }
                SfxLayer::Click {
                    gain,
                    filter_frequency,
                    ..
                } => (
                    noise_rng.uniform() * 2.0 - 1.0,
                    (-120.0 * t).exp() * exponential_lerp(*gain, 0.0001, progress),
                    SfxFilterKind::Highpass,
                    *filter_frequency,
                ),
            };
            let filtered = filter.process(raw, filter_kind, filter_frequency, f64::from(RATE));
            if let Some(output) = samples.get_mut(start + index) {
                *output += (filtered * gain * volume) as f32;
            }
        }
    }
    Ok(GeneratedSfxClip {
        sample_rate: RATE,
        samples: samples.into(),
    })
}

#[derive(Default)]
struct CanonicalBiquad {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl CanonicalBiquad {
    fn process(
        &mut self,
        input: f64,
        kind: SfxFilterKind,
        frequency: f64,
        sample_rate: f64,
    ) -> f64 {
        // The authored recipes use Web Audio's default biquad Q. The Rust renderer
        // owns one deterministic realization so every backend consumes the same PCM.
        let cutoff = frequency.clamp(20.0, sample_rate * 0.5 - 1.0);
        let omega = 2.0 * PI * cutoff / sample_rate;
        let sin = omega.sin();
        let cos = omega.cos();
        let alpha = sin / 2.0;
        let (b0, b1, b2) = match kind {
            SfxFilterKind::Lowpass => ((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0),
            SfxFilterKind::Highpass => ((1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0),
            SfxFilterKind::Bandpass => (alpha, 0.0, -alpha),
        };
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos / a0;
        let a2 = (1.0 - alpha) / a0;
        let output =
            b0 / a0 * input + b1 / a0 * self.x1 + b2 / a0 * self.x2 - a1 * self.y1 - a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

fn exponential_lerp(start: f64, end: f64, progress: f64) -> f64 {
    start.max(20.0) * (end.max(20.0) / start.max(20.0)).powf(progress)
}

fn layer_start(layer: &SfxLayer) -> f64 {
    match layer {
        SfxLayer::Tone { start, .. }
        | SfxLayer::Noise { start, .. }
        | SfxLayer::Click { start, .. } => *start,
    }
}
fn layer_duration(layer: &SfxLayer) -> f64 {
    match layer {
        SfxLayer::Tone { duration, .. }
        | SfxLayer::Noise { duration, .. }
        | SfxLayer::Click { duration, .. } => *duration,
    }
}
fn layer_end(layer: &SfxLayer) -> f64 {
    layer_start(layer) + layer_duration(layer)
}
fn layer_seed(layer: &SfxLayer) -> String {
    match layer {
        SfxLayer::Tone {
            name,
            start,
            duration,
            gain,
            ..
        }
        | SfxLayer::Noise {
            name,
            start,
            duration,
            gain,
            ..
        } => format!("{name}:{start}:{duration}:{gain}"),
        SfxLayer::Click {
            filter_frequency,
            gain,
            ..
        } => format!("transient:{filter_frequency}:{gain}"),
    }
}

fn layer_name(layer: &SfxLayer) -> &str {
    match layer {
        SfxLayer::Tone { name, .. } | SfxLayer::Noise { name, .. } => name,
        SfxLayer::Click { .. } => "transient",
    }
}

fn ri(rng: &mut Mulberry32, min: u32, max: u32) -> f64 {
    f64::from(rng.int_inclusive(min, max))
}

fn sort_layers(layers: &mut [SfxLayer]) {
    layers.sort_by(|left, right| {
        layer_start(left)
            .total_cmp(&layer_start(right))
            .then_with(|| layer_name(left).cmp(layer_name(right)))
    });
}

fn web_audio_layer_gain(t: f64, duration: f64, attack: f64, release: f64, gain: f64) -> f64 {
    let peak = gain.max(0.0001);
    if t < attack {
        exponential_lerp(0.0001, peak, t / attack.max(f64::EPSILON))
    } else {
        let release_end = (attack + 0.01).max(duration - release);
        if t < release_end {
            exponential_lerp(
                peak,
                0.0001,
                (t - attack) / (release_end - attack).max(f64::EPSILON),
            )
        } else {
            0.0001
        }
    }
}
fn wave_sample(wave: Waveform, phase: f64) -> f64 {
    let unit = (phase / (2.0 * PI)).rem_euclid(1.0);
    match wave {
        Waveform::Sine => phase.sin(),
        Waveform::Triangle => 1.0 - 4.0 * (unit - 0.5).abs(),
        Waveform::Square => {
            if unit < 0.5 {
                0.5
            } else {
                -0.5
            }
        }
        Waveform::Sawtooth => 1.0 - unit * 2.0,
    }
}

fn numeric_seed(text: &str) -> u32 {
    text.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
        .trunc()
        .max(0.0)
        .min(f64::from(u32::MAX)) as u32
}

fn ps_defaults(seed: u32) -> PuzzleScriptParams {
    PuzzleScriptParams {
        wave_type: 0,
        env_attack: 0.0,
        env_sustain: 0.3,
        env_punch: 0.0,
        env_decay: 0.4,
        base_freq: 0.3,
        freq_limit: 0.0,
        freq_ramp: 0.0,
        freq_dramp: 0.0,
        vib_strength: 0.0,
        vib_speed: 0.0,
        arp_mod: 0.0,
        arp_speed: 0.0,
        duty: 0.0,
        duty_ramp: 0.0,
        repeat_speed: 0.0,
        pha_offset: 0.0,
        pha_ramp: 0.0,
        lpf_freq: 1.0,
        lpf_ramp: 0.0,
        lpf_resonance: 0.0,
        hpf_freq: 0.0,
        hpf_ramp: 0.0,
        sound_volume: 0.25,
        sample_rate: 5512,
        seed,
    }
}

fn generate_puzzlescript(seed: u32) -> PuzzleScriptParams {
    let mut rng = Rc4::from_js_integer((seed / 100) as i32);
    let mut p = ps_defaults(seed);
    let frnd = |rng: &mut Rc4, range: f64| rng.uniform_56() * range;
    let rnd = |rng: &mut Rc4, max: u32| (rng.uniform_56() * f64::from(max + 1)).floor() as u32;
    match (seed % 100) % 10 {
        0 => {
            p.wave_type = frnd(&mut rng, 6.0).floor() as u8;
            if p.wave_type == 3 {
                p.wave_type = 0;
            }
            p.base_freq = 0.4 + frnd(&mut rng, 0.5);
            p.env_sustain = frnd(&mut rng, 0.1);
            p.env_decay = 0.1 + frnd(&mut rng, 0.4);
            p.env_punch = 0.3 + frnd(&mut rng, 0.3);
            if rnd(&mut rng, 1) != 0 {
                p.arp_speed = 0.5 + frnd(&mut rng, 0.2);
                let num = ((frnd(&mut rng, 7.0) as i32) | 1) + 1;
                let den = num + ((frnd(&mut rng, 7.0) as i32) | 1) + 2;
                p.arp_mod = f64::from(num) / f64::from(den);
            }
        }
        1 => {
            p.wave_type = frnd(&mut rng, 6.0).floor() as u8;
            if p.wave_type == 3 {
                p.wave_type = 0;
            }
            p.base_freq = 0.5 + frnd(&mut rng, 0.5);
            p.freq_limit = 0.2_f64.max(p.base_freq - 0.2 - frnd(&mut rng, 0.6));
            p.freq_ramp = -0.15 - frnd(&mut rng, 0.2);
            if rnd(&mut rng, 2) == 0 {
                p.base_freq = 0.3 + frnd(&mut rng, 0.6);
                p.freq_limit = frnd(&mut rng, 0.1);
                p.freq_ramp = -0.35 - frnd(&mut rng, 0.3);
            }
            if rnd(&mut rng, 1) != 0 {
                p.duty = frnd(&mut rng, 0.5);
                p.duty_ramp = frnd(&mut rng, 0.2);
            } else {
                p.duty = 0.4 + frnd(&mut rng, 0.5);
                p.duty_ramp = -frnd(&mut rng, 0.7);
            }
            p.env_sustain = 0.1 + frnd(&mut rng, 0.2);
            p.env_decay = frnd(&mut rng, 0.4);
            if rnd(&mut rng, 1) != 0 {
                p.env_punch = frnd(&mut rng, 0.3);
            }
            if rnd(&mut rng, 2) == 0 {
                p.pha_offset = frnd(&mut rng, 0.2);
                p.pha_ramp = -frnd(&mut rng, 0.2);
            }
            if rnd(&mut rng, 1) != 0 {
                p.hpf_freq = frnd(&mut rng, 0.3);
            }
        }
        2 => {
            if rnd(&mut rng, 1) != 0 {
                p.base_freq = 0.1 + frnd(&mut rng, 0.4);
                p.freq_ramp = -0.1 + frnd(&mut rng, 0.4);
            } else {
                p.base_freq = 0.2 + frnd(&mut rng, 0.7);
                p.freq_ramp = -0.2 - frnd(&mut rng, 0.2);
            }
            p.base_freq *= p.base_freq;
            if rnd(&mut rng, 4) == 0 {
                p.freq_ramp = 0.0;
            }
            if rnd(&mut rng, 2) == 0 {
                p.repeat_speed = 0.3 + frnd(&mut rng, 0.5);
            }
            p.env_sustain = 0.1 + frnd(&mut rng, 0.3);
            p.env_decay = frnd(&mut rng, 0.5);
            if rnd(&mut rng, 1) == 0 {
                p.pha_offset = -0.3 + frnd(&mut rng, 0.9);
                p.pha_ramp = -frnd(&mut rng, 0.3);
            }
            p.env_punch = 0.2 + frnd(&mut rng, 0.6);
            if rnd(&mut rng, 1) != 0 {
                p.vib_strength = frnd(&mut rng, 0.7);
                p.vib_speed = frnd(&mut rng, 0.6);
            }
            if rnd(&mut rng, 2) == 0 {
                p.arp_speed = 0.6 + frnd(&mut rng, 0.3);
                p.arp_mod = 0.8 - frnd(&mut rng, 1.6);
            }
        }
        3 => {
            p.wave_type = frnd(&mut rng, 6.0).floor() as u8;
            if p.wave_type == 3 {
                p.wave_type = 0;
            }
            if rnd(&mut rng, 1) != 0 {
                p.base_freq = 0.2 + frnd(&mut rng, 0.3);
                p.freq_ramp = 0.1 + frnd(&mut rng, 0.4);
                p.repeat_speed = 0.4 + frnd(&mut rng, 0.4);
            } else {
                p.base_freq = 0.2 + frnd(&mut rng, 0.3);
                p.freq_ramp = 0.05 + frnd(&mut rng, 0.2);
                if rnd(&mut rng, 1) != 0 {
                    p.vib_strength = frnd(&mut rng, 0.7);
                    p.vib_speed = frnd(&mut rng, 0.6);
                }
            }
            p.env_sustain = frnd(&mut rng, 0.4);
            p.env_decay = 0.1 + frnd(&mut rng, 0.4);
        }
        4 => {
            p.wave_type = frnd(&mut rng, 6.0).floor() as u8;
            p.base_freq = 0.2 + frnd(&mut rng, 0.6);
            p.freq_ramp = -0.3 - frnd(&mut rng, 0.4);
            p.env_sustain = frnd(&mut rng, 0.1);
            p.env_decay = 0.1 + frnd(&mut rng, 0.2);
            if rnd(&mut rng, 1) != 0 {
                p.hpf_freq = frnd(&mut rng, 0.3);
            }
        }
        5 => {
            p.wave_type = frnd(&mut rng, 6.0).floor() as u8;
            if p.wave_type == 3 {
                p.wave_type = 0;
            }
            p.duty = frnd(&mut rng, 0.6);
            p.base_freq = 0.3 + frnd(&mut rng, 0.3);
            p.freq_ramp = 0.1 + frnd(&mut rng, 0.2);
            p.env_sustain = 0.1 + frnd(&mut rng, 0.3);
            p.env_decay = 0.1 + frnd(&mut rng, 0.2);
            if rnd(&mut rng, 1) != 0 {
                p.hpf_freq = frnd(&mut rng, 0.3);
            }
            if rnd(&mut rng, 1) != 0 {
                p.lpf_freq = 1.0 - frnd(&mut rng, 0.6);
            }
        }
        6 => {
            p.wave_type = frnd(&mut rng, 6.0).floor() as u8;
            if p.wave_type == 3 {
                p.wave_type = rnd(&mut rng, 1) as u8;
            }
            if p.wave_type == 0 {
                p.duty = frnd(&mut rng, 0.6);
            }
            p.base_freq = 0.2 + frnd(&mut rng, 0.4);
            p.env_sustain = 0.1 + frnd(&mut rng, 0.1);
            p.env_decay = frnd(&mut rng, 0.2);
            p.hpf_freq = 0.1;
        }
        7 => {
            p.wave_type = frnd(&mut rng, 6.0).floor() as u8;
            if p.wave_type == 2 {
                p.wave_type += 1;
            }
            if p.wave_type == 0 {
                p.wave_type = 3;
            }
            p.base_freq = 0.1 + frnd(&mut rng, 0.4);
            p.freq_ramp = 0.05 + frnd(&mut rng, 0.2);
            p.env_attack = 0.01 + frnd(&mut rng, 0.09);
            p.env_sustain = 0.01 + frnd(&mut rng, 0.09);
            p.env_decay = 0.01 + frnd(&mut rng, 0.09);
            p.repeat_speed = 0.3 + frnd(&mut rng, 0.5);
            p.pha_offset = -0.3 + frnd(&mut rng, 0.9);
            p.pha_ramp = -frnd(&mut rng, 0.3);
            p.arp_speed = 0.6 + frnd(&mut rng, 0.3);
            p.arp_mod = 0.8 - frnd(&mut rng, 1.6);
        }
        8 => {
            p.wave_type = frnd(&mut rng, 6.0).floor() as u8;
            p.base_freq = (frnd(&mut rng, 2.0) - 1.0).powi(2);
            if rnd(&mut rng, 1) != 0 {
                p.base_freq = (frnd(&mut rng, 2.0) - 1.0).powi(3) + 0.5;
            }
            p.freq_ramp = (frnd(&mut rng, 2.0) - 1.0).powi(5);
            if p.base_freq > 0.7 && p.freq_ramp > 0.2 || p.base_freq < 0.2 && p.freq_ramp < -0.05 {
                p.freq_ramp = -p.freq_ramp;
            }
            p.freq_dramp = (frnd(&mut rng, 2.0) - 1.0).powi(3);
            p.duty = frnd(&mut rng, 2.0) - 1.0;
            p.duty_ramp = (frnd(&mut rng, 2.0) - 1.0).powi(3);
            p.vib_strength = (frnd(&mut rng, 2.0) - 1.0).powi(3);
            p.vib_speed = frnd(&mut rng, 2.0) - 1.0;
            p.env_attack = (frnd(&mut rng, 2.0) - 1.0).powi(3);
            p.env_sustain = (frnd(&mut rng, 2.0) - 1.0).powi(2);
            p.env_decay = frnd(&mut rng, 2.0) - 1.0;
            p.env_punch = frnd(&mut rng, 0.8).powi(2);
            if p.env_attack + p.env_sustain + p.env_decay < 0.2 {
                p.env_sustain += 0.2 + frnd(&mut rng, 0.3);
                p.env_decay += 0.2 + frnd(&mut rng, 0.3);
            }
            p.lpf_resonance = frnd(&mut rng, 2.0) - 1.0;
            p.lpf_freq = 1.0 - frnd(&mut rng, 1.0).powi(3);
            p.lpf_ramp = (frnd(&mut rng, 2.0) - 1.0).powi(3);
            if p.lpf_freq < 0.1 && p.lpf_ramp < -0.05 {
                p.lpf_ramp = -p.lpf_ramp;
            }
            p.hpf_freq = frnd(&mut rng, 1.0).powi(5);
            p.hpf_ramp = (frnd(&mut rng, 2.0) - 1.0).powi(5);
            p.pha_offset = (frnd(&mut rng, 2.0) - 1.0).powi(3);
            p.pha_ramp = (frnd(&mut rng, 2.0) - 1.0).powi(3);
            p.repeat_speed = frnd(&mut rng, 2.0) - 1.0;
            p.arp_speed = frnd(&mut rng, 2.0) - 1.0;
            p.arp_mod = frnd(&mut rng, 2.0) - 1.0;
        }
        9 => generate_bird(&mut rng, &mut p),
        _ => unreachable!(),
    }
    p
}

fn bird_jitter(rng: &mut Rc4, base: f64) -> f64 {
    base + rng.uniform_56() * 0.2 - 0.1
}

fn bird_wave(rng: &mut Rc4) -> u8 {
    let wave = (rng.uniform_56() * 6.0).floor() as u8;
    if wave == 3 { 0 } else { wave }
}

fn generate_bird(rng: &mut Rc4, p: &mut PuzzleScriptParams) {
    if rng.uniform_56() * 10.0 < 1.0 {
        p.wave_type = bird_wave(rng);
        p.env_attack = bird_jitter(rng, 0.4304400932967592);
        p.env_sustain = bird_jitter(rng, 0.15739346034252394);
        p.env_punch = bird_jitter(rng, 0.004488201744871758);
        p.env_decay = bird_jitter(rng, 0.07478075528212291);
        p.base_freq = bird_jitter(rng, 0.9865265720147687);
        p.freq_limit = rng.uniform_56() * 0.2 - 0.1;
        p.freq_ramp = bird_jitter(rng, -0.2995018224359539);
        if rng.uniform_56() < 0.5 {
            p.freq_ramp = 0.1 + rng.uniform_56() * 0.15;
        }
        p.freq_dramp = bird_jitter(rng, 0.004598608156964473);
        p.vib_strength = bird_jitter(rng, -0.2202799497929496);
        p.vib_speed = bird_jitter(rng, 0.8084998703158364);
        p.arp_mod = 0.0;
        p.arp_speed = 0.0;
        p.duty = bird_jitter(rng, -0.9031808754347107);
        p.duty_ramp = bird_jitter(rng, -0.8128699999808343);
        p.repeat_speed = bird_jitter(rng, 0.6014860189319991);
        p.pha_offset = bird_jitter(rng, -0.9424902314367765);
        p.pha_ramp = bird_jitter(rng, -0.1055482222272056);
        p.lpf_freq = bird_jitter(rng, 0.9989765717851521);
        p.lpf_ramp = bird_jitter(rng, -0.25051720626043017);
        p.lpf_resonance = bird_jitter(rng, 0.32777871505494693);
        p.hpf_freq = bird_jitter(rng, 0.0023548750981756753);
        p.hpf_ramp = bird_jitter(rng, -0.002375673204842568);
        return;
    }
    if rng.uniform_56() * 10.0 < 1.0 {
        p.wave_type = bird_wave(rng);
        p.env_attack = bird_jitter(rng, 0.5277795946672003);
        p.env_sustain = bird_jitter(rng, 0.18243733568468432);
        p.env_punch = bird_jitter(rng, -0.020159754546840117);
        p.env_decay = bird_jitter(rng, 0.1561353422051903);
        p.base_freq = bird_jitter(rng, 0.9028855606533718);
        p.freq_limit = -0.008842787837148716;
        p.freq_ramp = -0.1;
        p.freq_dramp = -0.012891241489551925;
        p.vib_strength = bird_jitter(rng, -0.17923136138403065);
        p.vib_speed = bird_jitter(rng, 0.908263385610142);
        p.arp_mod = bird_jitter(rng, 0.41690153355414894);
        p.arp_speed = bird_jitter(rng, 0.0010766233195860703);
        p.duty = bird_jitter(rng, -0.8735363011184684);
        p.duty_ramp = bird_jitter(rng, -0.7397985366747507);
        p.repeat_speed = bird_jitter(rng, 0.0591789344172107);
        p.pha_offset = bird_jitter(rng, -0.9961184222777699);
        p.pha_ramp = bird_jitter(rng, -0.08234769395850523);
        p.lpf_freq = bird_jitter(rng, 0.9412475115697335);
        p.lpf_ramp = bird_jitter(rng, -0.18261358925834958);
        p.lpf_resonance = bird_jitter(rng, 0.24541438107389477);
        p.hpf_freq = bird_jitter(rng, -0.01831940280978611);
        p.hpf_ramp = bird_jitter(rng, -0.03857383633171346);
        return;
    }
    if rng.uniform_56() * 10.0 < 1.0 {
        p.wave_type = bird_wave(rng);
        p.env_attack = bird_jitter(rng, 0.4304400932967592);
        p.env_sustain = bird_jitter(rng, 0.15739346034252394);
        p.env_punch = bird_jitter(rng, 0.004488201744871758);
        p.env_decay = bird_jitter(rng, 0.07478075528212291);
        p.base_freq = bird_jitter(rng, 0.9865265720147687);
        p.freq_limit = rng.uniform_56() * 0.2 - 0.1;
        p.freq_ramp = bird_jitter(rng, -0.2995018224359539);
        p.freq_dramp = bird_jitter(rng, 0.004598608156964473);
        p.vib_strength = bird_jitter(rng, -0.2202799497929496);
        p.vib_speed = bird_jitter(rng, 0.8084998703158364);
        p.arp_mod = bird_jitter(rng, -0.46410459213693644);
        p.arp_speed = bird_jitter(rng, -0.10955361249587248);
        p.duty = bird_jitter(rng, -0.9031808754347107);
        p.duty_ramp = bird_jitter(rng, -0.8128699999808343);
        p.repeat_speed = bird_jitter(rng, 0.7014860189319991);
        p.pha_offset = bird_jitter(rng, -0.9424902314367765);
        p.pha_ramp = bird_jitter(rng, -0.1055482222272056);
        p.lpf_freq = bird_jitter(rng, 0.9989765717851521);
        p.lpf_ramp = bird_jitter(rng, -0.25051720626043017);
        p.lpf_resonance = bird_jitter(rng, 0.32777871505494693);
        p.hpf_freq = bird_jitter(rng, 0.0023548750981756753);
        p.hpf_ramp = bird_jitter(rng, -0.002375673204842568);
        return;
    }
    if rng.uniform_56() * 5.0 > 1.0 {
        p.wave_type = bird_wave(rng);
        if (rng.uniform_56() * 2.0).floor() as u32 != 0 {
            p.arp_mod = bird_jitter(rng, 0.2697849293151393);
            p.arp_speed = bird_jitter(rng, -0.3131172257760948);
            p.base_freq = bird_jitter(rng, 0.8090588299313949);
            p.duty = bird_jitter(rng, -0.6210022920964955);
            p.duty_ramp = bird_jitter(rng, -0.00043441813553182567);
            p.env_attack = bird_jitter(rng, 0.004321877246874195);
            p.env_decay = bird_jitter(rng, 0.1);
            p.env_punch = bird_jitter(rng, 0.061737781504416146);
            p.env_sustain = bird_jitter(rng, 0.4987252564798832);
            p.freq_dramp = bird_jitter(rng, 0.31700340314222614);
            p.freq_limit = rng.uniform_56() * 0.2 - 0.1;
            p.freq_ramp = bird_jitter(rng, -0.163380391341416);
            p.hpf_freq = bird_jitter(rng, 0.4709005021145149);
            p.hpf_ramp = bird_jitter(rng, 0.6924667290539194);
            p.lpf_freq = bird_jitter(rng, 0.8351398631384511);
            p.lpf_ramp = bird_jitter(rng, 0.36616557192873134);
            p.lpf_resonance = bird_jitter(rng, -0.08685777111664439);
            p.pha_offset = bird_jitter(rng, -0.036084571580025544);
            p.pha_ramp = bird_jitter(rng, -0.014806445085568108);
            p.repeat_speed = bird_jitter(rng, -0.8094368475518489);
            p.vib_speed = bird_jitter(rng, 0.4496665457171294);
            p.vib_strength = bird_jitter(rng, 0.23413762515532424);
        } else {
            p.arp_mod = bird_jitter(rng, -0.35697118026766184);
            p.arp_speed = bird_jitter(rng, 0.3581140690559588);
            p.base_freq = bird_jitter(rng, 1.3260897696157528);
            p.duty = bird_jitter(rng, -0.30984900436710694);
            p.duty_ramp = bird_jitter(rng, -0.0014374759133411626);
            p.env_attack = bird_jitter(rng, 0.3160357835682254);
            p.env_decay = bird_jitter(rng, 0.1);
            p.env_punch = bird_jitter(rng, 0.24323114016870148);
            p.env_sustain = bird_jitter(rng, 0.4);
            p.freq_dramp = bird_jitter(rng, 0.2866475886237244);
            p.freq_limit = rng.uniform_56() * 0.2 - 0.1;
            p.freq_ramp = bird_jitter(rng, -0.10956352368742976);
            p.hpf_freq = bird_jitter(rng, 0.20772718017889846);
            p.hpf_ramp = bird_jitter(rng, 0.1564090637378835);
            p.lpf_freq = bird_jitter(rng, 0.6021372770637031);
            p.lpf_ramp = bird_jitter(rng, 0.24016227139979027);
            p.lpf_resonance = bird_jitter(rng, -0.08787383821160144);
            p.pha_offset = bird_jitter(rng, -0.381597686151701);
            p.pha_ramp = bird_jitter(rng, -0.0002481687661373495);
            p.repeat_speed = bird_jitter(rng, 0.07812112809425686);
            p.vib_speed = bird_jitter(rng, -0.13648848579133943);
            p.vib_strength = bird_jitter(rng, 0.0018874158972302657);
        }
        return;
    }
    p.wave_type = (rng.uniform_56() * 6.0).floor() as u8;
    if p.wave_type == 1 || p.wave_type == 3 {
        p.wave_type = 2;
    }
    p.base_freq = 0.85 + rng.uniform_56() * 0.15;
    p.freq_ramp = 0.3 + rng.uniform_56() * 0.15;
    p.env_attack = rng.uniform_56() * 0.09;
    p.env_sustain = 0.2 + rng.uniform_56() * 0.3;
    p.env_decay = rng.uniform_56() * 0.1;
    p.duty = rng.uniform_56() * 2.0 - 1.0;
    p.duty_ramp = (rng.uniform_56() * 2.0 - 1.0).powi(3);
    p.repeat_speed = 0.5 + rng.uniform_56() * 0.1;
    p.pha_offset = -0.3 + rng.uniform_56() * 0.9;
    p.pha_ramp = -rng.uniform_56() * 0.3;
    p.arp_speed = 0.4 + rng.uniform_56() * 0.6;
    p.arp_mod = 0.8 + rng.uniform_56() * 0.1;
    p.lpf_resonance = rng.uniform_56() * 2.0 - 1.0;
    p.lpf_freq = 1.0 - rng.uniform_56().powi(3);
    p.lpf_ramp = (rng.uniform_56() * 2.0 - 1.0).powi(3);
    if p.lpf_freq < 0.1 && p.lpf_ramp < -0.05 {
        p.lpf_ramp = -p.lpf_ramp;
    }
    p.hpf_freq = rng.uniform_56().powi(5);
    p.hpf_ramp = (rng.uniform_56() * 2.0 - 1.0).powi(5);
}

// PuzzleScript-compatible rendering is adapted from PuzzleScript's sfxr.js.
//
// MIT License
// Copyright (c) 2013 Stephen Lavelle
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
fn render_puzzlescript(ps: &PuzzleScriptParams, volume: f64) -> Result<GeneratedSfxClip, String> {
    let legacy = render_puzzlescript_legacy(ps, volume)?;
    Ok(resample_linear(&legacy, 48_000))
}

fn resample_linear(source: &GeneratedSfxClip, target_rate: u32) -> GeneratedSfxClip {
    if source.sample_rate == target_rate {
        return source.clone();
    }
    let target_len = (source.samples.len() as u64 * u64::from(target_rate))
        .div_ceil(u64::from(source.sample_rate)) as usize;
    let scale = f64::from(source.sample_rate) / f64::from(target_rate);
    let mut samples = Vec::with_capacity(target_len);
    for target_index in 0..target_len {
        let position = target_index as f64 * scale;
        let left = position.floor() as usize;
        let fraction = (position - left as f64) as f32;
        let a = source.samples.get(left).copied().unwrap_or(0.0);
        let b = source.samples.get(left + 1).copied().unwrap_or(a);
        samples.push(a + (b - a) * fraction);
    }
    GeneratedSfxClip {
        sample_rate: target_rate,
        samples: samples.into(),
    }
}

fn render_puzzlescript_legacy(
    ps: &PuzzleScriptParams,
    volume: f64,
) -> Result<GeneratedSfxClip, String> {
    let mut noise_rng = Mulberry32::from_text(&format!("puzzlescript-noise:{}", ps.seed));
    let env_length = [
        (ps.env_attack * ps.env_attack * 100_000.0).floor() as usize,
        (ps.env_sustain * ps.env_sustain * 100_000.0).floor() as usize,
        (ps.env_decay * ps.env_decay * 100_000.0).floor() as usize,
    ];
    let env_total = (env_length[0] + env_length[1] + env_length[2]).max(1);
    let summands = (44_100 / ps.sample_rate).max(1) as usize;
    let output_rate = ps.sample_rate.max(22_050);
    let expansion = if ps.sample_rate < 22_050 {
        output_rate.div_ceil(ps.sample_rate) as usize
    } else {
        1
    };
    let buffer_len = env_total.div_ceil(summands) * expansion + expansion + 8;
    let mut output = vec![0_f32; buffer_len];
    let mut fperiod = 100.0 / (ps.base_freq * ps.base_freq + 0.001);
    let fmaxperiod = 100.0 / (ps.freq_limit * ps.freq_limit + 0.001);
    let mut fslide = 1.0 - ps.freq_ramp.powi(3) * 0.01;
    let fdslide = -ps.freq_dramp.powi(3) * 0.000001;
    let mut duty = 0.5 - ps.duty * 0.5;
    let duty_slide = -ps.duty_ramp * 0.00005;
    let arp_mod = if ps.arp_mod >= 0.0 {
        1.0 - ps.arp_mod.powi(2) * 0.9
    } else {
        1.0 + ps.arp_mod.powi(2) * 10.0
    };
    let mut arp_limit = ((1.0 - ps.arp_speed).powi(2) * 20_000.0 + 32.0).floor() as usize;
    if ps.arp_speed == 1.0 {
        arp_limit = 0;
    }
    let mut rep_limit = ((1.0 - ps.repeat_speed).powi(2) * 20_000.0 + 32.0).floor() as usize;
    if ps.repeat_speed == 0.0 {
        rep_limit = 0;
    }
    let mut fltp = 0.0;
    let mut fltdp = 0.0;
    let mut fltw = ps.lpf_freq.powi(3) * 0.1;
    let fltw_d = 1.0 + ps.lpf_ramp * 0.0001;
    let fltdmp = (5.0 / (1.0 + ps.lpf_resonance.powi(2) * 20.0) * (0.01 + fltw)).min(0.8);
    let mut fltphp = 0.0;
    let mut flthp = ps.hpf_freq.powi(2) * 0.1;
    let flthp_d = 1.0 + ps.hpf_ramp * 0.0003;
    let mut vib_phase: f64 = 0.0;
    let vib_speed = ps.vib_speed.powi(2) * 0.01;
    let vib_amp = ps.vib_strength * 0.5;
    let mut fphase = ps.pha_offset.powi(2) * 1020.0 * if ps.pha_offset < 0.0 { -1.0 } else { 1.0 };
    let fdphase = ps.pha_ramp.powi(2) * if ps.pha_ramp < 0.0 { -1.0 } else { 1.0 };
    let mut phaser = [0_f64; 1024];
    let mut noise = [0_f64; 32];
    for value in &mut noise {
        *value = noise_rng.uniform() * 2.0 - 1.0;
    }
    let gain = (ps.sound_volume.exp() - 1.0) * volume;
    let mut stage = 0;
    let mut env_time = 0;
    let mut phase = 0_usize;
    let mut ipp = 0_usize;
    let mut rep_time = 0_usize;
    let mut sample_sum = 0.0;
    let mut num_summed = 0;
    let mut out = 0;
    for t in 0..usize::MAX {
        if out >= output.len() {
            break;
        }
        if rep_limit != 0 {
            rep_time += 1;
            if rep_time >= rep_limit {
                rep_time = 0;
                fperiod = 100.0 / (ps.base_freq * ps.base_freq + 0.001);
                fslide = 1.0 - ps.freq_ramp.powi(3) * 0.01;
                duty = 0.5 - ps.duty * 0.5;
            }
        }
        if arp_limit != 0 && t >= arp_limit {
            arp_limit = 0;
            fperiod *= arp_mod;
        }
        fslide += fdslide;
        fperiod *= fslide;
        if fperiod > fmaxperiod {
            fperiod = fmaxperiod;
            if ps.freq_limit > 0.0 {
                break;
            }
        }
        let mut rfperiod = fperiod;
        if vib_amp > 0.0 {
            vib_phase += vib_speed;
            rfperiod *= 1.0 + vib_phase.sin() * vib_amp;
        }
        let period = (rfperiod.floor() as usize).max(8);
        duty = (duty + duty_slide).clamp(0.0, 0.5);
        env_time += 1;
        if env_time > env_length[stage] {
            env_time = 1;
            stage += 1;
            while stage < 3 && env_length[stage] == 0 {
                stage += 1;
            }
            if stage == 3 {
                break;
            }
        }
        let env = match stage {
            0 => env_time as f64 / env_length[0].max(1) as f64,
            1 => 1.0 + (1.0 - env_time as f64 / env_length[1].max(1) as f64) * 2.0 * ps.env_punch,
            _ => 1.0 - env_time as f64 / env_length[2].max(1) as f64,
        };
        fphase += fdphase;
        let iphase = (fphase.floor().abs() as usize).min(1023);
        if flthp_d != 0.0 {
            flthp = (flthp * flthp_d).clamp(0.00001, 0.1);
        }
        let mut sample = 0.0;
        for _ in 0..8 {
            phase += 1;
            if phase >= period {
                phase %= period;
                if ps.wave_type == 3 {
                    for value in &mut noise {
                        *value = noise_rng.uniform() * 2.0 - 1.0;
                    }
                }
            }
            let fp = phase as f64 / period as f64;
            let mut sub = match ps.wave_type {
                0 => {
                    if fp < duty {
                        0.5
                    } else {
                        -0.5
                    }
                }
                1 => 1.0 - fp * 2.0,
                2 => (fp * 2.0 * PI).sin(),
                3 => noise[phase * 32 / period],
                4 => (1.0 - fp * 2.0).abs() - 1.0,
                _ => (1.0 - fp * fp * 2.0).abs() - 1.0,
            };
            let old = fltp;
            fltw = (fltw * fltw_d).clamp(0.0, 0.1);
            if ps.lpf_freq != 1.0 {
                fltdp += (sub - fltp) * fltw;
                fltdp -= fltdp * fltdmp;
            } else {
                fltp = sub;
                fltdp = 0.0;
            }
            fltp += fltdp;
            fltphp += fltp - old;
            fltphp -= fltphp * flthp;
            sub = fltphp;
            phaser[ipp & 1023] = sub;
            sub += phaser[(ipp + 1024 - iphase) & 1023];
            ipp = (ipp + 1) & 1023;
            sample += sub * env;
        }
        sample_sum += sample;
        num_summed += 1;
        if num_summed < summands {
            continue;
        }
        num_summed = 0;
        let value = (sample_sum / summands as f64 / 8.0) * gain;
        sample_sum = 0.0;
        for _ in 0..expansion {
            if out < output.len() {
                output[out] = value as f32;
                out += 1;
            }
        }
    }
    Ok(GeneratedSfxClip {
        sample_rate: output_rate,
        samples: output.into(),
    })
}

fn lerp(min: f64, max: f64, value: f64) -> f64 {
    min + (max - min) * value
}
fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_all_authored_types_without_adapter_semantics() {
        for kind in SFX_TYPES.into_iter().chain(["wild", "random"]) {
            let generated = generate_sfx(&SfxRecipe {
                seed: "123456".into(),
                type_target: kind.into(),
                volume: 1.0,
            })
            .unwrap();
            assert!(!generated.resolved_type.is_empty());
            assert!(!render_sfx(&generated).unwrap().samples.is_empty());
        }
        assert_eq!(
            generate_sfx(&SfxRecipe {
                seed: "manual-seed-without-prefix".into(),
                type_target: "random".into(),
                volume: 1.0
            })
            .unwrap()
            .resolved_type,
            "step"
        );
    }

    #[test]
    fn matches_representative_javascript_pickup_structure() {
        let generated = generate_sfx(&SfxRecipe {
            seed: "123456".into(),
            type_target: "pickup".into(),
            volume: 1.0,
        })
        .unwrap();
        let SfxSynthesis::Layers {
            duration,
            profile,
            layers,
        } = generated.synthesis
        else {
            panic!()
        };
        assert_eq!(duration, 0.472);
        assert_eq!(
            (&*profile.engine, &*profile.variant, &*profile.pattern),
            ("bit-crush", "gritty", "sparkle")
        );
        assert_eq!(layers.len(), 5);
        assert_eq!(
            layers[0],
            SfxLayer::Tone {
                name: "note-1".into(),
                start: 0.0,
                duration: 0.212,
                waveform: Waveform::Sine,
                frequency_start: 834.0,
                frequency_end: 737.0,
                gain: 0.207,
                attack: 0.006,
                release: 0.068,
                filter_frequency: 5657.0,
                wobble: 0.06,
            }
        );
        assert_eq!(
            layers[4],
            SfxLayer::Noise {
                name: "sparkle".into(),
                color: NoiseColor::White,
                start: 0.207,
                duration: 0.132,
                gain: 0.102,
                attack: 0.004,
                release: 0.059,
                filter: SfxFilterKind::Highpass,
                filter_start: 6061.0,
                filter_end: 6992.0,
            }
        );
    }

    #[test]
    fn matches_javascript_category_goldens_for_one_shared_seed() {
        let expected = [
            ("wild", 0.553, "noise", 804_592_521),
            ("jump", 0.314, "spring", 1_530_672_734),
            ("step", 0.202, "wood", 2_250_997_928),
            ("hit", 0.27, "slash", 1_421_465_884),
            ("drag", 0.562, "stone-floor", 3_076_173_392),
            ("water", 0.517, "plop", 2_268_485_925),
            ("lock", 0.404, "deadbolt", 2_007_702_931),
            ("explosion", 0.921, "puff", 1_043_166_836),
            ("laser", 0.404, "zap", 835_709_962),
            ("powerup", 0.809, "swell", 3_956_496_367),
            ("select", 0.135, "blip", 419_878_067),
            ("error", 0.472, "fall", 3_953_821_844),
        ];
        for (kind, duration, pattern, fingerprint) in expected {
            let generated = generate_sfx(&SfxRecipe {
                seed: "123456".into(),
                type_target: kind.into(),
                volume: 1.0,
            })
            .unwrap();
            let SfxSynthesis::Layers {
                duration: actual_duration,
                profile,
                layers,
            } = generated.synthesis
            else {
                panic!()
            };
            assert_eq!(actual_duration, duration, "{kind} duration");
            assert_eq!(profile.pattern, pattern, "{kind} pattern");
            assert_eq!(layer_fingerprint(&layers), fingerprint, "{kind} layers");
        }
    }

    fn layer_fingerprint(layers: &[SfxLayer]) -> u32 {
        fn mix(hash: u32, value: u32) -> u32 {
            (hash ^ value).wrapping_mul(16_777_619)
        }
        fn q(value: Option<f64>) -> u32 {
            (value.unwrap_or(-1.0) * 1000.0).round() as i32 as u32
        }
        let mut hash = 2_166_136_261_u32;
        for layer in layers {
            for unit in layer_name(layer).encode_utf16() {
                hash = mix(hash, u32::from(unit));
            }
            let values: [Option<f64>; 11] = match layer {
                SfxLayer::Tone {
                    start,
                    duration,
                    gain,
                    frequency_start,
                    frequency_end,
                    filter_frequency,
                    attack,
                    release,
                    wobble,
                    ..
                } => [
                    Some(*start),
                    Some(*duration),
                    Some(*gain),
                    Some(*frequency_start),
                    Some(*frequency_end),
                    None,
                    None,
                    Some(*filter_frequency),
                    Some(*attack),
                    Some(*release),
                    Some(*wobble),
                ],
                SfxLayer::Noise {
                    start,
                    duration,
                    gain,
                    filter_start,
                    filter_end,
                    attack,
                    release,
                    ..
                } => [
                    Some(*start),
                    Some(*duration),
                    Some(*gain),
                    None,
                    None,
                    Some(*filter_start),
                    Some(*filter_end),
                    None,
                    Some(*attack),
                    Some(*release),
                    None,
                ],
                SfxLayer::Click {
                    start,
                    duration,
                    gain,
                    filter_frequency,
                } => [
                    Some(*start),
                    Some(*duration),
                    Some(*gain),
                    None,
                    None,
                    None,
                    None,
                    Some(*filter_frequency),
                    None,
                    None,
                    None,
                ],
            };
            for value in values {
                hash = mix(hash, q(value));
            }
        }
        hash
    }

    #[test]
    fn matches_javascript_structure_across_seeded_pattern_space() {
        let expected = [
            ("wild", 2_095_567_402),
            ("jump", 424_082_051),
            ("step", 3_885_317_334),
            ("pickup", 3_282_005_868),
            ("hit", 785_847_083),
            ("drag", 619_005_487),
            ("water", 1_282_481_193),
            ("lock", 3_010_213_902),
            ("explosion", 1_337_301_680),
            ("laser", 1_198_859_173),
            ("powerup", 2_448_211_293),
            ("select", 4_187_945_417),
            ("error", 4_160_451_980),
        ];
        for (kind, expected_hash) in expected {
            let mut hash = 2_166_136_261_u32;
            for seed in 100_000..100_032 {
                let generated = generate_sfx(&SfxRecipe {
                    seed: seed.to_string(),
                    type_target: kind.into(),
                    volume: 1.0,
                })
                .unwrap();
                let SfxSynthesis::Layers {
                    duration, layers, ..
                } = generated.synthesis
                else {
                    panic!()
                };
                hash = (hash ^ (duration * 1000.0).round() as u32).wrapping_mul(16_777_619);
                hash = (hash ^ layer_fingerprint(&layers)).wrapping_mul(16_777_619);
            }
            assert_eq!(hash, expected_hash, "{kind} 32-seed structure");
        }
    }

    #[test]
    fn canonical_layer_renderer_is_deterministic_and_applies_authored_filters() {
        let generated = generate_sfx(&SfxRecipe {
            seed: "123456".into(),
            type_target: "pickup".into(),
            volume: 1.0,
        })
        .unwrap();
        let first = render_sfx(&generated).unwrap();
        let second = render_sfx(&generated).unwrap();
        assert_eq!(first, second);

        let mut filtered_differently = generated;
        let SfxSynthesis::Layers { layers, .. } = &mut filtered_differently.synthesis else {
            panic!()
        };
        let Some(SfxLayer::Tone {
            filter_frequency, ..
        }) = layers
            .iter_mut()
            .find(|layer| matches!(layer, SfxLayer::Tone { .. }))
        else {
            panic!()
        };
        *filter_frequency = 80.0;
        let changed = render_sfx(&filtered_differently).unwrap();
        assert_ne!(first.samples, changed.samples);
    }

    #[test]
    fn matches_puzzlescript_parameter_and_pcm_vectors() {
        let pickup = generate_sfx(&SfxRecipe {
            seed: "17551700".into(),
            type_target: "puzzlescript".into(),
            volume: 1.0,
        })
        .unwrap();
        let SfxSynthesis::PuzzleScript(params) = &pickup.synthesis else {
            panic!()
        };
        assert_eq!(params.wave_type, 5);
        assert_eq!(params.base_freq, 0.7348149530671693);
        assert_eq!(params.env_sustain, 0.0072618410338281675);

        let noise = generate_sfx(&SfxRecipe {
            seed: "64059507".into(),
            type_target: "puzzlescript".into(),
            volume: 1.0,
        })
        .unwrap();
        let SfxSynthesis::PuzzleScript(noise_params) = &noise.synthesis else {
            panic!()
        };
        let legacy = render_puzzlescript_legacy(noise_params, 1.0).unwrap();
        assert_eq!(legacy.sample_rate, 22_050);
        assert_eq!(legacy.samples.len(), 203);
        assert_eq!(legacy.samples[0].to_bits(), 0.0022951534_f32.to_bits());
        assert_eq!(legacy.samples[5].to_bits(), 0.006513246_f32.to_bits());
        let canonical = render_sfx(&noise).unwrap();
        assert_eq!(canonical.sample_rate, 48_000);
        assert_eq!(canonical.samples.len(), 442);
    }

    #[test]
    fn matches_all_ten_puzzlescript_parameter_generators() {
        let expected = [
            1_353_468_675,
            2_314_168_192,
            149_824_745,
            2_823_638_659,
            3_952_371_588,
            58_706_693,
            771_269_915,
            1_071_895_432,
            2_236_957_277,
            244_622_797,
        ];
        for (index, expected_hash) in expected.into_iter().enumerate() {
            let params = generate_puzzlescript(17_551_700 + index as u32);
            assert_eq!(
                puzzlescript_fingerprint(&params),
                expected_hash,
                "generator {index}"
            );
        }
    }

    fn puzzlescript_fingerprint(params: &PuzzleScriptParams) -> u32 {
        fn mix(hash: u32, value: u32) -> u32 {
            (hash ^ value).wrapping_mul(16_777_619)
        }
        let mut hash = mix(2_166_136_261, u32::from(params.wave_type));
        for value in [
            params.env_attack,
            params.env_sustain,
            params.env_punch,
            params.env_decay,
            params.base_freq,
            params.freq_limit,
            params.freq_ramp,
            params.freq_dramp,
            params.vib_strength,
            params.vib_speed,
            params.arp_mod,
            params.arp_speed,
            params.duty,
            params.duty_ramp,
            params.repeat_speed,
            params.pha_offset,
            params.pha_ramp,
            params.lpf_freq,
            params.lpf_ramp,
            params.lpf_resonance,
            params.hpf_freq,
            params.hpf_ramp,
            params.sound_volume,
        ] {
            let quantized = (value * 1_000_000_000_000.0).round() as i64 as u64;
            hash = mix(hash, quantized as u32);
            hash = mix(hash, (quantized >> 32) as u32);
        }
        mix(hash, params.sample_rate)
    }
}
