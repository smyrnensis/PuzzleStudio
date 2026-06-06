import assert from "node:assert/strict";
import { generateSong, randomPreset } from "../seeded_music.mjs";

const first = generateSong("same-seed", { tone: 0.5, bpm: 110 });
const second = generateSong("same-seed", { tone: 0.5, bpm: 110 });
const different = generateSong("different-seed", { tone: 0.5, bpm: 110 });
const styleA = generateSong("12style-lock", { tone: 0.5, bpm: 110, bars: 64 });
const styleB = generateSong("34style-lock", { tone: 0.5, bpm: 110, bars: 64 });

assert.deepEqual(first, second, "music generator should stay deterministic");
assert.notDeepEqual(first, different, "different seeds should usually produce different songs");
assert.deepEqual(styleA.debug.seedParts, { variation: "12style-lock", style: "style-lock", width: 2 });
assert.deepEqual(styleB.debug.seedParts, { variation: "34style-lock", style: "style-lock", width: 2 });
assert.equal(styleA.debug.key, styleB.debug.key, "changing only the seed prefix should preserve key");
assert.equal(styleA.debug.scale, styleB.debug.scale, "changing only the seed prefix should preserve scale");
assert.deepEqual(styleA.debug.form, styleB.debug.form, "changing only the seed prefix should preserve form identity");
assert.deepEqual(styleA.debug.roles, styleB.debug.roles, "changing only the seed prefix should preserve composition roles");
assert.deepEqual(styleA.debug.timbres, styleB.debug.timbres, "changing only the seed prefix should preserve generated timbres");
assert.notDeepEqual(styleA.playbackScore.events, styleB.playbackScore.events, "changing only the seed prefix should still regenerate the musical content");
assert.equal(first.playbackScore.version, 1);
assert.equal(first.playbackScore.transport.bars, 8);
assert.equal(first.playbackScore.transport.stepsPerBar, 16);
assert.equal(first.input.bars, 8);
assert.equal(first.input.height, 0.5);
assert.equal(first.input.focus, 0.5);
assert.equal(first.input.brightness, 0.5);
assert.equal(first.input.presence, 0.5);
assert.equal(first.input.attack, 0.5);
assert.equal(first.playbackScore.mix.playbackTone.brightness, 0.5);
assert.equal(first.playbackScore.mix.playbackTone.presence, 0.5);
assert.equal(first.playbackScore.mix.playbackTone.attack, 0.5);
assert.equal(first.debug.sectionPlan.length, 1);
for (const field of ["novelty", "stability", "density", "tension", "closurePressure", "memoryDistance"]) {
  assert.ok(first.debug.sectionPlan.every((section) => section[field] >= 0 && section[field] <= 1), `section ${field} should be normalized`);
}
assert.equal(first.debug.barPlan.length, first.input.bars, "bar-level form projection should expose one phrase state per bar");
assert.ok(first.debug.sectionPlan.every((section) => !("phraseShape" in section)), "section anchors should not own phrase shapes");
assert.ok(first.debug.barPlan.every((bar) => bar.phraseBar.pace >= 0 && bar.phraseBar.pace <= 1 && bar.phraseBar.targetCenter >= -1 && bar.phraseBar.targetCenter <= 1), "bar-level phrase states should expose normalized pace and target center");
assert.ok(first.debug.sectionPlan[0].closurePressure >= 0.8, "default 8-bar songs should use a single compact loop-closing section");

