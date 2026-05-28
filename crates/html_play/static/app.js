const board = document.querySelector("#board");
const screenView = document.querySelector("#screenView");
const screenFrame = document.querySelector("#screenFrame") || screenView?.parentElement;
const playSurface = document.querySelector(".play-surface");
const shell = document.querySelector("#shell");
const componentEmbedMode = new URLSearchParams(window.location.search).get("component") === "1";
document.documentElement.classList.toggle("is-component-embed", componentEmbedMode);
document.body.classList.toggle("is-component-embed", componentEmbedMode);
const messageQueue = [];
let messagePopup = null;
let clientPendingWaits = 0;
let activeThemeClass = "";
const activeThemeVariables = new Set();
const activationConfirmDelayMs = 160;
const puzzlescriptTerminalWidth = 34;

class PuzzleSoundRuntime {
  constructor() {
    this.sounds = { sfx: [], music: [] };
    this.context = null;
    this.activeMusic = new Map();
    this.pausedMusic = new Map();
  }

  configure(sounds) {
    this.sounds = sounds || { sfx: [], music: [] };
  }

  applyEvents(events) {
    for (const event of events || []) {
      if (event.kind === "play_sfx") {
        this.playSfx(event.name);
      } else if (event.kind === "play_music") {
        this.playMusic(event.name);
      } else if (event.kind === "pause_music") {
        this.pauseMusic(event.name || null);
      } else if (event.kind === "resume_music") {
        this.resumeMusic(event.name || null);
      } else if (event.kind === "stop_music") {
        this.stopMusic(event.name || null);
      }
    }
  }

  ensureContext() {
    const AudioContext = window.AudioContext || window.webkitAudioContext;
    if (!AudioContext) {
      return null;
    }
    if (!this.context) {
      this.context = new AudioContext();
    }
    if (this.context.state === "suspended") {
      this.context.resume();
    }
    return this.context;
  }

  playSfx(name) {
    const def = this.sfxDef(name);
    const context = this.ensureContext();
    if (!def || !context) {
      return;
    }
    const api = window.PuzzleSoundGenerator || window.PuzzleSoundTools || null;
    if (def.type === "puzzlescript" && api?.generatePuzzleScriptSoundEffect && api?.createPuzzleScriptSfxPlayer) {
      const effect = api.generatePuzzleScriptSoundEffect(def.seed);
      const player = api.createPuzzleScriptSfxPlayer(context, effect);
      player.start();
      return;
    }
    if (api?.generateSoundEffect && api?.createSfxPlayer) {
      const effect = api.generateSoundEffect(def.seed, { type: def.type || "random" });
      const player = api.createSfxPlayer(context, effect);
      player.start();
      return;
    }
    const seed = this.seedValue(def.seed);
    const type = String(def.type || "random");
    const now = context.currentTime;
    const duration = 0.08 + ((seed % 90) / 1000);
    const gain = context.createGain();
    gain.gain.setValueAtTime(0.0001, now);
    gain.gain.exponentialRampToValueAtTime(0.16, now + 0.008);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);
    gain.connect(context.destination);

    if (type.includes("hit") || type.includes("step") || type.includes("noise")) {
      const buffer = context.createBuffer(1, Math.max(1, Math.floor(context.sampleRate * duration)), context.sampleRate);
      const data = buffer.getChannelData(0);
      let random = this.seededRandom(seed);
      for (let index = 0; index < data.length; index += 1) {
        data[index] = (random() * 2 - 1) * (1 - index / data.length);
      }
      const source = context.createBufferSource();
      const filter = context.createBiquadFilter();
      filter.type = "bandpass";
      filter.frequency.value = 400 + (seed % 2200);
      source.buffer = buffer;
      source.connect(filter).connect(gain);
      source.start(now);
      source.stop(now + duration);
      return;
    }

    const oscillator = context.createOscillator();
    oscillator.type = type.includes("jump") ? "square" : "triangle";
    const startHz = 180 + (seed % 520);
    const endHz = type.includes("jump") ? startHz * 1.75 : Math.max(80, startHz * 0.55);
    oscillator.frequency.setValueAtTime(startHz, now);
    oscillator.frequency.exponentialRampToValueAtTime(endHz, now + duration);
    oscillator.connect(gain);
    oscillator.start(now);
    oscillator.stop(now + duration);
  }

  hasSfx(name) {
    return Boolean(this.sfxDef(name));
  }

  sfxDef(name) {
    return (this.sounds.sfx || []).find((entry) => entry.name === name);
  }

  playMusic(name, resume = {}) {
    const def = (this.sounds.music || []).find((entry) => entry.name === name);
    const context = this.ensureContext();
    if (!def || !context) {
      return;
    }
    if (this.activeMusic.has(name)) {
      return;
    }
    this.stopMusic();
    this.pausedMusic.delete(name);
    const api = window.PuzzleSoundGenerator || window.PuzzleSoundTools || null;
    if (api?.generateSong && api?.createPlayer) {
      const progress = typeof resume === "number" ? 0 : Number(resume.progress || 0);
      const song = api.generateSong(def.seed, {
        tone: Number(def.tone ?? 0.62),
        bpm: Number(def.bpm || 104),
        volume: Number(def.volume ?? 0.5),
      });
      const player = api.createPlayer(context, song.playbackScore);
      const handle = { player, progress };
      this.activeMusic.set(name, handle);
      player.start(progress);
      return;
    }
    const startIndex = typeof resume === "number" ? resume : Number(resume.index || 0);
    const handle = { timers: [], sources: [], index: startIndex };
    this.activeMusic.set(name, handle);
    const bpm = Number(def.bpm || 104);
    const step = 60 / Math.max(40, bpm);
    const volume = Math.max(0, Math.min(1, Number(def.volume ?? 0.5))) * 0.08;
    const seed = this.seedValue(def.seed);
    const root = 48 + (seed % 12);
    const scale = [0, 3, 5, 7, 10, 12, 15, 17];
    const notes = scale.map((offset) => root + offset);
    const schedule = () => {
      if (!this.activeMusic.has(name)) {
        return;
      }
      const note = notes[(handle.index + seed) % notes.length];
      this.playMusicNote(context, handle, note, context.currentTime + 0.02, step * 0.85, volume, Number(def.tone ?? 0.62));
      handle.index += 1;
      handle.timers.push(window.setTimeout(schedule, step * 1000));
    };
    schedule();
  }

  playMusicNote(context, handle, midi, startsAt, duration, volume, tone) {
    const oscillator = context.createOscillator();
    const gain = context.createGain();
    const filter = context.createBiquadFilter();
    const hz = 440 * Math.pow(2, (midi - 69) / 12);
    oscillator.type = tone > 0.68 ? "sine" : tone > 0.35 ? "triangle" : "square";
    oscillator.frequency.value = hz;
    filter.type = "lowpass";
    filter.frequency.value = 600 + tone * 2400;
    gain.gain.setValueAtTime(0.0001, startsAt);
    gain.gain.exponentialRampToValueAtTime(Math.max(0.0001, volume), startsAt + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0001, startsAt + duration);
    oscillator.connect(filter).connect(gain).connect(context.destination);
    oscillator.start(startsAt);
    oscillator.stop(startsAt + duration + 0.02);
    handle.sources.push(oscillator);
  }

  stopMusic(name = null) {
    for (const [key, handle] of [...this.activeMusic.entries()]) {
      if (name && key !== name) {
        continue;
      }
      this.stopMusicHandle(handle);
      this.activeMusic.delete(key);
    }
    for (const key of [...this.pausedMusic.keys()]) {
      if (!name || key === name) {
        this.pausedMusic.delete(key);
      }
    }
  }

  pauseMusic(name = null) {
    for (const [key, handle] of [...this.activeMusic.entries()]) {
      if (name && key !== name) {
        continue;
      }
      this.stopMusicHandle(handle);
      this.activeMusic.delete(key);
      this.pausedMusic.set(key, {
        index: handle.index || 0,
        progress: handle.player?.loopProgress?.() ?? handle.progress ?? 0,
      });
    }
  }

  resumeMusic(name = null) {
    const entries = [...this.pausedMusic.entries()].filter(([key]) => !name || key === name);
    for (const [key, paused] of entries) {
      this.playMusic(key, paused);
      this.pausedMusic.delete(key);
    }
  }

  stopMusicHandle(handle) {
    if (handle.player) {
      try {
        handle.progress = handle.player.loopProgress?.() ?? handle.progress ?? 0;
        handle.player.stop();
      } catch (_) {
      }
    }
    for (const timer of handle.timers || []) {
      window.clearTimeout(timer);
    }
    for (const source of handle.sources || []) {
      try {
        source.stop();
      } catch (_) {
      }
    }
  }

  seedValue(seed) {
    return String(seed || "0").split("").reduce((value, ch) => ((value * 31) + ch.charCodeAt(0)) >>> 0, 2166136261);
  }

  seededRandom(seed) {
    let value = seed >>> 0;
    return () => {
      value = (value * 1664525 + 1013904223) >>> 0;
      return value / 4294967296;
    };
  }
}

const standaloneRuntime = window.PuzzleStandaloneRuntime
  ? new window.PuzzleStandaloneRuntime(window.PuzzleExport)
  : null;
const soundRuntime = new PuzzleSoundRuntime();

let currentState = null;
let swipeStart = null;
const puzzleViewports = new Map();
const puzzle3FrameIframes = new Map();
const standardChoiceCursors = new Map();
let screenScaleSyncFrame = 0;
let screenScaleSyncPasses = 0;
const defaultSceneLogicalSize = { width: 4, height: 3 };
const defaultSceneLayoutUnit = 180;
const pendingCommandQueue = [];
let drainingCommandQueue = false;
let sceneEditorPreview = null;

async function requestJson(url, options = {}) {
  if (standaloneRuntime) {
    return standaloneRuntime.requestJson(url, options);
  }
  const response = await fetch(url, options);
  const body = await response.json();
  if (!response.ok) {
    throw new Error(body.error || response.statusText);
  }
  return body;
}

async function loadState() {
  render(await requestJson("/api/state"));
}

async function post(url) {
  try {
    const nextState = await requestJson(url, { method: "POST" });
    render(nextState);
  } catch (error) {
    showError(error);
  }
}

