import { generateRandomTimbre, generateRandomTransient, randomTimbreSummary, randomTransientSummary } from "./timbre_axis_lab.mjs?v=timbre-3";

const KEYS = [
  ["C", 60],
  ["D", 62],
  ["E", 64],
  ["F", 65],
  ["G", 67],
  ["A", 69],
  ["Bb", 70],
];

const SCALES = {
  ionian: [0, 2, 4, 5, 7, 9, 11],
  naturalMinor: [0, 2, 3, 5, 7, 8, 10],
  mixolydian: [0, 2, 4, 5, 7, 9, 10],
  dorian: [0, 2, 3, 5, 7, 9, 10],
  lydian: [0, 2, 4, 6, 7, 9, 11],
  phrygian: [0, 1, 3, 5, 7, 8, 10],
  majorPentatonic: [0, 2, 4, 7, 9],
  minorPentatonic: [0, 3, 5, 7, 10],
  suspendedPentatonic: [0, 2, 5, 7, 10],
};

const DEFAULT_HEIGHT = 0.5;
const DEFAULT_BRIGHTNESS = 0.5;
const DEFAULT_PRESENCE = 0.5;
const DEFAULT_ATTACK = 0.5;
const DEFAULT_BPM = 110;
const DEFAULT_VOLUME = 0.5;
const DEFAULT_BARS = 64;
const BAR_OPTIONS = [8, 16, 32, 64];

const FUNCTION_NAMES = ["identity", "time", "tone", "motion", "color", "boundary"];

export function generateFunctionalSong(seed, options = {}) {
  const seedText = String(seed);
  const rng = mulberry32(hashSeed(seedText));
  const height = clamp(Number(options.height ?? options.focus ?? DEFAULT_HEIGHT), 0, 1);
  const brightness = clamp(Number(options.brightness ?? options.tone ?? DEFAULT_BRIGHTNESS), 0, 1);
  const presence = clamp(Number(options.presence ?? DEFAULT_PRESENCE), 0, 1);
  const attack = clamp(Number(options.attack ?? options.punch ?? DEFAULT_ATTACK), 0, 1);
  const bpm = clamp(Math.round(Number(options.bpm ?? DEFAULT_BPM)), 40, 180);
  const volume = clamp(Number(options.volume ?? DEFAULT_VOLUME), 0, 1);
  const bars = normalizeBars(options.bars);
  const [key, tonic] = pick(KEYS, rng);
  const scaleName = weightedPick([
    { item: "majorPentatonic", weight: 0.18 },
    { item: "minorPentatonic", weight: 0.18 },
    { item: "suspendedPentatonic", weight: 0.1 },
    { item: "ionian", weight: 0.14 },
    { item: "naturalMinor", weight: 0.14 },
    { item: "dorian", weight: 0.12 },
    { item: "mixolydian", weight: 0.09 },
    { item: "lydian", weight: 0.03 },
    { item: "phrygian", weight: 0.02 },
  ], rng);
  const scale = SCALES[scaleName];
  const form = buildFunctionForm(rng);
  const obligations = assignObligations(rng, form);
  const timbres = assignTimbres(rng, obligations, seedText);
  const progression = buildProgression(rng, scale.length);
  const sectionPlan = buildSectionPlan(rng, form, obligations, bars / 8);
  const stepsPerBar = 16;
  const events = [];

  for (let bar = 0; bar < bars; bar += 1) {
    const section = Math.floor(bar / 8);
    const localBar = bar % 8;
    const sectionState = sectionPlan[section];
    const chordRoot = progression[(bar + sectionState.progressionShift) % progression.length] + sectionState.degreeOffset;
    const chord = buildChord(tonic, scale, chordRoot);
    addIdentity(events, sectionObligation(obligations.identity, sectionState), timbres, tonic, scale, chordRoot, bar, localBar, sectionState);
    addTime(events, sectionObligation(obligations.time, sectionState), timbres, chord, bar, localBar, sectionState);
    addTone(events, sectionObligation(obligations.tone, sectionState), timbres, chord, bar, localBar, sectionState);
    addMotion(events, sectionObligation(obligations.motion, sectionState), timbres, tonic, scale, chordRoot, bar, localBar, sectionState);
    addColor(events, sectionObligation(obligations.color, sectionState), timbres, chord, bar, localBar, sectionState);
    addBoundary(events, sectionObligation(obligations.boundary, sectionState), timbres, tonic, scale, bar, localBar, sectionState);
  }

  events.sort((a, b) => a.step - b.step || a.track.localeCompare(b.track));
  const playbackScore = buildPlaybackScore({
    seed: seedText,
    height,
    brightness,
    presence,
    attack,
    bpm,
    volume,
    bars,
    stepsPerBar,
    events,
    timbres,
  });

  return {
    input: {
      seed: seedText,
      height,
      focus: height,
      brightness,
      presence,
      attack,
      punch: attack,
      bpm,
      volume,
      bars,
    },
    playbackScore,
    debug: {
      key,
      scale: scaleName,
      form,
      obligations,
      timbres,
      progression: progression.map((degree) => degree + 1),
      sectionPlan,
      trackMapping: trackMappingFor(obligations),
    },
  };
}

export function randomFunctionalPreset(seed = Date.now()) {
  const rng = mulberry32(hashSeed(String(seed)));
  return {
    seed: randomInt(rng, 100000, 999999).toString(),
    height: DEFAULT_HEIGHT,
    focus: DEFAULT_HEIGHT,
    brightness: DEFAULT_BRIGHTNESS,
    presence: DEFAULT_PRESENCE,
    attack: DEFAULT_ATTACK,
    punch: DEFAULT_ATTACK,
    tone: DEFAULT_BRIGHTNESS,
    bpm: DEFAULT_BPM,
    bars: DEFAULT_BARS,
  };
}

function buildFunctionForm(rng) {
  const focus = weightedPick([
    { item: "identity", weight: 0.28 },
    { item: "time", weight: 0.18 },
    { item: "tone", weight: 0.18 },
    { item: "motion", weight: 0.2 },
    { item: "color", weight: 0.16 },
  ], rng);
  return {
    focus,
    density: round2(0.28 + rng() * 0.56),
    space: round2(rng()),
    contrast: round2(rng()),
    pulse: round2(rng()),
  };
}

