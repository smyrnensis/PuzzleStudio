import assert from "node:assert/strict";
import { generateSong as generateCurrent } from "../seeded_music.mjs";
import { auditionInstruments, buildInstrumentAuditionScore, generateSong as generateLab, midiToFrequency, randomPreset } from "../seeded_music_lab.mjs";

const first = generateLab("same-seed", { tone: 0.5, bpm: 110 });
const second = generateLab("same-seed", { tone: 0.5, bpm: 110 });
const different = generateLab("different-seed", { tone: 0.5, bpm: 110 });

assert.deepEqual(first, second, "lab generator should stay deterministic");
assert.notDeepEqual(first, different, "different seeds should usually produce different lab songs");
assert.equal(first.playbackScore.transport.bars, 32);
assert.equal(first.playbackScore.version, 1);
assert.equal(first.playbackScore.source.seed, "same-seed");

for (const event of first.playbackScore.events) {
  assert.ok(first.playbackScore.timbres[event.timbre], "event timbre should resolve in playback score");
  for (const note of event.notes) {
    if (typeof note === "number") {
      assert.ok(Number.isFinite(midiToFrequency(note)));
    }
  }
}

const sampleSeeds = Array.from({ length: 420 }, (_, index) => `lab-${index}`);
const sampleSongs = sampleSeeds.map((seed) => generateLab(seed, { tone: 0.5, bpm: 110 }));
const currentSongs = sampleSeeds.map((seed) => generateCurrent(seed, { tone: 0.5, bpm: 110 }));

const leadNotes = (song) => song.playbackScore.events
  .filter((event) => event.track === "lead")
  .flatMap((event) => event.notes)
  .filter((note) => typeof note === "number");

const labLeadNotes = sampleSongs.flatMap(leadNotes);
const currentLeadNotes = currentSongs.flatMap(leadNotes);

assert.ok(labLeadNotes.length > 1000, "sample should include enough lab lead notes for distribution checks");
assert.ok(percentile(labLeadNotes, 0.95) <= 86, "lab lead should keep most melody notes out of the piercing upper register");
assert.ok(
  percentile(labLeadNotes, 0.95) <= percentile(currentLeadNotes, 0.95) - 8,
  "lab lead should materially lower the high-note tail versus the current generator",
);
assert.ok(Math.max(...labLeadNotes) <= 88, "lab lead should clamp even bright instruments below the old extreme range");

const melodyInstruments = sampleSongs.map((song) => song.debug.instruments.melody);
const instrumentSet = new Set(melodyInstruments);
for (const instrument of ["warm-pluck", "low-pluck", "fuzzy-pluck", "dust-lead"]) {
  assert.ok(instrumentSet.has(instrument), `lab melody should include ${instrument}`);
}

const abstractPlucks = new Set(["warm-pluck", "low-pluck", "fuzzy-pluck"]);
const abstractPluckRatio = melodyInstruments.filter((instrument) => abstractPlucks.has(instrument)).length / melodyInstruments.length;
assert.ok(abstractPluckRatio < 0.24, "lab melody should use abstract plucks as occasional color, not a dominant family");

const brightPercussive = new Set(["marimba", "music-box", "chip-lead", "triangle-lead"]);
const brightRatio = melodyInstruments.filter((instrument) => brightPercussive.has(instrument)).length / melodyInstruments.length;
assert.ok(brightRatio < 0.28, "lab melody should not be dominated by bright mallet/chiptune leads");

