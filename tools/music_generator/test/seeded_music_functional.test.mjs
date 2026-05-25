import assert from "node:assert/strict";
import { generateFunctionalSong, randomFunctionalPreset } from "../seeded_music_functional.mjs";

const first = generateFunctionalSong("same-seed", { tone: 0.5, bpm: 110 });
const second = generateFunctionalSong("same-seed", { tone: 0.5, bpm: 110 });
const different = generateFunctionalSong("different-seed", { tone: 0.5, bpm: 110 });

assert.deepEqual(first, second, "functional generator should stay deterministic");
assert.notDeepEqual(first, different, "different seeds should usually produce different functional songs");
assert.equal(first.playbackScore.version, 1);
assert.equal(first.playbackScore.transport.bars, 64);
assert.equal(first.playbackScore.transport.stepsPerBar, 16);
assert.equal(first.input.bars, 64);
assert.equal(first.input.height, 0.5);
assert.equal(first.input.focus, 0.5);
assert.equal(first.input.brightness, 0.5);
assert.equal(first.input.presence, 0.5);
assert.equal(first.input.attack, 0.5);
assert.equal(first.playbackScore.mix.playbackTone.brightness, 0.5);
assert.equal(first.playbackScore.mix.playbackTone.presence, 0.5);
assert.equal(first.playbackScore.mix.playbackTone.attack, 0.5);
assert.equal(first.debug.sectionPlan.length, 8);
assert.deepEqual(
  first.debug.sectionPlan.slice(0, 4).map((section) => section.name),
  ["establish", "vary-one-axis", "vary-two-axes", "late-variation"],
);
assert.deepEqual(
  first.debug.sectionPlan.slice(0, 4).map((section) => section.motifRole),
  ["primary", "answer", "contrast", "lift"],
);

const obligationNames = ["identity", "time", "tone", "motion", "color", "boundary"];
for (const name of obligationNames) {
  assert.ok(first.debug.obligations[name], `missing ${name} obligation`);
  assert.equal(first.debug.obligations[name].name, name);
  assert.ok(first.debug.obligations[name].carrier);
  assert.ok(first.debug.obligations[name].purpose);
  assert.ok(first.debug.trackMapping[name]);
}

for (const event of first.playbackScore.events) {
  assert.ok(first.playbackScore.timbres[event.timbre], `missing timbre ${event.timbre}`);
  assert.ok(event.step >= 0);
  assert.ok(event.step < first.playbackScore.transport.loopSteps);
  assert.ok(event.durationSteps > 0);
  assert.ok(event.notes.length > 0);
}

for (const bars of [8, 16, 32, 64]) {
  const sized = generateFunctionalSong(`bars-${bars}`, { tone: 0.5, bpm: 110, bars });
  assert.equal(sized.input.bars, bars, `${bars}-bar song should preserve requested bar count`);
  assert.equal(sized.playbackScore.source.bars, bars, `${bars}-bar song should expose source bar count`);
  assert.equal(sized.playbackScore.transport.bars, bars, `${bars}-bar song should set transport bar count`);
  assert.equal(sized.playbackScore.transport.loopSteps, bars * 16, `${bars}-bar song should set loop length`);
  assert.equal(sized.debug.sectionPlan.length, bars / 8, `${bars}-bar song should allocate one section per 8 bars`);
  assert.ok(
    sized.playbackScore.events.every((event) => event.step < sized.playbackScore.transport.loopSteps),
    `${bars}-bar song should not emit events beyond the loop`,
  );
}

const oldNamedMelodyTimbres = new Set([
  "breathy-flute",
  "nylon",
  "warm-pluck",
  "low-pluck",
  "fuzzy-pluck",
  "dust-lead",
  "reed",
  "soft-square",
  "harp",
  "marimba",
  "music-box",
  "chip-lead",
  "triangle-lead",
  "saw-lead",
]);