function assignObligations(rng, form) {
  const identityCarrier = weightedPick([
    { item: "melodic-line", weight: form.focus === "identity" ? 0.48 : 0.28 },
    { item: "bass-riff", weight: form.focus === "time" ? 0.26 : 0.16 },
    { item: "harmony-arp", weight: form.focus === "motion" ? 0.3 : 0.18 },
    { item: "rhythm-hook", weight: form.focus === "time" ? 0.28 : 0.14 },
  ], rng);
  const timeCarrier = weightedPick([
    { item: "drum-grid", weight: identityCarrier === "rhythm-hook" ? 0 : form.focus === "time" ? 0.44 : 0.25 },
    { item: "bass-pulse", weight: 0.25 },
    { item: "arp-pulse", weight: form.focus === "motion" ? 0.3 : 0.18 },
    { item: "thin-pulse", weight: form.space > 0.68 ? 0.34 : 0.12 },
  ], rng);
  const toneCarrier = weightedPick([
    { item: "root-bass", weight: 0.34 },
    { item: "chord-pad", weight: form.focus === "color" ? 0.34 : 0.24 },
    { item: "drone", weight: form.space > 0.58 ? 0.24 : 0.12 },
    { item: "implied", weight: form.density > 0.62 ? 0.22 : 0.1 },
  ], rng);
  const motionCarrier = weightedPick([
    { item: "none", weight: form.density > 0.68 ? 0.24 : 0.08 },
    { item: "answer-line", weight: identityCarrier === "melodic-line" ? 0.28 : 0.14 },
    { item: "harmony-arp", weight: timeCarrier !== "arp-pulse" ? 0.25 : 0.08 },
    { item: "bass-walk", weight: toneCarrier === "root-bass" ? 0.2 : 0.08 },
    { item: "percussion-fill", weight: timeCarrier === "drum-grid" ? 0.18 : 0.08 },
  ], rng);
  const colorCarrier = weightedPick([
    { item: "none", weight: form.focus === "color" ? 0.06 : 0.18 },
    { item: "air-pad", weight: 0.24 },
    { item: "noise-halo", weight: 0.2 },
    { item: "organ-bed", weight: 0.16 },
    { item: "bright-accent", weight: form.contrast > 0.56 ? 0.2 : 0.08 },
  ], rng);
  const boundaryCarrier = weightedPick([
    { item: "rest-gap", weight: form.space > 0.6 ? 0.28 : 0.1 },
    { item: "drum-fill", weight: timeCarrier === "drum-grid" ? 0.26 : 0.12 },
    { item: "contrast-note", weight: form.contrast > 0.42 ? 0.28 : 0.12 },
    { item: "register-turn", weight: identityCarrier === "melodic-line" ? 0.2 : 0.1 },
  ], rng);
  return {
    identity: obligation("identity", identityCarrier, "make the loop recognizable"),
    time: obligation("time", timeCarrier, "make the pulse and cycle readable"),
    tone: obligation("tone", toneCarrier, "give root and stability information"),
    motion: obligation("motion", motionCarrier, "avoid static repetition when needed"),
    color: obligation("color", colorCarrier, "set texture without taking over identity"),
    boundary: obligation("boundary", boundaryCarrier, "mark phrase edges and contrast"),
  };
}

function obligation(name, carrier, purpose) {
  return { name, carrier, purpose };
}

function assignTimbres(rng, obligations, seedText) {
  const pitched = Object.fromEntries(FUNCTION_NAMES.map((name) => [
    name,
    pitchedFieldTimbre(`${seedText}:pitched:${name}:${obligations[name].carrier}`, name),
  ]));
  const transient = {
    kick: transientFieldTimbre(`${seedText}:transient:kick`, "kick"),
    snare: transientFieldTimbre(`${seedText}:transient:snare`, "snare"),
    hat: transientFieldTimbre(`${seedText}:transient:hat`, "hat"),
  };
  return { pitched, transient };
}

