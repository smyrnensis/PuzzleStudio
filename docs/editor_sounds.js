const soundPlayIcon = editorIconSvg("play");
const soundPauseIcon = editorIconSvg("pause");
const soundMusicBarOptions = [8, 16, 32, 64];

function soundsApi() {
  return sounds.audio;
}

async function resetSoundsBuilder() {
  if (!sounds.audioPromise) {
    sounds.audioPromise = window.PuzzleStudioRuntime.editorAudio()
      .then((audio) => {
        sounds.audio = audio;
        return audio;
      })
      .catch((error) => {
        sounds.audioPromise = null;
        throw error;
      });
  }
  await sounds.audioPromise;
  const api = soundsApi();
  if (!api) {
    setSoundsUnavailable("Editor audio preview unavailable.");
    return;
  }
  if (!sounds.initialized) {
    api.setFeedbackHandler((diagnostic) => {
      setStatus(`Sounds failed: ${diagnostic}`, "is-error");
    });
    const types = await api.sfxTypes();
    for (const type of types) {
      const option = document.createElement("option");
      option.value = type;
      option.textContent = soundLabelForType(type);
      soundsSfxTypeSelect.append(option);
    }
    soundsSfxSeedInput.value = soundRandomSeed();
    soundsSfxTypeSelect.value = types[0] || "select";
    soundsSfxVolumeInput.value = 1;
    soundsMusicSeedInput.value = soundRandomSeed();
    soundsMusicHeightInput.value = 0.5;
    setSoundMusicBars(8);
    soundsMusicBpmInput.value = 110;
    sounds.initialized = true;
  }
  setSoundProgress(0);
  renderSoundsBuilder();
}

