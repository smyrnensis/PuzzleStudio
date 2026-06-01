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

function instrumentOutputGain(instrument) {
  if (FUNCTIONAL_TIMBRE_META[instrument]) {
    return FUNCTIONAL_TIMBRE_META[instrument].gain;
  }
  if (instrument === "chip-lead") {
    return 0.65;
  }
  return 1;
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

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
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
