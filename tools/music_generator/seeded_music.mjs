const KEYS = [
  ["C", 60],
  ["D", 62],
  ["E", 64],
  ["F", 65],
  ["G", 67],
  ["A", 69],
  ["Bb", 70],
];

const PROGRESSIONS = {
  dark: [
    [0, 5, 3, 4],
    [0, 2, 5, 3],
    [3, 0, 5, 2],
  ],
  neutral: [
    [0, 3, 4, 0],
    [5, 3, 0, 4],
    [0, 5, 3, 4],
  ],
  bright: [
    [0, 4, 5, 3],
    [0, 5, 3, 4],
    [0, 1, 4, 5],
  ],
};

const INSTRUMENTS = {
  melody: ["breathy-flute", "nylon", "harp", "marimba", "music-box", "chip-lead", "triangle-lead", "saw-lead"],
  counter: ["glass", "pluck"],
  harmony: ["pad", "organ", "muted-pluck"],
  bass: ["round-bass", "wood-bass"],
};

const MIN_BPM = 40;
const MAX_BPM = 180;
const DEFAULT_BPM = 104;
const DEFAULT_VOLUME = 0.5;

export function generateSong(seed, options = {}) {
  const seedText = String(seed);
  const rng = mulberry32(hashSeed(seedText));
  const tone = clamp(Number(options.tone ?? 0.62), 0, 1);
  const playbackTone = playbackToneFor(tone);
  const bpm = clamp(Math.round(Number(options.bpm ?? DEFAULT_BPM)), MIN_BPM, MAX_BPM);
  const volume = clamp(Number(options.volume ?? DEFAULT_VOLUME), 0, 1);
  const scale = buildToneScale(seedText, tone);
  const scaleDegrees = scale.degrees;
  const [key, tonic] = pick(KEYS, rng);
  const generated = generateGrammar(rng);
  const grammar = generated.grammar;
  const parameters = grammar.parameters;
  const form = generated.form;
  const type = arrangementType(parameters.arrangement);
  const progression = rotate(pickProgression(rng, tone), randomInt(rng, 0, 3));
  const melodyForm = buildMelodyForm(rng, scaleDegrees.length, generated.phrase, generated.hook);
  grammar.score.hook = melodyForm.hook.score;
  const instruments = {
    melody: pick(INSTRUMENTS.melody, rng),
    counter: pick(INSTRUMENTS.counter, rng),
    harmony: pick(INSTRUMENTS.harmony, rng),
    bass: pick(INSTRUMENTS.bass, rng),
  };
  const bars = 32;
  const stepsPerBar = 16;
  const stepDurationBeats = 0.25;
  const events = [];

  for (let bar = 0; bar < bars; bar += 1) {
    const section = form.sections[Math.floor(bar / 8)];
    const localBar = bar % 8;
    const barStart = bar * stepsPerBar;
    const chordDegree = progression[bar % progression.length];
    const chord = buildTriad(tonic, scaleDegrees, chordDegree);

    addHarmony(events, barStart, chord, type, instruments, localBar);
    addBass(events, barStart, chord, type, instruments, localBar);
    addDrums(events, barStart, type, generated.drums, localBar);
    addMelody(events, barStart, tonic, scaleDegrees, chordDegree, type, melodyForm, instruments, section, bar, localBar, tone);
    addCounter(events, barStart, tonic, scaleDegrees, type, instruments, section, localBar, tone);
  }

  events.sort((a, b) => a.step - b.step || a.track.localeCompare(b.track));

  const progressionNumbers = progression.map((degree) => degree + 1);
  const score = {
    key,
    scale,
    arrangement: type,
    form: grammar.score.form,
    phrase: grammar.score.phrase,
    hook: grammar.score.hook,
    instruments: activeInstruments(type, instruments),
    drums: grammar.score.drums,
    progression: progressionNumbers,
  };
  const playbackScore = buildPlaybackScore({
    seed: seedText,
    tone,
    playbackTone: playbackTone,
    bpm,
    volume,
    bars,
    stepsPerBar,
    stepDurationBeats,
    instruments,
    drums: generated.drums,
    events,
  });

  return {
    input: {
      seed: seedText,
      tone,
      bpm,
      volume,
    },
    playbackScore,
    debug: {
      key,
      type: arrangementSignature(type),
      form: formSignature(form.sections),
      roles: form.roles,
      melodyForm: generated.phrase.summary,
      grammar,
      score,
      instruments,
      progression: progressionNumbers,
    },
  };
}

function buildPlaybackScore({ seed, tone, playbackTone, bpm, volume, bars, stepsPerBar, stepDurationBeats, instruments, drums, events }) {
  const timbres = {
    lead: { kind: instruments.melody },
    counter: { kind: instruments.counter },
    chord: { kind: instruments.harmony },
    bass: { kind: instruments.bass },
    kick: { kind: "kick", ...drums.timbres.kick },
    snare: { kind: "snare", ...drums.timbres.snare },
    hat: { kind: "hat", ...drums.timbres.hat },
  };
  return {
    version: 1,
    source: {
      seed,
      tone,
    },
    transport: {
      bpm,
      bars,
      stepsPerBar,
      stepDurationBeats,
      loopSteps: bars * stepsPerBar,
    },
    mix: {
      volume,
      playbackTone,
    },
    timbres,
    events: events.map((event) => ({
      track: event.track,
      step: event.step,
      durationSteps: event.durationSteps,
      notes: [...event.notes],
      timbre: timbreForEvent(event),
      velocity: event.velocity,
    })),
  };
}

function timbreForEvent(event) {
  if (event.track === "drums") {
    return event.instrument;
  }
  if (event.track === "lead") {
    return "lead";
  }
  return event.track;
}

export function randomPreset(seed = Date.now()) {
  const rng = mulberry32(hashSeed(String(seed)));
  return {
    seed: randomInt(rng, 100000, 999999).toString(),
    tone: round2(rng()),
    bpm: randomInt(rng, MIN_BPM, MAX_BPM),
  };
}

function generateGrammar(rng) {
  const arrangement = generateArrangement(rng);
  const formParameters = generateFormParameters(rng);
  const rawDensity = 0.18 + rng() * 0.82;
  const noteDensity = round2(arrangement.leadPresence > 0.66 ? Math.max(0.42, rawDensity) : rawDensity);
  const cadenceHoldSteps = randomInt(rng, 6, 10);
  const phrase = generatePhraseGrammar(rng, noteDensity, cadenceHoldSteps);
  const hook = generateHookGrammar(rng, noteDensity);
  const drums = generateDrumGrammar(rng);
  const form = generateForm(rng, formParameters, phrase.contours.length);
  return {
    grammar: {
      parameters: {
        arrangement,
        form: formParameters,
        phrase: {
          noteDensity,
          rhythmOpenness: phrase.rhythmOpenness,
          syncopation: phrase.syncopation,
          pulseRegularity: phrase.pulseRegularity,
          runAmount: phrase.runAmount,
          intervalLeap: phrase.intervalLeap,
          finalLanding: phrase.finalLanding,
          peakBar: phrase.peakBar,
          peakDegree: phrase.peakDegree,
          cadenceHoldSteps,
          phraseCount: phrase.contours.length,
        },
        hook: {
          lengthSignal: hook.lengthSignal,
          lengthBars: hook.lengthBars,
          introSpace: hook.introSpace,
          startBar: hook.startBar,
          noteCount: hook.noteCount,
          barNoteCounts: hook.barNoteCounts,
          repeatRate: hook.repeatRate,
          syncopation: hook.syncopation,
          range: hook.range,
          answerShift: hook.answerShift,
          restRate: hook.restRate,
          lift: hook.lift,
          densityContrast: hook.densityContrast,
          durationContrast: hook.durationContrast,
        },
        drums: drums.parameters,
      },
      score: {
        form: { sections: form.sections },
        phrase: {
          restBars: phrase.restBars,
          contours: phrase.contours,
          midCadenceDegrees: phrase.midCadenceDegrees,
          finalCadenceDegree: phrase.finalCadenceDegree,
        },
        drums: drums.score,
      },
    },
    form,
    phrase,
    hook,
    drums,
  };
}

function generateArrangement(rng) {
  return {
    leadPresence: round2(rng()),
    grooveAmount: round2(rng()),
    textureAmount: round2(rng()),
    counterAmount: round2(rng()),
    harmonyMotion: round2(rng()),
    harmonySpace: round2(rng()),
  };
}

function generateFormParameters(rng) {
  return {
    sectionChange: round2(rng()),
    phraseChange: round2(rng()),
    intensityMotion: round2(rng()),
    openingSpace: round2(rng()),
  };
}

