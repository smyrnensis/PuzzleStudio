const board = document.querySelector("#board");
const screenView = document.querySelector("#screenView");
const screenFrame = document.querySelector("#screenFrame") || screenView?.parentElement;
const playSurface = document.querySelector(".play-surface");
const shell = document.querySelector("#shell");
const componentEmbedMode = new URLSearchParams(window.location.search).get("component") === "1";
let initialPuzzle3PreviewSurface = null;
/* puzzle-host:optional:puzzle3:start */
const PREVIEW_SURFACE_UPDATE_MESSAGE = "PuzzleStudioPreviewSurfaceUpdate";
const PUZZLE3_LEVEL_PREVIEW_KIND = "puzzle3-level";
const ISOLATED_PREVIEW_MODE = "isolated";
const PUZZLE3_MODEL_COMPONENT_PREVIEW_MESSAGE = "PuzzleStudioRenderPuzzle3ModelComponent";
initialPuzzle3PreviewSurface = normalizePuzzle3PreviewSurface(
  window.PuzzleStudioInitialPreviewSurface || window.PuzzleStudioInitialModelComponentPreview,
);
if (initialPuzzle3PreviewSurface) {
  window.PuzzleStudioInitialPreviewSurfaceConsumed = true;
  window.PuzzleStudioInitialModelComponentPreviewConsumed = true;
}
/* puzzle-host:optional:puzzle3:end */
document.documentElement.classList.toggle("is-component-embed", componentEmbedMode || Boolean(initialPuzzle3PreviewSurface));
document.body.classList.toggle("is-component-embed", componentEmbedMode || Boolean(initialPuzzle3PreviewSurface));
let clientPendingWaits = 0;
let activeThemeClass = "";
const activeThemeVariables = new Set();
const activationConfirmDelayMs = 160;
const STANDARD_COMPONENT_DEFINITIONS = Object.freeze([Object.freeze({
  name: "standard.message",
  layout: {},
  events: Object.freeze({
    dismiss: Object.freeze({ pointer: true, keys: "input" }),
  }),
  components: Object.freeze([Object.freeze({
    kind: "text",
    role: "body",
    source: "path",
    path: Object.freeze(["text"]),
  })]),
})]);

class PuzzleSoundRuntime {
  constructor() {
    this.sounds = { sfx: [], music: [] };
    this.context = null;
    this.activeMusic = new Map();
    this.pausedMusic = new Map();
    this.visibilityPausedMusic = new Map();
    this.activeSfx = new Map();
    this.sfxEffectCache = new Map();
    this.sfxEffectApi = null;
    this.soundWarnings = new Set();
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

  primePlayback() {
    this.ensureContext();
  }

  playSfx(name) {
    if (this.shouldSuppressPlayback()) {
      return;
    }
    const def = this.sfxDef(name);
    const context = this.ensureContext();
    if (!def || !context) {
      return;
    }
    const api = window.PuzzleSoundGenerator || window.PuzzleSoundTools || null;
    const volume = Number(def.volume ?? 1);
    if (api?.generateSoundEffect && api?.createSfxPlayer) {
      try {
        const effect = this.sfxEffect(api, def);
        const player = api.createSfxPlayer(context, effect, { volume });
        this.replaceActiveSfx(name, player);
        player.start(context.currentTime);
      } catch (error) {
        this.warnSoundIssue(`sfx:${name}:${def.type || "random"}`, `Sound effect "${name}" was skipped: ${error?.message || error}`);
      }
      return;
    }
    this.warnSoundIssue(`sfx:${name}`, `Sound effect "${name}" was skipped because the sound generator is unavailable.`);
  }

  replaceActiveSfx(name, player) {
    this.activeSfx.get(name)?.stop();
    this.activeSfx.set(name, player);
  }

  sfxDef(name) {
    return (this.sounds.sfx || []).find((entry) => entry.name === name);
  }

  sfxEffect(api, def) {
    if (this.sfxEffectApi !== api) {
      this.sfxEffectApi = api;
      this.sfxEffectCache.clear();
    }
    const type = def.type || "random";
    const key = `${String(def.seed)}\u0000${type}`;
    let effect = this.sfxEffectCache.get(key);
    if (!effect) {
      effect = api.generateSoundEffect(def.seed, { type });
      this.sfxEffectCache.set(key, effect);
    }
    return effect;
  }

  playMusic(name, resume = {}) {
    if (this.shouldSuppressPlayback()) {
      return;
    }
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
        height: Number(def.height ?? def.tone ?? 0.5),
        bars: Number(def.bars || 8),
        bpm: Number(def.bpm || 110),
        volume: Number(def.volume ?? 0.5),
      });
      const player = api.createPlayer(context, song.playbackScore);
      const handle = { player, progress };
      this.activeMusic.set(name, handle);
      player.start(progress);
      return;
    }
    this.warnSoundIssue(`music:${name}`, `Music "${name}" was skipped because the sound generator is unavailable.`);
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

  pauseForHiddenDocument() {
    for (const [key, handle] of [...this.activeMusic.entries()]) {
      const progress = handle.player?.loopProgress?.() ?? handle.progress ?? 0;
      this.stopMusicHandle(handle);
      this.activeMusic.delete(key);
      this.visibilityPausedMusic.set(key, {
        progress,
      });
    }
  }

  resumeAfterVisibleDocument() {
    if (this.shouldSuppressPlayback()) {
      return;
    }
    const entries = [...this.visibilityPausedMusic.entries()];
    this.visibilityPausedMusic.clear();
    for (const [key, paused] of entries) {
      if (!this.pausedMusic.has(key)) {
        this.playMusic(key, paused);
      }
    }
  }

  shouldSuppressPlayback() {
    return typeof document !== "undefined" && document.visibilityState === "hidden";
  }

  stopMusicHandle(handle) {
    if (handle.player) {
      try {
        handle.progress = handle.player.loopProgress?.() ?? handle.progress ?? 0;
        handle.player.stop();
      } catch (_) {
      }
    }
  }

  warnSoundIssue(key, message) {
    if (this.soundWarnings.has(key)) {
      return;
    }
    this.soundWarnings.add(key);
    console.warn(message);
  }

}

const puzzleBoot = window.PuzzleBoot || {};
const standaloneRuntime = window.PuzzleStandaloneRuntime
  ? new window.PuzzleStandaloneRuntime(puzzleBoot, window.PuzzleRuntimeExportJson)
  : null;
const soundRuntime = new PuzzleSoundRuntime();

document.addEventListener("keydown", () => soundRuntime.primePlayback(), { capture: true });
document.addEventListener("pointerdown", () => soundRuntime.primePlayback(), { capture: true });

let currentState = null;
let swipeStart = null;
const puzzleViewports = new Map();
/* puzzle-host:optional:puzzle3:start */
const puzzle3Controllers = new Map();
let puzzle3PreviewSurface = initialPuzzle3PreviewSurface;
/* puzzle-host:optional:puzzle3:end */
/* puzzle-host:optional:solver:start */
const activeSolveRequests = new Map();
let wasmSolverServicePromise = null;
/* puzzle-host:optional:solver:end */
const standardChoiceCursors = new Map();
let screenScaleSyncFrame = 0;
let screenScaleSyncPasses = 0;
let pendingModelInput = null;
const activeWaitTimers = new Set();
const pendingPresentationEvents = [];
let presentationAnimationBatchId = 0;
let dispatchingPresentationEvents = false;
let pendingSessionResume = false;
let resumingSession = false;
let sessionWaiting = false;
let drainingQueuedModelInput = false;
let sceneEditorPreview = null;

function sendHostModelInput(_input) {
  return false;
}

async function requestJson(url, options = {}) {
  if (standaloneRuntime) {
    return standaloneRuntime.requestJson(url, options);
  }
  if (puzzleBoot.editorPreview === true) {
    throw new Error("Editor preview requires its WASM session runtime; /api requests are unavailable in the preview iframe.");
  }
  const response = await fetch(url, options);
  const body = await response.json();
  if (!response.ok) {
    throw new Error(body.error || response.statusText);
  }
  return body;
}

/* puzzle-host:optional:solver:start */
async function loadWasmSolver() {
  if (!wasmSolverServicePromise) {
    wasmSolverServicePromise = import("./wasm/puzzle_wasm.js")
      .then(async (module) => {
        await module.default({ module_or_path: "./wasm/puzzle_wasm_bg.wasm" });
        if (typeof module.WasmSolverService !== "function") {
          throw new Error("Solver service is not available");
        }
        const service = new module.WasmSolverService();
        const puzzlePath = String(puzzleBoot.puzzlePath || "").trim();
        if (!puzzlePath) {
          throw new Error("Standalone solver requires an explicit puzzle path");
        }
        const prepared = service.prepare_source(
          puzzleBoot.source,
          puzzlePath,
          Date.now(),
        );
        service.pin_artifact(prepared.artifactId, Date.now());
        return { service, prepared };
      })
      .catch((error) => {
        wasmSolverServicePromise = null;
        throw error;
      });
  }
  return wasmSolverServicePromise;
}

