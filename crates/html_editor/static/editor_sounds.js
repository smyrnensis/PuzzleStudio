const soundPlayIcon = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m8 5 11 7-11 7V5z"></path></svg>';
const soundPauseIcon = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5v14"></path><path d="M16 5v14"></path></svg>';
const soundMusicBarOptions = [8, 16, 32, 64];

function soundsApi() {
  return window.PuzzleSoundGenerator || window.PuzzleSoundTools || null;
}

function resetSoundsBuilder() {
  const api = soundsApi();
  if (!api) {
    setSoundsUnavailable("Sounds generator unavailable.");
    return;
  }
  if (!sounds.initialized) {
    for (const type of api.SFX_TYPE_OPTIONS || []) {
      const option = document.createElement("option");
      option.value = type;
      option.textContent = soundLabelForType(type);
      soundsSfxTypeSelect.append(option);
    }
    const sfxPreset = api.randomSfxPreset(`${Date.now()}:${Math.random()}`);
    const musicPreset = api.randomPreset(`${Date.now()}:${Math.random()}`);
    soundsSfxSeedInput.value = sfxPreset.seed;
    soundsSfxTypeSelect.value = sfxPreset.type;
    soundsSfxVolumeInput.value = 1;
    soundsMusicSeedInput.value = musicPreset.seed;
    soundsMusicHeightInput.value = musicPreset.height ?? 0.5;
    setSoundMusicBars(musicPreset.bars ?? 8);
    soundsMusicBpmInput.value = musicPreset.bpm;
    sounds.initialized = true;
  }
  setSoundProgress(0);
  renderSoundsBuilder();
}

function renderSoundsBuilder() {
  const api = soundsApi();
  if (!api) {
    setSoundsUnavailable("Sounds generator unavailable.");
    return;
  }
  soundsHeaderTools.hidden = currentPreviewMode !== "sounds";
  soundsTopbarButton.classList.toggle("is-active", currentPreviewMode === "sounds");
  if (currentPreviewMode === "sounds") {
    gamePaneTitle.textContent = "Sound";
  }
  renderSoundSfx();
  renderSoundMusic();
}

function soundSfxType() {
  return soundsSfxTypeSelect.value || "random";
}

function soundSfxEffect() {
  const api = soundsApi();
  if (soundSfxType() === "puzzlescript" && api?.generatePuzzleScriptSoundEffect) {
    return api.generatePuzzleScriptSoundEffect(soundsSfxSeedInput.value);
  }
  return api.generateSoundEffect(soundsSfxSeedInput.value, { type: soundSfxType() });
}

function soundSfxVolume() {
  return soundClamp(Number(soundsSfxVolumeInput.value), 0, 1);
}

function soundMusicSong() {
  return soundsApi().generateSong(soundsMusicSeedInput.value, {
    height: Number(soundsMusicHeightInput.value),
    bars: soundMusicBars(),
    bpm: Number(soundsMusicBpmInput.value),
    volume: Number(soundsMusicVolumeInput.value),
  });
}

function renderSoundSfx() {
  soundsSfxVolumeValue.textContent = `${Math.round(soundSfxVolume() * 100)}%`;
  updateSoundRangeFill(soundsSfxVolumeInput);
  soundsSfxOutput.textContent = soundCurrentLine("sfx");
}

function renderSoundMusic() {
  const song = soundMusicSong();
  soundsMusicHeightValue.textContent = song.input.height.toFixed(2);
  soundsMusicBarsValue.textContent = `${song.input.bars}`;
  soundsMusicBpmValue.textContent = `${song.playbackScore.transport.bpm}`;
  soundsMusicVolumeValue.textContent = `${Math.round(song.playbackScore.mix.volume * 100)}%`;
  updateSoundRangeFills();
  soundsMusicOutput.textContent = soundCurrentLine("music");
}

function soundCurrentLine(kind = "sfx") {
  const definition = soundCurrentDefinition(kind);
  return definition ? definition.line : "";
}

function soundCurrentDefinition(kind = "sfx", options = {}) {
  if (kind === "music") {
    const song = soundMusicSong();
    const requestedName = soundIdentifierAtom(soundsMusicTitleInput.value, "");
    const name = options.uniqueForInsert
      ? nextSoundsDefinitionName("music", requestedName || "music", options.source)
      : requestedName || "music";
    return {
      kind: "music",
      name,
      line: `music ${name} seed=${soundAtom(song.input.seed, "123456")} bars=${song.input.bars} height=${song.input.height.toFixed(2)} bpm=${song.playbackScore.transport.bpm} volume=${song.playbackScore.mix.volume.toFixed(2)}`,
    };
  }
  const effect = soundSfxEffect();
  const type = soundAtom(effect.type, "random");
  const requestedName = soundIdentifierAtom(soundsSfxTitleInput.value, "");
  const name = options.uniqueForInsert
    ? nextSoundsDefinitionName("sfx", requestedName || "sfx", options.source)
    : requestedName || "sfx";
  return {
    kind: "sfx",
    name,
    line: `sfx ${name} seed=${soundAtom(soundsSfxSeedInput.value, "123456")} type=${type} volume=${soundSfxVolume().toFixed(2)}`,
  };
}

