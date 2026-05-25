import assert from "node:assert/strict";
import {
  AXIS_EXPERIMENTS,
  axisExperimentSummary,
  findAxisVariant,
  generateRandomTimbre,
  generateRandomTransient,
  generateTimbreSet,
  generateTransientSet,
  randomTransientSummary,
  randomTimbreSummary,
} from "../timbre_axis_lab.mjs";

const expectedAxes = [
  "harmonicity",
  "noise-role",
  "attack-identity",
  "brightness-decay",
  "resonance-body",
  "pitch-stability",
  "foreground-distance",
];

assert.deepEqual(AXIS_EXPERIMENTS.map((axis) => axis.id), expectedAxes);

for (const axis of AXIS_EXPERIMENTS) {
  assert.ok(axis.variants.length >= 3, `${axis.id} should expose multiple comparison variants`);
  assert.ok(axis.controlled.length >= 3, `${axis.id} should state controlled variables`);
  for (const variant of axis.variants) {
    assert.ok(variant.operations.length >= 1, `${axis.id}/${variant.id} should expose Web Audio operations`);
    assert.ok(variant.signal.envelope, `${axis.id}/${variant.id} should define an envelope`);
  }
}

const harmonic = findAxisVariant("harmonicity", "harmonic");
const inharmonic = findAxisVariant("harmonicity", "inharmonic");
assert.notDeepEqual(
  harmonic.signal.partials.map((partial) => partial[0]),
  inharmonic.signal.partials.map((partial) => partial[0]),
  "harmonicity axis must change partial ratios directly",
);

assert.equal(findAxisVariant("noise-role", "none").signal.noise, undefined);
assert.equal(findAxisVariant("noise-role", "carrier").signal.noise.role, "carrier");
assert.ok(findAxisVariant("attack-identity", "pluck").signal.pluck, "pluck attack should use buffer-pluck mechanism");
assert.ok(findAxisVariant("attack-identity", "strike").signal.partials.some((partial) => partial[2]), "strike should use unequal partial decay");
assert.ok(findAxisVariant("brightness-decay", "filter-decay").signal.filter.endFrequency, "filter decay should close cutoff over time");
assert.ok(
  findAxisVariant("brightness-decay", "filter-decay").signal.filter.frequency / findAxisVariant("brightness-decay", "filter-decay").signal.filter.endFrequency >= 8,
  "filter decay should be strong enough to be auditionable",
);
assert.equal(findAxisVariant("resonance-body", "comb").signal.body.type, "comb");
assert.ok(findAxisVariant("resonance-body", "formant").signal.body.gain <= 0.2, "formant body should be controlled but audible");
assert.ok(findAxisVariant("resonance-body", "comb").signal.body.gain <= 0.2, "comb body should be controlled but audible");
assert.ok(findAxisVariant("pitch-stability", "vibrato").signal.pitch.vibratoCents, "vibrato variant should modulate pitch");
assert.ok(findAxisVariant("pitch-stability", "vibrato").signal.pitch.vibratoCents >= 15, "vibrato should be above the current listening threshold");
assert.ok(findAxisVariant("pitch-stability", "jitter").signal.pitch.jitterCents >= 15, "jitter should be above the current listening threshold");

const summary = axisExperimentSummary();
assert.equal(summary.length, AXIS_EXPERIMENTS.length);
assert.ok(summary.every((axis) => axis.variants.every((variant) => variant.mechanism)), "summary should expose mechanism labels");
assert.ok(summary.every((axis) => axis.variants.every((variant) => variant.operations.length >= 1)), "summary should expose operations");

const generated = generateTimbreSet("dist-test", 16);
assert.deepEqual(generated, generateTimbreSet("dist-test", 16), "generated timbres should be deterministic by seed");
assert.notDeepEqual(generated, generateTimbreSet("dist-test-other", 16), "different seeds should change the generated distribution samples");
assert.equal(generated.length, 16);

for (const model of generated) {
  assert.ok(!("family" in model.parameters), "distributed timbre model should not use named instrument families");
  assert.ok(model.signal.partials.length >= 3, `${model.id} should generate a partial series`);
  assert.ok(model.signal.partials.length <= 24, `${model.id} should keep partial series bounded`);
  assert.equal(model.signal.partials[0][0], 1, `${model.id} should keep f as the pitch anchor`);
  assert.ok(Math.abs(model.parameters.partialEnergy - 1) <= 0.001, `${model.id} should normalize sum a_n(0)^2 to 1`);
  for (const [ratio, gain, decay] of model.signal.partials) {
    assert.ok(Number.isFinite(ratio) && ratio > 0, `${model.id} partial ratio should be positive`);
    assert.ok(Number.isFinite(gain) && gain >= 0 && gain <= 1, `${model.id} partial gain should be normalized`);
    assert.ok(decay === undefined || Number.isFinite(decay) && decay > 0, `${model.id} partial decay should be positive when present`);
  }
  assert.ok(model.parameters.normalizationGain >= 0.3 && model.parameters.normalizationGain <= 1.35, `${model.id} should expose bounded output normalization`);
  assert.ok(model.parameters.filterStart <= 7600, `${model.id} should keep high-frequency color variants bounded`);
  assert.ok(model.signal.distanceGain > 0 && model.signal.distanceGain <= 1.4, `${model.id} should apply bounded output gain`);
  assert.ok(randomTimbreSummary(model).includes("partials"), `${model.id} should expose an audit summary`);
}