async function solveStandaloneCurrentState(options = {}, control = null) {
  if (!standaloneRuntime) {
    if (puzzleBoot.editorPreview === true) {
      throw new Error("Editor preview requires its WASM session runtime; /api requests are unavailable in the preview iframe.");
    }
    return requestJson("/api/solve", { method: "POST" });
  }
  if (!puzzleBoot.source) {
    throw new Error("standalone solve requires PuzzleBoot.source");
  }
  const { service, prepared } = await loadWasmSolver();
  const state = currentState;
  if (!state?.solverState) {
    throw new Error("Standalone session snapshot is missing solverState");
  }
  const searchId = service.start(prepared.artifactId, {
    levelIndex: Number(options.levelIndex ?? state.levelIndex),
    state: state.solverState,
    materializeLevelStart: options.materializeLevelStart !== false,
    maxDepth: Number(options.maxDepth ?? 512),
    maxNodes: Number(options.maxNodes ?? 5_000_000),
  }, Date.now());
  if (control) {
    control.searchId = searchId;
    control.service = service;
  }
  for (;;) {
    if (control?.cancelled) {
      service.cancel(searchId, Date.now());
      return { result: "cancelled" };
    }
    const response = service.advance(searchId, 64, Date.now());
    if (response.status !== "paused") {
      return response.result;
    }
    await new Promise((resolve) => window.setTimeout(resolve, 0));
  }
}
/* puzzle-host:optional:solver:end */

async function loadState() {
  render(await requestJson("/api/state"));
}

async function post(url, options = {}) {
  try {
    const nextState = await requestJson(url, { ...options, method: "POST" });
    render(nextState);
  } catch (error) {
    showError(error);
  }
}

function postSessionAction(action) {
  return post("/api/action", {
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(action),
  });
}

function render(state) {
  currentState = state;
  sessionWaiting = state?.busy === true;
  window.__PuzzleCurrentState = state;
  applyTheme(state?.theme || puzzleBoot.theme || null);
  soundRuntime.configure(state?.sounds || puzzleBoot.sounds || { sfx: [], music: [] });
  const displayError = firstDisplayError(state);
  if (displayError) {
    showError(new Error(displayError));
    notifyPreviewState(state);
    return;
  }
  const presentationEvents = state?.presentationEvents || [];
  if (state) {
    state.busy = state.busy === true || clientPendingWaits > 0;
    state.presentationEvents = [];
  }
  renderSurface(state);
  scheduleScreenScaleSync(3);
  notifyPreviewState(state);
  applyPresentationEvents(presentationEvents);
}

function firstDisplayError(state) {
  if (state?.scene?.displayError) {
    return String(state.scene.displayError);
  }
  for (const layer of state?.surface?.components || []) {
    if (layer?.scene?.displayError) {
      return String(layer.scene.displayError);
    }
  }
  return "";
}

function scheduleScreenScaleSync(passes = 2) {
  if (componentEmbedMode || !screenFrame || !screenView || !playSurface) {
    return;
  }
  screenScaleSyncPasses = Math.max(screenScaleSyncPasses, Math.max(1, Math.trunc(Number(passes) || 1)));
  clampScreenScaleToFrame();
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

function clampScreenScaleToFrame() {
  if (componentEmbedMode || !screenFrame || !screenView) {
    return;
  }
  const virtualWidth = Number.parseFloat(screenView.style.getPropertyValue("--screen-virtual-width") || "");
  const virtualHeight = Number.parseFloat(screenView.style.getPropertyValue("--screen-virtual-height") || "");
  const currentScale = Number.parseFloat(screenView.style.getPropertyValue("--screen-scale") || "");
  if (!(virtualWidth > 0) || !(virtualHeight > 0) || !(currentScale > 0)) {
    return;
  }
  const frame = screenFrame.getBoundingClientRect();
  if (!(frame.width > 0) || !(frame.height > 0)) {
    return;
  }
  const nextScale = Math.max(
    0.0001,
    Math.min(currentScale, frame.width / virtualWidth, frame.height / virtualHeight),
  );
  if (nextScale < currentScale) {
    screenView.style.setProperty("--screen-scale", nextScale.toFixed(6));
  }
}

function installScreenScaleResizeHooks() {
  if (!screenFrame || !screenView || !playSurface || !shell) {
    return;
  }
  if (typeof ResizeObserver !== "function") {
    throw new Error("PuzzleStudio HTML play requires ResizeObserver for responsive screen scaling.");
  }
  const resizeObserver = new ResizeObserver(() => scheduleScreenScaleSync(4));
  resizeObserver.observe(shell);
  resizeObserver.observe(playSurface);
  window.addEventListener("resize", () => scheduleScreenScaleSync(4));
  window.addEventListener("orientationchange", () => scheduleScreenScaleSync(6));
  window.addEventListener("pageshow", () => scheduleScreenScaleSync(4));
  document.addEventListener("fullscreenchange", () => scheduleScreenScaleSync(6));
  window.visualViewport?.addEventListener("resize", () => scheduleScreenScaleSync(6));
  window.addEventListener("load", () => {
    scheduleScreenScaleSync(4);
  });
  document.fonts?.ready.then(() => scheduleScreenScaleSync(3)).catch(() => {});
}

function syncScreenScale() {
  if (componentEmbedMode || !screenFrame || !screenView || !playSurface) {
    return;
  }
  if (screenView.getClientRects().length === 0 || playSurface.getClientRects().length === 0) {
    return;
  }
  const available = elementContentBox(playSurface);
  if (available.width <= 0 || available.height <= 0) {
    return;
  }
  const viewport = fitSceneViewport(available, currentSceneAspectRatio());
  screenView.style.setProperty("--screen-virtual-width", `${viewport.width}px`);
  screenView.style.setProperty("--screen-virtual-height", `${viewport.height}px`);
  screenView.style.setProperty("--screen-scale", "1");
  screenFrame.style.width = `min(${Math.ceil(viewport.width)}px, 100%)`;
  screenFrame.style.height = `min(${Math.ceil(viewport.height)}px, 100%)`;
  screenFrame.dataset.screenScale = "1";
  screenFrame.dataset.screenVirtualWidth = String(viewport.width);
  screenFrame.dataset.screenVirtualHeight = String(viewport.height);
  syncCleanControlGroupWidths(screenView);
  fitPuzzleFrameComponents(screenView);
}

function currentSceneAspectRatio() {
  if (sceneEditorPreview?.layout?.aspectRatio) {
    return normalizedAspectRatio(sceneEditorPreview.layout.aspectRatio);
  }
  const layers = sceneLayers(currentState);
  const layer = layers.find((candidate) => candidate.focused === true) || layers[0];
  const sceneDef = sceneDefByName(layer?.name) || currentSceneDef();
  return normalizedAspectRatio(sceneDef?.layout?.aspectRatio);
}

function normalizedAspectRatio(ratio) {
  const width = Number(ratio?.width);
  const height = Number(ratio?.height);
  return Number.isFinite(width) && width > 0 && Number.isFinite(height) && height > 0
    ? width / height
    : null;
}

function fitSceneViewport(available, aspect) {
  if (!aspect) {
    return { width: Math.max(1, available.width), height: Math.max(1, available.height) };
  }
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
  const rect = element.getBoundingClientRect();
  const viewport = visibleViewportSize();
  const visibleWidth = Math.min(
    element.clientWidth,
    Math.max(0, Math.min(rect.right, viewport.width) - Math.max(rect.left, 0)),
  );
  const visibleHeight = Math.min(
    element.clientHeight,
    Math.max(0, Math.min(rect.bottom, viewport.height) - Math.max(rect.top, 0)),
  );
  const width = visibleWidth
    - parseFloat(style.paddingLeft || "0")
    - parseFloat(style.paddingRight || "0");
  const height = visibleHeight
    - parseFloat(style.paddingTop || "0")
    - parseFloat(style.paddingBottom || "0");
  return {
    width: Math.max(0, width),
    height: Math.max(0, height),
  };
}

function visibleViewportSize() {
  const visual = window.visualViewport;
  return {
    width: Math.max(1, Number(visual?.width) || Number(window.innerWidth) || document.documentElement.clientWidth || 1),
    height: Math.max(1, Number(visual?.height) || Number(window.innerHeight) || document.documentElement.clientHeight || 1),
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
  if (normalized === "bg") {
    return "background";
  }
  if (normalized === "ink") {
    return "text";
  }
  return /^(background|text|accent)$/.test(normalized) ? normalized : "";
}

function themeClassName(name) {
  const normalized = String(name || "")
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .toLowerCase();
  return normalized ? `theme-${normalized}` : "";
}

function notifyPreviewState(_state) {
}

function notifySceneEditorPreview(_requestId = "") {
}

function renderSceneEditorPreview(_config = {}) {
}

function annotateSceneEditorComponent(_element, _component, _scope = {}) {
}

function selectSceneEditorComponent(_component, _scope = {}) {
  return false;
}

/* puzzle-host:optional:scene-editor:start */
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
    screen: focusedComponentName(state),
    screenHasPuzzle: currentSceneAcceptsModelInput() || Boolean(state.scene),
    theme: state.theme || puzzleBoot.theme || null,
  }, "*");
}