function generateDrumGrammar(rng) {
  const parameters = {
    density: round2(rng()),
    syncopation: round2(rng()),
    ghostRate: round2(rng()),
    fillAmount: round2(rng()),
    hatTightness: round2(rng()),
    kickPitch: round2(rng()),
    kickDecay: round2(rng()),
    kickClick: round2(rng()),
    snareSnap: round2(rng()),
    snareBody: round2(rng()),
    snareDecay: round2(rng()),
    hatBrightness: round2(rng()),
    hatDecay: round2(rng()),
  };
  const score = {
    light: buildDrumCycle(rng, parameters, false),
    full: buildDrumCycle(rng, parameters, true),
  };
  return {
    parameters,
    score,
    timbres: {
      kick: {
        pitchStart: randomIntFromUnit(parameters.kickPitch, 42, 92),
        pitchEnd: randomIntFromUnit(1 - parameters.kickPitch, 30, 48),
        decay: round2(0.09 + parameters.kickDecay * 0.18),
        click: round2(0.04 + parameters.kickClick * 0.24),
      },
      snare: {
        tone: round2(0.08 + parameters.snareBody * 0.34),
        snap: round2(0.18 + parameters.snareSnap * 0.54),
        decay: round2(0.045 + parameters.snareDecay * 0.16),
        filter: randomIntFromUnit(parameters.snareSnap, 850, 3400),
      },
      hat: {
        brightness: round2(0.52 + parameters.hatBrightness * 0.88),
        decay: round2(0.018 + parameters.hatDecay * 0.082),
        filter: randomIntFromUnit(parameters.hatBrightness, 3600, 9000),
      },
    },
  };
}

function buildDrumCycle(rng, parameters, full) {
  return Array.from({ length: 8 }, (_, bar) => {
    const kick = buildKickHits(rng, parameters, full, bar);
    const snare = buildSnareHits(rng, parameters, full, bar);
    const hat = buildHatHits(rng, parameters, full, bar);
    return { kick, snare, hat };
  });
}

function buildKickHits(rng, parameters, full, bar) {
  const hits = [];
  if (full || bar % 2 === 0 || parameters.density > 0.62) {
    hits.push({ step: 0, velocity: full ? 0.3 : 0.16 });
  }
  for (const step of [3, 7, 10, 12, 14]) {
    const syncWeight = step === 3 || step === 10 || step === 14 ? parameters.syncopation : 0.48;
    const chance = (full ? 0.16 : 0.04) + parameters.density * 0.18 + syncWeight * 0.16;
    if (rng() < chance) {
      hits.push({ step, velocity: round2((full ? 0.18 : 0.11) + rng() * 0.13) });
    }
  }
  return uniqueHits(hits);
}

function buildSnareHits(rng, parameters, full, bar) {
  const hits = [];
  const backbeatShift = parameters.syncopation > 0.82 && bar % 4 === 2 ? 1 : 0;
  if (full) {
    hits.push({ step: 4 + (parameters.syncopation > 0.72 && bar % 2 === 1 ? -1 : 0), velocity: 0.18 + parameters.snareSnap * 0.07 });
  }
  hits.push({ step: 12 + backbeatShift, velocity: full ? 0.2 + parameters.snareSnap * 0.06 : 0.13 + parameters.snareSnap * 0.05 });
  for (const step of [6, 9, 11, 13, 15]) {
    const fillBar = bar % 4 === 3 || bar === 7;
    const chance = parameters.ghostRate * 0.12 + (fillBar ? parameters.fillAmount * 0.28 : 0);
    if (rng() < chance) {
      hits.push({ step, velocity: round2(0.06 + rng() * 0.11) });
    }
  }
  return uniqueHits(hits);
}

function buildHatHits(rng, parameters, full, bar) {
  const hits = [];
  const baseSteps = full ? [2, 6, 10, 14] : bar % 2 === 0 ? [6, 14] : [10];
  const extraSteps = parameters.syncopation > 0.58 ? [3, 8, 13] : [4, 12];
  for (const step of baseSteps) {
    const keepChance = 0.62 + parameters.density * 0.28 - parameters.hatTightness * 0.12;
    if (rng() < keepChance) {
      hits.push({ step, velocity: round2(0.045 + rng() * 0.045) });
    }
  }
  for (const step of extraSteps) {
    const chance = (full ? 0.1 : 0.04) + parameters.syncopation * 0.16 + parameters.density * 0.08;
    if (rng() < chance) {
      hits.push({ step, velocity: round2(0.035 + rng() * 0.04) });
    }
  }
  return uniqueHits(hits);
}

function uniqueHits(hits) {
  const byStep = new Map();
  for (const hit of hits) {
    const step = clamp(hit.step, 0, 15);
    const previous = byStep.get(step);
    if (!previous || hit.velocity > previous.velocity) {
      byStep.set(step, { step, velocity: round2(hit.velocity) });
    }
  }
  return [...byStep.values()].sort((a, b) => a.step - b.step);
}

function buildToneScale(seed, tone) {
  const weights = {
    "3": round2(1 - tone),
    "4": round2(tone),
    "8": round2(1 - tone),
    "9": round2(tone),
    "10": round2(1 - tone),
    "11": round2(tone),
  };
  const third = chooseWeightedDegree(seed, "third", [
    { degree: 3, weight: 1 - tone },
    { degree: 4, weight: tone },
  ]);
  const sixth = chooseWeightedDegree(seed, "sixth", [
    { degree: 8, weight: 1 - tone },
    { degree: 9, weight: tone },
  ]);
  const seventh = chooseWeightedDegree(seed, "seventh", [
    { degree: 10, weight: 1 - tone },
    { degree: 11, weight: tone },
  ]);
  return {
    degrees: [0, 2, third, 5, 7, sixth, seventh],
    weights,
  };
}

function chooseWeightedDegree(seed, label, candidates) {
  const total = candidates.reduce((sum, candidate) => sum + candidate.weight, 0);
  let ticket = hashUnit(`${seed}:scale:${label}`) * total;
  for (const candidate of candidates) {
    ticket -= candidate.weight;
    if (ticket <= 0) {
      return candidate.degree;
    }
  }
  return candidates[candidates.length - 1].degree;
}

function pickProgression(rng, tone) {
  const neutralWeight = Math.max(0, 1 - Math.abs(tone - 0.5) * 2);
  const candidates = [
    ...PROGRESSIONS.dark.map((progression) => ({ item: progression, weight: (1 - tone) ** 2 })),
    ...PROGRESSIONS.neutral.map((progression) => ({ item: progression, weight: neutralWeight ** 1.2 })),
    ...PROGRESSIONS.bright.map((progression) => ({ item: progression, weight: tone ** 2 })),
  ];
  return weightedPick(candidates, rng);
}

function arrangementType(arrangement) {
  const leadPresence = arrangement.leadPresence;
  const leadDensity = round2(clamp(leadPresence * 1.12 + arrangement.grooveAmount * 0.12 - arrangement.textureAmount * 0.22, 0, 1));
  const drums = arrangement.grooveAmount > 0.55 ? "full" : "light";
  const bass = arrangement.grooveAmount > 0.66 ? "groove" : leadPresence > 0.66 ? "sparse" : "simple";
  const harmony = arrangement.textureAmount > 0.66 ? "arp" : arrangement.grooveAmount > 0.72 ? "stab" : "pad";
  const harmonyPresence = round2(clamp(0.18 + arrangement.textureAmount * 0.82 - leadPresence * 0.18 + arrangement.counterAmount * 0.08, 0.12, 1));
  const counter = arrangement.counterAmount > 0.48 || (leadPresence < 0.22 && arrangement.textureAmount > 0.28);
  const focus = arrangementFocus(arrangement);
  const partPresence = arrangementPartPresence(arrangement, {
    leadPresence,
    harmonyPresence,
    counter,
    focus,
  });
  return {
    leadPresence,
    leadDensity,
    drums,
    bass,
    harmony,
    harmonyPresence,
    harmonyMotion: arrangement.harmonyMotion,
    harmonySpace: arrangement.harmonySpace,
    counter,
    focus,
    partPresence,
    droppedParts: droppedParts(partPresence),
  };
}

function arrangementSignature(type) {
  return `${type.focus}-l${Math.round(type.leadPresence * 4)}-g${type.drums}-${type.bass}-${type.harmony}-${type.counter ? "counter" : "solo"}`;
}

function arrangementFocus(arrangement) {
  const scores = {
    lead: arrangement.leadPresence,
    groove: arrangement.grooveAmount,
    texture: arrangement.textureAmount,
    space: round2(arrangement.harmonySpace * 0.55 + (1 - arrangement.grooveAmount) * 0.25 + (1 - arrangement.leadPresence) * 0.2),
  };
  const ranked = Object.entries(scores).sort((a, b) => b[1] - a[1]);
  if (ranked[0][1] < 0.58 || ranked[0][1] - ranked[1][1] < 0.12) {
    return "balanced";
  }
  return ranked[0][0];
}

