export const AXIS_EXPERIMENTS = [
  {
    id: "harmonicity",
    label: "Harmonicity",
    controlled: ["pitch", "duration", "gain", "attack"],
    variants: [
      variant("harmonic", "Harmonic partials", ["partials: 1, 2, 3, 4"], {
        partials: [[1, 1], [2, 0.32], [3, 0.16], [4, 0.08]],
        envelope: envelope(0.02, 0.5, 0.08, 0.62),
        filter: lowpass(2200, 0.45),
      }),
      variant("odd", "Odd partials", ["partials: 1, 3, 5"], {
        partials: [[1, 0.95], [3, 0.42], [5, 0.18]],
        envelope: envelope(0.02, 0.5, 0.08, 0.62),
        filter: lowpass(1800, 0.55),
      }),
      variant("inharmonic", "Inharmonic partials", ["partials: 1, 2.73, 5.38"], {
        partials: [[1, 1], [2.73, 0.38], [5.38, 0.16]],
        envelope: envelope(0.02, 0.5, 0.08, 0.62),
        filter: bandpass(2600, 0.8),
      }),
    ],
  },
  {
    id: "noise-role",
    label: "Noise Role",
    controlled: ["pitch", "duration", "harmonic partials", "gain"],
    variants: [
      variant("none", "Tone only", ["noise: none"], {
        partials: [[1, 1], [2, 0.16]],
        envelope: envelope(0.035, 0.54, 0.08, 0.62),
        filter: lowpass(1400, 0.4),
      }),
      variant("attack", "Attack noise", ["noise: short burst"], {
        partials: [[1, 1], [2, 0.16]],
        noise: { role: "attack", gain: 0.32, decay: 0.06, filter: bandpass(1700, 0.7) },
        envelope: envelope(0.02, 0.5, 0.08, 0.62),
        filter: lowpass(1400, 0.4),
      }),
      variant("sustain", "Sustain noise", ["noise: sustained air"], {
        partials: [[1, 0.92], [2, 0.1]],
        noise: { role: "sustain", gain: 0.24, decay: 0.8, filter: bandpass(1900, 0.45) },
        envelope: envelope(0.07, 0.56, 0.1, 0.62),
        filter: lowpass(1200, 0.35),
      }),
      variant("carrier", "Noise carrier", ["noise: main source", "tone: weak support"], {
        partials: [[1, 0.22]],
        noise: { role: "carrier", gain: 0.95, decay: 0.75, filter: bandpass(1350, 0.82) },
        envelope: envelope(0.03, 0.36, 0.08, 0.62),
        filter: bandpass(900, 0.9),
        distanceGain: 0.96,
      }),
    ],
  },
  {
    id: "attack-identity",
    label: "Attack Identity",
    controlled: ["pitch", "partials after attack", "duration", "gain"],
    variants: [
      variant("soft", "Soft air", ["attack: slow", "noise: breath onset"], {
        partials: [[1, 1], [2, 0.12]],
        noise: { role: "sustain", gain: 0.12, decay: 0.7, filter: bandpass(1800, 0.5) },
        envelope: envelope(0.11, 0.58, 0.1, 0.62),
        filter: lowpass(1050, 0.35),
      }),
      variant("pluck", "Pluck impulse", ["attack: impulse", "brightness: fast decay"], {
        pluck: { damping: 4.2, brightness: 0.34, body: 0.7, noise: 0.08 },
        envelope: envelope(0.004, 0.04, 0.04, 0.56),
        filter: lowpass(1650, 0.45),
      }),
      variant("strike", "Strike", ["attack: hard", "partials: unequal decay"], {
        partials: [[1, 1, 10], [2.28, 0.28, 16], [3.9, 0.1, 24]],
        envelope: envelope(0.004, 0.03, 0.06, 0.5),
        filter: bandpass(1850, 0.62),
        distanceGain: 1.18,
      }),
      variant("gate", "Gated oscillator", ["attack: immediate", "release: short"], {
        partials: [[1, 1], [2, 0.2], [3, 0.08]],
        envelope: envelope(0.004, 0.72, 0.025, 0.34),
        filter: lowpass(1600, 0.38),
      }),
    ],
  },
  {
    id: "brightness-decay",
    label: "Brightness Decay",
    controlled: ["pitch", "attack", "duration", "initial gain"],
    variants: [
      variant("amplitude-only", "Amplitude decay only", ["filter: static"], {
        partials: [[1, 1], [2, 0.48], [3, 0.34], [4, 0.22], [6, 0.12]],
        envelope: envelope(0.006, 0.22, 0.08, 0.82),
        filter: lowpass(5200, 0.35),
      }),
      variant("filter-decay", "Filter closes", ["filter: cutoff decays"], {
        partials: [[1, 1], [2, 0.48], [3, 0.34], [4, 0.22], [6, 0.12]],
        envelope: envelope(0.006, 0.22, 0.08, 0.82),
        filter: lowpass(6500, 0.45, 260),
      }),
      variant("partial-decay", "Upper partials decay faster", ["partials: separate decay rates"], {
        partials: [[1, 1, 2.4], [2, 0.48, 7], [3, 0.34, 12], [4, 0.22, 18], [6, 0.12, 24]],
        envelope: envelope(0.006, 0.22, 0.08, 0.82),
        filter: lowpass(5200, 0.35),
      }),
    ],
  },
  {
    id: "resonance-body",
    label: "Resonance / Body",
    controlled: ["pitch", "partials", "attack", "duration"],
    variants: [
      variant("dry", "Dry tone", ["body: none"], {
        partials: [[1, 1], [2, 0.18]],
        envelope: envelope(0.018, 0.48, 0.08, 0.62),
        filter: lowpass(1700, 0.35),
      }),
      variant("formant", "Body bump", ["filter: low-mid bandpass mix"], {
        partials: [[1, 1], [2, 0.18]],
        body: { type: "bandpass", frequency: 640, q: 0.7, gain: 0.18 },
        envelope: envelope(0.018, 0.48, 0.08, 0.62),
        filter: lowpass(1700, 0.35),
      }),
      variant("comb", "Comb body", ["delay: short feedback"], {
        partials: [[1, 1], [2, 0.18]],
        body: { type: "comb", delay: 0.019, feedback: 0.16, gain: 0.14 },
        envelope: envelope(0.018, 0.48, 0.08, 0.62),
        filter: lowpass(1700, 0.35),
      }),
    ],
  },
  {
    id: "pitch-stability",
    label: "Pitch Stability",
    controlled: ["partials", "noise", "filter", "gain"],
    variants: [
      variant("stable", "Stable pitch", ["pitch: fixed"], {
        partials: [[1, 1], [2, 0.12]],
        envelope: envelope(0.03, 0.64, 0.08, 1),
        filter: lowpass(1300, 0.35),
      }),
      variant("vibrato", "Vibrato", ["pitch: periodic detune"], {
        partials: [[1, 1], [2, 0.12]],
        pitch: { vibratoCents: 22, vibratoRate: 5.4 },
        envelope: envelope(0.03, 0.64, 0.08, 1),
        filter: lowpass(1300, 0.35),
      }),
      variant("jitter", "Pitch jitter", ["pitch: small random steps"], {
        partials: [[1, 1], [2, 0.12]],
        pitch: { jitterCents: 28, jitterRate: 14 },
        envelope: envelope(0.03, 0.64, 0.08, 1),
        filter: lowpass(1300, 0.35),
      }),
    ],
  },
  {
    id: "foreground-distance",
    label: "Foreground Distance",
    controlled: ["pitch", "source family", "duration"],
    variants: [
      variant("close", "Close", ["gain: high", "attack: sharp", "filter: bright"], {
        partials: [[1, 1], [2, 0.3], [3, 0.16]],
        envelope: envelope(0.006, 0.44, 0.04, 0.7),
        filter: lowpass(2900, 0.35),
        distanceGain: 1,
      }),
      variant("middle", "Middle", ["gain: moderate", "filter: controlled"], {
        partials: [[1, 1], [2, 0.18]],
        envelope: envelope(0.024, 0.5, 0.08, 0.62),
        filter: lowpass(1450, 0.35),
        distanceGain: 0.72,
      }),
      variant("background", "Background", ["gain: low", "attack: soft", "filter: dark"], {
        partials: [[1, 1], [2, 0.08]],
        envelope: envelope(0.12, 0.62, 0.16, 0.62),
        filter: lowpass(780, 0.32),
        distanceGain: 0.46,
      }),
    ],
  },
];