function buildSectionPlan(rng, form, obligations, sectionCount = 8) {
  const contrastOffset = pick([-1, 1, 2], rng);
  const lateOffset = pick([0, -1, 1, 2], rng);
  const returnOffset = pick([-1, 0, 1], rng);
  const deepOffset = pick([-2, -1, 2, 3], rng);
  const breakOffset = pick([-2, -1, 0], rng);
  const finalOffset = pick([0, 1, 2, 3], rng);
  const contrastShift = pick([1, 2], rng);
  const lateShift = pick([0, 1, 2], rng);
  const returnShift = pick([0, 2, 3], rng);
  const deepShift = pick([1, 2, 3], rng);
  const breakShift = pick([0, 1], rng);
  const finalShift = pick([2, 3], rng);
  const lift = form.contrast > 0.55 ? 1.08 : 1;
  const motif = () => randomInt(rng, 0, 99);
  const bIdentityCarrier = weightedPick([
    { item: "melodic-line", weight: obligations.identity.carrier === "melodic-line" ? 0 : form.focus === "identity" ? 0.28 : 0.34 },
    { item: "harmony-arp", weight: obligations.identity.carrier === "harmony-arp" ? 0 : form.focus === "motion" ? 0.3 : 0.22 },
    { item: "rhythm-hook", weight: obligations.identity.carrier === "rhythm-hook" ? 0 : form.focus === "time" ? 0.28 : 0.2 },
    { item: "bass-riff", weight: obligations.identity.carrier === "bass-riff" ? 0 : form.focus === "tone" ? 0.24 : 0.16 },
  ], rng);
  const bTimeCarrier = weightedPick([
    { item: "drum-grid", weight: bIdentityCarrier === "rhythm-hook" ? 0.02 : 0.34 },
    { item: "arp-pulse", weight: bIdentityCarrier === "harmony-arp" ? 0.1 : 0.28 },
    { item: "bass-pulse", weight: bIdentityCarrier === "bass-riff" ? 0.08 : 0.22 },
    { item: "thin-pulse", weight: form.space > 0.55 ? 0.24 : 0.12 },
  ], rng);
  const bToneCarrier = weightedPick([
    { item: "chord-pad", weight: 0.34 },
    { item: "drone", weight: form.space > 0.5 ? 0.28 : 0.14 },
    { item: "root-bass", weight: bIdentityCarrier === "bass-riff" ? 0.08 : 0.24 },
    { item: "implied", weight: form.density > 0.58 ? 0.2 : 0.1 },
  ], rng);
  const chorusIdentityCarrier = weightedPick([
    { item: "melodic-line", weight: bIdentityCarrier === "melodic-line" ? 0.62 : 0.58 },
    { item: "harmony-arp", weight: bIdentityCarrier === "harmony-arp" ? 0.2 : 0.32 },
  ], rng);
  const chorusOverrides = {
    identity: chorusIdentityCarrier,
    time: weightedPick([
      { item: "drum-grid", weight: chorusIdentityCarrier === "rhythm-hook" ? 0.08 : 0.42 },
      { item: "arp-pulse", weight: chorusIdentityCarrier === "harmony-arp" ? 0.1 : 0.28 },
      { item: "bass-pulse", weight: chorusIdentityCarrier === "bass-riff" ? 0.08 : 0.2 },
      { item: "thin-pulse", weight: 0.08 },
    ], rng),
    tone: weightedPick([
      { item: "chord-pad", weight: 0.44 },
      { item: "root-bass", weight: chorusIdentityCarrier === "bass-riff" ? 0.12 : 0.24 },
      { item: "drone", weight: 0.18 },
      { item: "implied", weight: 0.08 },
    ], rng),
    motion: weightedPick([
      { item: "answer-line", weight: chorusIdentityCarrier === "melodic-line" ? 0.16 : 0.3 },
      { item: "harmony-arp", weight: 0.26 },
      { item: "bass-walk", weight: 0.14 },
      { item: "percussion-fill", weight: 0.2 },
      { item: "none", weight: 0.04 },
    ], rng),
    color: weightedPick([
      { item: "air-pad", weight: 0.24 },
      { item: "organ-bed", weight: 0.28 },
      { item: "bright-accent", weight: 0.22 },
      { item: "noise-halo", weight: 0.18 },
      { item: "none", weight: 0.04 },
    ], rng),
    boundary: weightedPick([
      { item: "drum-fill", weight: 0.34 },
      { item: "contrast-note", weight: 0.3 },
      { item: "register-turn", weight: 0.22 },
      { item: "rest-gap", weight: 0.08 },
    ], rng),
  };
  const firstHalf = [
    sectionState("establish", 0, 0, 0, 0.94, 0.86, 0.78, 0.82, "primary", motif()),
    sectionState("vary-one-axis", 1, pick([0, 1], rng), pick([0, 1], rng), 1, 0.96, 0.9, 0.95, "answer", motif()),
    sectionState("vary-two-axes", 2, contrastOffset, contrastShift, 0.88 * lift, 1.18, 1.08, 1.12, "contrast", motif()),
    sectionState("late-variation", pick([0, 1, 3], rng), lateOffset, lateShift, 1.04, 1.04, 0.96, 1.18, "lift", motif()),
  ];
  const secondHalfMode = weightedPick([
    { item: "reprise", weight: 0.18 },
    { item: "develop", weight: 0.2 },
    { item: "expand", weight: 0.14 },
    { item: "bridge", weight: 0.24 },
    { item: "verse-chorus", weight: 0.24 },
  ], rng);
  if (secondHalfMode === "verse-chorus") {
    const bOverrides = {
      identity: bIdentityCarrier,
      time: bTimeCarrier,
      tone: bToneCarrier,
      motion: weightedPick([
        { item: "answer-line", weight: bIdentityCarrier === "melodic-line" ? 0.16 : 0.34 },
        { item: "harmony-arp", weight: bTimeCarrier === "arp-pulse" ? 0.1 : 0.28 },
        { item: "bass-walk", weight: bToneCarrier === "root-bass" ? 0.24 : 0.12 },
        { item: "percussion-fill", weight: bTimeCarrier === "drum-grid" ? 0.22 : 0.1 },
        { item: "none", weight: 0.06 },
      ], rng),
      color: weightedPick([
        { item: "air-pad", weight: 0.2 },
        { item: "noise-halo", weight: 0.24 },
        { item: "organ-bed", weight: 0.2 },
        { item: "bright-accent", weight: 0.14 },
        { item: "none", weight: 0.1 },
      ], rng),
      boundary: weightedPick([
        { item: "rest-gap", weight: 0.22 },
        { item: "contrast-note", weight: 0.32 },
        { item: "register-turn", weight: 0.2 },
        { item: "drum-fill", weight: 0.2 },
      ], rng),
    };
    const outroOverrides = form.contrast > 0.52 ? null : chorusOverrides;
    return [
      sectionState("a-verse", 0, 0, 0, 0.94, 0.84, 0.74, 0.8, "primary", motif()),
      sectionState("a-answer", 1, pick([0, 1], rng), pick([0, 1], rng), 1, 0.94, 0.84, 0.9, "answer", motif()),
      sectionState("b-verse", 2, deepOffset, deepShift, 0.94 * lift, 1.14, 1.02, 1.1, "deep-contrast", motif(), bOverrides),
      sectionState("b-prechorus", pick([2, 3], rng), deepOffset + pick([-1, 1], rng), pick([1, 3], rng), 1.04 * lift, 1.24, 1.08, 1.18, "break", motif(), bOverrides),
      sectionState("chorus", 3, finalOffset, finalShift, 1.2 * lift, 1.34, 1.16, 1.28, "final-lift", motif(), chorusOverrides),
      sectionState("chorus-answer", pick([1, 3], rng), pick([finalOffset, finalOffset - 1], rng), pick([2, 3], rng), 1.14 * lift, 1.2, 1.1, 1.22, "lift", motif(), chorusOverrides),
      sectionState("return-or-outro", pick([0, 1], rng), returnOffset, returnShift, 1.02, 0.94, 0.86, 1.02, "return", motif(), outroOverrides),
      sectionState("final-tag", pick([1, 3], rng), lateOffset, lateShift, 1.08, 1.04, 0.96, 1.18, form.contrast > 0.55 ? "final-lift" : "return", motif(), outroOverrides),
    ].slice(0, sectionCount);
  }
  if (secondHalfMode === "reprise") {
    return firstHalf.concat([
      sectionState("return", pick([0, 1], rng), pick([0, returnOffset], rng), pick([0, returnShift], rng), 1.02, 0.9, 0.8, 0.88, "return", motif()),
      sectionState("answer-return", 1, pick([0, 1], rng), pick([0, 1], rng), 0.98, 0.96, 0.84, 0.94, "answer", motif()),
      sectionState("contrast-return", 2, pick([0, contrastOffset], rng), contrastShift, 0.84 * lift, 1.08, 0.96, 1.02, "contrast", motif()),
      sectionState("late-return", pick([0, 1, 3], rng), lateOffset, lateShift, 1.06, 1.02, 0.9, 1.08, "lift", motif()),
    ]).slice(0, sectionCount);
  }
  if (secondHalfMode === "expand") {
    return firstHalf.concat([
      sectionState("return", pick([0, 1], rng), returnOffset, returnShift, 1.1, 0.94, 0.86, 0.98, "return", motif()),
      sectionState("deeper-contrast", 2, deepOffset, deepShift, 0.94 * lift, 1.28, 1.16, 1.14, "deep-contrast", motif()),
      sectionState("breakdown", pick([0, 3], rng), breakOffset, breakShift, 0.7, 0.68, 0.6, 0.84, "break", motif()),
      sectionState("final-lift", 3, finalOffset, finalShift, 1.18, 1.18, 1.04, 1.3, "final-lift", motif()),
    ]).slice(0, sectionCount);
  }
  if (secondHalfMode === "bridge") {
    const bOverrides = {
      identity: bIdentityCarrier,
      time: bTimeCarrier,
      tone: bToneCarrier,
      motion: weightedPick([
        { item: "answer-line", weight: bIdentityCarrier === "melodic-line" ? 0.18 : 0.3 },
        { item: "harmony-arp", weight: bTimeCarrier === "arp-pulse" ? 0.08 : 0.28 },
        { item: "bass-walk", weight: bToneCarrier === "root-bass" ? 0.24 : 0.12 },
        { item: "percussion-fill", weight: bTimeCarrier === "drum-grid" ? 0.22 : 0.1 },
        { item: "none", weight: form.density > 0.72 ? 0.18 : 0.06 },
      ], rng),
      color: weightedPick([
        { item: "air-pad", weight: 0.2 },
        { item: "noise-halo", weight: 0.24 },
        { item: "organ-bed", weight: 0.24 },
        { item: "bright-accent", weight: form.contrast > 0.45 ? 0.2 : 0.1 },
        { item: "none", weight: 0.08 },
      ], rng),
    };
    return firstHalf.concat([
      sectionState("b-entry", 3, deepOffset, deepShift, 0.98 * lift, 1.18, 1.06, 1.08, "deep-contrast", motif(), bOverrides),
      sectionState("b-answer", pick([2, 3], rng), deepOffset + pick([-1, 1], rng), pick([1, 3], rng), 1.08 * lift, 1.26, 1.12, 1.12, "break", motif(), bOverrides),
      sectionState("b-build", 3, finalOffset, finalShift, 1.16 * lift, 1.34, 1.18, 1.22, "final-lift", motif(), bOverrides),
      sectionState("b-landing", pick([1, 3], rng), returnOffset, returnShift, 1.04, 0.96, 0.92, 1.16, "return", motif(), bOverrides),
    ]).slice(0, sectionCount);
  }
  return firstHalf.concat([
    sectionState("return", pick([0, 1], rng), returnOffset, returnShift, 1.06, 0.92, 0.82, 0.94, "return", motif()),
    sectionState("deeper-contrast", 2, deepOffset, deepShift, 0.9 * lift, 1.18, 1.08, 1.08, "deep-contrast", motif()),
    sectionState("breakdown", pick([0, 3], rng), breakOffset, breakShift, 0.76, 0.74, 0.64, 0.88, "break", motif()),
    sectionState("final-lift", 3, finalOffset, finalShift, 1.12, 1.12, 0.98, 1.22, "final-lift", motif()),
  ]).slice(0, sectionCount);
}