const long = generateSong("same-seed", { tone: 0.5, bpm: 110, bars: 64 });
assert.equal(long.debug.sectionPlan.length, 8);
assert.ok(new Set(long.debug.sectionPlan.map((section) => `${Math.round(section.novelty * 10)}:${Math.round(section.tension * 10)}:${Math.round(section.closurePressure * 10)}:${Math.round(section.memoryDistance * 10)}`)).size >= 3, "64-bar form should allocate multiple state vectors");
assert.ok(long.debug.sectionPlan.some((section) => section.memoryDistance >= 0.5 || section.tension >= 0.65), "64-bar form should include a high-distance or high-tension section");
assert.ok(long.debug.sectionPlan.some((section) => section.energy >= 0.72 || section.closurePressure >= 0.78), "64-bar form should include an energy or closure peak");
assert.notDeepEqual(
  long.debug.sectionPlan.slice(0, 4).map((section) => `${section.novelty}:${section.stability}:${section.tension}:${section.closurePressure}:${section.memoryDistance}`),
  long.debug.sectionPlan.slice(4, 8).map((section) => `${section.novelty}:${section.stability}:${section.tension}:${section.closurePressure}:${section.memoryDistance}`),
  "64-bar form should not be a repeated 32-bar half",
);

const roleNames = ["identity", "time", "tone", "motion", "color", "boundary"];
for (const name of roleNames) {
  assert.ok(first.debug.roles[name], `missing ${name} role`);
  assert.equal(first.debug.roles[name].name, name);
  assert.ok(first.debug.roles[name].carrier);
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
  const sized = generateSong(`bars-${bars}`, { tone: 0.5, bpm: 110, bars });
  assert.equal(sized.input.bars, bars, `${bars}-bar song should preserve requested bar count`);
  assert.equal(sized.playbackScore.source.bars, bars, `${bars}-bar song should expose source bar count`);
  assert.equal(sized.playbackScore.transport.bars, bars, `${bars}-bar song should set transport bar count`);
  assert.equal(sized.playbackScore.transport.loopSteps, bars * 16, `${bars}-bar song should set loop length`);
  assert.equal(sized.debug.sectionPlan.length, bars / 8, `${bars}-bar song should allocate one section per 8 bars`);
  assert.deepEqual(
    sized.debug.sectionPlan.map((section) => Boolean(section.loopHandoff)),
    sized.debug.sectionPlan.map((_, index) => index === sized.debug.sectionPlan.length - 1),
    `${bars}-bar song should mark only its final section as the loop handoff`,
  );
  if (bars === 8) {
    assert.ok(sized.debug.sectionPlan[0].closurePressure >= 0.8, "8-bar songs should use a single compact loop-closing section");
  }
  if (bars === 16) {
    assert.ok(sized.debug.sectionPlan[0].novelty <= 0.14 && sized.debug.sectionPlan[0].memoryDistance <= 0.12, "16-bar songs should start near the loop identity");
    assert.ok(sized.debug.sectionPlan[1].closurePressure >= 0.7 || sized.debug.sectionPlan[1].novelty >= 0.12, "16-bar songs should use a compact second state");
  }
  if (bars === 32) {
    assert.ok(sized.debug.sectionPlan.some((section) => section.memoryDistance >= 0.34 || section.tension >= 0.48 || section.closurePressure >= 0.68), "32-bar songs should allow a moderate state change without requiring a large peak");
  }
  if (bars === 64) {
    assert.ok(sized.debug.sectionPlan.slice(4).some((section) => section.memoryDistance >= 0.5 || section.closurePressure >= 0.68 || section.density <= 0.32), "64-bar songs should allocate a second-half state change instead of repeating the first half");
  }
  assert.ok(
    sized.playbackScore.events.every((event) => event.step < sized.playbackScore.transport.loopSteps),
    `${bars}-bar song should not emit events beyond the loop`,
  );
  assert.ok(
    sized.playbackScore.events.some((event) => event.role === "boundary" && event.track === "bass" && event.step >= sized.playbackScore.transport.loopSteps - 2),
    `${bars}-bar song should end with a loop-handoff bass target near the wrap point`,
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
    assert.equal(timbre.kind, "transient-field", `composition drum timbre ${name} should use the transient field`);
    assert.equal(timbre.engine, "stochastic-transient-field", `composition drum timbre ${name} should expose its synthesis engine`);
    assert.ok(timbre.signal?.bands?.length >= 3, `composition drum timbre ${name} should expose noise bands`);
    assert.ok(Math.abs(timbre.parameters.noiseEnergy - 1) <= 0.001, `composition drum timbre ${name} should normalize noise band energy`);
    assert.ok(timbre.gain > 0 && timbre.gain <= 1, `composition drum timbre ${name} should have balanced intrinsic gain`);
    continue;
  }
  assert.ok(!oldNamedMelodyTimbres.has(timbre.kind), "composition playback should not use the old named melody palette");
  assert.equal(timbre.kind, "spectral-field", `composition timbre ${name} should use the stochastic spectral field`);
  assert.equal(timbre.engine, "stochastic-spectral-field", `composition timbre ${name} should expose its synthesis engine`);
  assert.ok(timbre.signal?.partials?.length >= 3, `composition timbre ${name} should expose partials`);
  assert.ok(Math.abs(timbre.parameters.partialEnergy - 1) <= 0.001, `composition timbre ${name} should normalize partial energy`);
  assert.ok(timbre.gain > 0 && timbre.gain <= 1, `composition timbre ${name} should have balanced intrinsic gain`);
}