function notifySceneEditorPreview(requestId = sceneEditorPreview?.requestId || "") {
  if (window.parent === window || !sceneEditorPreview) {
    return;
  }
  const sceneName = sceneEditorPreview.sceneName || focusedComponentName(currentState);
  const sceneDef = sceneDefByName(sceneName);
  const layout = mergedScenePreviewLayout(sceneDef, sceneEditorPreview.layout);
  window.parent.postMessage({
    type: "PuzzleStudioScenePreview",
    requestId,
    scene: sceneName,
    theme: sceneEditorPreview.theme || currentState?.theme || puzzleBoot.theme || null,
    layout,
    aspectRatio: normalizedAspectRatio(layout?.aspectRatio),
    components: sceneEditorComponentMetadata(sceneDef?.components || [], {
      __sceneDef: sceneDef,
      __sceneState: sceneEditorPreview.state || currentState?.sceneState || {},
      __standardChoiceCounter: { value: 0 },
    }),
    error: sceneDef ? null : `Unknown scene: ${sceneName}`,
  }, "*");
}

function renderSceneEditorPreview(config = {}) {
  const sceneName = String(config.scene?.name || config.sceneName || focusedComponentName(currentState)).trim();
  const sceneDef = sceneDefByName(sceneName);
  sceneEditorPreview = {
    requestId: String(config.requestId || ""),
    sceneName,
    theme: normalizeScenePreviewTheme(config.theme) || currentState?.theme || puzzleBoot.theme || null,
    layout: normalizeScenePreviewLayout(config.layout),
    state: normalizeScenePreviewState(config.state),
    inspect: config.inspect || {},
  };
  if (!sceneDef) {
    notifySceneEditorPreview(sceneEditorPreview.requestId);
    return;
  }
  const baseState = currentState || puzzleBoot || {};
  const existingLayer = sceneLayers(baseState).find((layer) => layer.name === sceneName);
  const previewState = {
    ...baseState,
    theme: sceneEditorPreview.theme,
    sceneState: sceneEditorPreview.state || existingLayer?.sceneState || existingLayer?.state || baseState.sceneState || {},
    surface: {
      root: sceneName,
      focus: sceneName,
      components: [{
        id: sceneName,
        definition: sceneName,
        placement: "root",
        visibility: "visible",
        modal: false,
        name: sceneName,
        focused: true,
        scene: existingLayer?.scene || baseState.scene || null,
        sceneState: sceneEditorPreview.state || existingLayer?.sceneState || existingLayer?.state || baseState.sceneState || {},
        scenePuzzles: existingLayer?.scenePuzzles || baseState.scenePuzzles || [],
      }],
    },
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
  const layer = sceneLayers(state)[0];
  const components = sceneDef?.components || [];
  const scope = {
    __sceneLayer: layer,
    __sceneDef: sceneDef,
    __sceneState: layer.sceneState || layer.state || {},
    __standardChoiceCounter: { value: 0 },
    __componentPath: ["components"],
  };
  const layerEl = document.createElement("div");
  layerEl.className = "scene-layer is-focused";
  layerEl.classList.toggle("has-ratio-content", components.some((component) => componentContainsSizingKind(component, "ratio")));
  layerEl.style.zIndex = "10";
  applySceneLayout(layerEl, mergedScenePreviewLayout(sceneDef, sceneEditorPreview?.layout), { root: true });
  renderSurfaceComponents(components, layerEl, scope);
  markSingleFrameComponentLayer(layerEl);
  screenView.append(layerEl);
  syncCleanControlGroupWidths(screenView);
  fitPuzzleFrameComponents(screenView);
}

function mergedScenePreviewLayout(sceneDef, override) {
  const base = cloneJson(sceneDef?.layout || {});
  const next = { ...base, ...(override || {}) };
  if (override?.aspectRatio) {
    next.aspectRatio = { ...(base.aspectRatio || {}), ...override.aspectRatio };
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
  if (layout.aspectRatio) {
    const width = Number(layout.aspectRatio.width);
    const height = Number(layout.aspectRatio.height);
    if (Number.isFinite(width) && width > 0 && Number.isFinite(height) && height > 0) {
      next.aspectRatio = { width, height };
    }
  }
  if (layout.gap !== undefined && layout.gap !== null && layout.gap !== "") {
    const gap = Number(layout.gap);
    if (Number.isFinite(gap) && gap >= 0) {
      next.gap = gap;
    }
  }
  if (["start", "center", "end", "stretch"].includes(layout.align)) {
    next.align = layout.align;
  }
  if (["start", "center", "end", "between"].includes(layout.distribute)) {
    next.distribute = layout.distribute;
  }
  if (layout.space?.kind === "fill") {
    const weight = Number(layout.space.weight);
    if (Number.isFinite(weight) && weight > 0) {
      next.space = { kind: "fill", weight };
    }
  } else if (layout.space?.kind === "fit") {
    next.space = { kind: "fit" };
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
  } else if (component.kind === "text") {
    meta.label = resolveLabel(component.content || component, scope);
  } else if (component.source) {
    meta.source = component.source;
  }
  return meta;
}
/* puzzle-host:optional:scene-editor:end */

function renderSurface(state) {
  screenView.replaceChildren();

  const layers = sceneLayers(state);
  syncVisualThemeForSceneStack(layers);
  if (componentEmbedMode && renderEmbeddedPuzzleComponent(layers)) {
    return;
  }
  /* puzzle-host:optional:puzzle3:start */
  if (puzzle3PreviewSurface && renderEmbeddedPuzzleComponent(layers)) {
    return;
  }
  /* puzzle-host:optional:puzzle3:end */
  screenView.classList.toggle("has-scene-stack", layers.length > 1);
  for (const [index, layer] of layers.entries()) {
    if (layer.visibility === "hidden") {
      continue;
    }
    const sceneDef = componentDefinitionByName(layer.definition || layer.name);
    if (!sceneDef) {
      throw new Error(`Unsupported presented component definition: ${String(layer.definition || layer.name || "")}`);
    }
    const components = sceneDef?.components || [];
    const scope = {
      __sceneLayer: layer,
      __sceneDef: sceneDef,
      __sceneState: layer.sceneState || layer.state || {},
      __componentProperties: layer.properties || {},
      __standardChoiceCounter: { value: 0 },
    };

    const layerEl = document.createElement("div");
    layerEl.className = "scene-layer";
    layerEl.classList.toggle("is-focused", layer.focused === true);
    layerEl.classList.toggle("is-modal", layer.modal === true);
    layerEl.classList.toggle("has-ratio-content", components.some((component) => componentContainsSizingKind(component, "ratio")));
    layerEl.style.zIndex = String(10 + index);
    if (layer.modal === true) {
      layerEl.setAttribute("role", "dialog");
      layerEl.setAttribute("aria-modal", "true");
      layerEl.tabIndex = -1;
    }
    applySceneLayout(layerEl, sceneDef?.layout, { root: true });
    const contentRoot = layer.modal === true ? document.createElement("div") : layerEl;
    if (contentRoot !== layerEl) {
      contentRoot.className = "surface-modal-panel";
      layerEl.append(contentRoot);
    }
    renderSurfaceComponents(components, contentRoot, scope);
    bindAwaitedComponentEvent(layerEl, layer, sceneDef);
    markSingleFrameComponentLayer(layerEl);
    screenView.append(layerEl);
    if (layer.modal === true) {
      queueMicrotask(() => layerEl.focus({ preventScroll: true }));
    }
  }
  syncCleanControlGroupWidths(screenView);
  fitPuzzleFrameComponents(screenView);
  scrollSelectedChoiceIntoView(screenView);
}

function bindAwaitedComponentEvent(root, instance, definition) {
  const eventName = instance.awaitEvent;
  if (!eventName) {
    return;
  }
  const binding = definition.events?.[eventName];
  if (!binding) {
    throw new Error(`Component definition ${definition.name} does not declare awaited event ${eventName}`);
  }
  if (binding.pointer === true) {
    root.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      event.stopPropagation();
      sendComponentEvent(instance.id, eventName);
    });
  }
}

function markSingleFrameComponentLayer(layerEl) {
  const visibleChildren = [...layerEl.children].filter((child) => !child.hidden);
  const singleFrameComponent =
    visibleChildren.length === 1 && visibleChildren[0]?.dataset.frameComponent === "true";
  layerEl.classList.toggle("has-single-frame-component", singleFrameComponent);
}

function syncCleanControlGroupWidths(root = screenView) {
  if (!root?.querySelectorAll) {
    return;
  }
  const groups = cleanControlGroups(root);
  for (const group of groups) {
    group.style.removeProperty("--clean-control-width");
  }
  if (!document.body.classList.contains("theme-clean")) {
    return;
  }
  for (const group of groups) {
    const controls = directCleanControlChildren(group);
    if (controls.length < 2) {
      continue;
    }
    const maxWidth = Math.ceil(controls.reduce((max, control) => (
      Math.max(max, cleanControlNaturalWidth(control))
    ), 0));
    if (maxWidth > 0) {
      group.style.setProperty("--clean-control-width", `${maxWidth}px`);
    }
  }
}

function cleanControlGroups(root = screenView) {
  const groups = [];
  if (root?.matches?.(".scene-layer, .view-column, .view-box")) {
    groups.push(root);
  }
  groups.push(...root.querySelectorAll(".scene-layer, .view-column, .view-box"));
  return groups;
}

function directCleanControlChildren(group) {
  return [...group.children].filter((child) => (
    !child.hidden
      && child.matches("button")
      && child.getClientRects().length > 0
  ));
}