export function axisExperimentSummary() {
  return AXIS_EXPERIMENTS.map((axis) => ({
    id: axis.id,
    label: axis.label,
    controlled: axis.controlled,
    variants: axis.variants.map((item) => ({
      id: item.id,
      label: item.label,
      operations: item.operations,
      mechanism: mechanismFor(item.signal),
    })),
  }));
}

export function playAxisVariant(audioContext, axisId, variantId, options = {}) {
  const axis = AXIS_EXPERIMENTS.find((item) => item.id === axisId);
  const item = axis?.variants.find((candidate) => candidate.id === variantId);
  if (!axis || !item) {
    throw new Error(`Unknown axis variant: ${axisId}/${variantId}`);
  }
  const started = audioContext.currentTime + 0.03;
  const note = Number(options.note ?? 69);
  const duration = Number(options.duration ?? 0.9);
  const gain = Number(options.gain ?? 0.18) * (item.signal.distanceGain ?? 0.78);
  return playSignal(audioContext, item.signal, note, started, duration, gain);
}

export async function measureAxisVariant(axisId, variantId, options = {}) {
  const axis = AXIS_EXPERIMENTS.find((item) => item.id === axisId);
  const item = axis?.variants.find((candidate) => candidate.id === variantId);
  if (!axis || !item) {
    throw new Error(`Unknown axis variant: ${axisId}/${variantId}`);
  }
  const sampleRate = Number(options.sampleRate ?? 16000);
  const duration = Number(options.duration ?? 1);
  const context = new OfflineAudioContext(1, Math.ceil(sampleRate * duration), sampleRate);
  playSignal(context, item.signal, Number(options.note ?? 69), 0.02, Number(options.noteDuration ?? 0.9), Number(options.gain ?? 0.18) * (item.signal.distanceGain ?? 0.78));
  const buffer = await context.startRendering();
  return extractAudioFeatures(buffer.getChannelData(0), sampleRate, Number(options.fundamental ?? midiToFrequency(Number(options.note ?? 69))));
}

export async function measureAxis(axisId, options = {}) {
  const axis = AXIS_EXPERIMENTS.find((item) => item.id === axisId);
  if (!axis) {
    throw new Error(`Unknown axis: ${axisId}`);
  }
  const measurements = [];
  for (const item of axis.variants) {
    measurements.push({
      axisId,
      variantId: item.id,
      label: item.label,
      features: await measureAxisVariant(axisId, item.id, options),
    });
  }
  return {
    axisId,
    label: axis.label,
    measurements,
    distances: normalizedDistanceMatrix(measurements.map((item) => item.features)),
  };
}

export function findAxisVariant(axisId, variantId) {
  const axis = AXIS_EXPERIMENTS.find((item) => item.id === axisId);
  return axis?.variants.find((item) => item.id === variantId) ?? null;
}

export function generateRandomTimbre(seed = "random") {
  return createDistributedTimbre(String(seed), 1);
}

export function generateTimbreSet(seed = "random", count = 12) {
  const size = Math.max(1, Math.min(64, Math.floor(Number(count) || 12)));
  return Array.from({ length: size }, (_, index) => createDistributedTimbre(`${seed}:${index + 1}`, index + 1));
}

export function randomTimbreSummary(model) {
  const parameters = model.parameters;
  const parts = [
    `partials ${parameters.partialCount}`,
    `energy ${parameters.partialEnergy.toFixed(2)}`,
    `alpha ${parameters.alpha.toFixed(2)}`,
    `smooth ${parameters.smoothness.toFixed(2)}`,
    `rough ${parameters.roughness.toFixed(2)}`,
    `drop ${parameters.dropoutRate.toFixed(2)}`,
    `drift ${parameters.ratioDrift.toFixed(4)}`,
    `decay ${parameters.decaySlope.toFixed(2)}`,
    `norm ${parameters.normalizationGain.toFixed(2)}`,
    `noise ${parameters.noiseRole}`,
    `body ${parameters.bodyType}`,
  ];
  return parts.join(" / ");
}

export function playGeneratedTimbre(audioContext, model, options = {}) {
  const started = audioContext.currentTime + 0.03;
  const note = Number(options.note ?? 69);
  const duration = Number(options.duration ?? 0.9);
  const gain = Number(options.gain ?? 0.18) * (model.signal.distanceGain ?? 0.72);
  return playSignal(audioContext, model.signal, note, started, duration, gain);
}

export function generateRandomTransient(seed = "transient") {
  return createTransientField(String(seed), 1);
}

export function generateTransientSet(seed = "transient", count = 12) {
  const size = Math.max(1, Math.min(64, Math.floor(Number(count) || 12)));
  return Array.from({ length: size }, (_, index) => createTransientField(`${seed}:${index + 1}`, index + 1));
}