function arrangementPartPresence(arrangement, derived) {
  const focus = derived.focus;
  return {
    lead: round2(clamp(
      derived.leadPresence * 0.92
        + (focus === "lead" ? 0.16 : 0)
        - (focus === "groove" ? 0.08 : 0)
        - (focus === "space" ? 0.12 : 0),
      0,
      1,
    )),
    harmony: round2(clamp(
      derived.harmonyPresence * 0.86
        + (focus === "texture" ? 0.18 : 0)
        + (focus === "space" ? 0.08 : 0)
        - (focus === "lead" ? 0.1 : 0),
      0,
      1,
    )),
    bass: round2(clamp(
      0.28
        + arrangement.grooveAmount * 0.36
        + (focus === "groove" ? 0.18 : 0)
        - (focus === "space" ? 0.16 : 0)
        - (derived.leadPresence > 0.72 ? 0.08 : 0),
      0,
      1,
    )),
    drums: round2(clamp(
      0.18
        + arrangement.grooveAmount * 0.62
        + (focus === "groove" ? 0.16 : 0)
        - (focus === "space" ? 0.22 : 0)
        - (focus === "texture" ? 0.08 : 0),
      0,
      1,
    )),
    counter: round2(clamp(
      (derived.counter ? 0.34 : 0)
        + arrangement.counterAmount * 0.44
        + (focus === "texture" ? 0.08 : 0)
        - (focus === "lead" ? 0.16 : 0)
        - (focus === "space" ? 0.18 : 0),
      0,
      1,
    )),
  };
}

function droppedParts(partPresence) {
  const threshold = 0.18;
  return Object.fromEntries(Object.entries(partPresence)
    .filter(([, presence]) => presence < threshold)
    .map(([part, presence]) => [part, { presence, reason: "below functional presence threshold" }]));
}

function playbackToneFor(tone) {
  const centered = tone - 0.5;
  return {
    brightness: round2(tone),
    toneFilter: round2(2 ** (centered * 1.7)),
    bassFilter: round2(2 ** (centered * 0.7)),
    noiseFilter: round2(2 ** (centered * 1.2)),
    leadGain: round2(0.88 + tone * 0.32),
    harmonyGain: round2(0.96 + tone * 0.12),
    bassGain: round2(1.14 - tone * 0.24),
    highPercussionGain: round2(0.86 + tone * 0.34),
    lowPercussionGain: round2(1.06 - tone * 0.12),
  };
}

function activeInstruments(type, instruments) {
  const active = {};
  if (type.partPresence.harmony >= 0.18) {
    active.harmony = instruments.harmony;
  }
  if (type.partPresence.bass >= 0.18) {
    active.bass = instruments.bass;
  }
  if (type.partPresence.lead >= 0.18) {
    active.lead = instruments.melody;
  }
  if (type.counter && type.partPresence.counter >= 0.18) {
    active.counter = instruments.counter;
  }
  return active;
}

function generatePhraseGrammar(rng, noteDensity, cadenceHoldSteps) {
  const intervalLeap = round2(rng());
  const rhythmOpenness = round2(rng());
  const syncopation = round2(rng());
  const pulseRegularity = round2(rng());
  const runAmount = round2(rng());
  const peakBar = randomInt(rng, 2, 6);
  const peakDegree = randomInt(rng, 4, 6);
  const finalLanding = round2(rng());
  const finalCadenceDegree = finalLanding < 0.64 ? 0 : finalLanding < 0.86 ? 1 : 2;
  const restBars = generateRestBars(rng, noteDensity, rhythmOpenness);
  const phraseCount = randomInt(rng, 2, 4);
  const degreeOffsets = Array.from({ length: phraseCount }, (_, index) => index === 0 ? 0 : randomInt(rng, 2, 5));
  const midCadenceDegrees = degreeOffsets.map((offset) => clamp(offset + randomInt(rng, 1, 4), 0, 6));
  const contours = degreeOffsets.map((offset, index) => generateContour(rng, intervalLeap, index, offset, peakBar, peakDegree));
  return {
    summary: `density:${noteDensity} regularity:${pulseRegularity} run:${runAmount} leap:${intervalLeap}`,
    noteDensity,
    rhythmOpenness,
    syncopation,
    pulseRegularity,
    runAmount,
    intervalLeap,
    finalLanding,
    peakBar,
    peakDegree,
    cadenceHoldSteps,
    midCadenceDegrees,
    finalCadenceDegree,
    degreeOffsets,
    restBars,
    contours,
  };
}

function generateHookGrammar(rng, noteDensity) {
  const lengthSignal = round2(rng());
  const lengthBars = lengthSignal < 0.34 ? 2 : lengthSignal < 0.82 ? 4 : 8;
  const baseNotesPerBar = randomInt(rng, noteDensity > 0.68 ? 3 : 2, noteDensity > 0.68 ? 6 : 5);
  const densityContrast = round2(rng());
  const barNoteCounts = buildHookBarNoteCounts(rng, lengthBars, baseNotesPerBar, densityContrast);
  const noteCount = barNoteCounts.reduce((sum, count) => sum + count, 0);
  const introSpace = round2(rng());
  const startBar = introSpace < 0.72 ? 0 : introSpace < 0.84 ? 1 : introSpace < 0.94 ? 2 : 4;
  return {
    lengthSignal,
    lengthBars,
    introSpace,
    startBar,
    noteCount,
    barNoteCounts,
    repeatRate: round2(0.24 + rng() * 0.58),
    syncopation: round2(rng()),
    range: randomInt(rng, 3, 6),
    answerShift: randomInt(rng, -2, 3),
    restRate: round2(rng() * 0.34),
    lift: round2(rng()),
    densityContrast,
    durationContrast: round2(rng()),
  };
}

function buildHookBarNoteCounts(rng, lengthBars, baseNotesPerBar, densityContrast) {
  const counts = [];
  let current = baseNotesPerBar + randomInt(rng, -1, 1);
  for (let bar = 0; bar < lengthBars; bar += 1) {
    if (bar > 0) {
      const jump = densityContrast > 0.7 && rng() < 0.42 ? randomInt(rng, -4, 4) : randomInt(rng, -2, 2);
      current = clamp(current + jump, 1, 8);
    }
    if (densityContrast > 0.55 && rng() < 0.24) {
      current = randomInt(rng, 1, 8);
    }
    counts.push(current);
  }
  if (counts.every((count) => count <= 2) && lengthBars >= 4) {
    counts[randomInt(rng, 0, lengthBars - 1)] = randomInt(rng, 4, 7);
  }
  return counts;
}

function generateRestBars(rng, noteDensity, rhythmOpenness) {
  const baseCount = noteDensity < 0.34 ? randomInt(rng, 1, 3) : noteDensity < 0.62 ? pick([0, 1], rng) : rng() < 0.18 ? 1 : 0;
  const count = rhythmOpenness > 0.66 ? Math.min(3, baseCount + 1) : baseCount;
  const candidates = [1, 2, 5, 6];
  const bars = new Set();
  while (bars.size < count) {
    bars.add(pick(candidates, rng));
  }
  return [...bars].sort((a, b) => a - b);
}

function generateContour(rng, intervalLeap, phraseIndex, offset, peakBar, peakDegree) {
  const contour = [];
  let current = clamp(randomInt(rng, 0, 2) + offset, 0, 6);
  let direction = pick([-1, 1], rng);
  const phrasePeakBar = clamp(peakBar + randomInt(rng, -1, 1), 1, 6);
  const phrasePeakDegree = clamp(peakDegree + Math.round(offset * 0.25) - phraseIndex, 3, 6);
  for (let bar = 0; bar < 8; bar += 1) {
    if (bar === phrasePeakBar) {
      current = phrasePeakDegree;
      contour.push(current);
      continue;
    }
    if (bar === 7) {
      contour.push(clamp(Math.round(offset * 0.2), 0, 2));
      continue;
    }
    if (rng() < 0.26) {
      direction *= -1;
    }
    const span = intervalLeap < 0.32
      ? pick([0, 1, 1, 2], rng)
      : intervalLeap > 0.72
        ? pick([1, 2, 3, 4], rng)
        : pick([0, 1, 2, 2, 3], rng);
    current = clamp(current + direction * span + pick([-1, 0, 0, 1], rng), 0, 6);
    contour.push(current);
  }
  return contour;
}