function cleanControlNaturalWidth(control) {
  return Math.max(control.scrollWidth, control.offsetWidth);
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
  /* puzzle-host:optional:puzzle3:start */
  if (puzzle3PreviewSurface) {
    const sceneName = puzzle3PreviewSurface.sceneName;
    const scope = {
      __sceneLayer: { name: sceneName, focused: true },
      __sceneDef: { name: sceneName, components: [puzzle3PreviewSurface.component] },
      __sceneState: {},
      __standardChoiceCounter: { value: 0 },
    };
    screenView.classList.remove("has-scene-stack");
    screenView.append(renderPuzzle3Frame(puzzle3PreviewSurface.component, scope));
    return true;
  }
  /* puzzle-host:optional:puzzle3:end */
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
  if (!state?.surface || !Array.isArray(state.surface.components)) {
    throw new Error("Runtime snapshot is missing the required surface component contract");
  }
  return state.surface.components;
}

function focusedComponentName(state = currentState) {
  const focus = state?.surface?.focus;
  if (typeof focus !== "string" || focus.length === 0) {
    throw new Error("Runtime snapshot is missing the required surface focus");
  }
  return focus;
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
    case "text":
    case "button":
    case "choice":
      return "flow";
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
  slot.append(renderFrameComponent(component, scope));
  return slot;
}

function renderFrameComponent(component, scope = {}) {
  /* puzzle-host:optional:puzzle3:start */
  if (component.kind === "puzzle3") {
    return renderPuzzle3Frame(component, scope);
  }
  /* puzzle-host:optional:puzzle3:end */
  return renderPuzzle(component, scope);
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
  return currentSceneAcceptsModelInput();
}

function currentSceneAcceptsModelInput() {
  return stateAcceptsModelInput(currentState || puzzleBoot || {});
}

function isControlPointerTarget(target) {
  return Boolean(target?.closest?.("button, a, input, select, textarea, [role='button'], [role='option']"));
}

function sceneInteractionProfile(scene = currentSceneDef(), options = {}) {
  const state = options.state || currentState || {};
  const standardChoices = scene ? standardChoiceFocusCells(scene) : [];
  return {
    acceptsModelInput: stateAcceptsModelInput(state),
    standardChoices,
  };
}

function currentSceneLayer(state = currentState, scene = currentSceneDef()) {
  const layers = sceneLayers(state || {});
  return layers.find((layer) => scene?.name && layer.name === scene.name)
    || layers.find((layer) => layer.focused === true)
    || layers[0]
    || null;
}

function stateAcceptsModelInput(state = currentState || puzzleBoot || {}) {
  return state?.acceptsModelInput === true
    || standaloneRuntime?.editorPreviewInputEnabled === true;
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
    default:
      throw new Error(`Unsupported scene component: ${String(component.kind || "unknown")}`);
  }
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
  root.dataset.scene = layer?.name || focusedComponentName(currentState);
  const key = `${root.dataset.scene}:${root.dataset.source}`;
  const renderer = new window.PuzzleRenderer(root, {
    renderMode: "canvas",
  });
  renderer.viewport = puzzleViewports.get(key);
  renderer.render(scene);
  puzzleViewports.set(key, renderer.viewport);
  return root;
}

/* puzzle-host:optional:puzzle3:start */
function renderPuzzle3Frame(component, scope = {}) {
  if (!window.Puzzle3DFrameFixture || !window.Puzzle3DFrameAssets || !window.Puzzle3Component) {
    throw new Error("Puzzle3 component assets are unavailable.");
  }
  const sceneName = scope.__sceneDef?.name || scope.__sceneLayer?.name || focusedComponentName(currentState);
  const source = component.source || "board";
  const key = `${sceneName}:${source}`;
  let entry = puzzle3Controllers.get(key);
  if (!entry) {
    const root = document.createElement("div");
    root.className = "puzzle3-component";
    root.dataset.frameComponent = "true";
    root.dataset.scene = sceneName;
    root.dataset.source = source;
    root.setAttribute("aria-label", `${sceneTitle(sceneName)} ${source}`);
    const canvas = document.createElement("canvas");
    canvas.width = 960;
    canvas.height = 640;
    canvas.setAttribute("aria-label", `${sceneTitle(sceneName)} ${source}`);
    root.append(canvas);
    const fixture = puzzle3FrameFixture(sceneName, source);
    const controller = window.Puzzle3Component.attach(canvas, {
      screenView: root,
      snapshot: fixture,
      scene: sceneName,
      component,
      onError(failure) {
        reportPuzzle3ComponentError(failure);
      },
    });
    entry = { root, canvas, controller, levelIndex: null };
    Promise.resolve(controller.ready).then(() => {
      window.PuzzleStudioPreviewRuntimeFailure = null;
      if (shouldPostPuzzle3ComponentMessages()) {
        window.parent.postMessage({ type: "PuzzleStudioPreviewRuntimeReady" }, "*");
      }
    }).catch(() => {});
    if (shouldPostPuzzle3ComponentMessages()) {
      controller.onView?.((view) => {
        window.parent?.postMessage({
          type: "PuzzleStudioPuzzle3View",
          source,
          scene: sceneName,
          view,
        }, "*");
      });
      controller.onStateChange?.((snapshot) => {
        window.parent?.postMessage({
          type: "PuzzleStudioPuzzle3State",
          source,
          scene: sceneName,
          snapshot,
        }, "*");
      });
    }
    puzzle3Controllers.set(key, entry);
  }
  syncPuzzle3ComponentLevel(entry);
  schedulePuzzle3ComponentConnectedResize(entry);
  return entry.root;
}

function reportPuzzle3ComponentError(failure = {}) {
  const error = failure.error;
  const detail = {
    label: String(failure.label || "render failed"),
    name: String(error?.name || "Error"),
    message: String(failure.message || error?.message || error || "unknown error"),
    stack: String(error?.stack || ""),
  };
  window.PuzzleStudioPreviewRuntimeFailure = detail;
  if (shouldPostPuzzle3ComponentMessages()) {
    window.parent.postMessage({
      type: "PuzzleStudioPreviewRuntimeError",
      ...detail,
    }, "*");
  }
}

function puzzle3FrameFixture(sceneName, source = "board") {
  const sessionSnapshot = currentState?.scenePuzzleState?.[source];
  if (sessionSnapshot && typeof sessionSnapshot === "object") {
    return mergePuzzle3SessionSnapshot(window.Puzzle3DFrameFixture, sessionSnapshot);
  }
  const fixture = JSON.parse(JSON.stringify(window.Puzzle3DFrameFixture));
  if (puzzle3PreviewSurface) {
    return puzzle3PreviewSurfaceFixture(fixture, sceneName);
  }
  throw new Error(`Puzzle3 session snapshot is missing for component source: ${source}`);
}

function mergePuzzle3SessionSnapshot(fixture, sessionSnapshot) {
  if (!fixture || typeof fixture !== "object") {
    throw new Error("Puzzle3 frame fixture is unavailable.");
  }
  const objectById = new Map(
    Object.values(fixture.objects || {}).map((object) => [Number(object.id), object]),
  );
  const cells = (sessionSnapshot.cells || []).map((cell) => ({
    ...cell,
    objects: (cell.objects || []).map((reference) => {
      const object = objectById.get(Number(reference.id));
      if (!object) {
        throw new Error(`Puzzle3 session references unknown object id: ${reference.id}`);
      }
      return {
        ...object,
        layer: Number.isInteger(Number(reference.layer)) ? Number(reference.layer) : object.layer,
      };
    }),
  }));
  return {
    ...fixture,
    ...sessionSnapshot,
    cells,
  };
}

function normalizePuzzle3PreviewSurface(update = null) {
  if (!update || (update.type !== PREVIEW_SURFACE_UPDATE_MESSAGE && update.type !== PUZZLE3_MODEL_COMPONENT_PREVIEW_MESSAGE)) {
    return null;
  }
  if (update.type === PREVIEW_SURFACE_UPDATE_MESSAGE && (update.kind !== PUZZLE3_LEVEL_PREVIEW_KIND || update.mode !== ISOLATED_PREVIEW_MODE)) {
    return null;
  }
  const payload = update.type === PREVIEW_SURFACE_UPDATE_MESSAGE
    ? update.payload || {}
    : legacyPuzzle3LevelPreviewPayload(update);
  const component = update.component || update.modelComponent || {};
  return {
    update,
    sceneName: update.scene || "__editor_model_preview__",
    payload,
    component: {
      kind: "puzzle3",
      source: component.source || update.source || "__editor_model_preview__",
      layout: component.layout || update.layout || {},
    },
  };
}

function legacyPuzzle3LevelPreviewPayload(update = {}) {
  return {
    levelIndex: update.levelIndex,
    level: update.level,
    resources: update.resources || update,
    camera: update.camera,
    view: update.view,
    settings: update.settings || {},
  };
}

function setPuzzle3PreviewSurface(update = null) {
  puzzle3PreviewSurface = normalizePuzzle3PreviewSurface(update);
  const embed = effectiveComponentEmbedMode();
  document.documentElement.classList.toggle("is-component-embed", embed);
  document.body.classList.toggle("is-component-embed", embed);
  if (currentState) {
    renderSurface(currentState);
  }
}

function applyPuzzleStudioPreviewSurfaceUpdate(update = null) {
  const surface = normalizePuzzle3PreviewSurface(update);
  if (!surface) {
    return false;
  }
  setPuzzle3PreviewSurface(update);
  const snapshot = puzzle3PreviewSurfaceFixture(
    JSON.parse(JSON.stringify(window.Puzzle3DFrameFixture)),
    surface.sceneName,
  );
  for (const entry of puzzle3Controllers.values()) {
    entry.controller?.replaceSnapshot(snapshot);
  }
  return true;
}

