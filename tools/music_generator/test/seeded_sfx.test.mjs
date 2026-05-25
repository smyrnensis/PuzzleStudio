import assert from "node:assert/strict";
import { generateSoundEffect, randomSfxPreset, SFX_TYPE_OPTIONS, SFX_TYPES } from "../seeded_sfx.mjs";

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
assert.equal(generatedByType.length, 8, "all expected game SFX types should be exposed");
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

const pickupVariants = Array.from({ length: 24 }, (_, index) => generateSoundEffect(`${100000 + index}`, { type: "pickup" }));
const hitVariants = Array.from({ length: 24 }, (_, index) => generateSoundEffect(`${100000 + index}`, { type: "hit" }));
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

const randomPreset = randomSfxPreset("preset-check");
assert.ok(randomPreset.seed, "random SFX presets should include a seed");
assert.ok(SFX_TYPE_OPTIONS.includes(randomPreset.type), "random SFX presets should choose a known type option");
assert.deepEqual(Object.keys(randomPreset).sort(), ["seed", "type"], "random presets should expose only author-facing controls");
assert.equal(generateSoundEffect(randomSfxPreset("target-check", "pickup").seed, { type: "pickup" }).type, "pickup", "targeted randomize should pair seed with an explicit type override");
assert.equal(generateSoundEffect(randomSfxPreset("target-wild", "wild").seed, { type: "wild" }).type, "wild", "targeted Wild randomize should pair seed with an explicit type override");
const targetedRandom = randomSfxPreset("target-random", "random");
assert.ok(/^\d+$/.test(targetedRandom.seed), "targeted Random should return a plain numeric seed");
assert.ok(SFX_TYPES.includes(generateSoundEffect(targetedRandom.seed).type), "targeted Random should resolve to a concrete effect type");
assert.equal(generateSoundEffect("123456", { type: "laser" }).type, "laser", "type override should select laser without encoding it in the seed");
assert.equal(generateSoundEffect("123456", { type: "wild" }).type, "wild", "type override should select Wild without encoding it in the seed");
assert.ok(SFX_TYPES.includes(generateSoundEffect("manual-seed-without-prefix").type), "plain seeds should still map deterministically to a concrete SFX type");
assert.equal(SFX_TYPE_OPTIONS[0], "random", "Random should be the first type option");
assert.equal(SFX_TYPE_OPTIONS.at(-1), "wild", "Wild should be the final type option");

console.log("seeded_sfx tests passed");

function totalGain(effect) {
  return effect.layers.reduce((sum, layer) => sum + layer.gain, 0);
}
