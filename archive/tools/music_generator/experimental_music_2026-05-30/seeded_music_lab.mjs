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
  melody: ["breathy-flute", "nylon", "warm-pluck", "low-pluck", "fuzzy-pluck", "dust-lead", "reed", "soft-square", "harp", "marimba", "music-box", "chip-lead", "triangle-lead", "saw-lead"],
  counter: ["glass", "pluck"],
  harmony: ["pad", "organ", "muted-pluck"],
  bass: ["round-bass", "wood-bass"],
};

const FUNCTIONAL_TIMBRES = [
  "breath-column",
  "reed-column",
  "body-pluck",
  "muted-body-pluck",
  "struck-bar",
  "soft-oscillator",
  "noise-fiber",
  "warm-pad",
  "air-column-pad",
  "round-low",
];

const FUNCTIONAL_TIMBRE_META = {
  "breath-column": { family: "breath", excitation: "air noise plus low-harmonic tone", resonator: "air column", distance: "middle", engine: "breath-additive", gain: 0.86 },
  "reed-column": { family: "reed", excitation: "reed-like odd-harmonic source", resonator: "closed air column", distance: "middle", engine: "odd-additive", gain: 0.72 },
  "body-pluck": { family: "pluck", excitation: "short pluck with body resonance", resonator: "string/body", distance: "middle", engine: "body-pluck", gain: 0.9 },
  "muted-body-pluck": { family: "pluck", excitation: "damped pluck", resonator: "damped string/body", distance: "middle-back", engine: "muted-pluck", gain: 0.98 },
  "struck-bar": { family: "strike", excitation: "mallet-like strike", resonator: "inharmonic bar", distance: "middle", engine: "inharmonic-strike", gain: 0.62 },
  "soft-oscillator": { family: "soft synth", excitation: "oscillator", resonator: "lowpass filter", distance: "middle", engine: "filtered-oscillator", gain: 0.68 },
  "noise-fiber": { family: "noise", excitation: "filtered noise plus weak pitch", resonator: "electronic filter", distance: "background", engine: "noise-pitched", gain: 0.54 },
  "warm-pad": { family: "soft synth", excitation: "slow oscillator", resonator: "lowpass filter", distance: "background", engine: "slow-pad", gain: 0.7 },
  "air-column-pad": { family: "breath", excitation: "soft air noise plus tone", resonator: "air column", distance: "background", engine: "breath-pad", gain: 0.66 },
  "round-low": { family: "low body", excitation: "low oscillator", resonator: "lowpass body", distance: "support", engine: "low-body", gain: 0.86 },
};

const MIN_BPM = 40;
const MAX_BPM = 180;
const DEFAULT_TONE = 0.5;
const DEFAULT_BPM = 110;
const DEFAULT_VOLUME = 0.5;

export const INSTRUMENT_AUDITION_GROUPS = [
  {
    id: "melody",
    label: "Melody",
    track: "lead",
    note: 72,
    noteLabel: "C5",
    durationSteps: 4,
    velocity: 0.16,
    instruments: [...INSTRUMENTS.melody],
  },
  {
    id: "functional",
    label: "Functional Palette",
    track: "lead",
    note: 69,
    noteLabel: "A4",
    durationSteps: 5,
    velocity: 0.16,
    instruments: [...FUNCTIONAL_TIMBRES],
  },
  {
    id: "counter",
    label: "Counter",
    track: "counter",
    note: 67,
    noteLabel: "G4",
    durationSteps: 4,
    velocity: 0.16,
    instruments: [...INSTRUMENTS.counter],
  },
  {
    id: "harmony",
    label: "Harmony",
    track: "chord",
    note: 60,
    noteLabel: "C4",
    durationSteps: 6,
    velocity: 0.16,
    instruments: [...INSTRUMENTS.harmony],
  },
  {
    id: "bass",
    label: "Bass",
    track: "bass",
    note: 48,
    noteLabel: "C3",
    durationSteps: 6,
    velocity: 0.16,
    instruments: [...INSTRUMENTS.bass],
  },
  {
    id: "drums",
    label: "Drums",
    track: "drums",
    noteLabel: "hit",
    durationSteps: 1,
    velocity: 0.18,
    instruments: ["kick", "snare", "hat"],
  },
];

const AUDITION_DRUM_TIMBRES = {
  kick: { kind: "kick", pitchStart: 72, pitchEnd: 38, decay: 0.18, click: 0.14 },
  snare: { kind: "snare", decay: 0.16, snap: 0.46, tone: 0.18, filter: 1700 },
  hat: { kind: "hat", decay: 0.055, brightness: 0.7, filter: 6200 },
};

export function generateSong(seed, options = {}) {
  const seedText = String(seed);
  const rng = mulberry32(hashSeed(seedText));
  const tone = clamp(Number(options.tone ?? DEFAULT_TONE), 0, 1);
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
  const type = arrangementType(parameters.arrangement, parameters.harmony);
  const progression = rotate(pickProgression(rng, tone), randomInt(rng, 0, 3));
  const melodyForm = buildMelodyForm(rng, scaleDegrees.length, generated.phrase, generated.hook);
  grammar.score.hook = melodyForm.hook.score;
  const instruments = {
    melody: pickMelodyInstrument(rng, tone),
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

  const mixPolicy = applyRoleDistancePolicy(events, type, instruments);
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
    mixPolicy,
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
      mixPolicy,
      progression: progressionNumbers,
    },
  };
}

