const screenView = document.querySelector("#screenView") || document.body;
const componentEmbedMode = new URLSearchParams(window.location.search).get("component") === "1"
  || window.Puzzle3DComponentEmbed === true;
let editorComponentEmbedMode = false;
document.documentElement.classList.toggle("is-component-embed", componentEmbedMode);
let activeThemeClass = "";
const activeThemeVariables = new Set();
applyTheme({ name: "clean" });
document.body.classList.toggle("is-component-embed", componentEmbedMode);
const puzzle3Frame = ensurePuzzle3ComponentFrame();
const canvas = puzzle3Frame.querySelector("#view");
const ctx = canvas.getContext("2d");
const PUZZLE3_APP_CAMERA_MIN_PITCH_DEGREES = -90;
const PUZZLE3_APP_CAMERA_MAX_PITCH_DEGREES = 90;

function ensurePuzzle3ComponentFrame() {
  let existing = document.querySelector("#view");
  let frame = existing?.closest(".puzzle3-component") || document.createElement("div");
  frame.className = "puzzle3-component";
  if (!existing) {
    existing = document.createElement("canvas");
    existing.id = "view";
    existing.width = 960;
    existing.height = 640;
    existing.setAttribute("aria-label", "Puzzle3 component");
  }
  if (existing.parentElement !== frame) {
    frame.append(existing);
  }
  return frame;
}

const fallbackSnapshot = {
  size: { width: 3, depth: 3, height: 3 },
  camera: {
    yawDegrees: 15,
    pitchDegrees: 55,
    zoom: 1,
  },
  view: {
    zoom: 1,
  },
  settings: {
    interactiveLook: false,
    interactiveZoom: false,
    grid: { visibility: 0 },
    shade: true,
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
let runtime = null;
let initialCamera = cloneCamera(fallbackSnapshot.camera);
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
const puzzle3Component = createPuzzle3Component();

async function loadSnapshot() {
  let nextSnapshot = null;
  try {
    if (window.Puzzle3DFixture) {
      nextSnapshot = window.Puzzle3DFixture;
    } else {
      const response = await fetch("./fixture.json", { cache: "no-store" });
      if (!response.ok) {
        throw new Error(response.statusText);
      }
      nextSnapshot = await response.json();
    }
  } catch {
    nextSnapshot = fallbackSnapshot;
  }
  const initialModelPreview = window.PuzzleStudioInitialModelComponentPreview;
  if (initialModelPreview?.type === "PuzzleStudioRenderPuzzle3ModelComponent") {
    window.PuzzleStudioInitialModelComponentPreviewConsumed = true;
    const next = puzzle3PreviewSnapshot(initialModelPreview, nextSnapshot);
    await loadSnapshotData(next, puzzle3ModelComponentPreviewLoadOptions(initialModelPreview));
    return;
  }
  await loadSnapshotData(nextSnapshot);
}

async function loadSnapshotData(source, options = {}) {
  snapshot = normalizeSnapshot(source || fallbackSnapshot);
  runtime = await createPuzzle3Runtime(snapshot);
  snapshot = runtime.snapshot();
  editorModelComponentPreview = options.modelComponentPreview || null;
  document.title = snapshot.title || "Puzzle3";
  currentSceneName = editorModelComponentPreview?.sceneName
    || options.scene
    || (options.preferPuzzleScene ? puzzleSceneName(snapshot) : "")
    || initialSceneName(snapshot);
  applyTheme(snapshot.theme || { name: "clean" });
  initialCamera = cloneCamera(snapshot.camera || fallbackSnapshot.camera);
  view.projectionFitKey = "";
  resetRenderGeometryCache();
  resetViewportMotion();
  renderScene();
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
      if (!key.startsWith("--")) {
        continue;
      }
      document.documentElement.style.setProperty(key, String(value));
      activeThemeVariables.add(key);
    }
  }
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
    variables: theme.variables && typeof theme.variables === "object" ? theme.variables : {},
  };
}

