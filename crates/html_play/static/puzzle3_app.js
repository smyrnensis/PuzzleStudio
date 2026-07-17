(() => {
function createPuzzle3Controller(options = {}) {
const controllerOptions = options && typeof options === "object" ? options : {};
const sessionManaged = controllerOptions.sessionManaged === true;
const inlineComponentMount = Boolean(controllerOptions.canvas) || controllerOptions.mountMode === "inline";
const screenView = controllerOptions.screenView || controllerOptions.container || document.querySelector("#screenView") || document.body;
const componentEmbedMode = Boolean(controllerOptions.componentEmbedMode)
  || (!inlineComponentMount && (
    new URLSearchParams(window.location.search).get("component") === "1"
    || window.Puzzle3DComponentEmbed === true
  ));
const PREVIEW_SURFACE_UPDATE_MESSAGE = "PuzzleStudioPreviewSurfaceUpdate";
const PUZZLE3_LEVEL_PREVIEW_KIND = "puzzle3-level";
const ISOLATED_PREVIEW_MODE = "isolated";
let editorComponentEmbedMode = false;
if (!inlineComponentMount) {
  document.documentElement.classList.toggle("is-component-embed", componentEmbedMode);
}
let activeThemeClass = "";
const activeThemeVariables = new Set();
if (!inlineComponentMount) {
  applyTheme({ name: "clean" });
  document.body.classList.toggle("is-component-embed", componentEmbedMode);
}
const puzzle3Frame = ensurePuzzle3ComponentFrame();
const canvas = puzzle3Frame.querySelector("#view");
const puzzle3RendererMode = resolvePuzzle3RendererMode(
  controllerOptions.renderer
    || controllerOptions.rendererMode
    || new URLSearchParams(window.location.search).get("puzzle3Renderer")
    || new URLSearchParams(window.location.search).get("renderer"),
);
const ctx = puzzle3RendererMode === "three" ? null : canvas.getContext("2d", { alpha: true });
const PUZZLE3_RUNTIME_CONTRACT_VERSION = 6;
const PUZZLE3_RENDERER_CONTRACT_VERSION = 1;
const PUZZLE3_APP_CAMERA_MIN_PITCH_DEGREES = -90;
const PUZZLE3_APP_CAMERA_MAX_PITCH_DEGREES = 90;

function ensurePuzzle3ComponentFrame() {
  let existing = controllerOptions.canvas || document.querySelector("#view");
  let frame = existing?.closest(".puzzle3-component")
    || (inlineComponentMount ? existing?.parentElement : null)
    || document.createElement("div");
  frame.className = "puzzle3-component";
  if (!existing) {
    existing = document.createElement("canvas");
    existing.id = "view";
    existing.width = 960;
    existing.height = 640;
    existing.setAttribute("aria-label", "Puzzle3 component");
  } else if (!existing.id) {
    existing.id = "view";
  }
  if (existing.parentElement !== frame) {
    frame.append(existing);
  }
  return frame;
}

const fallbackSnapshot = {
  size: { width: 3, depth: 3, height: 3 },
  view: {
    zoom: 1,
  },
  render: {
    camera: {
      yawDegrees: 15,
      pitchDegrees: 55,
      rollDegrees: 0,
      zoom: 1,
      interactiveLook: false,
      interactiveZoom: false,
    },
    grid: { visibility: 0 },
    sprite: { shade: true },
    shadow: false,
    pixelate: { enabled: false, scale: 1, smoothing: true },
    animation: { tween: { enabled: false, intervalMs: 250 } },
    viewport: null,
  },
  directions: {
    left: { dx: -1, dy: 0, dz: 0 },
    right: { dx: 1, dy: 0, dz: 0 },
    front: { dx: 0, dy: 1, dz: 0 },
    back: { dx: 0, dy: -1, dz: 0 },
    up: { dx: 0, dy: 0, dz: 1 },
    down: { dx: 0, dy: 0, dz: -1 },
  },
  directionSets: {
    horizontal: ["left", "right", "front", "back"],
    vertical: ["up", "down"],
  },
  controls: {
    keys: {
      ArrowLeft: "left",
      ArrowRight: "right",
      ArrowUp: "front",
      ArrowDown: "back",
    },
  },
  sprites: {},
  cells: [],
};

const view = {
  cellScale: 78,
  originX: 0,
  originY: 0,
  dragging: false,
  pointerId: null,
  lastPointerX: 0,
  lastPointerY: 0,
  shadowsEnabled: false,
  projectionFitKey: "",
  projectionWidth: 0,
  projectionHeight: 0,
  viewportSnapNext: true,
  viewportAnimationFrame: 0,
  primitiveSortCacheKey: "",
  primitiveSortCacheOrder: [],
};
let snapshot = fallbackSnapshot;
let snapshotLoaded = false;
let runtime = null;
let initialCamera = cloneCamera(fallbackSnapshot.render.camera);
let currentSceneName = initialSceneName(fallbackSnapshot);
let editorModelComponentPreview = null;
let sceneEditorPreview = null;
let queuedSceneInputs = [];
let queuedSceneInputFrame = 0;
const heldSceneInputs = new Map();
const SCENE_DEFAULT_WIDTH = 16;
const SCENE_DEFAULT_HEIGHT = 12;
const spriteVoxelTemplateCache = new WeakMap();
const renderGeometryCache = createRenderGeometryCache();
const pixelateBuffer = document.createElement("canvas");
let mountedPuzzle3Component = null;
let pendingResizeFrame = 0;
let pendingSceneLayoutRender = false;
let startupUrlCommandsApplied = false;
let puzzle3ThreeRenderer = null;
let puzzle3ThreeViewPayload = null;
const viewListeners = new Set();
const stateListeners = new Set();
const puzzle3Component = createPuzzle3Component();

async function loadSnapshot() {
  const nextSnapshot = await loadInitialPuzzle3Snapshot();
  const initialModelPreview = window.PuzzleStudioInitialModelComponentPreview;
  if (initialModelPreview?.type === "PuzzleStudioRenderPuzzle3ModelComponent") {
    window.PuzzleStudioInitialModelComponentPreviewConsumed = true;
    const next = puzzle3PreviewSnapshot(initialModelPreview, nextSnapshot);
    await loadSnapshotData(next, puzzle3ModelComponentPreviewLoadOptions(initialModelPreview));
    return;
  }
  await loadSnapshotData(nextSnapshot);
}

async function loadInitialPuzzle3Snapshot() {
  if (controllerOptions.fixture) {
    return controllerOptions.fixture;
  }
  if (window.Puzzle3DFixture) {
    return window.Puzzle3DFixture;
  }
  const response = await fetch("./fixture.json", { cache: "no-store" });
  if (!response.ok) {
    const status = `${response.status} ${response.statusText || ""}`.trim();
    throw new Error(`Could not load Puzzle3 fixture ./fixture.json (${status})`);
  }
  return response.json();
}

function requirePuzzle3Snapshot(source, label = "Puzzle3 snapshot") {
  if (!source || typeof source !== "object" || Array.isArray(source)) {
    throw new Error(`${label} is missing or invalid.`);
  }
  if (!source.render || typeof source.render !== "object" || Array.isArray(source.render)) {
    throw new Error(`${label}.render is missing or invalid.`);
  }
  if (!source.render.camera || !source.render.animation?.tween) {
    throw new Error(`${label}.render is missing camera or animation data.`);
  }
  return source;
}

function requireLoadedPuzzle3Snapshot(label = "Puzzle3 snapshot") {
  if (!snapshotLoaded) {
    throw new Error(`${label} is not loaded.`);
  }
  return requirePuzzle3Snapshot(snapshot, label);
}

async function loadSnapshotData(source, options = {}) {
  const previousCamera = snapshotLoaded ? cloneCamera(snapshot.render.camera) : null;
  snapshotLoaded = false;
  snapshot = normalizeSnapshot(requirePuzzle3Snapshot(source));
  let startupEffects = [];
  if (sessionManaged) {
    runtime = null;
    if (previousCamera && options.preserveCamera !== false) {
      snapshot.render.camera = previousCamera;
    }
  } else {
    runtime = await createPuzzle3Runtime(snapshot);
    snapshot = runtime.snapshot();
    startupEffects = runtime.takeLevelLoadEffects();
  }
  snapshotLoaded = true;
  editorModelComponentPreview = options.modelComponentPreview || null;
  document.title = snapshot.title || "Puzzle3";
  currentSceneName = editorModelComponentPreview?.sceneName
    || options.scene
    || controllerOptions.scene
    || (options.preferPuzzleScene ? puzzleSceneName(snapshot) : "")
    || initialSceneName(snapshot);
  if (!inlineComponentMount) {
    applyTheme(snapshot.theme || { name: "clean" });
  }
  initialCamera = cloneCamera(snapshot.render.camera);
  view.projectionFitKey = "";
  resetRenderGeometryCache();
  resetViewportMotion();
  renderScene();
  applyStartupPuzzle3UrlCommands();
  notifyPuzzle3StateChange();
  emitPuzzle3LifecycleEffects(startupEffects);
}

function showPuzzle3LoadError(error) {
  showPuzzle3FatalError("fixture load failed", error);
}

function showPuzzle3RenderError(error) {
  showPuzzle3FatalError("render failed", error);
}

function showPuzzle3FatalError(label, error) {
  console.error(error);
  const errorView = document.createElement("div");
  errorView.className = "puzzle3-load-error";
  errorView.setAttribute("role", "alert");
  errorView.textContent = `Puzzle3 ${label}: ${error?.message || error || "unknown error"}`;
  Object.assign(errorView.style, {
    boxSizing: "border-box",
    width: "100%",
    minHeight: "100%",
    padding: "24px",
    display: "grid",
    placeItems: "center",
    color: "#8a1f11",
    background: "#fff5f2",
    border: "1px solid #f0b8a8",
    fontFamily: "system-ui, sans-serif",
    fontSize: "14px",
    lineHeight: "1.5",
    textAlign: "center",
    whiteSpace: "pre-wrap",
  });
  screenView.replaceChildren(errorView);
}

function loadPuzzle3ControllerSnapshot() {
  return loadSnapshot().catch((error) => {
    showPuzzle3LoadError(error);
    throw error;
  });
}

function applyTheme(theme) {
  if (activeThemeClass) {
    document.body.classList.remove(activeThemeClass);
  }
  for (const variable of activeThemeVariables) {
    document.documentElement.style.removeProperty(variable);
  }
  activeThemeVariables.clear();
  const name = normalizeScenePreviewTheme(theme)?.name || "clean";
  activeThemeClass = `theme-${name}`;
  document.body.classList.add(activeThemeClass);
  const variables = theme && typeof theme === "object" ? theme.variables : null;
  if (variables && typeof variables === "object") {
    for (const [key, value] of Object.entries(variables)) {
      const variable = normalizeThemeVariableKey(key);
      if (!variable) {
        continue;
      }
      document.documentElement.style.setProperty(variable, String(value));
      activeThemeVariables.add(variable);
    }
  }
}

function normalizeThemeVariableKey(name) {
  const normalized = String(name || "")
    .replace(/^--/, "")
    .replace(/_/g, "-")
    .toLowerCase();
  if (normalized === "bg") {
    return "--background";
  }
  if (normalized === "ink") {
    return "--text";
  }
  return /^(background|text|accent)$/.test(normalized) ? `--${normalized}` : "";
}

function normalizeScenePreviewTheme(theme) {
  if (!theme) {
    return null;
  }
  if (typeof theme === "string") {
    const name = normalizeThemeName(theme);
    return name ? { name } : null;
  }
  if (typeof theme !== "object") {
    return null;
  }
  const name = normalizeThemeName(theme.name || theme.id || "clean");
  if (!name) {
    return null;
  }
  return {
    name,
    variables: normalizeThemeVariables(theme.variables),
  };
}

function normalizeThemeVariables(variables) {
  if (Array.isArray(variables)) {
    return Object.fromEntries(variables.map((variable, index) => {
      if (!variable || typeof variable !== "object" || Array.isArray(variable)) {
        throw new Error(`Puzzle3 theme.variables[${index}] is invalid.`);
      }
      if (typeof variable.name !== "string" || variable.name.length === 0) {
        throw new Error(`Puzzle3 theme.variables[${index}].name must be a non-empty string.`);
      }
      if (typeof variable.value !== "string") {
        throw new Error(`Puzzle3 theme.variables[${index}].value must be a string.`);
      }
      return [variable.name, variable.value];
    }));
  }
  return variables && typeof variables === "object" ? variables : {};
}

function normalizeThemeName(name) {
  return String(name || "")
    .trim()
    .replace(/^theme-/, "")
    .replace(/[^a-zA-Z0-9_-]/g, "")
    || "clean";
}

function resolvePuzzle3RendererMode(value) {
  return normalizePuzzle3RendererMode(value);
}

function normalizePuzzle3RendererMode(value) {
  const text = String(value || "").trim().toLowerCase();
  return text === "canvas" ? "canvas" : "three";
}

async function createPuzzle3Runtime(initialSnapshot) {
  const fixtureJson = JSON.stringify(requirePuzzle3Snapshot(initialSnapshot, "Puzzle3 runtime fixture"));
  const runtimeContract = requireRuntimeContract(initialSnapshot);
  const module = await window.PuzzleRuntimeWasmLoader.load(
    String(runtimeContract.version),
  );
  if (typeof module.WasmPuzzle3Runtime?.fromFixture !== "function") {
    throw new Error("Puzzle3 source-free WASM runtime is unavailable.");
  }
  return new Puzzle3SessionRuntime(initialSnapshot, module.WasmPuzzle3Runtime.fromFixture(fixtureJson));
}

class Puzzle3SessionRuntime {
  constructor(initialSnapshot, coreRuntime) {
    this.base = cloneRuntimeSnapshot(initialSnapshot);
    this.runtimeContract = this.base.runtimeContract;
    this.runtimeGame = requireRuntimeContractGame(this.runtimeContract);
    this.runtimeLevelBundle = requireRuntimeContractLevelBundle(this.runtimeContract);
    this.runtimeLayerCountValue = runtimeLayerCount(this.runtimeGame);
    this.semanticObjectsById = runtimeSemanticObjectsById(this.runtimeGame);
    this.presentationObjectsById = runtimePresentationObjectsById(this.base.objects);
    this.coreRuntime = coreRuntime;
    this.camera = cloneCamera(initialSnapshot.render.camera);
    this.levels = cloneRuntimeContractLevels(this.runtimeLevelBundle, initialSnapshot.levels || []);
    this.levelIndex = clampIndex(initialSnapshot.levelIndex || 0, this.levels.length);
    this.undoStack = [];
    this.moveCount = 0;
    this.cellsByKey = new Map();
    this.cells = [];
    this.animationEvents = [];
    this.initialStateHandle = null;
    this.completed = false;
    this.levelLoadEffects = this.loadLevel(this.levelIndex);
  }

  snapshot() {
    const level = this.currentLevel();
    return {
      ...this.base,
      size: { ...level.size },
      render: {
        ...this.base.render,
        camera: cloneCamera(this.camera),
      },
      cells: this.cells.map((cell) => ({
        position: { ...cell.position },
        objects: cell.objects.map((object) => ({ ...object })),
      })),
      animationEvents: this.animationEvents.map((event) => ({ ...event })),
      levelIndex: this.levelIndex,
      levelCount: this.levels.length,
      levelName: level.name,
      levelLabel: level.label || level.name,
      hasNextLevel: this.hasNextLevel(),
      hasPreviousLevel: this.hasPreviousLevel(),
      moveCount: this.moveCount,
      completed: this.completed,
    };
  }

  applyInput(inputName) {
    const inputId = this.inputIdForName(inputName);
    const before = this.historyEntry();
    const outcome = this.transitionCurrent("main", inputId);
    const effects = cloneRequiredJsonArray(outcome.effects, "runtime current outcome.effects");
    const handled = outcome.changed === true
      || effects.length > 0
      || cloneRequiredJsonArray(outcome.commands, "runtime current outcome.commands").length > 0
      || cloneRequiredJsonArray(outcome.firedRules, "runtime current outcome.firedRules").length > 0;
    if (!handled) {
      return false;
    }
    if (outcome.changed === true) {
      this.undoStack.push(before);
    }
    this.animationEvents = cloneRequiredJsonArray(
      outcome.animationEvents,
      "runtime current outcome.animationEvents",
    );
    this.applyRuntimeCells(cloneRequiredJsonArray(
      outcome.changedCells,
      "runtime current outcome.changedCells",
    ));
    this.moveCount += 1;
    const wasCompleted = this.completed;
    this.completed = outcome.completed === true;
    if (!wasCompleted && this.completed) {
      effects.push(...this.runLevelClearLifecycle());
    }
    return { handled: true, effects };
  }

  setCamera(camera) {
    this.camera = cloneCamera(camera || this.camera);
  }

  undo() {
    const previous = this.undoStack.pop();
    if (!previous) {
      return false;
    }
    this.coreRuntime.restore_saved_state(Number(previous.handle));
    this.loadCellsFromRuntime();
    this.animationEvents = [];
    this.moveCount = previous.moveCount;
    this.completed = previous.completed;
    return true;
  }

  restart() {
    const changed = this.moveCount !== 0
      || this.completed
      || this.undoStack.length > 0;
    if (!changed) {
      return false;
    }
    this.undoStack.push(this.historyEntry());
    this.coreRuntime.restore_saved_state(Number(this.initialStateHandle));
    this.loadCellsFromRuntime();
    this.animationEvents = [];
    this.moveCount = 0;
    this.completed = this.coreRuntime.is_current_complete() === true;
    return true;
  }

  historyEntry() {
    return {
      handle: this.coreRuntime.save_current_state(),
      moveCount: this.moveCount,
      completed: this.completed,
    };
  }

  nextLevel() {
    if (!this.hasNextLevel()) {
      return false;
    }
    return this.loadLevel(this.levelIndex + 1);
  }

  previousLevel() {
    if (!this.hasPreviousLevel()) {
      return false;
    }
    return this.loadLevel(this.levelIndex - 1);
  }

  hasNextLevel() {
    return this.levelIndex + 1 < this.levels.length;
  }

  hasPreviousLevel() {
    return this.levelIndex > 0;
  }

  loadLevel(levelIndex) {
    this.levelIndex = clampIndex(levelIndex, this.levels.length);
    const effects = this.loadInitialStateForCurrentLevel();
    this.undoStack = [];
    this.moveCount = 0;
    this.animationEvents = [];
    this.completed = this.coreRuntime.is_current_complete() === true;
    this.initialStateHandle = this.coreRuntime.save_current_state();
    return effects;
  }

  loadInitialStateForCurrentLevel() {
    const level = this.currentLevel();
    const raw = stateFromRuntimeCells(this, level.cells, level.size, level.variables);
    this.coreRuntime.set_state(JSON.stringify(raw));
    this.cellsByKey.clear();
    this.cells = [];
    this.applyRuntimeCells(level.cells);
    const outcome = this.transitionCurrent("level_start", 0);
    this.animationEvents = [];
    this.applyRuntimeCells(cloneRequiredJsonArray(
      outcome.changedCells,
      "runtime level_start outcome.changedCells",
    ));
    this.completed = outcome.completed === true;
    return cloneRequiredJsonArray(outcome.effects, "runtime level_start outcome.effects");
  }

  takeLevelLoadEffects() {
    const effects = this.levelLoadEffects;
    this.levelLoadEffects = [];
    return effects;
  }

  currentLevel() {
    return this.levels[this.levelIndex];
  }

  runLevelClearLifecycle() {
    const programKey = this.levelIndex + 1 >= this.levels.length
      ? "last_level_clear"
      : "level_clear";
    const outcome = this.transitionCurrent(programKey, 0);
    this.animationEvents.push(...cloneRequiredJsonArray(
      outcome.animationEvents,
      `runtime ${programKey} outcome.animationEvents`,
    ));
    this.applyRuntimeCells(cloneRequiredJsonArray(
      outcome.changedCells,
      `runtime ${programKey} outcome.changedCells`,
    ));
    return cloneRequiredJsonArray(outcome.effects, `runtime ${programKey} outcome.effects`);
  }

  transitionCurrent(programKey, inputId) {
    const raw = this.coreRuntime.transition_current_outcome(
      programKey,
      this.levelIndex,
      Number(inputId || 0),
    );
    return JSON.parse(raw);
  }

  loadCellsFromRuntime() {
    this.cellsByKey.clear();
    this.cells = [];
    this.applyRuntimeCells(JSON.parse(this.coreRuntime.current_cells()));
  }

  applyRuntimeCells(cells) {
    if (!Array.isArray(cells)) {
      throw new Error("Puzzle3 runtime cells must be an array.");
    }
    const level = this.currentLevel();
    for (const [index, cell] of cells.entries()) {
      const normalized = runtimeCellPosition(cell, `runtime cells[${index}]`);
      if (!runtimePositionInBounds(normalized, level.size)) {
        throw new Error(
          `Puzzle3 runtime cell ${cellKey(normalized)} is outside current level bounds `
          + `${runtimeSizeLabel(level.size)}.`,
        );
      }
      const key = cellKey(normalized);
      const objects = runtimeCellObjectIds(cell, `runtime cell ${key}`)
        .map((objectId) => this.objectForId(objectId));
      if (!objects.length) {
        this.cellsByKey.delete(key);
        continue;
      }
      this.cellsByKey.set(key, { position: normalized, objects });
    }
    this.cells = Array.from(this.cellsByKey.values());
  }

  inputIdForName(inputName) {
    const canonicalName = canonicalPuzzle3InputName(inputName);
    const input = this.base.inputs
      .find((candidate) => canonicalPuzzle3InputName(candidate.name) === canonicalName);
    if (!input) {
      throw new Error(`Unknown Puzzle3 runtime input: ${inputName}`);
    }
    return runtimeInputId(input, `runtimeContract.model.inputs.${input.name || canonicalName}`);
  }

  objectForId(objectId) {
    const semantic = this.semanticObjectForId(objectId);
    const object = this.presentationObjectsById.get(Number(objectId));
    if (!object) {
      throw new Error(
        `Puzzle3 visual fixture is missing presentation object metadata for runtime object id ${objectId}.`,
      );
    }
    return {
      ...object,
      id: semantic.id,
      layer: semantic.layerId,
    };
  }

  objectLayer(objectId) {
    return this.semanticObjectForId(objectId).layerId;
  }

  semanticObjectForId(objectId) {
    const id = runtimeObjectId(objectId, "runtime object id");
    const object = this.semanticObjectsById.get(id);
    if (!object) {
      throw new Error(`Unknown Puzzle3 runtime object id: ${id}`);
    }
    return object;
  }

  runtimeLayerCount() {
    return this.runtimeLayerCountValue;
  }
}

function applyPuzzle3PreviewUpdate(update = {}) {
  const next = puzzle3PreviewSnapshot(update);
  void loadSnapshotData(next, {
    scene: update.scene,
    preferPuzzleScene: update.preferPuzzleScene !== false,
  });
}

function applyPuzzle3ModelComponentPreviewUpdate(update = {}) {
  const next = puzzle3PreviewSnapshot(update);
  void loadSnapshotData(next, puzzle3ModelComponentPreviewLoadOptions(update));
}

function puzzle3PreviewUpdateFromSurface(update = {}) {
  if (
    update.kind
    && (update.kind !== PUZZLE3_LEVEL_PREVIEW_KIND || update.mode !== ISOLATED_PREVIEW_MODE)
  ) {
    return null;
  }
  const payload = update.payload || {};
  return {
    levelIndex: payload.levelIndex,
    level: payload.level,
    resources: payload.resources,
    camera: payload.camera,
    view: payload.view,
    settings: payload.settings || {},
    scene: update.scene,
    component: update.component,
    componentEmbed: update.componentEmbed === true,
  };
}

function puzzle3ModelComponentPreviewLoadOptions(update = {}) {
  return {
    modelComponentPreview: {
      sceneName: update.scene || "__editor_model_preview__",
      component: puzzle3ModelPreviewComponent(update),
    },
  };
}

function puzzle3PreviewSnapshot(update = {}, source = requireLoadedPuzzle3Snapshot("Puzzle3 preview source snapshot")) {
  const next = JSON.parse(JSON.stringify(requirePuzzle3Snapshot(source, "Puzzle3 preview source snapshot")));
  applyPuzzle3PreviewResources(next, update.resources || update);
  const rawLevelIndex = update.levelIndex ?? next.levelIndex ?? 0;
  const levelCount = Array.isArray(next.levels) && next.levels.length ? next.levels.length : 1;
  const levelIndex = clampIndex(rawLevelIndex, levelCount);
  next.levelIndex = levelIndex;

  const level = update.level || {};
  const size = level.size || update.size;
  const cells = Array.isArray(level.cells)
    ? level.cells
    : Array.isArray(update.cells)
      ? update.cells
      : null;
  if (!Array.isArray(next.levels) || !next.levels.length) {
    next.levels = [{
      name: level.name || next.levelName || "level_1",
      label: level.label || level.name || next.levelLabel || "Level 1",
      size: size || next.size || fallbackSnapshot.size,
      cells: cells || next.cells || [],
    }];
    next.levelIndex = 0;
  } else if (size || cells) {
    const target = next.levels[levelIndex] || {};
    next.levels[levelIndex] = {
      ...target,
      name: level.name || target.name,
      label: level.label || target.label || level.name || target.name,
      size: size ? { ...size } : target.size,
      cells: cells ? JSON.parse(JSON.stringify(cells)) : target.cells,
    };
  }
  if (size) {
    next.size = { ...size };
  }
  if (cells) {
    next.cells = JSON.parse(JSON.stringify(cells));
  }
  if (update.camera) {
    next.render.camera = cloneCamera({
      ...update.camera,
      zoom: update.camera.zoom ?? update.view?.zoom,
    });
  }
  if (update.view) {
    next.view = clonePuzzle3PreviewView(update.view, next.size || fallbackSnapshot.size);
  }
  if (update.settings) {
    next.render = mergePuzzle3PreviewRender(next.render, update.settings);
  }
  return next;
}

function puzzle3ModelPreviewComponent(update = {}) {
  const component = update.component || update.modelComponent || {};
  return {
    kind: "puzzle3",
    source: component.source || update.source || "__editor_model_preview__",
    layout: component.layout || update.layout || {},
  };
}

function applyPuzzle3PreviewResources(target, resources = {}) {
  if (Number.isFinite(Number(resources.layerCount))) {
    target.layerCount = Math.max(1, Math.trunc(Number(resources.layerCount)));
  }
  if (resources.objects && typeof resources.objects === "object") {
    target.objects = JSON.parse(JSON.stringify(resources.objects));
  }
  if (resources.sprites && typeof resources.sprites === "object") {
    target.sprites = JSON.parse(JSON.stringify(resources.sprites));
  }
}

function mergePuzzle3PreviewRender(base, patch) {
  const next = { ...base, ...patch };
  if (base.grid || patch.grid) {
    next.grid = {
      ...(typeof base.grid === "object" && base.grid ? base.grid : {}),
      ...(typeof patch.grid === "object" && patch.grid ? patch.grid : {}),
    };
  }
  return next;
}

function applySceneComponentMetadata(component, sceneName) {
  mountedPuzzle3Component = component || null;
  canvas.dataset.component = component?.kind || "puzzle3";
  canvas.dataset.source = component?.source || "board";
  canvas.dataset.scene = sceneName;
  canvas.setAttribute("aria-label", `${snapshot.title || "Puzzle3"} ${canvas.dataset.source}`);
}

function currentScene() {
  return snapshot.scenes?.find((scene) => scene.name === currentSceneName)
    || snapshot.scenes?.[0]
    || null;
}

function sceneByName(sceneName) {
  return snapshot.scenes?.find((scene) => scene.name === sceneName) || null;
}

function puzzle3ComponentFor(scene) {
  return findSceneComponent(scene?.components || [], (component) => component.kind === "puzzle3");
}

function renderSceneEditorPreview(config = {}) {
  const sceneName = String(config.scene?.name || config.sceneName || currentSceneName || initialSceneName(snapshot) || "").trim();
  sceneEditorPreview = {
    requestId: String(config.requestId || ""),
    sceneName,
    theme: normalizeScenePreviewTheme(config.theme) || normalizeScenePreviewTheme(snapshot.theme) || { name: "clean" },
  };
  applyTheme(sceneEditorPreview.theme);
  currentSceneName = sceneName;
  renderScene();
  notifySceneEditorPreview(sceneEditorPreview.requestId);
}

function notifySceneEditorPreview(requestId = sceneEditorPreview?.requestId || "") {
  if (window.parent === window || !sceneEditorPreview) {
    return;
  }
  const sceneName = sceneEditorPreview.sceneName || currentSceneName || "";
  const scene = sceneByName(sceneName);
  window.parent.postMessage({
    type: "PuzzleStudioScenePreview",
    requestId,
    scene: sceneName,
    theme: sceneEditorPreview.theme || normalizeScenePreviewTheme(snapshot.theme) || { name: "clean" },
    layout: scene?.layout || null,
    logicalSize: null,
    components: [],
    error: scene ? null : `Unknown scene: ${sceneName}`,
  }, "*");
}

function renderScene() {
  let sceneName = currentSceneName || initialSceneName(snapshot) || "default";
  let component = null;
  if (editorModelComponentPreview) {
    sceneName = editorModelComponentPreview.sceneName || "__editor_model_preview__";
    component = editorModelComponentPreview.component || puzzle3ModelPreviewComponent();
  } else {
    const scene = currentScene();
    sceneName = scene?.name || currentSceneName || "default";
    component = puzzle3ComponentFor(scene) || puzzle3ModelPreviewComponent();
  }
  if (inlineComponentMount) {
    screenView.dataset.scene = sceneName;
  } else {
    screenView.className = `scene ${sceneName}`;
  }
  puzzle3Component.mount(component, sceneName);
}

function puzzle3SceneDisplaySize() {
  return {
    width: SCENE_DEFAULT_WIDTH,
    height: SCENE_DEFAULT_HEIGHT,
  };
}

function findSceneComponent(components, predicate) {
  for (const component of components || []) {
    if (predicate(component)) {
      return component;
    }
    if (component.children) {
      const found = findSceneComponent(component.children, predicate);
      if (found) {
        return found;
      }
    }
  }
  return null;
}

function createPuzzle3Component() {
  return {
    mount(component, sceneName) {
      const embed = effectiveComponentEmbedMode();
      puzzle3Frame.style.position = embed ? "fixed" : "relative";
      puzzle3Frame.style.inset = embed ? "0" : "auto";
      puzzle3Frame.style.left = embed ? "0" : "auto";
      puzzle3Frame.style.top = embed ? "0" : "auto";
      puzzle3Frame.style.width = embed ? "auto" : "100%";
      puzzle3Frame.style.height = embed ? "auto" : "100%";
      canvas.style.position = embed ? "absolute" : "";
      canvas.style.inset = embed ? "0" : "";
      canvas.style.width = embed ? "100%" : "";
      canvas.style.height = embed ? "100%" : "";
      if (!inlineComponentMount) {
        screenView.replaceChildren(puzzle3Frame);
      } else if (!puzzle3Frame.contains(canvas)) {
        puzzle3Frame.append(canvas);
      }
      applySceneComponentMetadata(component, sceneName);
      updateCameraInteractionState();
      resizeCanvas();
      draw();
    },
    currentLevelIndex(levelsName) {
      return relativeLevelIndexForBundle(levelsName, snapshot.levelIndex || 0);
    },
    levelEntries(levelsName) {
      return levelIndexesForBundle(levelsName)
        .map((index) => snapshot.levels?.[index])
        .filter(Boolean);
    },
    handleResize() {
      resizeCanvas();
      draw();
    },
    applyInput(input) {
      if (sessionManaged) {
        return false;
      }
      runtime.setCamera(snapshot.render.camera);
      const beforeLevelIndex = snapshot.levelIndex || 0;
      let effects = [];
      if (input === "undo") {
        if (!runtime.undo()) {
          return false;
        }
      } else if (input === "restart") {
        if (!runtime.restart()) {
          return false;
        }
        resetViewportMotion();
      } else {
        const result = runtime.applyInput(input);
        if (!result.handled) {
          return false;
        }
        effects = result.effects || [];
      }
      snapshot = runtime.snapshot();
      if ((snapshot.levelIndex || 0) !== beforeLevelIndex) {
        resetViewportMotion();
      }
      requestSceneViewportDraw();
      notifyPuzzle3StateChange();
      emitPuzzle3LifecycleEffects(effects);
      return true;
    },
    nextLevel() {
      if (sessionManaged) {
        return false;
      }
      const effects = runtime.nextLevel();
      if (effects === false) {
        return false;
      }
      snapshot = runtime.snapshot();
      resetViewportMotion();
      draw();
      notifyPuzzle3StateChange();
      emitPuzzle3LifecycleEffects(effects);
      return true;
    },
    previousLevel() {
      if (sessionManaged) {
        return false;
      }
      const effects = runtime.previousLevel();
      if (effects === false) {
        return false;
      }
      snapshot = runtime.snapshot();
      resetViewportMotion();
      draw();
      notifyPuzzle3StateChange();
      emitPuzzle3LifecycleEffects(effects);
      return true;
    },
    gotoLevel(level) {
      if (sessionManaged) {
        return false;
      }
      const index = puzzle3LevelIndex(level);
      if (index === null) {
        return false;
      }
      const effects = runtime.loadLevel(index);
      snapshot = runtime.snapshot();
      resetViewportMotion();
      draw();
      notifyPuzzle3StateChange();
      emitPuzzle3LifecycleEffects(effects);
      return true;
    },
    resetCamera() {
      resetCamera();
      draw();
      return true;
    },
    handleKey(event) {
      const input = inputForEvent(event);
      if (!input) {
        return false;
      }
      event.preventDefault();
      return startHeldSceneInput(rawInputHoldId({ key: event.key, code: event.code }), input);
    },
  };
}

function emitPuzzle3LifecycleEffects(effects) {
  if (!Array.isArray(effects) || effects.length === 0) {
    return;
  }
  if (typeof controllerOptions.onLifecycleEffects !== "function") {
    throw new Error("Puzzle3 lifecycle effects require a scene host.");
  }
  controllerOptions.onLifecycleEffects(JSON.parse(JSON.stringify(effects)), {
    scene: currentSceneName,
    source: mountedPuzzle3Component?.source || "board",
  });
}

function resetProjection(rect = canvasLayoutFrame()) {
  updateProjectionFit(rect);
}

function initialSceneName(source) {
  return new URLSearchParams(window.location.search).get("scene")
    || source.currentScene
    || source.scenes?.[0]?.name
    || puzzleSceneName(source)
    || "default";
}

function puzzleSceneName(source = snapshot) {
  return source.scenes?.find((scene) => puzzle3ComponentFor(scene))?.name || null;
}

function levelIndexesForBundle(levelsName) {
  const levels = snapshot.levels || [];
  const bundle = snapshot.levelBundles?.[levelsName]
    || snapshot.levelBundles?.default
    || snapshot.levelBundles?.levels;
  if (Array.isArray(bundle)) {
    return bundle
      .map((index) => Number(index))
      .filter((index) => Number.isInteger(index) && index >= 0 && index < levels.length);
  }
  return levels.map((_, index) => index);
}

function absoluteLevelIndexForBundle(levelsName, relativeIndex) {
  const indexes = levelIndexesForBundle(levelsName);
  return indexes[clamp(relativeIndex || 0, 0, Math.max(0, indexes.length - 1))] ?? 0;
}

function puzzle3LevelIndex(level) {
  if (level === undefined || level === null || level === "") {
    return null;
  }
  const levels = snapshot.levels || [];
  const numeric = Number(level);
  if (Number.isInteger(numeric) && numeric >= 0 && numeric < levels.length) {
    return numeric;
  }
  const text = String(level);
  const index = levels.findIndex((candidate) => candidate?.name === text || candidate?.label === text);
  return index >= 0 ? index : null;
}

function applyStartupPuzzle3UrlCommands() {
  if (startupUrlCommandsApplied) {
    return;
  }
  startupUrlCommandsApplied = true;
  const params = new URLSearchParams(window.location.search);
  const level = params.get("level") || "";
  if (level) {
    puzzle3Component.gotoLevel(level);
  }
  const inputs = [
    ...params.getAll("input"),
    ...params
      .getAll("inputs")
      .flatMap((value) => String(value || "").split(",").map((input) => input.trim())),
  ].filter(Boolean);
  for (const input of inputs) {
    puzzle3Component.applyInput(input);
  }
}

function relativeLevelIndexForBundle(levelsName, absoluteIndex) {
  const indexes = levelIndexesForBundle(levelsName);
  const relative = indexes.indexOf(absoluteIndex);
  return relative >= 0 ? relative : 0;
}

function resizeCanvas() {
  const frame = canvasLayoutFrame();
  const scale = window.devicePixelRatio || 1;
  const nextWidth = Math.max(1, Math.floor(frame.width * scale));
  const nextHeight = Math.max(1, Math.floor(frame.height * scale));
  const changed = canvas.width !== nextWidth || canvas.height !== nextHeight;
  canvas.width = nextWidth;
  canvas.height = nextHeight;
  if (ctx) {
    ctx.setTransform(scale, 0, 0, scale, 0, 0);
  }
  updateProjectionFit(frame);
  return changed;
}

function canvasLayoutFrame() {
  const rect = canvas.getBoundingClientRect();
  const width = Math.max(1, Number(canvas.clientWidth) || Number(rect.width) || 1);
  const height = Math.max(1, Number(canvas.clientHeight) || Number(rect.height) || 1);
  return { width, height };
}

function schedulePuzzle3Resize(renderLayout = false) {
  pendingSceneLayoutRender = pendingSceneLayoutRender || Boolean(renderLayout);
  if (pendingResizeFrame) {
    return;
  }
  pendingResizeFrame = requestAnimationFrame(() => {
    const shouldRenderLayout = pendingSceneLayoutRender;
    pendingResizeFrame = 0;
    pendingSceneLayoutRender = false;
    if (shouldRenderLayout || !puzzle3ComponentFor(currentScene())) {
      renderScene();
    } else {
      puzzle3Component.handleResize();
    }
  });
}

function syncCanvasSize() {
  const frame = canvasLayoutFrame();
  const scale = window.devicePixelRatio || 1;
  const nextWidth = Math.max(1, Math.floor(frame.width * scale));
  const nextHeight = Math.max(1, Math.floor(frame.height * scale));
  if (canvas.width !== nextWidth || canvas.height !== nextHeight) {
    resizeCanvas();
  }
}

function updateProjectionFit(rect) {
  const size = snapshot.size || fallbackSnapshot.size;
  const camera = snapshot.render.camera;
  if (activeViewportFocusCell()) {
    return;
  }
  if (!shouldAutoFitFiniteStage(size)) {
    return;
  }
  const width = Math.max(1, Number(rect.width) || 1);
  const height = Math.max(1, Number(rect.height) || 1);
  const bounds = projectedSceneBoundsUnit(size, camera);
  const boundsWidth = Math.max(0.001, bounds.maxX - bounds.minX);
  const boundsHeight = Math.max(0.001, bounds.maxY - bounds.minY);
  const padding = 0.72;
  const scale = Math.min(width / boundsWidth, height / boundsHeight) * padding;
  view.cellScale = Math.max(0.0001, scale);
  view.originX = width / 2;
  view.originY = height / 2;
  view.projectionWidth = width;
  view.projectionHeight = height;
  view.projectionFitKey = projectionFitKey(size, camera);
}

function ensureProjectionFit() {
  const rect = canvasLayoutFrame();
  const size = snapshot.size || fallbackSnapshot.size;
  if (activeViewportFocusCell()) {
    return;
  }
  if (!shouldAutoFitFiniteStage(size)) {
    return;
  }
  const width = Math.max(1, Number(rect.width) || 1);
  const height = Math.max(1, Number(rect.height) || 1);
  const key = projectionFitKey(size, snapshot.render.camera);
  if (
    key !== view.projectionFitKey
    || Math.abs(width - view.projectionWidth) > 0.5
    || Math.abs(height - view.projectionHeight) > 0.5
  ) {
    updateProjectionFit({ width, height });
  }
}

function shouldAutoFitFiniteStage(size) {
  return finiteStageDimension(size?.width)
    && finiteStageDimension(size?.depth)
    && finiteStageDimension(size?.height);
}

function finiteStageDimension(value) {
  const number = Number(value);
  return Number.isFinite(number) && number > 0;
}

function projectionFitKey(size, camera) {
  return [
    Math.max(1, Number(size?.width) || 1),
    Math.max(1, Number(size?.depth) || 1),
    Math.max(1, Number(size?.height) || 1),
    Number(camera?.yawDegrees ?? 0),
    Number(camera?.pitchDegrees ?? 35),
    Number(camera?.rollDegrees ?? 0),
  ].join(":");
}

function projectionZoom(camera, previewView = puzzle3PreviewView()) {
  const cameraZoom = Math.max(0.1, Number(camera?.zoom ?? 1) || 1);
  const viewZoom = Math.max(0.1, Number(previewView?.zoom ?? 1) || 1);
  return cameraZoom * viewZoom;
}

function puzzle3PreviewView(source = snapshot) {
  const size = source?.size || snapshot?.size || fallbackSnapshot.size;
  return clonePuzzle3PreviewView(source?.view || fallbackSnapshot.view, size);
}

function clonePuzzle3PreviewView(source, size = snapshot?.size || fallbackSnapshot.size) {
  const target = source?.target || source?.origin || modelCenterForSize(size);
  return {
    zoom: Math.max(0.1, Number(source?.zoom ?? 1) || 1),
    target: {
      x: Number(target.x ?? 0) || 0,
      y: Number(target.y ?? 0) || 0,
      z: Number(target.z ?? 0) || 0,
    },
  };
}

function puzzle3ProjectionCamera(camera, previewView = puzzle3PreviewView()) {
  return {
    ...camera,
    zoom: projectionZoom(camera, previewView),
  };
}

function puzzle3ProjectionCenter(size, previewView = puzzle3PreviewView()) {
  const target = previewView?.target || modelCenterForSize(size);
  return {
    x: Number(target.x) || 0,
    y: Number(target.y) || 0,
    z: Number(target.z) || 0,
  };
}

function modelCenterForSize(size) {
  const normalized = normalizeModelSize(size);
  return {
    x: (normalized.width - 1) / 2,
    y: (normalized.depth - 1) / 2,
    z: (normalized.height - 1) / 2,
  };
}

function projectedSceneBoundsUnit(size, camera) {
  const { width, depth, height } = normalizeModelSize(size);
  const corners = [];
  for (const x of [-0.5, width - 0.5]) {
    for (const y of [-0.5, depth - 0.5]) {
      for (const z of [-0.55, height - 0.5]) {
        corners.push(projectScenePointUnit({ x, y, z }, { width, depth, height }, camera));
      }
    }
  }
  return projectedPointBounds(corners);
}

function normalizeModelSize(size) {
  return {
    width: Math.max(1, Number(size?.width) || 1),
    depth: Math.max(1, Number(size?.depth) || 1),
    height: Math.max(1, Number(size?.height) || 1),
  };
}

function projectScenePointUnit(position, size, camera) {
  const frame = Puzzle3VisualCore.cameraModelFrame(camera);
  const center = {
    x: (size.width - 1) / 2,
    y: (size.depth - 1) / 2,
    z: (size.height - 1) / 2,
  };
  const x = position.x - center.x;
  const y = position.y - center.y;
  const z = position.z - center.z;
  return {
    x: x * frame.right.x + y * frame.right.y + z * frame.right.z,
    y: x * frame.up.x + y * frame.up.y + z * frame.up.z,
  };
}

function cloneCamera(camera) {
  const next = {
    yawDegrees: Number(camera.yawDegrees),
    pitchDegrees: Number(camera.pitchDegrees),
    rollDegrees: Number(camera.rollDegrees),
    zoom: Number(camera.zoom),
  };
  if (String(camera?.projection || "").toLowerCase() === "orthographic") {
    next.projection = "orthographic";
  }
  return next;
}

function resetCamera() {
  snapshot.render.camera = cloneCamera(initialCamera);
  runtime.setCamera(snapshot.render.camera);
  resetViewportMotion();
}

function cameraLookEnabled() {
  return Boolean(snapshot.render.camera.interactiveLook);
}

function cameraZoomEnabled() {
  return Boolean(snapshot.render.camera.interactiveZoom);
}

function effectiveComponentEmbedMode() {
  return componentEmbedMode || editorComponentEmbedMode;
}

function setEditorComponentEmbedMode(enabled) {
  editorComponentEmbedMode = Boolean(enabled);
  const active = effectiveComponentEmbedMode();
  document.documentElement.classList.toggle("is-component-embed", active);
  document.body.classList.toggle("is-component-embed", active);
}

function updateCameraInteractionState() {
  canvas.classList.toggle("has-interactive-look", cameraLookEnabled());
}

function rotateCamera(deltaX, deltaY) {
  const camera = snapshot.render.camera;
  camera.yawDegrees = normalizeDegrees(camera.yawDegrees + deltaX * 0.35);
  camera.pitchDegrees = clamp(
    camera.pitchDegrees - deltaY * 0.25,
    PUZZLE3_APP_CAMERA_MIN_PITCH_DEGREES,
    PUZZLE3_APP_CAMERA_MAX_PITCH_DEGREES,
  );
  snapshot.render.camera = camera;
  resetProjection();
}

function zoomCamera(deltaY) {
  const camera = snapshot.render.camera;
  const currentZoom = Number(camera.zoom);
  camera.zoom = clamp(currentZoom * Math.exp(-deltaY * 0.001), 0.1, 8);
  snapshot.render.camera = camera;
}

function normalizeDegrees(value) {
  return ((value % 360) + 360) % 360;
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function projectWithDepth(position) {
  const camera = snapshot.render.camera;
  const size = snapshot.size || fallbackSnapshot.size;
  const previewView = puzzle3PreviewView();
  return Puzzle3VisualCore.projectOrthographic(position, {
    camera: puzzle3ProjectionCamera(camera, previewView),
    center: puzzle3ProjectionCenter(size, previewView),
    origin: { x: view.originX, y: view.originY },
    scale: view.cellScale,
  });
}

function degreesToRadians(value) {
  return (value * Math.PI) / 180;
}

function draw(options = {}) {
  if (puzzle3RendererMode === "three") {
    drawWithThree();
    return;
  }
  if (!ctx) {
    return;
  }
  const shadowError = canvasShadowRenderError();
  if (shadowError) {
    showPuzzle3RenderError(shadowError);
    return;
  }
  const advanceViewport = options.advanceViewport !== false;
  syncCanvasSize();
  ensureProjectionFit();
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  ctx.clearRect(0, 0, width, height);

  const renderContext = puzzle3RenderContext(width, height);
  const viewportFit = fitProjectionToViewport(renderContext, { advanceViewport });
  let primitives = scenePrimitives(renderContext);
  if (!viewportFit && fitProjectionToContent(primitives, width, height)) {
    resetRenderContextCandidates(renderContext);
    primitives = scenePrimitives(renderContext);
  }
  assignPrimitiveOrder(primitives);
  primitives = orderScenePrimitives(primitives);
  for (const primitive of primitives) {
    if (primitive.kind === "line") {
      lineSegment(primitive.from, primitive.to, primitive.stroke, primitive.width, primitive.alpha);
    } else {
      polygonPoints(primitive.points, primitive.fill);
    }
  }
  applyPixelatePostprocess();
  notifyPuzzle3View(width, height);
  if (hasRuntimeSpriteAnimation()) {
    scheduleViewportAnimation();
  }
}

function drawWithThree() {
  syncCanvasSize();
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  const renderer = ensurePuzzle3ThreeRenderer();
  if (!renderer) {
    return;
  }
  const input = puzzle3RendererContractInput(width, height);
  let result;
  try {
    result = renderer.render(input.snapshot, input.view);
  } catch (error) {
    showPuzzle3RenderError(error);
    return;
  }
  puzzle3ThreeViewPayload = result?.view || null;
  if (result?.rendered) {
    view.viewportSnapNext = false;
  }
  if (result?.animating) {
    scheduleViewportAnimation();
  }
  notifyPuzzle3View(width, height);
}

function canvasShadowRenderError() {
  const raw = snapshot.render.shadow;
  if (raw === undefined || raw === false) {
    return null;
  }
  if (typeof raw !== "boolean") {
    return new Error("Puzzle3 render setting `shadow` must be boolean.");
  }
  return new Error("Puzzle3 shadows require the Three.js renderer; Canvas renderer cannot render `shadow = true`.");
}

function puzzle3RendererContractInput(width, height) {
  return {
    version: PUZZLE3_RENDERER_CONTRACT_VERSION,
    snapshot: cloneRuntimeSnapshot(requireLoadedPuzzle3Snapshot("Puzzle3 renderer snapshot")),
    view: {
      width,
      height,
      editorView: puzzle3PreviewView(),
      viewportSnapNext: view.viewportSnapNext,
      background: "transparent",
    },
  };
}

function ensurePuzzle3ThreeRenderer() {
  if (puzzle3ThreeRenderer) {
    return puzzle3ThreeRenderer;
  }
  if (!window.Puzzle3ThreeRenderer) {
    return null;
  }
  puzzle3ThreeRenderer = window.Puzzle3ThreeRenderer.create(canvas, {
    onReady: () => draw(),
  });
  return puzzle3ThreeRenderer;
}

function resetViewportMotion() {
  view.viewportSnapNext = true;
}

function requestSceneViewportDraw() {
  if (smoothViewportActive()) {
    scheduleViewportAnimation();
  } else {
    draw();
  }
}

function smoothViewportActive() {
  const viewport = puzzle3ViewportSettings();
  return viewport?.mode === "centered"
    && viewport.follow === "smooth"
    && Boolean(viewportFocusCell(viewport));
}

function puzzle3RenderContext(width = canvas.clientWidth, height = canvas.clientHeight) {
  const frame = normalizeFrame({ width, height });
  const viewport = puzzle3ViewportSettings();
  const focusCell = viewportFocusMode(viewport) ? viewportFocusCell(viewport) : null;
  return {
    frame,
    viewport,
    focusCell,
    renderCells: null,
    opaqueOcclusion: null,
    visibleVoxelCells: null,
  };
}

function createRenderGeometryCache() {
  return {
    cellsSource: null,
    spritesSource: null,
    settingsKey: "",
    cells: new Map(),
    cellSignatures: new Map(),
    occupied: emptyVoxelOccupancy(),
    allDirty: true,
    revision: 0,
  };
}

function resetRenderGeometryCache() {
  renderGeometryCache.cellsSource = null;
  renderGeometryCache.spritesSource = null;
  renderGeometryCache.settingsKey = "";
  renderGeometryCache.cells.clear();
  renderGeometryCache.cellSignatures.clear();
  renderGeometryCache.occupied = emptyVoxelOccupancy();
  renderGeometryCache.allDirty = true;
  renderGeometryCache.revision += 1;
  view.primitiveSortCacheKey = "";
  view.primitiveSortCacheOrder = [];
}

function normalizeFrame(frame) {
  return {
    width: Math.max(1, Number(frame?.width) || 1),
    height: Math.max(1, Number(frame?.height) || 1),
  };
}

function resetRenderContextCandidates(renderContext) {
  if (renderContext) {
    renderContext.renderCells = null;
  }
}

function fitProjectionToViewport(renderContext, options = {}) {
  const target = viewportProjectionFitTarget(renderContext);
  if (!target) {
    return false;
  }
  const snap = target.follow !== "smooth" || view.viewportSnapNext;
  if (snap) {
    view.cellScale = target.cellScale;
    view.originX = target.originX;
    view.originY = target.originY;
    view.viewportSnapNext = false;
    return true;
  }
  if (options.advanceViewport === false) {
    scheduleViewportAnimation();
    return true;
  }
  const amount = 0.12;
  view.cellScale = lerp(view.cellScale, target.cellScale, amount);
  const origin = smoothViewportOrigin(
    lerp(view.originX, target.originX, amount),
    lerp(view.originY, target.originY, amount),
    target,
  );
  view.originX = origin.x;
  view.originY = origin.y;
  if (
    Math.abs(view.cellScale - target.cellScale) > 0.001
    || Math.abs(view.originX - target.originX) > 0.5
    || Math.abs(view.originY - target.originY) > 0.5
  ) {
    scheduleViewportAnimation();
  }
  return true;
}

function scheduleViewportAnimation() {
  if (view.viewportAnimationFrame) {
    return;
  }
  view.viewportAnimationFrame = requestAnimationFrame(() => {
    view.viewportAnimationFrame = 0;
    draw();
  });
}

function lerp(from, to, amount) {
  return from + (to - from) * amount;
}

function smoothViewportOrigin(nextX, nextY, target) {
  const dx = target.originX - nextX;
  const dy = target.originY - nextY;
  const distance = Math.hypot(dx, dy);
  const maxLag = smoothViewportMaxLag(target);
  if (!Number.isFinite(distance) || distance <= maxLag) {
    return { x: nextX, y: nextY };
  }
  const catchUp = (distance - maxLag) / distance;
  return {
    x: nextX + dx * catchUp,
    y: nextY + dy * catchUp,
  };
}

function smoothViewportMaxLag(target) {
  const camera = snapshot.render.camera;
  return Math.max(16, target.cellScale * projectionZoom(camera) * 3.5);
}

function viewportProjectionFitTarget(renderContext) {
  const viewport = renderContext?.viewport || null;
  if (!viewportFocusMode(viewport)) {
    return null;
  }
  const size = snapshot.size || fallbackSnapshot.size;
  const camera = snapshot.render.camera;
  const focus = renderContext?.focusCell || null;
  if (!focus) {
    return null;
  }
  const bounds = viewportFramingProjectionBounds(size, camera, viewport, focus);
  const anchorPoint = viewportFocusProjectionAnchor(size, camera, viewport, focus);
  return viewportFitForFrame(
    renderContext.frame,
    bounds,
    anchorPoint,
    projectionZoom(camera),
    viewport.follow,
  );
}

function viewportFitForFrame(frame, viewportBounds, centerPoint = null, zoom = 1, follow = "snap") {
  const { width: frameWidth, height: frameHeight } = normalizeFrame(frame);
  const minX = Number(viewportBounds?.minX ?? 0) || 0;
  const maxX = Number(viewportBounds?.maxX ?? 0) || 0;
  const minY = Number(viewportBounds?.minY ?? 0) || 0;
  const maxY = Number(viewportBounds?.maxY ?? 0) || 0;
  const centerX = Number(centerPoint?.x);
  const centerY = Number(centerPoint?.y);
  const anchorX = Number.isFinite(centerX) ? centerX : (minX + maxX) / 2;
  const anchorY = Number.isFinite(centerY) ? centerY : (minY + maxY) / 2;
  const halfWidth = Math.max(0.001, Math.max(Math.abs(minX - anchorX), Math.abs(maxX - anchorX)));
  const halfHeight = Math.max(0.001, Math.max(Math.abs(minY - anchorY), Math.abs(maxY - anchorY)));
  const baseScale = Math.max(0.0001, Math.min(frameWidth / (halfWidth * 2), frameHeight / (halfHeight * 2)));
  const effectiveScale = baseScale * Math.max(0.1, Number(zoom) || 1);
  return {
    follow,
    cellScale: baseScale,
    originX: frameWidth / 2 - anchorX * effectiveScale,
    originY: frameHeight / 2 - anchorY * effectiveScale,
  };
}

function puzzle3ViewportSettings(source = snapshot) {
  const raw = source.render.viewport;
  if (!raw || raw === true || raw === false) {
    return null;
  }
  const framing = raw.framingBox || raw.framing || {};
  const widthRaw = Number(framing.width);
  const depthRaw = Number(framing.depth);
  if (!Number.isFinite(widthRaw) || widthRaw <= 0 || !Number.isFinite(depthRaw) || depthRaw <= 0) {
    return null;
  }
  const width = Math.max(1, widthRaw);
  const depth = Math.max(1, depthRaw);
  return {
    mode: String(raw.mode || "centered"),
    follow: String(raw.follow || "snap"),
    focus: String(raw.focus || "Player"),
    focusObjects: Array.isArray(raw.focusObjects)
      ? raw.focusObjects.map((objectId) => Number(objectId)).filter((objectId) => Number.isFinite(objectId) && objectId > 0)
      : [],
    framingBox: {
      width,
      depth,
      height: framing.height === "full" || framing.height === undefined
        ? "full"
        : Math.max(1, Number(framing.height) || 1),
    },
  };
}

function viewportFocusCell(viewport) {
  const focusObjects = new Set(viewport.focusObjects || []);
  for (const cell of snapshot.cells || []) {
    if ((cell.objects || []).some((object) => viewportObjectMatches(object, viewport, focusObjects))) {
      return cell;
    }
  }
  return null;
}

function activeViewportFocusCell() {
  const viewport = puzzle3ViewportSettings();
  return viewportFocusMode(viewport) ? viewportFocusCell(viewport) : null;
}

function viewportFocusMode(viewport) {
  return viewport?.mode === "centered" || viewport?.mode === "paged";
}

function viewportObjectMatches(object, viewport, focusObjects) {
  const objectId = Number(object.id || 0);
  return (
    (focusObjects.size > 0 && focusObjects.has(objectId))
    || object.name === viewport.focus
    || object.sprite === viewport.focus
  );
}

function viewportFramingProjectionBounds(size, camera, viewport, focusCell) {
  const { width, depth, height } = normalizeModelSize(size);
  const ranges = viewportFramingRanges({ width, depth, height }, viewport, focusCell);
  const points = [];
  for (const x of [ranges.x.min, ranges.x.max]) {
    for (const y of [ranges.y.min, ranges.y.max]) {
      for (const z of [ranges.z.min, ranges.z.max]) {
        points.push(projectScenePointUnit({ x, y, z }, { width, depth, height }, camera));
      }
    }
  }
  return projectedPointBounds(points);
}

function viewportFramingRanges(size, viewport, focusCell) {
  const { width, depth, height } = normalizeModelSize(size);
  const position = focusCell.position || {};
  const xRange = viewportCellRange(Number(position.x) || 0, viewport.framingBox.width, viewport.mode);
  const yRange = viewportCellRange(Number(position.y) || 0, viewport.framingBox.depth, viewport.mode);
  const zRange = viewport.framingBox.height === "full"
    ? { min: -0.5, max: height - 0.5 }
    : viewportCellRange(Number(position.z) || 0, viewport.framingBox.height, viewport.mode);
  return { x: xRange, y: yRange, z: zRange };
}

function viewportFocusProjectionAnchor(size, camera, viewport, focusCell) {
  const visualAnchor = viewportFocusVisualProjectionAnchor(size, camera, viewport, focusCell);
  if (visualAnchor) {
    return visualAnchor;
  }
  const { width, depth, height } = normalizeModelSize(size);
  const position = focusCell.position || {};
  return projectScenePointUnit(
    {
      x: Number(position.x) || 0,
      y: Number(position.y) || 0,
      z: Number(position.z) || 0,
    },
    { width, depth, height },
    camera,
  );
}

function viewportFocusVisualProjectionAnchor(size, camera, viewport, focusCell) {
  const { width, depth, height } = normalizeModelSize(size);
  const focusObjects = new Set(viewport?.focusObjects || []);
  const points = [];
  for (const [objectIndex, object] of (focusCell.objects || []).entries()) {
    if (!viewportObjectMatches(object, viewport, focusObjects)) {
      continue;
    }
    const sourceKey = `${cellKey(focusCell.position)}:${objectIndex}`;
    for (const voxel of objectVoxels(focusCell.position || {}, object, sourceKey)) {
      const { x0, x1, y0, y1, z0, z1 } = voxel.bounds;
      for (const x of [x0, x1]) {
        for (const y of [y0, y1]) {
          for (const z of [z0, z1]) {
            points.push(projectScenePointUnit({ x, y, z }, { width, depth, height }, camera));
          }
        }
      }
    }
  }
  if (!points.length) {
    return null;
  }
  const bounds = projectedPointBounds(points);
  return {
    x: (bounds.minX + bounds.maxX) / 2,
    y: (bounds.minY + bounds.maxY) / 2,
  };
}

function virtualCenteredCellRange(center, span) {
  const safeSpan = Math.max(1, Number(span) || 1);
  const safeCenter = Number(center) || 0;
  return {
    min: safeCenter - safeSpan / 2,
    max: safeCenter + safeSpan / 2,
  };
}

function virtualPagedCellRange(center, span) {
  const safeSpan = Math.max(1, Number(span) || 1);
  const min = Math.floor((Number(center) || 0) / safeSpan) * safeSpan - 0.5;
  return {
    min,
    max: min + safeSpan,
  };
}

function viewportCellRange(center, span, mode) {
  return mode === "paged"
    ? virtualPagedCellRange(center, span)
    : virtualCenteredCellRange(center, span);
}

function projectedPointBounds(points) {
  return points.reduce(
    (bounds, point) => ({
      minX: Math.min(bounds.minX, point.x),
      maxX: Math.max(bounds.maxX, point.x),
      minY: Math.min(bounds.minY, point.y),
      maxY: Math.max(bounds.maxY, point.y),
    }),
    { minX: Infinity, maxX: -Infinity, minY: Infinity, maxY: -Infinity },
  );
}

function scenePrimitives(renderContext) {
  const candidates = renderCellCandidates(renderContext);
  const primitives = [...sceneFaces(candidates, renderContext), ...gridLines(gridSettings(), candidates)];
  if (view.shadowsEnabled) {
    primitives.push(...shadowFaces(candidates));
  }
  return primitives;
}

function fitProjectionToContent(primitives, width, height) {
  const fit = projectionContentFitSettings();
  if (!fit.enabled) {
    return false;
  }
  if (fit.mode === "xy") {
    return fitProjectionToXYStageBounds(width, height, fit);
  }
  if (fit.mode === "stage") {
    return fitProjectionToStageBounds(width, height, fit);
  }
  if (!primitives.length) {
    return false;
  }
  const points = primitives.flatMap(primitiveScreenPoints);
  if (!points.length) {
    return false;
  }
  const multiplier = fitScaleForProjectedBounds(
    { width, height },
    projectedPointBounds(points),
    fit.margin,
  );
  if (!Number.isFinite(multiplier) || multiplier <= 0 || Math.abs(multiplier - 1) < 0.0001) {
    return false;
  }
  view.originX = width / 2;
  view.originY = height / 2;
  view.cellScale *= multiplier;
  return true;
}

function fitProjectionToXYStageBounds(width, height, fit) {
  const size = snapshot.size || fallbackSnapshot.size;
  const camera = snapshot.render.camera;
  const bounds = stageProjectionUnitBounds(size, camera, "xy");
  const effectiveScale = fitScaleForProjectedBounds({ width, height }, bounds, fit.margin);
  const nextCellScale = effectiveScale / projectionZoom(camera);
  const nextOriginX = width / 2;
  const nextOriginY = height / 2;
  const changed = Math.abs(view.cellScale - nextCellScale) > 0.0001
    || Math.abs(view.originX - nextOriginX) > 0.0001
    || Math.abs(view.originY - nextOriginY) > 0.0001;
  view.cellScale = nextCellScale;
  view.originX = nextOriginX;
  view.originY = nextOriginY;
  return changed;
}

function fitProjectionToStageBounds(width, height, fit) {
  const size = snapshot.size || fallbackSnapshot.size;
  const camera = snapshot.render.camera;
  const bounds = stageProjectionUnitBounds(size, camera, fit.mode);
  const effectiveScale = fitScaleForProjectedBounds({ width, height }, bounds, fit.margin);
  const nextCellScale = effectiveScale / projectionZoom(camera);
  const nextOriginX = width / 2;
  const nextOriginY = height / 2;
  const changed = Math.abs(view.cellScale - nextCellScale) > 0.0001
    || Math.abs(view.originX - nextOriginX) > 0.0001
    || Math.abs(view.originY - nextOriginY) > 0.0001;
  view.cellScale = nextCellScale;
  view.originX = nextOriginX;
  view.originY = nextOriginY;
  return changed;
}

function fitScaleForProjectedBounds(frame, bounds, margin = 0) {
  const { width, height } = normalizeFrame(frame);
  const padding = Math.max(0, Number(margin) || 0);
  const availableWidth = Math.max(1, width - padding * 2);
  const availableHeight = Math.max(1, height - padding * 2);
  const minX = Number(bounds?.minX);
  const maxX = Number(bounds?.maxX);
  const minY = Number(bounds?.minY);
  const maxY = Number(bounds?.maxY);
  const boundsWidth = Number.isFinite(minX) && Number.isFinite(maxX)
    ? Math.max(0.001, maxX - minX)
    : 1;
  const boundsHeight = Number.isFinite(minY) && Number.isFinite(maxY)
    ? Math.max(0.001, maxY - minY)
    : 1;
  return Math.max(0.0001, Math.min(availableWidth / boundsWidth, availableHeight / boundsHeight));
}

function stageProjectionUnitBounds(size, camera, mode = "stage") {
  const { width, depth, height } = normalizeModelSize(size);
  const zValues = mode === "xy" ? [0] : [-0.5, height - 0.5];
  const points = [];
  for (const x of [-0.5, width - 0.5]) {
    for (const y of [-0.5, depth - 0.5]) {
      for (const z of zValues) {
        points.push(projectScenePointUnit({ x, y, z }, { width, depth, height }, camera));
      }
    }
  }
  return projectedPointBounds(points);
}

function projectionContentFitSettings() {
  const raw = snapshot.render.fitContent;
  if (!raw) {
    return { enabled: false };
  }
  if (raw === true) {
    return { enabled: true, mode: "content", margin: 18 };
  }
  return {
    enabled: raw.enabled !== false,
    mode: String(raw.mode || "content"),
    margin: Number(raw.margin ?? raw.padding ?? 18) || 18,
  };
}

function primitiveScreenPoints(primitive) {
  if (Array.isArray(primitive.points)) {
    return primitive.points;
  }
  return [primitive.from, primitive.to].filter(Boolean);
}

function notifyPuzzle3View(width, height) {
  const viewPayload = puzzle3ViewPayload(width, height);
  for (const listener of viewListeners) {
    listener(viewPayload);
  }
  if (!effectiveComponentEmbedMode() || !window.parent || window.parent === window) {
    return;
  }
  window.parent.postMessage({
    type: "PuzzleStudioPuzzle3View",
    source: canvas.dataset.source || "",
    scene: canvas.dataset.scene || "",
    view: viewPayload,
  }, "*");
}

function puzzle3ViewPayload(width, height) {
  if (puzzle3RendererMode === "three" && puzzle3ThreeViewPayload) {
    return {
      ...puzzle3ThreeViewPayload,
      cellFootprints: projectedStageCellFootprints(snapshot.size || fallbackSnapshot.size),
    };
  }
  const size = snapshot.size || fallbackSnapshot.size;
  const normalizedSize = normalizeModelSize(size);
  const camera = snapshot.render.camera;
  const previewView = puzzle3PreviewView();
  const canvasRect = canvas.getBoundingClientRect();
  return {
    width: Math.max(1, Number(width) || 1),
    height: Math.max(1, Number(height) || 1),
    viewport: {
      width: Math.max(1, Number(window.innerWidth) || Number(width) || 1),
      height: Math.max(1, Number(window.innerHeight) || Number(height) || 1),
    },
    canvasRect: {
      x: canvasRect.x,
      y: canvasRect.y,
      width: Math.max(1, canvasRect.width || Number(width) || 1),
      height: Math.max(1, canvasRect.height || Number(height) || 1),
    },
    coordinateSpace: "canvas-css-px",
    originX: view.originX,
    originY: view.originY,
    scale: view.cellScale * projectionZoom(camera, previewView),
    center: puzzle3ProjectionCenter(size, previewView),
    camera: cloneCamera(camera),
    editorView: previewView,
    size: {
      width: normalizedSize.width,
      depth: normalizedSize.depth,
      height: normalizedSize.height,
    },
    cellFootprints: projectedStageCellFootprints(size),
  };
}

function notifyPuzzle3StateChange() {
  const state = cloneRuntimeSnapshot(requireLoadedPuzzle3Snapshot("Puzzle3 state snapshot"));
  for (const listener of stateListeners) {
    listener(state);
  }
}

function puzzle3InspectState() {
  const frame = normalizeFrame({ width: canvas.clientWidth, height: canvas.clientHeight });
  const renderContext = puzzle3RenderContext(frame.width, frame.height);
  const focusCell = renderContext.focusCell || null;
  const target = viewportProjectionFitTarget(renderContext);
  const projectedFocus = focusCell ? projectWithDepth(focusCell.position || {}) : null;
  const projectedFocusVisual = focusCell
    ? viewportFocusVisualScreenAnchor(renderContext.viewport, focusCell)
    : null;
  return {
    scene: currentSceneName,
    source: canvas.dataset.source || "",
    frame,
    canvas: {
      clientWidth: canvas.clientWidth,
      clientHeight: canvas.clientHeight,
      width: canvas.width,
      height: canvas.height,
    },
    viewport: renderContext.viewport,
    focusCell,
    projectedFocus,
    projectedFocusVisual,
    target,
    view: {
      originX: view.originX,
      originY: view.originY,
      cellScale: view.cellScale,
      effectiveScale: view.cellScale * projectionZoom(snapshot.render.camera),
    },
    renderCellCount: renderCellCandidates(renderContext).length,
    cellCount: (snapshot.cells || []).length,
  };
}

window.Puzzle3DInspect = puzzle3InspectState;

function viewportFocusVisualScreenAnchor(viewport, focusCell) {
  const focusObjects = new Set(viewport?.focusObjects || []);
  const points = [];
  for (const [objectIndex, object] of (focusCell.objects || []).entries()) {
    if (!viewportObjectMatches(object, viewport, focusObjects)) {
      continue;
    }
    const sourceKey = `${cellKey(focusCell.position)}:${objectIndex}`;
    for (const voxel of objectVoxels(focusCell.position || {}, object, sourceKey)) {
      const { x0, x1, y0, y1, z0, z1 } = voxel.bounds;
      for (const x of [x0, x1]) {
        for (const y of [y0, y1]) {
          for (const z of [z0, z1]) {
            points.push(projectWithDepth({ x, y, z }));
          }
        }
      }
    }
  }
  if (!points.length) {
    return null;
  }
  const bounds = projectedPointBounds(points);
  return {
    x: (bounds.minX + bounds.maxX) / 2,
    y: (bounds.minY + bounds.maxY) / 2,
    bounds,
  };
}

function projectedStageCellFootprints(size) {
  const width = Math.max(1, Math.trunc(Number(size?.width) || 1));
  const depth = Math.max(1, Math.trunc(Number(size?.depth) || 1));
  const footprints = [];
  for (let y = 0; y < depth; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const points = [
        { x: x - 0.5, y: y - 0.5, z: 0 },
        { x: x + 0.5, y: y - 0.5, z: 0 },
        { x: x + 0.5, y: y + 0.5, z: 0 },
        { x: x - 0.5, y: y + 0.5, z: 0 },
      ].map((point) => {
        const projected = projectWithDepth(point);
        return { x: projected.x, y: projected.y };
      });
      footprints.push({
        position: { x, y, z: 0 },
        points,
      });
    }
  }
  return footprints;
}

function gridSettings() {
  const raw = snapshot.render.grid;
  if (!raw || raw === false || raw === true) {
    return { visibility: 0 };
  }
  const visibility = gridVisibility(raw);
  return {
    visibility,
    color: raw.color,
    frameColor: raw.frameColor || raw.frame_color,
    occupiedCells: raw.occupied_cells !== false && raw.occupiedCells !== false,
    stageFrame: Boolean(raw.stageFrame ?? raw.stage_frame ?? raw.frame),
    xyPlane: Boolean(raw.xyPlane ?? raw.xy_plane),
  };
}

function gridVisibility(raw) {
  return clamp01(raw.visibility);
}

function clamp01(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return 0;
  }
  return Math.max(0, Math.min(1, number));
}

function spriteRenderSettings() {
  const raw = snapshot.render.sprite;
  if (raw === false) {
    return { shade: false };
  }
  if (!raw || raw === true) {
    return { shade: true };
  }
  return {
    shade: raw.shade !== false,
  };
}

function pixelateSettings() {
  const raw = snapshot.render.pixelate;
  if (!raw) {
    return { enabled: false, scale: 1, smoothing: true };
  }
  if (raw === true) {
    return { enabled: true, scale: 4, smoothing: true };
  }
  const scale = Math.max(1, Math.trunc(Number(raw.scale ?? raw.size ?? 4) || 4));
  return {
    enabled: raw.enabled !== false,
    scale,
    smoothing: raw.smoothing !== false,
  };
}

function gridLines(grid, renderCells) {
  if (!grid.visibility) {
    return [];
  }
  const lines = [];
  if (grid.xyPlane) {
    lines.push(...xyPlaneGridLines(grid));
  }
  if (grid.occupiedCells) {
    const occupiedCells = occupiedCellSet(snapshot.cells || []);
    const edgeLines = new Map();
    for (const cell of renderCells) {
      if (!cell.objects?.length) {
        continue;
      }
      addVisibleOccupiedCellGridLines(cell.position, occupiedCells, edgeLines, grid);
    }
    lines.push(...edgeLines.values());
  }
  if (grid.stageFrame) {
    lines.push(...stageFrameGridLines(grid));
  }
  return lines;
}

function xyPlaneGridLines(grid) {
  const size = snapshot.size || fallbackSnapshot.size;
  const width = Math.max(1, Number(size?.width) || 1);
  const depth = Math.max(1, Number(size?.depth) || 1);
  const lines = [];
  for (let x = 0; x <= width; x += 1) {
    const edgeX = x - 0.5;
    lines.push(projectGridLine(
      { x: edgeX, y: -0.5, z: 0 },
      { x: edgeX, y: depth - 0.5, z: 0 },
      "minor",
      grid,
      gridOrder({ x: edgeX, y: (depth - 1) / 2, z: 0 }),
    ));
  }
  for (let y = 0; y <= depth; y += 1) {
    const edgeY = y - 0.5;
    lines.push(projectGridLine(
      { x: -0.5, y: edgeY, z: 0 },
      { x: width - 0.5, y: edgeY, z: 0 },
      "minor",
      grid,
      gridOrder({ x: (width - 1) / 2, y: edgeY, z: 0 }),
    ));
  }
  return lines;
}

function stageFrameGridLines(grid) {
  return Puzzle3VisualCore.stageFrameEdges(snapshot.size || fallbackSnapshot.size).map((edge) => {
    const line = projectGridLine(edge.from, edge.to, "stageFrame", grid, gridOrder(midpoint3(edge.from, edge.to)));
    line.renderPriority = 2;
    return line;
  });
}

function occupiedCellSet(cells) {
  const occupied = new Set();
  for (const cell of cells) {
    if (cell.objects?.length) {
      occupied.add(voxelKey(cell.position.x, cell.position.y, cell.position.z));
    }
  }
  return occupied;
}

function addVisibleOccupiedCellGridLines(position, occupiedCells, edgeLines, grid) {
  const ownerCell = cellRenderOwner(position);
  const { x, y, z } = position;
  const x0 = x - 0.5;
  const x1 = x + 0.5;
  const y0 = y - 0.5;
  const y1 = y + 0.5;
  const z0 = z - 0.5;
  const z1 = z + 0.5;
  const faces = [
    {
      normal: { x: -1, y: 0, z: 0 },
      neighbor: { x: x - 1, y, z },
      corners: [{ x: x0, y: y0, z: z0 }, { x: x0, y: y1, z: z0 }, { x: x0, y: y1, z: z1 }, { x: x0, y: y0, z: z1 }],
    },
    {
      normal: { x: 1, y: 0, z: 0 },
      neighbor: { x: x + 1, y, z },
      corners: [{ x: x1, y: y0, z: z0 }, { x: x1, y: y0, z: z1 }, { x: x1, y: y1, z: z1 }, { x: x1, y: y1, z: z0 }],
    },
    {
      normal: { x: 0, y: -1, z: 0 },
      neighbor: { x, y: y - 1, z },
      corners: [{ x: x0, y: y0, z: z0 }, { x: x1, y: y0, z: z0 }, { x: x1, y: y0, z: z1 }, { x: x0, y: y0, z: z1 }],
    },
    {
      normal: { x: 0, y: 1, z: 0 },
      neighbor: { x, y: y + 1, z },
      corners: [{ x: x0, y: y1, z: z0 }, { x: x0, y: y1, z: z1 }, { x: x1, y: y1, z: z1 }, { x: x1, y: y1, z: z0 }],
    },
    {
      normal: { x: 0, y: 0, z: -1 },
      neighbor: { x, y, z: z - 1 },
      corners: [{ x: x0, y: y0, z: z0 }, { x: x0, y: y1, z: z0 }, { x: x1, y: y1, z: z0 }, { x: x1, y: y0, z: z0 }],
    },
    {
      normal: { x: 0, y: 0, z: 1 },
      neighbor: { x, y, z: z + 1 },
      corners: [{ x: x0, y: y0, z: z1 }, { x: x1, y: y0, z: z1 }, { x: x1, y: y1, z: z1 }, { x: x0, y: y1, z: z1 }],
    },
  ];
  for (const face of faces) {
    if (directionDepth(face.normal) <= 0) {
      continue;
    }
    if (occupiedCells.has(voxelKey(face.neighbor.x, face.neighbor.y, face.neighbor.z))) {
      continue;
    }
    const gridOrder = faceGridOrder(face.corners);
    for (let index = 0; index < face.corners.length; index += 1) {
      const from = face.corners[index];
      const to = face.corners[(index + 1) % face.corners.length];
      const key = edgeKey(from, to);
      const line = projectGridLine(from, to, "occupied", grid, gridOrder, ownerCell);
      const existing = edgeLines.get(key);
      if (existing && comparePrimitiveOrder(existing, line) >= 0) {
        continue;
      }
      edgeLines.set(key, line);
    }
  }
}

function directionDepth(vector) {
  return Puzzle3VisualCore.directionDepth(vector, puzzle3VisualView());
}

function edgeKey(a, b) {
  const first = `${a.x},${a.y},${a.z}`;
  const second = `${b.x},${b.y},${b.z}`;
  return first < second ? `${first}|${second}` : `${second}|${first}`;
}

function faceGridOrder(corners) {
  return Puzzle3VisualCore.faceGridOrder(corners, puzzle3VisualView());
}

function projectGridLine(from, to, kind, grid, gridOrderOverride = null, ownerCell = null) {
  const a = projectWithDepth(from);
  const b = projectWithDepth(to);
  const key = `${kind}:line:${edgeKey(from, to)}:${ownerCell?.key || ""}`;
  return {
    kind: "line",
    key,
    from: a,
    to: b,
    gridOrder: gridOrderOverride ?? gridOrder(midpoint3(from, to)),
    ownerCell,
    renderPriority: 1,
    depth: (a.depth + b.depth) / 2,
    stroke: gridStroke(kind, grid),
    alpha: grid.visibility,
    width: kind === "stageFrame" ? 1.6 : (kind === "minor" ? 1 : 1.5),
  };
}

function comparePrimitiveOrder(a, b) {
  return Puzzle3VisualCore.comparePrimitiveOrder(a, b);
}

function assignPrimitiveOrder(primitives) {
  const keyCounts = new Map();
  for (const [index, primitive] of primitives.entries()) {
    const baseKey = primitive.key
      ? String(primitive.key)
      : `${primitive.kind || "primitive"}:${index}`;
    const occurrence = keyCounts.get(baseKey) || 0;
    keyCounts.set(baseKey, occurrence + 1);
    primitive.frameIndex = index;
    primitive.stableKey = occurrence === 0 ? baseKey : `${baseKey}#${occurrence}`;
  }
}

function orderScenePrimitives(primitives) {
  const cacheKey = primitiveSortCacheKey(primitives);
  if (cacheKey === view.primitiveSortCacheKey && view.primitiveSortCacheOrder.length === primitives.length) {
    const byStableKey = new Map();
    for (const primitive of primitives) {
      if (!primitive.stableKey || byStableKey.has(primitive.stableKey)) {
        return sortScenePrimitives(primitives, cacheKey);
      }
      byStableKey.set(primitive.stableKey, primitive);
    }
    const ordered = view.primitiveSortCacheOrder.map((stableKey) => byStableKey.get(stableKey));
    if (ordered.every(Boolean)) {
      return ordered;
    }
  }
  return sortScenePrimitives(primitives, cacheKey);
}

function sortScenePrimitives(primitives, cacheKey = primitiveSortCacheKey(primitives)) {
  primitives.sort(comparePrimitiveOrder);
  view.primitiveSortCacheKey = cacheKey;
  view.primitiveSortCacheOrder = primitives.map((primitive) => primitive.stableKey);
  return primitives;
}

function primitiveSortCacheKey(primitives) {
  return [
    cameraOrderKey(),
    primitives.length,
    primitives.map((primitive) => primitive.stableKey).join("\n"),
  ].join("|");
}

function midpoint3(a, b) {
  return {
    x: (a.x + b.x) / 2,
    y: (a.y + b.y) / 2,
    z: (a.z + b.z) / 2,
  };
}

function gridOrder(position) {
  return Puzzle3VisualCore.gridOrder(position, puzzle3VisualView());
}

function cameraOrderKey() {
  return Puzzle3VisualCore.cameraOrderKey(puzzle3VisualView());
}

function puzzle3VisualView() {
  return { camera: snapshot.render.camera };
}

function gridStroke(kind, grid) {
  if (kind === "stageFrame") {
    return grid.frameColor || themeColor("--text");
  }
  return grid.color || themeColor("--text");
}

function sceneFaces(renderCells, renderContext = null) {
  syncRenderGeometryCache(renderContext);
  const faces = [];
  for (const cell of renderCells) {
    if (!cellHasRenderableVoxels(cell)) {
      continue;
    }
    faces.push(...cellFaceGeometriesForRender(cell, renderContext).map(projectFaceGeometry));
  }
  return faces;
}

function shadowFaces(renderCells) {
  const faces = [];
  for (const cell of renderCells) {
    if (!cellHasRenderableVoxels(cell)) {
      continue;
    }
    const point = projectWithDepth({ x: cell.position.x, y: cell.position.y, z: -0.48 });
    faces.push({
      key: `shadow:${cellKey(cell.position)}`,
      points: [
        { x: point.x - 22, y: point.y - 8 },
        { x: point.x + 22, y: point.y - 8 },
        { x: point.x + 30, y: point.y + 4 },
        { x: point.x - 14, y: point.y + 4 },
      ],
      depth: point.depth + 0.02,
      renderPriority: -1,
      fill: themeColorWithAlpha("--text", 0.16),
    });
  }
  return faces;
}

function renderCellCandidates(renderContext = puzzle3RenderContext()) {
  if (renderContext.renderCells) {
    return renderContext.renderCells;
  }
  const cells = snapshot.cells || [];
  if (!viewportRenderCullingEnabled(renderContext)) {
    renderContext.renderCells = cells;
    return renderContext.renderCells;
  }
  renderContext.renderCells = cells.filter((cell) => cellProjectsIntoFrame(cell.position || {}, renderContext.frame));
  return renderContext.renderCells;
}

function viewportRenderCullingEnabled(renderContext) {
  const viewport = renderContext?.viewport || null;
  if (!viewportFocusMode(viewport)) {
    return false;
  }
  return Boolean(renderContext?.focusCell);
}

function cellProjectsIntoFrame(position, frame) {
  const x = Number(position.x) || 0;
  const y = Number(position.y) || 0;
  const z = Number(position.z) || 0;
  const points = [];
  for (const px of [x - 0.5, x + 0.5]) {
    for (const py of [y - 0.5, y + 0.5]) {
      for (const pz of [z - 0.5, z + 0.5]) {
        points.push(projectWithDepth({ x: px, y: py, z: pz }));
      }
    }
  }
  const bounds = projectedPointBounds(points);
  return bounds.maxX >= 0
    && bounds.minX <= frame.width
    && bounds.maxY >= 0
    && bounds.minY <= frame.height;
}

function cellHasRenderableVoxels(cell) {
  return (cell.objects || []).some((object) => object.sprite && snapshot.sprites?.[object.sprite]);
}

function cellVisibleVoxels(cell) {
  const stacks = new Map();
  for (const [objectIndex, object] of cell.objects.entries()) {
    const sourceKey = `${cellKey(cell.position)}:${objectIndex}`;
    const objectOrder = Puzzle3VisualCore.objectPriority(spriteOrder(), object, objectIndex);
    for (const voxel of objectVoxels(cell.position, object, sourceKey, objectOrder)) {
      const key = voxelGeometryKey(voxel);
      const stack = stacks.get(key) || [];
      stack.push(voxel);
      stacks.set(key, stack);
    }
  }
  const voxels = [];
  const occupied = emptyVoxelOccupancy();
  for (const [key, stack] of stacks) {
    const visibleStack = visibleVoxelStack(stack);
    if (visibleStack.length > 0) {
      voxels.push(...visibleStack);
      for (const voxel of visibleStack) {
        for (const sourceKey of voxel.sourceKeys || []) {
          occupied.bySource.add(`${sourceKey}|${key}`);
        }
      }
      if (visibleStack.some((candidate) => candidate.opaque === true || (candidate.opaque === undefined && isOpaqueFill(candidate.fill)))) {
        occupied.opaque.add(key);
      }
    } else {
      stacks.delete(key);
    }
  }
  return { voxels, occupied };
}

function emptyVoxelOccupancy() {
  return {
    opaque: new Set(),
    bySource: new Set(),
  };
}

function expandDirtyCellKeys(keys) {
  const expanded = new Set();
  for (const key of keys) {
    const position = positionFromCellKey(key);
    if (!position) {
      continue;
    }
    expanded.add(key);
    for (const offset of faceNeighborOffsets()) {
      expanded.add(cellKey({
        x: position.x + offset.x,
        y: position.y + offset.y,
        z: position.z + offset.z,
      }));
    }
  }
  return expanded;
}

function positionFromCellKey(key) {
  const parts = String(key).split(",").map((part) => Number(part));
  if (parts.length !== 3 || parts.some((part) => !Number.isFinite(part))) {
    return null;
  }
  return { x: parts[0], y: parts[1], z: parts[2] };
}

function faceNeighborOffsets() {
  return [
    { x: -1, y: 0, z: 0 },
    { x: 1, y: 0, z: 0 },
    { x: 0, y: -1, z: 0 },
    { x: 0, y: 1, z: 0 },
    { x: 0, y: 0, z: -1 },
    { x: 0, y: 0, z: 1 },
  ];
}

function renderOpaqueOcclusion(renderContext) {
  syncRenderGeometryCache(renderContext);
  const occupied = renderGeometryCache.occupied;
  if (renderContext) {
    renderContext.opaqueOcclusion = occupied;
  }
  return occupied;
}

function syncRenderGeometryCache(renderContext = null) {
  const cells = snapshot.cells || [];
  const settingsKey = renderGeometrySettingsKey();
  const sourcesUnchanged = renderGeometryCache.cellsSource === cells
    && renderGeometryCache.spritesSource === snapshot.sprites
    && renderGeometryCache.settingsKey === settingsKey
    && !renderGeometryCache.allDirty;
  if (sourcesUnchanged) {
    if (renderContext) {
      renderContext.opaqueOcclusion = renderGeometryCache.occupied;
    }
    return;
  }

  const previousSignatures = renderGeometryCache.cellSignatures;
  const nextSignatures = new Map();
  const cellsByKey = new Map();
  const dirtyCellKeys = new Set();
  const fullRebuild = renderGeometryCache.allDirty
    || renderGeometryCache.spritesSource !== snapshot.sprites
    || renderGeometryCache.settingsKey !== settingsKey;

  for (const cell of cells) {
    const key = cellKey(cell.position);
    const signature = renderCellSignature(cell);
    cellsByKey.set(key, cell);
    nextSignatures.set(key, signature);
    if (fullRebuild || previousSignatures.get(key) !== signature) {
      dirtyCellKeys.add(key);
    }
  }
  for (const key of previousSignatures.keys()) {
    if (!nextSignatures.has(key)) {
      dirtyCellKeys.add(key);
      renderGeometryCache.cells.delete(key);
    }
  }

  const rebuildCellKeys = fullRebuild ? new Set(cellsByKey.keys()) : expandDirtyCellKeys(dirtyCellKeys);
  for (const key of dirtyCellKeys) {
    const cell = cellsByKey.get(key);
    if (cell) {
      rebuildVisibleCellGeometry(key, cell, nextSignatures.get(key));
    }
  }
  if (fullRebuild) {
    for (const [key, cell] of cellsByKey) {
      if (!dirtyCellKeys.has(key)) {
        rebuildVisibleCellGeometry(key, cell, nextSignatures.get(key));
      }
    }
  }

  renderGeometryCache.occupied = renderCachedOpaqueOcclusion();
  for (const key of rebuildCellKeys) {
    const cell = cellsByKey.get(key);
    if (cell) {
      rebuildCachedCellFaces(key, cell);
    }
  }

  renderGeometryCache.cellsSource = cells;
  renderGeometryCache.spritesSource = snapshot.sprites;
  renderGeometryCache.settingsKey = settingsKey;
  renderGeometryCache.cellSignatures = nextSignatures;
  renderGeometryCache.allDirty = false;
  renderGeometryCache.revision += 1;
  if (renderContext) {
    renderContext.opaqueOcclusion = renderGeometryCache.occupied;
  }
}

function renderGeometrySettingsKey() {
  return JSON.stringify(spriteRenderSettings());
}

function renderCellSignature(cell) {
  const position = cell?.position || {};
  const objects = (cell?.objects || []).map((object) => [
    object?.id ?? "",
    object?.sprite ?? "",
    Puzzle3VisualCore.objectPriority(spriteOrder(), object),
  ].join(":"));
  return `${cellKey(position)}|${objects.join(";")}`;
}

function rebuildVisibleCellGeometry(key, cell, signature) {
  const visible = cellHasRenderableVoxels(cell)
    ? cellVisibleVoxels(cell)
    : { voxels: [], occupied: emptyVoxelOccupancy() };
  const entry = renderGeometryCache.cells.get(key) || {};
  entry.key = key;
  entry.cell = cell;
  entry.signature = signature;
  entry.visible = visible;
  entry.faceGeometries = entry.faceGeometries || [];
  renderGeometryCache.cells.set(key, entry);
}

function rebuildCachedCellFaces(key, cell) {
  const entry = renderGeometryCache.cells.get(key);
  if (!entry) {
    return;
  }
  entry.cell = cell;
  entry.faceGeometries = cellHasRenderableVoxels(cell)
    ? mergedVoxelFaces(entry.visible.voxels, renderGeometryCache.occupied, cellRenderOwnerGeometry(cell.position))
    : [];
}

function renderCachedOpaqueOcclusion() {
  const occupied = emptyVoxelOccupancy();
  for (const entry of renderGeometryCache.cells.values()) {
    if (!entry.visible) {
      continue;
    }
    for (const key of entry.visible.occupied.opaque) {
      occupied.opaque.add(key);
    }
    for (const key of entry.visible.occupied.bySource) {
      occupied.bySource.add(key);
    }
  }
  return occupied;
}

function cellFaceGeometriesForRender(cell, renderContext = null) {
  syncRenderGeometryCache(renderContext);
  return renderGeometryCache.cells.get(cellKey(cell.position))?.faceGeometries || [];
}

function cellVisibleVoxelsForRender(cell, renderContext = null) {
  if (!renderContext) {
    return cellVisibleVoxels(cell);
  }
  if (!renderContext.visibleVoxelCells) {
    renderContext.visibleVoxelCells = new Map();
  }
  let cached = renderContext.visibleVoxelCells.get(cell);
  if (!cached) {
    cached = cellVisibleVoxels(cell);
    renderContext.visibleVoxelCells.set(cell, cached);
  }
  return cached;
}

function visibleVoxelStack(stack) {
  const visible = [];
  const ordered = [...stack].sort((left, right) => objectVoxelOrder(left) - objectVoxelOrder(right));
  const priorities = [];
  for (const voxel of ordered) {
    const order = objectVoxelOrder(voxel);
    const group = priorities.at(-1);
    if (group && group.order === order) {
      group.voxels.push(voxel);
    } else {
      priorities.push({ order, voxels: [voxel] });
    }
  }
  for (const group of priorities) {
    const voxel = Puzzle3VisualCore.priorityDefinition(spriteOrder(), group.order).merge
      ? Puzzle3VisualCore.averageMergedVoxels(group.voxels, parseColor, formatColor)
      : group.voxels[0];
    const source = voxel.color || parseColor(voxel.fill);
    if (!source || source.a <= 0) {
      continue;
    }
    const renderVoxel = {
      ...voxel,
      color: source,
      opaque: source.a >= 0.999,
      fill: formatColor(source),
      sourceKeys: voxel.sourceKey ? [voxel.sourceKey] : [],
    };
    if (renderVoxel.opaque) {
      visible.length = 0;
    }
    visible.push(renderVoxel);
  }
  return visible;
}

function spriteOrder() {
  const order = snapshot.order || fallbackSnapshot.order;
  if (!order || !Array.isArray(order.direction_priority) || !Array.isArray(order.priorities)) {
    throw new Error("compiled sprite order contract is missing");
  }
  return order;
}

function objectVoxelOrder(voxel) {
  const order = Number(voxel?.objectOrder);
  return Number.isFinite(order) ? order : 0;
}

function objectVoxels(position, object, sourceKey, objectOrder = 0) {
  if (!object.sprite) {
    return [];
  }
  const template = spriteVoxelTemplate(object.sprite);
  if (!template) {
    return [];
  }
  return instantiateSpriteVoxelTemplate(position, template, sourceKey, objectOrder);
}

function spriteVoxelTemplate(spriteName) {
  const sprite = snapshot.sprites?.[spriteName];
  if (!sprite) {
    return null;
  }
  const animated = Array.isArray(sprite.frames) && sprite.frames.length > 1;
  if (!animated) {
    const cached = spriteVoxelTemplateCache.get(sprite);
    if (cached) {
      return cached;
    }
  }
  const template = buildSpriteVoxelTemplate(sprite);
  if (!animated) {
    spriteVoxelTemplateCache.set(sprite, template);
  }
  return template;
}

function buildSpriteVoxelTemplate(sprite) {
  const blocks = currentRuntimeSpriteLayers(sprite);
  const height = Math.max(1, blocks.length);
  const depth = Math.max(1, ...blocks.map((rows) => rows.length));
  const width = Math.max(1, ...blocks.flatMap((rows) => rows.map((row) => row.length)));
  const scale = 1 / Math.max(width, depth, height);
  const voxels = [];
  const palette = sprite.palette || {};

  for (let z = 0; z < blocks.length; z += 1) {
    const rows = blocks[z];
    for (let row = 0; row < rows.length; row += 1) {
      for (let col = 0; col < rows[row].length; col += 1) {
        const key = rows[row][col];
        const fill = palette[key];
        const color = parseColor(fill);
        if (!fill || color?.a <= 0) {
          continue;
        }
        const grid = standardSpriteGridPosition({ width, depth, height }, col, row, z);
        const localPosition = {
          x: (grid.x + 0.5 - width / 2) * scale,
          y: (grid.y + 0.5 - depth / 2) * scale,
          z: (grid.z + 0.5 - height / 2) * scale,
        };
        voxels.push({
          fill,
          color,
          opaque: !color || color.a >= 0.999,
          scale,
          grid,
          localPosition,
          localBounds: voxelBounds(localPosition, scale),
        });
      }
    }
  }

  return { voxels };
}

function instantiateSpriteVoxelTemplate(position, template, sourceKey = null, objectOrder = 0) {
  const x = Number(position?.x) || 0;
  const y = Number(position?.y) || 0;
  const z = Number(position?.z) || 0;
  return template.voxels.map((voxel) => ({
    fill: voxel.fill,
    color: voxel.color,
    opaque: voxel.opaque,
    scale: voxel.scale,
    grid: voxel.grid,
    position: {
      x: x + voxel.localPosition.x,
      y: y + voxel.localPosition.y,
      z: z + voxel.localPosition.z,
    },
    bounds: {
      x0: x + voxel.localBounds.x0,
      x1: x + voxel.localBounds.x1,
      y0: y + voxel.localBounds.y0,
      y1: y + voxel.localBounds.y1,
      z0: z + voxel.localBounds.z0,
      z1: z + voxel.localBounds.z1,
    },
    sourceKey,
    objectOrder,
  }));
}

function spriteVoxels(position, blocks, palette, sourceKey = null, objectOrder = 0) {
  const height = Math.max(1, blocks.length);
  const depth = Math.max(1, ...blocks.map((rows) => rows.length));
  const width = Math.max(1, ...blocks.flatMap((rows) => rows.map((row) => row.length)));
  const scale = 1 / Math.max(width, depth, height);
  const voxels = [];
  const occupied = new Set();

  for (let z = 0; z < blocks.length; z += 1) {
    const rows = blocks[z];
    for (let row = 0; row < rows.length; row += 1) {
      for (let col = 0; col < rows[row].length; col += 1) {
        const key = rows[row][col];
        const fill = palette[key];
        const color = parseColor(fill);
        if (!fill || color?.a <= 0) {
          continue;
        }
        const grid = standardSpriteGridPosition({ width, depth, height }, col, row, z);
        occupied.add(voxelKey(grid.x, grid.y, grid.z));
        const voxelPosition = {
          x: position.x + (grid.x + 0.5 - width / 2) * scale,
          y: position.y + (grid.y + 0.5 - depth / 2) * scale,
          z: position.z + (grid.z + 0.5 - height / 2) * scale,
        };
        voxels.push({
          fill,
          color,
          opaque: !color || color.a >= 0.999,
          scale,
          grid,
          position: voxelPosition,
          bounds: voxelBounds(voxelPosition, scale),
          sourceKey,
          objectOrder,
        });
      }
    }
  }

  return { voxels, occupied };
}

function mergedVoxelFaces(voxels, occupied, ownerCell) {
  const spriteSettings = spriteRenderSettings();
  return Puzzle3VisualCore.mergeVoxelFaces(voxels, {
    faces: voxelFaces,
    isFaceVisible: (voxel, face) => !isVoxelFaceOccluded(voxel, face.offset, occupied),
    group: (voxel, face) => {
      const fill = spriteSettings.shade ? shadeFill(voxel.fill, face.light) : voxel.fill;
      const info = voxelFaceGroupInfo(voxel, face.side);
      const groupKey = [
        ownerCell?.key || "",
        quantizeGeometryValue(voxel.objectOrder),
        face.side,
        quantizeGeometryValue(info.origin.x),
        quantizeGeometryValue(info.origin.y),
        quantizeGeometryValue(info.origin.z),
        quantizeGeometryValue(voxel.scale),
        info.planeIndex,
        fill,
      ].join("|");
      return {
        key: groupKey,
        u: info.u,
        v: info.v,
        group: {
          key: groupKey,
          ownerCell,
          objectOrder: voxel.objectOrder,
          side: face.side,
          origin: info.origin,
          scale: voxel.scale,
          planeIndex: info.planeIndex,
          fill,
        },
      };
    },
    face: (group, rect) => faceGeometry(mergedVoxelFaceCorners(group, rect), group.fill, group.ownerCell, group.objectOrder, `${group.key}:${rect.u0},${rect.u1},${rect.v0},${rect.v1}`),
  });
}

function isVoxelFaceOccluded(voxel, offset, occupied) {
  const adjacentKey = adjacentVoxelGeometryKey(voxel, offset);
  if (voxel.opaque !== false && occupied.opaque.has(adjacentKey)) {
    return true;
  }
  for (const sourceKey of voxel.sourceKeys || []) {
    if (occupied.bySource.has(`${sourceKey}|${adjacentKey}`)) {
      return true;
    }
  }
  return false;
}

function voxelFaces(voxel) {
  const { x0, x1, y0, y1, z0, z1 } = voxel.bounds;
  return [
    {
      side: "zNeg",
      offset: { x: 0, y: 0, z: -1 },
      light: -0.22,
      corners: [
        { x: x1, y: y0, z: z0 },
        { x: x0, y: y0, z: z0 },
        { x: x0, y: y1, z: z0 },
        { x: x1, y: y1, z: z0 },
      ],
    },
    {
      side: "zPos",
      offset: { x: 0, y: 0, z: 1 },
      light: 0.10,
      corners: [
        { x: x0, y: y0, z: z1 },
        { x: x1, y: y0, z: z1 },
        { x: x1, y: y1, z: z1 },
        { x: x0, y: y1, z: z1 },
      ],
    },
    {
      side: "xNeg",
      offset: { x: -1, y: 0, z: 0 },
      light: -0.08,
      corners: [
        { x: x0, y: y0, z: z0 },
        { x: x0, y: y0, z: z1 },
        { x: x0, y: y1, z: z1 },
        { x: x0, y: y1, z: z0 },
      ],
    },
    {
      side: "xPos",
      offset: { x: 1, y: 0, z: 0 },
      light: 0.02,
      corners: [
        { x: x1, y: y0, z: z1 },
        { x: x1, y: y0, z: z0 },
        { x: x1, y: y1, z: z0 },
        { x: x1, y: y1, z: z1 },
      ],
    },
    {
      side: "yPos",
      offset: { x: 0, y: 1, z: 0 },
      light: -0.04,
      corners: [
        { x: x0, y: y1, z: z1 },
        { x: x1, y: y1, z: z1 },
        { x: x1, y: y1, z: z0 },
        { x: x0, y: y1, z: z0 },
      ],
    },
    {
      side: "yNeg",
      offset: { x: 0, y: -1, z: 0 },
      light: -0.16,
      corners: [
        { x: x0, y: y0, z: z0 },
        { x: x1, y: y0, z: z0 },
        { x: x1, y: y0, z: z1 },
        { x: x0, y: y0, z: z1 },
      ],
    },
  ];
}

function voxelFaceGroupInfo(voxel, side) {
  const origin = {
    x: voxel.bounds.x0 - voxel.grid.x * voxel.scale,
    y: voxel.bounds.y0 - voxel.grid.y * voxel.scale,
    z: voxel.bounds.z0 - voxel.grid.z * voxel.scale,
  };
  if (side === "zNeg") {
    return { origin, planeIndex: voxel.grid.z, u: voxel.grid.x, v: voxel.grid.y };
  }
  if (side === "zPos") {
    return { origin, planeIndex: voxel.grid.z + 1, u: voxel.grid.x, v: voxel.grid.y };
  }
  if (side === "xNeg") {
    return { origin, planeIndex: voxel.grid.x, u: voxel.grid.y, v: voxel.grid.z };
  }
  if (side === "xPos") {
    return { origin, planeIndex: voxel.grid.x + 1, u: voxel.grid.y, v: voxel.grid.z };
  }
  if (side === "yPos") {
    return { origin, planeIndex: voxel.grid.y + 1, u: voxel.grid.x, v: voxel.grid.z };
  }
  return { origin, planeIndex: voxel.grid.y, u: voxel.grid.x, v: voxel.grid.z };
}

function mergedVoxelFaceCorners(group, rect) {
  const plane = axisValue(group.origin, group.side[0], group.planeIndex, group.scale);
  const a0 = rect.u0;
  const a1 = rect.u1 + 1;
  const b0 = rect.v0;
  const b1 = rect.v1 + 1;
  if (group.side === "zNeg") {
    return [
      { x: axisValue(group.origin, "x", a1, group.scale), y: axisValue(group.origin, "y", b0, group.scale), z: plane },
      { x: axisValue(group.origin, "x", a0, group.scale), y: axisValue(group.origin, "y", b0, group.scale), z: plane },
      { x: axisValue(group.origin, "x", a0, group.scale), y: axisValue(group.origin, "y", b1, group.scale), z: plane },
      { x: axisValue(group.origin, "x", a1, group.scale), y: axisValue(group.origin, "y", b1, group.scale), z: plane },
    ];
  }
  if (group.side === "zPos") {
    return [
      { x: axisValue(group.origin, "x", a0, group.scale), y: axisValue(group.origin, "y", b0, group.scale), z: plane },
      { x: axisValue(group.origin, "x", a1, group.scale), y: axisValue(group.origin, "y", b0, group.scale), z: plane },
      { x: axisValue(group.origin, "x", a1, group.scale), y: axisValue(group.origin, "y", b1, group.scale), z: plane },
      { x: axisValue(group.origin, "x", a0, group.scale), y: axisValue(group.origin, "y", b1, group.scale), z: plane },
    ];
  }
  if (group.side === "xNeg") {
    return [
      { x: plane, y: axisValue(group.origin, "y", a0, group.scale), z: axisValue(group.origin, "z", b0, group.scale) },
      { x: plane, y: axisValue(group.origin, "y", a0, group.scale), z: axisValue(group.origin, "z", b1, group.scale) },
      { x: plane, y: axisValue(group.origin, "y", a1, group.scale), z: axisValue(group.origin, "z", b1, group.scale) },
      { x: plane, y: axisValue(group.origin, "y", a1, group.scale), z: axisValue(group.origin, "z", b0, group.scale) },
    ];
  }
  if (group.side === "xPos") {
    return [
      { x: plane, y: axisValue(group.origin, "y", a0, group.scale), z: axisValue(group.origin, "z", b1, group.scale) },
      { x: plane, y: axisValue(group.origin, "y", a0, group.scale), z: axisValue(group.origin, "z", b0, group.scale) },
      { x: plane, y: axisValue(group.origin, "y", a1, group.scale), z: axisValue(group.origin, "z", b0, group.scale) },
      { x: plane, y: axisValue(group.origin, "y", a1, group.scale), z: axisValue(group.origin, "z", b1, group.scale) },
    ];
  }
  if (group.side === "yPos") {
    return [
      { x: axisValue(group.origin, "x", a0, group.scale), y: plane, z: axisValue(group.origin, "z", b1, group.scale) },
      { x: axisValue(group.origin, "x", a1, group.scale), y: plane, z: axisValue(group.origin, "z", b1, group.scale) },
      { x: axisValue(group.origin, "x", a1, group.scale), y: plane, z: axisValue(group.origin, "z", b0, group.scale) },
      { x: axisValue(group.origin, "x", a0, group.scale), y: plane, z: axisValue(group.origin, "z", b0, group.scale) },
    ];
  }
  return [
    { x: axisValue(group.origin, "x", a0, group.scale), y: plane, z: axisValue(group.origin, "z", b0, group.scale) },
    { x: axisValue(group.origin, "x", a1, group.scale), y: plane, z: axisValue(group.origin, "z", b0, group.scale) },
    { x: axisValue(group.origin, "x", a1, group.scale), y: plane, z: axisValue(group.origin, "z", b1, group.scale) },
    { x: axisValue(group.origin, "x", a0, group.scale), y: plane, z: axisValue(group.origin, "z", b1, group.scale) },
  ];
}

function axisValue(origin, axis, index, scale) {
  return origin[axis] + index * scale;
}

function faceGeometry(corners, fill, ownerCell = null, objectOrder = 0, key = "") {
  return {
    kind: "faceGeometry",
    key,
    corners,
    fill,
    ownerCell,
    objectOrder,
  };
}

function projectFace(corners, fill, ownerCell = null, objectOrder = 0, key = "") {
  return projectFaceGeometry(faceGeometry(corners, fill, ownerCell, objectOrder, key));
}

function projectFaceGeometry(geometry) {
  const corners = geometry.corners || [];
  const projected = corners.map(projectWithDepth);
  const primitive = geometry.primitive || {
    kind: "face",
    key: geometry.key,
    renderPriority: 0,
  };
  primitive.points = projected.map(({ x, y }) => ({ x, y }));
  primitive.depth = projected.reduce((total, point) => total + point.depth, 0) / projected.length;
  primitive.gridOrder = faceGridOrder(corners);
  primitive.ownerCell = projectCellRenderOwner(geometry.ownerCell);
  primitive.objectOrder = geometry.objectOrder;
  primitive.fill = geometry.fill;
  geometry.primitive = primitive;
  return primitive;
}

function voxelBounds(position, scale) {
  return {
    x0: position.x - scale / 2,
    x1: position.x + scale / 2,
    y0: position.y - scale / 2,
    y1: position.y + scale / 2,
    z0: position.z - scale / 2,
    z1: position.z + scale / 2,
  };
}

function voxelKey(x, y, z) {
  return `${x},${y},${z}`;
}

function voxelGeometryKey(voxel) {
  return voxelGeometryKeyAt(voxel.position, voxel.scale);
}

function adjacentVoxelGeometryKey(voxel, offset) {
  return voxelGeometryKeyAt({
    x: voxel.position.x + offset.x * voxel.scale,
    y: voxel.position.y + offset.y * voxel.scale,
    z: voxel.position.z + offset.z * voxel.scale,
  }, voxel.scale);
}

function voxelGeometryKeyAt(position, scale) {
  return [
    quantizeGeometryValue(position.x),
    quantizeGeometryValue(position.y),
    quantizeGeometryValue(position.z),
    quantizeGeometryValue(scale),
  ].join(",");
}

function quantizeGeometryValue(value) {
  return String(Math.round(value * 1000000) / 1000000);
}

function cellKey(position) {
  return voxelKey(position.x, position.y, position.z);
}

function cellRenderOwner(position) {
  return {
    key: cellKey(position),
    directionPriority: cellDirectionPriority(position),
    order: gridOrder(position),
    depth: projectWithDepth(position).depth,
  };
}

function cellRenderOwnerGeometry(position) {
  return {
    key: cellKey(position),
    position: {
      x: Number(position?.x) || 0,
      y: Number(position?.y) || 0,
      z: Number(position?.z) || 0,
    },
  };
}

function projectCellRenderOwner(ownerCell) {
  if (!ownerCell) {
    return null;
  }
  if (!ownerCell.position) {
    return ownerCell;
  }
  return {
    key: ownerCell.key,
    directionPriority: cellDirectionPriority(ownerCell.position),
    order: gridOrder(ownerCell.position),
    depth: projectWithDepth(ownerCell.position).depth,
  };
}

function cellDirectionPriority(position) {
  return spriteOrder().direction_priority.map((direction) => {
    switch (direction) {
      case "right": return Number(position.x) || 0;
      case "left": return -(Number(position.x) || 0);
      case "front": return Number(position.y) || 0;
      case "back": return -(Number(position.y) || 0);
      case "up": return Number(position.z) || 0;
      case "down": return -(Number(position.z) || 0);
      default: throw new Error(`invalid 3D sprite order direction: ${direction}`);
    }
  });
}

function currentRuntimeSpriteLayers(sprite, now = performance.now()) {
  const frames = Array.isArray(sprite?.frames) ? sprite.frames : [];
  if (!frames.length) {
    throw new Error("Puzzle3 runtime sprite frames are missing.");
  }
  const frameDuration = Number(sprite.frameDurationMs)
    || (Number(sprite.durationMs) > 0 ? Number(sprite.durationMs) / frames.length : 0);
  const index = frames.length > 1 && frameDuration > 0
    ? Math.floor(now / frameDuration) % frames.length
    : 0;
  const layers = frames[index]?.layers;
  if (!Array.isArray(layers) || !layers.length || layers.some((layer) => !Array.isArray(layer) || !layer.length)) {
    throw new Error("Puzzle3 runtime sprite frame layers are missing or invalid.");
  }
  return layers;
}

function hasRuntimeSpriteAnimation() {
  return Object.values(snapshot?.sprites || {}).some((sprite) => (
    Array.isArray(sprite?.frames)
    && sprite.frames.length > 1
    && (Number(sprite.frameDurationMs) > 0 || Number(sprite.durationMs) > 0)
  ));
}

function shadeFill(fill, light) {
  const color = parseColor(fill);
  if (!color) {
    return fill;
  }
  return formatColor({
    r: lightenChannel(color.r, light),
    g: lightenChannel(color.g, light),
    b: lightenChannel(color.b, light),
    a: color.a,
  });
}

function parseColor(fill) {
  if (typeof fill !== "string") {
    return null;
  }
  if (fill.startsWith("rgb(") || fill.startsWith("rgba(")) {
    return parseRgbColor(fill);
  }
  if (!fill.startsWith("#") || ![4, 5, 7, 9].includes(fill.length)) {
    return null;
  }
  const channels = fill.length <= 5
    ? [...fill.slice(1)].map((digit) => parseInt(`${digit}${digit}`, 16))
    : fill.slice(1).match(/../g).map((pair) => parseInt(pair, 16));
  if (channels.some((channel) => Number.isNaN(channel))) {
    return null;
  }
  return {
    r: channels[0],
    g: channels[1],
    b: channels[2],
    a: channels.length === 4 ? channels[3] / 255 : 1,
  };
}

function parseRgbColor(fill) {
  const match = fill.match(/^rgba?\(([^)]+)\)$/);
  if (!match) {
    return null;
  }
  const channels = match[1].split(",").map((part) => Number(part.trim()));
  if (channels.length < 3 || channels.length > 4 || channels.some((channel) => Number.isNaN(channel))) {
    return null;
  }
  return {
    r: channels[0],
    g: channels[1],
    b: channels[2],
    a: channels.length === 4 ? channels[3] : 1,
  };
}

function isOpaqueFill(fill) {
  const color = parseColor(fill);
  return !color || color.a >= 0.999;
}

function formatColor(color) {
  const r = clampColorChannel(color.r);
  const g = clampColorChannel(color.g);
  const b = clampColorChannel(color.b);
  const a = Math.max(0, Math.min(1, color.a));
  if (a >= 0.999) {
    return `rgb(${r}, ${g}, ${b})`;
  }
  return `rgba(${r}, ${g}, ${b}, ${Number(a.toFixed(3))})`;
}

function clampColorChannel(value) {
  return Math.max(0, Math.min(255, Math.round(value)));
}

function lightenChannel(value, light) {
  if (light < 0) {
    return Math.max(0, Math.min(255, Math.round(value + value * light)));
  }
  return Math.max(0, Math.min(255, Math.round(value + (255 - value) * light)));
}

function polygonPoints(points, fill) {
  if (!points?.length) {
    return;
  }
  ctx.fillStyle = fill;
  const expanded = expandPolygon(points, 0.35);
  ctx.beginPath();
  ctx.moveTo(expanded[0].x, expanded[0].y);
  for (const point of expanded.slice(1)) {
    ctx.lineTo(point.x, point.y);
  }
  ctx.closePath();
  ctx.fill();
}

function lineSegment(from, to, stroke, width, alpha = 1) {
  ctx.save();
  ctx.globalAlpha = clamp01(alpha);
  ctx.strokeStyle = stroke;
  ctx.lineWidth = width;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  ctx.moveTo(from.x, from.y);
  ctx.lineTo(to.x, to.y);
  ctx.stroke();
  ctx.restore();
}

function applyPixelatePostprocess() {
  const settings = pixelateSettings();
  if (!settings.enabled || settings.scale <= 1) {
    return;
  }
  const width = canvas.width;
  const height = canvas.height;
  if (width <= 1 || height <= 1) {
    return;
  }
  const targetWidth = Math.max(1, Math.ceil(width / settings.scale));
  const targetHeight = Math.max(1, Math.ceil(height / settings.scale));
  pixelateBuffer.width = targetWidth;
  pixelateBuffer.height = targetHeight;
  const bufferCtx = pixelateBuffer.getContext("2d", { alpha: true });
  bufferCtx.imageSmoothingEnabled = settings.smoothing;
  bufferCtx.clearRect(0, 0, targetWidth, targetHeight);
  bufferCtx.drawImage(canvas, 0, 0, width, height, 0, 0, targetWidth, targetHeight);

  ctx.save();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, width, height);
  ctx.drawImage(pixelateBuffer, 0, 0, targetWidth, targetHeight, 0, 0, width, height);
  ctx.restore();
}