function renderSoundsBuilder() {
  if (!soundsApi()) {
    setSoundsUnavailable("Editor audio preview unavailable.");
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

function soundSfxVolume() {
  return soundClamp(Number(soundsSfxVolumeInput.value), 0, 1);
}

function renderSoundSfx() {
  soundsSfxVolumeValue.textContent = `${Math.round(soundSfxVolume() * 100)}%`;
  updateSoundRangeFill(soundsSfxVolumeInput);
  refreshSoundCurrentLine("sfx");
}

function renderSoundMusic() {
  soundsMusicHeightValue.textContent = Number(soundsMusicHeightInput.value).toFixed(2);
  soundsMusicBarsValue.textContent = `${soundMusicBars()}`;
  soundsMusicBpmValue.textContent = `${Number(soundsMusicBpmInput.value)}`;
  soundsMusicVolumeValue.textContent = `${Math.round(Number(soundsMusicVolumeInput.value) * 100)}%`;
  updateSoundRangeFills();
  refreshSoundCurrentLine("music");
}

async function soundSourceRequest(source, request) {
  const api = window.PuzzleStudioRuntime?.soundSourceRequest;
  if (typeof api !== "function") {
    throw new Error("Rust sound source authoring is unavailable.");
  }
  return api(source, request);
}

async function refreshSoundCurrentLine(kind = "sfx") {
  try {
    const response = await soundSourceRequest(activeSoundEditSource(), {
      operation: "format",
      definition: soundCurrentDefinition(kind),
    });
    const output = kind === "music" ? soundsMusicOutput : soundsSfxOutput;
    output.textContent = response?.line || "";
  } catch (error) {
    setSoundsUnavailable(`Sound source authoring unavailable: ${error?.message || error}`);
  }
}

function soundCurrentDefinition(kind = "sfx") {
  if (kind === "music") {
    return {
      kind: "music",
      name: soundsMusicTitleInput.value,
      seed: soundsMusicSeedInput.value,
      bars: soundMusicBars(),
      height: Number(soundsMusicHeightInput.value),
      bpm: Number(soundsMusicBpmInput.value),
      volume: Number(soundsMusicVolumeInput.value),
    };
  }
  return {
    kind: "sfx",
    name: soundsSfxTitleInput.value,
    seed: soundsSfxSeedInput.value,
    type: soundSfxType(),
    volume: soundSfxVolume(),
  };
}

function setSoundsUnavailable(message) {
  soundsSfxOutput.textContent = message;
  soundsMusicOutput.textContent = message;
}

async function playSoundSfx() {
  const api = soundsApi();
  if (!api || shouldSuppressSoundPlayback()) {
    return;
  }
  await api.unlock();
  await api.playSfx({
    seed: soundsSfxSeedInput.value,
    type: soundSfxType(),
    volume: soundSfxVolume(),
  });
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
  await api.unlock();
  await api.playMusic(soundMusicPreviewRequest());
  setSoundMusicPlaying(true);
  startSoundProgress();
  renderSoundMusic();
}

function updateSoundMusic(options = {}) {
  renderSoundMusic();
  if (!sounds.musicPlaying) {
    return;
  }
  window.clearTimeout(sounds.musicRestartTimer);
  sounds.musicRestartTimer = window.setTimeout(() => {
    const progress = Number.isFinite(options.restartProgress)
      ? soundClamp(options.restartProgress, 0, 0.9999)
      : sounds.musicProgress;
    sounds.musicProgress = progress;
    soundsMusicProgress.value = sounds.musicProgress.toFixed(4);
    updateSoundRangeFill(soundsMusicProgress);
    soundsApi().playMusic(soundMusicPreviewRequest())
      .catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error"));
    startSoundProgress();
  }, 180);
}

function pauseSoundMusic() {
  window.clearTimeout(sounds.musicRestartTimer);
  sounds.musicRestartTimer = 0;
  const api = soundsApi();
  api?.pauseMusic()
    .then(() => api.musicProgress())
    .then((progress) => setSoundProgress(progress))
    .catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error"));
  setSoundMusicPlaying(false);
  cancelAnimationFrame(sounds.progressFrame);
  sounds.progressFrame = 0;
}

function stopSoundPlayback() {
  soundsApi()?.stop().catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error"));
  pauseSoundMusic();
}

function pauseSoundPlaybackForHiddenDocument() {
  sounds.visibilityPausedMusic = sounds.musicPlaying === true;
  soundsApi()?.setVisible(false).catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error"));
  setSoundMusicPlaying(false);
}

function resumeSoundPlaybackForVisibleDocument() {
  if (!sounds.visibilityPausedMusic || shouldSuppressSoundPlayback()) {
    return;
  }
  sounds.visibilityPausedMusic = false;
  soundsApi()?.setVisible(true)
    .then(() => {
      setSoundMusicPlaying(true);
      startSoundProgress();
    })
    .catch((error) => setStatus(`Could not resume music: ${error?.message || error}`, "is-error"));
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
    if (!sounds.musicPlaying) {
      sounds.progressFrame = 0;
      return;
    }
    soundsApi().musicProgress()
      .then((value) => {
        sounds.musicProgress = soundClamp(value, 0, 0.9999);
        soundsMusicProgress.value = sounds.musicProgress.toFixed(4);
        updateSoundRangeFill(soundsMusicProgress);
      })
      .catch((error) => {
        setSoundMusicPlaying(false);
        setStatus(`Sounds failed: ${error?.message || error}`, "is-error");
      })
      .finally(() => {
        sounds.progressFrame = sounds.musicPlaying ? requestAnimationFrame(tick) : 0;
      });
  };
  tick();
}

function seekSoundMusic(value) {
  setSoundProgress(value);
  if (!sounds.musicPlaying) {
    return;
  }
  soundsApi().playMusic(soundMusicPreviewRequest())
    .catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error"));
  startSoundProgress();
}

async function copySoundLine(kind = "sfx") {
  const response = await soundSourceRequest(activeSoundEditSource(), {
    operation: "format",
    definition: soundCurrentDefinition(kind),
  });
  const text = response?.line || "";
  if (!text) {
    return;
  }
  await copyTextToClipboard(text);
  setStatus("Copied sounds definition", "is-ok");
}

async function insertSoundsDefinition(kind = "sfx") {
  const document = activeSoundEditDocument();
  if (!document || !isTextDocument(document)) {
    return;
  }
  sounds.mode = kind === "music" ? "music" : "sfx";
  const source = activeSoundEditSource();
  const response = await soundSourceRequest(source, {
    operation: "insert",
    definition: soundCurrentDefinition(kind),
  });
  const insertion = response?.result;
  if (!insertion) throw new Error("Rust sound insertion returned no mutation.");
  const definition = insertion.definition;
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
    start: insertion.definitionStart,
    end: insertion.definitionEnd,
  }, document);
  renderSoundsBuilder();
  setStatus(`Added ${definition.kind} ${definition.name}`, "is-ok");
}

async function updateSoundsDefinition(kind = "sfx") {
  const definition = soundCurrentDefinition(kind);
  const document = activeSoundEditDocument();
  if (!definition || !document || !isTextDocument(document)) {
    return;
  }
  sounds.mode = kind === "music" ? "music" : "sfx";
  const editTarget = activeSoundEditTargetForDocument(document, definition.kind);
  const originalName = editTarget?.name || definition.name;
  const source = activeSoundEditSource();
  const response = await soundSourceRequest(source, {
    operation: "update",
    targetStart: editTarget?.start ?? -1,
    originalName,
    definition,
  });
  const replacement = response?.result;
  if (!replacement) throw new Error("Rust sound update returned no mutation.");
  document.source = replacement.source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorText(replacement.source, replacement.selectionStart, replacement.selectionEnd);
  }
  scheduleLocalSave();
  schedulePreview();
  sourceEditor.focus();
  setActiveSoundEditTarget({
    kind: definition.kind,
    name: replacement.definition.name,
    start: replacement.definitionStart,
    end: replacement.definitionEnd,
  }, document);
  renderSoundsBuilder();
  const referenceMessage = replacement.renamedReferenceCount > 0
    ? ` and ${replacement.renamedReferenceCount} reference${replacement.renamedReferenceCount === 1 ? "" : "s"}`
    : "";
  setStatus(`Updated ${replacement.definition.kind} ${replacement.definition.name}${referenceMessage}`, "is-ok");
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

