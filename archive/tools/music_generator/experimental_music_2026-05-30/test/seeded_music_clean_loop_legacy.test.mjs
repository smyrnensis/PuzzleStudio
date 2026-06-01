import assert from "node:assert/strict";
import { generateSong, midiToFrequency, randomPreset } from "../seeded_music.mjs";

const first = generateSong("same-seed", { tone: 0.62, bpm: 104 });
const second = generateSong("same-seed", { tone: 0.62, bpm: 104 });
const different = generateSong("different-seed", { tone: 0.62, bpm: 104 });

assert.deepEqual(first, second, "same seed and controls should produce the same song");
assert.notDeepEqual(first, different, "different seeds should usually produce different songs");
assert.equal(first.playbackScore.transport.bars, 32);
assert.equal(first.debug.roles.length, 4);
assert.equal(first.playbackScore.mix.volume, 0.5);
assert.ok(first.playbackScore.events.length > 0);
assert.equal(first.playbackScore.version, 1);
assert.equal(first.playbackScore.source.seed, "same-seed");
assert.equal(first.playbackScore.source.tone, 0.62);

for (const event of first.playbackScore.events) {
  assert.ok(event.step >= 0);
  assert.ok(event.step < first.playbackScore.transport.loopSteps);
  assert.ok(event.durationSteps > 0);
  assert.ok(first.playbackScore.timbres[event.timbre], "event timbre should resolve in playback score");
  assert.equal(Object.prototype.hasOwnProperty.call(event, "instrument"), false, "playback events should not depend on generator instrument fields");
  for (const note of event.notes) {
    if (typeof note === "number") {
      assert.ok(Number.isFinite(midiToFrequency(note)));
    }
  }
}