function expandPolygon(points, amount) {
  const center = points.reduce(
    (acc, point) => ({ x: acc.x + point.x / points.length, y: acc.y + point.y / points.length }),
    { x: 0, y: 0 },
  );
  return points.map((point) => {
    const dx = point.x - center.x;
    const dy = point.y - center.y;
    const length = Math.hypot(dx, dy) || 1;
    return {
      x: point.x + (dx / length) * amount,
      y: point.y + (dy / length) * amount,
    };
  });
}

function cssVar(name) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

function themeColor(name) {
  return cssVar(name) || "currentColor";
}

function themeColorWithAlpha(name, alpha) {
  const color = parseColor(themeColor(name));
  if (!color) {
    return themeColor(name);
  }
  return formatColor({ ...color, a: alpha });
}

canvas.addEventListener("pointerdown", (event) => {
  if (!cameraLookEnabled()) {
    return;
  }
  if (event.button !== 0) {
    return;
  }
  view.dragging = true;
  view.pointerId = event.pointerId;
  view.lastPointerX = event.clientX;
  view.lastPointerY = event.clientY;
  canvas.setPointerCapture(event.pointerId);
  canvas.classList.add("is-dragging");
});

canvas.addEventListener("pointermove", (event) => {
  if (!cameraLookEnabled()) {
    return;
  }
  if (!view.dragging || event.pointerId !== view.pointerId) {
    return;
  }
  const deltaX = event.clientX - view.lastPointerX;
  const deltaY = event.clientY - view.lastPointerY;
  view.lastPointerX = event.clientX;
  view.lastPointerY = event.clientY;
  rotateCamera(deltaX, deltaY);
  draw();
});