export function randomTransientSummary(model) {
  const parameters = model.parameters;
  const parts = [
    `bands ${parameters.bandCount}`,
    `energy ${parameters.noiseEnergy.toFixed(2)}`,
    `tilt ${parameters.spectralTilt.toFixed(2)}`,
    `smooth ${parameters.smoothness.toFixed(2)}`,
    `rough ${parameters.roughness.toFixed(2)}`,
    `drop ${parameters.dropoutRate.toFixed(2)}`,
    `attack ${parameters.attack.toFixed(3)}s`,
    `decay ${parameters.decay.toFixed(3)}s`,
    `res ${parameters.resonatorCount}`,
    `norm ${parameters.normalizationGain.toFixed(2)}`,
  ];
  return parts.join(" / ");
}

export function playGeneratedTransient(audioContext, model, options = {}) {
  const started = audioContext.currentTime + 0.03;
  const duration = Number(options.duration ?? 0.65);
  const gain = Number(options.gain ?? 0.18) * (model.signal.distanceGain ?? 0.78);
  return playTransientSignal(audioContext, model.signal, started, duration, gain);
}

function createDistributedTimbre(seed, serial) {
  const rng = randomFromSeed(seed);
  const referenceFrequency = 440;
  const partialCount = clampInt(3 + Math.floor(-Math.log(Math.max(1e-6, 1 - rng())) * 4.2), 3, 24);
  const alpha = clamp(logNormal(rng, Math.log(1.18), 0.58), 0.32, 3.45);
  const smoothField = randomSmoothSpectralField(rng);
  const roughness = clamp(rng() ** 0.88 * 0.58, 0.01, 0.58);
  const dropoutRate = clamp(rng() ** 1.35 * 0.58, 0, 0.58);
  const dropoutDepth = randomRange(rng, 0.6, 4.2);
  const ratioTail = rng() < 0.055 ? randomRange(rng, 0.008, 0.038) : 0;
  const ratioDrift = clamp(logNormal(rng, Math.log(0.00145), 0.92) + ratioTail, 0.00002, 0.048);
  const ratioBend = normal(rng) * ratioDrift * 0.11;
  const decayBase = clamp(logNormal(rng, Math.log(1.15), 0.95), 0.08, 9.4);
  const decaySlope = clamp(normal(rng) * 0.78 + randomRange(rng, -0.35, 0.75), -1.08, 2.18);
  const continuity = clamp(1 / (1 + decayBase * 0.42 + Math.max(0, decaySlope) * 0.72 - Math.min(0, decaySlope) * 0.32), 0.08, 0.96);
  const partials = [];

  for (let n = 1; n <= partialCount; n += 1) {
    const logN = Math.log(n);
    const log2N = Math.log2(n);
    const ratio = n === 1 ? 1 : Math.max(0.2, n * (1 + ratioBend * (n - 1) + normal(rng) * ratioDrift * Math.sqrt(n - 1)));
    const dropoutChance = n > 1 ? dropoutRate * (0.35 + 0.65 * Math.log1p(n) / Math.log1p(partialCount)) : 0;
    const dropoutPenalty = rng() < dropoutChance ? randomRange(rng, 0.55, dropoutDepth) : 0;
    const logGain = -alpha * logN + smoothField.value(log2N) + normal(rng) * roughness - dropoutPenalty;
    const rawGain = Math.exp(logGain);
    const decay = decayBase + decaySlope * logN + normal(rng) * 0.4;
    const partial = [round4(ratio), rawGain];
    if (decay > 0.42) {
      partial.push(round4(decay));
    }
    partials.push(partial);
  }

  const partialEnergy = Math.sqrt(partials.reduce((sum, partial) => sum + partial[1] * partial[1], 0));
  const partialEnergyScale = 1 / Math.max(0.0001, partialEnergy);
  for (let index = 0; index < partials.length; index += 1) {
    partials[index][1] = round4(partials[index][1] * partialEnergyScale);
  }

  const attackCenter = 0.008 + (1 - continuity) * 0.04;
  const attack = clamp(logNormal(rng, Math.log(attackCenter), 0.86), 0.002, 0.22);
  const sustain = clamp(0.12 + continuity * 0.72 + normal(rng) * 0.18, 0.04, 0.95);
  const release = clamp(logNormal(rng, Math.log(0.045 + continuity * 0.075), 0.66), 0.012, 0.32);
  const durationScale = clamp(0.28 + continuity * 0.62 + normal(rng) * 0.16, 0.18, 1);
  const filterStart = clamp(logNormal(rng, Math.log(2200 + smoothField.brightness * 760), 0.62), 320, 7600);
  const filterEnds = rng() < 0.14 + (1 - continuity) * 0.3;
  const filterEnd = filterEnds ? clamp(filterStart * randomRange(rng, 0.08, 0.68), 120, filterStart * 0.82) : null;
  const filterQ = randomRange(rng, 0.26, 0.82);

  const baseDistanceGain = round4(clamp(0.7 + normal(rng) * 0.13, 0.42, 0.98));
  const signal = {
    partials,
    envelope: envelope(round4(attack), round4(sustain), round4(release), round4(durationScale)),
    filter: lowpass(round4(filterStart), round4(filterQ), filterEnd ? round4(filterEnd) : null),
    distanceGain: baseDistanceGain,
  };

  const noise = randomNoise(rng, continuity);
  if (noise) {
    signal.noise = noise;
  }

  const body = randomBody(rng, referenceFrequency, continuity);
  if (body) {
    signal.body = body;
  }

  const pitch = randomPitchMotion(rng);
  if (pitch) {
    signal.pitch = pitch;
  }

  const loudnessEstimate = estimateSignalLoudness(signal);
  const normalizationCeiling = partialCount >= 12 && continuity >= 0.5 ? 0.95 : 1.35;
  const normalizationGain = clamp(0.92 / Math.max(0.18, loudnessEstimate), 0.3, normalizationCeiling);
  signal.distanceGain = round4(baseDistanceGain * normalizationGain);

  return {
    id: `random-${String(serial).padStart(2, "0")}`,
    seed,
    signal,
    parameters: {
      partialCount,
      partialEnergy: round4(partials.reduce((sum, partial) => sum + partial[1] * partial[1], 0)),
      alpha: round4(alpha),
      smoothness: round4(smoothField.smoothness),
      roughness: round4(roughness),
      dropoutRate: round4(dropoutRate),
      dropoutDepth: round4(dropoutDepth),
      ratioDrift: round4(ratioDrift),
      decayBase: round4(decayBase),
      decaySlope: round4(decaySlope),
      continuity: round4(continuity),
      filterStart: round4(filterStart),
      filterEnd: filterEnd ? round4(filterEnd) : null,
      loudnessEstimate: round4(loudnessEstimate),
      normalizationGain: round4(normalizationGain),
      baseDistanceGain,
      noiseRole: signal.noise?.role ?? "none",
      bodyType: signal.body?.type ?? "none",
      spectralField: smoothField.points.map((point) => ({
        centerLog2: round4(point.centerLog2),
        width: round4(point.width),
        height: round4(point.height),
      })),
    },
  };
}