const sampleSongs = Array.from({ length: 300 }, (_, index) => generateSong(`simple-${index}`, { tone: 0.62, bpm: 104 }));
assert.ok(new Set(sampleSongs.map((song) => song.debug.type)).size >= 8, "seed should vary arrangement output");
assert.ok(new Set(sampleSongs.map((song) => song.debug.score.arrangement.focus)).size >= 5, "seed should cover multiple arrangement focuses");
assert.ok(new Set(sampleSongs.map((song) => JSON.stringify(song.debug.grammar.parameters.arrangement))).size >= 260, "seed should vary numeric arrangement parameters");
assert.ok(new Set(sampleSongs.map((song) => song.debug.form)).size >= 5, "seed should vary form shape");
assert.ok(sampleSongs.some((song) => song.debug.roles.filter((role) => role === "space" || role === "intro").length >= 2), "seed should allow 00AA-like intro/space forms");
assert.ok(sampleSongs.some((song) => new Set(song.debug.roles).size >= 4), "seed should allow ABCD-like four-role forms");
assert.ok(new Set(sampleSongs.map((song) => song.debug.melodyForm)).size >= 4, "seed should vary generated phrase grammar");
assert.ok(new Set(sampleSongs.map((song) => JSON.stringify(song.debug.score.phrase.contours[0]))).size >= 24, "seed should generate varied phrase contours");
assert.ok(new Set(sampleSongs.map((song) => JSON.stringify(song.debug.score.phrase.restBars))).size >= 4, "seed should vary phrase spacing");
assert.ok(new Set(sampleSongs.map((song) => song.debug.instruments.melody)).size >= 8, "seed should vary melody instruments");
assert.ok(
  sampleSongs.some((song) => song.debug.instruments.melody === "breathy-flute")
    && sampleSongs.some((song) => song.debug.instruments.melody === "marimba")
    && sampleSongs.some((song) => song.debug.instruments.melody === "harp"),
  "melody should include distinct acoustic-leaning timbres",
);
assert.ok(
  sampleSongs.some((song) => song.debug.instruments.melody === "chip-lead")
    && sampleSongs.some((song) => song.debug.instruments.melody === "triangle-lead")
    && sampleSongs.some((song) => song.debug.instruments.melody === "saw-lead"),
  "melody should include distinct synth and chiptune timbres",
);
assert.ok(sampleSongs.some((song) => song.debug.grammar.parameters.phrase.phraseCount >= 3), "seed should sometimes create more than two phrase variants");
assert.ok(sampleSongs.some((song) => new Set(song.debug.score.form.sections.map((section) => section.phrase)).size >= 3), "seed should sometimes create larger section-level phrase changes");
assert.ok(new Set(sampleSongs.map((song) => song.debug.grammar.parameters.phrase.peakBar)).size >= 4, "seed should vary phrase peak position");
assert.ok(new Set(sampleSongs.map((song) => song.debug.score.phrase.finalCadenceDegree)).size >= 2, "seed should vary final phrase landing");
assert.ok(sampleSongs.every((song) => {
  const finalLanding = song.debug.grammar.parameters.phrase.finalLanding;
  const expected = finalLanding < 0.64 ? 0 : finalLanding < 0.86 ? 1 : 2;
  return song.debug.score.phrase.finalCadenceDegree === expected;
}), "final landing should derive from visible phrase parameter");
assert.deepEqual([...new Set(sampleSongs.map((song) => song.debug.grammar.parameters.hook.lengthBars))].sort((a, b) => a - b), [2, 4, 8], "seed should vary lead phrase length");
assert.ok(new Set(sampleSongs.map((song) => song.debug.grammar.parameters.hook.startBar)).size >= 3, "seed should vary lead entry timing");
assert.ok(sampleSongs.filter((song) => song.debug.grammar.parameters.hook.startBar === 0).length > sampleSongs.length * 0.5, "lead should usually start at the beginning");
assert.ok(sampleSongs.every((song) => {
  const lengthSignal = song.debug.grammar.parameters.hook.lengthSignal;
  const expected = lengthSignal < 0.34 ? 2 : lengthSignal < 0.82 ? 4 : 8;
  return song.debug.grammar.parameters.hook.lengthBars === expected;
}), "lead phrase length should derive from visible hook parameter");
assert.ok(sampleSongs.every((song) => {
  const introSpace = song.debug.grammar.parameters.hook.introSpace;
  const expected = introSpace < 0.72 ? 0 : introSpace < 0.84 ? 1 : introSpace < 0.94 ? 2 : 4;
  return song.debug.grammar.parameters.hook.startBar === expected;
}), "lead entry timing should derive from visible hook parameter");
assert.ok(sampleSongs.every((song) => {
  const openingSpace = song.debug.grammar.parameters.form.openingSpace;
  const expected = openingSpace < 0.7 ? 0 : openingSpace < 0.9 ? 4 : 8;
  return song.debug.score.form.sections[0].entryDelayBars === expected;
}), "form opening space should derive from visible form parameter");
assert.ok(new Set(sampleSongs.map((song) => song.debug.score.hook.barNoteCounts.join(","))).size >= 120, "seed should vary lead density across bars");
assert.ok(new Set(sampleSongs.map((song) => JSON.stringify(song.debug.grammar.parameters.hook))).size >= 260, "seed should vary hook parameters");
assert.ok(new Set(sampleSongs.map((song) => `${song.debug.score.hook.steps.join(",")}|${song.debug.score.hook.degrees.join(",")}`)).size >= 260, "seed should vary hook score");

