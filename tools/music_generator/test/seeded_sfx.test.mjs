import assert from "node:assert/strict";
import {
  createPuzzleScriptSfxPlayer,
  createSfxPlayer,
  generatePuzzleScriptSoundEffect,
  generateSoundEffect,
  randomSfxPreset,
  SFX_TYPE_OPTIONS,
  SFX_TYPES,
} from "../seeded_sfx.mjs";

const first = generateSoundEffect("123456", { type: "pickup", mood: 0.62, intensity: 0.7, length: 0.5 });
const second = generateSoundEffect("123456", { type: "pickup", mood: 0.62, intensity: 0.7, length: 0.5 });
const different = generateSoundEffect("654321", { type: "pickup", mood: 0.62, intensity: 0.7, length: 0.5 });
const defaulted = generateSoundEffect("222222", { type: "pickup" });
const defaultedAgain = generateSoundEffect("222222", { type: "pickup" });
const reseeded = generateSoundEffect("333333", { type: "pickup" });

assert.deepEqual(first, second, "same seed and controls should produce the same effect structure");
assert.notDeepEqual(first, different, "different seeds should vary synthesis choices within the specified type");
assert.equal(first.type, "pickup");
assert.equal(first.label, "Pickup");
assert.ok(first.duration > 0.1 && first.duration < 2, "effect duration should stay suitable for game SFX");
assert.ok(first.layers.length >= 2, "effect should contain multiple synthesis layers");
assert.deepEqual(defaulted, defaultedAgain, "seed alone should deterministically fill hidden synthesis choices");
assert.notDeepEqual(defaulted, reseeded, "re-taking the seed should generate another variation within the specified type");
assert.ok(defaulted.mood >= 0 && defaulted.mood <= 1);
assert.ok(defaulted.intensity >= 0 && defaulted.intensity <= 1);
assert.ok(defaulted.length >= 0 && defaulted.length <= 1);

for (const layer of first.layers) {
  assert.ok(["tone", "noise", "click"].includes(layer.kind), "layers should use known synthesis kinds");
  assert.ok(layer.start >= 0, "layer start should be non-negative");
  assert.ok(layer.duration > 0, "layer duration should be positive");
  assert.ok(layer.start < first.duration, "layer should start inside the effect");
  assert.ok(layer.gain > 0, "layer should have audible gain");
}

const dark = generateSoundEffect("123456", { type: "laser", mood: 0.05, intensity: 0.62, length: 0.5 });
const bright = generateSoundEffect("123456", { type: "laser", mood: 0.95, intensity: 0.62, length: 0.5 });
assert.equal(dark.tonalFamily, "dark");
assert.equal(bright.tonalFamily, "bright");
assert.notDeepEqual(dark.layers, bright.layers, "mood should shape pitch and tone layers");
assert.equal(dark.profile.engine, bright.profile.engine, "mood should not change seeded engine choice for the same seed");

const soft = generateSoundEffect("123456", { type: "hit", mood: 0.62, intensity: 0.1, length: 0.5 });
const hard = generateSoundEffect("123456", { type: "hit", mood: 0.62, intensity: 0.95, length: 0.5 });
assert.ok(totalGain(hard) > totalGain(soft), "intensity should raise the total layer gain");

const short = generateSoundEffect("123456", { type: "explosion", mood: 0.62, intensity: 0.7, length: 0 });
const long = generateSoundEffect("123456", { type: "explosion", mood: 0.62, intensity: 0.7, length: 1 });
assert.ok(long.duration > short.duration, "length should scale effect duration");

