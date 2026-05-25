export const SFX_TYPES = ["jump", "pickup", "hit", "explosion", "laser", "powerup", "select", "error"];
export const SFX_TYPE_OPTIONS = ["random", ...SFX_TYPES, "wild"];

const TYPE_CONFIG = {
  wild: { duration: 0.5, label: "Wild", baseFrequency: [60, 1400], shape: "freeform" },
  jump: { duration: 0.28, label: "Jump", baseFrequency: [230, 330], shape: "rise" },
  pickup: { duration: 0.42, label: "Pickup", baseFrequency: [620, 880], shape: "spark" },
  hit: { duration: 0.24, label: "Hit", baseFrequency: [90, 160], shape: "impact" },
  explosion: { duration: 0.82, label: "Explosion", baseFrequency: [45, 82], shape: "blast" },
  laser: { duration: 0.36, label: "Laser", baseFrequency: [460, 760], shape: "sweep" },
  powerup: { duration: 0.72, label: "Power Up", baseFrequency: [260, 430], shape: "climb" },
  select: { duration: 0.14, label: "Select", baseFrequency: [540, 820], shape: "tap" },
  error: { duration: 0.42, label: "Error", baseFrequency: [170, 260], shape: "fall" },
};

const TYPE_PATTERNS = {
  wild: ["tone", "noise", "clicks", "sweep", "broken", "stack"],
  jump: ["hop", "spring", "rubber", "whoosh"],
  pickup: ["coin", "sparkle", "gem", "chord"],
  hit: ["punch", "slash", "metal", "crunch"],
  explosion: ["boom", "puff", "crackle", "burst"],
  laser: ["pew", "zap", "down", "charge"],
  powerup: ["arpeggio", "swell", "sparkle", "fanfare"],
  select: ["tick", "blip", "confirm", "soft"],
  error: ["buzzer", "fall", "double", "glitch"],
};

export function generateSoundEffect(seed, options = {}) {
  const seedText = String(seed);
  const overrideType = normalizeType(options.type);
  const type = overrideType ?? typeFromSeed(seedText);
  const rng = mulberry32(hashSeed(seedText));
  const mood = clamp(Number(options.mood ?? rng()), 0, 1);
  const intensity = clamp(Number(options.intensity ?? lerp(0.45, 0.92, rng())), 0, 1);
  const length = clamp(Number(options.length ?? rng()), 0, 1);
  const tonalFamily = mood < 0.4 ? "dark" : mood > 0.6 ? "bright" : "neutral";
  const config = TYPE_CONFIG[type];
  const duration = round3(config.duration * lerp(0.72, 1.45, length) * lerp(0.94, 1.06, rng()));
  const baseFrequency = randomInt(rng, config.baseFrequency[0], config.baseFrequency[1]);
  const profile = buildProfile(rng, type, tonalFamily, intensity);
  const layers = buildLayers(type, baseFrequency, duration, mood, intensity, profile, rng);

  return {
    seed: seedText,
    typeOverride: overrideType,
    type,
    label: config.label,
    mood,
    intensity,
    length,
    tonalFamily,
    duration,
    profile,
    layers,
  };
}

export function randomSfxPreset(seed = Date.now(), targetType = null) {
  const rng = mulberry32(hashSeed(String(seed)));
  const type = typeof targetType === "string" && SFX_TYPE_OPTIONS.includes(targetType)
    ? targetType
    : "random";
  return {
    seed: randomInt(rng, 100000, 999999).toString(),
    type,
  };
}

export function createSfxPlayer(audioContext, effect) {
  const activeSources = new Set();

  function start() {
    stop();
    const startedAt = audioContext.currentTime + 0.03;
    for (const layer of effect.layers) {
      playLayer(audioContext, layer, startedAt, activeSources);
    }
  }

  function stop() {
    for (const source of activeSources) {
      try {
        source.stop();
      } catch {
        // Already-ended sources can be ignored.
      }
      source.disconnect();
    }
    activeSources.clear();
  }

  return { start, stop };
}

function buildProfile(rng, type, tonalFamily, intensity) {
  const bright = tonalFamily === "bright";
  const dark = tonalFamily === "dark";
  const waveforms = bright ? ["triangle", "sine", "square"] : dark ? ["sawtooth", "square", "triangle"] : ["sine", "triangle", "square"];
  const variants = type === "error" ? ["clean", "double", "gritty", "stepped"] : ["clean", "double", "gritty", "hollow", "wide", "stepped"];
  return {
    engine: pick(["arcade", "soft-synth", "bit-crush", "toy-speaker"], rng),
    variant: pick(variants, rng),
    pattern: pick(TYPE_PATTERNS[type], rng),
    waveform: type === "error" ? pick(["square", "sawtooth"], rng) : pick(waveforms, rng),
    noiseColor: dark || type === "explosion" || type === "error" ? "crackle" : "white",
    filterBias: round2(lerp(0.75, 1.35, intensity) * (type === "error" ? 0.78 : bright ? 1.18 : dark ? 0.82 : 1)),
    pitchWobble: round2(lerp(type === "error" ? 0.04 : 0.01, type === "error" ? 0.12 : 0.075, rng() * intensity)),
  };
}