const sampleSongs = Array.from({ length: 160 }, (_, index) => generateSong(`42function-style-${index}`, { tone: 0.5, bpm: 110, bars: 64 }));
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

const carriersByFunction = Object.fromEntries(roleNames.map((name) => [
  name,
  new Set(sampleSongs.map((song) => song.debug.roles[name].carrier)),
]));
const scaleNames = new Set(sampleSongs.map((song) => song.debug.scale));
const progressionSignatures = sampleSongs.map((song) => song.debug.progression.join("-"));
const progressionCounts = new Map();
for (const signature of progressionSignatures) {
  progressionCounts.set(signature, (progressionCounts.get(signature) || 0) + 1);
}
const firstSectionPhraseArchetypes = new Set(sampleSongs.map((song) => song.debug.barPlan[0].phraseArchetype));
const firstSectionModulationSignatures = new Set(sampleSongs.map((song) => song.debug.barPlan.slice(0, 8)
  .map(({ phraseBar: bar }) => `${Math.round(bar.targetCenter * 10)}:${Math.round(bar.heightBias * 10)}:${Math.round(bar.pace * 10)}:${Math.round(bar.energy * 10)}:${bar.toneAnchor ? "t" : "-"}:${bar.colorAccent ? "c" : "-"}:${Math.round(bar.boundary * 10)}`)
  .join("|")));

assert.ok(carriersByFunction.identity.size >= 4, "identity should not collapse to melody-only");
assert.ok(carriersByFunction.time.size >= 4, "time should not collapse to drums-only");
assert.ok(carriersByFunction.tone.size >= 4, "tone should not collapse to chord-only");
assert.ok(carriersByFunction.motion.size >= 4, "motion should expose multiple motion strategies");
assert.ok(carriersByFunction.color.size >= 4, "color should expose multiple texture strategies");
assert.ok(carriersByFunction.boundary.size >= 4, "boundary should expose multiple phrase-edge strategies");
assert.ok(scaleNames.size >= 7, "songs should draw from a broad pitch-set vocabulary");
assert.ok(sampleSongs.every((song) => song.debug.progression[0] === 1), "harmonic progression should start from home");
assert.ok(new Set(progressionSignatures).size >= 80, "harmonic progression should be stochastic instead of a small fixed table");
assert.ok(Math.max(...progressionCounts.values()) / progressionSignatures.length < 0.08, "harmonic progression should not be dominated by one hand-authored loop");
assert.ok(firstSectionPhraseArchetypes.size >= 5, "8-bar modulation should draw from multiple phrase-shape archetypes");
assert.ok(firstSectionModulationSignatures.size >= 100, "8-bar modulation should vary by seed instead of reusing one bar-role curve");