function render(state) {
  currentState = state;
  window.__PuzzleCurrentState = state;
  applyTheme(state?.theme || window.PuzzleExport?.theme || null);
  soundRuntime.configure(state?.sounds || window.PuzzleExport?.sounds || { sfx: [], music: [] });
  soundRuntime.applyEvents(state?.soundEvents || []);
  if (state) {
    state.soundEvents = [];
  }
  applyWaitEvents(state?.waitEvents || []);
  if (state) {
    state.busy = state.busy === true || clientPendingWaits > 0;
    state.waitEvents = [];
    const animationEvents = Array.isArray(state.animationEvents) ? state.animationEvents : [];
    if (state.scene && Array.isArray(animationEvents)) {
      state.scene.animationEvents = animationEvents;
    }
    if (Array.isArray(state.sceneLayers) && Array.isArray(animationEvents)) {
      const focusedLayer = state.sceneLayers.find((layer) => layer?.focused === true) || state.sceneLayers[0];
      if (focusedLayer?.scene) {
        focusedLayer.scene.animationEvents = animationEvents;
      }
    }
  }
  renderSceneStack(state);
  scheduleSelectedLevelMenuScroll();
  scheduleScreenScaleSync(3);
  notifyPreviewState(state);
  focusShell();
  applyMessageEvents(state?.messageEvents || []);
}

function scheduleScreenScaleSync(passes = 2) {
  if (componentEmbedMode || !screenFrame || !screenView || !playSurface) {
    return;
  }
  screenScaleSyncPasses = Math.max(screenScaleSyncPasses, Math.max(1, Math.trunc(Number(passes) || 1)));
  if (screenScaleSyncFrame) {
    return;
  }
  const tick = () => {
    screenScaleSyncFrame = 0;
    syncScreenScale();
    screenScaleSyncPasses -= 1;
    if (screenScaleSyncPasses > 0) {
      screenScaleSyncFrame = requestAnimationFrame(tick);
    }
  };
  screenScaleSyncFrame = requestAnimationFrame(tick);
}

function syncScreenScale() {
  if (componentEmbedMode || !screenFrame || !screenView || !playSurface) {
    return;
  }
  if (screenView.getClientRects().length === 0 || playSurface.getClientRects().length === 0) {
    return;
  }
  const logicalSize = currentSceneLogicalSize();
  const available = elementContentBox(playSurface);
  if (available.width <= 0 || available.height <= 0) {
    return;
  }
  const virtualSize = virtualSceneSize(logicalSize);
  const fit = fitLogicalSceneSize(virtualSize, available);
  const scale = Math.max(
    0.0001,
    Math.min(fit.width / virtualSize.width, fit.height / virtualSize.height),
  );
  const unit = virtualSize.unit;
  screenView.style.setProperty("--scene-layout-unit", `${unit}px`);
  screenView.style.setProperty("--scene-logical-width", String(logicalSize.width));
  screenView.style.setProperty("--scene-logical-height", String(logicalSize.height));
  screenView.style.setProperty("--screen-virtual-width", `${virtualSize.width}px`);
  screenView.style.setProperty("--screen-virtual-height", `${virtualSize.height}px`);
  screenView.style.setProperty("--screen-scale", scale.toFixed(6));
  screenFrame.style.width = `${Math.ceil(fit.width)}px`;
  screenFrame.style.height = `${Math.ceil(fit.height)}px`;
  screenFrame.dataset.screenScale = scale.toFixed(6);
  screenFrame.dataset.screenWidth = String(logicalSize.width);
  screenFrame.dataset.screenHeight = String(logicalSize.height);
  screenFrame.dataset.screenVirtualWidth = String(virtualSize.width);
  screenFrame.dataset.screenVirtualHeight = String(virtualSize.height);
  syncLogicalLayoutElementSizes(unit);
  fitPuzzleFrameComponents(screenView);
}

function currentSceneLogicalSize() {
  if (sceneEditorPreview?.layout?.size) {
    return logicalSceneSize(sceneEditorPreview.layout.size);
  }
  const layers = sceneLayers(currentState);
  const layer = layers.find((candidate) => candidate.focused === true) || layers[0];
  const sceneDef = sceneDefByName(layer?.name) || currentSceneDef();
  return logicalSceneSize(sceneDef?.layout?.size);
}

function logicalSceneSize(size) {
  const width = Math.max(1, Number(size?.width) || defaultSceneLogicalSize.width);
  const height = Math.max(1, Number(size?.height) || defaultSceneLogicalSize.height);
  return { width, height };
}

function virtualSceneSize(logicalSize) {
  const width = Math.max(1, logicalSize.width * defaultSceneLayoutUnit);
  const height = Math.max(1, logicalSize.height * defaultSceneLayoutUnit);
  return {
    width,
    height,
    unit: defaultSceneLayoutUnit,
  };
}

function fitLogicalSceneSize(virtualSize, available) {
  const aspect = virtualSize.width / virtualSize.height;
  let width = available.width;
  let height = width / aspect;
  if (height > available.height) {
    height = available.height;
    width = height * aspect;
  }
  return {
    width: Math.max(1, width),
    height: Math.max(1, height),
  };
}

function elementContentBox(element) {
  const style = window.getComputedStyle(element);
  const width = element.clientWidth
    - parseFloat(style.paddingLeft || "0")
    - parseFloat(style.paddingRight || "0");
  const height = element.clientHeight
    - parseFloat(style.paddingTop || "0")
    - parseFloat(style.paddingBottom || "0");
  return {
    width: Math.max(0, width),
    height: Math.max(0, height),
  };
}

function applyTheme(theme) {
  const root = document.body;
  for (const name of activeThemeVariables) {
    root.style.removeProperty(`--${name}`);
  }
  activeThemeVariables.clear();

  for (const [name, value] of Object.entries(theme?.variables || {})) {
    const variableName = normalizeThemeVariableName(name);
    if (!variableName) {
      continue;
    }
    root.style.setProperty(`--${variableName}`, String(value));
    activeThemeVariables.add(variableName);
  }

  if (activeThemeClass) {
    document.body.classList.remove(activeThemeClass);
    activeThemeClass = "";
  }
  const className = themeClassName(theme?.name);
  if (className) {
    document.body.classList.add(className);
    activeThemeClass = className;
  }
}

function normalizeThemeVariableName(name) {
  const normalized = String(name || "")
    .replace(/^--/, "")
    .replace(/_/g, "-")
    .toLowerCase();
  return /^[a-z0-9-]*[a-z][a-z0-9-]*$/.test(normalized) ? normalized : "";
}

function themeClassName(name) {
  const normalized = String(name || "")
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .toLowerCase();
  return normalized ? `theme-${normalized}` : "";
}

function applyMessageEvents(events) {
  for (const event of events || []) {
    if (event.kind === "message") {
      messageQueue.push(String(event.text || ""));
    }
  }
  showNextMessage();
}

function showNextMessage() {
  if (messagePopup || messageQueue.length === 0) {
    return;
  }
  if (document.body.classList.contains("theme-puzzlescript") && soundRuntime.hasSfx("ShowMessage")) {
    soundRuntime.playSfx("ShowMessage");
  }
  const text = messageQueue.shift();
  const backdrop = document.createElement("div");
  backdrop.className = "message-popup-backdrop";
  backdrop.setAttribute("role", "dialog");
  backdrop.setAttribute("aria-modal", "true");

  const panel = document.createElement("div");
  panel.className = "message-popup";
  const body = document.createElement("p");
  body.className = "message-popup-text";
  body.textContent = text;
  panel.append(body);
  backdrop.append(panel);
  backdrop.tabIndex = -1;
  backdrop.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    event.stopPropagation();
    backdrop.focus({ preventScroll: true });
  });
  shell.append(backdrop);
  messagePopup = backdrop;
  backdrop.focus();
}

function closeMessagePopup() {
  if (!messagePopup) {
    return;
  }
  if (document.body.classList.contains("theme-puzzlescript") && soundRuntime.hasSfx("CloseMessage")) {
    soundRuntime.playSfx("CloseMessage");
  }
  messagePopup.remove();
  messagePopup = null;
  focusShell();
  showNextMessage();
}

function notifyPreviewState(state) {
  if (window.parent === window) {
    return;
  }
  window.parent.postMessage({
    type: "PuzzleStudioPreviewState",
    levelIndex: state.levelIndex,
    rawScene: state.rawScene,
    scene: state.scene,
    inputs: state.inputs,
    screen: state.currentScene || state.screen,
    screenHasPuzzle: currentSceneHasPuzzle() || Boolean(state.scene),
    theme: state.theme || window.PuzzleExport?.theme || null,
  }, "*");
}

function notifySceneEditorPreview(requestId = sceneEditorPreview?.requestId || "") {
  if (window.parent === window || !sceneEditorPreview) {
    return;
  }
  const sceneName = sceneEditorPreview.sceneName || currentState?.currentScene || currentState?.screen || "";
  const sceneDef = sceneDefByName(sceneName);
  const layout = mergedScenePreviewLayout(sceneDef, sceneEditorPreview.layout);
  window.parent.postMessage({
    type: "PuzzleStudioScenePreview",
    requestId,
    scene: sceneName,
    theme: sceneEditorPreview.theme || currentState?.theme || window.PuzzleExport?.theme || null,
    layout,
    logicalSize: logicalSceneSize(layout?.size),
    components: sceneEditorComponentMetadata(sceneDef?.components || [], {
      __sceneDef: sceneDef,
      __sceneState: sceneEditorPreview.state || currentState?.sceneState || currentState?.screenState || {},
      __standardChoiceCounter: { value: 0 },
    }),
    error: sceneDef ? null : `Unknown scene: ${sceneName}`,
  }, "*");
}

function renderSceneEditorPreview(config = {}) {
  const sceneName = String(config.scene?.name || config.sceneName || currentState?.currentScene || currentState?.screen || "").trim();
  const sceneDef = sceneDefByName(sceneName);
  sceneEditorPreview = {
    requestId: String(config.requestId || ""),
    sceneName,
    theme: normalizeScenePreviewTheme(config.theme) || currentState?.theme || window.PuzzleExport?.theme || null,
    layout: normalizeScenePreviewLayout(config.layout),
    state: normalizeScenePreviewState(config.state),
    inspect: config.inspect || {},
  };
  if (!sceneDef) {
    notifySceneEditorPreview(sceneEditorPreview.requestId);
    return;
  }
  const baseState = currentState || window.PuzzleExport || {};
  const existingLayer = sceneLayers(baseState).find((layer) => layer.name === sceneName);
  const previewState = {
    ...baseState,
    currentScene: sceneName,
    screen: sceneName,
    theme: sceneEditorPreview.theme,
    sceneState: sceneEditorPreview.state || existingLayer?.sceneState || existingLayer?.state || baseState.sceneState || baseState.screenState || {},
    screenState: sceneEditorPreview.state || existingLayer?.sceneState || existingLayer?.state || baseState.sceneState || baseState.screenState || {},
    sceneLayers: [{
      name: sceneName,
      focused: true,
      scene: existingLayer?.scene || baseState.scene || null,
      sceneState: sceneEditorPreview.state || existingLayer?.sceneState || existingLayer?.state || baseState.sceneState || baseState.screenState || {},
      scenePuzzles: existingLayer?.scenePuzzles || baseState.scenePuzzles || baseState.screenPuzzles || [],
    }],
  };
  currentState = previewState;
  window.__PuzzleCurrentState = previewState;
  applyTheme(sceneEditorPreview.theme);
  renderSceneEditorLayer(sceneDef, previewState);
  scheduleScreenScaleSync(3);
  notifySceneEditorPreview(sceneEditorPreview.requestId);
}