window.applyPuzzleStudioPreviewSurfaceUpdate = applyPuzzleStudioPreviewSurfaceUpdate;

function effectiveComponentEmbedMode() {
  return componentEmbedMode || Boolean(puzzle3PreviewSurface);
}

function shouldPostPuzzle3ComponentMessages() {
  return Boolean((componentEmbedMode || puzzle3PreviewSurface) && window.parent && window.parent !== window);
}

function puzzle3PreviewSurfaceFixture(source, sceneName) {
  const surface = puzzle3PreviewSurface || {};
  const payload = surface.payload || {};
  if (!source || typeof source !== "object" || Array.isArray(source)) {
    throw new Error("Puzzle3 visual preview requires a view template.");
  }
  const next = JSON.parse(JSON.stringify(source));
  const resources = payload.resources || {};
  if (resources.layerCount != null) {
    next.layerCount = Math.max(1, Math.trunc(Number(resources.layerCount) || 1));
  }
  if (resources.objects && typeof resources.objects === "object") {
    next.objects = JSON.parse(JSON.stringify(resources.objects));
  }
  if (resources.visuals && typeof resources.visuals === "object") {
    next.visuals = JSON.parse(JSON.stringify(resources.visuals));
  }
  const level = payload.level || {};
  const size = level.size || payload.size;
  if (!size || typeof size !== "object" || Array.isArray(size)) {
    throw new Error("Puzzle3 visual preview update is missing level size.");
  }
  const cells = Array.isArray(level.cells)
    ? level.cells
    : Array.isArray(payload.cells)
      ? payload.cells
      : null;
  if (!cells) {
    throw new Error("Puzzle3 visual preview update is missing level cells.");
  }
  const rawLevelIndex = payload.levelIndex ?? next.levelIndex ?? 0;
  const levels = Array.isArray(next.levels) && next.levels.length ? next.levels : [{}];
  const levelIndex = Math.max(0, Math.min(levels.length - 1, Math.trunc(Number(rawLevelIndex) || 0)));
  const target = levels[levelIndex] || {};
  levels[levelIndex] = {
    ...target,
    name: level.name || target.name || "level_1",
    label: level.label || target.label || level.name || target.name || "Level 1",
    size: { ...size },
    cells: JSON.parse(JSON.stringify(cells)),
  };
  next.levels = levels;
  next.levelIndex = levelIndex;
  next.size = { ...size };
  next.cells = JSON.parse(JSON.stringify(cells));
  if (payload.camera) {
    next.render.camera = JSON.parse(JSON.stringify({
      ...payload.camera,
      zoom: payload.camera.zoom ?? payload.view?.zoom,
    }));
  }
  if (payload.view) {
    const view = payload.view || {};
    next.view = JSON.parse(JSON.stringify({
      zoom: view.zoom,
      target: view.target,
    }));
  }
  if (payload.settings) {
    const settings = JSON.parse(JSON.stringify(payload.settings));
    next.render = { ...next.render, ...settings };
    if (next.render.grid || settings.grid) {
      next.render.grid = {
        ...(typeof next.render.grid === "object" && next.render.grid ? next.render.grid : {}),
        ...(typeof settings.grid === "object" && settings.grid ? settings.grid : {}),
      };
    }
  }
  const previewSceneName = sceneName || surface.sceneName || "__editor_model_preview__";
  next.scenes = [{
    name: previewSceneName,
    components: [surface.component || { kind: "puzzle3", source: "__editor_model_preview__" }],
  }];
  next.surface = {
    root: previewSceneName,
    focus: previewSceneName,
    components: [{
      id: previewSceneName,
      definition: previewSceneName,
      name: previewSceneName,
      placement: "root",
      visibility: "visible",
      modal: false,
      focused: true,
      scene: null,
      sceneState: {},
      scenePuzzles: [],
    }],
  };
  return next;
}

function syncPuzzle3ComponentLevel(entry) {
  const controller = entry?.controller;
  if (!controller) {
    return;
  }
  if (puzzle3PreviewSurface) {
    return;
  }
  const sceneName = entry.root.dataset.scene;
  const source = entry.root.dataset.source || "board";
  const snapshot = puzzle3FrameFixture(sceneName, source);
  entry.controller.replaceSnapshot(snapshot);
}

function schedulePuzzle3ComponentConnectedResize(entry) {
  if (!entry?.controller || entry.connectedResizePending) {
    return;
  }
  entry.connectedResizePending = true;
  Promise.resolve(entry.controller.ready)
    .then(() => {
      requestAnimationFrame(() => {
        entry.connectedResizePending = false;
        if (entry.root?.isConnected) {
          entry.controller.resize();
        }
      });
    })
    .catch((error) => {
      entry.connectedResizePending = false;
      showError(error);
    });
}
/* puzzle-host:optional:puzzle3:end */

function renderText(component, scope = {}) {
  const text = document.createElement("p");
  text.className = "view-text";
  text.dataset.textRole = component.role || "body";
  if (component.source === "expr") {
    text.textContent = resolveLabel(component.content, scope);
  } else if (component.source === "path") {
    text.textContent = String(resolveViewPath(component.path, scope) ?? "");
  } else {
    text.textContent = component.value || "";
  }
  if (component.textAlign) {
    text.style.textAlign = sceneTextAlignCss(component.textAlign);
  }
  applySizingKind(text, component);
  applySceneLayout(text, component.layout);
  return text;
}

function renderButton(component, scope = {}) {
  const button = document.createElement("button");
  button.type = "button";
  setControlLabel(
    button,
    resolveLabel(component.label, scope) || sceneTitle(effectLabel(component.effect)),
  );
  annotateSceneEditorComponent(button, component, scope);
  button.addEventListener("click", () => {
    if (selectSceneEditorComponent(component, scope)) {
      return;
    }
    runEffectActivationConfirm(button, component.effect);
  });
  applySizingKind(button, component);
  applySceneLayout(button, component.layout);
  return button;
}

function renderChoice(component, scope = {}) {
  const choice = document.createElement("button");
  choice.type = "button";
  setControlLabel(
    choice,
    resolveLabel(component.label, scope),
  );
  choice.classList.add("standard-choice");
  annotateSceneEditorComponent(choice, component, scope);
  const counter = scope.__standardChoiceCounter || { value: 0 };
  scope.__standardChoiceCounter = counter;
  const index = counter.value;
  counter.value += 1;
  choice.dataset.standardChoiceIndex = String(index);
  choice.classList.toggle("is-selected", index === standardChoiceCursor(scope.__sceneDef));
  choice.addEventListener("click", () => {
    if (selectSceneEditorComponent(component, scope)) {
      return;
    }
    standardChoiceCursors.set(scope.__sceneDef.name, index);
    syncStandardChoiceSelection(choice, index);
    runEffectActivationConfirm(choice, component.effect);
  });
  applySizingKind(choice, component);
  applySceneLayout(choice, component.layout);
  return choice;
}

function syncStandardChoiceSelection(choice, selectedIndex) {
  const root = choice.closest(".scene-layer") || choice.parentElement || document;
  root.querySelectorAll(".standard-choice").forEach((item) => {
    item.classList.toggle("is-selected", Number(item.dataset.standardChoiceIndex) === selectedIndex);
  });
  scrollSelectedChoiceIntoView(root);
}

function scrollSelectedChoiceIntoView(root = screenView) {
  const selected = root?.querySelector?.(".standard-choice.is-selected");
  const scroll = selected?.closest?.(".is-scroll");
  if (!selected || !scroll || !root.contains(scroll)) {
    return;
  }
  const viewport = scroll.getBoundingClientRect();
  const item = selected.getBoundingClientRect();
  if (item.top < viewport.top) {
    scroll.scrollTop -= viewport.top - item.top;
  } else if (item.bottom > viewport.bottom) {
    scroll.scrollTop += item.bottom - viewport.bottom;
  }
  if (item.left < viewport.left) {
    scroll.scrollLeft -= viewport.left - item.left;
  } else if (item.right > viewport.right) {
    scroll.scrollLeft += item.right - viewport.right;
  }
}

function setControlLabel(control, label) {
  control.replaceChildren(...controlLabelNodes(label));
}

function controlLabelNodes(label) {
  const left = document.createElement("span");
  left.className = "ps-control-edge is-left";
  left.setAttribute("aria-hidden", "true");
  const text = document.createElement("span");
  text.className = "ps-control-label";
  text.textContent = label;
  const right = document.createElement("span");
  right.className = "ps-control-edge is-right";
  right.setAttribute("aria-hidden", "true");
  return [left, text, right];
}

/* puzzle-host:optional:scene-editor:start */
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
/* puzzle-host:optional:scene-editor:end */

function runEffectActivationConfirm(control, effect) {
  if (!shouldDelayActivationConfirm()) {
    return sendSceneEffect(effect);
  }
  return runActivationConfirm(control, () => sendSceneEffect(effect));
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
  clearActivationConfirmGlyphs(target);
  target.style.setProperty("--ps-confirm-fill", JSON.stringify(puzzlescriptConfirmFill(target)));
}

function clearActivationConfirmGlyphs(target) {
  target.style.removeProperty("--ps-confirm-fill");
}

