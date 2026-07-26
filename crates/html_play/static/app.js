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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
let activeThemeClass = "";
const activeThemeVariables = new Set();
const activationConfirmDelayMs = 160;

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

>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
const puzzleBoot = window.PuzzleBoot || {};
const standaloneRuntime = window.PuzzleStandaloneRuntime
  ? new window.PuzzleStandaloneRuntime(puzzleBoot, window.PuzzleRuntimeExportJson)
  : null;
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
const reportedAudioConsumerErrors = new Set();
=======
const soundRuntime = new PuzzleSoundRuntime();
let renderWasmModulePromise = null;

async function renderWasmModule() {
  if (standaloneRuntime) {
    await standaloneRuntime.ensureInitialized();
    return standaloneRuntime.wasmModule;
  }
  renderWasmModulePromise ||= import("./wasm_player/puzzle_wasm_player.js")
    .then(async (module) => {
      await module.default();
      return module;
    });
  return renderWasmModulePromise;
}

async function prepareResolvedRenderScene(renderScene) {
  const module = await renderWasmModule();
  if (!window.PuzzleRenderAssetDecoder?.hydrateRenderSceneImages) {
    throw new Error("Render asset decoder is unavailable.");
  }
  return window.PuzzleRenderAssetDecoder.hydrateRenderSceneImages(module, renderScene);
}

async function resolveRenderMoment(renderScene, moment) {
  const module = await renderWasmModule();
  if (typeof module.resolve_render_moment !== "function") {
    throw new Error("Rust render-moment resolver is unavailable.");
  }
  return JSON.parse(module.resolve_render_moment(
    JSON.stringify(renderScene),
    JSON.stringify(moment),
  ));
}
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544

function reportAudioConsumerError(error) {
  const message = String(error?.message || error || "Unknown audio consumer error");
  if (reportedAudioConsumerErrors.has(message)) {
    return;
  }
  reportedAudioConsumerErrors.add(message);
  console.error(`Audio consumer: ${message}`);
}

async function unlockAudioFromGesture() {
  try {
    if (!standaloneRuntime) {
      throw new Error("Rust browser audio backend is unavailable.");
    }
    await standaloneRuntime.unlockAudio();
  } catch (error) {
    reportAudioConsumerError(error);
  }
}

function reportPresentationEventConsumed() {
  if (standaloneRuntime) {
    standaloneRuntime.presentationEventConsumed();
  }
}

async function setAudioVisible(visible) {
  try {
    if (!standaloneRuntime) {
      throw new Error("Rust browser audio backend is unavailable.");
    }
    await standaloneRuntime.setAudioVisible(visible);
  } catch (error) {
    reportAudioConsumerError(error);
  }
}

document.addEventListener("keydown", async () => {
  await unlockAudioFromGesture();
}, { capture: true });
document.addEventListener("pointerdown", async () => {
  await unlockAudioFromGesture();
}, { capture: true });

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
let screenScaleSyncFrame = 0;
let screenScaleSyncPasses = 0;
const activeWaitTimers = new Set();
const pendingPresentationEvents = [];
let presentationAnimationBatchId = 0;
let dispatchingPresentationEvents = false;
let pendingSessionResume = false;
let resumingSession = false;
let sessionWaiting = false;
let sceneEditorPreview = null;

function sendHostModelInput(_input) {
  return false;
}

window.addEventListener("PuzzleProgressSaveError", (event) => {
  showError(new Error(event.detail?.message || "Progress save failed."));
});

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
    maxStoredNodes: Number(options.maxStoredNodes ?? 5_000_000),
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  try {
    render(await requestJson("/api/state"));
  } catch (error) {
    showError(error);
  }
=======
  render(await requestSessionAction({ kind: "initialize" }));
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
  return post("/api/action", sessionActionRequestOptions(action));
}

function requestSessionAction(action) {
  return requestJson("/api/action", sessionActionRequestOptions(action));
}

function sessionActionRequestOptions(action) {
  return {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(action),
  };
}

function render(state) {
  currentState = state;
  sessionWaiting = state?.busy === true;
  window.__PuzzleCurrentState = state;
  applyTheme(state?.theme);
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
  if (standaloneRuntime) {
    standaloneRuntime.presentationFrame();
  }
  applyPresentationEvents(presentationEvents);
}