function renderSceneEditorLayer(sceneDef, state) {
  screenView.replaceChildren();
  const layer = state.sceneLayers[0];
  const components = sceneDef?.components || [];
  const hasPuzzle = sceneHasComponent(sceneDef, "puzzle") || sceneHasComponent(sceneDef, "frame");
  const isMenuScene = sceneIsMenuLike(sceneDef, hasPuzzle);
  const scope = {
    __sceneLayer: layer,
    __sceneDef: sceneDef,
    __sceneState: layer.sceneState || layer.state || {},
    __sceneMenuLike: isMenuScene,
    __standardChoiceCounter: { value: 0 },
    __componentPath: ["components"],
  };
  const layerEl = document.createElement("div");
  layerEl.className = "scene-layer is-focused";
  layerEl.classList.toggle("is-menu-scene", isMenuScene);
  layerEl.classList.toggle("has-ratio-content", components.some((component) => componentContainsSizingKind(component, "ratio")));
  layerEl.style.zIndex = "10";
  applySceneLayout(layerEl, mergedScenePreviewLayout(sceneDef, sceneEditorPreview?.layout), { root: true });
  renderSurfaceComponents(components, layerEl, scope);
  markSingleFrameComponentLayer(layerEl);
  screenView.append(layerEl);
  fitPuzzleFrameComponents(screenView);
}

function mergedScenePreviewLayout(sceneDef, override) {
  const base = cloneJson(sceneDef?.layout || {});
  const next = { ...base, ...(override || {}) };
  if (override?.size) {
    next.size = { ...(base.size || {}), ...override.size };
  }
  if (override?.align) {
    next.align = { ...(base.align || {}), ...override.align };
  }
  return next;
}

function normalizeScenePreviewTheme(theme) {
  if (!theme) {
    return null;
  }
  if (typeof theme === "string") {
    return { name: theme, variables: {} };
  }
  const name = String(theme.name || "").trim();
  const variables = {};
  for (const [key, value] of Object.entries(theme.variables || {})) {
    const normalized = normalizeThemeVariableName(key);
    if (normalized) {
      variables[normalized] = String(value);
    }
  }
  return { name, variables };
}

function normalizeScenePreviewLayout(layout) {
  if (!layout || typeof layout !== "object") {
    return null;
  }
  const next = {};
  if (layout.size) {
    const width = Number(layout.size.width);
    const height = Number(layout.size.height);
    if (Number.isFinite(width) && width > 0 && Number.isFinite(height) && height > 0) {
      next.size = { width, height };
    }
  }
  if (layout.gap !== undefined && layout.gap !== null && layout.gap !== "") {
    const gap = Number(layout.gap);
    if (Number.isFinite(gap) && gap >= 0) {
      next.gap = gap;
    }
  }
  if (layout.align && typeof layout.align === "object") {
    next.align = {};
    if (layout.align.x) {
      next.align.x = String(layout.align.x);
    }
    if (layout.align.y) {
      next.align.y = String(layout.align.y);
    }
  }
  return Object.keys(next).length ? next : null;
}

function normalizeScenePreviewState(state) {
  return state && typeof state === "object" && !Array.isArray(state) ? cloneJson(state) : null;
}

function cloneJson(value) {
  return value == null ? value : JSON.parse(JSON.stringify(value));
}

function sceneEditorComponentMetadata(components, scope = {}) {
  const result = [];
  for (const [index, component] of (components || []).entries()) {
    const path = [...(scope.__componentPath || ["components"]), index];
    result.push(sceneEditorComponentMeta(component, path, scope));
    if (component.kind === "row" || component.kind === "column" || component.kind === "box") {
      result.push(...sceneEditorComponentMetadata(component.children || [], {
        ...scope,
        __componentPath: [...path, "children"],
      }));
    } else if (component.kind === "conditional") {
      result.push(...sceneEditorComponentMetadata(component.children || [], {
        ...scope,
        __componentPath: [...path, "children"],
      }));
      result.push(...sceneEditorComponentMetadata(component.elseChildren || [], {
        ...scope,
        __componentPath: [...path, "elseChildren"],
      }));
    } else if (component.kind === "for") {
      result.push(...sceneEditorComponentMetadata(component.children || [], {
        ...scope,
        __componentPath: [...path, "children"],
      }));
    }
  }
  return result;
}

function sceneEditorComponentMeta(component, path, scope = {}) {
  const meta = {
    path,
    kind: component.kind || "",
    layout: component.layout || null,
  };
  if (component.kind === "button" || component.kind === "choice") {
    meta.label = resolveLabel(component.label, scope) || sceneTitle(effectLabel(component.effect));
    meta.effect = component.effect || null;
  } else if (component.kind === "text" || component.kind === "title" || component.kind === "subtitle") {
    meta.label = resolveLabel(component.content || component, scope);
  } else if (component.source) {
    meta.source = component.source;
  }
  return meta;
}

function renderSceneStack(state) {
  screenView.replaceChildren();

  const layers = sceneLayers(state);
  syncVisualThemeForSceneStack(layers);
  if (componentEmbedMode && renderEmbeddedPuzzleComponent(layers)) {
    return;
  }
  screenView.classList.toggle("has-scene-stack", layers.length > 1);
  for (const [index, layer] of layers.entries()) {
    const sceneDef = sceneDefByName(layer.name);
    const components = sceneDef?.components || [];
    const hasPuzzle = sceneHasComponent(sceneDef, "puzzle") || sceneHasComponent(sceneDef, "frame");
    const isMenuScene = sceneIsMenuLike(sceneDef, hasPuzzle);
    const scope = {
      __sceneLayer: layer,
      __sceneDef: sceneDef,
      __sceneState: layer.sceneState || layer.state || {},
      __sceneMenuLike: isMenuScene,
      __standardChoiceCounter: { value: 0 },
    };

    const layerEl = document.createElement("div");
    layerEl.className = "scene-layer";
    layerEl.classList.toggle("is-focused", layer.focused === true);
    layerEl.classList.toggle("is-menu-scene", isMenuScene);
    layerEl.classList.toggle("has-ratio-content", components.some((component) => componentContainsSizingKind(component, "ratio")));
    layerEl.style.zIndex = String(10 + index);
    applySceneLayout(layerEl, sceneDef?.layout, { root: true });
    renderSurfaceComponents(components, layerEl, scope);
    markSingleFrameComponentLayer(layerEl);
    screenView.append(layerEl);
  }
  fitPuzzleFrameComponents(screenView);
}

function markSingleFrameComponentLayer(layerEl) {
  const visibleChildren = [...layerEl.children].filter((child) => !child.hidden);
  const singleFrameComponent =
    visibleChildren.length === 1 && visibleChildren[0]?.dataset.frameComponent === "true";
  layerEl.classList.toggle("has-single-frame-component", singleFrameComponent);
}

function fitPuzzleFrameComponents(root = screenView) {
  if (!root) {
    return;
  }
  for (const boardEl of root.querySelectorAll(".board[data-frame-component=\"true\"]")) {
    const parent = boardEl.parentElement;
    const cols = Math.max(1, Number(boardEl.dataset.viewportWidth) || 1);
    const rows = Math.max(1, Number(boardEl.dataset.viewportHeight) || 1);
    if (!parent || parent.getClientRects().length === 0) {
      continue;
    }
    const frame = elementContentBox(parent);
    if (frame.width <= 0 || frame.height <= 0) {
      continue;
    }
    const cellSize = Math.max(0.0001, Math.min(frame.width / cols, frame.height / rows));
    boardEl.style.width = `${cols * cellSize}px`;
    boardEl.style.height = `${rows * cellSize}px`;
    boardEl.style.setProperty("--cell-size", `${cellSize}px`);
  }
}

function renderEmbeddedPuzzleComponent(layers) {
  const layer = layers.find((candidate) => candidate.focused === true) || layers[0];
  const sceneDef = sceneDefByName(layer?.name);
  const component = findComponentByKind(sceneDef?.components || [], "puzzle");
  if (!layer || !sceneDef || !component) {
    return false;
  }
  const scope = {
    __sceneLayer: layer,
    __sceneDef: sceneDef,
    __sceneState: layer.sceneState || layer.state || {},
    __sceneMenuLike: false,
    __standardChoiceCounter: { value: 0 },
  };
  screenView.classList.remove("has-scene-stack");
  screenView.append(renderPuzzle(component, scope));
  return true;
}

function syncVisualThemeForSceneStack(layers) {
  const visualThemeClass = window.GameVisuals?.themeClass || "";
  if (!visualThemeClass || visualThemeClass === activeThemeClass) {
    return;
  }
  const hasPuzzleLayer = layers.some((layer) => {
    const scene = sceneDefByName(layer.name);
    return sceneHasComponent(scene, "puzzle") || sceneHasComponent(scene, "frame");
  });
  if (!hasPuzzleLayer) {
    document.body.classList.remove(visualThemeClass);
  }
}

function sceneLayers(state) {
  if (Array.isArray(state?.sceneLayers) && state.sceneLayers.length > 0) {
    return state.sceneLayers;
  }
  const name = state?.currentScene || state?.screen || "playing";
  return [{
    name,
    focused: true,
    scene: state?.scene || null,
    sceneState: state?.sceneState || state?.screenState || {},
    scenePuzzles: state?.scenePuzzles || state?.screenPuzzles || [],
  }];
}

function sceneHasComponent(scene, kind) {
  return Boolean(scene?.components?.some((component) => componentHasKind(component, kind)));
}

function componentHasKind(component, kind) {
  if (component.kind === kind) {
    return true;
  }
  return Boolean(component.children?.some((child) => componentHasKind(child, kind))
    || component.elseChildren?.some((child) => componentHasKind(child, kind)));
}

function componentSizingKind(component) {
  switch (component?.kind) {
    case "frame":
    case "puzzle":
    case "puzzle3":
      return "ratio";
    case "title":
    case "subtitle":
    case "text":
    case "button":
    case "choice":
      return "flow";
    case "level_menu":
    case "menu":
    case "for":
      return "collection";
    case "box":
    case "row":
    case "column":
      return "container";
    default:
      return "unknown";
  }
}

function componentContainsSizingKind(component, sizing) {
  if (componentSizingKind(component) === sizing) {
    return true;
  }
  return Boolean(component?.children?.some((child) => componentContainsSizingKind(child, sizing))
    || component?.elseChildren?.some((child) => componentContainsSizingKind(child, sizing)));
}