for (const [name, timbre] of Object.entries(first.playbackScore.timbres)) {
  if (["kick", "snare", "hat"].includes(name)) {
    assert.equal(timbre.kind, "transient-field", `functional drum timbre ${name} should use the transient field`);
    assert.equal(timbre.engine, "stochastic-transient-field", `functional drum timbre ${name} should expose its synthesis engine`);
    assert.ok(timbre.signal?.bands?.length >= 3, `functional drum timbre ${name} should expose noise bands`);
    assert.ok(Math.abs(timbre.parameters.noiseEnergy - 1) <= 0.001, `functional drum timbre ${name} should normalize noise band energy`);
    assert.ok(timbre.gain > 0 && timbre.gain <= 1, `functional drum timbre ${name} should have balanced intrinsic gain`);
    continue;
  }
  assert.ok(!oldNamedMelodyTimbres.has(timbre.kind), "functional playback should not use the old named melody palette");
  assert.equal(timbre.kind, "spectral-field", `functional timbre ${name} should use the stochastic spectral field`);
  assert.equal(timbre.engine, "stochastic-spectral-field", `functional timbre ${name} should expose its synthesis engine`);
  assert.ok(timbre.signal?.partials?.length >= 3, `functional timbre ${name} should expose partials`);
  assert.ok(Math.abs(timbre.parameters.partialEnergy - 1) <= 0.001, `functional timbre ${name} should normalize partial energy`);
  assert.ok(timbre.gain > 0 && timbre.gain <= 1, `functional timbre ${name} should have balanced intrinsic gain`);
}

const sampleSongs = Array.from({ length: 160 }, (_, index) => generateFunctionalSong(`function-${index}`, { tone: 0.5, bpm: 110 }));
const notesByTrack = {};
for (const song of sampleSongs) {
  for (const event of song.playbackScore.events) {
    for (const note of event.notes) {
      if (typeof note === "number") {
        assert.ok(Number.isFinite(note), "pitched events should not contain NaN or infinite notes");
        notesByTrack[event.track] ??= [];
        notesByTrack[event.track].push(note);
      }
    }
  }
}

const average = (values) => values.reduce((sum, value) => sum + value, 0) / values.length;
assert.ok(average(notesByTrack.lead) <= 68, "default lead register should stay in a middle range");
assert.ok(average(notesByTrack.counter) <= 62, "default counter register should not sit above the lead");
assert.ok(average(notesByTrack.chord) <= 68, "default chord register should not dominate in a high range");
assert.ok(Math.max(...notesByTrack.lead) <= 88, "default lead should avoid very high fundamentals");

const carriersByFunction = Object.fromEntries(obligationNames.map((name) => [
  name,
  new Set(sampleSongs.map((song) => song.debug.obligations[name].carrier)),
]));
const scaleNames = new Set(sampleSongs.map((song) => song.debug.scale));

assert.ok(carriersByFunction.identity.size >= 4, "identity should not collapse to melody-only");
assert.ok(carriersByFunction.time.size >= 4, "time should not collapse to drums-only");
assert.ok(carriersByFunction.tone.size >= 4, "tone should not collapse to chord-only");
assert.ok(carriersByFunction.motion.size >= 4, "motion should expose multiple motion strategies");
assert.ok(carriersByFunction.color.size >= 4, "color should expose multiple texture strategies");
assert.ok(carriersByFunction.boundary.size >= 4, "boundary should expose multiple phrase-edge strategies");
assert.ok(scaleNames.size >= 7, "functional songs should draw from a broad pitch-set vocabulary");

assert.ok(
  sampleSongs.some((song) => song.debug.obligations.identity.carrier !== "melodic-line"),
  "identity should sometimes be carried by non-melody material",
);
assert.ok(
  sampleSongs.some((song) => !song.debug.trackMapping.motion.playbackTracks.includes("counter")),
  "motion should sometimes be carried without a counter track",
);

const melodicRhythmProfiles = new Set(sampleSongs
  .filter((song) => song.debug.obligations.identity.carrier === "melodic-line")
  .map((song) => song.playbackScore.events
    .filter((event) => event.role === "identity" && event.track === "lead" && event.step < 16)
    .map((event) => `${event.step}:${event.durationSteps}`)
    .join("|")));
assert.ok(melodicRhythmProfiles.size >= 6, "melodic identity should draw from varied rhythm profiles across seeds");

const melodicLeadLines = sampleSongs
  .filter((song) => song.debug.obligations.identity.carrier === "melodic-line")
  .map((song) => song.playbackScore.events
    .filter((event) => event.role === "identity" && event.track === "lead")
    .sort((left, right) => left.step - right.step));