function normalizeThemeName(name) {
  return String(name || "")
    .trim()
    .replace(/^theme-/, "")
    .replace(/[^a-zA-Z0-9_-]/g, "")
    || "clean";
}

async function createPuzzle3Runtime(initialSnapshot) {
  const source = String(window.Puzzle3DSource || initialSnapshot.source || "");
  const puzzlePath = String(window.Puzzle3DPath || initialSnapshot.puzzlePath || "game.puzzle");
  const module = await window.PuzzleRuntimeWasmLoader.load(source.length || Date.now());
  if (typeof module.WasmPuzzle3Runtime !== "function") {
    throw new Error("Puzzle3 WASM runtime is unavailable.");
  }
  return new Puzzle3SessionRuntime(initialSnapshot, new module.WasmPuzzle3Runtime(source, puzzlePath));
}

class Puzzle3SessionRuntime {
  constructor(initialSnapshot, coreRuntime) {
    this.base = cloneRuntimeSnapshot(initialSnapshot);
    this.coreRuntime = coreRuntime;
    this.camera = cloneCamera(initialSnapshot.camera);
    this.levels = cloneRuntimeLevels(initialSnapshot.levels?.length
      ? initialSnapshot.levels
      : [runtimeSnapshotLevel(initialSnapshot)]);
    this.levelIndex = clampIndex(initialSnapshot.levelIndex || 0, this.levels.length);
    this.undoStack = [];
    this.moveCount = 0;
    this.cellsByKey = new Map();
    this.cells = [];
    this.initialStateHandle = null;
    this.completed = false;
    this.loadLevel(this.levelIndex);
  }