function applySizingKind(element, component) {
  const sizing = componentSizingKind(component);
  element.dataset.sceneSizing = sizing;
  if (sizing !== "unknown") {
    element.classList.add(`scene-${sizing}`);
  }
}

function renderRatioComponent(component, scope = {}) {
  const slot = document.createElement("div");
  slot.className = "scene-ratio-slot";
  slot.dataset.sceneSizing = "ratio";
  slot.dataset.frameComponent = "true";
  slot.dataset.frameKind = component.kind || "frame";
  applySceneLayout(slot, component.layout);
  slot.append(component.kind === "puzzle3"
    ? renderPuzzle3Frame(component, scope)
    : renderPuzzle(component, scope));
  return slot;
}

function findComponentByKind(components, kind) {
  for (const component of components || []) {
    if (component.kind === kind) {
      return component;
    }
    const found = findComponentByKind(component.children || [], kind);
    if (found) {
      return found;
    }
    const elseFound = findComponentByKind(component.elseChildren || [], kind);
    if (elseFound) {
      return elseFound;
    }
  }
  return null;
}

function currentSceneHasPuzzle() {
  return sceneHasComponent(currentSceneDef(), "puzzle") || sceneHasComponent(currentSceneDef(), "frame");
}

function currentSceneHasPuzzle3() {
  return sceneHasComponent(currentSceneDef(), "puzzle3");
}

function currentSceneHasLevelMenu() {
  return sceneHasComponent(currentSceneDef(), "level_menu");
}

function sceneIsMenuLike(scene, hasPuzzle = sceneHasComponent(scene, "puzzle") || sceneHasComponent(scene, "frame")) {
  return Boolean(scene && (sceneHasComponent(scene, "level_menu") || !hasPuzzle));
}

function sceneTitle(name) {
  return String(name || "")
    .split(/[_-]+/)
    .filter(Boolean)
    .map((part) => part[0]?.toUpperCase() + part.slice(1))
    .join(" ") || "Screen";
}

function renderSurfaceComponents(components, parent = screenView, scope = {}) {
  for (const [index, component] of (components || []).entries()) {
    const path = [...(scope.__componentPath || ["components"]), index];
    parent.append(renderComponent(component, {
      ...scope,
      __componentPath: path,
    }));
  }
}

function renderComponent(component, scope = {}) {
  switch (component.kind) {
    case "frame":
    case "puzzle":
    case "puzzle3":
      return renderRatioComponent(component, scope);
    case "title":
      return renderTitle(component, "view-title", scope);
    case "subtitle":
      return renderTitle(component, "view-subtitle", scope);
    case "text":
      return renderText(component, scope);
    case "button":
      return renderButton(component, scope);
    case "choice":
      return renderChoice(component, scope);
    case "box":
    case "row":
    case "column":
      return renderContainer(component, scope);
    case "conditional":
      return renderConditional(component, scope);
    case "for":
      return renderFor(component, scope);
    case "level_menu":
      return renderLevelMenu(currentState, component, scope);
    case "menu":
      return renderMenuInstance(component, scope);
    default: {
      const empty = document.createElement("div");
      empty.hidden = true;
      return empty;
    }
  }
}

function renderMenuInstance(component, scope = {}) {
  const menu = currentState?.menus?.find((candidate) => candidate.name === component.menu);
  const container = document.createElement("div");
  container.className = "view-menu";
  applySizingKind(container, component);
  if (!menu) {
    container.hidden = true;
    return container;
  }

  const menuScope = {
    ...scope,
    __menuInstance: component.name,
    __menuButtonCounter: { value: 0 },
    __menuCursor: Number((scope.__sceneState || currentState?.sceneState || currentState?.screenState)?.[`__menu_${component.name}_cursor`] || 0),
  };
  renderSurfaceComponents(menu.view || [], container, menuScope);
  return container;
}

function renderTitle(component, className, scope = {}) {
  const title = document.createElement("p");
  title.className = className;
  title.textContent = resolveLabel(component.content, scope);
  applySizingKind(title, component);
  applySceneLayout(title, component.layout);
  return title;
}

function renderPuzzle(component, scope = {}) {
  const layer = scope.__sceneLayer;
  const scene = layer?.scene || currentState.scene;
  if (!scene) {
    const empty = document.createElement("div");
    empty.hidden = true;
    return empty;
  }
  const root = document.createElement("div");
  root.className = "board";
  root.dataset.frameComponent = "true";
  root.dataset.source = component.source || "";
  root.dataset.scene = layer?.name || currentState?.currentScene || currentState?.screen || "";
  const key = `${root.dataset.scene}:${root.dataset.source}`;
  const renderer = new window.PuzzleRenderer(root, {
    renderMode: "canvas",
  });
  renderer.viewport = puzzleViewports.get(key);
  renderer.render(scene);
  puzzleViewports.set(key, renderer.viewport);
  return root;
}

function renderPuzzle3Frame(component, scope = {}) {
  if (!window.Puzzle3DFrameFixture || !window.Puzzle3DFrameAssets) {
    const empty = document.createElement("div");
    empty.hidden = true;
    return empty;
  }
  const sceneName = scope.__sceneDef?.name || scope.__sceneLayer?.name || currentState?.currentScene || currentState?.screen || "playing";
  const source = component.source || "board";
  const key = `${sceneName}:${source}`;
  let frame = puzzle3FrameIframes.get(key);
  if (!frame) {
    frame = document.createElement("iframe");
    frame.className = "puzzle3-frame";
    frame.dataset.frameComponent = "true";
    frame.loading = "eager";
    frame.dataset.scene = sceneName;
    frame.dataset.source = source;
    frame.title = `${sceneTitle(sceneName)} ${source}`;
    frame.srcdoc = puzzle3FrameSrcdoc(sceneName);
    puzzle3FrameIframes.set(key, frame);
  }
  return frame;
}

function puzzle3FrameSrcdoc(sceneName) {
  const assets = window.Puzzle3DFrameAssets || {};
  const fixture = JSON.parse(JSON.stringify(window.Puzzle3DFrameFixture || {}));
  fixture.currentScene = sceneName || fixture.currentScene || fixture.scenes?.[0]?.name || "playing";
  if (Number.isInteger(currentState?.levelIndex)) {
    fixture.levelIndex = currentState.levelIndex;
  } else if (Number.isInteger(currentState?.selectedLevelIndex)) {
    fixture.levelIndex = currentState.selectedLevelIndex;
  }
  const fixtureJson = puzzle3SafeScriptJson(fixture);
  const sourceJson = puzzle3SafeScriptJson(assets.source || "");
  const puzzlePathJson = puzzle3SafeScriptJson(assets.puzzlePath || "game.puzzle");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>${puzzle3SafeScriptText(assets.styleCss || "")}</style>
<style>html,body,#screenView{width:100%;height:100%;margin:0;overflow:hidden;background:transparent;}body{display:block;}.puzzle3-component{width:100%;height:100%;}</style>
</head>
<body class="theme-clean is-component-embed">
<main id="screenView" class="scene">
  <div class="puzzle3-component">
    <canvas id="view" width="960" height="640" aria-label="Puzzle3 component"></canvas>
  </div>
</main>
<script>window.Puzzle3DComponentEmbed=true;window.Puzzle3DFixture=${fixtureJson};window.Puzzle3DSource=${sourceJson};window.Puzzle3DPath=${puzzlePathJson};${puzzle3SafeScriptText(assets.embeddedWasmJs || "")}</script>
<script>${puzzle3SafeScriptText(assets.visualCoreJs || "")}</script>
<script>${puzzle3SafeScriptText(assets.appJs || "")}</script>
</body>
</html>`;
}

function puzzle3SafeScriptJson(value) {
  return JSON.stringify(value)
    .replace(/</g, "\\u003c")
    .replace(/>/g, "\\u003e")
    .replace(/&/g, "\\u0026");
}

function puzzle3SafeScriptText(value) {
  return String(value || "").replace(/<\/(script|style)/gi, "<\\/$1");
}

function renderText(component, scope = {}) {
  const text = document.createElement("p");
  text.className = "view-text";
  if (component.source === "path") {
    text.textContent = String(resolveViewPath(component.path, scope) ?? "");
  } else {
    text.textContent = component.value || "";
  }
  applySizingKind(text, component);
  applySceneLayout(text, component.layout);
  return text;
}

function renderButton(component, scope = {}) {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = resolveLabel(component.label, scope) || sceneTitle(effectLabel(component.effect));
  annotateSceneEditorComponent(button, component, scope);
  if (scope.__menuInstance && component.value) {
    const counter = scope.__menuButtonCounter || { value: 0 };
    const index = counter.value;
    counter.value += 1;
    button.classList.toggle("is-selected", index === scope.__menuCursor);
    button.addEventListener("click", () => {
      if (selectSceneEditorComponent(component, scope)) {
        return;
      }
      runActivationConfirm(button, () => sendCommand(`${scope.__menuInstance}.enter:${index}`));
    });
  } else {
    button.addEventListener("click", () => {
      if (selectSceneEditorComponent(component, scope)) {
        return;
      }
      runEffectActivationConfirm(button, component.effect, scope);
    });
  }
  applySizingKind(button, component);
  applySceneLayout(button, component.layout);
  return button;
}

function renderChoice(component, scope = {}) {
  const choice = document.createElement("button");
  choice.type = "button";
  choice.textContent = resolveLabel(component.label, scope) || sceneTitle(effectLabel(component.effect));
  choice.classList.add("standard-choice");
  annotateSceneEditorComponent(choice, component, scope);
  if (!scope.__insideFor) {
    const counter = scope.__standardChoiceCounter || { value: 0 };
    scope.__standardChoiceCounter = counter;
    const index = counter.value;
    counter.value += 1;
    choice.classList.toggle("is-selected", index === standardChoiceCursor(scope.__sceneDef));
    choice.addEventListener("click", () => {
      if (selectSceneEditorComponent(component, scope)) {
        return;
      }
      standardChoiceCursors.set(scope.__sceneDef.name, index);
      runEffectActivationConfirm(choice, component.effect, scope);
    });
  } else {
    choice.addEventListener("click", () => {
      if (selectSceneEditorComponent(component, scope)) {
        return;
      }
      runEffectActivationConfirm(choice, component.effect, scope);
    });
  }
  applySizingKind(choice, component);
  applySceneLayout(choice, component.layout);
  return choice;
}

function annotateSceneEditorComponent(element, component, scope = {}) {
  if (!sceneEditorPreview?.inspect?.enabled || !element) {
    return;
  }
  const path = scope.__componentPath || [];
  element.dataset.sceneEditorPath = JSON.stringify(path);
  element.dataset.sceneEditorKind = component.kind || "";
  element.style.outline = "2px solid rgba(41, 126, 255, 0.42)";
  element.style.outlineOffset = "2px";
  if (sameSceneEditorPath(path, sceneEditorPreview.inspect.selectedPath)) {
    element.style.outline = "3px solid rgba(255, 178, 48, 0.9)";
  }
}

function selectSceneEditorComponent(component, scope = {}) {
  if (!sceneEditorPreview?.inspect?.enabled) {
    return false;
  }
  const path = scope.__componentPath || [];
  sceneEditorPreview.inspect.selectedPath = path;
  window.parent?.postMessage({
    type: "PuzzleStudioSceneComponentSelected",
    requestId: sceneEditorPreview.requestId || "",
    scene: sceneEditorPreview.sceneName || "",
    component: sceneEditorComponentMeta(component, path, {
      ...scope,
      __componentPath: path,
    }),
  }, "*");
  renderSceneEditorPreview(sceneEditorPreview);
  return true;
}

function sameSceneEditorPath(left, right) {
  return JSON.stringify(left || []) === JSON.stringify(right || []);
}

function runEffectActivationConfirm(control, effect, scope = {}) {
  if (!shouldDelayActivationConfirm()) {
    return sendEffect(effect, scope);
  }
  const { immediate, delayed } = splitActivationEffects(effect);
  for (const child of immediate) {
    Promise.resolve(runImmediateActivationEffect(child, scope)).catch((error) => showError(error));
  }
  return runActivationConfirm(control, () => delayed ? sendEffect(delayed, scope) : undefined);
}

function splitActivationEffects(effect) {
  if (effect?.kind !== "sequence") {
    return { immediate: [], delayed: effect };
  }
  const effects = effect.effects || [];
  let delayedStart = 0;
  while (delayedStart < effects.length && isImmediateActivationEffect(effects[delayedStart].effect || effects[delayedStart])) {
    delayedStart += 1;
  }
  const immediate = effects.slice(0, delayedStart).map((child) => child.effect || child);
  const delayedEffects = effects.slice(delayedStart);
  if (delayedEffects.length === 0) {
    return { immediate, delayed: null };
  }
  if (delayedEffects.length === 1) {
    return { immediate, delayed: delayedEffects[0].effect || delayedEffects[0] };
  }
  return { immediate, delayed: { kind: "sequence", effects: delayedEffects } };
}

function isImmediateActivationEffect(effect) {
  return effect?.kind === "play_sfx";
}

function runImmediateActivationEffect(effect, scope = {}) {
  if (effect?.kind === "play_sfx") {
    soundRuntime.playSfx(effect.name);
    return undefined;
  }
  return sendEffect(effect, scope);
}

function runActivationConfirm(control, run) {
  if (!shouldDelayActivationConfirm()) {
    return run();
  }
  const target = control;
  if (!target) {
    return run();
  }
  if (target.dataset.confirming === "true") {
    return undefined;
  }
  target.dataset.confirming = "true";
  applyActivationConfirmGlyphs(target);
  target.classList.add("is-confirming");
  window.setTimeout(() => {
    Promise.resolve(run())
      .finally(() => {
        target.classList.remove("is-confirming");
        clearActivationConfirmGlyphs(target);
        delete target.dataset.confirming;
      })
      .catch((error) => showError(error));
  }, activationConfirmDelayMs);
  return undefined;
}

function shouldDelayActivationConfirm() {
  return document.body.classList.contains("theme-puzzlescript");
}

function applyActivationConfirmGlyphs(target) {
  if (!document.body.classList.contains("theme-puzzlescript")) {
    return;
  }
  const label = activationConfirmLabel(target);
  const labelWidth = Array.from(label).length;
  const dashCount = Math.max(0, puzzlescriptTerminalWidth - 2 - labelWidth);
  const leftCount = Math.floor(dashCount / 2);
  const rightCount = dashCount - leftCount;
  target.style.setProperty("--ps-confirm-before", JSON.stringify(`#${"-".repeat(leftCount)}`));
  target.style.setProperty("--ps-confirm-after", JSON.stringify(`${"-".repeat(rightCount)}#`));
}