function buildPlaybackScore({ seed, tone, playbackTone, bpm, volume, bars, stepsPerBar, stepDurationBeats, instruments, drums, events }) {
  const timbres = {
    lead: instrumentTimbre(instruments.melody),
    counter: instrumentTimbre(instruments.counter),
    chord: instrumentTimbre(instruments.harmony),
    bass: instrumentTimbre(instruments.bass),
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

function applyRoleDistancePolicy(events, type, instruments) {
  const context = roleDistanceContext(events, type, instruments);
  const policies = roleDistancePolicies(context);
  const decisions = Object.entries(policies)
    .filter(([, policy]) => policy.rules.length > 0)
    .map(([track, policy]) => ({
      track,
      reason: policy.rules.map((rule) => rule.label).join(", "),
      principle: policy.rules.map((rule) => rule.principle).join(" / "),
      rules: policy.rules,
      gain: policy.gain,
      minNote: policy.minNote,
      maxNote: policy.maxNote,
    }));

  for (const event of events) {
    const policy = policies[event.track];
    if (policy) {
      applyEventPolicy(event, policy);
    }
  }

  return {
    version: 2,
    role: "guardrail",
    principle: "A part is only constrained when its acoustic behavior contradicts its musical role.",
    decisions,
  };
}

const ROLE_DISTANCE_BASE = {
  lead: { minNote: 48, maxNote: 88, gain: 1 },
  chord: { minNote: 55, maxNote: 86, gain: 1 },
  counter: { minNote: 50, maxNote: 82, gain: 1 },
  bass: { minNote: 36, maxNote: 62, gain: 1 },
};

const BRIGHT_LEAD_INSTRUMENTS = new Set(["triangle-lead", "chip-lead", "music-box", "saw-lead"]);

const ROLE_DISTANCE_RULES = [
  {
    id: "foreground-congestion",
    track: "lead",
    label: "dense foreground lead",
    principle: "A foreground line can be active, but high density should buy distance in level and register.",
    when: ({ type }) => type.leadPresence > 0.78 && type.leadDensity > 0.7,
    effect: { gain: 0.72, maxNote: 82 },
  },
  {
    id: "nonfocus-lead-dominance",
    track: "lead",
    label: "lead foreground budget outside lead focus",
    principle: "When the arrangement is not lead-focused, lead can carry a motif but should not occupy most of the loop at foreground level.",
    when: ({ type, stats }) => (
      type.focus !== "lead"
      && type.leadPresence > 0.5
      && stats.lead.duty > 0.55
      && stats.lead.avgVelocity > 0.13
    ),
    effect: { gain: 0.82 },
  },
  {
    id: "bright-lead-too-close",
    track: "lead",
    label: "bright lead too close",
    principle: "Bright timbres need extra distance when they are exposed, dense, or already loud.",
    when: ({ instruments, facts, stats }) => (
      BRIGHT_LEAD_INSTRUMENTS.has(instruments.melody)
      && (facts.denseLead || facts.exposedLead || stats.lead.avgVelocity > 0.2)
    ),
    effect: { gain: 0.78, maxNote: 80 },
  },
  {
    id: "bright-lead-occupancy-budget",
    track: "lead",
    label: "bright lead long foreground occupancy",
    principle: "A bright lead can be foreground color, but if it occupies much of the loop it needs distance before it becomes glare.",
    when: ({ instruments, type, stats }) => (
      BRIGHT_LEAD_INSTRUMENTS.has(instruments.melody)
      && type.leadPresence > 0.55
      && stats.lead.duty > 0.45
      && stats.lead.max !== null
      && stats.lead.max > 76
    ),
    effect: { gain: 0.84, maxNote: 78 },
  },
  {
    id: "texture-focus-bright-lead-distance",
    track: "lead",
    label: "bright lead inside texture focus",
    principle: "In texture-focused arrangements, a bright lead should read as a line inside the texture rather than the nearest foreground object.",
    when: ({ instruments, type, stats }) => (
      type.focus === "texture"
      && BRIGHT_LEAD_INSTRUMENTS.has(instruments.melody)
      && type.leadPresence > 0.45
      && stats.lead.duty > 0.35
    ),
    effect: { gain: 0.88, maxNote: 78 },
  },
  {
    id: "unsupported-soft-square",
    track: "lead",
    label: "solo soft-square lead",
    principle: "A plain synthetic tone should not carry an exposed melody without extra distance.",
    when: ({ instruments, facts }) => instruments.melody === "soft-square" && facts.exposedLead,
    effect: { gain: 0.82, maxNote: 80 },
  },
  {
    id: "middle-distance-harmony-arp",
    track: "chord",
    label: "middle-distance harmony arp register",
    principle: "Harmony arps are middle-distance motion, not a second high foreground melody.",
    when: ({ type }) => type.harmony === "arp",
    effect: { maxNote: 84 },
  },
  {
    id: "high-harmony-arp",
    track: "chord",
    label: "high harmony arp",
    principle: "Repeated harmony motion in a high register should step back before it reads as lead.",
    when: ({ type, stats }) => type.harmony === "arp" && stats.chord.max !== null && stats.chord.max > 86,
    effect: { gain: 0.88 },
  },
  {
    id: "percussive-pad-arp",
    track: "chord",
    label: "pad used as short arp",
    principle: "A sustained pad timbre used as a short repeating figure needs more distance.",
    when: ({ type, instruments }) => type.harmony === "arp" && instruments.harmony === "pad",
    effect: { gain: 0.78, maxNote: 80 },
  },
  {
    id: "lead-register-ownership",
    track: "chord",
    label: "yield high register to lead",
    principle: "When lead is present, upper register belongs to lead unless harmony is deliberately foregrounded.",
    when: ({ type, stats }) => type.leadPresence > 0.58 && stats.chord.max !== null && stats.chord.max > 82,
    effect: { gain: 0.9, maxNote: 82 },
  },
  {
    id: "busy-counter-under-lead",
    track: "counter",
    label: "busy counter under active lead",
    principle: "A counter line may add motion, but should recede when it is busy under an active lead.",
    when: ({ type, stats }) => stats.counter.count > 48 && type.leadPresence > 0.4,
    effect: { gain: 0.86 },
  },
  {
    id: "low-frequency-sustain-budget",
    track: "bass",
    label: "sustained bass foreground pulse",
    principle: "Low sustained bass should anchor the loop; if it occupies most of the timeline, it needs distance rather than foreground loudness.",
    when: ({ stats }) => (
      stats.bass.min !== null
      && stats.bass.min < 60
      && stats.bass.avgDuration >= 6
      && stats.bass.duty > 0.52
    ),
    effect: { gain: 0.82 },
  },
];

function roleDistanceContext(events, type, instruments) {
  const stats = trackStats(events);
  return {
    type,
    instruments,
    stats,
    facts: {
      denseLead: type.leadPresence > 0.78 && type.leadDensity > 0.7,
      exposedLead: type.partPresence.harmony < 0.32 && type.partPresence.counter < 0.18,
    },
  };
}

function roleDistancePolicies(context) {
  const policies = Object.fromEntries(Object.entries(ROLE_DISTANCE_BASE)
    .map(([track, base]) => [track, { ...base, rules: [] }]));
  for (const rule of ROLE_DISTANCE_RULES) {
    if (!rule.when(context)) {
      continue;
    }
    applyRoleRule(policies[rule.track], rule);
  }
  return policies;
}

function applyRoleRule(policy, rule) {
  if (rule.effect.gain !== undefined) {
    policy.gain = round2(policy.gain * rule.effect.gain);
  }
  if (rule.effect.minNote !== undefined) {
    policy.minNote = Math.max(policy.minNote, rule.effect.minNote);
  }
  if (rule.effect.maxNote !== undefined) {
    policy.maxNote = Math.min(policy.maxNote, rule.effect.maxNote);
  }
  policy.rules.push({
    id: rule.id,
    label: rule.label,
    principle: rule.principle,
    effect: rule.effect,
  });
}

function applyEventPolicy(event, policy) {
  event.velocity = round2(event.velocity * policy.gain);
  event.notes = event.notes.map((note) => (
    typeof note === "number" ? clampToRegister(note, policy.minNote, policy.maxNote) : note
  ));
}

function trackStats(events) {
  const stats = {};
  const songEnd = Math.max(1, ...events.map((event) => event.step + event.durationSteps));
  for (const track of ["lead", "chord", "counter", "bass", "drums"]) {
    const trackEvents = events.filter((event) => event.track === track);
    const notes = trackEvents.flatMap((event) => event.notes).filter((note) => typeof note === "number");
    const totalDuration = trackEvents.reduce((sum, event) => (
      sum + event.durationSteps * Math.max(1, event.notes.filter((note) => typeof note === "number").length)
    ), 0);
    stats[track] = {
      count: trackEvents.length,
      notes,
      min: notes.length ? Math.min(...notes) : null,
      max: notes.length ? Math.max(...notes) : null,
      avgDuration: totalDuration / Math.max(1, trackEvents.length),
      duty: totalDuration / songEnd,
      avgVelocity: trackEvents.reduce((sum, event) => sum + event.velocity, 0) / Math.max(1, trackEvents.length),
    };
  }
  return stats;
}

export function randomPreset(seed = Date.now()) {
  const rng = mulberry32(hashSeed(String(seed)));
  return {
    seed: randomInt(rng, 100000, 999999).toString(),
    tone: DEFAULT_TONE,
    bpm: DEFAULT_BPM,
  };
}

export function buildInstrumentAuditionScore(options = {}) {
  const tone = clamp(Number(options.tone ?? DEFAULT_TONE), 0, 1);
  const bpm = clamp(Math.round(Number(options.bpm ?? DEFAULT_BPM)), MIN_BPM, MAX_BPM);
  const volume = clamp(Number(options.volume ?? DEFAULT_VOLUME), 0, 1);
  const spacingSteps = Math.max(4, Math.round(Number(options.spacingSteps ?? 8)));
  const selectedIds = options.instruments ? new Set(options.instruments) : null;
  const items = auditionInstruments().filter((item) => !selectedIds || selectedIds.has(item.id));
  const loopSteps = Math.max(16, items.length * spacingSteps + 8);
  const bars = Math.ceil(loopSteps / 16);
  const timbres = {};
  const events = items.map((item, index) => {
    timbres[item.id] = item.timbre;
    return {
      track: item.track,
      step: 2 + index * spacingSteps,
      durationSteps: item.durationSteps,
      notes: [...item.notes],
      timbre: item.id,
      velocity: item.velocity,
    };
  });
  return {
    version: 1,
    source: {
      seed: "instrument-audition",
      tone,
    },
    transport: {
      bpm,
      bars,
      stepsPerBar: 16,
      stepDurationBeats: 0.25,
      loopSteps: bars * 16,
    },
    mix: {
      volume,
      playbackTone: playbackToneFor(tone),
    },
    timbres,
    events,
    audition: items,
  };
}

export function auditionInstruments() {
  return INSTRUMENT_AUDITION_GROUPS.flatMap((group) => group.instruments.map((instrument) => {
    const drum = group.id === "drums";
    return {
      id: `${group.id}:${instrument}`,
      group: group.id,
      groupLabel: group.label,
      instrument,
      track: group.track,
      noteLabel: group.noteLabel,
      durationSteps: group.durationSteps,
      velocity: group.velocity,
      notes: drum ? [instrument] : [group.note],
      timbre: drum ? AUDITION_DRUM_TIMBRES[instrument] : instrumentTimbre(instrument),
    };
  }));
}

function instrumentTimbre(instrument) {
  const meta = FUNCTIONAL_TIMBRE_META[instrument];
  return {
    kind: instrument,
    gain: instrumentOutputGain(instrument),
    ...(meta ? {
      family: meta.family,
      excitation: meta.excitation,
      resonator: meta.resonator,
      distance: meta.distance,
      engine: meta.engine,
    } : {}),
  };
}

function instrumentOutputGain(instrument) {
  if (FUNCTIONAL_TIMBRE_META[instrument]) {
    return FUNCTIONAL_TIMBRE_META[instrument].gain;
  }
  if (instrument === "chip-lead") {
    return 0.65;
  }
  return 1;
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
  const harmony = generateHarmonyGrammar(rng);
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
        harmony,
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

function generateHarmonyGrammar(rng) {
  return {
    pulseRegularity: round2(rng()),
    phaseDrift: round2(rng()),
    gapRate: round2(rng()),
    anchorStrength: round2(rng()),
    barAnswer: round2(rng()),
    densityJitter: round2(rng()),
    directionBias: round2(rng()),
    registerSpread: round2(rng()),
    offbeatBias: round2(rng()),
    stepSalt: randomInt(rng, 0, 9999),
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

function arrangementType(arrangement, harmonyGrammar = defaultHarmonyGrammar()) {
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
    harmonyGrammar,
    counter,
    counterMotion: round2(arrangement.counterAmount * 0.58 + arrangement.harmonyMotion * 0.42),
    counterSpace: round2(arrangement.harmonySpace * 0.62 + (1 - arrangement.leadPresence) * 0.38),
    focus,
    partPresence,
    droppedParts: droppedParts(partPresence),
  };
}

function defaultHarmonyGrammar() {
  return {
    pulseRegularity: 0.74,
    phaseDrift: 0.24,
    gapRate: 0.18,
    anchorStrength: 0.7,
    barAnswer: 0.38,
    densityJitter: 0.42,
    directionBias: 0.5,
    registerSpread: 0.46,
    offbeatBias: 0.28,
    stepSalt: 0,
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

function pickMelodyInstrument(rng, tone) {
  const dark = 1 - tone;
  const bright = tone;
  return weightedPick([
    { item: "warm-pluck", weight: 0.55 + dark * 0.25 },
    { item: "low-pluck", weight: 0.28 + dark * 0.45 },
    { item: "fuzzy-pluck", weight: 0.25 + dark * 0.22 },
    { item: "dust-lead", weight: 0.78 + dark * 0.34 },
    { item: "reed", weight: 0.88 },
    { item: "soft-square", weight: 0.88 },
    { item: "breathy-flute", weight: 0.68 + dark * 0.28 },
    { item: "nylon", weight: 0.78 + dark * 0.34 },
    { item: "harp", weight: 0.44 },
    { item: "marimba", weight: 0.28 + bright * 0.2 },
    { item: "music-box", weight: 0.16 + bright * 0.22 },
    { item: "chip-lead", weight: 0.22 + bright * 0.2 },
    { item: "triangle-lead", weight: 0.26 + bright * 0.22 },
    { item: "saw-lead", weight: 0.34 + bright * 0.24 },
  ], rng);
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
    repeatRate: round2(0.12 + rng() * 0.44),
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
  const cycleRhythmShifts = Array.from({ length: cycleCount }, (_, index) => (
    index === 0 ? 0 : pick([-1, 0, 0, 1], rng)
  ));
  const cycleNoteDrops = Array.from({ length: cycleCount }, (_, index) => (
    index === 0 ? 0 : round2(0.08 + rng() * 0.24)
  ));
  const phraseAnswerShifts = phraseGrammar.contours.map((_, index) => (
    index === 0 ? 0 : randomInt(rng, -2, 2)
  ));
  const contrastMotif = buildContrastMotif(rng, scaleLength, phraseGrammar, hookGrammar, startDegree);
  return {
    lengthBars: hookGrammar.lengthBars,
    startBar: hookGrammar.startBar,
    motif,
    restCycles,
    variantShifts,
    cycleRhythmShifts,
    cycleNoteDrops,
    phraseAnswerShifts,
    contrastMotif,
    score: {
      lengthBars: hookGrammar.lengthBars,
      startBar: hookGrammar.startBar,
      barNoteCounts: hookGrammar.barNoteCounts,
      steps: motif.map((item) => item.b * 16 + item.s),
      degrees: motif.map((item) => item.n),
      accents: motif.map((item) => item.a),
      restCycles,
      variantShifts,
      cycleRhythmShifts,
      cycleNoteDrops,
      phraseAnswerShifts,
      contrastMotif,
    },
  };
}

function buildContrastMotif(rng, scaleLength, phraseGrammar, hookGrammar, startDegree) {
  const lengthBars = 1;
  const startBar = Math.max(0, hookGrammar.lengthBars - lengthBars);
  const center = clamp(
    startDegree + (hookGrammar.answerShift <= 0 ? 2 : -2) + pick([-1, 0, 1], rng),
    0,
    scaleLength - 1,
  );
  const direction = center >= startDegree ? -1 : 1;
  const bars = [];
  for (let bar = 0; bar < lengthBars; bar += 1) {
    const count = hookGrammar.noteCount < 10 ? 2 : randomInt(rng, 2, 3);
    const baseSteps = [2, 5, 9, 12];
    const steps = pickDistinctSteps(rng, baseSteps, count);
    bars.push(steps.map((step, index) => {
      const leap = index % 2 === 0 ? pick([2, 3], rng) : pick([1, 2], rng);
      const degree = index === steps.length - 1 && bar === lengthBars - 1
        ? phraseGrammar.finalCadenceDegree + pick([0, 1], rng)
        : center + direction * leap + pick([-1, 0, 1], rng);
      return {
        s: step,
        d: index === steps.length - 1 ? pick([2, 3], rng) : pick([1, 2], rng),
        n: clamp(degree, 0, scaleLength - 1),
        a: round2(index === 0 ? 0.86 : index === steps.length - 1 ? 0.78 : 0.72),
      };
    }));
  }
  return {
    startBar,
    lengthBars,
    bars,
    summary: `replace:${startBar}-${startBar + lengthBars - 1}`,
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
  const grammar = type.harmonyGrammar ?? defaultHarmonyGrammar();
  const sparse = type.harmonySpace > 0.62;
  const baseCount = sparse
    ? type.harmonyPresence < 0.72 ? 2 : 3
    : type.harmonyPresence < 0.42 ? 2 : type.harmonyPresence < 0.68 ? 3 : 4;
  const densityWave = Math.sin((localBar + 1) * (0.9 + grammar.densityJitter * 1.7) + grammar.stepSalt * 0.01);
  const countOffset = densityWave > 0.58 && grammar.densityJitter > 0.48 ? 1 : densityWave < -0.42 || grammar.gapRate > 0.72 ? -1 : 0;
  const maxCount = type.leadPresence > 0.58 || type.harmonySpace > 0.7 ? 3 : 4;
  const count = clamp(baseCount + countOffset, 1, maxCount);
  const phase = harmonyPhase(grammar, type, localBar);
  const candidates = Array.from({ length: 16 }, (_, step) => ({
    step,
    score: harmonyStepScore(grammar, type, localBar, step, phase),
  })).sort((a, b) => b.score - a.score || a.step - b.step);
  const steps = [];
  for (const candidate of candidates) {
    const adjacent = steps.some((step) => Math.abs(step - candidate.step) <= 1);
    if (adjacent && grammar.registerSpread < 0.58 && steps.length < count - 1) {
      continue;
    }
    steps.push(candidate.step);
    if (steps.length >= count) {
      break;
    }
  }
  return steps.sort((a, b) => a - b);
}

function arpOrder(type, localBar) {
  const grammar = type.harmonyGrammar ?? defaultHarmonyGrammar();
  const length = type.harmonyPresence > 0.68 && type.harmonySpace < 0.66 ? 4 : 3;
  const start = grammar.directionBias > 0.62
    ? 0
    : grammar.directionBias < 0.38
      ? 2
      : localBar % 2;
  const direction = grammar.directionBias >= 0.5 ? 1 : -1;
  const answer = grammar.barAnswer > 0.5 && localBar % 4 >= 2 ? -direction : direction;
  const order = [];
  let current = start;
  for (let index = 0; index < length; index += 1) {
    order.push(clamp(current, 0, 2));
    const leap = grammar.registerSpread > 0.66 && index % 2 === 0 ? 2 : 1;
    current += answer * leap;
    if (current > 2) {
      current = grammar.anchorStrength > 0.5 ? 1 : 0;
    }
    if (current < 0) {
      current = grammar.anchorStrength > 0.5 ? 1 : 2;
    }
  }
  return order;
}

function harmonyPhase(grammar, type, localBar) {
  const driftAmount = Math.round(grammar.phaseDrift * 4);
  const answerShift = grammar.barAnswer > 0.5 && localBar % 4 >= 2 ? Math.round(grammar.barAnswer * 3) : 0;
  const motionShift = type.harmonyMotion > 0.5 ? Math.round((localBar % 4) * type.harmonyMotion) : localBar % 2;
  return (driftAmount * localBar + answerShift + motionShift + grammar.stepSalt) % 4;
}

function harmonyStepScore(grammar, type, localBar, step, phase) {
  const pulseDistance = circularDistance(step % 4, phase % 4, 4);
  const pulseScore = (1 - pulseDistance / 2) * (0.8 + grammar.pulseRegularity * 1.2);
  const anchorScore = (step === 0 || step === 8 ? 1 : step % 4 === 0 ? 0.64 : 0) * grammar.anchorStrength;
  const offbeat = step % 4 === 1 || step % 4 === 3;
  const offbeatScore = offbeat ? grammar.offbeatBias * (0.9 + type.harmonyMotion * 0.6) : 0;
  const answerCenter = localBar % 4 >= 2 ? 10 + grammar.barAnswer * 3 : 4 + grammar.barAnswer * 2;
  const contourScore = (1 - Math.min(1, Math.abs(step - answerCenter) / 10)) * grammar.barAnswer * 0.7;
  const noise = hashUnit(`${grammar.stepSalt}:arp:${localBar}:${step}`);
  const gapPenalty = noise < grammar.gapRate * 0.28 ? 0.9 + grammar.gapRate * 0.8 : 0;
  const leadSpacePenalty = type.leadPresence > 0.58 && step >= 8 && step <= 12 ? 0.28 : 0;
  return pulseScore + anchorScore + offbeatScore + contourScore + noise * 0.34 - gapPenalty - leadSpacePenalty;
}

function circularDistance(a, b, size) {
  const distance = Math.abs(a - b);
  return Math.min(distance, size - distance);
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
  const register = melodyRegisterFor(instruments.melody, tone);
  const focusLevel = type.focus === "lead" ? 1 : type.focus === "groove" ? 0.84 : type.focus === "space" ? 0.78 : 0.9;
  const velocity = (0.1 + type.partPresence.lead * 0.17) * (0.78 + section.intensity * 0.34) * focusLevel;
  const melodyItems = hookItemsForBar(form.hook, section, leadLocalBar, type);
  for (const item of melodyItems) {
    const cadence = leadLocalBar === 7 && item.end;
    const degree = cadence ? chordDegree + section.transpose : item.n + section.transpose;
    const duration = item.d + (cadence ? 2 : 0);
    const note = clampToRegister(noteAtDegree(tonic, scale, degree) + register.octave, register.min, register.max);
    events.push({
      track: "lead",
      step: barStart + item.s,
      durationSteps: duration,
      notes: [note],
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
  const phraseShift = hook.phraseAnswerShifts[section.phrase % hook.phraseAnswerShifts.length] ?? 0;
  const rhythmShift = hook.cycleRhythmShifts[(cycle + section.phrase) % hook.cycleRhythmShifts.length] ?? 0;
  const noteDropRate = hook.cycleNoteDrops[(cycle + section.phrase) % hook.cycleNoteDrops.length] ?? 0;
  if (shouldUseContrastMotif(hook, section, cycle, cycleBar, type)) {
    const contrastBar = hook.contrastMotif.bars[cycleBar - hook.contrastMotif.startBar] ?? [];
    return contrastBar.map((item) => ({
      ...item,
      n: item.n + phraseShift,
      contrastMotif: true,
    }));
  }
  const items = hook.motif
    .filter((item) => item.b === cycleBar)
    .map((item, index, filtered) => ({
      ...item,
      s: index === 0 ? item.s : clamp(item.s + rhythmShift, 0, 15),
      n: item.n + shift + phraseShift,
      end: cycleBar === hook.lengthBars - 1 && index === filtered.length - 1,
    }))
    .filter((item, index, filtered) => {
      if (filtered.length <= 2 || item.end || index === 0) {
        return true;
      }
      const dropSlot = Math.floor(noteDropRate * 10);
      return ((index * 3 + cycle + section.phrase) % 10) >= dropSlot;
    });
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

function shouldUseContrastMotif(hook, section, cycle, cycleBar, type) {
  const contrast = hook.contrastMotif;
  if (!contrast || cycleBar < contrast.startBar || cycleBar >= contrast.startBar + contrast.lengthBars) {
    return false;
  }
  if (type.leadPresence < 0.34 || type.leadDensity > 0.9) {
    return false;
  }
  if (section.intensity < 0.54) {
    return false;
  }
  const phraseHasMoved = section.phrase > 0 || section.intensity > 0.72 || Math.abs(section.transpose) >= 2;
  if (hook.lengthBars < 8) {
    return cycle === Math.max(1, Math.floor((8 / hook.lengthBars) * 0.5)) && phraseHasMoved;
  }
  return phraseHasMoved && section.intensity > 0.62;
}

function melodyRegisterFor(instrument, tone) {
  if (instrument === "low-pluck") {
    return { octave: -12, min: 45, max: 72 };
  }
  if (instrument === "warm-pluck" || instrument === "fuzzy-pluck") {
    return { octave: tone > 0.82 ? 0 : -12, min: 48, max: 78 };
  }
  if (instrument === "dust-lead" || instrument === "reed" || instrument === "soft-square" || instrument === "breathy-flute") {
    return { octave: 0, min: 52, max: 82 };
  }
  if (instrument === "music-box" || instrument === "marimba" || instrument === "chip-lead" || instrument === "triangle-lead") {
    return { octave: tone > 0.78 ? 12 : 0, min: 56, max: 88 };
  }
  return { octave: tone > 0.88 ? 12 : 0, min: 52, max: 84 };
}

function clampToRegister(note, min, max) {
  let adjusted = note;
  while (adjusted > max) {
    adjusted -= 12;
  }
  while (adjusted < min) {
    adjusted += 12;
  }
  return adjusted;
}

function addCounter(events, barStart, tonic, scale, type, instruments, section, localBar, tone) {
  if (!type.counter || type.partPresence.counter < 0.18 || localBar < section.entryDelayBars || !counterActive(type, section, localBar)) {
    return;
  }
  const octave = tone > 0.72 ? 12 : 0;
  const presenceLevel = 0.54 + type.partPresence.counter * 0.56;
  const phrase = counterPhrase(type, section, localBar);
  for (const item of phrase) {
    events.push({
      track: "counter",
      step: barStart + item.step,
      durationSteps: item.duration,
      notes: [clampToRegister(noteAtDegree(tonic, scale, item.degree + section.transpose) + octave, 55, 84)],
      instrument: instruments.counter,
      velocity: (0.072 + section.intensity * 0.034) * presenceLevel * item.accent,
    });
  }
}

function counterActive(type, section, localBar) {
  const shifted = (localBar + section.phrase) % 8;
  if (type.counterSpace > 0.72) {
    return shifted === 2 || (section.intensity > 0.74 && shifted === 6);
  }
  if (type.counterMotion < 0.28) {
    return shifted === 2 || shifted === 6;
  }
  if (type.counterMotion < 0.62) {
    return [1, 4, 6].includes(shifted);
  }
  return ![0, 3].includes(shifted);
}

function counterPhrase(type, section, localBar) {
  const patterns = [
    [{ step: 2, degree: 0, duration: 2, accent: 0.92 }, { step: 9, degree: 2, duration: 2, accent: 0.78 }, { step: 13, degree: 1, duration: 2, accent: 0.72 }],
    [{ step: 1, degree: 0, duration: 3, accent: 0.84 }, { step: 7, degree: 1, duration: 2, accent: 0.72 }, { step: 12, degree: 2, duration: 2, accent: 0.78 }],
    [{ step: 3, degree: 2, duration: 2, accent: 0.78 }, { step: 8, degree: 0, duration: 3, accent: 0.86 }, { step: 14, degree: 1, duration: 1, accent: 0.68 }],
    [{ step: 0, degree: 0, duration: 2, accent: 0.76 }, { step: 5, degree: 2, duration: 2, accent: 0.7 }, { step: 10, degree: 1, duration: 2, accent: 0.82 }, { step: 15, degree: 2, duration: 1, accent: 0.62 }],
    [{ step: 4, degree: 0, duration: 4, accent: 0.86 }, { step: 12, degree: 2, duration: 2, accent: 0.72 }],
    [{ step: 2, degree: 1, duration: 2, accent: 0.72 }, { step: 6, degree: 0, duration: 2, accent: 0.84 }, { step: 11, degree: 2, duration: 3, accent: 0.76 }],
  ];
  const index = Math.floor(clamp(type.counterMotion * 3 + type.counterSpace * 1.5 + section.phrase + localBar * 0.5, 0, patterns.length - 0.001)) % patterns.length;
  const phrase = patterns[index];
  if (type.counterSpace > 0.62 && phrase.length > 2) {
    return phrase.filter((_, itemIndex) => itemIndex !== 1);
  }
  return phrase;
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
  const gain = eventTone.gain * volume * (timbre.gain ?? instrumentOutputGain(instrument));
  for (const note of event.notes) {
    if (typeof note === "number") {
      const playbackNote = note + (eventTone.pitchShift ?? 0);
      if (instrument === "spectral-field") {
        playSpectralField(audioContext, playbackNote, startsAt, duration, timbre, event.velocity * gain, eventTone, activeSources);
      } else if (isFunctionalTimbre(instrument)) {
        playFunctionalTimbre(audioContext, playbackNote, startsAt, duration, instrument, event.velocity * gain, eventTone.filter, activeSources);
      } else if (isPlucked(instrument)) {
        playPluck(audioContext, playbackNote, startsAt, duration, instrument, event.velocity * gain, eventTone.filter, activeSources);
      } else {
        playTone(audioContext, playbackNote, startsAt, duration, instrument, event.velocity * gain, eventTone.filter, activeSources);
      }
    } else {
      playNoise(audioContext, startsAt, duration, timbre, event.velocity * gain, eventTone, activeSources);
    }
  }
}

function toneForEvent(event, tone) {
  let result;
  if (event.track === "bass") {
    result = { gain: tone.bassGain, filter: tone.bassFilter };
  } else if (event.track === "lead" || event.track === "counter") {
    result = { gain: tone.leadGain, filter: tone.toneFilter };
  } else if (event.track === "chord") {
    result = { gain: tone.harmonyGain, filter: tone.toneFilter };
  } else if (event.timbre === "hat" || event.timbre === "snare") {
    result = { gain: tone.highPercussionGain, filter: tone.noiseFilter };
  } else {
    result = { gain: tone.lowPercussionGain, filter: tone.bassFilter };
  }
  return {
    gain: result.gain * rolePlaybackGain(event.role, tone),
    filter: result.filter,
    pitchShift: tone.pitchShift ?? 0,
    brightnessTilt: tone.brightnessTilt ?? 0,
    attackShape: tone.attackShape ?? 0,
  };
}

function rolePlaybackGain(role, tone) {
  if (role === "identity") {
    return tone.identityGain ?? 1;
  }
  if (role === "time") {
    return tone.timeGain ?? 1;
  }
  if (role === "color") {
    return tone.colorGain ?? 1;
  }
  if (role === "boundary") {
    return tone.boundaryGain ?? 1;
  }
  return 1;
}

function isFunctionalTimbre(instrument) {
  return Boolean(FUNCTIONAL_TIMBRE_META[instrument]);
}

function playFunctionalTimbre(audioContext, midiNote, startsAt, duration, instrument, velocity, filterMultiplier, activeSources) {
  if (instrument === "body-pluck" || instrument === "muted-body-pluck") {
    playPluck(audioContext, midiNote, startsAt, duration, instrument, velocity, filterMultiplier, activeSources);
    return;
  }
  if (instrument === "struck-bar") {
    playAdditiveTone(audioContext, midiNote, startsAt, duration, velocity, filterMultiplier, activeSources, {
      partials: [
        { ratio: 1, gain: 1, decay: 14 },
        { ratio: 2.74, gain: 0.44, decay: 18 },
        { ratio: 5.36, gain: 0.22, decay: 24 },
      ],
      attack: 0.004,
      sustain: 0.02,
      release: 0.05,
      filter: 2800,
      filterType: "bandpass",
      q: 0.85,
    });
    return;
  }
  if (instrument === "reed-column") {
    playAdditiveTone(audioContext, midiNote, startsAt, duration, velocity, filterMultiplier, activeSources, {
      partials: [
        { ratio: 1, gain: 0.9, decay: 1.7 },
        { ratio: 3, gain: 0.42, decay: 2.2 },
        { ratio: 5, gain: 0.18, decay: 2.8 },
      ],
      attack: 0.018,
      sustain: 0.54,
      release: 0.07,
      filter: 1350,
      filterType: "lowpass",
      q: 0.55,
      vibratoCents: 3,
      vibratoRate: 4.8,
    });
    playDustNoise(audioContext, startsAt, duration, velocity * 0.12, filterMultiplier, activeSources);
    return;
  }
  if (instrument === "breath-column") {
    playAdditiveTone(audioContext, midiNote, startsAt, duration, velocity, filterMultiplier, activeSources, {
      partials: [
        { ratio: 1, gain: 1, decay: 1.5 },
        { ratio: 2, gain: 0.14, decay: 2.1 },
        { ratio: 3, gain: 0.08, decay: 2.6 },
      ],
      attack: 0.07,
      sustain: 0.56,
      release: 0.08,
      filter: 1180,
      filterType: "lowpass",
      q: 0.4,
      vibratoCents: 4,
      vibratoRate: 5.4,
    });
    playBreathNoise(audioContext, startsAt, duration, velocity * 0.18, filterMultiplier, activeSources);
    return;
  }
  if (instrument === "air-column-pad") {
    playAdditiveTone(audioContext, midiNote, startsAt, duration, velocity, filterMultiplier, activeSources, {
      partials: [
        { ratio: 1, gain: 1, decay: 0.7 },
        { ratio: 2, gain: 0.1, decay: 0.9 },
      ],
      attack: 0.14,
      sustain: 0.66,
      release: 0.14,
      filter: 860,
      filterType: "lowpass",
      q: 0.35,
      vibratoCents: 2,
      vibratoRate: 4.2,
    });
    playBreathNoise(audioContext, startsAt, duration, velocity * 0.1, filterMultiplier, activeSources);
    return;
  }
  if (instrument === "noise-fiber") {
    playAdditiveTone(audioContext, midiNote, startsAt, duration, velocity * 0.28, filterMultiplier, activeSources, {
      partials: [{ ratio: 1, gain: 1, decay: 2.4 }],
      attack: 0.025,
      sustain: 0.34,
      release: 0.08,
      filter: 900,
      filterType: "bandpass",
      q: 0.9,
    });
    playDustNoise(audioContext, startsAt, duration, velocity * 0.52, filterMultiplier, activeSources);
    return;
  }
  if (instrument === "round-low") {
    playAdditiveTone(audioContext, midiNote - 12, startsAt, duration, velocity, filterMultiplier, activeSources, {
      partials: [
        { ratio: 1, gain: 1, decay: 1.1 },
        { ratio: 2, gain: 0.18, decay: 1.7 },
      ],
      attack: 0.018,
      sustain: 0.68,
      release: 0.08,
      filter: 680,
      filterType: "lowpass",
      q: 0.45,
    });
    return;
  }
  if (instrument === "warm-pad") {
    playAdditiveTone(audioContext, midiNote, startsAt, duration, velocity, filterMultiplier, activeSources, {
      partials: [
        { ratio: 1, gain: 0.78, decay: 0.5 },
        { ratio: 1.005, gain: 0.28, decay: 0.6 },
        { ratio: 2, gain: 0.12, decay: 0.8 },
      ],
      attack: 0.12,
      sustain: 0.72,
      release: 0.16,
      filter: 780,
      filterType: "lowpass",
      q: 0.42,
      vibratoCents: 1.5,
      vibratoRate: 3.2,
    });
    return;
  }
  playAdditiveTone(audioContext, midiNote, startsAt, duration, velocity, filterMultiplier, activeSources, {
    partials: [
      { ratio: 1, gain: 0.9, decay: 1.4 },
      { ratio: 2, gain: 0.2, decay: 2.4 },
    ],
    attack: 0.016,
    sustain: 0.38,
    release: 0.06,
    filter: 1450,
    filterType: "lowpass",
    q: 0.4,
  });
}

function playAdditiveTone(audioContext, midiNote, startsAt, duration, velocity, filterMultiplier, activeSources, config) {
  const output = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  filter.type = config.filterType ?? "lowpass";
  filter.frequency.value = (config.filter ?? 1400) * filterMultiplier;
  filter.Q.value = config.q ?? 0.5;
  output.connect(filter).connect(audioContext.destination);
  for (const partial of config.partials) {
    const oscillator = audioContext.createOscillator();
    const gain = audioContext.createGain();
    oscillator.type = "sine";
    oscillator.frequency.setValueAtTime(midiToFrequency(midiNote) * partial.ratio, startsAt);
    gain.gain.setValueAtTime(0.0001, startsAt);
    gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, velocity * partial.gain), startsAt + config.attack);
    gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, velocity * partial.gain * config.sustain), startsAt + Math.max(config.attack + 0.02, duration * 0.48));
    gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.05, duration + (config.release ?? 0.06)));
    if (partial.decay) {
      gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.min(Math.max(0.05, duration + (config.release ?? 0.06)), 1 / partial.decay + duration * 0.72));
    }
    if (config.vibratoCents) {
      const lfo = audioContext.createOscillator();
      const lfoGain = audioContext.createGain();
      lfo.frequency.value = config.vibratoRate ?? 5;
      lfoGain.gain.value = config.vibratoCents;
      lfo.connect(lfoGain).connect(oscillator.detune);
      trackSource(lfo, activeSources);
      lfo.start(startsAt);
      lfo.stop(startsAt + duration + 0.1);
    }
    oscillator.connect(gain).connect(output);
    trackSource(oscillator, activeSources);
    oscillator.start(startsAt);
    oscillator.stop(startsAt + duration + 0.1);
  }
}

function playSpectralField(audioContext, midiNote, startsAt, duration, timbre, velocity, tone, activeSources) {
  const signal = timbre.signal ?? {};
  const output = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  const envelopeConfig = signal.envelope ?? { attack: 0.02, sustain: 0.5, release: 0.08, durationScale: 0.7 };
  const effectiveDuration = duration * (envelopeConfig.durationScale ?? 0.7);
  const filterMultiplier = tone.filter ?? 1;
  filter.type = signal.filter?.type ?? "lowpass";
  filter.frequency.setValueAtTime((signal.filter?.frequency ?? 1600) * filterMultiplier, startsAt);
  if (signal.filter?.endFrequency) {
    filter.frequency.exponentialRampToValueAtTime(Math.max(20, signal.filter.endFrequency * filterMultiplier), startsAt + effectiveDuration);
  }
  filter.Q.value = signal.filter?.q ?? 0.4;
  output.connect(filter).connect(audioContext.destination);
  const sourceInput = connectPlaybackBody(audioContext, output, signal.body, startsAt, effectiveDuration, activeSources);
  const partials = spectralPartialsForBrightness(signal.partials ?? [[1, 1]], tone.brightnessTilt ?? 0);

  for (const partial of partials) {
    playSpectralPartial(audioContext, midiNote, startsAt, effectiveDuration, partial, signal.pitch, velocity, envelopeConfig, sourceInput, activeSources);
  }
  if (signal.noise) {
    playSpectralNoise(audioContext, startsAt, effectiveDuration, signal.noise, velocity, filterMultiplier, sourceInput, activeSources);
  }
}

function spectralPartialsForBrightness(partials, tilt) {
  if (!tilt) {
    return partials;
  }
  let baseEnergy = 0;
  let tiltedEnergy = 0;
  const tilted = partials.map((partial) => {
    const ratio = Math.max(1, partial[0] ?? 1);
    const amount = partial[1] ?? 0;
    const logRatio = Math.log2(ratio);
    const usefulBrightness = Math.min(logRatio, 2.35);
    const glare = Math.max(0, logRatio - 2.8);
    const brightnessCurve = usefulBrightness - glare * glare * 0.85;
    const nextAmount = amount * 2 ** (tilt * brightnessCurve);
    baseEnergy += amount * amount;
    tiltedEnergy += nextAmount * nextAmount;
    return [ratio, nextAmount, partial[2]];
  });
  if (baseEnergy <= 0 || tiltedEnergy <= 0) {
    return tilted;
  }
  const energyScale = Math.sqrt(baseEnergy / tiltedEnergy);
  return tilted.map(([ratio, amount, decay]) => [ratio, amount * energyScale, decay]);
}

function playSpectralPartial(audioContext, midiNote, startsAt, duration, partial, pitch, velocity, envelopeConfig, destination, activeSources) {
  const [ratio, amount, decay] = partial;
  const oscillator = audioContext.createOscillator();
  const gain = audioContext.createGain();
  oscillator.type = "sine";
  oscillator.frequency.setValueAtTime(midiToFrequency(midiNote) * ratio, startsAt);
  applyPlaybackPitchMotion(audioContext, oscillator, startsAt, duration, pitch, activeSources);
  gain.gain.setValueAtTime(0.0001, startsAt);
  gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, velocity * amount), startsAt + envelopeConfig.attack);
  gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, velocity * amount * envelopeConfig.sustain), startsAt + Math.max(envelopeConfig.attack + 0.02, duration * 0.55));
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + duration + envelopeConfig.release);
  if (decay) {
    gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.03, 1 / decay + duration * 0.35));
  }
  oscillator.connect(gain).connect(destination);
  trackSource(oscillator, activeSources);
  oscillator.start(startsAt);
  oscillator.stop(startsAt + duration + 0.18);
}