function generateForm(rng, formParameters, phraseCount) {
  const sections = [];
  let phrase = 0;
  let transpose = randomInt(rng, -1, 1);
  let intensity = round2(0.32 + rng() * 0.36);
  for (let index = 0; index < 4; index += 1) {
    if (index > 0) {
      if (rng() < formParameters.phraseChange) {
        phrase = (phrase + 1 + randomInt(rng, 0, Math.max(0, phraseCount - 2))) % phraseCount;
      }
      const transposeSpan = 1 + Math.round(formParameters.sectionChange * 3);
      transpose = clamp(transpose + randomInt(rng, -transposeSpan, transposeSpan), -4, 5);
      const intensitySpan = 0.16 + formParameters.intensityMotion * 0.58;
      intensity = round2(clamp(intensity + (rng() - 0.45) * intensitySpan, 0.18, 1));
    }
    const entryDelayBars = index === 0
      ? formParameters.openingSpace < 0.7 ? 0 : formParameters.openingSpace < 0.9 ? 4 : 8
      : index === 1 && sections[0].entryDelayBars === 8
        ? formParameters.openingSpace < 0.96 ? 0 : 4
        : 0;
    sections.push({
      entryDelayBars,
      phrase,
      transpose,
      intensity,
    });
  }
  const roles = sections.map((section, index) => roleFromSection(section, index));
  return {
    sections,
    roles,
  };
}

function roleFromSection(section, index) {
  if (section.entryDelayBars >= 8) {
    return "space";
  }
  if (section.entryDelayBars >= 4) {
    return "intro";
  }
  if (index === 3 && section.transpose < 0 && section.intensity < 0.58) {
    return "statementReturn";
  }
  if (index === 3 && section.transpose < -1) {
    return "resolution";
  }
  if (section.phrase === 1 && section.intensity > 0.68) {
    return "contrastVariation";
  }
  if (section.phrase === 1) {
    return "contrast";
  }
  if (section.intensity > 0.72) {
    return "development";
  }
  if (section.transpose > 0) {
    return "variation";
  }
  return "statement";
}

function formSignature(sections) {
  return sections
    .map((section) => `${section.entryDelayBars}:${section.phrase}:${section.transpose}:${Math.round(section.intensity * 9)}`)
    .join("/");
}

function buildMelodyForm(rng, scaleLength, phraseGrammar, hookGrammar) {
  const firstStart = randomInt(rng, 0, Math.min(2, scaleLength - 1));
  return {
    phrases: phraseGrammar.contours.map((_, index) => {
      const start = clamp(firstStart + phraseGrammar.degreeOffsets[index], 0, scaleLength - 1);
      return buildPhrase(rng, scaleLength, phraseGrammar, start, index);
    }),
    hook: buildHook(rng, scaleLength, phraseGrammar, hookGrammar, firstStart),
  };
}

function buildHook(rng, scaleLength, phraseGrammar, hookGrammar, startDegree) {
  const steps = hookGrammar.barNoteCounts.flatMap((count, bar) => (
    hookBarSteps(rng, count, bar * 16, hookGrammar.syncopation, bar === 0)
  )).sort((a, b) => a - b);
  const motif = [];
  let current = startDegree;
  let previousInterval = 0;
  const peakIndex = randomInt(rng, Math.max(1, Math.floor(steps.length * 0.35)), Math.max(1, Math.floor(steps.length * 0.72)));
  const answerStart = Math.max(1, Math.floor(steps.length * (0.45 + hookGrammar.durationContrast * 0.22)));
  for (let index = 0; index < steps.length; index += 1) {
    const bar = Math.floor(steps[index] / 16);
    if (index === 0) {
      current = startDegree;
    } else if (index === steps.length - 1) {
      current = phraseGrammar.finalCadenceDegree;
    } else if (index === peakIndex) {
      current = clamp(startDegree + hookGrammar.range, 0, scaleLength - 1);
    } else if (rng() < hookGrammar.repeatRate) {
      current = clamp(current + previousInterval, 0, scaleLength - 1);
    } else {
      const progress = index / Math.max(1, steps.length - 1);
      const target = index < answerStart
        ? startDegree + Math.round(hookGrammar.range * progress)
        : startDegree + hookGrammar.answerShift + Math.round(hookGrammar.range * 0.4);
      current = nextHookDegree(rng, current, target, scaleLength, hookGrammar);
    }
    if (motif.length >= 2 && current === motif[motif.length - 1].n && current === motif[motif.length - 2].n) {
      current = clamp(current + pick(current >= scaleLength - 2 ? [-1, -1, 0] : [-1, 1, 1], rng), 0, scaleLength - 1);
    }
    previousInterval = motif.length === 0 ? pick([-1, 0, 1], rng) : clamp(current - motif[motif.length - 1].n, -2, 2);
    motif.push({
      s: steps[index] % 16,
      b: bar,
      d: hookDuration(steps, index, hookGrammar),
      n: current,
      a: hookAccent(index, steps.length, steps[index]),
    });
  }
  const cycleCount = Math.max(1, Math.floor(8 / hookGrammar.lengthBars));
  const restCycles = Array.from({ length: cycleCount }, (_, index) => index > 0 && rng() < hookGrammar.restRate);
  if (cycleCount > 1 && restCycles.slice(1).every(Boolean)) {
    restCycles[randomInt(rng, 1, cycleCount - 1)] = false;
  }
  const variantShifts = Array.from({ length: cycleCount }, (_, index) => {
    if (index === 0) {
      return 0;
    }
    const raw = Math.round((rng() - 0.42) * (1 + hookGrammar.lift * 4));
    return clamp(raw, -2, 3);
  });
  return {
    lengthBars: hookGrammar.lengthBars,
    startBar: hookGrammar.startBar,
    motif,
    restCycles,
    variantShifts,
    score: {
      lengthBars: hookGrammar.lengthBars,
      startBar: hookGrammar.startBar,
      barNoteCounts: hookGrammar.barNoteCounts,
      steps: motif.map((item) => item.b * 16 + item.s),
      degrees: motif.map((item) => item.n),
      accents: motif.map((item) => item.a),
      restCycles,
      variantShifts,
    },
  };
}

function hookBarSteps(rng, count, barStart, syncopation, forceStart) {
  if (count <= 0) {
    return [];
  }
  const steps = forceStart ? [barStart + pick([0, 1, 2], rng)] : [];
  while (steps.length < count) {
    const weights = Array.from({ length: 16 }, (_, step) => {
      const absolute = barStart + step;
      if (steps.includes(absolute)) {
        return 0;
      }
      const nearExisting = steps.some((used) => Math.abs(used - absolute) <= 1);
      const offbeat = step % 4 === 1 || step % 4 === 3;
      const downbeat = step === 0 || step === 8;
      const edgeWeight = downbeat ? 1.2 - syncopation * 0.35 : 1;
      const syncWeight = offbeat ? 0.72 + syncopation * 1.4 : edgeWeight;
      return syncWeight * (nearExisting ? 0.42 : 1);
    });
    steps.push(barStart + weightedIndex(weights, rng));
  }
  return steps.sort((a, b) => a - b);
}

function nextHookDegree(rng, current, target, scaleLength, hookGrammar) {
  const direction = Math.sign(target - current);
  const step = direction === 0
    ? pick([-1, 0, 1], rng)
    : direction * pick(hookGrammar.range > 4 ? [1, 1, 2, 3] : [1, 1, 1, 2], rng);
  return clamp(current + step, 0, scaleLength - 1);
}

function hookDuration(steps, index, hookGrammar) {
  const current = steps[index];
  const next = steps[index + 1] ?? current + 6;
  const maxDuration = hookGrammar.durationContrast > 0.62 ? 8 : 6;
  return clamp(Math.min(maxDuration, next - current), 1, maxDuration);
}

function hookAccent(index, count, step) {
  const edge = index === 0 || index === count - 1 ? 0.16 : 0;
  const downbeat = step % 16 === 0 || step % 16 === 8 ? 0.08 : 0;
  return round2(clamp(0.92 + edge + downbeat, 0.84, 1.24));
}