function puzzlescriptConfirmFill(target) {
  const rect = target.getBoundingClientRect();
  const charWidth = puzzlescriptControlCharWidth(target);
  const count = Math.max(1, Math.ceil(rect.width / charWidth));
  return "#".repeat(count);
}

function puzzlescriptControlCharWidth(target) {
  const probe = document.createElement("span");
  probe.textContent = "#";
  probe.style.position = "absolute";
  probe.style.visibility = "hidden";
  probe.style.pointerEvents = "none";
  probe.style.whiteSpace = "nowrap";
  probe.style.font = window.getComputedStyle(target).font;
  document.body.append(probe);
  const width = probe.getBoundingClientRect().width;
  probe.remove();
  return Number.isFinite(width) && width > 0 ? width : 1;
}

function activationConfirmLabel(target) {
  const labelNode = target.querySelector?.(".ps-control-label");
  return (labelNode?.textContent || target.textContent || "").trim();
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
  const conditionTrue = isSceneConditionTrue(component.condition, scope);
  const children = conditionTrue
    ? component.children || []
    : component.elseChildren || [];
  renderSurfaceComponents(children, fragment, {
    ...scope,
    __componentPath: [...(scope.__componentPath || []), conditionTrue ? "children" : "elseChildren"],
  });
  return fragment;
}

function applySceneLayout(element, layout, options = {}) {
  if (!element || !layout) {
    return;
  }
  if (layout.space?.kind === "fill") {
    const weight = Math.max(1, Number(layout.space.weight) || 1);
    element.style.flex = `${weight} 1 0`;
    element.dataset.sceneSpace = "fill";
  } else {
    element.dataset.sceneSpace = "fit";
  }
  if (layout.aspectRatio) {
    const width = Math.max(1, Number(layout.aspectRatio.width) || 1);
    const height = Math.max(1, Number(layout.aspectRatio.height) || 1);
    element.style.aspectRatio = `${width} / ${height}`;
  }
  if (layout.alignSelf) {
    element.style.alignSelf = sceneLayoutAlignCss(layout.alignSelf);
  }
  if (layout.gap !== undefined && layout.gap !== null) {
    element.style.gap = `calc(${Math.max(0, Number(layout.gap) || 0)} * var(--scene-layout-gap-unit))`;
  }
  applySceneAlignment(element, layout.align, layout.distribute);
}

function applySceneAlignment(element, align = "center", distribute = "center") {
  element.style.alignItems = sceneLayoutAlignCss(align);
  element.style.justifyContent = sceneDistributionCss(distribute);
}

function sceneLayoutAlignCss(value) {
  return ["start", "center", "end", "stretch"].includes(value) ? value : "center";
}

function sceneDistributionCss(value) {
  if (value === "between") return "space-between";
  return ["start", "center", "end"].includes(value) ? value : "center";
}

function sceneTextAlignCss(value) {
  return ["start", "center", "end"].includes(value) ? value : "start";
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
  const parts = Array.isArray(path)
    ? path.map(String).filter(Boolean)
    : String(path || "").split(".").filter(Boolean);
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
    return (scope.__sceneState || currentState?.sceneState)?.[parts[0]]
      ?? currentState?.gameState?.[parts[0]];
  }
  let value = Object.prototype.hasOwnProperty.call(scope, parts[0])
    ? scope[parts[0]]
    : currentState?.[parts[0]];
  if (value === undefined) {
    value = scope.__componentProperties?.[parts[0]];
  }
  if (value === undefined) {
    value = (scope.__sceneState || currentState?.sceneState)?.[parts[0]];
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
    return displayExprText(label, scope);
  }
  if (label.kind === "binary") {
    const value = resolveExprValue(label, scope);
    return value === undefined || value === null ? "" : String(value);
  }
  if (label.kind === "if") {
    return resolveLabel(resolveBoolExpr(label.condition, scope) ? label.then : label.else, scope);
  }
  throw new Error(`Unsupported scene label expression: ${String(label.kind || "unknown")}`);
}

function resolveBoolExpr(expr, scope = {}) {
  const value = resolveExprValue(expr, scope);
  if (typeof value !== "boolean") {
    throw new Error(`Scene boolean expression resolved to ${value === undefined ? "undefined" : typeof value}`);
  }
  return value;
}

function resolveExprValue(expr, scope = {}) {
  if (!expr) {
    throw new Error(`Unsupported scene expression call: ${String(expr.name || "unknown")}`);
  }
  if (typeof expr === "string") {
    return expr;
  }
  if (expr.kind === "bool" || expr.kind === "int" || expr.kind === "text") {
    return expr.value;
  }
  if (expr.kind === "path") {
    return resolveViewPath(expr.path, scope);
  }
  if (expr.kind === "call") {
    if (expr.name === "join") {
      return (expr.args || []).map((arg) => resolveLabel(arg, scope)).join("");
    }
    return undefined;
  }
  if (expr.kind === "binary") {
    if (expr.op === "and") {
      return resolveBoolExpr(expr.left, scope) && resolveBoolExpr(expr.right, scope);
    }
    const left = resolveExprValue(expr.left, scope);
    const right = resolveExprValue(expr.right, scope);
    if (left === undefined || right === undefined) {
      return undefined;
    }
    if (expr.op === "eq") {
      return left === right;
    }
    if (expr.op === "neq") {
      return left !== right;
    }
  }
  if (expr.kind === "if") {
    return resolveExprValue(resolveBoolExpr(expr.condition, scope) ? expr.then : expr.else, scope);
  }
  throw new Error(`Unsupported scene expression: ${String(expr.kind || "unknown")}`);
}

function focusShell() {
  if (!shell || document.activeElement === shell || shell.contains(document.activeElement)) {
    return;
  }
  shell.focus({ preventScroll: true });
}

function isModalDismissKey(event) {
  const rawKey = String(event.key || "");
  const key = normalizedKeyName(rawKey);
  if (rawKey === "Enter"
    || key === "Space"
    || key === "x") {
    return true;
  }
  if (event.altKey || event.ctrlKey || event.metaKey) {
    return false;
  }
  return effectsForKey(event).length > 0;
}

function componentEventAcceptsKey(binding, event) {
  if (binding?.keys === "input") {
    return isModalDismissKey(event);
  }
  if (Array.isArray(binding?.keys)) {
    const key = normalizedKeyName(String(event.key || ""));
    return binding.keys.includes(key);
  }
  return false;
}

function standardSessionActionForKey(key) {
  if (key === "z") {
    return { kind: "undo" };
  }
  if (key === "y") {
    return { kind: "redo" };
  }
  return null;
}

function effectsForKey(event) {
  if (!currentState) {
    return [];
  }
  if (event.altKey || event.ctrlKey || event.metaKey) {
    return [];
  }
  const rawKey = String(event.key || "");
  const key = normalizedKeyName(rawKey);
  const sessionAction = standardSessionActionForKey(key);
  if (sessionAction) {
    return [{ kind: "session_action", action: sessionAction }];
  }
  const keyTokens = logicalKeyTokens(rawKey);
  const scene = currentSceneDef();
  const profile = sceneInteractionProfile(scene);
  const binding = scene?.keys?.find((binding) => binding.keys.some((candidate) => keyTokens.includes(candidate)));
  if (binding) {
    return [{ kind: "scene_effect", effect: binding.effect }];
  }

  const input = (currentState.inputs || []).find((input) =>
    keyTokens.includes(input.key)
    || keyTokens.includes(input.arrow)
    || (input.keys || []).some((candidate) => keyTokens.includes(candidate))
  );
  const standardInput = standardChoiceInputForKey(key);
  if (standardInput && profile.standardChoices.length > 0) {
    return [{ kind: "standard_choice", input: standardInput }];
  }
  if (input && profile.acceptsModelInput) {
    return [{ kind: "model_input", name: input.name }];
  }

  return [];
}

function normalizedKeyName(key) {
  if (key === " ") {
    return "Space";
  }
  if (key.length === 1) {
    return key.toLowerCase();
  }
  return key;
}

function logicalKeyTokens(key) {
  const normalized = normalizedKeyName(key);
  return normalized ? [normalized] : [];
}

function standardChoiceInputForKey(key) {
  if (key === "w" || key === "ArrowUp") {
    return "up";
  }
  if (key === "s" || key === "ArrowDown") {
    return "down";
  }
  if (key === "a" || key === "ArrowLeft") {
    return "left";
  }
  if (key === "d" || key === "ArrowRight") {
    return "right";
  }
  if (isStandardChoiceConfirmKey(key)) {
    return "enter";
  }
  return null;
}

function isStandardChoiceConfirmKey(key) {
  return key === "Enter" || key === "Space" || key === "x";
}

function inputByName(name) {
  if (!currentState) {
    return null;
  }
  return currentState.inputs.find((input) => input.name === name);
}

function currentSceneDef() {
  const source = currentState || puzzleBoot || {};
  const name = focusedComponentName(source);
  return componentDefinitionByName(name, source);
}

function componentDefinitionByName(name, source = currentState || puzzleBoot || {}) {
  const definitions = componentDefinitionsForSource(source);
  return definitions.find((definition) => definition.name === name) || null;
}

function sceneDefByName(name) {
  const source = currentState || puzzleBoot || {};
  return componentDefinitionByName(name, source);
}

function componentDefinitionsForSource(source) {
  return [...(nonEmptyArray(source?.scenes) || []), ...STANDARD_COMPONENT_DEFINITIONS];
}