function buildLayers(type, baseFrequency, duration, mood, intensity, profile, rng) {
  let layers;
  if (type === "wild") {
    const layerCount = randomInt(rng, 1, 5);
    layers = [];
    for (let i = 0; i < layerCount; i += 1) {
      const start = duration * lerp(0, 0.72, rng());
      const layerDuration = duration * lerp(0.08, 0.95 - Math.min(0.65, start / duration), rng());
      const layerKind = profile.pattern === "noise" ? "noise"
        : profile.pattern === "clicks" ? "click"
          : profile.pattern === "stack" ? "tone"
            : pick(["tone", "tone", "noise", "click"], rng);

      if (layerKind === "tone") {
        const sweep = profile.pattern === "sweep" ? lerp(0.18, 4.4, rng()) : lerp(0.35, 3.6, rng());
        layers.push(toneLayer(
          `wild-tone-${i + 1}`,
          start,
          layerDuration,
          pick(["sine", "triangle", "square", "sawtooth"], rng),
          baseFrequency * lerp(0.2, 3.2, rng()),
          baseFrequency * sweep,
          lerp(0.04, 0.28, rng()),
          intensity,
          profile,
        ));
      } else if (layerKind === "noise") {
        layers.push(noiseLayer(
          `wild-noise-${i + 1}`,
          start,
          layerDuration,
          lerp(0.03, 0.26, rng()),
          pick(["lowpass", "highpass", "bandpass"], rng),
          randomInt(rng, 160, 9000),
          randomInt(rng, 80, 9200),
          profile,
        ));
      } else {
        layers.push(clickLayer(start, lerp(0.008, 0.045, rng()), lerp(0.04, 0.22, rng()), randomInt(rng, 420, 9000)));
      }
    }

    if (profile.pattern === "broken") {
      layers.push(clickLayer(duration * lerp(0.16, 0.84, rng()), 0.012, 0.08 + intensity * 0.08, randomInt(rng, 1200, 7600)));
      layers.push(noiseLayer("wild-gap-noise", duration * lerp(0.34, 0.68, rng()), duration * 0.16, 0.08 + intensity * 0.08, "bandpass", randomInt(rng, 600, 6400), randomInt(rng, 180, 2800), profile));
    }
    return varyLayers(layers, duration, baseFrequency, intensity, profile, rng);
  }

  if (type === "jump") {
    if (profile.pattern === "spring") {
      layers = [
        toneLayer("coil-1", 0, duration * 0.55, "square", baseFrequency * 0.85, baseFrequency * 2.6, 0.22, intensity, profile),
        toneLayer("coil-2", duration * 0.22, duration * 0.45, "triangle", baseFrequency * 1.3, baseFrequency * 2.2, 0.16, intensity, profile),
        toneLayer("coil-3", duration * 0.42, duration * 0.35, "sine", baseFrequency * 1.7, baseFrequency * 2.9, 0.12, intensity, profile),
      ];
    } else if (profile.pattern === "rubber") {
      layers = [
        toneLayer("boing", 0, duration * 1.1, "sine", baseFrequency * 0.72, baseFrequency * 1.9, 0.36, intensity, profile),
        toneLayer("bend", duration * 0.12, duration * 0.72, "triangle", baseFrequency * 0.9, baseFrequency * 1.35, 0.14, intensity, profile),
      ];
    } else if (profile.pattern === "whoosh") {
      layers = [
        noiseLayer("lift-air", 0, duration * 0.86, 0.16 + intensity * 0.12, "highpass", 900, 5400, profile),
        toneLayer("body", duration * 0.06, duration * 0.58, "triangle", baseFrequency, baseFrequency * 2.2, 0.18, intensity, profile),
        clickLayer(0, 0.02, 0.08 + intensity * 0.06, 1800),
      ];
    } else {
      layers = [
        toneLayer("body", 0, duration, profile.waveform, baseFrequency, baseFrequency * lerp(1.85, 2.35, mood), 0.34, intensity, profile),
        toneLayer("edge", 0.015, duration * 0.62, "square", baseFrequency * 1.5, baseFrequency * 2.65, 0.12, intensity, profile),
        clickLayer(0, 0.026, 0.08 + intensity * 0.08, 2800 * profile.filterBias),
      ];
    }
    return varyLayers(layers, duration, baseFrequency, intensity, profile, rng);
  }

  if (type === "pickup") {
    const interval = duration / 4.8;
    if (profile.pattern === "coin") {
      layers = [
        toneLayer("coin", 0, duration * 0.34, "square", baseFrequency * 1.7, baseFrequency * 2.05, 0.24, intensity, profile),
        toneLayer("ring", duration * 0.06, duration * 0.62, "sine", baseFrequency * 2.45, baseFrequency * 2.42, 0.12, intensity, profile),
        clickLayer(0, 0.014, 0.08 + intensity * 0.08, 6200),
      ];
    } else if (profile.pattern === "gem") {
      layers = [
        toneLayer("gem-low", 0, duration * 0.7, "sine", baseFrequency * 0.92, baseFrequency * 0.94, 0.16, intensity, profile),
        toneLayer("gem-high", duration * 0.1, duration * 0.72, "triangle", baseFrequency * 2.15, baseFrequency * 2.18, 0.2, intensity, profile),
        noiseLayer("shine", duration * 0.16, duration * 0.36, 0.08, "highpass", 4800, 9000, profile),
      ];
    } else if (profile.pattern === "chord") {
      layers = [
        toneLayer("chord-root", 0, duration * 0.62, "triangle", baseFrequency, baseFrequency, 0.16, intensity, profile),
        toneLayer("chord-third", 0.012, duration * 0.58, "sine", baseFrequency * 1.25, baseFrequency * 1.25, 0.15, intensity, profile),
        toneLayer("chord-fifth", 0.024, duration * 0.54, "sine", baseFrequency * 1.5, baseFrequency * 1.5, 0.16, intensity, profile),
      ];
    } else {
      layers = [
        toneLayer("note-1", 0, duration * 0.45, "sine", baseFrequency, baseFrequency * 1.01, 0.18, intensity, profile),
        toneLayer("note-2", interval, duration * 0.42, "triangle", baseFrequency * pick([1.2, 1.25, 1.333], rng), baseFrequency * 1.26, 0.18, intensity, profile),
        toneLayer("note-3", interval * pick([1.75, 2, 2.35], rng), duration * 0.52, "sine", baseFrequency * pick([1.5, 1.667, 2], rng), baseFrequency * 1.5, 0.22, intensity, profile),
        noiseLayer("sparkle", interval * 2.1, duration * 0.28, 0.07 + intensity * 0.04, "highpass", 5200, 7600, profile),
      ];
    }
    return varyLayers(layers, duration, baseFrequency, intensity, profile, rng);
  }

  if (type === "hit") {
    if (profile.pattern === "slash") {
      layers = [
        noiseLayer("slice", 0, duration * 0.68, 0.18 + intensity * 0.2, "highpass", 6200, 1200, profile),
        clickLayer(0, 0.018, 0.12 + intensity * 0.14, 5200),
      ];
    } else if (profile.pattern === "metal") {
      layers = [
        toneLayer("clang", 0, duration * 1.12, "square", baseFrequency * 4.2, baseFrequency * 2.8, 0.22, intensity, profile),
        toneLayer("ring", duration * 0.04, duration * 0.95, "sine", baseFrequency * 6.1, baseFrequency * 5.7, 0.12, intensity, profile),
        clickLayer(0, 0.016, 0.14 + intensity * 0.12, 7200),
      ];
    } else if (profile.pattern === "crunch") {
      layers = [
        toneLayer("low-hit", 0, duration * 0.72, "sine", baseFrequency * 1.5, baseFrequency * 0.44, 0.34, intensity, profile),
        noiseLayer("crunch", 0, duration * 0.88, 0.26 + intensity * 0.22, "bandpass", 2600, 680, profile),
        noiseLayer("dust", duration * 0.12, duration * 0.62, 0.12, "lowpass", 900, 220, profile),
      ];
    } else {
      layers = [
        toneLayer("thud", 0, duration * 0.9, "sine", baseFrequency * 1.8, baseFrequency * 0.55, 0.4, intensity, profile),
        noiseLayer("body", 0, duration * 0.62, 0.22 + intensity * 0.2, "lowpass", 1800, 380, profile),
        clickLayer(0, 0.018, 0.18 + intensity * 0.18, 3200 * profile.filterBias),
      ];
    }
    return varyLayers(layers, duration, baseFrequency, intensity, profile, rng);
  }

  if (type === "explosion") {
    if (profile.pattern === "puff") {
      layers = [
        noiseLayer("puff", 0, duration * 0.72, 0.28 + intensity * 0.18, "lowpass", 1200, 140, profile),
        toneLayer("soft-sub", 0, duration * 0.42, "sine", baseFrequency, baseFrequency * 0.54, 0.2, intensity, profile),
      ];
    } else if (profile.pattern === "crackle") {
      layers = [
        noiseLayer("crackle", 0, duration * 0.95, 0.3 + intensity * 0.32, "bandpass", 3200, 420, profile),
        noiseLayer("smoke", duration * 0.2, duration * 0.8, 0.16 + intensity * 0.16, "lowpass", 900, 120, profile),
        clickLayer(0, 0.03, 0.18 + intensity * 0.18, 1400),
        clickLayer(duration * 0.16, 0.024, 0.08 + intensity * 0.12, 2200),
      ];
    } else if (profile.pattern === "burst") {
      layers = [
        toneLayer("blast-tone", 0, duration * 0.48, "sawtooth", baseFrequency * 3, baseFrequency * 0.5, 0.34, intensity, profile),
        noiseLayer("blast-noise", 0, duration * 0.7, 0.36 + intensity * 0.28, "highpass", 5400, 360, profile),
        clickLayer(0, 0.022, 0.22 + intensity * 0.14, 4200),
      ];
    } else {
      layers = [
        toneLayer("sub", 0, duration * 0.78, "sine", baseFrequency * 1.5, baseFrequency * 0.42, 0.46, intensity, profile),
        noiseLayer("blast", 0.01, duration, 0.35 + intensity * 0.34, "lowpass", 2600 * profile.filterBias, 170, profile),
        noiseLayer("debris", duration * 0.18, duration * 0.52, 0.12 + intensity * 0.16, "bandpass", 1200, 520, profile),
        clickLayer(0, 0.035, 0.2 + intensity * 0.16, 900),
      ];
    }
    return varyLayers(layers, duration, baseFrequency, intensity, profile, rng);
  }

  if (type === "laser") {
    const direction = mood > 0.5 ? 2.6 : 0.34;
    if (profile.pattern === "zap") {
      layers = [
        toneLayer("zap", 0, duration * 0.62, "square", baseFrequency * 1.6, baseFrequency * 0.42, 0.32, intensity, profile),
        clickLayer(0, 0.014, 0.12 + intensity * 0.08, 6400),
        noiseLayer("electric", duration * 0.05, duration * 0.32, 0.06 + intensity * 0.06, "bandpass", 5200, 1800, profile),
      ];
    } else if (profile.pattern === "down") {
      layers = [
        toneLayer("falling-beam", 0, duration, "sawtooth", baseFrequency * 2.8, baseFrequency * 0.38, 0.28, intensity, profile),
        toneLayer("thin-edge", duration * 0.08, duration * 0.54, "square", baseFrequency * 3.1, baseFrequency * 0.62, 0.1, intensity, profile),
      ];
    } else if (profile.pattern === "charge") {
      layers = [
        toneLayer("charge", 0, duration * 0.5, "triangle", baseFrequency * 0.55, baseFrequency * 2.4, 0.16, intensity, profile),
        toneLayer("fire", duration * 0.46, duration * 0.48, "square", baseFrequency * 2.4, baseFrequency * 0.7, 0.28, intensity, profile),
        clickLayer(duration * 0.46, 0.02, 0.12 + intensity * 0.1, 4600),
      ];
    } else {
      layers = [
        toneLayer("beam", 0, duration, profile.waveform, baseFrequency, baseFrequency * direction, 0.28, intensity, profile),
        toneLayer("alias", duration * 0.08, duration * 0.72, "square", baseFrequency * 1.01, baseFrequency * direction * 1.02, 0.11, intensity, profile),
        noiseLayer("air", 0, duration * 0.35, 0.05 + intensity * 0.06, "highpass", 4200, 1800, profile),
      ];
    }
    return varyLayers(layers, duration, baseFrequency, intensity, profile, rng);
  }

  if (type === "powerup") {
    const step = duration / 5.6;
    if (profile.pattern === "swell") {
      layers = [
        toneLayer("swell", 0, duration * 0.96, "sine", baseFrequency * 0.72, baseFrequency * 2.2, 0.32, intensity, profile),
        noiseLayer("air-lift", duration * 0.2, duration * 0.58, 0.08 + intensity * 0.06, "highpass", 1400, 7200, profile),
      ];
    } else if (profile.pattern === "sparkle") {
      layers = [
        toneLayer("spark-1", 0, duration * 0.24, "sine", baseFrequency * 2, baseFrequency * 2.02, 0.16, intensity, profile),
        toneLayer("spark-2", step * 1.2, duration * 0.22, "triangle", baseFrequency * 2.5, baseFrequency * 2.48, 0.16, intensity, profile),
        toneLayer("spark-3", step * 2.4, duration * 0.32, "sine", baseFrequency * 3, baseFrequency * 3.05, 0.18, intensity, profile),
        noiseLayer("dust", step * 2, duration * 0.34, 0.05 + intensity * 0.04, "highpass", 4800, 9600, profile),
      ];
    } else if (profile.pattern === "fanfare") {
      layers = [
        toneLayer("root", 0, duration * 0.54, "square", baseFrequency, baseFrequency * 1.01, 0.16, intensity, profile),
        toneLayer("fifth", duration * 0.18, duration * 0.54, "square", baseFrequency * 1.5, baseFrequency * 1.51, 0.16, intensity, profile),
        toneLayer("octave", duration * 0.36, duration * 0.62, "triangle", baseFrequency * 2, baseFrequency * 2.02, 0.2, intensity, profile),
      ];
    } else {
      layers = [
        toneLayer("step-1", 0, duration * 0.28, "triangle", baseFrequency, baseFrequency * 1.01, 0.18, intensity, profile),
        toneLayer("step-2", step, duration * 0.3, "triangle", baseFrequency * 1.25, baseFrequency * 1.26, 0.18, intensity, profile),
        toneLayer("step-3", step * 2, duration * 0.32, "sine", baseFrequency * 1.5, baseFrequency * 1.52, 0.2, intensity, profile),
        toneLayer("shine", step * 3.1, duration * 0.42, "sine", baseFrequency * 2, baseFrequency * 2.06, 0.22, intensity, profile),
        noiseLayer("lift", step * 2.4, duration * 0.34, 0.06 + intensity * 0.04, "highpass", 3600, 7000, profile),
      ];
    }
    return varyLayers(layers, duration, baseFrequency, intensity, profile, rng);
  }

  if (type === "select") {
    if (profile.pattern === "tick") {
      layers = [clickLayer(0, 0.014, 0.16 + intensity * 0.1, 5800)];
    } else if (profile.pattern === "confirm") {
      layers = [
        toneLayer("confirm-1", 0, duration * 0.62, "sine", baseFrequency, baseFrequency, 0.16, intensity, profile),
        toneLayer("confirm-2", duration * 0.34, duration * 0.58, "triangle", baseFrequency * 1.5, baseFrequency * 1.5, 0.18, intensity, profile),
      ];
    } else if (profile.pattern === "soft") {
      layers = [
        toneLayer("soft", 0, duration * 1.3, "sine", baseFrequency * 0.8, baseFrequency * 0.82, 0.16, intensity, profile),
      ];
    } else {
      layers = [
        toneLayer("tap", 0, duration, "triangle", baseFrequency, baseFrequency * 1.08, 0.2, intensity, profile),
        clickLayer(0, 0.015, 0.1 + intensity * 0.07, 4200 * profile.filterBias),
      ];
    }
    return varyLayers(layers, duration, baseFrequency, intensity, profile, rng);
  }

  if (profile.pattern === "fall") {
    layers = [
      toneLayer("fall-1", 0, duration * 0.42, "square", baseFrequency * 1.8, baseFrequency * 0.72, 0.26, intensity, profile),
      toneLayer("fall-2", duration * 0.24, duration * 0.42, "sawtooth", baseFrequency * 1.18, baseFrequency * 0.46, 0.2, intensity, profile),
      noiseLayer("bad-edge", 0, duration * 0.44, 0.07 + intensity * 0.08, "bandpass", 900, 360, profile),
    ];
  } else if (profile.pattern === "double") {
    layers = [
      toneLayer("buzz-1", 0, duration * 0.3, "square", baseFrequency * 1.08, baseFrequency * 0.86, 0.24, intensity, profile),
      toneLayer("rub-1", 0, duration * 0.3, "sawtooth", baseFrequency * 1.16, baseFrequency * 0.78, 0.12, intensity, profile),
      toneLayer("buzz-2", duration * 0.46, duration * 0.32, "square", baseFrequency * 0.9, baseFrequency * 0.58, 0.24, intensity, profile),
      clickLayer(duration * 0.44, 0.016, 0.06 + intensity * 0.08, 1800),
    ];
  } else if (profile.pattern === "glitch") {
    layers = [
      toneLayer("glitch", 0, duration * 0.55, "sawtooth", baseFrequency * 2.1, baseFrequency * 0.4, 0.18, intensity, profile),
      noiseLayer("static", 0, duration * 0.82, 0.12 + intensity * 0.14, "bandpass", 2800, 740, profile),
      clickLayer(duration * 0.16, 0.018, 0.08 + intensity * 0.08, 3400),
      clickLayer(duration * 0.36, 0.014, 0.06 + intensity * 0.07, 5200),
    ];
  } else {
    layers = [
      toneLayer("deny-low", 0, duration * 0.72, "square", baseFrequency * 1.02, baseFrequency * 0.58, 0.28, intensity, profile),
      toneLayer("deny-rub", 0.012, duration * 0.68, "sawtooth", baseFrequency * 1.09, baseFrequency * 0.62, 0.16, intensity, profile),
      noiseLayer("deny-grit", 0, duration * 0.5, 0.08 + intensity * 0.1, "bandpass", 760, 420, profile),
      clickLayer(0, 0.014, 0.06 + intensity * 0.07, 1400),
    ];
  }
  return varyLayers(layers, duration, baseFrequency, intensity, profile, rng);
}