function createTransientField(seed, serial) {
  const rng = randomFromSeed(seed);
  const bandCount = clampInt(3 + Math.floor(-Math.log(Math.max(1e-6, 1 - rng())) * 2.4), 3, 16);
  const spectralTilt = clamp(normal(rng) * 1.35, -2.6, 2.6);
  const smoothField = randomFrequencyField(rng);
  const roughness = clamp(rng() ** 0.86 * 1.05, 0.02, 1.05);
  const dropoutRate = clamp(rng() ** 1.25 * 0.62, 0, 0.62);
  const dropoutDepth = randomRange(rng, 0.5, 4.8);
  const lowLog = Math.log2(80);
  const highLog = Math.log2(11000);
  const bands = [];

  for (let index = 0; index < bandCount; index += 1) {
    const position = bandCount === 1 ? 0.5 : index / (bandCount - 1);
    const jittered = clamp(position + normal(rng) * 0.08, 0, 1);
    const logFrequency = lowLog + jittered * (highLog - lowLog);
    const frequency = 2 ** logFrequency;
    const centered = jittered - 0.5;
    const dropoutPenalty = rng() < dropoutRate ? randomRange(rng, 0.45, dropoutDepth) : 0;
    const logGain = spectralTilt * centered + smoothField.value(logFrequency) + normal(rng) * roughness - dropoutPenalty;
    const decay = clamp(logNormal(rng, Math.log(0.09), 0.78) * (1.45 - jittered * 0.65), 0.018, 0.72);
    bands.push({
      frequency: round4(frequency),
      q: round4(randomRange(rng, 0.42, 3.4)),
      gain: Math.exp(logGain),
      decay: round4(decay),
    });
  }

  const noiseEnergy = Math.sqrt(bands.reduce((sum, band) => sum + band.gain * band.gain, 0));
  const noiseEnergyScale = 1 / Math.max(0.0001, noiseEnergy);
  for (const band of bands) {
    band.gain = round4(band.gain * noiseEnergyScale);
  }

  const attack = clamp(logNormal(rng, Math.log(0.0045), 0.9), 0.0008, 0.075);
  const decay = clamp(logNormal(rng, Math.log(0.16), 0.78), 0.025, 0.9);
  const release = clamp(logNormal(rng, Math.log(0.035), 0.7), 0.008, 0.22);
  const body = randomTransientBody(rng, smoothField.brightness);
  const clickGain = rng() ** 1.7 * 0.32;
  const resonators = randomTransientResonators(rng, smoothField.brightness);

  const signal = {
    bands,
    envelope: { attack: round4(attack), decay: round4(decay), release: round4(release) },
    click: clickGain > 0.035 ? {
      gain: round4(clickGain),
      frequency: round4(logRange(rng, 900, 9000)),
      q: round4(randomRange(rng, 0.25, 1.3)),
      decay: round4(randomRange(rng, 0.006, 0.035)),
    } : null,
    resonators,
    body,
    distanceGain: round4(clamp(0.74 + normal(rng) * 0.12, 0.48, 0.98)),
  };
  const loudnessEstimate = estimateTransientLoudness(signal);
  const normalizationGain = clamp(0.62 / Math.max(0.16, loudnessEstimate), 0.28, 1.25);
  signal.distanceGain = round4(signal.distanceGain * normalizationGain);

  return {
    id: `transient-${String(serial).padStart(2, "0")}`,
    seed,
    signal,
    parameters: {
      bandCount,
      noiseEnergy: round4(bands.reduce((sum, band) => sum + band.gain * band.gain, 0)),
      spectralTilt: round4(spectralTilt),
      smoothness: round4(smoothField.smoothness),
      roughness: round4(roughness),
      dropoutRate: round4(dropoutRate),
      dropoutDepth: round4(dropoutDepth),
      attack: round4(attack),
      decay: round4(decay),
      release: round4(release),
      clickGain: signal.click?.gain ?? 0,
      resonatorCount: resonators.length,
      bodyType: body?.type ?? "none",
      loudnessEstimate: round4(loudnessEstimate),
      normalizationGain: round4(normalizationGain),
      spectralField: smoothField.points.map((point) => ({
        centerLog2: round4(point.centerLog2),
        width: round4(point.width),
        height: round4(point.height),
      })),
    },
  };
}

function randomFrequencyField(rng) {
  const pointCount = clampInt(2 + Math.floor(-Math.log(Math.max(1e-6, 1 - rng())) * 1.5), 2, 8);
  const smoothness = clamp(randomRange(rng, 0.18, 1.25), 0.18, 1.25);
  const lowLog = Math.log2(90);
  const highLog = Math.log2(11000);
  const points = [];
  let brightness = 0;
  for (let index = 0; index < pointCount; index += 1) {
    const centerLog2 = randomRange(rng, lowLog, highLog);
    const height = normal(rng) * smoothness;
    points.push({
      centerLog2,
      width: randomRange(rng, 0.22, 1.5),
      height,
    });
    brightness += height * ((centerLog2 - lowLog) / (highLog - lowLog) - 0.5);
  }
  return {
    smoothness,
    brightness: clamp(brightness / Math.max(1, pointCount), -1.2, 1.2),
    points,
    value(logFrequency) {
      let sum = 0;
      for (const point of points) {
        const distance = logFrequency - point.centerLog2;
        sum += point.height * Math.exp(-(distance * distance) / (2 * point.width * point.width));
      }
      return sum;
    },
  };
}

function randomTransientBody(rng, brightness) {
  if (rng() > 0.42) {
    return null;
  }
  return {
    type: "bandpass",
    frequency: round4(logRange(rng, 90, brightness > 0.25 ? 4200 : 1600)),
    q: round4(randomRange(rng, 0.5, 2.2)),
    gain: round4(rng() ** 1.6 * 0.2),
  };
}

function randomTransientResonators(rng, brightness) {
  const max = rng() < 0.78 ? 1 : 3;
  const count = clampInt(Math.floor(rng() * (max + 1)), 0, 3);
  const result = [];
  for (let index = 0; index < count; index += 1) {
    result.push({
      frequency: round4(logRange(rng, 70, brightness > 0.35 ? 3600 : 1200)),
      gain: round4(rng() ** 1.8 * 0.42),
      decay: round4(logRange(rng, 0.045, 0.75)),
    });
  }
  return result;
}

