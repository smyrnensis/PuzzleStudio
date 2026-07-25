use crate::prng::Mulberry32;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicFilterKind {
    Lowpass,
    Bandpass,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Filter {
    pub kind: MusicFilterKind,
    pub frequency: f64,
    pub q: f64,
    pub end_frequency: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub attack: f64,
    pub sustain: f64,
    pub release: f64,
    pub duration_scale: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpectralPartial {
    pub ratio: f64,
    pub gain: f64,
    pub decay: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoiseRole {
    Attack,
    Sustain,
    Carrier,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpectralNoise {
    pub role: NoiseRole,
    pub gain: f64,
    pub decay: f64,
    pub filter: Filter,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ResonantBody {
    Bandpass {
        frequency: f64,
        q: f64,
        gain: f64,
    },
    Comb {
        delay: f64,
        feedback: f64,
        gain: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PitchMotion {
    pub vibrato_cents: Option<f64>,
    pub vibrato_rate: Option<f64>,
    pub jitter_cents: Option<f64>,
    pub jitter_rate: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpectralFieldPoint {
    pub center_log2: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpectralTimbreSignal {
    pub partials: Vec<SpectralPartial>,
    pub envelope: Envelope,
    pub filter: Filter,
    pub distance_gain: f64,
    pub noise: Option<SpectralNoise>,
    pub body: Option<ResonantBody>,
    pub pitch: Option<PitchMotion>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpectralTimbreParameters {
    pub partial_count: usize,
    pub partial_energy: f64,
    pub alpha: f64,
    pub smoothness: f64,
    pub roughness: f64,
    pub dropout_rate: f64,
    pub dropout_depth: f64,
    pub ratio_drift: f64,
    pub decay_base: f64,
    pub decay_slope: f64,
    pub continuity: f64,
    pub filter_start: f64,
    pub filter_end: Option<f64>,
    pub loudness_estimate: f64,
    pub normalization_gain: f64,
    pub base_distance_gain: f64,
    pub spectral_field: Vec<SpectralFieldPoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpectralTimbre {
    pub id: String,
    pub seed: String,
    pub signal: SpectralTimbreSignal,
    pub parameters: SpectralTimbreParameters,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransientBand {
    pub frequency: f64,
    pub q: f64,
    pub gain: f64,
    pub decay: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransientEnvelope {
    pub attack: f64,
    pub decay: f64,
    pub release: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransientClick {
    pub gain: f64,
    pub frequency: f64,
    pub q: f64,
    pub decay: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransientResonator {
    pub frequency: f64,
    pub gain: f64,
    pub decay: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransientBody {
    pub frequency: f64,
    pub q: f64,
    pub gain: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransientSignal {
    pub bands: Vec<TransientBand>,
    pub envelope: TransientEnvelope,
    pub click: Option<TransientClick>,
    pub resonators: Vec<TransientResonator>,
    pub body: Option<TransientBody>,
    pub distance_gain: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransientParameters {
    pub band_count: usize,
    pub noise_energy: f64,
    pub spectral_tilt: f64,
    pub smoothness: f64,
    pub roughness: f64,
    pub dropout_rate: f64,
    pub dropout_depth: f64,
    pub attack: f64,
    pub decay: f64,
    pub release: f64,
    pub click_gain: f64,
    pub resonator_count: usize,
    pub loudness_estimate: f64,
    pub normalization_gain: f64,
    pub spectral_field: Vec<SpectralFieldPoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransientTimbre {
    pub id: String,
    pub seed: String,
    pub signal: TransientSignal,
    pub parameters: TransientParameters,
}

#[derive(Clone)]
struct SmoothField {
    smoothness: f64,
    brightness: f64,
    points: Vec<SpectralFieldPoint>,
}

impl SmoothField {
    fn value(&self, coordinate: f64) -> f64 {
        self.points
            .iter()
            .map(|point| {
                let distance = coordinate - point.center_log2;
                point.height * (-(distance * distance) / (2.0 * point.width * point.width)).exp()
            })
            .sum()
    }
}

/// Exact Rust translation of `generateRandomTimbre` in
/// the original JavaScript model. The rounding points are intentionally retained:
/// later synthesis consumes the rounded signal, while loudness normalization is
/// calculated from that same rounded signal.
pub fn generate_spectral_timbre(seed: &str) -> SpectralTimbre {
    create_distributed_timbre(seed, 1)
}

/// Exact Rust translation of `generateRandomTransient` in
/// that model.
pub fn generate_transient_timbre(seed: &str) -> TransientTimbre {
    create_transient_field(seed, 1)
}

fn create_distributed_timbre(seed: &str, serial: usize) -> SpectralTimbre {
    let mut rng = Mulberry32::from_text(seed);
    let reference_frequency = 440.0;
    let partial_count = clamp_int(
        3.0 + (-(1.0 - rng.uniform()).max(1e-6).ln() * 4.2).floor(),
        3,
        24,
    );
    let alpha = clamp(log_normal(&mut rng, 1.18_f64.ln(), 0.58), 0.32, 3.45);
    let smooth_field = random_smooth_spectral_field(&mut rng);
    let roughness = clamp(rng.uniform().powf(0.88) * 0.58, 0.01, 0.58);
    let dropout_rate = clamp(rng.uniform().powf(1.35) * 0.58, 0.0, 0.58);
    let dropout_depth = random_range(&mut rng, 0.6, 4.2);
    let ratio_tail = if rng.uniform() < 0.055 {
        random_range(&mut rng, 0.008, 0.038)
    } else {
        0.0
    };
    let ratio_drift = clamp(
        log_normal(&mut rng, 0.00145_f64.ln(), 0.92) + ratio_tail,
        0.00002,
        0.048,
    );
    let ratio_bend = normal(&mut rng) * ratio_drift * 0.11;
    let decay_base = clamp(log_normal(&mut rng, 1.15_f64.ln(), 0.95), 0.08, 9.4);
    let decay_slope = clamp(
        normal(&mut rng) * 0.78 + random_range(&mut rng, -0.35, 0.75),
        -1.08,
        2.18,
    );
    let continuity = clamp(
        1.0 / (1.0 + decay_base * 0.42 + decay_slope.max(0.0) * 0.72 - decay_slope.min(0.0) * 0.32),
        0.08,
        0.96,
    );
    let mut partials = Vec::with_capacity(partial_count);

    for n in 1..=partial_count {
        let n = n as f64;
        let log_n = n.ln();
        let ratio = if n == 1.0 {
            1.0
        } else {
            (n * (1.0 + ratio_bend * (n - 1.0) + normal(&mut rng) * ratio_drift * (n - 1.0).sqrt()))
                .max(0.2)
        };
        let dropout_chance = if n > 1.0 {
            dropout_rate * (0.35 + 0.65 * n.ln_1p() / (partial_count as f64).ln_1p())
        } else {
            0.0
        };
        let dropout_penalty = if rng.uniform() < dropout_chance {
            random_range(&mut rng, 0.55, dropout_depth)
        } else {
            0.0
        };
        let log_gain = -alpha * log_n + smooth_field.value(n.log2()) + normal(&mut rng) * roughness
            - dropout_penalty;
        let decay = decay_base + decay_slope * log_n + normal(&mut rng) * 0.4;
        partials.push(SpectralPartial {
            ratio: round4(ratio),
            gain: log_gain.exp(),
            decay: (decay > 0.42).then(|| round4(decay)),
        });
    }

    let partial_energy = partials
        .iter()
        .map(|partial| partial.gain * partial.gain)
        .sum::<f64>()
        .sqrt();
    let partial_energy_scale = 1.0 / partial_energy.max(0.0001);
    for partial in &mut partials {
        partial.gain = round4(partial.gain * partial_energy_scale);
    }

    let attack_center = 0.008 + (1.0 - continuity) * 0.04;
    let attack = clamp(log_normal(&mut rng, attack_center.ln(), 0.86), 0.002, 0.22);
    let sustain = clamp(
        0.12 + continuity * 0.72 + normal(&mut rng) * 0.18,
        0.04,
        0.95,
    );
    let release = clamp(
        log_normal(&mut rng, (0.045 + continuity * 0.075).ln(), 0.66),
        0.012,
        0.32,
    );
    let duration_scale = clamp(
        0.28 + continuity * 0.62 + normal(&mut rng) * 0.16,
        0.18,
        1.0,
    );
    let filter_start = clamp(
        log_normal(
            &mut rng,
            (2200.0 + smooth_field.brightness * 760.0).ln(),
            0.62,
        ),
        320.0,
        7600.0,
    );
    let filter_ends = rng.uniform() < 0.14 + (1.0 - continuity) * 0.3;
    let filter_end = filter_ends.then(|| {
        clamp(
            filter_start * random_range(&mut rng, 0.08, 0.68),
            120.0,
            filter_start * 0.82,
        )
    });
    let filter_q = random_range(&mut rng, 0.26, 0.82);
    let base_distance_gain = round4(clamp(0.7 + normal(&mut rng) * 0.13, 0.42, 0.98));
    let mut signal = SpectralTimbreSignal {
        partials,
        envelope: Envelope {
            attack: round4(attack),
            sustain: round4(sustain),
            release: round4(release),
            duration_scale: round4(duration_scale),
        },
        filter: Filter {
            kind: MusicFilterKind::Lowpass,
            frequency: round4(filter_start),
            q: round4(filter_q),
            end_frequency: filter_end.map(round4),
        },
        distance_gain: base_distance_gain,
        noise: random_noise(&mut rng, continuity),
        body: random_body(&mut rng, reference_frequency, continuity),
        pitch: random_pitch_motion(&mut rng),
    };
    let loudness_estimate = estimate_signal_loudness(&signal);
    let normalization_ceiling = if partial_count >= 12 && continuity >= 0.5 {
        0.95
    } else {
        1.35
    };
    let normalization_gain = clamp(
        0.92 / loudness_estimate.max(0.18),
        0.3,
        normalization_ceiling,
    );
    signal.distance_gain = round4(base_distance_gain * normalization_gain);

    SpectralTimbre {
        id: format!("random-{serial:02}"),
        seed: seed.to_string(),
        parameters: SpectralTimbreParameters {
            partial_count,
            partial_energy: round4(
                signal
                    .partials
                    .iter()
                    .map(|partial| partial.gain * partial.gain)
                    .sum(),
            ),
            alpha: round4(alpha),
            smoothness: round4(smooth_field.smoothness),
            roughness: round4(roughness),
            dropout_rate: round4(dropout_rate),
            dropout_depth: round4(dropout_depth),
            ratio_drift: round4(ratio_drift),
            decay_base: round4(decay_base),
            decay_slope: round4(decay_slope),
            continuity: round4(continuity),
            filter_start: round4(filter_start),
            filter_end: filter_end.map(round4),
            loudness_estimate: round4(loudness_estimate),
            normalization_gain: round4(normalization_gain),
            base_distance_gain,
            spectral_field: smooth_field
                .points
                .iter()
                .map(|point| SpectralFieldPoint {
                    center_log2: round4(point.center_log2),
                    width: round4(point.width),
                    height: round4(point.height),
                })
                .collect(),
        },
        signal,
    }
}

fn create_transient_field(seed: &str, serial: usize) -> TransientTimbre {
    let mut rng = Mulberry32::from_text(seed);
    let band_count = clamp_int(
        3.0 + (-(1.0 - rng.uniform()).max(1e-6).ln() * 2.4).floor(),
        3,
        16,
    );
    let spectral_tilt = clamp(normal(&mut rng) * 1.35, -2.6, 2.6);
    let smooth_field = random_frequency_field(&mut rng);
    let roughness = clamp(rng.uniform().powf(0.86) * 1.05, 0.02, 1.05);
    let dropout_rate = clamp(rng.uniform().powf(1.25) * 0.62, 0.0, 0.62);
    let dropout_depth = random_range(&mut rng, 0.5, 4.8);
    let low_log = 80.0_f64.log2();
    let high_log = 11_000.0_f64.log2();
    let mut bands = Vec::with_capacity(band_count);

    for index in 0..band_count {
        let position = if band_count == 1 {
            0.5
        } else {
            index as f64 / (band_count - 1) as f64
        };
        let jittered = clamp(position + normal(&mut rng) * 0.08, 0.0, 1.0);
        let log_frequency = low_log + jittered * (high_log - low_log);
        let frequency = 2.0_f64.powf(log_frequency);
        let centered = jittered - 0.5;
        let dropout_penalty = if rng.uniform() < dropout_rate {
            random_range(&mut rng, 0.45, dropout_depth)
        } else {
            0.0
        };
        let log_gain = spectral_tilt * centered
            + smooth_field.value(log_frequency)
            + normal(&mut rng) * roughness
            - dropout_penalty;
        let decay = clamp(
            log_normal(&mut rng, 0.09_f64.ln(), 0.78) * (1.45 - jittered * 0.65),
            0.018,
            0.72,
        );
        bands.push(TransientBand {
            frequency: round4(frequency),
            q: round4(random_range(&mut rng, 0.42, 3.4)),
            gain: log_gain.exp(),
            decay: round4(decay),
        });
    }
    let noise_energy = bands
        .iter()
        .map(|band| band.gain * band.gain)
        .sum::<f64>()
        .sqrt();
    let noise_energy_scale = 1.0 / noise_energy.max(0.0001);
    for band in &mut bands {
        band.gain = round4(band.gain * noise_energy_scale);
    }

    let attack = clamp(log_normal(&mut rng, 0.0045_f64.ln(), 0.9), 0.0008, 0.075);
    let decay = clamp(log_normal(&mut rng, 0.16_f64.ln(), 0.78), 0.025, 0.9);
    let release = clamp(log_normal(&mut rng, 0.035_f64.ln(), 0.7), 0.008, 0.22);
    let body = random_transient_body(&mut rng, smooth_field.brightness);
    let click_gain = rng.uniform().powf(1.7) * 0.32;
    let resonators = random_transient_resonators(&mut rng, smooth_field.brightness);
    let click = (click_gain > 0.035).then(|| TransientClick {
        gain: round4(click_gain),
        frequency: round4(log_range(&mut rng, 900.0, 9000.0)),
        q: round4(random_range(&mut rng, 0.25, 1.3)),
        decay: round4(random_range(&mut rng, 0.006, 0.035)),
    });
    let mut signal = TransientSignal {
        bands,
        envelope: TransientEnvelope {
            attack: round4(attack),
            decay: round4(decay),
            release: round4(release),
        },
        click,
        resonators,
        body,
        distance_gain: round4(clamp(0.74 + normal(&mut rng) * 0.12, 0.48, 0.98)),
    };
    let loudness_estimate = estimate_transient_loudness(&signal);
    let normalization_gain = clamp(0.62 / loudness_estimate.max(0.16), 0.28, 1.25);
    signal.distance_gain = round4(signal.distance_gain * normalization_gain);

    TransientTimbre {
        id: format!("transient-{serial:02}"),
        seed: seed.to_string(),
        parameters: TransientParameters {
            band_count,
            noise_energy: round4(signal.bands.iter().map(|band| band.gain * band.gain).sum()),
            spectral_tilt: round4(spectral_tilt),
            smoothness: round4(smooth_field.smoothness),
            roughness: round4(roughness),
            dropout_rate: round4(dropout_rate),
            dropout_depth: round4(dropout_depth),
            attack: round4(attack),
            decay: round4(decay),
            release: round4(release),
            click_gain: signal.click.as_ref().map_or(0.0, |click| click.gain),
            resonator_count: signal.resonators.len(),
            loudness_estimate: round4(loudness_estimate),
            normalization_gain: round4(normalization_gain),
            spectral_field: smooth_field
                .points
                .iter()
                .map(|point| SpectralFieldPoint {
                    center_log2: round4(point.center_log2),
                    width: round4(point.width),
                    height: round4(point.height),
                })
                .collect(),
        },
        signal,
    }
}

fn random_frequency_field(rng: &mut Mulberry32) -> SmoothField {
    let point_count = clamp_int(
        2.0 + (-(1.0 - rng.uniform()).max(1e-6).ln() * 1.5).floor(),
        2,
        8,
    );
    let smoothness = clamp(random_range(rng, 0.18, 1.25), 0.18, 1.25);
    let low_log = 90.0_f64.log2();
    let high_log = 11_000.0_f64.log2();
    let mut points = Vec::with_capacity(point_count);
    let mut brightness = 0.0;
    for _ in 0..point_count {
        let center_log2 = random_range(rng, low_log, high_log);
        let height = normal(rng) * smoothness;
        points.push(SpectralFieldPoint {
            center_log2,
            width: random_range(rng, 0.22, 1.5),
            height,
        });
        brightness += height * ((center_log2 - low_log) / (high_log - low_log) - 0.5);
    }
    SmoothField {
        smoothness,
        brightness: clamp(brightness / point_count.max(1) as f64, -1.2, 1.2),
        points,
    }
}

fn random_transient_body(rng: &mut Mulberry32, brightness: f64) -> Option<TransientBody> {
    (rng.uniform() <= 0.42).then(|| TransientBody {
        frequency: round4(log_range(
            rng,
            90.0,
            if brightness > 0.25 { 4200.0 } else { 1600.0 },
        )),
        q: round4(random_range(rng, 0.5, 2.2)),
        gain: round4(rng.uniform().powf(1.6) * 0.2),
    })
}

fn random_transient_resonators(rng: &mut Mulberry32, brightness: f64) -> Vec<TransientResonator> {
    let max = if rng.uniform() < 0.78 { 1 } else { 3 };
    let count = clamp_int((rng.uniform() * (max + 1) as f64).floor(), 0, 3);
    (0..count)
        .map(|_| TransientResonator {
            frequency: round4(log_range(
                rng,
                70.0,
                if brightness > 0.35 { 3600.0 } else { 1200.0 },
            )),
            gain: round4(rng.uniform().powf(1.8) * 0.42),
            decay: round4(log_range(rng, 0.045, 0.75)),
        })
        .collect()
}

fn estimate_transient_loudness(signal: &TransientSignal) -> f64 {
    let band_power = signal
        .bands
        .iter()
        .map(|band| {
            let duration_weight = clamp(0.25 + band.decay * 2.2, 0.28, 1.2);
            let brightness_weight = 1.0 + (band.frequency / 1300.0).log2().max(0.0) * 0.06;
            band.gain * band.gain * duration_weight * brightness_weight
        })
        .sum::<f64>();
    let resonator_power = signal
        .resonators
        .iter()
        .map(|item| item.gain * item.gain * clamp(item.decay * 1.8, 0.2, 1.4))
        .sum::<f64>();
    let click_power = signal
        .click
        .as_ref()
        .map_or(0.0, |click| click.gain * click.gain * 0.24);
    let body_weight = 1.0 + signal.body.as_ref().map_or(0.0, |body| body.gain) * 0.5;
    let envelope_weight = 0.52 + signal.envelope.decay * 1.15;
    (band_power + resonator_power + click_power)
        .max(0.0001)
        .sqrt()
        * body_weight
        * envelope_weight
}

fn random_smooth_spectral_field(rng: &mut Mulberry32) -> SmoothField {
    let point_count = clamp_int(
        2.0 + (-(1.0 - rng.uniform()).max(1e-6).ln() * 1.35).floor(),
        2,
        7,
    );
    let smoothness = clamp(random_range(rng, 0.1, 0.86), 0.1, 0.86);
    let mut points = Vec::with_capacity(point_count);
    let mut brightness = 0.0;
    for _ in 0..point_count {
        let center_log2 = random_range(rng, 0.15, 4.85);
        let height = normal(rng) * smoothness;
        points.push(SpectralFieldPoint {
            center_log2,
            width: random_range(rng, 0.35, 1.8),
            height,
        });
        brightness += height * (center_log2 - 2.2);
    }
    SmoothField {
        smoothness,
        brightness: clamp(brightness / point_count.max(1) as f64, -1.1, 1.1),
        points,
    }
}

fn random_noise(rng: &mut Mulberry32, continuity: f64) -> Option<SpectralNoise> {
    let attack_gain =
        rng.uniform().powf(1.9 + continuity * 0.8) * (0.16 + (1.0 - continuity) * 0.16);
    let sustain_gain = rng.uniform().powf(2.5 - continuity * 0.5) * (0.09 + continuity * 0.18);
    let carrier_chance = rng.uniform();
    if carrier_chance < 0.055 + continuity * 0.075 {
        return Some(noise(
            NoiseRole::Carrier,
            round4(random_range(rng, 0.08, 0.28)),
            round4(random_range(rng, 0.28, 1.05)),
            round4(log_range(rng, 520.0, 3200.0)),
            round4(random_range(rng, 0.32, 1.05)),
        ));
    }
    if sustain_gain > 0.035_f64.max(attack_gain * 0.8) {
        return Some(noise(
            NoiseRole::Sustain,
            round4(sustain_gain),
            round4(random_range(rng, 0.38, 1.15)),
            round4(log_range(rng, 700.0, 3600.0)),
            round4(random_range(rng, 0.28, 0.82)),
        ));
    }
    (attack_gain > 0.035).then(|| {
        noise(
            NoiseRole::Attack,
            round4(attack_gain),
            round4(random_range(rng, 0.018, 0.11)),
            round4(log_range(rng, 650.0, 4200.0)),
            round4(random_range(rng, 0.35, 1.1)),
        )
    })
}

fn noise(role: NoiseRole, gain: f64, decay: f64, frequency: f64, q: f64) -> SpectralNoise {
    SpectralNoise {
        role,
        gain,
        decay,
        filter: Filter {
            kind: MusicFilterKind::Bandpass,
            frequency,
            q,
            end_frequency: None,
        },
    }
}

fn random_body(
    rng: &mut Mulberry32,
    reference_frequency: f64,
    continuity: f64,
) -> Option<ResonantBody> {
    let draw = rng.uniform();
    let body_scale = 1.0 - continuity * 0.55;
    if draw < 0.22 * body_scale {
        return Some(ResonantBody::Bandpass {
            frequency: round4(log_range(
                rng,
                reference_frequency * 0.75,
                reference_frequency * 4.2,
            )),
            q: round4(random_range(rng, 0.55, 2.4)),
            gain: round4(rng.uniform().powf(1.8) * 0.18),
        });
    }
    (draw < 0.36 * body_scale).then(|| ResonantBody::Comb {
        delay: round4(random_range(rng, 0.004, 0.026)),
        feedback: round4(random_range(rng, 0.06, 0.26)),
        gain: round4(rng.uniform().powf(1.7) * 0.14),
    })
}

fn random_pitch_motion(rng: &mut Mulberry32) -> Option<PitchMotion> {
    let vibrato = rng.uniform() < 0.16;
    let jitter = rng.uniform() < 0.1;
    (vibrato || jitter).then(|| PitchMotion {
        vibrato_cents: vibrato.then(|| round4(random_range(rng, 3.5, 17.0))),
        vibrato_rate: vibrato.then(|| round4(random_range(rng, 4.2, 6.6))),
        jitter_cents: jitter.then(|| round4(random_range(rng, 2.5, 18.0))),
        jitter_rate: jitter.then(|| round4(random_range(rng, 8.0, 20.0))),
    })
}

fn estimate_signal_loudness(signal: &SpectralTimbreSignal) -> f64 {
    let partial_power = signal
        .partials
        .iter()
        .map(|partial| {
            let duration_weight = partial
                .decay
                .map(|decay| clamp(0.3 + 1.0 / (1.0 + decay * 0.28), 0.32, 0.82))
                .unwrap_or(1.0);
            partial.gain * partial.gain * duration_weight
        })
        .sum::<f64>();
    let (noise_gain, noise_weight) = signal.noise.as_ref().map_or((0.0, 0.0), |noise| {
        (
            noise.gain,
            match noise.role {
                NoiseRole::Carrier => 0.95,
                NoiseRole::Sustain => 0.58,
                NoiseRole::Attack => 0.24,
            },
        )
    });
    let sustain_weight = 0.58 + signal.envelope.sustain * signal.envelope.duration_scale * 0.62;
    let brightness_weight = 1.0 + (signal.filter.frequency / 1400.0).log2().max(0.0) * 0.08;
    let body_weight = 1.0
        + signal.body.as_ref().map_or(0.0, |body| match body {
            ResonantBody::Bandpass { gain, .. } | ResonantBody::Comb { gain, .. } => *gain,
        }) * 0.55;
    (partial_power + noise_gain * noise_gain * noise_weight)
        .max(0.0001)
        .sqrt()
        * sustain_weight
        * brightness_weight
        * body_weight
}

fn normal(rng: &mut Mulberry32) -> f64 {
    let left = rng.uniform().max(1e-8);
    let right = rng.uniform().max(1e-8);
    (-2.0 * left.ln()).sqrt() * (2.0 * std::f64::consts::PI * right).cos()
}

fn log_normal(rng: &mut Mulberry32, mean_log: f64, sigma: f64) -> f64 {
    (mean_log + normal(rng) * sigma).exp()
}

fn random_range(rng: &mut Mulberry32, min: f64, max: f64) -> f64 {
    min + rng.uniform() * (max - min)
}

fn log_range(rng: &mut Mulberry32, min: f64, max: f64) -> f64 {
    (min.ln() + rng.uniform() * (max.ln() - min.ln())).exp()
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.min(max).max(min)
}

fn clamp_int(value: f64, min: usize, max: usize) -> usize {
    (value.floor() as usize).min(max).max(min)
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0 + 0.5).floor() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectral_timbre_is_deterministic_and_uses_the_seed_as_identity() {
        let left = generate_spectral_timbre("same-seed:field:identity");
        let right = generate_spectral_timbre("same-seed:field:identity");
        assert_eq!(left, right);
        assert_eq!(left.seed, "same-seed:field:identity");
        assert_eq!(left.parameters.partial_count, 3);
        assert_eq!(left.parameters.partial_energy, 1.0001);
        assert_eq!(left.parameters.alpha, 1.8366);
        assert_eq!(left.parameters.filter_start, 2410.1442);
        assert_eq!(left.parameters.filter_end, Some(226.4732));
        assert_eq!(left.signal.distance_gain, 0.919);
        assert_eq!(left.signal.partials[0].ratio, 1.0);
        assert_eq!(left.signal.partials[0].gain, 0.9513);
    }

    #[test]
    fn transient_timbre_matches_javascript_golden() {
        let timbre = generate_transient_timbre("same-seed:transient:kick");
        assert_eq!(timbre.parameters.band_count, 4);
        assert_eq!(timbre.parameters.noise_energy, 1.0001);
        assert_eq!(timbre.parameters.spectral_tilt, 1.6637);
        assert_eq!(timbre.parameters.click_gain, 0.12);
        assert_eq!(timbre.parameters.resonator_count, 2);
        assert_eq!(timbre.signal.distance_gain, 0.5681);
        assert_eq!(timbre.signal.bands[0].frequency, 80.0);
        assert_eq!(timbre.signal.bands[0].gain, 0.0814);
    }
}