function playSpectralNoise(audioContext, startsAt, duration, noise, velocity, filterMultiplier, destination, activeSources) {
  const samples = Math.max(1, Math.floor(audioContext.sampleRate * Math.max(duration, noise.decay ?? 0.06)));
  const buffer = audioContext.createBuffer(1, samples, audioContext.sampleRate);
  const data = buffer.getChannelData(0);
  const rng = mulberry32(hashSeed(`spectral-noise:${startsAt}:${duration}:${noise.role}:${noise.filter?.frequency ?? 0}`));
  for (let i = 0; i < samples; i += 1) {
    const t = i / audioContext.sampleRate;
    const decay = noise.role === "attack" ? Math.exp(-t / (noise.decay ?? 0.06)) : Math.exp(-t * 0.7);
    data[i] = (rng() * 2 - 1) * decay;
  }
  const source = audioContext.createBufferSource();
  const gain = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  filter.type = noise.filter?.type ?? "bandpass";
  filter.frequency.value = (noise.filter?.frequency ?? 1500) * filterMultiplier;
  filter.Q.value = noise.filter?.q ?? 0.7;
  gain.gain.setValueAtTime(Math.max(0.0001, velocity * (noise.gain ?? 0.1)), startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.04, duration));
  source.buffer = buffer;
  source.connect(filter).connect(gain).connect(destination);
  trackSource(source, activeSources);
  source.start(startsAt);
}