function varyLayers(layers, duration, baseFrequency, intensity, profile, rng) {
  const varied = layers.map((layer) => {
    const gainJitter = lerp(0.84, 1.18, rng());
    if (layer.kind === "tone") {
      const pitchJitter = lerp(0.92, 1.09, rng());
      return {
        ...layer,
        frequencyStart: Math.round(layer.frequencyStart * pitchJitter),
        frequencyEnd: Math.round(layer.frequencyEnd * lerp(0.9, 1.12, rng())),
        gain: round3(layer.gain * gainJitter),
      };
    }
    if (layer.kind === "noise") {
      return {
        ...layer,
        filterStart: Math.round(layer.filterStart * lerp(0.74, 1.32, rng())),
        filterEnd: Math.round(layer.filterEnd * lerp(0.74, 1.32, rng())),
        gain: round3(layer.gain * gainJitter),
      };
    }
    return { ...layer, gain: round3(layer.gain * gainJitter) };
  });

  if (profile.variant === "double") {
    varied.push(toneLayer("ghost", duration * lerp(0.18, 0.42, rng()), duration * 0.24, profile.waveform, baseFrequency * 1.5, baseFrequency * lerp(1.2, 2.1, rng()), 0.09, intensity, profile));
  } else if (profile.variant === "gritty") {
    varied.push(noiseLayer("grit-tail", duration * 0.12, duration * 0.38, 0.08 + intensity * 0.08, "bandpass", 1800, 520, profile));
  } else if (profile.variant === "hollow") {
    for (const layer of varied) {
      if (layer.kind === "tone") {
        layer.waveform = "sine";
        layer.gain = round3(layer.gain * 0.82);
      }
    }
  } else if (profile.variant === "wide") {
    varied.push(toneLayer("upper", duration * 0.04, duration * 0.5, "triangle", baseFrequency * 2, baseFrequency * lerp(1.6, 2.8, rng()), 0.08, intensity, profile));
  } else if (profile.variant === "stepped") {
    varied.push(clickLayer(duration * lerp(0.22, 0.56, rng()), 0.018, 0.06 + intensity * 0.08, 3600 * profile.filterBias));
  }

  return varied.sort((a, b) => a.start - b.start || a.name.localeCompare(b.name));
}