function clearActivationConfirmGlyphs(target) {
  target.style.removeProperty("--ps-confirm-before");
  target.style.removeProperty("--ps-confirm-after");
}

function activationConfirmLabel(target) {
  const labelNode = target.querySelector?.("span:not(.level-clear-mark)");
  return (labelNode?.textContent || target.textContent || "").trim();
}

function activationConfirmTargetForCommand(effect) {
  const command = effectToCommand(effect, { __sceneDef: currentSceneDef() });
  if (!command) {
    return null;
  }
  if (currentSceneHasLevelMenu() && String(command).split(":", 1)[0] === "enter") {
    return selectedLevelMenuElement();
  }
  return null;
}

function selectedLevelMenuElement() {
  return document.querySelector(".scene-layer.is-focused .level-menu li.is-selected")
    || document.querySelector(".level-menu li.is-selected");
}

function renderContainer(component, scope = {}) {
  const container = document.createElement("div");
  container.className = `view-${component.kind}`;
  applySizingKind(container, component);
  container.classList.toggle("has-ratio-content", componentContainsSizingKind(component, "ratio"));
  if (component.layout?.scroll) {
    container.classList.add("is-scroll");
  }
  applySceneLayout(container, component.layout);
  renderSurfaceComponents(component.children || [], container, {
    ...scope,
    __componentPath: [...(scope.__componentPath || []), "children"],
  });
  return container;
}

function renderConditional(component, scope = {}) {
  const fragment = document.createDocumentFragment();
  const children = isSceneConditionTrue(component.condition)
    ? component.children || []
    : component.elseChildren || [];
  renderSurfaceComponents(children, fragment, {
    ...scope,
    __componentPath: [...(scope.__componentPath || []), isSceneConditionTrue(component.condition) ? "children" : "elseChildren"],
  });
  return fragment;
}

function applySceneLayout(element, layout, options = {}) {
  if (!element || !layout) {
    return;
  }
  if (layout.size) {
    const width = Math.max(1, Number(layout.size.width) || 1);
    const height = Math.max(1, Number(layout.size.height) || 1);
    element.classList.add("has-layout-size");
    element.style.setProperty("--layout-width", String(width));
    element.style.setProperty("--layout-height", String(height));
    element.dataset.layoutWidth = String(width);
    element.dataset.layoutHeight = String(height);
    element.dataset.layoutRoot = options.root ? "true" : "false";
    element.style.aspectRatio = `${width} / ${height}`;
    if (!options.root) {
      applyLogicalElementSize(element);
    }
  }
  if (layout.gap !== undefined && layout.gap !== null) {
    element.style.gap = `calc(${Math.max(0, Number(layout.gap) || 0)} * var(--scene-layout-gap-unit))`;
  }
  const align = layout.align || {};
  applySceneAlignment(element, align);
}

function applySceneAlignment(element, align = {}) {
  const x = align.x ? sceneLayoutAlignCss(align.x) : "";
  const y = align.y ? sceneLayoutAlignCss(align.y) : "";
  if (x) {
    element.style.justifyItems = x;
  }
  if (y) {
    element.style.alignContent = y;
  }
  const isColumnFlex =
    element.classList.contains("scene-layer")
    || element.classList.contains("view-column")
    || element.classList.contains("view-box");
  const isRowFlex = element.classList.contains("view-row");
  if (isColumnFlex) {
    if (x) {
      element.style.alignItems = x;
    }
    if (y) {
      element.style.justifyContent = y;
    }
    return;
  }
  if (isRowFlex) {
    if (x) {
      element.style.justifyContent = x;
    }
    if (y) {
      element.style.alignItems = y;
    }
    return;
  }
  if (x) {
    element.style.justifyContent = x;
  }
  if (y) {
    element.style.alignItems = y;
  }
}

function syncLogicalLayoutElementSizes(unit = currentSceneLayoutUnit()) {
  document.querySelectorAll(".has-layout-size").forEach((element) => {
    applyLogicalElementSize(element, unit);
  });
}

function applyLogicalElementSize(element, unit = currentSceneLayoutUnit()) {
  if (!element || element.dataset.layoutRoot === "true") {
    return;
  }
  const width = Math.max(1, Number(element.dataset.layoutWidth) || 1);
  const height = Math.max(1, Number(element.dataset.layoutHeight) || 1);
  element.style.inlineSize = `${Math.ceil(width * unit)}px`;
  element.style.blockSize = `${Math.ceil(height * unit)}px`;
}

function currentSceneLayoutUnit() {
  const inlineValue = Number.parseFloat(screenView?.style.getPropertyValue("--scene-layout-unit") || "");
  if (Number.isFinite(inlineValue) && inlineValue > 0) {
    return inlineValue;
  }
  const computedValue = Number.parseFloat(
    getComputedStyle(screenView || document.documentElement).getPropertyValue("--scene-layout-unit") || "",
  );
  return Number.isFinite(computedValue) && computedValue > 0 ? computedValue : 1;
}

function sceneLayoutAlignCss(value) {
  if (value === "left" || value === "top") {
    return "start";
  }
  if (value === "right" || value === "bottom") {
    return "end";
  }
  return "center";
}

function renderFor(component, scope = {}) {
  const list = document.createElement("ul");
  list.className = "view-list";
  applySizingKind(list, component);

  for (const item of viewItems(component, scope)) {
    const row = document.createElement("li");
    row.classList.toggle("is-selected", item.selected === true);
    renderSurfaceComponents(component.children || [], row, {
      ...scope,
      __insideFor: true,
      __componentPath: [...(scope.__componentPath || []), "children"],
      [component.binding]: item,
    });
    if (!row.childNodes.length) {
      row.textContent = item.label || item.name || "";
    }
    list.append(row);
  }

  return list;
}

function viewItems(component, scope = {}) {
  if (component.source === "levels") {
    return sceneLevelEntries(scope.__sceneDef).map(({ level, index }, position) => ({
      kind: "level",
      index,
      position,
      num: position + 1,
      number: position + 1,
      title: level.title || level.label || level.name || `Level ${index + 1}`,
      name: level.name || `Level ${index + 1}`,
      label: level.label || level.name || `Level ${index + 1}`,
      cleared: level.cleared === true,
      solved: level.cleared === true,
      current: index === currentState.levelIndex,
      selected: index === currentState.selectedLevelIndex,
    }));
  }
  return [];
}

function sceneLevelEntries(scene = currentSceneDef()) {
  return (currentState?.levels || [])
    .map((level, index) => ({ level, index }))
    .filter(({ level }) => sceneAcceptsResource(scene, "levels", level?.name || ""));
}

function sceneAcceptsResource(scene, kind, name) {
  const resources = scene?.resources || {};
  const mode = resources[`${kind}Mode`] || "all";
  if (mode !== "named") {
    return true;
  }
  return (resources[kind] || []).some((resource) => resourceMatches(resource, name));
}

function resourceMatches(resource, name) {
  return name === resource || String(name || "").startsWith(`${resource}.`);
}