  snapshot() {
    const level = this.currentLevel();
    return {
      ...this.base,
      size: { ...level.size },
      camera: cloneCamera(this.camera),
      cells: this.cells.map((cell) => ({
        position: { ...cell.position },
        objects: cell.objects.map((object) => ({ ...object })),
      })),
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
    if (inputId === undefined) {
      return false;
    }
    const before = this.historyEntry();
    const outcome = this.transitionCurrent("main", inputId);
    if (outcome.changed !== true) {
      return false;
    }
    this.undoStack.push(before);
    this.applyRuntimeCells(outcome.changedCells || []);
    this.moveCount += 1;
    const wasCompleted = this.completed;
    this.completed = outcome.completed === true;
    if (!wasCompleted && this.completed) {
      this.runLevelClearLifecycle();
    }
    return true;
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
    this.loadLevel(this.levelIndex + 1);
    return true;
  }

  previousLevel() {
    if (!this.hasPreviousLevel()) {
      return false;
    }
    this.loadLevel(this.levelIndex - 1);
    return true;
  }

  hasNextLevel() {
    return this.levelIndex + 1 < this.levels.length;
  }

  hasPreviousLevel() {
    return this.levelIndex > 0;
  }

  loadLevel(levelIndex) {
    this.levelIndex = clampIndex(levelIndex, this.levels.length);
    this.loadInitialStateForCurrentLevel();
    this.undoStack = [];
    this.moveCount = 0;
    this.completed = this.coreRuntime.is_current_complete() === true;
    this.initialStateHandle = this.coreRuntime.save_current_state();
  }

  loadInitialStateForCurrentLevel() {
    const level = this.currentLevel();
    const raw = stateFromRuntimeCells(this, level.cells, level.size);
    this.coreRuntime.set_state(JSON.stringify(raw));
    this.cellsByKey.clear();
    this.cells = [];
    this.applyRuntimeCells(level.cells);
    const outcome = this.transitionCurrent("level_start", 0);
    this.applyRuntimeCells(outcome.changedCells || []);
    this.completed = outcome.completed === true;
  }

  currentLevel() {
    return this.levels[this.levelIndex];
  }

  runLevelClearLifecycle() {
    for (const command of this.base.lifecycle?.onLevelClear || []) {
      if (command === "next_level") {
        this.nextLevel();
      }
    }
  }

  transitionCurrent(programKey, inputId) {
    const raw = this.coreRuntime.transition_current_outcome(
      programKey,
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
    for (const cell of cells || []) {
      const position = cell.position || {};
      const normalized = {
        x: Number(position.x || 0),
        y: Number(position.y || 0),
        z: Number(position.z || 0),
      };
      const key = cellKey(normalized);
      const objects = (cell.objects || [])
        .map((object) => this.objectForId(Number(object?.id ?? object ?? 0)))
        .filter((object) => object.id);
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
    const input = (this.base.inputs || [])
      .find((candidate) => canonicalPuzzle3InputName(candidate.name) === canonicalName);
    return input ? Number(input.id) : undefined;
  }

  objectForId(objectId) {
    const object = Object.values(this.base.objects || {}).find((candidate) => candidate.id === Number(objectId));
    return object ? { ...object } : { id: Number(objectId), name: `Object ${objectId}`, sprite: `Object ${objectId}` };
  }

  objectLayer(objectId) {
    const object = Object.values(this.base.objects || {}).find((candidate) => candidate.id === Number(objectId));
    return Number(object?.layer ?? 0);
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

function puzzle3ModelComponentPreviewLoadOptions(update = {}) {
  return {
    modelComponentPreview: {
      sceneName: update.scene || "__editor_model_preview__",
      component: puzzle3ModelPreviewComponent(update),
    },
  };
}

function puzzle3PreviewSnapshot(update = {}, source = snapshot || fallbackSnapshot) {
  const next = JSON.parse(JSON.stringify(source || fallbackSnapshot));
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
    next.camera = cloneCamera(update.camera);
  }
  if (update.view) {
    next.view = clonePuzzle3PreviewView(update.view, next.size || fallbackSnapshot.size);
  }
  if (update.settings) {
    next.settings = mergePuzzle3PreviewSettings(next.settings || {}, update.settings);
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

function mergePuzzle3PreviewSettings(base, patch) {
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
  screenView.className = `scene ${sceneName}`;
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
      screenView.replaceChildren(puzzle3Frame);
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
      runtime.setCamera(snapshot.camera);
      const beforeLevelIndex = snapshot.levelIndex || 0;
      if (input === "undo") {
        if (!runtime.undo()) {
          return false;
        }
      } else if (input === "restart") {
        if (!runtime.restart()) {
          return false;
        }
        resetViewportMotion();
      } else if (!runtime.applyInput(input)) {
        return false;
      }
      snapshot = runtime.snapshot();
      if ((snapshot.levelIndex || 0) !== beforeLevelIndex) {
        resetViewportMotion();
      }
      requestSceneViewportDraw();
      return true;
    },
    nextLevel() {
      if (!runtime.nextLevel()) {
        return false;
      }
      snapshot = runtime.snapshot();
      resetViewportMotion();
      draw();
      return true;
    },
    previousLevel() {
      if (!runtime.previousLevel()) {
        return false;
      }
      snapshot = runtime.snapshot();
      resetViewportMotion();
      draw();
      return true;
    },
    gotoLevel(level) {
      const index = puzzle3LevelIndex(level);
      if (index === null) {
        return false;
      }
      runtime.loadLevel(index);
      snapshot = runtime.snapshot();
      resetViewportMotion();
      draw();
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

function resetProjection(rect = canvas.getBoundingClientRect()) {
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

function globalLevelIndexForBundle(levelsName, relativeIndex) {
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

function relativeLevelIndexForBundle(levelsName, globalIndex) {
  const indexes = levelIndexesForBundle(levelsName);
  const relative = indexes.indexOf(globalIndex);
  return relative >= 0 ? relative : 0;
}

function resizeCanvas() {
  const rect = canvas.getBoundingClientRect();
  const scale = window.devicePixelRatio || 1;
  const nextWidth = Math.max(1, Math.floor(rect.width * scale));
  const nextHeight = Math.max(1, Math.floor(rect.height * scale));
  const changed = canvas.width !== nextWidth || canvas.height !== nextHeight;
  canvas.width = nextWidth;
  canvas.height = nextHeight;
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  updateProjectionFit(rect);
  return changed;
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
  const rect = canvas.getBoundingClientRect();
  const scale = window.devicePixelRatio || 1;
  const nextWidth = Math.max(1, Math.floor(rect.width * scale));
  const nextHeight = Math.max(1, Math.floor(rect.height * scale));
  if (canvas.width !== nextWidth || canvas.height !== nextHeight) {
    resizeCanvas();
  }
}

function updateProjectionFit(rect) {
  const size = snapshot.size || fallbackSnapshot.size;
  const camera = snapshot.camera || fallbackSnapshot.camera;
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
  const rect = canvas.getBoundingClientRect();
  const size = snapshot.size || fallbackSnapshot.size;
  if (activeViewportFocusCell()) {
    return;
  }
  if (!shouldAutoFitFiniteStage(size)) {
    return;
  }
  const width = Math.max(1, Number(rect.width) || 1);
  const height = Math.max(1, Number(rect.height) || 1);
  const key = projectionFitKey(size, snapshot.camera || fallbackSnapshot.camera);
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
  const yaw = degreesToRadians(camera.yawDegrees ?? 0);
  const pitch = degreesToRadians(camera.pitchDegrees ?? 35);
  const center = {
    x: (size.width - 1) / 2,
    y: (size.depth - 1) / 2,
    z: (size.height - 1) / 2,
  };
  const x = position.x - center.x;
  const y = position.y - center.y;
  const z = position.z - center.z;
  const yawX = x * Math.cos(yaw) - y * Math.sin(yaw);
  const yawY = x * Math.sin(yaw) + y * Math.cos(yaw);
  return {
    x: yawX,
    y: -yawY * Math.sin(pitch) - z * Math.cos(pitch),
  };
}

function cloneCamera(camera) {
  return {
    yawDegrees: Number(camera?.yawDegrees ?? fallbackSnapshot.camera.yawDegrees),
    pitchDegrees: Number(camera?.pitchDegrees ?? fallbackSnapshot.camera.pitchDegrees),
    zoom: Number(camera?.zoom ?? fallbackSnapshot.camera.zoom),
  };
}

function resetCamera() {
  snapshot.camera = cloneCamera(initialCamera);
  runtime.setCamera(snapshot.camera);
  resetViewportMotion();
}

function cameraLookEnabled() {
  return Boolean(snapshot.settings?.interactiveLook ?? snapshot.settings?.debugCamera);
}

function cameraZoomEnabled() {
  return Boolean(snapshot.settings?.interactiveZoom);
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
  const camera = snapshot.camera || fallbackSnapshot.camera;
  camera.yawDegrees = normalizeDegrees(camera.yawDegrees + deltaX * 0.35);
  camera.pitchDegrees = clamp(
    camera.pitchDegrees - deltaY * 0.25,
    PUZZLE3_APP_CAMERA_MIN_PITCH_DEGREES,
    PUZZLE3_APP_CAMERA_MAX_PITCH_DEGREES,
  );
  snapshot.camera = camera;
  resetProjection();
}

function zoomCamera(deltaY) {
  const camera = snapshot.camera || fallbackSnapshot.camera;
  const currentZoom = Number(camera.zoom ?? fallbackSnapshot.camera.zoom);
  camera.zoom = clamp(currentZoom * Math.exp(-deltaY * 0.001), 0.1, 8);
  snapshot.camera = camera;
}

function normalizeDegrees(value) {
  return ((value % 360) + 360) % 360;
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function projectWithDepth(position) {
  const camera = snapshot.camera || fallbackSnapshot.camera;
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
  const focusCell = viewport?.mode === "centered" ? viewportFocusCell(viewport) : null;
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
  const camera = snapshot.camera || fallbackSnapshot.camera;
  return Math.max(16, target.cellScale * projectionZoom(camera) * 3.5);
}

function viewportProjectionFitTarget(renderContext) {
  const viewport = renderContext?.viewport || null;
  if (!viewport || viewport.mode !== "centered") {
    return null;
  }
  const size = snapshot.size || fallbackSnapshot.size;
  const camera = snapshot.camera || fallbackSnapshot.camera;
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
  const raw = source?.viewport;
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
  return viewport?.mode === "centered" ? viewportFocusCell(viewport) : null;
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
  const xRange = virtualCenteredCellRange(Number(position.x) || 0, viewport.framingBox.width);
  const yRange = virtualCenteredCellRange(Number(position.y) || 0, viewport.framingBox.depth);
  const zRange = viewport.framingBox.height === "full"
    ? { min: -0.5, max: height - 0.5 }
    : virtualCenteredCellRange(Number(position.z) || 0, viewport.framingBox.height);
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
  const camera = snapshot.camera || fallbackSnapshot.camera;
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
  const camera = snapshot.camera || fallbackSnapshot.camera;
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
  const raw = snapshot.settings?.fitContent ?? snapshot.settings?.fit_content;
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
  if (!effectiveComponentEmbedMode() || !window.parent || window.parent === window) {
    return;
  }
  const size = snapshot.size || fallbackSnapshot.size;
  const normalizedSize = normalizeModelSize(size);
  const camera = snapshot.camera || fallbackSnapshot.camera;
  const previewView = puzzle3PreviewView();
  const canvasRect = canvas.getBoundingClientRect();
  window.parent.postMessage({
    type: "PuzzleStudioPuzzle3View",
    source: canvas.dataset.source || "",
    scene: canvas.dataset.scene || "",
    view: {
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
    },
  }, "*");
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
      effectiveScale: view.cellScale * projectionZoom(snapshot.camera || fallbackSnapshot.camera),
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
  const raw = snapshot.settings?.grid;
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
  const raw = snapshot.settings?.shade ?? snapshot.settings?.sprite;
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
  const raw = snapshot.settings?.pixelate ?? snapshot.settings?.pixel ?? false;
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
  return { camera: snapshot.camera || fallbackSnapshot.camera };
}

function gridStroke(kind, grid) {
  if (kind === "stageFrame") {
    return grid.frameColor || "rgba(29, 37, 44, 0.82)";
  }
  return grid.color || "rgba(31, 36, 40, 0.62)";
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
      fill: "rgba(15, 23, 42, 0.16)",
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
  if (!viewport || viewport.mode !== "centered") {
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
    const objectOrder = objectRenderOrder(object, objectIndex);
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
    objectRenderOrder(object),
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
  for (const voxel of ordered) {
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

function objectRenderOrder(object, fallbackIndex = 0) {
  const layer = Number(object?.layer);
  if (Number.isFinite(layer)) {
    return layer;
  }
  return Number(fallbackIndex) || 0;
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
  const cached = spriteVoxelTemplateCache.get(sprite);
  if (cached) {
    return cached;
  }
  const template = buildSpriteVoxelTemplate(sprite);
  spriteVoxelTemplateCache.set(sprite, template);
  return template;
}

function buildSpriteVoxelTemplate(sprite) {
  const blocks = bitmapBlocks(sprite.bitmap || []);
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
          x: (grid.x + 0.5) * scale - 0.5,
          y: (grid.y + 0.5) * scale - 0.5,
          z: (grid.z + 0.5) * scale - 0.5,
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
          x: position.x + (grid.x + 0.5) * scale - 0.5,
          y: position.y + (grid.y + 0.5) * scale - 0.5,
          z: position.z + (grid.z + 0.5) * scale - 0.5,
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
    order: gridOrder(ownerCell.position),
    depth: projectWithDepth(ownerCell.position).depth,
  };
}

function bitmapBlocks(bitmap) {
  const blocks = [[]];
  for (const row of bitmap) {
    if (row === "") {
      blocks.push([]);
    } else {
      blocks[blocks.length - 1].push(row);
    }
  }
  return blocks;
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
  const bufferCtx = pixelateBuffer.getContext("2d");
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

  if (event.code === "KeyZ") {
    event.preventDefault();
    puzzle3Component.applyInput("undo");
    return;
  }
  if (event.code === "KeyR") {
    event.preventDefault();
    puzzle3Component.applyInput("restart");
    return;
  }
  if (puzzle3ComponentFor(currentScene())) {
    puzzle3Component.handleKey(event);
  }
}

function handleComponentEmbedKeydown(event) {
  if (!puzzle3ComponentFor(currentScene())) {
    return;
  }
  if (event.code === "KeyZ") {
    event.preventDefault();
    puzzle3Component.applyInput("undo");
    return;
  }
  if (event.code === "KeyR") {
    event.preventDefault();
    puzzle3Component.applyInput("restart");
    return;
  }
  const input = inputForRawInput({ key: event.key, code: event.code }, puzzle3ComponentFor(currentScene()));
  if (!input) {
    return;
  }
  event.preventDefault();
  startHeldSceneInput(rawInputHoldId({ key: event.key, code: event.code }), input);
}

function handleStandaloneKeyup(event) {
  stopSceneRawInput({ key: event.key, code: event.code });
}

function handleComponentEmbedKeyup(event) {
  stopSceneRawInput({ key: event.key, code: event.code });
}

if (!effectiveComponentEmbedMode()) {
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
    } else {
      enqueueSceneRawInput(raw);
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
  return inputForRawInput({ key: event.key, code: event.code }, mountedPuzzle3Component);
}

function inputForRawInput(raw, component = mountedPuzzle3Component) {
  const keys = rawKeyCandidates(raw);
  const componentInputs = component?.inputs || {};
  for (const [input, bindings] of Object.entries(componentInputs)) {
    if ((bindings || []).some((binding) => keys.includes(normalizeRawKeyToken(binding)))) {
      return input;
    }
  }
  const defaultInputs = defaultPuzzle3Inputs();
  for (const [input, bindings] of Object.entries(defaultInputs)) {
    if (bindings.some((binding) => keys.includes(binding))) {
      return input;
    }
  }
  return snapshot.controls?.keys?.[raw.key]
    || snapshot.controls?.keys?.[raw.code]
    || snapshot.controls?.keys?.[String(raw.key || "").toLowerCase()]
    || null;
}

function rawKeyCandidates(raw) {
  return [...new Set([
    normalizeRawKeyToken(raw?.code),
    normalizeRawKeyToken(raw?.key),
  ].filter(Boolean))];
}

function normalizeRawKeyToken(value) {
  const token = String(value || "").trim();
  if (!token) {
    return "";
  }
  if (token.length === 1) {
    const ch = token[0];
    if (/[a-z]/i.test(ch)) {
      return `Key${ch.toUpperCase()}`;
    }
    if (/[0-9]/.test(ch)) {
      return `Digit${ch}`;
    }
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

function defaultPuzzle3Inputs() {
  return {
    left: ["KeyA", "ArrowLeft"],
    right: ["KeyD", "ArrowRight"],
    front: ["KeyW", "ArrowUp"],
    back: ["KeyS", "ArrowDown"],
  };
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
  const explicit = controls[event.key]
    || controls[event.code]
    || controls[String(event.key).toLowerCase()];
  if (explicit) {
    return explicit;
  }
  const keys = scene?.keys || {};
  const action = keys[event.key]
    || keys[event.code]
    || keys[String(event.key).toLowerCase()];
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
  const input = inputForRawInput(raw, puzzle3ComponentFor(currentScene()));
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
  const input = inputForRawInput(raw, puzzle3ComponentFor(currentScene()));
  if (!input) {
    return false;
  }
  return applySceneInput(input);
}

resizeCanvas();
loadSnapshot();

function cloneRuntimeSnapshot(source) {
  return {
    ...source,
    size: { ...source.size },
    camera: cloneCamera(source.camera),
    settings: { ...(source.settings || {}) },
    directions: cloneRuntimeRecord(source.directions || {}),
    directionSets: cloneRuntimeRecord(source.directionSets || {}),
    controls: {
      keys: { ...(source.controls?.keys || {}) },
    },
    inputs: JSON.parse(JSON.stringify(source.inputs || [])),
    rules: JSON.parse(JSON.stringify(source.rules || [])),
    winCondition: source.winCondition ? JSON.parse(JSON.stringify(source.winCondition)) : undefined,
    lifecycle: {
      onLevelStart: JSON.parse(JSON.stringify(source.lifecycle?.onLevelStart || [])),
      onLevelClear: [...(source.lifecycle?.onLevelClear || [])],
    },
    objects: cloneRuntimeObjects(source.objects || {}),
    sprites: cloneRuntimeSprites(source.sprites || {}),
    cells: cloneRuntimeCells(source.cells || []),
    levels: cloneRuntimeLevels(source.levels || []),
    levelBundles: cloneRuntimeLevelBundles(source.levelBundles || {}),
  };
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
        bitmap: [...(sprite.bitmap || [])],
      },
    ]),
  );
}

function cloneRuntimeCells(cells) {
  return cells.map((cell) => ({
    position: { ...cell.position },
    objects: (cell.objects || []).map((object) => ({ ...object })),
  }));
}

function cloneRuntimeLevels(levels) {
  return levels.map((level, index) => ({
    name: level.name || `level_${index + 1}`,
    label: level.label || level.name || `Level ${index + 1}`,
    size: { ...level.size },
    cells: cloneRuntimeCells(level.cells || []),
  }));
}

function cloneRuntimeLevelBundles(levelBundles) {
  return Object.fromEntries(
    Object.entries(levelBundles).map(([name, indexes]) => [
      name,
      Array.isArray(indexes) ? indexes.map((index) => Number(index)) : [],
    ]),
  );
}

function runtimeSnapshotLevel(source) {
  return {
    name: source.levelName || "level_1",
    label: source.levelLabel || source.levelName || "Level 1",
    size: { ...source.size },
    cells: cloneRuntimeCells(source.cells || []),
  };
}

function stateFromRuntimeCells(runtime, cells, size) {
  const layerCount = Number(runtime.base.layerCount || 1);
  const width = Number(size.width || 1);
  const depth = Number(size.depth || 1);
  const height = Number(size.height || 1);
  const slots = new Array(width * depth * height * layerCount).fill(0);
  for (const cell of cells || []) {
    const position = cell.position || {};
    const x = Number(position.x || 0);
    const y = Number(position.y || 0);
    const z = Number(position.z || 0);
    if (x < 0 || x >= width || y < 0 || y >= depth || z < 0 || z >= height) {
      continue;
    }
    for (const object of cell.objects || []) {
      const objectId = Number(object.id || 0);
      if (!objectId) {
        continue;
      }
      const layer = runtime.objectLayer(objectId);
      slots[runtimeSlotIndex(width, depth, layerCount, x, y, z, layer)] = objectId;
    }
  }
  return {
    kind: "puzzle3d",
    width,
    depth,
    height,
    layerCount,
    slots,
    levelFiredRules: [],
  };
}

function cellsFromRuntimeState(runtime, state) {
  const cells = [];
  const width = Number(state.width || 1);
  const depth = Number(state.depth || 1);
  const height = Number(state.height || 1);
  const layerCount = Number(state.layerCount || runtime.base.layerCount || 1);
  for (let z = 0; z < height; z += 1) {
    for (let y = 0; y < depth; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const objects = [];
        for (let layer = 0; layer < layerCount; layer += 1) {
          const objectId = Number(state.slots?.[runtimeSlotIndex(width, depth, layerCount, x, y, z, layer)] || 0);
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
  if (Array.isArray(source.levels) && source.levels.length > 0) {
    const levels = source.levels.map((level, index) => normalizeLevel(source, level, index));
    const levelIndex = clampIndex(source.levelIndex || 0, levels.length);
    const currentLevel = levels[levelIndex];
    return {
      ...source,
      levelIndex,
      levels,
      size: currentLevel.size,
      cells: currentLevel.cells,
      levelName: currentLevel.name,
      levelLabel: currentLevel.label,
    };
  }
  if (Array.isArray(source.cells)) {
    return source;
  }
  if (!Array.isArray(source.levelRows) || !source.legend || !source.objects) {
    return source;
  }
  const cells = [];
  for (let y = 0; y < source.levelRows.length; y += 1) {
    const row = source.levelRows[y];
    for (let x = 0; x < row.length; x += 1) {
      const objectNames = source.legend[row[x]] || [];
      const byZ = new Map();
      for (const name of objectNames) {
        const object = source.objects[name];
        if (!object) {
          continue;
        }
        const z = source.objectZ?.[name] ?? 0;
        const objects = byZ.get(z) || [];
        objects.push({ ...object });
        byZ.set(z, objects);
      }
      for (const [z, objects] of byZ) {
        cells.push({ position: { x, y, z }, objects });
      }
    }
  }
  return { ...source, cells };
}

function normalizeLevel(source, level, index) {
  if (Array.isArray(level.cells)) {
    return {
      name: level.name || `level_${index + 1}`,
      label: level.label || level.name || `Level ${index + 1}`,
      size: { ...(level.size || source.size) },
      cells: level.cells,
    };
  }
  if (Array.isArray(level.slices)) {
    const cells = cellsFromSlices(source, level.slices);
    return {
      name: level.name || `level_${index + 1}`,
      label: level.label || level.name || `Level ${index + 1}`,
      size: { ...(level.size || sizeFromSlices(level.slices)) },
      cells,
    };
  }
  if (Array.isArray(level.levelRows)) {
    return {
      name: level.name || `level_${index + 1}`,
      label: level.label || level.name || `Level ${index + 1}`,
      size: { ...(level.size || source.size) },
      cells: cellsFromRows(source, level.levelRows),
    };
  }
  return {
    name: level.name || `level_${index + 1}`,
    label: level.label || level.name || `Level ${index + 1}`,
    size: { ...(level.size || source.size) },
    cells: [],
  };
}

function cellsFromSlices(source, slices) {
  const cells = [];
  const size = sizeFromSlices(slices);
  for (let z = 0; z < slices.length; z += 1) {
    const rows = slices[z] || [];
    for (let y = 0; y < rows.length; y += 1) {
      const row = rows[y];
      for (let x = 0; x < row.length; x += 1) {
        const objects = objectsForLegendChar(source, row[x]);
        if (objects.length > 0) {
          cells.push({
            position: standardTextGridPosition(size, x, y, z),
            objects,
          });
        }
      }
    }
  }
  return cells;
}

function standardTextGridPosition(size, column, row, slice) {
  return {
    x: column,
    y: Math.max(0, Number(size.depth || 1) - 1 - row),
    z: Math.max(0, Number(size.height || 1) - 1 - slice),
  };
}

function standardSpriteGridPosition(size, column, row, slice) {
  return {
    x: column,
    y: Math.max(0, Number(size.depth || 1) - 1 - row),
    z: Math.max(0, Number(size.height || 1) - 1 - slice),
  };
}

function cellsFromRows(source, rows) {
  const cells = [];
  for (let y = 0; y < rows.length; y += 1) {
    const row = rows[y];
    for (let x = 0; x < row.length; x += 1) {
      const objectNames = source.legend[row[x]] || [];
      const byZ = new Map();
      for (const name of objectNames) {
        const object = source.objects[name];
        if (!object) {
          continue;
        }
        const z = source.objectZ?.[name] ?? 0;
        const objects = byZ.get(z) || [];
        objects.push({ ...object });
        byZ.set(z, objects);
      }
      for (const [z, objects] of byZ) {
        cells.push({ position: { x, y, z }, objects });
      }
    }
  }
  return cells;
}

function objectsForLegendChar(source, char) {
  return (source.legend?.[char] || [])
    .map((name) => source.objects?.[name])
    .filter(Boolean)
    .map((object) => ({ ...object }));
}

function sizeFromSlices(slices) {
  const depth = Math.max(1, ...slices.map((rows) => rows.length));
  const width = Math.max(1, ...slices.flatMap((rows) => rows.map((row) => row.length)));
  return { width, depth, height: Math.max(1, slices.length) };
}

function clampIndex(index, length) {
  return Math.max(0, Math.min(Math.max(0, length - 1), index));
}