function setSoundsUnavailable(message) {
  soundsSfxOutput.textContent = message;
  soundsMusicOutput.textContent = message;
}

async function ensureAudioContext() {
  const AudioContextClass = window.AudioContext || window.webkitAudioContext;
  if (!AudioContextClass) {
    throw new Error("WebAudio is not available");
  }
  sounds.context ??= new AudioContextClass();
  await sounds.context.resume();
}

async function playSoundSfx() {
  const api = soundsApi();
  if (!api || shouldSuppressSoundPlayback()) {
    return;
  }
  await ensureAudioContext();
  sounds.sfxPlayer?.stop();
  if (soundSfxType() === "puzzlescript" && api?.createPuzzleScriptSfxPlayer) {
    sounds.sfxPlayer = api.createPuzzleScriptSfxPlayer(sounds.context, soundSfxEffect(), { volume: soundSfxVolume() });
  } else {
    sounds.sfxPlayer = api.createSfxPlayer(sounds.context, soundSfxEffect(), { volume: soundSfxVolume() });
  }
  sounds.sfxPlayer.start(sounds.context.currentTime);
  renderSoundSfx();
}

async function toggleSoundMusic() {
  const api = soundsApi();
  if (!api || shouldSuppressSoundPlayback()) {
    return;
  }
  if (sounds.musicPlaying) {
    pauseSoundMusic();
    return;
  }
  await ensureAudioContext();
  sounds.musicPlayer?.stop();
  sounds.musicPlayer = api.createPlayer(sounds.context, soundMusicSong().playbackScore);
  sounds.musicPlayer.start(sounds.musicProgress);
  setSoundMusicPlaying(true);
  startSoundProgress();
  renderSoundMusic();
}

function updateSoundMusic(options = {}) {
  renderSoundMusic();
  if (!sounds.musicPlaying || !sounds.context) {
    return;
  }
  window.clearTimeout(sounds.musicRestartTimer);
  sounds.musicRestartTimer = window.setTimeout(() => {
    const progress = Number.isFinite(options.restartProgress)
      ? soundClamp(options.restartProgress, 0, 0.9999)
      : sounds.musicPlayer?.loopProgress() ?? sounds.musicProgress;
    sounds.musicPlayer?.stop();
    sounds.musicProgress = progress;
    soundsMusicProgress.value = sounds.musicProgress.toFixed(4);
    updateSoundRangeFill(soundsMusicProgress);
    sounds.musicPlayer = soundsApi().createPlayer(sounds.context, soundMusicSong().playbackScore);
    sounds.musicPlayer.start(sounds.musicProgress);
    startSoundProgress();
  }, 180);
}

function pauseSoundMusic() {
  window.clearTimeout(sounds.musicRestartTimer);
  sounds.musicRestartTimer = 0;
  sounds.musicProgress = sounds.musicPlayer?.loopProgress() ?? sounds.musicProgress;
  sounds.musicPlayer?.stop();
  sounds.musicPlayer = null;
  setSoundMusicPlaying(false);
  cancelAnimationFrame(sounds.progressFrame);
  sounds.progressFrame = 0;
  setSoundProgress(sounds.musicProgress);
}

function stopSoundPlayback() {
  sounds.sfxPlayer?.stop();
  sounds.sfxPlayer = null;
  pauseSoundMusic();
}

function pauseSoundPlaybackForHiddenDocument() {
  sounds.sfxPlayer?.stop();
  sounds.sfxPlayer = null;
  sounds.visibilityPausedMusic = sounds.musicPlaying === true;
  if (sounds.musicPlaying) {
    pauseSoundMusic();
  }
}

function resumeSoundPlaybackForVisibleDocument() {
  if (!sounds.visibilityPausedMusic || shouldSuppressSoundPlayback()) {
    return;
  }
  sounds.visibilityPausedMusic = false;
  toggleSoundMusic().catch((error) => setStatus(`Could not resume music: ${error?.message || error}`, "is-error"));
}

function shouldSuppressSoundPlayback() {
  return typeof document !== "undefined" && document.visibilityState === "hidden";
}

function setSoundMusicPlaying(nextPlaying) {
  sounds.musicPlaying = nextPlaying;
  soundsMusicPlayButton.innerHTML = nextPlaying ? soundPauseIcon : soundPlayIcon;
  soundsMusicPlayButton.setAttribute("aria-label", nextPlaying ? "Pause music" : "Play music");
  soundsMusicPlayButton.title = nextPlaying ? "Pause music" : "Play music";
}

function setSoundProgress(value) {
  sounds.musicProgress = soundClamp(value, 0, 0.9999);
  soundsMusicProgress.value = sounds.musicProgress.toFixed(4);
  updateSoundRangeFill(soundsMusicProgress);
}