function endCameraDrag(event) {
  if (!view.dragging) {
    return;
  }
  if (event.pointerId !== view.pointerId) {
    return;
  }
  view.dragging = false;
  view.pointerId = null;
  canvas.classList.remove("is-dragging");
  if (canvas.hasPointerCapture(event.pointerId)) {
    canvas.releasePointerCapture(event.pointerId);
  }
}

canvas.addEventListener("pointerup", endCameraDrag);
canvas.addEventListener("pointercancel", endCameraDrag);

canvas.addEventListener("wheel", (event) => {
  if (!cameraZoomEnabled()) {
    return;
  }
  event.preventDefault();
  zoomCamera(event.deltaY);
  draw();
}, { passive: false });

window.addEventListener("resize", () => schedulePuzzle3Resize(true));

if (window.ResizeObserver) {
  const resizeObserver = new ResizeObserver((entries) => {
    schedulePuzzle3Resize(entries.some((entry) => entry.target === screenView));
  });
  resizeObserver.observe(screenView);
  resizeObserver.observe(puzzle3Frame);
  resizeObserver.observe(canvas);
}

function handleStandaloneKeydown(event) {
  const control = sceneControlForEvent(event);
  if (control) {
    event.preventDefault();
    applySceneControl(control);
    return;
  }

  if (puzzle3ComponentFor(currentScene()) && puzzle3Component.handleKey(event)) {
    return;
  }
  applyPuzzle3CommandKey(event);
}