function nonEmptyArray(value) {
  return Array.isArray(value) && value.length > 0 ? value : null;
}

function isSceneConditionTrue(condition, scope = {}) {
  if (!condition || typeof condition !== "object") {
    throw new Error("Scene condition must be an expression object");
  }
  return resolveBoolExpr(condition, scope);
}

async function sendResolvedInput(input) {
  if (input?.kind === "standard_choice") {
    handleStandardChoiceInput(input.input);
    return;
  }
  if (input?.kind === "model_input") {
    await sendModelInput(input.name);
    return;
  }
  if (input?.kind === "scene_effect") {
    await sendSceneEffect(input.effect);
    return;
  }
  if (input?.kind === "session_action") {
    await postSessionAction(input.action);
    return;
  }
  throw new Error(`Unsupported resolved input: ${String(input?.kind || "unknown")}`);
}

async function sendSceneEffect(effect) {
  if (!effect || typeof effect !== "object" || Array.isArray(effect)) {
    throw new Error("Scene effect must be an object.");
  }
  await postSessionAction({ kind: "scene_effect", effect });
}

async function sendComponentEvent(instance, event) {
  await postSessionAction({ kind: "component_event", instance, event });
}

function applyPresentationEvents(events) {
  for (const event of events || []) {
    if (event.kind === "wait") {
      clientPendingWaits += 1;
    }
    pendingPresentationEvents.push(event);
  }
  if (currentState) {
    currentState.busy = clientPendingWaits > 0;
  }
  dispatchNextPresentationEvent();
}

function dispatchNextPresentationEvent() {
  if (dispatchingPresentationEvents || activeWaitTimers.size > 0) {
    return;
  }
  dispatchingPresentationEvents = true;
  while (pendingPresentationEvents.length > 0) {
    const event = pendingPresentationEvents.shift();
    if (event.kind === "wait") {
      dispatchingPresentationEvents = false;
      startPresentationWait(event);
      return;
    }
    if (event.kind === "animation_batch") {
      if (!Array.isArray(event.animations) || event.animations.length === 0) {
        throw new Error("Animation batch must contain at least one animation event.");
      }
      applyPresentationAnimations(event, event.animations);
    } else {
      soundRuntime.applyEvents([event]);
    }
  }
  dispatchingPresentationEvents = false;
  if (pendingSessionResume) {
    resumePendingSessionTurn();
    return;
  }
  drainQueuedModelInput();
}

async function resumePendingSessionTurn() {
  if (resumingSession) {
    return;
  }
  pendingSessionResume = false;
  resumingSession = true;
  if (currentState) {
    currentState.busy = true;
  }
  try {
    render(await requestJson("/api/resume", { method: "POST" }));
  } catch (error) {
    if (currentState) {
      currentState.busy = false;
    }
    showError(error);
  } finally {
    resumingSession = false;
    drainQueuedModelInput();
  }
}

function applyPresentationAnimations(event, animations) {
  const currentPuzzles = currentState?.scenePuzzles || [];
  if (!currentState
    || focusedComponentName(currentState) !== event.scene
    || currentState.levelIndex !== event.levelIndex
    || !currentPuzzles.includes(event.puzzle)) {
    return;
  }
  const puzzleSnapshot = currentState.scenePuzzleState?.[event.puzzle];
  if (!puzzleSnapshot || typeof puzzleSnapshot !== "object" || Array.isArray(puzzleSnapshot)) {
    throw new Error(`Animation target puzzle snapshot is missing: ${event.puzzle}`);
  }
  const batchId = ++presentationAnimationBatchId;
  puzzleSnapshot.animationEvents = animations;
  puzzleSnapshot.animationBatchId = batchId;
  if (currentState.scene) {
    currentState.scene.animationEvents = animations;
    currentState.scene.animationBatchId = batchId;
  }
  const layer = sceneLayers(currentState).find((candidate) =>
    (candidate?.name === event.scene || candidate?.scene?.name === event.scene)
      && (candidate?.scenePuzzles || []).includes(event.puzzle)
  );
  if (layer?.scene) {
    layer.scene.animationEvents = animations;
    layer.scene.animationBatchId = batchId;
  }
  renderSurface(currentState);
}

function startPresentationWait(event) {
  const config = inputBufferConfig();
  const waitTimer = {
    event,
    startedAt: 0,
    timeoutId: 0,
    done: false,
    resumesSession: sessionWaiting,
    fastForwardRequested: Boolean(
      pendingModelInput && config.queueDuringWait && config.fastForwardWait
    ),
    config,
  };
  waitTimer.complete = () => {
    if (waitTimer.done) {
      return;
    }
    waitTimer.done = true;
    activeWaitTimers.delete(waitTimer);
    clientPendingWaits = Math.max(0, clientPendingWaits - 1);
    pendingSessionResume = pendingSessionResume || waitTimer.resumesSession;
    if (currentState) {
      currentState.busy = clientPendingWaits > 0 || pendingSessionResume;
    }
    dispatchNextPresentationEvent();
  };
  activeWaitTimers.add(waitTimer);
  window.setTimeout(() => {
    if (waitTimer.done) {
      return;
    }
    waitTimer.startedAt = performance.now();
    waitTimer.timeoutId = setTimeout(
      waitTimer.complete,
      Math.max(0, Number(event.milliseconds || event.ms || 0)),
    );
    if (waitTimer.fastForwardRequested) {
      fastForwardWaitTimer(waitTimer);
    }
  }, 0);
}

function inputBufferConfig() {
  const source =
    currentState?.inputBuffer ||
    currentState?.scene?.settings?.inputBuffer ||
    puzzleBoot.inputBuffer ||
    {};
  return {
    queueDuringWait: source.queueDuringWait !== false,
    fastForwardWait: source.fastForwardWait !== false,
    minWaitMs: Math.max(0, Number(source.minWaitMs ?? 50)),
  };
}

function fastForwardActiveWaitsForQueuedInput(config = inputBufferConfig()) {
  if (!config.fastForwardWait) {
    return;
  }
  for (const waitTimer of activeWaitTimers) {
    waitTimer.config = config;
    fastForwardWaitTimer(waitTimer);
  }
}

function fastForwardWaitTimer(waitTimer) {
  if (waitTimer.done) {
    return;
  }
  if (!waitTimer.startedAt) {
    waitTimer.fastForwardRequested = true;
    return;
  }
  clearTimeout(waitTimer.timeoutId);
  const elapsed = performance.now() - waitTimer.startedAt;
  const remaining = Math.max(0, waitTimer.config.minWaitMs - elapsed);
  waitTimer.timeoutId = setTimeout(waitTimer.complete, remaining);
}

function sendModelInput(input) {
  if (currentState?.busy || clientPendingWaits > 0) {
    const config = inputBufferConfig();
    if (!config.queueDuringWait) {
      return undefined;
    }
    pendingModelInput = input;
    fastForwardActiveWaitsForQueuedInput(config);
    return undefined;
  }
  return sendModelInputNow(input);
}

function sendModelInputNow(input) {
  if (!input) {
    return undefined;
  }
  if (sendHostModelInput(input)) {
    return undefined;
  }
  return post(`/api/input/${encodeURIComponent(input)}`);
}

async function drainQueuedModelInput() {
  if (drainingQueuedModelInput || clientPendingWaits > 0 || currentState?.busy || !pendingModelInput) {
    return;
  }
  drainingQueuedModelInput = true;
  const input = pendingModelInput;
  pendingModelInput = null;
  try {
    await sendModelInputNow(input);
  } finally {
    drainingQueuedModelInput = false;
  }
}

function standardChoiceFocusCells(scene = currentSceneDef()) {
  if (!scene) {
    return [];
  }
  const footprint = componentColumnFootprint(scene.components || [], {
    focusKind: "choice",
    scope: {
      __sceneDef: scene,
      __sceneState: currentState?.sceneState || {},
    },
  });
  return footprint.cells.map((cell, index) => ({ ...cell, index }));
}

function componentFootprint(component, context = {}) {
  if (!component) {
    return emptyFootprint();
  }
  const focusKind = context.focusKind || "choice";
  if (focusKind === "choice" && component.kind === "choice") {
    return {
      width: 1,
      height: 1,
      cells: [{ x: 0, y: 0, kind: "component", component, scope: context.scope || {} }],
    };
  }
  if (["text", "frame", "puzzle", "puzzle3"].includes(component.kind)) {
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
      isSceneConditionTrue(component.condition, context.scope || {})
        ? component.children || []
        : component.elseChildren || [],
      context,
    );
  }
  return emptyCellFootprint();
}