function applyPlaybackPitchMotion(audioContext, oscillator, startsAt, duration, pitch, activeSources) {
  if (!pitch) {
    return;
  }
  if (pitch.vibratoCents) {
    const lfo = audioContext.createOscillator();
    const amount = audioContext.createGain();
    lfo.frequency.value = pitch.vibratoRate ?? 5;
    amount.gain.value = pitch.vibratoCents;
    lfo.connect(amount).connect(oscillator.detune);
    trackSource(lfo, activeSources);
    lfo.start(startsAt);
    lfo.stop(startsAt + duration + 0.18);
  }
  if (pitch.jitterCents) {
    const rate = pitch.jitterRate ?? 16;
    const steps = Math.floor(duration * rate);
    let seed = 2166136261;
    for (let index = 0; index <= steps; index += 1) {
      seed = Math.imul(seed ^ (index + 31), 16777619);
      const value = ((seed >>> 0) / 4294967295 - 0.5) * pitch.jitterCents * 2;
      oscillator.detune.setValueAtTime(value, startsAt + index / rate);
    }
  }
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
  if (instrument === "breath-column" || instrument === "air-column-pad") {
    playBreathNoise(audioContext, startsAt, duration, velocity * (instrument === "air-column-pad" ? 0.14 : 0.18), filterMultiplier, activeSources);
  }
  if (instrument === "chip-lead") {
    playChipClick(audioContext, startsAt, velocity * 0.22, filterMultiplier, activeSources);
  }
  if (instrument === "dust-lead" || instrument === "reed") {
    playDustNoise(audioContext, startsAt, duration, velocity * (instrument === "dust-lead" ? 0.32 : 0.18), filterMultiplier, activeSources);
  }
  if (instrument === "reed-column" || instrument === "noise-fiber") {
    playDustNoise(audioContext, startsAt, duration, velocity * (instrument === "noise-fiber" ? 0.34 : 0.14), filterMultiplier, activeSources);
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
  const pluckVariant = instrument === "warm-pluck" || instrument === "low-pluck" || instrument === "fuzzy-pluck";
  const bodyPluck = instrument === "body-pluck" || instrument === "muted-body-pluck";
  const struckBar = instrument === "struck-bar";
  const bodyMix = instrument === "harp" || instrument === "nylon" || pluckVariant || bodyPluck ? 0.66 : instrument === "marimba" || struckBar ? 0.48 : 0.55;
  const upperMix = pluckVariant || bodyPluck ? 0.12 : instrument === "kalimba" || instrument === "music-box" || struckBar ? 0.34 : instrument === "harp" ? 0.18 : 0.24;
  const rng = mulberry32(hashSeed(`${instrument}:${midiNote}:${duration}`));
  for (let i = 0; i < samples; i += 1) {
    const t = i / sampleRate;
    const env = Math.exp(-decay * t);
    const body = Math.sin(2 * Math.PI * frequency * t);
    const detunedBody = Math.sin(2 * Math.PI * frequency * (pluckVariant || bodyPluck ? 0.997 : 1.006) * t) * (pluckVariant || bodyPluck ? 0.24 : 0.18);
    const upperRatio = instrument === "marimba" || struckBar ? 2.7 : pluckVariant || bodyPluck ? 1.52 : 2.01;
    const upper = Math.sin(2 * Math.PI * frequency * upperRatio * t) * bright;
    const click = Math.exp(-90 * t) * Math.sin(2 * Math.PI * frequency * 7 * t) * bright * 0.35;
    const wood = instrument === "marimba" || struckBar ? Math.exp(-28 * t) * Math.sin(2 * Math.PI * frequency * 0.52 * t) * 0.28 : 0;
    const scrape = (rng() * 2 - 1) * Math.exp(-(pluckVariant || bodyPluck ? 12 : 38) * t) * (pluckVariant || bodyPluck ? 0.075 : instrument === "nylon" || instrument === "harp" ? 0.035 : 0.018);
    const lowBody = pluckVariant || bodyPluck ? Math.sin(2 * Math.PI * frequency * 0.5 * t) * 0.12 : 0;
    data[i] = (body * bodyMix + detunedBody + lowBody + upper * upperMix + click + wood + scrape) * env;
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
  if (pluckVariant || bodyPluck) {
    playStringNoise(audioContext, startsAt, duration, velocity * (instrument === "fuzzy-pluck" ? 0.22 : bodyPluck ? 0.08 : 0.1), filterMultiplier, activeSources);
  }
  if (instrument === "fuzzy-pluck") {
    playDustNoise(audioContext, startsAt, duration, velocity * 0.28, filterMultiplier, activeSources);
  }
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

function playStringNoise(audioContext, startsAt, duration, velocity, filterMultiplier, activeSources) {
  const samples = Math.max(1, Math.floor(audioContext.sampleRate * Math.max(0.08, duration * 0.8)));
  const buffer = audioContext.createBuffer(1, samples, audioContext.sampleRate);
  const data = buffer.getChannelData(0);
  const rng = mulberry32(hashSeed(`string:${startsAt}:${duration}`));
  for (let i = 0; i < samples; i += 1) {
    const t = i / audioContext.sampleRate;
    const burst = Math.exp(-22 * t);
    const scrape = Math.exp(-4.8 * t);
    data[i] = (rng() * 2 - 1) * (burst * 0.7 + scrape * 0.3);
  }
  const source = audioContext.createBufferSource();
  const gain = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  filter.type = "bandpass";
  filter.frequency.value = 1200 * filterMultiplier;
  filter.Q.value = 0.72;
  gain.gain.setValueAtTime(Math.max(0.0001, velocity), startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.06, duration * 0.9));
  source.buffer = buffer;
  source.connect(filter).connect(gain).connect(audioContext.destination);
  trackSource(source, activeSources);
  source.start(startsAt);
}

function playDustNoise(audioContext, startsAt, duration, velocity, filterMultiplier, activeSources) {
  const samples = Math.max(1, Math.floor(audioContext.sampleRate * Math.max(0.05, duration)));
  const buffer = audioContext.createBuffer(1, samples, audioContext.sampleRate);
  const data = buffer.getChannelData(0);
  const rng = mulberry32(hashSeed(`dust:${startsAt}:${duration}`));
  for (let i = 0; i < samples; i += 1) {
    const t = i / audioContext.sampleRate;
    const env = Math.exp(-3.2 * t);
    const crackle = rng() > 0.86 ? (rng() * 2 - 1) * 0.9 : 0;
    data[i] = ((rng() * 2 - 1) * 0.42 + crackle) * env;
  }
  const source = audioContext.createBufferSource();
  const gain = audioContext.createGain();
  const filter = audioContext.createBiquadFilter();
  filter.type = "bandpass";
  filter.frequency.value = 1800 * filterMultiplier;
  filter.Q.value = 0.48;
  gain.gain.setValueAtTime(0.0001, startsAt);
  gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, velocity), startsAt + 0.012);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.05, duration));
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

