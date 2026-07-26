(() => {
function validatePuzzle3ViewSnapshot(source, label = "Puzzle3 snapshot") {
  if (!source || typeof source !== "object" || Array.isArray(source)) {
    throw new Error(`${label} is missing or invalid.`);
  }
  if (!source.render || typeof source.render !== "object" || Array.isArray(source.render)) {
    throw new Error(`${label}.render is missing or invalid.`);
  }
  if (!source.render.camera || !source.render.animation?.tween) {
    throw new Error(`${label}.render is missing camera or animation data.`);
  }
  if (!["perspective", "orthographic"].includes(source.render.camera.projection)) {
    throw new Error(`${label}.render.camera.projection must be perspective or orthographic.`);
  }
  const size = source.size;
  if (!size || typeof size !== "object" || Array.isArray(size)) {
    throw new Error(`${label}.size is missing or invalid.`);
  }
  for (const axis of ["width", "depth", "height"]) {
    if (!Number.isInteger(Number(size[axis])) || Number(size[axis]) <= 0) {
      throw new Error(`${label}.size.${axis} must be a positive integer.`);
    }
  }
  if (!source.renderScene || typeof source.renderScene !== "object" || Array.isArray(source.renderScene)) {
    throw new Error(`${label}.renderScene is missing or invalid.`);
  }
  if (source.animationEvents !== undefined && !Array.isArray(source.animationEvents)) {
    throw new Error(`${label}.animationEvents is invalid.`);
  }
  const tweenEvents = (source.animationEvents || [])
    .filter((event) => event?.kind === "move" && event?.name === "tween");
  if (tweenEvents.length > 0
      && (!Number.isInteger(Number(source.animationBatchId)) || Number(source.animationBatchId) <= 0)) {
    throw new Error(`${label}.animationBatchId must identify its Tween event batch.`);
  }
  return source;
}

function createPuzzle3ComponentController(options = {}) {
const controllerOptions = options && typeof options === "object" ? options : {};
const canvas = controllerOptions.canvas;
if (!(canvas instanceof HTMLCanvasElement)) {
  throw new Error("Puzzle3 component requires an explicit canvas element.");
}
const screenView = controllerOptions.screenView || controllerOptions.container || canvas.parentElement;
if (!(screenView instanceof HTMLElement)) {
  throw new Error("Puzzle3 component requires an explicit container element.");
}
const componentEmbedMode = Boolean(controllerOptions.componentEmbedMode);
if (typeof controllerOptions.prepareRenderScene !== "function"
    || typeof controllerOptions.resolveRenderMoment !== "function") {
  throw new Error("Puzzle3 component requires the Rust render-scene bridge.");
}
const puzzle3Frame = ensurePuzzle3ComponentFrame();
const PUZZLE3_COMPONENT_CAMERA_MIN_PITCH_DEGREES = -90;
const PUZZLE3_COMPONENT_CAMERA_MAX_PITCH_DEGREES = 90;
const interaction = {
  dragging: false,
  pointerId: null,
  lastPointerX: 0,
  lastPointerY: 0,
  viewportSnapNext: true,
};
let snapshot = null;
let snapshotLoaded = false;
let initialCamera = null;
let currentSceneName = "";
let mountedPuzzle3Component = null;
let pendingResizeFrame = 0;
let pendingSceneLayoutRender = false;
let puzzle3ThreeRenderer = null;
let puzzle3ThreeViewPayload = null;
let preparedRenderScenePromise = null;
let preparedRenderScene = null;
let rendererSnapshot = null;
let resolvedRenderFrame = null;
let renderClockEpochMs = performance.now();
let animationClockKey = "idle";
let animationStartedAtMs = 0;
let drawRunning = false;
let drawAgain = false;
let animationFrame = 0;
let renderGeneration = 0;
const viewListeners = new Set();
const stateListeners = new Set();
const puzzle3Component = createPuzzle3Component();

function ensurePuzzle3ComponentFrame() {
  const existing = canvas;
  const frame = existing.closest(".puzzle3-component")
    || existing.parentElement
    || document.createElement("div");
  frame.className = "puzzle3-component";
  if (!existing.id) {
    existing.id = "view";
  }
  if (existing.parentElement !== frame) {
    frame.append(existing);
  }
  return frame;
}

async function loadSnapshot() {
  await loadSnapshotData(await loadInitialPuzzle3Snapshot());
}

async function loadInitialPuzzle3Snapshot() {
  if (!controllerOptions.snapshot) {
    throw new Error("Puzzle3 component requires an explicit view snapshot.");
  }
  return controllerOptions.snapshot;
}

function requireLoadedPuzzle3Snapshot(label = "Puzzle3 snapshot") {
  if (!snapshotLoaded) {
    throw new Error(`${label} is not loaded.`);
  }
  return validatePuzzle3ViewSnapshot(snapshot, label);
}

async function loadSnapshotData(source, options = {}) {
  renderGeneration += 1;
  const previousCamera = snapshotLoaded ? cloneCamera(snapshot.render.camera) : null;
  snapshotLoaded = false;
  snapshot = clonePuzzle3ViewSnapshot(validatePuzzle3ViewSnapshot(source));
  preparedRenderScenePromise = Promise.resolve(
    controllerOptions.prepareRenderScene(snapshot.renderScene),
  );
  preparedRenderScene = null;
  rendererSnapshot = createPuzzle3RendererSnapshot(snapshot);
  resolvedRenderFrame = null;
  renderClockEpochMs = performance.now();
  animationClockKey = "idle";
  animationStartedAtMs = 0;
  if (previousCamera && options.preserveCamera !== false) {
    snapshot.render.camera = previousCamera;
  }
  snapshotLoaded = true;
  currentSceneName = options.scene
    || controllerOptions.scene
    || initialSceneName(snapshot);
  initialCamera = cloneCamera(snapshot.render.camera);
  interaction.viewportSnapNext = true;
  renderScene();
  notifyPuzzle3StateChange();
}

function showPuzzle3FatalError(label, error) {
  const message = String(error?.message || error || "unknown error");
  controllerOptions.onError?.({ label, error, message });
  console.error(error);
  const errorView = document.createElement("div");
  errorView.className = "puzzle3-load-error";
  errorView.setAttribute("role", "alert");
  errorView.textContent = `Puzzle3 ${label}: ${message}`;
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

function loadPuzzle3ComponentSnapshot() {
  return loadSnapshot().catch((error) => {
    showPuzzle3LoadError(error);
    throw error;
  });
}

function showPuzzle3LoadError(error) {
  showPuzzle3FatalError("fixture load failed", error);
}

function applySceneComponentMetadata(component, sceneName) {
  mountedPuzzle3Component = component || null;
  canvas.dataset.component = component?.kind || "puzzle3";
  canvas.dataset.source = component?.source || "board";
  canvas.dataset.scene = sceneName;
  canvas.setAttribute("aria-label", `Puzzle3 ${canvas.dataset.source}`);
}

function renderScene() {
  const sceneName = currentSceneName || initialSceneName(snapshot) || "default";
  const component = controllerOptions.component;
  if (!component || typeof component !== "object" || component.kind !== "puzzle3") {
    throw new Error("Puzzle3 component metadata is missing or invalid.");
  }
  screenView.dataset.scene = sceneName;
  puzzle3Component.mount(component, sceneName);
}

function createPuzzle3Component() {
  return {
    mount(component, sceneName) {
      const embed = componentEmbedMode;
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
      if (!puzzle3Frame.contains(canvas)) {
        puzzle3Frame.append(canvas);
      }
      applySceneComponentMetadata(component, sceneName);
      updateCameraInteractionState();
      draw();
    },
    handleResize() {
      draw();
    },
    resetCamera() {
      snapshot.render.camera = cloneCamera(initialCamera);
      interaction.viewportSnapNext = true;
      draw();
      return true;
    },
  };
}

function emitPuzzle3CommandIntent(command, payload = {}) {
  const name = String(command || "");
  if (!name) {
    return false;
  }
  if (typeof controllerOptions.onCommand !== "function") {
    throw new Error("Puzzle3 command requires a session host.");
  }
  const result = controllerOptions.onCommand({ kind: name, ...payload }, {
    scene: currentSceneName,
    source: mountedPuzzle3Component?.source || "board",
  });
  if (result && typeof result.then === "function") {
    result.catch((error) => showPuzzle3FatalError("render failed", error));
  }
  return true;
}

function initialSceneName(source) {
  return source.component
    || source.surface?.focus
    || source.scenes?.[0]?.name
    || "default";
}

function cloneCamera(camera) {
  return {
    projection: camera.projection,
    yawDegrees: Number(camera.yawDegrees),
    pitchDegrees: Number(camera.pitchDegrees),
    rollDegrees: Number(camera.rollDegrees),
    zoom: Number(camera.zoom),
  };
}

function cameraLookEnabled() {
  return Boolean(snapshot.render.camera.interactiveLook);
}

function cameraZoomEnabled() {
  return Boolean(snapshot.render.camera.interactiveZoom);
}

function updateCameraInteractionState() {
  canvas.classList.toggle("has-interactive-look", cameraLookEnabled());
}

function rotateCamera(deltaX, deltaY) {
  const camera = snapshot.render.camera;
  camera.yawDegrees = normalizeDegrees(camera.yawDegrees + deltaX * 0.35);
  camera.pitchDegrees = clamp(
    camera.pitchDegrees - deltaY * 0.25,
    PUZZLE3_COMPONENT_CAMERA_MIN_PITCH_DEGREES,
    PUZZLE3_COMPONENT_CAMERA_MAX_PITCH_DEGREES,
  );
  interaction.viewportSnapNext = true;
}

function zoomCamera(deltaY) {
  const camera = snapshot.render.camera;
  camera.zoom = clamp(Number(camera.zoom) * Math.exp(-deltaY * 0.001), 0.1, 8);
}

function normalizeDegrees(value) {
  return ((value % 360) + 360) % 360;
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function canvasLayoutFrame() {
  const rect = canvas.getBoundingClientRect();
  return {
    width: Math.max(1, Number(canvas.clientWidth) || Number(rect.width) || 1),
    height: Math.max(1, Number(canvas.clientHeight) || Number(rect.height) || 1),
  };
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
    if (shouldRenderLayout) {
      renderScene();
    } else {
      puzzle3Component.handleResize();
    }
  });
}

function draw() {
  if (drawRunning) {
    drawAgain = true;
    return;
  }
  void drawResolvedFrame();
}

async function drawResolvedFrame() {
  drawRunning = true;
  const generation = renderGeneration;
  const { width, height } = canvasLayoutFrame();
  let result;
  try {
    const loadedSnapshot = requireLoadedPuzzle3Snapshot("Puzzle3 renderer snapshot");
    const now = performance.now();
    const animations = loadedSnapshot.animationEvents || [];
    const nextAnimationKey = animations.length
      ? `batch:${Number(loadedSnapshot.animationBatchId)}`
      : "idle";
    if (nextAnimationKey !== animationClockKey) {
      animationClockKey = nextAnimationKey;
      animationStartedAtMs = animations.length ? now : 0;
    }
    const renderScene = await preparedRenderScenePromise;
    preparedRenderScene = renderScene;
    if (generation !== renderGeneration) {
      finishStaleDraw();
      return;
    }
    if (!resolvedRenderFrame || resolvedRenderFrame.continueAnimation) {
      resolvedRenderFrame = await controllerOptions.resolveRenderMoment(renderScene, {
        clipElapsedMs: Math.max(0, Math.floor(now - renderClockEpochMs)),
        animationElapsedMs: animations.length
          ? Math.max(0, Math.floor(now - animationStartedAtMs))
          : 0,
        animations,
      });
    }
    if (generation !== renderGeneration) {
      finishStaleDraw();
      return;
    }
    const renderer = ensurePuzzle3ThreeRenderer();
    result = renderer.render(
      rendererSnapshot,
      renderScene,
      resolvedRenderFrame,
      {
        width,
        height,
        editorView: clonePuzzle3PreviewView(snapshot.view || { zoom: 1 }, snapshot.size),
        viewportSnapNext: interaction.viewportSnapNext,
        background: "transparent",
      },
    );
  } catch (error) {
    drawRunning = false;
    showPuzzle3FatalError("render failed", error);
    return;
  }
  puzzle3ThreeViewPayload = result?.view || null;
  if (result?.rendered) {
    interaction.viewportSnapNext = false;
  }
  if (puzzle3ThreeViewPayload) {
    notifyPuzzle3View(puzzle3ThreeViewPayload);
  }
  drawRunning = false;
  if (drawAgain) {
    drawAgain = false;
    draw();
    return;
  }
  if ((result?.animating || result?.continueAnimation) && !animationFrame) {
    animationFrame = requestAnimationFrame(() => {
      animationFrame = 0;
      draw();
    });
  }
}

function finishStaleDraw() {
  drawRunning = false;
  if (drawAgain) {
    drawAgain = false;
    draw();
  }
}

function createPuzzle3RendererSnapshot(source) {
  return {
    component: source.component,
    levelIndex: source.levelIndex,
    levelCount: source.levelCount,
    levelName: source.levelName,
    size: source.size,
    completed: source.completed,
    hasNextLevel: source.hasNextLevel,
    hasPreviousLevel: source.hasPreviousLevel,
    render: source.render,
    view: source.view,
  };
}

function ensurePuzzle3ThreeRenderer() {
  if (puzzle3ThreeRenderer) {
    return puzzle3ThreeRenderer;
  }
  if (!window.Puzzle3ThreeRenderer) {
    throw new Error("Puzzle3 Three.js renderer is unavailable.");
  }
  puzzle3ThreeRenderer = window.Puzzle3ThreeRenderer.create(canvas, { onReady: draw });
  return puzzle3ThreeRenderer;
}

function clonePuzzle3PreviewView(source, size) {
  const target = source?.target || source?.origin || {
    x: (Number(size?.width) - 1) / 2,
    y: (Number(size?.depth) - 1) / 2,
    z: (Number(size?.height) - 1) / 2,
  };
  return {
    zoom: Puzzle3VisualCore.normalizeZoom(source?.zoom),
    target: {
      x: Number(target.x) || 0,
      y: Number(target.y) || 0,
      z: Number(target.z) || 0,
    },
  };
}

function notifyPuzzle3View(viewPayload) {
  for (const listener of viewListeners) {
    listener(viewPayload);
  }
}

function notifyPuzzle3StateChange() {
  const state = clonePuzzle3ViewSnapshot(requireLoadedPuzzle3Snapshot("Puzzle3 state snapshot"));
  for (const listener of stateListeners) {
    listener(state);
  }
}

window.Puzzle3DInspect = () => ({
  scene: currentSceneName,
  source: canvas.dataset.source || "",
  frame: canvasLayoutFrame(),
  canvas: {
    clientWidth: canvas.clientWidth,
    clientHeight: canvas.clientHeight,
    width: canvas.width,
    height: canvas.height,
  },
  view: puzzle3ThreeViewPayload,
  cellCount: preparedRenderScene?.cells?.length || 0,
});

canvas.addEventListener("pointerdown", (event) => {
  if (!cameraLookEnabled() || event.button !== 0) {
    return;
  }
  interaction.dragging = true;
  interaction.pointerId = event.pointerId;
  interaction.lastPointerX = event.clientX;
  interaction.lastPointerY = event.clientY;
  canvas.setPointerCapture(event.pointerId);
  canvas.classList.add("is-dragging");
});

canvas.addEventListener("pointermove", (event) => {
  if (!cameraLookEnabled() || !interaction.dragging || event.pointerId !== interaction.pointerId) {
    return;
  }
  const deltaX = event.clientX - interaction.lastPointerX;
  const deltaY = event.clientY - interaction.lastPointerY;
  interaction.lastPointerX = event.clientX;
  interaction.lastPointerY = event.clientY;
  rotateCamera(deltaX, deltaY);
  draw();
});

function endCameraDrag(event) {
  if (!interaction.dragging || event.pointerId !== interaction.pointerId) {
    return;
  }
  interaction.dragging = false;
  interaction.pointerId = null;
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

const controllerApi = {
  element: puzzle3Frame,
  canvas,
  ready: null,
  replaceSnapshot(nextSnapshot) {
    this.ready = Promise.resolve(this.ready).then(() => loadSnapshotData(nextSnapshot, {
      scene: nextSnapshot?.component || controllerOptions.scene,
      preserveCamera: true,
    }));
    return this.ready;
  },
  command(command, payload = {}) {
    const name = String(command || "");
    if (name === "reset_camera") {
      return puzzle3Component.resetCamera();
    }
    return emitPuzzle3CommandIntent(name, payload);
  },
  snapshot() {
    return clonePuzzle3ViewSnapshot(requireLoadedPuzzle3Snapshot());
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
    renderGeneration += 1;
    resolvedRenderFrame = null;
    viewListeners.clear();
    stateListeners.clear();
    puzzle3ThreeRenderer?.destroy();
    puzzle3ThreeRenderer = null;
    if (animationFrame) {
      cancelAnimationFrame(animationFrame);
      animationFrame = 0;
    }
  },
};

controllerApi.ready = loadPuzzle3ComponentSnapshot();

function clonePuzzle3ViewSnapshot(source) {
  return JSON.parse(JSON.stringify(validatePuzzle3ViewSnapshot(source)));
}

return controllerApi;
}

window.Puzzle3Component = {
  validateSnapshot(source) {
    return validatePuzzle3ViewSnapshot(source);
  },
  attach(canvas, options = {}) {
    return createPuzzle3ComponentController({ ...options, canvas });
  },
};
})();
