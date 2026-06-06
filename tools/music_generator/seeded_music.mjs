import { generateRandomTimbre, generateRandomTransient, randomTimbreSummary, randomTransientSummary } from "./seeded_timbre_fields.mjs?v=fields-1";

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
const DEFAULT_BARS = 8;
const BAR_OPTIONS = [8, 16, 32, 64];

const COMPOSITION_ROLES = ["identity", "time", "tone", "motion", "color", "boundary"];

function splitCompositionSeed(seedText) {
  const text = String(seedText);
  if (text.length <= 2) {
    return { variation: text, style: text, width: 0 };
  }
  return {
    variation: text,
    style: text.slice(2),
    width: 2,
  };
}

export function generateSong(seed, options = {}) {
  const seedText = String(seed);
  const seedParts = splitCompositionSeed(seedText);
  const styleRng = mulberry32(hashSeed(`style:${seedParts.style}`));
  const compositionRng = mulberry32(hashSeed(`composition:${seedText}`));
  const height = clamp(Number(options.height ?? options.focus ?? DEFAULT_HEIGHT), 0, 1);
  const brightness = clamp(Number(options.brightness ?? options.tone ?? DEFAULT_BRIGHTNESS), 0, 1);
  const presence = clamp(Number(options.presence ?? DEFAULT_PRESENCE), 0, 1);
  const attack = clamp(Number(options.attack ?? options.punch ?? DEFAULT_ATTACK), 0, 1);
  const bpm = clamp(Math.round(Number(options.bpm ?? DEFAULT_BPM)), 40, 180);
  const volume = clamp(Number(options.volume ?? DEFAULT_VOLUME), 0, 1);
  const bars = normalizeBars(options.bars);
  const [key, tonic] = pick(KEYS, styleRng);
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
  ], styleRng);
  const scale = SCALES[scaleName];
  const form = buildCompositionForm(styleRng);
  const roles = assignRoles(styleRng, form);
  const timbres = assignTimbres(styleRng, roles, seedParts.style);
  const progression = buildProgression(compositionRng, scale.length);
  const sectionPlan = markLoopHandoff(stabilizeBackboneContinuity(buildSectionPlan(compositionRng, form, roles, bars / 8)));
  const barPlan = buildBarStateTrajectory(sectionPlan, bars);
  const stepsPerBar = 16;
  const events = [];

  for (let bar = 0; bar < bars; bar += 1) {
    const section = Math.floor(bar / 8);
    const localBar = bar % 8;
    const sectionState = withPhraseBar(sectionPlan[section], barPlan[bar].phraseBar);
    const chordRoot = progression[(bar + sectionState.progressionShift) % progression.length] + sectionState.degreeOffset;
    const chord = buildChord(tonic, scale, chordRoot);
    addIdentity(events, sectionState.roles.identity, timbres, tonic, scale, chordRoot, bar, localBar, sectionState, seedText);
    addTime(events, sectionState.roles.time, timbres, chord, bar, localBar, sectionState, seedText);
    addTone(events, sectionState.roles.tone, timbres, chord, bar, localBar, sectionState, seedText);
    addMotion(events, sectionState.roles.motion, timbres, tonic, scale, chordRoot, bar, localBar, sectionState, seedText);
    addColor(events, sectionState.roles.color, timbres, chord, bar, localBar, sectionState, seedText);
    addBoundary(events, sectionState.roles.boundary, timbres, tonic, scale, bar, localBar, sectionState, seedText);
    addSectionBridge(events, tonic, scale, bar, localBar, sectionState, seedText);
    addSectionEntryBridge(events, tonic, scale, chord, bar, localBar, sectionState, seedText);
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
      seedParts,
      key,
      scale: scaleName,
      form,
      roles,
      timbres,
      progression: progression.map((degree) => degree + 1),
      sectionPlan,
      barPlan,
      trackMapping: trackMappingFor(roles),
    },
  };
}