function handleComponentEmbedKeydown(event) {
  if (!puzzle3ComponentFor(currentScene())) {
    return;
  }
  const input = inputForRawInput({ key: event.key, code: event.code });
  if (input) {
    event.preventDefault();
    startHeldSceneInput(rawInputHoldId({ key: event.key, code: event.code }), input);
    return;
  }
  applyPuzzle3CommandKey(event);
}

function handleStandaloneKeyup(event) {
  stopSceneRawInput({ key: event.key, code: event.code });
}

function handleComponentEmbedKeyup(event) {
  stopSceneRawInput({ key: event.key, code: event.code });
}

if (inlineComponentMount) {
  // Inline controllers receive input through the host controller contract.
} else if (!effectiveComponentEmbedMode()) {
  window.addEventListener("keydown", handleStandaloneKeydown);
  window.addEventListener("keyup", handleStandaloneKeyup);
} else {
  window.addEventListener("keydown", handleComponentEmbedKeydown);
  window.addEventListener("keyup", handleComponentEmbedKeyup);
}

window.addEventListener("blur", stopAllHeldSceneInputs);

window.addEventListener("message", (event) => {
  if (event.data?.type === "PuzzleStudioSetScenePreview") {
    renderSceneEditorPreview(event.data || {});
    return;
  }

  if (event.data?.type === "PuzzleStudioRequestScenePreview") {
    notifySceneEditorPreview(String(event.data.requestId || ""));
    return;
  }

  if (event.data?.type === "PuzzleStudioUpdatePuzzle3Preview") {
    setEditorComponentEmbedMode(event.data.componentEmbed === true);
    applyPuzzle3PreviewUpdate(event.data);
    return;
  }

  if (event.data?.type === PREVIEW_SURFACE_UPDATE_MESSAGE) {
    const update = puzzle3PreviewUpdateFromSurface(event.data);
    if (update) {
      setEditorComponentEmbedMode(event.data.componentEmbed === true);
      applyPuzzle3ModelComponentPreviewUpdate(update);
    }
    return;
  }

  if (event.data?.type === "PuzzleStudioRenderPuzzle3ModelComponent") {
    setEditorComponentEmbedMode(event.data.componentEmbed !== false);
    applyPuzzle3ModelComponentPreviewUpdate(event.data);
    return;
  }

  if (event.data?.type === "PuzzleStudioSetPuzzle3Snapshot") {
    setEditorComponentEmbedMode(event.data.componentEmbed === true);
    void loadSnapshotData(event.data.snapshot, {
      scene: event.data.scene,
      preferPuzzleScene: Boolean(event.data.preferPuzzleScene),
    });
    return;
  }

  if (event.data?.type === "PuzzleStudioResize") {
    schedulePuzzle3Resize();
    return;
  }

  if (event.data?.type === "PuzzleStudioCommand") {
    const command = String(event.data.command || "");
    if (command === "undo" || command === "restart") {
      puzzle3Component.applyInput(command);
    } else if (command === "next_level") {
      puzzle3Component.nextLevel();
    } else if (command === "previous_level") {
      puzzle3Component.previousLevel();
    } else if (command === "goto_level" || command === "goto") {
      puzzle3Component.gotoLevel(event.data.level);
    } else if (command === "reset_camera") {
      puzzle3Component.resetCamera();
    }
    return;
  }

  if (event.data?.type === "PuzzleStudioKey") {
    if (!puzzle3ComponentFor(currentScene())) {
      return;
    }
    const raw = {
      key: String(event.data.key || ""),
      code: String(event.data.code || ""),
    };
    if (event.data.action === "up") {
      stopSceneRawInput(raw);
    } else if (!enqueueSceneRawInput(raw)) {
      applyPuzzle3CommandKey(raw);
    }
    return;
  }

  if (event.data?.type === "PuzzleStudioInput") {
    const input = String(event.data.input || "");
    if (!input || !puzzle3ComponentFor(currentScene())) {
      return;
    }
    applySceneInput(input);
  }
});

