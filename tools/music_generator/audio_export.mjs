export function exportSoundEffect(effect, typeTarget = "random") {
  const type = effect.type;
  return {
    kind: "sfx",
    seed: effect.seed,
    type,
    typeTarget,
    ps: `sfx ${type} ${effect.seed}`,
  };
}

export function exportMusicLoop(song) {
  const input = song.input;
  const transport = song.playbackScore.transport;
  const hasPlaybackKnobs = input.height !== undefined || input.focus !== undefined || input.brightness !== undefined || input.presence !== undefined || input.attack !== undefined || input.punch !== undefined;
  const height = input.height ?? input.focus ?? 0.5;
  const brightness = input.brightness ?? input.tone;
  const presence = input.presence ?? 0.5;
  const attack = input.attack ?? input.punch ?? 0.5;
  return {
    kind: "music",
    seed: input.seed,
    tone: input.tone ?? brightness,
    ...(hasPlaybackKnobs ? { height, brightness, presence, attack } : {}),
    bpm: input.bpm,
    volume: input.volume,
    key: song.debug.key,
    bars: transport.bars,
    ps: hasPlaybackKnobs
      ? `music ${input.seed} height=${height} bpm=${input.bpm} bars=${transport.bars} volume=${input.volume}`
      : `music ${input.seed} tone=${input.tone} bpm=${input.bpm} bars=${transport.bars} volume=${input.volume}`,
  };
}