function menuInstanceComponent(scene = currentSceneDef()) {
  const components = scene?.components || [];
  return findComponent(components, (component) => component.kind === "menu");
}

function findComponent(components, predicate) {
  for (const component of components || []) {
    if (predicate(component)) {
      return component;
    }
    const found = findComponent(component.children || [], predicate);
    if (found) {
      return found;
    }
    const elseFound = findComponent(component.elseChildren || [], predicate);
    if (elseFound) {
      return elseFound;
    }
  }
  return null;
}

function resolveViewPath(path, scope = {}) {
  const parts = String(path || "").split(".").filter(Boolean);
  if (parts.length === 0) {
    return "";
  }
  if (parts.length === 1) {
    if (Object.prototype.hasOwnProperty.call(scope, parts[0])) {
      return scope[parts[0]];
    }
    if (currentState && Object.prototype.hasOwnProperty.call(currentState, parts[0])) {
      return currentState[parts[0]];
    }
    return (scope.__sceneState || currentState?.sceneState || currentState?.screenState)?.[parts[0]]
      ?? currentState?.gameState?.[parts[0]];
  }
  if (parts.length >= 3 && parts[1] === "level") {
    const puzzle = currentState?.scenePuzzleState?.[parts[0]];
    if (puzzle !== undefined) {
      let value = puzzle;
      for (const part of parts.slice(1)) {
        value = value?.[part];
      }
      return value;
    }
  }
  let value = Object.prototype.hasOwnProperty.call(scope, parts[0])
    ? scope[parts[0]]
    : currentState?.[parts[0]];
  if (value === undefined) {
  value = (scope.__sceneState || currentState?.sceneState || currentState?.screenState)?.[parts[0]];
  }
  if (value === undefined) {
    value = currentState?.gameState?.[parts[0]];
  }
  for (const part of parts.slice(1)) {
    value = value?.[part];
  }
  return value;
}

function resolveLabel(label, scope = {}) {
  if (!label) {
    return "";
  }
  if (typeof label === "string") {
    return label;
  }
  if (label.kind === "text") {
    return label.value || "";
  }
  if (label.kind === "int" || label.kind === "bool") {
    return String(label.value);
  }
  if (label.kind === "path") {
    return String(resolveViewPath(label.path, scope) ?? "");
  }
  if (label.kind === "call") {
    if (label.name === "join") {
      return (label.args || []).map((arg) => resolveLabel(arg, scope)).join("");
    }
    return exprSource(label, scope);
  }
  return "";
}

function renderLevelMenu(state, component = {}, scope = {}) {
  const list = document.createElement("ul");
  list.className = "view-list level-menu";
  list.setAttribute("role", "listbox");
  applySizingKind(list, { kind: "level_menu" });
  applySceneLayout(list, component.layout);
  const columns = Number(component.columns || 0);
  if (columns > 0) {
    list.classList.add("is-matrix");
    list.style.gridTemplateColumns = `repeat(${columns}, minmax(0, 1fr))`;
  }
  const levels = sceneLevelEntries(scope.__sceneDef);
  for (const [position, { level, index }] of levels.entries()) {
    const item = document.createElement("li");
    item.setAttribute("role", "option");
    item.dataset.levelMenuPosition = String(position);
    item.classList.toggle("is-selected", index === state.selectedLevelIndex);
    item.setAttribute("aria-selected", index === state.selectedLevelIndex ? "true" : "false");

    if (component.showCleared) {
      const cleared = document.createElement("span");
      cleared.className = "level-clear-mark";
      cleared.classList.toggle("is-cleared", level?.cleared === true);
      item.append(cleared);
    }

    const label = document.createElement("span");
    const levelName = level?.name || `Level ${index + 1}`;
    label.textContent = component.showIndex ? `${position + 1}. ${levelName}` : levelName;
    item.append(label);
    item.addEventListener("click", () => runActivationConfirm(item, () => sendCommand(`enter:${position}`)));

    list.append(item);
  }
  for (const [commandIndex, commandButton] of (component.buttons || []).entries()) {
    const position = levels.length + commandIndex;
    const index = state.levelCount + commandIndex;
    const item = document.createElement("li");
    item.className = "level-menu-button";
    item.setAttribute("role", "option");
    item.dataset.levelMenuPosition = String(position);
    item.classList.toggle("is-selected", index === state.selectedLevelIndex);
    item.setAttribute("aria-selected", index === state.selectedLevelIndex ? "true" : "false");
    if (component.showCleared) {
      const cleared = document.createElement("span");
      cleared.className = "level-clear-mark";
      item.append(cleared);
    }
    const label = document.createElement("span");
    label.textContent = resolveLabel(commandButton.label);
    item.append(label);
    item.addEventListener("click", () => runActivationConfirm(item, () => sendCommand(`enter:${position}`)));
    list.append(item);
  }
  return list;
}

function scheduleSelectedLevelMenuScroll() {
  requestAnimationFrame(scrollSelectedLevelMenuItemIntoView);
}

function scrollSelectedLevelMenuItemIntoView() {
  const item = selectedLevelMenuElement();
  const list = item?.closest(".level-menu");
  if (!item || !list) {
    return;
  }
  const itemTop = item.offsetTop;
  const itemBottom = itemTop + item.offsetHeight;
  const visibleTop = list.scrollTop;
  const visibleBottom = visibleTop + list.clientHeight;
  if (itemTop < visibleTop) {
    list.scrollTop = itemTop;
  } else if (itemBottom > visibleBottom) {
    list.scrollTop = itemBottom - list.clientHeight;
  }
}

function focusShell() {
  if (!shell) {
    return;
  }
  shell.focus({ preventScroll: true });
}

function isMessageDismissKey(event) {
  const rawKey = String(event.key || "");
  const rawCode = String(event.code || "");
  const key = normalizedKeyName(rawKey, rawCode);
  return rawKey === "Enter"
    || rawKey === " "
    || rawCode === "Enter"
    || rawCode === "Space"
    || key === "x"
    || rawCode === "KeyX";
}

function effectsForKey(event) {
  if (!currentState) {
    return [];
  }
  const effects = [];
  const rawKey = String(event.key || "");
  const rawCode = String(event.code || "");
  const key = normalizedKeyName(rawKey, rawCode);
  const keyTokens = rawKeyTokens(rawKey, rawCode);
  const scene = currentSceneDef();
  const binding = scene?.keys?.find((binding) => binding.keys.some((candidate) => keyTokens.includes(candidate)));
  if (binding) {
    effects.push(binding.effect || { kind: "command", name: binding.command });
  }

  const menu = menuInstanceComponent(scene);
  if (menu) {
    const menuInput = menuInputForKey(key, key, rawCode);
    if (menuInput) {
      effects.push({ kind: "command", name: `${menu.name}.${menuInput}` });
    }
  }

  if (currentSceneHasLevelMenu()) {
    const menuInput = menuInputForKey(key, key, rawCode);
    if (menuInput) {
      const command = {
        up: "up",
        down: "down",
        left: "left",
        right: "right",
        enter: "enter",
        back: "back",
      }[menuInput];
      if (command) {
        effects.push({ kind: "command", name: command });
      }
    }
  }

  const input = (currentState.inputs || []).find((input) =>
    keyTokens.includes(input.key)
    || keyTokens.includes(input.arrow)
    || (input.keys || []).some((candidate) => keyTokens.includes(candidate))
  );
  const standardInput = standardChoiceInputForKey(key, key, rawCode);
  if (standardInput && standardChoiceComponents(scene).length > 0) {
    effects.push({ kind: "standard_choice", input: standardInput });
  }

  if (key === "z") {
    effects.push({ kind: "command", name: "undo" });
  }
  if (key === "y") {
    effects.push({ kind: "command", name: "redo" });
  }
  if (input && (currentSceneHasPuzzle() || currentState.scene)) {
    effects.push({ kind: "command", name: input.name });
  }
  return effects;
}

function commandForKey(event) {
  return effectsForKey(event)[0] || null;
}

function normalizedKeyName(key, code = "") {
  if (key.length === 1) {
    return key.toLowerCase();
  }
  if (!key && code.startsWith("Key") && code.length === 4) {
    return code.slice(3).toLowerCase();
  }
  return key;
}

function rawKeyTokens(key, code = "") {
  const normalized = normalizedKeyName(key, code);
  return [...new Set([normalized, key, code, codeToArrowName(code), codeToLetterName(code)].filter(Boolean))];
}

function codeToArrowName(code) {
  return ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(code) ? code : "";
}

function codeToLetterName(code) {
  return code.startsWith("Key") && code.length === 4 ? code.slice(3).toLowerCase() : "";
}

function standardChoiceInputForKey(key, rawKey, code = "") {
  if (key === "w" || rawKey === "ArrowUp" || code === "ArrowUp") {
    return "up";
  }
  if (key === "s" || rawKey === "ArrowDown" || code === "ArrowDown") {
    return "down";
  }
  if (key === "a" || rawKey === "ArrowLeft" || code === "ArrowLeft") {
    return "left";
  }
  if (key === "d" || rawKey === "ArrowRight" || code === "ArrowRight") {
    return "right";
  }
  if (isStandardMenuConfirmKey(key, rawKey, code)) {
    return "enter";
  }
  return null;
}

function isStandardMenuConfirmKey(key, rawKey, code = "") {
  return rawKey === "Enter"
    || rawKey === " "
    || code === "Enter"
    || code === "Space"
    || (document.body.classList.contains("theme-puzzlescript") && (key === "x" || code === "KeyX"));
}

function menuInputForKey(key, rawKey, code = "") {
  if (key === "w" || rawKey === "ArrowUp" || code === "ArrowUp") {
    return "up";
  }
  if (key === "s" || rawKey === "ArrowDown" || code === "ArrowDown") {
    return "down";
  }
  if (key === "a" || rawKey === "ArrowLeft" || code === "ArrowLeft") {
    return "left";
  }
  if (key === "d" || rawKey === "ArrowRight" || code === "ArrowRight") {
    return "right";
  }
  if (rawKey === "Enter" || rawKey === " " || code === "Enter" || code === "Space") {
    return "enter";
  }
  if (rawKey === "Escape" || code === "Escape" || key === "q") {
    return "back";
  }
  return null;
}

function inputByName(name) {
  if (!currentState) {
    return null;
  }
  return currentState.inputs.find((input) => input.name === name);
}

function currentSceneDef() {
  const source = currentState || window.PuzzleExport || {};
  const name = source.currentScene || source.screen || "playing";
  const scenes = source.scenes || source.screens || [];
  return scenes.find((scene) => scene.name === name) || null;
}

function sceneDefByName(name) {
  const source = currentState || window.PuzzleExport || {};
  const scenes = source.scenes || source.screens || [];
  return scenes.find((scene) => scene.name === name) || null;
}