function playNoise(audioContext, startsAt, duration, timbre, velocity, tone, activeSources) {
  const filterMultiplier = typeof tone === "number" ? tone : tone.filter ?? 1;
  const instrument = timbre.kind;
  if (instrument === "transient-field") {
    playTransientField(audioContext, startsAt, duration, timbre, velocity, tone, activeSources);
    return;
  }
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

function playTransientField(audioContext, startsAt, duration, timbre, velocity, tone, activeSources) {
  const signal = timbre.signal ?? {};
  const output = audioContext.createGain();
  const envelopeConfig = signal.envelope ?? { attack: 0.004, decay: 0.18, release: 0.03 };
  const filterMultiplier = typeof tone === "number" ? tone : tone.filter ?? 1;
  const attackShape = typeof tone === "number" ? 0 : tone.attackShape ?? 0;
  const attackScale = 2 ** (-attackShape * 1.4);
  const decayScale = 2 ** (-attackShape * 0.75);
  const attackDuration = Math.max(0.001, envelopeConfig.attack * attackScale);
  const transientDecay = Math.max(0.02, envelopeConfig.decay * decayScale);
  const effectiveDuration = Math.min(Math.max(0.03, duration + transientDecay), transientDecay + envelopeConfig.release + 0.16);
  output.gain.setValueAtTime(0.0001, startsAt);
  output.gain.exponentialRampToValueAtTime(Math.max(0.0001, velocity), startsAt + attackDuration);
  output.gain.exponentialRampToValueAtTime(0.0001, startsAt + effectiveDuration);
  const sourceInput = connectPlaybackBody(audioContext, output, signal.body, startsAt, effectiveDuration, activeSources);
  output.connect(audioContext.destination);

  const noiseDuration = Math.max(0.04, effectiveDuration + 0.04);
  for (const band of signal.bands ?? []) {
    playTransientBand(audioContext, startsAt, noiseDuration, band, filterMultiplier, sourceInput, activeSources, attackShape);
  }
  if (signal.click) {
    playTransientBand(audioContext, startsAt, Math.max(0.025, signal.click.decay * decayScale + 0.02), signal.click, filterMultiplier, sourceInput, activeSources, attackShape, true);
  }
  for (const resonator of signal.resonators ?? []) {
    playTransientFieldResonator(audioContext, startsAt, resonator, sourceInput, activeSources, attackShape);
  }
}

function playTransientBand(audioContext, startsAt, duration, band, filterMultiplier, destination, activeSources, attackShape = 0, isClick = false) {
  const samples = Math.max(1, Math.floor(audioContext.sampleRate * duration));
  const buffer = audioContext.createBuffer(1, samples, audioContext.sampleRate);
  const data = buffer.getChannelData(0);
  const rng = mulberry32(hashSeed(`transient-band:${startsAt}:${duration}:${band.frequency ?? 1200}`));
  for (let i = 0; i < samples; i += 1) {
    data[i] = rng() * 2 - 1;
  }
  const source = audioContext.createBufferSource();
  const filter = audioContext.createBiquadFilter();
  const gain = audioContext.createGain();
  filter.type = "bandpass";
  filter.frequency.value = (band.frequency ?? 1500) * filterMultiplier;
  filter.Q.value = band.q ?? 1;
  const clickScale = isClick ? 2 ** (attackShape * 0.35) : 1;
  const decayScale = 2 ** (-attackShape * 0.75);
  gain.gain.setValueAtTime(Math.max(0.0001, (band.gain ?? 0.1) * clickScale), startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.008, (band.decay ?? 0.12) * decayScale));
  source.buffer = buffer;
  source.connect(filter).connect(gain).connect(destination);
  trackSource(source, activeSources);
  source.start(startsAt);
  source.stop(startsAt + duration + 0.02);
}