function startSoundProgress() {
  cancelAnimationFrame(sounds.progressFrame);
  const tick = () => {
    if (sounds.musicPlayer && sounds.musicPlaying) {
      const value = sounds.musicPlayer.loopProgress();
      sounds.musicProgress = soundClamp(value, 0, 0.9999);
      soundsMusicProgress.value = sounds.musicProgress.toFixed(4);
      updateSoundRangeFill(soundsMusicProgress);
    }
    sounds.progressFrame = requestAnimationFrame(tick);
  };
  tick();
}

function seekSoundMusic(value) {
  setSoundProgress(value);
  if (!sounds.musicPlaying || !sounds.context) {
    return;
  }
  sounds.musicPlayer?.stop();
  sounds.musicPlayer = soundsApi().createPlayer(sounds.context, soundMusicSong().playbackScore);
  sounds.musicPlayer.start(sounds.musicProgress);
  startSoundProgress();
}

async function copySoundLine(kind = "sfx") {
  const text = soundCurrentLine(kind);
  if (!text) {
    return;
  }
  await copyTextToClipboard(text);
  setStatus("Copied sounds definition", "is-ok");
}

function insertSoundsDefinition(kind = "sfx") {
  const document = activeSoundEditDocument();
  if (!document || !isTextDocument(document)) {
    return;
  }
  sounds.mode = kind === "music" ? "music" : "sfx";
  const source = activeSoundEditSource();
  const definition = soundCurrentDefinition(kind, { uniqueForInsert: true, source });
  if (!definition) {
    return;
  }
  const insertion = insertSoundsDefinitionIntoSource(source, definition.line);
  document.source = insertion.source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorText(insertion.source, insertion.selectionStart, insertion.selectionEnd);
  }
  scheduleLocalSave();
  schedulePreview();
  sourceEditor.focus();
  setActiveSoundEditTarget({
    kind: definition.kind,
    name: definition.name,
    start: insertion.selectionStart,
    end: insertion.selectionStart,
  }, document);
  renderSoundsBuilder();
  setStatus(`Added ${definition.kind} ${definition.name}`, "is-ok");
}

function updateSoundsDefinition(kind = "sfx") {
  const definition = soundCurrentDefinition(kind);
  const document = activeSoundEditDocument();
  if (!definition || !document || !isTextDocument(document)) {
    return;
  }
  sounds.mode = kind === "music" ? "music" : "sfx";
  const editTarget = activeSoundEditTargetForDocument(document, definition.kind);
  const originalName = editTarget?.name || definition.name;
  const source = activeSoundEditSource();
  if (
    originalName !== definition.name
    && soundsDefinitionNameExists(source, definition.kind, definition.name, {
      exceptStart: editTarget?.start,
    })
  ) {
    setStatus(`${definition.kind} ${definition.name} already exists`, "is-error");
    return;
  }
  const replacement = replaceSoundsDefinitionInSource(source, definition, {
    originalName,
    originalStart: editTarget?.start,
  });
  if (!replacement) {
    setStatus(`No ${definition.kind} named ${originalName}`, "is-error");
    return;
  }
  const renamed = originalName !== definition.name;
  const referenceReplacement = renamed
    ? replaceSoundReferencesInSource(replacement.source, definition.kind, originalName, definition.name, {
      definitionStart: replacement.definitionStart,
      definitionEnd: replacement.definitionEnd,
      selectionStart: replacement.selectionStart,
    })
    : { source: replacement.source, count: 0 };
  document.source = referenceReplacement.source;
  const selectionStart = replacement.selectionStart + (referenceReplacement.selectionShift || 0);
  if (document.id === activeDocument()?.id) {
    setSourceEditorText(referenceReplacement.source, selectionStart, selectionStart);
  }
  scheduleLocalSave();
  schedulePreview();
  sourceEditor.focus();
  setActiveSoundEditTarget({
    kind: definition.kind,
    name: definition.name,
    start: replacement.definitionStart,
    end: replacement.definitionEnd,
  }, document);
  renderSoundsBuilder();
  const referenceMessage = referenceReplacement.count > 0
    ? ` and ${referenceReplacement.count} reference${referenceReplacement.count === 1 ? "" : "s"}`
    : "";
  setStatus(`Updated ${definition.kind} ${definition.name}${referenceMessage}`, "is-ok");
}

function activeSoundEditDocument() {
  const document = activeDocument();
  if (document && isTextDocument(document) && isPuzzleDocument(document)) {
    return document;
  }
  return activePreviewDocument();
}

function activeSoundEditSource() {
  const document = activeSoundEditDocument();
  if (!document || !isTextDocument(document)) {
    return "";
  }
  return document.id === activeDocument()?.id
    ? sourceEditorDocumentValue()
    : document.source || "";
}