function buildPhrase(rng, scaleLength, phraseGrammar, startDegree, phraseIndex) {
  const phrase = [];
  let current = startDegree;
  const contour = phraseGrammar.contours[phraseIndex];
  for (let bar = 0; bar < 8; bar += 1) {
    const cadence = bar === 3 || bar === 7;
    const density = phraseDensity(phraseGrammar, bar, cadence);
    const notes = [];
    if (density === 0) {
      phrase.push(notes);
      continue;
    }
    const rhythm = pickRhythm(rng, phraseGrammar, density, cadence, bar);
    for (let i = 0; i < rhythm.length; i += 1) {
      const target = contour[bar] + (i === rhythm.length - 1 ? 0 : pick([-1, 0, 1], rng));
      current = nextPhraseDegree(rng, current, target, scaleLength, phraseGrammar);
      notes.push({
        s: rhythm[i],
        d: noteDuration(phraseGrammar, i, cadence, rhythm.length),
        n: current,
        a: noteAccent(phraseGrammar, rhythm[i], i, cadence, rhythm.length),
      });
    }
    if (cadence) {
      notes[notes.length - 1].n = bar === 7 ? phraseGrammar.finalCadenceDegree : phraseGrammar.midCadenceDegrees[phraseIndex];
      notes[notes.length - 1].d += 2;
      notes[notes.length - 1].a = 1.18;
    }
    phrase.push(notes);
  }
  return phrase;
}

function phraseDensity(phraseGrammar, bar, cadence) {
  if (cadence) {
    return 1;
  }
  if (phraseGrammar.restBars.includes(bar)) {
    return 0;
  }
  const base = 1 + Math.round(phraseGrammar.noteDensity * 6);
  const rhythmOffset = phraseGrammar.rhythmOpenness > 0.66 ? -1 : phraseGrammar.noteDensity > 0.78 ? 1 : 0;
  const barOffset = bar % 3 === 1 ? -1 : bar % 4 === 2 ? 1 : 0;
  return clamp(base + rhythmOffset + barOffset, 1, 6);
}

function pickRhythm(rng, phraseGrammar, density, cadence, bar) {
  if (cadence) {
    return [pick([4, 5, 6, 7], rng)];
  }
  if (phraseGrammar.pulseRegularity > 0.8) {
    return pickEvenRhythm(rng, phraseGrammar, density);
  }
  const burstCount = density >= 5 && phraseGrammar.runAmount > 0.58 ? randomInt(rng, 2, Math.min(5, density)) : 0;
  const clusterCenter = clamp(Math.round((bar % 4) * 3.4 + rng() * 4), 1, 14);
  const steps = [];
  if (burstCount > 0) {
    const start = clamp(clusterCenter - randomInt(rng, 0, 2), 0, 14);
    const stride = phraseGrammar.syncopation > 0.58 ? pick([1, 1, 2], rng) : 1;
    for (let index = 0; index < burstCount; index += 1) {
      const step = clamp(start + index * stride + (rng() < 0.22 ? 1 : 0), 0, 15);
      if (!steps.includes(step)) {
        steps.push(step);
      }
    }
  }
  while (steps.length < density) {
    const step = weightedRhythmStep(rng, phraseGrammar, steps, clusterCenter);
    if (!steps.includes(step)) {
      steps.push(step);
    }
  }
  return steps.sort((a, b) => a - b);
}

function pickEvenRhythm(rng, phraseGrammar, density) {
  const spacing = 16 / density;
  const phase = phraseGrammar.rhythmOpenness > 0.66 ? rng() * spacing * 0.4 : rng() * spacing;
  const maxJitter = phraseGrammar.syncopation * Math.min(2.5, spacing * 0.45);
  const steps = [];
  for (let index = 0; index < density; index += 1) {
    const jitter = (rng() - 0.5) * maxJitter;
    const step = clamp(Math.round(phase + index * spacing + jitter), 0, 15);
    if (!steps.includes(step)) {
      steps.push(step);
    }
  }
  return steps.sort((a, b) => a - b);
}

function weightedRhythmStep(rng, phraseGrammar, usedSteps, clusterCenter) {
  const weights = Array.from({ length: 16 }, (_, step) => {
    if (usedSteps.includes(step)) {
      return 0;
    }
    const offbeat = step % 4 === 1 || step % 4 === 3;
    const beat = step % 4 === 0;
    const syncWeight = offbeat ? 0.7 + phraseGrammar.syncopation * 1.6 : beat ? 1.25 - phraseGrammar.syncopation * 0.5 : 1;
    const distance = Math.abs(step - clusterCenter);
    const clusterWeight = 1 + (1 - phraseGrammar.pulseRegularity) * Math.max(0, 5 - distance) * 0.45;
    const spacingPenalty = usedSteps.some((used) => Math.abs(used - step) <= 1) && phraseGrammar.runAmount < 0.44 ? 0.28 : 1;
    return syncWeight * clusterWeight * spacingPenalty;
  });
  const total = weights.reduce((sum, weight) => sum + weight, 0);
  let ticket = rng() * total;
  for (let step = 0; step < weights.length; step += 1) {
    ticket -= weights[step];
    if (ticket <= 0) {
      return step;
    }
  }
  return 15;
}

function noteAccent(phraseGrammar, step, index, cadence, count) {
  if (cadence) {
    return 1.18;
  }
  const position = step === 0 || step === 8 ? 0.1 : step % 4 === 0 ? 0.04 : 0;
  const phraseEdge = index === 0 || index === count - 1 ? 0.08 : 0;
  const shortRun = count >= 5 && phraseGrammar.runAmount > 0.5 ? (index % 2 === 0 ? 0.08 : -0.03) : 0;
  return round2(clamp(0.88 + position + phraseEdge + shortRun, 0.72, 1.22));
}

function noteDuration(phraseGrammar, index, cadence, count) {
  if (cadence) {
    return phraseGrammar.cadenceHoldSteps;
  }
  if (phraseGrammar.rhythmOpenness > 0.66) {
    return count === 1 ? 8 : index === 0 ? 5 : 4;
  }
  if (count >= 6) {
    return 1;
  }
  if (count >= 4) {
    return 2;
  }
  if (phraseGrammar.noteDensity > 0.72 || phraseGrammar.syncopation > 0.58) {
    return count === 1 ? 4 : 2;
  }
  return count === 1 ? 5 : 3;
}

function nextPhraseDegree(rng, current, target, scaleLength, phraseGrammar) {
  if (phraseGrammar.intervalLeap > 0.72 && rng() < 0.32) {
    return clamp(target + pick([-2, 2, 3], rng), 0, scaleLength - 1);
  }
  const direction = Math.sign(target - current);
  const steps = phraseGrammar.intervalLeap < 0.32 ? [1, 1, 1, 2] : [1, 1, 2, 3];
  const step = direction === 0 ? pick([-1, 0, 1], rng) : direction * pick(steps, rng);
  return clamp(current + step, 0, scaleLength - 1);
}

function addHarmony(events, barStart, chord, type, instruments, localBar) {
  if (type.partPresence.harmony < 0.18) {
    return;
  }
  if (!harmonyActive(type, localBar)) {
    return;
  }
  const level = 0.46 + type.partPresence.harmony * 0.68;
  if (type.harmony === "pad") {
    const step = type.leadPresence > 0.58 ? 2 : 0;
    const durationSteps = Math.round(7 + type.harmonyPresence * 5);
    const velocity = 0.1 - type.leadPresence * 0.035;
    events.push({ track: "chord", step: barStart + step, durationSteps, notes: voiceChord(chord), instrument: instruments.harmony, velocity: velocity * level });
    return;
  }
  if (type.harmony === "arp") {
    const steps = arpSteps(type, localBar);
    const order = arpOrder(type, localBar);
    const voiced = voiceChord(chord);
    const velocity = (0.085 - type.leadPresence * 0.025) * level * (0.82 + type.harmonyMotion * 0.22);
    for (let i = 0; i < steps.length; i += 1) {
      events.push({ track: "chord", step: barStart + steps[i], durationSteps: type.harmonyMotion > 0.72 ? 1 : 2, notes: [voiced[order[i % order.length]]], instrument: instruments.harmony, velocity });
    }
    return;
  }
  const velocity = 0.095 - type.leadPresence * 0.025;
  for (const step of [0, 10]) {
    events.push({ track: "chord", step: barStart + step, durationSteps: 3, notes: voiceChord(chord).slice(step === 0 ? 0 : 1), instrument: "organ", velocity: velocity * level });
  }
}

function harmonyActive(type, localBar) {
  const presence = type.harmonyPresence;
  const space = type.harmonySpace;
  if (presence < 0.26) {
    return localBar === 0;
  }
  if (presence < 0.46) {
    return localBar === 0 || localBar === 4;
  }
  if (presence < 0.66) {
    return localBar % 2 === 0;
  }
  if (type.harmony === "arp" && space > 0.68) {
    return ![1, 5].includes(localBar);
  }
  if (type.harmony === "arp" && space > 0.46) {
    return localBar % 4 !== 1;
  }
  return true;
}