function toneLayer(name, start, duration, waveform, frequencyStart, frequencyEnd, gain, intensity, profile) {
  return {
    kind: "tone",
    name,
    start: round3(start),
    duration: round3(Math.max(0.025, duration)),
    waveform,
    frequencyStart: Math.round(frequencyStart),
    frequencyEnd: Math.round(frequencyEnd),
    gain: round3(gain * lerp(0.62, 1.22, intensity)),
    attack: 0.006,
    release: round3(Math.max(0.018, duration * 0.32)),
    filterFrequency: Math.round(lerp(900, 4200, intensity) * profile.filterBias),
    wobble: profile.pitchWobble,
  };
}

function noiseLayer(name, start, duration, gain, filterType, filterStart, filterEnd, profile) {
  return {
    kind: "noise",
    name,
    color: profile.noiseColor,
    start: round3(start),
    duration: round3(Math.max(0.02, duration)),
    gain: round3(gain),
    attack: 0.004,
    release: round3(Math.max(0.018, duration * 0.45)),
    filterType,
    filterStart: Math.round(filterStart),
    filterEnd: Math.round(filterEnd),
  };
}

function clickLayer(start, duration, gain, filterFrequency) {
  return {
    kind: "click",
    name: "transient",
    start: round3(start),
    duration: round3(duration),
    gain: round3(gain),
    filterFrequency: Math.round(filterFrequency),
  };
}

function playLayer(audioContext, layer, effectStart, activeSources) {
  const startsAt = effectStart + layer.start;
  if (layer.kind === "tone") {
    playToneLayer(audioContext, layer, startsAt, activeSources);
  } else if (layer.kind === "noise") {
    playNoiseLayer(audioContext, layer, startsAt, activeSources);
  } else {
    playClickLayer(audioContext, layer, startsAt, activeSources);
  }
}

function playToneLayer(audioContext, layer, startsAt, activeSources) {
  const oscillator = audioContext.createOscillator();
  const filter = audioContext.createBiquadFilter();
  const gain = audioContext.createGain();
  const endsAt = startsAt + layer.duration;
  oscillator.type = layer.waveform;
  oscillator.frequency.setValueAtTime(Math.max(20, layer.frequencyStart), startsAt);
  oscillator.frequency.exponentialRampToValueAtTime(Math.max(20, layer.frequencyEnd), endsAt);
  if (layer.wobble > 0) {
    oscillator.detune.setValueAtTime(0, startsAt);
    oscillator.detune.linearRampToValueAtTime(layer.wobble * 1200, startsAt + layer.duration * 0.35);
    oscillator.detune.linearRampToValueAtTime(0, endsAt);
  }
  filter.type = "lowpass";
  filter.frequency.setValueAtTime(layer.filterFrequency, startsAt);
  gain.gain.setValueAtTime(0.0001, startsAt);
  gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, layer.gain), startsAt + layer.attack);
  gain.gain.exponentialRampToValueAtTime(0.0001, Math.max(startsAt + layer.attack + 0.01, endsAt - layer.release));
  oscillator.connect(filter).connect(gain).connect(audioContext.destination);
  trackSource(oscillator, activeSources);
  oscillator.start(startsAt);
  oscillator.stop(endsAt + 0.03);
}

function playNoiseLayer(audioContext, layer, startsAt, activeSources) {
  const samples = Math.max(1, Math.floor(audioContext.sampleRate * layer.duration));
  const buffer = audioContext.createBuffer(1, samples, audioContext.sampleRate);
  const data = buffer.getChannelData(0);
  const rng = mulberry32(hashSeed(`${layer.name}:${layer.start}:${layer.duration}:${layer.gain}`));
  for (let i = 0; i < samples; i += 1) {
    const white = rng() * 2 - 1;
    data[i] = layer.color === "crackle" && rng() > 0.72 ? white : white * 0.55;
  }

  const source = audioContext.createBufferSource();
  const filter = audioContext.createBiquadFilter();
  const gain = audioContext.createGain();
  const endsAt = startsAt + layer.duration;
  filter.type = layer.filterType;
  filter.frequency.setValueAtTime(Math.max(20, layer.filterStart), startsAt);
  filter.frequency.exponentialRampToValueAtTime(Math.max(20, layer.filterEnd), endsAt);
  gain.gain.setValueAtTime(0.0001, startsAt);
  gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, layer.gain), startsAt + layer.attack);
  gain.gain.exponentialRampToValueAtTime(0.0001, Math.max(startsAt + layer.attack + 0.01, endsAt - layer.release));
  source.buffer = buffer;
  source.connect(filter).connect(gain).connect(audioContext.destination);
  trackSource(source, activeSources);
  source.start(startsAt);
}

function playClickLayer(audioContext, layer, startsAt, activeSources) {
  const samples = Math.max(1, Math.floor(audioContext.sampleRate * layer.duration));
  const buffer = audioContext.createBuffer(1, samples, audioContext.sampleRate);
  const data = buffer.getChannelData(0);
  const rng = mulberry32(hashSeed(`${layer.name}:${layer.filterFrequency}:${layer.gain}`));
  for (let i = 0; i < samples; i += 1) {
    const t = i / audioContext.sampleRate;
    data[i] = (rng() * 2 - 1) * Math.exp(-120 * t);
  }

  const source = audioContext.createBufferSource();
  const filter = audioContext.createBiquadFilter();
  const gain = audioContext.createGain();
  filter.type = "highpass";
  filter.frequency.value = layer.filterFrequency;
  gain.gain.setValueAtTime(layer.gain, startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + layer.duration);
  source.buffer = buffer;
  source.connect(filter).connect(gain).connect(audioContext.destination);
  trackSource(source, activeSources);
  source.start(startsAt);
}

function trackSource(source, activeSources) {
  activeSources.add(source);
  source.addEventListener("ended", () => {
    activeSources.delete(source);
    source.disconnect();
  }, { once: true });
}

function normalizeType(type) {
  if (typeof type !== "string") {
    return null;
  }
  if (type === "random") {
    return null;
  }
  if (type === "wild") {
    return "wild";
  }
  return SFX_TYPES.includes(type) ? type : null;
}

function typeFromSeed(seed) {
  return SFX_TYPES[hashSeed(seed) % SFX_TYPES.length];
}