function loadSoundFromSourcePosition(position, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditorDocumentValue();
  const entry = findSoundsDefinitionAtPosition(source, position);
  if (!entry) {
    return null;
  }
  if (options.recordHistory && typeof pushSourceNavigationHistory === "function") {
    pushSourceNavigationHistory();
  }
  if (entry.kind === "music") {
    sounds.mode = "music";
    soundsMusicTitleInput.value = entry.name;
    soundsMusicSeedInput.value = entry.params.seed || soundsMusicSeedInput.value;
    if (entry.params.height !== undefined || entry.params.tone !== undefined) {
      soundsMusicHeightInput.value = entry.params.height ?? entry.params.tone;
    }
    if (entry.params.bars !== undefined) {
      setSoundMusicBars(entry.params.bars);
    }
    if (entry.params.bpm !== undefined) {
      soundsMusicBpmInput.value = entry.params.bpm;
    }
    if (entry.params.volume !== undefined) {
      soundsMusicVolumeInput.value = entry.params.volume;
    }
    setSoundProgress(0);
  } else {
    sounds.mode = "sfx";
    soundsSfxTitleInput.value = entry.name;
    soundsSfxSeedInput.value = entry.params.seed || soundsSfxSeedInput.value;
    if (entry.params.type !== undefined) {
      soundsSfxTypeSelect.value = entry.params.type;
    }
    soundsSfxVolumeInput.value = entry.params.volume ?? 1;
  }
  if (options.switchMode && currentPreviewMode !== "sounds") {
    setPreviewMode("sounds");
  } else {
    renderSoundsBuilder();
  }
  if (!options.silent) {
    setStatus(`Loaded ${entry.kind} ${entry.name}`, "is-ok");
  }
  setActiveSoundEditTarget(entry, activeDocument());
  return `sounds:${entry.kind}:${entry.name}:${entry.start}`;
}

function loadSoundSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const entry = {
    kind: target.soundKind,
    name: target.name,
    params: target.params || {},
    start: target.start,
  };
  if (!entry.kind || !entry.name) {
    return null;
  }
  if (options.recordHistory && typeof pushSourceNavigationHistory === "function") {
    pushSourceNavigationHistory();
  }
  if (entry.kind === "music") {
    sounds.mode = "music";
    soundsMusicTitleInput.value = entry.name;
    soundsMusicSeedInput.value = entry.params.seed || soundsMusicSeedInput.value;
    if (entry.params.height !== undefined || entry.params.tone !== undefined) {
      soundsMusicHeightInput.value = entry.params.height ?? entry.params.tone;
    }
    if (entry.params.bars !== undefined) {
      setSoundMusicBars(entry.params.bars);
    }
    if (entry.params.bpm !== undefined) {
      soundsMusicBpmInput.value = entry.params.bpm;
    }
    if (entry.params.volume !== undefined) {
      soundsMusicVolumeInput.value = entry.params.volume;
    }
    setSoundProgress(0);
  } else {
    sounds.mode = "sfx";
    soundsSfxTitleInput.value = entry.name;
    soundsSfxSeedInput.value = entry.params.seed || soundsSfxSeedInput.value;
    if (entry.params.type !== undefined) {
      soundsSfxTypeSelect.value = entry.params.type;
    }
    soundsSfxVolumeInput.value = entry.params.volume ?? 1;
  }
  if (options.switchMode && currentPreviewMode !== "sounds") {
    setPreviewMode("sounds");
  } else {
    renderSoundsBuilder();
  }
  if (!options.silent) {
    setStatus(`Loaded ${entry.kind} ${entry.name}`, "is-ok");
  }
  setActiveSoundEditTarget(entry, activeDocument());
  return `sounds:${entry.kind}:${entry.name}:${entry.start}`;
}

function setActiveSoundEditTarget(entry, document = activeSoundEditDocument()) {
  if (!entry?.kind || !entry?.name || !document?.id) {
    sounds.editTarget = null;
    return;
  }
  sounds.editTarget = {
    documentId: document.id,
    kind: entry.kind,
    name: entry.name,
    start: Number.isInteger(entry.start) ? entry.start : null,
    end: Number.isInteger(entry.end) ? entry.end : null,
  };
}

function activeSoundEditTargetForDocument(document, kind) {
  const target = sounds.editTarget;
  if (!target || target.documentId !== document?.id || target.kind !== kind) {
    return null;
  }
  return target;
}

function soundAtom(value, fallback) {
  const atom = String(value || "").trim().replace(/[^\w.-]+/g, "_").replace(/^_+|_+$/g, "");
  return atom || fallback;
}

function soundIdentifierAtom(value, fallback) {
  let atom = String(value || "").trim().replace(/[^A-Za-z0-9_]+/g, "_").replace(/^_+|_+$/g, "");
  if (!atom || /^[0-9]/.test(atom)) {
    atom = fallback;
  }
  return atom;
}