function estimateTransientLoudness(signal) {
  const bandPower = (signal.bands ?? []).reduce((sum, band) => {
    const durationWeight = clamp(0.25 + band.decay * 2.2, 0.28, 1.2);
    const brightnessWeight = 1 + Math.max(0, Math.log2(band.frequency / 1300)) * 0.06;
    return sum + band.gain * band.gain * durationWeight * brightnessWeight;
  }, 0);
  const resonatorPower = (signal.resonators ?? []).reduce((sum, item) => sum + item.gain * item.gain * clamp(item.decay * 1.8, 0.2, 1.4), 0);
  const clickPower = signal.click ? signal.click.gain * signal.click.gain * 0.24 : 0;
  const bodyWeight = 1 + (signal.body?.gain ?? 0) * 0.5;
  const envelopeWeight = 0.52 + (signal.envelope?.decay ?? 0.16) * 1.15;
  return Math.sqrt(Math.max(0.0001, bandPower + resonatorPower + clickPower)) * bodyWeight * envelopeWeight;
}

function estimateSignalLoudness(signal) {
  let partialPower = 0;
  for (const partial of signal.partials ?? []) {
    const gain = partial[1] ?? 0;
    const decay = partial[2];
    const durationWeight = decay ? clamp(0.3 + 1 / (1 + decay * 0.28), 0.32, 0.82) : 1;
    partialPower += gain * gain * durationWeight;
  }
  const noiseGain = signal.noise?.gain ?? 0;
  const noiseWeight = signal.noise?.role === "carrier" ? 0.95 : signal.noise?.role === "sustain" ? 0.58 : signal.noise?.role === "attack" ? 0.24 : 0;
  const noisePower = noiseGain * noiseGain * noiseWeight;
  const envelope = signal.envelope ?? {};
  const sustainWeight = 0.58 + (envelope.sustain ?? 0.5) * (envelope.durationScale ?? 0.7) * 0.62;
  const filterFrequency = signal.filter?.frequency ?? 1600;
  const brightnessWeight = 1 + Math.max(0, Math.log2(filterFrequency / 1400)) * 0.08;
  const bodyWeight = 1 + (signal.body?.gain ?? 0) * 0.55;
  return Math.sqrt(Math.max(0.0001, partialPower + noisePower)) * sustainWeight * brightnessWeight * bodyWeight;
}

function randomSmoothSpectralField(rng) {
  const pointCount = clampInt(2 + Math.floor(-Math.log(Math.max(1e-6, 1 - rng())) * 1.35), 2, 7);
  const smoothness = clamp(randomRange(rng, 0.1, 0.86), 0.1, 0.86);
  const peaks = [];
  let brightness = 0;
  for (let index = 0; index < pointCount; index += 1) {
    const centerLog2 = randomRange(rng, 0.15, 4.85);
    const height = normal(rng) * smoothness;
    peaks.push({
      centerLog2,
      width: randomRange(rng, 0.35, 1.8),
      height,
    });
    brightness += height * (centerLog2 - 2.2);
  }
  return {
    smoothness,
    brightness: clamp(brightness / Math.max(1, pointCount), -1.1, 1.1),
    points: peaks,
    value(log2N) {
      let sum = 0;
      for (const peak of peaks) {
        const distance = log2N - peak.centerLog2;
        sum += peak.height * Math.exp(-(distance * distance) / (2 * peak.width * peak.width));
      }
      return sum;
    },
  };
}

function randomNoise(rng, continuity) {
  const attackGain = rng() ** (1.9 + continuity * 0.8) * (0.16 + (1 - continuity) * 0.16);
  const sustainGain = rng() ** (2.5 - continuity * 0.5) * (0.09 + continuity * 0.18);
  const carrierChance = rng();
  if (carrierChance < 0.055 + continuity * 0.075) {
    return {
      role: "carrier",
      gain: round4(randomRange(rng, 0.08, 0.28)),
      decay: round4(randomRange(rng, 0.28, 1.05)),
      filter: bandpass(round4(logRange(rng, 520, 3200)), round4(randomRange(rng, 0.32, 1.05))),
    };
  }
  if (sustainGain > Math.max(0.035, attackGain * 0.8)) {
    return {
      role: "sustain",
      gain: round4(sustainGain),
      decay: round4(randomRange(rng, 0.38, 1.15)),
      filter: bandpass(round4(logRange(rng, 700, 3600)), round4(randomRange(rng, 0.28, 0.82))),
    };
  }
  if (attackGain > 0.035) {
    return {
      role: "attack",
      gain: round4(attackGain),
      decay: round4(randomRange(rng, 0.018, 0.11)),
      filter: bandpass(round4(logRange(rng, 650, 4200)), round4(randomRange(rng, 0.35, 1.1))),
    };
  }
  return null;
}

function randomBody(rng, referenceFrequency, continuity) {
  const draw = rng();
  const bodyScale = 1 - continuity * 0.55;
  if (draw < 0.22 * bodyScale) {
    return {
      type: "bandpass",
      frequency: round4(logRange(rng, referenceFrequency * 0.75, referenceFrequency * 4.2)),
      q: round4(randomRange(rng, 0.55, 2.4)),
      gain: round4(rng() ** 1.8 * 0.18),
    };
  }
  if (draw < 0.36 * bodyScale) {
    return {
      type: "comb",
      delay: round4(randomRange(rng, 0.004, 0.026)),
      feedback: round4(randomRange(rng, 0.06, 0.26)),
      gain: round4(rng() ** 1.7 * 0.14),
    };
  }
  return null;
}

function randomPitchMotion(rng) {
  const vibrato = rng() < 0.16;
  const jitter = rng() < 0.1;
  if (!vibrato && !jitter) {
    return null;
  }
  const pitch = {};
  if (vibrato) {
    pitch.vibratoCents = round4(randomRange(rng, 3.5, 17));
    pitch.vibratoRate = round4(randomRange(rng, 4.2, 6.6));
  }
  if (jitter) {
    pitch.jitterCents = round4(randomRange(rng, 2.5, 18));
    pitch.jitterRate = round4(randomRange(rng, 8, 20));
  }
  return pitch;
}

