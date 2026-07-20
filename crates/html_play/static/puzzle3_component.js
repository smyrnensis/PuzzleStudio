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
  const size = source.size;
  if (!size || typeof size !== "object" || Array.isArray(size)) {
    throw new Error(`${label}.size is missing or invalid.`);
  }
  for (const axis of ["width", "depth", "height"]) {
    if (!Number.isInteger(Number(size[axis])) || Number(size[axis]) <= 0) {
      throw new Error(`${label}.size.${axis} must be a positive integer.`);
    }
  }
  if (!Array.isArray(source.cells)) {
    throw new Error(`${label}.cells is missing or invalid.`);
  }
  if (!Array.isArray(source.inputs)) {
    throw new Error(`${label}.inputs is missing or invalid.`);
  }
  if (!source.objects || typeof source.objects !== "object" || Array.isArray(source.objects)) {
    throw new Error(`${label}.objects is missing or invalid.`);
  }
  if (!source.visuals || typeof source.visuals !== "object" || Array.isArray(source.visuals)) {
    throw new Error(`${label}.visuals is missing or invalid.`);
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
const puzzle3Frame = ensurePuzzle3ComponentFrame();
const puzzle3RendererMode = resolvePuzzle3RendererMode(
  controllerOptions.renderer
    || controllerOptions.rendererMode,
);
const ctx = puzzle3RendererMode === "three" ? null : canvas.getContext("2d", { alpha: true });
const PUZZLE3_RENDERER_CONTRACT_VERSION = 1;
const PUZZLE3_COMPONENT_CAMERA_MIN_PITCH_DEGREES = -90;
const PUZZLE3_COMPONENT_CAMERA_MAX_PITCH_DEGREES = 90;

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
let snapshot = null;
let snapshotLoaded = false;
let initialCamera = null;
let currentSceneName = "";
const SCENE_DEFAULT_WIDTH = 16;
const SCENE_DEFAULT_HEIGHT = 12;
const visualVoxelTemplateCache = new WeakMap();
const renderGeometryCache = createRenderGeometryCache();
const pixelateBuffer = document.createElement("canvas");
let mountedPuzzle3Component = null;
let pendingResizeFrame = 0;
let pendingSceneLayoutRender = false;
let puzzle3ThreeRenderer = null;
let puzzle3ThreeViewPayload = null;
const viewListeners = new Set();
const stateListeners = new Set();
const puzzle3Component = createPuzzle3Component();

async function loadSnapshot() {
  const nextSnapshot = await loadInitialPuzzle3Snapshot();
  await loadSnapshotData(nextSnapshot);
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
  const previousCamera = snapshotLoaded ? cloneCamera(snapshot.render.camera) : null;
  snapshotLoaded = false;
  snapshot = normalizeSnapshot(validatePuzzle3ViewSnapshot(source));
  if (previousCamera && options.preserveCamera !== false) {
    snapshot.render.camera = previousCamera;
  }
  snapshotLoaded = true;
  currentSceneName = options.scene
    || controllerOptions.scene
    || initialSceneName(snapshot);
  initialCamera = cloneCamera(snapshot.render.camera);
  view.projectionFitKey = "";
  resetRenderGeometryCache();
  resetViewportMotion();
  renderScene();
  notifyPuzzle3StateChange();
}

function showPuzzle3LoadError(error) {
  showPuzzle3FatalError("fixture load failed", error);
}

function showPuzzle3RenderError(error) {
  showPuzzle3FatalError("render failed", error);
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
  return loadSnapshot()
    .catch((error) => {
      showPuzzle3LoadError(error);
      throw error;
    });
}

function resolvePuzzle3RendererMode(value) {
  return normalizePuzzle3RendererMode(value);
}

function normalizePuzzle3RendererMode(value) {
  const text = String(value || "").trim().toLowerCase();
  return text === "canvas" ? "canvas" : "three";
}

function applySceneComponentMetadata(component, sceneName) {
  mountedPuzzle3Component = component || null;
  canvas.dataset.component = component?.kind || "puzzle3";
  canvas.dataset.source = component?.source || "board";
  canvas.dataset.scene = sceneName;
  canvas.setAttribute("aria-label", `${snapshot.title || "Puzzle3"} ${canvas.dataset.source}`);
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

function puzzle3SceneDisplaySize() {
  return {
    width: SCENE_DEFAULT_WIDTH,
    height: SCENE_DEFAULT_HEIGHT,
  };
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
      if (!puzzle3Frame.contains(canvas)) {
        puzzle3Frame.append(canvas);
      }
      applySceneComponentMetadata(component, sceneName);
      updateCameraInteractionState();
      resizeCanvas();
      draw();
    },
    handleResize() {
      resizeCanvas();
      draw();
    },
    resetCamera() {
      resetCamera();
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
  observeHostIntent(controllerOptions.onCommand({
    kind: name,
    ...payload,
  }, {
    scene: currentSceneName,
    source: mountedPuzzle3Component?.source || "board",
  }));
  return true;
}

function observeHostIntent(result) {
  if (result && typeof result.then === "function") {
    result.catch((error) => showPuzzle3RenderError(error));
  }
}

function resetProjection(rect = canvasLayoutFrame()) {
  updateProjectionFit(rect);
}

function initialSceneName(source) {
  return source.currentScene
    || source.scenes?.[0]?.name
    || "default";
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
    if (shouldRenderLayout) {
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
  const size = snapshot.size;
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
  const size = snapshot.size;
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
  const size = source?.size || requireLoadedPuzzle3Snapshot().size;
  return clonePuzzle3PreviewView(source?.view || { zoom: 1 }, size);
}

function clonePuzzle3PreviewView(source, size = requireLoadedPuzzle3Snapshot().size) {
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
  resetViewportMotion();
}

function cameraLookEnabled() {
  return Boolean(snapshot.render.camera.interactiveLook);
}

function cameraZoomEnabled() {
  return Boolean(snapshot.render.camera.interactiveZoom);
}

function effectiveComponentEmbedMode() {
  return componentEmbedMode;
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
  const size = snapshot.size;
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
  if (hasRuntimeVisualAnimation()) {
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
    snapshot: clonePuzzle3ViewSnapshot(requireLoadedPuzzle3Snapshot("Puzzle3 renderer snapshot")),
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
    visualsSource: null,
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
  renderGeometryCache.visualsSource = null;
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
  const size = snapshot.size;
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
    || object.visual === viewport.focus
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
  const size = snapshot.size;
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
  const size = snapshot.size;
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
}

function puzzle3ViewPayload(width, height) {
  if (puzzle3RendererMode === "three" && puzzle3ThreeViewPayload) {
    return {
      ...puzzle3ThreeViewPayload,
      cellFootprints: projectedStageCellFootprints(snapshot.size),
    };
  }
  const size = snapshot.size;
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
  const state = clonePuzzle3ViewSnapshot(requireLoadedPuzzle3Snapshot("Puzzle3 state snapshot"));
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

function visualRenderSettings() {
  const raw = snapshot.render.visual;
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
  const size = snapshot.size;
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
  return Puzzle3VisualCore.stageFrameEdges(snapshot.size).map((edge) => {
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
  return (cell.objects || []).some((object) => object.visual && snapshot.visuals?.[object.visual]);
}

function cellVisibleVoxels(cell) {
  const stacks = new Map();
  for (const [objectIndex, object] of cell.objects.entries()) {
    const sourceKey = `${cellKey(cell.position)}:${objectIndex}`;
    const objectOrder = Puzzle3VisualCore.objectPriority(visualOrder(), object, objectIndex);
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
    && renderGeometryCache.visualsSource === snapshot.visuals
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
    || renderGeometryCache.visualsSource !== snapshot.visuals
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
  renderGeometryCache.visualsSource = snapshot.visuals;
  renderGeometryCache.settingsKey = settingsKey;
  renderGeometryCache.cellSignatures = nextSignatures;
  renderGeometryCache.allDirty = false;
  renderGeometryCache.revision += 1;
  if (renderContext) {
    renderContext.opaqueOcclusion = renderGeometryCache.occupied;
  }
}

function renderGeometrySettingsKey() {
  return JSON.stringify(visualRenderSettings());
}

function renderCellSignature(cell) {
  const position = cell?.position || {};
  const objects = (cell?.objects || []).map((object) => [
    object?.id ?? "",
    object?.visual ?? "",
    Puzzle3VisualCore.objectPriority(visualOrder(), object),
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
    const voxel = Puzzle3VisualCore.priorityDefinition(visualOrder(), group.order).merge
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

function visualOrder() {
  const order = snapshot.order;
  if (!order || !Array.isArray(order.direction_priority) || !Array.isArray(order.priorities)) {
    throw new Error("compiled visual order contract is missing");
  }
  return order;
}

function objectVoxelOrder(voxel) {
  const order = Number(voxel?.objectOrder);
  return Number.isFinite(order) ? order : 0;
}

function objectVoxels(position, object, sourceKey, objectOrder = 0) {
  if (!object.visual) {
    return [];
  }
  const template = visualVoxelTemplate(object.visual);
  if (!template) {
    return [];
  }
  return instantiateVisualVoxelTemplate(position, template, sourceKey, objectOrder);
}

function visualVoxelTemplate(visualName) {
  const visual = snapshot.visuals?.[visualName];
  if (!visual) {
    return null;
  }
  const animated = Array.isArray(visual.frames) && visual.frames.length > 1;
  if (!animated) {
    const cached = visualVoxelTemplateCache.get(visual);
    if (cached) {
      return cached;
    }
  }
  const template = buildVisualVoxelTemplate(visual);
  if (!animated) {
    visualVoxelTemplateCache.set(visual, template);
  }
  return template;
}

function buildVisualVoxelTemplate(visual) {
  const blocks = currentRuntimeVisualLayers(visual);
  const height = Math.max(1, blocks.length);
  const depth = Math.max(1, ...blocks.map((rows) => rows.length));
  const width = Math.max(1, ...blocks.flatMap((rows) => rows.map((row) => row.length)));
  const scale = 1 / Math.max(width, depth, height);
  const spatialAffine = Puzzle3VisualCore.evaluateSpatialVisualAffine(visual.spatialOps);
  const voxels = [];
  const palette = visual.palette || {};

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
        const sourceGrid = standardVisualGridPosition({ width, depth, height }, col, row, z);
        const localPosition = Puzzle3VisualCore.transformSpatialPoint({
          x: (sourceGrid.x + 0.5 - width / 2) * scale,
          y: (sourceGrid.y + 0.5 - depth / 2) * scale,
          z: (sourceGrid.z + 0.5 - height / 2) * scale,
        }, spatialAffine);
        const grid = Puzzle3VisualCore.spatialGridPoint(localPosition, scale);
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

function instantiateVisualVoxelTemplate(position, template, sourceKey = null, objectOrder = 0) {
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

function mergedVoxelFaces(voxels, occupied, ownerCell) {
  const visualSettings = visualRenderSettings();
  return Puzzle3VisualCore.mergeVoxelFaces(voxels, {
    faces: voxelFaces,
    isFaceVisible: (voxel, face) => !isVoxelFaceOccluded(voxel, face.offset, occupied),
    group: (voxel, face) => {
      const fill = visualSettings.shade ? shadeFill(voxel.fill, face.light) : voxel.fill;
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
  return visualOrder().direction_priority.map((direction) => {
    switch (direction) {
      case "right": return Number(position.x) || 0;
      case "left": return -(Number(position.x) || 0);
      case "front": return Number(position.y) || 0;
      case "back": return -(Number(position.y) || 0);
      case "up": return Number(position.z) || 0;
      case "down": return -(Number(position.z) || 0);
      default: throw new Error(`invalid 3D visual order direction: ${direction}`);
    }
  });
}

function currentRuntimeVisualLayers(visual, now = performance.now()) {
  const frames = Array.isArray(visual?.frames) ? visual.frames : [];
  if (!frames.length) {
    throw new Error("Puzzle3 runtime visual frames are missing.");
  }
  const frameDuration = Number(visual.frameDurationMs)
    || (Number(visual.durationMs) > 0 ? Number(visual.durationMs) / frames.length : 0);
  const index = frames.length > 1 && frameDuration > 0
    ? Math.floor(now / frameDuration) % frames.length
    : 0;
  const layers = frames[index]?.layers;
  if (!Array.isArray(layers) || !layers.length || layers.some((layer) => !Array.isArray(layer) || !layer.length)) {
    throw new Error("Puzzle3 runtime visual frame layers are missing or invalid.");
  }
  return layers;
}

function hasRuntimeVisualAnimation() {
  return Object.values(snapshot?.visuals || {}).some((visual) => (
    Array.isArray(visual?.frames)
    && visual.frames.length > 1
    && (Number(visual.frameDurationMs) > 0 || Number(visual.durationMs) > 0)
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

const controllerApi = {
  element: puzzle3Frame,
  canvas,
  ready: null,
  replaceSnapshot(nextSnapshot) {
    this.ready = Promise.resolve(this.ready).then(() => loadSnapshotData(nextSnapshot, {
      scene: nextSnapshot?.currentScene,
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
    puzzle3ThreeRenderer?.destroy();
    puzzle3ThreeRenderer = null;
  },
};

controllerApi.ready = loadPuzzle3ComponentSnapshot();

function clonePuzzle3ViewSnapshot(source) {
  return JSON.parse(JSON.stringify(validatePuzzle3ViewSnapshot(source)));
}

function normalizeSnapshot(source) {
  return clonePuzzle3ViewSnapshot(source);
}

function standardVisualGridPosition(size, column, row, slice) {
  return {
    x: column,
    y: Math.max(0, Number(size.depth || 1) - 1 - row),
    z: Math.max(0, Number(size.height || 1) - 1 - slice),
  };
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
