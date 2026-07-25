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
const puzzleBoot = window.PuzzleBoot || {};
const standaloneRuntime = window.PuzzleStandaloneRuntime
  ? new window.PuzzleStandaloneRuntime(puzzleBoot, window.PuzzleRuntimeExportJson)
  : null;
const reportedAudioConsumerErrors = new Set();

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
  try {
    render(await requestJson("/api/state"));
  } catch (error) {
    showError(error);
  }
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
  fitPuzzleFrameComponents(screenView);
}

function currentSceneAspectRatio() {
  if (sceneEditorPreview?.layout?.aspectRatio) {
    return normalizedAspectRatio(sceneEditorPreview.layout.aspectRatio);
  }
  const layers = sceneLayers(currentState);
  const layer = layers.find((candidate) => candidate.id === currentState?.surface?.focus) || layers[0];
  return normalizedAspectRatio(layer?.presentation?.layout?.aspectRatio);
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
  const sceneDef = sceneDefByName(sceneName);
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

function renderSceneEditorPreview(config = {}) {
  const sceneName = String(config.scene?.name || config.sceneName || focusedComponentName(currentState)).trim();
  const sceneDef = config.presentation || sceneDefByName(sceneName);
  sceneEditorPreview = {
    requestId: String(config.requestId || ""),
    sceneName,
    theme: normalizeScenePreviewTheme(config.theme ?? currentState?.theme),
    layout: normalizeScenePreviewLayout(config.layout),
    state: normalizeScenePreviewState(config.state),
    inspect: config.inspect || {},
  };
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
  const layer = sceneLayers(state)[0];
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
    meta.label = component.label || "";
  } else if (component.kind === "text") {
    meta.label = component.value || "";
  } else if (component.source) {
    meta.source = component.source;
  }
  return meta;
}
/* puzzle-host:optional:scene-editor:end */

function renderSurface(state) {
  screenView.replaceChildren();

  const layers = sceneLayers(state);
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
    const presentation = layer.presentation;
    if (!presentation || typeof presentation !== "object") {
      throw new Error(`Presented component ${String(layer.id || "")} is missing its resolved presentation.`);
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
  const layer = layers.find((candidate) => candidate.id === currentState?.surface?.focus) || layers[0];
  const sceneDef = layer?.presentation;
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

function sceneLayers(state) {
  if (!state?.surface || !Array.isArray(state.surface.components)) {
    throw new Error("Runtime snapshot is missing the required surface component contract");
  }
  return state.surface.components;
}

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

function sceneInteractionProfile(scene = currentSceneDef(), options = {}) {
  const state = options.state || currentState || {};
  const standardChoices = resolvedChoiceNodes(scene?.components || []);
  return {
    acceptsModelInput: stateAcceptsModelInput(state),
    standardChoices,
  };
}

function resolvedChoiceNodes(components, choices = []) {
  for (const component of components || []) {
    if (component.kind === "choice") {
      choices.push(component);
    }
    resolvedChoiceNodes(component.children || [], choices);
  }
  return choices;
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
  const fixture = JSON.parse(JSON.stringify(window.Puzzle3DFrameFixture));
  if (puzzle3PreviewSurface) {
    return puzzle3PreviewSurfaceFixture(fixture, sceneName);
  }
  const sessionSnapshot = runtimeViewportSourceState({
    component: sceneName,
    source,
  });
  return mergePuzzle3SessionSnapshot(window.Puzzle3DFrameFixture, sessionSnapshot);
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
  const source = entry.root.dataset.source;
  if (!sceneName || !source) {
    throw new Error("Puzzle3 viewport controller is missing its typed source identity");
  }
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
  text.textContent = component.value || "";
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
    component.label,
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
    component.label,
  );
  choice.classList.add("standard-choice");
  annotateSceneEditorComponent(choice, component, scope);
  choice.classList.toggle("is-selected", component.selected === true);
  choice.addEventListener("click", () => {
    if (selectSceneEditorComponent(component, scope)) {
      return;
    }
    sendSceneActionToken(component.actionToken);
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
  renderSceneEditorPreview(sceneEditorPreview);
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
  return [...key].length === 1 ? { kind: "character", value: key } : null;
}

function inputByName(name) {
  if (!currentState) {
    return null;
  }
  return currentState.inputs.find((input) => input.name === name);
}

function currentSceneDef() {
  const source = currentState;
  const name = focusedComponentName(source);
  return componentDefinitionByName(name, source);
}

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
    await postSessionAction({ kind: "resume" });
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
  if (!currentState || currentState.levelIndex !== event.levelIndex) {
    return;
  }
  const puzzleSnapshot = runtimeViewportSourceState(event.source);
  const batchId = ++presentationAnimationBatchId;
  puzzleSnapshot.animationEvents = animations;
  puzzleSnapshot.animationBatchId = batchId;
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
  return postSessionAction({ kind: "input", name: input });
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