const generatedByType = SFX_TYPES.map((type) => generateSoundEffect("123456", { type, mood: 0.62, intensity: 0.7, length: 0.5 }));
assert.equal(generatedByType.length, 12, "all expected game SFX types should be exposed");
assert.ok(
  generatedByType.find((effect) => effect.type === "explosion").layers.some((layer) => layer.kind === "noise"),
  "explosions should include a noise layer",
);
assert.ok(
  generatedByType.find((effect) => effect.type === "pickup").layers.filter((layer) => layer.kind === "tone").length >= 2,
  "pickups should include a short tonal phrase",
);
assert.ok(
  generatedByType.find((effect) => effect.type === "hit").layers.some((layer) => layer.kind === "tone" || layer.kind === "noise"),
  "hits should include an impact layer",
);
assert.ok(
  generatedByType.find((effect) => effect.type === "step").layers.some((layer) => /foot|sole|step|grass|stone|wood/.test(layer.name)),
  "steps should include a contact or surface layer",
);
assert.ok(
  generatedByType.find((effect) => effect.type === "drag").layers.some((layer) => layer.name === "floor-rub")
    && generatedByType.find((effect) => effect.type === "drag").layers.some((layer) => layer.name === "release-dust"),
  "drags should include sustained floor rub plus a fading release",
);
assert.ok(
  generatedByType.find((effect) => effect.type === "water").layers.some((layer) => /water|splash|plop|ripple|bubble|pour|drip/.test(layer.name)),
  "water effects should include liquid contact layers",
);
assert.ok(
  generatedByType.find((effect) => effect.type === "lock").layers.filter((layer) => layer.kind === "click").length >= 2,
  "locks should include multiple mechanical transients",
);