function playTransientFieldResonator(audioContext, startsAt, resonator, destination, activeSources, attackShape = 0) {
  const oscillator = audioContext.createOscillator();
  const gain = audioContext.createGain();
  const decayScale = 2 ** (-attackShape * 0.65);
  oscillator.type = "sine";
  oscillator.frequency.setValueAtTime(resonator.frequency ?? 160, startsAt);
  gain.gain.setValueAtTime(Math.max(0.0001, resonator.gain ?? 0.08), startsAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + Math.max(0.02, (resonator.decay ?? 0.16) * decayScale));
  oscillator.connect(gain).connect(destination);
  trackSource(oscillator, activeSources);
  oscillator.start(startsAt);
  oscillator.stop(startsAt + Math.max(0.04, (resonator.decay ?? 0.16) * decayScale + 0.08));
}

function connectPlaybackBody(audioContext, dryInput, body, startsAt, duration, activeSources) {
  if (!body) {
    return dryInput;
  }
  const input = audioContext.createGain();
  const dry = audioContext.createGain();
  const wet = audioContext.createGain();
  dry.gain.value = 1;
  wet.gain.value = body.gain ?? 0.18;
  input.connect(dry).connect(dryInput);
  if (body.type === "bandpass") {
    const filter = audioContext.createBiquadFilter();
    filter.type = "bandpass";
    filter.frequency.value = body.frequency ?? 640;
    filter.Q.value = body.q ?? 1;
    input.connect(filter).connect(wet).connect(dryInput);
  }
  if (body.type === "comb") {
    const delay = audioContext.createDelay(0.05);
    const feedback = audioContext.createGain();
    delay.delayTime.value = body.delay ?? 0.012;
    feedback.gain.value = body.feedback ?? 0.18;
    input.connect(delay).connect(feedback).connect(delay);
    delay.connect(wet).connect(dryInput);
    const silent = audioContext.createConstantSource();
    silent.offset.value = 0;
    trackSource(silent, activeSources);
    silent.start(startsAt);
    silent.stop(startsAt + duration + 0.2);
  }
  return input;
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
  if (instrument === "flute" || instrument === "breathy-flute" || instrument === "glass" || instrument === "sine-bell" || instrument === "breath-column" || instrument === "air-column-pad") {
    return "sine";
  }
  if (instrument === "pad" || instrument === "organ" || instrument === "reed" || instrument === "saw-lead" || instrument === "dust-lead" || instrument === "warm-pad" || instrument === "noise-fiber") {
    return "sawtooth";
  }
  if (instrument === "round-bass" || instrument === "soft-square" || instrument === "triangle-lead" || instrument === "soft-oscillator" || instrument === "round-low") {
    return "triangle";
  }
  return "square";
}