function inputForEvent(event) {
  return inputForRawInput({ key: event.key, code: event.code });
}

function applyPuzzle3CommandKey(event) {
  const input = puzzle3CommandInputForEvent(event);
  if (!input) {
    return false;
  }
  event.preventDefault?.();
  return puzzle3Component.applyInput(input);
}

function puzzle3CommandInputForEvent(event) {
  const key = String(event.key || "").toLowerCase();
  if (key === "z") {
    return "undo";
  }
  if (key === "r") {
    return "restart";
  }
  return null;
}

function inputForRawInput(raw) {
  const keys = rawKeyCandidates(raw);
  for (const input of snapshot.inputs) {
    if ((input.keys || []).some((binding) => keys.includes(normalizeRawKeyToken(binding)))) {
      return input.name;
    }
  }
  return null;
}

function rawKeyCandidates(raw) {
  const key = normalizeRawKeyToken(raw?.key);
  return key ? [key] : [];
}

function normalizeRawKeyToken(value) {
  const raw = String(value || "");
  if (raw === " ") {
    return "Space";
  }
  const token = raw.trim();
  if (!token) {
    return "";
  }
  if (token.length === 1) {
    return token.toLowerCase();
  }
  const lower = token.toLowerCase();
  const aliases = {
    arrow_up: "ArrowUp",
    arrowup: "ArrowUp",
    up_arrow: "ArrowUp",
    arrow_down: "ArrowDown",
    arrowdown: "ArrowDown",
    down_arrow: "ArrowDown",
    arrow_left: "ArrowLeft",
    arrowleft: "ArrowLeft",
    left_arrow: "ArrowLeft",
    arrow_right: "ArrowRight",
    arrowright: "ArrowRight",
    right_arrow: "ArrowRight",
    esc: "Escape",
    escape: "Escape",
    enter: "Enter",
    return: "Enter",
    " ": "Space",
    space: "Space",
    spacebar: "Space",
  };
  return aliases[lower] || token;
}