let melodicIntervals = 0;
let melodicLeaps = 0;
let oneStepMelodyNotes = 0;
let melodyNotes = 0;
for (const line of melodicLeadLines) {
  for (const event of line) {
    melodyNotes += 1;
    if (event.durationSteps === 1) {
      oneStepMelodyNotes += 1;
    }
  }
  for (let index = 1; index < line.length; index += 1) {
    melodicIntervals += 1;
    if (Math.abs(line[index].notes[0] - line[index - 1].notes[0]) >= 3) {
      melodicLeaps += 1;
    }
  }
}
assert.ok(oneStepMelodyNotes / melodyNotes <= 0.03, "melodic identity should not dissolve into point notes");
assert.ok(melodicLeaps / melodicIntervals >= 0.22, "melodic identity should use contour leaps instead of mostly stepwise random walk");

const sectionSignature = (song, sectionIndex) => {
  const sectionStart = sectionIndex * 8 * 16;
  const sectionEnd = sectionStart + 8 * 16;
  return song.playbackScore.events
    .filter((event) => event.step >= sectionStart && event.step < sectionEnd)
    .map((event) => `${event.track}:${event.step - sectionStart}:${event.durationSteps}:${event.notes.join(".")}:${event.timbre}:${event.velocity.toFixed(3)}`)
    .join("|");
};

assert.ok(
  sampleSongs.filter((song) => sectionSignature(song, 0) !== sectionSignature(song, 2)).length >= 140,
  "64-bar songs should usually contain a real middle-section change",
);

const bridgeSongs = sampleSongs.filter((song) => song.debug.sectionPlan.slice(4).some((section) => section.name.startsWith("b-")));
assert.ok(bridgeSongs.length >= 35, "some 64-bar songs should use a true B-section instead of only A/A-prime");
for (const song of bridgeSongs) {
  const bSections = song.debug.sectionPlan.slice(4);
  assert.ok(bSections.every((section) => section.carrierOverrides), "B-section should expose section-local carrier changes");
  assert.ok(
    bSections.some((section) => section.carrierOverrides.identity !== song.debug.obligations.identity.carrier),
    "B-section should move identity to a different carrier from the A-section",
  );
}

const verseChorusSongs = sampleSongs.filter((song) => song.debug.sectionPlan.some((section) => section.name === "chorus"));
assert.ok(verseChorusSongs.length >= 25, "some 64-bar songs should use explicit A/B/chorus form");
for (const song of verseChorusSongs) {
  const aCarrier = song.debug.sectionPlan[0].carrierOverrides?.identity ?? song.debug.obligations.identity.carrier;
  const bCarrier = song.debug.sectionPlan[2].carrierOverrides?.identity ?? song.debug.obligations.identity.carrier;
  const chorusCarrier = song.debug.sectionPlan[4].carrierOverrides?.identity ?? song.debug.obligations.identity.carrier;
  assert.deepEqual(
    song.debug.sectionPlan.slice(0, 6).map((section) => section.name),
    ["a-verse", "a-answer", "b-verse", "b-prechorus", "chorus", "chorus-answer"],
    "explicit song form should allocate separate A, B, and chorus sections",
  );
  assert.notEqual(aCarrier, bCarrier, "B section should move identity away from A");
  assert.ok(
    ["melodic-line", "harmony-arp"].includes(chorusCarrier),
    "chorus should keep a pitched foreground hook instead of dropping to rhythm-only identity",
  );
}

const melodySong = generateFunctionalSong("melody-1", { height: 0.5, bpm: 110 });
assert.equal(melodySong.debug.obligations.identity.carrier, "melodic-line", "fixture should exercise melodic identity");
const leadIdentitySignature = (song, sectionIndex) => {
  const sectionStart = sectionIndex * 8 * 16;
  const sectionEnd = sectionStart + 8 * 16;
  return song.playbackScore.events
    .filter((event) => event.role === "identity" && event.track === "lead" && event.step >= sectionStart && event.step < sectionEnd)
    .map((event) => `${event.step - sectionStart}:${event.durationSteps}`)
    .join("|");
};
assert.notEqual(
  leadIdentitySignature(melodySong, 0),
  leadIdentitySignature(melodySong, 2),
  "melodic identity should use a distinct contrast motif in the middle section",
);
assert.notEqual(
  leadIdentitySignature(melodySong, 2),
  leadIdentitySignature(melodySong, 3),
  "late melodic section should not be a direct repeat of the contrast motif",
);
const leadIdentityBarSignature = (song, bar) => song.playbackScore.events
  .filter((event) => event.role === "identity" && event.track === "lead" && event.step >= bar * 16 && event.step < (bar + 1) * 16)
  .map((event) => `${event.step - bar * 16}:${event.durationSteps}:${event.notes.join(".")}`)
  .join("|");