const pickupVariants = Array.from({ length: 24 }, (_, index) => generateSoundEffect(`${100000 + index}`, { type: "pickup" }));
const hitVariants = Array.from({ length: 24 }, (_, index) => generateSoundEffect(`${100000 + index}`, { type: "hit" }));
const stepVariants = Array.from({ length: 32 }, (_, index) => generateSoundEffect(`${100000 + index}`, { type: "step" }));
const dragVariants = Array.from({ length: 32 }, (_, index) => generateSoundEffect(`${100000 + index}`, { type: "drag" }));
const waterVariants = Array.from({ length: 32 }, (_, index) => generateSoundEffect(`${100000 + index}`, { type: "water" }));
const lockVariants = Array.from({ length: 32 }, (_, index) => generateSoundEffect(`${100000 + index}`, { type: "lock" }));
const selectVariants = Array.from({ length: 32 }, (_, index) => generateSoundEffect(`${100000 + index}`, { type: "select" }));
const errorVariants = Array.from({ length: 32 }, (_, index) => generateSoundEffect(`${100000 + index}`, { type: "error" }));
assert.ok(new Set(pickupVariants.map((effect) => effect.profile.variant)).size >= 4, "same type should expose multiple seeded variants");
assert.ok(new Set(pickupVariants.map((effect) => effect.profile.pattern)).size >= 3, "same type should expose multiple sound patterns");
assert.ok(new Set(pickupVariants.map((effect) => JSON.stringify(effect.layers))).size >= 18, "same type should produce audibly distinct layer structures");
assert.ok(new Set(hitVariants.map((effect) => effect.profile.pattern)).size >= 3, "impact type should expose multiple sound patterns");
assert.ok(
  hitVariants.some((effect) => effect.layers.some((layer) => layer.name === "slice"))
    && hitVariants.some((effect) => effect.layers.some((layer) => layer.name === "clang"))
    && hitVariants.some((effect) => effect.layers.some((layer) => layer.name === "crunch")),
  "hit variants should include slash, metal, and crunch structures",
);
assert.ok(hitVariants.some((effect) => effect.layers.some((layer) => layer.kind === "click")), "some hit variants should include transient layers");
assert.ok(new Set(stepVariants.map((effect) => effect.profile.pattern)).size >= 5, "step type should expose multiple contact surface patterns");
assert.ok(new Set(stepVariants.map((effect) => effect.profile.variant)).size >= 5, "step type should expose multiple weight and follow-through variants");
assert.ok(new Set(stepVariants.map(layerParameterSignature)).size >= 26, "step variants should produce diverse contact timing and surface structures");
assert.ok(
  stepVariants.every(hasStepContactShape),
  "step variants should stay short and read as contact movement rather than UI, pickup, hit, or drag sounds",
);
assert.ok(new Set(dragVariants.map((effect) => effect.profile.pattern)).size >= 4, "drag type should expose multiple box-pull material patterns");
assert.ok(new Set(dragVariants.map((effect) => effect.profile.variant)).size >= 5, "drag type should expose multiple weight and friction variants");
assert.ok(new Set(dragVariants.map(layerParameterSignature)).size >= 28, "drag variants should produce diverse material, timing, and filter structures");
assert.ok(
  dragVariants.every(hasSustainedLowFriction),
  "every drag variant should be led by sustained low or low-mid friction, not a one-shot impact",
);
assert.ok(
  dragVariants.every(hasLowMassLayer),
  "every drag variant should carry a low crate body or weight layer under the rub",
);
assert.ok(
  dragVariants.every(hasSoftReleaseTail),
  "every drag variant should tail off through low dust instead of stopping with an impact",
);
assert.ok(
  dragVariants.every(avoidsExplosiveDragFinish),
  "drag variants should not add a loud late thump that reads as hit or explosion",
);
assert.ok(
  dragVariants.every((effect) => effect.layers.every((layer) => layer.kind !== "noise" || layer.color !== "crackle")),
  "drag variants should use smooth friction noise instead of crackle that reads as debris or explosion",
);
assert.ok(
  dragVariants.every(avoidsBrightClickLead),
  "drag variants should avoid click transients and highpass noise that read as hit, select, or lock sounds",
);
assert.ok(dragVariants.every((effect) => effect.duration >= 0.35 && effect.duration <= 0.7), "drag variants should stay in the one-cell pull range");
assert.ok(
  dragVariants.every((effect) => effect.layers.every((layer) => !["ghost", "upper"].includes(layer.name))),
  "drag variants should avoid melodic generic variation layers",
);
assert.ok(new Set(waterVariants.map((effect) => effect.profile.pattern)).size >= 5, "water type should expose multiple liquid contact patterns");
assert.ok(new Set(waterVariants.map((effect) => effect.profile.variant)).size >= 5, "water type should expose multiple liquid scale and motion variants");
assert.ok(new Set(waterVariants.map(layerParameterSignature)).size >= 28, "water variants should produce diverse liquid layer structures");
assert.ok(
  waterVariants.every(hasLiquidContactShape),
  "water variants should read as liquid contact or flow rather than impact, lock, UI, or drag sounds",
);
assert.ok(new Set(lockVariants.map((effect) => effect.profile.pattern)).size >= 5, "lock type should expose multiple mechanical patterns");
assert.ok(new Set(lockVariants.map((effect) => effect.profile.variant)).size >= 5, "lock type should expose multiple weight and rattle variants");
assert.ok(new Set(lockVariants.map((effect) => effect.layers.map((layer) => `${layer.kind}:${layer.name}`).join("|"))).size >= 18, "lock variants should produce diverse layer structures");
assert.ok(
  lockVariants.every((effect) =>
    effect.layers.filter((layer) => layer.kind === "click").length >= 2
      && effect.layers.some((layer) => layer.kind === "tone" || layer.kind === "noise"),
  ),
  "every lock variant should include latch clicks plus a body or scrape layer",
);
assert.ok(
  lockVariants.every((effect) => effect.layers.some((layer) => layer.name === "case-thump")),
  "every lock variant should include a low case-thump so it does not collapse into light clicks",
);
assert.ok(lockVariants.every((effect) => effect.duration < 0.7), "lock variants should stay compact enough for game SFX");
assert.ok(
  lockVariants.every((effect) => {
    const stop = effect.layers.find((layer) => layer.name === "lock-stop");
    return stop && stop.start >= effect.duration * 0.45 && stop.start <= effect.duration * 0.78;
  }),
  "every lock variant should resolve into a final lock-stop near the main transient",
);
assert.ok(
  lockVariants.every((effect) => effect.layers.every((layer) => !["ghost", "upper"].includes(layer.name))),
  "lock variants should avoid melodic generic variation layers",
);
assert.ok(new Set(selectVariants.map((effect) => effect.profile.pattern)).size >= 4, "select type should expose several UI acknowledgement patterns");
assert.ok(new Set(selectVariants.map(layerParameterSignature)).size >= 24, "select variants should remain seeded without collapsing to one click");
assert.ok(
  selectVariants.every(hasCompactUiSelectShape),
  "select variants should be short, light UI acknowledgements instead of impact, lock, pickup, or error sounds",
);
assert.ok(
  selectVariants.every((effect) => effect.layers.every((layer) => !["ghost", "upper", "grit-tail"].includes(layer.name))),
  "select variants should not inherit generic melodic or gritty variation tails",
);
assert.ok(new Set(errorVariants.map((effect) => effect.profile.pattern)).size >= 3, "error type should expose multiple failed-action patterns");
assert.ok(
  errorVariants.every((effect) => !["hollow", "wide"].includes(effect.profile.variant)),
  "error variants should avoid soft success-like variants",
);
assert.ok(
  errorVariants.every((effect) =>
    effect.layers.some((layer) => layer.kind === "noise" || layer.kind === "click")
      && effect.layers.some((layer) => layer.waveform === "square" || layer.waveform === "sawtooth"),
  ),
  "every error variant should include harsh tone plus transient/noise cues",
);