assert.ok(
  sampleSongs.some((song) => song.debug.score.hook.cycleRhythmShifts.some((shift) => shift !== 0)),
  "lab hooks should sometimes shift repeated-cycle timing",
);
assert.ok(
  sampleSongs.some((song) => song.debug.score.hook.cycleNoteDrops.some((drop) => drop > 0.12)),
  "lab hooks should sometimes thin repeated cycles",
);
assert.ok(
  sampleSongs.some((song) => song.debug.score.hook.phraseAnswerShifts.some((shift) => shift !== 0)),
  "lab hooks should sometimes vary repeated answers between phrase variants",
);
assert.ok(
  sampleSongs.every((song) => song.debug.score.hook.contrastMotif.lengthBars === 1),
  "lab hooks should expose a restrained one-bar contrast phrase for repeated material",
);
assert.ok(
  sampleSongs.every((song) => song.debug.score.hook.contrastMotif.bars.every((bar) => bar.length <= 3)),
  "contrast phrases should stay sparse enough not to take over the hook",
);
const contrastUsage = contrastUsageStats(sampleSongs);
assert.ok(contrastUsage.songsWith >= 90, "contrast phrases should appear often enough to be auditionable across random seeds");
assert.ok(contrastUsage.averageBarsPerSong < 0.9, "contrast phrases should remain rare within each song");

const counterRhythms = rhythmicSignatures(sampleSongs, "counter");
assert.ok(counterRhythms.unique >= 80, "middle counter phrases should vary rhythm across seeds");
assert.ok(counterRhythms.topCount / counterRhythms.nonEmpty < 0.08, "middle counter phrases should not collapse to one dominant tan-tan pattern");

assert.ok(sampleSongs.every((song) => typeof song.debug.grammar.parameters.harmony.pulseRegularity === "number"), "harmony arp should expose numeric grammar parameters");
const harmonyArps = harmonyArpStats(sampleSongs);
assert.ok(harmonyArps.arpSongs > 80, "sample should include enough harmony arp songs for distribution checks");
assert.ok(harmonyArps.uniqueSongSignatures / harmonyArps.arpSongs > 0.8, "harmony arp should be generated from numeric variation, not a small song-pattern list");
assert.ok(harmonyArps.uniqueBarPatterns >= 120, "harmony arp should produce varied bar-level ostinato patterns");
assert.ok(harmonyArps.topBarShare < 0.12, "harmony arp should not be dominated by a single tan-tan bar pattern");

const roleDistance = roleDistanceStats(sampleSongs);
assert.ok(roleDistance.leadP95 <= 82, "role-distance guardrail should keep lead notes out of piercing ranges");
assert.ok(roleDistance.shortChordP95 <= 84, "role-distance guardrail should keep short harmony notes in a middle-distance register");
assert.ok(roleDistance.denseBrightLeadDecisions >= 1, "role-distance guardrail should catch dense bright foreground leads");
assert.ok(roleDistance.harmonyArpDecisions >= 20, "role-distance guardrail should visibly account for high harmony arp cases");
assert.ok(roleDistance.explainableDecisions, "role-distance decisions should expose named rules with principles and effects");

const sustainedBassExample = generateLab("450758", { tone: 0.5, bpm: 110 });
assert.ok(
  sustainedBassExample.debug.mixPolicy.decisions.some((decision) => hasRoleRule(decision, "low-frequency-sustain-budget")),
  "role-distance guardrail should catch sustained low bass foreground pulses",
);
const brightLeadExample = generateLab("587755", { tone: 0.5, bpm: 110 });
assert.ok(
  brightLeadExample.debug.mixPolicy.decisions.some((decision) => hasRoleRule(decision, "bright-lead-occupancy-budget")),
  "role-distance guardrail should catch long bright lead occupancy",
);
assert.ok(
  brightLeadExample.debug.mixPolicy.decisions.some((decision) => hasRoleRule(decision, "texture-focus-bright-lead-distance")),
  "role-distance guardrail should keep bright leads inside texture-focused arrangements",
);
const leadDominanceExample = generateLab("314798", { tone: 0.5, bpm: 110 });
assert.ok(
  leadDominanceExample.debug.mixPolicy.decisions.some((decision) => hasRoleRule(decision, "nonfocus-lead-dominance")),
  "role-distance guardrail should catch lead dominance outside lead-focused arrangements",
);

const labCycleVariety = averageRepeatedCycleVariety(sampleSongs);
const currentCycleVariety = averageRepeatedCycleVariety(currentSongs);
assert.ok(
  labCycleVariety > currentCycleVariety,
  "lab lead cycles should vary repeated material more than the current generator",
);

const preset = randomPreset("lab-preset");
assert.ok(preset.seed);
assert.equal(preset.tone, 0.5);
assert.equal(preset.bpm, 110);