function isSceneConditionTrue(condition) {
  return String(condition || "")
    .split(" and ")
    .every((part) => isSceneConditionAtomTrue(part.trim()));
}

function isSceneConditionAtomTrue(condition) {
  const equalMatch = String(condition || "").match(/^(.+?)\s*==\s*(.+)$/);
  if (equalMatch) {
    const left = sceneConditionValue(equalMatch[1].trim());
    const right = sceneConditionValue(equalMatch[2].trim());
    return left !== undefined && right !== undefined && left === right;
  }
  const notEqualMatch = String(condition || "").match(/^(.+?)\s*!=\s*(.+)$/);
  if (notEqualMatch) {
    const left = sceneConditionValue(notEqualMatch[1].trim());
    const right = sceneConditionValue(notEqualMatch[2].trim());
    return left !== undefined && right !== undefined && left !== right;
  }
  const levelValue = resolveViewPath(condition);
  if (typeof levelValue === "boolean") {
    return levelValue;
  }
  return false;
}

function sceneConditionValue(value) {
  if (value === "true" || value === "false") {
    return value;
  }
  const levelValue = resolveViewPath(value);
  if (levelValue !== undefined && levelValue !== null && typeof levelValue !== "object") {
    return String(levelValue);
  }
  if (/^-?\d+$/.test(String(value))) {
    return String(Number(value));
  }
  const quoted = String(value).match(/^"(.*)"$/);
  if (quoted) {
    return quoted[1].replace(/\\"/g, "\"");
  }
  if (/^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*$/.test(String(value))) {
    return String(value);
  }
  return undefined;
}

async function sendEffect(effect, scope = {}) {
  if (effect?.kind === "standard_choice") {
    handleStandardChoiceInput(effect.input);
    return;
  }
  if (effect?.kind === "wait") {
    await waitForEffect(effect);
    return;
  }
  if (effect?.kind === "sequence") {
    for (const child of effect.effects || []) {
      await sendEffect(child.effect || child, scope);
    }
    return;
  }
  if (effect?.kind === "conditional") {
    if (isSceneConditionTrue(effect.condition)) {
      await sendEffect(effect.effect?.effect || effect.effect, scope);
    }
    return;
  }
  const command = effectToCommand(effect, scope);
  if (command) {
    await sendCommand(command);
  }
}