const leadEvents = (song) => song.playbackScore.events.filter((event) => event.track === "lead");
const leadForward = sampleSongs.find((song) => song.debug.score.arrangement.leadPresence > 0.7 && leadEvents(song).length > 40);
assert.ok(leadForward, "numeric lead presence should sometimes foreground a longer lead phrase");
assert.ok(leadForward.playbackScore.events.filter((event) => event.track === "chord").length <= 32, "high lead presence should keep harmony simple");
assert.ok(
  new Set(leadEvents(leadForward).map((event) => Math.floor(event.step / 128))).size >= 3,
  "foreground lead should span multiple eight-bar sections",
);
const hookRepeat = sampleSongs.find((song) => {
  const events = leadEvents(song);
  const lengthBars = song.debug.score.hook.lengthBars;
  if (lengthBars >= 8) {
    return false;
  }
  const firstCycle = events.filter((event) => Math.floor(event.step / song.playbackScore.transport.stepsPerBar) < lengthBars).map((event) => `${Math.floor(event.step / song.playbackScore.transport.stepsPerBar)}:${event.step % song.playbackScore.transport.stepsPerBar}`).join("|");
  const secondCycle = events.filter((event) => Math.floor(event.step / song.playbackScore.transport.stepsPerBar) >= lengthBars && Math.floor(event.step / song.playbackScore.transport.stepsPerBar) < lengthBars * 2).map((event) => `${Math.floor(event.step / song.playbackScore.transport.stepsPerBar) - lengthBars}:${event.step % song.playbackScore.transport.stepsPerBar}`).join("|");
  return firstCycle && secondCycle;
});
assert.ok(hookRepeat, "seed should create repeated lead phrase placement");
assert.ok(
  new Set(sampleSongs.filter((song) => song.debug.score.arrangement.leadPresence > 0.58).map((song) => (
    leadEvents(song).slice(0, 48).map((event) => event.step % song.playbackScore.transport.stepsPerBar).join(",")
  ))).size >= 40,
  "foreground lead should vary rhythm shapes across seeds",
);
assert.ok(
  sampleSongs.some((song) => song.debug.grammar.parameters.phrase.runAmount > 0.5 && leadEvents(song).some((event, _, events) => (
    events.some((other) => Math.floor(other.step / song.playbackScore.transport.stepsPerBar) === Math.floor(event.step / song.playbackScore.transport.stepsPerBar) && Math.abs((other.step % song.playbackScore.transport.stepsPerBar) - (event.step % song.playbackScore.transport.stepsPerBar)) === 1)
  ))),
  "seed should sometimes create close-step lead runs",
);

assert.ok(sampleSongs.some((song) => song.debug.score.arrangement.drums === "full"), "numeric arrangement should sometimes make drums denser");
assert.ok(sampleSongs.some((song) => song.debug.score.arrangement.drums === "light"), "numeric arrangement should sometimes keep drums sparse");
assert.ok(
  new Set(sampleSongs.map((song) => JSON.stringify(song.debug.grammar.parameters.drums))).size >= 260,
  "seed should vary numeric drum parameters instead of selecting from a small hand-authored kit list",
);
assert.ok(
  new Set(sampleSongs.map((song) => JSON.stringify(song.debug.score.drums))).size >= 120,
  "seed should vary generated drum hit placement",
);
assert.ok(
  new Set(sampleSongs.map((song) => JSON.stringify(song.playbackScore.timbres.kick))).size >= 120,
  "seed should vary kick timbre parameters",
);
assert.ok(
  sampleSongs.every((song) => ["kick", "snare", "hat"].every((timbre) => typeof song.playbackScore.timbres[timbre] === "object")),
  "drum timbres should expose accountable playback parameters",
);

assert.ok(sampleSongs.some((song) => song.debug.score.arrangement.leadPresence < 0.12 && leadEvents(song).length === 0), "low lead presence should sometimes remove lead events");
assert.ok(sampleSongs.some((song) => song.debug.score.arrangement.leadPresence > 0.7 && leadEvents(song).length > 40), "high lead presence should create clear lead events");
assert.ok(sampleSongs.some((song) => song.debug.score.arrangement.counter), "numeric arrangement should sometimes add a counter line");
assert.ok(
  sampleSongs.some((song) => Object.keys(song.debug.score.arrangement.droppedParts).length > 0),
  "low-presence parts should sometimes be explicitly dropped instead of left as barely audible background",
);
assert.ok(
  sampleSongs.every((song) => ["lead", "harmony", "bass", "drums", "counter"].every((part) => typeof song.debug.score.arrangement.partPresence[part] === "number")),
  "arrangement should expose part presence decisions",
);