const auditionItems = auditionInstruments();
assert.ok(auditionItems.some((item) => item.instrument === "chip-lead"), "audition list should include chip-lead");
assert.ok(auditionItems.some((item) => item.group === "drums" && item.instrument === "snare"), "audition list should include drum timbres");
const functionalAuditionItems = auditionItems.filter((item) => item.group === "functional");
assert.ok(functionalAuditionItems.length >= 8, "audition list should include a functional playback palette");
assert.ok(
  new Set(functionalAuditionItems.map((item) => item.timbre.family)).size >= 6,
  "functional playback palette should expose independent source families",
);
assert.ok(
  new Set(functionalAuditionItems.map((item) => item.timbre.engine)).size >= 8,
  "functional playback palette should not collapse to one synthesis engine",
);
assert.ok(
  functionalAuditionItems.every((item) => item.timbre.gain > 0 && item.timbre.gain <= 1),
  "functional playback palette should use bounded intrinsic gains",
);
const auditionScore = buildInstrumentAuditionScore({ tone: 0.5, bpm: 110, volume: 0.5 });
assert.equal(auditionScore.events.length, auditionItems.length);
for (const event of auditionScore.events) {
  assert.ok(auditionScore.timbres[event.timbre], "audition event timbre should resolve in playback score");
}
const chipAudition = buildInstrumentAuditionScore({ instruments: ["melody:chip-lead"], tone: 0.5, bpm: 110 });
assert.equal(chipAudition.events.length, 1);
assert.equal(chipAudition.timbres["melody:chip-lead"].kind, "chip-lead");
assert.equal(chipAudition.timbres["melody:chip-lead"].gain, 0.65);

function percentile(values, amount) {
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.min(sorted.length - 1, Math.max(0, Math.floor(sorted.length * amount)));
  return sorted[index];
}

function averageRepeatedCycleVariety(songs) {
  const scores = songs
    .map((song) => {
      const lengthBars = song.debug.score.hook.lengthBars;
      if (lengthBars >= 8) {
        return null;
      }
      const signatures = [];
      for (let bar = song.debug.score.hook.startBar; bar + lengthBars <= 32; bar += lengthBars) {
        const start = bar * song.playbackScore.transport.stepsPerBar;
        const end = start + lengthBars * song.playbackScore.transport.stepsPerBar;
        const events = song.playbackScore.events
          .filter((event) => event.track === "lead" && event.step >= start && event.step < end)
          .map((event) => `${(event.step - start) % (lengthBars * 16)}:${event.notes.join(",")}:${event.durationSteps}`)
          .join("|");
        if (events) {
          signatures.push(events);
        }
      }
      if (signatures.length < 2) {
        return null;
      }
      return new Set(signatures).size / signatures.length;
    })
    .filter((score) => score !== null);
  assert.ok(scores.length > 20, "sample should include enough repeated hook cycles");
  return scores.reduce((sum, score) => sum + score, 0) / scores.length;
}

function contrastUsageStats(songs) {
  let songsWith = 0;
  let totalBars = 0;
  for (const song of songs) {
    const hook = song.debug.score.hook;
    const type = song.debug.score.arrangement;
    let songBars = 0;
    for (let bar = hook.startBar; bar < 32; bar += 1) {
      const section = song.debug.score.form.sections[Math.floor(bar / 8)];
      const leadLocalBar = (bar - hook.startBar) % 8;
      const cycle = Math.floor(leadLocalBar / hook.lengthBars);
      const cycleBar = leadLocalBar % hook.lengthBars;
      const contrast = hook.contrastMotif;
      if (!contrast || cycleBar < contrast.startBar || cycleBar >= contrast.startBar + contrast.lengthBars) {
        continue;
      }
      const phraseHasMoved = section.phrase > 0 || section.intensity > 0.72 || Math.abs(section.transpose) >= 2;
      const uses = type.leadPresence >= 0.34
        && type.leadDensity <= 0.9
        && section.intensity >= 0.54
        && (hook.lengthBars < 8
          ? cycle === Math.max(1, Math.floor((8 / hook.lengthBars) * 0.5)) && phraseHasMoved
          : phraseHasMoved && section.intensity > 0.62);
      if (uses) {
        songBars += 1;
      }
    }
    if (songBars > 0) {
      songsWith += 1;
    }
    totalBars += songBars;
  }
  return {
    songsWith,
    averageBarsPerSong: totalBars / songs.length,
  };
}