const wildFirst = generateSoundEffect("123456", { type: "wild" });
const wildSecond = generateSoundEffect("123456", { type: "wild" });
const wildDifferent = generateSoundEffect("654321", { type: "wild" });
assert.deepEqual(wildFirst, wildSecond, "Wild type should still be deterministic for the same seed");
assert.equal(wildFirst.type, "wild", "Wild should remain category-less instead of resolving to a concrete effect type");
assert.equal(wildFirst.label, "Wild");
assert.notDeepEqual(wildFirst, wildDifferent, "Wild should still use the seed to create category-less variations");
const wildVariants = Array.from({ length: 40 }, (_, index) => generateSoundEffect(`${100000 + index}`, { type: "wild" }));
assert.ok(new Set(wildVariants.map((effect) => effect.profile.pattern)).size >= 5, "Wild should expose many category-less patterns");
assert.ok(new Set(wildVariants.map((effect) => effect.layers.map((layer) => layer.kind).join(","))).size >= 8, "Wild should produce structurally loose layer combinations");

const genericSfxTypeOptions = SFX_TYPE_OPTIONS.filter((type) => type !== "puzzlescript");
const timingVariants = genericSfxTypeOptions.flatMap((type) =>
  Array.from({ length: 64 }, (_, index) => generateSoundEffect(`${200000 + index}`, { type })),
);
assert.ok(
  timingVariants.every((effect) => firstLayerStart(effect) === 0),
  "generated SFX should not encode leading silence before the first audible layer",
);

const randomPreset = randomSfxPreset("preset-check");
assert.ok(randomPreset.seed, "random SFX presets should include a seed");
assert.ok(SFX_TYPE_OPTIONS.includes(randomPreset.type), "random SFX presets should choose a known type option");
assert.deepEqual(Object.keys(randomPreset).sort(), ["seed", "type"], "random presets should expose only author-facing controls");
assert.equal(generateSoundEffect(randomSfxPreset("target-check", "pickup").seed, { type: "pickup" }).type, "pickup", "targeted randomize should pair seed with an explicit type override");
assert.equal(generateSoundEffect(randomSfxPreset("target-wild", "wild").seed, { type: "wild" }).type, "wild", "targeted Wild randomize should pair seed with an explicit type override");
const targetedPuzzleScript = randomSfxPreset("target-puzzlescript", "puzzlescript");
assert.equal(targetedPuzzleScript.type, "puzzlescript", "targeted PuzzleScript randomize should preserve the dedicated type");
assert.equal(generatePuzzleScriptSoundEffect(targetedPuzzleScript.seed).type, "puzzlescript");
const targetedRandom = randomSfxPreset("target-random", "random");
assert.ok(/^\d+$/.test(targetedRandom.seed), "targeted Random should return a plain numeric seed");
assert.ok(SFX_TYPES.includes(generateSoundEffect(targetedRandom.seed).type), "targeted Random should resolve to a concrete effect type");
assert.equal(generateSoundEffect("123456", { type: "laser" }).type, "laser", "type override should select laser without encoding it in the seed");
assert.equal(generateSoundEffect("123456", { type: "wild" }).type, "wild", "type override should select Wild without encoding it in the seed");
assert.ok(SFX_TYPES.includes(generateSoundEffect("manual-seed-without-prefix").type), "plain seeds should still map deterministically to a concrete SFX type");
assert.throws(
  () => generateSoundEffect("123456", { type: "puzzlescript" }),
  /unsupported SFX type: puzzlescript/,
  "unsupported explicit SFX types should fail visibly instead of falling back to another type",
);
const puzzleScriptEffect = generatePuzzleScriptSoundEffect("17551700");
assert.equal(puzzleScriptEffect.type, "puzzlescript", "PuzzleScript SFX should use its explicit generator API");
assert.equal(puzzleScriptEffect.numericSeed, 17551700);
assert.equal(SFX_TYPE_OPTIONS[0], "random", "Random should be the first type option");
assert.ok(SFX_TYPE_OPTIONS.includes("puzzlescript"), "PuzzleScript should be an author-facing SFX type option");
assert.ok(SFX_TYPE_OPTIONS.indexOf("puzzlescript") > SFX_TYPE_OPTIONS.indexOf("wild"), "PuzzleScript should remain visually separate from generated SFX types");