const phrasePaces = sampleSongs.flatMap((song) => song.debug.barPlan.map((bar) => bar.phraseBar.pace));
assert.ok(phrasePaces.some((pace) => pace <= 0.34), "phrase pace should sometimes allow slower quarter-note-like motion");
assert.ok(phrasePaces.some((pace) => pace >= 0.62), "phrase pace should sometimes allow faster eighth-note-like motion");

const chordEventsByBar = [];
const chordEvents = [];
for (const song of sampleSongs) {
  for (let bar = 0; bar < song.playbackScore.transport.bars; bar += 1) {
    const events = song.playbackScore.events.filter((event) => event.track === "chord" && event.step >= bar * 16 && event.step < (bar + 1) * 16);
    chordEventsByBar.push(events.length);
    chordEvents.push(...events);
  }
}
const quantile = (values, amount) => [...values].sort((left, right) => left - right)[Math.floor((values.length - 1) * amount)];
assert.ok(average(chordEventsByBar) <= 2.8, "chord texture should not dominate average event density");
assert.ok(quantile(chordEventsByBar, 0.95) <= 7, "chord texture should avoid dense per-bar clusters");
assert.ok(chordEvents.filter((event) => event.durationSteps <= 2).length / chordEvents.length <= 0.32, "chord texture should favor fewer held tones over many short points");

assert.ok(
  sampleSongs.some((song) => song.debug.roles.identity.carrier !== "melodic-line"),
  "identity should sometimes be carried by non-melody material",
);
assert.ok(
  sampleSongs.some((song) => !song.debug.trackMapping.motion.playbackTracks.includes("counter")),
  "motion should sometimes be carried without a counter track",
);

const timeDownbeatTracks = new Set();
for (const song of sampleSongs) {
  for (let bar = 0; bar < song.playbackScore.transport.bars; bar += 1) {
    const downbeatTimeEvents = song.playbackScore.events.filter((event) => event.role === "time" && event.step === bar * 16);
    assert.ok(downbeatTimeEvents.length > 0, "each bar should keep one readable time anchor on the downbeat");
    for (const event of downbeatTimeEvents) {
      timeDownbeatTracks.add(event.track);
    }
  }
}
assert.ok(timeDownbeatTracks.size >= 3, "downbeat time anchors should be carried by multiple musical layers, not drums only");

const continuityCarrier = (name, carrier) => {
  if (name === "identity" || name === "time") return carrier !== "none";
  if (name === "tone") return carrier !== "implied" && carrier !== "none";
  if (name === "motion") return carrier === "answer-line" || carrier === "harmony-arp" || carrier === "bass-walk";
  if (name === "color") return carrier === "air-pad" || carrier === "noise-halo" || carrier === "organ-bed";
  return false;
};
const sectionCarriers = (song, section) => Object.fromEntries(roleNames.map((name) => [
  name,
  section.roles[name].carrier,
]));
for (const song of sampleSongs) {
  for (let index = 0; index < song.debug.sectionPlan.length; index += 1) {
    const left = sectionCarriers(song, song.debug.sectionPlan[index]);
    const right = sectionCarriers(song, song.debug.sectionPlan[(index + 1) % song.debug.sectionPlan.length]);
    assert.ok(
      roleNames.some((name) => left[name] === right[name] && continuityCarrier(name, left[name])),
      "adjacent sections should retain at least one dense continuity carrier",
    );
  }
}