const leadIdentityRhythmSignature = (song, sectionIndex) => {
  const sectionStart = sectionIndex * 8 * 16;
  return song.playbackScore.events
    .filter((event) => event.role === "identity" && event.track === "lead" && event.step >= sectionStart && event.step < sectionStart + 16)
    .map((event) => `${event.step - sectionStart}:${event.durationSteps}`)
    .join("|");
};
assert.notEqual(
  leadIdentityBarSignature(melodySong, 0),
  leadIdentityBarSignature(melodySong, 2),
  "primary motif should develop within a four-bar phrase instead of repeating every bar exactly",
);
assert.equal(
  new Set([0, 1, 2, 3].map((sectionIndex) => leadIdentityRhythmSignature(melodySong, sectionIndex))).size,
  4,
  "primary, answer, contrast, and lift motifs should have distinct rhythm profiles",
);

const rhythmHookSongs = sampleSongs.filter((song) => song.debug.obligations.identity.carrier === "rhythm-hook");
assert.ok(rhythmHookSongs.length > 0, "sample should include rhythm-hook identity cases");
for (const song of rhythmHookSongs) {
  const activeRhythmBars = new Set();
  for (const [index, section] of song.debug.sectionPlan.entries()) {
    const carrier = section.carrierOverrides?.identity ?? song.debug.obligations.identity.carrier;
    if (carrier === "rhythm-hook") {
      for (let localBar = 0; localBar < 8; localBar += 1) {
        activeRhythmBars.add(index * 8 + localBar);
      }
    }
  }
  const identityDrumBars = new Set(song.playbackScore.events
    .filter((event) => event.role === "identity" && event.track === "drums" && event.velocity >= 0.08)
    .map((event) => Math.floor(event.step / 16)));
  assert.ok(identityDrumBars.size >= Math.floor(activeRhythmBars.size * 0.75), "rhythm-hook identity should be present across most active rhythm-hook bars");
  assert.ok(
    song.debug.obligations.time.carrier !== "drum-grid",
    "rhythm-hook identity should not compete with drum-grid time by default",
  );
}

const preset = randomFunctionalPreset("functional-preset");
assert.ok(preset.seed);
assert.equal(preset.height, 0.5);
assert.equal(preset.focus, 0.5);
assert.equal(preset.brightness, 0.5);
assert.equal(preset.presence, 0.5);
assert.equal(preset.attack, 0.5);
assert.equal(preset.tone, 0.5);
assert.equal(preset.bpm, 110);

const higher = generateFunctionalSong("knob-check", { height: 1, bpm: 110 });
const lower = generateFunctionalSong("knob-check", { height: 0, bpm: 110 });
assert.equal(higher.playbackScore.mix.playbackTone.pitchShift, 12, "height should transpose pitched playback up by one octave");
assert.equal(lower.playbackScore.mix.playbackTone.pitchShift, -12, "height should transpose pitched playback down by one octave");
assert.equal(higher.playbackScore.mix.playbackTone.leadGain, lower.playbackScore.mix.playbackTone.leadGain, "height should not change pitched gain");
assert.equal(higher.playbackScore.mix.playbackTone.toneFilter, lower.playbackScore.mix.playbackTone.toneFilter, "height should not act as a loudness-like filter opener");
assert.equal(higher.playbackScore.mix.playbackTone.highPercussionGain, lower.playbackScore.mix.playbackTone.highPercussionGain, "height should not change percussion gain");
assert.equal(higher.playbackScore.mix.playbackTone.attackShape, lower.playbackScore.mix.playbackTone.attackShape, "height should not reshape transient envelopes");
assert.deepEqual(
  higher.playbackScore.timbres.identity.signal,
  lower.playbackScore.timbres.identity.signal,
  "playback knobs should not change generated spectral fields",
);

console.log("seeded_music_functional tests passed");