function totalGain(effect) {
  return effect.layers.reduce((sum, layer) => sum + layer.gain, 0);
}

function firstLayerStart(effect) {
  return Math.min(...effect.layers.map((layer) => layer.start));
}

function layerParameterSignature(effect) {
  return JSON.stringify(effect.layers.map((layer) => ({
    kind: layer.kind,
    name: layer.name,
    start: layer.start,
    duration: layer.duration,
    gain: layer.gain,
    filterType: layer.filterType,
    filterStart: layer.filterStart,
    filterEnd: layer.filterEnd,
    frequencyStart: layer.frequencyStart,
    frequencyEnd: layer.frequencyEnd,
  })));
}

function hasSustainedLowFriction(effect) {
  const layers = effect.layers
    .filter((layer) =>
      layer.kind === "noise"
        && /rub|grain/.test(layer.name)
        && layer.filterType !== "highpass"
        && Math.max(layer.filterStart, layer.filterEnd) <= 2200,
    )
    .sort((a, b) => a.start - b.start);
  if (layers.length === 0 || layers[0].start > effect.duration * 0.12) {
    return false;
  }
  const lastEnd = Math.max(...layers.map((layer) => layer.start + layer.duration));
  return lastEnd >= effect.duration * 0.75 && mergedDuration(layers) >= effect.duration * 0.62;
}

function hasLowMassLayer(effect) {
  return effect.layers.some((layer) =>
    layer.kind === "tone"
      && ["crate-body", "crate-weight", "crate-strain"].includes(layer.name)
      && layer.duration >= effect.duration * 0.45
      && layer.frequencyStart <= 130
      && layer.frequencyEnd <= 95,
  );
}

function hasSoftReleaseTail(effect) {
  const releaseDust = effect.layers.find((layer) => layer.name === "release-dust");
  return Boolean(
    releaseDust
      && releaseDust.kind === "noise"
      && releaseDust.start >= effect.duration * 0.62
      && releaseDust.start <= effect.duration * 0.86
      && releaseDust.start + releaseDust.duration >= effect.duration * 0.88
      && releaseDust.filterType === "lowpass"
      && releaseDust.filterStart <= 500
      && releaseDust.filterEnd <= 130
      && releaseDust.gain <= 0.11,
  );
}

function avoidsExplosiveDragFinish(effect) {
  return effect.layers.every((layer) => {
    if (["settle-thump", "case-thump", "low-hit", "thud", "blast-tone"].includes(layer.name)) {
      return false;
    }
    if (layer.start < effect.duration * 0.6) {
      return true;
    }
    if (layer.kind === "tone" && layer.frequencyStart <= 95 && layer.gain >= 0.12) {
      return false;
    }
    return layer.gain <= 0.13;
  });
}

function avoidsBrightClickLead(effect) {
  return effect.layers.every((layer) => {
    if (layer.kind === "noise") {
      return layer.filterType !== "highpass";
    }
    return layer.kind !== "click";
  });
}

function hasStepContactShape(effect) {
  if (effect.duration < 0.08 || effect.duration > 0.32 || effect.layers.length > 5) {
    return false;
  }
  if (!effect.layers.some((layer) => /foot|sole|step|grass|stone|wood/.test(layer.name))) {
    return false;
  }
  return effect.layers.every((layer) => {
    const endsAt = layer.start + layer.duration;
    if (endsAt > effect.duration + 0.000001 || /^ui-/.test(layer.name) || /coin|ring|lock|water|splash|bubble|pour/.test(layer.name)) {
      return false;
    }
    if (layer.kind === "click") {
      return layer.duration <= 0.014
        && layer.gain <= 0.2
        && layer.filterFrequency >= 800
        && layer.filterFrequency <= 4600;
    }
    if (layer.kind === "noise") {
      return layer.filterType !== "highpass"
        && layer.gain <= 0.2
        && Math.max(layer.filterStart, layer.filterEnd) <= 2600;
    }
    return layer.kind === "tone"
      && layer.gain <= 0.22
      && layer.frequencyStart <= 420
      && layer.frequencyEnd <= 320
      && layer.wobble <= 0.018;
  });
}