function randomFromSeed(seed) {
  let state = 2166136261;
  const text = String(seed);
  for (let index = 0; index < text.length; index += 1) {
    state ^= text.charCodeAt(index);
    state = Math.imul(state, 16777619);
  }
  return () => {
    state += 0x6d2b79f5;
    let value = state;
    value = Math.imul(value ^ (value >>> 15), value | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
  };
}

function normal(rng) {
  const left = Math.max(1e-8, rng());
  const right = Math.max(1e-8, rng());
  return Math.sqrt(-2 * Math.log(left)) * Math.cos(2 * Math.PI * right);
}

function logNormal(rng, meanLog, sigma) {
  return Math.exp(meanLog + normal(rng) * sigma);
}

function randomRange(rng, min, max) {
  return min + rng() * (max - min);
}

function logRange(rng, min, max) {
  return Math.exp(Math.log(min) + rng() * (Math.log(max) - Math.log(min)));
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function clampInt(value, min, max) {
  return Math.min(max, Math.max(min, Math.floor(value)));
}

function variant(id, label, operations, signal) {
  return { id, label, operations, signal };
}

function envelope(attack, sustain, release, durationScale) {
  return { attack, sustain, release, durationScale };
}

function lowpass(frequency, q, endFrequency = null) {
  return { type: "lowpass", frequency, q, endFrequency };
}

function bandpass(frequency, q, endFrequency = null) {
  return { type: "bandpass", frequency, q, endFrequency };
}

function mechanismFor(signal) {
  if (signal.pluck) {
    return "buffer-pluck";
  }
  if (signal.noise?.role === "carrier") {
    return "noise-carrier";
  }
  if (signal.body?.type === "comb") {
    return "comb-body";
  }
  if (signal.body?.type === "bandpass") {
    return "formant-body";
  }
  if (signal.partials?.some((partial) => partial[2])) {
    return "additive-separate-decay";
  }
  return "additive";
}

function playSignal(audioContext, signal, midiNote, startsAt, duration, gainValue) {
  const active = [];
  const output = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  const envelopeConfig = signal.envelope ?? envelope(0.02, 0.5, 0.08, 0.7);
  const effectiveDuration = duration * (envelopeConfig.durationScale ?? 0.7);
  filter.type = signal.filter?.type ?? "lowpass";
  filter.frequency.setValueAtTime(signal.filter?.frequency ?? 1600, startsAt);
  if (signal.filter?.endFrequency) {
    filter.frequency.exponentialRampToValueAtTime(Math.max(20, signal.filter.endFrequency), startsAt + effectiveDuration);
  }
  filter.Q.value = signal.filter?.q ?? 0.4;
  filter.gain.value = signal.filter?.gain ?? 0;
  output.gain.setValueAtTime(0.0001, startsAt);
  output.gain.exponentialRampToValueAtTime(Math.max(0.0001, gainValue), startsAt + envelopeConfig.attack);
  output.gain.exponentialRampToValueAtTime(Math.max(0.0001, gainValue * envelopeConfig.sustain), startsAt + Math.max(envelopeConfig.attack + 0.02, effectiveDuration * 0.55));
  output.gain.exponentialRampToValueAtTime(0.0001, startsAt + effectiveDuration + envelopeConfig.release);
  const sourceInput = connectBody(audioContext, output, signal.body, startsAt, effectiveDuration, active);
  output.connect(filter).connect(audioContext.destination);

  if (signal.pluck) {
    active.push(playPluckBuffer(audioContext, midiNote, startsAt, duration, signal.pluck, sourceInput));
  } else {
    for (const partial of signal.partials ?? [[1, 1]]) {
      active.push(playPartial(audioContext, midiNote, startsAt, effectiveDuration, partial, signal.pitch, sourceInput));
    }
  }
  if (signal.noise) {
    active.push(playNoise(audioContext, startsAt, effectiveDuration, signal.noise, sourceInput));
  }
  return {
    stop() {
      for (const source of active) {
        try {
          source.stop();
        } catch {
          // Already stopped sources can be ignored.
        }
      }
    },
  };
}

function playTransientSignal(audioContext, signal, startsAt, duration, gainValue) {
  const active = [];
  const output = audioContext.createGain();
  const envelopeConfig = signal.envelope ?? { attack: 0.004, decay: 0.18, release: 0.03 };
  const effectiveDuration = Math.min(duration, envelopeConfig.decay + envelopeConfig.release + 0.12);
  output.gain.setValueAtTime(0.0001, startsAt);
  output.gain.exponentialRampToValueAtTime(Math.max(0.0001, gainValue), startsAt + envelopeConfig.attack);
  output.gain.exponentialRampToValueAtTime(0.0001, startsAt + effectiveDuration);
  const sourceInput = connectBody(audioContext, output, signal.body, startsAt, effectiveDuration, active);
  output.connect(audioContext.destination);

  const noiseDuration = Math.max(0.05, effectiveDuration + 0.05);
  for (const band of signal.bands ?? []) {
    active.push(playNoiseBand(audioContext, startsAt, noiseDuration, band, sourceInput));
  }
  if (signal.click) {
    active.push(playNoiseBand(audioContext, startsAt, Math.max(0.025, signal.click.decay + 0.02), signal.click, sourceInput));
  }
  for (const resonator of signal.resonators ?? []) {
    active.push(playTransientResonator(audioContext, startsAt, resonator, sourceInput));
  }
  return {
    stop() {
      for (const source of active) {
        try {
          source.stop();
        } catch {
          // Already stopped sources can be ignored.
        }
      }
    },
  };
}

function playNoiseBand(audioContext, startsAt, duration, band, destination) {
  const samples = Math.max(1, Math.floor(audioContext.sampleRate * duration));
  const buffer = audioContext.createBuffer(1, samples, audioContext.sampleRate);
  const data = buffer.getChannelData(0);
  let seed = Math.imul(Math.floor((band.frequency ?? 1200) * 1000), 16777619) ^ 2166136261;
  for (let i = 0; i < samples; i += 1) {
    seed ^= i + 1;
    seed = Math.imul(seed, 16777619);
    data[i] = ((seed >>> 0) / 4294967295) * 2 - 1;
  }
  const source = audioContext.createBufferSource();
  const filter = audioContext.createBiquadFilter();
  const gain = audioContext.createGain();
  filter.type = "bandpass";
  filter.frequency.value = band.frequency ?? 1500;
  filter.Q.value = band.q ?? 1;
  gain.gain.setValueAtTime(Math.max(0.0001, band.gain ?? 0.1), startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.008, band.decay ?? 0.12));
  source.buffer = buffer;
  source.connect(filter).connect(gain).connect(destination);
  source.start(startsAt);
  source.stop(startsAt + duration + 0.02);
  return source;
}

function playTransientResonator(audioContext, startsAt, resonator, destination) {
  const oscillator = audioContext.createOscillator();
  const gain = audioContext.createGain();
  oscillator.type = "sine";
  oscillator.frequency.setValueAtTime(resonator.frequency ?? 160, startsAt);
  gain.gain.setValueAtTime(Math.max(0.0001, resonator.gain ?? 0.08), startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.02, resonator.decay ?? 0.16));
  oscillator.connect(gain).connect(destination);
  oscillator.start(startsAt);
  oscillator.stop(startsAt + Math.max(0.04, (resonator.decay ?? 0.16) + 0.08));
  return oscillator;
}