const transitionEntries = [];
for (const song of sampleSongs) {
  for (let index = 1; index < song.debug.barPlan.length; index += 1) {
    const bar = song.debug.barPlan[index];
    if (bar.localBar !== 0 || !bar.transitionIn) {
      continue;
    }
    transitionEntries.push({ song, previous: song.debug.barPlan[index - 1], bar });
  }
}
assert.ok(transitionEntries.length >= 40, "high-impact section changes should usually expose transition context");
for (const { song, previous, bar } of transitionEntries) {
  assert.ok(previous.transitionOut && previous.transitionOut.impact === bar.transitionIn.impact, "adjacent bars should share one transition context");
  assert.ok(bar.phraseBar.boundary <= 0.25, "incoming transition phrase should not begin with a strong phrase boundary accent");
  assert.ok(bar.phraseBar.pickup <= 0.35, "incoming transition phrase should not begin with a strong pickup accent");
  assert.ok(previous.phraseBar.transitionBridge, "outgoing transition bars should expose an explicit bridge instead of relying on state interpolation");
  assert.ok(bar.phraseBar.transitionEntryBridge, "incoming transition bars should expose an explicit entry bridge instead of relying on state interpolation");
  assert.equal(previous.phraseBar.transitionBridge.impact, previous.transitionOut.impact, "bridge should belong to the same transition context as the outgoing bar");
  assert.equal(bar.phraseBar.transitionEntryBridge.impact, bar.transitionIn.impact, "entry bridge should belong to the same transition context as the incoming bar");
  const bridge = previous.phraseBar.transitionBridge;
  const entryBridge = bar.phraseBar.transitionEntryBridge;
  assert.equal(entryBridge.track, bridge.track, "outgoing and incoming bridge should use the same continuity track");
  const bridgeEvents = song.playbackScore.events.filter((event) => (
    event.role === "boundary"
      && event.track === bridge.track
      && event.step >= previous.bar * 16 + 9
      && event.step < (previous.bar + 1) * 16
  ));
  const entryBridgeEvents = song.playbackScore.events.filter((event) => (
    event.track === entryBridge.track
      && event.step >= bar.bar * 16
      && event.step < bar.bar * 16 + 4
  ));
  assert.ok(bridgeEvents.length > 0, "outgoing transition bars should render a late bridge event on the continuity track");
  assert.ok(entryBridgeEvents.length > 0, "incoming transition bars should catch the bridge on the same continuity track");
}

const firstBarIdentitySignature = (song, track) => song.playbackScore.events
  .filter((event) => event.role === "identity" && event.track === track && event.step < 16)
  .map((event) => `${event.step}:${event.durationSteps}:${event.notes.join(".")}`)
  .join("|");
for (const [carrier, track] of [["bass-riff", "bass"], ["harmony-arp", "chord"], ["rhythm-hook", "drums"]]) {
  const signatures = sampleSongs
    .filter((song) => song.debug.roles.identity.carrier === carrier)
    .map((song) => firstBarIdentitySignature(song, track))
    .filter(Boolean);
  const counts = new Map();
  for (const signature of signatures) {
    counts.set(signature, (counts.get(signature) || 0) + 1);
  }
  assert.ok(signatures.length >= 20, `${carrier} should appear often enough for distribution checks`);
  assert.ok(counts.size / signatures.length >= 0.8, `${carrier} should be generated from a stochastic field instead of a small pattern table`);
  assert.ok(Math.max(...counts.values()) / signatures.length < 0.16, `${carrier} should not be dominated by one hand-authored first-bar pattern`);
}

for (const role of ["tone", "color"]) {
  const signatures = sampleSongs
    .map((song) => song.playbackScore.events
      .filter((event) => event.role === role && event.step < 16)
      .map((event) => `${event.step}:${event.track}`)
      .join("|"))
    .filter(Boolean);
  const counts = new Map();
  for (const signature of signatures) {
    counts.set(signature, (counts.get(signature) || 0) + 1);
  }
  assert.ok(signatures.length >= 40, `${role} should appear often enough for placement distribution checks`);
  assert.ok(counts.size >= 12, `${role} placement should not collapse to a few fixed steps`);
  assert.ok(Math.max(...counts.values()) / signatures.length < 0.18, `${role} placement should not be dominated by one fixed first-bar step pattern`);
}

const barRhythmSignature = (song, bar, role) => song.playbackScore.events
  .filter((event) => event.role === role && event.step >= bar * 16 && event.step < (bar + 1) * 16)
  .map((event) => `${event.track}:${event.step - bar * 16}`)
  .join("|");