async function loadSoundFromSourcePosition(position, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditorDocumentValue();
  const response = await soundSourceRequest(source, { operation: "inspect", cursor: position });
  const inspection = response?.definition;
  const entry = inspection ? {
    ...inspection.definition,
    start: inspection.start,
    end: inspection.end,
  } : null;
  if (!entry) {
    return null;
  }
  if (options.recordHistory && typeof pushSourceNavigationHistory === "function") {
    pushSourceNavigationHistory();
  }
  if (entry.kind === "music") {
    sounds.mode = "music";
    soundsMusicTitleInput.value = entry.name;
    soundsMusicSeedInput.value = entry.seed;
    soundsMusicHeightInput.value = entry.height;
    setSoundMusicBars(entry.bars);
    soundsMusicBpmInput.value = entry.bpm;
    soundsMusicVolumeInput.value = entry.volume;
    setSoundProgress(0);
  } else {
    sounds.mode = "sfx";
    soundsSfxTitleInput.value = entry.name;
    soundsSfxSeedInput.value = entry.seed;
    soundsSfxTypeSelect.value = entry.type;
    soundsSfxVolumeInput.value = entry.volume;
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

function randomizeSoundSfx() {
  const preset = soundsApi().randomSfxPreset(soundRandomSeed(), soundsSfxTypeSelect.value);
  soundsSfxSeedInput.value = preset.seed;
  soundsSfxTypeSelect.value = preset.type;
  playSoundSfx().catch((error) => setStatus(`Sounds failed: ${error?.message || error}`, "is-error"));
}

function randomizeSoundMusic() {
  const preset = soundsApi().randomMusicPreset(soundRandomSeed());
  soundsMusicSeedInput.value = preset.seed;
  soundsMusicHeightInput.value = preset.height;
  setSoundMusicBars(preset.bars);
  soundsMusicBpmInput.value = preset.bpm;
  setSoundProgress(0);
  updateSoundMusic({ restartProgress: 0 });
}

function soundMusicPreviewRequest() {
  return {
    seed: soundsMusicSeedInput.value,
    height: Number(soundsMusicHeightInput.value),
    bars: soundMusicBars(),
    bpm: Number(soundsMusicBpmInput.value),
    volume: Number(soundsMusicVolumeInput.value),
    progress: sounds.musicProgress,
  };
}

function soundRandomSeed() {
  const value = new Uint32Array(2);
  crypto.getRandomValues(value);
  return `${value[0].toString(16).padStart(8, "0")}${value[1].toString(16).padStart(8, "0")}`;
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

soundsSfxTitleInput.addEventListener("input", renderSoundSfx);
soundsSfxSeedInput.addEventListener("input", renderSoundSfx);
soundsSfxTypeSelect.addEventListener("change", renderSoundSfx);
soundsSfxVolumeInput.addEventListener("input", renderSoundSfx);
soundsSfxCopyButton.addEventListener("click", () => {
  copySoundLine("sfx").catch((error) => setStatus(`Could not copy sounds: ${error?.message || error}`, "is-error"));
});
soundsSfxInsertButton.addEventListener("click", () => {
  insertSoundsDefinition("sfx")
    .catch((error) => setStatus(`Could not insert sounds: ${error?.message || error}`, "is-error"));
});
soundsMusicTitleInput.addEventListener("input", updateSoundMusic);
soundsMusicSeedInput.addEventListener("input", updateSoundMusic);
soundsMusicHeightInput.addEventListener("input", updateSoundMusic);
soundsMusicBarsInput.addEventListener("change", () => updateSoundMusic({ restartProgress: 0 }));
soundsMusicBpmInput.addEventListener("input", updateSoundMusic);
soundsMusicVolumeInput.addEventListener("input", updateSoundMusic);
soundsMusicCopyButton.addEventListener("click", () => {
  copySoundLine("music").catch((error) => setStatus(`Could not copy sounds: ${error?.message || error}`, "is-error"));
});
soundsMusicInsertButton.addEventListener("click", () => {
  insertSoundsDefinition("music")
    .catch((error) => setStatus(`Could not insert sounds: ${error?.message || error}`, "is-error"));
});
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

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "hidden") {
    pauseSoundPlaybackForHiddenDocument();
  } else {
    resumeSoundPlaybackForVisibleDocument();
  }
});

resetSoundsBuilder().catch((error) => setSoundsUnavailable(`Editor audio preview unavailable: ${error?.message || error}`));