function rhythmicSignatures(songs, track) {
  const signatures = songs
    .map((song) => song.playbackScore.events
      .filter((event) => event.track === track)
      .slice(0, 64)
      .map((event) => event.step % song.playbackScore.transport.stepsPerBar)
      .join(","))
    .filter(Boolean);
  const counts = new Map();
  for (const signature of signatures) {
    counts.set(signature, (counts.get(signature) ?? 0) + 1);
  }
  return {
    nonEmpty: signatures.length,
    unique: counts.size,
    topCount: Math.max(...counts.values()),
  };
}

function harmonyArpStats(songs) {
  const arpSongs = songs.filter((song) => (
    song.debug.score.arrangement.harmony === "arp"
      && song.playbackScore.events.some((event) => event.track === "chord")
  ));
  const songSignatures = [];
  const barPatterns = [];
  for (const song of arpSongs) {
    const chordEvents = song.playbackScore.events.filter((event) => event.track === "chord");
    songSignatures.push(chordEvents
      .slice(0, 80)
      .map((event) => `${event.step % song.playbackScore.transport.stepsPerBar}:${event.durationSteps}:${event.notes.length}`)
      .join("|"));
    for (let bar = 0; bar < song.playbackScore.transport.bars; bar += 1) {
      const pattern = chordEvents
        .filter((event) => Math.floor(event.step / song.playbackScore.transport.stepsPerBar) === bar)
        .map((event) => `${event.step % song.playbackScore.transport.stepsPerBar}:${event.durationSteps}:${event.notes.length}`)
        .join("|");
      if (pattern) {
        barPatterns.push(pattern);
      }
    }
  }
  const counts = new Map();
  for (const pattern of barPatterns) {
    counts.set(pattern, (counts.get(pattern) ?? 0) + 1);
  }
  return {
    arpSongs: arpSongs.length,
    uniqueSongSignatures: new Set(songSignatures).size,
    uniqueBarPatterns: counts.size,
    topBarShare: Math.max(...counts.values()) / barPatterns.length,
  };
}

function roleDistanceStats(songs) {
  const leadNotes = songs.flatMap((song) => leadNotesFor(song));
  const shortChordNotes = songs.flatMap((song) => song.playbackScore.events
    .filter((event) => event.track === "chord")
    .filter((event) => event.durationSteps <= 3)
    .flatMap((event) => event.notes)
    .filter((note) => typeof note === "number"));
  const decisions = songs.flatMap((song) => song.debug.mixPolicy.decisions);
  return {
    leadP95: percentile(leadNotes, 0.95),
    shortChordP95: percentile(shortChordNotes, 0.95),
    denseBrightLeadDecisions: songs.filter((song) => song.debug.mixPolicy.decisions.some((decision) => hasRoleRule(decision, "bright-lead-too-close"))).length,
    harmonyArpDecisions: songs.filter((song) => song.debug.mixPolicy.decisions.some((decision) => hasRoleRule(decision, "middle-distance-harmony-arp") || hasRoleRule(decision, "high-harmony-arp"))).length,
    explainableDecisions: decisions.length > 0 && decisions.every((decision) => (
      decision.principle
      && Array.isArray(decision.rules)
      && decision.rules.length > 0
      && decision.rules.every((rule) => rule.id && rule.principle && rule.effect)
    )),
  };
}

function hasRoleRule(decision, id) {
  return decision.rules.some((rule) => rule.id === id);
}

function leadNotesFor(song) {
  return song.playbackScore.events
    .filter((event) => event.track === "lead")
    .flatMap((event) => event.notes)
    .filter((note) => typeof note === "number");
}

console.log("seeded_music_lab tests passed");