assert.equal(
  sampleSongs.filter((song) => [0, 1, 2, 3].map((bar) => barRhythmSignature(song, bar, "time")).every((signature, index, signatures) => signature && signature === signatures[0])).length,
  0,
  "time layer should not repeat one bar rhythm verbatim for the first four bars",
);
assert.equal(
  sampleSongs.filter((song) => [0, 1, 2, 3].every((bar) => {
    const steps = song.playbackScore.events
      .filter((event) => event.role === "time" && event.step >= bar * 16 && event.step < (bar + 1) * 16)
      .map((event) => event.step - bar * 16);
    return steps.length >= 4 && steps.every((step) => [0, 4, 8, 12].includes(step));
  })).length,
  0,
  "time layer should not devolve into four bars of straight quarter-grid pulses",
);

const directionChangeStats = (notes) => {
  const directions = [];
  for (let index = 1; index < notes.length; index += 1) {
    const direction = Math.sign(notes[index] - notes[index - 1]);
    if (direction) {
      directions.push(direction);
    }
  }
  let changes = 0;
  for (let index = 1; index < directions.length; index += 1) {
    if (directions[index] !== directions[index - 1]) {
      changes += 1;
    }
  }
  return { changes, directions: directions.length };
};
let checkedPitchGestures = 0;
let fullyAlternatingGestures = 0;
for (const song of sampleSongs) {
  for (let bar = 0; bar < 8; bar += 1) {
    const groupedEvents = new Map();
    for (const event of song.playbackScore.events.filter((candidate) => candidate.step >= bar * 16 && candidate.step < (bar + 1) * 16 && typeof candidate.notes[0] === "number")) {
      const key = `${event.role}:${event.track}`;
      groupedEvents.set(key, [...(groupedEvents.get(key) ?? []), event]);
    }
    for (const events of groupedEvents.values()) {
      const notes = events
        .sort((left, right) => left.step - right.step)
        .map((event) => event.notes[0]);
      if (notes.length < 4) {
        continue;
      }
      const { changes, directions } = directionChangeStats(notes);
      checkedPitchGestures += 1;
      if (directions >= 3 && changes >= directions - 1) {
        fullyAlternatingGestures += 1;
      }
    }
  }
}
assert.ok(checkedPitchGestures > 160, "sample should include enough intra-bar pitch gestures");
assert.ok(
  fullyAlternatingGestures / checkedPitchGestures < 0.34,
  "intra-bar pitch fields should not mostly collapse into up-down-up-down zigzags",
);

const melodicRhythmProfiles = new Set(sampleSongs
  .filter((song) => song.debug.roles.identity.carrier === "melodic-line")
  .map((song) => song.playbackScore.events
    .filter((event) => event.role === "identity" && event.track === "lead" && event.step < 16)
    .map((event) => `${event.step}:${event.durationSteps}`)
    .join("|")));
assert.ok(melodicRhythmProfiles.size >= 6, "melodic identity should draw from varied rhythm profiles across seeds");

const melodicLeadLines = sampleSongs
  .filter((song) => song.debug.roles.identity.carrier === "melodic-line")
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