function firstDisplayError(state) {
  for (const source of state?.viewportSources || []) {
    if (source?.state?.displayError) {
      return String(source.state.displayError);
    }
  }
  for (const layer of state?.surface?.components || []) {
    if (layer?.presentation?.error) {
      return String(layer.presentation.error);
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
  if (componentEmbedMode || !currentState || !screenFrame || !screenView || !playSurface) {
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
  fitPuzzleFrameComponents(screenView);
}

function currentSceneAspectRatio() {
  if (sceneEditorPreview?.layout?.aspectRatio) {
    return normalizedAspectRatio(sceneEditorPreview.layout.aspectRatio);
  }
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  const layers = sceneLayers(currentState);
  const layer = layers.find((candidate) => candidate.id === currentState?.surface?.focus) || layers[0];
  return normalizedAspectRatio(layer?.presentation?.layout?.aspectRatio);
=======
  const layers = surfaceComponents(currentState);
  const layer = layers.find((candidate) => candidate.focused === true) || layers[0];
  const sceneDef = componentDefinitionByName(layer?.name) || focusedComponentDefinition();
  return normalizedAspectRatio(sceneDef?.layout?.aspectRatio);
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
  const normalized = normalizeRuntimeTheme(theme);
  const root = document.body;
  const variables = {
    background: linearRgbaCss(normalized.background),
    text: linearRgbaCss(normalized.text),
    muted: linearRgbaCss(normalized.mutedText),
    accent: linearRgbaCss(normalized.accent),
    "panel-bg": linearRgbaCss(normalized.panel),
    "popup-bg": linearRgbaCss(normalized.panel),
    "button-bg": linearRgbaCss(normalized.control),
    "button-bg-hover": linearRgbaCss(normalized.controlFocused),
    "button-bg-active": linearRgbaCss(normalized.controlSelected),
    "menu-item-bg": linearRgbaCss(normalized.control),
    "menu-item-bg-selected": linearRgbaCss(normalized.controlSelected),
    "menu-item-ring": linearRgbaCss(normalized.controlSelectedBorder),
    "control-selected-border": linearRgbaCss(normalized.controlSelectedBorder),
    "text-heading-size": `${normalized.typography.heading.fontSizePx}px`,
    "text-heading-line-height": String(normalized.typography.heading.lineHeight),
    "text-subheading-size": `${normalized.typography.subheading.fontSizePx}px`,
    "text-subheading-line-height": String(normalized.typography.subheading.lineHeight),
    "text-body-size": `${normalized.typography.body.fontSizePx}px`,
    "text-body-line-height": String(normalized.typography.body.lineHeight),
    "text-caption-size": `${normalized.typography.caption.fontSizePx}px`,
    "text-caption-line-height": String(normalized.typography.caption.lineHeight),
    "control-padding-horizontal": `${normalized.controlLayout.paddingHorizontalPx}px`,
    "control-padding-vertical": `${normalized.controlLayout.paddingVerticalPx}px`,
    "control-margin": `${normalized.controlLayout.marginPx}px`,
    "control-border-width": `${normalized.controlLayout.borderWidthPx}px`,
    "radius-control": `${normalized.controlLayout.cornerRadiusPx}px`,
  };
  for (const [name, value] of Object.entries(variables)) {
    root.style.setProperty(`--${name}`, value);
  }
}

function normalizeRuntimeTheme(theme) {
  if (!theme || typeof theme !== "object" || Array.isArray(theme)) {
    throw new Error("Runtime snapshot is missing the required typed theme contract");
  }
  const colors = [
    "background",
    "text",
    "mutedText",
    "accent",
    "panel",
    "control",
    "controlFocused",
    "controlSelected",
    "controlSelectedBorder",
  ];
  const normalized = {};
  for (const name of colors) {
    normalized[name] = normalizeLinearRgba(theme[name], `theme.${name}`);
  }
  normalized.typography = {};
  for (const name of ["heading", "subheading", "body", "caption"]) {
    const style = theme.typography?.[name];
    const fontSizePx = Number(style?.fontSizePx);
    const lineHeight = Number(style?.lineHeight);
    if (!(fontSizePx > 0) || !Number.isFinite(lineHeight) || lineHeight <= 0) {
      throw new Error(`Runtime theme has an invalid typography.${name} contract`);
    }
    normalized.typography[name] = { fontSizePx, lineHeight };
  }
  const layout = theme.controlLayout;
  normalized.controlLayout = {};
  for (const name of [
    "paddingHorizontalPx",
    "paddingVerticalPx",
    "marginPx",
    "borderWidthPx",
    "cornerRadiusPx",
  ]) {
    const value = Number(layout?.[name]);
    if (!Number.isFinite(value) || value < 0) {
      throw new Error(`Runtime theme has an invalid controlLayout.${name} contract`);
    }
    normalized.controlLayout[name] = value;
  }
  return normalized;
}

function normalizeLinearRgba(color, label) {
  const normalized = {};
  for (const name of ["red", "green", "blue", "alpha"]) {
    const value = Number(color?.[name]);
    if (!Number.isFinite(value) || value < 0 || value > 1) {
      throw new Error(`Runtime theme has an invalid ${label}.${name} channel`);
    }
    normalized[name] = value;
  }
  return normalized;
}

function linearRgbaCss(color) {
  return `color(srgb-linear ${color.red} ${color.green} ${color.blue} / ${color.alpha})`;
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
    inputs: state.inputs,
    screen: focusedComponentName(state),
    screenHasPuzzle: currentSceneAcceptsModelInput()
      || (state.viewportSources || []).some((source) => source?.id?.component === state.surface?.focus),
    theme: state.theme,
  }, "*");
}

function notifySceneEditorPreview(requestId = sceneEditorPreview?.requestId || "") {
  if (window.parent === window || !sceneEditorPreview) {
    return;
  }
  const sceneName = sceneEditorPreview.sceneName || focusedComponentName(currentState);
  const sceneDef = sceneEditorPreview.definition || null;
  const layout = mergedScenePreviewLayout(sceneDef, sceneEditorPreview.layout);
  window.parent.postMessage({
    type: "PuzzleStudioScenePreview",
    requestId,
    scene: sceneName,
    theme: sceneEditorPreview.theme,
    layout,
    aspectRatio: normalizedAspectRatio(layout?.aspectRatio),
    components: sceneEditorComponentMetadata(sceneDef?.components || [], {
      __sceneDef: sceneDef,
    }),
    error: sceneDef ? null : `Unknown scene: ${sceneName}`,
  }, "*");
}

async function renderSceneEditorPreview(config = {}) {
  const sceneName = String(config.scene?.name || config.sceneName || focusedComponentName(currentState)).trim();
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  const sceneDef = config.presentation || sceneDefByName(sceneName);
=======
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  sceneEditorPreview = {
    requestId: String(config.requestId || ""),
    sceneName,
    theme: normalizeScenePreviewTheme(config.theme ?? currentState?.theme),
    layout: normalizeScenePreviewLayout(config.layout),
    state: normalizeScenePreviewState(config.state),
    inspect: config.inspect || {},
  };
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  if (sceneEditorPreview.state && !config.presentation) {
    showError(new Error(
      `Scene preview ${sceneName} requires a Rust-resolved presentation for the requested state.`
    ));
    return;
  }
  if (!sceneDef) {
    notifySceneEditorPreview(sceneEditorPreview.requestId);
    return;
  }
  if (!currentState) {
    throw new Error("Scene preview requires an initialized runtime snapshot");
  }
  const baseState = currentState;
=======
  if (!standaloneRuntime) {
    throw new Error("Scene editor preview requires the WASM scene presentation resolver.");
  }
  const sceneDef = await standaloneRuntime.resolveScenePresentation(
    sceneName,
    sceneEditorPreview.state || {},
  );
  sceneEditorPreview.definition = sceneDef;
  const baseState = currentState || puzzleBoot || {};
  const existingLayer = surfaceComponents(baseState).find((layer) => (
    layer.id === sceneName || layer.name === sceneName || layer.definition === sceneName
  ));
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  const previewState = {
    ...baseState,
    theme: sceneEditorPreview.theme,
    surface: {
      root: sceneName,
      focus: sceneName,
      components: [{
        id: sceneName,
        placement: "root",
        visibility: "visible",
        modal: false,
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
        name: sceneName,
        focused: true,
        scene: existingLayer?.scene || baseState.scene || null,
        sceneState: sceneEditorPreview.state || existingLayer?.sceneState || existingLayer?.state || baseState.sceneState || {},
        scenePuzzles: existingLayer?.scenePuzzles || baseState.scenePuzzles || [],
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
        presentation: sceneDef,
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
  const layer = surfaceComponents(state)[0];
  const components = sceneDef?.components || [];
  const scope = {
    __sceneLayer: layer,
    __sceneDef: sceneDef,
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
  return normalizeRuntimeTheme(theme);
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    meta.label = component.label || "";
  } else if (component.kind === "text") {
    meta.label = component.value || "";
=======
    meta.label = String(component.label || "") || sceneTitle(effectLabel(component.effect));
    meta.effect = component.effect || null;
  } else if (component.kind === "text") {
    meta.label = String(component.value || "");
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  } else if (component.source) {
    meta.source = component.source;
  }
  return meta;
}
/* puzzle-host:optional:scene-editor:end */

function renderSurface(state) {
  screenView.replaceChildren();

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  const layers = sceneLayers(state);
=======
  const layers = surfaceComponents(state);
  syncVisualThemeForSceneStack(layers);
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    const presentation = layer.presentation;
    if (!presentation || typeof presentation !== "object") {
      throw new Error(`Presented component ${String(layer.id || "")} is missing its resolved presentation.`);
=======
    const sceneDef = presentationDefinitionForLayer(layer);
    if (!sceneDef) {
      throw new Error(`Unsupported presented component definition: ${String(layer.definition || layer.name || "")}`);
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    }
    if (presentation.error) {
      throw new Error(`Presented component ${String(layer.id || "")} failed to resolve: ${presentation.error}`);
    }
    const components = presentation.components || [];
    const scope = {
      __sceneLayer: layer,
      __sceneDef: presentation,
    };

    const layerEl = document.createElement("div");
    layerEl.className = "scene-layer";
    layerEl.classList.toggle("is-focused", layer.id === state.surface.focus);
    layerEl.classList.toggle("is-modal", layer.modal === true);
    layerEl.classList.toggle("has-ratio-content", components.some((component) => componentContainsSizingKind(component, "ratio")));
    layerEl.style.zIndex = String(10 + index);
    if (layer.modal === true) {
      layerEl.setAttribute("role", "dialog");
      layerEl.setAttribute("aria-modal", "true");
      layerEl.tabIndex = -1;
    }
    applySceneLayout(layerEl, presentation.layout, { root: true });
    const contentRoot = layer.modal === true ? document.createElement("div") : layerEl;
    if (contentRoot !== layerEl) {
      contentRoot.className = "surface-modal-panel";
      layerEl.append(contentRoot);
    }
    renderSurfaceComponents(components, contentRoot, scope);
    bindAwaitedComponentEvent(layerEl, layer, presentation);
    markSingleFrameComponentLayer(layerEl);
    screenView.append(layerEl);
    if (layer.modal === true) {
      queueMicrotask(() => layerEl.focus({ preventScroll: true }));
    }
  }
  fitPuzzleFrameComponents(screenView);
  scrollSelectedChoiceIntoView(screenView);
}

function bindAwaitedComponentEvent(root, instance, presentation) {
  const eventName = instance.awaitEvent;
  if (!eventName) {
    return;
  }
  const binding = presentation.events?.[eventName];
  if (!binding) {
    throw new Error(`Presented component ${instance.id} does not declare awaited event ${eventName}`);
  }
  if (!binding.actionToken) {
    throw new Error(`Presented component ${instance.id} event ${eventName} is missing its action token`);
  }
  if (binding.pointer === true) {
    root.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      event.stopPropagation();
      sendSceneActionToken(binding.actionToken);
    });
  }
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
  /* puzzle-host:optional:puzzle3:start */
  if (puzzle3PreviewSurface) {
    const sceneName = puzzle3PreviewSurface.sceneName;
    const scope = {
      __sceneLayer: { name: sceneName, focused: true },
      __sceneDef: { components: [puzzle3PreviewSurface.component] },
    };
    screenView.classList.remove("has-scene-stack");
    screenView.append(renderPuzzle3Frame(puzzle3PreviewSurface.component, scope));
    return true;
  }
  /* puzzle-host:optional:puzzle3:end */
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  const layer = layers.find((candidate) => candidate.id === currentState?.surface?.focus) || layers[0];
  const sceneDef = layer?.presentation;
=======
  const layer = layers.find((candidate) => candidate.focused === true) || layers[0];
  const sceneDef = componentDefinitionByName(layer?.name);
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  const component = findComponentByKind(sceneDef?.components || [], "puzzle");
  if (!layer || !sceneDef || !component) {
    return false;
  }
  const scope = {
    __sceneLayer: layer,
    __sceneDef: sceneDef,
  };
  screenView.classList.remove("has-scene-stack");
  screenView.append(renderPuzzle(component, scope));
  return true;
}

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
function sceneLayers(state) {
=======
function syncVisualThemeForSceneStack(layers) {
  const visualThemeClass = window.GameVisuals?.themeClass || "";
  if (!visualThemeClass || visualThemeClass === activeThemeClass) {
    return;
  }
  const hasPuzzleLayer = layers.some((layer) => {
    const scene = componentDefinitionByName(layer.name);
    return sceneHasComponent(scene, "puzzle") || sceneHasComponent(scene, "frame");
  });
  if (!hasPuzzleLayer) {
    document.body.classList.remove(visualThemeClass);
  }
}

function surfaceComponents(state) {
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  if (!state?.surface || !Array.isArray(state.surface.components)) {
    throw new Error("Runtime snapshot is missing the required surface component contract");
  }
  return state.surface.components;
}

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
function runtimeViewportSourceId(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)
    || typeof value.component !== "string" || value.component.length === 0
    || typeof value.source !== "string" || value.source.length === 0) {
    throw new Error("Resolved viewport is missing its typed component/source identity");
  }
  return value;
}

function runtimeViewportSourceState(sourceId, state = currentState) {
  const id = runtimeViewportSourceId(sourceId);
  if (!Array.isArray(state?.viewportSources)) {
    throw new Error("Runtime snapshot is missing the typed viewport source registry");
  }
  const entry = state.viewportSources.find((candidate) =>
    candidate?.id?.component === id.component && candidate?.id?.source === id.source
  );
  if (!entry || !entry.state || typeof entry.state !== "object" || Array.isArray(entry.state)) {
    throw new Error(`Viewport source is missing from the runtime registry: ${id.component}/${id.source}`);
  }
  return entry.state;
=======
function presentationDefinitionForLayer(layer) {
  const presentation = layer?.presentation;
  if (!presentation || typeof presentation !== "object") {
    throw new Error(`Surface component ${String(layer?.id || layer?.definition || "unknown")} is missing its resolved presentation contract`);
  }
  if (presentation.error) {
    throw new Error(String(presentation.error));
  }
  return presentation;
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
  return Boolean(component.children?.some((child) => componentHasKind(child, kind)));
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
  return Boolean(component?.children?.some((child) => componentContainsSizingKind(child, sizing)));
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
  }
  return null;
}

function currentSceneHasPuzzle() {
  return currentSceneAcceptsModelInput();
}

function currentSceneAcceptsModelInput() {
  return stateAcceptsModelInput(currentState);
}

function isControlPointerTarget(target) {
  return Boolean(target?.closest?.("button, a, input, select, textarea, [role='button'], [role='option']"));
}

function sceneInteractionProfile(scene = focusedComponentDefinition(), options = {}) {
  const state = options.state || currentState || {};
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  const standardChoices = resolvedChoiceNodes(scene?.components || []);
=======
  const layer = scene
    ? surfaceComponents(state).find((candidate) => candidate?.focused === true) || null
    : null;
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  return {
    acceptsModelInput: stateAcceptsModelInput(state),
    hasStandardChoice: Number.isInteger(layer?.choiceCursor),
  };
}

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
function resolvedChoiceNodes(components, choices = []) {
  for (const component of components || []) {
    if (component.kind === "choice") {
      choices.push(component);
    }
    resolvedChoiceNodes(component.children || [], choices);
  }
  return choices;
=======
function currentSceneLayer(state = currentState, scene = focusedComponentDefinition()) {
  const layers = surfaceComponents(state || {});
  return layers.find((layer) => scene?.name && layer.name === scene.name)
    || layers.find((layer) => layer.focused === true)
    || layers[0]
    || null;
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
}

function stateAcceptsModelInput(state = currentState) {
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
    default:
      throw new Error(`Unsupported scene component: ${String(component.kind || "unknown")}`);
  }
}

function renderPuzzle(component, scope = {}) {
  const sourceId = runtimeViewportSourceId(component.source);
  const scene = runtimeViewportSourceState(sourceId);
  const root = document.createElement("div");
  root.className = "board";
  root.dataset.frameComponent = "true";
  root.dataset.source = sourceId.source;
  root.dataset.scene = sourceId.component;
  const key = `${sourceId.component}:${sourceId.source}`;
  const renderer = new window.PuzzleRenderer(root, {
    renderMode: "canvas",
    prepareRenderScene: prepareResolvedRenderScene,
    resolveRenderMoment,
    onError: showError,
  });
  renderer.viewport = puzzleViewports.get(key);
  renderer.render(scene);
  puzzleViewports.set(key, renderer.viewport);
  return root;
}

/* puzzle-host:optional:puzzle3:start */
function renderPuzzle3Frame(component, scope = {}) {
  if (!window.Puzzle3Component) {
    throw new Error("Puzzle3 component assets are unavailable.");
  }
  const sourceId = runtimeViewportSourceId(component.source);
  const sceneName = sourceId.component;
  const source = sourceId.source;
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
    const snapshot = puzzle3FrameSnapshot(sceneName, source);
    const controller = window.Puzzle3Component.attach(canvas, {
      screenView: root,
      snapshot,
      scene: sceneName,
      component,
      prepareRenderScene: prepareResolvedRenderScene,
      resolveRenderMoment,
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

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
function puzzle3FrameFixture(sceneName, source = "board") {
  const fixture = JSON.parse(JSON.stringify(window.Puzzle3DFrameFixture));
=======
function puzzle3FrameSnapshot(sceneName, source = "board") {
  const sessionSnapshot = currentState?.scenePuzzleState?.[source];
  if (sessionSnapshot && typeof sessionSnapshot === "object") {
    return sessionSnapshot;
  }
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  if (puzzle3PreviewSurface) {
    return puzzle3PreviewSurfaceFixture(puzzle3PreviewTemplate(), sceneName);
  }
  const sessionSnapshot = runtimeViewportSourceState({
    component: sceneName,
    source,
  });
  return mergePuzzle3SessionSnapshot(window.Puzzle3DFrameFixture, sessionSnapshot);
}

function puzzle3PreviewTemplate() {
  const fixture = window.Puzzle3DFrameFixture;
  if (!fixture || typeof fixture !== "object") {
    throw new Error("Puzzle3 visual preview template is unavailable.");
  }
  return JSON.parse(JSON.stringify(fixture));
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
  const component = update.component || update.modelComponent;
  if (!component || typeof component !== "object" || component.kind !== "puzzle3") {
    throw new Error("Puzzle3 preview update is missing its typed component");
  }
  const source = runtimeViewportSourceId(component.source);
  if (update.scene !== undefined && update.scene !== source.component) {
    throw new Error(
      `Puzzle3 preview scene ${String(update.scene)} does not match source component ${source.component}`,
    );
  }
  return {
    update,
    sceneName: source.component,
    payload,
    component: {
      kind: "puzzle3",
      source,
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
    puzzle3PreviewTemplate(),
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
  const previewSceneName = sceneName || surface.sceneName;
  if (!previewSceneName || !surface.component) {
    throw new Error("Puzzle3 preview surface is missing its typed component identity");
  }
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
      presentation: {
        name: previewSceneName,
        layout: {},
        components: [surface.component],
      },
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  const source = entry.root.dataset.source;
  if (!sceneName || !source) {
    throw new Error("Puzzle3 viewport controller is missing its typed source identity");
  }
  const snapshot = puzzle3FrameFixture(sceneName, source);
=======
  const source = entry.root.dataset.source || "board";
  const snapshot = puzzle3FrameSnapshot(sceneName, source);
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  text.textContent = component.value || "";
=======
  text.textContent = String(component.value || "");
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    component.label,
=======
    String(component.label || "") || sceneTitle(effectLabel(component.effect)),
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  );
  annotateSceneEditorComponent(button, component, scope);
  button.addEventListener("click", () => {
    if (selectSceneEditorComponent(component, scope)) {
      return;
    }
    sendSceneActionToken(component.actionToken);
  });
  button.disabled = !component.actionToken;
  applySizingKind(button, component);
  applySceneLayout(button, component.layout);
  return button;
}

function renderChoice(component, scope = {}) {
  const choice = document.createElement("button");
  choice.type = "button";
  setControlLabel(
    choice,
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    component.label,
  );
  choice.classList.add("standard-choice");
  annotateSceneEditorComponent(choice, component, scope);
  choice.classList.toggle("is-selected", component.selected === true);
=======
    String(component.label || ""),
  );
  choice.classList.add("standard-choice");
  annotateSceneEditorComponent(choice, component, scope);
  const counter = scope.__standardChoiceCounter || { value: 0 };
  scope.__standardChoiceCounter = counter;
  const index = counter.value;
  counter.value += 1;
  choice.dataset.standardChoiceIndex = String(index);
  choice.classList.toggle(
    "is-selected",
    scope.__sceneLayer?.focused === true && index === Number(scope.__sceneLayer?.choiceCursor),
  );
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  choice.addEventListener("click", () => {
    if (selectSceneEditorComponent(component, scope)) {
      return;
    }
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    sendSceneActionToken(component.actionToken);
=======
    runActivationConfirm(choice, () => sendChoiceActivate(index));
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  });
  choice.disabled = !component.actionToken;
  applySizingKind(choice, component);
  applySceneLayout(choice, component.layout);
  return choice;
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
  control.textContent = label;
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
  void renderSceneEditorPreview(sceneEditorPreview).catch((error) => showError(error));
  return true;
}

function sameSceneEditorPath(left, right) {
  return JSON.stringify(left || []) === JSON.stringify(right || []);
}
/* puzzle-host:optional:scene-editor:end */

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

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
function renderConditional(component, scope = {}) {
  const fragment = document.createDocumentFragment();
  if (typeof component.condition !== "boolean") {
    throw new Error("Resolved scene conditional is missing its boolean condition");
  }
  const conditionTrue = component.condition;
  const children = conditionTrue
    ? component.children || []
    : component.elseChildren || [];
  renderSurfaceComponents(children, fragment, {
    ...scope,
    __componentPath: [...(scope.__componentPath || []), conditionTrue ? "children" : "elseChildren"],
  });
  return fragment;
}

>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
  }
  return null;
}

function focusShell() {
  if (!shell || document.activeElement === shell || shell.contains(document.activeElement)) {
    return;
  }
  shell.focus({ preventScroll: true });
}

function runtimeKeyTriggerFromEvent(event) {
  if (event.altKey || event.ctrlKey || event.metaKey) {
    return null;
  }
  const key = String(event.key || "");
  const named = {
    ArrowUp: "arrow_up",
    ArrowDown: "arrow_down",
    ArrowLeft: "arrow_left",
    ArrowRight: "arrow_right",
    Enter: "enter",
    " ": "space",
    Escape: "escape",
    Tab: "tab",
    Backspace: "backspace",
  };
  if (named[key]) {
    return { kind: named[key] };
  }
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  return [...key].length === 1 ? { kind: "character", value: key } : null;
=======
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
  const scene = focusedComponentDefinition();
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
  if (standardInput && profile.hasStandardChoice) {
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
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
}

function inputByName(name) {
  if (!currentState) {
    return null;
  }
  return currentState.inputs.find((input) => input.name === name);
}

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
function currentSceneDef() {
  const source = currentState;
=======
function focusedComponentDefinition() {
  const source = currentState || puzzleBoot || {};
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  const name = focusedComponentName(source);
  return componentDefinitionByName(name, source);
}

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
function componentDefinitionByName(name, source = currentState) {
  const layer = sceneLayers(source).find((candidate) => candidate.id === name);
  return layer?.presentation || null;
}

function sceneDefByName(name) {
  const source = currentState;
  return componentDefinitionByName(name, source);
}

function nonEmptyArray(value) {
  return Array.isArray(value) && value.length > 0 ? value : null;
}

async function sendResolvedInput(input) {
=======
function componentDefinitionByName(name, source = currentState || puzzleBoot || {}) {
  const layer = Array.isArray(source?.surface?.components)
    ? source.surface.components.find((candidate) => (
      candidate.id === name
      || candidate.name === name
      || candidate.definition === name
    ))
    : null;
  if (layer) {
    return presentationDefinitionForLayer(layer);
  }
  return null;
}

async function sendResolvedInput(input) {
  if (input?.kind === "standard_choice") {
    if (input.input === "enter") {
      const selectedChoice = document.querySelector(".scene-layer.is-focused .standard-choice.is-selected");
      runActivationConfirm(selectedChoice, () => sendChoiceActivate());
    } else {
      await postSessionAction({ kind: "choice_move", direction: input.input });
    }
    return;
  }
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  if (input?.kind === "model_input") {
    await sendModelInput(input.name);
    return;
  }
  if (input?.kind === "session_action") {
    await postSessionAction(input.action);
    return;
  }
  throw new Error(`Unsupported resolved input: ${String(input?.kind || "unknown")}`);
}

async function sendSceneActionToken(token) {
  if (!token || typeof token !== "object" || Array.isArray(token)) {
    throw new Error("Resolved scene action token is unavailable.");
  }
  await postSessionAction({ kind: "scene_action", token });
}

async function sendChoiceActivate(index = null) {
  const action = { kind: "choice_activate" };
  if (Number.isInteger(index)) {
    action.index = index;
  }
  await postSessionAction(action);
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
      reportPresentationEventConsumed();
    } else {
      throw new Error(`Unsupported presentation event: ${String(event.kind || "unknown")}`);
    }
  }
  dispatchingPresentationEvents = false;
  if (pendingSessionResume) {
    resumePendingSessionTurn();
    return;
  }
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    await postSessionAction({ kind: "resume" });
=======
    render(await requestSessionAction({ kind: "resume" }));
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  } catch (error) {
    if (currentState) {
      currentState.busy = false;
    }
    showError(error);
  } finally {
    resumingSession = false;
  }
}

function applyPresentationAnimations(event, animations) {
  if (!currentState || currentState.levelIndex !== event.levelIndex) {
    return;
  }
  const puzzleSnapshot = runtimeViewportSourceState(event.source);
  const batchId = ++presentationAnimationBatchId;
  puzzleSnapshot.animationEvents = animations;
  puzzleSnapshot.animationBatchId = batchId;
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
  if (currentState.scene) {
    currentState.scene.animationEvents = animations;
    currentState.scene.animationBatchId = batchId;
  }
  const layer = surfaceComponents(currentState).find((candidate) =>
    (candidate?.name === event.scene || candidate?.scene?.name === event.scene)
      && (candidate?.scenePuzzles || []).includes(event.puzzle)
  );
  if (layer?.scene) {
    layer.scene.animationEvents = animations;
    layer.scene.animationBatchId = batchId;
  }
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
    fastForwardRequested: false,
    config,
  };
  waitTimer.complete = () => {
    if (waitTimer.done) {
      return;
    }
    waitTimer.done = true;
    reportPresentationEventConsumed();
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
  const source = currentState?.inputBuffer;
  if (
    !source
    || typeof source.queueDuringWait !== "boolean"
    || typeof source.fastForwardWait !== "boolean"
    || !Number.isFinite(source.minWaitMs)
    || source.minWaitMs < 0
  ) {
    throw new Error("Runtime snapshot is missing the required inputBuffer contract");
  }
  return {
    queueDuringWait: source.queueDuringWait,
    fastForwardWait: source.fastForwardWait,
    minWaitMs: source.minWaitMs,
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
    const queued = sendModelInputNow(input);
    if (config.queueDuringWait) {
      fastForwardActiveWaitsForQueuedInput(config);
    }
    return queued;
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
  return postSessionAction({ kind: "input", name: input });
}

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
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
=======
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
    return Array.isArray(expr.path) ? expr.path.join(".") : String(expr.path || "");
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
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
}

function dispatchKeyboardInput(event) {
  if (!currentState) {
    return false;
  }
  const trigger = runtimeKeyTriggerFromEvent(event);
  if (!trigger) {
    return false;
  }
  postSessionAction({ kind: "key", trigger });
  return true;
}

document.addEventListener("keydown", (event) => {
  if (componentEmbedMode) {
    return;
  }
  if (dispatchKeyboardInput(event)) {
    event.preventDefault();
  }
});

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
    await renderSceneEditorPreview(event.data || {});
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

loadState()
  .then(async () => {
    await setAudioVisible(document.visibilityState !== "hidden");
  })
  .catch((error) => {
    showError(error);
  });

document.addEventListener("visibilitychange", async () => {
  await setAudioVisible(document.visibilityState !== "hidden");
});

if (!componentEmbedMode) {
  document.addEventListener("pointerdown", focusShell);
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