export function randomPreset(seed = Date.now()) {
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

function buildCompositionForm(rng) {
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

function assignRoles(rng, form) {
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
    identity: role("identity", identityCarrier),
    time: role("time", timeCarrier),
    tone: role("tone", toneCarrier),
    motion: role("motion", motionCarrier),
    color: role("color", colorCarrier),
    boundary: role("boundary", boundaryCarrier),
  };
}

function role(name, carrier) {
  return { name, carrier };
}

function rolesWithCarriers(baseRoles, carriers = null) {
  return Object.fromEntries(COMPOSITION_ROLES.map((name) => [
    name,
    role(name, carriers?.[name] ?? baseRoles[name].carrier),
  ]));
}

function assignTimbres(rng, roles, seedText) {
  const pitched = Object.fromEntries(COMPOSITION_ROLES.map((name) => [
    name,
    pitchedFieldTimbre(`${seedText}:pitched:${name}:${roles[name].carrier}`, name),
  ]));
  const transient = {
    kick: transientFieldTimbre(`${seedText}:transient:kick`, "kick"),
    snare: transientFieldTimbre(`${seedText}:transient:snare`, "snare"),
    hat: transientFieldTimbre(`${seedText}:transient:hat`, "hat"),
  };
  return { pitched, transient };
}

function buildSectionPlan(rng, form, roles, sectionCount = 8) {
  const motif = () => randomInt(rng, 0, 9999);
  const distantIdentityCarrier = weightedPick([
    { item: "melodic-line", weight: roles.identity.carrier === "melodic-line" ? 0 : form.focus === "identity" ? 0.28 : 0.34 },
    { item: "harmony-arp", weight: roles.identity.carrier === "harmony-arp" ? 0 : form.focus === "motion" ? 0.3 : 0.22 },
    { item: "rhythm-hook", weight: roles.identity.carrier === "rhythm-hook" ? 0 : form.focus === "time" ? 0.28 : 0.2 },
    { item: "bass-riff", weight: roles.identity.carrier === "bass-riff" ? 0 : form.focus === "tone" ? 0.24 : 0.16 },
  ], rng);
  const distantTimeCarrier = weightedPick([
    { item: "drum-grid", weight: distantIdentityCarrier === "rhythm-hook" ? 0.02 : 0.34 },
    { item: "arp-pulse", weight: distantIdentityCarrier === "harmony-arp" ? 0.1 : 0.28 },
    { item: "bass-pulse", weight: distantIdentityCarrier === "bass-riff" ? 0.08 : 0.22 },
    { item: "thin-pulse", weight: form.space > 0.55 ? 0.24 : 0.12 },
  ], rng);
  const distantToneCarrier = weightedPick([
    { item: "chord-pad", weight: 0.34 },
    { item: "drone", weight: form.space > 0.5 ? 0.28 : 0.14 },
    { item: "root-bass", weight: distantIdentityCarrier === "bass-riff" ? 0.08 : 0.24 },
    { item: "implied", weight: form.density > 0.58 ? 0.2 : 0.1 },
  ], rng);
  const foregroundIdentityCarrier = weightedPick([
    { item: "melodic-line", weight: distantIdentityCarrier === "melodic-line" ? 0.62 : 0.58 },
    { item: "harmony-arp", weight: distantIdentityCarrier === "harmony-arp" ? 0.2 : 0.32 },
    { item: "bass-riff", weight: 0.08 },
  ], rng);
  const foregroundOverrides = {
    identity: foregroundIdentityCarrier,
    time: weightedPick([
      { item: "drum-grid", weight: 0.42 },
      { item: "arp-pulse", weight: foregroundIdentityCarrier === "harmony-arp" ? 0.1 : 0.28 },
      { item: "bass-pulse", weight: foregroundIdentityCarrier === "bass-riff" ? 0.08 : 0.2 },
      { item: "thin-pulse", weight: 0.08 },
    ], rng),
    tone: weightedPick([
      { item: "chord-pad", weight: 0.44 },
      { item: "root-bass", weight: foregroundIdentityCarrier === "bass-riff" ? 0.12 : 0.24 },
      { item: "drone", weight: 0.18 },
      { item: "implied", weight: 0.08 },
    ], rng),
    motion: weightedPick([
      { item: "answer-line", weight: foregroundIdentityCarrier === "melodic-line" ? 0.16 : 0.3 },
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
  const distantOverrides = {
    identity: distantIdentityCarrier,
    time: distantTimeCarrier,
    tone: distantToneCarrier,
    motion: weightedPick([
      { item: "answer-line", weight: distantIdentityCarrier === "melodic-line" ? 0.16 : 0.34 },
      { item: "harmony-arp", weight: distantTimeCarrier === "arp-pulse" ? 0.1 : 0.28 },
      { item: "bass-walk", weight: distantToneCarrier === "root-bass" ? 0.24 : 0.12 },
      { item: "percussion-fill", weight: distantTimeCarrier === "drum-grid" ? 0.22 : 0.1 },
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
  const trajectory = buildStateTrajectory(rng, form, sectionCount);
  return trajectory.map((state, index) => realizeSectionState(state, index, rng, form, roles, motif, distantOverrides, foregroundOverrides));
}

function buildStateTrajectory(rng, form, sectionCount) {
  if (sectionCount <= 1) {
    return [sectionVector(0, 0.08, 0.88, 0.42, 0.22, 0.86, 0.06)];
  }

  const trajectory = [];
  for (let index = 0; index < sectionCount; index += 1) {
    const progress = sectionCount <= 1 ? 0 : index / (sectionCount - 1);
    const previous = trajectory[index - 1] ?? null;
    const middleLift = Math.sin(progress * Math.PI);
    const noveltyBase = index === 0
      ? 0.08
      : clamp(0.1 + form.contrast * 0.28 + progress * 0.23 + middleLift * form.contrast * 0.12 + (rng() - 0.5) * 0.34, 0.02, 0.92);
    const closureBase = index === sectionCount - 1
      ? clamp(0.78 + rng() * 0.18, 0, 1)
      : clamp(0.16 + progress * progress * 0.42 + (rng() - 0.5) * 0.24, 0.04, 0.78);
    const memoryDistance = previous
      ? clamp(previous.memoryDistance * (0.48 + rng() * 0.22) + noveltyBase * 0.62 + middleLift * form.contrast * 0.08 - closureBase * 0.2, 0, 0.96)
      : 0.06;
    const tension = clamp(0.16 + noveltyBase * 0.5 + memoryDistance * 0.26 + progress * 0.14 + middleLift * form.contrast * 0.06 - closureBase * 0.22 + (rng() - 0.5) * 0.22, 0.04, 0.94);
    const density = clamp(form.density * 0.34 + 0.28 + tension * 0.22 + noveltyBase * 0.16 - form.space * 0.14 + (rng() - 0.5) * 0.18, 0.12, 0.92);
    const stability = clamp(0.9 - noveltyBase * 0.42 - memoryDistance * 0.22 - tension * 0.16 + closureBase * 0.24 + (rng() - 0.5) * 0.12, 0.08, 0.96);
    trajectory.push(sectionVector(progress, noveltyBase, stability, density, tension, closureBase, memoryDistance));
  }
  enforceStateTrajectory(trajectory, rng, sectionCount);
  return trajectory.slice(0, sectionCount);
}

function enforceStateTrajectory(trajectory, rng, sectionCount) {
  if (sectionCount >= 4 && !trajectory.some((state) => state.memoryDistance >= 0.5 || state.tension >= 0.68)) {
    const index = sectionCount === 4 ? 2 : randomInt(rng, 2, Math.max(2, sectionCount - 2));
    trajectory[index] = sectionVector(index / (sectionCount - 1), 0.68 + rng() * 0.18, 0.24 + rng() * 0.16, 0.56 + rng() * 0.22, 0.68 + rng() * 0.18, 0.24 + rng() * 0.16, 0.58 + rng() * 0.24);
  }
  if (sectionCount >= 8 && !trajectory.slice(4).some((state) => state.memoryDistance >= 0.5 || state.closurePressure >= 0.68 || state.density <= 0.32)) {
    const index = randomInt(rng, 4, sectionCount - 2);
    trajectory[index] = sectionVector(index / (sectionCount - 1), 0.58 + rng() * 0.28, 0.22 + rng() * 0.18, 0.24 + rng() * 0.54, 0.56 + rng() * 0.26, 0.34 + rng() * 0.34, 0.54 + rng() * 0.3);
  }
  const finalIndex = trajectory.length - 1;
  trajectory[finalIndex] = sectionVector(1, trajectory[finalIndex].novelty * 0.62, 0.68 + rng() * 0.22, trajectory[finalIndex].density, trajectory[finalIndex].tension * 0.72, 0.82 + rng() * 0.14, trajectory[finalIndex].memoryDistance * 0.48);
}

function sectionVector(progress, novelty, stability, density, tension, closurePressure, memoryDistance) {
  return {
    progress: round2(clamp(progress, 0, 1)),
    novelty: round2(clamp(novelty, 0, 1)),
    stability: round2(clamp(stability, 0, 1)),
    density: round2(clamp(density, 0, 1)),
    tension: round2(clamp(tension, 0, 1)),
    closurePressure: round2(clamp(closurePressure, 0, 1)),
    memoryDistance: round2(clamp(memoryDistance, 0, 1)),
  };
}

function realizeSectionState(state, index, rng, form, baseRoles, motif, distantOverrides, foregroundOverrides) {
  const variant = variantForState(state, rng);
  const degreeOffset = degreeOffsetForState(state, rng);
  const progressionShift = progressionShiftForState(state, rng);
  const energy = clamp(0.36 + state.density * 0.34 + state.tension * 0.24 + state.closurePressure * 0.16, 0.16, 0.96);
  const identityLevel = 0.76 + energy * 0.34 + state.stability * 0.12 - (state.density < 0.28 ? 0.16 : 0);
  const motionLevel = 0.66 + energy * 0.48 + state.memoryDistance * 0.18 + state.tension * 0.12 - state.closurePressure * 0.08;
  const colorLevel = 0.52 + state.density * 0.48 + state.novelty * 0.12;
  const boundaryLevel = 0.68 + state.closurePressure * 0.42 + state.memoryDistance * 0.18;
  const roleCarriers = roleCarriersForState(state, form, distantOverrides, foregroundOverrides, rng);
  const section = sectionState(
    index,
    variant,
    degreeOffset,
    progressionShift,
    clamp(identityLevel + state.progress * 0.01, 0.62, 1.28),
    clamp(motionLevel, 0.56, 1.36),
    clamp(colorLevel, 0.44, 1.24),
    clamp(boundaryLevel, 0.7, 1.34),
    motif(),
    state,
    energy,
    rolesWithCarriers(baseRoles, roleCarriers),
  );
  return section;
}

function variantForState(state, rng) {
  if (state.progress === 0) return 0;
  if (state.closurePressure > 0.72) return weightedPick([{ item: 0, weight: 0.28 }, { item: 1, weight: 0.24 }, { item: 3, weight: 0.48 }], rng);
  if (state.density < 0.32) return weightedPick([{ item: 0, weight: 0.36 }, { item: 3, weight: 0.42 }, { item: 1, weight: 0.22 }], rng);
  return state.memoryDistance > 0.5 ? pick([2, 3], rng) : pick([1, 2], rng);
}

function degreeOffsetForState(state, rng) {
  if (state.progress === 0) return 0;
  const center = Math.round((state.novelty - state.stability) * 2.2 + state.tension * 1.4 - state.closurePressure * 0.8);
  const width = state.memoryDistance > 0.55 ? 3 : 2;
  return clamp(center + randomInt(rng, -width, width), -2, 3);
}

function progressionShiftForState(state, rng) {
  if (state.progress === 0) return 0;
  const pullHome = state.closurePressure > 0.68 || state.stability > 0.72;
  return weightedPick([
    { item: 0, weight: pullHome ? 0.34 : 0.08 },
    { item: 1, weight: 0.18 + state.novelty * 0.16 },
    { item: 2, weight: 0.22 + state.tension * 0.18 },
    { item: 3, weight: 0.16 + state.memoryDistance * 0.24 },
  ], rng);
}

function roleCarriersForState(state, form, distantOverrides, foregroundOverrides, rng) {
  if (state.progress < 0.18 || state.memoryDistance < 0.2 && state.novelty < 0.24) {
    return null;
  }
  if (state.closurePressure > 0.7 && state.density >= 0.46) {
    return foregroundOverrides;
  }
  if (state.density <= 0.32 || form.space > 0.68 && state.stability < 0.48) {
    return {
      ...distantOverrides,
      time: "thin-pulse",
      color: form.space > 0.48 ? "noise-halo" : "air-pad",
      boundary: weightedPick([{ item: "rest-gap", weight: 0.58 }, { item: "register-turn", weight: 0.24 }, { item: "contrast-note", weight: 0.18 }], rng),
    };
  }
  if (state.memoryDistance >= 0.48 || state.novelty > 0.58) {
    return distantOverrides;
  }
  return rng() < 0.42 ? foregroundOverrides : null;
}

function sectionState(index, variant, degreeOffset, progressionShift, identityLevel, motionLevel, colorLevel, boundaryLevel, motifVariant, state, energy, roles) {
  return {
    name: `section-${index}`,
    index,
    variant,
    motifVariant,
    roles,
    degreeOffset,
    progressionShift,
    progress: state.progress,
    novelty: state.novelty,
    stability: state.stability,
    density: state.density,
    tension: state.tension,
    closurePressure: state.closurePressure,
    memoryDistance: state.memoryDistance,
    energy: round2(energy),
    identityLevel: round2(identityLevel),
    motionLevel: round2(motionLevel),
    colorLevel: round2(colorLevel),
    boundaryLevel: round2(boundaryLevel),
  };
}

function buildPhraseShape(index, variant, degreeOffset, progressionShift, motifVariant, state, context = {}) {
  const rng = mulberry32(hashSeed([
    index,
    variant,
    degreeOffset,
    progressionShift,
    motifVariant,
    state.novelty,
    state.stability,
    state.density,
    state.tension,
    state.closurePressure,
    state.memoryDistance,
  ].join(":")));
  const curve = buildPhraseEnergyCurve(rng, state, variant);
  const energies = Array.from({ length: 8 }, (_, index) => phraseEnergyAt(index, curve, rng));
  const bars = [];
  let previousTarget = 0;
  for (let index = 0; index < 8; index += 1) {
    const energy = energies[index];
    const nextEnergy = energies[Math.min(7, index + 1)];
    const previousEnergy = energies[Math.max(0, index - 1)];
    const slopeIn = energy - previousEnergy;
    const slopeOut = nextEnergy - energy;
    const barState = phraseBarState(index, energy, slopeIn, slopeOut, previousTarget, state, rng);
    const entryProgress = transitionEntryProgress(context, index);
    const boundary = phraseBoundaryForBar(index, barState, slopeIn, slopeOut, energy, rng) * entryProgress;
    const pickup = phrasePickupForBar(index, energy, slopeOut, boundary, rng) * entryProgress;
    const space = phraseSpaceForBar(energy, barState, rng);
    bars.push({
      index,
      targetCenter: round2(barState.targetCenter),
      heightBias: round2(barState.heightBias),
      closure: round2(barState.closure),
      tension: round2(barState.tension),
      stability: round2(barState.stability),
      pace: round2(barState.pace),
      energy: round2(energy),
      space: round2(space),
      boundary: round2(boundary),
      pickup: round2(pickup),
      toneAnchor: phraseToneAnchor(index, barState, energy, boundary, rng),
      colorAccent: phraseColorAccent(index, energy, pickup, space, rng),
      syncopation: round2(clamp(0.18 + rng() * 0.5 + pickup * 0.25 + Math.abs(slopeOut) * 0.18, 0.16, 0.84)),
    });
    previousTarget = barState.targetCenter;
  }
  return { archetype: curve.archetype, curve, bars };
}

function transitionEntryProgress(context, barIndex) {
  const incomingImpact = Number(context.transitionIn?.impact ?? 0);
  if (incomingImpact < 0.46) {
    return 1;
  }
  const bars = transitionSpan(incomingImpact);
  if (barIndex >= bars) {
    return 1;
  }
  return smoothstep((barIndex + 1) / (bars + 1));
}

function smoothstep(value) {
  const x = clamp(value, 0, 1);
  return x * x * (3 - 2 * x);
}

function buildPhraseEnergyCurve(rng, state, variant) {
  const firstPivot = randomInt(rng, 1, 3);
  const secondPivot = randomInt(rng, 4, 6);
  const latePressure = state.closurePressure * 0.68 + state.stability * 0.18;
  const spreadPressure = state.memoryDistance * 0.56 + state.tension * 0.28;
  const peakIndex = weightedPick(Array.from({ length: 7 }, (_, offset) => {
    const candidate = offset + 1;
    return {
      item: candidate,
      weight: 0.08
        + gaussianScore(candidate, 2 + spreadPressure * 3.5, 1.8) * (1 - latePressure)
        + gaussianScore(candidate, 5.8, 1.4) * latePressure,
    };
  }), rng);
  const valleyIndex = weightedPick(Array.from({ length: 6 }, (_, offset) => {
    const candidate = offset + 1;
    return {
      item: candidate,
      weight: 0.08 + gaussianScore(candidate, 2.4 + state.stability * 2.8 + (1 - state.density) * 1.4, 1.9),
    };
  }), rng);
  const start = clamp(0.7 + state.stability * 0.22 + state.density * 0.24 + rng() * 0.26, 0.56, 1.24);
  const end = clamp(0.68 + state.closurePressure * 0.38 + state.tension * 0.16 + rng() * 0.26, 0.56, 1.36);
  const peak = clamp(Math.max(start, end) + 0.1 + state.tension * 0.2 + state.density * 0.1 + rng() * 0.2, 0.82, 1.38);
  const valley = clamp(Math.min(start, end) - 0.08 - (1 - state.density) * 0.18 - state.stability * 0.06 - rng() * 0.16, 0.44, 1.08);
  const midA = clamp(start + (rng() - 0.44) * 0.34 + variant * 0.025, 0.56, 1.28);
  const midB = clamp(end + (rng() - 0.5) * 0.36 - (1 - state.density) * 0.08, 0.52, 1.3);
  const controls = mergePhraseControls([
    { index: 0, value: start },
    { index: firstPivot, value: midA },
    { index: valleyIndex, value: valley },
    { index: peakIndex, value: peak },
    { index: secondPivot, value: midB },
    { index: 7, value: end },
  ]);
  const archetype = `state-${peakIndex}-${valleyIndex}-${Math.round(state.novelty * 10)}-${Math.round(state.closurePressure * 10)}`;
  return { archetype, controls, peakIndex, valleyIndex };
}

function mergePhraseControls(points) {
  const merged = new Map();
  for (const point of points) {
    const previous = merged.get(point.index);
    if (!previous || Math.abs(point.value - 1) > Math.abs(previous.value - 1)) {
      merged.set(point.index, point);
    }
  }
  return [...merged.values()].sort((left, right) => left.index - right.index);
}

function phraseEnergyAt(index, curve, rng) {
  let left = curve.controls[0];
  let right = curve.controls[curve.controls.length - 1];
  for (let controlIndex = 0; controlIndex < curve.controls.length - 1; controlIndex += 1) {
    const a = curve.controls[controlIndex];
    const b = curve.controls[controlIndex + 1];
    if (index >= a.index && index <= b.index) {
      left = a;
      right = b;
      break;
    }
  }
  const span = Math.max(1, right.index - left.index);
  const x = (index - left.index) / span;
  const eased = x * x * (3 - 2 * x);
  const wave = Math.sin((index + 1) * 0.85 + curve.peakIndex) * 0.04;
  const jitter = (rng() - 0.5) * 0.08;
  return clamp(interpolate(left.value, right.value, eased) + wave + jitter, 0.5, 1.36);
}

function phraseBarState(index, energy, slopeIn, slopeOut, previousTarget, state, rng) {
  const rising = slopeOut > 0.06;
  const falling = slopeOut < -0.06;
  const localProgress = index / 7;
  const closure = clamp(state.closurePressure * (0.42 + localProgress * 0.68) + Number(index === 7) * 0.42 + (falling ? 0.12 : 0) + (rng() - 0.5) * 0.14, 0, 1);
  const tension = clamp(state.tension * 0.72 + state.novelty * 0.2 + (rising ? 0.18 : 0) + Math.max(0, energy - 1) * 0.22 - closure * 0.18 + (rng() - 0.5) * 0.16, 0, 1);
  const stability = clamp(state.stability * 0.72 + closure * 0.26 - tension * 0.18 + (rng() - 0.5) * 0.12, 0, 1);
  const drift = clamp(previousTarget * 0.42 + (state.memoryDistance - 0.5) * 0.8 + slopeOut * 1.8 + (rng() - 0.5) * (0.8 + state.novelty), -1, 1);
  const targetCenter = clamp(drift * (1 - closure * 0.42) - stability * 0.24 + tension * 0.28, -1, 1);
  const heightBias = clamp((energy - 0.94) * 0.72 + tension * 0.36 - stability * 0.18 + (rng() - 0.5) * 0.28, -1, 1);
  const pace = clamp(0.18 + state.density * 0.46 + tension * 0.26 - stability * 0.16 + Math.max(0, energy - 1) * 0.14 + (rng() - 0.5) * 0.16, 0.08, 0.92);
  return { targetCenter, heightBias, closure, tension, stability, pace };
}

function phraseBoundaryForBar(index, barState, slopeIn, slopeOut, energy, rng) {
  if (index === 7) {
    return 1;
  }
  const turns = Math.sign(slopeIn) !== Math.sign(slopeOut) && Math.abs(slopeIn - slopeOut) > 0.08;
  const probability = 0.06 + barState.closure * 0.44 + (turns ? 0.18 : 0) + (energy > 1.12 ? 0.08 : 0);
  return rng() < probability ? clamp(0.42 + rng() * 0.28 + barState.closure * 0.34, 0, 0.92) : 0;
}

function phrasePickupForBar(index, energy, slopeOut, boundary, rng) {
  if (index >= 7) {
    return 0;
  }
  const lift = Math.max(0, slopeOut);
  const boundaryPush = boundary > 0.58 ? 0.12 : 0;
  return clamp((lift > 0.05 ? 0.2 + lift * 0.82 : rng() < 0.12 ? 0.18 : 0) + boundaryPush + (energy > 1.12 ? 0.06 : 0), 0, 0.74);
}

function phraseSpaceForBar(energy, barState, rng) {
  const lowEnergySpace = clamp(0.38 - energy * 0.18, 0, 0.28);
  const instabilitySpace = clamp((1 - barState.stability) * 0.2 + (1 - barState.tension) * 0.08, 0, 0.28);
  const closureSpace = barState.closure > 0.7 ? 0.08 : 0;
  return clamp(0.1 + lowEnergySpace + instabilitySpace + closureSpace + rng() * 0.22, 0.08, 0.78);
}

function phraseToneAnchor(index, barState, energy, boundary, rng) {
  if (index === 0 || index === 7 || barState.stability > 0.66 || barState.closure > 0.68) {
    return true;
  }
  return rng() < 0.2 + (energy < 0.92 ? 0.18 : 0) + (boundary > 0.58 ? 0.18 : 0);
}

function phraseColorAccent(index, energy, pickup, space, rng) {
  if (index === 7) {
    return rng() < 0.42;
  }
  return rng() < 0.14 + (energy > 1.04 ? 0.22 : 0) + pickup * 0.18 + (space > 0.48 ? 0.12 : 0);
}

function phraseBar(sectionState, localBar) {
  return sectionState.phraseBar ?? {
    index: localBar % 8,
    targetCenter: 0,
    heightBias: 0,
    closure: localBar === 7 ? 1 : 0.24,
    tension: 0.32,
    stability: 0.62,
    pace: 0.42,
    energy: 1,
    space: 0.24,
    boundary: localBar === 7 ? 1 : 0,
    pickup: 0,
    toneAnchor: localBar % 2 === 0,
    colorAccent: localBar === 1 || localBar === 5,
    syncopation: 0.42,
  };
}

function withPhraseBar(sectionState, barShape) {
  return {
    ...sectionState,
    phraseBar: barShape,
  };
}

function markLoopHandoff(sectionPlan) {
  const finalIndex = sectionPlan.length - 1;
  return sectionPlan.map((section, index) => ({
    ...section,
    loopHandoff: index === finalIndex,
    loopTarget: index === finalIndex ? {
      degreeOffset: 0,
      progressionShift: 0,
    } : null,
  }));
}

function stabilizeBackboneContinuity(sectionPlan) {
  if (sectionPlan.length <= 1) {
    return sectionPlan;
  }
  const backbone = COMPOSITION_ROLES;
  const result = [...sectionPlan];
  for (let pass = 0; pass < result.length * 2; pass += 1) {
    let changed = false;
    for (let index = 0; index < result.length; index += 1) {
      const nextIndex = (index + 1) % result.length;
      const currentCarriers = effectiveCarriers(result[index], backbone);
      const nextCarriers = effectiveCarriers(result[nextIndex], backbone);
      if (sharesContinuityCarrier(currentCarriers, nextCarriers, backbone)) {
        continue;
      }
      result[nextIndex] = withSectionRoleCarrier(result[nextIndex], "time", currentCarriers.time);
      changed = true;
    }
    if (!changed) {
      break;
    }
  }
  return result;
}

function buildBarStateTrajectory(sectionPlan, bars) {
  if (sectionPlan.length === 0 || bars <= 0) {
    return [];
  }
  return sectionPlan.flatMap((section, index) => {
    const previous = index > 0 ? sectionPlan[index - 1] : null;
    const next = index < sectionPlan.length - 1 ? sectionPlan[index + 1] : null;
    const transitionIn = previous ? transitionContext(previous, section) : null;
    const transitionOut = next ? transitionContext(section, next) : null;
    const phraseShape = buildPhraseShape(section.index, section.variant, section.degreeOffset, section.progressionShift, section.motifVariant, sectionStateVector(section), {
      transitionIn,
      transitionOut,
    });
    return phraseShape.bars.map((phraseBar) => ({
      bar: index * 8 + phraseBar.index,
      sectionIndex: section.index,
      localBar: phraseBar.index,
      phraseArchetype: phraseShape.archetype,
      phraseBar: withTransitionProjection(phraseBar, previous, section, next, transitionIn, transitionOut),
      transitionIn,
      transitionOut,
    }));
  }).slice(0, bars);
}

function withTransitionProjection(phraseBar, previous, section, next, transitionIn, transitionOut) {
  return {
    ...phraseBar,
    transitionIn,
    transitionOut,
    transitionEntryBridge: previous && transitionIn ? transitionBridge(previous, section, transitionIn) : null,
    transitionBridge: next && transitionOut ? transitionBridge(section, next, transitionOut) : null,
  };
}

function transitionBridge(left, right, transitionOut) {
  const leftCarriers = effectiveCarriers(left, COMPOSITION_ROLES);
  const rightCarriers = effectiveCarriers(right, COMPOSITION_ROLES);
  const roleName = COMPOSITION_ROLES.find((name) => (
    leftCarriers[name] === rightCarriers[name]
      && isContinuityCarrier(name, leftCarriers[name])
  ));
  if (!roleName) {
    throw new Error(`transition ${left.name}->${right.name} has no continuity carrier`);
  }
  const carrier = rightCarriers[roleName];
  const tracks = tracksForCarrier(carrier);
  if (tracks.length === 0) {
    throw new Error(`transition ${left.name}->${right.name} continuity carrier ${carrier} has no playback track`);
  }
  return {
    role: roleName,
    carrier,
    track: tracks[0],
    targetDegreeOffset: right.degreeOffset,
    targetProgressionShift: right.progressionShift,
    impact: transitionOut.impact,
    bars: transitionOut.bars,
  };
}

function transitionContext(left, right) {
  const impact = sectionTransitionImpact(left, right);
  if (impact < 0.46) {
    return null;
  }
  return {
    impact: round2(impact),
    bars: transitionSpan(impact),
  };
}

function transitionSpan(impact) {
  return impact > 0.72 ? 3 : 2;
}

function sectionStateVector(section) {
  return sectionVector(
    section.progress,
    section.novelty,
    section.stability,
    section.density,
    section.tension,
    section.closurePressure,
    section.memoryDistance,
  );
}

function sectionTransitionImpact(left, right) {
  const carrierChange = carrierChangeRatio(left, right);
  const densityLift = Math.max(0, right.density - left.density);
  const energyLift = Math.max(0, right.energy - left.energy);
  const noveltyLift = Math.max(0, right.novelty - left.novelty);
  const distanceLift = Math.max(0, right.memoryDistance - left.memoryDistance);
  const closureLift = Math.max(0, right.closurePressure - left.closurePressure);
  const foregroundLift = carrierChange > 0 ? 0.1 : 0;
  return clamp(
    carrierChange * 0.34
      + densityLift * 0.24
      + energyLift * 0.44
      + noveltyLift * 0.18
      + distanceLift * 0.18
      + closureLift * 0.1
      + foregroundLift,
    0,
    1,
  );
}

function carrierChangeRatio(left, right) {
  const leftCarriers = effectiveCarriers(left, COMPOSITION_ROLES);
  const rightCarriers = effectiveCarriers(right, COMPOSITION_ROLES);
  const changed = COMPOSITION_ROLES.filter((name) => leftCarriers[name] !== rightCarriers[name]).length;
  return changed / COMPOSITION_ROLES.length;
}

function sharesContinuityCarrier(left, right, names) {
  return names.some((name) => (
    left[name] === right[name]
      && isContinuityCarrier(name, left[name])
  ));
}

function isContinuityCarrier(name, carrier) {
  if (name === "identity" || name === "time") {
    return carrier !== "none";
  }
  if (name === "tone") {
    return carrier !== "implied" && carrier !== "none";
  }
  if (name === "motion") {
    return carrier === "answer-line" || carrier === "harmony-arp" || carrier === "bass-walk";
  }
  if (name === "color") {
    return carrier === "air-pad" || carrier === "noise-halo" || carrier === "organ-bed";
  }
  return false;
}

function effectiveCarriers(section, names) {
  return Object.fromEntries(names.map((name) => [
    name,
    section.roles[name].carrier,
  ]));
}

function withSectionRoleCarrier(section, name, carrier) {
  return {
    ...section,
    roles: {
      ...section.roles,
      [name]: role(name, carrier),
    },
  };
}

function addIdentity(events, role, timbres, tonic, scale, chordRoot, bar, localBar, sectionState, seedText) {
  if (role.carrier === "melodic-line") {
    const pattern = generateMelodicLine(seedText, localBar, sectionState);
    for (const [step, offset, duration] of pattern) {
      events.push(noteEvent("lead", bar, step, duration, [degreeNote(tonic, scale, chordRoot + offset, 12)], "identity", 0.13 * sectionState.identityLevel));
    }
    return;
  }
  if (role.carrier === "bass-riff") {
    const pattern = generateBassRiff(seedText, localBar, sectionState, "identity");
    for (const [step, offset] of pattern) {
      events.push(noteEvent("bass", bar, step, bassDuration(step), [degreeNote(tonic, scale, chordRoot + offset, -24)], "identity", 0.17 * sectionState.identityLevel));
    }
    return;
  }
  if (role.carrier === "harmony-arp") {
    const pattern = generateHarmonyArp(seedText, localBar, sectionState, "identity");
    for (const [step, offset, duration] of pattern) {
      events.push(noteEvent("chord", bar, step, duration, [degreeNote(tonic, scale, chordRoot + offset, 12)], "identity", 0.075 * sectionState.identityLevel));
    }
    return;
  }
  const barLift = localBar === 7 ? 1.12 : 1;
  for (const hit of generateRhythmHook(seedText, localBar, sectionState, "identity")) {
    const base = hit.sound === "kick" ? 0.22 : hit.sound === "snare" ? 0.17 : 0.085;
    events.push(noiseEvent("drums", bar, hit.step, hit.sound, "identity", base * hit.weight * sectionState.identityLevel * barLift));
  }
}

function generateMelodicLine(seedText, localBar, sectionState) {
  const barShape = phraseBar(sectionState, localBar);
  const rng = eventRng(seedText, "identity", "melodic-line", sectionState, localBar);
  const sparse = sectionState.density < 0.32 || barShape.space > 0.6;
  const lift = sectionState.closurePressure > 0.66 && barShape.energy > 0.96 || barShape.tension > 0.68;
  const countBase = 1.6 + barShape.pace * 3.1 + (lift ? 0.72 : 0) - (sparse ? 0.58 : 0);
  const count = clamp(Math.round((countBase + rng() * 1.2 + sectionState.identityLevel * 0.18) * clamp(0.72 + barShape.energy * 0.28, 0.72, 1.12)), 1, 7);
  const start = barShape.pickup > 0.45 || barShape.tension > 0.64
    ? randomInt(rng, 1, 3)
    : sparse
      ? randomInt(rng, 0, 4)
      : randomInt(rng, 0, 1);
  const phraseEndStep = barShape.boundary > 0.7
    ? randomInt(rng, 13, 15)
    : barShape.tension > 0.58 && barShape.closure < 0.58
      ? randomInt(rng, 11, 14)
      : randomInt(rng, 9, 15);
  const steps = stochasticOnsets(rng, count, {
    min: start,
    max: phraseEndStep,
    anchorEnd: phraseEndStep,
    strongBeatBias: sparse ? 0.28 : 0.52,
    syncopation: barShape.syncopation + sectionState.variant * 0.06 + rng() * 0.12,
    minGap: barShape.pace < 0.34 || sparse ? 4 : barShape.pace < 0.56 ? 3 : 2,
  });
  return shapeMelodicPhrase(steps.map((step) => [step, 0, melodicDuration(step, phraseEndStep, sparse, barShape.pace, rng)]), localBar, sectionState, rng);
}

function shapeMelodicPhrase(pattern, localBar, sectionState, rng) {
  const barShape = phraseBar(sectionState, localBar);
  const frame = melodicFrameForBar(sectionState, barShape, rng);
  const targetPitch = melodicTargetForBar(frame, barShape);
  const count = pattern.length;
  const candidates = Array.from({ length: frame.max - frame.min + 1 }, (_, index) => frame.min + index);
  const offsets = samplePitchPath(candidates, count, rng, {
    initialPitch: frame.start,
    targetPitch,
    barShape,
    sectionState,
    exactFinal: barShape.boundary > 0.72,
  });

  return pattern.map(([step, sourceOffset, duration], index) => {
    const isLanding = index === count - 1 || step >= 14;
    const sourceDetail = isLanding ? 0 : clamp(Math.round((sourceOffset - offsets[index]) * 0.18), -1, 1);
    const offset = isLanding && barShape.boundary > 0.72
      ? targetPitch
      : clamp(offsets[index] + sourceDetail, frame.min, frame.max);
    const nextDuration = isLanding ? Math.max(duration, barShape.boundary > 0.58 ? 3 : 2) : Math.max(barShape.pace < 0.38 ? 3 : 2, duration);
    return [step, offset, nextDuration];
  });
}

function melodicFrameForBar(sectionState, barShape, rng) {
  const distance = Number(sectionState.memoryDistance ?? 0.24);
  const energy = Number(sectionState.energy ?? barShape.energy ?? 0.5);
  const lift = sectionState.closurePressure > 0.66 && barShape.energy > 1.02 || barShape.tension > 0.68;
  const sparse = sectionState.density < 0.32 || barShape.space > 0.58;
  const center = clamp(
    Math.round(sectionState.degreeOffset * 0.38 + distance * 5.2 + (energy - 0.5) * 2.1 + randomInt(rng, -1, 1)),
    -2,
    7,
  );
  const spread = clamp(
    Math.round(3 + distance * 3.4 + barShape.energy * 1.2 + (lift ? 1.2 : 0) - (sparse ? 1.1 : 0) + rng() * 1.8),
    3,
    10,
  );
  const min = clamp(center - Math.ceil(spread * (0.45 + rng() * 0.18)), -4, 5);
  const max = clamp(center + Math.ceil(spread * (0.55 + rng() * 0.22)), min + 2, 11);
  const start = clamp(center + randomInt(rng, -2, 1), min, max);
  const outward = clamp(start + weightedPick([
    { item: -2, weight: sparse ? 0.28 : 0.12 },
    { item: -1, weight: 0.18 },
    { item: 1, weight: 0.26 },
    { item: 2, weight: 0.26 },
    { item: 3, weight: lift ? 0.18 : 0.08 },
  ], rng), min, max);
  const upper = clamp(Math.max(start, outward) + randomInt(rng, 1, Math.max(2, max - Math.max(start, outward) + 1)), min, max);
  const lower = clamp(Math.min(start, outward) - randomInt(rng, 1, Math.max(2, Math.min(start, outward) - min + 1)), min, max);
  const settled = clamp(center + weightedPick([
    { item: 0, weight: sectionState.loopHandoff || sectionState.closurePressure > 0.68 ? 0.48 : 0.2 },
    { item: -1, weight: 0.2 },
    { item: 1, weight: 0.22 },
    { item: 2, weight: distance > 0.52 ? 0.2 : 0.1 },
  ], rng), min, max);
  const open = clamp(settled + weightedPick([
    { item: 1, weight: 0.34 },
    { item: 2, weight: 0.38 },
    { item: 3, weight: distance > 0.42 ? 0.18 : 0.08 },
    { item: -1, weight: 0.1 },
  ], rng), min, max);
  return { start, outward, upper, lower, settled, open, center, spread, min, max };
}

function melodicTargetForBar(frame, barShape) {
  const span = Math.max(1, frame.max - frame.min);
  const rawTarget = frame.center
    + barShape.targetCenter * span * 0.32
    + barShape.heightBias * span * 0.18
    + barShape.tension * 1.2
    - barShape.stability * 0.8;
  const stableTarget = interpolate(rawTarget, frame.settled, barShape.closure * 0.62);
  const openTarget = interpolate(stableTarget, frame.open, Math.max(0, barShape.tension - barShape.closure) * 0.42);
  return clamp(Math.round(openTarget), frame.min, frame.max);
}

function samplePitchPath(candidates, count, rng, { initialPitch, targetPitch, barShape, sectionState, exactFinal = false }) {
  const path = [];
  const used = [];
  let previous = nearestCandidate(candidates, initialPitch);
  let previousDirection = 0;
  const distance = Number(sectionState.memoryDistance ?? 0.3);
  const localRandomness = clamp(0.12 + distance * 0.22 + barShape.syncopation * 0.18 + rng() * 0.14, 0.08, 0.58);
  for (let index = 0; index < count; index += 1) {
    const remaining = count - index - 1;
    if (exactFinal && remaining === 0) {
      const finalPitch = nearestCandidate(candidates, targetPitch);
      path.push(finalPitch);
      break;
    }
    const progress = count <= 1 ? 1 : index / (count - 1);
    const bridgeCenter = previous + (targetPitch - previous) / Math.max(1, remaining + 1);
    const targetPull = clamp(progress * progress * (0.28 + barShape.boundary * 0.42 + (sectionState.loopHandoff ? 0.18 : 0)), 0.12, 0.92);
    const next = weightedPick(candidates.map((pitch) => {
      const bridgeScore = gaussianScore(pitch, bridgeCenter, 0.9 + localRandomness * 3.8);
      const targetScore = gaussianScore(pitch, targetPitch, 1.1 + (1 - targetPull) * 5);
      const smoothScore = gaussianScore(pitch, previous, 1.0 + localRandomness * 5.5);
      const actualDirection = Math.sign(pitch - previous);
      const reversal = previousDirection && actualDirection && actualDirection !== previousDirection;
      const reversalPenalty = reversal ? 0.54 + localRandomness * 0.54 : 1;
      const repeatWeight = pitch === previous ? 0.84 + localRandomness * 0.12 : 1;
      const reuseWeight = used.includes(pitch) ? 0.86 + localRandomness * 0.1 : 1.04;
      const feasible = gaussianScore(pitch, targetPitch, Math.max(1.2, (remaining + 1) * (2.2 + localRandomness * 3)));
      return {
        item: pitch,
        weight: Math.max(0.001, (
          0.04
          + bridgeScore * 0.52
          + targetScore * (0.16 + targetPull * 0.44)
          + smoothScore * 0.28
          + feasible * 0.18
          + rng() * localRandomness * 0.08
        ) * reversalPenalty * repeatWeight * reuseWeight),
      };
    }), rng);
    previousDirection = Math.sign(next - previous) || previousDirection;
    previous = next;
    path.push(next);
    used.push(next);
  }
  return path;
}

function nearestCandidate(candidates, target) {
  return candidates.reduce((best, candidate) => Math.abs(candidate - target) < Math.abs(best - target) ? candidate : best, candidates[0]);
}

function interpolate(left, right, amount) {
  return left + (right - left) * clamp(amount, 0, 1);
}

function eventRng(seedText, role, carrier, sectionState, localBar) {
  return mulberry32(hashSeed([
    seedText,
    role,
    carrier,
    sectionState.name,
    sectionState.motifVariant,
    sectionState.variant,
    sectionState.novelty,
    sectionState.stability,
    sectionState.tension,
    sectionState.closurePressure,
    localBar,
  ].join(":")));
}

function stochasticOnsets(rng, count, options = {}) {
  const min = Math.max(0, Number(options.min ?? 0));
  const max = Math.min(15, Number(options.max ?? 15));
  const minGap = Math.max(1, Number(options.minGap ?? 1));
  const targetCount = Math.max(1, Math.round(count));
  const selected = [];
  if (options.anchorStart) {
    selected.push(min);
  }
  if (options.anchorEnd !== undefined) {
    selected.push(clamp(Math.round(options.anchorEnd), min, max));
  }
  const candidates = [];
  for (let step = min; step <= max; step += 1) {
    candidates.push(step);
  }
  while (selected.length < targetCount && candidates.length > 0) {
    const available = candidates.filter((step) => !selected.includes(step) && selected.every((other) => Math.abs(step - other) >= minGap));
    const pool = available.length > 0 ? available : candidates.filter((step) => !selected.includes(step));
    if (pool.length === 0) {
      break;
    }
    const step = weightedStep(pool, rng, options);
    selected.push(step);
  }
  return [...new Set(selected)].sort((left, right) => left - right).slice(0, targetCount);
}

function weightedStep(steps, rng, options = {}) {
  const weights = steps.map((step) => onsetWeight(step, options));
  const total = weights.reduce((sum, weight) => sum + weight, 0);
  let ticket = rng() * total;
  for (let index = 0; index < steps.length; index += 1) {
    ticket -= weights[index];
    if (ticket <= 0) {
      return steps[index];
    }
  }
  return steps[steps.length - 1];
}

function onsetWeight(step, options = {}) {
  const strongBeatBias = Number(options.strongBeatBias ?? 0.5);
  const syncopation = Number(options.syncopation ?? 0.35);
  const strongDistance = Math.min(...[0, 4, 8, 12].map((anchor) => Math.abs(step - anchor)));
  const offDistance = Math.min(...[2, 6, 10, 14, 15].map((anchor) => Math.abs(step - anchor)));
  const strong = Math.exp(-strongDistance * 0.9);
  const off = Math.exp(-offDistance * 0.9);
  return 0.08 + strong * strongBeatBias + off * syncopation;
}

function pulseOnsets(rng, count, options = {}) {
  const phase = randomInt(rng, 0, Math.max(0, Number(options.phaseMax ?? 2)));
  const minGap = Math.max(1, Number(options.minGap ?? 2));
  const targetCount = Math.max(1, Math.round(count));
  const pulsePeriod = clamp(3 + rng() * 4, 3, 7);
  const swing = (rng() - 0.5) * 1.8;
  const steps = [];
  while (steps.length < targetCount) {
    const candidates = Array.from({ length: 16 }, (_, index) => index).filter((candidate) => steps.every((other) => Math.abs(candidate - other) >= minGap));
    if (candidates.length === 0) {
      break;
    }
    const weights = candidates.map((step) => {
      const pulsePosition = ((step - phase + 16) % pulsePeriod);
      const folded = Math.min(pulsePosition, pulsePeriod - pulsePosition);
      const pulse = Math.exp(-(folded * folded) / 2.2);
      const strongGridPenalty = [0, 4, 8, 12].includes(step) && steps.filter((other) => [0, 4, 8, 12].includes(other)).length >= 1 ? 0.58 : 1;
      const localSwing = step % 2 === 1 ? 1 + Math.max(0, swing) * 0.08 : 1 + Math.max(0, -swing) * 0.06;
      return (0.08 + pulse * 0.52 + onsetWeight(step, {
        strongBeatBias: Number(options.strongBeatBias ?? 0.28),
        syncopation: Number(options.syncopation ?? 0.62),
      }) * 0.48) * strongGridPenalty * localSwing;
    });
    let ticket = rng() * weights.reduce((sum, weight) => sum + weight, 0);
    for (let index = 0; index < candidates.length; index += 1) {
      ticket -= weights[index];
      if (ticket <= 0) {
        steps.push(candidates[index]);
        break;
      }
    }
  }
  return steps.sort((left, right) => left - right);
}

function texturalEventCount(rng, { mean, minimum = 1, burst = 0 }) {
  const base = poissonCount(rng, Math.max(0.1, mean));
  const burstCount = rng() < clamp(burst, 0, 0.5)
    ? 1 + poissonCount(rng, 1.4 + rng() * 1.6)
    : 0;
  return Math.max(minimum, base + burstCount);
}

function poissonCount(rng, mean) {
  const limit = Math.exp(-Math.max(0.01, mean));
  let product = 1;
  let count = 0;
  do {
    count += 1;
    product *= rng();
  } while (product > limit);
  return count - 1;
}

function melodicDuration(step, phraseEndStep, sparse, pace, rng) {
  if (step >= phraseEndStep - 1) {
    return sparse || pace < 0.38 ? randomInt(rng, 4, 7) : randomInt(rng, 2, 5);
  }
  if (pace < 0.32) {
    return randomInt(rng, 4, 7);
  }
  if (pace < 0.5 || sparse) {
    return randomInt(rng, 3, 5);
  }
  return randomInt(rng, 2, 3);
}

function bassDuration(step) {
  return step <= 1 ? 3 : step >= 12 ? 2 : 2 + Number(step % 4 === 0);
}

function generateBassRiff(seedText, localBar, sectionState, role) {
  const rng = eventRng(seedText, role, "bass-riff", sectionState, localBar);
  const barShape = phraseBar(sectionState, localBar);
  const count = clamp(Math.round(1.4 + barShape.pace * 2.2 + Number(rng() > 0.68 - barShape.pickup * 0.22 && sectionState.density > 0.28)), 1, 4);
  const steps = stochasticOnsets(rng, count, {
    min: 0,
    max: 14,
    anchorStart: true,
    strongBeatBias: 0.72,
    syncopation: 0.18 + barShape.syncopation * 0.65 + rng() * 0.18,
    minGap: barShape.pace < 0.42 ? 5 : 3,
  });
  return steps.map((step, index) => {
    if (index === 0 || barShape.boundary > 0.58 && index === steps.length - 1 && rng() < 0.62) {
      return [step, 0];
    }
    const approachWeight = step >= 11 ? 0.36 : 0.18;
    const offset = weightedPick([
      { item: 0, weight: 0.28 },
      { item: 1, weight: 0.18 + approachWeight },
      { item: 2, weight: 0.24 },
      { item: 4, weight: 0.18 },
      { item: -1, weight: 0.12 },
    ], rng);
    return [step, offset];
  });
}

function generateHarmonyArp(seedText, localBar, sectionState, role) {
  const rng = eventRng(seedText, role, "harmony-arp", sectionState, localBar);
  const barShape = phraseBar(sectionState, localBar);
  const dense = sectionState.closurePressure > 0.66 && sectionState.density > 0.52 || sectionState.motionLevel > 1.15;
  const count = texturalEventCount(rng, {
    mean: 0.9 + barShape.pace * 1.65 + (dense ? 0.34 : 0),
    minimum: barShape.pace < 0.34 ? 1 : 2,
    burst: 0.04 + barShape.tension * 0.08 + barShape.pickup * 0.1,
  });
  const steps = pulseOnsets(rng, count, { phaseMax: 3, minGap: barShape.pace < 0.5 ? 4 : 3, strongBeatBias: 0.2, syncopation: 0.58 + barShape.syncopation * 0.16 });
  const harmonicTargets = harmonicTargetsForBar(barShape);
  const candidates = Array.from({ length: 11 }, (_, value) => value - 1);
  const initialPitch = weightedPick(harmonicTargets.map((target) => ({ item: target, weight: target <= 4 ? 0.42 : 0.24 })), rng);
  const targetPitch = weightedPick(harmonicTargets.map((target) => ({ item: target, weight: barShape.boundary > 0.6 && target <= 2 ? 0.48 : 0.24 })), rng);
  const offsets = samplePitchPath(candidates, steps.length, rng, {
    initialPitch,
    targetPitch,
    barShape,
    sectionState,
  });
  return steps.map((step, index) => {
    const offset = offsets[index];
    const chordToneBias = [0, 2, 4, 7].includes(((offset % 7) + 7) % 7) ? 0 : rng() < 0.32 ? weightedPick([{ item: -1, weight: 0.3 }, { item: 1, weight: 0.3 }, { item: 0, weight: 0.4 }], rng) : 0;
    const duration = clamp(Math.round(2 + (1 - barShape.pace) * 2.2 + barShape.closure * 0.8 + rng() * 0.8), 2, 5);
    return [step, clamp(offset + chordToneBias, -1, 9), duration];
  });
}

function harmonicTargetsForBar(barShape) {
  const center = clamp(Math.round(2 + barShape.targetCenter * 3 + barShape.heightBias * 2 + barShape.tension * 2 - barShape.stability), -1, 8);
  const rootPull = barShape.closure * 0.68 + barShape.stability * 0.22;
  return [
    clamp(Math.round(interpolate(center, 0, rootPull)), -1, 8),
    clamp(Math.round(interpolate(center + 2, 2, rootPull * 0.7)), -1, 8),
    clamp(Math.round(center + (barShape.heightBias > 0 ? 3 : -1)), -1, 8),
  ];
}

function generateRhythmHook(seedText, localBar, sectionState, role) {
  const rng = eventRng(seedText, role, "rhythm-hook", sectionState, localBar);
  const barShape = phraseBar(sectionState, localBar);
  const density = 0.36 + sectionState.identityLevel * 0.16 + barShape.energy * 0.12 + barShape.boundary * 0.12 + barShape.pickup * 0.1;
  const hits = [];
  const kickSteps = stochasticOnsets(rng, 2 + Number(rng() < density), {
    min: 0,
    max: 14,
    anchorStart: true,
    strongBeatBias: 0.78,
    syncopation: 0.18 + barShape.syncopation * 0.62 + rng() * 0.18,
    minGap: 3,
  });
  const snareSteps = stochasticOnsets(rng, 1 + Number(rng() < density), {
    min: 3,
    max: 15,
    strongBeatBias: 0.34,
    syncopation: 0.44 + barShape.syncopation * 0.34,
    minGap: 4,
  });
  const hatSteps = stochasticOnsets(rng, 2 + Number(rng() < density) + Number(rng() < 0.35), {
    min: 1,
    max: 15,
    strongBeatBias: 0.18,
    syncopation: 0.5 + barShape.syncopation * 0.38,
    minGap: 2,
  });
  for (const step of kickSteps) hits.push({ sound: "kick", step, weight: step === 0 ? 1 : 0.72 + rng() * 0.22 });
  for (const step of snareSteps) hits.push({ sound: "snare", step, weight: 0.78 + rng() * 0.24 });
  for (const step of hatSteps) hits.push({ sound: "hat", step, weight: 0.72 + rng() * 0.28 });
  return hits.sort((left, right) => left.step - right.step || left.sound.localeCompare(right.sound));
}

function generateDrumGrid(seedText, localBar, sectionState, role) {
  const rng = eventRng(seedText, role, "drum-grid", sectionState, localBar);
  const barShape = phraseBar(sectionState, localBar);
  return [
    ...stochasticOnsets(rng, 1 + Number(barShape.pace > 0.34) + Number(barShape.energy > 1.18 && barShape.pace > 0.54 && rng() > 0.52), { min: 0, max: 12, anchorStart: true, strongBeatBias: 0.88, syncopation: 0.1 + barShape.syncopation * 0.22, minGap: 5 }).map((step, index) => ({ sound: "kick", step, weight: (index === 0 ? 1 : 0.72) * barShape.energy })),
    ...stochasticOnsets(rng, 2, { min: 3, max: 13, strongBeatBias: 0.74, syncopation: 0.18 + barShape.syncopation * 0.32, minGap: 5 }).map((step) => ({ sound: "snare", step, weight: 0.92 * barShape.energy })),
    ...stochasticOnsets(rng, clamp(Math.round(1.4 + barShape.pace * 3.6 + Number(barShape.pickup > 0.25)), 1, 6), { min: 1, max: 15, strongBeatBias: 0.36, syncopation: 0.36 + barShape.syncopation * 0.38, minGap: barShape.pace < 0.36 ? 4 : 2 }).map((step) => ({ sound: "hat", step, weight: 0.88 + barShape.pickup * 0.2 })),
  ].sort((left, right) => left.step - right.step || left.sound.localeCompare(right.sound));
}

function generateBassWalk(seedText, localBar, sectionState) {
  const rng = eventRng(seedText, "motion", "bass-walk", sectionState, localBar);
  const barShape = phraseBar(sectionState, localBar);
  const steps = stochasticOnsets(rng, 2 + Number(rng() > 0.56 - barShape.energy * 0.14 - barShape.pickup * 0.18), { min: 2, max: 14, strongBeatBias: 0.36, syncopation: 0.34 + barShape.syncopation * 0.46, minGap: 3 });
  let offset = weightedPick([{ item: 0, weight: 0.34 }, { item: 1, weight: 0.32 }, { item: 2, weight: 0.24 }, { item: -1, weight: 0.1 }], rng);
  return steps.map((step) => {
    offset += weightedPick([{ item: 1, weight: 0.48 }, { item: -1, weight: 0.24 }, { item: 0, weight: 0.28 }], rng);
    return [step, clamp(offset, -1, 4)];
  });
}

function addTime(events, role, timbres, chord, bar, localBar, sectionState, seedText) {
  const barShape = phraseBar(sectionState, localBar);
  const barStart = bar * 16;
  const beforeCount = events.length;
  if (role.carrier === "drum-grid") {
    for (const hit of generateDrumGrid(seedText, localBar, sectionState, "time")) {
      const base = hit.sound === "kick" ? 0.2 : hit.sound === "snare" ? 0.14 : 0.055;
      events.push(noiseEvent("drums", bar, hit.step, hit.sound, "time", base * hit.weight));
    }
    ensureBarPulseAnchor(events, role, chord, bar, barStart, beforeCount, barShape, sectionState, seedText, localBar);
    return;
  }
  if (role.carrier === "bass-pulse") {
    const rng = eventRng(seedText, "time", "bass-pulse", sectionState, localBar);
    const count = clamp(Math.round(1.2 + barShape.pace * 2.3 + Number(rng() > 0.76 - barShape.energy * 0.16 - barShape.pickup * 0.16)), 1, 4);
    for (const step of stochasticOnsets(rng, count, { min: 0, max: 13, anchorStart: true, strongBeatBias: 0.82, syncopation: 0.12 + barShape.syncopation * 0.32, minGap: barShape.pace < 0.4 ? 5 : 4 })) {
      events.push(noteEvent("bass", bar, step, step === 0 ? 3 : 2, [chord[0] - 24], "time", (step === 0 ? 0.15 : 0.11) * barShape.energy));
    }
    ensureBarPulseAnchor(events, role, chord, bar, barStart, beforeCount, barShape, sectionState, seedText, localBar);
    return;
  }
  if (role.carrier === "arp-pulse") {
    const rng = eventRng(seedText, "time", "arp-pulse", sectionState, localBar);
    const count = texturalEventCount(rng, {
      mean: 0.78 + barShape.pace * 1.72 + barShape.pickup * 0.42,
      minimum: barShape.pace < 0.36 ? 1 : 2,
      burst: 0.03 + barShape.tension * 0.07 + barShape.pickup * 0.08,
    });
    const steps = pulseOnsets(rng, count, { phaseMax: sectionState.variant === 2 ? 2 : 1, jitter: 1 + Number(barShape.syncopation > 0.58), minGap: barShape.pace < 0.5 ? 5 : 4 });
    const candidateNotes = harmonicPulseCandidates(chord);
    const notes = samplePitchPath(candidateNotes, steps.length, rng, {
      initialPitch: weightedPick(candidateNotes.map((note) => ({ item: note, weight: chord.includes(note) ? 0.42 : 0.22 })), rng),
      targetPitch: harmonicPulseTargetCenter(chord, barShape),
      barShape,
      sectionState,
    });
    for (const [index, step] of steps.entries()) {
      const note = notes[index];
      const duration = barShape.pace < 0.44 ? 2 : 1;
      events.push(noteEvent("chord", bar, step, duration, [note], "time", 0.052 * barShape.energy));
    }
    ensureBarPulseAnchor(events, role, chord, bar, barStart, beforeCount, barShape, sectionState, seedText, localBar);
    return;
  }
  if (role.carrier === "thin-pulse") {
    for (const hit of generateThinPulse(seedText, localBar, sectionState)) {
      events.push(noiseEvent("drums", bar, hit.step, hit.sound, "time", hit.velocity));
    }
    ensureBarPulseAnchor(events, role, chord, bar, barStart, beforeCount, barShape, sectionState, seedText, localBar);
  }
}

function ensureBarPulseAnchor(events, role, chord, bar, barStart, beforeCount, barShape, sectionState, seedText, localBar) {
  const timeEvents = events.slice(beforeCount).filter((event) => event.role === "time");
  if (timeEvents.some((event) => event.step === barStart)) {
    return;
  }
  const rng = eventRng(seedText, "time-anchor", role.carrier, sectionState, localBar);
  const baseVelocity = clamp(0.82 + barShape.energy * 0.2 + barShape.stability * 0.08, 0.78, 1.08);
  const carrier = role.carrier === "thin-pulse"
    ? weightedPick([
      { item: "drums", weight: 0.38 },
      { item: "bass", weight: 0.24 },
      { item: "chord", weight: 0.22 },
      { item: "lead", weight: 0.16 },
    ], rng)
    : role.carrier;
  if (carrier === "bass-pulse" || carrier === "bass") {
    events.push(noteEvent("bass", bar, 0, 2, [chord[0] - 24], "time", 0.075 * baseVelocity));
    return;
  }
  if (carrier === "arp-pulse" || carrier === "chord") {
    const note = pick([chord[0], chord[1], chord[2], chord[0] + 12], rng);
    events.push(noteEvent("chord", bar, 0, barShape.pace < 0.44 ? 2 : 1, [note], "time", 0.045 * baseVelocity));
    return;
  }
  if (carrier === "lead") {
    events.push(noteEvent("lead", bar, 0, 1, [chord[0] + 12], "time", 0.04 * baseVelocity));
    return;
  }
  events.push(noiseEvent("drums", bar, 0, "kick", "time", 0.052 * baseVelocity));
}

function harmonicPulseCandidates(chord) {
  return [...new Set([
    chord[0],
    chord[1],
    chord[2],
    chord[0] + 12,
    chord[1] + 12,
    chord[2] + 12,
    chord[0] + 2,
    chord[1] - 2,
    chord[2] + 2,
  ])];
}

function harmonicPulseTargetCenter(chord, barShape) {
  const harmonicNotes = [
    chord[0] - 12,
    chord[0],
    chord[1],
    chord[2],
    chord[0] + 12,
    chord[1] + 12,
    chord[2] + 12,
  ];
  const index = clamp(Math.round(2 + barShape.targetCenter * 1.8 + barShape.heightBias * 1.5 + barShape.tension - barShape.closure * 1.2), 0, harmonicNotes.length - 1);
  return harmonicNotes[index];
}

function generateThinPulse(seedText, localBar, sectionState) {
  const rng = eventRng(seedText, "time", "thin-pulse", sectionState, localBar);
  const barShape = phraseBar(sectionState, localBar);
  const activePressure = 0.28 + barShape.toneAnchor * 0.22 + barShape.boundary * 0.2 + barShape.pickup * 0.18 + barShape.energy * 0.08;
  if (rng() > activePressure) {
    return [];
  }
  const count = 1 + Number(rng() < 0.18 + barShape.pickup * 0.24 + barShape.boundary * 0.12);
  const steps = stochasticOnsets(rng, count, {
    min: barShape.pickup > 0.38 ? 1 : 0,
    max: barShape.boundary > 0.7 ? 15 : 13,
    strongBeatBias: 0.26 + barShape.stability * 0.18,
    syncopation: clamp(0.34 + barShape.syncopation * 0.5 + barShape.pickup * 0.18, 0, 1),
    minGap: 4,
  });
  return steps.map((step, index) => {
    const sound = weightedPick([
      { item: "kick", weight: step <= 2 ? 0.42 : 0.2 },
      { item: "hat", weight: 0.42 + barShape.space * 0.18 },
      { item: "snare", weight: step >= 7 ? 0.22 : 0.08 },
    ], rng);
    return {
      sound,
      step,
      velocity: (index === 0 ? 0.095 : 0.065) * clamp(0.74 + barShape.energy * 0.28, 0.7, 1.12),
    };
  });
}

function addTone(events, role, timbres, chord, bar, localBar, sectionState, seedText) {
  const barShape = phraseBar(sectionState, localBar);
  const rng = eventRng(seedText, "tone", role.carrier, sectionState, localBar);
  if (role.carrier === "root-bass") {
    if (barShape.toneAnchor || sectionState.variant === 2 && barShape.pickup > 0.28) {
      const step = harmonicSupportStep(rng, barShape, { earlyBias: 0.72, max: 6 });
      const duration = clamp(randomInt(rng, 5, 10) + Math.round(barShape.space * 4), 4, 13);
      events.push(noteEvent("bass", bar, step, duration, [chord[0] - 24], "tone", 0.1 * barShape.energy));
    }
    return;
  }
  if (role.carrier === "chord-pad") {
    if (barShape.toneAnchor) {
      const step = harmonicSupportStep(rng, barShape, { earlyBias: 0.64, max: 5 });
      const duration = clamp(randomInt(rng, 7, 13) + Math.round(barShape.space * 3), 6, 15);
      events.push(noteEvent("chord", bar, step, duration, chord.map((note) => note + 12), "tone", 0.048 * barShape.energy));
    }
    return;
  }
  if (role.carrier === "drone") {
    if (barShape.toneAnchor && (barShape.stability > 0.62 || barShape.boundary > 0.58 || barShape.energy < 0.9)) {
      const step = harmonicSupportStep(rng, barShape, { earlyBias: 0.82, max: 3 });
      const duration = clamp(16 - step + randomInt(rng, -1, 1), 8, 16);
      events.push(noteEvent("chord", bar, step, duration, [chord[0]], "tone", 0.064 * barShape.energy));
    }
    return;
  }
}

function harmonicSupportStep(rng, barShape, options = {}) {
  const max = Number(options.max ?? 6);
  const earlyBias = Number(options.earlyBias ?? 0.6);
  const candidates = Array.from({ length: max + 1 }, (_, index) => index);
  if (barShape.pickup > 0.34 && rng() < 0.62) {
    return weightedStep(candidates.filter((step) => step >= 1), rng, {
      strongBeatBias: 0.22,
      syncopation: clamp(barShape.syncopation + 0.2, 0, 1),
    });
  }
  return weightedStep(candidates, rng, {
    strongBeatBias: earlyBias,
    syncopation: clamp(barShape.syncopation * 0.42, 0, 1),
  });
}

function addMotion(events, role, timbres, tonic, scale, chordRoot, bar, localBar, sectionState, seedText) {
  const barShape = phraseBar(sectionState, localBar);
  if (role.carrier === "none") {
    return;
  }
  if (role.carrier === "answer-line") {
    const rng = eventRng(seedText, "motion", "answer-line", sectionState, localBar);
    const answerPressure = barShape.tension > 0.58 && barShape.closure < 0.62
      ? 0.58 + barShape.energy * 0.14 + sectionState.memoryDistance * 0.16
      : sectionState.variant === 2 && barShape.pickup > 0.24
        ? 0.48
        : 0.12 + barShape.pickup * 0.34;
    if (rng() < answerPressure) {
      const count = 2 + Number(rng() > 0.48 - barShape.energy * 0.12);
      const steps = stochasticOnsets(rng, count, { min: 1, max: 13, strongBeatBias: 0.34, syncopation: 0.32 + barShape.syncopation * 0.42, minGap: 3 });
      const frame = melodicFrameForBar(sectionState, barShape, rng);
      for (const [index, step] of steps.entries()) {
        const offset = frame.outward + (index === 0 ? 1 : index === steps.length - 1 ? -1 : randomInt(rng, -1, 2));
        events.push(noteEvent("counter", bar, step, 2, [degreeNote(tonic, scale, chordRoot + offset, 0)], "motion", 0.055 * sectionState.motionLevel));
      }
    }
    return;
  }
  if (role.carrier === "harmony-arp") {
    const pattern = generateHarmonyArp(seedText, localBar, sectionState, "motion");
    for (const [step, offset, duration] of pattern) {
      events.push(noteEvent("chord", bar, step, duration, [degreeNote(tonic, scale, chordRoot + offset, 12)], "motion", 0.052 * sectionState.motionLevel));
    }
    return;
  }
  if (role.carrier === "bass-walk") {
    const rng = eventRng(seedText, "motion", "bass-walk", sectionState, localBar);
    if (barShape.pickup > 0.24 || !barShape.toneAnchor || rng() < 0.16 + sectionState.motionLevel * 0.08 + barShape.energy * 0.08) {
      const pattern = generateBassWalk(seedText, localBar, sectionState);
      for (const [step, offset] of pattern) {
        events.push(noteEvent("bass", bar, step, 2, [degreeNote(tonic, scale, chordRoot + offset, -24)], "motion", 0.12 * sectionState.motionLevel));
      }
    }
    return;
  }
  if (barShape.boundary > 0.72 || barShape.pickup > 0.42) {
    const rng = eventRng(seedText, "motion", "percussion-fill", sectionState, localBar);
    const steps = stochasticOnsets(rng, 2, { min: 9, max: 15, strongBeatBias: 0.2, syncopation: 0.75, minGap: 2 });
    for (const [index, step] of steps.entries()) {
      events.push(noiseEvent("drums", bar, step, index === 0 ? "snare" : "hat", "motion", index === 0 ? 0.11 : 0.07));
    }
  }
}

function addColor(events, role, timbres, chord, bar, localBar, sectionState, seedText) {
  const barShape = phraseBar(sectionState, localBar);
  const rng = eventRng(seedText, "color", role.carrier, sectionState, localBar);
  if (role.carrier === "none") {
    return;
  }
  if (role.carrier === "air-pad" && barShape.colorAccent) {
    const step = colorAccentStep(rng, barShape, { max: 7 });
    events.push(noteEvent("chord", bar, step, randomInt(rng, 6, 12), [chord[1] + 12, chord[2] + 12], "color", 0.04 * sectionState.colorLevel));
  }
  if (role.carrier === "noise-halo" && (barShape.colorAccent || barShape.space > 0.58)) {
    const step = colorAccentStep(rng, barShape, { min: 4, max: 14 });
    events.push(noteEvent("lead", bar, step, randomInt(rng, 3, 7), [chord[1] + 12], "color", 0.036 * sectionState.colorLevel * (0.82 + barShape.space)));
  }
  if (role.carrier === "organ-bed" && barShape.toneAnchor) {
    const step = colorAccentStep(rng, barShape, { min: 1, max: 8 });
    events.push(noteEvent("chord", bar, step, randomInt(rng, 6, 10), chord.map((note) => note + 12), "color", 0.042 * sectionState.colorLevel * barShape.energy));
  }
  if (role.carrier === "bright-accent" && (barShape.pickup > 0.3 || barShape.boundary > 0.7)) {
    const step = colorAccentStep(rng, barShape, { min: 7, max: 15 });
    events.push(noteEvent("lead", bar, step, randomInt(rng, 1, 3), [chord[2] + 12], "color", 0.055 * sectionState.colorLevel * barShape.energy));
  }
}

function colorAccentStep(rng, barShape, options = {}) {
  const min = Number(options.min ?? 0);
  const max = Number(options.max ?? 15);
  const count = 1 + Number(barShape.energy > 1.08 && barShape.pickup > 0.28 && rng() < 0.34);
  return stochasticOnsets(rng, count, {
    min,
    max,
    strongBeatBias: barShape.boundary > 0.6 ? 0.22 : 0.34,
    syncopation: clamp(0.24 + barShape.syncopation * 0.58 + barShape.pickup * 0.2, 0, 1),
    minGap: 2,
  })[0];
}

function addBoundary(events, role, timbres, tonic, scale, bar, localBar, sectionState, seedText) {
  const barShape = phraseBar(sectionState, localBar);
  if (barShape.boundary < 0.72) {
    return;
  }
  if (sectionState.loopHandoff && localBar === 7) {
    addLoopHandoff(events, tonic, scale, bar, sectionState, seedText);
    return;
  }
  const rng = eventRng(seedText, "boundary", role.carrier, sectionState, localBar);
  if (role.carrier === "drum-fill") {
    const steps = stochasticOnsets(rng, 2 + Number(rng() > 0.58), { min: 10, max: 15, strongBeatBias: 0.18, syncopation: 0.84, minGap: 1 });
    for (const [index, step] of steps.entries()) {
      events.push(noiseEvent("drums", bar, step, index % 2 === 0 ? "snare" : "hat", "boundary", (index === 0 ? 0.12 : 0.08) * sectionState.boundaryLevel));
    }
    return;
  }
  if (role.carrier === "contrast-note") {
    const step = randomInt(rng, 11, 15);
    const frame = melodicFrameForBar(sectionState, barShape, rng);
    const approach = weightedPick([
      { item: melodicTargetForBar(frame, barShape), weight: 0.42 },
      { item: frame.open, weight: 0.28 },
      { item: frame.settled + randomInt(rng, -1, 1), weight: 0.2 },
      { item: frame.upper, weight: 0.1 },
    ], rng);
    events.push(noteEvent("lead", bar, step, 2, [degreeNote(tonic, scale, approach, 12)], "boundary", 0.075 * sectionState.boundaryLevel));
    return;
  }
  if (role.carrier === "register-turn") {
    const step = randomInt(rng, 10, 14);
    const frame = melodicFrameForBar(sectionState, barShape, rng);
    const turn = frame.outward + weightedPick([{ item: -2, weight: 0.24 }, { item: 2, weight: 0.38 }, { item: 4, weight: 0.38 }], rng);
    events.push(noteEvent("counter", bar, step, randomInt(rng, 2, 4), [degreeNote(tonic, scale, turn, 12)], "boundary", 0.06 * sectionState.boundaryLevel));
  }
}

function addLoopHandoff(events, tonic, scale, bar, sectionState, seedText) {
  const rng = eventRng(seedText, "boundary", "loop-handoff", sectionState, 7);
  const approach = weightedPick([
    { item: -1, weight: 0.26 },
    { item: 1, weight: 0.42 },
    { item: 2, weight: 0.2 },
    { item: 4, weight: 0.12 },
  ], rng);
  const leadStep = weightedPick([
    { item: 12, weight: 0.22 },
    { item: 13, weight: 0.24 },
    { item: 14, weight: 0.42 },
    { item: 15, weight: 0.12 },
  ], rng);
  const bassStep = leadStep >= 14 ? 15 : 14;
  events.push(noteEvent("lead", bar, leadStep, 1, [degreeNote(tonic, scale, approach, 12)], "boundary", 0.055 * sectionState.boundaryLevel));
  events.push(noteEvent("bass", bar, bassStep, 1, [degreeNote(tonic, scale, 0, -12)], "boundary", 0.06 * sectionState.boundaryLevel));
}

function addSectionBridge(events, tonic, scale, bar, localBar, sectionState, seedText) {
  const barShape = phraseBar(sectionState, localBar);
  const bridge = barShape.transitionBridge;
  if (!bridge || localBar < 8 - bridge.bars) {
    return;
  }
  const rng = eventRng(seedText, "boundary", `section-bridge:${bridge.role}:${bridge.carrier}`, sectionState, localBar);
  const progress = bridge.bars <= 1 ? 1 : (localBar - (8 - bridge.bars)) / (bridge.bars - 1);
  const velocity = (0.028 + bridge.impact * 0.044) * (0.72 + progress * 0.42);
  const step = localBar === 7
    ? weightedPick([
      { item: 12, weight: 0.24 },
      { item: 13, weight: 0.28 },
      { item: 14, weight: 0.34 },
      { item: 15, weight: 0.14 },
    ], rng)
    : randomInt(rng, 9, 13);
  if (bridge.track === "drums") {
    const sound = weightedPick([
      { item: "hat", weight: 0.48 },
      { item: "snare", weight: 0.34 },
      { item: "kick", weight: 0.18 },
    ], rng);
    events.push(noiseEvent("drums", bar, step, sound, "boundary", velocity));
    return;
  }
  const degree = bridge.targetDegreeOffset + weightedPick([
    { item: 0, weight: 0.48 },
    { item: 1, weight: 0.24 },
    { item: -1, weight: 0.16 },
    { item: 2, weight: 0.12 },
  ], rng);
  if (bridge.track === "bass") {
    events.push(noteEvent("bass", bar, step, 2, [degreeNote(tonic, scale, degree, -24)], "boundary", velocity * 1.2));
    return;
  }
  if (bridge.track === "chord") {
    const note = degreeNote(tonic, scale, degree + pick([0, 2, 4], rng), 12);
    events.push(noteEvent("chord", bar, step, 2, [note], "boundary", velocity * 0.9));
    return;
  }
  const octave = bridge.track === "counter" ? 0 : 12;
  events.push(noteEvent(bridge.track, bar, step, 2, [degreeNote(tonic, scale, degree, octave)], "boundary", velocity));
}

function addSectionEntryBridge(events, tonic, scale, chord, bar, localBar, sectionState, seedText) {
  const barShape = phraseBar(sectionState, localBar);
  const bridge = barShape.transitionEntryBridge;
  if (!bridge || localBar >= bridge.bars) {
    return;
  }
  const barStart = bar * 16;
  const hasEntryOnTrack = events.some((event) => (
    event.track === bridge.track
      && event.step >= barStart
      && event.step < barStart + 4
  ));
  if (hasEntryOnTrack) {
    return;
  }
  const rng = eventRng(seedText, "boundary", `section-entry:${bridge.role}:${bridge.carrier}`, sectionState, localBar);
  const progress = bridge.bars <= 1 ? 1 : 1 - localBar / (bridge.bars - 1);
  const velocity = (0.022 + bridge.impact * 0.034) * (0.68 + progress * 0.28);
  const step = weightedPick([
    { item: 0, weight: 0.42 },
    { item: 1, weight: 0.32 },
    { item: 2, weight: 0.18 },
    { item: 3, weight: 0.08 },
  ], rng);
  if (bridge.track === "drums") {
    events.push(noiseEvent("drums", bar, step, weightedPick([
      { item: "kick", weight: 0.4 },
      { item: "hat", weight: 0.38 },
      { item: "snare", weight: 0.22 },
    ], rng), "boundary", velocity));
    return;
  }
  if (bridge.track === "bass") {
    events.push(noteEvent("bass", bar, step, 2, [chord[0] - 24], "boundary", velocity * 1.18));
    return;
  }
  if (bridge.track === "chord") {
    events.push(noteEvent("chord", bar, step, 2, [pick(chord, rng)], "boundary", velocity * 0.9));
    return;
  }
  const octave = bridge.track === "counter" ? 0 : 12;
  events.push(noteEvent(bridge.track, bar, step, 2, [degreeNote(tonic, scale, bridge.targetDegreeOffset, octave)], "boundary", velocity));
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
  const roots = Array.from({ length: scaleLength }, (_, index) => index);
  const progression = [0];
  for (let phase = 1; phase < 4; phase += 1) {
    const previous = progression[progression.length - 1];
    const root = weightedPick(roots.map((candidate) => ({
      item: candidate,
      weight: harmonicRootWeight(candidate, phase, scaleLength, previous),
    })), rng);
    progression.push(root);
  }
  return progression;
}

function harmonicRootWeight(root, phase, scaleLength, previous) {
  const homeDistance = modularDistance(root, 0, scaleLength);
  const motionDistance = modularDistance(root, previous, scaleLength);
  const phaseTarget = {
    1: Math.min(2, scaleLength - 1),
    2: Math.min(3, scaleLength - 1),
    3: 1,
  }[phase] ?? 2;
  const departureScore = gaussianScore(homeDistance, phaseTarget, phase === 3 ? 0.9 : 1.25);
  const motionScore = gaussianScore(motionDistance, phase === 3 ? 1.5 : 2, 1.15);
  const homeScore = root === 0 ? (phase === 3 ? 0.72 : 0.04) : 0;
  const returnPrepScore = phase === 3
    ? (1 / (1 + homeDistance)) + (root === scaleLength - 2 ? 0.28 : 0)
    : 0;
  const tensionScore = phase === 2 ? homeDistance / Math.max(1, Math.floor(scaleLength / 2)) : 0;
  const repeatPenalty = root === previous ? (phase === 3 ? 0.48 : 0.16) : 1;
  return Math.max(0.001, (departureScore + motionScore * 0.58 + homeScore + returnPrepScore + tensionScore * 0.36) * repeatPenalty);
}

function modularDistance(left, right, size) {
  const direct = Math.abs(left - right) % size;
  return Math.min(direct, size - direct);
}

function gaussianScore(value, target, spread) {
  const distance = value - target;
  return Math.exp(-(distance * distance) / (2 * spread * spread));
}

function buildChord(tonic, scale, degree) {
  return [degreeNote(tonic, scale, degree, 0), degreeNote(tonic, scale, degree + 2, 0), degreeNote(tonic, scale, degree + 4, 0)];
}

function degreeNote(tonic, scale, degree, octave) {
  const scaleDegree = ((degree % scale.length) + scale.length) % scale.length;
  const scaleOctave = Math.floor(degree / scale.length) * 12;
  return tonic + scale[scaleDegree] + scaleOctave + octave;
}

function trackMappingFor(roles) {
  return Object.fromEntries(Object.entries(roles).map(([name, role]) => [name, {
    carrier: role.carrier,
    playbackTracks: tracksForCarrier(role.carrier),
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