function nextSoundsDefinitionName(kind, baseName, sourceOverride = null) {
  const source = sourceOverride !== null
    ? String(sourceOverride || "")
    : isTextDocument(documents[currentDocumentIndex])
      ? sourceEditorDocumentValue()
      : "";
  const names = existingSoundsDefinitionNames(source, kind);
  const base = soundIdentifierAtom(baseName, kind === "music" ? "music" : "sfx");
  if (!names.has(base)) {
    return base;
  }
  const sequence = soundDefinitionNameSequence(base);
  for (let index = sequence.nextIndex; index < 1000; index += 1) {
    const candidate = `${sequence.root}_${index}`;
    if (!names.has(candidate)) {
      return candidate;
    }
  }
  return `${sequence.root}_${Date.now()}`;
}

function soundDefinitionNameSequence(name) {
  const match = String(name || "").match(/^(.+)_([1-9][0-9]*)$/);
  if (!match) {
    return { root: name, nextIndex: 2 };
  }
  return {
    root: match[1],
    nextIndex: Number(match[2]) + 1,
  };
}

function existingSoundsDefinitionNames(source, kind) {
  const names = new Set();
  const pattern = new RegExp(`^\\s*${kind}\\s+([A-Za-z_][A-Za-z0-9_]*)\\b`);
  for (const line of String(source || "").split("\n")) {
    const match = stripLineComment(line).match(pattern);
    if (match) {
      names.add(match[1]);
    }
  }
  return names;
}

function soundsDefinitionNameExists(source, kind, name, options = {}) {
  const text = String(source || "");
  const lines = soundSourceLinesWithOffsets(text);
  const exceptStart = Number.isInteger(options.exceptStart) ? options.exceptStart : null;
  for (const soundsBlock of findSoundsBlocks(lines)) {
    for (let index = soundsBlock.startLine + 1; index < soundsBlock.endLine; index += 1) {
      const line = lines[index];
      const parsed = parseSoundsDefinitionLine(line?.text || "");
      if (!parsed || parsed.kind !== kind || parsed.name !== name) {
        continue;
      }
      if (exceptStart !== null && exceptStart >= line.start && exceptStart <= line.end) {
        continue;
      }
      return true;
    }
  }
  return false;
}

function findSoundsDefinitionAtPosition(source, position) {
  const text = String(source || "");
  const lines = soundSourceLinesWithOffsets(text);
  const soundsBlock = findSoundsBlockAtPosition(lines, position);
  if (!soundsBlock) {
    return null;
  }
  for (let index = soundsBlock.startLine + 1; index < soundsBlock.endLine; index += 1) {
    const line = lines[index];
    if (!line || position < line.start || position > line.end) {
      continue;
    }
    const parsed = parseSoundsDefinitionLine(line.text);
    if (!parsed) {
      return null;
    }
    return {
      ...parsed,
      start: line.start,
      end: line.end,
    };
  }
  return null;
}