function hasLiquidContactShape(effect) {
  if (effect.duration < 0.16 || effect.duration > 0.9) {
    return false;
  }
  if (!effect.layers.some((layer) => layer.kind === "noise" && /water|splash|plop|ripple|bubble|pour|drip/.test(layer.name))) {
    return false;
  }
  if (effect.layers.some((layer) => /^ui-|lock|floor-rub|crate|foot|sole|step|thud|clang|slice/.test(layer.name))) {
    return false;
  }
  return effect.layers.every((layer) => {
    const endsAt = layer.start + layer.duration;
    if (endsAt > effect.duration + 0.08 || layer.kind === "click") {
      return false;
    }
    if (layer.kind === "noise") {
      return layer.color === "white"
        && layer.filterType !== "highpass"
        && layer.gain <= 0.34
        && Math.max(layer.filterStart, layer.filterEnd) <= 5200;
    }
    return layer.kind === "tone"
      && layer.gain <= 0.26
      && layer.frequencyStart <= 800
      && layer.frequencyEnd <= 900;
  });
}

function hasCompactUiSelectShape(effect) {
  if (effect.duration < 0.07 || effect.duration > 0.19 || effect.layers.length > 3) {
    return false;
  }
  return effect.layers.every((layer) => {
    const endsAt = layer.start + layer.duration;
    if (endsAt > effect.duration + 0.000001 || layer.kind === "noise") {
      return false;
    }
    if (layer.kind === "click") {
      return layer.duration <= 0.012
        && layer.gain <= 0.16
        && layer.filterFrequency >= 3200
        && layer.filterFrequency <= 8200;
    }
    return layer.kind === "tone"
      && /^ui-/.test(layer.name)
      && layer.duration <= effect.duration * 0.72
      && layer.gain <= 0.18
      && layer.frequencyStart >= 500
      && layer.frequencyStart <= 2600
      && layer.frequencyEnd >= 500
      && layer.frequencyEnd <= 2600
      && layer.wobble <= 0.018;
  });
}

function mergedDuration(layers) {
  let total = 0;
  let currentStart = null;
  let currentEnd = null;
  for (const layer of layers) {
    const start = layer.start;
    const end = layer.start + layer.duration;
    if (currentStart === null) {
      currentStart = start;
      currentEnd = end;
    } else if (start <= currentEnd) {
      currentEnd = Math.max(currentEnd, end);
    } else {
      total += currentEnd - currentStart;
      currentStart = start;
      currentEnd = end;
    }
  }
  return currentStart === null ? total : total + currentEnd - currentStart;
}

class FakeAudioContext {
  constructor(options = {}) {
    this.currentTime = 7;
    this.sampleRate = 48000;
    this.sourceStartTimes = [];
    this.bufferCreateCount = 0;
    this.bufferRenderSeconds = Number(options.bufferRenderSeconds ?? 0);
    this.gainNodes = [];
    this.destination = new FakeAudioNode(this.sourceStartTimes, false);
  }

  createGain() {
    const node = new FakeAudioNode(this.sourceStartTimes, false);
    this.gainNodes.push(node);
    return node;
  }

  createOscillator() {
    return new FakeAudioNode(this.sourceStartTimes, true);
  }

  createBiquadFilter() {
    return new FakeAudioNode(this.sourceStartTimes, false);
  }

  createBufferSource() {
    return new FakeAudioNode(this.sourceStartTimes, true);
  }

  createBuffer(channels, samples) {
    this.bufferCreateCount += 1;
    this.currentTime += this.bufferRenderSeconds;
    const data = Array.from({ length: channels }, () => new Float32Array(samples));
    return {
      getChannelData(index) {
        return data[index];
      },
    };
  }
}