function arpSteps(type, localBar) {
  const sparse = type.harmonySpace > 0.62;
  const count = sparse
    ? type.harmonyPresence < 0.72 ? 2 : 3
    : type.harmonyPresence < 0.42 ? 2 : type.harmonyPresence < 0.68 ? 3 : 4;
  const phase = type.harmonyMotion < 0.34
    ? 0
    : type.harmonyMotion < 0.68
      ? (localBar % 2) * 2
      : [0, 1, 3, 2][localBar % 4];
  const candidates = type.harmonySpace > 0.72
    ? [phase, phase + 5, phase + 11, phase + 14]
    : type.harmonyMotion > 0.72
      ? [phase + 1, phase + 4, phase + 9, phase + 13]
      : [phase, phase + 4, phase + 9, phase + 12];
  return candidates
    .map((step) => clamp(step, 0, 15))
    .filter((step, index, steps) => index < count && steps.indexOf(step) === index)
    .sort((a, b) => a - b);
}

function arpOrder(type, localBar) {
  if (type.harmonyMotion > 0.76) {
    return localBar % 2 === 0 ? [0, 2, 1, 2] : [1, 0, 2, 1];
  }
  if (type.harmonySpace > 0.62) {
    return localBar % 2 === 0 ? [0, 2, 1] : [1, 2, 0];
  }
  return localBar % 2 === 0 ? [0, 1, 2, 1] : [1, 2, 0, 2];
}

function addBass(events, barStart, chord, type, instruments, localBar) {
  if (type.partPresence.bass < 0.18) {
    return;
  }
  const phraseBar = localBar % 4;
  const leadDucking = 1 - type.leadPresence * 0.24;
  const presenceLevel = 0.62 + type.partPresence.bass * 0.54;
  if (type.bass === "sparse") {
    const sparse = [
      [{ step: 2, degree: 0, duration: 8, velocity: 0.16 }],
      [],
      [{ step: 8, degree: 0, duration: 5, velocity: 0.14 }],
      [{ step: 12, degree: 2, duration: 2, velocity: 0.14 }],
    ][phraseBar];
    for (const hit of sparse) {
      events.push({ track: "bass", step: barStart + hit.step, durationSteps: hit.duration, notes: [chord[hit.degree] - 24], instrument: instruments.bass, velocity: hit.velocity * leadDucking * presenceLevel });
    }
    return;
  }
  if (type.bass === "groove") {
    const pattern = localBar % 2 === 0 ? [[2, 0], [6, 2], [10, 0], [14, 1]] : [[3, 0], [8, 1], [12, 2]];
    for (const [step, degree] of pattern) {
      events.push({ track: "bass", step: barStart + step, durationSteps: 2, notes: [chord[degree] - 24], instrument: instruments.bass, velocity: 0.3 * leadDucking * presenceLevel });
    }
    return;
  }
  events.push({ track: "bass", step: barStart + 2, durationSteps: 9, notes: [chord[0] - 24], instrument: instruments.bass, velocity: 0.16 * leadDucking * presenceLevel });
  if (phraseBar === 3) {
    events.push({ track: "bass", step: barStart + 12, durationSteps: 2, notes: [chord[2] - 24], instrument: instruments.bass, velocity: 0.19 * leadDucking * presenceLevel });
  }
}

function addDrums(events, barStart, type, drums, localBar) {
  if (type.partPresence.drums < 0.18) {
    return;
  }
  const full = type.drums === "full";
  const pattern = (full ? drums.score.full : drums.score.light)[localBar % 8];
  const presenceLevel = 0.58 + type.partPresence.drums * 0.56;
  for (const hit of pattern.kick) {
    events.push({ track: "drums", step: barStart + hit.step, durationSteps: 1, notes: ["kick"], instrument: "kick", velocity: hit.velocity * presenceLevel });
  }
  for (const hit of pattern.snare) {
    events.push({ track: "drums", step: barStart + hit.step, durationSteps: 1, notes: ["snare"], instrument: "snare", velocity: hit.velocity * presenceLevel });
  }
  for (const hit of pattern.hat) {
    events.push({ track: "drums", step: barStart + hit.step, durationSteps: 1, notes: ["hat"], instrument: "hat", velocity: hit.velocity * presenceLevel });
  }
}

function addMelody(events, barStart, tonic, scale, chordDegree, type, form, instruments, section, bar, localBar, tone) {
  if (type.partPresence.lead < 0.18) {
    return;
  }
  if (bar < form.hook.startBar) {
    return;
  }
  const leadLocalBar = (bar - form.hook.startBar) % 8;
  const octave = tone > 0.68 ? 24 : 12;
  const focusLevel = type.focus === "lead" ? 1 : type.focus === "groove" ? 0.84 : type.focus === "space" ? 0.78 : 0.9;
  const velocity = (0.1 + type.partPresence.lead * 0.17) * (0.78 + section.intensity * 0.34) * focusLevel;
  const melodyItems = hookItemsForBar(form.hook, section, leadLocalBar, type);
  for (const item of melodyItems) {
    const cadence = leadLocalBar === 7 && item.end;
    const degree = cadence ? chordDegree + section.transpose : item.n + section.transpose;
    const duration = item.d + (cadence ? 2 : 0);
    events.push({
      track: "lead",
      step: barStart + item.s,
      durationSteps: duration,
      notes: [noteAtDegree(tonic, scale, degree) + octave],
      instrument: instruments.melody,
      velocity: velocity * (item.a ?? 1),
    });
  }
}

function hookItemsForBar(hook, section, localBar, type) {
  const cycle = Math.floor(localBar / hook.lengthBars);
  const cycleBar = localBar % hook.lengthBars;
  const restIndex = (cycle + section.phrase) % hook.restCycles.length;
  if (type.leadPresence < 0.28 && cycle > 0) {
    return [];
  }
  if (type.leadPresence < 0.42 && cycle > 0 && cycle % 2 === 1) {
    return [];
  }
  if (cycle > 0 && hook.restCycles[restIndex] && section.intensity < 0.74) {
    return [];
  }
  const shift = hook.variantShifts[(cycle + section.phrase) % hook.variantShifts.length];
  const items = hook.motif
    .filter((item) => item.b === cycleBar)
    .map((item, index, filtered) => ({
      ...item,
      n: item.n + shift,
      end: cycleBar === 1 && index === filtered.length - 1,
    }));
  if (items.length <= 2 || type.leadDensity > 0.72) {
    return items;
  }
  const keepRatio = clamp(0.24 + type.leadDensity * 0.7 + section.intensity * 0.12, 0.18, 1);
  const keepCount = Math.max(1, Math.min(items.length, Math.ceil(items.length * keepRatio)));
  const stride = items.length > keepCount ? Math.max(1, Math.floor(items.length / keepCount)) : 1;
  const start = (cycle + section.phrase) % Math.max(1, stride);
  const selected = [];
  for (let index = start; index < items.length && selected.length < keepCount; index += stride) {
    selected.push(items[index]);
  }
  if (selected.length < keepCount) {
    selected.push(...items.filter((item) => !selected.includes(item)).slice(0, keepCount - selected.length));
  }
  return selected.sort((a, b) => a.s - b.s);
}

function addCounter(events, barStart, tonic, scale, type, instruments, section, localBar, tone) {
  if (!type.counter || type.partPresence.counter < 0.18 || localBar < section.entryDelayBars || ![2, 6].includes(localBar)) {
    return;
  }
  const octave = tone > 0.6 ? 24 : 12;
  const presenceLevel = 0.54 + type.partPresence.counter * 0.56;
  for (const [step, degree] of [[2, 0], [9, 2], [13, 1]]) {
    events.push({ track: "counter", step: barStart + step, durationSteps: 2, notes: [noteAtDegree(tonic, scale, degree + section.transpose) + octave], instrument: instruments.counter, velocity: (0.08 + section.intensity * 0.04) * presenceLevel });
  }
}

function phraseForSection(form, section, localBar) {
  const phrase = form.phrases[section.phrase % form.phrases.length];
  return phrase[localBar];
}

export function midiToFrequency(note) {
  return 440 * 2 ** ((note - 69) / 12);
}