function parseSoundsDefinitionLine(line) {
  const code = stripLineComment(line).trim();
  const match = code.match(/^(sfx|music)\s+([A-Za-z_][A-Za-z0-9_]*)\b(.*)$/);
  if (!match) {
    return null;
  }
  const params = {};
  const tail = match[3] || "";
  for (const param of tail.matchAll(/\b([A-Za-z_][A-Za-z0-9_]*)=("[^"]*"|'[^']*'|[^\s]+)/g)) {
    const raw = param[2] || "";
    params[param[1]] = raw.replace(/^["']|["']$/g, "");
  }
  return {
    kind: match[1],
    name: match[2],
    params,
  };
}

function insertSoundsDefinitionIntoSource(source, line) {
  const text = String(source || "");
  const lines = soundSourceLinesWithOffsets(text);
  const soundsBlock = findFirstSoundsBlock(lines);
  if (soundsBlock) {
    const insertText = `${line}\n`;
    const nextSource = `${text.slice(0, soundsBlock.insertIndex)}${insertText}${text.slice(soundsBlock.insertIndex)}`;
    const selectionStart = soundsBlock.insertIndex + insertText.length;
    return { source: nextSource, selectionStart, selectionEnd: selectionStart };
  }

  const block = `sounds {\n${line}\n}\n`;
  const afterName = findTopLevelNameInsertionIndex(lines);
  if (afterName > 0) {
    const prefix = text[afterName - 1] === "\n" ? "\n" : "\n\n";
    const insertText = `${prefix}${block}`;
    const nextSource = `${text.slice(0, afterName)}${insertText}${text.slice(afterName)}`;
    const selectionStart = afterName + insertText.length;
    return { source: nextSource, selectionStart, selectionEnd: selectionStart };
  }

  const suffix = text && !text.endsWith("\n") ? "\n\n" : text ? "\n" : "";
  const insertText = `${suffix}${block}`;
  const nextSource = `${text}${insertText}`;
  const selectionStart = nextSource.length;
  return { source: nextSource, selectionStart, selectionEnd: selectionStart };
}

function replaceSoundsDefinitionInSource(source, definition, options = {}) {
  const text = String(source || "");
  const lines = soundSourceLinesWithOffsets(text);
  const originalStart = Number.isInteger(options.originalStart) ? options.originalStart : null;
  const soundsBlocks = originalStart !== null
    ? [findSoundsBlockAtPosition(lines, originalStart)].filter(Boolean)
    : findSoundsBlocks(lines);
  if (!soundsBlocks.length) {
    return null;
  }
  const originalName = options.originalName || definition.name;
  let fallback = null;
  for (const soundsBlock of soundsBlocks) {
    for (let index = soundsBlock.startLine + 1; index < soundsBlock.endLine; index += 1) {
      const line = lines[index];
      const parsed = parseSoundsDefinitionLine(line?.text || "");
      if (!parsed || parsed.kind !== definition.kind || parsed.name !== originalName) {
        continue;
      }
      const candidate = replaceSoundsDefinitionLine(text, line, soundsBlock, definition);
      if (originalStart !== null && originalStart >= line.start && originalStart <= line.end) {
        return candidate;
      }
      fallback ??= candidate;
    }
  }
  return fallback;
}

function replaceSoundsDefinitionLine(text, line, soundsBlock, definition) {
  const hasNewline = line.text.endsWith("\n");
  const replacement = `${definition.line}${hasNewline ? "\n" : ""}`;
  const nextSource = `${text.slice(0, line.start)}${replacement}${text.slice(line.end)}`;
  const selectionStart = line.start + replacement.length;
  return {
    source: nextSource,
    selectionStart,
    selectionEnd: selectionStart,
    definitionStart: line.start,
    definitionEnd: line.start + replacement.length,
  };
}

function replaceSoundReferencesInSource(source, kind, oldName, newName, options = {}) {
  if (!oldName || !newName || oldName === newName) {
    return { source, count: 0 };
  }
  const text = String(source || "");
  const lines = soundSourceLinesWithOffsets(text);
  const soundsBlocks = findSoundsBlocks(lines);
  const definitionStart = Number.isInteger(options.definitionStart) ? options.definitionStart : -1;
  const definitionEnd = Number.isInteger(options.definitionEnd) ? options.definitionEnd : -1;
  const selectionStart = Number.isInteger(options.selectionStart) ? options.selectionStart : -1;
  let changed = false;
  let count = 0;
  let selectionShift = 0;
  const nextLines = lines.map((line, lineIndex) => {
    if (!line || line.start >= text.length && !line.text) {
      return line?.text || "";
    }
    if (definitionStart >= 0 && line.start < definitionEnd && line.end > definitionStart) {
      return line.text;
    }
    const parsed = parseSoundsDefinitionLine(line.text);
    if (
      parsed?.kind === kind
      && lineIsInSoundsBlock(lineIndex, soundsBlocks)
    ) {
      return line.text;
    }
    const commentStart = soundLineCommentStart(line.text);
    const code = commentStart >= 0 ? line.text.slice(0, commentStart) : line.text;
    const comment = commentStart >= 0 ? line.text.slice(commentStart) : "";
    const replaced = replaceSoundReferencesInCode(code, kind, oldName, newName);
    if (replaced.count > 0) {
      changed = true;
      count += replaced.count;
      if (selectionStart >= 0 && line.end <= selectionStart) {
        selectionShift += replaced.code.length - code.length;
      }
      return `${replaced.code}${comment}`;
    }
    return line.text;
  });
  return { source: changed ? nextLines.join("") : text, count, selectionShift };
}

function replaceSoundReferencesInCode(code, kind, oldName, newName) {
  let output = "";
  let index = 0;
  let count = 0;
  while (index < code.length) {
    const char = code[index];
    if (char === "\"" || char === "'") {
      const end = soundQuotedSegmentEnd(code, index);
      output += code.slice(index, end);
      index = end;
      continue;
    }
    if (!soundIdentifierStart(char)) {
      output += char;
      index += 1;
      continue;
    }
    const wordStart = index;
    const wordEnd = soundIdentifierEnd(code, wordStart);
    const word = code.slice(wordStart, wordEnd);
    const commandKind = soundReferenceCommandKind(word);
    if (commandKind !== kind) {
      output += word;
      index = wordEnd;
      continue;
    }
    const whitespaceEnd = soundWhitespaceEnd(code, wordEnd);
    const nameEnd = soundIdentifierEnd(code, whitespaceEnd);
    const name = code.slice(whitespaceEnd, nameEnd);
    if (whitespaceEnd > wordEnd && name === oldName) {
      output += `${code.slice(wordStart, whitespaceEnd)}${newName}`;
      index = nameEnd;
      count += 1;
      continue;
    }
    output += word;
    index = wordEnd;
  }
  return { code: output, count };
}

function soundReferenceCommandKind(word) {
  if (word === "sfx") {
    return "sfx";
  }
  if (word === "play_music" || word === "pause_music" || word === "resume_music" || word === "stop_music") {
    return "music";
  }
  return "";
}

function soundLineCommentStart(line) {
  let quote = "";
  let escaped = false;
  for (let index = 0; index < line.length - 1; index += 1) {
    const char = line[index];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (char === "\\") {
      escaped = true;
      continue;
    }
    if (quote) {
      if (char === quote) {
        quote = "";
      }
      continue;
    }
    if (char === "\"" || char === "'") {
      quote = char;
      continue;
    }
    if (char === "/" && line[index + 1] === "/") {
      return index;
    }
  }
  return -1;
}

function soundQuotedSegmentEnd(source, start) {
  const quote = source[start];
  let escaped = false;
  for (let index = start + 1; index < source.length; index += 1) {
    const char = source[index];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (char === "\\") {
      escaped = true;
      continue;
    }
    if (char === quote) {
      return index + 1;
    }
  }
  return source.length;
}

function soundIdentifierStart(char) {
  return /[A-Za-z_]/.test(char || "");
}

function soundIdentifierPart(char) {
  return /[A-Za-z0-9_]/.test(char || "");
}

function soundIdentifierEnd(source, start) {
  let index = start;
  if (!soundIdentifierStart(source[index])) {
    return start;
  }
  index += 1;
  while (index < source.length && soundIdentifierPart(source[index])) {
    index += 1;
  }
  return index;
}

function soundWhitespaceEnd(source, start) {
  let index = start;
  while (index < source.length && /\s/.test(source[index] || "")) {
    index += 1;
  }
  return index;
}

function soundSourceLinesWithOffsets(source) {
  const lines = [];
  let start = 0;
  while (start <= source.length) {
    const newline = source.indexOf("\n", start);
    const end = newline === -1 ? source.length : newline + 1;
    lines.push({ text: source.slice(start, end), start, end });
    if (newline === -1) {
      break;
    }
    start = newline + 1;
  }
  return lines;
}

function findSoundsBlocks(lines) {
  const blocks = [];
  for (let index = 0; index < lines.length; index += 1) {
    const entry = lines[index];
    const code = stripLineComment(entry.text).trim();
    if (code === "sounds" || code === "sounds {") {
      const braceStyle = code.endsWith("{");
      const end = findSoundsBlockEnd(lines, index, braceStyle);
      if (!end) {
        continue;
      }
      blocks.push({
        startLine: index,
        endLine: end.index,
        bodyStart: lines[index].end,
        bodyEnd: end.entry.start,
        insertIndex: end.entry.start,
        indent: entry.text.match(/^\s*/)?.[0] || "",
        entryIndent: inferSoundsEntryIndent(lines, index + 1, end.index),
      });
    }
  }
  return blocks;
}

function findFirstSoundsBlock(lines) {
  return findSoundsBlocks(lines)[0] || null;
}

function findSoundsBlockAtPosition(lines, position) {
  return findSoundsBlocks(lines).find((block) => position >= block.bodyStart && position <= block.bodyEnd) || null;
}

function lineIsInSoundsBlock(lineIndex, soundsBlocks) {
  return soundsBlocks.some((block) => lineIndex > block.startLine && lineIndex < block.endLine);
}

function findSoundsBlockEnd(lines, headerIndex, braceStyle) {
  if (!braceStyle) {
    for (let index = headerIndex + 1; index < lines.length; index += 1) {
      const code = stripLineComment(lines[index].text).trim();
      if (code === "end") {
        return { index, entry: lines[index] };
      }
    }
    return null;
  }

  let depth = 1;
  for (let index = headerIndex + 1; index < lines.length; index += 1) {
    const code = stripLineComment(lines[index].text).trim();
    depth += braceDelta(code);
    if (depth <= 0) {
      return { index, entry: lines[index] };
    }
  }
  return null;
}

function inferSoundsEntryIndent(lines, start, end) {
  for (let index = start; index < end; index += 1) {
    const code = stripLineComment(lines[index].text).trim();
    if (/^(sfx|music)\s+/.test(code)) {
      return lines[index].text.match(/^\s*/)?.[0] || "";
    }
  }
  return "";
}

function findTopLevelNameInsertionIndex(lines) {
  let depth = 0;
  for (const line of lines) {
    const code = stripLineComment(line.text).trim();
    if (depth === 0 && /^name\s+\S+/.test(code)) {
      return line.end;
    }
    depth = Math.max(0, depth + braceDelta(code));
  }
  return -1;
}

function braceDelta(code) {
  let delta = 0;
  for (const ch of code) {
    if (ch === "{") {
      delta += 1;
    } else if (ch === "}") {
      delta -= 1;
    }
  }
  return delta;
}

function stripLineComment(line) {
  return String(line || "").split("//", 1)[0];
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function randomizeSoundSfx() {
  const preset = soundsApi().randomSfxPreset(`${Date.now()}:${Math.random()}`, soundsSfxTypeSelect.value);
  soundsSfxSeedInput.value = preset.seed;
  soundsSfxTypeSelect.value = preset.type;
  playSoundSfx().catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error"));
}

function randomizeSoundMusic() {
  const preset = soundsApi().randomPreset(`${Date.now()}:${Math.random()}`);
  soundsMusicSeedInput.value = preset.seed;
  soundsMusicHeightInput.value = preset.height ?? 0.5;
  setSoundMusicBars(preset.bars ?? 8);
  soundsMusicBpmInput.value = preset.bpm;
  setSoundProgress(0);
  updateSoundMusic({ restartProgress: 0 });
}

function soundMusicBars() {
  const bars = Number(soundsMusicBarsInput.value) || 8;
  return soundMusicBarOptions.includes(bars) ? bars : 8;
}

function setSoundMusicBars(value) {
  const bars = Number(value) || 8;
  soundsMusicBarsInput.value = String(soundMusicBarOptions.includes(bars) ? bars : 8);
}

function soundLabelForType(type) {
  return String(type || "")
    .replace(/[-_]+/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function soundClamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function updateSoundRangeFills() {
  updateSoundRangeFill(soundsMusicProgress);
  updateSoundRangeFill(soundsSfxVolumeInput);
  updateSoundRangeFill(soundsMusicHeightInput);
  updateSoundRangeFill(soundsMusicBpmInput);
  updateSoundRangeFill(soundsMusicVolumeInput);
}

function updateSoundRangeFill(input) {
  if (!input) {
    return;
  }
  const min = Number(input.min || 0);
  const max = Number(input.max || 100);
  const value = Number(input.value || 0);
  const denominator = max - min;
  const progress = denominator > 0 ? soundClamp((value - min) / denominator, 0, 1) : 0;
  input.style.setProperty("--sounds-range-progress", `${(progress * 100).toFixed(2)}%`);
}

soundsSfxPlayButton.addEventListener("click", () => {
  playSoundSfx().catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error"));
});
soundsSfxRandomButton.addEventListener("click", randomizeSoundSfx);
soundsSfxTitleInput.addEventListener("input", renderSoundSfx);
soundsSfxSeedInput.addEventListener("input", renderSoundSfx);
soundsSfxTypeSelect.addEventListener("change", renderSoundSfx);
soundsSfxVolumeInput.addEventListener("input", renderSoundSfx);
soundsSfxCopyButton.addEventListener("click", () => {
  copySoundLine("sfx").catch((error) => setStatus(`Could not copy sounds: ${error?.message || error}`, "is-error"));
});
soundsSfxInsertButton.addEventListener("click", () => insertSoundsDefinition("sfx"));
soundsSfxUpdateButton.addEventListener("click", () => updateSoundsDefinition("sfx"));
soundsMusicPlayButton.addEventListener("click", () => {
  toggleSoundMusic().catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error"));
});
soundsMusicRandomButton.addEventListener("click", randomizeSoundMusic);
soundsMusicTitleInput.addEventListener("input", updateSoundMusic);
soundsMusicSeedInput.addEventListener("input", updateSoundMusic);
soundsMusicHeightInput.addEventListener("input", updateSoundMusic);
soundsMusicBarsInput.addEventListener("change", () => updateSoundMusic({ restartProgress: 0 }));
soundsMusicBpmInput.addEventListener("input", updateSoundMusic);
soundsMusicVolumeInput.addEventListener("input", updateSoundMusic);
soundsMusicCopyButton.addEventListener("click", () => {
  copySoundLine("music").catch((error) => setStatus(`Could not copy sounds: ${error?.message || error}`, "is-error"));
});
soundsMusicInsertButton.addEventListener("click", () => insertSoundsDefinition("music"));
soundsMusicUpdateButton.addEventListener("click", () => updateSoundsDefinition("music"));
soundsMusicProgress.addEventListener("input", () => seekSoundMusic(Number(soundsMusicProgress.value)));
soundsMusicProgress.addEventListener("keydown", (event) => {
  if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
    event.preventDefault();
    seekSoundMusic(sounds.musicProgress + (event.key === "ArrowRight" ? 0.025 : -0.025));
  } else if (event.key === "Home") {
    event.preventDefault();
    seekSoundMusic(0);
  } else if (event.key === "End") {
    event.preventDefault();
    seekSoundMusic(0.9999);
  }
});
registerSourceEditableTarget?.("sounds", {
  load: loadSoundFromSourcePosition,
});

window.addEventListener("PuzzleSoundToolsReady", () => {
  resetSoundsBuilder();
});

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "hidden") {
    pauseSoundPlaybackForHiddenDocument();
  } else {
    resumeSoundPlaybackForVisibleDocument();
  }
});

window.addEventListener("PuzzleSoundToolsError", (event) => {
  const message = event.detail?.message || "unknown error";
  setSoundsUnavailable(`Sounds generator unavailable: ${message}`);
});

resetSoundsBuilder();
