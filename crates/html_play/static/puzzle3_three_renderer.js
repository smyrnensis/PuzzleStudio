(() => {
const PUZZLE3_THREE_RENDERER_CONTRACT = {
  version: 3,
  input: ["snapshot", "resolvedScene", "resolvedFrame", "view"],
};

class Puzzle3ThreeRenderer {
  constructor(canvas, options = {}) {
    this.canvas = canvas;
    this.options = options;
    this.renderer = null;
    this.rendererAntialias = null;
    this.scene = null;
    this.camera = null;
    this.loading = null;
    this.failed = null;
    this.viewTarget = null;
    this.viewDistance = null;
    this.viewPayload = null;
    this.activeSnapshot = null;
    this.activeResolvedScene = null;
    this.activeResolvedFrame = null;
    this.frame = null;
    this.faceMaterialCache = new Map();
  }

  render(snapshot, resolvedScene, resolvedFrame, view = {}) {
    if (!snapshot) {
      return { rendered: false, reason: "missing-snapshot" };
    }
    requireResolvedRenderScene(resolvedScene);
    requireResolvedVoxelFrame(resolvedFrame);
    if (!window.THREE) {
      this.ensureThreeLoaded();
      return { rendered: false, reason: this.failed ? "three-load-failed" : "loading-three" };
    }

    const THREE = window.THREE;
    const snapshotChanged = snapshot !== this.activeSnapshot;
    const resolvedSceneChanged = resolvedScene !== this.activeResolvedScene;
    const resolvedFrameChanged = resolvedFrame !== this.activeResolvedFrame;
    this.activeSnapshot = snapshot;
    this.activeResolvedScene = resolvedScene;
    this.activeResolvedFrame = resolvedFrame;
    const frame = buildPuzzleStudioThreeFrame(snapshot, resolvedScene, resolvedFrame, view);
    this.frame = frame;
    const pixelate = pixelateSettings(frame);
    this.ensureRenderer(THREE, pixelate.enabled && pixelate.smoothing);
    frame.rendererViewTarget = this.viewTarget;
    frame.rendererViewDistance = this.viewDistance;
    const camera = buildCamera(THREE, frame, this.canvas, this.camera);
    const shadow = shadowSettings(frame);
    const rebuildScene = snapshotChanged
      || resolvedSceneChanged
      || resolvedFrameChanged
      || !this.scene
      || view.viewportSnapNext === true;
    if (rebuildScene) {
      frame.renderCells = frame.cells;
      const visible = frameVisibleVoxels(frame);
      const scene = new THREE.Scene();
      addLights(THREE, scene, frame, visible.voxels, shadow);
      addGrid(THREE, scene, frame);
      addMeshes(THREE, scene, visible, shadow, this.faceMaterialCache, visualShadeEnabled(frame));
      addShadowCatcher(THREE, scene, frame, shadow);
      disposeScene(this.scene, this.faceMaterialCache);
      this.scene = scene;
    }
    this.scene.background = threeBackground(THREE, view.background);
    this.configureShadowMap(THREE, shadow.enabled, rebuildScene);
    this.configureRenderResolution(pixelate);
    this.renderer.setClearColor(0x000000, this.scene.background ? 1 : 0);
    this.renderer.render(this.scene, camera);
    this.camera = camera;
    this.viewPayload = threeViewPayload(frame, camera, this.canvas);
    this.updateViewportMotion(frame);
    const viewportAnimating = frame.viewport?.follow === "smooth"
      && frame.viewportAnimating === true;
    return {
      rendered: true,
      objectCount: frame.objectCount,
      animating: viewportAnimating,
      continueAnimation: resolvedFrame.continueAnimation === true,
      view: this.viewPayload,
    };
  }

  destroy() {
    disposeScene(this.scene);
    this.faceMaterialCache.clear();
    this.renderer?.dispose?.();
    this.scene = null;
    this.renderer = null;
    this.rendererAntialias = null;
    this.frame = null;
    this.activeSnapshot = null;
    this.activeResolvedScene = null;
    this.activeResolvedFrame = null;
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

  ensureRenderer(THREE, antialias) {
    if (this.renderer && this.rendererAntialias === antialias) {
      return;
    }
    this.renderer?.dispose?.();
    this.renderer = new THREE.WebGLRenderer({
      canvas: this.canvas,
      antialias,
      alpha: true,
    });
    this.rendererAntialias = antialias;
  }

  configureRenderResolution(pixelate) {
    const width = Math.max(1, Number(this.canvas.clientWidth) || Number(this.canvas.width) || 1);
    const height = Math.max(1, Number(this.canvas.clientHeight) || Number(this.canvas.height) || 1);
    const ratio = window.devicePixelRatio || 1;
    const rasterScale = pixelate.enabled ? pixelate.scale : 1;
    this.renderer.setPixelRatio(ratio / rasterScale);
    this.renderer.setSize(width, height, false);
    if (this.canvas.style) {
      this.canvas.style.imageRendering = pixelate.enabled ? "pixelated" : "";
    }
  }

  configureShadowMap(THREE, enabled, sceneChanged) {
    if (enabled && THREE.PCFSoftShadowMap === undefined) {
      throw new Error("Three.js PCFSoftShadowMap is required for Puzzle3 shadows.");
    }
    this.renderer.shadowMap.enabled = enabled;
    this.renderer.shadowMap.autoUpdate = false;
    this.renderer.shadowMap.needsUpdate = enabled && sceneChanged;
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

function requireResolvedVoxelFrame(resolvedFrame) {
  if (!resolvedFrame || typeof resolvedFrame !== "object" || Array.isArray(resolvedFrame)) {
    throw new Error("Puzzle3 renderer requires a Rust-resolved render frame.");
  }
  if (!Array.isArray(resolvedFrame.batches)
      || typeof resolvedFrame.continueAnimation !== "boolean") {
    throw new Error("Puzzle3 resolved render frame is missing batches or animation state.");
  }
  for (const batch of resolvedFrame.batches) {
    if (batch?.content?.kind !== "voxels") {
      throw new Error(`Puzzle3 renderer received unsupported resolved primitive: ${String(batch?.content?.kind)}`);
    }
  }
  return resolvedFrame;
}

function requireResolvedRenderScene(resolvedScene) {
  if (!resolvedScene || typeof resolvedScene !== "object" || Array.isArray(resolvedScene)) {
    throw new Error("Puzzle3 renderer requires a Rust-resolved render scene.");
  }
  if (!Array.isArray(resolvedScene.cells)) {
    throw new Error("Puzzle3 resolved render scene is missing typed cells.");
  }
  for (const cell of resolvedScene.cells) {
    if (!Array.isArray(cell?.position)
        || cell.position.length !== 3
        || !Array.isArray(cell.objectIds)
        || cell.objectIds.some((objectId) => !Number.isInteger(Number(objectId)) || Number(objectId) <= 0)) {
      throw new Error("Puzzle3 resolved render scene contains an invalid focus cell.");
    }
  }
  return resolvedScene;
}

function requireSpatialAffine(value) {
  if (!Array.isArray(value)
      || value.length !== 4
      || value.some((row) => !Array.isArray(row)
        || row.length !== 4
        || row.some((entry) => typeof entry !== "number" || !Number.isFinite(entry)))) {
    throw new Error("Puzzle3 resolved batch is missing its affine transform.");
  }
  return value;
}

function buildPuzzleStudioThreeFrame(snapshot, resolvedScene, resolvedFrame, view = {}) {
  const size = normalizeSize(snapshot.size);
  const cells = resolvedScene.cells.map((cell) => ({
    position: normalizeResolvedPosition(cell.position),
    objectIds: cell.objectIds.map(Number),
  }));
  const frame = {
    size,
    cells,
    resolvedVoxels: resolvedFrame.batches.flatMap((batch, index) => (
      resolvedBatchVoxels(size, batch, index)
    )),
    objectCount: resolvedFrame.batches.length,
    camera: snapshot.render.camera,
    editorView: view.editorView || snapshot.view || {},
    settings: snapshot.render,
    viewport: normalizeViewport(snapshot.render.viewport),
    viewportSnapNext: view.viewportSnapNext === true,
  };
  frame.focusCell = frame.viewport ? viewportFocusCell(frame) : null;
  frame.viewportRanges = viewportRanges(frame);
  return frame;
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

function normalizeResolvedPosition(position) {
  return {
    x: Number(position[0]) || 0,
    y: Number(position[1]) || 0,
    z: Number(position[2]) || 0,
  };
}

function resolvedBatchVoxels(frameSize, batch, batchIndex) {
  const content = batch.content;
  const size = {
    width: requirePositiveInteger(content.width, "resolved voxel width"),
    depth: requirePositiveInteger(content.depth, "resolved voxel depth"),
    height: requirePositiveInteger(content.height, "resolved voxel height"),
  };
  const cell = requireResolvedCell(batch.cell);
  const spatialAffine = requireSpatialAffine(batch.transform);
  const opacity = requireUnitValue(batch.opacity, "resolved batch opacity");
  const objectIds = Array.isArray(batch.objectIds)
    ? batch.objectIds.map(Number).filter(Number.isFinite)
    : [];
  const step = 1 / Math.max(size.width, size.depth, size.height);
  const base = renderPositionForCell({ size: frameSize }, cell);
  const sourceKey = `batch:${batchIndex}`;
  const mirrored = affineDeterminant3(spatialAffine) < 0;
  return content.voxels.map((voxel, voxelIndex) => {
    const position = requireResolvedVoxelPosition(voxel.position, voxelIndex);
    const color = resolvedLinearColor(voxel.color, opacity);
    const visualLocalPosition = {
      x: (position[0] + 0.5 - size.width / 2) * step,
      y: (position[1] + 0.5 - size.depth / 2) * step,
      z: (position[2] + 0.5 - size.height / 2) * step,
    };
    const visualLocalGrid = Puzzle3VisualCore.spatialGridPoint(visualLocalPosition, step);
    const renderLocalPosition = visualPointToRenderPoint(visualLocalPosition);
    const localBounds = voxelBounds(renderLocalPosition, step);
    const renderPosition = transformRenderLocalPoint(renderLocalPosition, spatialAffine, base);
    return {
      fill: formatColor(color),
      color,
      opaque: color.a >= 0.999,
      scale: step,
      grid: { x: visualLocalGrid.x, y: -visualLocalGrid.z, z: visualLocalGrid.y },
      localPosition: renderLocalPosition,
      localBounds,
      spatialAffine,
      renderBase: base,
      mirrored,
      position: renderPosition,
      stackPosition: renderPosition,
      bounds: transformedVoxelBounds(renderPosition, step, spatialAffine),
      sourceKey,
      sourceKeys: [sourceKey],
      objectOrder: batchIndex,
      objectIds,
    };
  });
}

function requirePositiveInteger(value, label) {
  const number = Number(value);
  if (!Number.isInteger(number) || number <= 0) {
    throw new Error(`Puzzle3 ${label} must be a positive integer.`);
  }
  return number;
}

function requireResolvedCell(value) {
  if (!Array.isArray(value) || value.length !== 3
      || value.some((entry) => !Number.isInteger(Number(entry)))) {
    throw new Error("Puzzle3 resolved batch cell must contain three integers.");
  }
  return { x: Number(value[0]), y: Number(value[1]), z: Number(value[2]) };
}

function requireResolvedVoxelPosition(value, index) {
  if (!Array.isArray(value) || value.length !== 3
      || value.some((entry) => !Number.isInteger(Number(entry)))) {
    throw new Error(`Puzzle3 resolved voxel ${index} position must contain three integers.`);
  }
  return value.map(Number);
}

function requireUnitValue(value, label) {
  const number = Number(value);
  if (!Number.isFinite(number) || number < 0 || number > 1) {
    throw new Error(`Puzzle3 ${label} must be between zero and one.`);
  }
  return number;
}

function resolvedLinearColor(value, opacity) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("Puzzle3 resolved voxel is missing its linear RGBA color.");
  }
  const red = requireUnitValue(value.red, "linear red");
  const green = requireUnitValue(value.green, "linear green");
  const blue = requireUnitValue(value.blue, "linear blue");
  const alpha = requireUnitValue(value.alpha, "linear alpha") * opacity;
  return {
    r: linearSrgbByte(red),
    g: linearSrgbByte(green),
    b: linearSrgbByte(blue),
    a: alpha,
  };
}

function linearSrgbByte(value) {
  const srgb = value <= 0.0031308
    ? 12.92 * value
    : 1.055 * (value ** (1 / 2.4)) - 0.055;
  return Math.round(Math.max(0, Math.min(1, srgb)) * 255);
}

function normalizeViewport(raw) {
  if (raw === null || raw === undefined) {
    return null;
  }
  if (typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error("Puzzle3 viewport must be a typed object.");
  }
  const framing = raw.framingBox;
  if (!framing || typeof framing !== "object" || Array.isArray(framing)) {
    throw new Error("Puzzle3 viewport is missing its framing box.");
  }
  const width = requirePositiveInteger(framing.width, "viewport framing width");
  const depth = requirePositiveInteger(framing.depth, "viewport framing depth");
  const mode = String(raw.mode);
  if (!["full", "centered", "paged"].includes(mode)) {
    throw new Error(`Puzzle3 viewport mode is invalid: ${mode}`);
  }
  const follow = String(raw.follow);
  if (!["snap", "smooth"].includes(follow)) {
    throw new Error(`Puzzle3 viewport follow mode is invalid: ${follow}`);
  }
  if (!Array.isArray(raw.focusObjects)) {
    throw new Error("Puzzle3 viewport is missing typed focus object IDs.");
  }
  const focusObjects = raw.focusObjects.map((objectId) => {
    const id = Number(objectId);
    if (!Number.isInteger(id) || id <= 0) {
      throw new Error("Puzzle3 viewport contains an invalid focus object ID.");
    }
    return id;
  });
  if (mode !== "full" && focusObjects.length === 0) {
    throw new Error("Puzzle3 focused viewport requires at least one focus object ID.");
  }
  return {
    mode,
    follow,
    focusObjects,
    framingBox: {
      width,
      depth,
      height: framing.height === "full"
        ? "full"
        : requirePositiveInteger(framing.height, "viewport framing height"),
    },
  };
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

function pixelateSettings(frame) {
  const raw = frame.settings?.pixelate;
  if (!raw) {
    return { enabled: false, scale: 1, smoothing: true };
  }
  if (raw === true) {
    return { enabled: true, scale: 4, smoothing: true };
  }
  const scale = Math.max(1, Math.trunc(Number(raw.scale ?? raw.size ?? 4) || 4));
  return {
    enabled: raw.enabled !== false && scale > 1,
    scale,
    smoothing: raw.smoothing !== false,
  };
}

function visualShadeEnabled(frame) {
  const raw = frame.settings?.visual;
  if (raw === false) {
    return false;
  }
  return !raw || raw === true || raw.shade !== false;
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

function addMeshes(THREE, scene, visible, shadow, materialCache, shade) {
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
      const material = faceMaterial(THREE, face.fill, materialCache, shade);
      const mesh = new THREE.Mesh(geometry, material);
      mesh.renderOrder = 100 + Number(face.objectOrder || 0);
      mesh.castShadow = false;
      mesh.receiveShadow = false;
      scene.add(mesh);
      continue;
    }
    const key = faceMaterialKey(face.fill, shade);
    if (!opaqueGroups.has(key)) {
      opaqueGroups.set(key, []);
    }
    opaqueGroups.get(key).push(face);
  }
  for (const [key, groupFaces] of opaqueGroups) {
    const geometry = faceBufferGeometry(THREE, groupFaces);
    const material = faceMaterial(THREE, groupFaces[0].fill, materialCache, shade);
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

function frameVisibleVoxels(frame) {
  const stacks = new Map();
  for (const voxel of frame.resolvedVoxels) {
    const key = voxelGeometryKeyAt(voxel.stackPosition, voxel.scale);
    const stack = stacks.get(key) || [];
    stack.push(voxel);
    stacks.set(key, stack);
  }
  const voxels = [];
  const occupied = emptyVoxelOccupancy();
  for (const stack of stacks.values()) {
    const visibleStack = visibleResolvedVoxelStack(stack);
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

function visibleResolvedVoxelStack(stack) {
  const visible = [];
  for (const voxel of [...stack].sort((left, right) => left.objectOrder - right.objectOrder)) {
    if (voxel.color?.a <= 0) {
      continue;
    }
    if (voxel.opaque) {
      visible.length = 0;
    }
    visible.push(voxel);
  }
  return visible;
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
            spatialAffine: voxel.spatialAffine,
            renderBase: voxel.renderBase,
            mirrored: voxel.mirrored === true,
          },
        };
      },
      face: (group, rect) => faceGeometry(
        mergedVoxelFaceCorners(group, rect),
        group.fill,
        group.objectOrder,
        group.side,
        group.mirrored,
      ),
    });
  }
  return voxels.flatMap((voxel) => voxelFaces(voxel)
    .filter((face) => !isVoxelFaceOccluded(voxel, face.offset, occupied))
    .map((face) => faceGeometry(
      face.corners,
      voxel.fill,
      voxel.objectOrder,
      face.side,
      voxel.mirrored,
    )));
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
  const bounds = voxel.localBounds || voxel.bounds;
  const { x0, x1, y0, y1, z0, z1 } = bounds;
  const faces = [
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
  if (!voxel.spatialAffine || !voxel.renderBase) {
    return faces;
  }
  return faces.map((face) => ({
    ...face,
    corners: face.corners.map((corner) => transformRenderLocalPoint(
      corner,
      voxel.spatialAffine,
      voxel.renderBase,
    )),
  }));
}

function voxelFaceGroupInfo(voxel, side) {
  const bounds = voxel.localBounds || voxel.bounds;
  const origin = {
    x: bounds.x0 - voxel.grid.x * voxel.scale,
    y: bounds.y0 - voxel.grid.y * voxel.scale,
    z: bounds.z0 - voxel.grid.z * voxel.scale,
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
  const corners = mergedVoxelFaceLocalCorners(group, rect);
  if (!group.spatialAffine || !group.renderBase) {
    return corners;
  }
  return corners.map((corner) => transformRenderLocalPoint(
    corner,
    group.spatialAffine,
    group.renderBase,
  ));
}

function mergedVoxelFaceLocalCorners(group, rect) {
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

function faceGeometry(corners, fill, objectOrder = 0, side = "", mirrored = false) {
  return { corners, fill, objectOrder, side, mirrored };
}

function faceBufferGeometry(THREE, faces) {
  const positions = [];
  const normals = [];
  for (const face of faces) {
    const normal = faceNormal(face.corners, face.mirrored === true);
    const corners = face.corners || [];
    const indices = face.mirrored
      ? [0, 2, 1, 0, 3, 2]
      : [0, 1, 2, 0, 2, 3];
    for (const index of indices) {
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

function faceNormal(corners, mirrored = false) {
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
  const direction = mirrored ? -1 : 1;
  return {
    x: direction * nx / length,
    y: direction * ny / length,
    z: direction * nz / length,
  };
}

function faceMaterial(THREE, fill, cache, shade) {
  const key = faceMaterialKey(fill, shade);
  if (cache.has(key)) {
    return cache.get(key);
  }
  const color = parseColor(fill);
  const alpha = color ? color.a : 1;
  const Material = shade ? THREE.MeshLambertMaterial : THREE.MeshBasicMaterial;
  if (!Material) {
    throw new Error(`Three.js ${shade ? "MeshLambertMaterial" : "MeshBasicMaterial"} is required.`);
  }
  const material = new Material({
    color: color ? formatRgbColor(color) : fill,
    transparent: alpha < 0.999,
    opacity: Math.max(0, Math.min(1, alpha)),
    depthWrite: alpha >= 0.999,
  });
  cache.set(key, material);
  return material;
}

function faceMaterialKey(fill, shade = true) {
  const color = parseColor(fill);
  const value = color ? `${formatRgbColor(color)}:${Math.max(0, Math.min(1, color.a))}` : `${fill}:1`;
  return `${shade ? "lit" : "flat"}:${value}`;
}

function visualPointToRenderPoint(point) {
  return { x: point.x, y: -point.z, z: point.y };
}

function renderPointToVisualPoint(point) {
  return { x: point.x, y: point.z, z: -point.y };
}

function transformRenderLocalPoint(point, spatialAffine, base) {
  const transformed = Puzzle3VisualCore.transformSpatialPoint(
    renderPointToVisualPoint(point),
    spatialAffine,
  );
  const render = visualPointToRenderPoint(transformed);
  return {
    x: base.x + render.x,
    y: base.y + render.y,
    z: base.z + render.z,
  };
}

function transformedVoxelBounds(center, scale, spatialAffine) {
  const [a, b, c] = spatialAffine;
  const half = scale / 2;
  const extentX = half * (Math.abs(a[0]) + Math.abs(a[2]) + Math.abs(a[1]));
  const extentY = half * (Math.abs(c[0]) + Math.abs(c[2]) + Math.abs(c[1]));
  const extentZ = half * (Math.abs(b[0]) + Math.abs(b[2]) + Math.abs(b[1]));
  return {
    x0: center.x - extentX,
    x1: center.x + extentX,
    y0: center.y - extentY,
    y1: center.y + extentY,
    z0: center.z - extentZ,
    z1: center.z + extentZ,
  };
}

function affineDeterminant3(affine) {
  const [a, b, c] = affine;
  return a[0] * (b[1] * c[2] - b[2] * c[1])
    - a[1] * (b[0] * c[2] - b[2] * c[0])
    + a[2] * (b[0] * c[1] - b[1] * c[0]);
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
  if (voxel.localPosition && voxel.spatialAffine && voxel.renderBase) {
    return voxelGeometryKeyAt(transformRenderLocalPoint({
      x: voxel.localPosition.x + offset.x * voxel.scale,
      y: voxel.localPosition.y + offset.y * voxel.scale,
      z: voxel.localPosition.z + offset.z * voxel.scale,
    }, voxel.spatialAffine, voxel.renderBase), voxel.scale);
  }
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
    y: (frame.size.height - 1) / 2 - position.z,
    z: position.y - (frame.size.depth - 1) / 2,
  };
}

function viewportFocusCell(frame) {
  const viewport = frame.viewport;
  if (!viewport || (viewport.mode !== "centered" && viewport.mode !== "paged")) {
    return null;
  }
  const focusObjects = new Set(viewport.focusObjects || []);
  return frame.cells.find((cell) => viewportObjectMatches(cell, focusObjects)) || null;
}

function viewportObjectMatches(cell, focusObjects) {
  return cell.objectIds.some((objectId) => focusObjects.has(objectId));
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

function buildCamera(THREE, frame, canvas, currentCamera = null) {
  const width = canvas.clientWidth || canvas.width || 1;
  const height = canvas.clientHeight || canvas.height || 1;
  const aspect = width / Math.max(1, height);
  const cameraSettings = frame.camera || {};
  const zoom = cameraZoom(frame);
  const view = cameraViewForFrame(frame, aspect, zoom);
  const targetPoint = new THREE.Vector3(view.target.x, view.target.y, view.target.z);
  const distance = view.distance;
  const cameraFrame = cameraRenderFrame(cameraSettings);
  const projection = cameraSettings.projection;
  const near = 0.1;
  const far = Math.max(1000, distance * 4);
  const orthographic = projection === "orthographic";
  const camera = orthographic
    ? updateOrthographicCamera(
        currentCamera?.isOrthographicCamera ? currentCamera : null,
        THREE,
        aspect,
        view.visibleHeight,
        near,
        far,
      )
    : updatePerspectiveCamera(
        currentCamera?.isPerspectiveCamera ? currentCamera : null,
        THREE,
        aspect,
        near,
        far,
      );
  camera.up.set(cameraFrame.up.x, cameraFrame.up.y, cameraFrame.up.z);
  camera.position.set(
    targetPoint.x - cameraFrame.forward.x * distance,
    targetPoint.y - cameraFrame.forward.y * distance,
    targetPoint.z - cameraFrame.forward.z * distance,
  );
  camera.lookAt(targetPoint);
  return camera;
}

function updatePerspectiveCamera(camera, THREE, aspect, near, far) {
  const next = camera || new THREE.PerspectiveCamera(34, aspect, near, far);
  next.fov = 34;
  next.aspect = aspect;
  next.near = near;
  next.far = far;
  next.updateProjectionMatrix?.();
  return next;
}

function updateOrthographicCamera(camera, THREE, aspect, visibleHeight, near, far) {
  const height = Math.max(1, Number(visibleHeight) || 1);
  const width = height * Math.max(0.01, aspect);
  const next = camera || buildOrthographicCamera(THREE, aspect, visibleHeight, near, far);
  next.left = -width / 2;
  next.right = width / 2;
  next.top = height / 2;
  next.bottom = -height / 2;
  next.near = near;
  next.far = far;
  next.updateProjectionMatrix?.();
  return next;
}

function threeViewPayload(frame, camera, canvas) {
  const width = Math.max(1, Number(canvas.clientWidth) || Number(canvas.width) || 1);
  const height = Math.max(1, Number(canvas.clientHeight) || Number(canvas.height) || 1);
  const rect = canvas.getBoundingClientRect();
  const cameraView = frame.cameraView || cameraViewForFrame(frame, width / height, cameraZoom(frame));
  const payload = {
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
      zoom: Puzzle3VisualCore.normalizeZoom(frame.camera?.zoom ?? frame.editorView?.zoom),
      projection: frame.camera.projection,
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
  payload.cellFootprints = threeStageCellFootprints(payload);
  return payload;
}

function threeStageCellFootprints(view) {
  const size = view.threeProjection.size;
  const footprints = [];
  for (let y = 0; y < size.depth; y += 1) {
    for (let x = 0; x < size.width; x += 1) {
      footprints.push({
        position: { x, y, z: 0 },
        points: [
          { x: x - 0.5, y: y - 0.5, z: 0 },
          { x: x + 0.5, y: y - 0.5, z: 0 },
          { x: x + 0.5, y: y + 0.5, z: 0 },
          { x: x - 0.5, y: y + 0.5, z: 0 },
        ].map((point) => threeProjectLogicalPoint(point, view)),
      });
    }
  }
  return footprints;
}

function threeProjectLogicalPoint(position, view) {
  const projection = view.threeProjection;
  const size = projection.size;
  const cameraFrame = cameraRenderFrame(view.camera || {});
  const cameraPosition = {
    x: projection.target.x - cameraFrame.forward.x * projection.distance,
    y: projection.target.y - cameraFrame.forward.y * projection.distance,
    z: projection.target.z - cameraFrame.forward.z * projection.distance,
  };
  const world = {
    x: Number(position.x) - (size.width - 1) / 2,
    y: (size.height - 1) / 2 - Number(position.z),
    z: Number(position.y) - (size.depth - 1) / 2,
  };
  const relative = {
    x: world.x - cameraPosition.x,
    y: world.y - cameraPosition.y,
    z: world.z - cameraPosition.z,
  };
  const cameraX = dotVector3(relative, cameraFrame.right);
  const cameraY = dotVector3(relative, cameraFrame.up);
  const cameraDepth = Math.max(0.0001, dotVector3(relative, cameraFrame.forward));
  let ndcX;
  let ndcY;
  if (projection.projection === "orthographic") {
    const visibleWidth = projection.visibleHeight * projection.aspect;
    ndcX = cameraX / (visibleWidth / 2);
    ndcY = cameraY / (projection.visibleHeight / 2);
  } else {
    const tanHalfFov = Math.tan(degreesToRadians(projection.fovDegrees) / 2);
    ndcX = cameraX / (cameraDepth * tanHalfFov * projection.aspect);
    ndcY = cameraY / (cameraDepth * tanHalfFov);
  }
  return {
    x: ((ndcX + 1) / 2) * view.width,
    y: ((1 - ndcY) / 2) * view.height,
  };
}

function dotVector3(left, right) {
  return left.x * right.x + left.y * right.y + left.z * right.z;
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
  const cameraValue = Puzzle3VisualCore.normalizeZoom(cameraSettings.zoom);
  const viewValue = Puzzle3VisualCore.normalizeZoom(frame.editorView?.zoom);
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
  const focusedObjectIds = new Set(frame.focusCell?.objectIds || []);
  const focusBase = renderPositionForCell(frame, frame.focusCell?.position || {});
  const bounds = {
    minX: Infinity,
    maxX: -Infinity,
    minY: Infinity,
    maxY: -Infinity,
    minZ: Infinity,
    maxZ: -Infinity,
  };
  for (const voxel of frame.resolvedVoxels) {
    if (voxel.renderBase.x !== focusBase.x
        || voxel.renderBase.y !== focusBase.y
        || voxel.renderBase.z !== focusBase.z) {
      continue;
    }
    if (focusedObjectIds.size > 0
        && !voxel.objectIds.some((objectId) => focusedObjectIds.has(objectId))) {
      continue;
    }
    bounds.minX = Math.min(bounds.minX, voxel.bounds.x0);
    bounds.maxX = Math.max(bounds.maxX, voxel.bounds.x1);
    bounds.minY = Math.min(bounds.minY, voxel.bounds.y0);
    bounds.maxY = Math.max(bounds.maxY, voxel.bounds.y1);
    bounds.minZ = Math.min(bounds.minZ, voxel.bounds.z0);
    bounds.maxZ = Math.max(bounds.maxZ, voxel.bounds.z1);
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
  const pitch = degreesToRadians(clamp(Number(cameraSettings.pitchDegrees ?? 35), -90, 90));
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
  cameraRenderFrame,
  frameVisibleVoxels,
  mergedVoxelFaces,
};
})();