function stackColumnFootprints(footprints) {
  let width = 0;
  let height = 0;
  const cells = [];
  for (const child of footprints || []) {
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
    const cell = cells[cursor];
    runEffectActivationConfirm(selectedChoice, cell?.component?.effect || null);
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
  if (effect.kind === "command" || effect.kind === "input" || effect.kind === "component_effect" || effect.kind === "routine_call") {
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
    return effect.target ? `${effect.target}.next_level` : "next_level";
  }
  if (effect.kind === "puzzle_previous_level") {
    return effect.target ? `${effect.target}.previous_level` : "previous_level";
  }
  if (effect.kind === "puzzle_reset") {
    return effect.target ? `${effect.target}.restart` : "restart";
  }
  if (effect.kind === "puzzle_goto_level") {
    return effect.target ? `${effect.target}.goto` : "goto";
  }
  if (effect.kind === "back") {
    return "back";
  }
  return "";
}

function effectCommandName(effect) {
  return effect?.scene || effect?.screen || "";
}

function displayExprText(expr, scope = {}) {
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
    return resolved === undefined || resolved === null ? expr.path : displayExprValueText(resolved);
  }
  if (expr.kind === "call") {
    return `${expr.name}(${(expr.args || []).map((arg) => displayExprText(arg, scope)).join(", ")})`;
  }
  if (expr.kind === "binary") {
    const op = expr.op === "and" ? "and" : expr.op === "eq" ? "==" : "!=";
    return `${displayExprText(expr.left, scope)} ${op} ${displayExprText(expr.right, scope)}`;
  }
  if (expr.kind === "if") {
    return `if ${displayExprText(expr.condition, scope)} { ${displayExprText(expr.then, scope)} } else { ${displayExprText(expr.else, scope)} }`;
  }
  return "";
}

function displayExprValueText(value) {
  if (typeof value === "object" && value?.kind === "level" && value.name !== undefined) {
    return JSON.stringify(String(value.name));
  }
  return commandPayload(value);
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

function dispatchKeyboardInput(event) {
  const modal = activeModalComponent(currentState);
  if (modal) {
    const definition = componentDefinitionByName(modal.definition || modal.name);
    const eventName = modal.awaitEvent;
    if (!definition || !eventName) {
      throw new Error("Active modal component is missing its definition or awaited event");
    }
    const binding = definition.events?.[eventName];
    if (!binding) {
      throw new Error(`Component definition ${definition.name} does not declare awaited event ${eventName}`);
    }
    if (componentEventAcceptsKey(binding, event)) {
      sendComponentEvent(modal.id, eventName);
    }
    return true;
  }

  if (!currentState) {
    return false;
  }

  const effects = effectsForKey(event);
  if (effects.length === 0) {
    return false;
  }
  const dispatchEffects = event.repeat && (currentState?.busy || clientPendingWaits > 0)
    ? effects.filter((effect) => effect?.kind !== "model_input")
    : effects;
  for (const effect of dispatchEffects) {
    sendResolvedInput(effect);
  }
  return true;
}

function activeModalComponent(state) {
  const components = state?.surface?.components;
  if (!Array.isArray(components)) {
    return null;
  }
  return [...components]
    .reverse()
    .find((component) => component.modal === true && component.visibility !== "hidden") || null;
}

document.addEventListener("keydown", (event) => {
  if (componentEmbedMode) {
    return;
  }
  if (dispatchKeyboardInput(event)) {
    event.preventDefault();
  }
});

if (standaloneRuntime) {
  window.addEventListener("PuzzleStandaloneStateChanged", () => {
    loadState().catch((error) => {
      showError(error);
    });
  });
}

/* puzzle-host:optional:studio-bridge:start */
let studioPreviewDebugMode = false;

function notifyPreviewDebugTrace(debug, snapshot) {
  if (window.parent === window) {
    return;
  }
  window.parent.postMessage({
    type: "PuzzleStudioPreviewDebugTrace",
    debug: debug || null,
    snapshot: snapshot || null,
    levelIndex: snapshot?.levelIndex ?? null,
    scene: snapshot ? focusedComponentName(snapshot) : "",
  }, "*");
}

async function postStudioPreviewDebugInput(input) {
  try {
    if (!standaloneRuntime) {
      throw new Error("Debug input requires the embedded WASM game runtime.");
    }
    const response = standaloneRuntime.applyDebugInputName(input);
    const snapshot = response?.snapshot;
    if (!snapshot) {
      throw new Error("Debug input response did not include a snapshot.");
    }
    render(snapshot);
    notifyPreviewDebugTrace(response.debug || null, snapshot);
  } catch (error) {
    showError(error);
  }
}

sendHostModelInput = (input) => {
  if (!studioPreviewDebugMode) {
    return false;
  }
  postStudioPreviewDebugInput(input);
  return true;
};

window.addEventListener("message", async (event) => {
  if (event.data?.type === "PuzzleStudioSetPreviewDebugMode") {
    studioPreviewDebugMode = event.data.enabled === true;
    return;
  }

  if (event.data?.type === "PuzzleStudioSetScenePreview") {
    renderSceneEditorPreview(event.data || {});
    return;
  }

  if (event.data?.type === "PuzzleStudioRequestScenePreview") {
    notifySceneEditorPreview(String(event.data.requestId || ""));
    return;
  }

  /* puzzle-host:optional:puzzle3:start */
  if (event.data?.type === PREVIEW_SURFACE_UPDATE_MESSAGE || event.data?.type === PUZZLE3_MODEL_COMPONENT_PREVIEW_MESSAGE) {
    applyPuzzleStudioPreviewSurfaceUpdate(event.data || {});
    return;
  }

  if (event.data?.type === "PuzzleStudioRequestPuzzle3State") {
    for (const entry of puzzle3Controllers.values()) {
      window.parent?.postMessage({
        type: "PuzzleStudioPuzzle3State",
        source: entry.root?.dataset.source || "",
        scene: entry.root?.dataset.scene || "",
        snapshot: entry.controller?.snapshot?.() || null,
      }, "*");
    }
    return;
  }
  /* puzzle-host:optional:puzzle3:end */

  if (event.data?.type === "PuzzleStudioSetState") {
    if (standaloneRuntime && event.data.state) {
      await standaloneRuntime.setCurrentState(event.data.state, {
        levelIndex: event.data.levelIndex,
        regions: event.data.regions,
        animationEvents: event.data.animationEvents,
        acceptModelInput: event.data.acceptModelInput === true,
        materializeLevelStart: event.data.materializeLevelStart === true,
        materializeDisplay: event.data.materializeDisplay === true,
        materializeTurnStart: event.data.materializeTurnStart === true,
      });
      const snapshot = standaloneRuntime.snapshot({ forceJs: true });
      if (event.data.silent === true) {
        notifyPreviewState(snapshot);
      } else {
        render(snapshot);
      }
    }
    return;
  }

  /* puzzle-host:optional:solver:start */
  if (event.data?.type === "PuzzleStudioSolve") {
    const requestId = event.data.requestId;
    const solveRequest = { cancelled: false };
    activeSolveRequests.set(requestId, solveRequest);
    try {
      if (standaloneRuntime && event.data.state) {
        await standaloneRuntime.setCurrentState(event.data.state, {
          levelIndex: event.data.levelIndex,
          regions: event.data.regions,
          animationEvents: event.data.animationEvents,
          acceptModelInput: event.data.acceptModelInput === true,
          materializeLevelStart: event.data.materializeLevelStart === true,
          materializeDisplay: event.data.materializeDisplay === true,
          materializeTurnStart: event.data.materializeTurnStart === true,
        });
        const snapshot = standaloneRuntime.snapshot({ forceJs: true });
        if (event.data.silent === true) {
          notifyPreviewState(snapshot);
        } else {
          render(snapshot);
        }
      }
      const solution = await solveStandaloneCurrentState(event.data.options || {}, solveRequest);
      window.parent.postMessage({
        type: "PuzzleStudioSolveResult",
        requestId,
        solution,
      }, "*");
    } catch (error) {
      window.parent.postMessage({
        type: "PuzzleStudioSolveResult",
        requestId,
        error: String(error?.message || error),
      }, "*");
    } finally {
      activeSolveRequests.delete(requestId);
    }
    return;
  }

  if (event.data?.type === "PuzzleStudioCancelSolve") {
    const solveRequest = activeSolveRequests.get(event.data.requestId);
    if (solveRequest) {
      solveRequest.cancelled = true;
    }
    return;
  }
  /* puzzle-host:optional:solver:end */

  if (event.data?.type === "PuzzleStudioKey") {
    soundRuntime.primePlayback();
    const keyEvent = {
      key: String(event.data.key || ""),
      code: String(event.data.code || ""),
      repeat: event.data.repeat === true,
      altKey: event.data.altKey === true,
      ctrlKey: event.data.ctrlKey === true,
      metaKey: event.data.metaKey === true,
      shiftKey: event.data.shiftKey === true,
    };
    dispatchKeyboardInput(keyEvent);
    return;
  }
});
/* puzzle-host:optional:studio-bridge:end */

playSurface.addEventListener("pointerdown", (event) => {
  if (!currentState || currentState.busy || !currentSceneAcceptsModelInput()) {
    return;
  }
  if (isControlPointerTarget(event.target)) {
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
    sendModelInput(input.name);
  }
});

playSurface.addEventListener("pointercancel", () => {
  swipeStart = null;
});

loadState().catch((error) => {
  showError(error);
});

if (!componentEmbedMode) {
  document.addEventListener("pointerdown", focusShell);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") {
      soundRuntime.pauseForHiddenDocument();
    } else {
      soundRuntime.resumeAfterVisibleDocument();
    }
  });
  installScreenScaleResizeHooks();
}

function showError(error) {
  console.error(error);
  const message = String(error?.message || error || "Unknown runtime error");
  if (!screenView) {
    return;
  }
  const panel = document.createElement("div");
  panel.className = "runtime-error";
  panel.setAttribute("role", "alert");
  panel.textContent = message;
  screenView.replaceChildren(panel);
  scheduleScreenScaleSync(2);
}