function sectionState(name, variant, degreeOffset, progressionShift, identityLevel, motionLevel, colorLevel, boundaryLevel, motifRole, motifVariant, carrierOverrides = null) {
  return {
    name,
    variant,
    motifRole,
    motifVariant,
    carrierOverrides,
    degreeOffset,
    progressionShift,
    identityLevel: round2(identityLevel),
    motionLevel: round2(motionLevel),
    colorLevel: round2(colorLevel),
    boundaryLevel: round2(boundaryLevel),
  };
}

function sectionObligation(obligation, sectionState) {
  const carrier = sectionState.carrierOverrides?.[obligation.name] ?? obligation.carrier;
  if (carrier === obligation.carrier) {
    return obligation;
  }
  return { ...obligation, carrier };
}

function addIdentity(events, obligation, timbres, tonic, scale, chordRoot, bar, localBar, sectionState) {
  if (obligation.carrier === "melodic-line") {
    const patterns = melodicPatternsForRole(sectionState.motifRole);
    const pattern = developMelodicPattern(patterns[sectionState.motifVariant % patterns.length], localBar, sectionState);
    for (const [step, offset, duration] of pattern) {
      events.push(noteEvent("lead", bar, step, duration, [degreeNote(tonic, scale, chordRoot + offset, 12)], "identity", 0.13 * sectionState.identityLevel));
    }
    return;
  }
  if (obligation.carrier === "bass-riff") {
    const pattern = sectionState.variant === 2 ? [[0, 0], [5, 1], [9, 3], [14, 2]] : [[0, 0], [6, 2], [10, 0], [14, 1]];
    for (const [step, offset] of pattern) {
      events.push(noteEvent("bass", bar, step, 2, [degreeNote(tonic, scale, chordRoot + offset, -24)], "identity", 0.17 * sectionState.identityLevel));
    }
    return;
  }
  if (obligation.carrier === "harmony-arp") {
    const pattern = sectionState.variant === 2 ? [[1, 2], [4, 1], [10, 3], [14, 1]] : [[0, 0], [5, 1], [9, 2], [13, 1]];
    for (const [step, index] of pattern) {
      events.push(noteEvent("chord", bar, step, 2, [degreeNote(tonic, scale, chordRoot + index * 2, 12)], "identity", 0.075 * sectionState.identityLevel));
    }
    return;
  }
  const patterns = [
    { kick: [0, 10], snare: [6, 12], hat: [3, 9, 14] },
    { kick: [0, 7], snare: [5, 12], hat: [2, 10, 15] },
    { kick: [0, 8], snare: [4, 11, 15], hat: [3, 6, 13] },
    { kick: [0, 10], snare: [6, 11], hat: [2, 9, 14, 15] },
  ];
  const pattern = patterns[sectionState.variant % patterns.length];
  const barLift = localBar === 7 ? 1.12 : 1;
  for (const step of pattern.kick) {
    events.push(noiseEvent("drums", bar, step, "kick", "identity", 0.22 * sectionState.identityLevel * barLift));
  }
  for (const step of pattern.snare) {
    events.push(noiseEvent("drums", bar, step, "snare", "identity", 0.17 * sectionState.identityLevel * barLift));
  }
  for (const step of pattern.hat) {
    if (sectionState.variant !== 0 || localBar % 2 === 0 || step !== 14) {
      events.push(noiseEvent("drums", bar, step, "hat", "identity", 0.085 * sectionState.identityLevel * barLift));
    }
  }
}