class FakeAudioNode {
  constructor(sourceStartTimes, recordsStart) {
    this.sourceStartTimes = sourceStartTimes;
    this.recordsStart = recordsStart;
    this.frequency = new FakeAudioParam();
    this.detune = new FakeAudioParam();
    this.gain = new FakeAudioParam();
  }

  connect(destination) {
    return destination || this;
  }

  disconnect() {}

  start(time) {
    if (this.recordsStart) {
      this.sourceStartTimes.push(time);
    }
  }

  stop() {}

  addEventListener() {}
}

class FakeAudioParam {
  constructor() {
    this.value = 0;
  }

  setValueAtTime(value) {
    this.value = value;
  }

  exponentialRampToValueAtTime(value) {
    this.value = value;
  }

  linearRampToValueAtTime(value) {
    this.value = value;
  }
}

const fakeAudio = new FakeAudioContext();
const timedEffect = generateSoundEffect("timing-check", { type: "select" });
const timedPlayer = createSfxPlayer(fakeAudio, timedEffect);
assert.throws(
  () => timedPlayer.start(),
  /explicit AudioContext time/,
  "SFX players should not decide playback time on behalf of the adapter",
);
timedPlayer.start(12.5);
const earliestLayerStart = Math.min(...timedEffect.layers.map((layer) => layer.start));
assert.ok(
  fakeAudio.sourceStartTimes.some((time) => Math.abs(time - (12.5 + earliestLayerStart)) < 0.000001),
  "SFX layer scheduling should be relative to the adapter-provided start time",
);

const loudAudio = new FakeAudioContext();
const loudPlayer = createSfxPlayer(loudAudio, timedEffect, { volume: 1.75 });
loudPlayer.start(12.5);
assert.equal(loudAudio.gainNodes[0].gain.value, 1.75, "SFX playback volume should preserve gain above 1");

const negativeVolumePlayer = createSfxPlayer(new FakeAudioContext(), timedEffect, { volume: -0.1 });
assert.throws(
  () => negativeVolumePlayer.start(12.5),
  /SFX volume must be zero or greater/,
  "negative SFX volume should fail visibly",
);

const puzzleScriptAudio = new FakeAudioContext();
const puzzleScriptPlayer = createPuzzleScriptSfxPlayer(puzzleScriptAudio, puzzleScriptEffect, { volume: 0.35 });
puzzleScriptPlayer.start(12.5);
assert.ok(puzzleScriptAudio.bufferCreateCount > 0, "PuzzleScript SFX playback should render an audio buffer");
assert.equal(puzzleScriptAudio.gainNodes[0].gain.value, 0.35, "PuzzleScript SFX playback should use requested volume");
assert.ok(
  puzzleScriptAudio.sourceStartTimes.every((time) => time >= 12.5),
  "PuzzleScript SFX playback should schedule relative to the adapter-provided start time",
);

const cachedBufferEffect = {
  layers: [
    {
      kind: "noise",
      name: "cached-noise",
      start: 0,
      duration: 0.01,
      gain: 0.2,
      color: "white",
      filterType: "lowpass",
      filterStart: 1200,
      filterEnd: 240,
      attack: 0.001,
      release: 0.001,
    },
    {
      kind: "click",
      name: "cached-click",
      start: 0.002,
      duration: 0.01,
      gain: 0.12,
      filterFrequency: 2200,
    },
  ],
};
const cachedBufferPlayer = createSfxPlayer(fakeAudio, cachedBufferEffect);
const buffersBeforeCacheCheck = fakeAudio.bufferCreateCount;
cachedBufferPlayer.start(20);
assert.equal(fakeAudio.bufferCreateCount - buffersBeforeCacheCheck, 2, "first playback should render noise and click buffers");
cachedBufferPlayer.start(21);
assert.equal(fakeAudio.bufferCreateCount - buffersBeforeCacheCheck, 2, "repeated playback should reuse rendered SFX buffers");

const slowAudio = new FakeAudioContext({ bufferRenderSeconds: 0.02 });
const slowBufferPlayer = createSfxPlayer(slowAudio, cachedBufferEffect);
slowBufferPlayer.start(slowAudio.currentTime);
assert.ok(
  Math.min(...slowAudio.sourceStartTimes) > slowAudio.currentTime,
  "SFX playback should schedule after synchronous buffer preparation instead of starting sources in the past",
);

console.log("seeded_sfx tests passed");