function canonicalPuzzle3InputName(inputName) {
  if (inputName === "forward") {
    return "front";
  }
  if (inputName === "backward") {
    return "back";
  }
  return inputName;
}

function sceneControlForEvent(event) {
  const scene = currentScene();
  const controls = scene?.controls || {};
  const key = normalizeRawKeyToken(event.key);
  const explicit = controls[key];
  if (explicit) {
    return explicit;
  }
  const keys = scene?.keys || {};
  const action = keys[key];
  return action || null;
}

function applySceneControl(control) {
  if (control.kind === "input") {
    applySceneInput(control.input);
  }
}

function applySceneInput(input) {
  const scene = currentScene();
  const rules = scene?.rules || [];
  const component = puzzle3ComponentFor(scene);
  if (!component) {
    return false;
  }
  if (rules.length === 0) {
    return puzzle3Component.applyInput(input);
  }
  let changed = false;
  for (const rule of rules) {
    if (rule.kind !== "component_rules" || rule.target !== component.source) {
      continue;
    }
    const componentInput = rule.inputMap?.[input] || input;
    changed = puzzle3Component.applyInput(componentInput) || changed;
  }
  return changed;
}

function enqueueSceneInput(input) {
  if (!input) {
    return false;
  }
  queuedSceneInputs.push(input);
  if (queuedSceneInputs.length > 4) {
    queuedSceneInputs = queuedSceneInputs.slice(-4);
  }
  scheduleQueuedSceneInput();
  return true;
}