function melodicPatternsForRole(motifRole) {
  if (motifRole === "deep-contrast") {
    return [
      [[0, 6, 2], [4, 2, 2], [7, 5, 2], [11, 1, 2], [14, 2, 2]],
      [[1, 5, 2], [5, 1, 3], [9, 6, 2], [12, 3, 2]],
      [[2, 6, 2], [6, 3, 2], [9, 1, 3], [13, 2, 2]],
    ];
  }
  if (motifRole === "contrast") {
    return [
      [[0, 5, 2], [4, 2, 3], [10, 6, 2], [13, 3, 2]],
      [[1, 2, 2], [5, 5, 2], [9, 1, 3], [14, 0, 2]],
      [[0, 4, 2], [4, 5, 2], [8, 1, 3], [13, 2, 2]],
    ];
  }
  if (motifRole === "final-lift") {
    return [
      [[0, 0, 2], [2, 2, 2], [5, 4, 2], [9, 5, 2], [12, 7, 2], [15, 5, 1]],
      [[0, 1, 2], [3, 3, 2], [6, 5, 2], [10, 6, 2], [14, 8, 2]],
      [[1, 0, 2], [4, 2, 2], [7, 4, 2], [11, 6, 2], [13, 7, 2]],
    ];
  }
  if (motifRole === "lift") {
    return [
      [[0, 0, 2], [3, 2, 2], [6, 4, 2], [10, 5, 3], [14, 7, 2]],
      [[0, 1, 2], [4, 3, 2], [7, 5, 2], [11, 4, 2], [14, 6, 2]],
      [[1, 0, 2], [5, 2, 2], [8, 4, 2], [12, 6, 2], [15, 7, 1]],
    ];
  }
  if (motifRole === "break") {
    return [
      [[0, 0, 5], [9, 2, 3], [14, 1, 2]],
      [[2, 1, 4], [8, 0, 3], [13, 2, 2]],
      [[0, 2, 3], [7, 0, 4], [12, 1, 2]],
    ];
  }
  if (motifRole === "return") {
    return [
      [[0, 0, 4], [4, 1, 2], [8, 2, 2], [12, 1, 3]],
      [[0, 0, 3], [5, 2, 2], [9, 1, 2], [13, 3, 2]],
      [[1, 3, 2], [5, 1, 3], [9, 2, 2], [13, 0, 2]],
    ];
  }
  if (motifRole === "answer") {
    return [
      [[2, 3, 2], [6, 1, 3], [11, 2, 2], [14, 0, 2]],
      [[3, 4, 2], [7, 2, 2], [10, 3, 2], [14, 1, 2]],
      [[2, 2, 2], [5, 0, 3], [10, 1, 2], [13, 3, 2]],
    ];
  }
  return [
    [[0, 0, 4], [4, 1, 2], [7, 2, 3], [12, 1, 2]],
    [[0, 0, 3], [5, 2, 2], [8, 1, 3], [13, 3, 2]],
    [[1, 3, 3], [5, 4, 2], [9, 2, 3], [13, 1, 2]],
    [[0, 0, 4], [4, 1, 2], [8, 2, 2], [12, 4, 3]],
    [[0, 0, 3], [4, 1, 2], [7, 3, 4], [13, 2, 2]],
    [[2, 0, 3], [7, 2, 2], [10, 1, 1], [14, 4, 2]],
    [[0, 0, 5], [8, 2, 2], [11, 4, 1], [15, 1, 1]],
    [[1, 1, 2], [5, 0, 2], [9, 3, 3], [14, 2, 2]],
  ];
}