function connectBody(audioContext, dryInput, body, startsAt, duration, active) {
  if (!body) {
    return dryInput;
  }
  const input = audioContext.createGain();
  const dry = audioContext.createGain();
  const wet = audioContext.createGain();
  dry.gain.value = 1;
  wet.gain.value = body.gain ?? 0.25;
  input.connect(dry).connect(dryInput);
  if (body.type === "bandpass") {
    const filter = audioContext.createBiquadFilter();
    filter.type = "bandpass";
    filter.frequency.value = body.frequency;
    filter.Q.value = body.q ?? 1;
    input.connect(filter).connect(wet).connect(dryInput);
  }
  if (body.type === "comb") {
    const delay = audioContext.createDelay(0.05);
    const feedback = audioContext.createGain();
    delay.delayTime.value = body.delay ?? 0.012;
    feedback.gain.value = body.feedback ?? 0.25;
    input.connect(delay).connect(feedback).connect(delay);
    delay.connect(wet).connect(dryInput);
    const silent = audioContext.createConstantSource();
    silent.offset.value = 0;
    active.push(silent);
    silent.start(startsAt);
    silent.stop(startsAt + duration + 0.2);
  }
  return input;
}

function playPartial(audioContext, midiNote, startsAt, duration, partial, pitch, destination) {
  const [ratio, amount, decay] = partial;
  const oscillator = audioContext.createOscillator();
  const gain = audioContext.createGain();
  oscillator.type = "sine";
  oscillator.frequency.setValueAtTime(midiToFrequency(midiNote) * ratio, startsAt);
  applyPitchMotion(audioContext, oscillator, startsAt, duration, pitch);
  gain.gain.setValueAtTime(Math.max(0.0001, amount), startsAt);
  if (decay) {
    gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.03, 1 / decay + duration * 0.35));
  }
  oscillator.connect(gain).connect(destination);
  oscillator.start(startsAt);
  oscillator.stop(startsAt + duration + 0.2);
  return oscillator;
}

function applyPitchMotion(audioContext, oscillator, startsAt, duration, pitch) {
  if (!pitch) {
    return;
  }
  if (pitch.vibratoCents) {
    const lfo = audioContext.createOscillator();
    const amount = audioContext.createGain();
    lfo.frequency.value = pitch.vibratoRate ?? 5;
    amount.gain.value = pitch.vibratoCents;
    lfo.connect(amount).connect(oscillator.detune);
    lfo.start(startsAt);
    lfo.stop(startsAt + duration + 0.2);
  }
  if (pitch.jitterCents) {
    const rate = pitch.jitterRate ?? 16;
    const steps = Math.floor(duration * rate);
    let seed = 2166136261;
    for (let index = 0; index <= steps; index += 1) {
      seed = Math.imul(seed ^ (index + 31), 16777619);
      const value = ((seed >>> 0) / 4294967295 - 0.5) * pitch.jitterCents * 2;
      oscillator.detune.setValueAtTime(value, startsAt + index / rate);
    }
  }
}

function playNoise(audioContext, startsAt, duration, noise, destination) {
  const samples = Math.max(1, Math.floor(audioContext.sampleRate * Math.max(duration, noise.decay ?? 0.06)));
  const buffer = audioContext.createBuffer(1, samples, audioContext.sampleRate);
  const data = buffer.getChannelData(0);
  let seed = 2166136261;
  for (let i = 0; i < samples; i += 1) {
    seed ^= i + 1;
    seed = Math.imul(seed, 16777619);
    const random = ((seed >>> 0) / 4294967295) * 2 - 1;
    const t = i / audioContext.sampleRate;
    const decay = noise.role === "attack" ? Math.exp(-t / (noise.decay ?? 0.06)) : Math.exp(-t * 0.7);
    data[i] = random * decay;
  }
  const source = audioContext.createBufferSource();
  const gain = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  filter.type = noise.filter?.type ?? "bandpass";
  filter.frequency.value = noise.filter?.frequency ?? 1500;
  filter.Q.value = noise.filter?.q ?? 0.7;
  gain.gain.setValueAtTime(Math.max(0.0001, noise.gain), startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.04, duration));
  source.buffer = buffer;
  source.connect(filter).connect(gain).connect(destination);
  source.start(startsAt);
  return source;
}

function playPluckBuffer(audioContext, midiNote, startsAt, duration, pluck, destination) {
  const sampleRate = audioContext.sampleRate;
  const seconds = Math.max(0.25, duration);
  const samples = Math.floor(sampleRate * seconds);
  const buffer = audioContext.createBuffer(1, samples, sampleRate);
  const data = buffer.getChannelData(0);
  const frequency = midiToFrequency(midiNote);
  let seed = 2166136261;
  for (let i = 0; i < samples; i += 1) {
    seed ^= i + 1;
    seed = Math.imul(seed, 16777619);
    const random = ((seed >>> 0) / 4294967295) * 2 - 1;
    const t = i / sampleRate;
    const env = Math.exp(-(pluck.damping ?? 4) * t);
    const body = Math.sin(2 * Math.PI * frequency * t) * (pluck.body ?? 0.6);
    const detuned = Math.sin(2 * Math.PI * frequency * 0.997 * t) * 0.2;
    const upper = Math.sin(2 * Math.PI * frequency * 1.52 * t) * (pluck.brightness ?? 0.25);
    const noise = random * Math.exp(-18 * t) * (pluck.noise ?? 0.05);
    data[i] = (body + detuned + upper + noise) * env;
  }
  const source = audioContext.createBufferSource();
  source.buffer = buffer;
  source.connect(destination);
  source.start(startsAt);
  return source;
}

function midiToFrequency(note) {
  return 440 * 2 ** ((note - 69) / 12);
}

function extractAudioFeatures(samples, sampleRate, fundamental) {
  const frameSize = 512;
  const hop = 512;
  const frames = [];
  for (let start = 0; start + frameSize <= samples.length; start += hop) {
    const frame = samples.subarray(start, start + frameSize);
    const rms = Math.sqrt(frame.reduce((sum, value) => sum + value * value, 0) / frame.length);
    if (rms < 0.0002) {
      continue;
    }
    frames.push(analyzeFrame(frame, sampleRate, fundamental, rms));
    if (frames.length >= 12) {
      break;
    }
  }
  if (frames.length === 0) {
    return emptyFeatures();
  }
  const rmsEnvelope = frameRmsEnvelope(samples, frameSize, hop);
  return {
    rms: round4(Math.sqrt(samples.reduce((sum, value) => sum + value * value, 0) / samples.length)),
    attackTime: round4(attackTime(rmsEnvelope, hop, sampleRate)),
    decayTime: round4(decayTime(rmsEnvelope, hop, sampleRate)),
    spectralCentroid: round4(mean(frames.map((frame) => frame.centroid))),
    spectralSpread: round4(mean(frames.map((frame) => frame.spread))),
    spectralFlatness: round4(mean(frames.map((frame) => frame.flatness))),
    harmonicEnergyRatio: round4(mean(frames.map((frame) => frame.harmonicEnergyRatio))),
    oddEvenRatio: round4(mean(frames.map((frame) => frame.oddEvenRatio))),
    inharmonicEnergyRatio: round4(mean(frames.map((frame) => frame.inharmonicEnergyRatio))),
    pitchSalience: round4(mean(frames.map((frame) => frame.pitchSalience))),
    spectralFlux: round4(spectralFlux(frames)),
  };
}