function startHeldSceneInput(holdId, input) {
  if (!holdId || !input) {
    return false;
  }
  heldSceneInputs.set(holdId, input);
  return enqueueSceneInput(input);
}

function stopHeldSceneInput(holdId) {
  if (!heldSceneInputs.has(holdId)) {
    return false;
  }
  heldSceneInputs.delete(holdId);
  return true;
}

function stopAllHeldSceneInputs() {
  for (const holdId of [...heldSceneInputs.keys()]) {
    stopHeldSceneInput(holdId);
  }
}

function enqueueSceneRawInput(raw) {
  const input = inputForRawInput(raw);
  if (!input) {
    return false;
  }
  return startHeldSceneInput(rawInputHoldId(raw), input);
}

function stopSceneRawInput(raw) {
  return stopHeldSceneInput(rawInputHoldId(raw));
}

function rawInputHoldId(raw) {
  const code = String(raw?.code || "");
  const key = String(raw?.key || "");
  return code || key;
}

function scheduleQueuedSceneInput() {
  if (queuedSceneInputFrame) {
    return;
  }
  queuedSceneInputFrame = requestAnimationFrame(() => {
    queuedSceneInputFrame = 0;
    const input = queuedSceneInputs.shift();
    if (input) {
      applySceneInput(input);
    }
    if (queuedSceneInputs.length > 0) {
      scheduleQueuedSceneInput();
    }
  });
}

function applySceneRawInput(raw) {
  const input = inputForRawInput(raw);
  if (!input) {
    return false;
  }
  return applySceneInput(input);
}