function developMelodicPattern(pattern, localBar, sectionState) {
  const phrasePhase = localBar % 4;
  const motifRole = sectionState.motifRole;
  const developed = pattern.map(([step, offset, duration], index) => {
    let nextStep = step;
    let nextOffset = offset;
    let nextDuration = duration;
    if (phrasePhase === 1 && index >= 2) {
      nextOffset += sectionState.variant === 2 ? -1 : 1;
    }
    if (phrasePhase === 2 && index === 1) {
      nextStep += motifRole === "contrast" ? -1 : 1;
      nextDuration = Math.max(2, duration - 1);
    }
    if (phrasePhase === 3 && index === pattern.length - 1) {
      nextOffset = motifRole === "lift" || motifRole === "final-lift" ? offset + 1 : sectionState.variant === 2 ? 2 : 0;
      nextDuration = Math.max(duration, motifRole === "contrast" ? 2 : 3);
    }
    return [nextStep, nextOffset, nextDuration];
  });
  if (phrasePhase === 2 && motifRole !== "answer") {
    const anchor = pattern[2] ?? pattern[pattern.length - 1];
    developed.splice(3, 0, [Math.min(14, anchor[0] + 2), anchor[1] + (sectionState.variant === 2 ? 1 : -1), 2]);
  }
  if (phrasePhase === 3 && motifRole !== "contrast" && motifRole !== "deep-contrast" && motifRole !== "break") {
    developed.push([14, motifRole === "lift" || motifRole === "final-lift" ? 7 : sectionState.variant === 2 ? 5 : 4, 2]);
  }
  return shapeMelodicPhrase(developed, localBar, sectionState);
}

function shapeMelodicPhrase(pattern, localBar, sectionState) {
  const phrasePhase = localBar % 4;
  const profile = melodicProfileForRole(sectionState.motifRole);
  const contour = (sectionState.motifVariant + sectionState.variant + phrasePhase) % 5;
  const landing = melodicLandingForPhase(profile, phrasePhase);
  const count = pattern.length;

  return pattern.map(([step, sourceOffset, duration], index) => {
    const position = count <= 1 ? 1 : index / (count - 1);
    const isLanding = index === count - 1 || step >= 14;
    const target = melodicContourOffset(profile, contour, position, landing);
    const sourceDetail = isLanding ? 0 : clamp(Math.round((sourceOffset - target) * 0.28), -1, 1);
    const offset = isLanding ? landing : clamp(Math.round(target + sourceDetail), profile.min, profile.max);
    const nextDuration = isLanding ? Math.max(duration, phrasePhase === 3 ? 3 : 2) : Math.max(2, duration);
    return [step, offset, nextDuration];
  });
}

function melodicProfileForRole(motifRole) {
  const profiles = {
    primary: { start: 0, depart: 2, apex: 4, low: -1, cadence: 0, question: 2, min: -2, max: 5 },
    return: { start: 0, depart: 2, apex: 3, low: -1, cadence: 0, question: 1, min: -2, max: 4 },
    answer: { start: 3, depart: 1, apex: 4, low: 0, cadence: 0, question: 2, min: -1, max: 5 },
    contrast: { start: 4, depart: 2, apex: 6, low: 1, cadence: 3, question: 5, min: 0, max: 7 },
    "deep-contrast": { start: 6, depart: 3, apex: 7, low: 1, cadence: 2, question: 5, min: 0, max: 8 },
    lift: { start: 1, depart: 3, apex: 7, low: 0, cadence: 6, question: 5, min: 0, max: 8 },
    "final-lift": { start: 1, depart: 4, apex: 8, low: 0, cadence: 7, question: 6, min: 0, max: 9 },
    break: { start: 1, depart: 0, apex: 3, low: -1, cadence: 0, question: 2, min: -2, max: 4 },
  };
  return profiles[motifRole] ?? profiles.primary;
}

function melodicLandingForPhase(profile, phrasePhase) {
  if (phrasePhase === 1) {
    return profile.question;
  }
  if (phrasePhase === 2) {
    return profile.depart;
  }
  if (phrasePhase === 3) {
    return profile.cadence;
  }
  return profile.start;
}

function melodicContourOffset(profile, contour, position, landing) {
  const x = clamp(position, 0, 1);
  if (contour === 0) {
    return x < 0.58
      ? interpolate(profile.start, profile.apex, x / 0.58)
      : interpolate(profile.apex, landing, (x - 0.58) / 0.42);
  }
  if (contour === 1) {
    return x < 0.45
      ? interpolate(profile.start, profile.depart, x / 0.45)
      : interpolate(profile.depart, landing, (x - 0.45) / 0.55);
  }
  if (contour === 2) {
    return x < 0.5
      ? interpolate(profile.apex, profile.low, x / 0.5)
      : interpolate(profile.low, landing, (x - 0.5) / 0.5);
  }
  if (contour === 3) {
    return x < 0.33
      ? interpolate(profile.start, profile.low, x / 0.33)
      : x < 0.72
        ? interpolate(profile.low, profile.apex, (x - 0.33) / 0.39)
        : interpolate(profile.apex, landing, (x - 0.72) / 0.28);
  }
  return x < 0.28
    ? interpolate(profile.start, profile.apex, x / 0.28)
    : x < 0.64
      ? interpolate(profile.apex, profile.depart, (x - 0.28) / 0.36)
      : interpolate(profile.depart, landing, (x - 0.64) / 0.36);
}

function interpolate(left, right, amount) {
  return left + (right - left) * clamp(amount, 0, 1);
}

function addTime(events, obligation, timbres, chord, bar, localBar, sectionState) {
  if (obligation.carrier === "drum-grid") {
    events.push(noiseEvent("drums", bar, 0, "kick", "time", 0.2));
    events.push(noiseEvent("drums", bar, sectionState.variant === 2 ? 10 : 8, "kick", "time", 0.13));
    events.push(noiseEvent("drums", bar, 4, "snare", "time", 0.14));
    events.push(noiseEvent("drums", bar, 12, "snare", "time", 0.13));
    const hats = sectionState.variant === 2 ? [3, 6, 11, 14] : localBar % 2 === 0 ? [2, 6, 10, 14] : [3, 7, 11, 15];
    for (const step of hats) {
      events.push(noiseEvent("drums", bar, step, "hat", "time", 0.055));
    }
    return;
  }
  if (obligation.carrier === "bass-pulse") {
    events.push(noteEvent("bass", bar, 0, 3, [chord[0] - 24], "time", 0.15));
    events.push(noteEvent("bass", bar, sectionState.variant === 2 ? 10 : 8, 2, [chord[0] - 24], "time", 0.11));
    return;
  }
  if (obligation.carrier === "arp-pulse") {
    const steps = sectionState.variant === 2 ? [1, 5, 9, 13] : [0, 4, 8, 12];
    for (let index = 0; index < steps.length; index += 1) {
      events.push(noteEvent("chord", bar, steps[index], 1, [chord[index % 3] + 12], "time", 0.055));
    }
    return;
  }
  if (localBar % 2 === 0) {
    events.push(noiseEvent("drums", bar, 0, "kick", "time", 0.14));
  }
}