function analyzeFrame(frame, sampleRate, fundamental, rms) {
  const spectrum = magnitudeSpectrum(frame);
  const binHz = sampleRate / frame.length;
  let total = 0;
  let weighted = 0;
  for (let bin = 1; bin < spectrum.length; bin += 1) {
    const magnitude = spectrum[bin];
    const frequency = bin * binHz;
    total += magnitude;
    weighted += magnitude * frequency;
  }
  const centroid = total > 0 ? weighted / total : 0;
  let spreadTotal = 0;
  let logSum = 0;
  let harmonicEnergy = 0;
  let oddEnergy = 0;
  let evenEnergy = 0;
  let fundamentalEnergy = 0;
  for (let bin = 1; bin < spectrum.length; bin += 1) {
    const magnitude = spectrum[bin];
    const frequency = bin * binHz;
    spreadTotal += magnitude * (frequency - centroid) ** 2;
    logSum += Math.log(Math.max(1e-9, magnitude));
    const harmonic = Math.max(1, Math.round(frequency / fundamental));
    const distance = Math.abs(frequency - harmonic * fundamental);
    if (distance <= Math.max(18, fundamental * 0.035)) {
      harmonicEnergy += magnitude;
      if (harmonic === 1) {
        fundamentalEnergy += magnitude;
      } else if (harmonic % 2 === 0) {
        evenEnergy += magnitude;
      } else {
        oddEnergy += magnitude;
      }
    }
  }
  const arithmeticMean = total / Math.max(1, spectrum.length - 1);
  const geometricMean = Math.exp(logSum / Math.max(1, spectrum.length - 1));
  return {
    rms,
    magnitudes: spectrum,
    centroid,
    spread: total > 0 ? Math.sqrt(spreadTotal / total) : 0,
    flatness: arithmeticMean > 0 ? geometricMean / arithmeticMean : 0,
    harmonicEnergyRatio: total > 0 ? harmonicEnergy / total : 0,
    oddEvenRatio: evenEnergy > 0 ? oddEnergy / evenEnergy : oddEnergy > 0 ? 9 : 0,
    inharmonicEnergyRatio: total > 0 ? 1 - harmonicEnergy / total : 0,
    pitchSalience: total > 0 ? fundamentalEnergy / total : 0,
  };
}

function magnitudeSpectrum(frame) {
  const half = frame.length / 2;
  const result = new Array(half).fill(0);
  for (let k = 0; k < half; k += 1) {
    let real = 0;
    let imag = 0;
    for (let n = 0; n < frame.length; n += 1) {
      const windowed = frame[n] * (0.5 - 0.5 * Math.cos((2 * Math.PI * n) / (frame.length - 1)));
      const phase = (2 * Math.PI * k * n) / frame.length;
      real += windowed * Math.cos(phase);
      imag -= windowed * Math.sin(phase);
    }
    result[k] = Math.sqrt(real * real + imag * imag);
  }
  return result;
}

function frameRmsEnvelope(samples, frameSize, hop) {
  const values = [];
  for (let start = 0; start + frameSize <= samples.length; start += hop) {
    const frame = samples.subarray(start, start + frameSize);
    values.push(Math.sqrt(frame.reduce((sum, value) => sum + value * value, 0) / frame.length));
  }
  return values;
}

function attackTime(envelopeValues, hop, sampleRate) {
  const peak = Math.max(...envelopeValues);
  if (peak <= 0) {
    return 0;
  }
  const threshold = peak * 0.9;
  const index = envelopeValues.findIndex((value) => value >= threshold);
  return index < 0 ? 0 : (index * hop) / sampleRate;
}

function decayTime(envelopeValues, hop, sampleRate) {
  const peak = Math.max(...envelopeValues);
  if (peak <= 0) {
    return 0;
  }
  const peakIndex = envelopeValues.findIndex((value) => value === peak);
  const threshold = peak * 0.25;
  for (let index = peakIndex; index < envelopeValues.length; index += 1) {
    if (envelopeValues[index] <= threshold) {
      return ((index - peakIndex) * hop) / sampleRate;
    }
  }
  return ((envelopeValues.length - peakIndex) * hop) / sampleRate;
}

function spectralFlux(frames) {
  if (frames.length < 2) {
    return 0;
  }
  const values = [];
  for (let index = 1; index < frames.length; index += 1) {
    const prev = frames[index - 1].magnitudes;
    const next = frames[index].magnitudes;
    const prevSum = prev.reduce((sum, value) => sum + value, 0) || 1;
    const nextSum = next.reduce((sum, value) => sum + value, 0) || 1;
    let sum = 0;
    for (let bin = 0; bin < Math.min(prev.length, next.length); bin += 1) {
      const diff = next[bin] / nextSum - prev[bin] / prevSum;
      sum += diff * diff;
    }
    values.push(Math.sqrt(sum));
  }
  return mean(values);
}

function normalizedDistanceMatrix(features) {
  const keys = Object.keys(emptyFeatures()).filter((key) => key !== "rms");
  const stats = Object.fromEntries(keys.map((key) => {
    const values = features.map((item) => item[key]);
    const min = Math.min(...values);
    const max = Math.max(...values);
    return [key, { min, max }];
  }));
  return features.map((left) => features.map((right) => {
    let sum = 0;
    let count = 0;
    for (const key of keys) {
      const { min, max } = stats[key];
      const range = max - min;
      if (range <= 1e-9) {
        continue;
      }
      const diff = (left[key] - right[key]) / range;
      sum += diff * diff;
      count += 1;
    }
    return round4(count > 0 ? Math.sqrt(sum / count) : 0);
  }));
}

function emptyFeatures() {
  return {
    rms: 0,
    attackTime: 0,
    decayTime: 0,
    spectralCentroid: 0,
    spectralSpread: 0,
    spectralFlatness: 0,
    harmonicEnergyRatio: 0,
    oddEvenRatio: 0,
    inharmonicEnergyRatio: 0,
    pitchSalience: 0,
    spectralFlux: 0,
  };
}

function mean(values) {
  return values.reduce((sum, value) => sum + value, 0) / Math.max(1, values.length);
}

function round4(value) {
  return Math.round(value * 10000) / 10000;
}