const controllerApi = {
  element: puzzle3Frame,
  canvas,
  ready: null,
  update(update = {}) {
    const next = puzzle3PreviewSnapshot(update || {});
    this.ready = loadSnapshotData(next, {
      scene: update.scene,
      preferPuzzleScene: update.preferPuzzleScene !== false,
    });
    return this.ready;
  },
  replaceSessionSnapshot(nextSnapshot) {
    if (!sessionManaged) {
      throw new Error("Session snapshots require a session-managed Puzzle3 controller.");
    }
    this.ready = Promise.resolve(this.ready).then(() => loadSnapshotData(nextSnapshot, {
      scene: nextSnapshot?.currentScene,
      preserveCamera: true,
    }));
    return this.ready;
  },
  applyKey(event) {
    return puzzle3Component.handleKey(event || {}) || applyPuzzle3CommandKey(event || {});
  },
  releaseKey(event) {
    stopSceneRawInput({
      key: String(event?.key || ""),
      code: String(event?.code || ""),
    });
    return true;
  },
  applyInput(input) {
    return puzzle3Component.applyInput(String(input || ""));
  },
  command(command, payload = {}) {
    const name = String(command || "");
    if (name === "undo" || name === "restart") {
      return puzzle3Component.applyInput(name);
    }
    if (name === "next_level") {
      return puzzle3Component.nextLevel();
    }
    if (name === "previous_level") {
      return puzzle3Component.previousLevel();
    }
    if (name === "goto_level" || name === "goto") {
      return puzzle3Component.gotoLevel(payload.level);
    }
    if (name === "reset_camera") {
      return puzzle3Component.resetCamera();
    }
    return false;
  },
  snapshot() {
    return JSON.parse(JSON.stringify(requireLoadedPuzzle3Snapshot()));
  },
  onView(listener) {
    if (typeof listener !== "function") {
      return () => {};
    }
    viewListeners.add(listener);
    return () => viewListeners.delete(listener);
  },
  onStateChange(listener) {
    if (typeof listener !== "function") {
      return () => {};
    }
    stateListeners.add(listener);
    return () => stateListeners.delete(listener);
  },
  resize() {
    schedulePuzzle3Resize(true);
  },
  destroy() {
    viewListeners.clear();
    stateListeners.clear();
    stopAllHeldSceneInputs();
  },
};

resizeCanvas();
controllerApi.ready = loadPuzzle3ControllerSnapshot();

function cloneRuntimeSnapshot(source) {
  const runtimeContract = requireRuntimeContract(source);
  const runtimeModel = requireRuntimeContractPuzzle3Model(runtimeContract);
  const runtimeGame = requireRuntimeContractGame(runtimeContract);
  const runtimeLevelBundle = requireRuntimeContractLevelBundle(runtimeContract);
  return {
    ...source,
    size: { ...source.size },
    render: JSON.parse(JSON.stringify(source.render)),
    directions: cloneRuntimeRecord(source.directions || {}),
    directionSets: cloneRuntimeRecord(source.directionSets || {}),
    layerCount: runtimeLayerCount(runtimeGame),
    inputs: cloneRuntimeInputs(runtimeModel.inputs),
    rules: cloneRequiredJsonArray(runtimeModel.rules, "runtimeContract.model.rules"),
    winCondition: runtimeModel.winCondition
      ? JSON.parse(JSON.stringify(runtimeModel.winCondition))
      : null,
    runtimeContract: JSON.parse(JSON.stringify(runtimeContract)),
    objects: cloneRuntimeObjects(source.objects || {}),
    sprites: cloneRuntimeSprites(source.sprites || {}),
    cells: cloneRuntimeCells(source.cells || []),
    levels: cloneRuntimeContractLevels(runtimeLevelBundle, source.levels || []),
    levelBundles: cloneRuntimeLevelBundles(source.levelBundles || {}),
  };
}

function requireRuntimeContract(source) {
  const contract = source?.runtimeContract;
  if (!contract || typeof contract !== "object") {
    throw new Error("Puzzle3 runtime fixture is missing runtimeContract.");
  }
  if (Number(contract.version) !== PUZZLE3_RUNTIME_CONTRACT_VERSION) {
    throw new Error(`Unsupported Puzzle3 runtimeContract version: ${contract.version}`);
  }
  requireRuntimeContractPuzzle3Model(contract);
  requireRuntimeContractGame(contract);
  requireRuntimeContractLevelBundle(contract);
  requireRuntimeContractLifecycle(contract);
  return contract;
}

function requireRuntimeContractPuzzle3Model(contract) {
  const model = contract?.model;
  if (!model || typeof model !== "object" || Array.isArray(model)) {
    throw new Error("Puzzle3 runtimeContract.model is missing or invalid.");
  }
  if (model.kind !== "puzzle3") {
    throw new Error(`Unsupported runtimeContract model kind: ${model.kind}`);
  }
  return model;
}

function requireRuntimeContractGame(contract) {
  const game = requireRuntimeContractPuzzle3Model(contract).game;
  if (!game || typeof game !== "object" || Array.isArray(game)) {
    throw new Error("Puzzle3 runtimeContract.model.game is missing or invalid.");
  }
  runtimeLayerCount(game);
  if (!Array.isArray(game.objects)) {
    throw new Error("Puzzle3 runtimeContract.model.game.objects is missing or invalid.");
  }
  if (!Array.isArray(game.inputs)) {
    throw new Error("Puzzle3 runtimeContract.model.game.inputs is missing or invalid.");
  }
  return game;
}

function requireRuntimeContractLevelBundle(contract) {
  const levelBundle = requireRuntimeContractPuzzle3Model(contract).levelBundle;
  if (!levelBundle || typeof levelBundle !== "object" || Array.isArray(levelBundle)) {
    throw new Error("Puzzle3 runtimeContract.model.levelBundle is missing or invalid.");
  }
  if (!Array.isArray(levelBundle.levels) || levelBundle.levels.length === 0) {
    throw new Error("Puzzle3 runtimeContract.model.levelBundle.levels is missing or empty.");
  }
  return levelBundle;
}

function requireRuntimeContractLifecycle(contract) {
  const lifecycle = requireRuntimeContractPuzzle3Model(contract).lifecycle;
  if (!lifecycle || typeof lifecycle !== "object" || Array.isArray(lifecycle)) {
    throw new Error("Puzzle3 runtimeContract.model.lifecycle is missing or invalid.");
  }
  for (const key of ["onLevelStart", "onLevelClear", "onLastLevelClear"]) {
    if (lifecycle[key] !== null && lifecycle[key] !== undefined && !Array.isArray(lifecycle[key])) {
      throw new Error(`Puzzle3 runtimeContract.model.lifecycle.${key} is invalid.`);
    }
  }
  return lifecycle;
}

function cloneRuntimeRecord(record) {
  return Object.fromEntries(
    Object.entries(record).map(([key, value]) => [
      key,
      Array.isArray(value) ? [...value] : { ...value },
    ]),
  );
}

function cloneRuntimeObjects(objects) {
  return Object.fromEntries(
    Object.entries(objects).map(([name, object]) => [name, { ...object }]),
  );
}

function cloneRuntimeSprites(sprites) {
  return Object.fromEntries(
    Object.entries(sprites).map(([name, sprite]) => [
      name,
      {
        ...sprite,
        palette: { ...(sprite.palette || {}) },
        frames: (sprite.frames || []).map((frame) => ({
          ...frame,
          layers: (frame.layers || []).map((layer) => [...layer]),
        })),
      },
    ]),
  );
}

function cloneRuntimeCells(cells) {
  return cells.map((cell) => ({
    position: { ...cell.position },
    objects: (cell.objects || []).map((object) => (
      object && typeof object === "object" ? { ...object } : object
    )),
  }));
}

function cloneRuntimeInputs(inputs) {
  return inputs.map((input, index) => ({
    ...input,
    id: runtimeInputId(input, `runtimeContract.model.inputs[${index}]`),
    name: runtimeInputName(input, `runtimeContract.model.inputs[${index}]`),
    keys: Array.isArray(input.keys) ? [...input.keys] : [],
  }));
}

function cloneRequiredJsonArray(value, label) {
  if (!Array.isArray(value)) {
    throw new Error(`Puzzle3 ${label} must be an array.`);
  }
  return JSON.parse(JSON.stringify(value));
}

function cloneRuntimeContractLevels(levelBundle, presentationLevels = []) {
  const levels = levelBundle.levels.map((entry, index) => {
    const label = `runtimeContract.model.levelBundle.levels[${index}]`;
    if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
      throw new Error(`Puzzle3 ${label} is missing or invalid.`);
    }
    const name = runtimeString(entry.name, `${label}.name`);
    const level = entry.level;
    if (!level || typeof level !== "object" || Array.isArray(level)) {
      throw new Error(`Puzzle3 ${label}.level is missing or invalid.`);
    }
    const size = runtimeSize(level.size, `${label}.level.size`);
    const variables = cloneRequiredJsonArray(level.variables, `${label}.level.variables`)
      .map((value, variableIndex) => runtimeInteger(
        value,
        `${label}.level.variables[${variableIndex}]`,
      ));
    if (!Array.isArray(level.cells)) {
      throw new Error(`Puzzle3 ${label}.level.cells must be an array.`);
    }
    const cells = level.cells.map((cell, cellIndex) => {
      const cellLabel = `${label}.level.cells[${cellIndex}]`;
      const position = runtimeCellPosition(cell, cellLabel);
      if (!runtimePositionInBounds(position, size)) {
        throw new Error(
          `Puzzle3 ${cellLabel}.position ${cellKey(position)} is outside level bounds `
          + `${runtimeSizeLabel(size)}.`,
        );
      }
      return {
        position,
        objects: runtimeCellObjectIds(cell, cellLabel),
      };
    });
    const presentation = presentationLevels[index] || {};
    return {
      name,
      label: presentation.label || name,
      size,
      cells,
      variables,
    };
  });
  if (levels.length === 0) {
    throw new Error("Puzzle3 runtimeContract.model.levelBundle.levels is empty.");
  }
  return levels;
}

function cloneRuntimeLevelBundles(levelBundles) {
  return Object.fromEntries(
    Object.entries(levelBundles).map(([name, indexes]) => [
      name,
      Array.isArray(indexes) ? indexes.map((index) => Number(index)) : [],
    ]),
  );
}

function runtimeLayerCount(game) {
  return runtimePositiveInteger(game.layer_count, "runtimeContract.model.game.layer_count");
}

function runtimeSemanticObjectsById(game) {
  const layerCount = runtimeLayerCount(game);
  const objects = new Map();
  for (const [index, object] of game.objects.entries()) {
    const label = `runtimeContract.model.game.objects[${index}]`;
    if (!object || typeof object !== "object" || Array.isArray(object)) {
      throw new Error(`Puzzle3 ${label} is missing or invalid.`);
    }
    const id = runtimeObjectId(object.id, `${label}.id`);
    const layerId = runtimeLayerId(object.layer_id, `${label}.layer_id`);
    if (layerId >= layerCount) {
      throw new Error(`Puzzle3 ${label}.layer_id ${layerId} is outside layer count ${layerCount}.`);
    }
    if (objects.has(id)) {
      throw new Error(`Puzzle3 runtimeContract.model.game.objects contains duplicate object id ${id}.`);
    }
    objects.set(id, { id, layerId });
  }
  return objects;
}

function runtimePresentationObjectsById(objects) {
  const map = new Map();
  for (const [name, object] of Object.entries(objects || {})) {
    if (!object || typeof object !== "object" || Array.isArray(object)) {
      throw new Error(`Puzzle3 sprite object metadata for ${name} is invalid.`);
    }
    const id = runtimeObjectId(object.id, `sprite object ${name}.id`);
    if (map.has(id)) {
      throw new Error(`Puzzle3 sprite object metadata contains duplicate object id ${id}.`);
    }
    map.set(id, { ...object });
  }
  return map;
}

function runtimeSize(size, label) {
  if (!size || typeof size !== "object" || Array.isArray(size)) {
    throw new Error(`Puzzle3 ${label} is missing or invalid.`);
  }
  return {
    width: runtimePositiveInteger(size.width, `${label}.width`),
    depth: runtimePositiveInteger(size.depth, `${label}.depth`),
    height: runtimePositiveInteger(size.height, `${label}.height`),
  };
}

function runtimeCellPosition(cell, label) {
  if (!cell || typeof cell !== "object" || Array.isArray(cell)) {
    throw new Error(`Puzzle3 ${label} is missing or invalid.`);
  }
  const position = cell.position;
  if (!position || typeof position !== "object" || Array.isArray(position)) {
    throw new Error(`Puzzle3 ${label}.position is missing or invalid.`);
  }
  return {
    x: runtimeUnsignedInteger(position.x, `${label}.position.x`),
    y: runtimeUnsignedInteger(position.y, `${label}.position.y`),
    z: runtimeUnsignedInteger(position.z, `${label}.position.z`),
  };
}

function runtimeCellObjectIds(cell, label) {
  if (!Array.isArray(cell.objects)) {
    throw new Error(`Puzzle3 ${label}.objects must be an array.`);
  }
  return cell.objects.map((object, index) => {
    const value = object && typeof object === "object" ? object.id : object;
    return runtimeObjectId(value, `${label}.objects[${index}]`);
  });
}

function runtimePositionInBounds(position, size) {
  return position.x < size.width
    && position.y < size.depth
    && position.z < size.height;
}

function runtimeSizeLabel(size) {
  return `${size.width}x${size.depth}x${size.height}`;
}

function runtimeObjectId(value, label) {
  return runtimePositiveInteger(value, label);
}

function runtimeInputId(input, label) {
  return runtimeUnsignedInteger(input?.id, `${label}.id`);
}

function runtimeInputName(input, label) {
  return runtimeString(input?.name, `${label}.name`);
}

function runtimeLayerId(value, label) {
  return runtimeUnsignedInteger(value, label);
}

function runtimeString(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`Puzzle3 ${label} must be a non-empty string.`);
  }
  return value;
}

function runtimePositiveInteger(value, label) {
  const number = runtimeUnsignedInteger(value, label);
  if (number <= 0) {
    throw new Error(`Puzzle3 ${label} must be greater than zero.`);
  }
  return number;
}

function runtimeUnsignedInteger(value, label) {
    const number = Number(value);
  if (!Number.isInteger(number) || number < 0) {
    throw new Error(`Puzzle3 ${label} must be an unsigned integer.`);
  }
  return number;
}

function runtimeInteger(value, label) {
  const number = Number(value);
  if (!Number.isSafeInteger(number)) {
    throw new Error(`Puzzle3 ${label} must be an integer.`);
  }
  return number;
}

function stateFromRuntimeCells(runtime, cells, size, variables) {
  const layerCount = runtime.runtimeLayerCount();
  const width = runtimePositiveInteger(size.width, "runtime level size.width");
  const depth = runtimePositiveInteger(size.depth, "runtime level size.depth");
  const height = runtimePositiveInteger(size.height, "runtime level size.height");
  const slots = new Array(width * depth * height * layerCount).fill(0);
  for (const [index, cell] of cells.entries()) {
    const position = runtimeCellPosition(cell, `runtime level cells[${index}]`);
    if (!runtimePositionInBounds(position, { width, depth, height })) {
      throw new Error(
        `Puzzle3 runtime level cell ${cellKey(position)} is outside level bounds `
        + `${runtimeSizeLabel({ width, depth, height })}.`,
      );
    }
    for (const objectId of runtimeCellObjectIds(cell, `runtime level cells[${index}]`)) {
      const layer = runtime.objectLayer(objectId);
      slots[runtimeSlotIndex(width, depth, layerCount, position.x, position.y, position.z, layer)] = objectId;
    }
  }
  return {
    kind: "puzzle3d",
    width,
    depth,
    height,
    layerCount,
    slots,
    variables: [...variables],
    levelFiredRules: [],
  };
}

function cellsFromRuntimeState(runtime, state) {
  const cells = [];
  const width = runtimePositiveInteger(state.width, "runtime state.width");
  const depth = runtimePositiveInteger(state.depth, "runtime state.depth");
  const height = runtimePositiveInteger(state.height, "runtime state.height");
  const layerCount = runtimeUnsignedInteger(state.layerCount, "runtime state.layerCount");
  if (layerCount !== runtime.runtimeLayerCount()) {
    throw new Error(
      `Puzzle3 runtime state layerCount ${layerCount} does not match contract layer count `
      + `${runtime.runtimeLayerCount()}.`,
    );
  }
  if (!Array.isArray(state.slots)) {
    throw new Error("Puzzle3 runtime state.slots must be an array.");
  }
  const expectedSlots = width * depth * height * layerCount;
  if (state.slots.length !== expectedSlots) {
    throw new Error(
      `Puzzle3 runtime state.slots length ${state.slots.length} does not match expected `
      + `${expectedSlots}.`,
    );
  }
  for (let z = 0; z < height; z += 1) {
    for (let y = 0; y < depth; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const objects = [];
        for (let layer = 0; layer < layerCount; layer += 1) {
          const slotIndex = runtimeSlotIndex(width, depth, layerCount, x, y, z, layer);
          const objectId = runtimeUnsignedInteger(state.slots[slotIndex], `runtime state.slots[${slotIndex}]`);
          if (objectId) {
            objects.push(runtime.objectForId(objectId));
          }
        }
        if (objects.length) {
          cells.push({ position: { x, y, z }, objects });
        }
      }
    }
  }
  return cells;
}

function runtimeSlotIndex(width, depth, layerCount, x, y, z, layer) {
  return ((((z * depth) + y) * width + x) * layerCount) + layer;
}

function cloneRuntimeState(state) {
  return {
    ...state,
    slots: [...(state.slots || [])],
    levelFiredRules: [...(state.levelFiredRules || [])],
  };
}

function runtimeStateKey(state) {
  return JSON.stringify({
    width: state.width,
    depth: state.depth,
    height: state.height,
    layerCount: state.layerCount,
    slots: state.slots || [],
    levelFiredRules: state.levelFiredRules || [],
  });
}

function normalizeSnapshot(source) {
  const runtimeContract = requireRuntimeContract(source);
  const levels = cloneRuntimeContractLevels(
    requireRuntimeContractLevelBundle(runtimeContract),
    source.levels || [],
  );
  const levelIndex = clampIndex(source.levelIndex || 0, levels.length);
  const currentLevel = levels[levelIndex];
  return {
    ...source,
    levelIndex,
    levels,
    size: source.size || currentLevel.size,
    cells: Array.isArray(source.cells) ? source.cells : currentLevel.cells,
    levelName: source.levelName || currentLevel.name,
    levelLabel: source.levelLabel || currentLevel.label,
  };
}

function standardSpriteGridPosition(size, column, row, slice) {
  return {
    x: column,
    y: Math.max(0, Number(size.depth || 1) - 1 - row),
    z: Math.max(0, Number(size.height || 1) - 1 - slice),
  };
}

function clampIndex(index, length) {
  return Math.max(0, Math.min(Math.max(0, length - 1), index));
}
return controllerApi;
}

window.Puzzle3Controller = {
  attach(canvas, options = {}) {
    return createPuzzle3Controller({ ...options, canvas, mountMode: "inline" });
  },
  boot(options = {}) {
    return createPuzzle3Controller(options);
  },
};

if (window.Puzzle3ControllerAutoBoot !== false) {
  window.Puzzle3ControllerInstance = window.Puzzle3Controller.boot();
}
})();