function addTone(events, obligation, timbres, chord, bar, localBar, sectionState) {
  if (obligation.carrier === "root-bass") {
    if (localBar % 2 === 0 || sectionState.variant === 2 && localBar === 5) {
      events.push(noteEvent("bass", bar, 2, 8, [chord[0] - 24], "tone", 0.11));
    }
    return;
  }
  if (obligation.carrier === "chord-pad") {
    if (localBar % 2 === 0) {
      events.push(noteEvent("chord", bar, 0, 12, chord.map((note) => note + 12), "tone", 0.052));
    }
    return;
  }
  if (obligation.carrier === "drone") {
    if (localBar === 0 || localBar === 4) {
      events.push(noteEvent("chord", bar, 0, 16, [chord[0]], "tone", 0.07));
    }
    return;
  }
}

function addMotion(events, obligation, timbres, tonic, scale, chordRoot, bar, localBar, sectionState) {
  if (obligation.carrier === "none") {
    return;
  }
  if (obligation.carrier === "answer-line") {
    if ([2, 6].includes(localBar) || sectionState.variant === 2 && localBar === 4) {
      const pattern = sectionState.variant === 2 ? [[1, 5], [7, 3], [12, 4]] : [[2, 4], [6, 3], [11, 2]];
      for (const [step, offset] of pattern) {
        events.push(noteEvent("counter", bar, step, 2, [degreeNote(tonic, scale, chordRoot + offset, 0)], "motion", 0.055 * sectionState.motionLevel));
      }
    }
    return;
  }
  if (obligation.carrier === "harmony-arp") {
    const pattern = sectionState.variant === 2 ? [[2, 1], [7, 3], [12, 4]] : [[1, 0], [6, 2], [10, 4]];
    for (const [step, offset] of pattern) {
      events.push(noteEvent("chord", bar, step, 2, [degreeNote(tonic, scale, chordRoot + offset, 12)], "motion", 0.052 * sectionState.motionLevel));
    }
    return;
  }
  if (obligation.carrier === "bass-walk") {
    if (localBar % 2 === 1) {
      const pattern = sectionState.variant === 2 ? [[2, 1], [8, 2], [13, 3]] : [[3, 0], [7, 1], [12, 2]];
      for (const [step, offset] of pattern) {
        events.push(noteEvent("bass", bar, step, 2, [degreeNote(tonic, scale, chordRoot + offset, -24)], "motion", 0.12 * sectionState.motionLevel));
      }
    }
    return;
  }
  if (localBar === 7) {
    events.push(noiseEvent("drums", bar, 10, "snare", "motion", 0.11));
    events.push(noiseEvent("drums", bar, 13, "hat", "motion", 0.07));
  }
}

function addColor(events, obligation, timbres, chord, bar, localBar, sectionState) {
  if (obligation.carrier === "none") {
    return;
  }
  if (obligation.carrier === "air-pad" && (localBar === 1 || localBar === 5)) {
    events.push(noteEvent("chord", bar, sectionState.variant === 2 ? 2 : 0, 10, [chord[1] + 12, chord[2] + 12], "color", 0.04 * sectionState.colorLevel));
  }
  if (obligation.carrier === "noise-halo" && localBar % 4 === 0) {
    events.push(noteEvent("lead", bar, sectionState.variant === 2 ? 9 : 11, 5, [chord[1] + 12], "color", 0.04 * sectionState.colorLevel));
  }
  if (obligation.carrier === "organ-bed" && localBar % 2 === 0) {
    events.push(noteEvent("chord", bar, 4, 8, chord.map((note) => note + 12), "color", 0.045 * sectionState.colorLevel));
  }
  if (obligation.carrier === "bright-accent" && localBar === 6) {
    events.push(noteEvent("lead", bar, sectionState.variant === 2 ? 10 : 12, 2, [chord[2] + 12], "color", 0.06 * sectionState.colorLevel));
  }
}

function addBoundary(events, obligation, timbres, tonic, scale, bar, localBar, sectionState) {
  if (localBar !== 7) {
    return;
  }
  if (obligation.carrier === "drum-fill") {
    events.push(noiseEvent("drums", bar, 12, "snare", "boundary", 0.12 * sectionState.boundaryLevel));
    events.push(noiseEvent("drums", bar, 14, "hat", "boundary", 0.08 * sectionState.boundaryLevel));
    return;
  }
  if (obligation.carrier === "contrast-note") {
    events.push(noteEvent("lead", bar, 13, 2, [degreeNote(tonic, scale, sectionState.variant === 2 ? 6 : 1, 12)], "boundary", 0.075 * sectionState.boundaryLevel));
    return;
  }
  if (obligation.carrier === "register-turn") {
    events.push(noteEvent("counter", bar, 12, 3, [degreeNote(tonic, scale, sectionState.variant === 2 ? 5 : 2, 12)], "boundary", 0.06 * sectionState.boundaryLevel));
  }
}

function buildPlaybackScore({ seed, height, brightness, presence, attack, bpm, volume, bars, stepsPerBar, events, timbres }) {
  const playbackTimbres = {
    identity: timbres.pitched.identity,
    time: timbres.pitched.time,
    tone: timbres.pitched.tone,
    motion: timbres.pitched.motion,
    color: timbres.pitched.color,
    boundary: timbres.pitched.boundary,
    kick: timbres.transient.kick,
    snare: timbres.transient.snare,
    hat: timbres.transient.hat,
  };
  return {
    version: 1,
    source: { seed, height, focus: height, brightness, presence, attack, punch: attack, bars },
    transport: {
      bpm,
      bars,
      stepsPerBar,
      stepDurationBeats: 0.25,
      loopSteps: bars * stepsPerBar,
    },
    mix: {
      volume,
      playbackTone: playbackToneFor({ height, brightness, presence, attack }),
    },
    timbres: playbackTimbres,
    events,
  };
}

