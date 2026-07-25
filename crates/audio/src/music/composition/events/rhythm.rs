use crate::prng::Mulberry32;

#[derive(Clone, Copy, Debug)]
pub(super) struct OnsetOptions {
    pub min: i32,
    pub max: i32,
    pub min_gap: i32,
    pub anchor_start: bool,
    pub anchor_end: Option<i32>,
    pub strong_beat_bias: f64,
    pub syncopation: f64,
}

pub(super) fn stochastic_onsets(
    rng: &mut Mulberry32,
    count: f64,
    options: OnsetOptions,
) -> Vec<u8> {
    let min = options.min.max(0);
    let max = options.max.min(15);
    let min_gap = options.min_gap.max(1);
    let target_count = (count.round() as usize).max(1);
    let mut selected = Vec::new();
    if options.anchor_start {
        selected.push(min);
    }
    if let Some(anchor_end) = options.anchor_end {
        selected.push(anchor_end.clamp(min, max));
    }
    let candidates = (min..=max).collect::<Vec<_>>();
    while selected.len() < target_count {
        let available = candidates
            .iter()
            .copied()
            .filter(|step| {
                !selected.contains(step)
                    && selected.iter().all(|other| (step - other).abs() >= min_gap)
            })
            .collect::<Vec<_>>();
        let pool = if available.is_empty() {
            candidates
                .iter()
                .copied()
                .filter(|step| !selected.contains(step))
                .collect::<Vec<_>>()
        } else {
            available
        };
        if pool.is_empty() {
            break;
        }
        selected.push(weighted_step(&pool, rng, options));
    }
    selected.sort_unstable();
    selected.dedup();
    selected.truncate(target_count);
    selected.into_iter().map(|step| step as u8).collect()
}

pub(super) fn weighted_step(steps: &[i32], rng: &mut Mulberry32, options: OnsetOptions) -> i32 {
    let weights = steps
        .iter()
        .map(|&step| onset_weight(step, options))
        .collect::<Vec<_>>();
    let mut ticket = rng.uniform() * weights.iter().sum::<f64>();
    for (&step, weight) in steps.iter().zip(weights) {
        ticket -= weight;
        if ticket <= 0.0 {
            return step;
        }
    }
    *steps.last().expect("onset candidates are non-empty")
}

pub(super) fn onset_weight(step: i32, options: OnsetOptions) -> f64 {
    let strong_distance = [0, 4, 8, 12]
        .into_iter()
        .map(|anchor| (step - anchor).abs())
        .min()
        .unwrap();
    let off_distance = [2, 6, 10, 14, 15]
        .into_iter()
        .map(|anchor| (step - anchor).abs())
        .min()
        .unwrap();
    0.08 + (-f64::from(strong_distance) * 0.9).exp() * options.strong_beat_bias
        + (-f64::from(off_distance) * 0.9).exp() * options.syncopation
}

pub(super) fn pulse_onsets(
    rng: &mut Mulberry32,
    count: f64,
    phase_max: i32,
    min_gap: i32,
    strong_beat_bias: f64,
    syncopation: f64,
) -> Vec<u8> {
    let phase = random_int(rng, 0, phase_max.max(0));
    let min_gap = min_gap.max(1);
    let target_count = (count.round() as usize).max(1);
    let pulse_period = (3.0 + rng.uniform() * 4.0).clamp(3.0, 7.0);
    let swing = (rng.uniform() - 0.5) * 1.8;
    let mut steps = Vec::<i32>::new();
    while steps.len() < target_count {
        let candidates = (0..16)
            .filter(|candidate| {
                steps
                    .iter()
                    .all(|other| (candidate - other).abs() >= min_gap)
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            break;
        }
        let weights = candidates
            .iter()
            .map(|&step| {
                let pulse_position = f64::from(step - phase + 16) % pulse_period;
                let folded = pulse_position.min(pulse_period - pulse_position);
                let pulse = (-(folded * folded) / 2.2).exp();
                let strong_count = steps
                    .iter()
                    .filter(|step| [0, 4, 8, 12].contains(step))
                    .count();
                let strong_penalty = if [0, 4, 8, 12].contains(&step) && strong_count >= 1 {
                    0.58
                } else {
                    1.0
                };
                let local_swing = if step % 2 == 1 {
                    1.0 + swing.max(0.0) * 0.08
                } else {
                    1.0 + (-swing).max(0.0) * 0.06
                };
                (0.08
                    + pulse * 0.52
                    + onset_weight(
                        step,
                        OnsetOptions {
                            min: 0,
                            max: 15,
                            min_gap,
                            anchor_start: false,
                            anchor_end: None,
                            strong_beat_bias,
                            syncopation,
                        },
                    ) * 0.48)
                    * strong_penalty
                    * local_swing
            })
            .collect::<Vec<_>>();
        let mut ticket = rng.uniform() * weights.iter().sum::<f64>();
        for (&candidate, weight) in candidates.iter().zip(weights) {
            ticket -= weight;
            if ticket <= 0.0 {
                steps.push(candidate);
                break;
            }
        }
    }
    steps.sort_unstable();
    steps.into_iter().map(|step| step as u8).collect()
}

pub(super) fn textural_event_count(
    rng: &mut Mulberry32,
    mean: f64,
    minimum: usize,
    burst: f64,
) -> usize {
    let base = poisson_count(rng, mean.max(0.1));
    let burst_count = if rng.uniform() < burst.clamp(0.0, 0.5) {
        let burst_mean = 1.4 + rng.uniform() * 1.6;
        1 + poisson_count(rng, burst_mean)
    } else {
        0
    };
    minimum.max(base + burst_count)
}

fn poisson_count(rng: &mut Mulberry32, mean: f64) -> usize {
    let limit = (-mean.max(0.01)).exp();
    let mut product = 1.0;
    let mut count = 0;
    loop {
        count += 1;
        product *= rng.uniform();
        if product <= limit {
            return count - 1;
        }
    }
}

fn random_int(rng: &mut Mulberry32, min: i32, max: i32) -> i32 {
    min + (rng.uniform() * f64::from(max - min + 1)).floor() as i32
}