const stateSignatures = new Set(sampleSongs.flatMap((song) => song.debug.sectionPlan.map((section) => `${Math.round(section.novelty * 10)}:${Math.round(section.stability * 10)}:${Math.round(section.density * 10)}:${Math.round(section.tension * 10)}:${Math.round(section.closurePressure * 10)}:${Math.round(section.memoryDistance * 10)}`)));
assert.ok(stateSignatures.size >= 80, "form trajectory should expose a broad stochastic state-vector space");
assert.ok(
  sampleSongs.filter((song) => song.debug.sectionPlan.slice(4).some((section) => section.memoryDistance >= 0.5 || section.tension >= 0.65)).length >= 100,
  "64-bar songs should usually recontextualize the second half with higher memory distance or tension",
);
assert.ok(
  new Set(sampleSongs.map((song) => song.debug.sectionPlan.map((section) => `${Math.round(section.novelty * 10)}.${Math.round(section.tension * 10)}.${Math.round(section.closurePressure * 10)}.${Math.round(section.memoryDistance * 10)}`).join(">"))).size >= 120,
  "form trajectories should be stochastic state sequences instead of a small named-form table",
);
for (const song of sampleSongs) {
  assert.ok(song.debug.sectionPlan[0].novelty <= 0.14 && song.debug.sectionPlan[0].memoryDistance <= 0.12 && song.debug.sectionPlan[0].stability >= 0.72, "form trajectory should start near a stable loop identity");
  assert.ok(
    song.debug.sectionPlan.slice(4).some((section) => section.memoryDistance >= 0.5 || section.closurePressure >= 0.68 || section.density <= 0.32),
    "64-bar form should give the second half a state change or wrap pressure",
  );
  for (const section of song.debug.sectionPlan) {
    if (!section.loopHandoff && section.progress >= 0.18 && (section.memoryDistance >= 0.5 || section.novelty >= 0.58)) {
      assert.notDeepEqual(section.roles, song.debug.roles, "high-distance or high-novelty states should expose section-local role changes");
    }
  }
}

const melodySong = generateSong("00melody-2", { height: 0.5, bpm: 110, bars: 64 });
assert.equal(melodySong.debug.roles.identity.carrier, "melodic-line", "fixture should exercise melodic identity");
const melodicSectionIndexes = melodySong.debug.sectionPlan
  .map((section, index) => ({ index, carrier: section.roles.identity.carrier }))
  .filter((section) => section.carrier === "melodic-line")
  .map((section) => section.index);
assert.ok(melodicSectionIndexes.length >= 2, "fixture should keep melodic identity active in multiple sections");
const leadIdentitySignature = (song, sectionIndex) => {
  const sectionStart = sectionIndex * 8 * 16;
  const sectionEnd = sectionStart + 8 * 16;
  return song.playbackScore.events
    .filter((event) => event.role === "identity" && event.track === "lead" && event.step >= sectionStart && event.step < sectionEnd)
    .map((event) => `${event.step - sectionStart}:${event.durationSteps}`)
    .join("|");
};
assert.notEqual(
  leadIdentitySignature(melodySong, melodicSectionIndexes[0]),
  leadIdentitySignature(melodySong, melodicSectionIndexes[1]),
  "melodic identity should change rhythm profile across active melodic sections",
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
  "melodic identity should develop within a four-bar phrase instead of repeating every bar exactly",
);
assert.equal(
  new Set(melodicSectionIndexes.slice(0, 3).map((sectionIndex) => leadIdentityRhythmSignature(melodySong, sectionIndex))).size,
  Math.min(3, melodicSectionIndexes.length),
  "active melodic sections should have distinct rhythm profiles",
);

const rhythmHookSongs = sampleSongs.filter((song) => song.debug.roles.identity.carrier === "rhythm-hook");
assert.ok(rhythmHookSongs.length > 0, "sample should include rhythm-hook identity cases");
for (const song of rhythmHookSongs) {
  const activeRhythmBars = new Set();
  for (const [index, section] of song.debug.sectionPlan.entries()) {
    const carrier = section.roles.identity.carrier;
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
    song.debug.roles.time.carrier !== "drum-grid",
    "rhythm-hook identity should not compete with drum-grid time by default",
  );
}

const preset = randomPreset("music-preset");
assert.ok(preset.seed);
assert.equal(preset.height, 0.5);
assert.equal(preset.focus, 0.5);
assert.equal(preset.brightness, 0.5);
assert.equal(preset.presence, 0.5);
assert.equal(preset.attack, 0.5);
assert.equal(preset.tone, 0.5);
assert.equal(preset.bpm, 110);

const higher = generateSong("knob-check", { height: 1, bpm: 110 });
const lower = generateSong("knob-check", { height: 0, bpm: 110 });
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

console.log("seeded_music tests passed");