function noteEvent(track, bar, step, durationSteps, notes, timbre, velocity) {
  return {
    track,
    step: bar * 16 + step,
    durationSteps,
    notes: notes.map((note) => typeof note === "number" ? fitRegisterForTrack(track, note + registerShiftForTrack(track)) : note),
    timbre,
    role: timbre,
    velocity,
  };
}

function registerShiftForTrack(track) {
  return {
    lead: -24,
    counter: -24,
    chord: -24,
    bass: 0,
  }[track] ?? 0;
}

function fitRegisterForTrack(track, note) {
  const ranges = {
    lead: [45, 88],
    counter: [38, 76],
    chord: [36, 86],
    bass: [32, 70],
  };
  const [min, max] = ranges[track] ?? [24, 96];
  let fitted = note;
  while (fitted > max) {
    fitted -= 12;
  }
  while (fitted < min) {
    fitted += 12;
  }
  return fitted;
}

function noiseEvent(track, bar, step, sound, timbre, velocity) {
  return {
    track,
    step: bar * 16 + step,
    durationSteps: 1,
    notes: [sound],
    timbre: sound,
    role: timbre,
    velocity,
  };
}

function buildProgression(rng, scaleLength) {
  return pick([
    [0, 3, 4, 0],
    [0, 4, 3, 4],
    [0, 2, 4, 3],
    [0, scaleLength - 2, 3, 4],
  ], rng);
}

function buildChord(tonic, scale, degree) {
  return [degreeNote(tonic, scale, degree, 0), degreeNote(tonic, scale, degree + 2, 0), degreeNote(tonic, scale, degree + 4, 0)];
}

function degreeNote(tonic, scale, degree, octave) {
  const scaleDegree = ((degree % scale.length) + scale.length) % scale.length;
  const scaleOctave = Math.floor(degree / scale.length) * 12;
  return tonic + scale[scaleDegree] + scaleOctave + octave;
}

function trackMappingFor(obligations) {
  return Object.fromEntries(Object.entries(obligations).map(([name, obligation]) => [name, {
    carrier: obligation.carrier,
    playbackTracks: tracksForCarrier(obligation.carrier),
  }]));
}

function tracksForCarrier(carrier) {
  if (carrier.includes("bass") || carrier === "root-bass") {
    return ["bass"];
  }
  if (carrier.includes("drum") || carrier.includes("rhythm") || carrier.includes("pulse")) {
    return carrier === "arp-pulse" ? ["chord"] : ["drums"];
  }
  if (carrier.includes("arp") || carrier.includes("pad") || carrier.includes("drone") || carrier === "organ-bed") {
    return ["chord"];
  }
  if (carrier.includes("answer") || carrier.includes("register")) {
    return ["counter"];
  }
  if (carrier === "none" || carrier === "implied" || carrier === "rest-gap") {
    return [];
  }
  return ["lead"];
}

function pitchedFieldTimbre(seed, role) {
  const model = generateRandomTimbre(seed);
  return {
    kind: "spectral-field",
    role,
    seed: model.seed,
    gain: roleGain(role) * (model.signal.distanceGain ?? 0.72),
    engine: "stochastic-spectral-field",
    summary: randomTimbreSummary(model),
    parameters: model.parameters,
    signal: model.signal,
  };
}

function transientFieldTimbre(seed, role) {
  const model = generateRandomTransient(seed);
  return {
    kind: "transient-field",
    role,
    seed: model.seed,
    gain: transientRoleGain(role) * (model.signal.distanceGain ?? 0.78),
    engine: "stochastic-transient-field",
    summary: randomTransientSummary(model),
    parameters: model.parameters,
    signal: model.signal,
  };
}

function roleGain(role) {
  return {
    identity: 0.78,
    time: 0.64,
    tone: 0.58,
    motion: 0.56,
    color: 0.42,
    boundary: 0.5,
  }[role] ?? 0.6;
}

function transientRoleGain(role) {
  return {
    kick: 0.72,
    snare: 0.58,
    hat: 0.42,
  }[role] ?? 0.5;
}

function playbackToneFor({ height, brightness, presence, attack, punch }) {
  const heightValue = height ?? DEFAULT_HEIGHT;
  const heightCentered = heightValue - 0.5;
  const brightnessCentered = brightness - 0.5;
  const presenceCentered = presence - 0.5;
  const attackValue = attack ?? punch ?? DEFAULT_ATTACK;
  const attackCentered = attackValue - 0.5;
  return {
    height: round2(heightValue),
    focus: round2(heightValue),
    brightness: round2(brightness),
    presence: round2(presence),
    attack: round2(attackValue),
    punch: round2(attackValue),
    pitchShift: round2(heightCentered * 24),
    brightnessTilt: round2(brightnessCentered * 0.9),
    attackShape: round2(attackCentered * 2),
    toneFilter: 1,
    bassFilter: 1,
    noiseFilter: 1,
    leadGain: round2(2 ** (presenceCentered * 1.45)),
    harmonyGain: round2(2 ** (-presenceCentered * 0.38)),
    bassGain: round2(2 ** (-presenceCentered * 0.24)),
    highPercussionGain: 1,
    lowPercussionGain: 1,
    identityGain: round2(2 ** (presenceCentered * 1.05)),
    timeGain: 1,
    colorGain: round2(2 ** (-presenceCentered * 0.72)),
    boundaryGain: 1,
  };
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

function randomInt(rng, min, max) {
  return Math.floor(rng() * (max - min + 1)) + min;
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function normalizeBars(value) {
  const requested = Number(value ?? DEFAULT_BARS);
  return BAR_OPTIONS.reduce((best, candidate) => (
    Math.abs(candidate - requested) < Math.abs(best - requested) ? candidate : best
  ), DEFAULT_BARS);
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

function mulberry32(seed) {
  return function next() {
    let value = seed += 0x6D2B79F5;
    value = Math.imul(value ^ value >>> 15, value | 1);
    value ^= value + Math.imul(value ^ value >>> 7, value | 61);
    return ((value ^ value >>> 14) >>> 0) / 4294967296;
  };
}
