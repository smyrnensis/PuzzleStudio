(() => {
const PUZZLE3_THREE_RENDERER_CONTRACT = {
  version: 1,
  input: ["snapshot", "view"],
};

class Puzzle3ThreeRenderer {
  constructor(canvas, options = {}) {
    this.canvas = canvas;
    this.options = options;
    this.renderer = null;
    this.scene = null;
    this.camera = null;
    this.loading = null;
    this.failed = null;
    this.viewTarget = null;
    this.viewDistance = null;
    this.viewPayload = null;
    this.animationFrame = 0;
    this.animationKey = "";
    this.animationStartedAt = 0;
    this.activeSnapshot = null;
    this.frame = null;
    this.faceMaterialCache = new Map();
  }

  render(snapshot, view = {}) {
    if (!snapshot) {
      return { rendered: false, reason: "missing-snapshot" };
    }
    if (!window.THREE) {
      this.ensureThreeLoaded();
      return { rendered: false, reason: this.failed ? "three-load-failed" : "loading-three" };
    }

    const THREE = window.THREE;
    this.ensureRenderer(THREE);
    if (snapshot !== this.activeSnapshot) {
      if (this.animationFrame) {
        cancelAnimationFrame(this.animationFrame);
        this.animationFrame = 0;
      }
      this.activeSnapshot = snapshot;
      this.frame = null;
    }
    const animation = threeAnimationState(snapshot);
    if (animation.key !== this.animationKey) {
      this.animationKey = animation.key;
      this.animationStartedAt = animation.events.length ? performance.now() : 0;
    }
    animation.progress = animation.events.length && this.animationStartedAt
      ? Math.min(1, Math.max(0, (performance.now() - this.animationStartedAt) / animation.durationMs))
      : 1;
    const visualAnimating = hasLoopingVisualAnimation(snapshot);
    const builtFrame = visualAnimating || !this.frame;
    const frame = !builtFrame
      ? updatePuzzleStudioThreeFrame(this.frame, snapshot, view, animation.progress)
      : buildPuzzleStudioThreeFrame(snapshot, { ...view, animationProgress: animation.progress });
    if (!visualAnimating && builtFrame) {
      this.frame = frame;
    }
    const scene = new THREE.Scene();
    scene.background = threeBackground(THREE, view.background);
    frame.rendererViewTarget = this.viewTarget;
    frame.rendererViewDistance = this.viewDistance;
    const camera = buildCamera(THREE, frame, this.canvas);
    applyProjectedRenderCulling(THREE, frame, camera, this.canvas);
    const visible = frameVisibleVoxels(frame);
    const shadow = shadowSettings(frame);
    this.configureShadowMap(THREE, shadow.enabled);
    addLights(THREE, scene, frame, visible.voxels, shadow);
    addGrid(THREE, scene, frame);
    addMeshes(THREE, scene, visible, shadow, this.faceMaterialCache);
    addShadowCatcher(THREE, scene, frame, shadow);
    this.renderer.setSize(this.canvas.clientWidth || this.canvas.width || 1, this.canvas.clientHeight || this.canvas.height || 1, false);
    this.renderer.setClearColor(0x000000, scene.background ? 1 : 0);
    disposeScene(this.scene, this.faceMaterialCache);
    this.renderer.render(scene, camera);
    this.scene = scene;
    this.camera = camera;
    this.viewPayload = threeViewPayload(frame, camera, this.canvas);
    this.updateViewportMotion(frame);
    this.scheduleAnimationFrame(
      snapshot,
      view,
      animation.progress,
      visualAnimating,
      frame.viewport?.follow === "smooth" && frame.viewportAnimating === true,
    );
    return {
      rendered: true,
      objectCount: frame.objectCount,
      animating: animation.progress < 1 || visualAnimating || (frame.viewport?.follow === "smooth" && frame.viewportAnimating === true),
      view: this.viewPayload,
    };
  }

  scheduleAnimationFrame(snapshot, view, progress, visualAnimating, viewportAnimating) {
    if ((progress >= 1 && !visualAnimating && !viewportAnimating) || this.animationFrame) {
      return;
    }
    this.animationFrame = requestAnimationFrame(() => {
      this.animationFrame = 0;
      this.render(snapshot, view);
    });
  }

  destroy() {
    if (this.animationFrame) {
      cancelAnimationFrame(this.animationFrame);
      this.animationFrame = 0;
    }
    disposeScene(this.scene);
    this.faceMaterialCache.clear();
    this.renderer?.dispose?.();
    this.scene = null;
    this.renderer = null;
    this.frame = null;
    this.activeSnapshot = null;
  }

  updateViewportMotion(frame) {
    const next = frame.cameraView;
    if (!next) {
      this.viewTarget = null;
      this.viewDistance = null;
      return;
    }
    this.viewTarget = { ...next.target };
    this.viewDistance = next.distance;
  }

  ensureRenderer(THREE) {
    if (this.renderer) {
      return;
    }
    this.renderer = new THREE.WebGLRenderer({
      canvas: this.canvas,
      antialias: false,
      alpha: true,
    });
    this.renderer.setPixelRatio(window.devicePixelRatio || 1);
  }

  configureShadowMap(THREE, enabled) {
    if (enabled && THREE.PCFSoftShadowMap === undefined) {
      throw new Error("Three.js PCFSoftShadowMap is required for Puzzle3 shadows.");
    }
    this.renderer.shadowMap.enabled = enabled;
    this.renderer.shadowMap.autoUpdate = enabled;
    if (enabled) {
      this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    }
  }

  ensureThreeLoaded() {
    if (this.loading || this.failed) {
      return;
    }
    this.loading = loadThree().then(() => {
      this.loading = null;
      this.options.onReady?.();
    }).catch((error) => {
      this.loading = null;
      this.failed = error;
      console.error("Failed to load Three.js renderer", error);
    });
  }
}

function threeBackground(THREE, value) {
  const text = String(value || "").trim().toLowerCase();
  if (!text || text === "transparent" || text === "none") {
    return null;
  }
  return new THREE.Color(value || "#111318");
}

function disposeScene(scene, preservedMaterials = null) {
  if (!scene) {
    return;
  }
  const disposedGeometries = new Set();
  const disposedMaterials = new Set();
  scene.traverse((object) => {
    if (object.geometry && !disposedGeometries.has(object.geometry)) {
      disposedGeometries.add(object.geometry);
      object.geometry.dispose?.();
    }
    const materials = Array.isArray(object.material) ? object.material : [object.material];
    for (const material of materials) {
      if (material
          && !preservedMaterials?.has(material)
          && !disposedMaterials.has(material)) {
        disposedMaterials.add(material);
        material.dispose?.();
      }
    }
  });
}

async function loadThree() {
  if (window.THREE) {
    return window.THREE;
  }
  const source = window.Puzzle3ThreeModuleSource;
  if (!source) {
    throw new Error("Puzzle3ThreeModuleSource is unavailable.");
  }
  const blob = new Blob([source], { type: "text/javascript" });
  const url = URL.createObjectURL(blob);
  try {
    const module = await import(url);
    window.THREE = module;
    return module;
  } finally {
    URL.revokeObjectURL(url);
  }
}

function buildPuzzleStudioThreeFrame(snapshot, view = {}) {
  const size = normalizeSize(snapshot.size);
  const objectCatalog = buildObjectCatalog(snapshot);
  const visuals = snapshot.visuals || {};
  const visualCache = new Map();
  const animationEvents = Array.isArray(snapshot.animationEvents) ? snapshot.animationEvents : [];
  const cells = (snapshot.cells || []).map((cell) => ({
    position: normalizePosition(cell.position),
    objects: (cell.objects || [])
      .map((object) => resolveObject(object, objectCatalog, visuals, visualCache))
      .filter(Boolean),
  }));
  const frame = {
    size,
    cells,
    objectCatalog,
    visuals,
    visualCache,
    objectCount: cells.reduce((count, cell) => count + cell.objects.length, 0),
    camera: snapshot.render.camera,
    editorView: view.editorView || snapshot.view || {},
    settings: snapshot.render,
    order: snapshot.order,
    animationEvents,
    animationEventIndex: indexTweenAnimationEvents(animationEvents),
    animationProgress: Number.isFinite(Number(view.animationProgress)) ? Number(view.animationProgress) : 1,
    viewport: normalizeViewport(snapshot.render.viewport),
    viewportSnapNext: view.viewportSnapNext === true,
  };
  frame.focusCell = frame.viewport ? viewportFocusCell(frame) : null;
  frame.viewportRanges = viewportRanges(frame);
  frame.renderCells = cells;
  return frame;
}

function updatePuzzleStudioThreeFrame(frame, snapshot, view, animationProgress) {
  frame.camera = snapshot.render.camera;
  frame.editorView = view.editorView || snapshot.view || {};
  frame.settings = snapshot.render;
  const animationEvents = Array.isArray(snapshot.animationEvents) ? snapshot.animationEvents : [];
  if (animationEvents !== frame.animationEvents) {
    frame.animationEvents = animationEvents;
    frame.animationEventIndex = indexTweenAnimationEvents(animationEvents);
  }
  frame.animationProgress = animationProgress;
  frame.viewport = normalizeViewport(snapshot.render.viewport);
  frame.viewportSnapNext = view.viewportSnapNext === true;
  frame.focusCell = frame.viewport ? viewportFocusCell(frame) : null;
  frame.viewportRanges = viewportRanges(frame);
  frame.renderCells = frame.cells;
  return frame;
}

function threeAnimationState(snapshot) {
  const events = tweenAnimationEvents(snapshot);
  const tween = snapshot.render.animation.tween;
  if (events.length && tween.enabled !== true) {
    throw new Error("Puzzle3 received Tween animation events while tween is disabled.");
  }
  const durationMs = Number(tween.intervalMs);
  if (!Number.isFinite(durationMs) || durationMs <= 0) {
    throw new Error("Puzzle3 Tween duration must be a positive number of milliseconds.");
  }
  const batchId = Number(snapshot.animationBatchId);
  if (events.length && (!Number.isInteger(batchId) || batchId <= 0)) {
    throw new Error("Puzzle3 Tween animation events require a positive animationBatchId.");
  }
  return {
    events,
    durationMs,
    key: events.length ? `batch:${batchId}` : "idle",
    progress: 1,
  };
}

function tweenAnimationEvents(snapshot) {
  return (Array.isArray(snapshot?.animationEvents) ? snapshot.animationEvents : [])
    .filter((event) => event?.kind === "move" && event?.name === "tween");
}

function normalizeSize(size) {
  return {
    width: Math.max(1, Number(size?.width) || 1),
    depth: Math.max(1, Number(size?.depth) || 1),
    height: Math.max(1, Number(size?.height) || 1),
  };
}

function normalizePosition(position) {
  return {
    x: Number(position?.x) || 0,
    y: Number(position?.y) || 0,
    z: Number(position?.z) || 0,
  };
}

function normalizeViewport(raw) {
  if (!raw || raw === true || raw === false) {
    return null;
  }
  const framing = raw.framingBox || raw.framing || {};
  const width = Number(framing.width);
  const depth = Number(framing.depth);
  if (!Number.isFinite(width) || width <= 0 || !Number.isFinite(depth) || depth <= 0) {
    return null;
  }
  const mode = String(raw.mode || "centered");
  return {
    mode,
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

function buildObjectCatalog(snapshot) {
  const catalog = new Map();
  for (const object of Object.values(snapshot.objects || {})) {
    const id = Number(object?.id);
    if (Number.isFinite(id)) {
      catalog.set(id, { ...object, id });
    }
  }
  for (const cell of snapshot.cells || []) {
    for (const object of cell.objects || []) {
      const id = Number(object?.id);
      if (Number.isFinite(id) && !catalog.has(id)) {
        catalog.set(id, { ...object, id });
      }
    }
  }
  return catalog;
}

function resolveObject(object, catalog, visuals, visualCache = null) {
  const id = Number(object?.id);
  const base = Number.isFinite(id) ? catalog.get(id) || {} : {};
  const merged = { ...base, ...object, id: Number.isFinite(id) ? id : base.id };
  const visualName = merged.visual || merged.name;
  const name = merged.name || visualName || (Number.isFinite(Number(merged.id)) ? `object_${merged.id}` : "");
  let resolvedVisual = visualCache?.get(visualName);
  if (!resolvedVisual && !visualCache?.has(visualName)) {
    resolvedVisual = visual(visuals[visualName]);
    visualCache?.set(visualName, resolvedVisual);
  }
  if ((!Number.isFinite(Number(merged.id)) && !name && !visualName) || !resolvedVisual) {
    return null;
  }
  return {
    id: Number.isFinite(Number(merged.id)) ? Number(merged.id) : name,
    name,
    visual: visualName,
    layer: Number(merged.layer ?? base.layer ?? 0) || 0,
    visual: resolvedVisual,
  };
}

function visual(visual) {
  if (!visual) {
    return null;
  }
  const spatialAffine = Puzzle3VisualCore.evaluateSpatialVisualAffine(visual.spatialOps);
  const palette = visual.palette || {};
  const blocks = currentVisualLayers(visual);
  const height = Math.max(1, blocks.length);
  const depth = Math.max(1, Math.max(...blocks.map((rows) => rows.length), 1));
  const width = Math.max(1, Math.max(...blocks.flat().map((row) => String(row).length), 1));
  const voxels = [];
  for (let slice = 0; slice < blocks.length; slice += 1) {
    const rows = blocks[slice] || [];
    for (let rowIndex = 0; rowIndex < rows.length; rowIndex += 1) {
      const row = String(rows[rowIndex] || "");
      for (let x = 0; x < row.length; x += 1) {
        const token = row[x];
        const fill = palette[token];
        const color = parseColor(fill);
        if (!fill || fill === "transparent" || token === "." || token === " " || color?.a <= 0) {
          continue;
        }
        voxels.push({
          x,
          y: Math.max(0, depth - 1 - rowIndex),
          z: Math.max(0, height - 1 - slice),
          fill,
          color,
          opaque: !color || color.a >= 0.999,
        });
      }
    }
  }
  if (!voxels.length) {
    return null;
  }
  return {
    kind: "voxels",
    size: { width, depth, height },
    spatialOps: visual.spatialOps,
    spatialAffine,
    voxels,
  };
}

function currentVisualLayers(visual, now = performance.now()) {
  const frames = Array.isArray(visual?.frames) ? visual.frames : [];
  if (!frames.length) {
    throw new Error("Puzzle3 visual frames are missing.");
  }
  const frameDuration = Number(visual.frameDurationMs)
    || (Number(visual.durationMs) > 0 ? Number(visual.durationMs) / frames.length : 0);
  const index = frames.length > 1 && frameDuration > 0
    ? Math.floor(now / frameDuration) % frames.length
    : 0;
  const layers = frames[index]?.layers;
  if (!Array.isArray(layers) || !layers.length || layers.some((layer) => !Array.isArray(layer) || !layer.length)) {
    throw new Error("Puzzle3 visual frame layers are missing or invalid.");
  }
  return layers;
}

function hasLoopingVisualAnimation(snapshot) {
  return Object.values(snapshot?.visuals || {}).some((visual) => (
    Array.isArray(visual?.frames)
    && visual.frames.length > 1
    && (Number(visual.frameDurationMs) > 0 || Number(visual.durationMs) > 0)
  ));
}

function shadowSettings(frame) {
  const raw = frame.settings?.shadow;
  if (raw === undefined) {
    return { enabled: false };
  }
  if (typeof raw !== "boolean") {
    throw new Error("Puzzle3 render setting `shadow` must be boolean.");
  }
  return { enabled: raw };
}

function addLights(THREE, scene, frame, voxels, shadow) {
  const bounds = shadowFrameBounds(frame, voxels);
  const center = boundsCenter(bounds);
  const size = Math.max(bounds.maxX - bounds.minX, bounds.maxY - bounds.minY, bounds.maxZ - bounds.minZ, 1);
  scene.add(new THREE.AmbientLight("#ffffff", 1.35));
  const key = new THREE.DirectionalLight("#ffffff", 0.72);
  key.position.set(center.x + size * 1.2, center.y + size * 2.2, center.z + size * 0.9);
  key.target.position.set(center.x, center.y, center.z);
  key.castShadow = shadow.enabled;
  if (shadow.enabled) {
    configureDirectionalShadow(key, bounds, voxels);
  }
  scene.add(key, key.target);
  const fill = new THREE.DirectionalLight("#dbeafe", 0.42);
  fill.position.set(center.x - size * 1.5, center.y + size, center.z - size * 1.1);
  fill.target.position.set(center.x, center.y, center.z);
  fill.castShadow = false;
  scene.add(fill.target);
  scene.add(fill);
}

function shadowFrameBounds(frame, voxels) {
  if (!voxels.length) {
    return {
      minX: -frame.size.width / 2,
      maxX: frame.size.width / 2,
      minY: -frame.size.height / 2,
      maxY: frame.size.height / 2,
      minZ: -frame.size.depth / 2,
      maxZ: frame.size.depth / 2,
    };
  }
  const bounds = {
    minX: Infinity,
    maxX: -Infinity,
    minY: Infinity,
    maxY: -Infinity,
    minZ: Infinity,
    maxZ: -Infinity,
  };
  for (const voxel of voxels) {
    bounds.minX = Math.min(bounds.minX, voxel.bounds.x0);
    bounds.maxX = Math.max(bounds.maxX, voxel.bounds.x1);
    bounds.minY = Math.min(bounds.minY, voxel.bounds.y0);
    bounds.maxY = Math.max(bounds.maxY, voxel.bounds.y1);
    bounds.minZ = Math.min(bounds.minZ, voxel.bounds.z0);
    bounds.maxZ = Math.max(bounds.maxZ, voxel.bounds.z1);
  }
  bounds.minY = Math.min(bounds.minY, -frame.size.height / 2 - 0.02);
  return bounds;
}

function boundsCenter(bounds) {
  return {
    x: (bounds.minX + bounds.maxX) / 2,
    y: (bounds.minY + bounds.maxY) / 2,
    z: (bounds.minZ + bounds.maxZ) / 2,
  };
}

function configureDirectionalShadow(light, bounds, voxels) {
  const width = Math.max(1, bounds.maxX - bounds.minX);
  const height = Math.max(1, bounds.maxY - bounds.minY);
  const depth = Math.max(1, bounds.maxZ - bounds.minZ);
  const radius = Math.hypot(width, height, depth) / 2 + 0.75;
  light.shadow.mapSize.set(1024, 1024);
  light.shadow.camera.left = -radius;
  light.shadow.camera.right = radius;
  light.shadow.camera.top = radius;
  light.shadow.camera.bottom = -radius;
  light.shadow.camera.near = 0.1;
  light.shadow.camera.far = radius * 8;
  light.shadow.bias = -0.0003;
  light.shadow.normalBias = Math.min(0.04, minimumVoxelScale(voxels) * 0.08);
  light.shadow.camera.updateProjectionMatrix();
}

function minimumVoxelScale(voxels) {
  const scales = voxels
    .map((voxel) => Number(voxel.scale))
    .filter((scale) => Number.isFinite(scale) && scale > 0);
  return scales.length ? Math.min(...scales) : 1;
}

function addGrid(THREE, scene, frame) {
  const grid = frame.settings?.grid || {};
  const visibility = Number(grid.visibility ?? (grid.occupied_cells ? 1 : 0)) || 0;
  if (visibility <= 0) {
    return;
  }
  const width = Math.max(1, frame.size.width);
  const depth = Math.max(1, frame.size.depth);
  const x0 = -width / 2;
  const x1 = width / 2;
  const z0 = -depth / 2;
  const z1 = depth / 2;
  const y = -((frame.size.height - 1) / 2) - 0.515;
  const points = [];
  for (let x = 0; x <= width; x += 1) {
    const rx = x - width / 2;
    points.push(rx, y, z0, rx, y, z1);
  }
  for (let z = 0; z <= depth; z += 1) {
    const rz = z - depth / 2;
    points.push(x0, y, rz, x1, y, rz);
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(points, 3));
  const material = new THREE.LineBasicMaterial({
    color: "#ffffff",
    transparent: true,
    opacity: Math.min(0.45, Math.max(0.08, visibility * 0.18)),
    depthWrite: false,
  });
  scene.add(new THREE.LineSegments(geometry, material));
}

function addMeshes(THREE, scene, visible, shadow, materialCache) {
  const { voxels, occupied } = visible;
  const faces = mergedVoxelFaces(voxels, occupied);
  const opaqueGroups = new Map();
  for (const face of faces) {
    const color = parseColor(face.fill);
    const alpha = color ? color.a : 1;
    if (alpha <= 0) {
      continue;
    }
    if (alpha < 0.999) {
      const geometry = faceBufferGeometry(THREE, [face]);
      const material = faceMaterial(THREE, face.fill, materialCache);
      const mesh = new THREE.Mesh(geometry, material);
      mesh.renderOrder = 100 + Number(face.objectOrder || 0);
      mesh.castShadow = false;
      mesh.receiveShadow = false;
      scene.add(mesh);
      continue;
    }
    const key = faceMaterialKey(face.fill);
    if (!opaqueGroups.has(key)) {
      opaqueGroups.set(key, []);
    }
    opaqueGroups.get(key).push(face);
  }
  for (const [key, groupFaces] of opaqueGroups) {
    const geometry = faceBufferGeometry(THREE, groupFaces);
    const material = faceMaterial(THREE, groupFaces[0].fill, materialCache);
    const mesh = new THREE.Mesh(geometry, material);
    mesh.castShadow = shadow.enabled;
    mesh.receiveShadow = shadow.enabled;
    scene.add(mesh);
  }
}

function addShadowCatcher(THREE, scene, frame, shadow) {
  if (!shadow.enabled) {
    return;
  }
  const geometry = new THREE.PlaneGeometry(frame.size.width + 1, frame.size.depth + 1);
  const material = new THREE.ShadowMaterial({
    color: 0x000000,
    opacity: 0.28,
    transparent: true,
    depthWrite: false,
  });
  const catcher = new THREE.Mesh(geometry, material);
  catcher.rotation.x = -Math.PI / 2;
  catcher.position.y = -frame.size.height / 2 - 0.02;
  catcher.castShadow = false;
  catcher.receiveShadow = true;
  catcher.renderOrder = -100;
  scene.add(catcher);
}

function applyProjectedRenderCulling(THREE, frame, camera, canvas) {
  if (!projectedRenderCullingEnabled(frame)) {
    frame.renderCells = frame.cells;
    return;
  }
  camera.updateProjectionMatrix?.();
  camera.updateMatrixWorld?.();
  const width = Math.max(1, Number(canvas.clientWidth) || Number(canvas.width) || 1);
  const height = Math.max(1, Number(canvas.clientHeight) || Number(canvas.height) || 1);
  const marginPixels = Math.max(24, Math.min(width, height) * 0.08);
  const marginX = (marginPixels / width) * 2;
  const marginY = (marginPixels / height) * 2;
  const extent = conservativeCellRenderExtent(frame);
  const boundsCache = frame.cellRenderBoundsCache || new Map();
  frame.cellRenderBoundsCache = boundsCache;
  frame.renderCells = frame.cells.filter((cell) => {
    const key = cellKey(cell.position);
    const animated = cellHasTweenAnimation(frame, cell);
    let bounds = animated ? null : boundsCache.get(key);
    if (!bounds) {
      bounds = cellCoordinateRenderBounds(frame, cell, extent);
      if (!animated) {
        boundsCache.set(key, bounds);
      }
    }
    const projected = projectedRenderBounds(THREE, bounds, camera);
    return projected
      && projected.maxX >= -1 - marginX
      && projected.minX <= 1 + marginX
      && projected.maxY >= -1 - marginY
      && projected.minY <= 1 + marginY
      && projected.maxZ >= -1
      && projected.minZ <= 1;
  });
}

function projectedRenderCullingEnabled(frame) {
  return Boolean(frame.viewportRanges && frame.focusCell);
}

function cellCoordinateRenderBounds(frame, cell, extent = conservativeCellRenderExtent(frame)) {
  const positions = [cell.position || {}];
  for (const object of cell.objects || []) {
    const animation = animationForObjectAtPosition(frame, object, cell.position || {});
    if (animation) {
      positions.push(animation.from);
    }
  }
  const bases = positions.map((position) => renderPositionForCell(frame, position));
  return {
    minX: Math.min(...bases.map((base) => base.x)) - extent.x,
    maxX: Math.max(...bases.map((base) => base.x)) + extent.x,
    minY: Math.min(...bases.map((base) => base.y)) - extent.yBelow,
    maxY: Math.max(...bases.map((base) => base.y)) + extent.yAbove,
    minZ: Math.min(...bases.map((base) => base.z)) - extent.z,
    maxZ: Math.max(...bases.map((base) => base.z)) + extent.z,
  };
}

function conservativeCellRenderExtent(frame) {
  return {
    x: 0.65,
    yBelow: 0.65,
    yAbove: 0.65,
    z: 0.65,
  };
}

function emptyProjectedBounds() {
  return {
    minX: Infinity,
    maxX: -Infinity,
    minY: Infinity,
    maxY: -Infinity,
    minZ: Infinity,
    maxZ: -Infinity,
  };
}

function projectedRenderBounds(THREE, bounds, camera) {
  const projected = emptyProjectedBounds();
  const point = new THREE.Vector3();
  for (const x of [bounds.minX, bounds.maxX]) {
    for (const y of [bounds.minY, bounds.maxY]) {
      for (const z of [bounds.minZ, bounds.maxZ]) {
        point.set(x, y, z).project(camera);
        if (!Number.isFinite(point.x) || !Number.isFinite(point.y) || !Number.isFinite(point.z)) {
          continue;
        }
        projected.minX = Math.min(projected.minX, point.x);
        projected.maxX = Math.max(projected.maxX, point.x);
        projected.minY = Math.min(projected.minY, point.y);
        projected.maxY = Math.max(projected.maxY, point.y);
        projected.minZ = Math.min(projected.minZ, point.z);
        projected.maxZ = Math.max(projected.maxZ, point.z);
      }
    }
  }
  return Number.isFinite(projected.minX) ? projected : null;
}

function frameVisibleVoxels(frame) {
  const voxels = [];
  const occupied = emptyVoxelOccupancy();
  const staticCellCache = frame.staticCellVisibilityCache || new Map();
  frame.staticCellVisibilityCache = staticCellCache;
  for (const cell of frame.renderCells || frame.cells) {
    const key = cellKey(cell.position);
    const animated = cellHasTweenAnimation(frame, cell);
    let visible = animated ? null : staticCellCache.get(key);
    if (!visible) {
      visible = cellVisibleVoxels(frame, cell);
      if (!animated) {
        staticCellCache.set(key, visible);
      }
    }
    voxels.push(...visible.voxels);
    for (const key of visible.occupied.opaque) {
      occupied.opaque.add(key);
    }
    for (const key of visible.occupied.bySource) {
      occupied.bySource.add(key);
    }
  }
  return { voxels, occupied };
}

function cellHasTweenAnimation(frame, cell) {
  return (cell.objects || []).some((object) => (
    Boolean(animationForObjectAtPosition(frame, object, cell.position || {}))
  ));
}

function cellVisibleVoxels(frame, cell) {
  const stacks = new Map();
  for (const [objectIndex, object] of (cell.objects || []).entries()) {
    const sourceKey = `${cellKey(cell.position)}:${objectIndex}`;
    const priority = Puzzle3VisualCore.objectPriority(visualOrder(frame), object, objectIndex);
    const objectOrder = (cellRenderIndex(frame, cell.position) * visualOrder(frame).priorities.length) + priority;
    for (const voxel of objectVoxels(frame, cell.position, object, sourceKey, objectOrder)) {
      const key = voxelGeometryKeyAt(voxel.stackPosition, voxel.scale);
      const stack = stacks.get(key) || [];
      stack.push(voxel);
      stacks.set(key, stack);
    }
  }
  const voxels = [];
  const occupied = emptyVoxelOccupancy();
  for (const stack of stacks.values()) {
    const visibleStack = visibleVoxelStack(stack);
    voxels.push(...visibleStack);
    for (const voxel of visibleStack) {
      const key = voxelGeometryKey(voxel);
      for (const sourceKey of voxel.sourceKeys || []) {
        occupied.bySource.add(`${sourceKey}|${key}`);
      }
      if (voxel.opaque) {
        occupied.opaque.add(key);
      }
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
    const order = frameOrder(stack);
    const priority = group.order % Math.max(1, order?.priorities?.length || 1);
    const voxel = Puzzle3VisualCore.priorityDefinition(order, priority).merge
      ? Puzzle3VisualCore.averageMergedVoxels(group.voxels, parseColor, formatColor)
      : group.voxels[0];
    const source = voxel.color || parseColor(voxel.fill);
    if (source?.a <= 0) {
      continue;
    }
    const renderVoxel = {
      ...voxel,
      color: source || null,
      opaque: !source || source.a >= 0.999,
      fill: source ? formatColor(source) : voxel.fill,
      sourceKeys: voxel.sourceKey ? [voxel.sourceKey] : [],
    };
    if (renderVoxel.opaque) {
      visible.length = 0;
    }
    visible.push(renderVoxel);
  }
  return visible;
}

function visualOrder(frame) {
  const order = frame?.order;
  if (!order || !Array.isArray(order.direction_priority) || !Array.isArray(order.priorities)) {
    throw new Error("compiled visual order contract is missing");
  }
  return order;
}

function cellRenderIndex(frame, position) {
  const spans = {
    x: Math.max(1, Number(frame?.size?.width) || 1),
    y: Math.max(1, Number(frame?.size?.depth) || 1),
    z: Math.max(1, Number(frame?.size?.height) || 1),
  };
  let index = 0;
  for (const direction of visualOrder(frame).direction_priority) {
    let value;
    let span;
    switch (direction) {
      case "right": value = Number(position.x) || 0; span = spans.x; break;
      case "left": value = spans.x - 1 - (Number(position.x) || 0); span = spans.x; break;
      case "front": value = Number(position.y) || 0; span = spans.y; break;
      case "back": value = spans.y - 1 - (Number(position.y) || 0); span = spans.y; break;
      case "up": value = Number(position.z) || 0; span = spans.z; break;
      case "down": value = spans.z - 1 - (Number(position.z) || 0); span = spans.z; break;
      default: throw new Error(`invalid 3D visual order direction: ${direction}`);
    }
    index = (index * span) + value;
  }
  return index;
}

function frameOrder(stack) {
  return stack[0]?.frameOrder;
}

function objectVoxelOrder(voxel) {
  const order = Number(voxel?.objectOrder);
  return Number.isFinite(order) ? order : 0;
}

function objectVoxels(frame, position, object, sourceKey, objectOrder = 0) {
  if (object.visual?.kind === "voxels") {
    return voxelInstances(frame, position, object, sourceKey, objectOrder);
  }
  return [];
}

function voxelInstances(frame, position, object, sourceKey, objectOrder = 0) {
  const visual = object.visual;
  const size = visual.size;
  const step = 1 / Math.max(size.width, size.height, size.depth, 1);
  const base = renderPositionForCell(frame, position);
  const animation = animationForObjectAtPosition(frame, object, position);
  const offset = animationOffset3(frame, animation);
  const spatialAffine = animationSpatialAffine(frame, object, animation);
  base.x += offset.x;
  base.y += offset.y;
  base.z += offset.z;
  return visual.voxels.map((voxel) => {
    const local = Puzzle3VisualCore.transformSpatialPoint(
      visualVoxelLocalPosition(voxel, size, step),
      spatialAffine,
    );
    const localGrid = Puzzle3VisualCore.spatialGridPoint(local, step);
    const renderPosition = {
      x: base.x + local.x,
      y: base.y + local.z,
      z: base.z - local.y,
    };
    const stackPosition = {
      x: base.x + local.x,
      y: base.y + local.z,
      z: base.z - local.y,
    };
    return {
      fill: voxel.fill,
      color: voxel.color,
      opaque: voxel.opaque,
      scale: step,
      grid: { x: localGrid.x, y: localGrid.z, z: -localGrid.y },
      position: renderPosition,
      stackPosition,
      bounds: voxelBounds(renderPosition, step),
      sourceKey,
      objectOrder,
      frameOrder: frame.order,
    };
  });
}

function animationForObjectAtPosition(frame, object, position) {
  const objectId = Number(object?.id);
  if (!Number.isFinite(objectId)) {
    return null;
  }
  return frame.animationEventIndex?.get(tweenAnimationEventKey(objectId, position)) || null;
}

function indexTweenAnimationEvents(events) {
  if (!window.PuzzleVisualTweenCore) {
    throw new Error("Visual tween core is unavailable.");
  }
  const index = new Map();
  for (const event of window.PuzzleVisualTweenCore.resolveAnimationChannels(events || [])) {
    if (event?.kind !== "move" || event?.name !== "tween") {
      continue;
    }
    const objectId = Number(event.objectId);
    if (!Number.isFinite(objectId)) {
      throw new Error("Puzzle3 Tween animation event objectId must be finite.");
    }
    const key = tweenAnimationEventKey(objectId, event.to);
    if (index.has(key)) {
      throw new Error(`Puzzle3 Tween animation event target is duplicated: ${key}`);
    }
    index.set(key, event);
  }
  return index;
}

function tweenAnimationEventKey(objectId, position) {
  return `${Number(objectId)}@${Number(position?.x)},${Number(position?.y)},${Number(position?.z)}`;
}

function animationSpatialAffine(frame, object, animation) {
  if (!animation?.visualTween) {
    return object.visual.spatialAffine;
  }
  if (!window.PuzzleVisualTweenCore) {
    throw new Error("Visual tween core is unavailable.");
  }
  const state = window.PuzzleVisualTweenCore.interpolate(
    animation.visualTween,
    frame.animationProgress,
  );
  const operations = state.transforms.map((transform) => {
    if (transform.kind === "rotate") {
      return { ...transform, kind: "rotate3" };
    }
    if (transform.kind === "translate") {
      return { ...transform, kind: "translate3" };
    }
    if (transform.kind === "scale") {
      return { ...transform, kind: "scale3" };
    }
    if (transform.kind === "flip") {
      return { ...transform, kind: "flip3" };
    }
    throw new Error(`Unknown Puzzle3 visual tween transform: ${String(transform.kind)}`);
  });
  return Puzzle3VisualCore.evaluateSpatialVisualAffine(operations);
}

function animationOffset3(frame, animation) {
  if (!animation) {
    return { x: 0, y: 0, z: 0 };
  }
  const progress = Math.min(1, Math.max(0, Number(frame.animationProgress) || 0));
  const remaining = 1 - progress;
  return {
    x: (Number(animation.from?.x) - Number(animation.to?.x)) * remaining,
    y: (Number(animation.from?.z) - Number(animation.to?.z)) * remaining,
    z: -(Number(animation.from?.y) - Number(animation.to?.y)) * remaining,
  };
}

function mergedVoxelFaces(voxels, occupied) {
  if (window.Puzzle3VisualCore?.mergeVoxelFaces) {
    return Puzzle3VisualCore.mergeVoxelFaces(voxels, {
      faces: voxelFaces,
      isFaceVisible: (voxel, face) => !isVoxelFaceOccluded(voxel, face.offset, occupied),
      group: (voxel, face) => {
        const info = voxelFaceGroupInfo(voxel, face.side);
        const groupKey = [
          quantizeGeometryValue(voxel.objectOrder),
          face.side,
          quantizeGeometryValue(info.origin.x),
          quantizeGeometryValue(info.origin.y),
          quantizeGeometryValue(info.origin.z),
          quantizeGeometryValue(voxel.scale),
          info.planeIndex,
          voxel.fill,
        ].join("|");
        return {
          key: groupKey,
          u: info.u,
          v: info.v,
          group: {
            key: groupKey,
            objectOrder: voxel.objectOrder,
            side: face.side,
            origin: info.origin,
            scale: voxel.scale,
            planeIndex: info.planeIndex,
            fill: voxel.fill,
          },
        };
      },
      face: (group, rect) => faceGeometry(mergedVoxelFaceCorners(group, rect), group.fill, group.objectOrder, group.side),
    });
  }
  return voxels.flatMap((voxel) => voxelFaces(voxel)
    .filter((face) => !isVoxelFaceOccluded(voxel, face.offset, occupied))
    .map((face) => faceGeometry(face.corners, voxel.fill, voxel.objectOrder, face.side)));
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
      corners: [{ x: x1, y: y0, z: z0 }, { x: x0, y: y0, z: z0 }, { x: x0, y: y1, z: z0 }, { x: x1, y: y1, z: z0 }],
    },
    {
      side: "zPos",
      offset: { x: 0, y: 0, z: 1 },
      corners: [{ x: x0, y: y0, z: z1 }, { x: x1, y: y0, z: z1 }, { x: x1, y: y1, z: z1 }, { x: x0, y: y1, z: z1 }],
    },
    {
      side: "xNeg",
      offset: { x: -1, y: 0, z: 0 },
      corners: [{ x: x0, y: y0, z: z0 }, { x: x0, y: y0, z: z1 }, { x: x0, y: y1, z: z1 }, { x: x0, y: y1, z: z0 }],
    },
    {
      side: "xPos",
      offset: { x: 1, y: 0, z: 0 },
      corners: [{ x: x1, y: y0, z: z1 }, { x: x1, y: y0, z: z0 }, { x: x1, y: y1, z: z0 }, { x: x1, y: y1, z: z1 }],
    },
    {
      side: "yPos",
      offset: { x: 0, y: 1, z: 0 },
      corners: [{ x: x0, y: y1, z: z1 }, { x: x1, y: y1, z: z1 }, { x: x1, y: y1, z: z0 }, { x: x0, y: y1, z: z0 }],
    },
    {
      side: "yNeg",
      offset: { x: 0, y: -1, z: 0 },
      corners: [{ x: x0, y: y0, z: z0 }, { x: x1, y: y0, z: z0 }, { x: x1, y: y0, z: z1 }, { x: x0, y: y0, z: z1 }],
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

function faceGeometry(corners, fill, objectOrder = 0, side = "") {
  return { corners, fill, objectOrder, side };
}

function faceBufferGeometry(THREE, faces) {
  const positions = [];
  const normals = [];
  for (const face of faces) {
    const normal = faceNormal(face.side, face.corners);
    const corners = face.corners || [];
    for (const index of [0, 1, 2, 0, 2, 3]) {
      const point = corners[index];
      positions.push(point.x, point.y, point.z);
      normals.push(normal.x, normal.y, normal.z);
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute("normal", new THREE.Float32BufferAttribute(normals, 3));
  return geometry;
}

function faceNormal(side, corners) {
  if (side === "xNeg") {
    return { x: -1, y: 0, z: 0 };
  }
  if (side === "xPos") {
    return { x: 1, y: 0, z: 0 };
  }
  if (side === "yNeg") {
    return { x: 0, y: -1, z: 0 };
  }
  if (side === "yPos") {
    return { x: 0, y: 1, z: 0 };
  }
  if (side === "zNeg") {
    return { x: 0, y: 0, z: -1 };
  }
  if (side === "zPos") {
    return { x: 0, y: 0, z: 1 };
  }
  const [a, b, c] = corners || [];
  if (!a || !b || !c) {
    return { x: 0, y: 1, z: 0 };
  }
  const ux = b.x - a.x;
  const uy = b.y - a.y;
  const uz = b.z - a.z;
  const vx = c.x - a.x;
  const vy = c.y - a.y;
  const vz = c.z - a.z;
  const nx = uy * vz - uz * vy;
  const ny = uz * vx - ux * vz;
  const nz = ux * vy - uy * vx;
  const length = Math.hypot(nx, ny, nz) || 1;
  return { x: nx / length, y: ny / length, z: nz / length };
}

function faceMaterial(THREE, fill, cache) {
  const key = faceMaterialKey(fill);
  if (cache.has(key)) {
    return cache.get(key);
  }
  const color = parseColor(fill);
  const alpha = color ? color.a : 1;
  const material = new THREE.MeshLambertMaterial({
    color: color ? formatRgbColor(color) : fill,
    transparent: alpha < 0.999,
    opacity: Math.max(0, Math.min(1, alpha)),
    depthWrite: alpha >= 0.999,
  });
  cache.set(key, material);
  return material;
}

function faceMaterialKey(fill) {
  const color = parseColor(fill);
  return color ? `${formatRgbColor(color)}:${Math.max(0, Math.min(1, color.a))}` : `${fill}:1`;
}

function visualVoxelLocalPosition(voxel, size, step) {
  return {
    x: (voxel.x + 0.5 - size.width / 2) * step,
    y: (voxel.y + 0.5 - size.depth / 2) * step,
    z: (voxel.z + 0.5 - size.height / 2) * step,
  };
}

function voxelBounds(position, scale) {
  const half = scale / 2;
  return {
    x0: position.x - half,
    x1: position.x + half,
    y0: position.y - half,
    y1: position.y + half,
    z0: position.z - half,
    z1: position.z + half,
  };
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
  return String(Math.round(Number(value) * 1000000) / 1000000);
}

function cellKey(position) {
  return `${Number(position?.x) || 0},${Number(position?.y) || 0},${Number(position?.z) || 0}`;
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

function formatRgbColor(color) {
  return `rgb(${clampColorChannel(color.r)}, ${clampColorChannel(color.g)}, ${clampColorChannel(color.b)})`;
}

function clampColorChannel(value) {
  return Math.max(0, Math.min(255, Math.round(value)));
}

function renderPositionForCell(frame, position) {
  return {
    x: position.x - (frame.size.width - 1) / 2,
    y: position.z - (frame.size.height - 1) / 2,
    z: (frame.size.depth - 1) / 2 - position.y,
  };
}

function viewportFocusCell(frame) {
  const viewport = frame.viewport;
  if (!viewport || (viewport.mode !== "centered" && viewport.mode !== "paged")) {
    return null;
  }
  const focusObjects = new Set(viewport.focusObjects || []);
  return frame.cells.find((cell) => (
    cell.objects || []
  ).some((object) => viewportObjectMatches(object, viewport, focusObjects))) || null;
}

function viewportObjectMatches(object, viewport, focusObjects) {
  const objectId = Number(object.id || 0);
  return (
    (focusObjects.size > 0 && focusObjects.has(objectId))
    || object.name === viewport.focus
    || object.visual === viewport.focus
  );
}

function viewportRanges(frame) {
  const viewport = frame.viewport;
  if (!viewport || !frame.focusCell || (viewport.mode !== "centered" && viewport.mode !== "paged")) {
    return null;
  }
  const position = frame.focusCell.position || {};
  const zRange = viewport.framingBox.height === "full"
    ? { min: -0.5, max: frame.size.height - 0.5 }
    : rangeForViewportAxis(Number(position.z) || 0, viewport.framingBox.height, viewport.mode);
  return {
    x: rangeForViewportAxis(Number(position.x) || 0, viewport.framingBox.width, viewport.mode),
    y: rangeForViewportAxis(Number(position.y) || 0, viewport.framingBox.depth, viewport.mode),
    z: zRange,
  };
}

function rangeForViewportAxis(center, span, mode) {
  const safeSpan = Math.max(1, Number(span) || 1);
  const safeCenter = Number(center) || 0;
  if (mode === "paged") {
    const min = Math.floor(safeCenter / safeSpan) * safeSpan - 0.5;
    return { min, max: min + safeSpan };
  }
  return {
    min: safeCenter - safeSpan / 2,
    max: safeCenter + safeSpan / 2,
  };
}

function buildCamera(THREE, frame, canvas) {
  const width = canvas.clientWidth || canvas.width || 1;
  const height = canvas.clientHeight || canvas.height || 1;
  const aspect = width / Math.max(1, height);
  const cameraSettings = frame.camera || {};
  const zoom = cameraZoom(frame);
  const view = cameraViewForFrame(frame, aspect, zoom);
  const targetPoint = new THREE.Vector3(view.target.x, view.target.y, view.target.z);
  const distance = view.distance;
  const cameraFrame = cameraRenderFrame(cameraSettings);
  const projection = String(cameraSettings.projection || "").toLowerCase();
  const near = 0.1;
  const far = Math.max(1000, distance * 4);
  const camera = projection === "orthographic"
    ? buildOrthographicCamera(THREE, aspect, view.visibleHeight, near, far)
    : new THREE.PerspectiveCamera(34, aspect, near, far);
  camera.up.set(cameraFrame.up.x, cameraFrame.up.y, cameraFrame.up.z);
  camera.position.set(
    targetPoint.x - cameraFrame.forward.x * distance,
    targetPoint.y - cameraFrame.forward.y * distance,
    targetPoint.z - cameraFrame.forward.z * distance,
  );
  camera.lookAt(targetPoint);
  return camera;
}

function threeViewPayload(frame, camera, canvas) {
  const width = Math.max(1, Number(canvas.clientWidth) || Number(canvas.width) || 1);
  const height = Math.max(1, Number(canvas.clientHeight) || Number(canvas.height) || 1);
  const rect = canvas.getBoundingClientRect();
  const cameraView = frame.cameraView || cameraViewForFrame(frame, width / height, cameraZoom(frame));
  return {
    width,
    height,
    viewport: {
      width: Math.max(1, Number(window.innerWidth) || width),
      height: Math.max(1, Number(window.innerHeight) || height),
    },
    canvasRect: {
      x: Number(rect.x) || 0,
      y: Number(rect.y) || 0,
      width: Math.max(1, Number(rect.width) || width),
      height: Math.max(1, Number(rect.height) || height),
    },
    coordinateSpace: "canvas-css-px",
    scale: 1,
    center: logicalTargetForThreeView(frame, cameraView),
    camera: {
      yawDegrees: Number(frame.camera?.yawDegrees ?? 0),
      pitchDegrees: Number(frame.camera?.pitchDegrees ?? 35),
      rollDegrees: Number(frame.camera?.rollDegrees ?? 0),
      zoom: Number(frame.camera?.zoom ?? frame.editorView?.zoom ?? 1) || 1,
      projection: String(frame.camera?.projection || "").toLowerCase() === "orthographic" ? "orthographic" : "",
    },
    threeProjection: {
      size: { ...frame.size },
      target: { ...cameraView.target },
      distance: Number(cameraView.distance) || 1,
      visibleHeight: Number(cameraView.visibleHeight) || 1,
      fovDegrees: Number(camera.fov) || 34,
      aspect: width / Math.max(1, height),
      projection: camera.isOrthographicCamera ? "orthographic" : "perspective",
    },
  };
}

function logicalTargetForThreeView(frame, cameraView) {
  const size = frame.size || { width: 1, depth: 1, height: 1 };
  return {
    x: Number(cameraView.target?.x) + (size.width - 1) / 2,
    y: (size.depth - 1) / 2 - Number(cameraView.target?.z),
    z: Number(cameraView.target?.y) + (size.height - 1) / 2,
  };
}

function buildOrthographicCamera(THREE, aspect, visibleHeight, near, far) {
  const height = Math.max(1, Number(visibleHeight) || 1);
  const width = height * Math.max(0.01, aspect);
  return new THREE.OrthographicCamera(
    -width / 2,
    width / 2,
    height / 2,
    -height / 2,
    near,
    far,
  );
}

function cameraViewForFrame(frame, aspect, zoom) {
  const ranges = frame.viewportRanges || fullFrameRanges(frame);
  const center = {
    x: (ranges.x.min + ranges.x.max) / 2,
    y: (ranges.y.min + ranges.y.max) / 2,
    z: (ranges.z.min + ranges.z.max) / 2,
  };
  const target = frame.viewportRanges
    ? viewportFocusRenderTarget(frame)
    : renderPositionForCell(frame, {
        ...center,
        ...(frame.editorView?.target || {}),
      });
  const visibleWidth = Math.max(1, ranges.x.max - ranges.x.min);
  const visibleDepth = Math.max(1, ranges.y.max - ranges.y.min);
  const fov = 34 * Math.PI / 180;
  const visibleHeight = frame.viewportRanges
    ? viewportProjectedVisibleHeight(frame, target, aspect)
    : Math.max(visibleDepth, visibleWidth / Math.max(0.01, aspect)) * 1.08;
  const fittedVisibleHeight = visibleHeight * 1.12 / zoom;
  const targetDistance = Math.max(4, fittedVisibleHeight / (2 * Math.tan(fov / 2)));
  const snap = frame.viewport?.follow !== "smooth" || frame.viewportSnapNext || !thisLikeHasView(frame);
  const previousTarget = frame.rendererViewTarget || null;
  const previousDistance = Number(frame.rendererViewDistance);
  const cameraTarget = snap || !previousTarget
    ? target
    : smoothViewportTarget(
        {
          x: lerp(previousTarget.x, target.x, 0.12),
          y: lerp(previousTarget.y, target.y, 0.12),
          z: lerp(previousTarget.z, target.z, 0.12),
        },
        target,
        frame,
      );
  const distance = snap || !Number.isFinite(previousDistance)
    ? targetDistance
    : lerp(previousDistance, targetDistance, 0.12);
  frame.viewportAnimating = !snap && (
    Math.abs(cameraTarget.x - target.x) > 0.01
    || Math.abs(cameraTarget.y - target.y) > 0.01
    || Math.abs(cameraTarget.z - target.z) > 0.01
    || Math.abs(distance - targetDistance) > 0.01
  );
  frame.cameraView = { target: cameraTarget, distance, visibleHeight: fittedVisibleHeight };
  return frame.cameraView;
}

function thisLikeHasView(frame) {
  return Boolean(frame.rendererViewTarget) && Number.isFinite(Number(frame.rendererViewDistance));
}

function smoothViewportTarget(next, target, frame) {
  const dx = target.x - next.x;
  const dy = target.y - next.y;
  const dz = target.z - next.z;
  const distance = Math.hypot(dx, dy, dz);
  const maxLag = smoothViewportMaxLag(frame);
  if (!Number.isFinite(distance) || distance <= maxLag) {
    return next;
  }
  const catchUp = (distance - maxLag) / distance;
  return {
    x: next.x + dx * catchUp,
    y: next.y + dy * catchUp,
    z: next.z + dz * catchUp,
  };
}

function smoothViewportMaxLag(frame) {
  const ranges = frame.viewportRanges;
  if (!ranges) {
    return 3.5;
  }
  const width = Math.max(1, ranges.x.max - ranges.x.min);
  const depth = Math.max(1, ranges.y.max - ranges.y.min);
  const horizontalLag = Math.min(width, depth) / 2;
  if (frame.viewport?.framingBox?.height === "full") {
    return Math.max(1, horizontalLag);
  }
  const height = Math.max(1, ranges.z.max - ranges.z.min);
  return Math.max(1, Math.min(horizontalLag, height / 2));
}

function cameraZoom(frame) {
  const cameraSettings = frame.camera || {};
  const cameraValue = Math.max(0.1, Number(cameraSettings.zoom ?? 1) || 1);
  const viewValue = Math.max(0.1, Number(frame.editorView?.zoom ?? 1) || 1);
  return cameraValue * viewValue;
}

function viewportFocusRenderTarget(frame) {
  const visualBounds = viewportFocusVisualRenderBounds(frame);
  if (visualBounds) {
    return {
      x: (visualBounds.minX + visualBounds.maxX) / 2,
      y: (visualBounds.minY + visualBounds.maxY) / 2,
      z: (visualBounds.minZ + visualBounds.maxZ) / 2,
    };
  }
  return renderPositionForCell(frame, frame.focusCell?.position || {});
}

function viewportFocusVisualRenderBounds(frame) {
  const viewport = frame.viewport || {};
  const focusObjects = new Set(viewport.focusObjects || []);
  const bounds = {
    minX: Infinity,
    maxX: -Infinity,
    minY: Infinity,
    maxY: -Infinity,
    minZ: Infinity,
    maxZ: -Infinity,
  };
  for (const [objectIndex, object] of (frame.focusCell?.objects || []).entries()) {
    if (!viewportObjectMatches(object, viewport, focusObjects)) {
      continue;
    }
    const sourceKey = `${cellKey(frame.focusCell.position)}:${objectIndex}`;
    const priority = Puzzle3VisualCore.objectPriority(visualOrder(frame), object, objectIndex);
    const objectOrder = (cellRenderIndex(frame, frame.focusCell.position || {}) * visualOrder(frame).priorities.length) + priority;
    for (const voxel of objectVoxels(frame, frame.focusCell.position || {}, object, sourceKey, objectOrder)) {
      bounds.minX = Math.min(bounds.minX, voxel.bounds.x0);
      bounds.maxX = Math.max(bounds.maxX, voxel.bounds.x1);
      bounds.minY = Math.min(bounds.minY, voxel.bounds.y0);
      bounds.maxY = Math.max(bounds.maxY, voxel.bounds.y1);
      bounds.minZ = Math.min(bounds.minZ, voxel.bounds.z0);
      bounds.maxZ = Math.max(bounds.maxZ, voxel.bounds.z1);
    }
  }
  return Number.isFinite(bounds.minX) ? bounds : null;
}

function viewportProjectedVisibleHeight(frame, target, aspect) {
  const bounds = viewportProjectedBounds(frame);
  if (!bounds) {
    const ranges = frame.viewportRanges || fullFrameRanges(frame);
    const visibleWidth = Math.max(1, ranges.x.max - ranges.x.min);
    const visibleDepth = Math.max(1, ranges.y.max - ranges.y.min);
    return Math.max(visibleDepth, visibleWidth / Math.max(0.01, aspect));
  }
  const anchor = projectRenderPointForCamera(target, frame.camera || {});
  const halfWidth = Math.max(
    0.001,
    Math.max(Math.abs(bounds.minX - anchor.x), Math.abs(bounds.maxX - anchor.x)),
  );
  const halfHeight = Math.max(
    0.001,
    Math.max(Math.abs(bounds.minY - anchor.y), Math.abs(bounds.maxY - anchor.y)),
  );
  return Math.max(halfHeight * 2, (halfWidth * 2) / Math.max(0.01, aspect));
}

function viewportProjectedBounds(frame) {
  if (!frame.viewportRanges) {
    return null;
  }
  const points = [];
  for (const x of [frame.viewportRanges.x.min, frame.viewportRanges.x.max]) {
    for (const y of [frame.viewportRanges.y.min, frame.viewportRanges.y.max]) {
      for (const z of [frame.viewportRanges.z.min, frame.viewportRanges.z.max]) {
        points.push(
          projectRenderPointForCamera(renderPositionForCell(frame, { x, y, z }), frame.camera || {}),
        );
      }
    }
  }
  return projectedPointBounds(points);
}

function projectRenderPointForCamera(point, cameraSettings) {
  const frame = cameraRenderFrame(cameraSettings);
  return {
    x: point.x * frame.right.x + point.y * frame.right.y + point.z * frame.right.z,
    y: point.x * frame.up.x + point.y * frame.up.y + point.z * frame.up.z,
  };
}

function cameraRenderFrame(cameraSettings) {
  const yaw = degreesToRadians(cameraSettings.yawDegrees ?? 0);
  const pitch = degreesToRadians(clamp(Number(cameraSettings.pitchDegrees ?? 35) || 35, -90, 90));
  const roll = degreesToRadians(cameraSettings.rollDegrees ?? 0);
  const horizontal = Math.cos(pitch);
  const baseRight = { x: Math.cos(yaw), y: 0, z: Math.sin(yaw) };
  const forward = {
    x: Math.sin(yaw) * horizontal,
    y: -Math.sin(pitch),
    z: -Math.cos(yaw) * horizontal,
  };
  const baseUp = {
    x: baseRight.y * forward.z - baseRight.z * forward.y,
    y: baseRight.z * forward.x - baseRight.x * forward.z,
    z: baseRight.x * forward.y - baseRight.y * forward.x,
  };
  const cosRoll = Math.cos(roll);
  const sinRoll = Math.sin(roll);
  const right = {
    x: baseRight.x * cosRoll + baseUp.x * sinRoll,
    y: baseRight.y * cosRoll + baseUp.y * sinRoll,
    z: baseRight.z * cosRoll + baseUp.z * sinRoll,
  };
  const up = {
    x: -baseRight.x * sinRoll + baseUp.x * cosRoll,
    y: -baseRight.y * sinRoll + baseUp.y * cosRoll,
    z: -baseRight.z * sinRoll + baseUp.z * cosRoll,
  };
  return {
    right,
    up,
    forward,
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

function fullFrameRanges(frame) {
  return {
    x: { min: -0.5, max: frame.size.width - 0.5 },
    y: { min: -0.5, max: frame.size.depth - 0.5 },
    z: { min: -0.5, max: frame.size.height - 0.5 },
  };
}

function lerp(from, to, amount) {
  return from + (to - from) * amount;
}

function degreesToRadians(value) {
  return (value * Math.PI) / 180;
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

window.Puzzle3ThreeRenderer = {
  contract: PUZZLE3_THREE_RENDERER_CONTRACT,
  create(canvas, options) {
    return new Puzzle3ThreeRenderer(canvas, options);
  },
  buildPuzzleStudioThreeFrame,
  updatePuzzleStudioThreeFrame,
  animationOffset3,
};
})();