export function createPlayer(audioContext, playbackScore) {
  let timer = null;
  let loopStart = 0;
  const scheduled = new Set();
  const activeSources = new Set();
  const lookaheadMs = 80;
  const scheduleAheadSeconds = 0.28;
  const stepSeconds = (60 / playbackScore.transport.bpm) * playbackScore.transport.stepDurationBeats;
  const loopSeconds = playbackScore.transport.loopSteps * stepSeconds;

  function start(progress = 0) {
    stop();
    scheduled.clear();
    loopStart = audioContext.currentTime + 0.05 - progress * loopSeconds;
    timer = setInterval(schedule, lookaheadMs);
    schedule();
  }

  function stop() {
    if (timer !== null) {
      clearInterval(timer);
      timer = null;
    }
    for (const source of activeSources) {
      try {
        source.stop();
      } catch {
        // Stopped sources can be ignored.
      }
      source.disconnect();
    }
    activeSources.clear();
    scheduled.clear();
  }

  function schedule() {
    const now = audioContext.currentTime;
    const firstLoop = Math.max(0, Math.floor((now - loopStart) / loopSeconds));
    for (const loopIndex of [firstLoop, firstLoop + 1]) {
      const candidateLoopStart = loopStart + loopIndex * loopSeconds;
      for (let eventIndex = 0; eventIndex < playbackScore.events.length; eventIndex += 1) {
        const event = playbackScore.events[eventIndex];
        const startsAt = candidateLoopStart + event.step * stepSeconds;
        const key = `${loopIndex}:${eventIndex}`;
        if (!scheduled.has(key) && startsAt >= now && startsAt < now + scheduleAheadSeconds) {
          scheduled.add(key);
          playEvent(audioContext, playbackScore, event, startsAt, event.durationSteps * stepSeconds, activeSources);
        }
      }
    }
    for (const key of scheduled) {
      const [loopIndexText] = key.split(":");
      const loopIndex = Number(loopIndexText);
      if (loopStart + loopIndex * loopSeconds < now - loopSeconds) {
        scheduled.delete(key);
      }
    }
  }

  function loopProgress() {
    if (timer === null) {
      return 0;
    }
    const elapsed = Math.max(0, audioContext.currentTime - loopStart);
    return (elapsed % loopSeconds) / loopSeconds;
  }

  return { start, stop, loopProgress };
}

function playEvent(audioContext, playbackScore, event, startsAt, duration, activeSources) {
  const timbre = playbackScore.timbres[event.timbre] ?? { kind: event.timbre };
  const instrument = timbre?.kind || event.timbre;
  const playbackTone = playbackScore.mix.playbackTone;
  const volume = playbackScore.mix.volume;
  const eventTone = toneForEvent(event, playbackTone);
  const gain = eventTone.gain * volume;
  for (const note of event.notes) {
    if (typeof note === "number") {
      if (isPlucked(instrument)) {
        playPluck(audioContext, note, startsAt, duration, instrument, event.velocity * gain, eventTone.filter, activeSources);
      } else {
        playTone(audioContext, note, startsAt, duration, instrument, event.velocity * gain, eventTone.filter, activeSources);
      }
    } else {
      playNoise(audioContext, startsAt, duration, timbre, event.velocity * gain, eventTone.filter, activeSources);
    }
  }
}

function toneForEvent(event, tone) {
  if (event.track === "bass") {
    return { gain: tone.bassGain, filter: tone.bassFilter };
  }
  if (event.track === "lead" || event.track === "counter") {
    return { gain: tone.leadGain, filter: tone.toneFilter };
  }
  if (event.track === "chord") {
    return { gain: tone.harmonyGain, filter: tone.toneFilter };
  }
  if (event.timbre === "hat" || event.timbre === "snare") {
    return { gain: tone.highPercussionGain, filter: tone.noiseFilter };
  }
  return { gain: tone.lowPercussionGain, filter: tone.bassFilter };
}

function playTone(audioContext, midiNote, startsAt, duration, instrument, velocity, filterMultiplier, activeSources) {
  const oscillator = audioContext.createOscillator();
  const gain = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  oscillator.type = waveformFor(instrument);
  oscillator.frequency.setValueAtTime(midiToFrequency(midiNote), startsAt);
  const envelope = envelopeFor(instrument, duration);
  filter.type = "lowpass";
  filter.frequency.setValueAtTime(envelope.filter * filterMultiplier, startsAt);
  gain.gain.setValueAtTime(0.0001, startsAt);
  gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, velocity), startsAt + envelope.attack);
  gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, velocity * envelope.sustain), startsAt + Math.max(envelope.attack + 0.01, duration * 0.5));
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.04, duration));
  oscillator.connect(filter).connect(gain).connect(audioContext.destination);
  trackSource(oscillator, activeSources);
  oscillator.start(startsAt);
  oscillator.stop(startsAt + duration + 0.05);
  if (instrument === "breathy-flute") {
    playBreathNoise(audioContext, startsAt, duration, velocity * 0.18, filterMultiplier, activeSources);
  }
  if (instrument === "chip-lead") {
    playChipClick(audioContext, startsAt, velocity * 0.22, filterMultiplier, activeSources);
  }
}

function playPluck(audioContext, midiNote, startsAt, duration, instrument, velocity, filterMultiplier, activeSources) {
  const sampleRate = audioContext.sampleRate;
  const seconds = Math.max(0.15, Math.min(1.5, duration + 0.25));
  const samples = Math.floor(sampleRate * seconds);
  const buffer = audioContext.createBuffer(1, samples, sampleRate);
  const data = buffer.getChannelData(0);
  const frequency = midiToFrequency(midiNote);
  const decay = pluckDecayFor(instrument);
  const bright = pluckBrightnessFor(instrument);
  const bodyMix = instrument === "harp" || instrument === "nylon" ? 0.66 : instrument === "marimba" ? 0.48 : 0.55;
  const upperMix = instrument === "kalimba" || instrument === "music-box" ? 0.34 : instrument === "harp" ? 0.18 : 0.24;
  const rng = mulberry32(hashSeed(`${instrument}:${midiNote}:${duration}`));
  for (let i = 0; i < samples; i += 1) {
    const t = i / sampleRate;
    const env = Math.exp(-decay * t);
    const body = Math.sin(2 * Math.PI * frequency * t);
    const detunedBody = Math.sin(2 * Math.PI * frequency * 1.006 * t) * 0.18;
    const upper = Math.sin(2 * Math.PI * frequency * (instrument === "marimba" ? 2.7 : 2.01) * t) * bright;
    const click = Math.exp(-90 * t) * Math.sin(2 * Math.PI * frequency * 7 * t) * bright * 0.35;
    const wood = instrument === "marimba" ? Math.exp(-28 * t) * Math.sin(2 * Math.PI * frequency * 0.52 * t) * 0.28 : 0;
    const scrape = (rng() * 2 - 1) * Math.exp(-38 * t) * (instrument === "nylon" || instrument === "harp" ? 0.035 : 0.018);
    data[i] = (body * bodyMix + detunedBody + upper * upperMix + click + wood + scrape) * env;
  }
  const source = audioContext.createBufferSource();
  const gain = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  filter.type = "lowpass";
  filter.frequency.value = pluckFilterFor(instrument) * filterMultiplier;
  gain.gain.setValueAtTime(Math.max(0.0001, velocity), startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + seconds);
  source.buffer = buffer;
  source.connect(filter).connect(gain).connect(audioContext.destination);
  trackSource(source, activeSources);
  source.start(startsAt);
}

function playBreathNoise(audioContext, startsAt, duration, velocity, filterMultiplier, activeSources) {
  const samples = Math.max(1, Math.floor(audioContext.sampleRate * Math.max(0.04, duration)));
  const buffer = audioContext.createBuffer(1, samples, audioContext.sampleRate);
  const data = buffer.getChannelData(0);
  const rng = mulberry32(hashSeed(`breath:${startsAt}:${duration}`));
  for (let i = 0; i < samples; i += 1) {
    const t = i / audioContext.sampleRate;
    data[i] = (rng() * 2 - 1) * Math.exp(-2.4 * t);
  }
  const source = audioContext.createBufferSource();
  const gain = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  filter.type = "bandpass";
  filter.frequency.value = 1800 * filterMultiplier;
  filter.Q.value = 0.8;
  gain.gain.setValueAtTime(0.0001, startsAt);
  gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, velocity), startsAt + 0.035);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.06, duration));
  source.buffer = buffer;
  source.connect(filter).connect(gain).connect(audioContext.destination);
  trackSource(source, activeSources);
  source.start(startsAt);
}

function playChipClick(audioContext, startsAt, velocity, filterMultiplier, activeSources) {
  const samples = Math.max(1, Math.floor(audioContext.sampleRate * 0.018));
  const buffer = audioContext.createBuffer(1, samples, audioContext.sampleRate);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < samples; i += 1) {
    const t = i / audioContext.sampleRate;
    data[i] = Math.sign(Math.sin(2 * Math.PI * 3600 * t)) * Math.exp(-180 * t);
  }
  const source = audioContext.createBufferSource();
  const gain = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  filter.type = "highpass";
  filter.frequency.value = 1800 * filterMultiplier;
  gain.gain.setValueAtTime(Math.max(0.0001, velocity), startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + 0.018);
  source.buffer = buffer;
  source.connect(filter).connect(gain).connect(audioContext.destination);
  trackSource(source, activeSources);
  source.start(startsAt);
}