function waitForEffect(effect) {
  const milliseconds = Math.max(0, Number(effect?.milliseconds || effect?.ms || 0));
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function applyWaitEvents(events) {
  for (const event of events || []) {
    if (event.kind !== "wait" && event.kind !== "continue_effects") {
      continue;
    }
    clientPendingWaits += 1;
    if (currentState) {
      currentState.busy = true;
    }
    setTimeout(() => {
      clientPendingWaits = Math.max(0, clientPendingWaits - 1);
      if (currentState) {
        currentState.busy = clientPendingWaits > 0;
      }
      if (event.kind === "continue_effects") {
        sendCommandNow("__continue_effects").then(drainQueuedCommands);
      } else {
        drainQueuedCommands();
      }
    }, Math.max(0, Number(event.milliseconds || event.ms || 0)));
  }
}

function sendCommand(command) {
  if (currentState?.busy || clientPendingWaits > 0) {
    pendingCommandQueue.push(command);
    return undefined;
  }
  return sendCommandNow(command);
}

async function sendCommandNow(command) {
  if (sendPuzzle3Command(command)) {
    return undefined;
  }
  if (applyStandaloneEditorInput(command)) {
    return undefined;
  }
  if (currentSceneHasLevelMenu() && isLevelMenuCommandName(command)) {
    return post(`/api/command/${encodeURIComponent(command)}`);
  }
  if (currentState?.inputs?.some((input) => input.name === command)) {
    return post(`/api/input/${encodeURIComponent(command)}`);
  }
  return post(`/api/command/${encodeURIComponent(command)}`);
}

async function drainQueuedCommands() {
  if (drainingCommandQueue || clientPendingWaits > 0 || currentState?.busy) {
    return;
  }
  drainingCommandQueue = true;
  try {
    while (pendingCommandQueue.length > 0 && clientPendingWaits === 0 && !currentState?.busy) {
      const command = pendingCommandQueue.shift();
      await sendCommandNow(command);
    }
  } finally {
    drainingCommandQueue = false;
  }
}

function sendPuzzle3Command(command) {
  const parsed = parsePuzzle3Command(command);
  if (!parsed) {
    return false;
  }
  const frame = puzzle3FrameForTarget(parsed.target);
  if (!frame?.contentWindow) {
    return false;
  }
  frame.contentWindow.postMessage({
    type: "PuzzleStudioCommand",
    command: parsed.command,
    level: parsed.level,
  }, "*");
  return true;
}

function parsePuzzle3Command(command) {
  const text = String(command || "").trim();
  const match = text.match(/^([A-Za-z_][A-Za-z0-9_.]*)\.(restart|reset_camera|next_level|previous_level|goto|goto_level)(?:\s+(.+))?$/);
  if (!match) {
    return null;
  }
  const [, target, rawCommand, level] = match;
  const commandName = rawCommand === "goto" ? "goto_level" : rawCommand;
  return { target, command: commandName, level };
}

function puzzle3FrameForTarget(target) {
  const targetName = String(target || "").split(".").pop();
  const frames = [...document.querySelectorAll("iframe.puzzle3-frame")];
  return frames.find((frame) => frame.dataset.source === targetName)
    || frames.find((frame) => frame.dataset.source === target)
    || null;
}

function applyStandaloneEditorInput(command) {
  const acceptsEditorInput = standaloneRuntime?.editorPreviewInputEnabled
    || (standaloneRuntime?.editorPreviewSceneEnabled && currentState?.scene);
  if (!acceptsEditorInput || !standaloneRuntime?.inputIdsByName?.has(command)) {
    return false;
  }
  try {
    standaloneRuntime.applyInputName(command);
    render(standaloneRuntime.snapshot());
  } catch (error) {
    showError(error);
  }
  return true;
}

function isLevelMenuCommandName(command) {
  const name = String(command || "").split(":", 1)[0];
  return [
    "up",
    "down",
    "left",
    "right",
    "enter",
    "back",
  ].includes(name);
}

function standardChoiceComponents(scene = currentSceneDef()) {
  return standardChoiceFocusCells(scene).map((cell) => cell.component);
}

function standardChoiceFocusCells(scene = currentSceneDef()) {
  if (!scene) {
    return [];
  }
  const footprint = componentColumnFootprint(scene.components || [], { insideFor: false });
  return footprint.cells.map((cell, index) => ({ ...cell, index }));
}

function componentFootprint(component, context = {}) {
  if (!component) {
    return emptyFootprint();
  }
  if (component.kind === "choice" && !context.insideFor) {
    return {
      width: 1,
      height: 1,
      cells: [{ x: 0, y: 0, component }],
    };
  }
  if (["title", "subtitle", "text", "frame", "puzzle", "puzzle3"].includes(component.kind)) {
    return emptyCellFootprint();
  }
  if (component.kind === "row") {
    return componentRowFootprint(component.children || [], context);
  }
  if (component.kind === "column" || component.kind === "box") {
    return componentColumnFootprint(component.children || [], context);
  }
  if (component.kind === "conditional") {
    return componentColumnFootprint(
      isSceneConditionTrue(component.condition)
        ? component.children || []
        : component.elseChildren || [],
      context,
    );
  }
  if (component.kind === "for") {
    return componentColumnFootprint(component.children || [], { ...context, insideFor: true });
  }
  return emptyCellFootprint();
}

function componentRowFootprint(components, context = {}) {
  let width = 0;
  let height = 0;
  const cells = [];
  for (const component of components || []) {
    const child = componentFootprint(component, context);
    for (const cell of child.cells) {
      cells.push({ ...cell, x: cell.x + width });
    }
    width += child.width;
    height = Math.max(height, child.height);
  }
  return {
    width: Math.max(1, width),
    height: Math.max(1, height),
    cells,
  };
}

function componentColumnFootprint(components, context = {}) {
  let width = 0;
  let height = 0;
  const cells = [];
  for (const component of components || []) {
    const child = componentFootprint(component, context);
    for (const cell of child.cells) {
      cells.push({ ...cell, y: cell.y + height });
    }
    width = Math.max(width, child.width);
    height += child.height;
  }
  return {
    width: Math.max(1, width),
    height: Math.max(1, height),
    cells,
  };
}

function emptyFootprint() {
  return { width: 0, height: 0, cells: [] };
}

function emptyCellFootprint() {
  return { width: 1, height: 1, cells: [] };
}

function standardChoiceCursor(scene = currentSceneDef()) {
  const cells = standardChoiceFocusCells(scene);
  const max = Math.max(0, cells.length - 1);
  const cursor = Number(standardChoiceCursors.get(scene?.name) || 0);
  return Math.max(0, Math.min(max, cursor));
}

function handleStandardChoiceInput(input) {
  const scene = currentSceneDef();
  const cells = standardChoiceFocusCells(scene);
  if (!scene || cells.length === 0) {
    return;
  }
  const cursor = standardChoiceCursor(scene);
  if (["up", "down", "left", "right"].includes(input)) {
    const next = standardChoiceDirectionalTarget(cells, cursor, input);
    if (next !== null) {
      standardChoiceCursors.set(scene.name, next);
      render(currentState);
    }
    return;
  }
  if (input === "enter") {
    const selectedChoice = document.querySelector(".standard-choice.is-selected");
    runEffectActivationConfirm(selectedChoice, cells[cursor]?.component?.effect || null, { __sceneDef: scene });
  }
}

function standardChoiceDirectionalTarget(cells, cursor, direction) {
  const current = cells[cursor];
  if (!current) {
    return null;
  }
  const candidates = cells.filter((cell) => {
    if (direction === "left") {
      return cell.y === current.y && cell.x < current.x;
    }
    if (direction === "right") {
      return cell.y === current.y && cell.x > current.x;
    }
    if (direction === "up") {
      return cell.x === current.x && cell.y < current.y;
    }
    if (direction === "down") {
      return cell.x === current.x && cell.y > current.y;
    }
    return false;
  });
  if (candidates.length === 0) {
    return null;
  }
  candidates.sort((left, right) => {
    if (direction === "left") {
      return right.x - left.x;
    }
    if (direction === "right") {
      return left.x - right.x;
    }
    if (direction === "up") {
      return right.y - left.y;
    }
    return left.y - right.y;
  });
  return candidates[0].index;
}

function effectLabel(effect) {
  if (!effect) {
    return "";
  }
  if (typeof effect === "string") {
    return effect;
  }
  if (effect.kind === "command" || effect.kind === "input" || effect.kind === "component_effect") {
    return effect.name;
  }
  if (effect.kind === "message") {
    return "message";
  }
  if (effect.kind === "wait") {
    return "wait";
  }
  if (effect.kind === "conditional") {
    return effectLabel(effect.effect?.effect || effect.effect);
  }
  if (effect.kind === "play_sfx" || effect.kind === "play_music" || effect.kind === "pause_music" || effect.kind === "resume_music") {
    return effect.name;
  }
  if (effect.kind === "stop_music") {
    return "stop music";
  }
  if (["goto", "enter", "create", "reset", "delete", "show", "hide", "toggle", "focus"].includes(effect.kind)) {
    return effectCommandName(effect);
  }
  if (effect.kind === "start_level") {
    return `goto ${effect.scene}`;
  }
  if (effect.kind === "continue_level") {
    return `goto ${effect.scene}`;
  }
  if (effect.kind === "puzzle_next_level") {
    return `${effect.target}.next_level`;
  }
  if (effect.kind === "puzzle_previous_level") {
    return `${effect.target}.previous_level`;
  }
  if (effect.kind === "puzzle_reset") {
    return `${effect.target}.restart`;
  }
  if (effect.kind === "puzzle_goto_level") {
    return `${effect.target}.goto`;
  }
  if (effect.kind === "back") {
    return "back";
  }
  return "";
}

function effectToCommand(effect, scope = {}) {
  if (!effect) {
    return "";
  }
  if (typeof effect === "string") {
    return effect;
  }
  if (effect.kind === "command" || effect.kind === "input" || effect.kind === "component_effect") {
    return commandWithScope(effect.name, scope);
  }
  if (effect.kind === "message") {
    return `message ${exprSource(effect.text, scope)}`.trim();
  }
  if (effect.kind === "wait") {
    return "";
  }
  if (effect.kind === "sequence") {
    const commands = (effect.effects || [])
      .map((child) => effectToCommand(child.effect || child, scope))
      .filter(Boolean);
    return commands.at(-1) || "";
  }
  if (effect.kind === "conditional") {
    return isSceneConditionTrue(effect.condition)
      ? effectToCommand(effect.effect?.effect || effect.effect, scope)
      : "";
  }
  if (effect.kind === "play_sfx") {
    return `sfx ${effect.name || ""}`.trim();
  }
  if (effect.kind === "play_music") {
    return `play_music ${effect.name || ""}`.trim();
  }
  if (effect.kind === "pause_music") {
    return effect.name ? `pause_music ${effect.name}` : "pause_music";
  }
  if (effect.kind === "resume_music") {
    return effect.name ? `resume_music ${effect.name}` : "resume_music";
  }
  if (effect.kind === "stop_music") {
    return effect.name ? `stop_music ${effect.name}` : "stop_music";
  }
  if (["goto", "enter", "create", "reset", "delete", "show", "hide", "toggle", "focus"].includes(effect.kind)) {
    return effectCommand(effect, scope);
  }
  if (effect.kind === "start_level") {
    return `goto ${effect.scene}`;
  }
  if (effect.kind === "continue_level") {
    return `goto ${effect.scene}`;
  }
  if (effect.kind === "puzzle_next_level") {
    return `${effect.target}.next_level`;
  }
  if (effect.kind === "puzzle_previous_level") {
    return `${effect.target}.previous_level`;
  }
  if (effect.kind === "puzzle_reset") {
    return `${effect.target}.restart`;
  }
  if (effect.kind === "puzzle_goto_level") {
    return `${effect.target}.goto ${effectValueToCommand(effect.level, scope)}`;
  }
  if (effect.kind === "back") {
    return "back";
  }
  return "";
}

function effectCommandName(effect) {
  return effect?.scene || effect?.screen || "";
}

function effectValueToCommand(value, scope = {}) {
  if (!value) {
    return "";
  }
  if (value.kind === "bool" || value.kind === "int") {
    return String(value.value);
  }
  if (value.kind === "text") {
    return value.value || "";
  }
  if (value.kind === "path") {
    return commandWithScope(value.path || "", scope);
  }
  return "";
}

function effectCommand(effect, scope = {}) {
  const screen = effect?.scene || effect?.screen || "";
  if (!screen) {
    return "";
  }
  if ((effect.kind === "goto" || effect.kind === "enter") && (effect.params || []).length === 1 && effect.params[0]?.name === "level") {
    return `${effect.kind} ${screen}(${exprSource(effect.params[0].value, scope)})`;
  }
  const params = (effect.params || []).map((param) => `${param.name} = ${exprSource(param.value, scope)}`);
  const base = `${effect.kind} ${screen}`;
  return params.length ? `${base} with ${params.join(", ")}` : base;
}

function exprSource(expr, scope = {}) {
  if (!expr) {
    return "";
  }
  if (expr.kind === "text") {
    return JSON.stringify(expr.value || "");
  }
  if (expr.kind === "int" || expr.kind === "bool") {
    return String(expr.value);
  }
  if (expr.kind === "path") {
    const resolved = resolveViewPath(expr.path, scope);
    return resolved === undefined || resolved === null ? expr.path : exprValueSource(resolved);
  }
  if (expr.kind === "call") {
    return `${expr.name}(${(expr.args || []).map((arg) => exprSource(arg, scope)).join(", ")})`;
  }
  return "";
}

function exprValueSource(value) {
  if (typeof value === "object" && value?.kind === "level" && value.name !== undefined) {
    return JSON.stringify(value.name);
  }
  return commandPayload(value);
}

function commandWithScope(command, scope = {}) {
  const [name, payload] = String(command || "").split(":", 2);
  if (!payload) {
    return command;
  }
  const scoped = resolveViewPath(payload, scope);
  if (scoped === undefined || scoped === null) {
    return command;
  }
  return `${name}:${commandPayload(scoped)}`;
}

function commandPayload(value) {
  if (typeof value === "object") {
    if (value.index !== undefined) {
      return String(value.index);
    }
    if (value.name !== undefined) {
      return String(value.name);
    }
    if (value.label !== undefined) {
      return String(value.label);
    }
  }
  return String(value);
}

document.addEventListener("keydown", (event) => {
  if (componentEmbedMode) {
    return;
  }
  if (messagePopup) {
    event.preventDefault();
    if (isMessageDismissKey(event)) {
      closeMessagePopup();
    }
    return;
  }

  if (!currentState) {
    return;
  }
  broadcastPuzzle3Key(event, "down");

  const effects = effectsForKey(event);
  if (effects.length > 0) {
    event.preventDefault();
    for (const effect of effects) {
      const confirmTarget = activationConfirmTargetForCommand(effect);
      if (confirmTarget) {
        runEffectActivationConfirm(confirmTarget, effect);
      } else {
        sendEffect(effect);
      }
    }
    return;
  }
});

document.addEventListener("keyup", (event) => {
  if (componentEmbedMode) {
    return;
  }
  broadcastPuzzle3Key(event, "up");
});

function broadcastPuzzle3Key(event, action = "down") {
  if (!currentSceneHasPuzzle3()) {
    return;
  }
  for (const frame of document.querySelectorAll("iframe.puzzle3-frame")) {
    frame.contentWindow?.postMessage({
      type: "PuzzleStudioKey",
      action,
      key: String(event.key || ""),
      code: String(event.code || ""),
    }, "*");
  }
}

if (standaloneRuntime) {
  window.addEventListener("PuzzleStandaloneStateChanged", () => {
    loadState().catch((error) => {
      showError(error);
    });
  });
}

window.addEventListener("message", async (event) => {
  if (event.data?.type === "PuzzleStudioSetScenePreview") {
    renderSceneEditorPreview(event.data || {});
    return;
  }

  if (event.data?.type === "PuzzleStudioRequestScenePreview") {
    notifySceneEditorPreview(String(event.data.requestId || ""));
    return;
  }

  if (event.data?.type === "PuzzleStudioInput") {
    if (standaloneRuntime && event.data.input) {
      sendCommand(String(event.data.input));
    }
    return;
  }

  if (event.data?.type === "PuzzleStudioSetState") {
    if (standaloneRuntime && event.data.state) {
      standaloneRuntime.setCurrentState(event.data.state, {
        levelIndex: event.data.levelIndex,
        regions: event.data.regions,
        acceptModelInput: event.data.acceptModelInput === true,
        materializeLevelStart: event.data.materializeLevelStart === true,
        materializeDisplay: event.data.materializeDisplay === true,
        materializeTurnStart: event.data.materializeTurnStart === true,
      });
      if (event.data.silent === true) {
        notifyPreviewState(standaloneRuntime.snapshot());
      } else {
        loadState();
      }
    }
    return;
  }

  if (event.data?.type === "PuzzleStudioKey") {
    if (currentState?.busy) {
      return;
    }
    const keyEvent = {
      key: String(event.data.key || ""),
      code: String(event.data.code || ""),
    };
    broadcastPuzzle3Key(keyEvent);
    for (const effect of effectsForKey(keyEvent)) {
      sendEffect(effect);
    }
    return;
  }

  if (event.data?.type !== "PuzzleStudioCommand") {
    return;
  }
  const command = String(event.data.command || "");
  if (command) {
    sendCommand(command);
  }
});

playSurface.addEventListener("pointerdown", (event) => {
  if (!currentState || currentState.busy || !currentSceneHasPuzzle()) {
    return;
  }
  swipeStart = { x: event.clientX, y: event.clientY, pointerId: event.pointerId };
  playSurface.setPointerCapture(event.pointerId);
});

playSurface.addEventListener("pointerup", (event) => {
  if (!swipeStart || swipeStart.pointerId !== event.pointerId) {
    return;
  }

  const dx = event.clientX - swipeStart.x;
  const dy = event.clientY - swipeStart.y;
  swipeStart = null;

  const threshold = 24;
  if (Math.max(Math.abs(dx), Math.abs(dy)) < threshold) {
    return;
  }

  const inputName = Math.abs(dx) > Math.abs(dy)
    ? (dx > 0 ? "right" : "left")
    : (dy > 0 ? "down" : "up");
  const input = inputByName(inputName);
  if (input) {
    sendCommand(input.name);
  }
});

playSurface.addEventListener("pointercancel", () => {
  swipeStart = null;
});

loadState().catch((error) => {
  showError(error);
});

if (!componentEmbedMode) {
  document.addEventListener("DOMContentLoaded", focusShell);
  document.addEventListener("pointerdown", focusShell);
  window.addEventListener("resize", () => scheduleScreenScaleSync(2));
  window.addEventListener("load", () => {
    scheduleScreenScaleSync(3);
    focusShell();
    requestAnimationFrame(focusShell);
    setTimeout(focusShell, 0);
  });
  window.addEventListener("focus", focusShell);
  document.fonts?.ready.then(() => scheduleScreenScaleSync(2)).catch(() => {});
}

function showError(error) {
  console.error(error);
}
