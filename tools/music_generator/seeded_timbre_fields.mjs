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

function envelope(attack, sustain, release, durationScale) {
  return { attack, sustain, release, durationScale };
}

function lowpass(frequency, q, endFrequency = null) {
  return { type: "lowpass", frequency, q, endFrequency };
}

function bandpass(frequency, q, endFrequency = null) {
  return { type: "bandpass", frequency, q, endFrequency };
}

function round4(value) {
  return Math.round(value * 10000) / 10000;
}
