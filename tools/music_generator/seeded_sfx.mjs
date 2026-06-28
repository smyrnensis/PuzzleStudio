export const SFX_TYPES = ["jump", "pickup", "hit", "drag", "lock", "explosion", "laser", "powerup", "select", "error"];
export const SFX_TYPE_OPTIONS = ["random", ...SFX_TYPES, "wild"];

const TYPE_CONFIG = {
  wild: { duration: 0.5, label: "Wild", baseFrequency: [60, 1400], shape: "freeform" },
  jump: { duration: 0.28, label: "Jump", baseFrequency: [230, 330], shape: "rise" },
  pickup: { duration: 0.42, label: "Pickup", baseFrequency: [620, 880], shape: "spark" },
  hit: { duration: 0.24, label: "Hit", baseFrequency: [90, 160], shape: "impact" },
  drag: { duration: 0.5, label: "Drag", baseFrequency: [72, 138], shape: "scrape" },
  lock: { duration: 0.36, label: "Lock", baseFrequency: [115, 210], shape: "latch" },
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
  drag: ["wood-floor", "stone-floor", "rough-floor", "stuck-start", "short-pull", "soft-floor"],
  lock: ["latch", "deadbolt", "key-turn", "tumblers", "old-lock", "padlock"],
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
  let duration = round3(config.duration * lerp(0.72, 1.45, length) * lerp(0.94, 1.06, rng()));
  if (type === "drag") {
    duration = round3(clamp(duration, 0.36, 0.7));
  }
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

export function createSfxPlayer(audioContext, effect, options = {}) {
  const activeSources = new Set();
  let output = null;

  function disconnectOutput() {
    if (output) {
      output.disconnect();
      output = null;
    }
  }

  function start(startsAt) {
    if (!Number.isFinite(startsAt)) {
      throw new Error("SFX player start requires an explicit AudioContext time");
    }
    stop();
    output = audioContext.createGain();
    output.gain.value = clamp(Number(options.volume ?? 1), 0, 1);
    output.connect(audioContext.destination);
    for (const layer of effect.layers) {
      playLayer(audioContext, layer, startsAt, activeSources, output, () => {
        if (activeSources.size === 0) {
          disconnectOutput();
        }
      });
    }
    if (activeSources.size === 0) {
      disconnectOutput();
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
    disconnectOutput();
  }

  return { start, stop };
}

function buildProfile(rng, type, tonalFamily, intensity) {
  const bright = tonalFamily === "bright";
  const dark = tonalFamily === "dark";
  const waveforms = bright ? ["triangle", "sine", "square"] : dark ? ["sawtooth", "square", "triangle"] : ["sine", "triangle", "square"];
  const variants = type === "drag" ? ["dry", "grainy", "heavy", "stuck", "soft", "rough"] : type === "lock" ? ["dry", "double", "gritty", "stepped", "heavy", "stuck"] : type === "error" ? ["clean", "double", "gritty", "stepped"] : ["clean", "double", "gritty", "hollow", "wide", "stepped"];
  return {
    engine: pick(["arcade", "soft-synth", "bit-crush", "toy-speaker"], rng),
    variant: pick(variants, rng),
    pattern: pick(TYPE_PATTERNS[type], rng),
    waveform: type === "error" ? pick(["square", "sawtooth"], rng) : type === "drag" ? pick(["sine", "triangle"], rng) : type === "lock" ? pick(["square", "triangle"], rng) : pick(waveforms, rng),
    noiseColor: dark || type === "explosion" || type === "error" || type === "lock" || type === "drag" ? "crackle" : "white",
    filterBias: round2(lerp(0.75, 1.35, intensity) * (type === "error" ? 0.78 : type === "drag" ? 0.72 : bright ? 1.18 : dark ? 0.82 : 1)),
    pitchWobble: round2(type === "lock" ? lerp(0, 0.018, rng() * intensity) : type === "drag" ? lerp(0.004, 0.02, rng() * intensity) : lerp(type === "error" ? 0.04 : 0.01, type === "error" ? 0.12 : 0.075, rng() * intensity)),
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

  if (type === "drag") {
    return buildBoxPullLayers(duration, intensity, profile, rng);
  }

  if (type === "lock") {
    if (profile.pattern === "deadbolt") {
      layers = [
        clickLayer(0, 0.009, 0.08 + intensity * 0.08, 7200),
        noiseLayer("bolt-drag", duration * 0.1, duration * 0.42, 0.075 + intensity * 0.09, "bandpass", 3600, 540, profile),
        toneLayer("bolt-slide", duration * 0.16, duration * 0.34, "square", baseFrequency * 1.05, baseFrequency * 0.58, 0.09, intensity, profile),
        clickLayer(duration * 0.62, 0.014, 0.28 + intensity * 0.18, 3000),
        toneLayer("lock-stop", duration * 0.62, duration * 0.18, "square", baseFrequency * 1.15, baseFrequency * 0.5, 0.3, intensity, profile),
        noiseLayer("case-thump", duration * 0.62, duration * 0.16, 0.14 + intensity * 0.16, "lowpass", 1400, 320, profile),
      ];
    } else if (profile.pattern === "key-turn") {
      layers = [
        clickLayer(0, 0.007, 0.08 + intensity * 0.07, 8200),
        clickLayer(duration * 0.18, 0.008, 0.07 + intensity * 0.07, 6500),
        noiseLayer("key-scrape", duration * 0.2, duration * 0.26, 0.05 + intensity * 0.06, "highpass", 5400, 2100, profile),
        toneLayer("key-turn", duration * 0.26, duration * 0.26, "triangle", baseFrequency * 1.45, baseFrequency * 0.68, 0.075, intensity, profile),
        clickLayer(duration * 0.56, 0.013, 0.24 + intensity * 0.16, 3300),
        toneLayer("lock-stop", duration * 0.56, duration * 0.16, "square", baseFrequency * 1.05, baseFrequency * 0.48, 0.28, intensity, profile),
        noiseLayer("case-thump", duration * 0.56, duration * 0.14, 0.12 + intensity * 0.14, "lowpass", 1200, 300, profile),
      ];
    } else if (profile.pattern === "tumblers") {
      layers = [
        clickLayer(0, 0.007, 0.075 + intensity * 0.06, 8400),
        clickLayer(duration * 0.15, 0.007, 0.065 + intensity * 0.06, 7200),
        clickLayer(duration * 0.3, 0.008, 0.07 + intensity * 0.07, 6100),
        noiseLayer("pin-scrape", duration * 0.22, duration * 0.24, 0.045 + intensity * 0.055, "highpass", 5200, 1800, profile),
        clickLayer(duration * 0.6, 0.013, 0.23 + intensity * 0.16, 3100),
        toneLayer("lock-stop", duration * 0.6, duration * 0.17, "square", baseFrequency, baseFrequency * 0.46, 0.29, intensity, profile),
        noiseLayer("case-thump", duration * 0.6, duration * 0.15, 0.13 + intensity * 0.15, "lowpass", 1300, 340, profile),
      ];
    } else if (profile.pattern === "old-lock") {
      layers = [
        clickLayer(0, 0.011, 0.1 + intensity * 0.08, 6500),
        noiseLayer("old-bolt-grind", duration * 0.08, duration * 0.5, 0.1 + intensity * 0.12, "bandpass", 2600, 420, profile),
        clickLayer(duration * 0.28, 0.012, 0.11 + intensity * 0.09, 4700),
        toneLayer("old-case", duration * 0.3, duration * 0.36, "square", baseFrequency * 0.95, baseFrequency * 0.42, 0.14, intensity, profile),
        clickLayer(duration * 0.72, 0.017, 0.3 + intensity * 0.2, 2400),
        toneLayer("lock-stop", duration * 0.72, duration * 0.2, "square", baseFrequency * 0.85, baseFrequency * 0.36, 0.34, intensity, profile),
        noiseLayer("case-thump", duration * 0.72, duration * 0.18, 0.16 + intensity * 0.18, "lowpass", 1100, 260, profile),
      ];
    } else if (profile.pattern === "padlock") {
      layers = [
        clickLayer(0, 0.008, 0.09 + intensity * 0.08, 7600),
        toneLayer("shackle-snap", duration * 0.12, duration * 0.24, "triangle", baseFrequency * 2.3, baseFrequency * 1.1, 0.11, intensity, profile),
        noiseLayer("metal-shell", duration * 0.14, duration * 0.22, 0.08 + intensity * 0.1, "bandpass", 4800, 1100, profile),
        clickLayer(duration * 0.5, 0.013, 0.26 + intensity * 0.17, 3600),
        toneLayer("lock-stop", duration * 0.5, duration * 0.18, "square", baseFrequency * 1.35, baseFrequency * 0.62, 0.27, intensity, profile),
        noiseLayer("case-thump", duration * 0.5, duration * 0.16, 0.13 + intensity * 0.15, "lowpass", 1500, 380, profile),
        toneLayer("metal-ring", duration * 0.56, duration * 0.28, "triangle", baseFrequency * 3.1, baseFrequency * 2.2, 0.06, intensity, profile),
      ];
    } else {
      layers = [
        clickLayer(0, 0.008, 0.08 + intensity * 0.07, 7800),
        noiseLayer("latch-scrape", duration * 0.16, duration * 0.24, 0.06 + intensity * 0.07, "bandpass", 3900, 840, profile),
        clickLayer(duration * 0.42, 0.01, 0.13 + intensity * 0.1, 5200),
        clickLayer(duration * 0.54, 0.014, 0.26 + intensity * 0.17, 3100),
        toneLayer("lock-stop", duration * 0.54, duration * 0.16, "square", baseFrequency * 1.1, baseFrequency * 0.52, 0.3, intensity, profile),
        noiseLayer("case-thump", duration * 0.54, duration * 0.14, 0.13 + intensity * 0.15, "lowpass", 1300, 340, profile),
      ];
    }
    return varyLockLayers(layers, duration, intensity, profile, rng);
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

function buildBoxPullLayers(duration, intensity, profile, rng) {
  const material = boxPullMaterial(profile.pattern);
  const stopAt = duration * lerp(0.72, 0.84, rng());
  const stictionDuration = duration * (profile.variant === "stuck" ? lerp(0.18, 0.26, rng()) : lerp(0.1, 0.17, rng()));
  const rubStart = duration * lerp(0.035, 0.08, rng());
  const rubDuration = Math.max(duration * 0.64, stopAt + duration * 0.08 - rubStart);
  const bodyStart = randomInt(rng, 54, 112);
  const bodyEnd = randomInt(rng, 36, 68);
  const bodyGain = (profile.variant === "heavy" ? 0.22 : 0.16) + intensity * 0.1;
  const rubGain = (profile.variant === "soft" ? 0.12 : 0.17) + intensity * 0.16;
  const stictionGain = (profile.variant === "stuck" ? 0.15 : 0.09) + intensity * 0.08;
  const layers = [
    noiseLayer("stiction-break", 0, stictionDuration, stictionGain, "bandpass", material.stictionStart, material.stictionEnd, profile),
    noiseLayer("floor-rub", rubStart, rubDuration, rubGain, material.filterType, material.rubStart, material.rubEnd, profile),
    toneLayer("crate-body", 0, stopAt + duration * 0.12, "sine", bodyStart, bodyEnd, bodyGain, intensity, profile),
    noiseLayer("settle-dust", stopAt, duration * lerp(0.13, 0.2, rng()), 0.07 + intensity * 0.09, "lowpass", material.settleStart, material.settleEnd, profile),
    toneLayer("settle-thump", stopAt, duration * lerp(0.16, 0.24, rng()), "sine", randomInt(rng, 52, 82), randomInt(rng, 30, 48), 0.14 + intensity * 0.08, intensity, profile),
  ];

  if (material.secondary) {
    layers.push(noiseLayer(
      material.secondary.name,
      duration * material.secondary.start,
      duration * material.secondary.duration,
      material.secondary.gain + intensity * 0.05,
      "bandpass",
      material.secondary.filterStart,
      material.secondary.filterEnd,
      profile,
    ));
  }

  if (profile.pattern === "stuck-start") {
    layers.push(toneLayer("crate-strain", duration * 0.04, duration * 0.3, "triangle", randomInt(rng, 120, 180), randomInt(rng, 70, 105), 0.045 + intensity * 0.035, intensity, profile));
  }

  return varyBoxPullLayers(layers, duration, stopAt, intensity, profile, rng);
}

function boxPullMaterial(pattern) {
  if (pattern === "stone-floor") {
    return {
      filterType: "lowpass",
      stictionStart: 980,
      stictionEnd: 340,
      rubStart: 760,
      rubEnd: 170,
      settleStart: 420,
      settleEnd: 120,
      secondary: { name: "stone-grit", start: 0.18, duration: 0.46, gain: 0.045, filterStart: 1180, filterEnd: 440 },
    };
  }
  if (pattern === "rough-floor") {
    return {
      filterType: "bandpass",
      stictionStart: 1180,
      stictionEnd: 420,
      rubStart: 980,
      rubEnd: 260,
      settleStart: 500,
      settleEnd: 140,
      secondary: { name: "rough-grain", start: 0.16, duration: 0.56, gain: 0.06, filterStart: 1500, filterEnd: 620 },
    };
  }
  if (pattern === "stuck-start") {
    return {
      filterType: "bandpass",
      stictionStart: 1050,
      stictionEnd: 360,
      rubStart: 860,
      rubEnd: 220,
      settleStart: 450,
      settleEnd: 130,
      secondary: { name: "stall-rub", start: 0.08, duration: 0.38, gain: 0.06, filterStart: 1020, filterEnd: 320 },
    };
  }
  if (pattern === "short-pull") {
    return {
      filterType: "bandpass",
      stictionStart: 920,
      stictionEnd: 320,
      rubStart: 820,
      rubEnd: 240,
      settleStart: 440,
      settleEnd: 120,
      secondary: null,
    };
  }
  if (pattern === "soft-floor") {
    return {
      filterType: "lowpass",
      stictionStart: 640,
      stictionEnd: 220,
      rubStart: 540,
      rubEnd: 150,
      settleStart: 320,
      settleEnd: 100,
      secondary: { name: "soft-dust", start: 0.22, duration: 0.42, gain: 0.035, filterStart: 520, filterEnd: 170 },
    };
  }
  return {
    filterType: "bandpass",
    stictionStart: 820,
    stictionEnd: 280,
    rubStart: 700,
    rubEnd: 210,
    settleStart: 380,
    settleEnd: 110,
    secondary: { name: "wood-grain", start: 0.2, duration: 0.5, gain: 0.045, filterStart: 860, filterEnd: 260 },
  };
}

function varyBoxPullLayers(layers, duration, stopAt, intensity, profile, rng) {
  const varied = layers.map((layer) => {
    const gainJitter = profile.variant === "soft" ? lerp(0.72, 0.96, rng()) : lerp(0.9, 1.16, rng());
    if (layer.kind === "tone") {
      return {
        ...layer,
        frequencyStart: Math.round(layer.frequencyStart * lerp(0.95, 1.04, rng())),
        frequencyEnd: Math.round(layer.frequencyEnd * lerp(0.92, 1.06, rng())),
        gain: round3(layer.gain * gainJitter),
      };
    }
    if (layer.kind === "noise") {
      return {
        ...layer,
        filterStart: Math.round(layer.filterStart * lerp(0.9, 1.12, rng())),
        filterEnd: Math.round(layer.filterEnd * lerp(0.88, 1.14, rng())),
        gain: round3(layer.gain * gainJitter),
      };
    }
    return { ...layer, gain: round3(layer.gain * gainJitter) };
  });

  if (profile.variant === "grainy" || profile.variant === "rough") {
    varied.push(noiseLayer("loose-grit", duration * 0.18, duration * 0.46, 0.045 + intensity * 0.055, "bandpass", randomInt(rng, 980, 1500), randomInt(rng, 360, 680), profile));
  } else if (profile.variant === "heavy") {
    varied.push(toneLayer("crate-weight", 0, stopAt + duration * 0.1, "sine", randomInt(rng, 38, 62), randomInt(rng, 28, 42), 0.1 + intensity * 0.08, intensity, profile));
  } else if (profile.variant === "stuck") {
    varied.push(noiseLayer("stiction-hold", duration * 0.08, duration * 0.24, 0.055 + intensity * 0.06, "bandpass", randomInt(rng, 720, 1100), randomInt(rng, 220, 420), profile));
  }

  return varied.sort((a, b) => a.start - b.start || a.name.localeCompare(b.name));
}

function varyLockLayers(layers, duration, intensity, profile, rng) {
  const varied = layers.map((layer) => {
    const gainJitter = lerp(0.96, 1.22, rng());
    if (layer.kind === "tone") {
      return {
        ...layer,
        frequencyStart: Math.round(layer.frequencyStart * lerp(0.94, 1.05, rng())),
        frequencyEnd: Math.round(layer.frequencyEnd * lerp(0.9, 1.08, rng())),
        gain: round3(layer.gain * gainJitter),
      };
    }
    if (layer.kind === "noise") {
      return {
        ...layer,
        filterStart: Math.round(layer.filterStart * lerp(0.88, 1.16, rng())),
        filterEnd: Math.round(layer.filterEnd * lerp(0.84, 1.18, rng())),
        gain: round3(layer.gain * gainJitter),
      };
    }
    return { ...layer, gain: round3(layer.gain * gainJitter) };
  });

  const stopStart = varied.find((layer) => layer.name === "lock-stop")?.start ?? duration * 0.56;
  if (profile.variant === "double") {
    varied.push(clickLayer(stopStart + duration * lerp(0.05, 0.12, rng()), 0.01, 0.08 + intensity * 0.09, randomInt(rng, 2600, 5200)));
  } else if (profile.variant === "gritty") {
    varied.push(noiseLayer("lock-grit", duration * 0.18, duration * 0.28, 0.06 + intensity * 0.08, "bandpass", randomInt(rng, 2600, 5200), randomInt(rng, 520, 1200), profile));
  } else if (profile.variant === "stepped") {
    varied.push(clickLayer(duration * lerp(0.26, 0.46, rng()), 0.009, 0.08 + intensity * 0.08, randomInt(rng, 4200, 7200)));
  } else if (profile.variant === "heavy") {
    varied.push(toneLayer("lock-mass", stopStart, duration * 0.24, "triangle", randomInt(rng, 95, 145), randomInt(rng, 48, 82), 0.16 + intensity * 0.08, intensity, profile));
    varied.push(noiseLayer("lock-wood-hit", stopStart, duration * 0.2, 0.1 + intensity * 0.12, "lowpass", randomInt(rng, 700, 1200), randomInt(rng, 180, 360), profile));
  } else if (profile.variant === "stuck") {
    varied.push(noiseLayer("stuck-scrape", duration * 0.08, duration * 0.42, 0.07 + intensity * 0.09, "bandpass", randomInt(rng, 1800, 3800), randomInt(rng, 360, 900), profile));
    varied.push(clickLayer(duration * lerp(0.34, 0.5, rng()), 0.012, 0.08 + intensity * 0.08, randomInt(rng, 3200, 6000)));
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

function playLayer(audioContext, layer, effectStart, activeSources, destination, onIdle) {
  const startsAt = effectStart + layer.start;
  if (layer.kind === "tone") {
    playToneLayer(audioContext, layer, startsAt, activeSources, destination, onIdle);
  } else if (layer.kind === "noise") {
    playNoiseLayer(audioContext, layer, startsAt, activeSources, destination, onIdle);
  } else {
    playClickLayer(audioContext, layer, startsAt, activeSources, destination, onIdle);
  }
}

function playToneLayer(audioContext, layer, startsAt, activeSources, destination, onIdle) {
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
  oscillator.connect(filter).connect(gain).connect(destination);
  trackSource(oscillator, activeSources, onIdle);
  oscillator.start(startsAt);
  oscillator.stop(endsAt + 0.03);
}

function playNoiseLayer(audioContext, layer, startsAt, activeSources, destination, onIdle) {
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
  source.connect(filter).connect(gain).connect(destination);
  trackSource(source, activeSources, onIdle);
  source.start(startsAt);
}

function playClickLayer(audioContext, layer, startsAt, activeSources, destination, onIdle) {
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
  source.connect(filter).connect(gain).connect(destination);
  trackSource(source, activeSources, onIdle);
  source.start(startsAt);
}

function trackSource(source, activeSources, onIdle = null) {
  activeSources.add(source);
  source.addEventListener("ended", () => {
    activeSources.delete(source);
    source.disconnect();
    if (activeSources.size === 0) {
      onIdle?.();
    }
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
  if (SFX_TYPES.includes(type)) {
    return type;
  }
  throw new Error(`unsupported SFX type: ${type}`);
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

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}