function playNoise(audioContext, startsAt, duration, timbre, velocity, filterMultiplier, activeSources) {
  const instrument = timbre.kind;
  if (instrument === "kick") {
    playKick(audioContext, startsAt, timbre, velocity, filterMultiplier, activeSources);
    return;
  }
  const decay = instrument === "hat" ? timbre.decay : timbre.decay ?? duration;
  const samples = Math.max(1, Math.floor(audioContext.sampleRate * Math.max(0.018, decay)));
  const buffer = audioContext.createBuffer(1, samples, audioContext.sampleRate);
  const data = buffer.getChannelData(0);
  const rng = mulberry32(hashSeed(`${instrument}:${startsAt}:${duration}:${timbre.filter ?? 0}`));
  for (let i = 0; i < samples; i += 1) {
    const t = i / audioContext.sampleRate;
    const env = Math.exp(-t * (instrument === "hat" ? 42 : 16));
    const noise = rng() * 2 - 1;
    const body = instrument === "snare" ? Math.sin(2 * Math.PI * 185 * t) * (timbre.tone ?? 0.1) : 0;
    data[i] = (noise * (instrument === "hat" ? timbre.brightness ?? 1 : timbre.snap ?? 0.4) + body) * env;
  }
  const source = audioContext.createBufferSource();
  const gain = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  filter.type = instrument === "snare" ? "bandpass" : "highpass";
  filter.frequency.value = (timbre.filter ?? (instrument === "hat" ? 5200 : 900)) * filterMultiplier;
  if (instrument === "snare") {
    filter.Q.value = 0.9 + (timbre.snap ?? 0.4);
  }
  gain.gain.setValueAtTime(velocity, startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.018, decay));
  source.buffer = buffer;
  source.connect(filter).connect(gain).connect(audioContext.destination);
  trackSource(source, activeSources);
  source.start(startsAt);
}

function playKick(audioContext, startsAt, timbre, velocity, filterMultiplier, activeSources) {
  const oscillator = audioContext.createOscillator();
  const gain = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  const duration = Math.max(0.06, timbre.decay ?? 0.14);
  oscillator.type = "sine";
  oscillator.frequency.setValueAtTime(timbre.pitchStart ?? 64, startsAt);
  oscillator.frequency.exponentialRampToValueAtTime(Math.max(20, timbre.pitchEnd ?? 36), startsAt + duration);
  filter.type = "lowpass";
  filter.frequency.value = 700 * filterMultiplier;
  gain.gain.setValueAtTime(Math.max(0.0001, velocity), startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + duration);
  oscillator.connect(filter).connect(gain).connect(audioContext.destination);
  trackSource(oscillator, activeSources);
  oscillator.start(startsAt);
  oscillator.stop(startsAt + duration + 0.03);
  playChipClick(audioContext, startsAt, velocity * (timbre.click ?? 0.12), filterMultiplier, activeSources);
}

function waveformFor(instrument) {
  if (instrument === "flute" || instrument === "breathy-flute" || instrument === "glass" || instrument === "sine-bell") {
    return "sine";
  }
  if (instrument === "pad" || instrument === "organ" || instrument === "reed" || instrument === "saw-lead") {
    return "sawtooth";
  }
  if (instrument === "round-bass" || instrument === "soft-square" || instrument === "triangle-lead") {
    return "triangle";
  }
  return "square";
}

function envelopeFor(instrument, duration) {
  if (instrument === "breathy-flute") {
    return { attack: 0.075, sustain: 0.54, filter: 1050 };
  }
  if (instrument === "flute") {
    return { attack: 0.055, sustain: 0.62, filter: 1150 };
  }
  if (instrument === "clarinet") {
    return { attack: 0.026, sustain: 0.58, filter: 920 };
  }
  if (instrument === "reed") {
    return { attack: 0.016, sustain: 0.48, filter: 1450 };
  }
  if (instrument === "sine-bell") {
    return { attack: 0.008, sustain: 0.36, filter: 3600 };
  }
  if (instrument === "chip-lead") {
    return { attack: 0.004, sustain: 0.2, filter: 5200 };
  }
  if (instrument === "triangle-lead") {
    return { attack: 0.012, sustain: 0.42, filter: 2400 };
  }
  if (instrument === "saw-lead") {
    return { attack: 0.01, sustain: 0.34, filter: 1700 };
  }
  if (instrument === "soft-square") {
    return { attack: 0.01, sustain: 0.34, filter: 2100 };
  }
  if (instrument === "pad") {
    return { attack: 0.06, sustain: 0.56, filter: 900 };
  }
  if (instrument === "organ") {
    return { attack: 0.02, sustain: 0.72, filter: 1300 };
  }
  if (instrument === "round-bass" || instrument === "wood-bass") {
    return { attack: 0.014, sustain: 0.65, filter: 650 };
  }
  if (instrument === "kick") {
    return { attack: 0.004, sustain: 0.18, filter: 600 };
  }
  if (duration < 0.12) {
    return { attack: 0.005, sustain: 0.25, filter: 2300 };
  }
  return { attack: 0.012, sustain: 0.42, filter: 2400 };
}

function isPlucked(instrument) {
  return instrument === "nylon"
    || instrument === "harp"
    || instrument === "kalimba"
    || instrument === "music-box"
    || instrument === "marimba"
    || instrument === "pluck"
    || instrument === "muted-pluck";
}

function pluckDecayFor(instrument) {
  if (instrument === "nylon") {
    return 4.8;
  }
  if (instrument === "harp") {
    return 5.4;
  }
  if (instrument === "kalimba" || instrument === "music-box") {
    return 6.2;
  }
  if (instrument === "marimba") {
    return 8;
  }
  return 6.2;
}

function pluckBrightnessFor(instrument) {
  if (instrument === "music-box" || instrument === "kalimba") {
    return 0.72;
  }
  if (instrument === "marimba") {
    return 0.34;
  }
  if (instrument === "nylon") {
    return 0.28;
  }
  if (instrument === "harp") {
    return 0.54;
  }
  return 0.5;
}

function pluckFilterFor(instrument) {
  if (instrument === "nylon") {
    return 1500;
  }
  if (instrument === "marimba") {
    return 2200;
  }
  if (instrument === "harp") {
    return 3100;
  }
  if (instrument === "kalimba" || instrument === "music-box") {
    return 4200;
  }
  return 2600;
}

function trackSource(source, activeSources) {
  activeSources.add(source);
  source.addEventListener("ended", () => {
    activeSources.delete(source);
    source.disconnect();
  }, { once: true });
}

function buildTriad(tonic, scale, degree) {
  return [noteAtDegree(tonic, scale, degree), noteAtDegree(tonic, scale, degree + 2), noteAtDegree(tonic, scale, degree + 4)];
}

function voiceChord(chord) {
  return chord.map((note) => note + 12);
}

function noteAtDegree(tonic, scale, degree) {
  const octave = Math.floor(degree / scale.length) * 12;
  const index = ((degree % scale.length) + scale.length) % scale.length;
  return tonic + octave + scale[index];
}

function rotate(items, offset) {
  const copy = [...items];
  return copy.slice(offset).concat(copy.slice(0, offset));
}

function pick(items, rng) {
  return items[Math.floor(rng() * items.length)];
}

function weightedPick(candidates, rng) {
  const total = candidates.reduce((sum, candidate) => sum + candidate.weight, 0);
  let ticket = rng() * total;
  for (const candidate of candidates) {
    ticket -= candidate.weight;
    if (ticket <= 0) {
      return candidate.item;
    }
  }
  return candidates[candidates.length - 1].item;
}

function weightedIndex(weights, rng) {
  const total = weights.reduce((sum, weight) => sum + weight, 0);
  let ticket = rng() * total;
  for (let index = 0; index < weights.length; index += 1) {
    ticket -= weights[index];
    if (ticket <= 0) {
      return index;
    }
  }
  return weights.length - 1;
}

function randomInt(rng, min, max) {
  return Math.floor(rng() * (max - min + 1)) + min;
}

function randomIntFromUnit(value, min, max) {
  return Math.round(min + clamp(value, 0, 1) * (max - min));
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function round2(value) {
  return Math.round(value * 100) / 100;
}

function hashSeed(seed) {
  let hash = 2166136261;
  for (let i = 0; i < seed.length; i += 1) {
    hash ^= seed.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function hashUnit(seed) {
  return hashSeed(seed) / 4294967296;
}

function mulberry32(seed) {
  return function next() {
    let value = seed += 0x6D2B79F5;
    value = Math.imul(value ^ value >>> 15, value | 1);
    value ^= value + Math.imul(value ^ value >>> 7, value | 61);
    return ((value ^ value >>> 14) >>> 0) / 4294967296;
  };
}