const textureForward = sampleSongs.find((song) => song.playbackScore.events.filter((event) => event.track === "chord").length > leadForward.playbackScore.events.filter((event) => event.track === "chord").length);
assert.ok(textureForward, "numeric arrangement should sometimes lean on harmony texture");
const arpSongs = sampleSongs.filter((song) => song.debug.score.arrangement.harmony === "arp");
assert.ok(arpSongs.length > 0, "seed should sometimes create arpeggiated harmony");
assert.ok(
  new Set(arpSongs.map((song) => song.playbackScore.events.filter((event) => event.track === "chord").slice(0, 32).map((event) => event.step % song.playbackScore.transport.stepsPerBar).join(","))).size >= 8,
  "arpeggiated harmony should vary timing across seeds",
);
assert.ok(
  !arpSongs.every((song) => song.playbackScore.events.filter((event) => event.track === "chord").slice(0, 16).map((event) => event.step % song.playbackScore.transport.stepsPerBar).join(",").includes("0,4,8,12")),
  "arpeggiated harmony should not always use a fixed four-beat pulse",
);

const dark = generateSong("axis", { tone: 0.05, bpm: 104 });
const bright = generateSong("axis", { tone: 0.95, bpm: 104 });
const quiet = generateSong("axis", { tone: 0.95, bpm: 104, volume: 0.25 });
const loud = generateSong("axis", { tone: 0.95, bpm: 104, volume: 1 });
assert.equal(dark.debug.type, bright.debug.type, "tone should not change arrangement output");
assert.deepEqual(dark.debug.grammar.parameters.arrangement, bright.debug.grammar.parameters.arrangement, "tone should not change arrangement parameters");
assert.equal(dark.debug.melodyForm, bright.debug.melodyForm, "tone should not change melody form");
assert.deepEqual(dark.debug.grammar.parameters.phrase, bright.debug.grammar.parameters.phrase, "tone should not change phrase parameters");
assert.deepEqual(dark.debug.score.phrase, bright.debug.score.phrase, "tone should not change phrase score");
assert.equal(dark.debug.form, bright.debug.form, "tone should not change form shape");
assert.deepEqual(dark.debug.roles, bright.debug.roles, "tone should not change role sequence");
assert.notDeepEqual(dark.playbackScore.mix.playbackTone, bright.playbackScore.mix.playbackTone, "tone should continuously change playback tone");
assert.ok(dark.playbackScore.mix.playbackTone.toneFilter < bright.playbackScore.mix.playbackTone.toneFilter, "brighter tone should open the tone filter");
assert.ok(dark.playbackScore.mix.playbackTone.bassGain > bright.playbackScore.mix.playbackTone.bassGain, "darker tone should lean more on bass gain");
assert.ok(dark.debug.score.scale.weights["3"] > bright.debug.score.scale.weights["3"], "darker tone should favor minor third");
assert.ok(dark.debug.score.scale.weights["4"] < bright.debug.score.scale.weights["4"], "brighter tone should favor major third");
assert.equal(dark.debug.score.scale.degrees.length, 7);
assert.equal(bright.debug.score.scale.degrees.length, 7);
assert.equal(quiet.playbackScore.mix.volume, 0.25);
assert.equal(loud.playbackScore.mix.volume, 1);
assert.deepEqual(quiet.playbackScore.events, loud.playbackScore.events, "volume should not change generated note events");

const toneRamp = Array.from({ length: 11 }, (_, index) => generateSong("axis", { tone: index / 10, bpm: 104 }));
assert.ok(new Set(toneRamp.map((song) => JSON.stringify(song.debug.progression))).size >= 2, "tone should affect progression selection over a continuous ramp");

const preset = randomPreset("preset");
assert.ok(preset.seed);
assert.ok(preset.tone >= 0 && preset.tone <= 1);
assert.ok(preset.bpm >= 40 && preset.bpm <= 180);

console.log("simple seeded_music tests passed");