const single = generateRandomTimbre("single");
assert.equal(single.id, "random-01");
assert.equal(single.signal.partials[0][0], 1);
assert.ok(Math.abs(single.parameters.partialEnergy - 1) <= 0.001);

const largerSample = generateTimbreSet("distribution-coverage", 64);
assert.ok(new Set(largerSample.map((model) => model.parameters.partialCount)).size >= 4, "partial-count distribution should vary");
assert.ok(new Set(largerSample.map((model) => model.parameters.noiseRole)).size >= 3, "noise-role distribution should vary");
assert.ok(new Set(largerSample.map((model) => model.parameters.bodyType)).size >= 2, "body distribution should vary");
assert.ok(largerSample.some((model) => model.parameters.spectralField.length >= 3), "some samples should have multiple smooth spectral field points");
assert.ok(largerSample.some((model) => model.parameters.alpha <= 0.55), "some samples should keep upper partials strong");
assert.ok(largerSample.some((model) => model.parameters.alpha >= 1.8), "some samples should concentrate around lower partials");
assert.ok(largerSample.some((model) => model.parameters.dropoutRate >= 0.25), "some samples should have strong coefficient dropout");
assert.ok(largerSample.some((model) => model.parameters.ratioDrift >= 0.01), "some samples should allow audible ratio drift");
assert.ok(largerSample.some((model) => model.parameters.decaySlope <= -0.2), "some samples should preserve or emphasize upper partials over time");
assert.ok(largerSample.some((model) => model.parameters.decaySlope >= 0.8), "some samples should decay upper partials faster");
assert.ok(
  largerSample.some((model) => model.parameters.continuity >= 0.55 && model.signal.partials.filter((partial) => partial.length >= 3).length <= model.signal.partials.length / 2),
  "some stochastic fields should produce sustained spectra without choosing a named pattern",
);
const denseSustained = largerSample.filter((model) => model.signal.partials.length >= 12 && model.parameters.continuity >= 0.5);
assert.ok(denseSustained.every((model) => model.parameters.normalizationGain < 1), "dense sustained spectra should be attenuated after coefficient normalization");

const transientSet = generateTransientSet("transient-test", 16);
assert.deepEqual(transientSet, generateTransientSet("transient-test", 16), "transient fields should be deterministic by seed");
assert.notDeepEqual(transientSet, generateTransientSet("transient-other", 16), "different transient seeds should change samples");
assert.equal(transientSet.length, 16);

for (const model of transientSet) {
  assert.ok(!("family" in model.parameters), "transient model should not use named drum families");
  assert.ok(model.signal.bands.length >= 3, `${model.id} should generate noise bands`);
  assert.ok(model.signal.bands.length <= 16, `${model.id} should keep noise bands bounded`);
  assert.ok(Math.abs(model.parameters.noiseEnergy - 1) <= 0.001, `${model.id} should normalize sum band gain^2 to 1`);
  for (const band of model.signal.bands) {
    assert.ok(Number.isFinite(band.frequency) && band.frequency >= 70 && band.frequency <= 12000, `${model.id} band frequency should stay audible`);
    assert.ok(Number.isFinite(band.gain) && band.gain >= 0 && band.gain <= 1, `${model.id} band gain should be normalized`);
    assert.ok(Number.isFinite(band.decay) && band.decay > 0, `${model.id} band decay should be positive`);
  }
  assert.ok(model.parameters.attack >= 0.0008 && model.parameters.attack <= 0.075, `${model.id} should use transient attacks`);
  assert.ok(model.parameters.normalizationGain >= 0.28 && model.parameters.normalizationGain <= 1.25, `${model.id} should expose bounded output normalization`);
  assert.ok(model.signal.distanceGain > 0 && model.signal.distanceGain <= 1.25, `${model.id} should apply bounded output gain`);
  assert.ok(randomTransientSummary(model).includes("bands"), `${model.id} should expose an audit summary`);
}

const singleTransient = generateRandomTransient("single-transient");
assert.equal(singleTransient.id, "transient-01");
assert.ok(Math.abs(singleTransient.parameters.noiseEnergy - 1) <= 0.001);

const transientCoverage = generateTransientSet("transient-coverage", 64);
assert.ok(new Set(transientCoverage.map((model) => model.parameters.bandCount)).size >= 4, "transient band-count distribution should vary");
assert.ok(transientCoverage.some((model) => model.parameters.spectralTilt >= 1), "some transient samples should emphasize high bands");
assert.ok(transientCoverage.some((model) => model.parameters.spectralTilt <= -1), "some transient samples should emphasize low bands");
assert.ok(transientCoverage.some((model) => model.parameters.dropoutRate >= 0.25), "some transient samples should have strong band dropout");
assert.ok(transientCoverage.some((model) => model.parameters.clickGain > 0.08), "some transient samples should have click energy");
assert.ok(transientCoverage.some((model) => model.parameters.resonatorCount > 0), "some transient samples should include resonant bumps");

console.log("timbre_axis_lab tests passed");