function envelopeFor(instrument, duration) {
  if (instrument === "breath-column") {
    return { attack: 0.07, sustain: 0.56, filter: 1120 };
  }
  if (instrument === "air-column-pad") {
    return { attack: 0.12, sustain: 0.62, filter: 880 };
  }
  if (instrument === "reed-column") {
    return { attack: 0.026, sustain: 0.5, filter: 1220 };
  }
  if (instrument === "soft-oscillator") {
    return { attack: 0.018, sustain: 0.38, filter: 1380 };
  }
  if (instrument === "noise-fiber") {
    return { attack: 0.035, sustain: 0.34, filter: 980 };
  }
  if (instrument === "warm-pad") {
    return { attack: 0.1, sustain: 0.62, filter: 820 };
  }
  if (instrument === "round-low") {
    return { attack: 0.018, sustain: 0.66, filter: 620 };
  }
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
  if (instrument === "dust-lead") {
    return { attack: 0.018, sustain: 0.38, filter: 980 };
  }
  if (instrument === "soft-square") {
    return { attack: 0.016, sustain: 0.4, filter: 1500 };
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
    || instrument === "warm-pluck"
    || instrument === "low-pluck"
    || instrument === "fuzzy-pluck"
    || instrument === "body-pluck"
    || instrument === "muted-body-pluck"
    || instrument === "struck-bar"
    || instrument === "harp"
    || instrument === "kalimba"
    || instrument === "music-box"
    || instrument === "marimba"
    || instrument === "pluck"
    || instrument === "muted-pluck";
}

function pluckDecayFor(instrument) {
  if (instrument === "muted-body-pluck") {
    return 6.8;
  }
  if (instrument === "body-pluck") {
    return 4.2;
  }
  if (instrument === "struck-bar") {
    return 7.8;
  }
  if (instrument === "low-pluck") {
    return 3.2;
  }
  if (instrument === "warm-pluck") {
    return 3.8;
  }
  if (instrument === "fuzzy-pluck") {
    return 2.8;
  }
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
  if (instrument === "muted-body-pluck") {
    return 0.16;
  }
  if (instrument === "body-pluck") {
    return 0.24;
  }
  if (instrument === "struck-bar") {
    return 0.38;
  }
  if (instrument === "low-pluck") {
    return 0.14;
  }
  if (instrument === "warm-pluck") {
    return 0.2;
  }
  if (instrument === "fuzzy-pluck") {
    return 0.42;
  }
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
  if (instrument === "muted-body-pluck") {
    return 1100;
  }
  if (instrument === "body-pluck") {
    return 1450;
  }
  if (instrument === "struck-bar") {
    return 2200;
  }
  if (instrument === "low-pluck") {
    return 900;
  }
  if (instrument === "warm-pluck") {
    return 1250;
  }
  if (instrument === "fuzzy-pluck") {
    return 1550;
  }
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

function pickDistinctSteps(rng, candidates, count) {
  const pool = [...candidates];
  const steps = [];
  while (steps.length < count && pool.length > 0) {
    const index = Math.floor(rng() * pool.length);
    steps.push(pool.splice(index, 1)[0]);
  }
  return steps.sort((a, b) => a - b);
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