function hashSeed(seed) {
  let hash = 2166136261;
  for (let i = 0; i < seed.length; i += 1) {
    hash ^= seed.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function mulberry32(seed) {
  return function next() {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function pick(values, rng) {
  return values[Math.floor(rng() * values.length)];
}

function randomInt(rng, min, max) {
  return Math.floor(rng() * (max - min + 1)) + min;
}

function lerp(min, max, value) {
  return min + (max - min) * value;
}

function round2(value) {
  return Math.round(value * 100) / 100;
}

function round3(value) {
  return Math.round(value * 1000) / 1000;
}

// PuzzleScript-compatible SFX support is adapted from PuzzleScript's sfxr.js.
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
const PS_SFX_SOUND_VOL = 0.25;
const PS_SFX_SAMPLE_RATE = 5512;
const PS_SFX_MIN_SAMPLE_RATE = 22050;
const PS_SQUARE = 0;
const PS_SAWTOOTH = 1;
const PS_SINE = 2;
const PS_NOISE = 3;
const PS_TRIANGLE = 4;
const PS_BREAKER = 5;
const PS_SHAPE_COUNT = 6;

let psSfxRng = null;

export function generatePuzzleScriptSoundEffect(seed) {
  const numericSeed = Math.max(0, Math.trunc(Number(seed) || 0));
  const params = psGenerateFromSeed(numericSeed);
  params.sound_vol = PS_SFX_SOUND_VOL;
  params.sample_rate = PS_SFX_SAMPLE_RATE;
  return {
    seed: String(seed),
    numericSeed,
    type: "puzzlescript",
    params,
  };
}

export function createPuzzleScriptSfxPlayer(audioContext, effect) {
  let source = null;
  let output = null;

  function stop() {
    if (source) {
      try {
        source.stop();
      } catch {
        // Already-ended one-shot sources can be ignored.
      }
      source.disconnect();
      source = null;
    }
    if (output) {
      output.disconnect();
      output = null;
    }
  }

  function start() {
    stop();
    const buffer = psRenderBuffer(audioContext, effect.params);
    source = audioContext.createBufferSource();
    const filter1 = audioContext.createBiquadFilter();
    const filter2 = audioContext.createBiquadFilter();
    const filter3 = audioContext.createBiquadFilter();
    filter1.frequency.value = 1600;
    filter2.frequency.value = 1600;
    filter3.frequency.value = 1600;
    source.buffer = buffer;
    source.connect(filter1);
    filter1.connect(filter2);
    filter2.connect(filter3);
    filter3.connect(audioContext.destination);
    output = filter3;
    source.addEventListener("ended", stop, { once: true });
    source.start(audioContext.currentTime);
  }

  return { start, stop };
}

function psParams() {
  return {
    wave_type: PS_SQUARE,
    p_env_attack: 0.0,
    p_env_sustain: 0.3,
    p_env_punch: 0.0,
    p_env_decay: 0.4,
    p_base_freq: 0.3,
    p_freq_limit: 0.0,
    p_freq_ramp: 0.0,
    p_freq_dramp: 0.0,
    p_vib_strength: 0.0,
    p_vib_speed: 0.0,
    p_arp_mod: 0.0,
    p_arp_speed: 0.0,
    p_duty: 0.0,
    p_duty_ramp: 0.0,
    p_repeat_speed: 0.0,
    p_pha_offset: 0.0,
    p_pha_ramp: 0.0,
    p_lpf_freq: 1.0,
    p_lpf_ramp: 0.0,
    p_lpf_resonance: 0.0,
    p_hpf_freq: 0.0,
    p_hpf_ramp: 0.0,
    sound_vol: 0.5,
    sample_rate: 44100,
  };
}

function psFrnd(range) {
  return (psSfxRng ? psSfxRng.uniform() : Math.random()) * range;
}

function psRnd(max) {
  return Math.floor((psSfxRng ? psSfxRng.uniform() : Math.random()) * (max + 1));
}

function psGenerateFromSeed(seed) {
  psSfxRng = new PsRng((seed / 100) | 0);
  const generatorIndex = seed % 100;
  const generator = PS_GENERATORS[generatorIndex % PS_GENERATORS.length];
  const params = generator();
  params.seed = seed;
  psSfxRng = null;
  return params;
}

function psPickupCoin() {
  const p = psParams();
  p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
  if (p.wave_type === PS_NOISE) {
    p.wave_type = PS_SQUARE;
  }
  p.p_base_freq = 0.4 + psFrnd(0.5);
  p.p_env_attack = 0.0;
  p.p_env_sustain = psFrnd(0.1);
  p.p_env_decay = 0.1 + psFrnd(0.4);
  p.p_env_punch = 0.3 + psFrnd(0.3);
  if (psRnd(1)) {
    p.p_arp_speed = 0.5 + psFrnd(0.2);
    const num = (psFrnd(7) | 1) + 1;
    const den = num + (psFrnd(7) | 1) + 2;
    p.p_arp_mod = num / den;
  }
  return p;
}

function psLaserShoot() {
  const p = psParams();
  p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
  if (p.wave_type === PS_NOISE) {
    p.wave_type = PS_SQUARE;
  }
  p.p_base_freq = 0.5 + psFrnd(0.5);
  p.p_freq_limit = Math.max(0.2, p.p_base_freq - 0.2 - psFrnd(0.6));
  p.p_freq_ramp = -0.15 - psFrnd(0.2);
  if (psRnd(2) === 0) {
    p.p_base_freq = 0.3 + psFrnd(0.6);
    p.p_freq_limit = psFrnd(0.1);
    p.p_freq_ramp = -0.35 - psFrnd(0.3);
  }
  if (psRnd(1)) {
    p.p_duty = psFrnd(0.5);
    p.p_duty_ramp = psFrnd(0.2);
  } else {
    p.p_duty = 0.4 + psFrnd(0.5);
    p.p_duty_ramp = -psFrnd(0.7);
  }
  p.p_env_attack = 0.0;
  p.p_env_sustain = 0.1 + psFrnd(0.2);
  p.p_env_decay = psFrnd(0.4);
  if (psRnd(1)) {
    p.p_env_punch = psFrnd(0.3);
  }
  if (psRnd(2) === 0) {
    p.p_pha_offset = psFrnd(0.2);
    p.p_pha_ramp = -psFrnd(0.2);
  }
  if (psRnd(1)) {
    p.p_hpf_freq = psFrnd(0.3);
  }
  return p;
}

function psExplosion() {
  const p = psParams();
  if (psRnd(1)) {
    p.p_base_freq = 0.1 + psFrnd(0.4);
    p.p_freq_ramp = -0.1 + psFrnd(0.4);
  } else {
    p.p_base_freq = 0.2 + psFrnd(0.7);
    p.p_freq_ramp = -0.2 - psFrnd(0.2);
  }
  p.p_base_freq *= p.p_base_freq;
  if (psRnd(4) === 0) {
    p.p_freq_ramp = 0.0;
  }
  if (psRnd(2) === 0) {
    p.p_repeat_speed = 0.3 + psFrnd(0.5);
  }
  p.p_env_attack = 0.0;
  p.p_env_sustain = 0.1 + psFrnd(0.3);
  p.p_env_decay = psFrnd(0.5);
  if (psRnd(1) === 0) {
    p.p_pha_offset = -0.3 + psFrnd(0.9);
    p.p_pha_ramp = -psFrnd(0.3);
  }
  p.p_env_punch = 0.2 + psFrnd(0.6);
  if (psRnd(1)) {
    p.p_vib_strength = psFrnd(0.7);
    p.p_vib_speed = psFrnd(0.6);
  }
  if (psRnd(2) === 0) {
    p.p_arp_speed = 0.6 + psFrnd(0.3);
    p.p_arp_mod = 0.8 - psFrnd(1.6);
  }
  return p;
}

function psBirdSound() {
  const p = psParams();
  if (psFrnd(10) < 1) {
    p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
    if (p.wave_type === PS_NOISE) {
      p.wave_type = PS_SQUARE;
    }
    p.p_env_attack = 0.4304400932967592 + psFrnd(0.2) - 0.1;
    p.p_env_sustain = 0.15739346034252394 + psFrnd(0.2) - 0.1;
    p.p_env_punch = 0.004488201744871758 + psFrnd(0.2) - 0.1;
    p.p_env_decay = 0.07478075528212291 + psFrnd(0.2) - 0.1;
    p.p_base_freq = 0.9865265720147687 + psFrnd(0.2) - 0.1;
    p.p_freq_limit = psFrnd(0.2) - 0.1;
    p.p_freq_ramp = -0.2995018224359539 + psFrnd(0.2) - 0.1;
    if (psFrnd(1.0) < 0.5) {
      p.p_freq_ramp = 0.1 + psFrnd(0.15);
    }
    p.p_freq_dramp = 0.004598608156964473 + psFrnd(0.1) - 0.05;
    p.p_vib_strength = -0.2202799497929496 + psFrnd(0.2) - 0.1;
    p.p_vib_speed = 0.8084998703158364 + psFrnd(0.2) - 0.1;
    p.p_arp_mod = 0;
    p.p_arp_speed = 0;
    p.p_duty = -0.9031808754347107 + psFrnd(0.2) - 0.1;
    p.p_duty_ramp = -0.8128699999808343 + psFrnd(0.2) - 0.1;
    p.p_repeat_speed = 0.6014860189319991 + psFrnd(0.2) - 0.1;
    p.p_pha_offset = -0.9424902314367765 + psFrnd(0.2) - 0.1;
    p.p_pha_ramp = -0.1055482222272056 + psFrnd(0.2) - 0.1;
    p.p_lpf_freq = 0.9989765717851521 + psFrnd(0.2) - 0.1;
    p.p_lpf_ramp = -0.25051720626043017 + psFrnd(0.2) - 0.1;
    p.p_lpf_resonance = 0.32777871505494693 + psFrnd(0.2) - 0.1;
    p.p_hpf_freq = 0.0023548750981756753 + psFrnd(0.2) - 0.1;
    p.p_hpf_ramp = -0.002375673204842568 + psFrnd(0.2) - 0.1;
    return p;
  }
  if (psFrnd(10) < 1) {
    p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
    if (p.wave_type === PS_NOISE) {
      p.wave_type = PS_SQUARE;
    }
    p.p_env_attack = 0.5277795946672003 + psFrnd(0.2) - 0.1;
    p.p_env_sustain = 0.18243733568468432 + psFrnd(0.2) - 0.1;
    p.p_env_punch = -0.020159754546840117 + psFrnd(0.2) - 0.1;
    p.p_env_decay = 0.1561353422051903 + psFrnd(0.2) - 0.1;
    p.p_base_freq = 0.9028855606533718 + psFrnd(0.2) - 0.1;
    p.p_freq_limit = -0.008842787837148716;
    p.p_freq_ramp = -0.1;
    p.p_freq_dramp = -0.012891241489551925;
    p.p_vib_strength = -0.17923136138403065 + psFrnd(0.2) - 0.1;
    p.p_vib_speed = 0.908263385610142 + psFrnd(0.2) - 0.1;
    p.p_arp_mod = 0.41690153355414894 + psFrnd(0.2) - 0.1;
    p.p_arp_speed = 0.0010766233195860703 + psFrnd(0.2) - 0.1;
    p.p_duty = -0.8735363011184684 + psFrnd(0.2) - 0.1;
    p.p_duty_ramp = -0.7397985366747507 + psFrnd(0.2) - 0.1;
    p.p_repeat_speed = 0.0591789344172107 + psFrnd(0.2) - 0.1;
    p.p_pha_offset = -0.9961184222777699 + psFrnd(0.2) - 0.1;
    p.p_pha_ramp = -0.08234769395850523 + psFrnd(0.2) - 0.1;
    p.p_lpf_freq = 0.9412475115697335 + psFrnd(0.2) - 0.1;
    p.p_lpf_ramp = -0.18261358925834958 + psFrnd(0.2) - 0.1;
    p.p_lpf_resonance = 0.24541438107389477 + psFrnd(0.2) - 0.1;
    p.p_hpf_freq = -0.01831940280978611 + psFrnd(0.2) - 0.1;
    p.p_hpf_ramp = -0.03857383633171346 + psFrnd(0.2) - 0.1;
    return p;
  }
  if (psFrnd(10) < 1) {
    p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
    if (p.wave_type === PS_NOISE) {
      p.wave_type = PS_SQUARE;
    }
    p.p_env_attack = 0.4304400932967592 + psFrnd(0.2) - 0.1;
    p.p_env_sustain = 0.15739346034252394 + psFrnd(0.2) - 0.1;
    p.p_env_punch = 0.004488201744871758 + psFrnd(0.2) - 0.1;
    p.p_env_decay = 0.07478075528212291 + psFrnd(0.2) - 0.1;
    p.p_base_freq = 0.9865265720147687 + psFrnd(0.2) - 0.1;
    p.p_freq_limit = psFrnd(0.2) - 0.1;
    p.p_freq_ramp = -0.2995018224359539 + psFrnd(0.2) - 0.1;
    p.p_freq_dramp = 0.004598608156964473 + psFrnd(0.2) - 0.1;
    p.p_vib_strength = -0.2202799497929496 + psFrnd(0.2) - 0.1;
    p.p_vib_speed = 0.8084998703158364 + psFrnd(0.2) - 0.1;
    p.p_arp_mod = -0.46410459213693644 + psFrnd(0.2) - 0.1;
    p.p_arp_speed = -0.10955361249587248 + psFrnd(0.2) - 0.1;
    p.p_duty = -0.9031808754347107 + psFrnd(0.2) - 0.1;
    p.p_duty_ramp = -0.8128699999808343 + psFrnd(0.2) - 0.1;
    p.p_repeat_speed = 0.7014860189319991 + psFrnd(0.2) - 0.1;
    p.p_pha_offset = -0.9424902314367765 + psFrnd(0.2) - 0.1;
    p.p_pha_ramp = -0.1055482222272056 + psFrnd(0.2) - 0.1;
    p.p_lpf_freq = 0.9989765717851521 + psFrnd(0.2) - 0.1;
    p.p_lpf_ramp = -0.25051720626043017 + psFrnd(0.2) - 0.1;
    p.p_lpf_resonance = 0.32777871505494693 + psFrnd(0.2) - 0.1;
    p.p_hpf_freq = 0.0023548750981756753 + psFrnd(0.2) - 0.1;
    p.p_hpf_ramp = -0.002375673204842568 + psFrnd(0.2) - 0.1;
    return p;
  }
  if (psFrnd(5) > 1) {
    p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
    if (p.wave_type === PS_NOISE) {
      p.wave_type = PS_SQUARE;
    }
    if (psRnd(1)) {
      p.p_arp_mod = 0.2697849293151393 + psFrnd(0.2) - 0.1;
      p.p_arp_speed = -0.3131172257760948 + psFrnd(0.2) - 0.1;
      p.p_base_freq = 0.8090588299313949 + psFrnd(0.2) - 0.1;
      p.p_duty = -0.6210022920964955 + psFrnd(0.2) - 0.1;
      p.p_duty_ramp = -0.00043441813553182567 + psFrnd(0.2) - 0.1;
      p.p_env_attack = 0.004321877246874195 + psFrnd(0.2) - 0.1;
      p.p_env_decay = 0.1 + psFrnd(0.2) - 0.1;
      p.p_env_punch = 0.061737781504416146 + psFrnd(0.2) - 0.1;
      p.p_env_sustain = 0.4987252564798832 + psFrnd(0.2) - 0.1;
      p.p_freq_dramp = 0.31700340314222614 + psFrnd(0.2) - 0.1;
      p.p_freq_limit = psFrnd(0.2) - 0.1;
      p.p_freq_ramp = -0.163380391341416 + psFrnd(0.2) - 0.1;
      p.p_hpf_freq = 0.4709005021145149 + psFrnd(0.2) - 0.1;
      p.p_hpf_ramp = 0.6924667290539194 + psFrnd(0.2) - 0.1;
      p.p_lpf_freq = 0.8351398631384511 + psFrnd(0.2) - 0.1;
      p.p_lpf_ramp = 0.36616557192873134 + psFrnd(0.2) - 0.1;
      p.p_lpf_resonance = -0.08685777111664439 + psFrnd(0.2) - 0.1;
      p.p_pha_offset = -0.036084571580025544 + psFrnd(0.2) - 0.1;
      p.p_pha_ramp = -0.014806445085568108 + psFrnd(0.2) - 0.1;
      p.p_repeat_speed = -0.8094368475518489 + psFrnd(0.2) - 0.1;
      p.p_vib_speed = 0.4496665457171294 + psFrnd(0.2) - 0.1;
      p.p_vib_strength = 0.23413762515532424 + psFrnd(0.2) - 0.1;
    } else {
      p.p_arp_mod = -0.35697118026766184 + psFrnd(0.2) - 0.1;
      p.p_arp_speed = 0.3581140690559588 + psFrnd(0.2) - 0.1;
      p.p_base_freq = 1.3260897696157528 + psFrnd(0.2) - 0.1;
      p.p_duty = -0.30984900436710694 + psFrnd(0.2) - 0.1;
      p.p_duty_ramp = -0.0014374759133411626 + psFrnd(0.2) - 0.1;
      p.p_env_attack = 0.3160357835682254 + psFrnd(0.2) - 0.1;
      p.p_env_decay = 0.1 + psFrnd(0.2) - 0.1;
      p.p_env_punch = 0.24323114016870148 + psFrnd(0.2) - 0.1;
      p.p_env_sustain = 0.4 + psFrnd(0.2) - 0.1;
      p.p_freq_dramp = 0.2866475886237244 + psFrnd(0.2) - 0.1;
      p.p_freq_limit = psFrnd(0.2) - 0.1;
      p.p_freq_ramp = -0.10956352368742976 + psFrnd(0.2) - 0.1;
      p.p_hpf_freq = 0.20772718017889846 + psFrnd(0.2) - 0.1;
      p.p_hpf_ramp = 0.1564090637378835 + psFrnd(0.2) - 0.1;
      p.p_lpf_freq = 0.6021372770637031 + psFrnd(0.2) - 0.1;
      p.p_lpf_ramp = 0.24016227139979027 + psFrnd(0.2) - 0.1;
      p.p_lpf_resonance = -0.08787383821160144 + psFrnd(0.2) - 0.1;
      p.p_pha_offset = -0.381597686151701 + psFrnd(0.2) - 0.1;
      p.p_pha_ramp = -0.0002481687661373495 + psFrnd(0.2) - 0.1;
      p.p_repeat_speed = 0.07812112809425686 + psFrnd(0.2) - 0.1;
      p.p_vib_speed = -0.13648848579133943 + psFrnd(0.2) - 0.1;
      p.p_vib_strength = 0.0018874158972302657 + psFrnd(0.2) - 0.1;
    }
    return p;
  }
  p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
  if (p.wave_type === PS_SAWTOOTH || p.wave_type === PS_NOISE) {
    p.wave_type = PS_SINE;
  }
  p.p_base_freq = 0.85 + psFrnd(0.15);
  p.p_freq_ramp = 0.3 + psFrnd(0.15);
  p.p_env_attack = psFrnd(0.09);
  p.p_env_sustain = 0.2 + psFrnd(0.3);
  p.p_env_decay = psFrnd(0.1);
  p.p_duty = psFrnd(2.0) - 1.0;
  p.p_duty_ramp = Math.pow(psFrnd(2.0) - 1.0, 3.0);
  p.p_repeat_speed = 0.5 + psFrnd(0.1);
  p.p_pha_offset = -0.3 + psFrnd(0.9);
  p.p_pha_ramp = -psFrnd(0.3);
  p.p_arp_speed = 0.4 + psFrnd(0.6);
  p.p_arp_mod = 0.8 + psFrnd(0.1);
  p.p_lpf_resonance = psFrnd(2.0) - 1.0;
  p.p_lpf_freq = 1.0 - Math.pow(psFrnd(1.0), 3.0);
  p.p_lpf_ramp = Math.pow(psFrnd(2.0) - 1.0, 3.0);
  if (p.p_lpf_freq < 0.1 && p.p_lpf_ramp < -0.05) {
    p.p_lpf_ramp = -p.p_lpf_ramp;
  }
  p.p_hpf_freq = Math.pow(psFrnd(1.0), 5.0);
  p.p_hpf_ramp = Math.pow(psFrnd(2.0) - 1.0, 5.0);
  return p;
}

function psPowerUp() {
  const p = psParams();
  p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
  if (p.wave_type === PS_NOISE) {
    p.wave_type = PS_SQUARE;
  }
  if (psRnd(1)) {
    p.p_base_freq = 0.2 + psFrnd(0.3);
    p.p_freq_ramp = 0.1 + psFrnd(0.4);
    p.p_repeat_speed = 0.4 + psFrnd(0.4);
  } else {
    p.p_base_freq = 0.2 + psFrnd(0.3);
    p.p_freq_ramp = 0.05 + psFrnd(0.2);
    if (psRnd(1)) {
      p.p_vib_strength = psFrnd(0.7);
      p.p_vib_speed = psFrnd(0.6);
    }
  }
  p.p_env_attack = 0.0;
  p.p_env_sustain = psFrnd(0.4);
  p.p_env_decay = 0.1 + psFrnd(0.4);
  return p;
}

function psHitHurt() {
  const p = psParams();
  p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
  p.p_base_freq = 0.2 + psFrnd(0.6);
  p.p_freq_ramp = -0.3 - psFrnd(0.4);
  p.p_env_attack = 0.0;
  p.p_env_sustain = psFrnd(0.1);
  p.p_env_decay = 0.1 + psFrnd(0.2);
  if (psRnd(1)) {
    p.p_hpf_freq = psFrnd(0.3);
  }
  return p;
}

function psJump() {
  const p = psParams();
  p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
  if (p.wave_type === PS_NOISE) {
    p.wave_type = PS_SQUARE;
  }
  p.p_duty = psFrnd(0.6);
  p.p_base_freq = 0.3 + psFrnd(0.3);
  p.p_freq_ramp = 0.1 + psFrnd(0.2);
  p.p_env_attack = 0.0;
  p.p_env_sustain = 0.1 + psFrnd(0.3);
  p.p_env_decay = 0.1 + psFrnd(0.2);
  if (psRnd(1)) {
    p.p_hpf_freq = psFrnd(0.3);
  }
  if (psRnd(1)) {
    p.p_lpf_freq = 1.0 - psFrnd(0.6);
  }
  return p;
}

function psBlipSelect() {
  const p = psParams();
  p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
  if (p.wave_type === PS_NOISE) {
    p.wave_type = psRnd(1);
  }
  if (p.wave_type === PS_SQUARE) {
    p.p_duty = psFrnd(0.6);
  }
  p.p_base_freq = 0.2 + psFrnd(0.4);
  p.p_env_attack = 0.0;
  p.p_env_sustain = 0.1 + psFrnd(0.1);
  p.p_env_decay = psFrnd(0.2);
  p.p_hpf_freq = 0.1;
  return p;
}

function psPushSound() {
  const p = psParams();
  p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
  if (p.wave_type === PS_SINE) {
    p.wave_type += 1;
  }
  if (p.wave_type === PS_SQUARE) {
    p.wave_type = PS_NOISE;
  }
  p.p_base_freq = 0.1 + psFrnd(0.4);
  p.p_freq_ramp = 0.05 + psFrnd(0.2);
  p.p_env_attack = 0.01 + psFrnd(0.09);
  p.p_env_sustain = 0.01 + psFrnd(0.09);
  p.p_env_decay = 0.01 + psFrnd(0.09);
  p.p_repeat_speed = 0.3 + psFrnd(0.5);
  p.p_pha_offset = -0.3 + psFrnd(0.9);
  p.p_pha_ramp = -psFrnd(0.3);
  p.p_arp_speed = 0.6 + psFrnd(0.3);
  p.p_arp_mod = 0.8 - psFrnd(1.6);
  return p;
}

function psRandomSound() {
  const p = psParams();
  p.wave_type = Math.floor(psFrnd(PS_SHAPE_COUNT));
  p.p_base_freq = Math.pow(psFrnd(2.0) - 1.0, 2.0);
  if (psRnd(1)) {
    p.p_base_freq = Math.pow(psFrnd(2.0) - 1.0, 3.0) + 0.5;
  }
  p.p_freq_limit = 0.0;
  p.p_freq_ramp = Math.pow(psFrnd(2.0) - 1.0, 5.0);
  if (p.p_base_freq > 0.7 && p.p_freq_ramp > 0.2) {
    p.p_freq_ramp = -p.p_freq_ramp;
  }
  if (p.p_base_freq < 0.2 && p.p_freq_ramp < -0.05) {
    p.p_freq_ramp = -p.p_freq_ramp;
  }
  p.p_freq_dramp = Math.pow(psFrnd(2.0) - 1.0, 3.0);
  p.p_duty = psFrnd(2.0) - 1.0;
  p.p_duty_ramp = Math.pow(psFrnd(2.0) - 1.0, 3.0);
  p.p_vib_strength = Math.pow(psFrnd(2.0) - 1.0, 3.0);
  p.p_vib_speed = psFrnd(2.0) - 1.0;
  p.p_env_attack = Math.pow(psFrnd(2.0) - 1.0, 3.0);
  p.p_env_sustain = Math.pow(psFrnd(2.0) - 1.0, 2.0);
  p.p_env_decay = psFrnd(2.0) - 1.0;
  p.p_env_punch = Math.pow(psFrnd(0.8), 2.0);
  if (p.p_env_attack + p.p_env_sustain + p.p_env_decay < 0.2) {
    p.p_env_sustain += 0.2 + psFrnd(0.3);
    p.p_env_decay += 0.2 + psFrnd(0.3);
  }
  p.p_lpf_resonance = psFrnd(2.0) - 1.0;
  p.p_lpf_freq = 1.0 - Math.pow(psFrnd(1.0), 3.0);
  p.p_lpf_ramp = Math.pow(psFrnd(2.0) - 1.0, 3.0);
  if (p.p_lpf_freq < 0.1 && p.p_lpf_ramp < -0.05) {
    p.p_lpf_ramp = -p.p_lpf_ramp;
  }
  p.p_hpf_freq = Math.pow(psFrnd(1.0), 5.0);
  p.p_hpf_ramp = Math.pow(psFrnd(2.0) - 1.0, 5.0);
  p.p_pha_offset = Math.pow(psFrnd(2.0) - 1.0, 3.0);
  p.p_pha_ramp = Math.pow(psFrnd(2.0) - 1.0, 3.0);
  p.p_repeat_speed = psFrnd(2.0) - 1.0;
  p.p_arp_speed = psFrnd(2.0) - 1.0;
  p.p_arp_mod = psFrnd(2.0) - 1.0;
  return p;
}

const PS_GENERATORS = [
  psPickupCoin,
  psLaserShoot,
  psExplosion,
  psPowerUp,
  psHitHurt,
  psJump,
  psBlipSelect,
  psPushSound,
  psRandomSound,
  psBirdSound,
];

function psRenderBuffer(audioContext, ps) {
  let repTime;
  let fperiod;
  let period;
  let fmaxperiod;
  let fslide;
  let fdslide;
  let squareDuty;
  let squareSlide;
  let arpMod;
  let arpLimit;

  function repeat() {
    repTime = 0;
    fperiod = 100.0 / (ps.p_base_freq * ps.p_base_freq + 0.001);
    period = Math.floor(fperiod);
    fmaxperiod = 100.0 / (ps.p_freq_limit * ps.p_freq_limit + 0.001);
    fslide = 1.0 - Math.pow(ps.p_freq_ramp, 3.0) * 0.01;
    fdslide = -Math.pow(ps.p_freq_dramp, 3.0) * 0.000001;
    squareDuty = 0.5 - ps.p_duty * 0.5;
    squareSlide = -ps.p_duty_ramp * 0.00005;
    arpMod = ps.p_arp_mod >= 0.0
      ? 1.0 - Math.pow(ps.p_arp_mod, 2.0) * 0.9
      : 1.0 + Math.pow(ps.p_arp_mod, 2.0) * 10.0;
    arpLimit = Math.floor(Math.pow(1.0 - ps.p_arp_speed, 2.0) * 20000 + 32);
    if (ps.p_arp_speed === 1.0) {
      arpLimit = 0;
    }
  }

  repeat();

  let fltp = 0.0;
  let fltdp = 0.0;
  let fltw = Math.pow(ps.p_lpf_freq, 3.0) * 0.1;
  const fltwD = 1.0 + ps.p_lpf_ramp * 0.0001;
  let fltdmp = 5.0 / (1.0 + Math.pow(ps.p_lpf_resonance, 2.0) * 20.0) * (0.01 + fltw);
  if (fltdmp > 0.8) {
    fltdmp = 0.8;
  }
  let fltphp = 0.0;
  let flthp = Math.pow(ps.p_hpf_freq, 2.0) * 0.1;
  const flthpD = 1.0 + ps.p_hpf_ramp * 0.0003;
  let vibPhase = 0.0;
  const vibSpeed = Math.pow(ps.p_vib_speed, 2.0) * 0.01;
  const vibAmp = ps.p_vib_strength * 0.5;
  let envStage = 0;
  let envTime = 0;
  const envLength = [
    Math.floor(ps.p_env_attack * ps.p_env_attack * 100000.0),
    Math.floor(ps.p_env_sustain * ps.p_env_sustain * 100000.0),
    Math.floor(ps.p_env_decay * ps.p_env_decay * 100000.0),
  ];
  const envTotalLength = Math.max(1, envLength[0] + envLength[1] + envLength[2]);
  let fphase = Math.pow(ps.p_pha_offset, 2.0) * 1020.0;
  if (ps.p_pha_offset < 0.0) {
    fphase = -fphase;
  }
  let fdphase = Math.pow(ps.p_pha_ramp, 2.0);
  if (ps.p_pha_ramp < 0.0) {
    fdphase = -fdphase;
  }
  let iphase = Math.abs(Math.floor(fphase));
  let ipp = 0;
  const phaserBuffer = new Array(1024).fill(0.0);
  const noiseBuffer = Array.from({ length: 32 }, () => Math.random() * 2.0 - 1.0);
  let repLimit = Math.floor(Math.pow(1.0 - ps.p_repeat_speed, 2.0) * 20000 + 32);
  if (ps.p_repeat_speed === 0.0) {
    repLimit = 0;
  }
  const gain = Math.exp(ps.sound_vol) - 1;
  let sampleSum = 0;
  let numSummed = 0;
  const summands = Math.max(1, Math.floor(44100 / ps.sample_rate));
  const outputRate = ps.sample_rate < PS_SFX_MIN_SAMPLE_RATE ? PS_SFX_MIN_SAMPLE_RATE : ps.sample_rate;
  const expansion = ps.sample_rate < PS_SFX_MIN_SAMPLE_RATE ? Math.ceil(outputRate / ps.sample_rate) : 1;
  const bufferLength = Math.ceil(envTotalLength / summands) * expansion + expansion + 8;
  const audioBuffer = audioContext.createBuffer(1, bufferLength, outputRate);
  const buffer = audioBuffer.getChannelData(0);
  let bufferIndex = 0;
  let phase = 0;

  for (let t = 0; bufferIndex < buffer.length; t += 1) {
    if (repLimit !== 0 && ++repTime >= repLimit) {
      repeat();
    }
    if (arpLimit !== 0 && t >= arpLimit) {
      arpLimit = 0;
      fperiod *= arpMod;
    }
    fslide += fdslide;
    fperiod *= fslide;
    if (fperiod > fmaxperiod) {
      fperiod = fmaxperiod;
      if (ps.p_freq_limit > 0.0) {
        break;
      }
    }
    let rfperiod = fperiod;
    if (vibAmp > 0.0) {
      vibPhase += vibSpeed;
      rfperiod = fperiod * (1.0 + Math.sin(vibPhase) * vibAmp);
    }
    period = Math.max(8, Math.floor(rfperiod));
    squareDuty = Math.max(0.0, Math.min(0.5, squareDuty + squareSlide));
    envTime += 1;
    if (envTime > envLength[envStage]) {
      envTime = 1;
      envStage += 1;
      while (envStage < 3 && envLength[envStage] === 0) {
        envStage += 1;
      }
      if (envStage === 3) {
        break;
      }
    }
    const envVol = envStage === 0
      ? envTime / Math.max(1, envLength[0])
      : envStage === 1
        ? 1.0 + Math.pow(1.0 - envTime / Math.max(1, envLength[1]), 1.0) * 2.0 * ps.p_env_punch
        : 1.0 - envTime / Math.max(1, envLength[2]);
    fphase += fdphase;
    iphase = Math.min(1023, Math.abs(Math.floor(fphase)));
    if (flthpD !== 0.0) {
      flthp = Math.max(0.00001, Math.min(0.1, flthp * flthpD));
    }

    let sample = 0.0;
    for (let si = 0; si < 8; si += 1) {
      let subSample = 0.0;
      phase += 1;
      if (phase >= period) {
        phase %= period;
        if (ps.wave_type === PS_NOISE) {
          for (let i = 0; i < 32; i += 1) {
            noiseBuffer[i] = Math.random() * 2.0 - 1.0;
          }
        }
      }
      const fp = phase / period;
      if (ps.wave_type === PS_SQUARE) {
        subSample = fp < squareDuty ? 0.5 : -0.5;
      } else if (ps.wave_type === PS_SAWTOOTH) {
        subSample = 1.0 - fp * 2;
      } else if (ps.wave_type === PS_SINE) {
        subSample = Math.sin(fp * 2 * Math.PI);
      } else if (ps.wave_type === PS_NOISE) {
        subSample = noiseBuffer[Math.floor(phase * 32 / period)];
      } else if (ps.wave_type === PS_TRIANGLE) {
        subSample = Math.abs(1 - fp * 2) - 1;
      } else if (ps.wave_type === PS_BREAKER) {
        subSample = Math.abs(1 - fp * fp * 2) - 1;
      }
      const previousFltp = fltp;
      fltw = Math.max(0.0, Math.min(0.1, fltw * fltwD));
      if (ps.p_lpf_freq !== 1.0) {
        fltdp += (subSample - fltp) * fltw;
        fltdp -= fltdp * fltdmp;
      } else {
        fltp = subSample;
        fltdp = 0.0;
      }
      fltp += fltdp;
      fltphp += fltp - previousFltp;
      fltphp -= fltphp * flthp;
      subSample = fltphp;
      phaserBuffer[ipp & 1023] = subSample;
      subSample += phaserBuffer[(ipp - iphase + 1024) & 1023];
      ipp = (ipp + 1) & 1023;
      sample += subSample * envVol;
    }
    sampleSum += sample;
    if (++numSummed < summands) {
      continue;
    }
    numSummed = 0;
    const value = (sampleSum / summands / 8) * gain;
    sampleSum = 0;
    for (let i = 0; i < expansion && bufferIndex < buffer.length; i += 1) {
      buffer[bufferIndex] = value;
      bufferIndex += 1;
    }
  }

  return audioBuffer;
}

class PsRng {
  constructor(seed) {
    this.state = new PsRc4(JSON.stringify(seed));
  }

  uniform() {
    const bytes = 7;
    let output = 0;
    for (let i = 0; i < bytes; i += 1) {
      output *= 256;
      output += this.state.next();
    }
    return output / (Math.pow(2, bytes * 8) - 1);
  }
}

class PsRc4 {
  constructor(seed) {
    this.s = Array.from({ length: 256 }, (_, index) => index);
    this.i = 0;
    this.j = 0;
    this.mix(seed);
  }

  mix(seed) {
    const input = psStringBytes(seed);
    let j = 0;
    for (let i = 0; i < this.s.length; i += 1) {
      j = (j + this.s[i] + input[i % input.length]) % 256;
      this.swap(i, j);
    }
  }

  next() {
    this.i = (this.i + 1) % 256;
    this.j = (this.j + this.s[this.i]) % 256;
    this.swap(this.i, this.j);
    return this.s[(this.s[this.i] + this.s[this.j]) % 256];
  }

  swap(i, j) {
    const tmp = this.s[i];
    this.s[i] = this.s[j];
    this.s[j] = tmp;
  }
}

function psStringBytes(value) {
  const output = [];
  for (let i = 0; i < value.length; i += 1) {
    let code = value.charCodeAt(i);
    const bytes = [];
    do {
      bytes.push(code & 0xff);
      code >>= 8;
    } while (code > 0);
    output.push(...bytes.reverse());
  }
  return output.length ? output : [0];
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}
